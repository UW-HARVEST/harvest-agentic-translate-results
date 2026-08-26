//! Translation of common/debug.c (only hosts the global g_debuglevel)
#![allow(non_upper_case_globals)]

/// DEBUGLEVEL defaults to 0
#[unsafe(no_mangle)]
pub static mut g_debuglevel: core::ffi::c_int = 0;
