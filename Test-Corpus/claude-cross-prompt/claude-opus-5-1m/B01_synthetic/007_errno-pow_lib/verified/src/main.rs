// Translated from c_src/. The original C code (c_src/src/pow.c) defines
// only the `my_pow` library function; it has no `main` of its own.
// To match the behavior of the original (which would produce no output
// when run as a standalone executable), this binary simply exits without
// producing output.

mod pow_lib;

#[allow(dead_code)]
pub use pow_lib::my_pow;

fn main() {
    // The original C source compiles to a shared library and has no
    // standalone behavior. Reproduce that here by performing no I/O.
}
