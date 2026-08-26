use std::ffi::{c_char, c_int};

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn print_int_ptr_line(int_number: &c_int) {
    unsafe {
        printf(b"%d\n\0".as_ptr().cast(), *int_number);
    }
}

fn bad() {
    let value: c_int;
    unsafe {
        // This is the null load emitted for the C uninitialized-pointer bug.
        std::arch::asm!(
            "mov {value:e}, dword ptr [0]",
            value = out(reg) value,
            options(nostack, readonly)
        );
    }
    print_int_ptr_line(&value);
}

fn good() {
    let data: c_int = 5;
    let data_addr = &data;
    print_int_ptr_line(data_addr);
}

fn main() {
    let mut x: c_int = 0;
    unsafe {
        scanf(b"%d\0".as_ptr().cast(), &mut x);
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
