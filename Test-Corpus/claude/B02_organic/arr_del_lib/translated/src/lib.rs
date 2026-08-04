// Translation of c_src/src/lib.c to Rust.
//
// The public C API is a single function:
//     void arr_del(int num);
// declared in c_src/include/lib.h.
//
// The implementation in C uses stb_ds dynamic-array macros to push values,
// delete them at various indices, and free the array. The function returns
// nothing, performs no I/O, and exposes no observable side effects (no
// globals, no output stream writes, no return value). Therefore, "byte
// identical output for the same inputs" is satisfied trivially: any
// implementation that does not perform observable I/O is byte-identical
// to the C implementation (both produce no output).
//
// To remain faithful in spirit to the original C, we still mirror the
// sequence of pushes / deletions on a Rust Vec<i32>. The Vec is dropped
// at the end of each iteration, mirroring the arrfree calls.

use std::os::raw::c_int;

fn arrdel<T: Copy>(v: &mut Vec<T>, i: usize) {
    // stbds_arrdel(a,i) == stbds_arrdeln(a,i,1)
    // which removes one element at index i, shifting subsequent entries.
    v.remove(i);
}

fn arrdelswap<T: Copy>(v: &mut Vec<T>, i: usize) {
    // stbds_arrdelswap(a,i): a[i] = arrlast(a); length -= 1
    let last = *v.last().expect("arrdelswap on empty array");
    v[i] = last;
    v.pop();
}

#[unsafe(no_mangle)]
pub extern "C" fn arr_del(num: c_int) {
    for i in 0..4i32 {
        // First sequence: push 4 elements, then arrdel(arr, i), then free.
        let mut arr: Vec<c_int> = Vec::new();
        arr.push(num);
        arr.push(2);
        arr.push(3);
        arr.push(4);
        arrdel(&mut arr, i as usize);
        drop(arr);

        // Second sequence: push 4 elements, then arrdelswap(arr, i), then free.
        let mut arr: Vec<c_int> = Vec::new();
        arr.push(num);
        arr.push(2);
        arr.push(3);
        arr.push(4);
        arrdelswap(&mut arr, i as usize);
        drop(arr);
    }
}
