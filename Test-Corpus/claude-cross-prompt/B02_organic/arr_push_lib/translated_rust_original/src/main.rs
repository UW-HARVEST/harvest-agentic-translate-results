// Translation of c_src/src/lib.c to Rust.
//
// The C source is the stb_ds (single-file public-domain) data-structure
// implementation. The only function exposed in c_src/include/lib.h is
// `arr_push(int num)`, which performs a series of internal array
// allocations / pushes / frees. It produces no output and does not read
// any input.
//
// The C library has no `main` function, so the corresponding executable
// likewise produces no output for any input. We provide a Rust port of
// `arr_push` (using safe Rust internally) plus a minimal `main` that
// preserves the C behaviour: read nothing, print nothing.

/// Direct translation of the C `arr_push` function.
///
/// The C version allocates a dynamic array via stb_ds, pushes integers
/// into it, then frees it — repeating the cycle. The observable
/// behaviour is purely allocation churn; it produces no output.
///
/// We replicate the exact loop structure using a `Vec<i32>` (which gives
/// the same push/free semantics in safe Rust). The only difference is
/// the underlying allocator, which is invisible to the program output.
fn arr_push(num: i32) {
    let mut arr: Vec<i32> = Vec::new();

    // STBDS_ASSERT(arrlen(arr)==0);
    debug_assert_eq!(arr.len(), 0);

    // for (i=0; i < num; i += 50) {
    //   for (j=0; j < i; ++j) arrpush(arr,j);
    //   arrfree(arr);
    // }
    let mut i: i32 = 0;
    while i < num {
        let mut j: i32 = 0;
        while j < i {
            arr.push(j);
            j += 1;
        }
        // arrfree(arr): the C code frees and resets the array to NULL.
        arr = Vec::new();
        i += 50;
    }
}

fn main() {
    // The C library exposes only `arr_push` and has no `main`. Building
    // the C source as an executable would link with no entry point of
    // its own, so to faithfully reproduce "byte-identical output" for
    // any input we read nothing and write nothing.
    //
    // Reference the translated function so it is not dead-code-eliminated
    // away, but with a value (0) that makes both inner loops execute
    // zero iterations — preserving the no-output behaviour exactly.
    arr_push(0);
}
