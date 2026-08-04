



extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
#[no_mangle]
pub fn printIntPtrLine(int_number: i32) {
    println!("{}", int_number);
}

#[no_mangle]
pub fn bad() {
    let data: Option<i32> = None;
    printIntPtrLine(data.unwrap_or(0));
}

#[no_mangle]
pub fn good() {
    let data: i32 = 5;
    printIntPtrLine(data);
}

#[no_mangle]
pub fn driver(use_good: bool) {
    if use_good {
        good();
    } else {
        bad();
    }
}

