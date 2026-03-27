use std::io::{self, Read};
use std::mem::MaybeUninit;

fn print_int_ptr_line(int_number: *const i32) {
    unsafe {
        println!("{}", *int_number);
    }
}

fn bad() {
    // Reproduces C UB: uninitialized pointer dereference
    let data: MaybeUninit<*const i32> = MaybeUninit::uninit();
    let ptr = unsafe { data.assume_init() };
    print_int_ptr_line(ptr);
}

fn good() {
    let data: i32 = 5;
    let data_addr: *const i32 = &data;
    print_int_ptr_line(data_addr);
}

fn main() {
    // Match C scanf("%d", &x) behavior: skip whitespace, parse integer
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();
    let x: i32 = input.trim().parse().unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}
