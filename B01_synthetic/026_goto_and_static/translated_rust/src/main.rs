use std::io::{self, Read};

static mut Y: i32 = 123;

fn multi_stage(x: i32, z: i32) -> i32 {
    let result;
    // Emulate goto-fail pattern with a loop-break block
    loop {
        if x != 1 {
            print!("Error: x != 1\n");
            result = 1;
            break;
        }
        if unsafe { Y } != 2 {
            print!("Error: x == 1 but y != 2\n");
            result = 2;
            break;
        }
        if z != 3 {
            print!("Error: x == 1 and y == 2, but z != 3\n");
            result = 3;
            break;
        }
        print!("Ok!\n");
        return 0;
    }
    // fail:
    print!("Operation failed\n");
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
    print!("Result: {}\n", result);
}
