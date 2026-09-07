//! Translation of `lib/sha2/src/hash_sha2.c`.

use super::sha2::*;
use crate::address::{addr_mut, addr_ref, Addr};
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::split_digest;
use core::ffi::c_ulonglong;

// The `#if SPX_N >= 24` block at the top of hash_sha2.c that selects between
// SHA-256 and SHA-512 ("shaX").
#[cfg(spx_n_ge_24)]
pub(crate) const SPX_SHAX_OUTPUT_BYTES: usize = SPX_SHA512_OUTPUT_BYTES;
#[cfg(spx_n_ge_24)]
pub(crate) const SPX_SHAX_BLOCK_BYTES: usize = SPX_SHA512_BLOCK_BYTES;
#[cfg(not(spx_n_ge_24))]
pub(crate) const SPX_SHAX_OUTPUT_BYTES: usize = SPX_SHA256_OUTPUT_BYTES;
#[cfg(not(spx_n_ge_24))]
pub(crate) const SPX_SHAX_BLOCK_BYTES: usize = SPX_SHA256_BLOCK_BYTES;

/// `uint8_t state[8 + SPX_SHAX_OUTPUT_BYTES]`
pub(crate) const SHAX_STATE_LEN: usize = 8 + SPX_SHAX_OUTPUT_BYTES;

#[inline]
pub(crate) fn shax_inc_init(state: &mut [u8; SHAX_STATE_LEN]) {
    #[cfg(spx_n_ge_24)]
    sha512_inc_init_rs(state);
    #[cfg(not(spx_n_ge_24))]
    sha256_inc_init_rs(state);
}

#[inline]
pub(crate) fn shax_inc_blocks(state: &mut [u8; SHAX_STATE_LEN], inp: &[u8], inblocks: usize) {
    #[cfg(spx_n_ge_24)]
    sha512_inc_blocks_rs(state, inp, inblocks);
    #[cfg(not(spx_n_ge_24))]
    sha256_inc_blocks_rs(state, inp, inblocks);
}

#[inline]
pub(crate) fn shax_inc_finalize(out: &mut [u8], state: &mut [u8; SHAX_STATE_LEN], inp: &[u8]) {
    #[cfg(spx_n_ge_24)]
    sha512_inc_finalize_rs(out, state, inp);
    #[cfg(not(spx_n_ge_24))]
    sha256_inc_finalize_rs(out, state, inp);
}

#[inline]
fn shax(out: &mut [u8], inp: &[u8]) {
    #[cfg(spx_n_ge_24)]
    sha512_rs(out, inp);
    #[cfg(not(spx_n_ge_24))]
    sha256_rs(out, inp);
}

#[inline]
fn mgf1_x(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    #[cfg(spx_n_ge_24)]
    mgf1_512_rs(out, outlen, inp, inlen);
    #[cfg(not(spx_n_ge_24))]
    mgf1_256_rs(out, outlen, inp, inlen);
}

/// For SHA, there is no immediate reason to initialize at the start, so this
/// function only seeds the precomputed states.
pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    seed_state_rs(ctx);
}

/// Computes `PRF(pk_seed, sk_seed, addr)`.
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &Addr) {
    let mut sha2_state = [0u8; 40];
    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    /* Retrieve precomputed state containing pub_seed */
    sha2_state.copy_from_slice(&ctx.state_seeded);

    /* Remainder: ADDR^c || SK.seed */
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);

    sha256_inc_finalize_rs(&mut outbuf, &mut sha2_state, &buf);

    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

/// Computes the message-dependent randomness R, using a secret seed as a key
/// for HMAC, and an optional randomization value prefixed to the message.
pub fn gen_message_random(
    r: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    mlen: u64,
    _ctx: &SpxCtx,
) {
    let mut buf = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    let mut state = [0u8; SHAX_STATE_LEN];

    /* This implements HMAC-SHA */
    for i in 0..SPX_N {
        buf[i] = 0x36 ^ sk_prf[i];
    }
    for b in buf[SPX_N..SPX_SHAX_BLOCK_BYTES].iter_mut() {
        *b = 0x36;
    }

    shax_inc_init(&mut state);
    {
        let src = buf;
        shax_inc_blocks(&mut state, &src, 1);
    }

    buf[..SPX_N].copy_from_slice(&optrand[..SPX_N]);

    let mlen = mlen as usize;
    /* If optrand + message cannot fill up an entire block */
    if SPX_N + mlen < SPX_SHAX_BLOCK_BYTES {
        buf[SPX_N..SPX_N + mlen].copy_from_slice(&m[..mlen]);
        let (head, tail) = buf.split_at_mut(SPX_SHAX_BLOCK_BYTES);
        shax_inc_finalize(tail, &mut state, &head[..mlen + SPX_N]);
    }
    /* Otherwise first fill a block, so that finalize only uses the message */
    else {
        let k = SPX_SHAX_BLOCK_BYTES - SPX_N;
        buf[SPX_N..SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..k]);
        {
            let src = buf;
            shax_inc_blocks(&mut state, &src, 1);
        }

        let (_head, tail) = buf.split_at_mut(SPX_SHAX_BLOCK_BYTES);
        shax_inc_finalize(tail, &mut state, &m[k..mlen]);
    }

    for i in 0..SPX_N {
        buf[i] = 0x5c ^ sk_prf[i];
    }
    for b in buf[SPX_N..SPX_SHAX_BLOCK_BYTES].iter_mut() {
        *b = 0x5c;
    }

    /* shaX(buf, buf, SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES) */
    let src = buf;
    shax(&mut buf, &src);
    r[..SPX_N].copy_from_slice(&buf[..SPX_N]);
}

/// Round `SPX_N + SPX_PK_BYTES` up to a multiple of `SPX_SHAX_BLOCK_BYTES`.
const SPX_INBLOCKS: usize = ((SPX_N + SPX_PK_BYTES + SPX_SHAX_BLOCK_BYTES - 1)
    & !(SPX_SHAX_BLOCK_BYTES - 1))
    / SPX_SHAX_BLOCK_BYTES;

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
    let mut seed = [0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES];
    let mut inbuf = [0u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; SHAX_STATE_LEN];

    shax_inc_init(&mut state);

    // seed: SHA-X(R || PK.seed || PK.root || M)
    inbuf[..SPX_N].copy_from_slice(&r[..SPX_N]);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    let total = SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES;
    let mlen = mlen as usize;

    /* If R + pk + message cannot fill up an entire block */
    if SPX_N + SPX_PK_BYTES + mlen < total {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen].copy_from_slice(&m[..mlen]);
        shax_inc_finalize(
            &mut seed[2 * SPX_N..],
            &mut state,
            &inbuf[..SPX_N + SPX_PK_BYTES + mlen],
        );
    }
    /* Otherwise first fill a block, so that finalize only uses the message */
    else {
        let k = total - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..].copy_from_slice(&m[..k]);
        shax_inc_blocks(&mut state, &inbuf, SPX_INBLOCKS);
        shax_inc_finalize(&mut seed[2 * SPX_N..], &mut state, &m[k..mlen]);
    }

    // H_msg: MGF1-SHA-X(R || PK.seed || seed)
    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    /* By doing this in two steps, we prevent hashing the message twice;
       otherwise each iteration in MGF1 would hash the message again. */
    mgf1_x(
        &mut buf,
        SPX_DGST_BYTES,
        &seed,
        2 * SPX_N + SPX_SHAX_OUTPUT_BYTES,
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
