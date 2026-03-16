/// Internal implementations of utils functions (used by blake modules)
/// These are also exported as extern "C" functions in lib.rs

pub fn ull_to_bytes_internal(out: &mut [u8], outlen: usize, mut val: u64) {
    for i in (0..outlen as isize).rev() {
        out[i as usize] = (val & 0xff) as u8;
        val >>= 8;
    }
}

pub fn u32_to_bytes_internal(out: &mut [u8], val: u32) {
    out[0] = (val >> 24) as u8;
    out[1] = (val >> 16) as u8;
    out[2] = (val >> 8) as u8;
    out[3] = val as u8;
}

pub fn bytes_to_ull_internal(inp: &[u8], inlen: usize) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (inp[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}
