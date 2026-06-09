// The original C code (c_src/src/lib.c) is a library with no `main` and no I/O.
// To satisfy the executable requirement we provide an empty entry point that
// produces no output, matching the absence of any printf/stdin usage in C.

use translated_rust as _;

fn main() {}
