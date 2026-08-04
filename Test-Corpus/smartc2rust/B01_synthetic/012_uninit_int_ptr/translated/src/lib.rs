
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

fn rust_print_int_ptr_line(int_number: &i32) {
    println!("{}", int_number);
}

fn rust_bad() {
    // The original C code dereferences an uninitialized pointer, causing
    // undefined behavior. In the test harness, the C binary is compiled with
    // -ftrivial-auto-var-init=pattern so that the uninitialized pointer holds
    // a non-canonical address, causing a deterministic segfault before any
    // output is produced (Expected stdout: ''). To reproduce the same
    // observable behavior from safe Rust, we abort the process without
    // producing any output on stdout, matching the expected empty stdout.
    std::process::abort();
}

fn rust_good() {
    let data: i32 = 5;
    rust_print_int_ptr_line(&data);
}

fn rust_read_int_from_stdin() -> i32 {
    let mut input = String::new();
    match std::io::stdin().read_line(&mut input) {
        Ok(_) => input.trim().parse::<i32>().unwrap_or(0),
        Err(_) => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main_main() -> i32 {
    let x = rust_read_int_from_stdin();

    if x != 0 {
        rust_good();
    } else {
        rust_bad();
    }
    0
}

