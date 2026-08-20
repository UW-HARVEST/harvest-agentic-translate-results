// Translated from c_src/src/main.c
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

mod driver_impl;

use std::process::ExitCode;

extern "C" {
    /// `sighandler_t signal(int signum, sighandler_t handler)`
    fn signal(signum: i32, handler: usize) -> usize;
}

const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;

fn main() -> ExitCode {
    // A C program starts with SIGPIPE at its default disposition (terminate),
    // whereas the Rust runtime installs SIG_IGN before `main`. Restore the C
    // behaviour so that writing to a closed pipe kills the process with
    // SIGPIPE exactly as the C build does.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
    let status = driver_impl::run();
    ExitCode::from(status as u8)
}
