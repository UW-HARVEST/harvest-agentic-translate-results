//! Translation of `process_negation` from `c_src/src/lib.c`.

use core::ffi::c_int;

/// C: `int process_negation(int var1)`
///
/// The original body is `var2 = !!var1; return var2;` -- the classic
/// "double negation" idiom that normalises any non-zero value to `1`.
#[unsafe(no_mangle)]
pub extern "C" fn process_negation(var1: c_int) -> c_int {
    c_int::from(var1 != 0)
}
