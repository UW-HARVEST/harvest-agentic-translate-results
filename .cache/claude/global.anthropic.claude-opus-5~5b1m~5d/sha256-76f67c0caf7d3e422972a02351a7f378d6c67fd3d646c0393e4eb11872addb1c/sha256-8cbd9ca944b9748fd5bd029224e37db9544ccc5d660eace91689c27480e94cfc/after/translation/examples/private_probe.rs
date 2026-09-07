//! A test-only cdylib that re-exports the `static` helpers of the translation
//! under `probe_*` names.
//!
//! `c_src` is built at `-O0`, so the C shared object keeps every `static`
//! function as a LOCAL symbol in `.symtab` (see `nm` without `-D`). Those
//! addresses can be reached at runtime by adding the `nm` offset to the
//! library's load base, which lets `tests/private.rs` differentially test
//! `cp_paeth`, `cp_make32`, `cp_chunk`, `cp_find` and `cp_unfilter` — the five
//! functions that no exported entry point reaches.
//!
//! This target exists ONLY for that purpose; the shipped `.so`
//! (`libconvert_pix_lib.so`) is unaffected and still exports exactly the nine
//! symbols the C `.so` exports.

#![allow(non_camel_case_types)]
#![allow(unsafe_op_in_unsafe_fn)]

#[path = "../src/lib.rs"]
mod cp;

use std::ffi::{c_char, c_int};

#[repr(C)]
pub struct RawPng {
    pub p: *const u8,
    pub end: *const u8,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn probe_cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    cp::cp_paeth(a, b, c)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn probe_cp_make32(s: *const u8) -> u32 {
    cp::cp_make32(s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn probe_cp_chunk(
    png: *mut RawPng,
    chunk: *const c_char,
    minlen: u32,
) -> *const u8 {
    cp::cp_chunk(png as *mut cp::cp_raw_png_t, chunk, minlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn probe_cp_find(
    png: *mut RawPng,
    chunk: *const c_char,
    minlen: u32,
) -> *const u8 {
    cp::cp_find(png as *mut cp::cp_raw_png_t, chunk, minlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn probe_cp_unfilter(
    w: c_int,
    h: c_int,
    bpp: c_int,
    raw: *mut u8,
) -> c_int {
    cp::cp_unfilter(w, h, bpp, raw)
}
