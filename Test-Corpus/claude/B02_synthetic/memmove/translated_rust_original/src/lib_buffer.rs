// Translated from c_src/src/lib.c — preserves the original behavior exactly,
// including any quirks present in the C code.

/// Process buffer based on operation flags. Mirrors `process_buffer` in lib.c.
pub fn process_buffer(
    buffer: &mut [u8],
    length: usize,
    flags: u32,
    param1: i32,
    param2: i32,
) -> usize {
    let mut new_len = length;

    // The C code checks `buffer == NULL || length == 0`. In Rust, `buffer` is a
    // slice and is never null; we still preserve the length-zero check.
    if length == 0 {
        return 0;
    }

    if flags & 0x01 != 0 {
        // Rotate
        // C: int offset = param1 % (int)length;
        let offset = param1 % (length as i32);
        if offset != 0 {
            rotate_buffer(buffer, length, offset);
        }
    }

    if flags & 0x02 != 0 {
        // Compact runs
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

    if (flags & 0x08) != 0 && new_len >= 2 {
        interleave_halves(buffer, new_len);
    }

    if (flags & 0x10) != 0 && new_len >= 4 {
        let seg_size: usize = if param1 > 0 { param1 as usize } else { 4 };
        if seg_size <= new_len {
            reverse_segments(buffer, new_len, seg_size);
        }
    }

    new_len
}

/// Rotate buffer by offset positions. Reproduces the C implementation exactly,
/// including its differing behavior between the small- and large-offset paths.
fn rotate_buffer(buf: &mut [u8], len: usize, offset: i32) {
    if len <= 1 {
        return;
    }

    // Normalize offset
    let mut offset = offset % (len as i32);
    if offset < 0 {
        offset += len as i32;
    }
    if offset == 0 {
        return;
    }
    let offset = offset as usize;

    // Use reversal algorithm with memmove
    let mut temp = [0u8; 256];
    let chunk = if offset < 256 { offset } else { 256 };

    if offset < len / 2 {
        // Small offset: move prefix aside, shift main part, restore prefix
        let mut i: usize = 0;
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

/// Compact consecutive runs of the same value when run length >= threshold.
fn compact_runs(buf: &mut [u8], len: usize, threshold: u8) -> usize {
    let mut len = len;
    let mut read: usize = 0;
    let mut write: usize = 0;

    while read < len {
        let current = buf[read];
        let mut run_len: usize = 1;

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
                // memmove(buf + write, buf + read + run_len, remaining)
                buf.copy_within(read + run_len..read + run_len + remaining, write);
            }

            len = write + (len - (read + run_len));
            read = write;
        } else {
            // Keep run as-is, but may need to move it
            if write != read {
                // memmove(buf + write, buf + read, run_len)
                buf.copy_within(read..read + run_len, write);
            }
            write += run_len;
            read += run_len;
        }
    }

    len
}

/// Remove duplicate values; behavior differs based on `preserve_order`.
fn remove_duplicates(buf: &mut [u8], len: usize, preserve_order: bool) -> usize {
    if len <= 1 {
        return len;
    }

    if preserve_order {
        // O(n^2) — preserves original ordering
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
        // Swap-to-front approach using a "seen" lookup
        let mut seen = [0u8; 256];
        let mut write: usize = 0;

        for i in 0..len {
            if seen[buf[i] as usize] == 0 {
                seen[buf[i] as usize] = 1;
                if write != i {
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
        // memmove(temp, buf, half)
        temp[..half].copy_from_slice(&buf[..half]);

        for i in 0..half {
            // memmove(buf + i*2 + 1, buf + half + i, 1)
            // (single-byte copy_within)
            let v = buf[half + i];
            buf[i * 2 + 1] = v;
            buf[i * 2] = temp[i];
        }
        if odd != 0 {
            buf[len - 1] = buf[half];
        }
    } else {
        // In-place for large buffers
        for i in 0..half {
            let src = half + i;
            let dst = i * 2 + 1;
            if dst < src {
                let val = buf[src];
                // memmove(buf + dst + 1, buf + dst, src - dst)
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

        for i in 0..(seg_size / 2) {
            let left = base + i;
            let right = base + seg_size - 1 - i;
            let temp = buf[left];
            buf[left] = buf[right];
            buf[right] = temp;
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
