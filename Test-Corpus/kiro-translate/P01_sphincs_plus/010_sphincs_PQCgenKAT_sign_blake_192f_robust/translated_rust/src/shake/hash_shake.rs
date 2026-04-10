use crate::params::*;
use crate::context::SpxCtx;
use crate::utils::bytes_to_ull;
use crate::shake::fips202::{shake256, shake256_inc_init, shake256_inc_absorb, shake256_inc_finalize, shake256_inc_squeeze};

pub fn initialize_hash_function(_ctx: &mut SpxCtx) {}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = vec![0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe { &*(addr as *const [u32; 8] as *const [u8; SPX_ADDR_BYTES]) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..2 * SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ctx.sk_seed);
    shake256(out, SPX_N, &buf);
}

pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], _mlen: u64, _ctx: &SpxCtx) {
    let mut s_inc = [0u64; 26];
    shake256_inc_init(&mut s_inc);
    shake256_inc_absorb(&mut s_inc, &sk_prf[..SPX_N]);
    shake256_inc_absorb(&mut s_inc, &optrand[..SPX_N]);
    shake256_inc_absorb(&mut s_inc, m);
    shake256_inc_finalize(&mut s_inc);
    shake256_inc_squeeze(r, SPX_N, &mut s_inc);
}

pub fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32, r: &[u8], pk: &[u8], m: &[u8], _mlen: u64, _ctx: &SpxCtx) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u64; 26];

    shake256_inc_init(&mut s_inc);
    shake256_inc_absorb(&mut s_inc, &r[..SPX_N]);
    shake256_inc_absorb(&mut s_inc, &pk[..SPX_PK_BYTES]);
    shake256_inc_absorb(&mut s_inc, m);
    shake256_inc_finalize(&mut s_inc);
    shake256_inc_squeeze(&mut buf, SPX_DGST_BYTES, &mut s_inc);

    let mut bufp = 0;
    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[bufp..bufp + SPX_FORS_MSG_BYTES]);
    bufp += SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
