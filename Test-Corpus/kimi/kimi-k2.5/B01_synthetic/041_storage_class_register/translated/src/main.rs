use std::io;

fn driver(x: i32) {
    let y = 2 * x;
    let y = y + 300;
    println!("{}", y);
}

fn main() {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let x: i32 = input.trim().parse().unwrap();
    driver(x);
}