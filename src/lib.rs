// pyusp — Uniscribe (usp10) step-by-step OpenType shaping tracer (Rust).
//
// Milestone 1 (watershed probe): drive Uniscribe's *OpenType* shaping API
// (ScriptItemizeOpenType → ScriptShapeOpenType → ScriptPlaceOpenType) on a
// real GDI font and emit the **final** glyph run in the babelsoft
// `/api/opentype/shape` schema, so usp10's terminal state can be compared
// 1:1 against uharfbuzz / pydwshape goldens (the "is it worth going deeper"
// check).
//
// usp10.dll is loaded *dynamically* (bundled copy preferred, system
// fallback) rather than statically linked, so the same loaded module can be
// self-hooked later (per-lookup trace) and so the wheel can ship an
// app-local usp10.dll.

#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::c_void;
use std::mem;
use std::ptr;

use serde_json::{json, Value};

// Windows build: the `windows` crate provides the Win32/Uniscribe types and
// the loading functions. Non-Windows: the crate has no `Win32` module, so we
// use src/ffi.rs mirrors (identical layouts) + a dlopen-based loader.
#[cfg(windows)]
pub use windows::core::{PCSTR, PCWSTR};
#[cfg(windows)]
pub use windows::Win32::Foundation::HMODULE;
#[cfg(windows)]
pub use windows::Win32::Globalization::{
    GOFFSET, OPENTYPE_FEATURE_RECORD, SCRIPT_ANALYSIS, SCRIPT_CHARPROP, SCRIPT_CONTROL,
    SCRIPT_GLYPHPROP, SCRIPT_ITEM, SCRIPT_STATE, SCRIPT_VISATTR, TEXTRANGE_PROPERTIES,
};
#[cfg(windows)]
pub use windows::Win32::Graphics::Gdi::{
    ABC, DEFAULT_CHARSET, DEFAULT_QUALITY, FR_PRIVATE, HDC, HFONT, HGDIOBJ, LOGFONTW,
};
#[cfg(windows)]
pub use windows::Win32::Graphics::Gdi::{
    AddFontResourceExW, CreateCompatibleDC, CreateFontIndirectW, DeleteDC, DeleteObject,
    RemoveFontResourceExW, SelectObject,
};
#[cfg(windows)]
pub use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

#[cfg(not(windows))]
mod ffi;
#[cfg(not(windows))]
pub use ffi::{
    ABC, GOFFSET, HDC, HFONT, HGDIOBJ, HMODULE, OPENTYPE_FEATURE_RECORD, SCRIPT_ANALYSIS,
    SCRIPT_CHARPROP, SCRIPT_CONTROL, SCRIPT_GLYPHPROP, SCRIPT_ITEM, SCRIPT_STATE,
    SCRIPT_VISATTR, TEXTRANGE_PROPERTIES,
};

// In-process inline-hook machinery is a Windows-only RE probe (native
// TextShaping per-glyph trace). Not present on Linux/macOS.
#[cfg(windows)]
pub mod pe;
#[cfg(windows)]
pub mod selfhook;
#[cfg(windows)]
pub mod worker_hook;

// ---------------------------------------------------------------------------
// loader — dynamic module loading + symbol resolution (Windows LoadLibraryW /
// GetProcAddress, POSIX dlopen / dlsym).
// ---------------------------------------------------------------------------
#[cfg(windows)]
mod loader {
    use super::*;
    pub type Lib = HMODULE;

    pub unsafe fn open(path: Option<&str>) -> Result<Lib, String> {
        let dll = path.unwrap_or("usp10.dll");
        let w: Vec<u16> = dll.encode_utf16().chain(std::iter::once(0)).collect();
        let m = LoadLibraryW(PCWSTR(w.as_ptr()))
            .map_err(|e| format!("LoadLibraryW({dll}): {e}"))?;
        if m.0.is_null() {
            return Err(format!("LoadLibraryW({dll}) returned null handle"));
        }
        Ok(m)
    }

    pub unsafe fn symbol(lib: Lib, name: &[u8]) -> Option<*const c_void> {
        GetProcAddress(lib, PCSTR(name.as_ptr())).map(|p| p as *const c_void)
    }

    pub fn base(lib: Lib) -> u64 {
        lib.0 as u64
    }

    pub unsafe fn module_handle_wide_forced(wname: &[u16]) -> u64 {
        // RE probes: force a DLL into memory (returns 0 if it is not present —
        // e.g. TextShaping on older builds).
        LoadLibraryW(PCWSTR(wname.as_ptr()))
            .map(|h| h.0 as u64)
            .unwrap_or(0)
    }

    pub unsafe fn module_handle_a(name: &[u8]) -> u64 {
        windows::Win32::System::LibraryLoader::GetModuleHandleA(PCSTR(
            name.as_ptr(),
        ))
        .map(|h| h.0 as u64)
        .unwrap_or(0)
    }
}

#[cfg(not(windows))]
mod loader {
    use std::ffi::c_void;
    pub type Lib = crate::HMODULE;

    extern "C" {
        fn dlopen(filename: *const u8, flag: i32) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const u8) -> *mut c_void;
    }

    /// Default POSIX module name (the natively-built Wine Uniscribe port).
    pub fn default_module_name() -> String {
        std::env::var("PYUSP_WINEUSP")
            .unwrap_or_else(|_| {
                if cfg!(target_os = "macos") {
                    "libwineusp.dylib".to_owned()
                } else {
                    "libwineusp.so".to_owned()
                }
            })
    }

    pub unsafe fn open(path: Option<&str>) -> Result<Lib, String> {
        let dll = path.map(|s| s.to_owned()).unwrap_or_else(default_module_name);
        let c = std::ffi::CString::new(dll.as_str())
            .map_err(|e| format!("bad module path: {e}"))?;
        // RTLD_NOW = 2 on glibc and macOS.
        let h = dlopen(c.as_ptr() as *const u8, 2);
        if h.is_null() {
            return Err(format!("dlopen({dll}) failed (libwineusp not built?)"));
        }
        Ok(crate::HMODULE(h))
    }

    pub unsafe fn symbol(lib: Lib, name: &[u8]) -> Option<*const c_void> {
        // name must be NUL-terminated (callers pass b"name\0").
        let p = dlsym(lib.0, name.as_ptr());
        if p.is_null() {
            None
        } else {
            Some(p)
        }
    }

    pub fn base(lib: Lib) -> u64 {
        lib.0 as u64
    }

    // RE probe helpers are Windows-only; no-ops on POSIX (never armed there).
    pub unsafe fn module_handle_wide_forced(_wname: &[u16]) -> u64 {
        0
    }
    pub unsafe fn module_handle_a(_name: &[u8]) -> u64 {
        0
    }
}

// Non-Windows no-op stand-in for worker_hook (the Windows in-process inline
// hook machinery). The RE-probe call sites in shape_item/shape_value compile
// against these signatures unchanged; on Linux/macOS the probe env vars are
// never set, and even if they were the stubs return empty/no-ops.
#[cfg(not(windows))]
mod worker_hook {
    pub struct Guard;
    pub struct RipGuard;

    pub fn arm_rvas(_base: u64, _rvas: &[(u64, usize)]) -> Guard {
        Guard
    }
    pub fn arm_rvas_ret(_base: u64, _rvas: &[(u64, usize)]) -> Guard {
        Guard
    }
    pub fn arm_rvas_run(_base: u64, _rvas: &[(u64, usize)]) -> Guard {
        Guard
    }
    pub fn arm_once() {}
    pub fn set_glyph_buf(_ptr: *mut u16, _cap: usize) {}
    pub fn drain_run_steps() -> Vec<(u32, Vec<u16>)> {
        Vec::new()
    }
    pub fn drain_glyph_steps() -> Vec<(u64, Vec<u16>)> {
        Vec::new()
    }
    pub fn drain_rets() -> Vec<(u64, u64)> {
        Vec::new()
    }
    pub fn drain() -> Vec<(u32, u64)> {
        Vec::new()
    }
    pub fn rip_start() -> RipGuard {
        RipGuard
    }
    pub fn rip_report() {}
}

// ---------------------------------------------------------------------------
// SCRIPT_ANALYSIS / SCRIPT_STATE bit masks (usp10.h, 10.0.22621):
//   SCRIPT_ANALYSIS : u16 { eScript:10 fRTL:1 fLayoutRTL:1 fLinkBefore:1
//                           fLinkAfter:1 fLogicalOrder:1 fNoGlyphIndex:1 }
//   SCRIPT_STATE    : u16 { uBidiLevel:5 ... fArabicNumContext:1(b11) ... }
// ---------------------------------------------------------------------------
const A_ESCRIPT_MASK: u16 = 0x03FF;
const A_FRTL: u16 = 0x0400; // bit 10
const A_FLOGICAL_ORDER: u16 = 0x4000; // bit 14
const S_FARABICNUMCTX: u16 = 0x0800; // bit 11
const S_UBIDILEVEL_MASK: u16 = 0x001F;

const TAG_DFLT: u32 = 0x746C_6664; // MS_MAKE_TAG('d','f','l','t') — fourcc LSB-first
const E_OUTOFMEMORY: i32 = -2147024882; // 0x8007000E
const E_INVALIDARG: i32 = -2147024809; // 0x80070057

#[inline]
fn ana_script(a: &SCRIPT_ANALYSIS) -> u16 {
    a._bitfield & A_ESCRIPT_MASK
}
#[inline]
fn ana_rtl(a: &SCRIPT_ANALYSIS) -> bool {
    a._bitfield & A_FRTL != 0
}

