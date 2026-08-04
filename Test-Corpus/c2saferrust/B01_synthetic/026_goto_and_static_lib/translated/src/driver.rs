

extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
static mut y: ::core::ffi::c_int = 123 as ::core::ffi::c_int;
fn multi_stage(x: i32, z: i32) -> i32 {
    let y_value = unsafe { y };

    if x != 1 {
        println!("Error: x != 1");
        println!("Operation failed");
        1
    } else if y_value != 2 {
        println!("Error: x == 1 but y != 2");
        println!("Operation failed");
        2
    } else if z != 3 {
        println!("Error: x == 1 and y == 2, but z != 3");
        println!("Operation failed");
        3
    } else {
        println!("Ok!");
        0
    }
}

#[no_mangle]
pub fn driver(x: i32, local_y: i32, z: i32) {
    unsafe {
        y = local_y;
    }
    let result: i32 = multi_stage(x, z);
    println!("Result: {}", result);
}

