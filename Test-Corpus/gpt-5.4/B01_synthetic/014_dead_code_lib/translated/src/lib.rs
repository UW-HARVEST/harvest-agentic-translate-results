use std::ffi::{c_char, CStr};

fn print_line(line: *const c_char) {
    if !line.is_null() {
        let c_str = unsafe { CStr::from_ptr(line) };
        println!("{}", c_str.to_string_lossy());
    }
}

fn helper_bad() {
    print_line(c"helperBad()".as_ptr());
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    print_line(c"bad()".as_ptr());
}

fn helper_good() {
    print_line(c"helperGood()".as_ptr());
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    print_line(c"good()".as_ptr());
    helper_good();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    print_line(c"Calling good()...".as_ptr());
    good();
    print_line(c"Finished good()".as_ptr());
    print_line(c"Calling bad()...".as_ptr());
    bad();
    print_line(c"Finished bad()".as_ptr());
}
