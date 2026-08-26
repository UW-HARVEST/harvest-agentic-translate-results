use std::io::{self, Read, Write};

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
    }
}

fn helper_bad() -> Option<&'static str> {
    // The C version returns the address of a local stack variable, which is
    // undefined behavior. With the original compiler this effectively returns
    // NULL, so `bad()` produces no output. Mimic that observable behavior.
    None
}

fn bad() {
    print_line(helper_bad());
}

fn helper_good1() -> Option<&'static str> {
    Some("helperGood1 string")
}

fn good() {
    print_line(helper_good1());
}

/// Mimics the C standard `scanf("%d", &x)` for reading a single integer from
/// stdin. Skips leading whitespace, then reads an optional sign followed by
/// digits. Returns the parsed value (or 0 if no digits were available — the
/// same as leaving the pre-initialized variable untouched).
fn scanf_int(buf: &[u8], pos: &mut usize) -> i32 {
    // Skip leading whitespace.
    while *pos < buf.len() && (buf[*pos] as char).is_ascii_whitespace() {
        *pos += 1;
    }

    if *pos >= buf.len() {
        return 0;
    }

    let start = *pos;
    if buf[*pos] == b'-' || buf[*pos] == b'+' {
        *pos += 1;
    }

    let digits_start = *pos;
    while *pos < buf.len() && (buf[*pos] as char).is_ascii_digit() {
        *pos += 1;
    }

    if *pos == digits_start {
        // No digits were consumed; scanf would not modify the variable.
        // Reset position to before any sign so subsequent reads see it.
        *pos = start;
        return 0;
    }

    let s = std::str::from_utf8(&buf[start..*pos]).unwrap_or("");
    s.parse::<i32>().unwrap_or_else(|_| {
        // Fall back to wrapping parse via i64 if overflow; default to 0.
        s.parse::<i64>().map(|v| v as i32).unwrap_or(0)
    })
}

fn main() {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).ok();

    let mut pos: usize = 0;
    let x: i32 = scanf_int(&input, &mut pos);

    if x != 0 {
        good();
    } else {
        bad();
    }

    io::stdout().flush().ok();
}
