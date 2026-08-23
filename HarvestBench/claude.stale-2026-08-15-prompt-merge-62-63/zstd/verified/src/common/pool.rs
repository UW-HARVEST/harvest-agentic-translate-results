//! Translation of common/pool.c — single-threaded (no ZSTD_MULTITHREAD) variant.
#![allow(dead_code)]
use super::allocations::ZSTD_customMem;
use core::ffi::c_void;

#[repr(C)]
pub struct POOL_ctx_s {
    pub dummy: i32,
}
pub type POOL_ctx = POOL_ctx_s;
pub type POOL_function = extern "C" fn(*mut c_void);

// static POOL_ctx g_poolCtx;
static mut G_POOL_CTX: POOL_ctx = POOL_ctx { dummy: 0 };

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_create(numThreads: usize, queueSize: usize) -> *mut POOL_ctx {
    POOL_create_advanced(numThreads, queueSize, super::allocations::ZSTD_defaultCMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_create_advanced(
    _numThreads: usize,
    _queueSize: usize,
    _customMem: ZSTD_customMem,
) -> *mut POOL_ctx {
    core::ptr::addr_of_mut!(G_POOL_CTX)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_free(_ctx: *mut POOL_ctx) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_joinJobs(_ctx: *mut POOL_ctx) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_resize(_ctx: *mut POOL_ctx, _numThreads: usize) -> i32 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_add(_ctx: *mut POOL_ctx, function: POOL_function, opaque: *mut c_void) {
    function(opaque);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_tryAdd(
    _ctx: *mut POOL_ctx,
    function: POOL_function,
    opaque: *mut c_void,
) -> i32 {
    function(opaque);
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_sizeof(ctx: *const POOL_ctx) -> usize {
    if ctx.is_null() {
        return 0;
    }
    core::mem::size_of::<POOL_ctx>()
}
