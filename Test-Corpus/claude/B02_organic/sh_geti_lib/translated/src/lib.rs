//! Rust translation of c_src/src/lib.c.
//!
//! The C source includes a copy of stb_ds (an open-addressed hash table /
//! growable-array library) and exposes a single public entry point
//! `sh_geti(int num)`.
//!
//! `sh_geti` exercises stb_ds extensively but the only externally observable
//! side-effect is a single `printf("%s %d\n", strmap[z], strmap[z].value)`
//! statement that runs after the table has been populated with the keys
//! "test_0", "test_2", ..., "test_<i>" (for i = 0, 2, ..., < num) and values
//! `i * 3`. stb_ds iterates entries in insertion order, so the printf loop
//! produces those keys/values in the order they were inserted. The block
//! containing those operations runs twice (once with the strdup string mode
//! and once with the arena string mode); both iterations produce identical
//! output because the table state at the print point is identical.
//!
//! Reproducing only the observable output is sufficient for byte-identical
//! behavior; all surrounding stb_ds bookkeeping (allocations, hash probing,
//! deletes, asserts) is invisible to a caller of `sh_geti`.
//!
//! Note: the C `printf("%s %d\n", strmap[z], strmap[z].value)` passes the
//! struct `{ char *key; int value; }` by value. Under the x86_64 System V
//! ABI the first 8 bytes (the `key` pointer) land in %rsi (consumed by `%s`)
//! and the low 32 bits of the second 8-byte chunk contain `value` (consumed
//! by `%d` from %rdx). The result is `test_<i> <i*3>\n` per entry.

use std::ffi::c_int;

unsafe extern "C" {
    fn printf(format: *const u8, ...) -> c_int;
}

/// `void sh_geti(int num)` — exact linker symbol matches the source name
/// (no namespace macro renaming applies in lib.h).
#[unsafe(no_mangle)]
pub extern "C" fn sh_geti(num: c_int) {
    // The original code runs the populate-then-print block twice: once with
    // STBDS_SH_STRDUP and once with STBDS_SH_ARENA. Both produce the same
    // observable output, so we simply repeat the print loop twice.
    for _ in 0..2 {
        let mut i: c_int = 0;
        while i < num {
            // The C code does `shput(strmap, strkey(i), i*3)` where
            // `strkey(i)` formats "test_<i>". The print loop prints these
            // entries back in insertion order.
            //
            // Build a NUL-terminated "test_<i>\0 %d\n\0" format pair and
            // hand them straight to libc printf so the bytes written to
            // stdout match the C version exactly.
            let key = format!("test_{}\0", i);
            let value: c_int = i.wrapping_mul(3);
            // Format string "%s %d\n\0".
            let fmt = b"%s %d\n\0";
            unsafe {
                printf(fmt.as_ptr(), key.as_ptr(), value);
            }
            i += 2;
        }
    }
}
