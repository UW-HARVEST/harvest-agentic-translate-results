//! Translation of `lib/sha2/src/hash_sha2.c`.

use core::ffi::c_ulonglong;

use crate::address::addr_bytes;
use crate::context::SpxCtx;
use crate::params::*;
use crate::sha2::SPX_SHA512;
use crate::sha2::sha2::*;
use crate::utils::bytes_to_ull;

/* `shaX_*` in the C file: SHA-512 for SPX_N >= 24, SHA-256 otherwise. */

/// `SPX_SHAX_OUTPUT_BYTES`
pub const SPX_SHAX_OUTPUT_BYTES: usize = if SPX_SHA512 {
    SPX_SHA512_OUTPUT_BYTES
} else {
    SPX_SHA256_OUTPUT_BYTES
};
/// `SPX_SHAX_BLOCK_BYTES`
pub const SPX_SHAX_BLOCK_BYTES: usize = if SPX_SHA512 {
    SPX_SHA512_BLOCK_BYTES
} else {
    SPX_SHA256_BLOCK_BYTES
};
/// `shaX_state_len`, i.e. `8 + SPX_SHAX_OUTPUT_BYTES`
const SHAX_STATE_LEN: usize = 8 + SPX_SHAX_OUTPUT_BYTES;

#[inline]
fn shax_inc_init(state: &mut [u8]) {
    if SPX_SHA512 {
        sha512_inc_init(state);
    } else {
        sha256_inc_init(state);
    }
}

#[inline]
fn shax_inc_blocks(state: &mut [u8], inp: &[u8], inblocks: usize) {
    if SPX_SHA512 {
        sha512_inc_blocks(state, inp, inblocks);
    } else {
        sha256_inc_blocks(state, inp, inblocks);
    }
}

#[inline]
fn shax_inc_finalize(out: &mut [u8], state: &mut [u8], inp: &[u8], inlen: usize) {
    if SPX_SHA512 {
        sha512_inc_finalize(out, state, inp, inlen);
    } else {
        sha256_inc_finalize(out, state, inp, inlen);
    }
}

#[inline]
fn shax(out: &mut [u8], inp: &[u8], inlen: usize) {
    if SPX_SHA512 {
        sha512(out, inp, inlen);
    } else {
        sha256(out, inp, inlen);
    }
}

#[inline]
fn mgf1_x(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    if SPX_SHA512 {
        mgf1_512(out, outlen, inp, inlen);
    } else {
        mgf1_256(out, outlen, inp, inlen);
    }
}

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    seed_state(ctx);
}

/// Computes `PRF(pk_seed, sk_seed, addr)`.
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    /* Retrieve precomputed state containing pub_seed */
    sha2_state.copy_from_slice(&ctx.backend.state_seeded);

    /* Remainder: ADDR^c || SK.seed */
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);

    sha256_inc_finalize(
        &mut outbuf,
        &mut sha2_state,
        &buf,
        SPX_SHA256_ADDR_BYTES + SPX_N,
    );

    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

