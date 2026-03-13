use std::ffi::c_int;

static mut SUM: c_int = 0;

#[unsafe(no_mangle)]
pub extern "C" fn static_sum(update: c_int) -> c_int {
    unsafe {
        SUM += update;
        SUM
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(stride: c_int) {
    for i in 0..10 {
        let result = static_sum(i * stride);
        // Match C printf("%d\n", ...)
        println!("{}", result);
    }
}
