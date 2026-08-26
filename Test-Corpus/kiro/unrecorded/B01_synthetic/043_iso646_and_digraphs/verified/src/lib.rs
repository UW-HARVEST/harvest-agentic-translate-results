#[cfg(not(test))]
use std::io::{self, Read};

#[no_mangle]
pub extern "C" fn driver(x: i32, y: i32) {
    let result: i32 = x | !y;
    println!("{}", result);
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> i32 {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut iter = input.split_whitespace();
    let x: i32 = iter.next().unwrap().parse().unwrap();
    let y: i32 = iter.next().unwrap().parse().unwrap();
    driver(x, y);
    0
}
