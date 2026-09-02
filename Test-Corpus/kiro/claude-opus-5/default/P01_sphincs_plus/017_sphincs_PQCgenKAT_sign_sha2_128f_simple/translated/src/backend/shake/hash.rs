//! Translation of `lib/shake/src/hash_shake.c`.

use super::fips202::*;
use crate::address::{addr_mut, addr_ref, Addr};
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::split_digest;
use core::ffi::c_ulonglong;

/// For SHAKE256, there is no immediate reason to initialize at the start,
/// so this function is an empty operation.
pub fn initialize_hash_function(_ctx: &mut SpxCtx) {}

/// Computes `PRF(pk_seed, sk_seed, addr)`.
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &Addr) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr);
    buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);

    shake256_rs(&mut out[..SPX_N], &buf);
}

/// Computes the message-dependent randomness R, using a secret seed and an
/// optional randomization value as well as the message.
pub fn gen_message_random(
    r: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    mlen: u64,
    _ctx: &SpxCtx,
) {
    let mut s_inc = [0u64; 26];

    shake256_inc_init_rs(&mut s_inc);
    shake256_inc_absorb_rs(&mut s_inc, &sk_prf[..SPX_N]);
    shake256_inc_absorb_rs(&mut s_inc, &optrand[..SPX_N]);
    shake256_inc_absorb_rs(&mut s_inc, &m[..mlen as usize]);
    shake256_inc_finalize_rs(&mut s_inc);
    shake256_inc_squeeze_rs(&mut r[..SPX_N], &mut s_inc);
}

/// Computes the message hash using R, the public key, and the message.
pub fn hash_message(
    digest: &mut [u8],
    tree: &mut u64,
    leaf_idx: &mut u32,
    r: &[u8],
    pk: &[u8],
    m: &[u8],
    mlen: u64,
    _ctx: &SpxCtx,
) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u64; 26];

    shake256_inc_init_rs(&mut s_inc);
    shake256_inc_absorb_rs(&mut s_inc, &r[..SPX_N]);
    shake256_inc_absorb_rs(&mut s_inc, &pk[..SPX_PK_BYTES]);
    shake256_inc_absorb_rs(&mut s_inc, &m[..mlen as usize]);
    shake256_inc_finalize_rs(&mut s_inc);
    shake256_inc_squeeze_rs(&mut buf, &mut s_inc);

    split_digest(digest, tree, leaf_idx, &buf);
}

// ---------------------------------------------------------------------------
// C ABI
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    initialize_hash_function(&mut *ctx);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let o = core::slice::from_raw_parts_mut(out, SPX_N);
    prf_addr(o, &*ctx, addr_ref(addr));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_gen_message_random(
    r: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: c_ulonglong,
    ctx: *const SpxCtx,
) {
    let r_s = core::slice::from_raw_parts_mut(r, SPX_N);
    let sk_s = core::slice::from_raw_parts(sk_prf, SPX_N);
    let opt_s = core::slice::from_raw_parts(optrand, SPX_N);
    let m_s = core::slice::from_raw_parts(m, mlen as usize);
    gen_message_random(r_s, sk_s, opt_s, m_s, mlen as u64, &*ctx);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    r: *const u8,
    pk: *const u8,
    m: *const u8,
    mlen: c_ulonglong,
    ctx: *const SpxCtx,
) {
    let d = core::slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES);
    let r_s = core::slice::from_raw_parts(r, SPX_N);
    let pk_s = core::slice::from_raw_parts(pk, SPX_PK_BYTES);
    let m_s = core::slice::from_raw_parts(m, mlen as usize);
    hash_message(d, &mut *tree, &mut *leaf_idx, r_s, pk_s, m_s, mlen as u64, &*ctx);
}

#[allow(dead_code)]
unsafe fn _unused(addr: *mut u32) -> &'static mut Addr {
    addr_mut(addr)
}
