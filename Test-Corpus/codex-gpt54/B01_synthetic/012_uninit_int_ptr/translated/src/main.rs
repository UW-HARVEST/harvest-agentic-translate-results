use core::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn print_int_ptr_line(int_number: *const c_int) {
    unsafe {
        printf(c"%d\n".as_ptr(), *int_number);
    }
}

fn bad() {
    let data = 0;
    let data_addr = &data as *const c_int;
    print_int_ptr_line(data_addr);
}

fn good() {
    let data: c_int = 5;
    let data_addr = &data as *const c_int;
    print_int_ptr_line(data_addr);
}

fn main() {
    let mut x: c_int = 0;

    unsafe {
        scanf(c"%d".as_ptr(), &mut x);
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
