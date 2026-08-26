// Rust translation of c_src/src/main.c
//
// Behaviour-preserving port: the same argument validation order, the same
// glibc pseudo-random number stream, the same (wrapping) signed integer
// arithmetic, and the same stdout/stderr text.
//
// The program logic lives in `program.rs`, which is shared verbatim with the
// `libdriver.so` C-ABI shim in `lib.rs` (used by the differential tests).

mod program;
mod rng;
mod strtoul;

use std::os::unix::ffi::OsStrExt;

/// Rust's runtime installs `SIG_IGN` for `SIGPIPE` before `main`, but a C
/// program starts with the inherited (default) disposition. Without this, a
/// `driver <seed>` whose stdout reader has gone away would exit 1 instead of
/// dying from signal 13 the way the C program does.
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn main() {
    restore_default_sigpipe();

    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let argv: Vec<Option<&[u8]>> = args.iter().map(|a| Some(a.as_bytes())).collect();

    // argc == 0 leaves `argv` empty; `program::run` then renders argv[0] the
    // way glibc's printf renders a NULL "%s": "(null)".
    std::process::exit(program::run(&argv));
}
