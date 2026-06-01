use std::io::{self, Read, Write};

const CHAR_ARRAY_SIZE: usize = 20;

fn print_line(line: &str) {
    // Matches C's printf("%s\n", line); writes to stdout
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(line.as_bytes());
    let _ = out.write_all(b"\n");
}

fn print_int_line(n: i32) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", n);
}

/// Reads up to `n - 1` bytes from stdin into a buffer, stopping at newline or EOF.
/// Mirrors C's fgets: returns None if EOF occurs before any byte is read,
/// otherwise returns the bytes read (newline included if seen).
fn c_fgets(n: usize, stdin: &mut impl Read) -> Option<Vec<u8>> {
    if n == 0 {
        return None;
    }
    if n == 1 {
        // fgets writes a single null byte and returns the buffer; but we
        // don't use this path so just treat as no-op success on any input.
        return Some(Vec::new());
    }
    let max_chars = n - 1;
    let mut buf: Vec<u8> = Vec::with_capacity(max_chars);
    let mut tmp = [0u8; 1];
    while buf.len() < max_chars {
        match stdin.read(&mut tmp) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf.push(tmp[0]);
                if tmp[0] == b'\n' {
                    break;
                }
            }
            Err(_) => return None,
        }
    }
    if buf.is_empty() {
        return None;
    }
    Some(buf)
}

/// Replicates C's atof (double atof(const char *s)).
/// Skips whitespace, parses optional sign, digits, optional decimal,
/// optional exponent. Also handles INF/INFINITY/NAN (case insensitive).
/// Returns 0.0 on parse failure.
fn c_atof(s: &[u8]) -> f64 {
    let bytes = s;
    let mut i = 0usize;
    // Skip whitespace (matches C isspace ASCII set)
    while i < bytes.len()
        && matches!(
            bytes[i],
            b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r'
        )
    {
        i += 1;
    }
    let start = i;
    // Optional sign
    let mut sign_negative = false;
    if i < bytes.len() {
        if bytes[i] == b'+' {
            i += 1;
        } else if bytes[i] == b'-' {
            sign_negative = true;
            i += 1;
        }
    }
    // Check for INF/INFINITY/NAN (case insensitive)
    let rest = &bytes[i..];
    let lower_starts_with = |needle: &[u8]| -> bool {
        if rest.len() < needle.len() {
            return false;
        }
        for (a, b) in rest.iter().zip(needle.iter()) {
            if a.to_ascii_lowercase() != *b {
                return false;
            }
        }
        true
    };
    if lower_starts_with(b"infinity") {
        return if sign_negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    if lower_starts_with(b"inf") {
        return if sign_negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    if lower_starts_with(b"nan") {
        // Optionally consume (n-char-sequence) — atof doesn't strictly need this
        // but strtod does. atof = strtod(nptr, NULL).
        let mut j = i + 3;
        if j < bytes.len() && bytes[j] == b'(' {
            let mut k = j + 1;
            while k < bytes.len() && bytes[k] != b')' {
                k += 1;
            }
            if k < bytes.len() {
                j = k + 1;
            }
        }
        let _ = j;
        return f64::NAN;
    }

    // Parse integer part
    let mut has_int_digits = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
        has_int_digits = true;
    }
    // Parse fractional part
    let mut has_frac_digits = false;
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            has_frac_digits = true;
        }
    }
    if !has_int_digits && !has_frac_digits {
        return 0.0;
    }
    // Optional exponent
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let exp_start = i;
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let mut exp_has_digits = false;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            exp_has_digits = true;
        }
        if !exp_has_digits {
            i = exp_start;
        }
    }
    let slice = &bytes[start..i];
    // Use Rust's parser on the validated prefix
    match std::str::from_utf8(slice) {
        Ok(text) => text.parse::<f64>().unwrap_or(0.0),
        Err(_) => 0.0,
    }
}

/// Mirrors C's `(int)double_value` cast on x86_64 (cvttsd2si):
/// out-of-range / NaN / Inf produce INT_MIN (0x80000000).
fn f64_to_i32_c(v: f64) -> i32 {
    if v.is_nan() || v >= 2147483648.0 || v < -2147483648.0 {
        return i32::MIN;
    }
    // Truncate toward zero. v is in (-2147483649.0, 2147483648.0), so
    // trunc(v) fits in i32. Use safe `as` on f64 (saturating in Rust 1.45+),
    // which produces correct in-range conversions identical to truncation.
    v.trunc() as i32
}

fn bad(stdin: &mut impl Read) {
    let mut data: f32 = 0.0;
    {
        match c_fgets(CHAR_ARRAY_SIZE, stdin) {
            Some(buf) => {
                data = c_atof(&buf) as f32;
            }
            None => {
                print_line("fgets() failed.");
            }
        }
    }
    {
        let result = f64_to_i32_c(100.0 / (data as f64));
        print_int_line(result);
    }
}

fn good_g2b() {
    let data: f32 = 2.0;
    let result = f64_to_i32_c(100.0 / (data as f64));
    print_int_line(result);
}

fn good_b2g(stdin: &mut impl Read) {
    let mut data: f32 = 0.0;
    {
        match c_fgets(CHAR_ARRAY_SIZE, stdin) {
            Some(buf) => {
                data = c_atof(&buf) as f32;
            }
            None => {
                print_line("fgets() failed.");
            }
        }
    }
    if (data as f64).abs() > 0.000001 {
        let result = f64_to_i32_c(100.0 / (data as f64));
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
}
