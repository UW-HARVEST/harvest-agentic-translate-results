pub fn process_buffer(buffer: &mut [u8], length: usize, flags: u32, param1: i32, param2: i32) -> usize {
    if buffer.is_empty() || length == 0 {
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
        new_len = compact_runs(&mut buffer[..new_len], threshold);
    }
    
    if (flags & 0x04) != 0 {
        let preserve = param2 != 0;
        new_len = remove_duplicates(&mut buffer[..new_len], preserve);
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

fn rotate_buffer(buf: &mut [u8], mut offset: i32) {
    let len = buf.len();
    if len <= 1 {
        return;
    }
    
    offset %= len as i32;
    if offset < 0 {
        offset += len as i32;
    }
    if offset == 0 {
        return;
    }
    
    let offset = offset as usize;
    buf.rotate_left(offset);
}

fn compact_runs(buf: &mut [u8], threshold: u8) -> usize {
    let mut len = buf.len();
    let mut read = 0;
    let mut write = 0;
    
    while read < len {
        let current = buf[read];
        let mut run_len = 1;
        
        while read + run_len < len && buf[read + run_len] == current {
            run_len += 1;
        }
        
        if run_len >= threshold as usize {
            let mut capped_run_len = run_len;
            if capped_run_len > 255 {
                capped_run_len = 255;
            }
            
            buf[write] = current;
            buf[write + 1] = capped_run_len as u8;
            write += 2;
            
            if read + run_len < len {
                let remaining = len - (read + run_len);
                buf.copy_within((read + run_len)..(read + run_len + remaining), write);
            }
            len = write + (len - (read + run_len));
            read = write;
        } else {
            if write != read {
                buf.copy_within(read..(read + run_len), write);
            }
            write += run_len;
            read += run_len;
        }
    }
    
    len
}

fn remove_duplicates(buf: &mut [u8], preserve_order: bool) -> usize {
    let len = buf.len();
    if len <= 1 {
        return len;
    }
    
    if preserve_order {
        let mut write = 1;
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
        let mut seen = [false; 256];
        let mut write = 0;
        
        for i in 0..len {
            let val = buf[i];
            if !seen[val as usize] {
                seen[val as usize] = true;
                if write != i {
                    buf.swap(write, i);
                }
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
    let odd = len % 2 != 0;
    
    if half <= 256 {
        let mut temp = [0u8; 256];
        temp[..half].copy_from_slice(&buf[..half]);
        
        for i in 0..half {
            buf.copy_within((half + i)..(half + i + 1), i * 2 + 1);
            buf[i * 2] = temp[i];
        }
        if odd {
            let val = buf[half];
            buf[len - 1] = val;
        }
    } else {
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

fn reverse_segments(buf: &mut [u8], seg_size: usize) {
    let len = buf.len();
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
