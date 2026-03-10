use std::io::{self, Read};

static mut Y: i32 = 123;

fn multi_stage(x: i32, z: i32) -> i32 {
    let result;
    if x != 1 {
        println!("Error: x != 1");
        result = 1;
    } else if unsafe { Y } != 2 {
        println!("Error: x == 1 but y != 2");
        result = 2;
    } else if z != 3 {
        println!("Error: x == 1 and y == 2, but z != 3");
        result = 3;
    } else {
        println!("Ok!");
        return 0;
    }
    println!("Operation failed");
    result
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut nums = input.split_whitespace().map(|s| s.parse::<i32>().unwrap());
    let x = nums.next().unwrap();
    unsafe { Y = nums.next().unwrap() };
    let z = nums.next().unwrap();
    let result = multi_stage(x, z);
    println!("Result: {}", result);
}
