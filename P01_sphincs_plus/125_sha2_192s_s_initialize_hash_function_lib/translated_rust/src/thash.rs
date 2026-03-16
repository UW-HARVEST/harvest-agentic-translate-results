use crate::context::SpxCtx;
use crate::params::*;
use crate::sha2;

/// thash for simple variant: 1-block uses SHA-256, multi-block uses SHA-512
pub fn thash_internal(
    out: &mut [u8],
    in_data: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    if inblocks > 1 {
        thash_512(out, in_data, inblocks, ctx, addr);
        return;
    }

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);

    let buf_len = SPX_SHA256_ADDR_BYTES + (inblocks as usize) * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let addr_bytes: &[u8] = unsafe {
        core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32)
    };
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..buf_len].copy_from_slice(&in_data[..(inblocks as usize) * SPX_N]);

    sha2::sha256_inc_finalize_internal(&mut outbuf, &mut sha2_state, &buf, buf_len);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn thash_512(
    out: &mut [u8],
    in_data: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 72];
    sha2_state.copy_from_slice(&ctx.state_seeded_512);

    let buf_len = SPX_SHA256_ADDR_BYTES + (inblocks as usize) * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let addr_bytes: &[u8] = unsafe {
        core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32)
    };
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..buf_len].copy_from_slice(&in_data[..(inblocks as usize) * SPX_N]);

    sha2::sha512_inc_finalize_internal(&mut outbuf, &mut sha2_state, &buf, buf_len);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
