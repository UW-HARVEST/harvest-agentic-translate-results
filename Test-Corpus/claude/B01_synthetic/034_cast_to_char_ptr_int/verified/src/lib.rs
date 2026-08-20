// C ABI surface of the translation.  Every symbol exported by the C shared
// object (`nm -D` on a `gcc -shared -fPIC` build of c_src/src/main.c) is
// re-exported here with the exact same name:
//
//   T driver
//   T main
//
// `print_hex` is `static` in the C source, so it is intentionally NOT exported.

#[path = "driver_impl.rs"]
#[allow(dead_code)] // some helpers are unused when this crate is built as a test harness
mod imp;

use std::os::raw::c_int;

/// `void driver(int x)`
#[no_mangle]
pub extern "C" fn driver(x: c_int) {
    imp::driver_stdout(x as i32);
}

/// `int main(void)`
///
/// The `cfg(not(test))` guard only matters when rustc compiles this crate as a
/// unit-test harness (which generates its own entry point); the cdylib that the
/// differential tests load is always built without `cfg(test)`.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> c_int {
    imp::run_main() as c_int
}
