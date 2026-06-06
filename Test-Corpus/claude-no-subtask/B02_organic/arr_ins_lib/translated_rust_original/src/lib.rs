use std::ffi::c_int;

// Translated from c_src/src/lib.c
//
// The header (c_src/include/lib.h) only exposes `arr_ins`, so that is the
// only public C ABI entry point we need to provide.  All of the stbds_*
// machinery in the C source is internal scaffolding that is never reached
// from `arr_ins` (the function operates entirely on a dynamic array of
// `int`s and never touches the hash-map or string-arena code paths).
//
// `arr_ins` produces no observable output: it builds a small array, inserts
// a value, and triggers a few assertions.  The translation reproduces the
// exact semantics of those assertions using a `Vec<c_int>` as the backing
// store, which has the same observable effects (no I/O, just panics on
// assertion failure) for the same inputs.

#[unsafe(no_mangle)]
pub extern "C" fn arr_ins(num: c_int) {
    for i in 0..5usize {
        // `int *arr = NULL;` followed by four `arrpush` calls.
        let mut arr: Vec<c_int> = Vec::new();
        arr.push(1);
        arr.push(2);
        arr.push(3);
        arr.push(4);

        // `stbds_arrins(arr, i, num)` inserts `num` at index `i`,
        // shifting the existing elements to the right.
        arr.insert(i, num);

        // STBDS_ASSERT(arr[i] == num);
        assert!(arr[i] == num);

        // if (i < 4) STBDS_ASSERT(arr[4] == 4);
        if i < 4 {
            assert!(arr[4] == 4);
        }

        // `arrfree(arr)` — Vec is dropped automatically at end of scope.
    }
}
