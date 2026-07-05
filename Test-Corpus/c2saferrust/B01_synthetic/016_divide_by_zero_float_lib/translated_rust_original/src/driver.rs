






extern "C" {
    fn fabs(__x: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub fn printLine(line: &str) {
    println!("{}", line);
}

#[no_mangle]
pub fn printIntLine(int_number: ::core::ffi::c_int) {
    println!("{}", int_number);
}

#[no_mangle]
pub fn bad(data: f32) {
    let result = (100.0f64 / data as f64) as i32;
    printIntLine(result);
}

fn goodG2B() {
    let data: f32 = 2.0;
    let result: i32 = (100.0f64 / data as f64) as i32;
    printIntLine(result);
}

fn goodB2G(data: f32) {
    if (data as f64).abs() > 0.000001f64 {
        let result: i32 = (100.0f64 / data as f64) as i32;
        printIntLine(result);
    } else {
        printLine("This would result in a divide by zero");
    }
}

#[no_mangle]
pub fn good(data: f32) {
    goodG2B();
    goodB2G(data);
}

#[no_mangle]
pub fn driver(good_data: f32, bad_data: f32) {
    printLine("Calling good()...");
    good(good_data);
    printLine("Finished good()");
    printLine("Calling bad()...");
    bad(bad_data);
    printLine("Finished bad()");
}

