//! The original C source (c_src/src/lib.c) is a library — it has no `main`,
//! reads no stdin, and writes no stdout. To preserve byte-identical output for
//! the same inputs, this executable is a no-op: it consumes no input and
//! produces no output, exactly like the C artifact would when invoked with no
//! caller of `gaussian_kernel`.

fn main() {
    // No I/O, matching the C library's behavior.
}
