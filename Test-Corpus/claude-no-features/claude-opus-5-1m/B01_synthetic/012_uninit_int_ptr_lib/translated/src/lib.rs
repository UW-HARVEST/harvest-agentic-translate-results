use std::ffi::c_int;
use std::os::raw::c_char;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

fn print_int_ptr_line(int_number: *const c_int) {
    unsafe {
        printf(b"%d\n\0".as_ptr() as *const c_char, *int_number);
    }
}

fn bad() {
    // Mirror the C code's behavior: declare an uninitialized pointer and
    // dereference it via printIntPtrLine. This reproduces the C UB exactly.
    let data: *const c_int;
    unsafe {
        // Use MaybeUninit to obtain an uninitialized pointer value, matching
        // the original C code's `int *data;` (uninitialized pointer).
        let uninit: std::mem::MaybeUninit<*const c_int> = std::mem::MaybeUninit::uninit();
        data = uninit.assume_init();
    }
    print_int_ptr_line(data);
}

fn good() {
    let mut data: c_int;
    data = 5;
    let data_addr: *const c_int;
    data_addr = &data as *const c_int;
    print_int_ptr_line(data_addr);
    let _ = &mut data;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
