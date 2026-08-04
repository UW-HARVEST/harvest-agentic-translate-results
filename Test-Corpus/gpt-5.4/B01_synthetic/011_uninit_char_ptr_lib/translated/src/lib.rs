use std::ffi::{c_char, CStr};
use std::os::raw::c_int;

fn print_line(line: Option<&CStr>) {
    if let Some(line) = line {
        println!("{}", line.to_string_lossy());
    }
}

fn bad() {
    let data: Option<&CStr> = None;
    print_line(data);
}

fn good() {
    let data = c"string";
    print_line(Some(data));
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
