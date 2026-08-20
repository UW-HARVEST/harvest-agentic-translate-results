// Rust translation of c_src/src/main.c — program entry point.
//
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
//
// The whole translation lives in `lib.rs` so that the C translation unit's
// externally visible function (`printLine`) can also be exported from a shared
// object, exactly like the C source provides it.

/// Rust's runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, which a C
/// program's `main` never does — it inherits the default disposition. Without
/// restoring it, a failed write to a pipe with no reader would let this program
/// exit 0 where the C program dies from `SIGPIPE` (status 141).
///
/// This belongs to the *program* entry point, not to the library: loading a C
/// shared object does not change a process's signal dispositions either.
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
    let status = driver::run();
    // Returning 0 from C's `main` runs `exit`, whose stream cleanup flushes
    // `stdout` and rewinds a seekable `stdin` to its logical position.
    driver::cleanup_streams();
    std::process::exit(status);
}
