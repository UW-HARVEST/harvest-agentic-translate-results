//! Translation of common/pool.c (ZSTD_MULTITHREAD is NOT defined)
#![allow(non_snake_case, dead_code, non_upper_case_globals, non_camel_case_types)]

use crate::zstd_h::*;

/// `struct POOL_ctx_s { int dummy; }`
#[repr(C)]
pub struct POOL_ctx {
    pub dummy: core::ffi::c_int,
}

pub type POOL_function = Option<unsafe extern "C" fn(*mut core::ffi::c_void)>;

static mut g_poolCtx: POOL_ctx = POOL_ctx { dummy: 0 };

#[unsafe(no_mangle)]
pub extern "C" fn POOL_create(_numThreads: usize, _queueSize: usize) -> *mut POOL_ctx {
    POOL_create_advanced(_numThreads, _queueSize, ZSTD_defaultCMem)
}

#[unsafe(no_mangle)]
pub extern "C" fn POOL_create_advanced(
    _numThreads: usize,
    _queueSize: usize,
    _customMem: ZSTD_customMem,
) -> *mut POOL_ctx {
    unsafe { core::ptr::addr_of_mut!(g_poolCtx) }
}

#[unsafe(no_mangle)]
pub extern "C" fn POOL_free(_ctx: *mut POOL_ctx) {}

#[unsafe(no_mangle)]
pub extern "C" fn POOL_joinJobs(_ctx: *mut POOL_ctx) {}

#[unsafe(no_mangle)]
pub extern "C" fn POOL_resize(_ctx: *mut POOL_ctx, _numThreads: usize) -> core::ffi::c_int {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_add(
    _ctx: *mut POOL_ctx,
    function: POOL_function,
    opaque: *mut core::ffi::c_void,
) {
    (function.unwrap())(opaque);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_tryAdd(
    _ctx: *mut POOL_ctx,
    function: POOL_function,
    opaque: *mut core::ffi::c_void,
) -> core::ffi::c_int {
    (function.unwrap())(opaque);
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn POOL_sizeof(ctx: *const POOL_ctx) -> usize {
    if ctx.is_null() {
        return 0;
    }
    core::mem::size_of::<POOL_ctx>()
}
