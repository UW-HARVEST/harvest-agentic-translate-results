use std::io::{self, Read, Write};

const CHAR_ARRAY_SIZE: usize = 20;

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

/// Mimic C's fgets: reads up to `n - 1` bytes from stdin, stopping after a
/// newline (which is kept in the buffer). Returns `None` if no bytes were read
/// (EOF or error before any character), otherwise returns the bytes read.
fn fgets(n: usize, stdin: &mut impl Read) -> Option<Vec<u8>> {
    if n == 0 {
        return None;
    }
    let mut buf: Vec<u8> = Vec::new();
    let max = n - 1;
    let mut byte = [0u8; 1];
    while buf.len() < max {
        match stdin.read(&mut byte) {
            Ok(0) => break,
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

/// Mimic C's atof: parses leading whitespace, optional sign, digits, optional
/// fractional part, optional exponent. Stops at the first invalid character.
/// Returns 0.0 if no valid conversion could be performed.
fn atof(bytes: &[u8]) -> f64 {
    let mut i = 0;
    let n = bytes.len();

    // Skip leading whitespace (matches C isspace for ASCII).
    while i < n {
        let c = bytes[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0B || c == 0x0C {
            i += 1;
        } else {
            break;
        }
    }

    let start = i;

    // Optional sign.
    if i < n && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }

    let digits_start = i;

    // Integer part digits.
    while i < n && bytes[i].is_ascii_digit() {
        i += 1;
    }

    let had_int_digits = i > digits_start;

    // Fractional part.
    let mut had_frac_digits = false;
    if i < n && bytes[i] == b'.' {
        i += 1;
        let frac_start = i;
        while i < n && bytes[i].is_ascii_digit() {
            i += 1;
        }
        had_frac_digits = i > frac_start;
    }

    if !had_int_digits && !had_frac_digits {
        return 0.0;
    }

    let mut end = i;

    // Optional exponent.
    if end < n && (bytes[end] == b'e' || bytes[end] == b'E') {
        let mut j = end + 1;
        if j < n && (bytes[j] == b'+' || bytes[j] == b'-') {
            j += 1;
        }
        let exp_start = j;
        while j < n && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j > exp_start {
            end = j;
        }
    }

    let s = std::str::from_utf8(&bytes[start..end]).unwrap_or("");
    s.parse::<f64>().unwrap_or(0.0)
}

/// Convert f64 to i32 matching C's `(int)` cast on x86_64 (cvttsd2si):
/// - In-range finite values are truncated toward zero.
/// - NaN, infinities, and out-of-range values yield `i32::MIN` (the
///   "invalid conversion" indicator value 0x80000000), which is the typical
///   behavior of GCC/Clang on x86_64 even though the C standard considers
///   it undefined behavior.
fn f64_to_i32_c(v: f64) -> i32 {
    if v.is_nan() || v >= 2147483648.0 || v < -2147483648.0 {
        i32::MIN
    } else {
        v as i32
    }
}

fn bad(stdin: &mut impl Read) {
    let mut data: f32 = 0.0;
    {
        match fgets(CHAR_ARRAY_SIZE, stdin) {
            Some(input_buffer) => {
                data = atof(&input_buffer) as f32;
            }
            None => {
                print_line("fgets() failed.");
            }
        }
    }
    {
        let result = f64_to_i32_c(100.0_f64 / data as f64);
        print_int_line(result);
    }
}

fn good_g2b() {
    let data: f32 = 2.0;
    let result = f64_to_i32_c(100.0_f64 / data as f64);
    print_int_line(result);
}

fn good_b2g(stdin: &mut impl Read) {
    let mut data: f32 = 0.0;
    {
        match fgets(CHAR_ARRAY_SIZE, stdin) {
            Some(input_buffer) => {
                data = atof(&input_buffer) as f32;
            }
            None => {
                print_line("fgets() failed.");
            }
        }
    }
    if (data as f64).abs() > 0.000001 {
        let result = f64_to_i32_c(100.0_f64 / data as f64);
        print_int_line(result);
    } else {
        print_line("This would result in a divide by zero");
    }
}

fn good(stdin: &mut impl Read) {
    good_g2b();
    good_b2g(stdin);
}

fn main() {
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    print_line("Calling good()...");
    good(&mut handle);
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad(&mut handle);
    print_line("Finished bad()");

    // Ensure stdout is flushed before exit.
    let _ = io::stdout().flush();
}
