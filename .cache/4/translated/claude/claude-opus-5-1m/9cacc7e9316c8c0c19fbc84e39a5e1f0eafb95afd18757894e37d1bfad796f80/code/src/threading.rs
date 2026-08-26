//! Translation of common/threading.c (ZSTD_MULTITHREAD is NOT defined)
#![allow(non_upper_case_globals)]

/// create fake symbol to avoid empty translation unit warning
#[unsafe(no_mangle)]
pub static mut g_ZSTD_threading_useless_symbol: core::ffi::c_int = 0;
