use std::ffi::c_char;

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            libc::printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: libc::c_int) {
    unsafe {
        libc::printf(b"%d\n\0".as_ptr() as *const c_char, int_number);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let int_one: libc::c_int = 1;
    let int_two: libc::c_int = 1;
    let int_sum: libc::c_int = 0;
    printIntLine(int_sum);
    let _ = int_one + int_two;
    printIntLine(int_sum);
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let int_one: libc::c_int = 1;
    let int_two: libc::c_int = 1;
    let mut int_sum: libc::c_int = 0;
    printIntLine(int_sum);
    int_sum = int_one + int_two;
    printIntLine(int_sum);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    printLine(b"Calling good()...\0".as_ptr() as *const c_char);
    good();
    printLine(b"Finished good()\0".as_ptr() as *const c_char);
    printLine(b"Calling bad()...\0".as_ptr() as *const c_char);
    bad();
    printLine(b"Finished bad()\0".as_ptr() as *const c_char);
}
