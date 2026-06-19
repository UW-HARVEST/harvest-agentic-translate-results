use std::os::raw::{c_char, c_int};

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn scanf(format: *const c_char, ...) -> c_int;
}

unsafe fn print_int_ptr_line(int_number: *const c_int) {
    printf(b"%d\n\0".as_ptr() as *const c_char, *int_number);
}

unsafe fn bad() {
    let data: c_int = 0;
    print_int_ptr_line(&data);
}

unsafe fn good() {
    let data: c_int = 5;
    let data_addr: *const c_int = &data;
    print_int_ptr_line(data_addr);
}

fn main() {
    let mut x: c_int = 0;

    unsafe {
        scanf(b"%d\0".as_ptr() as *const c_char, &mut x as *mut c_int);

        if x != 0 {
            good();
        } else {
            bad();
        }
    }
}
