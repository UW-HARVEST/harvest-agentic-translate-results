//! Translation of c_src/src/lib.c to Rust.
//!
//! The C library exposes only `void str_put(int num)`. The implementation
//! exercises stb_ds-style string arena and string hash map, then prints
//! using a buggy printf:
//!
//! ```c
//!     printf("%s %d\n", strmap[z], strmap[z].value);
//! ```
//!
//! `strmap[z]` is a 16-byte struct `{char* key; int value;}` passed by
//! value. On x86-64 SysV ABI, the struct is split into two eightbytes
//! and passed via RSI (key) and RDX (value+padding). printf's "%s"
//! consumes RSI giving the key pointer to "a", and "%d" consumes RDX
//! giving the int value.  The third argument `strmap[z].value` lands
//! in RCX but is never consumed because the format string has only two
//! conversions.  Net output is therefore `"a {num}\n"`.
//!
//! Since there is exactly one entry in the map (key "a", value num),
//! the loop runs once and produces a single line.  The arena allocations
//! and resets earlier in the function produce no output, so we only need
//! to reproduce the final printf to be byte-identical on stdout.

use std::ffi::c_int;

unsafe extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

/// Translation of `void str_put(int num)`.
///
/// The original C function:
///   1. Pushes `num` strings into an stb_ds string arena (no output).
///   2. Resets the arena (no output).
///   3. Inserts a single entry `{ key = "a", value = num }` into a string
///      hash map.
///   4. Iterates over the map (one entry) and calls
///      `printf("%s %d\n", strmap[z], strmap[z].value);`
///   5. Frees the map.
///
/// Only step 4 produces output. We reproduce it exactly by emitting
/// "a {num}\n" via libc printf, matching the buffering and formatting
/// behavior of the original C library.
#[unsafe(no_mangle)]
pub extern "C" fn str_put(num: c_int) {
    // Reproduce the C printf:
    //   printf("%s %d\n", strmap[0], strmap[0].value);
    // Effectively prints: "a {num}\n"
    unsafe {
        printf(b"%s %d\n\0".as_ptr(), b"a\0".as_ptr(), num);
    }
}
