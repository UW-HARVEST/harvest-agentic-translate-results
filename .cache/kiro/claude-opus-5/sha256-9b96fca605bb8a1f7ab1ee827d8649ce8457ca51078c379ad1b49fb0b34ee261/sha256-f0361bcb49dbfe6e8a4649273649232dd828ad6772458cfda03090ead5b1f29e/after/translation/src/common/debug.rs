//! Translation of `common/debug.c`.
use core::ffi::c_int;

/// `int g_debuglevel = DEBUGLEVEL;` (DEBUGLEVEL defaults to 0)
#[unsafe(no_mangle)]
pub static mut g_debuglevel: c_int = 0;
