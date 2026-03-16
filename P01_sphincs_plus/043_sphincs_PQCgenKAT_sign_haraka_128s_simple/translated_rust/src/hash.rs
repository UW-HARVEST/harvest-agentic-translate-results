use crate::context::SpxCtx;
use crate::params::*;
use crate::haraka::*;
use crate::address::bytes_to_ull;

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    tweak_constants(ctx);
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u8; 32]) {
    let mut outbuf = [0u8; 32];
    let mut buf = [0u8; 64];
    buf[..32].copy_from_slice(addr);
    buf[32..32 + SPX_N].copy_from_slice(&ctx.sk_seed);
    haraka512(&mut outbuf, &buf, ctx);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], ctx: &SpxCtx) {
    let mut s_inc = [0u8; 65];
    haraka_s_inc_init(&mut s_inc);
    haraka_s_inc_absorb(&mut s_inc, &sk_prf[..SPX_N], ctx);
    haraka_s_inc_absorb(&mut s_inc, &optrand[..SPX_N], ctx);
    haraka_s_inc_absorb(&mut s_inc, m, ctx);
    haraka_s_inc_finalize(&mut s_inc);
    haraka_s_inc_squeeze(&mut r[..SPX_N], SPX_N, &mut s_inc, ctx);
}

pub fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
                    r: &[u8], pk: &[u8], m: &[u8], ctx: &SpxCtx) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u8; 65];
    haraka_s_inc_init(&mut s_inc);
    haraka_s_inc_absorb(&mut s_inc, &r[..SPX_N], ctx);
    haraka_s_inc_absorb(&mut s_inc, &pk[SPX_N..2 * SPX_N], ctx);
    haraka_s_inc_absorb(&mut s_inc, m, ctx);
    haraka_s_inc_finalize(&mut s_inc);
    haraka_s_inc_squeeze(&mut buf, SPX_DGST_BYTES, &mut s_inc, ctx);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let bufp = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }

    *leaf_idx = bytes_to_ull(&buf[bufp + SPX_TREE_BYTES..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
