//! Translation of `lib/haraka/src/hash_haraka.c`.

use super::haraka::*;
use crate::address::{addr_mut, addr_ref, Addr};
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::split_digest;
use core::ffi::c_ulonglong;

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    tweak_constants_rs(ctx);
}

/// Computes `PRF(key, addr)`, given a secret key of `SPX_N` bytes and an
/// address.
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &Addr) {
    /* Since SPX_N may be smaller than 32, we need temporary buffers. */
    let mut outbuf = [0u8; 32];
    let mut buf = [0u8; 64];

    buf[..SPX_ADDR_BYTES].copy_from_slice(addr);
    buf[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);

    haraka512_rs(&mut outbuf, &buf, ctx);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

/// Computes the message-dependent randomness R, using a secret seed and an
/// optional randomization value as well as the message.
pub fn gen_message_random(
    r: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    mlen: u64,
    ctx: &SpxCtx,
) {
    let mut s_inc = [0u8; 65];

    haraka_s_inc_init_rs(&mut s_inc);
    haraka_s_inc_absorb_rs(&mut s_inc, &sk_prf[..SPX_N], ctx);
    haraka_s_inc_absorb_rs(&mut s_inc, &optrand[..SPX_N], ctx);
    haraka_s_inc_absorb_rs(&mut s_inc, &m[..mlen as usize], ctx);
    haraka_s_inc_finalize_rs(&mut s_inc);
    haraka_s_inc_squeeze_rs(&mut r[..SPX_N], &mut s_inc, ctx);
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
    ctx: &SpxCtx,
) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u8; 65];

    haraka_s_inc_init_rs(&mut s_inc);
    haraka_s_inc_absorb_rs(&mut s_inc, &r[..SPX_N], ctx);
    /* Only absorb root part of pk */
    haraka_s_inc_absorb_rs(&mut s_inc, &pk[SPX_N..2 * SPX_N], ctx);
    haraka_s_inc_absorb_rs(&mut s_inc, &m[..mlen as usize], ctx);
    haraka_s_inc_finalize_rs(&mut s_inc);
    haraka_s_inc_squeeze_rs(&mut buf, &mut s_inc, ctx);

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
