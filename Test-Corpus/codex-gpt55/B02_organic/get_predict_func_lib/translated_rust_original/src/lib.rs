use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn get_predict_func(pfcn: c_int) -> c_int {
    match pfcn {
        0..=11 => 1,
        _ => 0,
    }
}
