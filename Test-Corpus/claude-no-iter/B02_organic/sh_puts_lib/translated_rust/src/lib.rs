//! Rust translation of c_src/src/lib.c
//!
//! The original C library exposes a single public function, `sh_puts(int num)`,
//! whose only observable side-effect is a single `printf("%s %d\n", ...)`
//! call on stdout. All of the surrounding stb_ds activity (string-arena
//! allocations, hash-map insertion, asserts, etc.) is internal and produces
//! no output, so byte-identical observable behavior only requires reproducing
//! the printf output.
//!
//! In the original code:
//!     for (int z=0; z < shlen(strmap); ++z)
//!         printf("%s %d\n", strmap[z], strmap[z].value);
//! `strmap` is a hash map containing exactly one entry with key `"a"` and
//! value `num`, and the loop runs exactly once. When the struct
//! `{char* key; int value;}` is passed through `printf`'s varargs, the
//! `char*` is consumed by `%s` (yielding the literal string "a") and the
//! `int` is consumed by `%d` (yielding `num`). The explicit
//! `strmap[z].value` argument is unused. The line printed is therefore
//! always `"a <num>\n"`.

use libc::{c_int, printf};

#[unsafe(no_mangle)]
pub extern "C" fn sh_puts(num: c_int) {
    // The format string and argument list reproduce the exact bytes that the
    // original C `printf("%s %d\n", strmap[z], strmap[z].value)` writes for
    // the single hash-map entry with key "a" and value `num`.
    let fmt = b"%s %d\n\0".as_ptr() as *const libc::c_char;
    let key = b"a\0".as_ptr() as *const libc::c_char;
    unsafe {
        printf(fmt, key, num);
    }
}
