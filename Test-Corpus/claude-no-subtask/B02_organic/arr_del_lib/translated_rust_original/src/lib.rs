// Translation of c_src/src/lib.c
//
// The only public symbol exposed by the C library (per c_src/include/lib.h)
// is `arr_del(int num)`. The rest of the C source is the internal `stb_ds`
// data-structures library used to implement `arr_del`.
//
// `arr_del` has no observable output (no return value, no I/O). It only
// allocates, mutates, and frees a dynamic array. The C implementation
// performs the following sequence for i in 0..4:
//
//     arr = NULL
//     push(arr, num); push(arr, 2); push(arr, 3); push(arr, 4);
//     arrdel(arr, i);                  // remove element at index i,
//                                      // shifting subsequent elements left
//     arrfree(arr); arr = NULL;
//     push(arr, num); push(arr, 2); push(arr, 3); push(arr, 4);
//     arrdelswap(arr, i);              // replace element at index i with
//                                      // last element, then shrink
//     arrfree(arr); arr = NULL;
//
// Because the function produces no output, a faithful in-process
// reproduction of the operations is sufficient to be byte-identical for
// any input.

use std::ffi::c_int;

/// Mirrors `stbds_arrdel(arr, i)` which calls `stbds_arrdeln(arr, i, 1)`.
///
/// The C macro performs:
///   memmove(&arr[i], &arr[i+1], sizeof *arr * (length - 1 - i));
///   length -= 1;
///
/// In safe Rust, this is exactly `Vec::remove`.
fn arrdel(arr: &mut Vec<c_int>, i: usize) {
    arr.remove(i);
}

/// Mirrors `stbds_arrdelswap(arr, i)`:
///   arr[i] = arr[length - 1];
///   length -= 1;
///
/// In safe Rust, this is exactly `Vec::swap_remove`.
fn arrdelswap(arr: &mut Vec<c_int>, i: usize) {
    arr.swap_remove(i);
}

#[unsafe(no_mangle)]
pub extern "C" fn arr_del(num: c_int) {
    let mut arr: Vec<c_int> = Vec::new();

    let mut i: c_int = 0;
    while i < 4 {
        // First sequence: arrdel
        arr.push(num);
        arr.push(2);
        arr.push(3);
        arr.push(4);
        arrdel(&mut arr, i as usize);
        arr.clear();
        arr.shrink_to_fit();

        // Second sequence: arrdelswap
        arr.push(num);
        arr.push(2);
        arr.push(3);
        arr.push(4);
        arrdelswap(&mut arr, i as usize);
        arr.clear();
        arr.shrink_to_fit();

        i += 1;
    }
}
