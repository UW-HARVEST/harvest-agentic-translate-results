fn compact_runs(buf: &mut [u8], len: usize, threshold: u8) -> usize;
fn rotate_buffer(buf: &mut [u8], len: usize, offset: i32);
fn remove_duplicates(buf: &mut [u8], len: usize, preserve_order: bool) -> usize;
fn interleave_halves(buf: &mut [u8], len: usize);
fn reverse_segments(buf: &mut [u8], len: usize, seg_size: usize);

pub fn process_buffer(buffer: &mut [u8], length: usize, flags: u32, param1: i32, param2: i32) -> usize {
    let mut new_len = length;
    
    if length == 0 {
        return 0;
    }
    
    if flags & 0x01 != 0 {
        let offset = param1 % length as i32;
        if offset != 0 {
            rotate_buffer(buffer, length, offset);
        }
    }
    
    if flags & 0x02 != 0 {
        let threshold = if param1 > 0 && param1 <= 255 { param1 as u8 } else { 3 };
        new_len = compact_runs(buffer, new_len, threshold);
    }
    
    if flags & 0x04 != 0 {
        let preserve = param2 != 0;
        new_len = remove_duplicates(buffer, new_len, preserve);
    }
    
    if flags & 0x08 != 0 && new_len >= 2 {
        interleave_halves(buffer, new_len);
    }
    
    if flags & 0x10 != 0 && new_len >= 4 {
        let seg_size = if param1 > 0 { param1 as usize } else { 4 };
        if seg_size <= new_len {
            reverse_segments(buffer, new_len, seg_size);
        }
    }
    
    new_len
}

fn rotate_buffer(buf: &mut [u8], len: usize, offset: i32) {
    if len <= 1 {
        return;
    }
    
    let mut offset = offset % len as i32;
    if offset < 0 {
        offset += len as i32;
    }
    if offset == 0 {
        return;
    }
    
    let offset = offset as usize;
    buf[..len].rotate_right(offset);
}

fn compact_runs(buf: &mut [u8], len: usize, threshold: u8) -> usize {
    let mut read = 0;
    let mut write = 0;
    let mut len = len;
    
    while read < len {
        let current = buf[read];
        let mut run_len = 1usize;
        
        while read + run_len < len && buf[read + run_len] == current {
            run_len += 1;
        }
        
        if run_len >= threshold as usize {
            let run_len = if run_len > 255 { 255 } else { run_len };
            
            buf[write] = current;
            write += 1;
            buf[write] = run_len as u8;
            write += 1;
            
            if read + run_len < len {
                let remaining = len - (read + run_len);
                buf.copy_within(read + run_len..len, write);
            }
            len = write + (len - (read + run_len));
            read = write;
        } else {
            if write != read {
                buf.copy_within(read..read + run_len, write);
            }
            write += run_len;
            read += run_len;
        }
    }
    
    len
}

fn remove_duplicates(buf: &mut [u8], len: usize, preserve_order: bool) -> usize {
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
            if !seen[buf[i] as usize] {
                seen[buf[i] as usize] = true;
                if write != i {
                    buf.swap(write, i);
                }
                write += 1;
            }
        }
        write
    }
}

fn interleave_halves(buf: &mut [u8], len: usize) {
    if len < 2 {
        return;
    }
    
    let half = len / 2;
    let odd = len % 2;
    
    let mut temp = vec![0u8; half];
    temp.copy_from_slice(&buf[..half]);
    
    let second_half: Vec<u8> = buf[half..len - odd].to_vec();
    
    for i in 0..half {
        buf[i * 2] = temp[i];
        buf[i * 2 + 1] = second_half[i];
    }
    
    if odd == 1 {
        buf[len - 1] = buf[half];
    }
}

fn reverse_segments(buf: &mut [u8], len: usize, seg_size: usize) {
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
        buf[base..base + remainder].reverse();
    }
}
