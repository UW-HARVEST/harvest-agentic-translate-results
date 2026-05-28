use std::ffi::c_char;

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> i32;
}

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        unsafe {
            printf(fmt, line);
        }
    }
}

#[allow(dead_code)]
fn helper_bad() {
    let s = b"helperBad()\0".as_ptr() as *const c_char;
    printLine(s);
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    let s = b"bad()\0".as_ptr() as *const c_char;
    printLine(s);
}

fn helper_good() {
    let s = b"helperGood()\0".as_ptr() as *const c_char;
    printLine(s);
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    let s = b"good()\0".as_ptr() as *const c_char;
    printLine(s);
    helper_good();
}

#[unsafe(no_mangle)]
pub extern "C" fn driver() {
    let s1 = b"Calling good()...\0".as_ptr() as *const c_char;
    printLine(s1);
    good();
    let s2 = b"Finished good()\0".as_ptr() as *const c_char;
    printLine(s2);
    let s3 = b"Calling bad()...\0".as_ptr() as *const c_char;
    printLine(s3);
    bad();
    let s4 = b"Finished bad()\0".as_ptr() as *const c_char;
    printLine(s4);
}
