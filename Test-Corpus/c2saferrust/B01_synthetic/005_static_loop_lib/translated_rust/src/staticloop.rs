
use std::sync::atomic::{AtomicI32, Ordering};

extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
#[no_mangle]
pub fn static_sum(update: i32) -> i32 {
    static SUM: AtomicI32 = AtomicI32::new(0);
    SUM.fetch_add(update, Ordering::SeqCst) + update
}

#[no_mangle]
pub fn driver(stride: i32) {
    for i in 0..10 {
        let value = unsafe { static_sum(i * stride) };
        println!("{}", value);
    }
}

