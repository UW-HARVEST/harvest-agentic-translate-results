use std::os::raw::c_int;
use std::sync::Mutex;

static SUM: Mutex<i32> = Mutex::new(0);

#[unsafe(no_mangle)]
pub extern "C" fn static_sum(update: c_int) -> c_int {
    let mut sum = SUM.lock().unwrap();
    *sum += update;
    *sum
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(stride: c_int) {
    for i in 0..10 {
        let result = static_sum(i * stride);
        println!("{}", result);
    }
}