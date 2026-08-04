// The original C source (c_src/src/lib.c) is a shared library with no `main`
// function and produces no stdout output of its own. To match the C build's
// behavior byte-for-byte for the same inputs, this Rust executable is a no-op
// that produces no output.

fn main() {}
