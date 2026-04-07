use std::io::{self, Read};
use std::mem::MaybeUninit;

#[no_mangle]
pub extern "C" fn printIntPtrLine(int_number: *const i32) {
    unsafe {
        println!("{}", *int_number);
    }
}

#[no_mangle]
pub extern "C" fn bad() {
    unsafe {
        let data: MaybeUninit<*const i32> = MaybeUninit::uninit();
        printIntPtrLine(data.assume_init());
    }
}

#[no_mangle]
pub extern "C" fn good() {
    let data: i32 = 5;
    let data_addr: *const i32 = &data;
    printIntPtrLine(data_addr);
}

#[no_mangle]
pub extern "C" fn main() -> i32 {
    let mut x: i32 = 0;
    let mut input = String::new();
    let mut buf = [0u8; 1];
    loop {
        match io::stdin().read(&mut buf) {
            Ok(0) => break,
            Ok(_) => {
                let c = buf[0] as char;
                if !c.is_ascii_whitespace() {
                    input.push(c);
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if !input.is_empty() {
        loop {
            match io::stdin().read(&mut buf) {
                Ok(0) => break,
                Ok(_) => {
                    let c = buf[0] as char;
                    if c.is_ascii_whitespace() {
                        break;
                    }
                    input.push(c);
                }
                Err(_) => break,
            }
        }
    }
    if let Ok(val) = input.parse::<i32>() {
        x = val;
    }
    if x != 0 {
        good();
    } else {
        bad();
    }
    0
}
