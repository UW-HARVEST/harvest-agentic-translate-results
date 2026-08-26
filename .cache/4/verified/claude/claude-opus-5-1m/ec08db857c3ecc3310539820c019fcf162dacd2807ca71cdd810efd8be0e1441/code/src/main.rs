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

//! `driver` executable — the translation of `c_src/src/main.c`'s `main()`.
//!
//! `c_src/CMakeLists.txt` declares `add_executable(driver src/main.c)`, so this
//! binary is the artifact that has to be byte-for-byte compatible with the C
//! build.  The C translation unit gives `run()` external linkage, so this target
//! re-exports it under the C ABI as well, keeping the executables' global symbol
//! tables in step (`nm` shows `T main` and `T run` for both).

use std::os::raw::c_int;

/// `void run(int extra_bedrooms)` — the only non-`static` function of the C
/// translation unit besides `main`, exported here with the same name and ABI.
#[no_mangle]
pub extern "C" fn run(extra_bedrooms: c_int) {
    driver::run_global(extra_bedrooms);
}

fn main() {
    // `int main()` always `return 0;`, which is what an empty Rust `main` does.
    let status = driver::c_main_with(run);
    debug_assert_eq!(status, 0);
}
