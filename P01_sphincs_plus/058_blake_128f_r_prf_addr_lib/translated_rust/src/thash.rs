use crate::params::*;
use crate::blake256;

/// thash - robust variant for blake-128f (SPX_BLAKE512 = false)
/// Takes an array of inblocks concatenated arrays of SPX_N bytes.
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe {
        core::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES)
    };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);

    blake256::blake256_mgf1(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    blake256::blake256(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + inblocks * SPX_N) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[repr(C)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
}
