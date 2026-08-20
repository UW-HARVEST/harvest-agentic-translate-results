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

//! C ABI surface of the library.
//!
//! `c_src/src/lib.c` exports exactly one symbol (`process_buffer`); everything
//! else in that translation unit is `static`.  This module re-exports the
//! translated implementation under the very same name / signature so that an
//! external consumer cannot tell the two shared objects apart.
//!
//! ```c
//! size_t process_buffer(uint8_t *buffer, size_t length, uint32_t flags,
//!                       int param1, int param2);
//! ```

#![allow(unsafe_code)]

use core::slice;

/// Number of bytes of `buffer` the C implementation may legally touch for a
/// given `length`.
///
/// The C code only ever writes inside `buffer[0 .. length)` **except** in
/// `compact_runs()` (`flags & 0x02`), which can *grow* the logical length: a
/// run shorter than `threshold` is kept verbatim while a run of `n >= threshold`
/// bytes is rewritten as the two bytes `{value, n}`.  With `threshold == 1`
/// every single byte run therefore turns into two bytes, so the logical length
/// can reach - but never exceed - `2 * length`.  (The original C program is
/// happy to run off the end of its own 256 byte stack array in that case; that
/// out-of-bounds write is a bug in the C code which this translation cannot
/// avoid without diverging.)
#[inline]
fn view_len(length: usize, flags: u32) -> usize {
    if flags & 0x02 != 0 {
        length.saturating_mul(2)
    } else {
        length
    }
}

/// `size_t process_buffer(uint8_t *buffer, size_t length, uint32_t flags, int param1, int param2)`
///
/// # Safety
///
/// `buffer` must either be NULL or point to at least [`view_len`] writable
/// bytes, exactly as required by the C implementation.
#[no_mangle]
pub unsafe extern "C" fn process_buffer(
    buffer: *mut u8,
    length: usize,
    flags: u32,
    param1: core::ffi::c_int,
    param2: core::ffi::c_int,
) -> usize {
    /* `if (buffer == NULL || length == 0) return 0;` */
    if buffer.is_null() || length == 0 {
        return 0;
    }

    let buf = slice::from_raw_parts_mut(buffer, view_len(length, flags));
    crate::process_buffer(buf, length, flags, param1 as i32, param2 as i32)
}
