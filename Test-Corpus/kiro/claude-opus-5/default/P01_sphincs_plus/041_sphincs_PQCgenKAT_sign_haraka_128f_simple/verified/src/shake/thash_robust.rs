//! Translation of `lib/shake/src/thash_shake_robust.c`.

use crate::address::addr_bytes;
use crate::context::SpxCtx;
use crate::params::*;
use crate::shake::fips202::shake256;
use crate::vla::Vla;

/// Takes an array of `inblocks` concatenated arrays of `SPX_N` bytes.
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let n = inblocks * SPX_N;
    let mut vla =
        Vla::<{ SPX_N + SPX_ADDR_BYTES + SPX_MAX_INBLOCKS * SPX_N }>::new(SPX_N + SPX_ADDR_BYTES + n);
    let buf = vla.as_mut_slice();
    let mut mask_vla = Vla::<{ SPX_MAX_INBLOCKS * SPX_N }>::new(n);
    let bitmask = mask_vla.as_mut_slice();

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes(addr));

    shake256(
        &mut bitmask[..n],
        n,
        &buf[..SPX_N + SPX_ADDR_BYTES],
        SPX_N + SPX_ADDR_BYTES,
    );

    for i in 0..n {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    let total = SPX_N + SPX_ADDR_BYTES + n;
    shake256(&mut out[..SPX_N], SPX_N, &buf[..total], total);
}
