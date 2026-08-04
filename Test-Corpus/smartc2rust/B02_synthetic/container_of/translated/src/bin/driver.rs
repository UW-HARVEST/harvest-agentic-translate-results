fn main() {
    use std::os::raw::{c_char, c_int};
    let args: Vec<std::ffi::CString> = std::env::args()
        .map(|a| std::ffi::CString::new(a).unwrap()).collect();
    let mut argv: Vec<*mut c_char> = args.iter()
        .map(|a| a.as_ptr() as *mut c_char).collect();
    argv.push(std::ptr::null_mut());
    let rc = unsafe { driver::container_of_main(args.len() as c_int, argv.as_mut_ptr()) };
    std::process::exit(rc as i32);
}
