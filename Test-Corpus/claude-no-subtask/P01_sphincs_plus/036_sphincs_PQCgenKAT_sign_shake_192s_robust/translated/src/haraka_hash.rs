// Haraka hash implementation (PRF, message hash)
#![cfg(feature = "haraka")]
#![allow(dead_code)]

use crate::context::SpxCtx;
use crate::haraka::*;
use crate::params::*;
use crate::utils::bytes_to_ull;

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    tweak_constants(ctx);
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut outbuf = [0u8; 32];
    let mut buf = [0u8; 64];

    let addr_bytes: &[u8; 32] = unsafe { &*(addr.as_ptr() as *const [u8; 32]) };
    buf[..SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);

    haraka512(&mut outbuf, &buf, ctx);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

pub fn gen_message_random(
    r: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    mlen: u64,
    ctx: &SpxCtx,
) {
    let mut s_inc = [0u8; 65];
    haraka_S_inc_init(&mut s_inc);
    haraka_S_inc_absorb(&mut s_inc, sk_prf, SPX_N, ctx);
    haraka_S_inc_absorb(&mut s_inc, optrand, SPX_N, ctx);
    haraka_S_inc_absorb(&mut s_inc, m, mlen as usize, ctx);
    haraka_S_inc_finalize(&mut s_inc);
    haraka_S_inc_squeeze(r, SPX_N, &mut s_inc, ctx);
}

pub fn hash_message(
    digest: &mut [u8],
    tree: &mut u64,
    leaf_idx: &mut u32,
    r: &[u8],
    pk: &[u8],
    m: &[u8],
    mlen: u64,
    ctx: &SpxCtx,
) {
    const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

    let mut buf = vec![0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u8; 65];

    haraka_S_inc_init(&mut s_inc);
    haraka_S_inc_absorb(&mut s_inc, r, SPX_N, ctx);
    haraka_S_inc_absorb(&mut s_inc, &pk[SPX_N..], SPX_N, ctx);
    haraka_S_inc_absorb(&mut s_inc, m, mlen as usize, ctx);
    haraka_S_inc_finalize(&mut s_inc);
    haraka_S_inc_squeeze(&mut buf, SPX_DGST_BYTES, &mut s_inc, ctx);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let bufp_off = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[bufp_off..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }

    let bufp_off = bufp_off + SPX_TREE_BYTES;
    *leaf_idx = bytes_to_ull(&buf[bufp_off..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
