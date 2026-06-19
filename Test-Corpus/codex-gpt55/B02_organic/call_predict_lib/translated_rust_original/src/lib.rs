use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn call_predict(pfcn: c_int) -> c_int {
    match pfcn {
        0..=11 => 1,
        _ => 0,
    }
}
