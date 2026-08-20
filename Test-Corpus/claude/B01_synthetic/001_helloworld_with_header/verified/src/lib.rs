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

//! Shared-object surface of the translation.
//!
//! The C sources are `c_src/src/sillymain.c` (defines `helloworld`) and
//! `c_src/src/main.c` (defines `main`). Compiled together into a shared object
//! they export exactly two dynamic symbols:
//!
//! ```text
//! T helloworld
//! T main
//! ```
//!
//! This module re-exports the Rust translation under those exact names with C
//! ABI/linkage so that an external caller (`dlopen` + `dlsym`) cannot tell the
//! two libraries apart. See `SYMBOLS.md`.

pub mod sillymain;

/// `int helloworld()` from `c_src/src/sillymain.h` / `sillymain.c`.
///
/// Note that the C declaration is an unprototyped (K&R) declarator, so C
/// callers may pass any number of arguments; extra arguments are ignored, which
/// `extern "C"` reproduces on all supported ABIs.
#[no_mangle]
pub extern "C" fn helloworld() -> std::os::raw::c_int {
    sillymain::helloworld()
}

/// `int main()` from `c_src/src/main.c`:
///
/// ```c
/// int main() {
///     return helloworld();
/// }
/// ```
///
/// Present so the shared object exports the same symbol set as the C one. The
/// real process entry point of the `driver` binary lives in `src/main.rs`.
///
/// Suppressed when this crate is compiled as a `#[test]` harness, because there
/// the symbol would collide with the entry point libtest generates. The
/// `cdylib` that the differential tests load is not built with `cfg(test)`, so
/// it always exports `main`.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> std::os::raw::c_int {
    // Mirrors C: the value of the (internal) call is returned unchanged.
    sillymain::helloworld()
}
