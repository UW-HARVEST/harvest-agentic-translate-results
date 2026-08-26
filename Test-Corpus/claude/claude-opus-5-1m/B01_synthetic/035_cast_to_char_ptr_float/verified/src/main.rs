// Rust translation of c_src/src/main.c — executable entry point.
//
// The C program is:
//
//     int main() {
//         float x = 0.f;
//         scanf("%f", &x);
//         driver(x);          /* prints the 4 bytes of `x` as lowercase hex */
//         return 0;
//     }
//
// All of the behaviour lives in `src/lib.rs` so that the `driver` symbol can
// also be exported from a `cdylib` and compared against the C `driver`
// through the FFI boundary. This file must stay a byte-for-byte equivalent
// of the C `main`: read one `%f`, ignore the return value, call `driver`,
// exit 0.
//
// With the `c_main` feature the library exports its own C-ABI `main`, so this
// crate must not emit a second one; `#![no_main]` hands the entry point over
// to the library and the resulting program behaves identically.
#![cfg_attr(feature = "c_main", no_main)]

#[cfg(not(feature = "c_main"))]
fn main() {
    driver::run();
}

// Without a `main` of its own the binary crate has no reference into the
// library, so keep the library object (and with it the exported `main`) alive
// for the linker.
#[cfg(feature = "c_main")]
#[used]
static KEEP_LIB_ALIVE: extern "C" fn(f32) = driver::driver;
