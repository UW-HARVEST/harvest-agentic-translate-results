use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let x: i32 = input.trim().parse().unwrap();

    if x != 0 {
        driver::good();
    } else {
        driver::bad();
    }
}
