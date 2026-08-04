use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let s = unsafe { CStr::from_ptr(line) };
        println!("{}", s.to_str().unwrap_or(""));
    }
}

#[no_mangle]
pub extern "C" fn bad() {
    let data: MaybeUninit<*const c_char> = MaybeUninit::zeroed();
    let data = unsafe { data.assume_init() };
    printLine(data);
}

#[no_mangle]
pub extern "C" fn good() {
    printLine(b"string\0".as_ptr() as *const c_char);
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> std::os::raw::c_int {
    use std::io::Read;
    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);
    let x: i32 = input.split_whitespace().next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    if x != 0 { good(); } else { bad(); }
    0
}
