// Translated from c_src/src/main.c -- executable entry point.
//
// The translation itself lives in `translated.rs`. It is pulled in here with
// `#[path]` (rather than through the library crate) so that the binary never
// links the library's `#[no_mangle] extern "C" fn main` wrapper, which would
// collide with the `main` symbol rustc generates for this binary.

#[path = "translated.rs"]
mod translated;

/// Undo the `SIGPIPE` disposition that the Rust runtime installs before `main`.
///
/// Rust's `std` sets `SIGPIPE` to `SIG_IGN` during startup; a C program never
/// does. `c_src/src/main.c` therefore runs with the inherited default
/// disposition, so when its `printf` output goes to a pipe with no reader the
/// process is *killed by SIGPIPE* (wait status "signal 13") rather than getting
/// an `EPIPE` error and exiting 0. Restoring `SIG_DFL` as the first thing `main`
/// does makes the translated executable terminate identically.
///
/// This belongs to the executable only: when `c_src/src/main.c` is compiled as
/// a shared object, its `main` does not touch the signal disposition either, so
/// the `#[no_mangle] extern "C" fn main` wrapper in `lib.rs` must leave whatever
/// the host process configured untouched.
fn restore_default_sigpipe() {
    const SIGPIPE: std::ffi::c_int = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: std::ffi::c_int, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn main() {
    restore_default_sigpipe();

    // C: `int main() { ...; return 0; }`
    let code = translated::c_main();
    std::process::exit(code);
}
