use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::bytes_to_ull;
use sha2::{Digest, Sha256};

pub fn initialize_hash_function(_ctx: &mut SpxCtx) {}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let addr_bytes = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
    let mut h = Sha256::new();
    h.update(ctx.pub_seed);
    h.update(addr_bytes);
    h.update(ctx.sk_seed);
    let digest = h.finalize();
    out[..SPX_N].copy_from_slice(&digest[..SPX_N]);
}

pub fn gen_message_random(
    r: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    _ctx: &SpxCtx,
) {
    let mut h = Sha256::new();
    h.update(sk_prf);
    h.update(optrand);
    h.update(m);
    let digest = h.finalize();
    r[..SPX_N].copy_from_slice(&digest[..SPX_N]);
}

pub fn hash_message(
    digest: &mut [u8],
    tree: &mut u64,
    leaf_idx: &mut u32,
    r: &[u8],
    pk: &[u8],
    m: &[u8],
    _ctx: &SpxCtx,
) {
    const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = SPX_TREE_BITS.div_ceil(8);
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = SPX_LEAF_BITS.div_ceil(8);
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;
    let mut outbuf = vec![0u8; SPX_DGST_BYTES];
    let mut ctr = 0u32;
    let mut filled = 0usize;
    while filled < SPX_DGST_BYTES {
        let mut h = Sha256::new();
        h.update(r);
        h.update(pk);
        h.update(m);
        h.update(ctr.to_be_bytes());
        let block = h.finalize();
        let take = (SPX_DGST_BYTES - filled).min(block.len());
        outbuf[filled..filled + take].copy_from_slice(&block[..take]);
        filled += take;
        ctr = ctr.wrapping_add(1);
    }
    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&outbuf[..SPX_FORS_MSG_BYTES]);
    let mut off = SPX_FORS_MSG_BYTES;
    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&outbuf[off..off + SPX_TREE_BYTES], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    off += SPX_TREE_BYTES;
    *leaf_idx = bytes_to_ull(&outbuf[off..off + SPX_LEAF_BYTES], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
