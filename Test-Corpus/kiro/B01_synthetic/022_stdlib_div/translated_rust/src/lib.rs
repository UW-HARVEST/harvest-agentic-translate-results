#[no_mangle]
pub extern "C" fn c_div(x: i32, y: i32, quot: *mut i32, rem: *mut i32) {
    unsafe {
        *quot = x / y;
        *rem = x % y;
    }
}
