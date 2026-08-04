use std::os::raw::{c_char, c_int};

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(b"%s\n\0".as_ptr().cast(), line);
        }
    }
}

fn bad() {
    // The original C uses an uninitialized pointer here, which is undefined
    // behavior. On this environment's default reference build, the observed
    // result is behavior equivalent to passing a pointer to an empty string,
    // which prints only the trailing newline.
    let data = b"\0".as_ptr().cast();
    print_line(data);
}

fn good() {
    let data = b"string\0".as_ptr().cast();
    print_line(data);
}

fn main() {
    let mut x: c_int = 0;
    unsafe {
        scanf(b"%d\0".as_ptr().cast(), &mut x);
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
