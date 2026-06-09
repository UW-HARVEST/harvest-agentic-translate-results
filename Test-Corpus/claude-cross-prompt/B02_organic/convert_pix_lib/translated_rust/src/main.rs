// The original C source (c_src/src/lib.c) defines a library with no `main`
// and performs no I/O. We expose the translated functions via `lib.rs` and
// provide a minimal binary entry point that mirrors C's no-I/O behavior so
// the project produces a runnable executable.

use translated_rust as lib;

fn main() {
    // The C library has no `main` and no stdin/stdout/stderr behavior, so
    // the corresponding executable performs no I/O and produces no output.
    let _ = lib::cp_fixed_table.len();
}
