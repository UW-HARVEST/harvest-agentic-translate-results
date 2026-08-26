pub type size_t = usize;
pub type ZSTD_allocFunction =
    Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, size_t) -> *mut ::core::ffi::c_void>;
pub type ZSTD_freeFunction =
    Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> ()>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_customMem {
    pub customAlloc: ZSTD_allocFunction,
    pub customFree: ZSTD_freeFunction,
    pub opaque: *mut ::core::ffi::c_void,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct POOL_ctx_s {
    pub dummy: ::core::ffi::c_int,
}
pub type POOL_ctx = POOL_ctx_s;
pub type POOL_function = Option<unsafe extern "C" fn(*mut ::core::ffi::c_void) -> ()>;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
static mut ZSTD_defaultCMem: ZSTD_customMem = ZSTD_customMem {
    customAlloc: None,
    customFree: None,
    opaque: NULL,
};
static mut g_poolCtx: POOL_ctx = POOL_ctx { dummy: 0 };
#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_create(
    mut numThreads: size_t,
    mut queueSize: size_t,
) -> *mut POOL_ctx {
    return POOL_create_advanced(numThreads, queueSize, ZSTD_defaultCMem);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_create_advanced(
    mut numThreads: size_t,
    mut queueSize: size_t,
    mut customMem: ZSTD_customMem,
) -> *mut POOL_ctx {
    return &raw mut g_poolCtx;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_free(mut ctx: *mut POOL_ctx) {}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_joinJobs(mut ctx: *mut POOL_ctx) {}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_resize(
    mut ctx: *mut POOL_ctx,
    mut numThreads: size_t,
) -> ::core::ffi::c_int {
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_add(
    mut ctx: *mut POOL_ctx,
    mut function: POOL_function,
    mut opaque: *mut ::core::ffi::c_void,
) {
    function.expect("non-null function pointer")(opaque);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_tryAdd(
    mut ctx: *mut POOL_ctx,
    mut function: POOL_function,
    mut opaque: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    function.expect("non-null function pointer")(opaque);
    return 1 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn POOL_sizeof(mut ctx: *const POOL_ctx) -> size_t {
    if ctx.is_null() {
        return 0 as size_t;
    }
    return ::core::mem::size_of::<POOL_ctx>() as size_t;
}
