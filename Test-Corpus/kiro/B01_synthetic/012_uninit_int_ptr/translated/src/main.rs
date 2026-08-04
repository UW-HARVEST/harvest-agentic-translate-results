use std::io::{self, Read};
use std::mem::MaybeUninit;

fn print_int_ptr_line(int_number: *const i32) {
    unsafe {
        println!("{}", *int_number);
    }
}

fn bad() {
    unsafe {
        let data: MaybeUninit<*const i32> = MaybeUninit::uninit();
        print_int_ptr_line(data.assume_init());
    }
}

fn good() {
    let data: i32 = 5;
    let data_addr: *const i32 = &data;
    print_int_ptr_line(data_addr);
}

fn main() {
    let mut x: i32 = 0;

    // Match scanf("%d", &x): read whitespace-delimited token, parse as i32
    let mut input = String::new();
    let mut buf = [0u8; 1];
    // Skip leading whitespace (scanf behavior)
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
    // Read until whitespace or EOF
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
}
