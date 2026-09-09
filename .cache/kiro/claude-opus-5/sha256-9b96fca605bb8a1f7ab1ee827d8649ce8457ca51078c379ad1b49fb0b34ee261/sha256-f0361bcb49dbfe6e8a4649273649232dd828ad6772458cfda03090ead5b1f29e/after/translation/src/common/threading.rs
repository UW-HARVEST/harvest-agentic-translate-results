//! Translation of `common/threading.c` (non-ZSTD_MULTITHREAD build).
use core::ffi::c_int;

/// fake symbol to avoid empty translation unit warning
#[unsafe(no_mangle)]
pub static mut g_ZSTD_threading_useless_symbol: c_int = 0;
