//! Translation of `lib/haraka/src/thash_haraka_robust.c`.

use crate::address::addr_bytes;
use crate::context::SpxCtx;
use crate::haraka::haraka::{haraka256, haraka512, haraka_s};
use crate::params::*;
use crate::vla::Vla;

/// Takes an array of `inblocks` concatenated arrays of `SPX_N` bytes.
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut vla =
        Vla::<{ SPX_ADDR_BYTES + SPX_MAX_INBLOCKS * SPX_N }>::new(SPX_ADDR_BYTES + inblocks * SPX_N);
    let buf = vla.as_mut_slice();
    let mut mask_vla = Vla::<{ SPX_MAX_INBLOCKS * SPX_N }>::new(inblocks * SPX_N);
    let bitmask = mask_vla.as_mut_slice();
    let mut outbuf = [0u8; 32];
    let mut buf_tmp = [0u8; 64];

    if inblocks == 1 {
        /* F function */
        /* Since SPX_N may be smaller than 32, we need a temporary buffer. */
        buf_tmp.fill(0);
        buf_tmp[..32].copy_from_slice(addr_bytes(addr));

        let head: [u8; 32] = buf_tmp[..32].try_into().unwrap();
        haraka256(&mut outbuf, &head, ctx);
        for i in 0..inblocks * SPX_N {
            buf_tmp[SPX_ADDR_BYTES + i] = inp[i] ^ outbuf[i];
        }
        haraka512(&mut outbuf, &buf_tmp, ctx);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    } else {
        /* All other tweakable hashes */
        let n = inblocks * SPX_N;
        buf[..32].copy_from_slice(addr_bytes(addr));
        haraka_s(
            &mut bitmask[..n],
            n as u64,
            &buf[..SPX_ADDR_BYTES],
            SPX_ADDR_BYTES as u64,
            ctx,
        );

        for i in 0..n {
            buf[SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
        }

        let total = SPX_ADDR_BYTES + n;
        let (out_part, buf_part) = (&mut out[..SPX_N], &buf[..total]);
        haraka_s(
            out_part,
            SPX_N as u64,
            buf_part,
            total as u64,
            ctx,
        );
    }
}
