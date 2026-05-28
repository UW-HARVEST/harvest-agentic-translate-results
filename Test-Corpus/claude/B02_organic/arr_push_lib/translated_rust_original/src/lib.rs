//! Rust translation of c_src/src/lib.c.
//!
//! The C library only exposes one public symbol via include/lib.h:
//!     void arr_push(int num);
//!
//! The internal stbds_* dynamic-array implementation is library-private and
//! not exported as part of the public C ABI for this library, so the Rust
//! translation only needs to faithfully reproduce the externally observable
//! behaviour of `arr_push` (which has no return value and no I/O).

use std::ffi::c_int;

/// Translation of the C function `arr_push(int num)` from c_src/src/lib.c.
///
/// The C implementation:
///   void arr_push(int num)
///   {
///     int *arr = NULL;
///     int i, j;
///     STBDS_ASSERT(arrlen(arr) == 0);
///     for (i = 0; i < num; i += 50) {
///       for (j = 0; j < i; ++j)
///         arrpush(arr, j);
///       arrfree(arr);
///     }
///   }
///
/// `arrlen(NULL)` evaluates to 0 so the assert always succeeds. The body
/// then walks i = 0, 50, 100, ... < num, building up a fresh array of
/// 0..i and freeing it after each iteration. There is no observable
/// side-effect from outside the library (no return value, no I/O), so a
/// straightforward safe Rust port using Vec produces byte-identical
/// observable output.
#[unsafe(no_mangle)]
pub extern "C" fn arr_push(num: c_int) {
    // Mirror the C `arrlen(NULL) == 0` precondition assertion.
    let arr: Vec<c_int> = Vec::new();
    assert_eq!(arr.len(), 0);
    drop(arr);

    let mut i: c_int = 0;
    while i < num {
        let mut arr: Vec<c_int> = Vec::new();
        let mut j: c_int = 0;
        while j < i {
            arr.push(j);
            j += 1;
        }
        // arrfree(arr) — drop the Vec.
        drop(arr);
        i += 50;
    }
}
