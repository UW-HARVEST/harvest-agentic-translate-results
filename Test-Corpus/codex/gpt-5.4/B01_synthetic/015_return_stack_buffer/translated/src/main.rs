use std::io::{self, Read, Write};

fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        let mut stdout = io::stdout().lock();
        let _ = stdout.write_all(line.as_bytes());
        let _ = stdout.write_all(b"\n");
    }
}

fn helper_bad() -> Option<&'static str> {
    None
}

fn bad() {
    print_line(helper_bad());
}

fn helper_good1() -> &'static str {
    "helperGood1 string"
}

fn good() {
    print_line(Some(helper_good1()));
}

fn scanf_decimal_i32(input: &[u8]) -> Option<i32> {
    let mut idx = 0usize;

    while idx < input.len() && input[idx].is_ascii_whitespace() {
        idx += 1;
    }

    let mut negative = false;
    if idx < input.len() {
        match input[idx] {
            b'+' => idx += 1,
            b'-' => {
                negative = true;
                idx += 1;
            }
            _ => {}
        }
    }

    let digit_start = idx;
    let mut value: i64 = 0;
    while idx < input.len() && input[idx].is_ascii_digit() {
        value = value * 10 + i64::from(input[idx] - b'0');
        idx += 1;
    }

    if idx == digit_start {
        return None;
    }

    if negative {
        value = -value;
    }

    i32::try_from(value).ok()
}

fn main() {
    let mut x = 0i32;
    let mut input = Vec::new();
    let _ = io::stdin().lock().read_to_end(&mut input);

    if let Some(parsed) = scanf_decimal_i32(&input) {
        x = parsed;
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
