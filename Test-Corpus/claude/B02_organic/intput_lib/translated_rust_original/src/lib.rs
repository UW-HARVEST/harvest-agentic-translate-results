// Translation of c_src/src/lib.c to Rust.
//
// The only public C function is `void intput(int num)` declared in
// c_src/include/lib.h. The bulk of the C file vendors a copy of the
// stb_ds dynamic-array / hash-map library; for this translation we only
// need to reproduce the behaviour observable through `intput`.
//
// Behaviourally, `intput` builds an integer-keyed map, inserts three
// entries (overwriting earlier ones if a key repeats) and then asserts
// that subsequent lookups match the most-recently-written value. Using
// a Rust `HashMap` for the storage gives the same observable behaviour
// (no I/O is performed by the function) while keeping the implementation
// in safe Rust.

use std::collections::HashMap;
use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn intput(num: c_int) {
    // struct { int key; int value; } *intmap = NULL;
    // intmap = NULL;
    let mut intmap: HashMap<c_int, c_int> = HashMap::new();

    // hmput(intmap, num, 7);
    intmap.insert(num, 7);
    // hmput(intmap, 11, 3);
    intmap.insert(11, 3);
    // hmput(intmap,  9, num);
    intmap.insert(9, num);

    // STBDS_ASSERT(hmget(intmap,  9) == num);
    assert!(*intmap.get(&9).unwrap_or(&0) == num);
    // STBDS_ASSERT(hmget(intmap, 11) == 3);
    assert!(*intmap.get(&11).unwrap_or(&0) == 3);
    // STBDS_ASSERT(hmget(intmap, num) == 7);
    assert!(*intmap.get(&num).unwrap_or(&0) == 7);
}
