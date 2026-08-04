// Translated from c_src/src/lib.c
// The only public C symbol is `arr_push`, declared in lib.h.
//
// The original C function uses the stb_ds dynamic-array macros to push
// values 0..i into a heap-allocated array, then frees it, repeating in
// steps of 50 from 0 up to (but not including) `num`. The function has
// no return value and produces no I/O, so byte-identical output is
// trivially preserved by performing an equivalent (allocate / push /
// free) sequence in safe Rust.

use std::os::raw::c_int;

/// Faithful translation of:
///
/// ```c
/// void arr_push(int num)
/// {
///   int *arr = NULL;
///   int i, j;
///   STBDS_ASSERT(arrlen(arr) == 0);
///   for (i = 0; i < num; i += 50) {
///     for (j = 0; j < i; ++j)
///       arrpush(arr, j);
///     arrfree(arr);
///   }
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn arr_push(num: c_int) {
    // STBDS_ASSERT(arrlen(NULL) == 0): trivially true; debug_assert documents intent.
    debug_assert_eq!(0_isize, 0_isize);

    let mut i: c_int = 0;
    while i < num {
        // Allocate a fresh dynamic array (mirrors `int *arr = NULL` re-init each
        // outer iteration via the trailing arrfree).
        let mut arr: Vec<c_int> = Vec::new();
        let mut j: c_int = 0;
        while j < i {
            arr.push(j);
            j += 1;
        }
        // arrfree(arr): drop releases the storage.
        drop(arr);
        i += 50;
    }
}
