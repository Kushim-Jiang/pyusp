// selfhook.rs — minimal inline-hook engine for the *loaded* usp10.dll,
// ported from the DWriteCore tracer (pydwshape/src/lib.rs). Reused to hook an
// internal Uniscribe lookup dispatcher once the RE delivers:
//   - the dispatcher RVA / prologue signature (see src/lookup_hook.rs TODO),
//   - which register/stack slot carries the glyph-buffer object,
//   - the glyph-record layout (offset of gid / flags / count).
//
// The engine itself is generic over those parameters; nothing here is
// usp10-specific except the calling code that supplies them.

#![allow(clippy::missing_safety_doc)]

use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr;

use windows::Win32::System::Memory::{
    VirtualAlloc, VirtualFree, VirtualProtect, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE,
    PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS,
};

/// Where on entry to the hooked function the glyph-buffer object lives.
#[derive(Clone, Copy)]
pub struct ArgLoc {
    /// register (0=rcx,1=rdx,2=r8,3=r9) or -1 for a stack slot
    pub reg: i8,
    /// stack slot index (1-based arg number) when reg<0 → [rsp+0x28+(n-5)*8]
    pub stack_arg: i8,
}

pub struct HookSpec {
    pub expected_rva: u64,
    pub prologue: Vec<u8>,
    pub arg: ArgLoc,
    /// glyph-buffer object offsets
    pub rec_ptr_off: u64, // [obj + off] -> pointer to glyph records
    pub count_off: u64,   // [obj + off] -> glyph count
    pub rec_stride: u64,  // bytes per glyph record
    pub gid_off: u64,     // u16 glyph id offset within a record
}

#[derive(Clone, PartialEq)]
pub struct GlyphRec {
    pub gid: u16,
    pub extra: u16,
    pub idx: u16,
}

pub struct SnapEvt {
    pub table: String,
    pub n: u32,
    pub recs: Vec<GlyphRec>,
}

struct Ctx {
    pub active: bool,
    pub table: String,
    pub snaps: Vec<SnapEvt>,
    pub rec_ptr_off: u64,
    pub count_off: u64,
    pub rec_stride: u64,
    pub gid_off: u64,
}

thread_local! {
    static CTX: RefCell<Ctx> = RefCell::new(Ctx {
        active: false,
        table: "GSUB".into(),
        snaps: Vec::new(),
        rec_ptr_off: 0,
        count_off: 0,
        rec_stride: 8,
        gid_off: 0,
    });
}

unsafe fn rd_u16(p: *const u8, off: usize) -> u16 {
    ptr::read_unaligned(p.add(off) as *const u16)
}
unsafe fn rd_u64(p: *const u8, off: usize) -> u64 {
    ptr::read_unaligned(p.add(off) as *const u64)
}

/// Rust side of the hook: called with the glyph-buffer object pointer.
extern "C" fn on_lookup(buf: u64) -> u64 {
    CTX.with(|s| {
        let mut st = s.borrow_mut();
        if !st.active || buf == 0 {
            return;
        }
        let spec = (st.rec_ptr_off, st.count_off, st.rec_stride, st.gid_off);
        let obj = buf as *const u8;
        let recptr = unsafe { rd_u64(obj, spec.0 as usize) };
        let count = unsafe { rd_u64(obj, spec.1 as usize) } as usize;
        if recptr == 0 || count == 0 || count > 8192 {
            return;
        }
        let base = recptr as *const u8;
        let mut recs: Vec<GlyphRec> = Vec::with_capacity(count);
        for i in 0..count {
            let o = (i as u64 * spec.2) as usize;
            recs.push(GlyphRec {
                gid: unsafe { rd_u16(base, o + spec.3 as usize) },
                extra: unsafe { rd_u16(base, o + spec.3 as usize + 2) },
                idx: unsafe { rd_u16(base, o + spec.3 as usize + 4) },
            });
        }
        let table = st.table.clone();
        st.snaps.push(SnapEvt {
            table,
            n: count as u32,
            recs,
        });
    });
    0
}

fn abs_jmp(dst: u64) -> Vec<u8> {
    let mut v = vec![0x48, 0xB8];
    v.extend_from_slice(&dst.to_le_bytes());
    v.extend_from_slice(&[0xFF, 0xE0]);
    v
}

unsafe fn alloc_near(module: u64, size: usize) -> *mut c_void {
    let hint_start = (module + 0x1000 + 0xFFF) & !0xFFF;
    for i in 0..4096u64 {
        let hint = hint_start.wrapping_add(i * 0x10000);
        let p = VirtualAlloc(
            Some(hint as *const c_void),
            size,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        );
        if !p.is_null() {
            let d = (p as i64 - module as i64).unsigned_abs();
            if d <= i32::MAX as u64 {
                return p;
            }
            let _ = VirtualFree(p, 0, MEM_RELEASE);
        }
    }
    panic!("could not allocate scratch page near module {module:#x}");
}

/// load r10 with the selected argument value.
fn load_arg(arg: ArgLoc) -> Vec<u8> {
    if arg.reg >= 0 {
        let m = match arg.reg {
            0 => vec![0x49, 0x89, 0xCA], // mov r10, rcx
            1 => vec![0x49, 0x89, 0xD2], // mov r10, rdx
            2 => vec![0x4D, 0x89, 0xC2], // mov r10, r8
            _ => vec![0x4D, 0x89, 0xCA], // mov r10, r9
        };
        m
    } else {
        // mov r10, [rsp + 0x28 + (stack_arg-5)*8]
        let n = arg.stack_arg;
        let off = 0x28u32.wrapping_add(((n as i32) - 5) as u32 * 8);
        if off <= 0x7F {
            vec![0x4C, 0x8B, 0x54, 0x24, off as u8] // mov r10,[rsp+off]
        } else {
            vec![
                0x4C, 0x8B, 0x94, 0x24, (off & 0xFF) as u8, (off >> 8) as u8, 0, 0,
            ]
        }
    }
}

