// Rust translation of sh_puts from c_src/src/lib.c
//
// The original C function `sh_puts(int num)` performs three logical steps:
//   1. It allocates `num` strings via the stbds string arena and then resets
//      the arena. Neither of these operations produces any output.
//   2. It creates a single-entry string map with key="a" and value=num and
//      asserts a few invariants. None of these operations produce output.
//   3. It iterates `for z in 0..shlen(strmap)` (which is exactly 1 iteration
//      because exactly one entry was inserted) and calls
//          printf("%s %d\n", strmap[z], strmap[z].value);
//      Note the buggy first argument: `strmap[z]` is the entire
//      `{char *key; int value;}` struct rather than `strmap[z].key`. On the
//      x86_64 SysV ABI such a 16-byte struct is split across two integer
//      registers (key pointer in RSI, value+padding in RDX), so `%s` ends up
//      reading the key pointer ("a") and `%d` reads the value (num). The
//      explicit `strmap[z].value` argument lands in RCX and is ignored by
//      printf since the format string only consumes two specifiers.
//
// As a result, for any input `num`, `sh_puts(num)` prints exactly
//   "a <num>\n"
// to stdout. To preserve byte-identical output we delegate to libc's printf
// with the same format string.

use std::ffi::c_int;

unsafe extern "C" {
    fn printf(format: *const u8, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn sh_puts(num: c_int) {
    // Format string: "%s %d\n\0"
    static FORMAT: &[u8] = b"%s %d\n\0";
    // Key string: "a\0"
    static KEY: &[u8] = b"a\0";

    // Reproduce the original printf call. The byte-identical output for any
    // value of `num` is "a <num>\n".
    unsafe {
        printf(FORMAT.as_ptr(), KEY.as_ptr(), num);
    }
}
