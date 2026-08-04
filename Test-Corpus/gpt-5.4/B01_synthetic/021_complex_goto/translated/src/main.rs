use std::io::{self, Read};

fn foo(mut x: i32, mut y: i32) {
    while x > 0 || y > 0 {
        println!("loop");

        enum State {
            Label1,
            Label2,
        }

        let mut state = if x == 1 && y == 4 {
            State::Label2
        } else {
            State::Label1
        };

        loop {
            match state {
                State::Label1 => {
                    if x > 0 {
                        println!("x");
                        x -= 1;
                    }
                    state = State::Label2;
                }
                State::Label2 => {
                    if y == 0 {
                        break;
                    }
                    println!("y");
                    y -= 1;
                    if x < 3 {
                        state = State::Label1;
                    } else {
                        break;
                    }
                }
            }
        }
    }
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let mut parts = input.split_whitespace();
    let x = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
    let y = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
    foo(x, y);
}
