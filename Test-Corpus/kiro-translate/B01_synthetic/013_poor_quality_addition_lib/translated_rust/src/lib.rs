use std::ffi::c_char;

fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            libc::printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

fn print_int_line(int_number: i32) {
    unsafe {
        libc::printf(b"%d\n\0".as_ptr() as *const c_char, int_number);
    }
}

fn bad() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let int_sum: i32 = 0;
    print_int_line(int_sum);
    // Bug preserved: intOne + intTwo with no assignment
    let _ = int_one + int_two;
    print_int_line(int_sum);
}

fn good() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let mut int_sum: i32 = 0;
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
