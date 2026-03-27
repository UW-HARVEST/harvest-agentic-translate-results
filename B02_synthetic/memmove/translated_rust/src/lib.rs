/// Equivalent of memmove: copy len bytes from src_start to dst_start within buf,
/// handling overlapping regions correctly.
fn buf_memmove(buf: &mut [u8], dst_start: usize, src_start: usize, len: usize) {
    if len == 0 || dst_start == src_start {
        return;
    }
    buf.copy_within(src_start..src_start + len, dst_start);
}

#[no_mangle]
pub extern "C" fn process_buffer(buffer: &mut [u8; 256], length: usize, flags: u32, param1: i32, param2: i32) -> usize {
    if length == 0 {
        return 0;
    }

    let mut new_len = length;

    if flags & 0x01 != 0 {
        let offset = param1 % (length as i32);
        if offset != 0 {
            rotate_buffer(buffer, length, offset);
        }
    }

    if flags & 0x02 != 0 {
        let threshold: u8 = if param1 > 0 && param1 <= 255 { param1 as u8 } else { 3 };
        new_len = compact_runs(buffer, new_len, threshold);
    }

    if flags & 0x04 != 0 {
        let preserve = param2 != 0;
        new_len = remove_duplicates(buffer, new_len, preserve);
    }

    if (flags & 0x08 != 0) && new_len >= 2 {
        interleave_halves(buffer, new_len);
    }

    if (flags & 0x10 != 0) && new_len >= 4 {
        let seg_size: usize = if param1 > 0 { param1 as usize } else { 4 };
        if seg_size <= new_len {
            reverse_segments(buffer, new_len, seg_size);
        }
    }

    new_len
}

fn rotate_buffer(buf: &mut [u8; 256], len: usize, offset: i32) {
    if len <= 1 {
        return;
    }

    let mut offset = offset % (len as i32);
    if offset < 0 {
        offset += len as i32;
    }
    if offset == 0 {
        return;
    }
    let offset = offset as usize;

    let mut temp = [0u8; 256];

    if offset < len / 2 {
        let chunk = if offset < 256 { offset } else { 256 };
        let mut i = 0usize;
        while i < offset {
            let copy_len = if offset - i < chunk { offset - i } else { chunk };
            temp[..copy_len].copy_from_slice(&buf[i..i + copy_len]);
            buf_memmove(buf, i, offset, len - offset);
            buf[len - offset..len - offset + copy_len].copy_from_slice(&temp[..copy_len]);
            i += chunk;
        }
    } else {
        let shift = len - offset;
        temp[..shift].copy_from_slice(&buf[..shift]);
        buf_memmove(buf, 0, shift, offset);
        buf[offset..offset + shift].copy_from_slice(&temp[..shift]);
    }
}

fn compact_runs(buf: &mut [u8; 256], len: usize, threshold: u8) -> usize {
    let mut len = len;
    let mut read = 0usize;
    let mut write = 0usize;

    while read < len {
        let current = buf[read];
        let mut run_len = 1usize;

        while read + run_len < len && buf[read + run_len] == current {
            run_len += 1;
        }

        if run_len >= threshold as usize {
            if run_len > 255 {
                run_len = 255;
            }

            buf[write] = current;
            write += 1;
            buf[write] = run_len as u8;
            write += 1;

            if read + run_len < len {
                let remaining = len - (read + run_len);
                buf_memmove(buf, write, read + run_len, remaining);
            }
            len = write + (len - (read + run_len));
            read = write;
        } else {
            if write != read {
                buf_memmove(buf, write, read, run_len);
            }
            write += run_len;
            read += run_len;
        }
    }

    len
}

fn remove_duplicates(buf: &mut [u8; 256], len: usize, preserve_order: bool) -> usize {
    if len <= 1 {
        return len;
    }

    if preserve_order {
        let mut write = 1usize;
        for i in 1..len {
            let mut found = false;
            for j in 0..write {
                if buf[i] == buf[j] {
                    found = true;
                    break;
                }
            }
            if !found {
                if write != i {
                    buf[write] = buf[i];
                }
                write += 1;
            }
        }
        write
    } else {
        let mut seen = [0u8; 256];
        let mut write = 0usize;

        for i in 0..len {
            if seen[buf[i] as usize] == 0 {
                seen[buf[i] as usize] = 1;
                if write != i {
                    let tmp = buf[write];
                    buf[write] = buf[i];
                    buf[i] = tmp;
                }
                write += 1;
            }
        }
        write
    }
}

fn interleave_halves(buf: &mut [u8; 256], len: usize) {
    if len < 2 {
        return;
    }

    let half = len / 2;
    let odd = len % 2;

    if half <= 256 {
        let mut temp = [0u8; 256];
        temp[..half].copy_from_slice(&buf[..half]);

        for i in 0..half {
            buf[i * 2 + 1] = buf[half + i];
            buf[i * 2] = temp[i];
        }
        if odd != 0 {
            buf[len - 1] = buf[half];
        }
    } else {
        for i in 0..half {
            let src = half + i;
            let dst = i * 2 + 1;
            if dst < src {
                let val = buf[src];
                buf_memmove(buf, dst + 1, dst, src - dst);
                buf[dst] = val;
            }
        }
    }
}

fn reverse_segments(buf: &mut [u8; 256], len: usize, seg_size: usize) {
    if seg_size <= 1 || len < seg_size {
        return;
    }

    let num_segments = len / seg_size;
    let remainder = len % seg_size;

    for seg in 0..num_segments {
        let base = seg * seg_size;
        for i in 0..seg_size / 2 {
            let left = base + i;
            let right = base + seg_size - 1 - i;
            let tmp = buf[left];
            buf[left] = buf[right];
            buf[right] = tmp;
        }
    }

    if remainder > 1 {
        let base = num_segments * seg_size;
        for i in 0..remainder / 2 {
            let tmp = buf[base + i];
            buf[base + i] = buf[base + remainder - 1 - i];
            buf[base + remainder - 1 - i] = tmp;
        }
    }
}
