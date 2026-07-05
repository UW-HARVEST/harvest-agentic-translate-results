




extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
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
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let int_sum: i32 = int_one + int_two;
    printIntLine(int_sum);
    printIntLine(int_sum);
}

#[no_mangle]
pub fn good() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let mut int_sum: i32 = 0;
    printIntLine(int_sum);
    int_sum = int_one + int_two;
    printIntLine(int_sum);
}

#[no_mangle]
pub fn driver() {
    println!("Calling good()...");
    good();
    println!("Finished good()");
    println!("Calling bad()...");
    bad();
    println!("Finished bad()");
}

