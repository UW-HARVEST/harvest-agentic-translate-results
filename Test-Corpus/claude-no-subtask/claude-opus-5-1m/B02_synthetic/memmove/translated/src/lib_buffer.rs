/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Translated to Rust from original C code.
 */

/// Main entrance function - processes buffer based on operation flags.
///
/// Bit flags:
///  - bit 0: rotate buffer
///  - bit 1: compact runs
///  - bit 2: remove duplicates
///  - bit 3: interleave halves
///  - bit 4: reverse segments
pub fn process_buffer(
    buffer: &mut [u8],
    length: usize,
    flags: u32,
    param1: i32,
    param2: i32,
) -> usize {
    let mut new_len = length;

    if buffer.is_empty() || length == 0 {
        return 0;
    }

    // Branch based on multiple flags - creates diverse control flow
    if flags & 0x01 != 0 {
        // Rotate
        // C: int offset = param1 % (int)length;
        let offset = c_rem_i32(param1, length as i32);
        if offset != 0 {
            rotate_buffer(buffer, length, offset);
        }
    }

    if flags & 0x02 != 0 {
        // Compact runs
        // C: uint8_t threshold = (param1 > 0 && param1 <= 255) ? (uint8_t)param1 : 3;
        let threshold: u8 = if param1 > 0 && param1 <= 255 {
            param1 as u8
        } else {
            3
        };
        new_len = compact_runs(buffer, new_len, threshold);
    }

    if flags & 0x04 != 0 {
        // Remove duplicates
        let preserve = param2 != 0;
        new_len = remove_duplicates(buffer, new_len, preserve);
    }

    // Conditional chaining based on length
    if (flags & 0x08 != 0) && new_len >= 2 {
        // Interleave
        interleave_halves(buffer, new_len);
    }

    if (flags & 0x10 != 0) && new_len >= 4 {
        // Reverse segments
        // C: size_t seg_size = (param1 > 0) ? (size_t)param1 : 4;
        let seg_size: usize = if param1 > 0 { param1 as usize } else { 4 };
        if seg_size <= new_len {
            reverse_segments(buffer, new_len, seg_size);
        }
    }

    new_len
}

/// C-style truncated remainder for i32 (matches `a % b` in C for two's complement).
#[inline]
fn c_rem_i32(a: i32, b: i32) -> i32 {
    // Rust's `%` on i32 is also truncated remainder (matches C99+).
    // Using wrapping_rem to handle the edge case of i32::MIN % -1, which
    // would panic in debug mode otherwise.
    a.wrapping_rem(b)
}

/// Rotate buffer by offset positions (positive = right, negative = left).
/// Reproduces the C code's exact algorithm using memmove-equivalent operations.
fn rotate_buffer(buf: &mut [u8], len: usize, mut offset: i32) {
    if len <= 1 {
        return;
    }

    // Normalize offset
    offset = offset.wrapping_rem(len as i32);
    if offset < 0 {
        offset = offset.wrapping_add(len as i32);
    }
    if offset == 0 {
        return;
    }

    let offset = offset as usize;

    let mut temp = [0u8; 256];
    let chunk = if offset < 256 { offset } else { 256 };

    if offset < len / 2 {
        // Small offset: move prefix aside, shift main part, restore prefix
        let mut i = 0usize;
        while i < offset {
            let copy_len = if offset - i < chunk { offset - i } else { chunk };
            // memmove(temp, buf + i, copy_len);
            temp[..copy_len].copy_from_slice(&buf[i..i + copy_len]);
            // memmove(buf + i, buf + offset, len - offset);
            buf.copy_within(offset..offset + (len - offset), i);
            // memmove(buf + len - offset, temp, copy_len);
            buf[len - offset..len - offset + copy_len].copy_from_slice(&temp[..copy_len]);

            i += chunk;
        }
    } else {
        // Large offset: work from the right
        let shift = len - offset;
        // memmove(temp, buf, shift);
        temp[..shift].copy_from_slice(&buf[..shift]);
        // memmove(buf, buf + shift, offset);
        buf.copy_within(shift..shift + offset, 0);
        // memmove(buf + offset, temp, shift);
        buf[offset..offset + shift].copy_from_slice(&temp[..shift]);
    }
}

