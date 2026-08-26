use std::io::{self, Read, Write};

fn scan_ints(input: &[u8]) -> (i32, i32) {
    let mut values = [0_i32; 2];
    let mut pos = 0;

    for slot in &mut values {
        while pos < input.len() && input[pos].is_ascii_whitespace() {
            pos += 1;
        }

        if pos >= input.len() {
            break;
        }

        let mut sign = 1_i64;
        if input[pos] == b'+' || input[pos] == b'-' {
            if input[pos] == b'-' {
                sign = -1;
            }
            pos += 1;
        }

        if pos >= input.len() || !input[pos].is_ascii_digit() {
            break;
        }

        let mut value = 0_i64;
        while pos < input.len() && input[pos].is_ascii_digit() {
            value = value * 10 + i64::from(input[pos] - b'0');
            pos += 1;
        }

        *slot = (sign * value) as i32;
    }

    (values[0], values[1])
}

fn foo(mut x: i32, mut y: i32, out: &mut impl Write) {
    while x > 0 || y > 0 {
        let _ = writeln!(out, "loop");

        let mut at_label2 = x == 1 && y == 4;

        loop {
            if !at_label2 && x > 0 {
                let _ = writeln!(out, "x");
                x -= 1;
            }
            at_label2 = false;

            if y == 0 {
                break;
            }

            let _ = writeln!(out, "y");
            y -= 1;

            if x < 3 {
                continue;
            }

            break;
        }
    }
}

fn main() {
    let mut input = Vec::new();
    let _ = io::stdin().read_to_end(&mut input);

    let (x, y) = scan_ints(&input);
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    foo(x, y, &mut out);
}
