// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Reproduces byte-identical output.

use std::io::{self, Read, Write};

static mut Y: i32 = 123;

fn multi_stage(x: i32, z: i32) -> i32 {
    let mut result: i32 = 0;
    // Mirror C control flow with goto fail
    let mut failed = false;
    loop {
        if x != 1 {
            println!("Error: x != 1");
            result = 1;
            failed = true;
            break;
        }

        // SAFETY: single-threaded program; mirrors the static `y` in C
        let y_val = unsafe { Y };
        if y_val != 2 {
            println!("Error: x == 1 but y != 2");
            result = 2;
            failed = true;
            break;
        }

        if z != 3 {
            println!("Error: x == 1 and y == 2, but z != 3");
            result = 3;
            failed = true;
            break;
        }

        println!("Ok!");
        return result;
    }

    if failed {
        println!("Operation failed");
    }
    result
}

/// Mimic scanf("%d %d %d", ...). Reads up to 3 integers from stdin,
/// skipping whitespace. If parsing fails or input ends before all are
/// read, the corresponding output variable remains unchanged.
fn read_three_ints(x: &mut i32, y: &mut i32, z: &mut i32) {
    let mut buf = String::new();
    if io::stdin().read_to_string(&mut buf).is_err() {
        return;
    }
    let mut iter = buf.split_ascii_whitespace();
    let targets: [&mut i32; 3] = [x, y, z];
    for target in targets {
        match iter.next() {
            Some(tok) => match tok.parse::<i32>() {
                Ok(v) => *target = v,
                Err(_) => return,
            },
            None => return,
        }
    }
}

fn main() {
    let mut x: i32 = 0;
    let mut z: i32 = 0;
    // SAFETY: single-threaded program; mirrors C's static `y`
    unsafe {
        read_three_ints(&mut x, &mut Y, &mut z);
    }
    let result = multi_stage(x, z);
    println!("Result: {}", result);
    let _ = io::stdout().flush();
}
