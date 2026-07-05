




extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub fn printLine(line: &str) {
    println!("{}", line);
}

#[no_mangle]
pub fn bad() {
    printLine("bad()");
}

fn helperGood() {
    printLine("helperGood()");
}

#[no_mangle]
pub fn good() {
    printLine("good()");
    helperGood();
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

