//! Translation of `c_src/lib/sha2/src/hash_sha2.c`.

use crate::address::addr_bytes;
use crate::backend::sha2;
use crate::backend::sha2::{
    SPX_SHA256_ADDR_BYTES, SPX_SHA256_BLOCK_BYTES, SPX_SHA256_OUTPUT_BYTES, SPX_SHA512_BLOCK_BYTES,
    SPX_SHA512_OUTPUT_BYTES,
};
use crate::context::SpxCtx;
use crate::params::{
    SPX_D, SPX_FORS_MSG_BYTES, SPX_N, SPX_PK_BYTES, SPX_TREE_HEIGHT,
};
use crate::utils::bytes_to_ull;

// ---------------------------------------------------------------------------
// `#if SPX_N >= 24` -- select between SHA-512 and SHA-256 for `shaX`.
// ---------------------------------------------------------------------------

/// `SPX_N >= 24` in the C preprocessor.
const USE_512: bool = SPX_N >= 24;

pub const SPX_SHAX_OUTPUT_BYTES: usize = if USE_512 {
    SPX_SHA512_OUTPUT_BYTES
} else {
    SPX_SHA256_OUTPUT_BYTES
};
pub const SPX_SHAX_BLOCK_BYTES: usize = if USE_512 {
    SPX_SHA512_BLOCK_BYTES
} else {
    SPX_SHA256_BLOCK_BYTES
};

/// `uint8_t state[8 + SPX_SHAX_OUTPUT_BYTES]`
const SPX_SHAX_STATE_BYTES: usize = 8 + SPX_SHAX_OUTPUT_BYTES;

#[inline]
fn shax_inc_init(state: &mut [u8]) {
    if USE_512 {
        sha2::sha512_inc_init(state);
    } else {
        sha2::sha256_inc_init(state);
    }
}

#[inline]
fn shax_inc_blocks(state: &mut [u8], input: &[u8], inblocks: usize) {
    if USE_512 {
        sha2::sha512_inc_blocks(state, input, inblocks);
    } else {
        sha2::sha256_inc_blocks(state, input, inblocks);
    }
}

#[inline]
fn shax_inc_finalize(out: &mut [u8], state: &mut [u8], input: &[u8]) {
    if USE_512 {
        sha2::sha512_inc_finalize(out, state, input);
    } else {
        sha2::sha256_inc_finalize(out, state, input);
    }
}

#[inline]
fn shax(out: &mut [u8], input: &[u8]) {
    if USE_512 {
        sha2::sha512(out, input);
    } else {
        sha2::sha256(out, input);
    }
}

#[inline]
fn mgf1_x(out: &mut [u8], input: &[u8]) {
    if USE_512 {
        sha2::mgf1_512(out, input);
    } else {
        sha2::mgf1_256(out, input);
    }
}

// ---------------------------------------------------------------------------
// hash.h API
// ---------------------------------------------------------------------------

/// For SHA, there is no immediate reason to initialize at the start, so this
/// function is an empty operation.
pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    sha2::seed_state(ctx);
}

/// Computes PRF(pk_seed, sk_seed, addr).
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    /* Retrieve precomputed state containing pub_seed */
    sha2_state.copy_from_slice(&ctx.state_seeded[..40]);

    /* Remainder: ADDR^c ‖ SK.seed */
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed[..SPX_N]);

    sha2::sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf);

    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

