use crate::blake256::{blake256, blake256_mgf1};
use crate::params::*;

/// thash - supports in-place (out and inp may overlap).
/// Caller can pass the same slice for out and inp.
pub fn thash(
    out: &mut [u8],
    inp: &[u8],
    inblocks: usize,
    ctx: &SpxCtx,
    addr: &[u32; 8],
) {
    // Copy input to handle aliasing
    let inp_copy: Vec<u8> = inp[..inblocks * SPX_N].to_vec();
    let inp = &inp_copy;
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes: &[u8] = unsafe {
        core::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES)
    };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);

    blake256_mgf1(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    blake256(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + inblocks * SPX_N) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
