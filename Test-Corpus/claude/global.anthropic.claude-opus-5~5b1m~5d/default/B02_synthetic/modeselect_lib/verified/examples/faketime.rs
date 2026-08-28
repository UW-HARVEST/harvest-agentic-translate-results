// LD_PRELOAD interposer for libc `time()`.
//
// `get_modified_time` (and therefore `modeselect`) reads the wall clock, which
// makes the `time_t current = time(NULL); current >>= 29;` code path impossible
// to exercise for anything other than "the current second".  In particular an
// *arithmetic* vs a *logical* `>> 29` is indistinguishable while the clock is
// positive, and the `current + offset` addition is never tested near the
// interesting `time_t` boundaries.
//
// Preloading this object makes `time()` return the value of the environment
// variable `DIFFTEST_FAKE_TIME`, so both the C and the Rust library observe the
// same, chosen, clock -- including negative and extreme values.
//
// Deliberately allocation-free and free of any call back into `time()`.

use std::ffi::{c_char, c_long};

unsafe extern "C" {
    unsafe fn getenv(name: *const c_char) -> *const c_char;
}

const VAR: &[u8] = b"DIFFTEST_FAKE_TIME\0";

fn fake_value() -> c_long {
    unsafe {
        let p = getenv(VAR.as_ptr() as *const c_char);
        if p.is_null() {
            return 0;
        }
        let mut i: isize = 0;
        let mut neg = false;
        if *p == b'-' as c_char {
            neg = true;
            i = 1;
        } else if *p == b'+' as c_char {
            i = 1;
        }
        let mut acc: i64 = 0;
        loop {
            let c = *p.offset(i) as u8;
            if !c.is_ascii_digit() {
                break;
            }
            acc = acc.wrapping_mul(10).wrapping_add((c - b'0') as i64);
            i += 1;
        }
        if neg {
            acc.wrapping_neg()
        } else {
            acc
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn time(tloc: *mut c_long) -> c_long {
    let v = fake_value();
    if !tloc.is_null() {
        unsafe { *tloc = v };
    }
    v
}

// Cargo requires an example target to have a `main`, even for `crate-type =
// ["cdylib"]`, where it is never linked in.
#[allow(dead_code)]
fn main() {}
