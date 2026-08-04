// Translation of c_src/src/lib.c to Rust.
// The original C is a library (no main). The accompanying CMakeLists builds it
// as a shared library, and `gjk_cache` performs no I/O. The translated
// executable mirrors that behavior: it produces no output, matching what an
// empty `main` linked against the C library would do.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

mod gjk;

fn main() {
    // The original library has no main and produces no output.
    // We deliberately do nothing here so output is byte-identical (empty).
}
