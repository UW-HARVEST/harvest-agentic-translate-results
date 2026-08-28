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

// Translation of c_src/src/main.c
//
// C original:
//     int main() {
//         printf("Hello World!\n");
//         return 0;
//     }

use std::io::Write;
use std::process::ExitCode;

// The Rust runtime sets SIGPIPE to SIG_IGN before `main` runs, which a C
// program does not do. Without restoring the default disposition, writing to a
// pipe with no reader makes this program ignore the EPIPE error and exit 0,
// whereas the C program is killed by SIGPIPE (shell-visible status 141).
// Restore SIG_DFL so the observable behavior matches the C original.
#[cfg(unix)]
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13; // Linux and macOS
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

fn main() -> ExitCode {
    restore_default_sigpipe();

    // printf("Hello World!\n");
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    // Write raw bytes so the output is byte-identical to C's printf.
    // Write errors are discarded: C's printf/exit-time flush failure likewise
    // does not change the value returned from main.
    let _ = out.write_all(b"Hello World!\n");
    let _ = out.flush();

    // return 0;
    ExitCode::from(0)
}
