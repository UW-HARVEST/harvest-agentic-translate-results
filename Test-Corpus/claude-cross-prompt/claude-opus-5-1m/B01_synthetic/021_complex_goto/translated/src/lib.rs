use std::ffi::c_int;
use std::io::{self, Read, Write};

fn foo(mut x: c_int, mut y: c_int) {
    'outer: while x > 0 || y > 0 {
        println!("loop");

        // If `x == 1 && y == 4`, the C code does `goto label2`, which skips
        // the label1 block on the first pass through the inner loop.
        let mut skip_label1 = x == 1 && y == 4;

        loop {
            if !skip_label1 {
                // label1:
                if x > 0 {
                    println!("x");
                    x -= 1;
                }
            }
            skip_label1 = false;

            // label2:
            if y == 0 {
                // C `continue` continues the outer `while` loop.
                continue 'outer;
            }
            println!("y");
            y -= 1;
            if x < 3 {
                // C `goto label1` jumps back to label1 within the same
                // iteration of the outer loop.
                continue;
            }
            // Fall out of the inner loop to re-evaluate the outer condition.
            break;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> c_int {
    let mut x: c_int = 0;
    let mut y: c_int = 0;

    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);

    // Emulate `scanf("%d %d", &x, &y)`: skip leading whitespace, parse an
    // int, and on failure leave the prior value (initialized to 0).
    let mut iter = input.split_ascii_whitespace();
    if let Some(tok) = iter.next() {
        if let Ok(v) = tok.parse::<c_int>() {
            x = v;
            if let Some(tok2) = iter.next() {
                if let Ok(v2) = tok2.parse::<c_int>() {
                    y = v2;
                }
            }
        }
    }

    foo(x, y);

    let _ = io::stdout().flush();
    0
}
