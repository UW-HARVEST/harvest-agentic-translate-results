// The original C source (c_src/src/lib.c) is a library with no `main`
// (it builds as a SHARED library per CMakeLists.txt). It performs no I/O
// and produces no output. To produce byte-identical output, this binary
// also produces no output.

fn main() {
    // Intentionally empty: the C library has no `main` and produces no output.
}
