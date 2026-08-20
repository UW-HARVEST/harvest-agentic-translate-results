// Binary entry point for the translation of c_src/src/main.c.
//
// The modules are included by path rather than through the library crate so
// that this binary does not link the library's `#[no_mangle] extern "C" fn
// main` export (which would collide with this crate's own `main` symbol).

mod ctype;
mod tables;

// SIGPIPE and SIG_DFL as defined by <signal.h> on Linux.
const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;

extern "C" {
    /// `sighandler_t signal(int signum, sighandler_t handler)`
    fn signal(signum: i32, handler: usize) -> usize;
}

fn main() {
    // The Rust runtime sets SIGPIPE to SIG_IGN before main runs, while a C
    // program starts with the default disposition. Restore it so that writing
    // to a closed pipe terminates the process with SIGPIPE (status 141), as it
    // does for the C program, instead of silently failing.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }

    // int main() { char c = getchar(); driver(c); }
    // Falls off the end of main, i.e. exit status 0.
    let status = ctype::c_main();
    std::process::exit(status);
}
