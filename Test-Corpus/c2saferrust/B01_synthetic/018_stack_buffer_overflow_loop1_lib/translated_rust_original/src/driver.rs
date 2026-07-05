




extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
pub type size_t = usize;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub fn printLine(line: &str) {
    println!("{}", line);
}

#[no_mangle]
pub fn printIntLine(int_number: i32) {
    println!("{}", int_number);
}

#[no_mangle]
pub fn bad() {
    let mut data = vec![0; 10];
    let source = [0; 10];
    data.copy_from_slice(&source);
    printIntLine(data[0]);
}

#[no_mangle]
pub fn good() {
    let data = vec![0 as ::core::ffi::c_int; 10];
    printIntLine(data[0]);
}

#[no_mangle]
pub fn driver(use_good: bool) {
    if use_good {
        good();
    } else {
        bad();
    }
}

