// Rust translation of c_src/src/lib.c
//
// The original C library exposes a single public function `str_dups(int num)`.
// Internally it exercises a stb_ds string arena (`stralloc`/`strreset`) and a
// string-keyed hash map (`sh_new_strdup` / `shputs` / `shlen` / `shfree`).
//
// The hash map ends up holding exactly one entry `{ key: "a", value: num }`,
// after which the loop prints it with:
//
//     printf("%s %d\n", strmap[z], strmap[z].value);
//
// Note that `strmap[z]` is the *struct* `{ char *key; int value; }`, not a
// string pointer.  Under the System V AMD64 ABI used by Linux x86_64, that
// 12-byte aggregate is passed in two integer registers when used as a
// variadic argument: the first 8 bytes (the `char *key` pointer to "a") go
// into the first integer slot, and the next 4 bytes (the `int value`) go into
// the second integer slot.  `printf` therefore reads "a" for `%s` and `num`
// for `%d`, producing `"a <num>\n"`.
//
// The string-arena and hash-map work that precedes the print performs no I/O,
// so producing byte-identical output only requires emitting that single
// printf call.  We do this by calling libc's `printf` directly so that
// stdio buffering, line-ending conventions, and locale handling all match
// the original C library exactly.

use std::ffi::c_int;
use std::os::raw::c_char;

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Mirror of the C function `void str_dups(int num)` from `c_src/include/lib.h`.
///
/// The header has no namespace macro, so the linker symbol stays `str_dups`.
#[unsafe(no_mangle)]
pub extern "C" fn str_dups(num: c_int) {
    // The original C exercises the stb_ds string arena and string-keyed
    // hash-map facilities.  None of those operations produce output (the
    // assertions are debug-only under typical builds and never fail here),
    // so we replicate only the single user-visible side effect: the
    // `printf` inside the `for (int z=0; z < shlen(strmap); ++z)` loop,
    // which iterates exactly once because exactly one entry is inserted.
    //
    // The format string is `"%s %d\n"`; `%s` consumes the `key` pointer
    // ("a"), and `%d` consumes the `value` (num).
    unsafe {
        printf(
            b"%s %d\n\0".as_ptr() as *const c_char,
            b"a\0".as_ptr() as *const c_char,
            num,
        );
    }
}
