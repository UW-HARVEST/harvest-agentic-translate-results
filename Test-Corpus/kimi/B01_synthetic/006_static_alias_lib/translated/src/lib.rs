use std::os::raw::{c_int, c_void};
use std::ptr;
use std::sync::Mutex;

static INNER: Mutex<c_int> = Mutex::new(1);

#[unsafe(no_mangle)]
pub extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
    let outer_val = unsafe { *outer };
    let mut inner_guard = INNER.lock().unwrap();
    if outer_val >= *inner_guard {
        *inner_guard += outer_val;
        drop(inner_guard);
        &raw mut INNER as *mut c_int
    } else {
        unsafe { *outer += *inner_guard; }
        drop(inner_guard);
        outer
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(initial_value: c_int, iterations: c_int) {
    let mut running_sum: c_int = initial_value;
    let mut ptr: *mut c_int = &mut running_sum;
    for _ in 0..iterations {
        ptr = static_alias(ptr);
        unsafe {
            println!("{}", *ptr);
        }
    }
}
