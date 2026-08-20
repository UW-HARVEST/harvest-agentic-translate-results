/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Faithful Rust translation of the `c_src` reference library.
//!
//! `c_src/src/lib.c` exports exactly one symbol, `process_decisions`; every
//! other function in that translation unit is `static`.  The `#[no_mangle]`
//! wrapper below reproduces that ABI byte for byte, including the aliasing
//! rewrite of the caller's buffer that `validate_sequence` performs.

pub mod decisions;

use core::ffi::c_int;

/// C ABI entry point, matching
/// `int process_decisions(char *decision_string, size_t length, int operation, int param)`.
///
/// # Safety
///
/// `decision_string` must either be NULL or point to a readable/writable buffer
/// covering every byte the requested `operation` touches, exactly as the C
/// function requires:
///
/// * operation 0 / 1: the first 3 bytes (only when `length >= 3`)
/// * operation 2: the first `min(length, 32)` bytes
/// * operation 3: the first `length` bytes (these are also **written**)
#[no_mangle]
pub unsafe extern "C" fn process_decisions(
    decision_string: *mut core::ffi::c_char,
    length: usize,
    operation: c_int,
    param: c_int,
) -> c_int {
    /* The C code checks the NULL pointer first, before it can dereference
     * anything, and reports the very same -1 for a zero length. */
    if decision_string.is_null() || length == 0 {
        return -1;
    }

    /* Number of bytes the C implementation can actually touch for this
     * operation.  Building the slice with exactly this length keeps the Rust
     * translation from forming a reference over memory the C code would never
     * have read either. */
    let access_len = match operation {
        0 | 1 => {
            if length < 3 {
                0
            } else {
                3
            }
        }
        2 => {
            if length < 32 {
                length
            } else {
                32
            }
        }
        3 => length,
        _ => 0,
    };

    let buffer: &mut [u8] =
        core::slice::from_raw_parts_mut(decision_string as *mut u8, access_len);

    decisions::process_decisions(buffer, length, operation, param)
}
