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

//! `#[no_mangle] extern "C"` wrappers that give the Rust cdylib the exact same
//! exported surface a shared-library build of `c_src/src/main.c` has:
//!
//! ```text
//! T bad
//! T good
//! T main
//! T printLine
//! ```
//!
//! Everything writes to / reads from the raw file descriptors 1 and 0 with
//! unbuffered `write(2)` / `read(2)` calls so that the bytes a caller observes
//! do not depend on any hidden Rust-side buffering (C's `printf` is buffered but
//! its buffer is flushed by the caller / at process exit; the resulting byte
//! stream is identical).

#![allow(non_snake_case)] // `printLine` is the C spelling and must be kept.
// Compiling this file as a unit-test harness excludes the `main` export (see the
// bottom of the file), which leaves the stdin side of the translation unreached.
#![cfg_attr(test, allow(dead_code))]

use std::ffi::{c_char, c_int, CStr};
use std::io::Write;

#[path = "prog.rs"]
mod prog;

extern "C" {
    fn write(fd: c_int, buf: *const u8, count: usize) -> isize;
    #[link_name = "__errno_location"]
    fn errno_location() -> *mut c_int;
}

const EINTR: c_int = 4;

fn last_errno() -> c_int {
    unsafe { *errno_location() }
}

/// Unbuffered writer over a raw file descriptor.
struct FdWriter(c_int);

impl Write for FdWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        loop {
            let n = unsafe { write(self.0, buf.as_ptr(), buf.len()) };
            if n >= 0 {
                return Ok(n as usize);
            }
            if last_errno() == EINTR {
                continue;
            }
            return Err(std::io::Error::from_raw_os_error(last_errno()));
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// `const char *` -> optional byte string, `NULL` mapping to `None`.
///
/// # Safety
/// `p` must be NULL or point to a NUL-terminated string, exactly as C's
/// `printLine` requires of its argument.
unsafe fn cstr_bytes<'a>(p: *const c_char) -> Option<&'a [u8]> {
    if p.is_null() {
        None
    } else {
        Some(CStr::from_ptr(p).to_bytes())
    }
}

/// `void printLine(const char *line)`
///
/// # Safety
/// See [`cstr_bytes`].
#[no_mangle]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    let mut out = FdWriter(1);
    prog::print_line(cstr_bytes(line), &mut out);
}

/// `void bad()`
///
/// Reproduces the `bad()` of the *executable* build (see `prog::BAD_DATA`):
/// `printLine` is handed a non-NULL pointer to an empty string.  The C
/// function's own output here is undefined-behaviour garbage that depends on the
/// caller's stack contents, not on the source, so it differs between the
/// executable and the shared-library build of the very same `main.c`.
#[no_mangle]
pub unsafe extern "C" fn bad() {
    let mut out = FdWriter(1);
    prog::bad(&mut out);
}

/// `void good()`
#[no_mangle]
pub unsafe extern "C" fn good() {
    let mut out = FdWriter(1);
    prog::good(&mut out);
}

/// The library's `stdin`, shared by every call to [`main`].
///
/// libc's `stdin` is one process-global `FILE`, so a C caller that invokes the
/// exported `main` twice sees the second conversion continue where the first
/// stopped — including the character the first one pushed back. Measured on the
/// C `.so` with `so_runner <lib> main 3`: `"--5"` yields bad/good/bad and
/// `"5x7"` yields good/bad/bad. A per-call reader would answer good/good and
/// good/bad/good respectively, so this state has to outlive the call.
#[cfg(not(test))]
fn stdin_state() -> &'static std::sync::Mutex<prog::CStdin> {
    static S: std::sync::OnceLock<std::sync::Mutex<prog::CStdin>> = std::sync::OnceLock::new();
    S.get_or_init(|| std::sync::Mutex::new(prog::CStdin::new()))
}

/// `int main()`
///
/// `#[cfg(not(test))]` keeps this out of the way of libtest's own generated
/// `main` when the crate is compiled as a unit-test harness.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn main() -> c_int {
    let mut input = stdin_state()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut out = FdWriter(1);
    let rc = prog::run(&mut *input, &mut out) as c_int;

    // libc rewinds a seekable stdin to the logical stream position when the
    // process exits. Doing it at the end of every call is observationally the
    // same (the next conversion re-reads from that position) and avoids
    // registering an `atexit` handler from a library that may be `dlclose`d.
    input.reposition_if_seekable();
    rc
}
