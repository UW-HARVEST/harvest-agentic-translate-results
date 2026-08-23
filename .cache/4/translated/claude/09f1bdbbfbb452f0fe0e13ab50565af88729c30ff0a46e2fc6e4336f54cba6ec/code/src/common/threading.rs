//! Translation of `common/threading.c`
//! ZSTD_MULTITHREAD is not defined in this build, so this translation unit only
//! provides the "useless symbol" that keeps it non-empty.

/// `int g_ZSTD_threading_useless_symbol;`
#[unsafe(no_mangle)]
pub static mut g_ZSTD_threading_useless_symbol: core::ffi::c_int = 0;
