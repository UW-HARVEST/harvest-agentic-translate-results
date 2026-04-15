use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
    static mut INNER: c_int = 1;
    unsafe {
        if *outer >= INNER {
            INNER += *outer;
            std::ptr::addr_of_mut!(INNER)
        } else {
            *outer += INNER;
            outer
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(mut initial_value: c_int, iterations: c_int) {
    let mut running_sum: *mut c_int = &mut initial_value;
    for _ in 0..iterations {
        running_sum = static_alias(running_sum);
        unsafe {
            println!("{}", *running_sum);
        }
    }
}
