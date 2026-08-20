// cdylib façade: exports exactly the symbols that `c_src/src/main.c` exports
// when it is compiled into a shared library:
//
//   $ nm -D --defined-only libcdriver.so
//   0000000000001193 T driver
//   00000000000011b8 T main
//
// (`print_hex` is `static` in the C source and therefore not exported.)

#[path = "logic.rs"]
mod logic;

use std::os::raw::c_int;

/// `void driver(int x)`
#[no_mangle]
pub extern "C" fn driver(x: c_int) {
    logic::driver(x as i32);
}

/// `int main()`
///
/// Returning from a `dlsym`'d `main` does not run libc's exit-time cleanup, so
/// (like the C `.so`) stdin is left wherever the buffered reads stopped.
#[no_mangle]
pub extern "C" fn main() -> c_int {
    logic::program_main(false) as c_int
}
