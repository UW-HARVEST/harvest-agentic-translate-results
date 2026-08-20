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

//! C ABI surface of the translation.
//!
//! `c_src/src/main.c` compiled with `gcc -shared -fPIC` exports exactly two
//! dynamic symbols:
//!
//! ```text
//! T driver
//! T main
//! ```
//!
//! This module re-exports the translated implementations under those same
//! names so that the Rust `cdylib` is a drop-in, `dlopen`-able twin of the C
//! shared object.  It deliberately contains no logic of its own — every
//! behavioural decision lives in `src/main.rs`, which is shared verbatim
//! between the executable and this library.

#![allow(dead_code)]

#[path = "main.rs"]
mod prog;

/// `void driver(int x)` — see `prog::driver`.
///
/// # Safety
/// Plain C ABI function; writes the formatted result to `stdout`.
#[no_mangle]
pub extern "C" fn driver(x: core::ffi::c_int) {
    prog::driver(x as i32);
}

/// `int main(void)` — see `prog::run`.
///
/// Reads one integer from `stdin` with `scanf("%d", &x)` semantics, calls
/// `driver`, and returns `0` exactly like the C `main`.
///
/// Note that this calls [`prog::run`] rather than `prog::main`: the C `main`
/// compiled into a shared object does not modify signal dispositions, so the
/// `SIGPIPE` restoration that the *executable* entry point performs must not
/// happen here.
///
/// # Safety
/// Plain C ABI function; consumes from `stdin` and writes to `stdout`.
///
/// `cfg(not(test))` only because `cargo test --lib` compiles this crate into a
/// test harness that generates its own `main`; the `cdylib` artifact is never
/// built with `--test`, so the export is always present in the real `.so`.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> core::ffi::c_int {
    prog::run();
    0
}
