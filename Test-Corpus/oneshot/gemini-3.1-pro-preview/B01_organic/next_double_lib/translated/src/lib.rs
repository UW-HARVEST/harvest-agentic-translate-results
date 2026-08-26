#[repr(C)]
pub struct cn_rnd_t {
    pub state: [u64; 2],
}

impl cn_rnd_t {
    fn next(&mut self) -> u64 {
        let mut x = self.state[0];
        let y = self.state[1];
        self.state[0] = y;
        x ^= x << 23;
        x ^= x >> 17;
        x ^= y ^ (y >> 26);
        self.state[1] = x;
        x.wrapping_add(y)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn next_double(rnd: *mut cn_rnd_t) -> f64 {
    let rnd_ref = unsafe { &mut *rnd };
    let value = rnd_ref.next();
    let exponent: u64 = 1023;
    let mantissa = value >> 12;
    let result = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}
