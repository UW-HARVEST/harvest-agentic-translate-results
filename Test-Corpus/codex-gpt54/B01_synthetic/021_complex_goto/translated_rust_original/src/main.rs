use std::io::{self, Read, Write};

fn skip_ascii_whitespace(bytes: &[u8], mut idx: usize) -> usize {
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }
    idx
}

fn parse_c_int(bytes: &[u8], start: usize) -> Option<(i32, usize)> {
    let mut idx = skip_ascii_whitespace(bytes, start);
    let sign_idx = idx;

    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        idx += 1;
    }

    let digits_start = idx;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
    }

    if digits_start == idx {
        return None;
    }

    let token = std::str::from_utf8(&bytes[sign_idx..idx]).ok()?;
    let value_i64 = token.parse::<i64>().ok()?;
    let value = i32::try_from(value_i64).ok()?;
    Some((value, idx))
}

fn foo(mut x: i32, mut y: i32, out: &mut impl Write) -> io::Result<()> {
    'outer: while x > 0 || y > 0 {
        out.write_all(b"loop\n")?;

        let mut at_label2 = false;
        if x == 1 && y == 4 {
            at_label2 = true;
        }

        loop {
            if !at_label2 {
                if x > 0 {
                    out.write_all(b"x\n")?;
                    x = x.wrapping_sub(1);
                }
            }

            at_label2 = false;
            if y == 0 {
                continue 'outer;
            }

            out.write_all(b"y\n")?;
            y = y.wrapping_sub(1);

            if x < 3 {
                continue;
            }

            break;
        }
    }

    Ok(())
}

fn run() -> io::Result<()> {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input)?;

    let mut x = 0_i32;
    let mut y = 0_i32;
    let mut idx = 0_usize;

    if let Some((parsed_x, next_idx)) = parse_c_int(&input, idx) {
        x = parsed_x;
        idx = next_idx;

        if let Some((parsed_y, _)) = parse_c_int(&input, idx) {
            y = parsed_y;
        }
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    foo(x, y, &mut out)
}

fn main() {
    if let Err(err) = run() {
        if err.kind() != io::ErrorKind::BrokenPipe {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}
