// Rust translation of c_src/src/driver.c
//
// The original C source is written using ISO 646 digraphs (`%:` for `#`,
// `<%`/`%>` for `{`/`}`) and the alternative operator spellings from
// <iso646.h>: `bitor` is `|` and `compl` is `~`.
//
// Therefore the body
//     int result = x bitor compl y;
// means
//     int result = x | ~y;
//
// Output is produced with the C library's `printf`/`puts` so that the emitted
// bytes and the stdio buffering behaviour match the original exactly:
// `printf("%d", result)` followed by `puts("")` writes the decimal value and
// then a single newline.

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn puts(s: *const c_char) -> c_int;
}

/// void driver(int x, int y);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(x: c_int, y: c_int) {
    let result: c_int = x | !y;

    unsafe {
        printf(c"%d".as_ptr(), result);
        puts(c"".as_ptr());
    }
}
