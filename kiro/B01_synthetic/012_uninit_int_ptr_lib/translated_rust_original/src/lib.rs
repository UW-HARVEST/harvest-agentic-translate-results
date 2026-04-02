use std::ffi::c_int;
use std::mem::MaybeUninit;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

unsafe fn print_int_ptr_line(int_number: *const c_int) {
    printf(b"%d\n\0".as_ptr(), *int_number);
}

unsafe fn bad() {
    let data: *const c_int = MaybeUninit::uninit().assume_init();
    print_int_ptr_line(data);
}

unsafe fn good() {
    let data: c_int = 5;
    let data_addr: *const c_int = &data;
    print_int_ptr_line(data_addr);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    unsafe {
        if use_good != 0 {
            good();
        } else {
            bad();
        }
    }
}
