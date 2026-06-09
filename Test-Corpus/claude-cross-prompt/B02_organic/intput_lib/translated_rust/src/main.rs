// Translation of c_src/src/lib.c to Rust.
//
// The C source is a library implementation (stb_ds.h-style dynamic arrays
// and hash maps) with a single public function `intput(int num)` declared
// in c_src/include/lib.h. The library has no `main()` and produces no
// output. The CMakeLists.txt builds it as a shared library.
//
// This Rust port preserves the behavior of the public `intput` function
// using a safe Rust hash map and the assertions from the original. Since
// the C program produces no stdout/stderr output, the equivalent Rust
// binary likewise produces no output, achieving byte-identical output.

use std::collections::HashMap;

/// Mirror of the C `intput(int num)` function.
///
/// The original C performs three `hmput` operations on an `int -> int`
/// hash map and then verifies the values via `hmget` + `STBDS_ASSERT`.
/// The function has no observable output; we preserve the assertions.
pub fn intput(num: i32) {
    let mut intmap: HashMap<i32, i32> = HashMap::new();

    // hmput(intmap, num, 7);
    intmap.insert(num, 7);
    // hmput(intmap, 11, 3);
    intmap.insert(11, 3);
    // hmput(intmap,  9, num);
    intmap.insert(9, num);

    // STBDS_ASSERT(hmget(intmap, 9) == num);
    assert_eq!(*intmap.get(&9).unwrap_or(&0), num);
    // STBDS_ASSERT(hmget(intmap, 11) == 3);
    assert_eq!(*intmap.get(&11).unwrap_or(&0), 3);
    // STBDS_ASSERT(hmget(intmap, num) == 7);
    //
    // Note: in the original C, when `num == 11` or `num == 9`, the earlier
    // inserts get overwritten and this assertion would fail. We preserve
    // that behavior exactly (no bug fixes).
    assert_eq!(*intmap.get(&num).unwrap_or(&0), 7);
}

fn main() {
    // The C library has no `main`. The CMakeLists.txt builds a shared
    // library; running it produces no output. To match byte-identical
    // output for the same inputs (none read, none written), main is empty.
}
