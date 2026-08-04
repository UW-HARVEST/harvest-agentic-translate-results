use std::io::{self, Read};
use std::sync::atomic::{AtomicI32, Ordering};

static Y: AtomicI32 = AtomicI32::new(123);

fn multi_stage(x: i32, z: i32) -> i32 {
    let result;
    if x != 1 {
        println!("Error: x != 1");
        result = 1;
        println!("Operation failed");
        return result;
    }

    if Y.load(Ordering::SeqCst) != 2 {
        println!("Error: x == 1 but y != 2");
        result = 2;
        println!("Operation failed");
        return result;
    }

    if z != 3 {
        println!("Error: x == 1 and y == 2, but z != 3");
        result = 3;
        println!("Operation failed");
        return result;
    }

    println!("Ok!");
    0
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut values = input.split_whitespace();

    let x = values.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
    let y = values.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
    let z = values.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);

    Y.store(y, Ordering::SeqCst);
    let result = multi_stage(x, z);
    println!("Result: {}", result);
}