type HRES = i32;

type FnItemizeOT = unsafe extern "system" fn(
    *const u16,
    i32,
    i32,
    *const SCRIPT_CONTROL,
    *const SCRIPT_STATE,
    *mut SCRIPT_ITEM,
    *mut u32,
    *mut i32,
) -> HRES;
type FnShapeOT = unsafe extern "system" fn(
    HDC,
    *mut *mut c_void,
    *mut SCRIPT_ANALYSIS,
    u32,
    u32,
    *const i32,
    *const *const TEXTRANGE_PROPERTIES,
    i32,
    *const u16,
    i32,
    i32,
    *mut u16,
    *mut SCRIPT_CHARPROP,
    *mut u16,
    *mut SCRIPT_GLYPHPROP,
    *mut i32,
) -> HRES;
type FnPlaceOT = unsafe extern "system" fn(
    HDC,
    *mut *mut c_void,
    *mut SCRIPT_ANALYSIS,
    u32,
    u32,
    *const i32,
    *const *const TEXTRANGE_PROPERTIES,
    i32,
    *const u16,
    *const u16,
    *const SCRIPT_CHARPROP,
    i32,
    *const u16,
    *const SCRIPT_GLYPHPROP,
    i32,
    *mut i32,
    *mut GOFFSET,
    *mut ABC,
) -> HRES;
type FnFreeCache = unsafe extern "system" fn(*mut *mut c_void) -> HRES;
type FnScriptTags = unsafe extern "system" fn(
    HDC,
    *mut *mut c_void,
    *const SCRIPT_ANALYSIS,
    i32,
    *mut u32,
    *mut i32,
) -> HRES;
// Optional per-lookup trace sink exports from the Wine-based wineusp.dll
// port (usp_trace_*). Absent on system usp10.dll / gdi32full forwarders.
type FnTraceBegin = unsafe extern "system" fn() -> i32;
type FnTraceCount = unsafe extern "system" fn() -> i32;
type FnTraceStage = unsafe extern "system" fn(i32, *mut u8, i32, *mut i32, *mut *const u16) -> i32;
type FnTraceStop = unsafe extern "system" fn();
// Cross-platform font-bytes provider (wineusp.dll port only): register raw
// font bytes so Script*OpenType can shape with a NULL hdc (no GDI font / DC).
type FnSetFontBytes = unsafe extern "system" fn(*const c_void, usize) -> i32;
type FnClearFontBytes = unsafe extern "system" fn();

/// Dynamically-loaded handle to usp10.dll.
pub struct Usp10 {
    _module: HMODULE,
    pub itemize_ot: FnItemizeOT,
    pub shape_ot: FnShapeOT,
    pub place_ot: FnPlaceOT,
    pub free_cache: FnFreeCache,
    pub script_tags: FnScriptTags,
    /// Optional wineusp.dll per-lookup trace sink (None for system usp10).
    pub trace_begin: Option<FnTraceBegin>,
    pub trace_count: Option<FnTraceCount>,
    pub trace_stage: Option<FnTraceStage>,
    pub trace_stop: Option<FnTraceStop>,
    /// Optional wineusp.dll raw font-bytes provider (None for system usp10).
    pub set_font_bytes: Option<FnSetFontBytes>,
    pub clear_font_bytes: Option<FnClearFontBytes>,
}

fn hres_msg(hr: HRES) -> &'static str {
    match hr {
        0 => "S_OK",
        E_OUTOFMEMORY => "E_OUTOFMEMORY",
        E_INVALIDARG => "E_INVALIDARG",
        _ => "HRESULT error",
    }
}

fn hr_fmt(hr: HRES) -> String {
    format!("{} (0x{:08X})", hres_msg(hr), hr as u32)
}

impl Usp10 {
    pub fn module_base(&self) -> u64 {
        self._module.0 as u64
    }

    /// Load usp10/wineusp: `prefer` = explicit module path, else the platform
    /// default (Windows: system usp10.dll; POSIX: libwineusp.so/.dylib).
    pub unsafe fn load(prefer: Option<&str>) -> Result<Usp10, String> {
        let module = loader::open(prefer)?;
        macro_rules! sym {
            ($n:literal, $t:ty) => {{
                let name = concat!($n, "\0");
                let p = loader::symbol(module, name.as_bytes())
                    .ok_or_else(|| format!("{} missing in module", $n))?;
                mem::transmute::<*const c_void, $t>(p)
            }};
        }
        macro_rules! opt_sym {
            ($n:literal, $t:ty) => {
                loader::symbol(module, concat!($n, "\0").as_bytes())
                    .map(|p| mem::transmute::<*const c_void, $t>(p))
            };
        }
        Ok(Usp10 {
            _module: module,
            itemize_ot: sym!("ScriptItemizeOpenType", FnItemizeOT),
            shape_ot: sym!("ScriptShapeOpenType", FnShapeOT),
            place_ot: sym!("ScriptPlaceOpenType", FnPlaceOT),
            free_cache: sym!("ScriptFreeCache", FnFreeCache),
            script_tags: sym!("ScriptGetFontScriptTags", FnScriptTags),
            trace_begin: opt_sym!("usp_trace_begin", FnTraceBegin),
            trace_count: opt_sym!("usp_trace_count", FnTraceCount),
            trace_stage: opt_sym!("usp_trace_stage", FnTraceStage),
            trace_stop: opt_sym!("usp_trace_stop", FnTraceStop),
            set_font_bytes: opt_sym!("usp_set_font_bytes", FnSetFontBytes),
            clear_font_bytes: opt_sym!("usp_clear_font_bytes", FnClearFontBytes),
        })
    }
}

fn hex_to_bytes(s: &str) -> Result<Vec<u8>, String> {
    let clean: String = s.chars().filter(|c| !c.is_whitespace() && *c != ',').collect();
    if clean.len() % 2 != 0 {
        return Err("odd hex length".into());
    }
    (0..clean.len() / 2)
        .map(|i| {
            u8::from_str_radix(&clean[i * 2..i * 2 + 2], 16)
                .map_err(|e| format!("bad hex at {i}: {e}"))
        })
        .collect()
}

/// RE helper: scan a loaded module (default usp10.dll) for all
/// executable-section matches of a hex signature; returns RVAs.
/// e.g. --scan-module gdi32full.dll --find-sig "41 57 41 56 41 55"
#[cfg(windows)]
pub fn scan_module_for_sig(module_path: Option<&str>, hexsig: &str) -> Result<Vec<u64>, String> {
    let dll: String = module_path.map(|s| s.to_owned()).unwrap_or_else(|| "usp10.dll".into());
    let w: Vec<u16> = dll.encode_utf16().chain(std::iter::once(0)).collect();
    let module: HMODULE = unsafe { LoadLibraryW(PCWSTR(w.as_ptr())) }
        .map_err(|e| format!("LoadLibraryW({dll}): {e}"))?;
    let sig = hex_to_bytes(hexsig)?;
    let rvas = unsafe { pe::find_all_sig_rvas(module.0 as u64, &sig) };
    Ok(rvas)
}

// ---------------------------------------------------------------------------
// sfnt helpers
// ---------------------------------------------------------------------------
fn be16(d: &[u8], o: usize) -> u16 {
    ((d[o] as u16) << 8) | d[o + 1] as u16
}
fn be32(d: &[u8], o: usize) -> u32 {
    ((d[o] as u32) << 24) | ((d[o + 1] as u32) << 16) | ((d[o + 2] as u32) << 8) | d[o + 3] as u32
}

/// (upem, glyph_count) from a (possibly TTC) sfnt blob.
fn sfnt_meta(data: &[u8]) -> (u32, u32) {
    if data.len() < 12 {
        return (2048, 0);
    }
    let tag = &data[0..4];
    let num = if tag == b"ttcf" {
        if data.len() < 16 {
            return (2048, 0);
        }
        let of = be32(data, 8) as usize;
        if of + 12 > data.len() {
            return (2048, 0);
        }
        be16(data, of + 4) as usize
    } else if tag == b"OTTO" || tag == b"\x00\x01\x00\x00" || tag == b"true" || tag == b"typ1" {
        0
    } else {
        return (2048, 0);
    };
    let ntables = be16(data, num + 4) as usize;
    let mut head_off: Option<usize> = None;
    let mut maxp_off: Option<usize> = None;
    for i in 0..ntables {
        let rec = num + 12 + i * 16;
        if rec + 16 > data.len() {
            break;
        }
        match &data[rec..rec + 4] {
            b"head" => head_off = Some(be32(data, rec + 8) as usize),
            b"maxp" => maxp_off = Some(be32(data, rec + 8) as usize),
            _ => {}
        }
    }
    let upem = head_off
        .filter(|&o| o + 20 <= data.len())
        .map(|o| be16(data, o + 18))
        .unwrap_or(2048) as u32;
    let nglyphs = maxp_off
        .filter(|&o| o + 6 <= data.len())
        .map(|o| be16(data, o + 4))
        .unwrap_or(0) as u32;
    (upem, nglyphs)
}

