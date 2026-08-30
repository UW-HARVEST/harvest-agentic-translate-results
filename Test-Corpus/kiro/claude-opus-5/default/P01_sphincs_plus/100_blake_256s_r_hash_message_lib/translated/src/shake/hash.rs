//! Translation of `lib/shake/src/hash_shake.c`.

use core::ffi::c_ulonglong;

use crate::address::addr_bytes;
use crate::context::SpxCtx;
use crate::params::*;
use crate::shake::fips202::{
    shake256, shake256_inc_absorb, shake256_inc_finalize, shake256_inc_init, shake256_inc_squeeze,
};
use crate::utils::bytes_to_ull;

/// For SHAKE256, there is no immediate reason to initialize at the start, so
/// this function is an empty operation.
pub fn initialize_hash_function(_ctx: &mut SpxCtx) {
    /* Suppress an 'unused parameter' warning. */
}

/// Computes `PRF(pk_seed, sk_seed, addr)`.
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes(addr));
    buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);

    shake256(out, SPX_N, &buf, 2 * SPX_N + SPX_ADDR_BYTES);
}

/// Computes the message-dependent randomness `R`, using a secret seed and an
/// optional randomization value as well as the message.
pub fn gen_message_random(
    r_out: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    _ctx: &SpxCtx,
) {
    let mut s_inc = [0u64; 26];

    shake256_inc_init(&mut s_inc);
    shake256_inc_absorb(&mut s_inc, sk_prf, SPX_N);
    shake256_inc_absorb(&mut s_inc, optrand, SPX_N);
    shake256_inc_absorb(&mut s_inc, m, m.len());
    shake256_inc_finalize(&mut s_inc);
    shake256_inc_squeeze(r_out, SPX_N, &mut s_inc);
}

/// Computes the message hash using `R`, the public key, and the message.
pub fn hash_message(
    digest: &mut [u8],
    tree: &mut u64,
    leaf_idx: &mut u32,
    r_in: &[u8],
    pk: &[u8],
    m: &[u8],
    _ctx: &SpxCtx,
) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u64; 26];

    shake256_inc_init(&mut s_inc);
    shake256_inc_absorb(&mut s_inc, r_in, SPX_N);
    shake256_inc_absorb(&mut s_inc, pk, SPX_PK_BYTES);
    shake256_inc_absorb(&mut s_inc, m, m.len());
    shake256_inc_finalize(&mut s_inc);
    shake256_inc_squeeze(&mut buf, SPX_DGST_BYTES, &mut s_inc);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[bufp..bufp + SPX_TREE_BYTES]);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[bufp..bufp + SPX_LEAF_BYTES]) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}

// ---------------------------------------------------------------------------
// C ABI.  `hash.h` renames everything through `SPX_NAMESPACE`.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    unsafe { initialize_hash_function(&mut *ctx) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    unsafe {
        prf_addr(
            core::slice::from_raw_parts_mut(out, SPX_N),
            &*ctx,
            &*(addr as *const [u32; 8]),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_gen_message_random(
    r_out: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: c_ulonglong,
    ctx: *const SpxCtx,
) {
    unsafe {
        gen_message_random(
            core::slice::from_raw_parts_mut(r_out, SPX_N),
            core::slice::from_raw_parts(sk_prf, SPX_N),
            core::slice::from_raw_parts(optrand, SPX_N),
            core::slice::from_raw_parts(m, mlen as usize),
            &*ctx,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    r_in: *const u8,
    pk: *const u8,
    m: *const u8,
    mlen: c_ulonglong,
    ctx: *const SpxCtx,
) {
    unsafe {
        hash_message(
            core::slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES),
            &mut *tree,
            &mut *leaf_idx,
            core::slice::from_raw_parts(r_in, SPX_N),
            core::slice::from_raw_parts(pk, SPX_PK_BYTES),
            core::slice::from_raw_parts(m, mlen as usize),
            &*ctx,
        )
    }
}
