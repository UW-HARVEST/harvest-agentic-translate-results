// Rust translation of c_src/src/lib.c
//
// The only public C symbol is `arr_push(int num)`. The function performs:
//   for i in 0,50,100,...,<num:
//     for j in 0..i:  arrpush(arr, j)
//     arrfree(arr)
//
// arrpush/arrfree are stb_ds dynamic-array macros; the function returns void
// and produces no observable output. We faithfully reproduce the loop using a
// Rust Vec — push/free semantics map directly onto Vec::push/drop.

use std::os::raw::c_int;

/// C signature: `void arr_push(int num);`
///
/// Mirrors the C body exactly: iterate i in steps of 50 from 0 up to (but not
/// including) num, push j=0..i onto a fresh dynamic array each iteration,
/// then free the array.
#[unsafe(no_mangle)]
pub extern "C" fn arr_push(num: c_int) {
    let mut arr: Vec<c_int> = Vec::new();
    debug_assert!(arr.is_empty());

    let mut i: c_int = 0;
    while i < num {
        let mut j: c_int = 0;
        while j < i {
            arr.push(j);
            j += 1;
        }
        // arrfree(arr): release storage and reset pointer to "null"
        arr.clear();
        arr.shrink_to_fit();
        // Re-create as fresh empty Vec so the next iteration starts from zero
        // capacity, matching the C behavior where arr is set back to NULL.
        arr = Vec::new();

        i += 50;
    }
}
