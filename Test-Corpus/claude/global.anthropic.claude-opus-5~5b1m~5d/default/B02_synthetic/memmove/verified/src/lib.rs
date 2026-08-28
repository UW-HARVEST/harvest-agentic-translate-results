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

//! Faithful Rust translation of `c_src/src/lib.c`.
//!
//! The buffer is modelled as a mutable byte slice plus an explicit logical
//! length, exactly like the `uint8_t *buf, size_t len` pairs of the C code.
//! Every operation is reproduced step by step (including the quirks of the
//! original implementation); `memmove` becomes `copy_within` /
//! `copy_from_slice`, which have the same "as if copied through a temporary"
//! semantics for overlapping ranges.
//!
//! # Why writes are tracked
//!
//! `compact_runs()` can grow the logical length well past the 256 bytes that
//! `main.c` actually reserves (`uint8_t buffer[256]`), so the original program
//! writes past the end of its own array.  Those stray writes are observable:
//! they land on other locals of `main` and, far enough out, on `main`'s return
//! address.  To reproduce that observable behaviour the caller needs to know
//! how far the writes reached, so every store goes through [`Buffer`], which
//! remembers the highest index touched.  See `main.rs` for the frame layout
//! that turns those indices back into observable output.

/// A byte buffer that remembers the highest index ever written.
///
/// Stands in for the bare `uint8_t *` of the C code.
pub struct Buffer<'a> {
    data: &'a mut [u8],
    /// Highest index written so far; `None` when nothing has been written.
    max_written: Option<usize>,
}

impl<'a> Buffer<'a> {
    pub fn new(data: &'a mut [u8]) -> Self {
        Buffer {
            data,
            max_written: None,
        }
    }

