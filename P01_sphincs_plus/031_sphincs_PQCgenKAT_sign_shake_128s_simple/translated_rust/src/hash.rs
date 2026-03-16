use crate::params::*;
use crate::address::*;
use crate::fips202;

pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
}

pub fn initialize_hash_function(_ctx: &SpxCtx) {
    // For SHAKE256, nothing to do
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let ab = addr_bytes(addr);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ab);
    buf[SPX_N + SPX_ADDR_BYTES..2 * SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ctx.sk_seed);
    fips202::shake256(&mut out[..SPX_N], SPX_N, &buf);
}

pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], _ctx: &SpxCtx) {
    let mut inc = fips202::Shake256Inc::new();
    inc.absorb(&sk_prf[..SPX_N]);
    inc.absorb(&optrand[..SPX_N]);
    inc.absorb(m);
    inc.finalize();
    inc.squeeze(&mut r[..SPX_N], SPX_N);
}

pub fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
                    r_val: &[u8], pk: &[u8], m: &[u8], _ctx: &SpxCtx) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut inc = fips202::Shake256Inc::new();
    inc.absorb(&r_val[..SPX_N]);
    inc.absorb(&pk[..SPX_PK_BYTES]);
    inc.absorb(m);
    inc.finalize();
    inc.squeeze(&mut buf, SPX_DGST_BYTES);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut off = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[off..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    off += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[off..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}

pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let ab = addr_bytes(addr);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ab);
    buf[SPX_N + SPX_ADDR_BYTES..buf_len].copy_from_slice(&inp[..inblocks * SPX_N]);
    fips202::shake256(&mut out[..SPX_N], SPX_N, &buf);
}
