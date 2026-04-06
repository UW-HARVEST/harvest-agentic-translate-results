use std::io::{self, Read, Write};

fn print_line(line: &[u8]) {
    let len = line.iter().position(|&b| b == 0).unwrap_or(line.len());
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(&line[..len]);
    let _ = out.write_all(b"\n");
}

fn main() {
    let mut data: i32 = -1;

    // fgets(inputBuffer, 14, stdin): reads up to 13 bytes, stops at newline (inclusive)
    {
        let mut buf = [0u8; 13];
        let mut pos = 0usize;
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        loop {
            if pos >= 13 {
                break;
            }
            let mut byte = [0u8; 1];
            match handle.read(&mut byte) {
                Ok(0) => break,  // EOF
                Ok(_) => {
                    buf[pos] = byte[0];
                    pos += 1;
                    if byte[0] == b'\n' {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        if pos == 0 {
            // fgets returned NULL (EOF with no data read)
            print_line(b"fgets() failed.");
        } else {
            // null-terminate and atoi
            let s = std::str::from_utf8(&buf[..pos]).unwrap_or("");
            data = c_atoi(s);
        }
    }

    {
        let mut source = [0u8; 100];
        let mut dest = [0u8; 100];
        for i in 0..99 {
            source[i] = b'A';
        }
        source[99] = 0;

        if data < 100 {
            // strncpy(dest, source, data): data cast to size_t
            let n = data as usize;
            unsafe {
                c_strncpy(dest.as_mut_ptr(), source.as_ptr(), n);
            }
            // dest[data] = '\0': signed index, UB when negative
            unsafe {
                *dest.as_mut_ptr().offset(data as isize) = 0;
            }
        }
        print_line(&dest);
    }
}

fn c_atoi(s: &str) -> i32 {
    let s = s.trim_start();
    let mut chars = s.chars().peekable();
    let neg = match chars.peek() {
        Some('-') => { chars.next(); true }
        Some('+') => { chars.next(); false }
        _ => false,
    };
    let mut result: i32 = 0;
    for c in chars {
        if c.is_ascii_digit() {
            result = result.wrapping_mul(10).wrapping_add(c as i32 - '0' as i32);
        } else {
            break;
        }
    }
    if neg { result.wrapping_neg() } else { result }
}

unsafe fn c_strncpy(dest: *mut u8, src: *const u8, n: usize) {
    let mut i = 0usize;
    while i < n {
        let b = *src.add(i);
        *dest.add(i) = b;
        if b == 0 {
            i += 1;
            break;
        }
        i += 1;
    }
    while i < n {
        *dest.add(i) = 0;
        i += 1;
    }
}