/// Compact consecutive runs of same value if run length >= threshold.
fn compact_runs(buf: &mut [u8], mut len: usize, threshold: u8) -> usize {
    let mut read = 0usize;
    let mut write = 0usize;

    while read < len {
        let current = buf[read];
        let mut run_len: usize = 1;

        // Count run length
        while read + run_len < len && buf[read + run_len] == current {
            run_len += 1;
        }

        if run_len >= threshold as usize {
            // Compact to 2 elements: value, count
            if run_len > 255 {
                run_len = 255;
            }

            buf[write] = current;
            write += 1;
            buf[write] = run_len as u8;
            write += 1;

            // Shift remaining data if needed
            if read + run_len < len {
                let remaining = len - (read + run_len);
                // memmove(buf + write, buf + read + run_len, remaining);
                buf.copy_within(read + run_len..read + run_len + remaining, write);
            }
            len = write + (len - (read + run_len));
            read = write;
        } else {
            // Keep run as-is, but may need to move it
            if write != read {
                // memmove(buf + write, buf + read, run_len);
                buf.copy_within(read..read + run_len, write);
            }
            write += run_len;
            read += run_len;
        }
    }

    len
}

/// Remove duplicate values - different paths for ordered/unordered.
fn remove_duplicates(buf: &mut [u8], len: usize, preserve_order: bool) -> usize {
    if len <= 1 {
        return len;
    }

    if preserve_order {
        // Preserve order: O(n^2) but maintains sequence
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
        // Don't preserve order: sort-like approach with swaps
        let mut seen = [0u8; 256];
        let mut write: usize = 0;

        for i in 0..len {
            let v = buf[i] as usize;
            if seen[v] == 0 {
                seen[v] = 1;
                if write != i {
                    // Swap to front
                    buf.swap(write, i);
                }
                write += 1;
            }
        }
        write
    }
}

/// Interleave first and second halves of buffer.
fn interleave_halves(buf: &mut [u8], len: usize) {
    if len < 2 {
        return;
    }

    let half = len / 2;
    let odd = len % 2;
    let mut temp = [0u8; 512];

    if half <= 256 {
        // Use temp buffer for small sizes
        // memmove(temp, buf, half);
        temp[..half].copy_from_slice(&buf[..half]);

        for i in 0..half {
            // memmove(buf + i*2 + 1, buf + half + i, 1);
            buf[i * 2 + 1] = buf[half + i];
            buf[i * 2] = temp[i];
        }
        if odd != 0 {
            buf[len - 1] = buf[half];
        }
    } else {
        // In-place for large buffers - more complex
        for i in 0..half {
            let src = half + i;
            let dst = i * 2 + 1;
            if dst < src {
                let val = buf[src];
                // memmove(buf + dst + 1, buf + dst, src - dst);
                buf.copy_within(dst..dst + (src - dst), dst + 1);
                buf[dst] = val;
            }
        }
    }
}

/// Reverse buffer in fixed-size segments.
fn reverse_segments(buf: &mut [u8], len: usize, seg_size: usize) {
    if seg_size <= 1 || len < seg_size {
        return;
    }

    let num_segments = len / seg_size;
    let remainder = len % seg_size;

    // Process complete segments
    for seg in 0..num_segments {
        let base = seg * seg_size;

        // Reverse within segment
        for i in 0..(seg_size / 2) {
            let left = base + i;
            let right = base + seg_size - 1 - i;
            buf.swap(left, right);
        }
    }

    // Handle remainder if exists and is > 1
    if remainder > 1 {
        let base = num_segments * seg_size;
        for i in 0..(remainder / 2) {
            let temp = buf[base + i];
            buf[base + i] = buf[base + remainder - 1 - i];
            buf[base + remainder - 1 - i] = temp;
        }
    }
}
