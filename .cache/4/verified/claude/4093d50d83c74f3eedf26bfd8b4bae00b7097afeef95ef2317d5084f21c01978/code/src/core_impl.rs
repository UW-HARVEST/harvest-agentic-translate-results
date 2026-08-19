// Translation of c_src/src/main.c to Rust (shared implementation core).
//
// This module is included by BOTH the `driver` binary (src/main.rs) and the
// `driver` cdylib (src/lib.rs) via `#[path = "core_impl.rs"] mod core_impl;`
// so that the FFI-exported symbols and the executable share one single
// implementation and can never drift apart.
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

#![allow(dead_code)]

use std::io::{self, Read, Write};
use std::os::raw::{c_char, c_int};

/// `char in[1000]` in `main`.
pub const BUF_LEN: usize = 1000;

extern "C" {
    /// libc `signal(2)`; used only to restore the default SIGPIPE disposition
    /// that a C program has at startup (see [`main_impl`]).
    fn signal(sig: c_int, handler: usize) -> usize;
}

/// Faithful replica of C's `strchr(s, c)`.
///
/// * Scans forward from `s` for the byte `(char)c`.
/// * Returns a pointer to the first match, or NULL when the terminating NUL is
///   reached without a match.
/// * Note that the equality test happens BEFORE the NUL test, exactly as in
///   `strchr`: therefore `strchr(s, '\0')` returns a pointer to the terminating
///   NUL rather than NULL.
///
/// # Safety
/// `s` must point to a NUL-terminated byte string (same contract as `strchr`).
#[inline]
pub unsafe fn c_strchr(s: *const c_char, c: c_char) -> *const c_char {
    // strchr converts `c` to `char` and compares; comparing the raw bytes is
    // equivalent on every platform where `c_char` is 8 bits.
    let needle = c as u8;
    let mut p = s as *const u8;
    loop {
        // `read_volatile` rather than `*p`: a plain dereference is instrumented
        // with a debug-only "null pointer dereference occurred" check that turns
        // C's SIGSEGV into a Rust abort (SIGABRT). Reading volatile keeps the
        // faulting access itself, so invalid pointers fail exactly like the C
        // original does (`strchr(NULL, c)` -> SIGSEGV) in every build profile.
        let b = std::ptr::read_volatile(p);
        if b == needle {
            return p as *const c_char;
        }
        if b == 0 {
            return std::ptr::null();
        }
        p = p.add(1);
    }
}

/// Equivalent of:
///
/// ```c
/// int foo(const char *in, char c) {
///     int res = 0;
///     for (const char *s = in; s = strchr(s, c); s++) {
///         res++;
///     }
///     return res;
/// }
/// ```
///
/// The loop counts every occurrence of `c` in the NUL-terminated string `in`.
/// After a hit at `p` the walk resumes at `p + 1` (the `s++` step).
///
/// # Safety
/// Same contract as the C function: `in` must be a valid NUL-terminated string.
/// (A NULL pointer, a non-terminated buffer, or `c == 0` are undefined behavior
/// in the C original and are reproduced here by walking memory identically.)
#[inline]
pub unsafe fn foo_impl(input: *const c_char, c: c_char) -> c_int {
    let mut res: c_int = 0;
    let mut s: *const c_char = input;
    loop {
        let p = c_strchr(s, c);
        if p.is_null() {
            return res;
        }
        // res++ (int overflow would be UB in C; wrapping keeps Rust panic-free)
        res = res.wrapping_add(1);
        // s++ in the for-loop increment
        s = p.add(1);
    }
}

/// Equivalent of:
///
/// ```c
/// void driver(const char *in) {
///     printf("A: %d\n", foo(in, 'A'));
///     printf("x: %d\n", foo(in, 'x'));
/// }
/// ```
///
/// # Safety
/// `in` must be a valid NUL-terminated string (see [`foo_impl`]).
pub unsafe fn driver_impl(input: *const c_char) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    // `%d` on a C `int` and Rust's `{}` on `i32` produce identical text for
    // every value, so the emitted bytes match byte-for-byte.
    let _ = write!(out, "A: {}\n", foo_impl(input, b'A' as c_char));
    let _ = write!(out, "x: {}\n", foo_impl(input, b'x' as c_char));
    let _ = out.flush();
}

/// Equivalent of:
///
/// ```c
/// int main() {
///     char in[1000] = "";
///     fread(in, 1, sizeof(in), stdin);
///     driver(in);
///     return 0;
/// }
/// ```
///
/// `char in[1000] = ""` zero-initializes the whole array, `fread` overwrites at
/// most 1000 bytes, and `driver` then treats the array as a C string. The extra
/// guard byte below keeps the pointer walk in bounds for the pathological case
/// where all 1000 bytes read are non-NUL (the C original walks off the end of
/// the array there — undefined behavior; the observed behavior of the compiled
/// C program is to stop right after the array, which the guard byte reproduces).
pub fn main_impl() -> c_int {
    // A C program starts with SIGPIPE at its default disposition, so the C
    // original is killed by SIGPIPE (exit status 128+13) when its `printf`
    // hits a closed stdout. Rust's runtime installs SIG_IGN for SIGPIPE
    // instead, which would make this program silently exit 0 in that case.
    // Restore the C disposition so the observable behavior matches.
    unsafe {
        const SIGPIPE: c_int = 13;
        const SIG_DFL: usize = 0;
        signal(SIGPIPE, SIG_DFL);
    }

    // char in[1000] = "";  -> the entire array is zero-initialized.
    // (+1 guard NUL, never written to, see the doc comment above.)
    let mut buf = [0u8; BUF_LEN + 1];

    // fread(in, 1, sizeof(in), stdin): read up to 1000 raw bytes from stdin.
    // fread does not stop at newlines; it keeps going until the request is
    // satisfied or EOF/error is hit. The return value is ignored by the C code.
    let mut stdin = io::stdin();
    let mut filled = 0usize;
    while filled < BUF_LEN {
        match stdin.read(&mut buf[filled..BUF_LEN]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    unsafe { driver_impl(buf.as_ptr() as *const c_char) }

    0
}
