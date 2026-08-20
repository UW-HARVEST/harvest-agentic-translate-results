// `driver` executable — mirrors `int main(void)` from c_src/src/main.c.
//
// The whole translation lives in driver_impl.rs, which is also compiled into
// the cdylib (src/lib.rs) so the binary and the exported C ABI symbols share
// one implementation.

#[path = "driver_impl.rs"]
#[allow(dead_code)] // `driver_stdout` is only used by the cdylib's `driver` export
mod imp;

use std::sync::atomic::{AtomicUsize, Ordering};

const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;
const SIG_ERR: usize = usize::MAX; // (sighandler_t)-1

extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
}

/// The `SIGPIPE` disposition this process was started with.
///
/// A C program simply inherits it: started normally it is `SIG_DFL`, so writing
/// to a pipe with no reader kills the process, and started from a parent that
/// ignores `SIGPIPE` it is `SIG_IGN`, so the write fails with `EPIPE` instead.
/// Rust's runtime overwrites it with `SIG_IGN` before calling `main`, which would
/// make the translation survive where the C build dies — so the original value is
/// captured in an ELF constructor (those run before the Rust runtime does) and
/// restored at the top of `main`.
static INHERITED_SIGPIPE: AtomicUsize = AtomicUsize::new(SIG_DFL);

extern "C" fn capture_inherited_sigpipe() {
    // `signal` returns the previous handler, so setting and immediately putting
    // back reads the disposition without changing it.  Nothing can raise SIGPIPE
    // this early, so the momentary change is unobservable.
    unsafe {
        let previous = signal(SIGPIPE, SIG_DFL);
        if previous != SIG_ERR {
            signal(SIGPIPE, previous);
            INHERITED_SIGPIPE.store(previous, Ordering::SeqCst);
        }
    }
}

/// Run `capture_inherited_sigpipe` before `main` (and therefore before the Rust
/// runtime installs its own `SIGPIPE` handler).
#[used]
#[link_section = ".init_array"]
static INIT_SIGPIPE_CAPTURE: extern "C" fn() = capture_inherited_sigpipe;

fn restore_inherited_sigpipe() {
    unsafe {
        signal(SIGPIPE, INHERITED_SIGPIPE.load(Ordering::SeqCst));
    }
}

fn main() {
    restore_inherited_sigpipe();
    // C's `main` returns 0 unconditionally.
    let _ = imp::run_main();
}
