// The original C code in `c_src/` provides only a library (no `main` and no
// I/O). To satisfy the requirement of producing an executable, we expose a
// minimal binary that performs no I/O — matching the original program's
// (empty) stdout output exactly.

fn main() {
    // The C library has no `main` and produces no output on its own. To
    // remain byte-identical, this binary intentionally writes nothing.
}
