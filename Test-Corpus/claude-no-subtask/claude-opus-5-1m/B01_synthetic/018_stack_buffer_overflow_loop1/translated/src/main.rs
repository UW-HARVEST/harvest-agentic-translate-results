use std::io::{self, Read, Write};

fn print_int_line(int_number: i32) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = writeln!(handle, "{}", int_number);
}

fn bad() {
    // Mimics the C `bad()` function. The C code allocates 10 bytes via
    // `alloca(10)` then writes 10 ints into it (a buffer overflow), but
    // because the `source` array is zero-initialized, `data[0]` is 0 and
    // that is what gets printed. We reproduce the observable output here.
    let mut data: [i32; 10] = [0; 10];
    let source: [i32; 10] = [0; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

fn good() {
    let mut data: [i32; 10] = [0; 10];
    let source: [i32; 10] = [0; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    print_int_line(data[0]);
}

/// Reads an integer from stdin in a manner equivalent to C's `scanf("%d", &x)`.
/// Returns Some(value) on successful parse, None if no integer could be read.
fn scan_int() -> Option<i32> {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return None;
    }
    let bytes = input.as_bytes();
    let mut i = 0;
    // Skip leading whitespace (as scanf with %d does).
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    let start = i;
    // Optional sign
    if bytes[i] == b'+' || bytes[i] == b'-' {
        i += 1;
    }
    let digits_start = i;
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        i += 1;
    }
    if i == digits_start {
        return None;
    }
    let s = std::str::from_utf8(&bytes[start..i]).ok()?;
    // Use wrapping behavior to match C's signed int parsing in spirit.
    s.parse::<i64>().ok().map(|v| v as i32)
}

fn main() {
    let mut x: i32 = 0;
    if let Some(v) = scan_int() {
        x = v;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
