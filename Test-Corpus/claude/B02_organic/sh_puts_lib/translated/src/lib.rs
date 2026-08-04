// Rust translation of c_src/src/lib.c
//
// The original C function `sh_puts(int num)` performs a sequence of
// stb_ds string-arena allocations and hashmap manipulations. None of
// those operations produce stdout output. The only externally visible
// output comes from this loop, which executes exactly once because
// shlen(strmap) == 1 after a single insertion:
//
//     for (int z=0; z < shlen(strmap); ++z)
//         printf("%s %d\n", strmap[z], strmap[z].value);
//
// The struct `{char *key; int value;}` is passed by value through
// varargs. On standard ABIs (System V AMD64, AArch64 AAPCS, etc.) the
// first 8 bytes (the `key` pointer) and the following `int` value are
// consumed by `%s` and `%d` respectively. The hashmap was loaded with
// `{ key: "a", value: num }`, so the program prints exactly:
//
//     "a <num>\n"
//
// We reproduce this byte-for-byte by calling libc's printf, which
// preserves the same line-buffering and integer formatting as the C
// implementation.

use std::ffi::c_int;

unsafe extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn sh_puts(num: c_int) {
    // Equivalent to printf("%s %d\n", "a", num)
    let fmt = b"%s %d\n\0";
    let key = b"a\0";
    unsafe {
        printf(fmt.as_ptr(), key.as_ptr(), num);
    }
}
