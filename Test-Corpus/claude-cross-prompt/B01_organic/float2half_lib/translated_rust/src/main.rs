// Translation of c_src/src/lib.c
// The original C code defines a library function `float2half` and has no `main`.
// This binary exposes the same function while producing no output by default,
// matching the original library's lack of any side effects.

mod lib_translated;

#[allow(unused_imports)]
use lib_translated::float2half;

fn main() {
    // The original C source is a shared library with no executable entry point.
    // Reading from stdin or producing output would not match the original
    // behavior, so main intentionally does nothing.
}
