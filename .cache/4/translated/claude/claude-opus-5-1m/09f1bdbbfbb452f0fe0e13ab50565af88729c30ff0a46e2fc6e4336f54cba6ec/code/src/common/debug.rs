//! Translation of `common/debug.c`
//! DEBUGLEVEL is 0 in this build, so `g_debuglevel` is 0 and all logging is
//! compiled out.

/// `int g_debuglevel = DEBUGLEVEL;`
#[unsafe(no_mangle)]
pub static mut g_debuglevel: core::ffi::c_int = 0;
