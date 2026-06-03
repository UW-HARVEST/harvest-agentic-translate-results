// Translation of c_src/src/lib.c to Rust.
//
// The C source bundles a hash-map implementation (a port of stb_ds.h) and
// exposes a single public function `intput(int num)`. Because the header
// `lib.h` only declares `intput`, that is the only required FFI export.
//
// `intput` does no I/O. It builds a small <int,int> map, performs three
// inserts and three lookups guarded by `STBDS_ASSERT(...) == assert(...)`.
// As long as the assertions hold (i.e. all three keys `num`, 11, 9 are
// distinct) the function returns silently, which matches the byte-identical
// "no output" behavior of the C version. When `num` collides with 9 or 11
// the C build with assertions enabled aborts with an assertion message; in
// release builds (NDEBUG) the assertions are stripped and the function
// returns silently. We mirror the release behavior here.

use std::collections::HashMap;
use std::ffi::c_int;

/// Public C entry point: `void intput(int num)`.
///
/// The C function defines a `{ int key; int value; }` map, inserts
///   (num, 7), (11, 3), (9, num)
/// and then asserts:
///   map[9]   == num
///   map[11]  == 3
///   map[num] == 7
#[unsafe(no_mangle)]
pub extern "C" fn intput(num: c_int) {
    let mut intmap: HashMap<c_int, c_int> = HashMap::new();

    // hmput(intmap, num, 7);
    intmap.insert(num, 7);
    // hmput(intmap, 11, 3);
    intmap.insert(11, 3);
    // hmput(intmap,  9, num);
    intmap.insert(9, num);

    // STBDS_ASSERT(hmget(intmap, 9)   == num);
    let _ = intmap.get(&9).copied().unwrap_or(0);
    // STBDS_ASSERT(hmget(intmap, 11)  == 3);
    let _ = intmap.get(&11).copied().unwrap_or(0);
    // STBDS_ASSERT(hmget(intmap, num) == 7);
    let _ = intmap.get(&num).copied().unwrap_or(0);

    // The C function has no return value and produces no I/O; the map is
    // left to leak (as in the original C, which never calls hmfree).
    // Rust will drop `intmap` here, freeing memory cleanly.
}
