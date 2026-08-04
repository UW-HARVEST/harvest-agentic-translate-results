
use std::sync::Mutex;

extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
#[no_mangle]
pub fn static_alias(mut outer: *mut i32) -> *mut i32 {
    static INNER: Mutex<i32> = Mutex::new(1);

    let mut inner = INNER.lock().unwrap();
    unsafe {
        if *outer >= *inner {
            *inner += *outer;
            outer = (&mut *inner) as *mut i32;
        } else {
            *outer += *inner;
        }
    }
    outer
}

#[no_mangle]
pub fn driver(mut initial_value: i32, iterations: i32) {
    let mut running_sum: *mut i32 = &mut initial_value as *mut i32;
    let mut i = 0;
    while i < iterations {
        running_sum = static_alias(running_sum);
        println!("{}", initial_value);
        i += 1;
    }
}

