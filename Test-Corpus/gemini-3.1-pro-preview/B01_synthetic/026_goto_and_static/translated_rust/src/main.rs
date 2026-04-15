use std::io::{self, Read};
use std::sync::atomic::{AtomicI32, Ordering};

static Y: AtomicI32 = AtomicI32::new(123);

fn multi_stage(x: i32, z: i32) -> i32 {
    let mut result = 0;

    if x != 1 {
        println!("Error: x != 1");
        result = 1;
    } else if Y.load(Ordering::SeqCst) != 2 {
        println!("Error: x == 1 but y != 2");
        result = 2;
    } else if z != 3 {
        println!("Error: x == 1 and y == 2, but z != 3");
        result = 3;
    } else {
        println!("Ok!");
        return result;
    }

    println!("Operation failed");
    result
}

fn main() {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_ok() {
        let mut tokens = input.split_whitespace();

        let x: i32 = tokens.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let y_val: i32 = tokens.next().and_then(|s| s.parse().ok()).unwrap_or(123);
        let z: i32 = tokens.next().and_then(|s| s.parse().ok()).unwrap_or(0);

        Y.store(y_val, Ordering::SeqCst);

        let result = multi_stage(x, z);
        println!("Result: {}", result);
    }
}
