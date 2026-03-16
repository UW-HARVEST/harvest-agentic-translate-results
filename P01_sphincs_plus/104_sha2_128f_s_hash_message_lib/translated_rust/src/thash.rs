use crate::address::*;
use crate::context::SpxCtx;
use crate::params::*;
use crate::sha2::*;

/// thash for SHA2-simple (no SHA-512 since SPX_SHA512=0).
pub fn thash(
    out: &mut [u8],
    in_data: &[u8],
    inblocks: usize,
    ctx: &SpxCtx,
    addr: &[u32; 8],
) {
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);

    let buf_len = SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];

    let addr_bytes = crate::address::addr_bytes(addr);
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..buf_len].copy_from_slice(&in_data[..inblocks * SPX_N]);

    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, buf_len);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
