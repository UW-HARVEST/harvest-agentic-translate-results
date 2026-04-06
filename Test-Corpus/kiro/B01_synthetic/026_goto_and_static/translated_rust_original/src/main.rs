use std::io::{self, Read};

static mut Y: i32 = 123;

fn multi_stage(x: i32, z: i32) -> i32 {
    let result;

    // Emulate goto-fail pattern with a loop-break block
    loop {
        if x != 1 {
            println!("Error: x != 1");
            result = 1;
            break;
        }
        if unsafe { Y } != 2 {
            println!("Error: x == 1 but y != 2");
            result = 2;
            break;
        }
        if z != 3 {
            println!("Error: x == 1 and y == 2, but z != 3");
            result = 3;
            break;
        }
        println!("Ok!");
        return 0;
    }

    // fail:
    println!("Operation failed");
    result
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut iter = input.split_whitespace();

    let x: i32 = iter.next().unwrap().parse().unwrap();
    let y: i32 = iter.next().unwrap().parse().unwrap();
    let z: i32 = iter.next().unwrap().parse().unwrap();

    unsafe { Y = y; }
    let result = multi_stage(x, z);
    println!("Result: {}", result);
}
