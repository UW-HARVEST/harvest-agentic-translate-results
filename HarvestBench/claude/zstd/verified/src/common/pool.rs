//! Translation of `common/pool.c` (the `ZSTD_MULTITHREAD` *not* defined branch)
#![allow(dead_code)]

use crate::common::zstd_internal::ZSTD_customMem;
use core::ffi::{c_int, c_void};

/// `struct POOL_ctx_s { int dummy; };`
#[repr(C)]
pub struct POOL_ctx {
    pub dummy: c_int,
}

pub type POOL_function = Option<unsafe extern "C" fn(*mut c_void)>;

/// `static POOL_ctx g_poolCtx;`
static mut g_poolCtx: POOL_ctx = POOL_ctx { dummy: 0 };

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_create(numThreads: usize, queueSize: usize) -> *mut POOL_ctx {
    POOL_create_advanced(numThreads, queueSize, crate::common::zstd_internal::ZSTD_defaultCMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_create_advanced(
    _numThreads: usize,
    _queueSize: usize,
    _customMem: ZSTD_customMem,
) -> *mut POOL_ctx {
    core::ptr::addr_of_mut!(g_poolCtx)
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
pub unsafe extern "C" fn POOL_add(_ctx: *mut POOL_ctx, function: POOL_function, opaque: *mut c_void) {
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
