// Translation of c_src/src/lib.c and c_src/include/lib.h
// The original C code is a shared library (no main()), so this executable
// produces no output, matching the behavior of an empty main.

#[derive(Clone, Copy, Debug)]
pub struct CnRndT {
    pub state: [u64; 2],
}

fn cn_rnd_next(rnd: &mut CnRndT) -> u64 {
    let mut x = rnd.state[0];
    let y = rnd.state[1];
    rnd.state[0] = y;
    x ^= x << 23;
    x ^= x >> 17;
    x ^= y ^ (y >> 26);
    rnd.state[1] = x;
    x.wrapping_add(y)
}

pub fn next_double(rnd: &mut CnRndT) -> f64 {
    let value = cn_rnd_next(rnd);
    let exponent: u64 = 1023;
    let mantissa: u64 = value >> 12;
    let result: u64 = (exponent << 52) | mantissa;
    f64::from_bits(result) - 1.0
}

fn main() {
    // The original C source defines only library functions and no main(),
    // so the executable produces no output.
}
