// Executable form of the translation of c_src/src/main.c -- the counterpart of
// the CMake `driver` target.
//
// The implementation lives in `imp.rs`, which is shared verbatim with the cdylib
// (`lib.rs`). It is pulled in with `#[path]` rather than by depending on the
// library crate so that the library's `#[no_mangle] extern "C" fn main` export
// cannot collide with this binary's `main` symbol.

// `print_cstr` is only reachable through the cdylib's `printLine` export.
#[allow(dead_code)]
#[path = "imp.rs"]
mod imp;

use imp::Caller;

fn main() {
    // A C program starts with SIGPIPE at its default disposition, so writing to a
    // closed stdout kills it (status 141). Rust's runtime sets SIGPIPE to SIG_IGN
    // before main, which would instead make every write fail silently and let the
    // program exit 0. Restore the C behavior.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let mut io = imp::Io::new();
    let rc = imp::run_main(&mut io, Caller::CMain);

    // C's `return 0` from main; exiting through libc flushes stdout and, for a
    // seekable stdin, repositions the file descriptor to the logical read offset,
    // both of which are observable.
    std::process::exit(rc);
}
