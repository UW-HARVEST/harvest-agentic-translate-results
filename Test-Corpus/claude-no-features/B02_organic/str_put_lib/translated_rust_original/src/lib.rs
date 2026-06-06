// Translation of c_src/src/lib.c to Rust.
//
// The only public symbol of the original C library is `str_put`. After
// reproducing all the side-effects of the original (which are internal to
// the library's heap allocations and ultimately observable only through
// stdout), the function prints exactly one line: `"a {num}\n"`.
//
// In the C source, the loop:
//
//     for (int z=0; z < shlen(strmap); ++z)
//         printf("%s %d\n", strmap[z], strmap[z].value);
//
// passes the entire struct `{ char *key; int value; }` to printf rather
// than `strmap[z].key`. With the System V AMD64 ABI used on Linux, the
// first 8 bytes of the struct (the `key` pointer) end up in RSI which
// `%s` consumes, and the next 4 bytes (the `value` int) end up in RDX
// which `%d` consumes — so the output is identical to passing
// `strmap[z].key` and `strmap[z].value` separately.
//
// Since `shputs` always inserts exactly one entry (`{"a", num}`), the
// loop iterates exactly once and prints `"a {num}\n"`.
//
// To preserve byte-identical output (including stdout buffering behavior),
// we call libc's `printf` directly.

use core::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn str_put(num: c_int) {
    // The body of the original `str_put` consists of:
    //   1. A loop over `stralloc`/`strreset` whose only effect is on
    //      heap-allocated arena memory that is freed before any output.
    //   2. A `shputs` of `{"a", num}` followed by a single iteration that
    //      prints `"a {num}\n"`.
    //
    // Step 1 has no observable side-effects, so it is omitted. Step 2
    // reduces to a single printf call.
    unsafe {
        printf(b"%s %d\n\0".as_ptr() as *const c_char,
               b"a\0".as_ptr() as *const c_char,
               num);
    }
}
