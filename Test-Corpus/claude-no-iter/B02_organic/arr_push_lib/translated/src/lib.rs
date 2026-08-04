// Rust translation of c_src/src/lib.c
//
// Only `arr_push` (declared in lib.h) is part of the public C ABI. The C
// implementation uses the stb_ds dynamic array macros to build and free a
// transient array; it produces no externally observable output. We reproduce
// the same control-flow semantics here so the function is a no-op that runs
// to completion for the same inputs.

use std::ffi::c_int;

/// Mirrors the C function `void arr_push(int num);` from `c_src/include/lib.h`.
///
/// The C version performs:
///
/// ```c
/// void arr_push(int num) {
///   int *arr = NULL;
///   int i, j;
///   assert(arrlen(arr) == 0);
///   for (i = 0; i < num; i += 50) {
///     for (j = 0; j < i; ++j)
///       arrpush(arr, j);
///     arrfree(arr);
///   }
/// }
/// ```
///
/// We use `Vec<c_int>` for the internal storage. The `arr` pointer is `NULL`
/// at function entry, so `arrlen(arr) == 0` is trivially true; we therefore
/// preserve that assertion via `debug_assert!` (matching the C `assert`,
/// which is compiled out in NDEBUG builds).
#[unsafe(no_mangle)]
pub extern "C" fn arr_push(num: c_int) {
    let mut arr: Vec<c_int> = Vec::new();

    // STBDS_ASSERT(arrlen(arr) == 0); -- arr is NULL, length is 0.
    debug_assert!(arr.len() == 0);

    let mut i: c_int = 0;
    while i < num {
        let mut j: c_int = 0;
        while j < i {
            arr.push(j);
            j += 1;
        }
        // arrfree(arr): release storage and reset to "NULL".
        arr = Vec::new();
        i += 50;
    }
}
