use crate::params::*;

pub fn ull_to_bytes(out: &mut [u8], outlen: usize, mut val: u64) {
    for i in (0..outlen).rev() {
        out[i] = (val & 0xff) as u8;
        val >>= 8;
    }
}

pub fn u32_to_bytes(out: &mut [u8], val: u32) {
    out[0] = (val >> 24) as u8;
    out[1] = (val >> 16) as u8;
    out[2] = (val >> 8) as u8;
    out[3] = val as u8;
}

pub fn bytes_to_ull(data: &[u8], inlen: usize) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (data[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

pub fn addr_as_bytes(addr: &[u32; 8]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES) }
}

pub fn addr_as_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8] {
    unsafe { std::slice::from_raw_parts_mut(addr.as_mut_ptr() as *mut u8, SPX_ADDR_BYTES) }
}
