// The original C source under c_src/ defines only a library (lib.c/lib.h)
// without a `main` function. To satisfy the executable build requirement we
// provide an empty `main` that performs no I/O, producing no output — which
// matches what the original C produces when compiled with no entry point of
// its own.

#[allow(unused_imports)]
use ima_parse as _;

fn main() {}
