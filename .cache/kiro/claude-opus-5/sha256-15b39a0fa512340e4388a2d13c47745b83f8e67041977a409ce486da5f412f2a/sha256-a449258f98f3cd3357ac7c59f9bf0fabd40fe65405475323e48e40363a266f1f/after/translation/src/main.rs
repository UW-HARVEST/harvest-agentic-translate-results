// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Translation of `c_src/src/main.c`.
//!
//! The C entry point simply forwards the return value of `helloworld()` as the
//! process exit status:
//!
//! ```c
//! int main() {
//!     return helloworld();
//! }
//! ```

mod sillymain;

use sillymain::helloworld;

/// Restore the C default disposition for `SIGPIPE`.
///
/// A C program starts with `SIGPIPE` set to `SIG_DFL`, so `printf` to a pipe
/// whose reader has gone away terminates the process by signal (wait status
/// `128 + 13 = 141` as reported by a shell). The Rust runtime sets `SIGPIPE` to
/// `SIG_IGN` before `main` runs, which turns the same situation into an ignored
/// `EPIPE` write error and an exit status of 0. Undoing that is required for the
/// exit status to match the C program.
///
/// Declared directly rather than pulling in the `libc` crate so the crate keeps
/// zero dependencies. `SIGPIPE` is 13 and `SIG_DFL` is the null handler on
/// Linux.
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;

    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }

    // Safety: `signal` is being called with a valid signal number and the
    // well-known `SIG_DFL` handler value. Nothing else in this program installs
    // signal handlers, so there is no handler state to clobber.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn main() {
    restore_default_sigpipe();

    // `return helloworld();` from `main` in C sets the process exit status to
    // the (low 8 bits of the) returned value. `std::process::exit` reproduces
    // that. stdout is explicitly flushed inside `helloworld`, because
    // `process::exit` does not run any of Rust's cleanup handlers.
    let status: i32 = helloworld();
    std::process::exit(status);
}
