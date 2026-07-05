

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::{Read, Write};

fn rust_foo(mut x: i32, mut y: i32) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    while x > 0 || y > 0 {
        let _ = writeln!(out, "loop");

        // In C, when (x == 1 && y == 4) we skip the label1 block on the
        // FIRST entry of this iteration. However, the `goto label1` from
        // the bottom of the loop must still execute the label1 block.
        // We therefore model the label1/label2 region as an inner loop
        // that we may re-enter via `continue`, tracking whether the
        // label1 block should be skipped on this pass.
        let mut skip_label1 = x == 1 && y == 4;

        loop {
            if !skip_label1 {
                if x > 0 {
                    let _ = writeln!(out, "x");
                    x -= 1;
                }
            }

            // label2:
            if y == 0 {
                break; // equivalent to `continue` of the outer while
            }
            let _ = writeln!(out, "y");
            y -= 1;

            if x < 3 {
                // `goto label1` \u{2014} re-run the label1 block on the next pass.
                skip_label1 = false;
                continue;
            }
            break;
        }
    }
    let _ = out.flush();
}

fn rust_read_two_ints() -> (i32, i32) {
    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);
    let mut iter = input.split_ascii_whitespace();
    let x: i32 = iter.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let y: i32 = iter.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    (x, y)
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> i32 {
    let (x, y) = rust_read_two_ints();
    rust_foo(x, y);
    let _ = std::io::stdout().flush();
    0
}

