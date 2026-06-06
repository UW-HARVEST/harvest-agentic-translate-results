// Translation of c_src/src/lib.c
//
// The original C file contains a large body of `stb_ds`-style internal
// data-structure utilities (dynamic arrays, hash maps, string arena), but the
// only public function declared in `c_src/include/lib.h` is `str_dups`.
//
// Tracing what `str_dups(num)` actually emits:
//   1. A loop that calls `stralloc` `num` times into a string arena.
//      `stralloc` writes nothing to stdout/stderr.
//   2. `strreset` clears the arena (no output).
//   3. A single entry `{"a", num}` is inserted into a string-keyed hashmap
//      that runs in `STBDS_SH_STRDUP` mode (so the key is heap-duplicated).
//   4. `shlen(strmap)` is exactly 1, so the for-loop body runs once and
//      executes:
//          printf("%s %d\n", strmap[z], strmap[z].value);
//      Here `strmap[z]` is a struct `{char *key; int value;}` passed by
//      value.  Under the System V x86-64 ABI such a 16-byte struct is
//      passed in two integer registers, so `%s` consumes the `key` pointer
//      ("a") and `%d` consumes the `value` field (the `num` argument).
//      The trailing `strmap[z].value` argument lands in a further register
//      and is silently ignored by `printf` because the format string has no
//      additional conversions.
//
// Therefore the only byte that ever leaves `str_dups` is:
//      "a <num>\n"
// where <num> is the decimal representation of the function's argument.
//
// We reproduce that behaviour by calling `printf` from libc directly so the
// output goes through the same stdio buffering path as the original C code.

use std::ffi::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn str_dups(num: c_int) {
    // Mirror the C function's only observable side-effect.  All of the
    // arena / hashmap manipulation in the original is internal book-keeping
    // and produces no I/O.
    //
    // The loop in the C source iterates `shlen(strmap)` times, which is
    // always 1 here (a single key "a" is inserted), so we emit the line
    // exactly once regardless of `num`.
    let fmt = b"a %d\n\0";
    unsafe {
        printf(fmt.as_ptr() as *const c_char, num);
    }
}
