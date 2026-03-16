use crate::params::*;
use crate::fips202::{shake256, Shake256Inc};
use crate::address::*;

pub fn initialize_hash_function(_ctx: &SpxCtx) {
    // No-op for SHAKE
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes(addr));
    buf[SPX_N + SPX_ADDR_BYTES..2 * SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ctx.sk_seed);
    shake256(&mut out[..SPX_N], &buf);
}

pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], ctx: &SpxCtx) {
    let _ = ctx;
    let mut s = Shake256Inc::new();
    s.absorb(&sk_prf[..SPX_N]);
    s.absorb(&optrand[..SPX_N]);
    s.absorb(m);
    s.finalize();
    s.squeeze(&mut r[..SPX_N]);
}

pub fn hash_message(
    digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
    r_val: &[u8], pk: &[u8], m: &[u8], ctx: &SpxCtx,
) {
    let _ = ctx;
    const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s = Shake256Inc::new();
    s.absorb(&r_val[..SPX_N]);
    s.absorb(&pk[..SPX_PK_BYTES]);
    s.absorb(m);
    s.finalize();
    s.squeeze(&mut buf);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut off = SPX_FORS_MSG_BYTES;

    *tree = bytes_to_ull(&buf[off..], SPX_TREE_BYTES);
    *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    off += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[off..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}

// thash - robust variant
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let mut bitmask = vec![0u8; inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes(addr));

    shake256(&mut bitmask, &buf[..SPX_N + SPX_ADDR_BYTES]);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    shake256(&mut out[..SPX_N], &buf);
}
