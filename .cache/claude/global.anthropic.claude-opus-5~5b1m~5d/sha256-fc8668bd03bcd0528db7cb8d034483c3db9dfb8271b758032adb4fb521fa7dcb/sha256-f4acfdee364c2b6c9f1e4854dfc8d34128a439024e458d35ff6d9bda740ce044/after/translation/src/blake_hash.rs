//! Translation of `lib/blake/src/hash_blake.c`.
//!
//! IMPORTANT: this custom codebase calls `blakeX_update` directly with *byte*
//! counts, whereas the `update` routine interprets its length argument as a
//! *bit* count (only `blake256()`/`blake512()` multiply by 8). We reproduce
//! this behaviour exactly so the output is byte-identical.

use crate::address::addr_bytes;
use crate::blake256::{blake256, blake256_final, blake256_init, blake256_mgf1, BlakeState256};
use crate::blake512::{blake512_final, blake512_init, blake512_mgf1, blake512_update, BlakeState512};
use crate::blake256::blake256_update;
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::bytes_to_ull;

const BLAKEX_OUTPUT: usize = if SPX_N >= 24 { 64 } else { 32 };

pub fn initialize_hash_function(_ctx: &mut SpxCtx) {}

/// Computes PRF(key, addr). Always uses BLAKE-256, and (matching the C) only
/// hashes pub_seed || addr — the copied sk_seed bytes are not hashed.
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; 32];
    let ab = addr_bytes(addr);
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ab[..SPX_ADDR_BYTES]);
    buf[SPX_N + SPX_ADDR_BYTES..2 * SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ctx.sk_seed);

    blake256(&mut outbuf, &buf, (SPX_N + SPX_ADDR_BYTES) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

/// NOTE (faithful to `hash_blake.c:68`): the C code finalises straight into
/// `R`, so it writes the FULL `blakeX` digest — `SPX_BLAKE256_OUTPUT_BYTES`
/// (32) for the 128-bit level and 64 bytes for 192/256 — not just `SPX_N`
/// bytes. `crypto_sign_signature` passes `sig`, whose following bytes are
/// overwritten by `fors_sign` afterwards, so the extra bytes are harmless
/// there, but an external caller of `SPX_gen_message_random` observes them.
/// `r` must therefore be at least `BLAKEX_OUTPUT` bytes long.
pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], _ctx: &SpxCtx) {
    let mlen = m.len();
    if SPX_N >= 24 {
        let mut s = BlakeState512::new();
        blake512_init(&mut s);
        blake512_update(&mut s, &sk_prf[..SPX_N], SPX_N as u64);
        blake512_update(&mut s, &optrand[..SPX_N], SPX_N as u64);
        blake512_update(&mut s, m, mlen as u64);
        blake512_final(&mut s, &mut r[..BLAKEX_OUTPUT]);
    } else {
        let mut s = BlakeState256::new();
        blake256_init(&mut s);
        blake256_update(&mut s, &sk_prf[..SPX_N], SPX_N as u64);
        blake256_update(&mut s, &optrand[..SPX_N], SPX_N as u64);
        blake256_update(&mut s, m, mlen as u64);
        blake256_final(&mut s, &mut r[..BLAKEX_OUTPUT]);
    }
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
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut seed = [0u8; 2 * SPX_N + BLAKEX_OUTPUT];
    let mlen = m.len();

    if SPX_N >= 24 {
        let mut s = BlakeState512::new();
        blake512_init(&mut s);
        blake512_update(&mut s, &r[..SPX_N], SPX_N as u64);
        blake512_update(&mut s, &pk[..SPX_PK_BYTES], SPX_PK_BYTES as u64);
        blake512_update(&mut s, m, mlen as u64);
        blake512_final(&mut s, &mut seed[2 * SPX_N..]);
    } else {
        let mut s = BlakeState256::new();
        blake256_init(&mut s);
        blake256_update(&mut s, &r[..SPX_N], SPX_N as u64);
        blake256_update(&mut s, &pk[..SPX_PK_BYTES], SPX_PK_BYTES as u64);
        blake256_update(&mut s, m, mlen as u64);
        blake256_final(&mut s, &mut seed[2 * SPX_N..]);
    }

    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    if SPX_N >= 24 {
        blake512_mgf1(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + BLAKEX_OUTPUT);
    } else {
        blake256_mgf1(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + BLAKEX_OUTPUT);
    }

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

// ------------------------------------------------------------------
// Exported C ABI wrappers.
// ------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    initialize_hash_function(&mut *ctx);
}
#[no_mangle]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    prf_addr(core::slice::from_raw_parts_mut(out, SPX_N), &*ctx, &*(addr as *const [u32; 8]));
}
#[no_mangle]
pub unsafe extern "C" fn SPX_gen_message_random(
    r: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: core::ffi::c_ulonglong,
    ctx: *const SpxCtx,
) {
    // See the note on `gen_message_random`: the C writes the full blakeX
    // digest to `R`, so the exported wrapper must expose that many bytes.
    gen_message_random(
        core::slice::from_raw_parts_mut(r, BLAKEX_OUTPUT),
        core::slice::from_raw_parts(sk_prf, SPX_N),
        core::slice::from_raw_parts(optrand, SPX_N),
        core::slice::from_raw_parts(m, mlen as usize),
        &*ctx,
    );
}
#[no_mangle]
pub unsafe extern "C" fn SPX_hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    r: *const u8,
    pk: *const u8,
    m: *const u8,
    mlen: core::ffi::c_ulonglong,
    ctx: *const SpxCtx,
) {
    hash_message(
        core::slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES),
        &mut *tree,
        &mut *leaf_idx,
        core::slice::from_raw_parts(r, SPX_N),
        core::slice::from_raw_parts(pk, SPX_PK_BYTES),
        core::slice::from_raw_parts(m, mlen as usize),
        &*ctx,
    );
}



