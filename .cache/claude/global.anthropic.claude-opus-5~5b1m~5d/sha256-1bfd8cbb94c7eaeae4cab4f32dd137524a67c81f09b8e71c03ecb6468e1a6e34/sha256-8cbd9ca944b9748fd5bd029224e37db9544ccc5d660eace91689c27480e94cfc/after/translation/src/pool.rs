//! Translation of `common/pool.c` (the `ZSTD_MULTITHREAD` *not* defined variant,
//! which is what this build uses) and `common/threading.c`.
#![allow(dead_code)]

use core::ffi::{c_int, c_void};

use crate::zstd_h::ZSTD_customMem;

/* threading.c : create fake symbol to avoid empty translation unit warning */
#[unsafe(no_mangle)]
pub static mut g_ZSTD_threading_useless_symbol: c_int = 0;

/* debug.c : global debug level */
#[unsafe(no_mangle)]
pub static mut g_debuglevel: c_int = 0;

#[repr(C)]
pub struct POOL_ctx {
    pub dummy: c_int,
}

pub type POOL_function = Option<unsafe extern "C" fn(*mut c_void)>;

static mut g_poolCtx: POOL_ctx = POOL_ctx { dummy: 0 };

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_create(_numThreads: usize, _queueSize: usize) -> *mut POOL_ctx {
    &raw mut g_poolCtx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_create_advanced(
    _numThreads: usize,
    _queueSize: usize,
    _customMem: ZSTD_customMem,
) -> *mut POOL_ctx {
    &raw mut g_poolCtx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_free(_ctx: *mut POOL_ctx) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_joinJobs(_ctx: *mut POOL_ctx) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_resize(_ctx: *mut POOL_ctx, _numThreads: usize) -> c_int {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_add(
    _ctx: *mut POOL_ctx,
    function: POOL_function,
    opaque: *mut c_void,
) {
    (function.unwrap())(opaque);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_tryAdd(
    _ctx: *mut POOL_ctx,
    function: POOL_function,
    opaque: *mut c_void,
) -> c_int {
    (function.unwrap())(opaque);
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_sizeof(ctx: *const POOL_ctx) -> usize {
    if ctx.is_null() {
        return 0;
    }
    core::mem::size_of::<POOL_ctx>()
}
