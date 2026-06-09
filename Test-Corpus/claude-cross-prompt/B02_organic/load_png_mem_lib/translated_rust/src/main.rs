// The C source provides a library (load_png_mem) with no main(). To produce
// byte-identical output for the same inputs as the original program, the
// executable performs no I/O — matching the behavior of an executable built
// from the library code.

#[allow(unused_imports)]
use translated_rust::*;

fn main() {
    // No output, matching C library behavior (no main in original code).
}
