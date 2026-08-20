// C ABI surface of the translated `c_src/src/main.c`.
//
// Every symbol the C shared object exports is re-exported here under the exact
// same name so that an external consumer (or a differential test harness) can
// `dlopen` either object interchangeably.

use std::os::raw::c_int;

#[path = "imp.rs"]
mod imp;

/// `void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len)`
#[no_mangle]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    imp::fma_array_raw(out, mul1, mul2, add, len)
}

/// `void driver(int *out, int len)`
#[no_mangle]
pub unsafe extern "C" fn driver(out: *mut c_int, len: c_int) {
    imp::driver_raw(out, len)
}

/// `int main()`
///
/// Exported so the shared object's symbol table matches the C one.  It is
/// omitted from `cfg(test)` builds because the test harness supplies its own
/// `main` and the two would collide at link time.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> c_int {
    imp::c_main()
}
