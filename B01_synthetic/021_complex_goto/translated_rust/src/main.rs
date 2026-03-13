use std::io::{self, Read};

fn foo(mut x: i32, mut y: i32) {
    // Model the goto control flow with a state machine.
    // States correspond to entry points in the C code:
    //   WhileCheck -> top of while loop (condition + "loop" print)
    //   Label1     -> goto label1 target
    //   Label2     -> goto label2 target
    enum State { WhileCheck, Label1, Label2 }
    let mut state = State::WhileCheck;
    loop {
        match state {
            State::WhileCheck => {
                if !(x > 0 || y > 0) { break; }
                println!("loop");
                if x == 1 && y == 4 {
                    state = State::Label2;
                    continue;
                }
                state = State::Label1;
                continue;
            }
            State::Label1 => {
                if x > 0 {
                    println!("x");
                    x -= 1;
                }
                state = State::Label2;
                continue;
            }
            State::Label2 => {
                if y == 0 {
                    state = State::WhileCheck;
                    continue;
                }
                println!("y");
                y -= 1;
                if x < 3 {
                    state = State::Label1;
                    continue;
                }
                state = State::WhileCheck;
            }
        }
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut iter = input.split_whitespace();
    let x: i32 = iter.next().unwrap_or("0").parse().unwrap_or(0);
    let y: i32 = iter.next().unwrap_or("0").parse().unwrap_or(0);
    foo(x, y);
}
