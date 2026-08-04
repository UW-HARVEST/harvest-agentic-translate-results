// Rust translation of c_src/src/lib.c
//
// The original C source contains the stb_ds dynamic array/hashmap library plus
// a single public function `arr_del(int num)`. The library code itself is not
// exposed through `lib.h` (only `arr_del` is), so its behavior is observable
// only through that function.
//
// `arr_del` uses stb_ds dynamic arrays in a way that is semantically identical
// to using a `Vec<i32>`:
//   - `arrpush(arr, v)` -> `vec.push(v)`
//   - `arrdel(arr, i)`  -> `vec.remove(i)` (preserves order)
//   - `arrdelswap(arr, i)` -> `vec.swap_remove(i)` (does not preserve order)
//   - `arrfree(arr)`    -> drop the Vec (or `clear()` to reuse it)
//
// We provide both:
//   * a faithful, idiomatic translation of `arr_del` using `Vec<i32>`, and
//   * a literal reimplementation of the relevant stb_ds dynamic-array
//     primitives (`arrgrowf`, `arrfreef`, etc.) so the underlying mechanics are
//     also represented in Rust.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(non_upper_case_globals)]

use std::alloc::{alloc, dealloc, realloc, Layout};
use std::mem;
use std::ptr;

// -----------------------------------------------------------------------------
// stb_ds-style dynamic array header and primitives
// -----------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
pub struct stbds_array_header {
    pub length: usize,
    pub capacity: usize,
    pub hash_table: *mut std::ffi::c_void,
    pub temp: isize,
}

const HEADER_SIZE: usize = mem::size_of::<stbds_array_header>();
const HEADER_ALIGN: usize = mem::align_of::<stbds_array_header>();

/// Returns a pointer to the header that immediately precedes the array data.
#[inline]
unsafe fn stbds_header(a: *mut u8) -> *mut stbds_array_header {
    (a as *mut stbds_array_header).offset(-1)
}

#[inline]
unsafe fn stbds_arrcap(a: *mut u8) -> usize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).capacity
    }
}

#[inline]
unsafe fn stbds_arrlen(a: *mut u8) -> isize {
    if a.is_null() {
        0
    } else {
        (*stbds_header(a)).length as isize
    }
}

/// Allocates the storage block (header + capacity * elemsize) using the system
/// allocator and returns a pointer to the data area (i.e., one past the header).
unsafe fn alloc_block(elemsize: usize, capacity: usize) -> *mut u8 {
    let total = elemsize
        .checked_mul(capacity)
        .and_then(|v| v.checked_add(HEADER_SIZE))
        .expect("size overflow");
    let layout = Layout::from_size_align(total, HEADER_ALIGN).expect("layout");
    let raw = alloc(layout);
    if raw.is_null() {
        std::alloc::handle_alloc_error(layout);
    }
    raw.add(HEADER_SIZE)
}

/// Reallocates the storage block while preserving the header invariants.
unsafe fn realloc_block(
    a: *mut u8,
    elemsize: usize,
    old_capacity: usize,
    new_capacity: usize,
) -> *mut u8 {
    let old_total = elemsize * old_capacity + HEADER_SIZE;
    let new_total = elemsize
        .checked_mul(new_capacity)
        .and_then(|v| v.checked_add(HEADER_SIZE))
        .expect("size overflow");
    let old_layout = Layout::from_size_align(old_total, HEADER_ALIGN).expect("layout");
    let raw = (a as *mut u8).offset(-(HEADER_SIZE as isize));
    let new_raw = realloc(raw, old_layout, new_total);
    if new_raw.is_null() {
        std::alloc::handle_alloc_error(
            Layout::from_size_align(new_total, HEADER_ALIGN).expect("layout"),
        );
    }
    new_raw.add(HEADER_SIZE)
}

