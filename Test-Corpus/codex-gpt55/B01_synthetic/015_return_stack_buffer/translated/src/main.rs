use std::io::{self, Read, Write};

fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        let mut stdout = io::stdout().lock();
        let _ = writeln!(stdout, "{line}");
    }
}

fn helper_bad() -> Option<&'static str> {
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

fn scanf_d(input: &[u8]) -> i32 {
    let mut pos = 0;
    while pos < input.len() && matches!(input[pos], b' ' | b'\n' | b'\r' | b'\t' | 0x0b | 0x0c) {
        pos += 1;
    }

    let mut negative = false;
    if pos < input.len() {
        if input[pos] == b'-' {
            negative = true;
            pos += 1;
        } else if input[pos] == b'+' {
            pos += 1;
        }
    }

    if pos >= input.len() || !input[pos].is_ascii_digit() {
        return 0;
    }

    let limit = if negative {
        (i64::MAX as u64) + 1
    } else {
        i64::MAX as u64
    };
    let mut value: u64 = 0;
    let mut saturated = false;

    while pos < input.len() && input[pos].is_ascii_digit() {
        let digit = (input[pos] - b'0') as u64;
        if !saturated {
            if value > (limit - digit) / 10 {
                value = limit;
                saturated = true;
            } else {
                value = value * 10 + digit;
            }
        }
        pos += 1;
    }

    let stored = if negative {
        if value == (i64::MAX as u64) + 1 {
            i64::MIN
        } else {
            -(value as i64)
        }
    } else {
        value as i64
    };

    stored as i32
}

fn main() {
    let mut input = Vec::new();
    let _ = io::stdin().read_to_end(&mut input);

    let x = scanf_d(&input);
    if x != 0 {
        good();
    } else {
        bad();
    }
}
