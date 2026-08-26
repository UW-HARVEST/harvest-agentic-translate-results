// Translation of c_src/src/lib.c to Rust.
//
// The public C API (declared in c_src/include/lib.h) consists of a single
// function: `void arr_ins(int num)`.
//
// The original C source contains a substantial amount of code copied from the
// stb_ds.h dynamic-array / hash-map library, but only the `arr_ins` function
// is actually exposed and exercised. In Rust we can simply use `Vec<i32>` to
// model the dynamic array and translate the function body directly.

use std::os::raw::c_int;

/// Inserts an element at index `i` in the vector `arr`, shifting subsequent
/// elements one position to the right (mirrors stbds_arrins).
fn arrins(arr: &mut Vec<i32>, i: usize, v: i32) {
    arr.insert(i, v);
}

/// Pushes a value onto the back of the vector (mirrors arrpush / stbds_arrput).
fn arrpush(arr: &mut Vec<i32>, v: i32) {
    arr.push(v);
}

/// Frees the vector by clearing it (mirrors arrfree which sets the array to
/// NULL after freeing). In Rust we drop & re-create.
fn arrfree(arr: &mut Vec<i32>) {
    arr.clear();
    arr.shrink_to_fit();
}

/// Translation of the original C `arr_ins` function. Keeps the same observable
/// behavior, including the runtime assertions.
#[no_mangle]
pub extern "C" fn arr_ins(num: c_int) {
    let mut arr: Vec<i32> = Vec::new();

    for i in 0..5 {
        arrpush(&mut arr, 1);
        arrpush(&mut arr, 2);
        arrpush(&mut arr, 3);
        arrpush(&mut arr, 4);
        arrins(&mut arr, i, num);
        assert_eq!(arr[i], num);
        if i < 4 {
            assert_eq!(arr[4], 4);
        }
        arrfree(&mut arr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arr_ins() {
        // Should not panic for various values of num.
        arr_ins(0);
        arr_ins(42);
        arr_ins(-1);
    }
}
