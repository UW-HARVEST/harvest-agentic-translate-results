use std::ffi::{c_char, c_int, CStr};

fn print_line(line: *const c_char) {
    if !line.is_null() {
        let s = unsafe { CStr::from_ptr(line) };
        println!("{}", s.to_str().unwrap());
    }
}

fn print_int_line(int_number: c_int) {
    println!("{}", int_number);
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    print_line(line);
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    print_int_line(int_number);
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let int_one: c_int = 1;
    let _int_two: c_int = 1;
    let int_sum: c_int = 0;
    print_int_line(int_sum);
    // Bug preserved: intOne + intTwo; is a no-op, intSum stays 0
    let _ = int_one + _int_two;
    print_int_line(int_sum);
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let int_one: c_int = 1;
    let int_two: c_int = 1;
    let mut int_sum: c_int = 0;
    print_int_line(int_sum);
    int_sum = int_one + int_two;
    print_int_line(int_sum);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    print_line(b"Calling good()...\0".as_ptr() as *const c_char);
    good();
    print_line(b"Finished good()\0".as_ptr() as *const c_char);
    print_line(b"Calling bad()...\0".as_ptr() as *const c_char);
    bad();
    print_line(b"Finished bad()\0".as_ptr() as *const c_char);
}
