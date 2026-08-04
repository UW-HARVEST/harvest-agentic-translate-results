// Translation of c_src/src/lib.c to Rust.
// The original C code is a library (no main/stdin/stdout/printf).
// This executable's main produces no output, matching the C library's
// behavior when no functions are invoked.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

mod c2;

fn main() {
    // Library has no entry point in C. Produce no output.
}
