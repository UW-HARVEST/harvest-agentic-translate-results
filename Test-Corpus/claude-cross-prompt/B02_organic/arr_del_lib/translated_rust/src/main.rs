// Translation of c_src/src/lib.c
//
// The C source is a single-file port of stb_ds plus an `arr_del(int num)`
// entry point exposed via include/lib.h. The CMake target produces a SHARED
// library — there is no `main()` in the C and no I/O is performed.
//
// Therefore, the byte-identical output for the executable form is empty
// (no stdout, no stderr). We provide a faithful port of `arr_del` for
// completeness and reference it from `main` so it is reachable, but we do
// not invoke it (and it has no observable I/O even if called).

#![allow(dead_code)]

/// Equivalent of the C `arr_del(int num)` exposed by lib.h.
///
/// Reproduces the same sequence of dynamic-array operations the C version
/// performs:
///   for i in 0..4 {
///     push(num); push(2); push(3); push(4);
///     remove index i;
///     free;
///     push(num); push(2); push(3); push(4);
///     swap_remove index i;
///     free;
///   }
fn arr_del(num: i32) {
    for i in 0..4usize {
        // arrpush(arr,num); arrpush(arr,2); arrpush(arr,3); arrpush(arr,4);
        let mut arr: Vec<i32> = Vec::new();
        arr.push(num);
        arr.push(2);
        arr.push(3);
        arr.push(4);
        // arrdel(arr,i);
        arr.remove(i);
        // arrfree(arr);
        drop(arr);

        // arrpush(arr,num); arrpush(arr,2); arrpush(arr,3); arrpush(arr,4);
        let mut arr: Vec<i32> = Vec::new();
        arr.push(num);
        arr.push(2);
        arr.push(3);
        arr.push(4);
        // arrdelswap(arr,i): arr[i] = arr[len-1]; --len;
        let last = arr.pop().expect("arrdelswap on empty array");
        if i < arr.len() {
            arr[i] = last;
        }
        // arrfree(arr);
        drop(arr);
    }
}

fn main() {
    // The C code has no main() and the only exposed function (arr_del)
    // performs no I/O. Producing no output preserves byte-identical
    // behavior of the (non-existent) executable.
    let _ = arr_del;
}
