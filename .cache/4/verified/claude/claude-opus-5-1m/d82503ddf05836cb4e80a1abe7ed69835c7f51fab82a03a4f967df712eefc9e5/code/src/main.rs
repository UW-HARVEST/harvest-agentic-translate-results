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

//! `driver` executable - the artifact `c_src/CMakeLists.txt` builds.

use std::ffi::c_int;
use std::io::Write;

#[path = "prog.rs"]
mod prog;

extern "C" {
    fn signal(sig: c_int, handler: usize) -> usize;
}

const SIGPIPE: c_int = 13;
const SIG_DFL: usize = 0;

/// Undo the one thing Rust's runtime does that a C `main` never sees.
///
/// `std`'s startup code sets `SIGPIPE` to `SIG_IGN` before calling `main`, so a
/// Rust program whose stdout is a pipe with no reader survives the write (it
/// fails with `EPIPE`, which this program discards, and the process exits `0`).
/// The C program keeps the default disposition and is *killed by signal 13*
/// instead - `rc=0 signal=13` vs `rc=0 signal=0` when measured with `waitpid`.
/// Restoring `SIG_DFL` here makes the Rust executable die exactly like the C one.
///
/// Note this belongs to the *executable* only: a shared-library build of
/// `main.c` does not touch the disposition either, so `src/lib.rs`'s exported
/// `main` deliberately leaves whatever the host process installed alone.
fn restore_c_signal_dispositions() {
    // SAFETY: plain libc call with a valid signal number and SIG_DFL.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn main() {
    restore_c_signal_dispositions();

    // `CStdin` rather than `std::io::stdin()`: it reproduces glibc's refill
    // granularity and its one character of push-back, both of which a process
    // sharing fd 0 can observe. See `prog::CStdin`.
    let mut input = prog::CStdin::new();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    // C's `main` returns 0 unconditionally, which is also what falling off the
    // end of Rust's `fn main()` produces.
    let _ = prog::run(&mut input, &mut out);

    let _ = out.flush();

    // libc's exit-time cleanup rewinds a seekable stdin to the logical stream
    // position; do the same before this process goes away.
    input.reposition_if_seekable();
}
