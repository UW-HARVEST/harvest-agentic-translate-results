use std::io::{self, Read};

static mut Y: i32 = 123;

pub fn multi_stage(x: i32, z: i32) -> i32 {
    let result;

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

    println!("Operation failed");
    result
}

pub fn driver_main() {
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

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    driver_main();
    0
}
