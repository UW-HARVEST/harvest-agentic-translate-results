use std::ffi::c_int;
use std::mem::MaybeUninit;

unsafe fn print_int_ptr_line(int_number: *const c_int) {
    unsafe {
        println!("{}", *int_number);
    }
}

fn bad() {
    let data: MaybeUninit<*const c_int> = MaybeUninit::uninit();
    unsafe {
        print_int_ptr_line(data.assume_init());
    }
}

fn good() {
    let data: c_int = 5;
    let data_addr: *const c_int = &data;
    unsafe {
        print_int_ptr_line(data_addr);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