/// Computes the message-dependent randomness R, using a secret seed as a key
/// for HMAC, and an optional randomization value prefixed to the message.
pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], ctx: &SpxCtx) {
    let _ = ctx;

    let mut buf = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    let mut state = [0u8; SPX_SHAX_STATE_BYTES];

    let mlen = m.len() as u64;

    /* This implements HMAC-SHA */
    for i in 0..SPX_N {
        buf[i] = 0x36 ^ sk_prf[i];
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x36;
    }

    shax_inc_init(&mut state);
    shax_inc_blocks(&mut state, &buf, 1);

    buf[..SPX_N].copy_from_slice(&optrand[..SPX_N]);

    /* If optrand + message cannot fill up an entire block */
    if (SPX_N as u64) + mlen < SPX_SHAX_BLOCK_BYTES as u64 {
        let ml = mlen as usize;
        buf[SPX_N..SPX_N + ml].copy_from_slice(&m[..ml]);
        let (inpart, outpart) = buf.split_at_mut(SPX_SHAX_BLOCK_BYTES);
        shax_inc_finalize(outpart, &mut state, &inpart[..ml + SPX_N]);
    }
    /* Otherwise first fill a block, so that finalize only uses the message */
    else {
        buf[SPX_N..SPX_SHAX_BLOCK_BYTES]
            .copy_from_slice(&m[..SPX_SHAX_BLOCK_BYTES - SPX_N]);
        shax_inc_blocks(&mut state, &buf, 1);

        let m2 = &m[SPX_SHAX_BLOCK_BYTES - SPX_N..];
        let (_, outpart) = buf.split_at_mut(SPX_SHAX_BLOCK_BYTES);
        shax_inc_finalize(outpart, &mut state, m2);
    }

    for i in 0..SPX_N {
        buf[i] = 0x5c ^ sk_prf[i];
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x5c;
    }

    /* `shaX(buf, buf, ...)` in C: the whole input is consumed before the
       output is written, so this in-place call is well defined. */
    let mut tmp = [0u8; SPX_SHAX_OUTPUT_BYTES];
    shax(&mut tmp, &buf);
    buf[..SPX_SHAX_OUTPUT_BYTES].copy_from_slice(&tmp);

    r[..SPX_N].copy_from_slice(&buf[..SPX_N]);
}

// ---------------------------------------------------------------------------
// hash_message
// ---------------------------------------------------------------------------

const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

/* Round to nearest multiple of SPX_SHAX_BLOCK_BYTES; assumes that
   SPX_SHAX_BLOCK_BYTES is a power of 2 (the `& -SPX_SHAX_BLOCK_BYTES` in C). */
const SPX_INBLOCKS: usize = ((SPX_N + SPX_PK_BYTES + SPX_SHAX_BLOCK_BYTES - 1)
    & !(SPX_SHAX_BLOCK_BYTES - 1))
    / SPX_SHAX_BLOCK_BYTES;

/// `(~(uint64_t)0) >> (64 - SPX_TREE_BITS)`; only used when `SPX_D != 1`, in
/// which case `SPX_TREE_BITS` is in `1..=64`.
const TREE_MASK: u64 = if SPX_TREE_BITS == 0 || SPX_TREE_BITS >= 64 {
    u64::MAX
} else {
    u64::MAX >> (64 - SPX_TREE_BITS)
};

/// `(~(uint32_t)0) >> (32 - SPX_LEAF_BITS)`
const LEAF_MASK: u32 = if SPX_LEAF_BITS == 0 || SPX_LEAF_BITS >= 32 {
    u32::MAX
} else {
    u32::MAX >> (32 - SPX_LEAF_BITS)
};

/// Computes the message hash using R, the public key, and the message.
/// Outputs the message digest and the index of the leaf. The index is split in
/// the tree index and the leaf index, for convenient copying to an address.
pub fn hash_message(
    digest: &mut [u8],
    tree: &mut u64,
    leaf_idx: &mut u32,
    r: &[u8],
    pk: &[u8],
    m: &[u8],
    ctx: &SpxCtx,
) {
    let _ = ctx;

    let mut seed = [0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES];
    let mut inbuf = [0u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; SPX_SHAX_STATE_BYTES];

    let mlen = m.len() as u64;

    shax_inc_init(&mut state);

    // seed: SHA-X(R ‖ PK.seed ‖ PK.root ‖ M)
    inbuf[..SPX_N].copy_from_slice(&r[..SPX_N]);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    /* If R + pk + message cannot fill up an entire block */
    if (SPX_N + SPX_PK_BYTES) as u64 + mlen < (SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES) as u64 {
        let ml = mlen as usize;
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + ml].copy_from_slice(&m[..ml]);
        shax_inc_finalize(
            &mut seed[2 * SPX_N..],
            &mut state,
            &inbuf[..SPX_N + SPX_PK_BYTES + ml],
        );
    }
    /* Otherwise first fill a block, so that finalize only uses the message */
    else {
        let take = SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..].copy_from_slice(&m[..take]);
        shax_inc_blocks(&mut state, &inbuf, SPX_INBLOCKS);

        let m2 = &m[take..];
        shax_inc_finalize(&mut seed[2 * SPX_N..], &mut state, m2);
    }

    // H_msg: MGF1-SHA-X(R ‖ PK.seed ‖ seed)
    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    /* By doing this in two steps, we prevent hashing the message twice;
       otherwise each iteration in MGF1 would hash the message again. */
    mgf1_x(&mut buf, &seed);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES as u32);
        *tree &= TREE_MASK;
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES as u32) as u32;
    *leaf_idx &= LEAF_MASK;
}

