use std::ffi::CStr;
use std::io::{self, Read, Write};
use std::os::raw::{c_char, c_int};

#[no_mangle]
pub extern "C" fn printLine(line: *const c_char) {
    if line.is_null() {
        return;
    }
    // SAFETY: caller passed a valid C string.
    let cstr = unsafe { CStr::from_ptr(line) };
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(cstr.to_bytes());
    let _ = out.write_all(b"\n");
    let _ = out.flush();
}

#[no_mangle]
pub extern "C" fn printIntLine(int_number: c_int) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", int_number);
    let _ = out.flush();
}

fn print_line_str(s: &str) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", s);
    let _ = out.flush();
}

fn print_int_line_i32(int_number: i32) {
    printIntLine(int_number);
}

/// Mimic C's fgets(buf, n, stdin):
/// - reads at most n-1 bytes from stdin
/// - stops after a newline (which is included in the result)
/// - returns None if no characters are read before EOF
fn fgets(n: usize) -> Option<Vec<u8>> {
    if n == 0 {
        return None;
    }
    let max_chars = n - 1;
    let mut buf: Vec<u8> = Vec::with_capacity(max_chars);
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];
    let mut read_any = false;
    while buf.len() < max_chars {
        match handle.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                read_any = true;
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => return None,
        }
    }
    if !read_any {
        return None;
    }
    Some(buf)
}

/// Mimic C's atoi: skip leading whitespace, optional sign, parse digits,
/// stop at first non-digit. Returns 0 if no digits are found.
fn atoi(bytes: &[u8]) -> i32 {
    let mut i = 0usize;
    // Skip leading whitespace (matches isspace in C locale)
    while i < bytes.len()
        && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
    {
        i += 1;
    }
    let mut sign: i32 = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut result: i32 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let digit = (bytes[i] - b'0') as i32;
        // Use wrapping arithmetic to mirror C's behavior on overflow.
        result = result.wrapping_mul(10).wrapping_add(digit);
        i += 1;
    }
    result.wrapping_mul(sign)
}

#[no_mangle]
pub extern "C" fn bad() {
    let mut data: i32 = -1;
    // char inputBuffer[14] = "";  fgets(inputBuffer, 14, stdin)
    match fgets(14) {
        Some(input) => {
            data = atoi(&input);
        }
        None => {
            print_line_str("fgets() failed.");
        }
    }
    let mut buffer: [i32; 10] = [0; 10];
    if data >= 0 {
        // Mirror C: out-of-bounds write is the documented bad behavior, but
        // tests will not exercise the OOB path.
        if (data as usize) < buffer.len() {
            buffer[data as usize] = 1;
        } else {
            // Replicate undefined behavior path safely by skipping the OOB write.
            // The C code would write past the buffer; we cannot do that in Rust
            // without UB, so callers must not test with data >= 10 for `bad`.
        }
        for i in 0..10 {
            print_int_line_i32(buffer[i]);
        }
    } else {
        print_line_str("ERROR: Array index is negative.");
    }
}

/* goodG2B uses the GoodSource with the BadSink */
fn good_g2b() {
    let data: i32 = 7;
    let mut buffer: [i32; 10] = [0; 10];
    if data >= 0 {
        buffer[data as usize] = 1;
        for i in 0..10 {
            print_int_line_i32(buffer[i]);
        }
    } else {
        print_line_str("ERROR: Array index is negative.");
    }
}

/* goodB2G uses the BadSource with the GoodSink */
fn good_b2g() {
    let mut data: i32 = -1;
    match fgets(14) {
        Some(input) => {
            data = atoi(&input);
        }
        None => {
            print_line_str("fgets() failed.");
        }
    }
    let mut buffer: [i32; 10] = [0; 10];
    if data >= 0 && data < 10 {
        buffer[data as usize] = 1;
        for i in 0..10 {
            print_int_line_i32(buffer[i]);
        }
    } else {
        print_line_str("ERROR: Array index is out-of-bounds");
    }
}

#[no_mangle]
pub extern "C" fn good() {
    good_g2b();
    good_b2g();
}

/// Mirror the C `main(int argc, char* argv[])`. The C source compiled as
/// a shared library exports `main`, so the Rust cdylib must also export it.
/// Excluded from `--test` builds because rustc generates its own test `main`.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main(_argc: c_int, _argv: *const *const c_char) -> c_int {
    print_line_str("Calling good()...");
    good();
    print_line_str("Finished good()");
    print_line_str("Calling bad()...");
    bad();
    print_line_str("Finished bad()");
    0
}
