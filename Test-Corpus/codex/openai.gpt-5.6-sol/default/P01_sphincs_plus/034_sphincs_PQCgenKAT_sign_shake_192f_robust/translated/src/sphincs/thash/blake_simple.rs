use crate::{blake_impl::*, context::SpxCtx, params::*, utils::*};

pub fn thash<const B: usize>(
    out: &mut [u8], input: Option<&[u8]>, ctx: &SpxCtx, addr: &[u32],
) {
    let mut buf = Vec::with_capacity(SPX_N + SPX_ADDR_BYTES + B * SPX_N);
    buf.extend_from_slice(&ctx.pub_seed);
    buf.extend_from_slice(&address_to_bytes(addr));
    buf.extend_from_slice(&input.unwrap_or(out)[..B * SPX_N]);
    let hash = if SPX_N >= 24 && B > 1 { blake512(&buf).to_vec() } else { blake256(&buf).to_vec() };
    out[..SPX_N].copy_from_slice(&hash[..SPX_N]);
}
