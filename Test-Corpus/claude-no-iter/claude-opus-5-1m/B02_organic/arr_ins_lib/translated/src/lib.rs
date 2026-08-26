// Rust translation of c_src/src/lib.c
//
// The only public C function exposed through include/lib.h is:
//   void arr_ins(int num);
//
// The body of `arr_ins` exercises the stb_ds dynamic-array helpers
// (arrpush / arrins / arrfree) and uses STBDS_ASSERT (= assert) to
// validate the resulting layout. The function produces no stdout
// output; its observable behaviour is "return without aborting"
// for inputs that satisfy its internal assertions.
//
// We reproduce that behaviour using a `Vec<c_int>` internally,
// which exposes the same insert/push/clear semantics that stb_ds
// implements on top of malloc/realloc.

use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn arr_ins(num: c_int) {
    for i in 0..5usize {
        let mut arr: Vec<c_int> = Vec::new();

        // arrpush(arr,1); arrpush(arr,2); arrpush(arr,3); arrpush(arr,4);
        arr.push(1);
        arr.push(2);
        arr.push(3);
        arr.push(4);

        // stbds_arrins(arr, i, num) -- insert `num` at index `i`,
        // shifting subsequent elements to the right.
        arr.insert(i, num);

        // STBDS_ASSERT(arr[i] == num);
        assert!(arr[i] == num);

        // if (i < 4) STBDS_ASSERT(arr[4] == 4);
        if i < 4 {
            assert!(arr[4] == 4);
        }

        // arrfree(arr); -- Vec drops at end of scope.
        drop(arr);
    }
}
