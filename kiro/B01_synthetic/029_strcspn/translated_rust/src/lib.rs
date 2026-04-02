use std::io::BufRead;

pub fn strcspn(s1: &[u8], s2: &[u8]) -> usize {
    for (i, &b) in s1.iter().enumerate() {
        if s2.contains(&b) {
            return i;
        }
    }
    s1.len()
}

#[no_mangle]
pub extern "C" fn driver(s1: *const std::os::raw::c_char, s2: *const std::os::raw::c_char) {
    let s1 = unsafe { std::ffi::CStr::from_ptr(s1) }.to_bytes();
    let s2 = unsafe { std::ffi::CStr::from_ptr(s2) }.to_bytes();
    println!("{}", strcspn(s1, s2));
}

pub fn run_main() -> i32 {
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();

    let mut s1 = lines.next().unwrap().unwrap();
    let mut s2 = lines.next().and_then(|r| r.ok()).unwrap_or_default();

    s1.truncate(99);
    s2.truncate(99);

    let c_s1 = std::ffi::CString::new(s1).unwrap();
    let c_s2 = std::ffi::CString::new(s2).unwrap();
    driver(c_s1.as_ptr(), c_s2.as_ptr());
    0
}
