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

//! C-ABI shared-library view of the translated program.
//!
//! It exports exactly the symbols that `gcc -shared c_src/src/main.c` exports
//! (`static_sum` and `main`) so the Rust translation can be differentially
//! tested against the C build through the FFI boundary.

#[path = "logic.rs"]
mod logic;

use std::ffi::c_char;
use std::ffi::c_int;

/// ```c
/// int static_sum(int update);
/// ```
#[no_mangle]
pub extern "C" fn static_sum(update: c_int) -> c_int {
    logic::static_sum(update)
}

/// ```c
/// int main(int argc, char **argv);
/// ```
///
/// # Safety
/// `argv` must be a valid `char **` with at least `argc` entries whenever
/// `argc == 2` (that is the only case in which the C code dereferences it).
#[no_mangle]
pub unsafe extern "C" fn main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    logic::run(argc, || {
        // C: strtol(argv[1], &end, 10)
        let p = *argv.add(1);
        let mut len = 0usize;
        while *p.add(len) != 0 {
            len += 1;
        }
        std::slice::from_raw_parts(p as *const u8, len).to_vec()
    })
}
