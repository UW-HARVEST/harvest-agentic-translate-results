use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn print_int_ptr_line(int_number: &c_int) {
    unsafe {
        printf(b"%d\n\0".as_ptr().cast(), *int_number);
    }
}

fn bad(data: &c_int) {
    print_int_ptr_line(data);
}

fn good() {
    let data = 5;
    let data_addr = &data;
    print_int_ptr_line(data_addr);
}

fn main() {
    let mut x = 0;
    unsafe {
        scanf(b"%d\0".as_ptr().cast(), &raw mut x);
    }

    if x != 0 {
        good();
    } else {
        // This is the value observed from the C program's uninitialized pointer
        // path in its default build, without introducing undefined behavior here.
        bad(&x);
    }
}
