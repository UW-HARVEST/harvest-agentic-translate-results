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
// Executable entry point: mirrors C `int main(void)` from c_src/src/main.c.

#[path = "core.rs"]
mod core_impl;

const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;

extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
}

/// Rust's runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, while a C
/// program keeps the default disposition. Without restoring the default, the C
/// binary would be killed by `SIGPIPE` (exit status 141) when its stdout pipe
/// has no reader while this binary would silently exit 0 — an observable
/// difference. Restore the C behaviour before any I/O happens.
fn restore_c_signal_defaults() {
    // SAFETY: plain libc call, executed before any thread is spawned.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn main() {
    restore_c_signal_defaults();
    std::process::exit(core_impl::run());
}
