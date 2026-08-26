use std::os::raw::c_int;
use std::sync::atomic::{AtomicI32, Ordering};

static SUM: AtomicI32 = AtomicI32::new(0);

#[unsafe(no_mangle)]
pub extern "C" fn static_sum(update: c_int) -> c_int {
    (SUM.fetch_add(update as i32, Ordering::SeqCst) + update as i32) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(stride: c_int) {
    for i in 0..10 {
        println!("{}", static_sum(i * stride));
    }
}