/// Extract the Windows family name (nameID 1, preferred; 16 fallback) from an
/// sfnt blob — the face name to select via CreateFontIndirectW.
fn sfnt_family(data: &[u8]) -> Option<String> {
    if data.len() < 12 {
        return None;
    }
    let tag = &data[0..4];
    let num = if tag == b"ttcf" {
        if data.len() < 16 {
            return None;
        }
        let of = be32(data, 8) as usize;
        if of + 12 > data.len() {
            return None;
        }
        be16(data, of + 4) as usize
    } else if tag == b"OTTO" || tag == b"\x00\x01\x00\x00" || tag == b"true" {
        0
    } else {
        return None;
    };
    let ntables = be16(data, num + 4) as usize;
    let mut name_off: Option<(usize, usize)> = None; // (offset, length)
    for i in 0..ntables {
        let rec = num + 12 + i * 16;
        if rec + 16 > data.len() {
            break;
        }
        if &data[rec..rec + 4] == b"name" {
            name_off = Some((be32(data, rec + 8) as usize, be32(data, rec + 12) as usize));
            break;
        }
    }
    let (noff, nlen) = name_off?;
    if noff + 6 > data.len() {
        return None;
    }
    let count = be16(data, noff + 2) as usize;
    let str_off = be16(data, noff + 4) as usize; // header stringOffset
    let strings = noff + str_off;
    // Prefer Windows platform (3) nameID 1 (GDI family); fall back to 16.
    let mut best: Option<(u16, String)> = None; // (nameID, string)
    for i in 0..count {
        let rec = noff + 6 + i * 12;
        if rec + 12 > data.len() || rec + 12 > noff + nlen {
            break;
        }
        let platform = be16(data, rec);
        let encoding = be16(data, rec + 2);
        let name_id = be16(data, rec + 6);
        let len = be16(data, rec + 8) as usize;
        let off = be16(data, rec + 10) as usize;
        // Windows platform 3 (enc 1 = BMP UTF-16BE, enc 10 = full UTF-16BE)
        if platform != 3 || (encoding != 1 && encoding != 10) {
            continue;
        }
        if name_id != 1 && name_id != 16 {
            continue;
        }
        let so = strings + off;
        if so + len > data.len() {
            continue;
        }
        let mut s = String::new();
        let mut k = so;
        while k + 1 < so + len {
            let u = ((data[k] as u32) << 8) | data[k + 1] as u32;
            s.push(char::from_u32(u).unwrap_or('\u{FFFD}'));
            k += 2;
        }
        if s.is_empty() {
            continue;
        }
        let take = match best {
            None => true,
            Some((cur_id, _)) => (name_id == 1 && cur_id != 1) || (name_id == 16 && cur_id == 16 && false),
        };
        if take {
            best = Some((name_id, s));
        }
    }
    best.map(|(_, s)| s)
}

// ---------------------------------------------------------------------------
// Options / result
// ---------------------------------------------------------------------------
pub struct ShapeOpts {
    pub font: String,
    pub text: String,
    pub script: String,
    pub language: String,
    pub direction: String,
    pub show_all: bool,
    pub usp10_path: Option<String>,
    pub features_arg: String,
    /// Enable the wineusp per-lookup trace (no-op when the loaded DLL has no
    /// usp_trace_* exports, e.g. system usp10.dll).
    pub trace: bool,
    /// Shape via the wineusp raw font-bytes provider: register the font file
    /// bytes and drive Script*OpenType with a NULL hdc (no GDI font/DC). This
    /// is the same code path Linux/macOS use — fully parity-checked on
    /// Windows against the GDI reference (tools/wine_usp/port/test_bytesmode).
    pub wine_bytes: bool,
    /// Shape via the **system Microsoft TextShaping engine** (Windows x64):
    /// drive system usp10 with a real GDI font and hook TextShaping.dll's OT
    /// apply driver to capture a native per-application glyph-run trace
    /// (stages named "textshaping apply N"). Windows-only; errors when the
    /// local TextShaping.dll is outside the supported (validated) set.
    pub textshaping: bool,
}

/// OpenType 4CC from a feature-tag string.
///
/// Follows HarfBuzz's `hb_tag_from_string` ("Valid tags are four characters.
/// Shorter input strings will be padded with spaces. Longer input strings will
/// be truncated") for the padding — **spaces (0x20), not NULs** — and requires
/// printable ASCII. Where HarfBuzz silently *truncates* an over-long tag we
/// fail instead: a truncated tag selects a different feature, which is exactly
/// the kind of silent divergence this tracer exists to rule out.
/// OpenType 4CC from a feature-tag string, in the **Uniscribe `OPENTYPE_TAG`
/// byte order**.
///
/// A tag is exactly four characters, so a shorter one is **right-padded with
/// spaces (0x20)** — not NULs — and must be printable ASCII. Where HarfBuzz
/// silently *truncates* an over-long tag we fail instead: a truncated tag
/// selects a different feature, and that is exactly the kind of silent
/// divergence this tracer exists to rule out.
///
/// Note the byte order: `OPENTYPE_TAG` stores the fourcc **LSB-first**
/// (`MS_MAKE_TAG` in usp10_internal.h, wine's opentype.c builds the LangSys
/// tags the same way), i.e. the DWORD's *memory* bytes spell the tag, because
/// the engine compares those bytes against the font's big-endian fourcc. So
/// `liga` is 0x6167696C here, not HarfBuzz's `hb_tag_t` 0x6C696761. Using
/// `from_be_bytes` produced a byte-reversed tag that matched nothing, which
/// silently dropped every requested feature.
fn feature_tag(tag: &str) -> Result<u32, String> {
    let b = tag.as_bytes();
    if b.is_empty() || b.len() > 4 {
        return Err(format!(
            "feature tag {tag:?} must be 1..=4 characters (got {})",
            b.len()
        ));
    }
    let mut out = [b' '; 4]; // right-padded with spaces, like hb_tag_from_string
    for (i, &c) in b.iter().enumerate() {
        if !(0x20..=0x7e).contains(&c) {
            return Err(format!(
                "feature tag {tag:?} contains a non-printable-ASCII byte {c:#04x}"
            ));
        }
        out[i] = c;
    }
    Ok(u32::from_le_bytes(out))
}

/// Parse one feature item: `[+|-] tag [=] value` (a HarfBuzz
/// `hb_feature_from_string` subset, with the same precedence).
fn parse_one_feature(part: &str) -> Result<(u32, i32), String> {
    let mut rest = part;

    // The +/- prefix only supplies the *default* value; an explicit value
    // parsed below overrides it, because HarfBuzz runs
    // parse_feature_value_prefix() first and parse_feature_value_postfix()
    // last. So `+kern=0` means *off* — the old parser short-circuited on the
    // '+', ignored the `=0` entirely and turned the feature ON instead.
    let mut value: i32 = 1;
    if let Some(r) = rest.strip_prefix('+') {
        rest = r;
    } else if let Some(r) = rest.strip_prefix('-') {
        value = 0;
        rest = r;
    }
    rest = rest.trim_start();

    // Tag: either CSS-style quoted, or up to the first whitespace / '=' / '['.
    let quote = rest.chars().next().filter(|c| *c == '\'' || *c == '"');
    let (tag, tail) = match quote {
        Some(q) => {
            let inner = &rest[1..];
            let end = inner
                .find(q)
                .ok_or_else(|| format!("unterminated quoted tag {rest:?}"))?;
            (&inner[..end], &inner[end + 1..])
        }
        None => {
            if rest.is_empty() {
                return Err("missing feature tag".into());
            }
            let end = rest
                .find(|c: char| c.is_ascii_whitespace() || c == '=' || c == '[')
                .unwrap_or(rest.len());
            (&rest[..end], &rest[end..])
        }
    };

    let mut tail = tail.trim_start();
    if tail.starts_with('[') {
        return Err(
            "range syntax `[start:end]` is not supported — features apply to the whole run"
                .into(),
        );
    }
    if let Some(t) = tail.strip_prefix('=') {
        tail = t.trim_start();
        if tail.is_empty() {
            return Err("`=` without a value".into());
        }
    }
    if !tail.is_empty() {
        value = parse_feature_value(tail.trim())?;
    }
    Ok((feature_tag(tag)?, value))
}

/// Uniscribe `lParameter`: `0` (disabled) / `1` (active, first alternative) /
/// `N` (alternative index), plus the CSS aliases `off` / `on`.
fn parse_feature_value(v: &str) -> Result<i32, String> {
    if v.eq_ignore_ascii_case("off") {
        return Ok(0);
    }
    if v.eq_ignore_ascii_case("on") {
        return Ok(1);
    }
    if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!(
            "feature value {v:?} must be a non-negative integer or on/off"
        ));
    }
    v.parse::<i32>()
        .map_err(|e| format!("feature value {v:?} out of range: {e}"))
}

