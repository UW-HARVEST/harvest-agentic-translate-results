// Safe Rust translation of the relevant portions of c_src/src/lib.c.
//
// The C file is essentially a copy of stb_ds.h together with a single
// exported helper `arr_ins(int num)` that exercises the dynamic-array
// macros (`arrpush`, `arrins`, `arrfree`) and validates expected element
// values via `assert`.
//
// Faithfully reproducing the assertions is the only externally observable
// behavior; in C, a failed assert would `abort()` the process, producing
// the same kind of failure as a Rust panic. For this small `arr_ins`
// routine the assertions are always satisfied for any `num`, so the
// function simply runs to completion with no output.
//
// The implementation here uses `Vec<i32>` instead of stb_ds's hand-rolled
// growable array, since the only operations used by `arr_ins` are:
//   - push (arrpush)
//   - insert at index (arrins)
//   - free (arrfree)

/// Mirror of `arr_ins(int num)` from c_src/src/lib.c.
///
/// For each `i` in 0..5:
///   1. Push 1, 2, 3, 4 onto the array.
///   2. Insert `num` at position `i`.
///   3. Assert `arr[i] == num`.
///   4. If `i < 4`, assert `arr[4] == 4`.
///   5. Free the array.
pub fn arr_ins(num: i32) {
    for i in 0..5usize {
        let mut arr: Vec<i32> = Vec::new();
        arr.push(1);
        arr.push(2);
        arr.push(3);
        arr.push(4);
        arr.insert(i, num);
        assert_eq!(arr[i], num);
        if i < 4 {
            assert_eq!(arr[4], 4);
        }
        // arrfree(arr) — `arr` is dropped at end of loop iteration.
        drop(arr);
    }
}
