use std::ffi::{c_char, c_int, c_long};
use std::ptr;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn strlen(value: *const c_char) -> usize;
    fn strtol(value: *const c_char, end: *mut *mut c_char, base: c_int) -> c_long;
}

#[no_mangle]
pub unsafe extern "C" fn main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    if argc > 4 || argc == 1 {
        printf(c"Error: there should be one to three arguments passed:\n".as_ptr());
        printf(c"<string> [start] [stop]\n".as_ptr());
        return 1;
    }

    let string = *argv.add(1);
    let len = strlen(string);
    let mut end = ptr::null_mut();

    let start = if argc >= 3 {
        let argument = *argv.add(2);
        let value = strtol(argument, &mut end, 10) as c_int;
        if end == argument {
            printf(c"Second argument must be an integer!".as_ptr());
            return 1;
        }
        if value as usize > len {
            printf(c"Error: start is off the end of the string!\n".as_ptr());
            return 1;
        }
        value
    } else {
        0
    };

    let stop = if argc == 4 {
        let argument = *argv.add(3);
        let value = strtol(argument, ptr::null_mut(), 10) as c_int;
        if end == argument {
            printf(c"Third argument must be an integer!".as_ptr());
            return 1;
        }
        if value as usize > len {
            printf(c"Error: stop is off the end of the string!\n".as_ptr());
            return 1;
        }
        if value <= start {
            printf(c"Error: stop must come after start!\n".as_ptr());
            return 1;
        }
        value
    } else {
        len as c_int
    };

    printf(c"%.*s\n".as_ptr(), stop - start, string.add(start as usize));
    0
}
