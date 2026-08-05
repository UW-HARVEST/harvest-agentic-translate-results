//! Translation of common/allocations.h — custom allocation primitives.
//! Uses libc malloc/calloc/free to match C runtime behavior exactly.
#![allow(dead_code)]
use core::ffi::c_void;

pub type ZSTD_allocFunction = Option<extern "C" fn(*mut c_void, usize) -> *mut c_void>;
pub type ZSTD_freeFunction = Option<extern "C" fn(*mut c_void, *mut c_void)>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_customMem {
    pub customAlloc: ZSTD_allocFunction,
    pub customFree: ZSTD_freeFunction,
    pub opaque: *mut c_void,
}

pub const ZSTD_defaultCMem: ZSTD_customMem = ZSTD_customMem {
    customAlloc: None,
    customFree: None,
    opaque: core::ptr::null_mut(),
};

extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memset(dest: *mut c_void, c: i32, n: usize) -> *mut c_void;
    pub fn qsort(
        base: *mut c_void,
        nmemb: usize,
        size: usize,
        compar: extern "C" fn(*const c_void, *const c_void) -> i32,
    );
}

#[inline]
pub unsafe fn zstd_custom_malloc(size: usize, customMem: ZSTD_customMem) -> *mut c_void {
    if let Some(alloc) = customMem.customAlloc {
        return alloc(customMem.opaque, size);
    }
    malloc(size)
}

#[inline]
pub unsafe fn zstd_custom_calloc(size: usize, customMem: ZSTD_customMem) -> *mut c_void {
    if let Some(alloc) = customMem.customAlloc {
        let ptr = alloc(customMem.opaque, size);
        memset(ptr, 0, size);
        return ptr;
    }
    calloc(1, size)
}

#[inline]
pub unsafe fn zstd_custom_free(ptr: *mut c_void, customMem: ZSTD_customMem) {
    if !ptr.is_null() {
        if let Some(f) = customMem.customFree {
            f(customMem.opaque, ptr);
        } else {
            free(ptr);
        }
    }
}
