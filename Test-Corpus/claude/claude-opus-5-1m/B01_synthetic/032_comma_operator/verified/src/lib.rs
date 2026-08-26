// C-ABI surface of the translation of c_src/src/main.c.
//
// `nm -D` on the shared library built from the C source exports exactly two
// symbols: `driver` and `main`.  Both are re-exported here with the same names
// and the same C signatures so that an external caller (and the differential
// test suite) cannot tell the two libraries apart.

#[path = "imp.rs"]
pub mod imp;

use std::os::raw::c_int;

/// `void driver(int x)`
#[no_mangle]
pub extern "C" fn driver(x: c_int) {
    imp::driver_stdout(x as i32);
}

/// `int main(void)`
#[no_mangle]
pub extern "C" fn main() -> c_int {
    imp::c_main() as c_int
}
