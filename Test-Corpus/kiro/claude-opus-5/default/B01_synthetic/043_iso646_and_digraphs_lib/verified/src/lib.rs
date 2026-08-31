// Rust translation of c_src/src/driver.c
//
// The original C source is written with digraphs and ISO 646 alternative
// tokens, which the C preprocessor/lexer maps as follows:
//
//   %:      ->  #        (digraph for the stringize/directive introducer)
//   <%  %>  ->  {  }     (digraphs for braces)
//   bitor   ->  |        (<iso646.h> alternative operator)
//   compl   ->  ~        (<iso646.h> alternative operator)
//
// So the body of `driver` is:
//
//   int result = x | ~y;
//   printf("%d", result);
//   puts("");
//
// The header declares `driver` with no namespace/renaming macro, so the final
// linker symbol is simply `driver`.

use std::ffi::c_char;
use std::ffi::c_int;

unsafe extern "C" {
    /// C `printf`, declared with the exact fixed-argument prefix we use.
    #[link_name = "printf"]
    fn c_printf(fmt: *const c_char, ...) -> c_int;

    /// C `puts`.
    #[link_name = "puts"]
    fn c_puts(s: *const c_char) -> c_int;
}

/// Format string `"%d"` as a NUL-terminated byte string.
const FMT_D: [u8; 3] = [b'%', b'd', 0];

/// Empty NUL-terminated byte string, the argument to `puts("")`.
const EMPTY: [u8; 1] = [0];

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    // `x bitor compl y` == `x | ~y`
    let result: c_int = x | !y;

    // Reuse C stdio so that buffering and interleaving with any other C
    // output in the process is byte-for-byte identical to the original.
    unsafe {
        c_printf(FMT_D.as_ptr() as *const c_char, result);
        c_puts(EMPTY.as_ptr() as *const c_char);
    }
}
