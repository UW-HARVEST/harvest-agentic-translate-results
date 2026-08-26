// The C source (c_src/src/lib.c) is a library with no `main` function.
// Building it produces a shared library, not an executable. To preserve the
// "no observable output" behavior of running the C library directly (which
// you can't — it has no entry point), this binary's main does nothing.

mod cp;

fn main() {
    // Intentionally empty: the original C target is a SHARED library.
    let _ = cp::unfilter;
    let _ = cp::cp_inflate;
}
