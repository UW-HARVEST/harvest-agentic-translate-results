use crate::params::*;
use crate::context::SpxCtx;
use crate::address;
use crate::fips202;
use crate::utils;

pub fn initialize_hash_function(_ctx: &mut SpxCtx) {}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(address::addr_bytes(addr));
    buf[SPX_N + SPX_ADDR_BYTES..2 * SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ctx.sk_seed);
    fips202::shake256(out, SPX_N, &buf);
}

pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8],
                          m: &[u8], mlen: usize, _ctx: &SpxCtx) {
    let mut s_inc = [0u64; 26];
    fips202::shake256_inc_init(&mut s_inc);
    fips202::shake256_inc_absorb(&mut s_inc, &sk_prf[..SPX_N]);
    fips202::shake256_inc_absorb(&mut s_inc, &optrand[..SPX_N]);
    fips202::shake256_inc_absorb(&mut s_inc, &m[..mlen]);
    fips202::shake256_inc_finalize(&mut s_inc);
    fips202::shake256_inc_squeeze(r, SPX_N, &mut s_inc);
}

pub fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
                    r: &[u8], pk: &[u8], m: &[u8], mlen: usize, _ctx: &SpxCtx) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u64; 26];

    fips202::shake256_inc_init(&mut s_inc);
    fips202::shake256_inc_absorb(&mut s_inc, &r[..SPX_N]);
    fips202::shake256_inc_absorb(&mut s_inc, &pk[..SPX_PK_BYTES]);
    fips202::shake256_inc_absorb(&mut s_inc, &m[..mlen]);
    fips202::shake256_inc_finalize(&mut s_inc);
    fips202::shake256_inc_squeeze(&mut buf, SPX_DGST_BYTES, &mut s_inc);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = utils::bytes_to_ull(&buf[SPX_FORS_MSG_BYTES..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }

    *leaf_idx = utils::bytes_to_ull(&buf[SPX_FORS_MSG_BYTES + SPX_TREE_BYTES..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
