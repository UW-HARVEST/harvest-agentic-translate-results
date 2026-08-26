use std::env;
use std::ffi::{CString, OsStr, OsString};
use std::os::raw::{c_char, c_int};
use std::os::unix::ffi::OsStrExt;

extern "C" {
    fn atoi(value: *const c_char) -> c_int;
}

fn atoi_arg(value: &OsStr) -> i32 {
    let value = CString::new(value.as_bytes()).expect("arguments cannot contain NUL bytes");

    // SAFETY: CString supplies the NUL-terminated byte string required by C atoi.
    unsafe { atoi(value.as_ptr()) }
}

fn next_atoi(args: &mut impl Iterator<Item = OsString>) -> i32 {
    match args.next() {
        Some(value) => atoi_arg(&value),
        // SAFETY: This intentionally preserves the C program's null argv access.
        None => unsafe { atoi(std::ptr::null()) },
    }
}

fn main() {
    let mut args = env::args_os();
    args.next();

    let a = next_atoi(&mut args);
    let b = next_atoi(&mut args);

    println!("{}", a.wrapping_add(b));
}
