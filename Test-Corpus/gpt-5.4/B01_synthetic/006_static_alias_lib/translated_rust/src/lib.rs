use std::os::raw::c_int;
use std::sync::atomic::{AtomicI32, Ordering};

static INNER: AtomicI32 = AtomicI32::new(1);

#[unsafe(no_mangle)]
pub extern "C" fn static_alias(outer: *mut c_int) -> *mut c_int {
    if outer.is_null() {
        return outer;
    }

    let outer_ref = unsafe { &mut *outer };
    let inner = INNER.load(Ordering::SeqCst);

    if *outer_ref >= inner {
        let new_inner = INNER.fetch_add(*outer_ref, Ordering::SeqCst) + *outer_ref;
        let inner_ptr = (&INNER as *const AtomicI32).cast::<c_int>() as *mut c_int;
        let _ = new_inner;
        inner_ptr
    } else {
        *outer_ref += inner;
        outer
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(initial_value: c_int, iterations: c_int) {
    let mut initial_value = initial_value;
    let mut running_sum: *mut c_int = &mut initial_value;
    let mut i = 0;
    while i < iterations {
        running_sum = static_alias(running_sum);
        if !running_sum.is_null() {
            let value = unsafe { *running_sum };
            println!("{}", value);
        }
        i += 1;
    }
}