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

mod sillymain;

use sillymain::helloworld;

/// Restores the default disposition of `SIGPIPE`.
///
/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, which a C
/// program started from a shell does not do. Without this, writing to a pipe
/// whose reader has hung up makes the write fail with `EPIPE` and this program
/// exit 0, whereas the C program is killed by signal 13. Resetting the handler
/// to `SIG_DFL` makes the Rust binary's exit status match the C binary's.
fn restore_default_sigpipe() {
    // `signal(2)`; declared directly to avoid pulling in a dependency.
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

/// Mirrors `int main() { return helloworld(); }` — the value returned by
/// `helloworld` becomes the process exit status.
fn main() {
    restore_default_sigpipe();
    std::process::exit(helloworld());
}
