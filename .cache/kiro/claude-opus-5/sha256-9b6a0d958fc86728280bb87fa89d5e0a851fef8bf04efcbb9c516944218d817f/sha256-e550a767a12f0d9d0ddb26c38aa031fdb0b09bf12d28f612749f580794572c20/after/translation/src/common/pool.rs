//! Translation of `common/pool.c` (non-ZSTD_MULTITHREAD build).
#![allow(dead_code)]

use super::mem::size_t;
use super::zstd_internal::ZSTD_customMem;
use core::ffi::{c_int, c_void};

/// `struct POOL_ctx_s { int dummy; };`
#[repr(C)]
pub struct POOL_ctx {
    pub dummy: c_int,
}

static mut g_poolCtx: POOL_ctx = POOL_ctx { dummy: 0 };

pub type POOL_function = unsafe extern "C" fn(*mut c_void);

#[unsafe(no_mangle)]
pub extern "C" fn POOL_create(numThreads: size_t, queueSize: size_t) -> *mut POOL_ctx {
    POOL_create_advanced(numThreads, queueSize, ZSTD_customMem::default())
}

#[unsafe(no_mangle)]
pub extern "C" fn POOL_create_advanced(
    _numThreads: size_t,
    _queueSize: size_t,
    _customMem: ZSTD_customMem,
) -> *mut POOL_ctx {
    unsafe { &raw mut g_poolCtx }
}

#[unsafe(no_mangle)]
pub extern "C" fn POOL_free(_ctx: *mut POOL_ctx) {}

#[unsafe(no_mangle)]
pub extern "C" fn POOL_joinJobs(_ctx: *mut POOL_ctx) {}

#[unsafe(no_mangle)]
pub extern "C" fn POOL_resize(_ctx: *mut POOL_ctx, _numThreads: size_t) -> c_int {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_add(
    _ctx: *mut POOL_ctx,
    function: POOL_function,
    opaque: *mut c_void,
) {
    function(opaque);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_tryAdd(
    _ctx: *mut POOL_ctx,
    function: POOL_function,
    opaque: *mut c_void,
) -> c_int {
    function(opaque);
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn POOL_sizeof(ctx: *const POOL_ctx) -> size_t {
    if ctx.is_null() {
        return 0;
    }
    core::mem::size_of::<POOL_ctx>()
}