/// Parse a HarfBuzz-style feature list into `OPENTYPE_FEATURE_RECORD` pairs.
///
/// ```text
///   feature := [ '+' | '-' ] tag [ '=' ] value
///   tag     := 1..=4 printable-ASCII chars, optionally quoted ('kern' "kern")
///   value   := non-negative integer | 'on' | 'off'
/// ```
///
/// with the documented Uniscribe `lParameter` semantics (0 = disabled,
/// 1 = active / first alternative, >1 = alternative index): `kern`/`+kern`
/// and `kern=1` → 1, `-kern` and `kern=0` → 0, `aalt=2` → 2.
///
/// Every malformed item is an error: silently dropping it (as this used to do
/// for `ss01=abc`) or truncating its tag changes the shape result with no
/// diagnostic. Duplicate tags keep the first position but the last value,
/// because Uniscribe takes one record per feature.
fn parse_features(s: &str) -> Result<Vec<(u32, i32)>, String> {
    let mut out: Vec<(u32, i32)> = Vec::new();
    for part in s.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (tag, value) =
            parse_one_feature(part).map_err(|e| format!("bad feature {part:?}: {e}"))?;
        match out.iter_mut().find(|(t, _)| *t == tag) {
            Some(slot) => slot.1 = value,
            None => out.push((tag, value)),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod feature_tests {
    use super::*;

    fn t(s: &str) -> u32 {
        feature_tag(s).unwrap()
    }

    /// The tag as the engine sees it: bytes in memory, in fourcc order.
    fn tag_bytes(s: &str) -> [u8; 4] {
        feature_tag(s).unwrap().to_le_bytes()
    }

    #[test]
    fn tag_is_space_padded_fourcc_lsb_first() {
        // OPENTYPE_TAG (MS_MAKE_TAG) stores the fourcc LSB-first, so the
        // DWORD's memory bytes spell the tag; the engine compares those.
        assert_eq!(tag_bytes("liga"), *b"liga");
        assert_eq!(tag_bytes("kern"), *b"kern");
        assert_eq!(tag_bytes("aBc"), *b"aBc "); // space-padded, not NUL-padded
        assert_eq!(tag_bytes("aB"), *b"aB  ");
        assert_eq!(feature_tag("liga").unwrap(), 0x6167_696C);
        assert_eq!(TAG_DFLT.to_le_bytes(), *b"dflt");
    }

    #[test]
    fn tag_is_not_byte_reversed() {
        // Regression guard: from_be_bytes made `liga` 0x6C696761, which is
        // MS_MAKE_TAG('a','g','i','l') — it matched no feature in any font and
        // silently dropped the whole feature list.
        assert_ne!(feature_tag("liga").unwrap(), 0x6C69_6761);
        assert_eq!(feature_tag("liga").unwrap().to_le_bytes(), *b"liga");
    }

    #[test]
    fn tag_length_and_charset_are_enforced() {
        assert!(feature_tag("").is_err());
        assert!(feature_tag("kernx").is_err()); // no silent truncation
        assert!(feature_tag("k\u{00e9}rn").is_err());
        assert!(feature_tag("ke\u{0007}rn").is_err());
    }

    #[test]
    fn uniscribe_parameter_values() {
        assert_eq!(parse_features("kern").unwrap(), vec![(t("kern"), 1)]);
        assert_eq!(parse_features("+kern").unwrap(), vec![(t("kern"), 1)]);
        assert_eq!(parse_features("-kern").unwrap(), vec![(t("kern"), 0)]);
        assert_eq!(parse_features("kern=0").unwrap(), vec![(t("kern"), 0)]);
        assert_eq!(parse_features("aalt=2").unwrap(), vec![(t("aalt"), 2)]);
        assert_eq!(parse_features("kern=on").unwrap(), vec![(t("kern"), 1)]);
        assert_eq!(parse_features("kern=off").unwrap(), vec![(t("kern"), 0)]);
        assert_eq!(parse_features("kern 0").unwrap(), vec![(t("kern"), 0)]);
        assert_eq!(parse_features("kern = 0").unwrap(), vec![(t("kern"), 0)]);
        assert_eq!(parse_features("'kern' 1").unwrap(), vec![(t("kern"), 1)]);
        assert_eq!(
            parse_features("\"kern\"=0").unwrap(),
            vec![(t("kern"), 0)]
        );
    }

    #[test]
    fn explicit_value_overrides_the_prefix_like_harfbuzz() {
        assert_eq!(parse_features("+kern=0").unwrap(), vec![(t("kern"), 0)]);
        assert_eq!(parse_features("-kern=1").unwrap(), vec![(t("kern"), 1)]);
        assert_eq!(parse_features("+ss01=2").unwrap(), vec![(t("ss01"), 2)]);
    }

    #[test]
    fn malformed_items_are_errors() {
        for bad in [
            "kern=abc",
            "kern=",
            "kern=-1",
            "kern=1.5",
            "kernx=1",
            "kern[3:5]",
            "aalt[3:5]=2",
            "+",
            "-",
            "=0",
            "kern==",
            "kern==1",
        ] {
            assert!(parse_features(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn duplicates_keep_last_value_and_first_position() {
        assert_eq!(
            parse_features("kern=0,kern=1").unwrap(),
            vec![(t("kern"), 1)]
        );
        assert_eq!(
            parse_features("aalt=2,kern=0,aalt=3").unwrap(),
            vec![(t("aalt"), 3), (t("kern"), 0)]
        );
    }

    #[test]
    fn empty_input_is_no_features() {
        assert!(parse_features("").unwrap().is_empty());
        assert!(parse_features("  ,  ,").unwrap().is_empty());
    }
}

// ---------------------------------------------------------------------------
// Shape driver
// ---------------------------------------------------------------------------
struct FontDc {
    hdc: HDC,
    hfont: HFONT,
    prev: HGDIOBJ,
    family: String,
}

/// Add a private font resource from `path`, then select it into a memory DC.
/// (Windows-only: POSIX builds always shape in bytes mode with a NULL hdc.)
#[cfg(windows)]
unsafe fn make_font_dc(path: &str, data: &[u8]) -> Result<FontDc, String> {
    let family = sfnt_family(data)
        .unwrap_or_else(|| {
            std::path::Path::new(path)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Arial".into())
        });
    // Debug knob: skip AddFontResourceExW (select the installed system font,
    // like the reference python shaper does) when PYUSP_NO_PRIVATE is set.
    let no_private = std::env::var("PYUSP_NO_PRIVATE").is_ok();
    let w: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let mut added = 0i32;
    if !no_private {
        added = AddFontResourceExW(PCWSTR(w.as_ptr()), FR_PRIVATE, Some(ptr::null()));
    }
    let hdc = CreateCompatibleDC(None);
    if hdc.0.is_null() {
        if added > 0 {
            let _ = RemoveFontResourceExW(PCWSTR(w.as_ptr()), FR_PRIVATE.0 as u32, None);
        }
        return Err("CreateCompatibleDC failed".into());
    }
    let mut lf = LOGFONTW::default();
    let upem = sfnt_meta(data).0;
    // Debug knob: override the em pixel height (env PYUSP_EM_PX) to test
    // whether ScriptPlace advances scale proportionally with font size.
    let em_px: i32 = std::env::var("PYUSP_EM_PX")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(upem.max(1) as i32);
    lf.lfHeight = -em_px;
    lf.lfWeight = 400;
    lf.lfCharSet = DEFAULT_CHARSET;
    lf.lfQuality = DEFAULT_QUALITY;
    let mut fn16 = [0u16; 32];
    for (i, u) in family.encode_utf16().take(31).enumerate() {
        fn16[i] = u;
    }
    lf.lfFaceName = fn16;
    let hfont = CreateFontIndirectW(&lf);
    if hfont.0.is_null() {
        let _ = DeleteDC(hdc);
        if added > 0 {
            let _ = RemoveFontResourceExW(PCWSTR(w.as_ptr()), FR_PRIVATE.0 as u32, None);
        }
        return Err(format!("CreateFontIndirectW failed for face {:?}", family));
    }
    let prev = SelectObject(hdc, hfont);
    Ok(FontDc {
        hdc,
        hfont,
        prev,
        family,
    })
}

impl FontDc {
    unsafe fn cleanup(&mut self, path: &str) {
        if self.hdc.0.is_null() {
            // bytes mode (or POSIX): no GDI font/DC was created — nothing to release.
            return;
        }
        #[cfg(windows)]
        {
            let _ = SelectObject(self.hdc, self.prev);
            let _ = DeleteObject(self.hfont);
            let _ = DeleteDC(self.hdc);
            if std::env::var("PYUSP_NO_PRIVATE").is_ok() {
                return;
            }
            let w: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
            let _ = RemoveFontResourceExW(PCWSTR(w.as_ptr()), FR_PRIVATE.0 as u32, None);
        }
        #[cfg(not(windows))]
        {
            let _ = (self.hfont, self.prev, path);
        }
    }
}

fn has_rtl(text: &[u16]) -> bool {
    text.iter().any(|&c| {
        (0x0590..=0x08FF).contains(&c)
            || (0xFB1D..=0xFDFF).contains(&c)
            || (0xFE70..=0xFEFC).contains(&c)
    })
}

struct ShapeRun<'a> {
    usp: &'a Usp10,
    hdc: HDC,
    psc: *mut *mut c_void,
    trace_on: bool,
    /// (stage-name, glyph-id run) captured from wineusp during ScriptShapeOpenType,
    /// in the engine's (logical) order — same order as `glyphs` before RTL reversal.
    trace_stages: Vec<(String, Vec<u16>)>,
    /// textshaping backend: capture the native TextShaping per-application
    /// glyph-run trace (backend="textshaping" only).
    native: bool,
    native_armed: bool,
    native_stages: Vec<(String, Vec<u16>)>,
    /// RE probe guard (PYUSP_CALLEE_PROBE) — keeps callee hooks armed.
    callee_guard: Option<worker_hook::Guard>,
}

impl<'a> ShapeRun<'a> {
    /// Query the OT script tag for an itemized analysis. The font's own
    /// script tags (ScriptGetFontScriptTags) are authoritative — e.g. a
    /// modern Devanagari face lists 'dev2' while ScriptItemizeOpenType only
    /// reports 'deva' from the code points. Fall back to the item tag.
    unsafe fn resolve_script_tag(
        &self,
        psa: &SCRIPT_ANALYSIS,
        item_tag: u32,
    ) -> Result<u32, String> {
        let mut tags = [0u32; 8];
        let mut n = 0i32;
        let hr = (self.usp.script_tags)(
            self.hdc,
            self.psc,
            psa,
            8,
            tags.as_mut_ptr(),
            &mut n,
        );
        if hr == 0 && n > 0 && tags[0] != 0 {
            return Ok(tags[0]);
        }
        if item_tag != 0 {
            return Ok(item_tag);
        }
        Ok(TAG_DFLT)
    }

    unsafe fn resolve_langsys(&self, _psa: &SCRIPT_ANALYSIS, _tag_script: u32) -> Result<u32, String> {
        // Language-tag query (ScriptGetFontLanguageTags) is unreliable against
        // installed fonts (returns USP_E_SCRIPT_NOT_IN_FONT 0x80040200 even
        // when the OT tables exist). HarfBuzz/Uniscribe both default to the
        // 'dflt' LangSys, so we use it unconditionally.
        Ok(TAG_DFLT)
    }

    /// Shape one itemized run via ScriptShapeOpenType + ScriptPlaceOpenType.
    #[allow(clippy::too_many_arguments)]
    unsafe fn shape_item(
        &mut self,
        seg: &[u16],
        item_tag: u32,
        item_lang: u32,
        psa: &mut SCRIPT_ANALYSIS,
        upem: i64,
        features: &[(u32, i32)],
    ) -> Result<Vec<Value>, String> {
        // Arm the wineusp per-lookup trace before shaping this run.
        self.trace_stages.clear();
        self.native_stages.clear();
        self.native_armed = false;
        if self.trace_on {
            if let Some(b) = self.usp.trace_begin {
                (b)();
            }
        }
        // RE probe: capture gdi32full threadpool worker entry (ntdll
        // TpSimpleTryPost callback) — or, for PYUSP_CALLEE_PROBE, count which
        // of the statically-identified callees of ScriptShapeOpenType actually
        // fire during this shape.
        let worker_probe = std::env::var_os("PYUSP_WORKER_PROBE").is_some();
        let callee_probe = std::env::var_os("PYUSP_CALLEE_PROBE").is_some();
        let glyph_probe = std::env::var_os("PYUSP_GLYPH_PROBE").is_some();
        let ret_probe = std::env::var_os("PYUSP_RET_PROBE").is_some();
        let ts_probe = std::env::var_os("PYUSP_TS_PROBE").is_some();
        let ts_ret = std::env::var_os("PYUSP_TS_RET").is_some();
        let native_run =
            std::env::var_os("PYUSP_NATIVE_RUN").is_some() || self.native;
        if ts_probe {
            // TextShaping is the real OT engine; count fires of its candidate
            // apply-driver / parse entries to find the per-lookup frequency.
            // TextShaping loads lazily during ScriptShapeOpenType, so force it.
            let wname: Vec<u16> = "TextShaping.dll\0".encode_utf16().collect();
            unsafe {
                let _ = loader::module_handle_wide_forced(&wname);
            }
            let cname = std::ffi::CString::new("TextShaping.dll").unwrap();
            let gbase = unsafe { loader::module_handle_a(cname.as_bytes_with_nul()) };
            if gbase != 0 {
                eprintln!("[ts] base={gbase:#x} arming TextShaping probes");
                let all: [(u64, usize); 7] = [
                    (0xad60, 5),  // apply driver (parses + loops glyphs)
                    (0xc1c0, 5),  // GSUB/GPOS table parser
                    (0xcc10, 5),
                    (0xcf18, 5),
                    (0x1500, 6),
                    (0x25350, 5), // GSUB cache wrapper
                    (0x25c40, 5),
                ];
                // PYUSP_TS_ONE=<hex rva>: hook only that rva (bisection).
                let one = std::env::var("PYUSP_TS_ONE")
                    .ok()
                    .and_then(|v| u64::from_str_radix(v.trim_start_matches("0x"), 16).ok());
                let rvas: Vec<(u64, usize)> = match one {
                    Some(rva) => all
                        .iter()
                        .copied()
                        .filter(|(r, _)| *r == rva)
                        .collect(),
                    None => all.to_vec(),
                };
                if !rvas.is_empty() {
                    self.callee_guard = if ts_ret {
                        Some(unsafe { worker_hook::arm_rvas_ret(gbase, &rvas) })
                    } else {
                        Some(unsafe { worker_hook::arm_rvas(gbase, &rvas) })
                    };
                }
            }
        } else if ret_probe {
            // Capture the return address of each per-glyph getter fire so the
            // per-glyph driver loop (unique concentrated caller) is located.
            let cname = std::ffi::CString::new("gdi32full.dll").unwrap();
            let gbase = unsafe { loader::module_handle_a(cname.as_bytes_with_nul()) };
            if gbase != 0 {
                self.callee_guard = Some(unsafe {
                    worker_hook::arm_rvas_ret(
                        gbase,
                        &[(0x4b720, 5), (0x4e180, 11), (0x4cfb0, 8)],
                    )
                });
            }
        } else if callee_probe || glyph_probe {
            let cname = std::ffi::CString::new("gdi32full.dll").unwrap();
            let gbase = unsafe { loader::module_handle_a(cname.as_bytes_with_nul()) };
            if gbase != 0 {
                // Dispatch-table members only (guaranteed function entries).
                // (rva, safe prologue prefix B from objdump of gdi32full 10.0.26100)
                let rvas: [(u64, usize); 11] = [
                    (0x4eb50, 8), (0x53b90, 7), (0x4b430, 8), (0x7b950, 8), (0x7a10, 5),
                    (0x29ab0, 8), (0x4b720, 5), (0x4e180, 11), (0x4bb50, 8), (0x57910, 5),
                    (0x4cfb0, 8),
                ];
                let list: Vec<(u64, usize)> = if glyph_probe {
                    // Only the per-glyph tag getters that fire ~per glyph.
                    vec![(0x4b720, 5), (0x4e180, 11), (0x4cfb0, 8)]
                } else {
                    // PYUSP_CALLEE_ONE=<hex rva>: hook only that rva (bisection).
                    let single = std::env::var("PYUSP_CALLEE_ONE")
                        .ok()
                        .and_then(|v| u64::from_str_radix(v.trim_start_matches("0x"), 16).ok());
                    match single {
                        Some(rva) => rvas
                            .iter()
                            .copied()
                            .filter(|(r, _)| *r == rva)
                            .collect(),
                        None => rvas.to_vec(),
                    }
                };
                self.callee_guard = Some(unsafe { worker_hook::arm_rvas(gbase, &list) });
            }
        } else if worker_probe {
            worker_hook::arm_once();
        } else if native_run {
            // In-process native per-application trace (PoC): hook the TextShaping
            // "apply one OT op" driver and record the r9-> glyph run at every
            // fire, so the real engine's per-glyph/per-app stages are produced
            // without frida. TextShaping loads lazily during ScriptShapeOpenType,
            // so force it into memory first.
            //
            // Instead of a hardcoded build-specific RVA, the driver is located by
            // scanning the loaded module's .text for its unique prologue signature
            // (6 pushes + `lea rbp,[rsp-0x218]; sub rsp,0x318`). This supports any
            // Windows build whose engine keeps that prologue, and degrades
            // gracefully (no native stages) when it is not found. The hooked
            // prologue is 5 bytes (push rbp; push rsi; push r12 — an exact
            // instruction boundary; the 6th byte is the REX prefix of push r13,
            // so 6 would replay a partial instruction and corrupt the stack).
            // Signature from Win11 10.0.26100 TextShaping @0x15650.
            let wname: Vec<u16> = "TextShaping.dll\0".encode_utf16().collect();
            unsafe {
                let _ = loader::module_handle_wide_forced(&wname);
            }
            let cname = std::ffi::CString::new("TextShaping.dll").unwrap();
            let gbase = unsafe { loader::module_handle_a(cname.as_bytes_with_nul()) };
            if gbase != 0 {
                // signature: push rbp; push rsi; push r12..r15;
                // lea rbp,[rsp-0x218]; sub rsp,0x318 (full 24-byte prologue).
                // Instruction boundaries within the 6 pushes are at 2,3,5,7,9,11
                // bytes (r12..r15 pushes are 41+reg = 2 bytes), so the replay
                // prefix must be a boundary — 5 bytes (three pushes) is the
                // largest clean prefix < 5+2. Using the FULL prologue (incl. the
                // validated 0x218/0x318 frame offsets) intentionally EXCLUDES
                // builds whose entry has the same 6-push shape but a different
                // internal layout (e.g. Win11 22H2 22621 matches the 13-byte
                // prefix but not the offsets) — those are not runtime-verified
                // here, so they fall back instead of being hooked on assumptions.
                const SIG: [u8; 24] = [
                    0x40, 0x55, 0x56, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41,
                    0x57, 0x48, 0x8D, 0xAC, 0x24, 0xE8, 0xFD, 0xFF, 0xFF, 0x48,
                    0x81, 0xEC, 0x18, 0x03,
                ];
                let basep = gbase as *const u8;
                unsafe fn r16(p: *const u8) -> u16 {
                    std::ptr::read_unaligned(p as *const u16)
                }
                unsafe fn r32(p: *const u8) -> u32 {
                    std::ptr::read_unaligned(p as *const u32)
                }
                let img_size = unsafe {
                    let pe = r32(basep.add(0x3C)) as usize;
                    let magic = r16(basep.add(pe + 24));
                    let _ = magic;
                    r32(basep.add(pe + 24 + 56)) as usize // SizeOfImage
                };
                let mut drv: Option<u64> = None;
                let blob =
                    unsafe { std::slice::from_raw_parts(basep, img_size) };
                let mut i = 0x1000usize;
                while i + SIG.len() <= img_size {
                    if &blob[i..i + SIG.len()] == &SIG {
                        if drv.is_none() {
                            drv = Some(i as u64);
                            i += SIG.len();
                            continue;
                        }
                        drv = None; // ambiguous: multiple matches -> unsafe
                        break;
                    }
                    i += 1;
                }
                if let Some(rva) = drv {
                    eprintln!(
                        "[nativerun] base={gbase:#x} sig-driver rva={rva:#x} (img {img_size:#x})"
                    );
                    self.callee_guard = Some(unsafe {
                        worker_hook::arm_rvas_run(gbase, &[(rva, 5)])
                    });
                    self.native_armed = true;
                } else {
                    eprintln!(
                        "[nativerun] TextShaping driver signature not found (img {img_size:#x}) — no native stages on this build"
                    );
                }
            }
        }
        // Ask for logical-order glyph arrays (like HarfBuzz) so RTL runs can
        // be compared 1:1. Preserve all flags ScriptItemize set.
        psa._bitfield |= A_FLOGICAL_ORDER;

        let cchars = seg.len() as i32;
        let mut maxg = (cchars * 3 + 8).max(16);
        let mut logclust = vec![0u16; cchars as usize];
        let mut charprops = vec![SCRIPT_CHARPROP { _bitfield: 0 }; cchars as usize];
        let mut glyphs: Vec<u16> = Vec::new();
        let mut glyphprops: Vec<SCRIPT_GLYPHPROP> = Vec::new();
        let mut cglyphs = 0i32;

        // feature ranges (single range over the whole run when any feature)
        //
        // NOTE: supplying range properties *replaces* the script's default GSUB
        // feature set for the covered characters, it does not add to it.
        // Verified against system usp10: `--features liga=1` on Calibri "fi"
        // yields the fi ligature via the explicit record, while a feature array
        // whose tags match nothing loses the default liga (296,349 = f,i). So
        // `--features` means "apply exactly these", not "also apply these".
        let range_chars = [cchars];
        let mut recs: Vec<OPENTYPE_FEATURE_RECORD> = Vec::new();
        let mut range_prop: TEXTRANGE_PROPERTIES = TEXTRANGE_PROPERTIES {
            potfRecords: ptr::null_mut(),
            cotfRecords: 0,
        };
        let mut cranges = 0i32;
        let mut rp_range: *const TEXTRANGE_PROPERTIES = ptr::null();
        if !features.is_empty() {
            for (tag, param) in features {
                recs.push(OPENTYPE_FEATURE_RECORD {
                    tagFeature: *tag,
                    lParameter: *param,
                });
            }
            range_prop.potfRecords = recs.as_mut_ptr();
            range_prop.cotfRecords = recs.len() as i32;
            rp_range = &range_prop;
            cranges = 1;
        }

        // ScriptShapeOpenType with buffer growth.
        loop {
            glyphs.clear();
            glyphs.resize(maxg as usize, 0);
            glyphprops.clear();
            glyphprops.resize(
                maxg as usize,
                SCRIPT_GLYPHPROP {
                    sva: SCRIPT_VISATTR { _bitfield: 0 },
                    reserved: 0,
                },
            );
            let mut rp_arr: [*const TEXTRANGE_PROPERTIES; 1] = [rp_range];
            if glyph_probe {
                // stash the caller out-buffer so each per-glyph hook fire can
                // snapshot whether gdi32full mutates it in place
                worker_hook::set_glyph_buf(glyphs.as_mut_ptr(), glyphs.len());
            }
            let hr = (self.usp.shape_ot)(
                self.hdc,
                self.psc,
                psa,
                item_tag,
                item_lang,
                if cranges > 0 { range_chars.as_ptr() } else { ptr::null() },
                if cranges > 0 { rp_arr.as_mut_ptr() } else { ptr::null() },
                cranges,
                seg.as_ptr(),
                cchars,
                maxg,
                logclust.as_mut_ptr(),
                charprops.as_mut_ptr(),
                glyphs.as_mut_ptr(),
                glyphprops.as_mut_ptr(),
                &mut cglyphs,
            );
            if hr == 0 {
                break;
            }
            if hr == E_OUTOFMEMORY {
                // E_OUTOFMEMORY → grow
                if maxg > 1 << 16 {
                    return Err("ScriptShapeOpenType: glyph buffer overflow".into());
                }
                maxg *= 2;
                // a retry re-shapes from scratch: reset the trace log
                if self.trace_on {
                    if let Some(b) = self.usp.trace_begin {
                        (b)();
                    }
                }
                continue;
            }
            return Err(format!(
                "ScriptShapeOpenType tag={:#010x}: {}",
                item_tag,
                hr_fmt(hr)
            ));
        }
        let n = cglyphs as usize;

        // In-process native per-application trace report (PYUSP_NATIVE_RUN):
        // dump the glyph-run state at every TextShaping 0x15650 application and
        // validate the final captured run equals the shaped glyph output.
        if native_run {
            // backend="textshaping" is a *supported-version* contract: if the
            // driver was never armed (signature not found / TextShaping not
            // loaded) fail loudly instead of silently returning no trace.
            if self.native && !self.native_armed {
                return Err(
                    "TextShaping native trace: this build of TextShaping.dll is not in \
                     the supported set (validated: Win11 24H2 / 10.0.26100 x64 driver \
                     layout). Use backend='wineusp' (cross-platform per-lookup) or \
                     'usp10' (single-stage)."
                        .into(),
                );
            }
            let steps = worker_hook::drain_run_steps();
            eprintln!("[nativerun] apps={}", steps.len());
            let fin: Vec<u16> = glyphs[..n].to_vec();
            let mut last: Option<Vec<u16>> = None;
            for (i, (rva, gids)) in steps.iter().enumerate() {
                let changed = last.as_ref().map(|l| l != gids).unwrap_or(true);
                if changed {
                    eprintln!(
                        "[nativerun] app#{} rva={rva:#x} run={gids:?}",
                        i + 1
                    );
                }
                last = Some(gids.clone());
            }
            eprintln!(
                "[nativerun] last_run={:?} shaped_final={fin:?} match={}",
                steps.last().map(|s| &s.1),
                steps
                    .last()
                    .map(|s| s.1 == fin)
                    .unwrap_or(false)
            );
            // Structured native per-application trace for backend="textshaping".
            // Cross-check: the last captured apply run must equal the engine's
            // final shaped run. A drift (capture stops before the engine's last
            // apply step) means this TextShaping build drives that step through
            // a different path, so the captured timeline is not trustworthy here
            // — refuse rather than return misleading stages.
            if self.native {
                let ok = steps
                    .last()
                    .map(|s| s.1 == fin)
                    .unwrap_or(false);
                if !ok {
                    return Err(
                        "TextShaping native trace cross-check failed: no OT \
                         application was captured, or the captured final run \
                         differs from the engine's shaped output (this \
                         TextShaping.dll build is outside the validated set: \
                         Win11 24H2 / 10.0.26100 x64). Use backend='wineusp' or \
                         'usp10'."
                            .into(),
                    );
                }
                for (k, (_, gids)) in steps.iter().enumerate() {
                    self.native_stages
                        .push((format!("apply {}", k + 1), gids.clone()));
                }
            }
        }

        // Glyph in-place probe report: dedupe consecutive buffer states across
        // per-glyph getter fires (0x4b720/0x4e180/0x4cfb0).
        if glyph_probe {
            let steps = worker_hook::drain_glyph_steps();
            eprintln!("[glyph] fires={} cap={}", steps.len(), glyphs.len());
            let mut last: Option<Vec<u16>> = None;
            let mut distinct = 0;
            for (i, (tag, snap)) in steps.iter().enumerate() {
                let changed = last.as_ref().map(|l| l != snap).unwrap_or(true);
                if changed {
                    distinct += 1;
                    // trim trailing zero padding (caller buffer was zeroed)
                    let nz = snap
                        .iter()
                        .rposition(|&x| x != 0)
                        .map(|p| p + 1)
                        .unwrap_or(0);
                    let show = &snap[..nz.min(snap.len())];
                    eprintln!("[glyph] fire#{i} rva={tag:#x} buf={show:?}");
                }
                last = Some(snap.clone());
            }
            eprintln!("[glyph] distinct_states={distinct}");
        }

        // Return-address probe report: aggregate caller continuation RVAs.
        if ret_probe || (ts_probe && ts_ret) {
            let rets = worker_hook::drain_rets();
            use std::ffi::CString;
            let cname = CString::new("gdi32full.dll").unwrap();
            let gbase = unsafe { loader::module_handle_a(cname.as_bytes_with_nul()) };
            eprintln!("[ret] fires={} base={gbase:#x}", rets.len());
            let mut agg: std::collections::BTreeMap<(u64, u64), u32> =
                std::collections::BTreeMap::new();
            for (rva, ret) in &rets {
                let caller = if gbase != 0 && *ret >= gbase { ret - gbase } else { *ret };
                *agg.entry((*rva, caller)).or_insert(0) += 1;
            }
            // resolve the module owning each caller address (which module is
            // driving the OT engine?) — short lines (<70 chars) to dodge the
            // console-width truncation of the *> redirect.
            let mut modbase: u64 = 0;
            for ((rva, caller), c) in agg {
                let (mbase, mname) = {
                    #[cfg(windows)]
                    {
                        unsafe {
                            let mut h = windows::Win32::Foundation::HMODULE(std::ptr::null_mut());
                            let ok = windows::Win32::System::LibraryLoader::GetModuleHandleExW(
                                windows::Win32::System::LibraryLoader::GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
                                windows::core::PCWSTR(caller as *const u16),
                                &mut h,
                            )
                            .is_ok();
                            if ok && !h.is_invalid() {
                                let mut buf = [0u16; 260];
                                let n = windows::Win32::System::LibraryLoader::GetModuleFileNameW(
                                    h,
                                    &mut buf,
                                );
                                let name = String::from_utf16_lossy(&buf[..n as usize]);
                                let short = name
                                    .rsplit('\\')
                                    .next()
                                    .unwrap_or(&name)
                                    .to_string();
                                (h.0 as u64, short)
                            } else {
                                (0, String::new())
                            }
                        }
                    }
                    #[cfg(not(windows))]
                    {
                        (0u64, String::new())
                    }
                };
                if mbase != 0 {
                    modbase = mbase;
                }
                let caller_rva = if mbase != 0 && caller >= mbase {
                    caller - mbase
                } else {
                    caller
                };
                eprintln!("[ret] h={rva:#x} c={caller_rva:#x} x{c} m={mname}");
            }
            if modbase != 0 {
                eprintln!("[ret] modbase={modbase:#x}");
            }
        }

        // RE probe report: which gdi32full functions were posted to a thread /
        // threadpool during ScriptShapeOpenType (worker/engine entries).
        if worker_probe {
            let evs = worker_hook::drain();
            use std::ffi::CString;
            let name = CString::new("gdi32full.dll").unwrap();
            let base = unsafe { loader::module_handle_a(name.as_bytes_with_nul()) };
            eprintln!("[worker] events={} gdi32full_base={base:#x}", evs.len());
            for (tag, addr) in evs {
                let t = match tag {
                    1 => "post",
                    2 => "alloc",
                    3 => "thread",
                    _ => "?",
                };
                if base != 0 && addr >= base && addr < base + 0x200000 {
                    eprintln!("[worker] {t} gdi32full RVA {:#x}", addr - base);
                }
            }
        }
        if callee_probe || ts_probe {
            let evs = worker_hook::drain();
            let mut agg: std::collections::BTreeMap<u64, u32> = std::collections::BTreeMap::new();
            for (tag, _v) in evs {
                *agg.entry(tag as u64).or_insert(0) += 1;
            }
            eprintln!("[callee] fires (ts_probe={ts_probe}):");
            for (rva, c) in agg {
                eprintln!("[callee]   {rva:#x} x{c}");
            }
        }

        // Drain the wineusp per-lookup trace (cmap → per-lookup → final).
        if self.trace_on {
            if let (Some(cnt), Some(stage), Some(stop)) =
                (self.usp.trace_count, self.usp.trace_stage, self.usp.trace_stop)
            {
                let total = (cnt)().max(0) as i32;
                for i in 0..total {
                    let mut name = [0u8; 64];
                    let mut gc: i32 = 0;
                    let mut gp: *const u16 = ptr::null();
                    let rc = (stage)(i, name.as_mut_ptr(), 64, &mut gc, &mut gp);
                    if rc == 0 && gc > 0 && !gp.is_null() {
                        let gids =
                            std::slice::from_raw_parts(gp, gc as usize).to_vec();
                        let nlen = name
                            .iter()
                            .position(|&b| b == 0)
                            .unwrap_or(name.len());
                        let nm =
                            String::from_utf8_lossy(&name[..nlen]).into_owned();
                        self.trace_stages.push((nm, gids));
                    }
                }
                (stop)();
            }
        }

        // placements
        let mut advances = vec![0i32; n.max(1)];
        let mut goff = vec![
            GOFFSET { du: 0, dv: 0 };
            n.max(1)
        ];
        let mut rp_arr: [*const TEXTRANGE_PROPERTIES; 1] = [rp_range];
        let hr = (self.usp.place_ot)(
            self.hdc,
            self.psc,
            psa,
            item_tag,
            item_lang,
            if cranges > 0 { range_chars.as_ptr() } else { ptr::null() },
            if cranges > 0 { rp_arr.as_mut_ptr() } else { ptr::null() },
            cranges,
            seg.as_ptr(),
            logclust.as_ptr(),
            charprops.as_ptr(),
            cchars,
            glyphs.as_ptr(),
            glyphprops.as_ptr(),
            n as i32,
            advances.as_mut_ptr(),
            goff.as_mut_ptr(),
            ptr::null_mut(),
        );
        if hr != 0 {
            return Err(format!("ScriptPlaceOpenType: {}", hr_fmt(hr)));
        }

        // cluster: char index → first glyph index (logical order)
        let mut glyph_to_char: Vec<usize> = vec![usize::MAX; n];
        for (ci, &gi) in logclust.iter().enumerate() {
            let gi = gi as usize;
            if gi < n && glyph_to_char[gi] == usize::MAX {
                glyph_to_char[gi] = ci;
            }
        }
        // lfHeight = -upem ⇒ 1 device px ≈ 1 font design unit, so advances are
        // already in font units (same space HarfBuzz reports).
        let _ = upem;
        let mut out = Vec::with_capacity(n);
        for gi in 0..n {
            let cl = if glyph_to_char[gi] != usize::MAX {
                glyph_to_char[gi]
            } else {
                (cchars.max(1) - 1) as usize
            };
            out.push(json!({
                "g": glyphs[gi],
                "cl": cl,
                "dx": goff[gi].du,
                "dy": goff[gi].dv,
                "ax": advances[gi],
                "ay": 0,
                "flags": 0,
            }));
        }
        Ok(out)
    }
}

/// Shape `opts.text` with Uniscribe on `opts.font`; return the babelsoft
/// `/api/opentype/shape` engine dict as JSON.
pub fn shape_json(opts: ShapeOpts) -> Result<String, String> {
    let v = shape_value(opts)?;
    serde_json::to_string_pretty(&v).map_err(|e| e.to_string())
}

pub fn shape_value(opts: ShapeOpts) -> Result<Value, String> {
    // Probe hook: let an attached tracer attach before we shape
    // (PYUSP_PRESLEEP_MS=<millis>). Debug/RE only.
    if let Ok(ms) = std::env::var("PYUSP_PRESLEEP_MS") {
        if let Ok(n) = ms.parse::<u64>() {
            std::thread::sleep(std::time::Duration::from_millis(n));
        }
    }
    // Force-load the real engine DLL early so an external tracer (frida) can
    // attach to TextShaping before the (fast) shape runs. RE only.
    if std::env::var_os("PYUSP_PRELOAD_TS").is_some() {
        let wname: Vec<u16> = "TextShaping.dll\0".encode_utf16().collect();
        unsafe {
            let _ = loader::module_handle_wide_forced(&wname);
        }
        // After forcing the engine DLL into memory, hold BEFORE the shape runs
        // so an external tracer (frida) can deterministically attach
        // (PYUSP_POSTLOAD_MS=<millis>). RE only.
        if let Ok(ms) = std::env::var("PYUSP_POSTLOAD_MS") {
            if let Ok(n) = ms.parse::<u64>() {
                std::thread::sleep(std::time::Duration::from_millis(n));
            }
        }
    }
    let data = std::fs::read(&opts.font).map_err(|e| format!("read font {}: {e}", opts.font))?;
    let (upem, nglyphs) = sfnt_meta(&data);

    // backend="textshaping" drives the Windows TextShaping engine in-process;
    // that machinery is Windows/x64-only (worker_hook). Refuse clearly elsewhere.
    if opts.textshaping && cfg!(not(windows)) {
        return Err(
            "backend='textshaping' is Windows-only (it drives the system TextShaping \
             engine via usp10); on this platform use backend='wineusp' for the \
             cross-platform per-lookup trace."
                .into(),
        );
    }

    let usp = unsafe { Usp10::load(opts.usp10_path.as_deref()) }?;

    // Feature records for the whole call, parsed up front so malformed input
    // fails before any GDI/shaping work happens.
    let features = parse_features(&opts.features_arg)?;

    // The Wine-based engine cannot honour feature records at all: its
    // ScriptShapeOpenType/ScriptPlaceOpenType ports log
    // FIXME("Ranges not supported yet") and drop rpRangeProperties entirely
    // (tools/wine_usp/port/src/usp10.c), and the only lParameter consumer there
    // (shape.c SHAPE_ApplyOpenTypeFeatures) is reached only with the hard-coded
    // per-script default feature list. Quietly ignoring the caller's features
    // would make every cross-engine comparison of feature behaviour meaningless
    // (the same request DID apply them on system usp10), so refuse instead —
    // the Wine port identifies itself via the usp_set_font_bytes export.
    if !features.is_empty() && usp.set_font_bytes.is_some() {
        return Err(format!(
            "features \"{}\" cannot be applied by the loaded Wine Uniscribe \
             port: its ScriptShapeOpenType/ScriptPlaceOpenType log \"Ranges not \
             supported yet\" and drop the OPENTYPE_FEATURE_RECORD array, so the \
             result would silently ignore them. Use backend='usp10' (system \
             Uniscribe) or backend='textshaping' for feature selection.",
            opts.features_arg
        ));
    }

    // text → utf16
    let text16: Vec<u16> = opts.text.encode_utf16().collect();
    if text16.is_empty() {
        return Err("empty text".into());
    }
    let base_level: u16 = match opts.direction.as_str() {
        "rtl" => 1,
        "ltr" => 0,
        _ => {
            if has_rtl(&text16) {
                1
            } else {
                0
            }
        }
    };

    // Wine-bytes mode: when the loaded module (wineusp) exposes the raw
    // font-bytes provider, register the font and drive shaping with a NULL
    // hdc — the same path Linux/macOS use (no GDI font/DC needed). On
    // non-Windows this is the ONLY path (there is no GDI/AddFontResource).
    #[cfg(windows)]
    let wine_bytes = if opts.textshaping {
        // textshaping needs the real GDI font + system usp10 path (the native
        // engine lives in TextShaping.dll, reached through usp10 with a DC).
        false
    } else {
        opts.wine_bytes && usp.set_font_bytes.is_some()
    };
    #[cfg(not(windows))]
    let wine_bytes = if usp.set_font_bytes.is_some() {
        true
    } else {
        return Err("this libwineusp has no usp_set_font_bytes export — rebuild the port with fontbytes".into());
    };
    let mut fd = if wine_bytes {
        let rc = unsafe { (usp.set_font_bytes.unwrap())(data.as_ptr() as *const c_void, data.len()) };
        if rc != 0 {
            return Err(format!("usp_set_font_bytes failed (rc={rc})"));
        }
        let family = sfnt_family(&data)
            .unwrap_or_else(|| {
                std::path::Path::new(&opts.font)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "pyusp".into())
            });
        FontDc {
            hdc: HDC(ptr::null_mut()),
            hfont: HFONT(ptr::null_mut()),
            prev: HGDIOBJ(ptr::null_mut()),
            family,
        }
    } else {
        #[cfg(windows)]
        {
            unsafe { make_font_dc(&opts.font, &data) }?
        }
        #[cfg(not(windows))]
        {
            unreachable!("non-Windows always uses the wine-bytes path")
        }
    };
    let mut psc: *mut c_void = ptr::null_mut();
    let mut run = ShapeRun {
        usp: &usp,
        hdc: fd.hdc,
        psc: &mut psc,
        trace_on: opts.trace,
        trace_stages: Vec::new(),
        native: opts.textshaping && opts.trace,
        native_armed: false,
        native_stages: Vec::new(),
        callee_guard: None,
    };

    let rip_probe = std::env::var_os("PYUSP_RIP_PROBE").is_some();
    let _ripg = if rip_probe {
        Some(unsafe { worker_hook::rip_start() })
    } else {
        None
    };
    let result = (|| -> Result<Value, String> {
        // itemize (OpenType: gives per-item script tags)
        let cchars = text16.len() as i32;
        let cmax = (cchars * 2 + 8).max(16) as usize;
        let mut items = vec![
            SCRIPT_ITEM {
                iCharPos: 0,
                a: SCRIPT_ANALYSIS {
                    _bitfield: 0,
                    s: SCRIPT_STATE { _bitfield: 0 },
                }
            };
            cmax + 1
        ];
        let mut script_tags = vec![0u32; cmax + 1];
        let mut nitems = 0i32;
        let ctrl = SCRIPT_CONTROL { _bitfield: 0 };
        let mut state = SCRIPT_STATE { _bitfield: 0 };
        state._bitfield = (state._bitfield & !S_UBIDILEVEL_MASK) | (base_level & S_UBIDILEVEL_MASK);
        if base_level == 1 {
            state._bitfield |= S_FARABICNUMCTX;
        }
        let hr = unsafe {
            (usp.itemize_ot)(
                text16.as_ptr(),
                cchars,
                cmax as i32,
                &ctrl,
                &state,
                items.as_mut_ptr(),
                script_tags.as_mut_ptr(),
                &mut nitems,
            )
        };
        if hr != 0 {
            return Err(format!("ScriptItemizeOpenType: {}", hr_fmt(hr)));
        }
        let nitems = nitems.max(0) as usize;

        let mut stages: Vec<Value> = Vec::new();
        let mut final_glyphs: Vec<Value> = Vec::new();
        let mut messages: Vec<String> = Vec::new();

        for it in 0..nitems {
            let item = items[it];
            let start = item.iCharPos.max(0) as usize;
            let end = if it + 1 < items.len() {
                (items[it + 1].iCharPos).max(0) as usize
            } else {
                text16.len()
            };
            let end = end.min(text16.len()).max(start);
            if end <= start {
                continue;
            }
            let seg = &text16[start..end];
            let mut psa = item.a;
            let rtl = ana_rtl(&psa);
            let script_code = ana_script(&psa);
            let tag_script = unsafe {
                run.resolve_script_tag(&psa, script_tags[it]).unwrap_or(0)
            };
            if tag_script == 0 {
                messages.push(format!(
                    "item {it}: script {script_code} no OT script tag — skipped"
                ));
                continue;
            }
            let tag_lang = unsafe { run.resolve_langsys(&psa, tag_script) }?;
            let glyphs = unsafe {
                run.shape_item(seg, tag_script, tag_lang, &mut psa, upem as i64, &features)?
            };
            let raw_trace = if run.native {
                std::mem::take(&mut run.native_stages)
            } else {
                std::mem::take(&mut run.trace_stages)
            };
            let n = glyphs.len();
            messages.push(format!(
                "Uniscribe item {it}: script {script_code} tag {tag_script:#010x} chars {start}..{end} -> {n} glyphs{}",
                if rtl { " (RTL, visual order out)" } else { "" }
            ));
            let logical_final = glyphs; // logical order, cl relative to seg
            let mut with_cl = logical_final.clone();
            // HarfBuzz/uharfbuzz store RTL glyph runs in *visual* order
            // (left→right); we shape in logical order (fLogicalOrder), so
            // reverse RTL runs at the boundary. Clusters stay logical char
            // indices either way (same as HarfBuzz).
            if rtl {
                with_cl.reverse();
            }
            for g in with_cl.iter_mut() {
                if let Some(cl) = g.get_mut("cl") {
                    if let Some(c) = cl.as_i64() {
                        *cl = json!(start as i64 + c);
                    }
                }
            }
            if raw_trace.is_empty() || !run.trace_on {
                // No per-lookup trace available (system usp10 or trace off):
                // a single whole-run stage.
                stages.push(json!({
                    "m": format!("Uniscribe shape item {it} (script {script_code}, tag {tag_script:#010x})"),
                    "glyphs": with_cl,
                    "depth": 0,
                    "effective": true,
                }));
            } else {
                // Genuine per-application trace: wineusp per-lookup (named) or
                // TextShaping native apply runs. Each captured run is in
                // logical order; mirror the final run's transform (reverse
                // RTL, then add the run start to every cluster).
                let stage_prefix = if run.native {
                    "textshaping"
                } else {
                    "wineusp lookup"
                };
                let nfinal = logical_final.len();
                let seg_len = seg.len().max(1);
                messages.push(if run.native {
                    format!(
                        "textshaping native trace: {} stages (per OT application)",
                        raw_trace.len()
                    )
                } else {
                    format!(
                        "wineusp per-lookup trace: {} stages (cmap + GSUB lookups + final)",
                        raw_trace.len()
                    )
                });
                for (st_name, gids) in &raw_trace {
                    let m = gids.len();
                    let mut objs: Vec<Value> = if m == nfinal {
                        // 1:1 with the final run: reuse the logical-order
                        // final cluster for each glyph.
                        gids.iter()
                            .enumerate()
                            .map(|(i, &g)| {
                                let cl = logical_final
                                    .get(i)
                                    .and_then(|o| o.get("cl"))
                                    .and_then(|c| c.as_i64())
                                    .unwrap_or(0);
                                json!({
                                    "g": g,
                                    "cl": cl,
                                    "dx": 0,
                                    "dy": 0,
                                    "ax": 0,
                                    "ay": 0,
                                    "flags": 0,
                                })
                            })
                            .collect()
                    } else {
                        // Count changed (ligature/expansion): monotonic approx
                        // in logical order, relative cluster.
                        gids.iter()
                            .enumerate()
                            .map(|(i, &g)| {
                                let cl = ((i * seg_len) / m).min(seg_len - 1) as i64;
                                json!({
                                    "g": g,
                                    "cl": cl,
                                    "dx": 0,
                                    "dy": 0,
                                    "ax": 0,
                                    "ay": 0,
                                    "flags": 0,
                                })
                            })
                            .collect()
                    };
                    if rtl {
                        objs.reverse();
                    }
                    for o in objs.iter_mut() {
                        if let Some(cl) = o.get_mut("cl") {
                            if let Some(c) = cl.as_i64() {
                                *cl = json!(start as i64 + c);
                            }
                        }
                    }
                    stages.push(json!({
                        "m": format!("{stage_prefix} {st_name} (item {it})"),
                        "glyphs": objs,
                        "depth": 0,
                        "effective": true,
                    }));
                }
            }
            final_glyphs.extend(with_cl);
        }
        if final_glyphs.is_empty() {
            return Err("no glyphs produced".into());
        }
        Ok(json!({
            "upem": upem,
            "glyph_count": nglyphs,
            "engine": "uniscribe",
            "stages": stages,
            "final": final_glyphs,
            "messages": messages,
            "font_info": {
                "family": fd.family,
                "path": opts.font,
            }
        }))
    })();

    if rip_probe {
        drop(_ripg);
        worker_hook::rip_report();
    }

    unsafe {
        let _ = (usp.free_cache)(&mut psc);
        if wine_bytes {
            if let Some(clr) = usp.clear_font_bytes {
                clr();
            }
        } else {
            fd.cleanup(&opts.font);
        }
    }

    result
}
