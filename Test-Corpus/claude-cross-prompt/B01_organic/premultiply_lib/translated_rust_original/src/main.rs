//! Entry point for the translated crate.
//!
//! The original C source (`c_src/src/lib.c`) is a library with no `main`
//! function and performs no I/O. To preserve byte-identical behavior we
//! provide an executable shell that produces no output, exits cleanly, and
//! exposes the translated `premultiply` routine via the library crate.

fn main() {
    // The original C code has no main and produces no output.
    // We reproduce that exactly: do nothing and exit successfully.
}