/// Install a hook at `target` whose prologue is `orig[0..len]`; on entry the
/// stub calls `on_lookup` with the configured argument, then runs the
/// trampoline (orig + jump back). Returns an RAII restore guard.
pub struct HookGuard {
    target: u64,
    block: *mut c_void,
    restore: Vec<u8>,
}

impl Drop for HookGuard {
    fn drop(&mut self) {
        unsafe {
            let mut old = PAGE_PROTECTION_FLAGS(0);
            let page = (self.target & !0xFFF) as *const c_void;
            VirtualProtect(page, 0x1000, PAGE_EXECUTE_READWRITE, &mut old).ok();
            ptr::copy_nonoverlapping(self.restore.as_ptr(), self.target as *mut u8, self.restore.len());
            let _ = VirtualProtect(page, 0x1000, old, &mut old);
            let _ = VirtualFree(self.block, 0, MEM_RELEASE);
        }
    }
}

/// Install the hook. `base` = loaded module base; `rva` = verified target
/// offset (see pe::verify_or_relocate); `spec` carries prologue + arg layout.
pub unsafe fn install(base: u64, rva: u64, spec: &HookSpec) -> HookGuard {
    let target = base + rva;
    let orig = ptr::slice_from_raw_parts(target as *const u8, spec.prologue.len());
    let orig: &[u8] = &*orig;
    debug_assert_eq!(orig, spec.prologue.as_slice(), "prologue mismatch at hook");

    let block = alloc_near(base, 0x300);
    let block_base = block as u64;
    let tramp = block_base;
    let stub = block_base + 0x100;

    // trampoline: orig[0..12] + abs jmp (target+12)
    let mut t: Vec<u8> = Vec::new();
    let n = spec.prologue.len().min(12);
    t.extend_from_slice(&orig[..n]);
    t.extend_from_slice(&abs_jmp(target + n as u64));
    ptr::copy_nonoverlapping(t.as_ptr(), tramp as *mut u8, t.len());

    // stub
    let mut s: Vec<u8> = Vec::new();
    s.extend_from_slice(&load_arg(spec.arg));
    s.extend_from_slice(&[0x48, 0x83, 0xEC, 0x48]); // sub rsp,0x48
    s.extend_from_slice(&[0x48, 0x89, 0x4C, 0x24, 0x20]); // [rsp+20]=rcx
    s.extend_from_slice(&[0x48, 0x89, 0x54, 0x24, 0x28]); // [rsp+28]=rdx
    s.extend_from_slice(&[0x4C, 0x89, 0x44, 0x24, 0x30]); // [rsp+30]=r8
    s.extend_from_slice(&[0x4C, 0x89, 0x4C, 0x24, 0x38]); // [rsp+38]=r9
    s.extend_from_slice(&[0x4C, 0x89, 0xD1]); // mov rcx, r10
    s.extend_from_slice(&[0x48, 0xB8]);
    s.extend_from_slice(&(on_lookup as *const () as usize as u64).to_le_bytes());
    s.extend_from_slice(&[0xFF, 0xD0]); // call rax
    s.extend_from_slice(&[0x48, 0x8B, 0x4C, 0x24, 0x20]);
    s.extend_from_slice(&[0x48, 0x8B, 0x54, 0x24, 0x28]);
    s.extend_from_slice(&[0x4C, 0x8B, 0x44, 0x24, 0x30]);
    s.extend_from_slice(&[0x4C, 0x8B, 0x4C, 0x24, 0x38]);
    s.extend_from_slice(&[0x48, 0x83, 0xC4, 0x48]); // add rsp,0x48
    s.extend_from_slice(&abs_jmp(tramp));
    ptr::copy_nonoverlapping(s.as_ptr(), stub as *mut u8, s.len());

    // patch target
    let patch = abs_jmp(stub);
    let restore = orig[..patch.len()].to_vec();
    let mut old = PAGE_PROTECTION_FLAGS(0);
    let page = (target & !0xFFF) as *const c_void;
    VirtualProtect(page, 0x1000, PAGE_EXECUTE_READWRITE, &mut old).ok();
    ptr::copy_nonoverlapping(patch.as_ptr(), target as *mut u8, patch.len());
    let _ = VirtualProtect(page, 0x1000, old, &mut old);

    HookGuard {
        target,
        block,
        restore,
    }
}

/// Begin/end a capture window (called on our own thread before shaping).
pub fn begin(rec_ptr_off: u64, count_off: u64, rec_stride: u64, gid_off: u64) {
    CTX.with(|s| {
        let mut st = s.borrow_mut();
        st.active = true;
        st.snaps.clear();
        st.rec_ptr_off = rec_ptr_off;
        st.count_off = count_off;
        st.rec_stride = rec_stride;
        st.gid_off = gid_off;
    });
}

pub fn end() -> Vec<SnapEvt> {
    CTX.with(|s| {
        let mut st = s.borrow_mut();
        st.active = false;
        std::mem::take(&mut st.snaps)
    })
}

/// Accessor for tooling: counts per snapshot.
pub fn snapshot_glyphs(s: &SnapEvt) -> Vec<u16> {
    s.recs.iter().map(|r| r.gid).collect()
}
