use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut tokens = input.split_whitespace();

    macro_rules! next_token {
        ($err:expr) => {
            match tokens.next() {
                Some(t) => t,
                None => {
                    eprint!("{}", $err);
                    std::process::exit(1);
                }
            }
        };
    }

    let flags: u32 = match next_token!("Error reading flags\n").parse() {
        Ok(v) => v,
        Err(_) => { eprint!("Error reading flags\n"); std::process::exit(1); }
    };
    let param1: i32 = match next_token!("Error reading param1\n").parse() {
        Ok(v) => v,
        Err(_) => { eprint!("Error reading param1\n"); std::process::exit(1); }
    };
    let param2: i32 = match next_token!("Error reading param2\n").parse() {
        Ok(v) => v,
        Err(_) => { eprint!("Error reading param2\n"); std::process::exit(1); }
    };
    let length: usize = match next_token!("Error reading length\n").parse() {
        Ok(v) => v,
        Err(_) => { eprint!("Error reading length\n"); std::process::exit(1); }
    };

    if length > 256 {
        eprint!("Error: length {} exceeds maximum 256\n", length);
        std::process::exit(1);
    }

    let mut buffer = [0u8; 256];
    for i in 0..length {
        let byte: u32 = match next_token!(format!("Error reading byte {}\n", i)).parse() {
            Ok(v) => v,
            Err(_) => { eprint!("Error reading byte {}\n", i); std::process::exit(1); }
        };
        buffer[i] = byte as u8;
    }

    let new_length = process_buffer(&mut buffer, length, flags, param1, param2);

    print!("{}", new_length);
    for i in 0..new_length {
        print!(" {}", buffer[i]);
    }
    println!();
}

fn process_buffer(buffer: &mut [u8; 256], length: usize, flags: u32, param1: i32, param2: i32) -> usize {
    let mut new_len = length;

    if length == 0 {
        return 0;
    }

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

fn rotate_buffer(buf: &mut [u8], len: usize, mut offset: i32) {
    if len <= 1 { return; }

    offset = offset % (len as i32);
    if offset < 0 { offset += len as i32; }
    if offset == 0 { return; }

    let mut temp = [0u8; 256];
    let chunk = if (offset as usize) < 256 { offset as usize } else { 256 };

    if (offset as usize) < len / 2 {
        let off = offset as usize;
        let mut i = 0usize;
        while i < off {
            let copy_len = if off - i < chunk { off - i } else { chunk };
            temp[..copy_len].copy_from_slice(&buf[i..i + copy_len]);
            buf.copy_within(off..len, i);
            buf[len - off..len - off + copy_len].copy_from_slice(&temp[..copy_len]);
            i += chunk;
        }
    } else {
        let shift = len - offset as usize;
        temp[..shift].copy_from_slice(&buf[..shift]);
        buf.copy_within(shift..shift + offset as usize, 0);
        buf[offset as usize..offset as usize + shift].copy_from_slice(&temp[..shift]);
    }
}

fn compact_runs(buf: &mut [u8], mut len: usize, threshold: u8) -> usize {
    let mut read = 0usize;
    let mut write = 0usize;

    while read < len {
        let current = buf[read];
        let mut run_len = 1usize;

        while read + run_len < len && buf[read + run_len] == current {
            run_len += 1;
        }

        if run_len >= threshold as usize {
            if run_len > 255 { run_len = 255; }

            buf[write] = current;
            buf[write + 1] = run_len as u8;
            write += 2;

            if read + run_len < len {
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
    if len <= 1 { return len; }

    if preserve_order {
        let mut write = 1usize;
        for i in 1..len {
            let val = buf[i];
            let mut found = false;
            for j in 0..write {
                if val == buf[j] {
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

fn interleave_halves(buf: &mut [u8], len: usize) {
    if len < 2 { return; }

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
                buf.copy_within(dst..src, dst + 1);
                buf[dst] = val;
            }
        }
    }
}

fn reverse_segments(buf: &mut [u8], len: usize, seg_size: usize) {
    if seg_size <= 1 || len < seg_size { return; }

    let num_segments = len / seg_size;
    let remainder = len % seg_size;

    for seg in 0..num_segments {
        let base = seg * seg_size;
        for i in 0..seg_size / 2 {
            let left = base + i;
            let right = base + seg_size - 1 - i;
            buf.swap(left, right);
        }
    }

    if remainder > 1 {
        let base = num_segments * seg_size;
        for i in 0..remainder / 2 {
            buf.swap(base + i, base + remainder - 1 - i);
        }
    }
}
