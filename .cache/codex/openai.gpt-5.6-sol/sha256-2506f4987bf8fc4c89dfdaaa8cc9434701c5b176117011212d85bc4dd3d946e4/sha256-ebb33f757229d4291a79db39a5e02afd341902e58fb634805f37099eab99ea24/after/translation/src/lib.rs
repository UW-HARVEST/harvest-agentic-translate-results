use std::ffi::{c_char, c_int};
use std::ptr;

static mut INNER: c_int = 1;
const PRINT_FORMAT: &[u8] = b"%d\n\0";

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
    let outer_value = unsafe { outer.read() };
    let inner_value = unsafe { INNER };

    if outer_value >= inner_value {
        unsafe {
            INNER = inner_value.wrapping_add(outer_value);
        }
        ptr::addr_of_mut!(INNER)
    } else {
        unsafe {
            outer.write(outer_value.wrapping_add(inner_value));
        }
        outer
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(initial_value: c_int, iterations: c_int) {
    let mut initial_value = initial_value;
    let mut running_sum = ptr::addr_of_mut!(initial_value);
    let mut i = 0;

    while i < iterations {
        running_sum = unsafe { static_alias(running_sum) };
        unsafe {
            printf(PRINT_FORMAT.as_ptr().cast(), running_sum.read());
        }
        i += 1;
    }
}
