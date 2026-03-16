use crate::context::SpxCtx;
use crate::params::*;
use crate::sha2::sha256_inc_finalize;

pub fn thash(
    out: &mut [u8],
    input: &[u8],
    inblocks: usize,
    ctx: &SpxCtx,
    addr: &[u32; 8],
) {
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 40];
    let buf_len = SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];

    sha2_state.copy_from_slice(&ctx.state_seeded);

    let addr_bytes: &[u8] = unsafe {
        core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32)
    };
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..buf_len].copy_from_slice(&input[..inblocks * SPX_N]);

    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, buf_len);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
