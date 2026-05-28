// Library exports for the translated Rust code, used for FFI testing.
// The implementation uses libc's printf/scanf to mirror C's stdout output
// byte-for-byte.

use std::os::raw::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
    fn __isoc99_scanf(fmt: *const u8, ...) -> c_int;
}

/// Mirrors the C `driver(int x)` function exactly.
///
/// ```c
/// void driver(int x) {
///     for (int i = 0, j = 0; i < x; i++, j += 2) {
///         printf("%d %d\n", i, j);
///     }
/// }
/// ```
#[no_mangle]
pub extern "C" fn driver(x: c_int) {
    let mut i: c_int = 0;
    let mut j: c_int = 0;
    while i < x {
        unsafe {
            printf(b"%d %d\n\0".as_ptr(), i, j);
        }
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
}

/// Mirrors the C `main` function. Reads an integer from stdin and calls driver.
///
/// ```c
/// int main() {
///     int x = 0;
///     scanf("%d", &x);
///     driver(x);
///     return 0;
/// }
/// ```
// Gate the `main` export behind `not(test)`: cargo test compiles the rlib
// alongside the test binary, and Rust auto-generates a `main` for the test
// harness — exporting our own `main` here would clash. The cdylib produced by
// `cargo build` does include this symbol, matching the C .so exports.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> c_int {
    let mut x: c_int = 0;
    unsafe {
        __isoc99_scanf(b"%d\0".as_ptr(), &mut x as *mut c_int);
    }
    driver(x);
    0
}
