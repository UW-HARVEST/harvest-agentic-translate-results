//! Translation of `lib/blake/src/hash_blake.c`.
//!
//! NOTE: `blakeX_update()` takes its length argument in *bits*, but
//! `hash_blake.c` passes byte counts (`SPX_N`, `SPX_PK_BYTES`, `mlen`).  That is
//! reproduced verbatim here.

use super::blake256::*;
#[cfg(spx_n_ge_24)]
use super::blake512::*;
use crate::address::{addr_mut, addr_ref, Addr};
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::split_digest;
use core::ffi::c_ulonglong;

// The `#if SPX_N >= 24` block at the top of hash_blake.c ("blakeX").
#[cfg(spx_n_ge_24)]
const SPX_BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE512_OUTPUT_BYTES;
#[cfg(not(spx_n_ge_24))]
const SPX_BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE256_OUTPUT_BYTES;

#[cfg(spx_n_ge_24)]
pub(crate) type BlakeStateX = BlakeState512;
#[cfg(not(spx_n_ge_24))]
pub(crate) type BlakeStateX = BlakeState256;

#[inline]
pub(crate) fn blakex_init(s: &mut BlakeStateX) {
    #[cfg(spx_n_ge_24)]
    blake512_init_rs(s);
    #[cfg(not(spx_n_ge_24))]
    blake256_init_rs(s);
}

#[inline]
pub(crate) fn blakex_update(s: &mut BlakeStateX, data: &[u8], datalen: u64) {
    #[cfg(spx_n_ge_24)]
    blake512_update_rs(s, data, datalen);
    #[cfg(not(spx_n_ge_24))]
    blake256_update_rs(s, data, datalen);
}

#[inline]
pub(crate) fn blakex_final(s: &mut BlakeStateX, digest: &mut [u8]) {
    #[cfg(spx_n_ge_24)]
    blake512_final_rs(s, digest);
    #[cfg(not(spx_n_ge_24))]
    blake256_final_rs(s, digest);
}

#[inline]
fn blakex_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    #[cfg(spx_n_ge_24)]
    blake512_mgf1_rs(out, outlen, inp, inlen);
    #[cfg(not(spx_n_ge_24))]
    blake256_mgf1_rs(out, outlen, inp, inlen);
}

pub fn initialize_hash_function(_ctx: &mut SpxCtx) {}

/// Computes `PRF(key, addr)`, given a secret key of `SPX_N` bytes and an
/// address.
///
/// Note that the C code always uses BLAKE-256 here and only hashes
/// `SPX_N + SPX_ADDR_BYTES` of the `2*SPX_N + SPX_ADDR_BYTES` byte buffer.
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &Addr) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr);
    buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);

    blake256_rs(&mut outbuf, &buf, (SPX_N + SPX_ADDR_BYTES) as u64);

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
    _ctx: &SpxCtx,
) {
    let mut s = BlakeStateX::new();

    blakex_init(&mut s);
    blakex_update(&mut s, sk_prf, SPX_N as u64);
    blakex_update(&mut s, optrand, SPX_N as u64);
    blakex_update(&mut s, m, mlen);
    blakex_final(&mut s, r);
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
    let mut seed = [0u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES];

    let mut s = BlakeStateX::new();
    blakex_init(&mut s);

    blakex_update(&mut s, r, SPX_N as u64);
    blakex_update(&mut s, pk, SPX_PK_BYTES as u64);
    blakex_update(&mut s, m, mlen);

    blakex_final(&mut s, &mut seed[2 * SPX_N..]);

    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    blakex_mgf1(
        &mut buf,
        SPX_DGST_BYTES,
        &seed,
        2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES,
    );

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
    let r_s = core::slice::from_raw_parts_mut(r, SPX_BLAKEX_OUTPUT_BYTES);
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
