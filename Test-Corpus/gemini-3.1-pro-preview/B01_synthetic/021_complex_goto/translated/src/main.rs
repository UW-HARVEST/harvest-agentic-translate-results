use std::io::{self, BufRead};

fn foo(mut x: i32, mut y: i32) {
    while x > 0 || y > 0 {
        println!("loop");

        let mut goto_label2 = x == 1 && y == 4;

        loop {
            if !goto_label2 {
                if x > 0 {
                    println!("x");
                    x -= 1;
                }
            }
            goto_label2 = false;

            if y == 0 {
                break;
            }
            println!("y");
            y -= 1;
            if x < 3 {
                continue;
            }
            break;
        }
    }
}

fn main() {
    let stdin = io::stdin();
    let mut iterator = stdin.lock().lines().flat_map(|line| {
        line.unwrap_or_default()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    });

    let x: i32 = iterator.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let y: i32 = iterator.next().and_then(|s| s.parse().ok()).unwrap_or(0);

    foo(x, y);
}
