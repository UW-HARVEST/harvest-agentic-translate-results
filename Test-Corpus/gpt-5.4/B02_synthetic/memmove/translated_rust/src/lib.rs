pub fn process_buffer(buffer: &mut [u8], length: usize, flags: u32, param1: i32, param2: i32) -> usize {
    if length == 0 || length > buffer.len() {
        return 0;
    }

    let mut new_len = length;

    if (flags & 0x01) != 0 {
        let offset = param1 % (length as i32);
        if offset != 0 {
            rotate_buffer(&mut buffer[..length], offset);
        }
    }

    if (flags & 0x02) != 0 {
        let threshold = if param1 > 0 && param1 <= 255 { param1 as u8 } else { 3 };
        new_len = compact_runs(buffer, new_len, threshold);
    }

    if (flags & 0x04) != 0 {
        let preserve = param2 != 0;
        new_len = remove_duplicates(buffer, new_len, preserve);
    }

    if (flags & 0x08) != 0 && new_len >= 2 {
        interleave_halves(&mut buffer[..new_len]);
    }

    if (flags & 0x10) != 0 && new_len >= 4 {
        let seg_size = if param1 > 0 { param1 as usize } else { 4 };
        if seg_size <= new_len {
            reverse_segments(&mut buffer[..new_len], seg_size);
        }
    }

    new_len
}

fn rotate_buffer(buf: &mut [u8], offset: i32) {
    let len = buf.len();
    if len <= 1 {
        return;
    }

    let mut offset = offset % (len as i32);
    if offset < 0 {
        offset += len as i32;
    }
    let offset = offset as usize;
    if offset == 0 {
        return;
    }

    buf.rotate_right(offset);
}

fn compact_runs(buf: &mut [u8], len: usize, threshold: u8) -> usize {
    let mut out = Vec::with_capacity(len);
    let mut read = 0;

    while read < len {
        let current = buf[read];
        let mut run_len = 1usize;
        while read + run_len < len && buf[read + run_len] == current {
            run_len += 1;
        }

        if run_len >= threshold as usize {
            out.push(current);
            out.push(run_len.min(255) as u8);
        } else {
            out.extend_from_slice(&buf[read..read + run_len]);
        }

        read += run_len;
    }

    let new_len = out.len();
    buf[..new_len].copy_from_slice(&out);
    new_len
}

fn remove_duplicates(buf: &mut [u8], len: usize, preserve_order: bool) -> usize {
    if len <= 1 {
        return len;
    }

    if preserve_order {
        let mut seen = [false; 256];
        let mut write = 0usize;
        for i in 0..len {
            let b = buf[i] as usize;
            if !seen[b] {
                seen[b] = true;
                buf[write] = buf[i];
                write += 1;
            }
        }
        write
    } else {
        let mut seen = [false; 256];
        let mut write = 0usize;
        for i in 0..len {
            let b = buf[i] as usize;
            if !seen[b] {
                seen[b] = true;
                buf.swap(write, i);
                write += 1;
            }
        }
        write
    }
}

fn interleave_halves(buf: &mut [u8]) {
    let len = buf.len();
    if len < 2 {
        return;
    }

    let half = len / 2;
    let mut out = Vec::with_capacity(len);

    for i in 0..half {
        out.push(buf[i]);
        out.push(buf[half + i]);
    }

    if len % 2 != 0 {
        out.push(buf[half]);
    }

    buf.copy_from_slice(&out);
}

fn reverse_segments(buf: &mut [u8], seg_size: usize) {
    let len = buf.len();
    if seg_size <= 1 || len < seg_size {
        return;
    }

    let num_segments = len / seg_size;
    let remainder = len % seg_size;

    for seg in 0..num_segments {
        let base = seg * seg_size;
        buf[base..base + seg_size].reverse();
    }

    if remainder > 1 {
        let base = num_segments * seg_size;
        buf[base..].reverse();
    }
}
