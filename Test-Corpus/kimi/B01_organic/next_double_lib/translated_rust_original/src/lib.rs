use std::os::raw::{c_double, c_void};

#[repr(C)]
pub struct cn_rnd_t {
    pub state: [u64; 2],
}

fn cn_rnd_next(rnd: &mut cn_rnd_t) -> u64 {
    let x = rnd.state[0];
    let y = rnd.state[1];
    rnd.state[0] = y;
    let mut x = x ^ (x << 23);
    x ^= x >> 17;
    x ^= y ^ (y >> 26);
    rnd.state[1] = x;
    x + y
}

#[unsafe(no_mangle)]
pub extern "C" fn next_double(rnd: *mut cn_rnd_t) -> c_double {
    let rnd = unsafe { &mut *rnd };
    let value = cn_rnd_next(rnd);
    let exponent: u64 = 1023;
    let mantissa = value >> 12;
    let result = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}