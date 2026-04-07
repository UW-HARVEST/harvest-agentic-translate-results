use std::ffi::CStr;
use std::os::raw::c_char;

fn driver_impl(s1: &[u8], s2: &[u8]) -> usize {
    s1.iter().take_while(|b| !s2.contains(b)).count()
}

#[no_mangle]
pub extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    let s1 = unsafe { CStr::from_ptr(s1) }.to_bytes();
    let s2 = unsafe { CStr::from_ptr(s2) }.to_bytes();
    println!("{}", driver_impl(s1, s2));
}
