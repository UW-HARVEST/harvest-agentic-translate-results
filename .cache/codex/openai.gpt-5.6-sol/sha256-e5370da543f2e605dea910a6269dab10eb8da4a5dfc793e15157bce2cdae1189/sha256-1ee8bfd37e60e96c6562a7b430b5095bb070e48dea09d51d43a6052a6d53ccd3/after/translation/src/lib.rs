#[repr(C)]
pub struct CnRnd {
    state: [u64; 2],
}

#[unsafe(no_mangle)]
pub extern "C" fn next_double(rnd: *mut CnRnd) -> f64 {
    // SAFETY: The C ABI requires rnd to point to a valid cn_rnd_t.
    let rnd = unsafe { &mut *rnd };
    let mut x = rnd.state[0];
    let y = rnd.state[1];
    rnd.state[0] = y;
    x ^= x << 23;
    x ^= x >> 17;
    x ^= y ^ (y >> 26);
    rnd.state[1] = x;

    let value = x.wrapping_add(y);
    let exponent = 1023_u64;
    let mantissa = value >> 12;
    let result = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}
