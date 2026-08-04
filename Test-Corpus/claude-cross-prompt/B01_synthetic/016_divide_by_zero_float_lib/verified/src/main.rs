use std::io::{self, Read, Write};

fn print_line(line: &str) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(line.as_bytes());
    let _ = out.write_all(b"\n");
}

fn print_int_line(int_number: i32) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", int_number);
}

/// Mimic C's `(int)` cast from a double on x86-64 (cvttsd2si):
/// - In-range finite values truncate toward zero.
/// - NaN or out-of-range values produce the "indefinite integer" 0x80000000 (i32::MIN).
fn f64_to_int_c(val: f64) -> i32 {
    if val.is_nan() {
        return i32::MIN;
    }
    // The exact threshold for cvttsd2si: any value > 2147483647.0 or < -2147483648.0
    // returns INT_MIN. Note 2147483648.0 is exactly representable in f64 but not as i32.
    if val >= 2147483648.0_f64 || val < -2147483648.0_f64 {
        return i32::MIN;
    }
    val as i32
}

fn bad(data: f32) {
    // C: int result = (int)(100.0 / data);
    // 100.0 is a double in C, so the division is performed in double precision.
    let result = f64_to_int_c(100.0_f64 / (data as f64));
    print_int_line(result);
}

fn good_g2b() {
    let data: f32 = 2.0_f32;
    let result = f64_to_int_c(100.0_f64 / (data as f64));
    print_int_line(result);
}

fn good_b2g(data: f32) {
    // C: if (fabs(data) > 0.000001) — fabs takes a double, so data is promoted.
    if (data as f64).abs() > 0.000001_f64 {
        let result = f64_to_int_c(100.0_f64 / (data as f64));
        print_int_line(result);
    } else {
        print_line("This would result in a divide by zero");
    }
}

fn good(data: f32) {
    good_g2b();
    good_b2g(data);
}

fn driver(good_data: f32, bad_data: f32) {
    print_line("Calling good()...");
    good(good_data);
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad(bad_data);
    print_line("Finished bad()");
}

/// Read whitespace-separated tokens from stdin, mimicking scanf's "%f" behavior
/// which skips leading whitespace (including newlines) and reads a float token.
fn read_two_floats() -> (f32, f32) {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let mut tokens = input.split_ascii_whitespace();
    let a: f32 = tokens
        .next()
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.0);
    let b: f32 = tokens
        .next()
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.0);
    (a, b)
}

fn main() {
    let (good_data, bad_data) = read_two_floats();
    driver(good_data, bad_data);
}
