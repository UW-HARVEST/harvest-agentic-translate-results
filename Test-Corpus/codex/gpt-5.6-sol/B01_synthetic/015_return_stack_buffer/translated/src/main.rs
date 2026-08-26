use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
}

fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        println!("{line}");
    }
}

fn helper_bad() -> Option<&'static str> {
    // GCC turns the C function's invalid pointer to its local array into null.
    None
}

fn bad() {
    print_line(helper_bad());
}

fn helper_good_1() -> &'static str {
    "helperGood1 string"
}

fn good() {
    print_line(Some(helper_good_1()));
}

fn main() {
    let mut x: c_int = 0;

    // SAFETY: the format expects one pointer to an int, which is supplied.
    unsafe {
        scanf(b"%d\0".as_ptr().cast(), &mut x);
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
