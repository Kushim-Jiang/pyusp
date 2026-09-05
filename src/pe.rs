// pe.rs — in-memory PE helpers for self-hooking the loaded usp10.dll.
//
// pe_lookup_rva_for_sig(base, sig): scan the executable sections of a loaded
// PE module for a byte signature and return its RVA. Combined with a pinned
// "expected RVA" this auto-relocates across usp10 builds (the DWriteCore
// playbook, now applied to usp10).

#![allow(clippy::missing_safety_doc)]

use std::ptr;

fn rd_u16(p: *const u8, off: usize) -> u16 {
    unsafe { ptr::read_unaligned(p.add(off) as *const u16) }
}
fn rd_u32(p: *const u8, off: usize) -> u32 {
    unsafe { ptr::read_unaligned(p.add(off) as *const u32) }
}

const IMAGE_SCN_MEM_EXECUTE: u32 = 0x2000_0000;

/// Verify `sig` at `base + rva`; if it matches return Ok(rva), else scan the
/// PE executable sections for the signature and return the relocated RVA.
/// `expected` may be 0 to force a scan.
pub unsafe fn verify_or_relocate(base: u64, expected: u64, sig: &[u8]) -> Result<u64, String> {
    if expected != 0 && bytes_match(base.wrapping_add(expected) as *const u8, sig) {
        return Ok(expected);
    }
    pe_lookup_rva_for_sig(base, sig)
        .ok_or_else(|| format!("signature {sig:02X?} not found in module @{base:#x}"))
}

fn bytes_match(p: *const u8, sig: &[u8]) -> bool {
    for (i, &b) in sig.iter().enumerate() {
        if unsafe { *p.add(i) } != b {
            return false;
        }
    }
    true
}

/// Scan the PE executable sections of the module at `base` for `sig`.
pub unsafe fn pe_lookup_rva_for_sig(base: u64, sig: &[u8]) -> Option<u64> {
    find_all_sig_rvas(base, sig).into_iter().next()
}

/// Every executable-section match for `sig` (RE/triage helper).
pub unsafe fn find_all_sig_rvas(base: u64, sig: &[u8]) -> Vec<u64> {
    let mut out = Vec::new();
    if sig.is_empty() || sig.len() > 0x40 {
        return out;
    }
    let dos = base as *const u8;
    if unsafe { *dos } != b'M' || unsafe { *dos.add(1) } != b'Z' {
        eprintln!("[pe] not MZ at base {base:#x}");
        return out;
    }
    let e_lfanew = rd_u32(dos, 0x3C) as u64;
    let pe = base + e_lfanew;
    if unsafe { *((pe) as *const u8) } != b'P' {
        eprintln!("[pe] no PE sig at {pe:#x}");
        return out;
    }
    let nsec = rd_u16(pe as *const u8, 6) as usize;
    let opt_size = rd_u16(pe as *const u8, 20) as usize;
    let sections = pe + 24 + opt_size as u64;
    let dbg = std::env::var("PYUSP_DEBUG_PE").is_ok();
    if dbg {
        eprintln!("[pe] base={base:#x} nsec={nsec} opt_size={opt_size}");
    }
    for i in 0..nsec {
        let s = sections + (i as u64) * 40;
        let va = rd_u32(s as *const u8, 12) as u64;
        let vsize = rd_u32(s as *const u8, 8) as u64;
        let chars = rd_u32(s as *const u8, 36);
        if dbg {
            eprintln!("[pe] sec {i}: va={va:#x} vsize={vsize:#x} chars={chars:#x}");
        }
        if chars & IMAGE_SCN_MEM_EXECUTE == 0 {
            continue;
        }
        // Loaded images are mapped at base + VirtualAddress with VirtualSize.
        let start = base + va;
        let size = vsize.min(0x100000); // sanity bound
        let end = start + size;
        if end < start {
            continue;
        }
        // scan for sig
        let mut p = start;
        while p + sig.len() as u64 <= end {
            if bytes_match(p as *const u8, sig) {
                out.push(va + (p - start));
            }
            p += 1;
        }
    }
    out
}
