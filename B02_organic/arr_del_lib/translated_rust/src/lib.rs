#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    unused_assignments,
    unused_variables,
    clippy::all
)]

use std::ffi::{c_int, c_void};
use std::ptr;

extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

// ============================================================
// stbds_array_header
// ============================================================
#[repr(C)]
struct stbds_array_header {
    length: usize,
    capacity: usize,
    hash_table: *mut u8,
    temp: isize,
}

const HEADER_SIZE: usize = std::mem::size_of::<stbds_array_header>();

unsafe fn stbds_header(t: *mut u8) -> *mut stbds_array_header {
    (t as *mut stbds_array_header).offset(-1)
}

unsafe fn stbds_arrlen(a: *mut u8) -> isize {
    if a.is_null() { 0 } else { (*stbds_header(a)).length as isize }
}

unsafe fn stbds_arrcap(a: *mut u8) -> usize {
    if a.is_null() { 0 } else { (*stbds_header(a)).capacity }
}

// ============================================================
// stbds_arrgrowf
// ============================================================
unsafe fn stbds_arrgrowf(a: *mut u8, elemsize: usize, addlen: usize, min_cap_in: usize) -> *mut u8 {
    let min_len = (stbds_arrlen(a) as usize).wrapping_add(addlen);
    let mut min_cap = min_cap_in;

    if min_len > min_cap {
        min_cap = min_len;
    }
    if min_cap <= stbds_arrcap(a) {
        return a;
    }

    let double_cap = stbds_arrcap(a).wrapping_mul(2);
    if min_cap < double_cap {
        min_cap = double_cap;
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let alloc_size = elemsize * min_cap + HEADER_SIZE;
    let old_ptr = if a.is_null() {
        ptr::null_mut()
    } else {
        stbds_header(a) as *mut u8
    };

    let b_raw = libc_realloc(old_ptr, alloc_size);
    let b = b_raw.add(HEADER_SIZE);

    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    }
    (*stbds_header(b)).capacity = min_cap;

    b
}

// Thin wrappers around libc malloc/realloc/free to match C behavior
unsafe fn libc_realloc(ptr: *mut u8, size: usize) -> *mut u8 {
    if ptr.is_null() {
        libc_malloc(size)
    } else {
        realloc(ptr as *mut c_void, size) as *mut u8
    }
}

unsafe fn libc_malloc(size: usize) -> *mut u8 {
    malloc(size) as *mut u8
}

unsafe fn libc_free(p: *mut u8) {
    free(p as *mut c_void);
}

// ============================================================
// Array macros reimplemented as functions operating on *mut i32
// ============================================================

/// arrpush for i32 arrays
unsafe fn arr_i32_push(a: &mut *mut i32, v: i32) {
    let ptr = *a as *mut u8;
    let elemsize = std::mem::size_of::<i32>();
    // maybegrow
    if ptr.is_null() || (*stbds_header(ptr)).length + 1 > (*stbds_header(ptr)).capacity {
        *a = stbds_arrgrowf(ptr, elemsize, 1, 0) as *mut i32;
    }
    let p = *a as *mut u8;
    let len = (*stbds_header(p)).length;
    *(*a).add(len) = v;
    (*stbds_header(p)).length = len + 1;
}

/// arrdel(a, i) => arrdeln(a, i, 1)
unsafe fn arr_i32_del(a: *mut i32, i: usize) {
    let p = a as *mut u8;
    let elemsize = std::mem::size_of::<i32>();
    let hdr = stbds_header(p);
    let len = (*hdr).length;
    // memmove(&a[i], &a[i+1], sizeof(*a) * (len - 1 - i))
    ptr::copy(a.add(i + 1), a.add(i), len - 1 - i);
    (*hdr).length -= 1;
}

/// arrdelswap(a, i)
unsafe fn arr_i32_delswap(a: *mut i32, i: usize) {
    let p = a as *mut u8;
    let hdr = stbds_header(p);
    let len = (*hdr).length;
    *a.add(i) = *a.add(len - 1);
    (*hdr).length -= 1;
}

/// arrfree(a)
unsafe fn arr_i32_free(a: &mut *mut i32) {
    let p = *a as *mut u8;
    if !p.is_null() {
        libc_free(stbds_header(p) as *mut u8);
    }
    *a = ptr::null_mut();
}

// ============================================================
// arr_del — the public function
// ============================================================
#[unsafe(no_mangle)]
pub extern "C" fn arr_del(num: c_int) {
    unsafe {
        let mut arr: *mut i32 = ptr::null_mut();

        for i in 0..4u32 {
            let idx = i as usize;

            arr_i32_push(&mut arr, num);
            arr_i32_push(&mut arr, 2);
            arr_i32_push(&mut arr, 3);
            arr_i32_push(&mut arr, 4);
            arr_i32_del(arr, idx);
            arr_i32_free(&mut arr);

            arr_i32_push(&mut arr, num);
            arr_i32_push(&mut arr, 2);
            arr_i32_push(&mut arr, 3);
            arr_i32_push(&mut arr, 4);
            arr_i32_delswap(arr, idx);
            arr_i32_free(&mut arr);
        }
    }
}
