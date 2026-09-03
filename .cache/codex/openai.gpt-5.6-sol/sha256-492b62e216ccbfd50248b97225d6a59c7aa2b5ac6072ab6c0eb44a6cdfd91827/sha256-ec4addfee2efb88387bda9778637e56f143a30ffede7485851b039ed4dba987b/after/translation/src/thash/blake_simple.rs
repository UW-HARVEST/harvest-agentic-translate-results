use crate::blake::{blake256, blake512};
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::address_to_bytes;

pub fn thash<const BLOCKS: usize>(
    out: &mut [u8],
    input: Option<&[u8]>,
    ctx: &SpxCtx,
    addr: &[u32],
) {
    let input = input.unwrap_or(out);
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + BLOCKS * SPX_N];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&address_to_bytes(addr));
    buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&input[..BLOCKS * SPX_N]);
    if SPX_N >= 24 && BLOCKS > 1 {
        out[..SPX_N].copy_from_slice(&blake512(&buf)[..SPX_N]);
    } else {
        out[..SPX_N].copy_from_slice(&blake256(&buf)[..SPX_N]);
    }
}
