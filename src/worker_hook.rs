// worker_hook.rs — in-process inline hooks over thread/threadpool primitives.
//
// RE: gdi32full's ScriptShapeOpenType OT engine is not visibly on the calling
// thread; we hook the thread/threadpool entry points ntdll/kernel32 expose so
// we can observe (from inside our own process, no agent thread to fight)
// whether a worker callback lands in gdi32full, and capture its RVA.

#![allow(clippy::missing_safety_doc)]

use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr;
use std::sync::Mutex;

use windows::core::PCSTR;
use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
use windows::Win32::System::Memory::{
    VirtualAlloc, VirtualFree, VirtualProtect, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE,
    PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS,
};

/// (tag, callback-pointer) recorded for every captured invocation.
pub static EVENTS: Mutex<Vec<(u32, u64)>> = Mutex::new(Vec::new());

const TAG_POST: u32 = 1; // TpSimpleTryPost / TpPostWork (arg0 = callback)
const TAG_ALLOC: u32 = 2; // TpAllocWork / CreateThreadpoolWork (arg0 = callback)
const TAG_THREAD: u32 = 3; // CreateThread (arg2 = start routine)
const TAG_SUBMIT: u32 = 4; // SubmitThreadpoolWork (arg0 = work object)

/// Stashed caller glyph-out buffer (ptr, cap) set right before shaping, so a
/// per-fire snapshot reveals whether gdi32full mutates the caller buffer in
/// place (== a cheap native per-step glyph trace source).
pub static GLYPH_BUF: Mutex<Option<(u64, usize)>> = Mutex::new(None);
/// Per-fire snapshots of the stashed buffer: (tag=rva-as-u32, Vec<u16>).
pub static GLYPH_STEPS: Mutex<Vec<(u64, Vec<u16>)>> = Mutex::new(Vec::new());

/// Stash the caller glyph-out buffer for the next shape, and clear prior
/// step snapshots. `cap` is the buffer length (full zero-padded window).
pub fn set_glyph_buf(ptr: *mut u16, cap: usize) {
    if let Ok(mut g) = GLYPH_BUF.lock() {
        *g = Some((ptr as u64, cap));
    }
    if let Ok(mut s) = GLYPH_STEPS.lock() {
        s.clear();
    }
}

pub fn drain_glyph_steps() -> Vec<(u64, Vec<u16>)> {
    std::mem::take(&mut *GLYPH_STEPS.lock().unwrap())
}

// ---- return-address capture probe ----
// Records (hooked-rva, caller-return-address) per fire so the per-glyph
// driver loop that calls the tag getters can be located statically.
pub static RETS: Mutex<Vec<(u64, u64)>> = Mutex::new(Vec::new());

unsafe extern "C" fn on_ret(tag: u32, v: u64) -> u64 {
    if let Ok(mut r) = RETS.lock() {
        r.push((tag as u64, v));
    }
    0
}

pub unsafe fn arm_rvas_ret(base: u64, rvas: &[(u64, usize)]) -> Guard {
    if let Ok(mut r) = RETS.lock() {
        r.clear();
    }
    let mut hooks = Vec::new();
    for &(rva, prologue) in rvas {
        if prologue < 5 {
            continue;
        }
        hooks.push(hook_boundary_ret(base + rva, rva as u32, prologue));
    }
    Guard { hooks }
}

pub fn drain_rets() -> Vec<(u64, u64)> {
    std::mem::take(&mut *RETS.lock().unwrap())
}

