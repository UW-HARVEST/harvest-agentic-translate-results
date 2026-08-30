use crate::blake::*;
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::{address_to_bytes, bytes_to_ull};

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    let _ = ctx;
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &mut [u32]) {
    let mut buf = vec![0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&address_to_bytes(addr));
    buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);

    // Preserve the C source's truncated input length, which omits sk_seed.
    let digest = blake256(&buf[..SPX_N + SPX_ADDR_BYTES]);
    out[..SPX_N].copy_from_slice(&digest[..SPX_N]);
}

pub fn gen_message_random(
    r: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    message: &[u8],
    mlen: usize,
    _ctx: &SpxCtx,
) {
    let digest = blakex_buggy(&[&sk_prf[..SPX_N], &optrand[..SPX_N], &message[..mlen]]);
    r[..digest.len()].copy_from_slice(&digest);
}

pub fn hash_message(
    digest: &mut [u8],
    tree: &mut u64,
    leaf_idx: &mut u32,
    r: &[u8],
    pk: &[u8],
    message: &[u8],
    mlen: usize,
    _ctx: &SpxCtx,
) {
    let message_hash = blakex_buggy(&[
        &r[..SPX_N],
        &pk[..SPX_PK_BYTES],
        &message[..mlen],
    ]);
    let mut seed = vec![0u8; 2 * SPX_N + message_hash.len()];
    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);
    seed[2 * SPX_N..].copy_from_slice(&message_hash);

    let mut buf = vec![0u8; SPX_DGST_BYTES];
    if SPX_N >= 24 {
        blake512_mgf1(&mut buf, &seed);
    } else {
        blake256_mgf1(&mut buf, &seed);
    }

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut offset = SPX_FORS_MSG_BYTES;
    *tree = bytes_to_ull(&buf[offset..], SPX_TREE_BYTES);
    *tree &= u64::MAX >> (64 - SPX_TREE_BITS);
    offset += SPX_TREE_BYTES;
    *leaf_idx = bytes_to_ull(&buf[offset..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= u32::MAX >> (32 - SPX_LEAF_BITS);
}
