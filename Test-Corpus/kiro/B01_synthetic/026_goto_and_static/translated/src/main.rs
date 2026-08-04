use std::io::{self, Read};

fn multi_stage(x: i32, y: i32, z: i32) -> i32 {
    let result;
    let failed;

    if x != 1 {
        println!("Error: x != 1");
        result = 1;
        failed = true;
    } else if y != 2 {
        println!("Error: x == 1 but y != 2");
        result = 2;
        failed = true;
    } else if z != 3 {
        println!("Error: x == 1 and y == 2, but z != 3");
        result = 3;
        failed = true;
    } else {
        println!("Ok!");
        result = 0;
        failed = false;
    }

    if failed {
        println!("Operation failed");
    }
    result
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut iter = input.split_whitespace();

    let x: i32 = iter.next().map(|s| s.parse().unwrap_or(0)).unwrap_or(0);
    let y: i32 = iter.next().map(|s| s.parse().unwrap_or(0)).unwrap_or(0);
    let z: i32 = iter.next().map(|s| s.parse().unwrap_or(0)).unwrap_or(0);

    let result = multi_stage(x, y, z);
    println!("Result: {}", result);
}
