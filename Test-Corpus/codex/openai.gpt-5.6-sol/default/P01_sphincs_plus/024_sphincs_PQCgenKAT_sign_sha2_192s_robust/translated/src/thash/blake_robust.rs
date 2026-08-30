use crate::blake::{blake256, blake256_mgf1, blake512, blake512_mgf1};
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::address_to_bytes;

pub fn thash<const BLOCKS: usize>(
    out: &mut [u8],
    input: Option<&[u8]>,
    ctx: &SpxCtx,
    addr: &[u32],
) where
    [(); SPX_N + SPX_ADDR_BYTES + BLOCKS * SPX_N]: Sized,
{
    let input = input.unwrap_or(out);
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + BLOCKS * SPX_N];
    let mut bitmask = vec![0u8; BLOCKS * SPX_N];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&address_to_bytes(addr));

    if SPX_N >= 24 && BLOCKS > 1 {
        blake512_mgf1(&mut bitmask, &buf[..SPX_N + SPX_ADDR_BYTES]);
    } else {
        blake256_mgf1(&mut bitmask, &buf[..SPX_N + SPX_ADDR_BYTES]);
    }
    for i in 0..BLOCKS * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
    }

    // The robust C implementation excludes pub_seed from the final hash.
    let final_input = &buf[SPX_N..];
    if SPX_N >= 24 && BLOCKS > 1 {
        out[..SPX_N].copy_from_slice(&blake512(final_input)[..SPX_N]);
    } else {
        out[..SPX_N].copy_from_slice(&blake256(final_input)[..SPX_N]);
    }
}

