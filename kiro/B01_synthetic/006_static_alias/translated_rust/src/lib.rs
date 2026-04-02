use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

static mut INNER: i32 = 1;

#[no_mangle]
pub unsafe extern "C" fn static_alias(outer: *mut i32) -> *mut i32 {
    if *outer >= INNER {
        INNER += *outer;
        &raw mut INNER
    } else {
        *outer += INNER;
        outer
    }
}

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[no_mangle]
#[cfg(not(test))]
pub unsafe extern "C" fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    if argc != 3 {
        printf(b"Error: should only be two (integer) arguments!\n\0".as_ptr() as *const c_char);
        return 1;
    }

    let arg1 = CStr::from_ptr(*argv.add(1));
    let arg2 = CStr::from_ptr(*argv.add(2));

    let (initial_value, ok1) = strtol_like(arg1.to_bytes());
    if !ok1 {
        printf(b"Error: first argument must be an integer!\n\0".as_ptr() as *const c_char);
        return 1;
    }

    let (iterations, ok2) = strtol_like(arg2.to_bytes());
    if !ok2 {
        printf(b"Error: second argument must be an integer!\n\0".as_ptr() as *const c_char);
        return 1;
    }

    INNER = 1;
    let mut outer_val: i32 = initial_value as i32;
    let mut running_sum: *mut i32 = &mut outer_val;
    for _ in 0..iterations {
        running_sum = static_alias(running_sum);
        printf(b"%d\n\0".as_ptr() as *const c_char, *running_sum);
    }

    0
}

fn strtol_like(bytes: &[u8]) -> (i64, bool) {
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let neg = if i < bytes.len() && bytes[i] == b'-' {
        i += 1;
        true
    } else if i < bytes.len() && bytes[i] == b'+' {
        i += 1;
        false
    } else {
        false
    };
    let mut found = false;
    let mut val: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        found = true;
        val = val.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    if neg {
        val = -val;
    }
    (val, found)
}
