// MUTANT 2: "helpfully" avoids the divide-by-zero trap with checked_div,
// printing 0,0 instead of dying with SIGFPE.
use std::ffi::{c_char, c_int};
extern "C" { fn printf(f: *const c_char, ...) -> c_int; }
#[no_mangle]
pub unsafe extern "C" fn driver(x: c_int, y: c_int) {
    let (q, r) = match (x.checked_div(y), x.checked_rem(y)) {
        (Some(q), Some(r)) => (q, r),
        _ => (0, 0),
    };
    printf(b"quotient: %d, remainder: %d\n\0".as_ptr() as *const c_char, q, r);
}
