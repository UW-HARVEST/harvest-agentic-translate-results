// Rust translation of the C library in `c_src/`.
//
// The C sources are written using ISO C trigraph-era *digraphs* and the
// `<iso646.h>` alternative operator spellings, which can obscure what the code
// actually does. Resolving them:
//
//   `%:`  -> `#`        (digraph for the preprocessor introducer)
//   `<%`  -> `{`        (digraph for an opening brace)
//   `%>`  -> `}`        (digraph for a closing brace)
//   `bitor` -> `|`      (from <iso646.h>)
//   `compl` -> `~`      (from <iso646.h>)
//
// So `c_src/src/driver.c` is equivalent to:
//
//   #include "driver.h"
//   #include <stdio.h>
//   #include <iso646.h>
//
//   void driver(int x, int y) {
//       int result = x | ~y;
//       printf("%d", result);
//       puts("");
//   }
//
// and `c_src/include/driver.h` declares exactly `void driver(int x, int y);`
// behind a `DRIVER_H_` include guard. There are no namespace/renaming macros,
// so the single exported linker symbol is `driver`.

use std::ffi::{c_char, c_int};

// Bind directly to the platform C library's stdio routines rather than using
// Rust's `std::io::stdout`. The C library writes through `stdout`'s FILE
// buffer, so reusing the very same buffer keeps the emitted bytes -- and the
// flush/interleaving behaviour -- identical to the original.
extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn puts(s: *const c_char) -> c_int;
}

/// `%d` format string, NUL terminated, matching the C source literal.
const FMT_D: &[u8] = b"%d\0";

/// Empty string literal passed to `puts`, NUL terminated.
const EMPTY: &[u8] = b"\0";

/// Translation of `void driver(int x, int y)`.
///
/// Computes `x | ~y` using wrapping `int` (two's complement 32-bit) semantics,
/// prints it with `%d` and no trailing newline, then emits a bare newline via
/// `puts("")` -- exactly as the C does.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    // `x bitor compl y` == `x | ~y`. Bitwise operations on `int` cannot
    // overflow, so a plain `|` / `!` pair reproduces the C result bit for bit.
    let result: c_int = x | !y;

    unsafe {
        printf(FMT_D.as_ptr() as *const c_char, result);
        puts(EMPTY.as_ptr() as *const c_char);
    }
}
