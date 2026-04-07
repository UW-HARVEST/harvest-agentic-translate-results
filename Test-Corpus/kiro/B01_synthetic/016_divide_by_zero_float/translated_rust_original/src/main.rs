use std::io::{self, BufRead};

/// Mimics C's (int)(double) cast: UB for out-of-range values produces
/// 0x80000000 on x86 (cvttsd2si behavior).
#[inline]
fn c_double_to_i32(v: f64) -> i32 {
    if v.is_nan() || v.is_infinite() || v > i32::MAX as f64 || v < i32::MIN as f64 {
        i32::MIN
    } else {
        v as i32
    }
}

const CHAR_ARRAY_SIZE: usize = 20;

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(n: i32) {
    println!("{}", n);
}

/// Mimics C atof: parse leading numeric portion, return 0.0 on failure.
fn c_atof(s: &str) -> f64 {
    let s = s.trim_start();
    // Find longest prefix that parses as f64
    let mut last_ok = 0.0_f64;
    let mut found = false;
    for i in 1..=s.len() {
        if let Ok(v) = s[..i].parse::<f64>() {
            last_ok = v;
            found = true;
        } else if found {
            break;
        }
    }
    last_ok
}

/// Mimics fgets(buf, CHAR_ARRAY_SIZE, stdin): reads up to CHAR_ARRAY_SIZE-1
/// bytes including the newline. Returns None on EOF/error.
fn fgets_stdin() -> Option<String> {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buf = Vec::new();
    let limit = CHAR_ARRAY_SIZE - 1; // 19 bytes max like C fgets
    loop {
        let available = match handle.fill_buf() {
            Ok(b) if b.is_empty() => {
                if buf.is_empty() {
                    return None;
                }
                break;
            }
            Ok(b) => b,
            Err(_) => {
                if buf.is_empty() {
                    return None;
                }
                break;
            }
        };
        let remaining = limit - buf.len();
        let to_copy = available.len().min(remaining);
        let chunk = &available[..to_copy];
        if let Some(nl) = chunk.iter().position(|&b| b == b'\n') {
            buf.extend_from_slice(&chunk[..=nl]);
            handle.consume(nl + 1);
            break;
        }
        buf.extend_from_slice(chunk);
        handle.consume(to_copy);
        if buf.len() >= limit {
            break;
        }
    }
    Some(String::from_utf8_lossy(&buf).into_owned())
}

fn bad() {
    let mut data: f32 = 0.0;
    if let Some(input) = fgets_stdin() {
        data = c_atof(&input) as f32;
    } else {
        print_line("fgets() failed.");
    }
    let result = c_double_to_i32(100.0_f64 / data as f64);
    print_int_line(result);
}

fn good_g2b() {
    let data: f32 = 2.0;
    let result = c_double_to_i32(100.0_f64 / data as f64);
    print_int_line(result);
}

fn good_b2g() {
    let mut data: f32 = 0.0;
    if let Some(input) = fgets_stdin() {
        data = c_atof(&input) as f32;
    } else {
        print_line("fgets() failed.");
    }
    if (data as f64).abs() > 0.000001 {
        let result = c_double_to_i32(100.0_f64 / data as f64);
        print_int_line(result);
    } else {
        print_line("This would result in a divide by zero");
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
