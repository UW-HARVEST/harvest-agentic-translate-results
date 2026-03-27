use std::ffi::c_char;

unsafe fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            libc::printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

#[allow(dead_code)]
fn helper_bad() {
    unsafe {
        print_line(b"helperBad()\0".as_ptr() as *const c_char);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    unsafe {
        print_line(b"bad()\0".as_ptr() as *const c_char);
    }
}

fn helper_good() {
    unsafe {
        print_line(b"helperGood()\0".as_ptr() as *const c_char);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    unsafe {
        print_line(b"good()\0".as_ptr() as *const c_char);
    }
    helper_good();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    unsafe {
        print_line(b"Calling good()...\0".as_ptr() as *const c_char);
    }
    good();
    unsafe {
        print_line(b"Finished good()\0".as_ptr() as *const c_char);
        print_line(b"Calling bad()...\0".as_ptr() as *const c_char);
    }
    bad();
    unsafe {
        print_line(b"Finished bad()\0".as_ptr() as *const c_char);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    unsafe {
        print_line(line);
    }
}