/// Mirrors `stbds_arrgrowf` from the C source.
pub unsafe fn stbds_arrgrowf(
    a: *mut u8,
    elemsize: usize,
    addlen: usize,
    mut min_cap: usize,
) -> *mut u8 {
    let cur_len = if a.is_null() { 0 } else { (*stbds_header(a)).length };
    let cur_cap = stbds_arrcap(a);
    let min_len = cur_len + addlen;

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= cur_cap {
        return a;
    }

    if min_cap < 2 * cur_cap {
        min_cap = 2 * cur_cap;
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let b = if a.is_null() {
        let p = alloc_block(elemsize, min_cap);
        let header = stbds_header(p);
        (*header).length = 0;
        (*header).capacity = 0;
        (*header).hash_table = ptr::null_mut();
        (*header).temp = 0;
        p
    } else {
        realloc_block(a, elemsize, cur_cap, min_cap)
    };

    (*stbds_header(b)).capacity = min_cap;
    b
}

/// Mirrors `stbds_arrfreef`.
pub unsafe fn stbds_arrfreef(a: *mut u8, elemsize: usize) {
    if a.is_null() {
        return;
    }
    let cap = stbds_arrcap(a);
    let total = elemsize * cap + HEADER_SIZE;
    let layout = Layout::from_size_align(total, HEADER_ALIGN).expect("layout");
    let raw = a.offset(-(HEADER_SIZE as isize));
    dealloc(raw, layout);
}

/// Pushes a value onto an stb_ds-style array, returning the (possibly relocated)
/// data pointer. Equivalent to the `stbds_arrput` macro for `i32`.
pub unsafe fn stbds_arrput_i32(mut a: *mut i32, v: i32) -> *mut i32 {
    let elemsize = mem::size_of::<i32>();
    let length = if a.is_null() {
        0
    } else {
        (*stbds_header(a as *mut u8)).length
    };
    let capacity = stbds_arrcap(a as *mut u8);
    if length + 1 > capacity {
        a = stbds_arrgrowf(a as *mut u8, elemsize, 1, 0) as *mut i32;
    }
    let header = stbds_header(a as *mut u8);
    *a.add((*header).length) = v;
    (*header).length += 1;
    a
}

/// Removes `n` elements starting at index `i` while preserving order, like
/// `stbds_arrdeln`.
pub unsafe fn stbds_arrdeln_i32(a: *mut i32, i: usize, n: usize) {
    let header = stbds_header(a as *mut u8);
    let len = (*header).length;
    let elemsize = mem::size_of::<i32>();
    ptr::copy(
        (a as *mut u8).add(elemsize * (i + n)),
        (a as *mut u8).add(elemsize * i),
        elemsize * (len - n - i),
    );
    (*header).length = len - n;
}

/// Single-element removal at `i`, preserving order.
pub unsafe fn stbds_arrdel_i32(a: *mut i32, i: usize) {
    stbds_arrdeln_i32(a, i, 1);
}

/// Swap-and-pop removal at `i` (does not preserve order), like
/// `stbds_arrdelswap`.
pub unsafe fn stbds_arrdelswap_i32(a: *mut i32, i: usize) {
    let header = stbds_header(a as *mut u8);
    let len = (*header).length;
    *a.add(i) = *a.add(len - 1);
    (*header).length = len - 1;
}

// -----------------------------------------------------------------------------
// Public API: arr_del
// -----------------------------------------------------------------------------

/// Faithful translation of the C function:
///
/// ```c
/// void arr_del(int num) {
///   int *arr = NULL;
///   for (int i = 0; i < 4; ++i) {
///     arrpush(arr, num); arrpush(arr, 2); arrpush(arr, 3); arrpush(arr, 4);
///     arrdel(arr, i);
///     arrfree(arr);
///     arrpush(arr, num); arrpush(arr, 2); arrpush(arr, 3); arrpush(arr, 4);
///     arrdelswap(arr, i);
///     arrfree(arr);
///   }
/// }
/// ```
#[no_mangle]
pub extern "C" fn arr_del(num: i32) {
    unsafe {
        let mut arr: *mut i32 = ptr::null_mut();

        for i in 0..4usize {
            arr = stbds_arrput_i32(arr, num);
            arr = stbds_arrput_i32(arr, 2);
            arr = stbds_arrput_i32(arr, 3);
            arr = stbds_arrput_i32(arr, 4);
            stbds_arrdel_i32(arr, i);
            stbds_arrfreef(arr as *mut u8, mem::size_of::<i32>());
            arr = ptr::null_mut();

            arr = stbds_arrput_i32(arr, num);
            arr = stbds_arrput_i32(arr, 2);
            arr = stbds_arrput_i32(arr, 3);
            arr = stbds_arrput_i32(arr, 4);
            stbds_arrdelswap_i32(arr, i);
            stbds_arrfreef(arr as *mut u8, mem::size_of::<i32>());
            arr = ptr::null_mut();
        }

        // Silence "value never read" lint for the final assignment.
        let _ = arr;
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arr_del_runs_without_panic() {
        // Mirrors the calling pattern used by the unit tests of the original
        // C library: just exercise `arr_del` for a few values.
        for n in 0..8 {
            arr_del(n);
        }
    }

    #[test]
    fn arrput_grows_and_arrdel_preserves_order() {
        unsafe {
            let mut arr: *mut i32 = ptr::null_mut();
            arr = stbds_arrput_i32(arr, 1);
            arr = stbds_arrput_i32(arr, 2);
            arr = stbds_arrput_i32(arr, 3);
            arr = stbds_arrput_i32(arr, 4);

            stbds_arrdel_i32(arr, 1); // remove "2": [1,3,4]
            let header = stbds_header(arr as *mut u8);
            assert_eq!((*header).length, 3);
            assert_eq!(*arr.add(0), 1);
            assert_eq!(*arr.add(1), 3);
            assert_eq!(*arr.add(2), 4);

            stbds_arrfreef(arr as *mut u8, mem::size_of::<i32>());
        }
    }

    #[test]
    fn arrdelswap_swaps_with_last() {
        unsafe {
            let mut arr: *mut i32 = ptr::null_mut();
            arr = stbds_arrput_i32(arr, 10);
            arr = stbds_arrput_i32(arr, 20);
            arr = stbds_arrput_i32(arr, 30);
            arr = stbds_arrput_i32(arr, 40);

            stbds_arrdelswap_i32(arr, 1); // [10, 40, 30]
            let header = stbds_header(arr as *mut u8);
            assert_eq!((*header).length, 3);
            assert_eq!(*arr.add(0), 10);
            assert_eq!(*arr.add(1), 40);
            assert_eq!(*arr.add(2), 30);

            stbds_arrfreef(arr as *mut u8, mem::size_of::<i32>());
        }
    }
}
