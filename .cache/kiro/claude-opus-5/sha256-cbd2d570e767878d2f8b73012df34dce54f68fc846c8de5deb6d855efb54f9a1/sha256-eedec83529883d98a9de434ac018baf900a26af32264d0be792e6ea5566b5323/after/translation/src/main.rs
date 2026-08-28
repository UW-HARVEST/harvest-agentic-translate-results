// Translation of c_src/src/main.c
//
// Original copyright notice from the C source:
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

mod sillymain;

use sillymain::helloworld;

// The Rust runtime sets SIGPIPE to SIG_IGN before calling `main`, which a C
// program does not do. Without restoring the default disposition, a write to a
// pipe whose reader has closed makes this program report an I/O error and exit
// 0, while the C program is killed by signal 13 (shell status 141). Restore
// SIG_DFL so process termination matches the C behavior exactly.
//
// SIGPIPE == 13 and SIG_DFL == 0 on Linux, the platform the C is compared on.
const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;

extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
}

fn restore_default_sigpipe() {
    // Safety: `signal` is the libc entry point we already link against, and
    // installing SIG_DFL for SIGPIPE has no Rust-side invariants to uphold.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn main() {
    restore_default_sigpipe();

    // C: int main() { return helloworld(); }
    // main's return value becomes the process exit status.
    let status = helloworld();
    std::process::exit(status);
}
