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
        let data = data.assume_init();
        print_int_ptr_line(data);
    }
}

fn good() {
    let data: i32 = 5;
    let data_addr: *const i32 = &data;
    print_int_ptr_line(data_addr);
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let x: i32 = input.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
