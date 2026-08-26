use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn div_euclid(v1: c_int, v2: c_int) -> c_int {
    if v2 == 0 {
        return 0;
    }
    v1.checked_div_euclid(v2).unwrap_or(0)
}
