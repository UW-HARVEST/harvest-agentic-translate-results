// Translation of c_src/src/lib.c

/// Main entrance function - processes buffer based on operation flags
pub fn process_buffer(
    buffer: &mut [u8],
    length: usize,
    flags: u32,
    param1: i32,
    param2: i32,
) -> usize {
    let mut new_len = length;

    // C: if (buffer == NULL || length == 0) return 0;
    if length == 0 {
        return 0;
    }

    // Rotate
    if (flags & 0x01) != 0 {
        // int offset = param1 % (int)length;
        let offset = c_mod_i32(param1, length as i32);
        if offset != 0 {
            rotate_buffer(buffer, length, offset);
        }
    }

    // Compact runs
    if (flags & 0x02) != 0 {
        let threshold: u8 = if param1 > 0 && param1 <= 255 {
            param1 as u8
        } else {
            3
        };
        new_len = compact_runs(buffer, new_len, threshold);
    }

    // Remove duplicates
    if (flags & 0x04) != 0 {
        let preserve = param2 != 0;
        new_len = remove_duplicates(buffer, new_len, preserve);
    }

    // Interleave halves
    if (flags & 0x08) != 0 && new_len >= 2 {
        interleave_halves(buffer, new_len);
    }

    // Reverse segments
    if (flags & 0x10) != 0 && new_len >= 4 {
        let seg_size: usize = if param1 > 0 { param1 as usize } else { 4 };
        if seg_size <= new_len {
            reverse_segments(buffer, new_len, seg_size);
        }
    }

    new_len
}

/// C-style truncated integer remainder (matches C's `%` for i32).
fn c_mod_i32(a: i32, b: i32) -> i32 {
    // Rust's % matches C's truncated modulo for signed integers,
    // except for overflow corner case (i32::MIN % -1) which is UB in C anyway.
    // Use wrapping_rem to avoid panic on that pathological case.
    if b == 0 {
        // Mirror C undefined behavior; producing 0 is a reasonable choice.
        return 0;
    }
    a.wrapping_rem(b)
}

/// Rotate buffer by offset positions (positive = right, negative = left).
/// Mirrors the C implementation's exact memmove sequence.
fn rotate_buffer(buf: &mut [u8], len: usize, offset_in: i32) {
    if len <= 1 {
        return;
    }

    // Normalize offset: offset = offset % (int)len; if (offset < 0) offset += len;
    let mut offset_i = c_mod_i32(offset_in, len as i32);
    if offset_i < 0 {
        offset_i += len as i32;
    }
    if offset_i == 0 {
        return;
    }
    let offset: usize = offset_i as usize;

    let mut temp: [u8; 256] = [0; 256];
    let chunk: usize = if offset < 256 { offset } else { 256 };

    if offset < len / 2 {
        // Small offset path
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
        // Large offset path
        let shift = len - offset;
        // memmove(temp, buf, shift);
        temp[..shift].copy_from_slice(&buf[..shift]);
        // memmove(buf, buf + shift, offset);
        buf.copy_within(shift..shift + offset, 0);
        // memmove(buf + offset, temp, shift);
        buf[offset..offset + shift].copy_from_slice(&temp[..shift]);
    }
}

/// Compact consecutive runs of same value if run length >= threshold
fn compact_runs(buf: &mut [u8], len_in: usize, threshold: u8) -> usize {
    let mut len = len_in;
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

            if read + run_len < len {
                let remaining = len - (read + run_len);
                // memmove(buf + write, buf + read + run_len, remaining);
                let src_start = read + run_len;
                buf.copy_within(src_start..src_start + remaining, write);
            }
            len = write + (len - (read + run_len));
            read = write;
        } else {
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

/// Remove duplicate values - different paths for ordered/unordered
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
        let mut seen: [u8; 256] = [0; 256];
        let mut write: usize = 0;

        for i in 0..len {
            let val = buf[i];
            if seen[val as usize] == 0 {
                seen[val as usize] = 1;
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

/// Interleave first and second halves of buffer
fn interleave_halves(buf: &mut [u8], len: usize) {
    if len < 2 {
        return;
    }

    let half = len / 2;
    let odd = len % 2;
    let mut temp: [u8; 512] = [0; 512];

    if half <= 256 {
        // Use temp buffer for small sizes
        // memmove(temp, buf, half);
        temp[..half].copy_from_slice(&buf[..half]);

        // for i in 0..half:
        //   memmove(buf + i*2 + 1, buf + half + i, 1);
        //   buf[i*2] = temp[i];
        for i in 0..half {
            let src = half + i;
            let dst = i * 2 + 1;
            // single-byte memmove
            buf[dst] = buf[src];
            buf[i * 2] = temp[i];
        }
        if odd != 0 {
            buf[len - 1] = buf[half];
        }
    } else {
        // In-place path for large buffers
        for i in 0..half {
            let src = half + i;
            let dst = i * 2 + 1;
            if dst < src {
                let val = buf[src];
                // memmove(buf + dst + 1, buf + dst, src - dst);
                buf.copy_within(dst..src, dst + 1);
                buf[dst] = val;
            }
        }
    }
}

/// Reverse buffer in fixed-size segments
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

            // temp = buf[left];
            // memmove(buf + left, buf + right, 1);
            // memmove(buf + right, &temp, 1);
            let temp = buf[left];
            buf[left] = buf[right];
            buf[right] = temp;
        }
    }

    if remainder > 1 {
        let base = num_segments * seg_size;
        for i in 0..(remainder / 2) {
            let temp = buf[base + i];
            buf[base + i] = buf[base + remainder - 1 - i];
            buf[base + remainder - 1 - i] = temp;
        }
    }
}
