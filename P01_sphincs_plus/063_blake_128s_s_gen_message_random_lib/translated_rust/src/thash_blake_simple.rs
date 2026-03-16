use crate::blake256::blake256;
use crate::context::SpxCtx;
use crate::params::*;

/// Simple thash: no mask, just hash pub_seed || addr || in
pub fn thash(
    out: &mut [u8],
    input: &[u8],
    inblocks: usize,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..buf_len].copy_from_slice(&input[..inblocks * SPX_N]);

    blake256(&mut outbuf, &buf, buf_len as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
