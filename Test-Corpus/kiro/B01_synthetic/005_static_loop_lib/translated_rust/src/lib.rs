use std::ffi::c_int;
use std::sync::atomic::{AtomicI32, Ordering};

static SUM: AtomicI32 = AtomicI32::new(0);

#[unsafe(no_mangle)]
pub extern "C" fn static_sum(update: c_int) -> c_int {
    let new = SUM.fetch_add(update, Ordering::Relaxed) + update;
    new
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(stride: c_int) {
    for i in 0..10 {
        let result = static_sum(i * stride);
        unsafe {
            libc::printf(b"%d\n\0".as_ptr() as *const libc::c_char, result);
        }
    }
}
