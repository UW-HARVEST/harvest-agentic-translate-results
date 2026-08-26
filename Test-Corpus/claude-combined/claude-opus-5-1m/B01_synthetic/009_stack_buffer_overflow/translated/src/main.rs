use std::io::{self, Read, Write};

/// Emulate C's fgets(buffer, size, stdin):
/// - Reads up to (size-1) bytes from stdin
/// - Stops after reading a newline (which is included)
/// - Stops at EOF
/// - Returns None if EOF is reached before any byte is read (matching NULL return)
/// - Returns Some(bytes_read) on success (the buffer is the bytes read; null terminator
///   is conceptual - in Rust we use the Vec<u8>)
fn fgets<R: Read>(reader: &mut R, max_size: usize) -> Option<Vec<u8>> {
    if max_size == 0 {
        return None;
    }
    // Read up to (max_size - 1) bytes from stdin, byte-by-byte, stopping on newline
    let limit = max_size - 1;
    let mut buf: Vec<u8> = Vec::with_capacity(limit);
    let mut byte = [0u8; 1];
    let mut got_any = false;
    while buf.len() < limit {
        match reader.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                got_any = true;
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if !got_any {
        return None;
    }
    Some(buf)
}

/// Emulate C's atoi: skip leading whitespace, optional sign, parse digits.
/// Returns 0 on no conversion. Overflow is UB in C; we wrap.
fn atoi(s: &[u8]) -> i32 {
    let mut i = 0usize;
    // Skip leading whitespace (matches C's isspace)
    while i < s.len() {
        match s[i] {
            b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c => i += 1,
            _ => break,
        }
    }
    let mut negative = false;
    if i < s.len() {
        if s[i] == b'-' {
            negative = true;
            i += 1;
        } else if s[i] == b'+' {
            i += 1;
        }
    }
    let mut result: i32 = 0;
    while i < s.len() {
        let c = s[i];
        if c < b'0' || c > b'9' {
            break;
        }
        let digit = (c - b'0') as i32;
        // Wrapping arithmetic to mirror C's UB on overflow without panicking
        result = result.wrapping_mul(10);
        if negative {
            result = result.wrapping_sub(digit);
        } else {
            result = result.wrapping_add(digit);
        }
        i += 1;
    }
    result
}

fn print_line(stdout: &mut impl Write, line: &str) {
    // Matches printf("%s\n", line)
    let _ = writeln!(stdout, "{}", line);
}

fn print_int_line(stdout: &mut impl Write, n: i32) {
    let _ = writeln!(stdout, "{}", n);
}

fn bad<R: Read, W: Write>(reader: &mut R, stdout: &mut W) {
    let mut data: i32 = -1;
    {
        // char inputBuffer[14] = ""
        match fgets(reader, 14) {
            Some(input_buffer) => {
                data = atoi(&input_buffer);
            }
            None => {
                print_line(stdout, "fgets() failed.");
            }
        }
    }
    {
        let mut buffer: [i32; 10] = [0; 10];
        if data >= 0 {
            // Out-of-bounds writes are UB in C. We replicate the index-into-array
            // semantics for in-bounds; for out-of-bounds we deliberately do nothing
            // because UB cannot be portably reproduced. The CWE test cases here
            // typically use small in-bounds indices in practice.
            if (data as usize) < buffer.len() {
                buffer[data as usize] = 1;
            }
            for i in 0..10 {
                print_int_line(stdout, buffer[i]);
            }
        } else {
            print_line(stdout, "ERROR: Array index is negative.");
        }
    }
}

fn good_g2b<W: Write>(stdout: &mut W) {
    // Original C: int data = -1; data = 7;
    let data: i32 = 7;
    {
        let mut buffer: [i32; 10] = [0; 10];
        if data >= 0 {
            buffer[data as usize] = 1;
            for i in 0..10 {
                print_int_line(stdout, buffer[i]);
            }
        } else {
            print_line(stdout, "ERROR: Array index is negative.");
        }
    }
}

fn good_b2g<R: Read, W: Write>(reader: &mut R, stdout: &mut W) {
    let mut data: i32 = -1;
    {
        match fgets(reader, 14) {
            Some(input_buffer) => {
                data = atoi(&input_buffer);
            }
            None => {
                print_line(stdout, "fgets() failed.");
            }
        }
    }
    {
        let mut buffer: [i32; 10] = [0; 10];
        if data >= 0 && data < 10 {
            buffer[data as usize] = 1;
            for i in 0..10 {
                print_int_line(stdout, buffer[i]);
            }
        } else {
            print_line(stdout, "ERROR: Array index is out-of-bounds");
        }
    }
}

fn good<R: Read, W: Write>(reader: &mut R, stdout: &mut W) {
    good_g2b(stdout);
    good_b2g(reader, stdout);
}

fn main() {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    print_line(&mut out, "Calling good()...");
    good(&mut reader, &mut out);
    print_line(&mut out, "Finished good()");
    print_line(&mut out, "Calling bad()...");
    bad(&mut reader, &mut out);
    print_line(&mut out, "Finished bad()");
}
