#[repr(C)]
pub struct cn_rnd_t {
    pub state: [u64; 2],
}

fn cn_rnd_next(rnd: &mut cn_rnd_t) -> u64 {
    let mut x = rnd.state[0];
    let y = rnd.state[1];
    rnd.state[0] = y;
    x ^= x.wrapping_shl(23);
    x ^= x >> 17;
    x ^= y ^ (y >> 26);
    rnd.state[1] = x;
    x.wrapping_add(y)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn next_double(rnd: *mut cn_rnd_t) -> f64 {
    let value = cn_rnd_next(unsafe { &mut *rnd });
    let exponent = 1023_u64;
    let mantissa = value >> 12;
    let result = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}
