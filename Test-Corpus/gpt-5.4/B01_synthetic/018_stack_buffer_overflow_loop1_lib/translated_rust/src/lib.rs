use std::ffi::c_char;
use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if line.is_null() {
        return;
    }
    let s = unsafe { std::ffi::CStr::from_ptr(line) };
    println!("{}", s.to_string_lossy());
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    println!("{}", int_number);
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let mut data = [0i32; 2];
    let source = [0i32; 10];
    for i in 0..10 {
        if i < data.len() {
            data[i] = source[i];
        }
    }
    printIntLine(data[0] as c_int);
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let mut data = [0i32; 10];
    let source = [0i32; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    printIntLine(data[0] as c_int);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
