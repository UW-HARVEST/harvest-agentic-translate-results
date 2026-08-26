use std::io;

fn foo(mut x: i32, mut y: i32) {
    while x > 0 || y > 0 {
        println!("loop");

        if x == 1 && y == 4 {
            if y == 0 {
                continue;
            }
            println!("y");
            y -= 1;
            if x < 3 {
                if x > 0 {
                    println!("x");
                    x -= 1;
                }
            }
            continue;
        }

        'label1: loop {
            if x > 0 {
                println!("x");
                x -= 1;
            }

            if y == 0 {
                break;
            }
            println!("y");
            y -= 1;
            if x < 3 {
                continue 'label1;
            }
            break;
        }
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let mut parts = input.split_whitespace();
    let x: i32 = parts.next().unwrap().parse().unwrap();
    let y: i32 = parts.next().unwrap().parse().unwrap();
    foo(x, y);
}