/// Like `hook_one_boundary` but the stub captures the *return address* (the
/// caller's continuation) instead of an argument — [rsp+0x50] after our
/// `sub rsp,0x50` == the original return address pushed by the `call`.
unsafe fn hook_boundary_ret(target: u64, tag: u32, prologue: usize) -> One {
    let cap = prologue.clamp(5, 64);
    let orig: &[u8] = &*ptr::slice_from_raw_parts(target as *const u8, cap);
    let block = alloc_near(target, 0x300);
    let block_base = block as u64;
    let tramp = block_base;
    let stub = block_base + 0x100;

    let mut t: Vec<u8> = Vec::new();
    t.extend_from_slice(&orig[..cap]);
    t.extend_from_slice(&rel_jmp(tramp + t.len() as u64, target + cap as u64));
    ptr::copy_nonoverlapping(t.as_ptr(), tramp as *mut u8, t.len());

    let mut s: Vec<u8> = Vec::new();
    s.extend_from_slice(&[0x48, 0x83, 0xEC, 0x50]); // sub rsp,0x50
    s.extend_from_slice(&[0x48, 0x89, 0x4C, 0x24, 0x20]); // save rcx
    s.extend_from_slice(&[0x48, 0x89, 0x54, 0x24, 0x28]); // save rdx
    s.extend_from_slice(&[0x4C, 0x89, 0x44, 0x24, 0x30]); // save r8
    s.extend_from_slice(&[0x4C, 0x89, 0x4C, 0x24, 0x38]); // save r9
    // rcx = tag (imm64)
    s.extend_from_slice(&[0x48, 0xB9]);
    s.extend_from_slice(&(tag as u64).to_le_bytes());
    // rdx = [rsp+0x50]  (original return address)
    s.extend_from_slice(&[0x48, 0x8B, 0x54, 0x24, 0x50]);
    // rax = on_ret; call rax
    s.extend_from_slice(&[0x48, 0xB8]);
    s.extend_from_slice(&(on_ret as *const () as usize as u64).to_le_bytes());
    s.extend_from_slice(&[0xFF, 0xD0]);
    // restore regs
    s.extend_from_slice(&[0x48, 0x8B, 0x4C, 0x24, 0x20]);
    s.extend_from_slice(&[0x48, 0x8B, 0x54, 0x24, 0x28]);
    s.extend_from_slice(&[0x4C, 0x8B, 0x44, 0x24, 0x30]);
    s.extend_from_slice(&[0x4C, 0x8B, 0x4C, 0x24, 0x38]);
    s.extend_from_slice(&[0x48, 0x83, 0xC4, 0x50]); // add rsp,0x50
    s.extend_from_slice(&rel_jmp(stub + s.len() as u64, tramp));
    ptr::copy_nonoverlapping(s.as_ptr(), stub as *mut u8, s.len());

    // patch target with 5-byte rel32 jmp -> stub
    let patch = rel_jmp(target, stub);
    let restore = orig[..5].to_vec();
    let mut old = PAGE_PROTECTION_FLAGS(0);
    let page = (target & !0xFFF) as *const c_void;
    VirtualProtect(page, 0x1000, PAGE_EXECUTE_READWRITE, &mut old).ok();
    ptr::copy_nonoverlapping(patch.as_ptr(), target as *mut u8, patch.len());
    let _ = VirtualProtect(page, 0x1000, old, &mut old);

    One { target, block, restore }
}

unsafe extern "C" fn on_capture(tag: u32, v: u64) -> u64 {
    if let Ok(mut e) = EVENTS.lock() {
        e.push((tag, v));
    }
    // glyph snapshot of the stashed caller buffer (in-place mutation test)
    if let Ok(guard) = GLYPH_BUF.lock() {
        if let Some((ptr, cap)) = *guard {
            if ptr != 0 && cap != 0 {
                let n = cap.min(256);
                let mut buf = vec![0u16; n];
                std::ptr::copy_nonoverlapping(ptr as *const u16, buf.as_mut_ptr(), n);
                drop(guard);
                if let Ok(mut s) = GLYPH_STEPS.lock() {
                    s.push((tag as u64, buf));
                }
                return 0;
            }
        }
    }
    0
}

fn abs_jmp(dst: u64) -> Vec<u8> {
    let mut v = vec![0x48, 0xB8];
    v.extend_from_slice(&dst.to_le_bytes());
    v.extend_from_slice(&[0xFF, 0xE0]);
    v
}

/// 5-byte E9 rel32 jump from `from` to `to` (both must be within ±2GB).
fn rel_jmp(from: u64, to: u64) -> Vec<u8> {
    let d = to.wrapping_sub(from.wrapping_add(5)) as i32;
    let mut v = vec![0xE9];
    v.extend_from_slice(&d.to_le_bytes());
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
    panic!("worker_hook: could not allocate scratch near {module:#x}");
}

