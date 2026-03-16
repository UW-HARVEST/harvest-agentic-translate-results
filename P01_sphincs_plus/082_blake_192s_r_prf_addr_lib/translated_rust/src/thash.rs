use crate::blake256::{blake256, blake256_mgf1};
use crate::blake512::{blake512, blake512_mgf1};
use crate::context::SpxCtx;
use crate::params::*;

/// thash robust for blake - dispatches to 512 for inblocks > 1 (since SPX_BLAKE512 is set)
pub fn thash(
    out: &mut [u8],
    input: &[u8],
    inblocks: usize,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    if inblocks > 1 {
        thash_512(out, input, inblocks, ctx, addr);
        return;
    }
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);

    blake256_mgf1(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
    }

    blake256(
        &mut outbuf,
        &buf[SPX_N..],
        (SPX_ADDR_BYTES + inblocks * SPX_N) as u64,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn thash_512(
    out: &mut [u8],
    input: &[u8],
    inblocks: usize,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);

    blake512_mgf1(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
    }

    blake512(
        &mut outbuf,
        &buf[SPX_N..],
        (SPX_ADDR_BYTES + inblocks * SPX_N) as u64,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
