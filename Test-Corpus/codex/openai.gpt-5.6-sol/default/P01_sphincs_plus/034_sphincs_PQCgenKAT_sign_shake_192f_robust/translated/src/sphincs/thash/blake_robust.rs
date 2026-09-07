use crate::{blake_impl::*, context::SpxCtx, params::*, utils::*};

pub fn thash<const B: usize>(
    out: &mut [u8], input: Option<&[u8]>, ctx: &SpxCtx, addr: &[u32],
) {
    let wide = SPX_N >= 24 && B > 1;
    let addr_bytes = address_to_bytes(addr);
    let mut seed = Vec::with_capacity(SPX_N + SPX_ADDR_BYTES);
    seed.extend_from_slice(&ctx.pub_seed);
    seed.extend_from_slice(&addr_bytes);
    let mut mask = vec![0u8; B * SPX_N];
    mgf1(&mut mask, &seed, wide);
    let source = input.unwrap_or(out);
    let mut buf = Vec::with_capacity(SPX_ADDR_BYTES + B * SPX_N);
    buf.extend_from_slice(&addr_bytes);
    buf.extend((0..B * SPX_N).map(|i| source[i] ^ mask[i]));
    let hash = if wide { blake512(&buf).to_vec() } else { blake256(&buf).to_vec() };
    out[..SPX_N].copy_from_slice(&hash[..SPX_N]);
}
