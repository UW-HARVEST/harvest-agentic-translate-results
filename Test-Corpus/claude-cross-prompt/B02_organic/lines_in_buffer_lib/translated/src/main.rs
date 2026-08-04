//! The original C package (`c_src/`) defines only a shared library (`driver`)
//! built from `src/lib.c`; it does not contain a `main` entry point. The
//! library exposes `UTIL_createLinePointers`, which has been translated in
//! `lib.rs`. Because no C `main` exists, the executable here is a no-op:
//! running it produces no output, matching the (absent) behavior of the C
//! source.

fn main() {
    // No I/O is performed because the C source has no main function.
    // Reference the library symbol so it isn't dead-code eliminated and to
    // make the dependency explicit.
    let _ = translated_rust::util_create_line_pointers(b"", 0);
}
