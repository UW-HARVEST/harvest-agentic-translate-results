use std::ffi::c_int;
use std::ffi::c_char;

unsafe fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe { libc::printf(b"%s\n\0".as_ptr() as *const c_char, line) };
    }
}

fn print_int_line(int_number: c_int) {
    unsafe { libc::printf(b"%d\n\0".as_ptr() as *const c_char, int_number) };
}

fn bad() {
    let int_sum: c_int = 0;
    print_int_line(int_sum);
    // Original C: intOne + intTwo; (result discarded, intSum unchanged)
    print_int_line(int_sum);
}

fn good() {
    let mut int_sum: c_int = 0;
    print_int_line(int_sum);
    int_sum = 1 + 1;
    print_int_line(int_sum);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    unsafe { print_line(b"Calling good()...\0".as_ptr() as *const c_char) };
    good();
    unsafe { print_line(b"Finished good()\0".as_ptr() as *const c_char) };
    unsafe { print_line(b"Calling bad()...\0".as_ptr() as *const c_char) };
    bad();
    unsafe { print_line(b"Finished bad()\0".as_ptr() as *const c_char) };
}
