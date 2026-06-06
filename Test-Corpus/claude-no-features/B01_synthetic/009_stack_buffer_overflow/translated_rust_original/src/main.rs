use std::io::{self, Read, Write};

fn print_line(line: &str) {
    // C: printf("%s\n", line); - matches non-NULL case
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(line.as_bytes());
    let _ = handle.write_all(b"\n");
}

fn print_int_line(int_number: i32) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = write!(handle, "{}\n", int_number);
}

/// Mimics C's `fgets(buf, size, stdin)`.
/// Reads up to `size - 1` bytes from stdin into a String, stopping when a
/// newline is read (newline is kept) or EOF is reached. Returns None if
/// no bytes were read before EOF (which is what fgets returns NULL for).
fn fgets_stdin(size: usize) -> Option<Vec<u8>> {
    if size == 0 {
        return None;
    }
    let max_chars = size - 1;
    let mut buf: Vec<u8> = Vec::with_capacity(max_chars);
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];
    while buf.len() < max_chars {
        match handle.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

/// Mimics C's `atoi`. Skips leading whitespace, reads optional sign,
/// then reads digits, returning 0 if no valid conversion. Wraps on overflow
/// per typical C behavior (though that's UB in C, we use wrapping).
fn c_atoi(bytes: &[u8]) -> i32 {
    let mut i = 0usize;
    // Skip leading whitespace (matches isspace())
    while i < bytes.len()
        && matches!(
            bytes[i],
            b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r'
        )
    {
        i += 1;
    }
    let mut negative = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        negative = bytes[i] == b'-';
        i += 1;
    }
    let mut result: i32 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let digit = (bytes[i] - b'0') as i32;
        result = result.wrapping_mul(10);
        result = if negative {
            result.wrapping_sub(digit)
        } else {
            result.wrapping_add(digit)
        };
        i += 1;
    }
    result
}

fn bad() {
    let mut data: i32;
    // Initialize data
    data = -1;
    {
        // char inputBuffer[14] = "";
        if let Some(input_buffer) = fgets_stdin(14) {
            // Convert to int
            data = c_atoi(&input_buffer);
        } else {
            print_line("fgets() failed.");
        }
    }
    {
        let mut buffer: [i32; 10] = [0; 10];
        if data >= 0 {
            // buffer[data] = 1; — reproduces the OOB write bug if data >= 10.
            // Use safe Rust by emulating a write with bounds checking; but the
            // C code performs an out-of-bounds write which is UB. For values
            // 0..=9, this writes correctly. For values >= 10 we replicate by
            // skipping (since UB; we cannot reproduce arbitrary memory writes
            // safely). The print loop output for valid range is what matters
            // for byte-identical behavior on valid inputs.
            if (data as usize) < buffer.len() {
                buffer[data as usize] = 1;
            }
            // Print the array values
            for i in 0..10 {
                print_int_line(buffer[i]);
            }
        } else {
            print_line("ERROR: Array index is negative.");
        }
    }
}

/* goodG2B uses the GoodSource with the BadSink */
fn good_g2b() {
    let data: i32;
    // Initialize data (C sets to -1 then immediately overwrites with 7)
    let _initial: i32 = -1;
    let _ = _initial;
    data = 7;
    {
        let mut buffer: [i32; 10] = [0; 10];
        if data >= 0 {
            buffer[data as usize] = 1;
            for i in 0..10 {
                print_int_line(buffer[i]);
            }
        } else {
            print_line("ERROR: Array index is negative.");
        }
    }
}

/* goodB2G uses the BadSource with the GoodSink */
fn good_b2g() {
    let mut data: i32;
    data = -1;
    {
        if let Some(input_buffer) = fgets_stdin(14) {
            data = c_atoi(&input_buffer);
        } else {
            print_line("fgets() failed.");
        }
    }
    {
        let mut buffer: [i32; 10] = [0; 10];
        if data >= 0 && data < 10 {
            buffer[data as usize] = 1;
            for i in 0..10 {
                print_int_line(buffer[i]);
            }
        } else {
            print_line("ERROR: Array index is out-of-bounds");
        }
    }
}

fn good() {
    good_g2b();
    good_b2g();
}

fn main() {
    print_line("Calling good()...");
    good();
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad();
    print_line("Finished bad()");
}
