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

//! Direct translation of `c_src/src/lib.c`.
//!
//! Every quirk of the original implementation (including behaviour that looks
//! like a bug, e.g. the two rotation branches rotating in opposite directions,
//! or `interleave_halves` dropping the last element of an odd-length buffer) is
//! reproduced verbatim.
//!
//! Note on buffer sizing: `compact_runs` in the C original can *grow* the
//! logical length (a threshold of 1 turns every single-byte run into a
//! `value, count` pair), which makes the C code write past the end of the
//! caller's 256-byte array. The growth is bounded by `2 * length`, so callers
//! here pass a slice with room for `2 * length` bytes and the extra writes land
//! in real storage instead of clobbering unrelated memory.

/// Main entrance function - processes buffer based on operation flags
///
/// * `buffer` - Input/output buffer
/// * `length` - Buffer length
/// * `flags`  - Bit flags:
///   - bit 0: rotate buffer
///   - bit 1: compact runs
///   - bit 2: remove duplicates
///   - bit 3: interleave halves
///   - bit 4: reverse segments
/// * `param1` - Operation-specific parameter (rotation offset, run threshold, segment size)
/// * `param2` - Secondary parameter (preserve order flag, etc)
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

    // `buffer == NULL` cannot happen with a slice; the `length == 0` half of the
    // original guard is kept, and an empty slice is treated the same way.
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
///
/// (The doc comment above is the original's; the small-offset branch actually
/// rotates left while the large-offset branch rotates right. Preserved as-is.)
fn rotate_buffer(buf: &mut [u8], len: usize, offset_in: i32) {
    if len <= 1 {
        return;
    }

    /* Normalize offset */
    let mut offset = offset_in.wrapping_rem(len as i32);
    if offset < 0 {
        // C: `offset += len;` (unsigned arithmetic truncated back to int)
        offset = (offset as i64 + len as i64) as i32;
    }
    if offset == 0 {
        return;
    }

    /* Use reversal algorithm with memmove */
    let mut temp = [0u8; 256];
    let offset_u = offset as usize;
    let chunk: usize = if offset < 256 { offset as usize } else { 256 };

    if offset_u < len / 2 {
        /* Small offset: move prefix aside, shift main part, restore prefix */
        let mut i: usize = 0;
        while i < offset_u {
            let copy_len = if offset_u - i < chunk {
                offset_u - i
            } else {
                chunk
            };
            temp[..copy_len].copy_from_slice(&buf[i..i + copy_len]);
            buf.copy_within(offset_u..len, i);
            let dst = len - offset_u;
            buf[dst..dst + copy_len].copy_from_slice(&temp[..copy_len]);
            i += chunk;
        }
    } else {
        /* Large offset: work from the right */
        let shift = len - offset_u;
        temp[..shift].copy_from_slice(&buf[..shift]);
        buf.copy_within(shift..shift + offset_u, 0);
        buf[offset_u..offset_u + shift].copy_from_slice(&temp[..shift]);
    }
}

/// Compact consecutive runs of same value if run length >= threshold
/// Complex nested loops with multiple data paths
fn compact_runs(buf: &mut [u8], len_in: usize, threshold: u8) -> usize {
    let mut len = len_in;
    let mut read: usize = 0;
    let mut write: usize = 0;

    while read < len {
        let current = buf[read];
        let mut run_len: usize = 1;

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
                buf.copy_within(read + run_len..read + run_len + remaining, write);
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
        let mut write: usize = 1;
        for i in 1..len {
            let mut j: usize = 0;
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
        let mut write: usize = 0;

        for i in 0..len {
            if seen[buf[i] as usize] == 0 {
                seen[buf[i] as usize] = 1;
                if write != i {
                    /* Swap to front */
                    let temp = buf[write];
                    buf[write] = buf[i];
                    buf[i] = temp;
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
                buf.copy_within(dst..src, dst + 1);
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

            let temp = buf[left];
            buf[left] = buf[right];
            buf[right] = temp;
        }
    }

    /* Handle remainder if exists and is > 1 */
    if remainder > 1 {
        let base = num_segments * seg_size;
        for i in 0..(remainder / 2) {
            let temp = buf[base + i];
            buf[base + i] = buf[base + remainder - 1 - i];
            buf[base + remainder - 1 - i] = temp;
        }
    }
}
