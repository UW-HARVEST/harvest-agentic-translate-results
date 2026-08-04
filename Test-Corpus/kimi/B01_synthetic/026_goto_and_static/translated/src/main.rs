use std::io;

static mut Y: i32 = 123;

fn multi_stage(x: i32, z: i32) -> i32 {
    let mut result = 0;
    if x != 1 {
        println!("Error: x != 1");
        result = 1;
        return fail(result);
    }

    unsafe {
        if Y != 2 {
            println!("Error: x == 1 but y != 2");
            result = 2;
            return fail(result);
        }
    }

    if z != 3 {
        println!("Error: x == 1 and y == 2, but z != 3");
        result = 3;
        return fail(result);
    }

    println!("Ok!");
    result
}

fn fail(result: i32) -> i32 {
    println!("Operation failed");
    result
}

fn main() {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let values: Vec<i32> = input
        .split_whitespace()
        .map(|s| s.parse().unwrap())
        .collect();
    let x = values[0];
    unsafe {
        Y = values[1];
    }
    let z = values[2];
    let result = multi_stage(x, z);
    println!("Result: {}", result);
}