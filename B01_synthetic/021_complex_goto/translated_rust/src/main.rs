use std::io::{self, Read};

fn foo(mut x: i32, mut y: i32) {
    'outer: loop {
        if !(x > 0 || y > 0) {
            break;
        }
        println!("loop");

        let mut skip_label1 = x == 1 && y == 4;

        'label1: loop {
            if !skip_label1 {
                if x > 0 {
                    println!("x");
                    x -= 1;
                }
            }
            skip_label1 = false;

            // label2:
            if y == 0 {
                continue 'outer;
            }
            println!("y");
            y -= 1;
            if x < 3 {
                continue 'label1;
            }
            break 'label1;
        }
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut iter = input.split_whitespace();
    let x: i32 = iter.next().map(|s| s.parse().unwrap_or(0)).unwrap_or(0);
    let y: i32 = iter.next().map(|s| s.parse().unwrap_or(0)).unwrap_or(0);
    foo(x, y);
}
