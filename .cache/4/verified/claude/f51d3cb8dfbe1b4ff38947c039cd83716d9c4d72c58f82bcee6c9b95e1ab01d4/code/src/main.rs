// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
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
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

// The translated logic lives in `hello.rs` so that this executable and the
// exported `main` symbol of the shared library (`lib.rs`) share one
// implementation and cannot drift apart.
mod hello;

use std::process::ExitCode;

/// Restore the signal disposition a C program starts with.
///
/// A C program launched by the shell inherits `SIGPIPE` at `SIG_DFL`, so writing
/// to a pipe whose read end is closed *terminates* the process (wait status
/// 128+13 = 141) and produces no output. Rust's runtime sets `SIGPIPE` to
/// `SIG_IGN` before `main` runs, which would instead turn the failed write into
/// an ignored `EPIPE` and exit status 0 — an observable divergence from
/// `c_src/src/main.c`. Reset it to `SIG_DFL` so the process-level behavior
/// matches the C program exactly.
///
/// This models the C *runtime* environment, not the body of `main`, which is why
/// it lives here and not in `hello::c_main` (the C `main` itself never touches
/// signal dispositions, and neither does the exported `main` in `lib.rs`).
#[cfg(unix)]
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    // SAFETY: `signal` is async-signal-safe here; we are simply restoring the
    // default disposition before any I/O happens.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

fn main() -> ExitCode {
    restore_default_sigpipe();

    // C: printf("Hello World!\n"); return 0;
    let status = hello::c_main();

    ExitCode::from(status as u8)
}
