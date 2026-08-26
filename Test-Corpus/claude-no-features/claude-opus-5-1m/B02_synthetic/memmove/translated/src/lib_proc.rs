//! Translation of c_src/src/lib.c. The semantics are preserved exactly,
//! including any quirks in the original C implementation.

/// Main entrance function - processes buffer based on operation flags.
///
/// `buffer` must be large enough to hold any intermediate growth produced by
/// `compact_runs` (which may double the size when the threshold is small).
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

    if (flags & 0x01) != 0 {
        // Rotate
        // C: `int offset = param1 % (int)length;` — truncated toward zero,
        // which matches Rust's `%` for i32 values.
        let offset_c = param1 % (length as i32);
        if offset_c != 0 {
            rotate_buffer(buffer, length, offset_c);
        }
    }

    if (flags & 0x02) != 0 {
        // Compact runs
        let threshold: u8 = if param1 > 0 && param1 <= 255 {
            param1 as u8
        } else {
            3
        };
        new_len = compact_runs(buffer, new_len, threshold);
    }

    if (flags & 0x04) != 0 {
        // Remove duplicates
        let preserve = param2 != 0;
        new_len = remove_duplicates(buffer, new_len, preserve);
    }

    if (flags & 0x08) != 0 && new_len >= 2 {
        // Interleave
        interleave_halves(buffer, new_len);
    }

    if (flags & 0x10) != 0 && new_len >= 4 {
        // Reverse segments
        let seg_size: usize = if param1 > 0 { param1 as usize } else { 4 };
        if seg_size <= new_len {
            reverse_segments(buffer, new_len, seg_size);
        }
    }

    new_len
}

/// Rotate buffer by `offset` positions. Mirrors the (somewhat inconsistent)
/// behavior of the original C implementation byte-for-byte.
fn rotate_buffer(buf: &mut [u8], len: usize, offset_in: i32) {
    if len <= 1 {
        return;
    }

    // Normalize offset to a non-negative value < len.
    let mut offset_signed = offset_in % (len as i32);
    if offset_signed < 0 {
        offset_signed += len as i32;
    }
    if offset_signed == 0 {
        return;
    }
    let offset = offset_signed as usize;

    // Use a 256-byte temp buffer as in the C code.
    let mut temp = [0u8; 256];
    let chunk = if offset < 256 { offset } else { 256 };

    if offset < len / 2 {
        // Small offset path: a `for (i = 0; i < offset; i += chunk)` loop in C.
        let mut i: usize = 0;
        while i < offset {
            let copy_len = if offset - i < chunk { offset - i } else { chunk };
            // memmove(temp, buf + i, copy_len)
            temp[..copy_len].copy_from_slice(&buf[i..i + copy_len]);
            // memmove(buf + i, buf + offset, len - offset)
            memmove(buf, offset, i, len - offset);
            // memmove(buf + len - offset, temp, copy_len)
            buf[len - offset..len - offset + copy_len].copy_from_slice(&temp[..copy_len]);
            i += chunk;
        }
    } else {
        // Large offset path.
        let shift = len - offset;
        // memmove(temp, buf, shift)
        temp[..shift].copy_from_slice(&buf[..shift]);
        // memmove(buf, buf + shift, offset)
        memmove(buf, shift, 0, offset);
        // memmove(buf + offset, temp, shift)
        buf[offset..offset + shift].copy_from_slice(&temp[..shift]);
    }
}

/// Compact consecutive runs of the same value when run length >= threshold.
fn compact_runs(buf: &mut [u8], mut len: usize, threshold: u8) -> usize {
    let mut read: usize = 0;
    let mut write: usize = 0;

    while read < len {
        let current = buf[read];
        let mut run_len: usize = 1;

        while read + run_len < len && buf[read + run_len] == current {
            run_len += 1;
        }

        if run_len >= threshold as usize {
            // Cap at 255 to fit in a u8 count byte.
            if run_len > 255 {
                run_len = 255;
            }

            buf[write] = current;
            write += 1;
            buf[write] = run_len as u8;
            write += 1;

            if read + run_len < len {
                let remaining = len - (read + run_len);
                // memmove(buf + write, buf + read + run_len, remaining)
                memmove(buf, read + run_len, write, remaining);
            }
            len = write + (len - (read + run_len));
            read = write;
        } else {
            if write != read {
                memmove(buf, read, write, run_len);
            }
            write += run_len;
            read += run_len;
        }
    }

    len
}

/// Remove duplicate values; behavior depends on `preserve_order`.
fn remove_duplicates(buf: &mut [u8], len: usize, preserve_order: bool) -> usize {
    if len <= 1 {
        return len;
    }

    if preserve_order {
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
        let mut seen = [0u8; 256];
        let mut write: usize = 0;

        for i in 0..len {
            let v = buf[i] as usize;
            if seen[v] == 0 {
                seen[v] = 1;
                if write != i {
                    buf.swap(write, i);
                }
                write += 1;
            }
        }
        write
    }
}

/// Interleave first and second halves of the buffer.
fn interleave_halves(buf: &mut [u8], len: usize) {
    if len < 2 {
        return;
    }

    let half = len / 2;
    let odd = len % 2;
    // The C code allocates uint8_t temp[512]; we mirror that.
    let mut temp = [0u8; 512];

    if half <= 256 {
        // memmove(temp, buf, half)
        temp[..half].copy_from_slice(&buf[..half]);

        for i in 0..half {
            // memmove(buf + i*2 + 1, buf + half + i, 1)
            buf[i * 2 + 1] = buf[half + i];
            buf[i * 2] = temp[i];
        }
        if odd != 0 {
            buf[len - 1] = buf[half];
        }
    } else {
        // In-place path for large buffers (unreachable when length <= 256
        // since half <= 128, but kept for parity with the C source).
        for i in 0..half {
            let src = half + i;
            let dst = i * 2 + 1;
            if dst < src {
                let val = buf[src];
                // memmove(buf + dst + 1, buf + dst, src - dst)
                memmove(buf, dst, dst + 1, src - dst);
                buf[dst] = val;
            }
        }
    }
}

/// Reverse the buffer in fixed-size segments, plus any remainder.
fn reverse_segments(buf: &mut [u8], len: usize, seg_size: usize) {
    if seg_size <= 1 || len < seg_size {
        return;
    }

    let num_segments = len / seg_size;
    let remainder = len % seg_size;

    for seg in 0..num_segments {
        let base = seg * seg_size;
        for i in 0..(seg_size / 2) {
            let left = base + i;
            let right = base + seg_size - 1 - i;
            buf.swap(left, right);
        }
    }

    if remainder > 1 {
        let base = num_segments * seg_size;
        for i in 0..(remainder / 2) {
            buf.swap(base + i, base + remainder - 1 - i);
        }
    }
}

/// Replicates `memmove(buf + dst, buf + src, count)` semantics on a single
/// mutable slice. C's `memmove` is defined to handle overlapping regions.
fn memmove(buf: &mut [u8], src: usize, dst: usize, count: usize) {
    if count == 0 || src == dst {
        return;
    }
    if dst < src {
        for i in 0..count {
            buf[dst + i] = buf[src + i];
        }
    } else {
        for i in (0..count).rev() {
            buf[dst + i] = buf[src + i];
        }
    }
}
