use crate::types::{json_free_t, json_malloc_t, json_realloc_t};
use std::ffi::c_void;
use std::ptr;
use std::sync::RwLock;

#[derive(Clone, Copy)]
struct Allocators {
    malloc: json_malloc_t,
    realloc: json_realloc_t,
    free: json_free_t,
}

unsafe extern "C" fn default_malloc(size: usize) -> *mut c_void {
    libc::malloc(size)
}

unsafe extern "C" fn default_realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    libc::realloc(ptr, size)
}

unsafe extern "C" fn default_free(ptr: *mut c_void) {
    libc::free(ptr)
}

static ALLOCATORS: RwLock<Allocators> = RwLock::new(Allocators {
    malloc: Some(default_malloc),
    realloc: Some(default_realloc),
    free: Some(default_free),
});

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_malloc(size: usize) -> *mut c_void {
    if size == 0 {
        return ptr::null_mut();
    }
    let malloc = ALLOCATORS.read().unwrap().malloc.unwrap();
    malloc(size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_realloc(
    ptr: *mut c_void,
    original_size: usize,
    new_size: usize,
) -> *mut c_void {
    let alloc = *ALLOCATORS.read().unwrap();
    if let Some(realloc) = alloc.realloc {
        return realloc(ptr, new_size);
    }
    if new_size == 0 {
        if !ptr.is_null() {
            alloc.free.unwrap()(ptr);
        }
        return ptr::null_mut();
    }
    let new_ptr = alloc.malloc.unwrap()(new_size);
    if !new_ptr.is_null() && !ptr.is_null() {
        ptr::copy_nonoverlapping(
            ptr.cast::<u8>(),
            new_ptr.cast::<u8>(),
            original_size.min(new_size),
        );
        alloc.free.unwrap()(ptr);
    }
    new_ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_free(ptr: *mut c_void) {
    if !ptr.is_null() {
        (ALLOCATORS.read().unwrap().free.unwrap())(ptr);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_strndup(src: *const i8, len: usize) -> *mut i8 {
    let dst = jsonp_malloc(len + 1).cast::<i8>();
    if dst.is_null() {
        return ptr::null_mut();
    }
    ptr::copy_nonoverlapping(src, dst, len);
    *dst.add(len) = 0;
    dst
}

#[unsafe(no_mangle)]
pub extern "C" fn json_set_alloc_funcs(malloc_fn: json_malloc_t, free_fn: json_free_t) {
    json_set_alloc_funcs2(malloc_fn, None, free_fn);
}

#[unsafe(no_mangle)]
pub extern "C" fn json_set_alloc_funcs2(
    malloc_fn: json_malloc_t,
    realloc_fn: json_realloc_t,
    free_fn: json_free_t,
) {
    *ALLOCATORS.write().unwrap() = Allocators {
        malloc: malloc_fn,
        realloc: realloc_fn,
        free: free_fn,
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_get_alloc_funcs(
    malloc_fn: *mut json_malloc_t,
    free_fn: *mut json_free_t,
) {
    let alloc = *ALLOCATORS.read().unwrap();
    if !malloc_fn.is_null() {
        *malloc_fn = alloc.malloc;
    }
    if !free_fn.is_null() {
        *free_fn = alloc.free;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_get_alloc_funcs2(
    malloc_fn: *mut json_malloc_t,
    realloc_fn: *mut json_realloc_t,
    free_fn: *mut json_free_t,
) {
    let alloc = *ALLOCATORS.read().unwrap();
    if !malloc_fn.is_null() {
        *malloc_fn = alloc.malloc;
    }
    if !realloc_fn.is_null() {
        *realloc_fn = alloc.realloc;
    }
    if !free_fn.is_null() {
        *free_fn = alloc.free;
    }
}
