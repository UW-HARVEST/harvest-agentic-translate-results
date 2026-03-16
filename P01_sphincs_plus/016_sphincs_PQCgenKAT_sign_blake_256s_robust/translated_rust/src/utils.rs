pub fn ull_to_bytes(out: &mut [u8], val: u64) {
    let outlen = out.len();
    for i in (0..outlen).rev() {
        out[i] = ((val >> (8 * (outlen - 1 - i))) & 0xff) as u8;
    }
}

pub fn u32_to_bytes(out: &mut [u8], val: u32) {
    out[0] = (val >> 24) as u8;
    out[1] = (val >> 16) as u8;
    out[2] = (val >> 8) as u8;
    out[3] = val as u8;
}

pub fn bytes_to_ull(inp: &[u8]) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inp.len() {
        retval |= (inp[i] as u64) << (8 * (inp.len() - 1 - i));
    }
    retval
}
