use crate::address::Addr;
use crate::context::SpxCtx;
use crate::fips202;
use crate::params::*;
use crate::utils::bytes_to_ull;

pub fn initialize_hash_function(_ctx: &SpxCtx) {}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &Addr) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr);
    buf[SPX_N + SPX_ADDR_BYTES..2 * SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ctx.sk_seed);
    fips202::shake256(&mut out[..SPX_N], &buf);
}

pub fn gen_message_random(
    r: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    _ctx: &SpxCtx,
) {
    let mut s_inc = [0u64; 26];
    fips202::shake256_inc_init(&mut s_inc);
    fips202::shake256_inc_absorb(&mut s_inc, &sk_prf[..SPX_N]);
    fips202::shake256_inc_absorb(&mut s_inc, &optrand[..SPX_N]);
    fips202::shake256_inc_absorb(&mut s_inc, m);
    fips202::shake256_inc_finalize(&mut s_inc);
    fips202::shake256_inc_squeeze(&mut r[..SPX_N], &mut s_inc);
}

pub fn hash_message(
    digest: &mut [u8],
    tree: &mut u64,
    leaf_idx: &mut u32,
    r_val: &[u8],
    pk: &[u8],
    m: &[u8],
    _ctx: &SpxCtx,
) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u64; 26];

    fips202::shake256_inc_init(&mut s_inc);
    fips202::shake256_inc_absorb(&mut s_inc, &r_val[..SPX_N]);
    fips202::shake256_inc_absorb(&mut s_inc, &pk[..SPX_PK_BYTES]);
    fips202::shake256_inc_absorb(&mut s_inc, m);
    fips202::shake256_inc_finalize(&mut s_inc);
    fips202::shake256_inc_squeeze(&mut buf, &mut s_inc);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);

    let bufp = &buf[SPX_FORS_MSG_BYTES..];
    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(bufp, SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }

    let bufp2 = &buf[SPX_FORS_MSG_BYTES + SPX_TREE_BYTES..];
    *leaf_idx = bytes_to_ull(bufp2, SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
