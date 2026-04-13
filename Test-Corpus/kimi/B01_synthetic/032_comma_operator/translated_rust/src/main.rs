use std::io;

fn driver(x: i32) {
    let mut j = 0;
    for i in 0..x {
        println!("{} {}", i, j);
        j += 2;
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let x: i32 = input.trim().parse().unwrap();
    driver(x);
}