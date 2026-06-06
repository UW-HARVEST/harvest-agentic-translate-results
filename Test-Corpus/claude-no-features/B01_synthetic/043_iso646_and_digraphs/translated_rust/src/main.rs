use std::io::{self, Read, Write};

fn driver(x: i32, y: i32) {
    // x bitor compl y  =>  x | ~y
    let result: i32 = x | !y;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    // printf("%d", result); puts("");
    write!(out, "{}", result).ok();
    writeln!(out).ok();
}

/// Read all of stdin into a String, mimicking scanf's tolerance for whitespace
/// (spaces, tabs, newlines) when parsing integers with %d.
fn read_all_stdin() -> String {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s).ok();
    s
}

/// Mimic scanf("%d", ...). Returns the parsed value and the number of bytes consumed
/// (including any leading whitespace and the digits). If no integer could be parsed
/// (EOF or invalid), returns None.
///
/// scanf %d:
///   - Skips leading whitespace
///   - Optionally accepts leading '+' or '-'
///   - Reads decimal digits until the first non-digit
fn scan_i32(input: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip whitespace
    while *pos < input.len() && (input[*pos] as char).is_ascii_whitespace() {
        *pos += 1;
    }
    if *pos >= input.len() {
        return None;
    }

    let start = *pos;
    let mut idx = *pos;

    // Optional sign
    if idx < input.len() && (input[idx] == b'+' || input[idx] == b'-') {
        idx += 1;
    }

    let digits_start = idx;
    while idx < input.len() && (input[idx] as char).is_ascii_digit() {
        idx += 1;
    }

    if idx == digits_start {
        // No digits found; scanf would fail. Return None and don't advance pos.
        return None;
    }

    let num_str = std::str::from_utf8(&input[start..idx]).ok()?;
    *pos = idx;

    // C scanf %d behavior on overflow is undefined; we use wrapping parse
    // by parsing as i64 and casting to i32 to mirror typical 32-bit int wrap.
    // For well-formed inputs that fit in i32, this matches.
    match num_str.parse::<i32>() {
        Ok(v) => Some(v),
        Err(_) => {
            // Fallback: attempt i64 parse and truncate (best-effort)
            match num_str.parse::<i64>() {
                Ok(v) => Some(v as i32),
                Err(_) => None,
            }
        }
    }
}

fn main() {
    let input = read_all_stdin();
    let bytes = input.as_bytes();
    let mut pos: usize = 0;

    let mut x: i32 = 0;
    let mut y: i32 = 0;

    if let Some(v) = scan_i32(bytes, &mut pos) {
        x = v;
    }
    if let Some(v) = scan_i32(bytes, &mut pos) {
        y = v;
    }

    driver(x, y);

    // Flush stdout to ensure output is written before exit.
    io::stdout().flush().ok();
}
