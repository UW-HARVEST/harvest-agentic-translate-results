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
//! Every routine reproduces the exact behaviour of the original C code,
//! including its quirks (e.g. `rotate_buffer` rotating left for small
//! offsets but right for large ones, and `compact_runs` being able to
//! *grow* the logical length when the threshold is 1 or 2).
//!
//! `memmove` is modelled with `slice::copy_within` (identical overlapping
//! copy semantics) and single byte `memmove`s are plain assignments.

#![forbid(unsafe_code)]

/// Main entrance function - processes buffer based on operation flags
///
/// * `buffer`  - Input/output buffer
/// * `length`  - Buffer length
/// * `flags`   - Bit flags:
///   * bit 0: rotate buffer
///   * bit 1: compact runs
///   * bit 2: remove duplicates
///   * bit 3: interleave halves
///   * bit 4: reverse segments
/// * `param1`  - Operation-specific parameter (rotation offset, run threshold, segment size)
/// * `param2`  - Secondary parameter (preserve order flag, etc)
///
/// Returns the new buffer length after processing.
pub fn process_buffer(
    buffer: &mut [u8],
    length: usize,
    flags: u32,
    param1: i32,
    param2: i32,
) -> usize {
    let mut new_len = length;

    /* `buffer == NULL || length == 0` in the original */
    if buffer.is_empty() || length == 0 {
        return 0;
    }

    /* Branch based on multiple flags - creates diverse control flow */
    if flags & 0x01 != 0 {
        /* Rotate */
        let offset = param1 % (length as i32);
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
fn rotate_buffer(buf: &mut [u8], len: usize, offset: i32) {
    if len <= 1 {
        return;
    }

    /* Normalize offset */
    let mut offset = offset % (len as i32);
    if offset < 0 {
        /* C: `offset += len;` with `len` a size_t, truncated back to int */
        offset = offset.wrapping_add(len as i32);
    }
    if offset == 0 {
        return;
    }

    /* Use reversal algorithm with memmove */
    let mut temp = [0u8; 256];
    let offset = offset as usize;
    let chunk: usize = if offset < 256 { offset } else { 256 };

    if offset < len / 2 {
        /* Small offset: move prefix aside, shift main part, restore prefix */
        let mut i = 0usize;
        while i < offset {
            let copy_len = if offset - i < chunk { offset - i } else { chunk };
            temp[..copy_len].copy_from_slice(&buf[i..i + copy_len]);
            buf.copy_within(offset..offset + (len - offset), i);
            let dst = len - offset;
            buf[dst..dst + copy_len].copy_from_slice(&temp[..copy_len]);
            i += chunk;
        }
    } else {
        /* Large offset: work from the right */
        let shift = len - offset;
        temp[..shift].copy_from_slice(&buf[..shift]);
        buf.copy_within(shift..shift + offset, 0);
        buf[offset..offset + shift].copy_from_slice(&temp[..shift]);
    }
}

/// Compact consecutive runs of same value if run length >= threshold
/// Complex nested loops with multiple data paths
fn compact_runs(buf: &mut [u8], len: usize, threshold: u8) -> usize {
    let mut len = len;
    let mut read = 0usize;
    let mut write = 0usize;

    while read < len {
        let current = buf[read];
        let mut run_len = 1usize;

        /* Count run length */
        while read + run_len < len && buf[read + run_len] == current {
            run_len += 1;
        }

        if run_len >= threshold as usize {
            /* Compact to 2 elements: value, count */
            if run_len > 255 {
                run_len = 255; /* Cap at 255 */
            }

            buf[write] = current;
            write += 1;
            buf[write] = run_len as u8;
            write += 1;

            /* Shift remaining data if needed */
            if read + run_len < len {
                let remaining = len - (read + run_len);
                buf.copy_within((read + run_len)..(read + run_len + remaining), write);
            }
            len = write + (len - (read + run_len));
            read = write;
        } else {
            /* Keep run as-is, but may need to move it */
            if write != read {
                buf.copy_within(read..read + run_len, write);
            }
            write += run_len;
            read += run_len;
        }
    }

    len
}

/// Remove duplicate values - different paths for ordered/unordered
fn remove_duplicates(buf: &mut [u8], len: usize, preserve_order: bool) -> usize {
    if len <= 1 {
        return len;
    }

    if preserve_order {
        /* Preserve order: O(n^2) but maintains sequence */
        let mut write = 1usize;
        for i in 1..len {
            let mut j = 0usize;
            while j < write {
                if buf[i] == buf[j] {
                    break;
                }
                j += 1;
            }
            if j == write {
                if write != i {
                    buf[write] = buf[i];
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
            if seen[buf[i] as usize] == 0 {
                seen[buf[i] as usize] = 1;
                if write != i {
                    /* Swap to front */
                    buf.swap(write, i);
                }
                write += 1;
            }
        }
        write
    }
}

/// Interleave first and second halves of buffer
/// Complex memmove pattern with temporary storage
fn interleave_halves(buf: &mut [u8], len: usize) {
    if len < 2 {
        return;
    }

    let half = len / 2;
    let odd = len % 2;
    let mut temp = [0u8; 512];

    if half <= 256 {
        /* Use temp buffer for small sizes */
        temp[..half].copy_from_slice(&buf[..half]);

        for i in 0..half {
            buf[i * 2 + 1] = buf[half + i];
            buf[i * 2] = temp[i];
        }
        if odd != 0 {
            buf[len - 1] = buf[half];
        }
    } else {
        /* In-place for large buffers - more complex */
        for i in 0..half {
            let src = half + i;
            let dst = i * 2 + 1;
            if dst < src {
                let val = buf[src];
                buf.copy_within(dst..dst + (src - dst), dst + 1);
                buf[dst] = val;
            }
        }
    }
}

/// Reverse buffer in fixed-size segments
/// Nested loops with conditional memmove operations
fn reverse_segments(buf: &mut [u8], len: usize, seg_size: usize) {
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

            buf.swap(left, right);
        }
    }

    /* Handle remainder if exists and is > 1 */
    if remainder > 1 {
        let base = num_segments * seg_size;
        for i in 0..(remainder / 2) {
            buf.swap(base + i, base + remainder - 1 - i);
        }
    }
}
