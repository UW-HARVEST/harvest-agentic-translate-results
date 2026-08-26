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
// C-ABI shared-library surface. Exports exactly the symbols the C translation
// unit (c_src/src/main.c) exports when built as a shared object:
//
//     T driver
//     T main
//
// so that an external caller (and the differential test suite) can dlopen
// either library and observe identical behaviour.

#[path = "core.rs"]
// When cargo compiles this lib target as a unit-test harness the exported
// `main` (and with it most of `core_impl`) is cfg'd out; don't warn about it.
#[cfg_attr(test, allow(dead_code))]
mod core_impl;

use std::os::raw::c_char;

/// Reinterpret a NUL-terminated C string as a byte slice that *includes* the
/// terminating NUL, i.e. the exact view the C code has of one of its `char[]`
/// buffers.
///
/// Like the C code, no NULL check is performed: passing NULL dereferences the
/// null pointer, exactly as `strcspn`/`strlen` do in the C original — the
/// process dies with SIGSEGV. `read_volatile` is used for the scan so that the
/// faulting load is emitted verbatim (a plain `*p` deref is turned into a
/// `panic!`/SIGABRT by rustc's debug assertions, which would *not* match the
/// C behaviour).
unsafe fn cstr_with_nul<'a>(p: *const c_char) -> &'a [u8] {
    let bytes = p as *const u8;
    let mut n = 0usize;
    while bytes.add(n).read_volatile() != 0 {
        n += 1;
    }
    std::slice::from_raw_parts(bytes, n + 1)
}

/// C: `void driver(const char *s1, const char *s2)`
///
/// ```c
/// void driver(const char *s1, const char *s2) {
///     printf("%zu\n", strcspn(s1, s2));
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    let a = cstr_with_nul(s1);
    let b = cstr_with_nul(s2);
    core_impl::driver(a, b);
}

/// C: `int main(void)` — reads the two lines from stdin, chops the last byte of
/// each, calls `driver`, returns 0.
/// `cfg(not(test))`: when cargo compiles this lib target as a *unit-test*
/// harness it generates its own entry point, which would clash with this
/// `#[no_mangle] main`. The cdylib artifact (the one under test) is always
/// built without `cfg(test)`, so the export is present there.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> std::os::raw::c_int {
    core_impl::run() as std::os::raw::c_int
}
