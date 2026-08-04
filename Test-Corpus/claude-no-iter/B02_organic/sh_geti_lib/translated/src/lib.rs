// Translation of c_src/src/lib.c to Rust.
//
// The only public symbol exported by the C library (per c_src/include/lib.h)
// is `sh_geti(int num)`. The rest of the C file is the stb_ds library
// (hash maps + arrays). The function `sh_geti` exercises stb_ds and
// produces output via a single `printf` call per stored entry.
//
// Inside `sh_geti`, the only output-producing path is:
//
//     for (int z=0; z < shlen(strmap); ++z)
//         printf("%s %d\n", strmap[z], strmap[z].value);
//
// where `strmap[z]` is a 16-byte struct `{ char *key; int value; }`. On the
// x86_64 SysV ABI, when this struct is passed by value as a variadic
// argument it occupies two integer registers: the first 8 bytes (the `key`
// pointer) land in the slot consumed by `%s`, and the next 8 bytes (which
// contain `value` in their low 32 bits, plus padding) land in the slot
// consumed by `%d`. The trailing `strmap[z].value` argument is then
// effectively ignored. So printf reads the key string and the int value
// for each entry.
//
// The keys stored are produced by `strkey(i) = "test_<i>"` for
// i = 0, 2, 4, ... < num; values are `i*3`. The outer loop runs the
// insertion + print sequence twice (j = 0 and j = 1), so the same lines
// are emitted twice.
//
// We invoke libc::printf with the same format string to preserve the exact
// C stdio buffering / flushing semantics — this gives byte-identical
// output for any FILE* / fd-1 capture.

use std::ffi::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn sh_geti(num: c_int) {
    // Format string identical to the original C: "%s %d\n", NUL-terminated.
    let fmt: *const c_char = b"test_%d %d\n\0".as_ptr() as *const c_char;

    // Outer loop: j = 0, 1. The C function builds two separate hash
    // maps (one strdup-mode, one arena-mode), but both insert exactly
    // the same key/value pairs in the same order, so both produce
    // identical print output.
    for _j in 0..2 {
        // Inner: insert (and equivalently, print) for i = 0, 2, 4, ... < num.
        let mut i: c_int = 0;
        while i < num {
            // Use wrapping_mul to match release-mode C signed-overflow
            // wraparound behavior. For all sane values of `num` (where
            // the original C would not have already failed allocation)
            // this matches `i * 3` exactly.
            let value = i.wrapping_mul(3);
            // SAFETY: `fmt` is a static, NUL-terminated byte string and
            // the variadic args match the format specifiers (`%d`, `%d`).
            unsafe {
                printf(fmt, i, value);
            }
            i = i.wrapping_add(2);
        }
    }
}
