use std::sync::atomic::{AtomicI32, Ordering};

static SUM: AtomicI32 = AtomicI32::new(0);

#[no_mangle]
pub extern "C" fn static_sum(update: i32) -> i32 {
    let new = SUM.fetch_add(update, Ordering::SeqCst) + update;
    new
}

/// Reset for testing — not part of the C API
#[no_mangle]
pub extern "C" fn static_sum_reset() {
    SUM.store(0, Ordering::SeqCst);
}
