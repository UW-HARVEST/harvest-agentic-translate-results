// Translation of c_src/src/lib.c
//
// The C source includes a large stb_ds-style data structure implementation,
// but the only public function exported is `arr_del(int num)`. That function
// has no observable output (no I/O, no return value): it merely allocates,
// pushes integers into a dynamic array, deletes an element, and frees it.
//
// To preserve byte-identical output for the same inputs, we reproduce the
// same logical sequence of array operations using a Rust `Vec<i32>`. Since
// nothing is printed and nothing is returned, the externally observable
// output is empty in both implementations.

use std::ffi::c_int;

/// Reimplementation of the C macro `arrdel(a,i)`, which is
/// `memmove(&a[i], &a[i+1], sizeof(*a) * (length - 1 - i)); length -= 1;`.
///
/// In Rust this is equivalent to `Vec::remove`, except that the original C
/// code performs no bounds check beyond what's implied by the memmove length.
/// We mirror the same effect: shift elements left by one and pop the tail.
fn arr_del_at(arr: &mut Vec<c_int>, i: usize) {
    // Equivalent to memmove with length (len - 1 - i) and then length -= 1.
    if i < arr.len() {
        arr.remove(i);
    }
}

/// Reimplementation of the C macro `arrdelswap(a,i)`, which is
/// `a[i] = arrlast(a); length -= 1;` (swap-remove with the last element).
fn arr_delswap_at(arr: &mut Vec<c_int>, i: usize) {
    if i < arr.len() {
        arr.swap_remove(i);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn arr_del(num: c_int) {
    let mut arr: Vec<c_int> = Vec::new();

    for i in 0..4usize {
        // First sub-iteration: push four elements then arrdel(arr, i)
        arr.push(num);
        arr.push(2);
        arr.push(3);
        arr.push(4);
        arr_del_at(&mut arr, i);
        // arrfree -> drop the contents (equivalent to setting to NULL)
        arr.clear();
        arr.shrink_to_fit();

        // Second sub-iteration: push four elements then arrdelswap(arr, i)
        arr.push(num);
        arr.push(2);
        arr.push(3);
        arr.push(4);
        arr_delswap_at(&mut arr, i);
        arr.clear();
        arr.shrink_to_fit();
    }

    // arr goes out of scope and is dropped here, equivalent to the final
    // arrfree on the C side at function exit (well, in C it's already freed
    // by the last arrfree call inside the loop; same semantics here).
    let _ = arr;
}
