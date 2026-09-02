//! Translation of `process_negation`.

use core::ffi::c_int;

/// ```c
/// int process_negation(int var1) {
///     int var2;
///     var2 = !!var1;
///     return var2;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn process_negation(var1: c_int) -> c_int {
    let var2: c_int = c_int::from(var1 != 0);
    var2
}
