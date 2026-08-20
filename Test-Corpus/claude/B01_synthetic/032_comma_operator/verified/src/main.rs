// Rust translation of c_src/src/main.c (executable entry point).
//
// The C project builds `src/main.c` into the `driver` executable; this binary is
// the behavioural equivalent.  The actual translation lives in `src/imp.rs`,
// which is shared verbatim with the cdylib (`src/lib.rs`) that exposes the
// `driver` / `main` C symbols for differential testing.

#[path = "imp.rs"]
mod imp;

/// Rust's runtime sets `SIGPIPE` to `SIG_IGN` before entering `main`, which a C
/// program compiled from `c_src/src/main.c` does not do: there, writing to a
/// pipe whose reader has gone away kills the process with signal 13.  Restore
/// the default disposition so the executable's observable behaviour (its wait
/// status) is identical to the C one.
///
/// This deliberately lives only in the binary entry point: the `main` symbol
/// exported from the cdylib must *not* touch the signal disposition, because
/// the C shared library does not either — it simply inherits whatever the host
/// process installed.
#[cfg(unix)]
fn restore_default_sigpipe() {
    // `SIGPIPE` is 13 and `SIG_DFL` is 0 on every Unix target.
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
    let code = imp::c_main();
    std::process::exit(code);
}
