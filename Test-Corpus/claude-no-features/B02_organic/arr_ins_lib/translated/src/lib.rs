// Translation of c_src/src/lib.c to Rust.
//
// The only public C function exposed by the header is `arr_ins(int num)`.
// The rest of the C file consists of stb_ds (https://github.com/nothings/stb)
// internals used to implement a dynamic array. We reproduce the behaviour of
// `arr_ins` faithfully — the function performs no observable I/O, only
// internal assertions, so a correct Rust implementation that exhibits the
// same behaviour produces byte-identical output (i.e., no output at all).

use std::ffi::c_int;

/// Replicates the C `arr_ins(int num)` function from c_src/src/lib.c.
///
/// The original C code, for each `i` in 0..5:
///   - Pushes 1, 2, 3, 4 onto a dynamic array (resulting array: [1, 2, 3, 4]).
///   - Inserts `num` at index `i`.
///   - Asserts that the element at index `i` equals `num`.
///   - If `i < 4`, asserts that the element at index `4` equals `4`.
///   - Frees the dynamic array.
///
/// The function has no return value, no I/O, and no externally visible side
/// effects beyond running these assertions.
#[unsafe(no_mangle)]
pub extern "C" fn arr_ins(num: c_int) {
    for i in 0..5usize {
        // Build the base array [1, 2, 3, 4] via repeated pushes.
        let mut arr: Vec<c_int> = Vec::new();
        arr.push(1);
        arr.push(2);
        arr.push(3);
        arr.push(4);

        // stbds_arrins(arr, i, num): grow by one, shift elements [i..] right
        // by one, then store num at index i. Equivalent to Vec::insert.
        arr.insert(i, num);

        // STBDS_ASSERT(arr[i] == num);
        assert!(arr[i] == num);

        // if (i < 4) STBDS_ASSERT(arr[4] == 4);
        if i < 4 {
            assert!(arr[4] == 4);
        }

        // arrfree(arr); — Vec is dropped automatically at end of scope.
        drop(arr);
    }
}
