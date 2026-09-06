// src/ffi.rs — Win32/Uniscribe FFI types for NON-Windows builds.
//
// The `windows` crate does not expose any `windows::Win32` items when
// compiling for a non-Windows target, so on Linux/macOS we define the handful
// of types the engine touches. Layouts mirror the `windows` crate (0.58)
// definitions 1:1 — and therefore the Uniscribe C headers (usp10.h) — so the
// same #[repr(C)] structs can be passed to libwineusp.so/.dylib.
//
// Only reachable with `#[cfg(not(windows))]`.

use std::ffi::c_void;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GOFFSET {
    pub du: i32,
    pub dv: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OPENTYPE_FEATURE_RECORD {
    pub tagFeature: u32,
    pub lParameter: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SCRIPT_CONTROL {
    pub _bitfield: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SCRIPT_STATE {
    pub _bitfield: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SCRIPT_ANALYSIS {
    pub _bitfield: u16,
    pub s: SCRIPT_STATE,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SCRIPT_VISATTR {
    pub _bitfield: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SCRIPT_CHARPROP {
    pub _bitfield: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SCRIPT_GLYPHPROP {
    pub sva: SCRIPT_VISATTR,
    pub reserved: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SCRIPT_ITEM {
    pub iCharPos: i32,
    pub a: SCRIPT_ANALYSIS,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TEXTRANGE_PROPERTIES {
    pub potfRecords: *mut OPENTYPE_FEATURE_RECORD,
    pub cotfRecords: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ABC {
    pub abcA: i32,
    pub abcB: u32,
    pub abcC: i32,
}

/* Opaque handle types (mirror the windows-crate newtypes: pub .0 pointer so
 * `.0.is_null()` / `HDC(ptr::null_mut())` compile unchanged). */
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HDC(pub *mut c_void);
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HFONT(pub *mut c_void);
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HGDIOBJ(pub *mut c_void);
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HMODULE(pub *mut c_void);

impl Default for HDC {
    fn default() -> Self {
        Self(std::ptr::null_mut())
    }
}
impl Default for HFONT {
    fn default() -> Self {
        Self(std::ptr::null_mut())
    }
}
impl Default for HGDIOBJ {
    fn default() -> Self {
        Self(std::ptr::null_mut())
    }
}
impl Default for HMODULE {
    fn default() -> Self {
        Self(std::ptr::null_mut())
    }
}
