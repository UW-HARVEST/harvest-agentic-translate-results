use crate::params::*;
use crate::context::SpxCtx;
use sha3::{Shake256, digest::{Update, ExtendableOutput, XofReader}};

pub fn initialize_hash_function(_ctx: &mut SpxCtx) {}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut hasher = Shake256::default();
    hasher.update(&ctx.pub_seed);
    let addr_bytes = unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    hasher.update(addr_bytes);
    hasher.update(&ctx.sk_seed);
    let mut reader = hasher.finalize_xof();
    reader.read(&mut out[..SPX_N]);
}

pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], _ctx: &SpxCtx) {
    let mut hasher = Shake256::default();
    hasher.update(sk_prf);
    hasher.update(optrand);
    hasher.update(m);
    let mut reader = hasher.finalize_xof();
    reader.read(&mut r[..SPX_N]);
}

pub fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32, r: &[u8], pk: &[u8], m: &[u8], _ctx: &SpxCtx) {
    let mut hasher = Shake256::default();
    hasher.update(&r[..SPX_N]);
    hasher.update(&pk[..SPX_PK_BYTES]);
    hasher.update(m);
    let mut reader = hasher.finalize_xof();
    
    let mut buf = vec![0u8; SPX_DGST_BYTES];
    reader.read(&mut buf);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = crate::utils::bytes_to_ull(&buf[bufp..bufp + SPX_TREE_BYTES], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = crate::utils::bytes_to_ull(&buf[bufp..bufp + SPX_LEAF_BYTES], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
