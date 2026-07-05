
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::io::{self, Read, Write};

unsafe extern "C" {
    static mut y: std::os::raw::c_int;
}

pub fn rust_get_y() -> std::os::raw::c_int {
    unsafe { y }
}

pub fn rust_set_y(val: std::os::raw::c_int) {
    unsafe { y = val; }
}

enum StageError {
    XNotOne,
    YNotTwo,
    ZNotThree,
}

fn multi_stage(x: i32, z: i32) -> Result<(), StageError> {
    if x != 1 {
        return Err(StageError::XNotOne);
    }
    if rust_get_y() != 2 {
        return Err(StageError::YNotTwo);
    }
    if z != 3 {
        return Err(StageError::ZNotThree);
    }
    Ok(())
}

fn run_multi_stage(x: i32, z: i32) -> i32 {
    match multi_stage(x, z) {
        Ok(()) => {
            println!("Ok!");
            0
        }
        Err(e) => {
            let code = match e {
                StageError::XNotOne => {
                    println!("Error: x != 1");
                    1
                }
                StageError::YNotTwo => {
                    println!("Error: x == 1 but y != 2");
                    2
                }
                StageError::ZNotThree => {
                    println!("Error: x == 1 and y == 2, but z != 3");
                    3
                }
            };
            println!("Operation failed");
            code
        }
    }
}

fn read_three_ints() -> (i32, i32, i32) {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer).ok();

    let mut values = buffer
        .split_whitespace()
        .filter_map(|tok| tok.parse::<i32>().ok());

    let x = values.next().unwrap_or(0);
    let y_val = values.next().unwrap_or_else(rust_get_y);
    let z = values.next().unwrap_or(0);

    (x, y_val, z)
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> std::os::raw::c_int {
    let (x, y_val, z) = read_three_ints();
    rust_set_y(y_val);

    let result = run_multi_stage(x, z);
    println!("Result: {}", result);
    io::stdout().flush().ok();
    0
}
