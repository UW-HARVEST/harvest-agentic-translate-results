//! Translation of c_src/src/main.c to Rust.

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use driver::process_decisions;

fn read_line(stdin: &mut impl BufRead) -> Option<String> {
    let mut line = String::new();
    match stdin.read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => Some(line),
        Err(_) => None,
    }
}

/// Mimic C's atoi: parse a leading optional sign and digits, ignore leading whitespace,
/// return 0 for non-numeric input.
fn atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r' || bytes[i] == 0x0b || bytes[i] == 0x0c) {
        i += 1;
    }
    let mut neg = false;
    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
        neg = bytes[i] == b'-';
        i += 1;
    }
    let mut result: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        result = result.saturating_mul(10).saturating_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    if neg {
        result = -result;
    }
    if result > i32::MAX as i64 {
        i32::MAX
    } else if result < i32::MIN as i64 {
        i32::MIN
    } else {
        result as i32
    }
}

fn main() -> ExitCode {
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    let operation_line = match read_line(&mut handle) {
        Some(l) => l,
        None => {
            let _ = writeln!(io::stderr(), "Error reading operation");
            return ExitCode::from(1);
        }
    };
    let operation = atoi(&operation_line);

    let param_line = match read_line(&mut handle) {
        Some(l) => l,
        None => {
            let _ = writeln!(io::stderr(), "Error reading parameter");
            return ExitCode::from(1);
        }
    };
    let param = atoi(&param_line);

    let decision_line = match read_line(&mut handle) {
        Some(l) => l,
        None => {
            let _ = writeln!(io::stderr(), "Error reading decision string");
            return ExitCode::from(1);
        }
    };

    // Strip trailing newline (and trailing CR if present)
    let mut bytes: Vec<u8> = decision_line.into_bytes();
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
    }
    let len = bytes.len();

    let result = process_decisions(&bytes, len, operation, param);

    println!("{}", result);

    ExitCode::from(0)
}