struct One {
    target: u64,
    block: *mut c_void,
    restore: Vec<u8>,
}

impl Drop for One {
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

pub struct Guard {
    hooks: Vec<One>,
}

unsafe fn module_base(name: &str) -> Option<u64> {
    let c = std::ffi::CString::new(name).ok()?;
    GetModuleHandleA(PCSTR(c.as_ptr() as *const u8))
        .ok()
        .map(|h| h.0 as u64)
}

unsafe fn export_addr(modname: &str, name: &str) -> Option<u64> {
    let mc = std::ffi::CString::new(modname).ok()?;
    let nc = std::ffi::CString::new(name).ok()?;
    let hm = GetModuleHandleA(PCSTR(mc.as_ptr() as *const u8)).ok()?;
    let p = GetProcAddress(hm, PCSTR(nc.as_ptr() as *const u8))?;
    Some(p as usize as u64)
}

unsafe fn hook_one(target: u64, arg: i8, tag: u32) -> One {
    let orig: &[u8] = &*ptr::slice_from_raw_parts(target as *const u8, 12);
    let block = alloc_near(target, 0x300);
    let block_base = block as u64;
    let tramp = block_base;
    let stub = block_base + 0x100;

    let mut t: Vec<u8> = Vec::new();
    t.extend_from_slice(&orig[..12]);
    t.extend_from_slice(&abs_jmp(target + 12));
    ptr::copy_nonoverlapping(t.as_ptr(), tramp as *mut u8, t.len());

    let mut s: Vec<u8> = Vec::new();
    s.extend_from_slice(&[0x48, 0x83, 0xEC, 0x50]); // sub rsp,0x50
    s.extend_from_slice(&[0x48, 0x89, 0x4C, 0x24, 0x20]); // [rsp+20]=rcx
    s.extend_from_slice(&[0x48, 0x89, 0x54, 0x24, 0x28]); // [rsp+28]=rdx
    s.extend_from_slice(&[0x4C, 0x89, 0x44, 0x24, 0x30]); // [rsp+30]=r8
    s.extend_from_slice(&[0x4C, 0x89, 0x4C, 0x24, 0x38]); // [rsp+38]=r9
    // mov rcx, tag (imm64)
    s.extend_from_slice(&[0x48, 0xB9]);
    s.extend_from_slice(&(tag as u64).to_le_bytes());
    // rdx = selected original arg
    let sel = match arg {
        0 => [0x48, 0x8B, 0x54, 0x24, 0x20],
        1 => [0x48, 0x8B, 0x54, 0x24, 0x28],
        2 => [0x48, 0x8B, 0x54, 0x24, 0x30],
        _ => [0x48, 0x8B, 0x54, 0x24, 0x38],
    };
    s.extend_from_slice(&sel);
    s.extend_from_slice(&[0x48, 0xB8]);
    s.extend_from_slice(&(on_capture as *const () as usize as u64).to_le_bytes());
    s.extend_from_slice(&[0xFF, 0xD0]); // call rax
    s.extend_from_slice(&[0x48, 0x8B, 0x4C, 0x24, 0x20]);
    s.extend_from_slice(&[0x48, 0x8B, 0x54, 0x24, 0x28]);
    s.extend_from_slice(&[0x4C, 0x8B, 0x44, 0x24, 0x30]);
    s.extend_from_slice(&[0x4C, 0x8B, 0x4C, 0x24, 0x38]);
    s.extend_from_slice(&[0x48, 0x83, 0xC4, 0x50]);
    s.extend_from_slice(&abs_jmp(tramp));
    ptr::copy_nonoverlapping(s.as_ptr(), stub as *mut u8, s.len());

    let patch = abs_jmp(stub);
    let restore = orig[..patch.len()].to_vec();
    let mut old = PAGE_PROTECTION_FLAGS(0);
    let page = (target & !0xFFF) as *const c_void;
    VirtualProtect(page, 0x1000, PAGE_EXECUTE_READWRITE, &mut old).ok();
    ptr::copy_nonoverlapping(patch.as_ptr(), target as *mut u8, patch.len());
    let _ = VirtualProtect(page, 0x1000, old, &mut old);

    One { target, block, restore }
}

/// Arm hooks on the thread/threadpool primitives.
pub unsafe fn arm() -> Guard {
    if let Ok(mut e) = EVENTS.lock() {
        e.clear();
    }
    let mut hooks = Vec::new();
    for (m, name, arg, tag) in [
        ("ntdll.dll", "TpSimpleTryPost", 0, TAG_POST),
        ("ntdll.dll", "TpPostWork", 0, TAG_POST),
        ("ntdll.dll", "TpAllocWork", 0, TAG_ALLOC),
        ("kernel32.dll", "CreateThreadpoolWork", 0, TAG_ALLOC),
        ("kernel32.dll", "TrySubmitThreadpoolCallback", 0, TAG_ALLOC),
        ("kernel32.dll", "CreateThread", 2, TAG_THREAD),
    ] {
        if let Some(a) = export_addr(m, name) {
            hooks.push(hook_one(a, arg, tag));
        }
    }
    Guard { hooks }
}

pub fn drain() -> Vec<(u32, u64)> {
    let mut out = Vec::new();
    if let Ok(mut e) = EVENTS.lock() {
        out.append(&mut e);
    }
    out
}

thread_local! {
    static PROBE: RefCell<Option<Guard>> = const { RefCell::new(None) };
}

/// Arm once per thread (keeps the guard alive for the thread's lifetime).
pub fn arm_once() {
    PROBE.with(|p| {
        let mut b = p.borrow_mut();
        if b.is_none() {
            *b = Some(unsafe { arm() });
        }
    });
}

/// Arm counters on a list of absolute RVAs in a loaded module (used to find
/// which callees of a driver function actually fire). Each hit pushes
/// (rva as tag, 0); callers aggregate. `pros` is the per-RVA safe prologue
/// length (an instruction boundary ≥5, from objdump) used to build a correct
/// trampoline.
pub unsafe fn arm_rvas(base: u64, rvas: &[(u64, usize)]) -> Guard {
    if let Ok(mut e) = EVENTS.lock() {
        e.clear();
    }
    let mut hooks = Vec::new();
    for &(rva, prologue) in rvas {
        if prologue < 5 {
            continue;
        }
        let target = base + rva;
        hooks.push(hook_one_boundary(target, 0, rva as u32, prologue));
    }
    Guard { hooks }
}

/// Boundary-aware inline hook: 5-byte E9 patch to `stub`; trampoline replays
/// `orig[0..prologue]` (an instruction boundary) then E9-jumps back to
/// `target+prologue`. Avoids replaying a partial instruction.
unsafe fn hook_one_boundary(target: u64, arg: i8, tag: u32, prologue: usize) -> One {
    let cap = prologue.clamp(5, 64);
    let orig: &[u8] = &*ptr::slice_from_raw_parts(target as *const u8, cap);
    let block = alloc_near(target, 0x300);
    let block_base = block as u64;
    let tramp = block_base;
    let stub = block_base + 0x100;

    // trampoline: orig[0..cap] + rel32 jmp -> target+cap
    let mut t: Vec<u8> = Vec::new();
    t.extend_from_slice(&orig[..cap]);
    t.extend_from_slice(&rel_jmp(tramp + t.len() as u64, target + cap as u64));
    ptr::copy_nonoverlapping(t.as_ptr(), tramp as *mut u8, t.len());

    // stub: save rcx/rdx/r8/r9, call on_capture(tag, selected arg), restore
    let mut s: Vec<u8> = Vec::new();
    s.extend_from_slice(&[0x48, 0x83, 0xEC, 0x50]); // sub rsp,0x50
    s.extend_from_slice(&[0x48, 0x89, 0x4C, 0x24, 0x20]);
    s.extend_from_slice(&[0x48, 0x89, 0x54, 0x24, 0x28]);
    s.extend_from_slice(&[0x4C, 0x89, 0x44, 0x24, 0x30]);
    s.extend_from_slice(&[0x4C, 0x89, 0x4C, 0x24, 0x38]);
    s.extend_from_slice(&[0x48, 0xB9]); // mov rcx, tag (imm64)
    s.extend_from_slice(&(tag as u64).to_le_bytes());
    let sel = match arg {
        0 => [0x48, 0x8B, 0x54, 0x24, 0x20],
        1 => [0x48, 0x8B, 0x54, 0x24, 0x28],
        2 => [0x48, 0x8B, 0x54, 0x24, 0x30],
        _ => [0x48, 0x8B, 0x54, 0x24, 0x38],
    };
    s.extend_from_slice(&sel);
    s.extend_from_slice(&[0x48, 0xB8]); // mov rax, on_capture
    s.extend_from_slice(&(on_capture as *const () as usize as u64).to_le_bytes());
    s.extend_from_slice(&[0xFF, 0xD0]); // call rax
    s.extend_from_slice(&[0x48, 0x8B, 0x4C, 0x24, 0x20]);
    s.extend_from_slice(&[0x48, 0x8B, 0x54, 0x24, 0x28]);
    s.extend_from_slice(&[0x4C, 0x8B, 0x44, 0x24, 0x30]);
    s.extend_from_slice(&[0x4C, 0x8B, 0x4C, 0x24, 0x38]);
    s.extend_from_slice(&[0x48, 0x83, 0xC4, 0x50]); // add rsp,0x50
    s.extend_from_slice(&rel_jmp(stub + s.len() as u64, tramp));
    ptr::copy_nonoverlapping(s.as_ptr(), stub as *mut u8, s.len());

    // patch target with 5-byte rel32 jmp -> stub
    let patch = rel_jmp(target, stub);
    let restore = orig[..5].to_vec();
    let mut old = PAGE_PROTECTION_FLAGS(0);
    let page = (target & !0xFFF) as *const c_void;
    VirtualProtect(page, 0x1000, PAGE_EXECUTE_READWRITE, &mut old).ok();
    ptr::copy_nonoverlapping(patch.as_ptr(), target as *mut u8, patch.len());
    let _ = VirtualProtect(page, 0x1000, old, &mut old);

    One { target, block, restore }
}

// ---- native per-application run capture (PoC, PYUSP_NATIVE_RUN) ----
// TextShaping driver 0x15650 = "apply one OT op over the run". At its entry,
// r9 (4th arg) points to a stack slot whose qword value is the heap u16
// glyph-run array; each record is {u16 gid, u16 flag, u16 logIdx, u16 1}.
// This lightweight hook reads the gid run at every fire so the native
// per-application trace can be recorded in-process (no frida needed).
pub static RUN_STEPS: Mutex<Vec<(u32, Vec<u16>)>> = Mutex::new(Vec::new());

pub fn run_clear() {
    if let Ok(mut s) = RUN_STEPS.lock() {
        s.clear();
    }
}

pub fn drain_run_steps() -> Vec<(u32, Vec<u16>)> {
    let mut out = Vec::new();
    if let Ok(mut s) = RUN_STEPS.lock() {
        out.append(&mut s);
    }
    out
}

unsafe extern "C" fn on_capture_run(tag: u32, v: u64) -> u64 {
    // Fail-safe guards: only trust user-mode canonical pointers. On a build
    // where the signature matched a different function, r9 may not be a
    // pointer — skip instead of faulting the shaping thread.
    if v < 0x10000 || v > 0x0000_7fff_ffff_ffff {
        return 0;
    }
    let run = *(v as *const u64); // deref the stack slot -> heap run ptr
    if run < 0x10000 || run > 0x0000_7fff_ffff_ffff {
        return 0;
    }
    // records {gid, flag, logIdx, _}: collect gids while logIdx increments.
    let mut gids: Vec<u16> = Vec::with_capacity(16);
    let mut prev: i32 = -1;
    for i in 0..24usize {
        let rec = (run as *const u16).add(i * 4);
        let g = ptr::read_volatile(rec);
        let idx = ptr::read_volatile(rec.add(2)) as i32;
        if g == 0 {
            break;
        }
        if i > 0 && idx != prev + 1 {
            break;
        }
        prev = idx;
        gids.push(g);
    }
    if let Ok(mut s) = RUN_STEPS.lock() {
        s.push((tag, gids));
    }
    0
}

/// Arm run capture on a set of RVAs (each hooked at entry, capturing r9).
pub unsafe fn arm_rvas_run(base: u64, rvas: &[(u64, usize)]) -> Guard {
    run_clear();
    let mut hooks = Vec::new();
    for &(rva, prologue) in rvas {
        if prologue < 5 {
            continue;
        }
        hooks.push(hook_one_boundary_run(base + rva, rva as u32, prologue));
    }
    Guard { hooks }
}

/// Boundary-aware hook that forwards the original r9 (arg 3) to on_capture_run.
unsafe fn hook_one_boundary_run(target: u64, tag: u32, prologue: usize) -> One {
    let cap = prologue.clamp(5, 64);
    let orig: &[u8] = &*ptr::slice_from_raw_parts(target as *const u8, cap);
    let block = alloc_near(target, 0x300);
    let block_base = block as u64;
    let tramp = block_base;
    let stub = block_base + 0x100;

    let mut t: Vec<u8> = Vec::new();
    t.extend_from_slice(&orig[..cap]);
    t.extend_from_slice(&rel_jmp(tramp + t.len() as u64, target + cap as u64));
    ptr::copy_nonoverlapping(t.as_ptr(), tramp as *mut u8, t.len());

    let mut s: Vec<u8> = Vec::new();
    s.extend_from_slice(&[0x48, 0x83, 0xEC, 0x50]); // sub rsp,0x50
    s.extend_from_slice(&[0x48, 0x89, 0x4C, 0x24, 0x20]);
    s.extend_from_slice(&[0x48, 0x89, 0x54, 0x24, 0x28]);
    s.extend_from_slice(&[0x4C, 0x89, 0x44, 0x24, 0x30]);
    s.extend_from_slice(&[0x4C, 0x89, 0x4C, 0x24, 0x38]); // [rsp+0x38]=r9
    s.extend_from_slice(&[0x48, 0xB9]); // rcx = tag
    s.extend_from_slice(&(tag as u64).to_le_bytes());
    s.extend_from_slice(&[0x48, 0x8B, 0x54, 0x24, 0x38]); // rdx = orig r9
    s.extend_from_slice(&[0x48, 0xB8]); // rax = on_capture_run
    s.extend_from_slice(&(on_capture_run as *const () as usize as u64).to_le_bytes());
    s.extend_from_slice(&[0xFF, 0xD0]); // call rax
    s.extend_from_slice(&[0x48, 0x8B, 0x4C, 0x24, 0x20]);
    s.extend_from_slice(&[0x48, 0x8B, 0x54, 0x24, 0x28]);
    s.extend_from_slice(&[0x4C, 0x8B, 0x44, 0x24, 0x30]);
    s.extend_from_slice(&[0x4C, 0x8B, 0x4C, 0x24, 0x38]);
    s.extend_from_slice(&[0x48, 0x83, 0xC4, 0x50]);
    s.extend_from_slice(&rel_jmp(stub + s.len() as u64, tramp));
    ptr::copy_nonoverlapping(s.as_ptr(), stub as *mut u8, s.len());

    let patch = rel_jmp(target, stub);
    let restore = orig[..5].to_vec();
    let mut old = PAGE_PROTECTION_FLAGS(0);
    let page = (target & !0xFFF) as *const c_void;
    VirtualProtect(page, 0x1000, PAGE_EXECUTE_READWRITE, &mut old).ok();
    ptr::copy_nonoverlapping(patch.as_ptr(), target as *mut u8, patch.len());
    let _ = VirtualProtect(page, 0x1000, old, &mut old);

    One { target, block, restore }
}

// ---- RIP sampler (poor-man's profiler) ----
// A helper thread suspends the (main, shaping) thread and reads its RIP,
// bucketed to 16 bytes within gdi32full, so Mongolian-vs-latin shape heat can
// be diffed to locate the inlined OT engine loop.

pub static RIP_HIST: Mutex<Vec<u32>> = Mutex::new(Vec::new()); // bucket rva>>4

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Threading::{
    GetCurrentThreadId, OpenThread, ResumeThread, SuspendThread, THREAD_GET_CONTEXT,
    THREAD_SUSPEND_RESUME,
};

pub struct RipGuard {
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Drop for RipGuard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn sampler_loop(tid: u32, modname: String, stop: Arc<AtomicBool>) {
    unsafe {
        // raw GetThreadContext (windows-rs hides CONTEXT behind cfg)
        let gtc = export_addr("kernel32.dll", "GetThreadContext")
            .map(|a| std::mem::transmute::<u64, unsafe extern "system" fn(*mut c_void, *mut u8) -> i32>(a));
        let gtc = match gtc {
            Some(f) => f,
            None => return,
        };
        let h = match OpenThread(
            THREAD_SUSPEND_RESUME | THREAD_GET_CONTEXT,
            false,
            tid,
        ) {
            Ok(h) => h,
            Err(_) => return,
        };
        let mut gdi_lo = module_base(&modname).unwrap_or(0);
        let mut n = 0u32;
        #[repr(align(16))]
        struct Align16([u8; 0x4D0]);
        let mut ctx = Align16([0u8; 0x4D0]);
        let mut reported = false;
        while !stop.load(Ordering::SeqCst) && n < 200_000 {
            n += 1;
            // module may be lazily loaded mid-shape; re-resolve until found
            if gdi_lo == 0 {
                gdi_lo = module_base(&modname).unwrap_or(0);
                if gdi_lo == 0 {
                    std::thread::yield_now();
                    continue;
                }
            }
            let s = SuspendThread(h);
            if s == u32::MAX {
                break;
            }
            // x64 CONTEXT: ContextFlags @ 0x30 (CONTEXT_AMD64|CONTEXT_CONTROL =
            // 0x100001); Rip @ 0xF8. Buffer 0x4D0 bytes.
            ctx.0[0x30..0x34].copy_from_slice(&0x100001u32.to_le_bytes());
            let ok = gtc(h.0, ctx.0.as_mut_ptr());
            if ok != 0 {
                let rip = u64::from_le_bytes(ctx.0[0xF8..0x100].try_into().unwrap());
                if rip >= gdi_lo && rip < gdi_lo + 0x200000 {
                    let bucket = ((rip - gdi_lo) >> 4) as u32;
                    if let Ok(mut v) = RIP_HIST.lock() {
                        v.push(bucket);
                    }
                }
            } else if !reported {
                reported = true;
                eprintln!(
                    "[rip] GetThreadContext failed err={}",
                    windows::Win32::Foundation::GetLastError().0
                );
            }
            ResumeThread(h);
            // let the shaping thread make progress (avoid starvation)
            std::thread::yield_now();
        }
        let _ = CloseHandle(h);
    }
}

/// Start sampling the current thread's RIP while it shapes. The sampled module
/// is chosen by env PYUSP_RIP_MODULE (default "gdi32full.dll"); the sampled
/// window is the module base .. +0x200000.
pub unsafe fn rip_start() -> RipGuard {
    if let Ok(mut v) = RIP_HIST.lock() {
        v.clear();
    }
    let tid = GetCurrentThreadId();
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let (name, size) = std::env::var("PYUSP_RIP_MODULE")
        .map(|m| (m, 0x200000u64))
        .unwrap_or_else(|_| ("gdi32full.dll".to_string(), 0x200000u64));
    let gdi_lo = module_base(&name).unwrap_or(0);
    eprintln!("[rip] module={name} base={gdi_lo:#x}");
    let handle = std::thread::spawn(move || sampler_loop(tid, name, stop2));
    RipGuard {
        stop,
        handle: Some(handle),
    }
}

/// Print the histogram (bucket rva>>4 -> count), most-hit first.
pub fn rip_report() {
    let hist = std::mem::take(&mut *RIP_HIST.lock().unwrap());
    let mut agg: std::collections::BTreeMap<u32, u32> = std::collections::BTreeMap::new();
    for b in hist {
        *agg.entry(b).or_insert(0) += 1;
    }
    eprintln!("[rip] samples={}", agg.values().sum::<u32>());
    let mut v: Vec<(u32, u32)> = agg.into_iter().collect();
    v.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    for (b, c) in v {
        eprintln!("[rip] rva16 {:#x} x{c}", (b as u64) << 4);
    }
}
