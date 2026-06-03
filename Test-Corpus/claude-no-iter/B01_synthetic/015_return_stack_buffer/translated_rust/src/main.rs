// Translation of c_src/src/main.c to Rust.
//
// The original C contains a deliberate use-after-scope bug in helperBad():
// it returns the address of a local stack array. When compiled (gcc -O0/-O2),
// the resulting pointer effectively yields NULL (the function ends up returning
// 0 in %rax), so printLine() sees a NULL pointer and prints nothing. We
// reproduce that observable behavior exactly: when the "bad" branch is taken,
// no output is produced.

use std::io::{self, Read, Write};

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        // C: printf("%s\n", line);
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(s.as_bytes());
        let _ = handle.write_all(b"\n");
    }
}

fn helper_bad() -> Option<&'static str> {
    // The C version returns a pointer to a stack-allocated array, which is
    // undefined behavior. In practice (with gcc), the helper function returns
    // 0 in %rax, so the caller receives a NULL pointer. Reproduce that:
    None
}

fn bad() {
    print_line(helper_bad());
}

fn helper_good1() -> Option<&'static str> {
    // C: static char charString[] = "helperGood1 string"; return charString;
    Some("helperGood1 string")
}

fn good() {
    print_line(helper_good1());
}

/// Emulate C's `scanf("%d", &x)`: skip leading whitespace, then parse an
/// optional sign followed by decimal digits. Returns the parsed value, or
/// `None` if no integer could be parsed (matching scanf's failure mode where
/// the destination variable is left unchanged).
fn scanf_int(buf: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip leading whitespace (matches isspace: space, tab, newline, vtab,
    // form feed, carriage return).
    while *pos < buf.len() {
        let c = buf[*pos];
        if c == b' ' || c == b'\t' || c == b'\n' || c == 0x0b || c == 0x0c || c == b'\r' {
            *pos += 1;
        } else {
            break;
        }
    }

    if *pos >= buf.len() {
        return None;
    }

    let start = *pos;
    // Optional sign
    if buf[*pos] == b'+' || buf[*pos] == b'-' {
        *pos += 1;
    }

    let digits_start = *pos;
    while *pos < buf.len() && buf[*pos].is_ascii_digit() {
        *pos += 1;
    }

    if *pos == digits_start {
        // No digits found; restore position and report failure.
        *pos = start;
        return None;
    }

    // Parse the matched span as i32 with C-style wraparound on overflow.
    let s = std::str::from_utf8(&buf[start..*pos]).ok()?;
    // C scanf with %d on overflow is undefined; use wrapping parse via i64
    // fallback then truncate to i32 to closely mirror typical glibc behavior
    // for in-range values. For out-of-range we fall back to 0.
    if let Ok(v) = s.parse::<i32>() {
        Some(v)
    } else if let Ok(v) = s.parse::<i64>() {
        Some(v as i32)
    } else {
        Some(0)
    }
}

fn main() {
    // Read all of stdin so we can mimic scanf's "read across newlines" behavior.
    let mut buf = Vec::new();
    let _ = io::stdin().read_to_end(&mut buf);

    let mut pos = 0usize;
    let mut x: i32 = 0;
    if let Some(v) = scanf_int(&buf, &mut pos) {
        x = v;
    }
    // If scanf fails, x remains 0 (matching the C initializer `int x = 0;`).

    if x != 0 {
        good();
    } else {
        bad();
    }
}
