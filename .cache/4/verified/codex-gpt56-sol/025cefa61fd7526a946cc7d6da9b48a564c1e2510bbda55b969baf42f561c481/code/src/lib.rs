use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn puts(line: *const c_char) -> c_int;
    fn scanf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // SAFETY: This has the same caller contract as C's printf("%s\n", line).
        unsafe {
            puts(line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad() {
    // GCC 11.5 emits a null return for helperBad's pointer to its local array.
    unsafe {
        printLine(std::ptr::null());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good() {
    unsafe {
        printLine(c"helperGood1 string".as_ptr());
    }
}

#[unsafe(export_name = "main")]
pub extern "C" fn c_main() -> c_int {
    let mut x: c_int = 0;

    unsafe {
        scanf(c"%d".as_ptr(), &mut x);
    }

    if x != 0 {
        good();
    } else {
        bad();
    }

    0
}
