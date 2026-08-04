// Rust translation of c_src/src/lib.c
//
// The C library exposes a single public function `arr_ins(int num)` declared in
// c_src/include/lib.h. The implementation in lib.c uses the stb_ds dynamic
// array (`arrpush`, `stbds_arrins`, `arrfree`) along with `STBDS_ASSERT`
// (`assert`) on internal invariants. There is no console / file output, no
// global mutable state, and no return value: the only observable behaviour
// for callers is "the function returns" (or aborts via `assert`).
//
// This translation reproduces the same observable behaviour:
//   * Builds an array, pushes 1,2,3,4 four times via the loop body.
//   * Inserts `num` at index i.
//   * Asserts that the inserted value lands at index i and (when i < 4) that
//     the value originally at the tail (4) still appears at index 4.
//   * Frees the array (handled automatically via Vec's Drop).
//
// We use a safe `Vec<c_int>` internally; the FFI surface (signature and
// linker symbol name) matches the C declaration `void arr_ins(int num)`.

use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn arr_ins(num: c_int) {
    for i in 0..5usize {
        let mut arr: Vec<c_int> = Vec::new();

        // arrpush(arr,1); arrpush(arr,2); arrpush(arr,3); arrpush(arr,4);
        arr.push(1);
        arr.push(2);
        arr.push(3);
        arr.push(4);

        // stbds_arrins(arr,i,num);
        // Equivalent to: grow length by 1, memmove tail to make a hole, then
        // store `num` at index i. With Vec::insert this is a single call.
        arr.insert(i, num);

        // STBDS_ASSERT(arr[i] == num);
        assert!(arr[i] == num);

        // if (i < 4) STBDS_ASSERT(arr[4] == 4);
        if i < 4 {
            assert!(arr[4] == 4);
        }

        // arrfree(arr);  -- handled by Vec's Drop at end of scope.
        drop(arr);
    }
}
