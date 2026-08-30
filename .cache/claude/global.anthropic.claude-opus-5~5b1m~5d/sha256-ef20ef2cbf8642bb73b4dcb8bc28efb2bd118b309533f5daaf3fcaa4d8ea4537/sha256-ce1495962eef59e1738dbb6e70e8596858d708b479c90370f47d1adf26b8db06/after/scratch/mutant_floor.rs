// MUTANT 1: uses floor division / Euclidean remainder instead of C's
// truncate-toward-zero. A very plausible mis-translation.
use std::ffi::{c_char, c_int};
extern "C" { fn printf(f: *const c_char, ...) -> c_int; }
#[no_mangle]
pub unsafe extern "C" fn driver(x: c_int, y: c_int) {
    let q = (x as i64).div_euclid(y as i64) as i32;
    let r = (x as i64).rem_euclid(y as i64) as i32;
    printf(b"quotient: %d, remainder: %d\n\0".as_ptr() as *const c_char, q, r);
}