/// Computes the message-dependent randomness `R`, using a secret seed as a key
/// for HMAC, and an optional randomization value prefixed to the message.
pub fn gen_message_random(
    r_out: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    _ctx: &SpxCtx,
) {
    let mut buf = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    let mut state = [0u8; SHAX_STATE_LEN];
    let mut mlen = m.len();
    let mut moff = 0usize;

    const _: () = assert!(
        SPX_N <= SPX_SHAX_BLOCK_BYTES,
        "Currently only supports SPX_N of at most SPX_SHAX_BLOCK_BYTES"
    );

    /* This implements HMAC-SHA */
    for i in 0..SPX_N {
        buf[i] = 0x36 ^ sk_prf[i];
    }
    buf[SPX_N..SPX_SHAX_BLOCK_BYTES].fill(0x36);

    shax_inc_init(&mut state);
    let block: [u8; SPX_SHAX_BLOCK_BYTES] = buf[..SPX_SHAX_BLOCK_BYTES].try_into().unwrap();
    shax_inc_blocks(&mut state, &block, 1);

    buf[..SPX_N].copy_from_slice(&optrand[..SPX_N]);

    /* If optrand + message cannot fill up an entire block */
    if SPX_N + mlen < SPX_SHAX_BLOCK_BYTES {
        buf[SPX_N..SPX_N + mlen].copy_from_slice(&m[..mlen]);
        let input: [u8; SPX_SHAX_BLOCK_BYTES] = buf[..SPX_SHAX_BLOCK_BYTES].try_into().unwrap();
        shax_inc_finalize(
            &mut buf[SPX_SHAX_BLOCK_BYTES..],
            &mut state,
            &input,
            mlen + SPX_N,
        );
    }
    /* Otherwise first fill a block, so that finalize only uses the message */
    else {
        buf[SPX_N..SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..SPX_SHAX_BLOCK_BYTES - SPX_N]);
        let block: [u8; SPX_SHAX_BLOCK_BYTES] = buf[..SPX_SHAX_BLOCK_BYTES].try_into().unwrap();
        shax_inc_blocks(&mut state, &block, 1);

        moff += SPX_SHAX_BLOCK_BYTES - SPX_N;
        mlen -= SPX_SHAX_BLOCK_BYTES - SPX_N;
        shax_inc_finalize(
            &mut buf[SPX_SHAX_BLOCK_BYTES..],
            &mut state,
            &m[moff..moff + mlen],
            mlen,
        );
    }

    for i in 0..SPX_N {
        buf[i] = 0x5c ^ sk_prf[i];
    }
    buf[SPX_N..SPX_SHAX_BLOCK_BYTES].fill(0x5c);

    let input = buf;
    shax(
        &mut buf,
        &input,
        SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES,
    );
    r_out[..SPX_N].copy_from_slice(&buf[..SPX_N]);
}

/// `SPX_INBLOCKS`: `SPX_N + SPX_PK_BYTES` rounded up to a multiple of
/// `SPX_SHAX_BLOCK_BYTES`.
const SPX_INBLOCKS: usize = (SPX_N + SPX_PK_BYTES + SPX_SHAX_BLOCK_BYTES - 1) / SPX_SHAX_BLOCK_BYTES;

const _: () = assert!(
    SPX_SHAX_BLOCK_BYTES & (SPX_SHAX_BLOCK_BYTES - 1) == 0,
    "Assumes that SPX_SHAX_BLOCK_BYTES is a power of 2"
);

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
    let mut seed = [0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES];
    let mut inbuf = [0u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; SHAX_STATE_LEN];
    let mut mlen = m.len();
    let mut moff = 0usize;

    shax_inc_init(&mut state);

    // seed: SHA-X(R || PK.seed || PK.root || M)
    inbuf[..SPX_N].copy_from_slice(&r_in[..SPX_N]);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    /* If R + pk + message cannot fill up an entire block */
    if SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen].copy_from_slice(&m[..mlen]);
        shax_inc_finalize(
            &mut seed[2 * SPX_N..],
            &mut state,
            &inbuf,
            SPX_N + SPX_PK_BYTES + mlen,
        );
    }
    /* Otherwise first fill a block, so that finalize only uses the message */
    else {
        let fill = SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..].copy_from_slice(&m[..fill]);
        shax_inc_blocks(&mut state, &inbuf, SPX_INBLOCKS);

        moff += fill;
        mlen -= fill;
        shax_inc_finalize(
            &mut seed[2 * SPX_N..],
            &mut state,
            &m[moff..moff + mlen],
            mlen,
        );
    }

    // H_msg: MGF1-SHA-X(R || PK.seed || seed)
    seed[..SPX_N].copy_from_slice(&r_in[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    /* By doing this in two steps, we prevent hashing the message twice;
       otherwise each iteration in MGF1 would hash the message again. */
    mgf1_x(
        &mut buf,
        SPX_DGST_BYTES,
        &seed,
        2 * SPX_N + SPX_SHAX_OUTPUT_BYTES,
    );

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
