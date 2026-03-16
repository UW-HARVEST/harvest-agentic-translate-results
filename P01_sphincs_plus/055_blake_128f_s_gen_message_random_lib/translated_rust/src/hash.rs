use crate::blake256::{blake256, blake256_mgf1, Blake256State};
use crate::context::SpxCtx;
use crate::params::*;
use crate::address::*;

pub fn initialize_hash_function(_ctx: &mut SpxCtx) {
    // No-op for BLAKE
}

/// PRF(key, addr) - note: hashes only SPX_N + SPX_ADDR_BYTES bytes
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u8; 32]) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr);
    buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);

    blake256(&mut outbuf, &buf, (SPX_N + SPX_ADDR_BYTES) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

/// gen_message_random using blake256 (since SPX_N < 24)
pub fn gen_message_random(
    r: &mut [u8], sk_prf: &[u8], optrand: &[u8],
    m: &[u8], mlen: u64, _ctx: &SpxCtx,
) {
    let mut s = Blake256State::new();
    s.update(sk_prf, (SPX_N as u64) * 8);
    s.update(optrand, (SPX_N as u64) * 8);
    s.update(m, mlen * 8);
    let mut full_out = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    s.finalize(&mut full_out);
    r[..SPX_N].copy_from_slice(&full_out[..SPX_N]);
}

/// hash_message using blake256 + blake256_mgf1
pub fn hash_message(
    digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
    r: &[u8], pk: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx,
) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut seed = [0u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES];

    let mut s = Blake256State::new();
    s.update(r, (SPX_N as u64) * 8);
    s.update(pk, (SPX_PK_BYTES as u64) * 8);
    s.update(m, mlen * 8);
    s.finalize(&mut seed[2 * SPX_N..]);

    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    blake256_mgf1(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

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

/// thash - SIMPLE variant (no bitmask)
pub fn thash(
    out: &mut [u8], input: &[u8], inblocks: usize,
    ctx: &SpxCtx, addr: &[u8; 32],
) {
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr);
    buf[SPX_N + SPX_ADDR_BYTES..buf_len].copy_from_slice(&input[..inblocks * SPX_N]);

    blake256(&mut outbuf, &buf, buf_len as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
