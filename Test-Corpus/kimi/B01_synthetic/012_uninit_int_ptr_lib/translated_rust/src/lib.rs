use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}

fn print_int_ptr_line(int_number: &i32) {
    println!("{}", int_number);
}

fn bad() {
    let data: i32 = 0;
    print_int_ptr_line(&data);
}

fn good() {
    let data: i32 = 5;
    print_int_ptr_line(&data);
}