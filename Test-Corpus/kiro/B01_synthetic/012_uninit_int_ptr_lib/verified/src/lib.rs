use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn printIntPtrLine(int_number: *const c_int) {
    unsafe {
        libc::printf(b"%d\n\0".as_ptr() as *const libc::c_char, *int_number);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    unsafe {
        let data: *const c_int = std::mem::MaybeUninit::uninit().assume_init();
        printIntPtrLine(data);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let data: c_int = 5;
    printIntPtrLine(&data as *const c_int);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