    /// Highest index written so far, `None` if no write happened.
    pub fn max_written(&self) -> Option<usize> {
        self.max_written
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Read-only view of the bytes.
    pub fn as_slice(&self) -> &[u8] {
        self.data
    }

    #[inline]
    fn note(&mut self, highest: usize) {
        match self.max_written {
            Some(m) if m >= highest => {}
            _ => self.max_written = Some(highest),
        }
    }

    #[inline]
    pub fn get(&self, i: usize) -> u8 {
        self.data[i]
    }

    /// `buf[i] = v`
    #[inline]
    pub fn set(&mut self, i: usize, v: u8) {
        self.data[i] = v;
        self.note(i);
    }

    /// `memmove(buf + dst, buf + src, n)`
    #[inline]
    fn memmove_within(&mut self, dst: usize, src: usize, n: usize) {
        if n == 0 {
            return;
        }
        self.data.copy_within(src..src + n, dst);
        self.note(dst + n - 1);
    }

    /// `memmove(buf + dst, temp, n)`
    #[inline]
    fn memmove_from(&mut self, dst: usize, temp: &[u8], n: usize) {
        if n == 0 {
            return;
        }
        self.data[dst..dst + n].copy_from_slice(&temp[..n]);
        self.note(dst + n - 1);
    }
}

/// Main entrance function - processes buffer based on operation flags
///
/// * `buffer` - Input/output buffer
/// * `length` - Buffer length
/// * `flags`  - Bit flags:
///              bit 0: rotate buffer
///              bit 1: compact runs
///              bit 2: remove duplicates
///              bit 3: interleave halves
///              bit 4: reverse segments
/// * `param1` - Operation-specific parameter (rotation offset, run threshold, segment size)
/// * `param2` - Secondary parameter (preserve order flag, etc)
///
/// Returns the new buffer length after processing.
pub fn process_buffer(buffer: &mut [u8], length: usize, flags: u32, param1: i32, param2: i32) -> usize {
    let mut buf = Buffer::new(buffer);
    process_buffer_tracked(&mut buf, length, flags, param1, param2)
}

/// Same as [`process_buffer`] but operating on a [`Buffer`] so the caller can
/// inspect how far the writes reached afterwards.
pub fn process_buffer_tracked(
    buffer: &mut Buffer<'_>,
    length: usize,
    flags: u32,
    param1: i32,
    param2: i32,
) -> usize {
    let mut new_len = length;

    /* `buffer == NULL` cannot be expressed with a slice; an empty slice is the
     * closest analogue and is handled together with `length == 0`. */
    if buffer.is_empty() || length == 0 {
        return 0;
    }

    /* Branch based on multiple flags - creates diverse control flow */
    if flags & 0x01 != 0 {
        /* Rotate */
        let offset = param1.wrapping_rem(length as i32);
        if offset != 0 {
            rotate_buffer(buffer, length, offset);
        }
    }

    if flags & 0x02 != 0 {
        /* Compact runs */
        let threshold: u8 = if param1 > 0 && param1 <= 255 {
            param1 as u8
        } else {
            3
        };
        new_len = compact_runs(buffer, new_len, threshold);
    }

    if flags & 0x04 != 0 {
        /* Remove duplicates */
        let preserve = param2 != 0;
        new_len = remove_duplicates(buffer, new_len, preserve);
    }

    /* Conditional chaining based on length */
    if (flags & 0x08 != 0) && new_len >= 2 {
        /* Interleave */
        interleave_halves(buffer, new_len);
    }

    if (flags & 0x10 != 0) && new_len >= 4 {
        /* Reverse segments */
        let seg_size: usize = if param1 > 0 { param1 as usize } else { 4 };
        if seg_size <= new_len {
            reverse_segments(buffer, new_len, seg_size);
        }
    }

    new_len
}

/// Rotate buffer by offset positions (positive = right, negative = left)
/// Uses multiple memmove operations with different patterns
fn rotate_buffer(buf: &mut Buffer<'_>, len: usize, offset: i32) {
    if len <= 1 {
        return;
    }

    /* Normalize offset */
    let mut offset = offset.wrapping_rem(len as i32);
    if offset < 0 {
        offset += len as i32;
    }
    if offset == 0 {
        return;
    }
    let offset = offset as usize;

    /* Use reversal algorithm with memmove */
    let mut temp = [0u8; 256];
    let chunk = if offset < 256 { offset } else { 256 };

    if offset < len / 2 {
        /* Small offset: move prefix aside, shift main part, restore prefix */
        let mut i = 0usize;
        while i < offset {
            let copy_len = if offset - i < chunk { offset - i } else { chunk };
            temp[..copy_len].copy_from_slice(&buf.as_slice()[i..i + copy_len]);
            buf.memmove_within(i, offset, len - offset);
            buf.memmove_from(len - offset, &temp, copy_len);
            i += chunk;
        }
    } else {
        /* Large offset: work from the right */
        let shift = len - offset;
        temp[..shift].copy_from_slice(&buf.as_slice()[..shift]);
        buf.memmove_within(0, shift, offset);
        buf.memmove_from(offset, &temp, shift);
    }
}

/// Compact consecutive runs of same value if run length >= threshold
/// Complex nested loops with multiple data paths
fn compact_runs(buf: &mut Buffer<'_>, len: usize, threshold: u8) -> usize {
    let mut len = len;
    let mut read = 0usize;
    let mut write = 0usize;

    while read < len {
        let current = buf.get(read);
        let mut run_len = 1usize;

        /* Count run length */
        while read + run_len < len && buf.get(read + run_len) == current {
            run_len += 1;
        }

        if run_len >= threshold as usize {
            /* Compact to 2 elements: value, count */
            if run_len > 255 {
                run_len = 255; /* Cap at 255 */
            }

            buf.set(write, current);
            write += 1;
            buf.set(write, run_len as u8);
            write += 1;

            /* Shift remaining data if needed */
            if read + run_len < len {
                let remaining = len - (read + run_len);
                buf.memmove_within(write, read + run_len, remaining);
            }
            len = write + (len - (read + run_len));
            read = write;
        } else {
            /* Keep run as-is, but may need to move it */
            if write != read {
                buf.memmove_within(write, read, run_len);
            }
            write += run_len;
            read += run_len;
        }
    }

    len
}

/// Remove duplicate values - different paths for ordered/unordered
fn remove_duplicates(buf: &mut Buffer<'_>, len: usize, preserve_order: bool) -> usize {
    if len <= 1 {
        return len;
    }

    if preserve_order {
        /* Preserve order: O(n^2) but maintains sequence */
        let mut write = 1usize;
        for i in 1..len {
            let mut j = 0usize;
            while j < write {
                if buf.get(i) == buf.get(j) {
                    break;
                }
                j += 1;
            }
            if j == write {
                if write != i {
                    let v = buf.get(i);
                    buf.set(write, v);
                }
                write += 1;
            }
        }
        write
    } else {
        /* Don't preserve order: sort-like approach with memmove */
        let mut seen = [0u8; 256];
        let mut write = 0usize;

        for i in 0..len {
            if seen[buf.get(i) as usize] == 0 {
                seen[buf.get(i) as usize] = 1;
                if write != i {
                    /* Swap to front */
                    let temp = buf.get(write);
                    let v = buf.get(i);
                    buf.set(write, v);
                    buf.set(i, temp);
                }
                write += 1;
            }
        }
        write
    }
}

/// Interleave first and second halves of buffer
/// Complex memmove pattern with temporary storage
fn interleave_halves(buf: &mut Buffer<'_>, len: usize) {
    if len < 2 {
        return;
    }

    let half = len / 2;
    let odd = len % 2;
    let mut temp = [0u8; 512];

    if half <= 256 {
        /* Use temp buffer for small sizes */
        temp[..half].copy_from_slice(&buf.as_slice()[..half]);

        for i in 0..half {
            let v = buf.get(half + i);
            buf.set(i * 2 + 1, v);
            buf.set(i * 2, temp[i]);
        }
        if odd != 0 {
            let v = buf.get(half);
            buf.set(len - 1, v);
        }
    } else {
        /* In-place for large buffers - more complex */
        for i in 0..half {
            let src = half + i;
            let dst = i * 2 + 1;
            if dst < src {
                let val = buf.get(src);
                buf.memmove_within(dst + 1, dst, src - dst);
                buf.set(dst, val);
            }
        }
    }
}

/// Reverse buffer in fixed-size segments
/// Nested loops with conditional memmove operations
fn reverse_segments(buf: &mut Buffer<'_>, len: usize, seg_size: usize) {
    if seg_size <= 1 || len < seg_size {
        return;
    }

    let num_segments = len / seg_size;
    let remainder = len % seg_size;

    /* Process complete segments */
    for seg in 0..num_segments {
        let base = seg * seg_size;

        /* Reverse within segment using memmove */
        for i in 0..(seg_size / 2) {
            let left = base + i;
            let right = base + seg_size - 1 - i;

            let temp = buf.get(left);
            let v = buf.get(right);
            buf.set(left, v);
            buf.set(right, temp);
        }
    }

    /* Handle remainder if exists and is > 1 */
    if remainder > 1 {
        let base = num_segments * seg_size;
        for i in 0..(remainder / 2) {
            let temp = buf.get(base + i);
            let v = buf.get(base + remainder - 1 - i);
            buf.set(base + i, v);
            buf.set(base + remainder - 1 - i, temp);
        }
    }
}
