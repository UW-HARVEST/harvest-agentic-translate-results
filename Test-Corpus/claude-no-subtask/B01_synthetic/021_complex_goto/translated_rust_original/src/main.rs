use std::io::{self, Read, Write, BufWriter};

fn foo<W: Write>(mut x: i32, mut y: i32, out: &mut W) {
    // Translated from C with goto label1/label2 inside a while loop.
    'outer: while x > 0 || y > 0 {
        out.write_all(b"loop\n").unwrap();

        // If condition matches, we jump past label1 directly to label2.
        let mut skip_label1 = x == 1 && y == 4;

        loop {
            if !skip_label1 {
                // label1:
                if x > 0 {
                    out.write_all(b"x\n").unwrap();
                    x = x.wrapping_sub(1);
                }
            }
            // label2:
            skip_label1 = false;

            if y == 0 {
                // continue; (goes back to while loop test)
                continue 'outer;
            }
            out.write_all(b"y\n").unwrap();
            y = y.wrapping_sub(1);
            if x < 3 {
                // goto label1; (jump back into inner block at label1)
                continue;
            }
            // Falls out of the labels region; re-check the while condition.
            break;
        }
    }
}

fn main() {
    // Match C's `int x = 0, y = 0; scanf("%d %d", &x, &y);` behavior:
    // - scanf("%d") skips leading whitespace (including newlines) and parses
    //   an optional sign followed by decimal digits.
    // - If a conversion fails, the corresponding variable is left at its
    //   initial value (0).
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);

    let mut x: i32 = 0;
    let mut y: i32 = 0;

    let mut iter = input.split_ascii_whitespace();
    if let Some(tok) = iter.next() {
        if let Ok(v) = tok.parse::<i64>() {
            // C scanf with %d on overflow is undefined; emulate truncation.
            x = v as i32;
            if let Some(tok2) = iter.next() {
                if let Ok(v2) = tok2.parse::<i64>() {
                    y = v2 as i32;
                }
            }
        }
    }

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    foo(x, y, &mut out);
    out.flush().unwrap();
}
