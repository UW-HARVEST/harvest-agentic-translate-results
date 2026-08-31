//! Translation of `common/allocations.h` plus the `ZSTD_customMem` type from
//! `zstd.h`. The C build maps `ZSTD_malloc`/`ZSTD_calloc`/`ZSTD_free` straight
//! onto libc, so we call libc too rather than Rust's allocator.
#![allow(dead_code)]
#![allow(non_upper_case_globals)]

use core::ffi::c_void;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

pub type ZSTD_allocFunction = Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>;
pub type ZSTD_freeFunction = Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>;

/// `ZSTD_customMem`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_customMem {
    pub customAlloc: ZSTD_allocFunction,
    pub customFree: ZSTD_freeFunction,
    pub opaque: *mut c_void,
}

impl Default for ZSTD_customMem {
    fn default() -> Self {
        ZSTD_customMem {
            customAlloc: None,
            customFree: None,
            opaque: core::ptr::null_mut(),
        }
    }
}

/// `ZSTD_defaultCMem` — `{ NULL, NULL, NULL }`
pub const ZSTD_defaultCMem: ZSTD_customMem = ZSTD_customMem {
    customAlloc: None,
    customFree: None,
    opaque: core::ptr::null_mut(),
};

/// `ZSTD_customMalloc()`
#[inline]
pub unsafe fn zstd_custom_malloc(size: usize, custom_mem: ZSTD_customMem) -> *mut c_void {
    if let Some(alloc) = custom_mem.customAlloc {
        return alloc(custom_mem.opaque, size);
    }
    malloc(size)
}

/// `ZSTD_customCalloc()`
#[inline]
pub unsafe fn zstd_custom_calloc(size: usize, custom_mem: ZSTD_customMem) -> *mut c_void {
    if let Some(alloc) = custom_mem.customAlloc {
        let ptr = alloc(custom_mem.opaque, size);
        core::ptr::write_bytes(ptr as *mut u8, 0, size);
        return ptr;
    }
    calloc(1, size)
}

/// `ZSTD_customFree()`
#[inline]
pub unsafe fn zstd_custom_free(ptr: *mut c_void, custom_mem: ZSTD_customMem) {
    if !ptr.is_null() {
        if let Some(f) = custom_mem.customFree {
            f(custom_mem.opaque, ptr);
        } else {
            free(ptr);
        }
    }
}
