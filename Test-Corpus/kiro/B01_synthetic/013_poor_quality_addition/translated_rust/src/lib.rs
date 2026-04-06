use std::ffi::CStr;
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let c_str = unsafe { CStr::from_ptr(line) };
        if let Ok(s) = c_str.to_str() {
            println!("{}", s);
        }
    }
}

#[no_mangle]
pub extern "C" fn printIntLine(int_number: i32) {
    println!("{}", int_number);
}

#[no_mangle]
pub extern "C" fn bad() {
    let _int_one: i32 = 1;
    let _int_two: i32 = 1;
    let int_sum: i32 = 0;
    printIntLine(int_sum);
    let _ = _int_one + _int_two;
    printIntLine(int_sum);
}

#[no_mangle]
pub extern "C" fn good() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let mut int_sum: i32 = 0;
    printIntLine(int_sum);
    int_sum = int_one + int_two;
    printIntLine(int_sum);
}

#[no_mangle]
pub extern "C" fn entry() {
    let calling_good = std::ffi::CString::new("Calling good()...").unwrap();
    let finished_good = std::ffi::CString::new("Finished good()").unwrap();
    let calling_bad = std::ffi::CString::new("Calling bad()...").unwrap();
    let finished_bad = std::ffi::CString::new("Finished bad()").unwrap();
    printLine(calling_good.as_ptr());
    good();
    printLine(finished_good.as_ptr());
    printLine(calling_bad.as_ptr());
    bad();
    printLine(finished_bad.as_ptr());
}

// Export "main" for C .so symbol compatibility
#[cfg(not(test))]
#[export_name = "main"]
pub extern "C" fn c_main(_argc: i32, _argv: *const *const c_char) -> i32 {
    entry();
    0
}
