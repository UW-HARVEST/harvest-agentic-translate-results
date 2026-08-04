use std::io::{self, BufRead, Write};
use std::mem::MaybeUninit;

fn print_int_ptr_line(int_number: *const i32) {
    // Mirrors the C function: dereferences the pointer and prints the value.
    unsafe {
        println!("{}", *int_number);
    }
}

fn bad() {
    // Mirrors the C function which uses an uninitialized pointer.
    // This is undefined behavior in C; we replicate it using MaybeUninit
    // so the program compiles. Calling this triggers UB at runtime.
    let data: MaybeUninit<*const i32> = MaybeUninit::uninit();
    let data_ptr: *const i32 = unsafe { data.assume_init() };
    print_int_ptr_line(data_ptr);
}

fn good() {
    let data: i32 = 5;
    let data_addr: *const i32 = &data;
    print_int_ptr_line(data_addr);
}

fn main() {
    let mut x: i32 = 0;

    // Read an integer from stdin (mimicking scanf("%d", &x)).
    let stdin = io::stdin();
    let mut input = String::new();
    // Make sure any prior prints are flushed before reading.
    let _ = io::stdout().flush();
    if stdin.lock().read_line(&mut input).is_ok() {
        let trimmed = input.trim();
        // Find the first whitespace-separated token and parse it.
        if let Some(token) = trimmed.split_whitespace().next() {
            if let Ok(parsed) = token.parse::<i32>() {
                x = parsed;
            }
        }
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
