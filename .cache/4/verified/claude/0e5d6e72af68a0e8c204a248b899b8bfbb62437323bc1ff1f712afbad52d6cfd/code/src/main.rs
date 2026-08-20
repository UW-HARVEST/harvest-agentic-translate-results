// Rust translation of c_src/src/luggage.c — executable entry point.
// Original C by Jan Wrobel <wrr@mixedbit.org>
//
// The whole translation lives in `luggage_core`, which is shared with the
// C-ABI shared library target (`src/lib.rs`) so that both artifacts run
// exactly the same code.

mod luggage_core;

use luggage_core::{arg_bytes, luggage_main_impl};

/// A C program starts with `SIGPIPE` at its default disposition (terminate the
/// process), while the Rust runtime sets it to `SIG_IGN` before `main` runs.
/// Restoring the default keeps the observable behaviour identical to the C
/// program's: `./driver - - - - | head -1` is killed by `SIGPIPE` (shell status
/// 141) instead of silently exiting with 0.
#[cfg(unix)]
fn restore_default_sigpipe() {
    // SIGPIPE == 13 and SIG_DFL == 0 on every Unix target.
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

fn main() {
    restore_default_sigpipe();

    // `argc` / `argv` of the C program.
    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let args: Vec<Vec<u8>> = argv.iter().map(arg_bytes).collect();
    luggage_main_impl(&args);
}
