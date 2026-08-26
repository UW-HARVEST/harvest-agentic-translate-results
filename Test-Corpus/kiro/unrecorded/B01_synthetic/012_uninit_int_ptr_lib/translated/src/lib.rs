use std::ffi::c_int;

fn print_int_ptr_line(int_number: *const c_int) {
    unsafe {
        libc::printf(b"%d\n\0".as_ptr() as *const libc::c_char, *int_number);
    }
}

fn bad() {
    unsafe {
        let data: *const c_int = std::mem::MaybeUninit::uninit().assume_init();
        print_int_ptr_line(data);
    }
}

fn good() {
    let data: c_int = 5;
    print_int_ptr_line(&data as *const c_int);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