// ---------------------------------------------------------------------------
// C ABI wrappers -- app/include/hash.h
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    unsafe {
        initialize_hash_function(&mut *ctx);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    unsafe {
        let addr_ref = &*(addr as *const [u32; 8]);
        prf_addr(
            core::slice::from_raw_parts_mut(out, SPX_N),
            &*ctx,
            addr_ref,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_gen_message_random(
    r: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: u64,
    ctx: *const SpxCtx,
) {
    unsafe {
        gen_message_random(
            core::slice::from_raw_parts_mut(r, SPX_N),
            core::slice::from_raw_parts(sk_prf, SPX_N),
            core::slice::from_raw_parts(optrand, SPX_N),
            core::slice::from_raw_parts(m, mlen as usize),
            &*ctx,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    r: *const u8,
    pk: *const u8,
    m: *const u8,
    mlen: u64,
    ctx: *const SpxCtx,
) {
    unsafe {
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
}

// ---------------------------------------------------------------------------
// C ABI wrappers -- lib/sha2/include/sha2.h
//
// `sha256*`/`sha512*` are NOT namespaced in the C header, so they keep their
// plain names; `mgf1_256`, `mgf1_512` and `seed_state` are namespaced.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn sha256_inc_init(state: *mut u8) {
    unsafe {
        sha2::sha256_inc_init(core::slice::from_raw_parts_mut(state, 40));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha256_inc_blocks(state: *mut u8, input: *const u8, inblocks: usize) {
    unsafe {
        sha2::sha256_inc_blocks(
            core::slice::from_raw_parts_mut(state, 40),
            core::slice::from_raw_parts(input, 64 * inblocks),
            inblocks,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha256_inc_finalize(
    out: *mut u8,
    state: *mut u8,
    input: *const u8,
    inlen: usize,
) {
    unsafe {
        sha2::sha256_inc_finalize(
            core::slice::from_raw_parts_mut(out, 32),
            core::slice::from_raw_parts_mut(state, 40),
            core::slice::from_raw_parts(input, inlen),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha256(out: *mut u8, input: *const u8, inlen: usize) {
    unsafe {
        sha2::sha256(
            core::slice::from_raw_parts_mut(out, 32),
            core::slice::from_raw_parts(input, inlen),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha512_inc_init(state: *mut u8) {
    unsafe {
        sha2::sha512_inc_init(core::slice::from_raw_parts_mut(state, 72));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha512_inc_blocks(state: *mut u8, input: *const u8, inblocks: usize) {
    unsafe {
        sha2::sha512_inc_blocks(
            core::slice::from_raw_parts_mut(state, 72),
            core::slice::from_raw_parts(input, 128 * inblocks),
            inblocks,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha512_inc_finalize(
    out: *mut u8,
    state: *mut u8,
    input: *const u8,
    inlen: usize,
) {
    unsafe {
        sha2::sha512_inc_finalize(
            core::slice::from_raw_parts_mut(out, 64),
            core::slice::from_raw_parts_mut(state, 72),
            core::slice::from_raw_parts(input, inlen),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha512(out: *mut u8, input: *const u8, inlen: usize) {
    unsafe {
        sha2::sha512(
            core::slice::from_raw_parts_mut(out, 64),
            core::slice::from_raw_parts(input, inlen),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_mgf1_256(out: *mut u8, outlen: u64, input: *const u8, inlen: u64) {
    unsafe {
        sha2::mgf1_256(
            core::slice::from_raw_parts_mut(out, outlen as usize),
            core::slice::from_raw_parts(input, inlen as usize),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_mgf1_512(out: *mut u8, outlen: u64, input: *const u8, inlen: u64) {
    unsafe {
        sha2::mgf1_512(
            core::slice::from_raw_parts_mut(out, outlen as usize),
            core::slice::from_raw_parts(input, inlen as usize),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_seed_state(ctx: *mut SpxCtx) {
    unsafe {
        sha2::seed_state(&mut *ctx);
    }
}
