#[cfg(not(test))]
use std::io::{self, Read};
use std::os::raw::c_int;

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> c_int {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let mut iter = input.split_whitespace();
    let x: c_int = iter.next().map(|s| s.parse().unwrap_or(0)).unwrap_or(0);
    let y: c_int = iter.next().map(|s| s.parse().unwrap_or(0)).unwrap_or(0);
    let z: c_int = iter.next().map(|s| s.parse().unwrap_or(0)).unwrap_or(0);
    let result = multi_stage(x, y, z);
    println!("Result: {}", result);
    0
}

#[no_mangle]
pub extern "C" fn multi_stage(x: c_int, y: c_int, z: c_int) -> c_int {
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
