//! Translation of `lib/sha2/src/hash_sha2.c`.
//!
//! The C file selects between the SHA-256 and the SHA-512 primitives with the
//! preprocessor:
//!
//! ```c
//! #if SPX_N >= 24
//! #define SPX_SHAX_OUTPUT_BYTES SPX_SHA512_OUTPUT_BYTES
//! #define SPX_SHAX_BLOCK_BYTES  SPX_SHA512_BLOCK_BYTES
//! #define shaX_inc_init         sha512_inc_init
//! ...
//! #endif
//! ```
//!
//! Since a single Rust file has to compile for every parameter set, the
//! constants become `const`s evaluated from `crate::params::SPX_SHA512`
//! (which is exactly `SPX_N >= 24`) and the aliased functions become small
//! private wrappers that dispatch on that same constant. `SPX_SHA512` is a
//! compile-time constant, so the dispatch is folded away by the optimiser and
//! the behaviour is bit-identical to the C code.

use core::ptr::{addr_of, copy_nonoverlapping, write_bytes};

use crate::context::SpxCtx;
use crate::params::{SPX_D, SPX_FORS_MSG_BYTES, SPX_N, SPX_PK_BYTES, SPX_SHA512, SPX_TREE_HEIGHT};
use crate::sha2::sha2::{
    sha256, sha256_inc_blocks, sha256_inc_finalize, sha256_inc_init, sha512, sha512_inc_blocks,
    sha512_inc_finalize, sha512_inc_init, SPX_mgf1_256, SPX_mgf1_512, SPX_seed_state,
    SPX_SHA256_ADDR_BYTES, SPX_SHA256_BLOCK_BYTES, SPX_SHA256_OUTPUT_BYTES, SPX_SHA512_BLOCK_BYTES,
    SPX_SHA512_OUTPUT_BYTES,
};
use crate::utils::SPX_bytes_to_ull;

// ---------------------------------------------------------------------------
// `#if SPX_N >= 24` ... `#else` ... `#endif` (the `shaX` family)
// ---------------------------------------------------------------------------

/// `SPX_SHAX_OUTPUT_BYTES`
const SPX_SHAX_OUTPUT_BYTES: usize = if SPX_SHA512 {
    SPX_SHA512_OUTPUT_BYTES
} else {
    SPX_SHA256_OUTPUT_BYTES
};

/// `SPX_SHAX_BLOCK_BYTES`
const SPX_SHAX_BLOCK_BYTES: usize = if SPX_SHA512 {
    SPX_SHA512_BLOCK_BYTES
} else {
    SPX_SHA256_BLOCK_BYTES
};

/// Length of the incremental `shaX` state: `8 + SPX_SHAX_OUTPUT_BYTES`
/// (40 bytes for SHA-256, 72 bytes for SHA-512).
const SHAX_STATE_LEN: usize = 8 + SPX_SHAX_OUTPUT_BYTES;

/// `#define shaX_inc_init sha512_inc_init` / `sha256_inc_init`
#[inline(always)]
unsafe fn shaX_inc_init(state: *mut u8) {
    if SPX_SHA512 {
        sha512_inc_init(state)
    } else {
        sha256_inc_init(state)
    }
}

/// `#define shaX_inc_blocks sha512_inc_blocks` / `sha256_inc_blocks`
#[inline(always)]
unsafe fn shaX_inc_blocks(state: *mut u8, in_: *const u8, inblocks: usize) {
    if SPX_SHA512 {
        sha512_inc_blocks(state, in_, inblocks)
    } else {
        sha256_inc_blocks(state, in_, inblocks)
    }
}

/// `#define shaX_inc_finalize sha512_inc_finalize` / `sha256_inc_finalize`
#[inline(always)]
unsafe fn shaX_inc_finalize(out: *mut u8, state: *mut u8, in_: *const u8, inlen: usize) {
    if SPX_SHA512 {
        sha512_inc_finalize(out, state, in_, inlen)
    } else {
        sha256_inc_finalize(out, state, in_, inlen)
    }
}

/// `#define shaX sha512` / `sha256`
#[inline(always)]
unsafe fn shaX(out: *mut u8, in_: *const u8, inlen: usize) {
    if SPX_SHA512 {
        sha512(out, in_, inlen)
    } else {
        sha256(out, in_, inlen)
    }
}

/// `#define mgf1_X mgf1_512` / `mgf1_256`
#[inline(always)]
unsafe fn mgf1_X(out: *mut u8, outlen: u64, in_: *const u8, inlen: u64) {
    if SPX_SHA512 {
        SPX_mgf1_512(out, outlen, in_, inlen)
    } else {
        SPX_mgf1_256(out, outlen, in_, inlen)
    }
}

/* For SHA, there is no immediate reason to initialize at the start,
so this function is an empty operation. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    SPX_seed_state(ctx);
}

/*
 * Computes PRF(pk_seed, sk_seed, addr).
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let mut sha2_state = [0u8; 40];
    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    let sha2_state_ptr: *mut u8 = sha2_state.as_mut_ptr();
    let buf_ptr: *mut u8 = buf.as_mut_ptr();
    let outbuf_ptr: *mut u8 = outbuf.as_mut_ptr();

    /* Retrieve precomputed state containing pub_seed */
    copy_nonoverlapping(
        addr_of!((*ctx).state_seeded) as *const u8,
        sha2_state_ptr,
        40,
    );

    /* Remainder: ADDR^c ‖ SK.seed */
    copy_nonoverlapping(addr as *const u8, buf_ptr, SPX_SHA256_ADDR_BYTES);
    copy_nonoverlapping(
        addr_of!((*ctx).sk_seed) as *const u8,
        buf_ptr.add(SPX_SHA256_ADDR_BYTES),
        SPX_N,
    );

    sha256_inc_finalize(
        outbuf_ptr,
        sha2_state_ptr,
        buf_ptr as *const u8,
        SPX_SHA256_ADDR_BYTES + SPX_N,
    );

    copy_nonoverlapping(outbuf_ptr as *const u8, out, SPX_N);
}

/* C:
 *   #if SPX_N > SPX_SHAX_BLOCK_BYTES
 *       #error "Currently only supports SPX_N of at most SPX_SHAX_BLOCK_BYTES"
 *   #endif
 */
const _: () = assert!(
    SPX_N <= SPX_SHAX_BLOCK_BYTES,
    "Currently only supports SPX_N of at most SPX_SHAX_BLOCK_BYTES"
);

/**
 * Computes the message-dependent randomness R, using a secret seed as a key
 * for HMAC, and an optional randomization value prefixed to the message.
 * This requires m to have at least SPX_SHAX_BLOCK_BYTES + SPX_N space
 * available in front of the pointer, i.e. before the message to use for the
 * prefix. This is necessary to prevent having to move the message around (and
 * allocate memory for it).
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_gen_message_random(
    R: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: u64,
    ctx: *const SpxCtx,
) {
    let _ = ctx;

    let mut m = m;
    let mut mlen = mlen;

    let mut buf = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    let mut state = [0u8; SHAX_STATE_LEN];
    let mut i: usize;

    let buf_ptr: *mut u8 = buf.as_mut_ptr();
    let state_ptr: *mut u8 = state.as_mut_ptr();

    /* This implements HMAC-SHA */
    i = 0;
    while i < SPX_N {
        *buf_ptr.add(i) = 0x36 ^ *sk_prf.add(i);
        i += 1;
    }
    write_bytes(buf_ptr.add(SPX_N), 0x36, SPX_SHAX_BLOCK_BYTES - SPX_N);

    shaX_inc_init(state_ptr);
    shaX_inc_blocks(state_ptr, buf_ptr as *const u8, 1);

    copy_nonoverlapping(optrand, buf_ptr, SPX_N);

    /* If optrand + message cannot fill up an entire block */
    if SPX_N as u64 + mlen < SPX_SHAX_BLOCK_BYTES as u64 {
        copy_nonoverlapping(m, buf_ptr.add(SPX_N), mlen as usize);
        shaX_inc_finalize(
            buf_ptr.add(SPX_SHAX_BLOCK_BYTES),
            state_ptr,
            buf_ptr as *const u8,
            (mlen + SPX_N as u64) as usize,
        );
    }
    /* Otherwise first fill a block, so that finalize only uses the message */
    else {
        copy_nonoverlapping(m, buf_ptr.add(SPX_N), SPX_SHAX_BLOCK_BYTES - SPX_N);
        shaX_inc_blocks(state_ptr, buf_ptr as *const u8, 1);

        m = m.add(SPX_SHAX_BLOCK_BYTES - SPX_N);
        mlen -= (SPX_SHAX_BLOCK_BYTES - SPX_N) as u64;
        shaX_inc_finalize(
            buf_ptr.add(SPX_SHAX_BLOCK_BYTES),
            state_ptr,
            m,
            mlen as usize,
        );
    }

    i = 0;
    while i < SPX_N {
        *buf_ptr.add(i) = 0x5c ^ *sk_prf.add(i);
        i += 1;
    }
    write_bytes(buf_ptr.add(SPX_N), 0x5c, SPX_SHAX_BLOCK_BYTES - SPX_N);

    shaX(
        buf_ptr,
        buf_ptr as *const u8,
        SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES,
    );
    copy_nonoverlapping(buf_ptr as *const u8, R, SPX_N);
}

/* The `#define`s that hash_sha2.c places inside hash_message(); as
 * compile-time constants they adapt to the selected parameter set. */
const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

/* C:
 *   #if (SPX_SHAX_BLOCK_BYTES & (SPX_SHAX_BLOCK_BYTES-1)) != 0
 *       #error "Assumes that SPX_SHAX_BLOCK_BYTES is a power of 2"
 *   #endif
 */
const _: () = assert!(
    (SPX_SHAX_BLOCK_BYTES & (SPX_SHAX_BLOCK_BYTES - 1)) == 0,
    "Assumes that SPX_SHAX_BLOCK_BYTES is a power of 2"
);

/* Round to nearest multiple of SPX_SHAX_BLOCK_BYTES.
 * C: #define SPX_INBLOCKS (((SPX_N + SPX_PK_BYTES + SPX_SHAX_BLOCK_BYTES - 1) & \
 *                            -SPX_SHAX_BLOCK_BYTES) / SPX_SHAX_BLOCK_BYTES)
 * `-SPX_SHAX_BLOCK_BYTES` is `!(SPX_SHAX_BLOCK_BYTES - 1)` in two's complement
 * (the assert above guarantees the power-of-two property this relies on). */
const SPX_INBLOCKS: usize = ((SPX_N + SPX_PK_BYTES + SPX_SHAX_BLOCK_BYTES - 1)
    & !(SPX_SHAX_BLOCK_BYTES - 1))
    / SPX_SHAX_BLOCK_BYTES;

/* C:
 *   #if SPX_TREE_BITS > 64
 *       #error For given height and depth, 64 bits cannot represent all subtrees
 *   #endif
 */
const _: () = assert!(
    SPX_TREE_BITS <= 64,
    "For given height and depth, 64 bits cannot represent all subtrees"
);

/**
 * Computes the message hash using R, the public key, and the message.
 * Outputs the message digest and the index of the leaf. The index is split in
 * the tree index and the leaf index, for convenient copying to an address.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    R: *const u8,
    pk: *const u8,
    m: *const u8,
    mlen: u64,
    ctx: *const SpxCtx,
) {
    let _ = ctx;

    let mut m = m;
    let mut mlen = mlen;

    let mut seed = [0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES];
    let mut inbuf = [0u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES];

    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; SHAX_STATE_LEN];

    let seed_ptr: *mut u8 = seed.as_mut_ptr();
    let inbuf_ptr: *mut u8 = inbuf.as_mut_ptr();
    let mut bufp: *mut u8 = buf.as_mut_ptr();
    let state_ptr: *mut u8 = state.as_mut_ptr();

    shaX_inc_init(state_ptr);

    // seed: SHA-X(R ‖ PK.seed ‖ PK.root ‖ M)
    copy_nonoverlapping(R, inbuf_ptr, SPX_N);
    copy_nonoverlapping(pk, inbuf_ptr.add(SPX_N), SPX_PK_BYTES);

    /* If R + pk + message cannot fill up an entire block */
    if (SPX_N + SPX_PK_BYTES) as u64 + mlen < (SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES) as u64 {
        copy_nonoverlapping(m, inbuf_ptr.add(SPX_N + SPX_PK_BYTES), mlen as usize);
        shaX_inc_finalize(
            seed_ptr.add(2 * SPX_N),
            state_ptr,
            inbuf_ptr as *const u8,
            ((SPX_N + SPX_PK_BYTES) as u64 + mlen) as usize,
        );
    }
    /* Otherwise first fill a block, so that finalize only uses the message */
    else {
        copy_nonoverlapping(
            m,
            inbuf_ptr.add(SPX_N + SPX_PK_BYTES),
            SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES,
        );
        shaX_inc_blocks(state_ptr, inbuf_ptr as *const u8, SPX_INBLOCKS);

        m = m.add(SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES);
        mlen -= (SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES) as u64;
        shaX_inc_finalize(seed_ptr.add(2 * SPX_N), state_ptr, m, mlen as usize);
    }

    // H_msg: MGF1-SHA-X(R ‖ PK.seed ‖ seed)
    copy_nonoverlapping(R, seed_ptr, SPX_N);
    copy_nonoverlapping(pk, seed_ptr.add(SPX_N), SPX_N);

    /* By doing this in two steps, we prevent hashing the message twice;
    otherwise each iteration in MGF1 would hash the message again. */
    mgf1_X(
        bufp,
        SPX_DGST_BYTES as u64,
        seed_ptr as *const u8,
        (2 * SPX_N + SPX_SHAX_OUTPUT_BYTES) as u64,
    );

    copy_nonoverlapping(bufp as *const u8, digest, SPX_FORS_MSG_BYTES);
    bufp = bufp.add(SPX_FORS_MSG_BYTES);

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = SPX_bytes_to_ull(bufp as *const u8, SPX_TREE_BYTES as u32);
        /* C: *tree &= (~(uint64_t)0) >> (64 - SPX_TREE_BITS);
         * `wrapping_shr` keeps the shift well-defined when SPX_TREE_BITS == 0
         * (i.e. SPX_D == 1), in which case this branch is never taken. */
        *tree &= (!0u64).wrapping_shr((64 - SPX_TREE_BITS) as u32);
    }
    bufp = bufp.add(SPX_TREE_BYTES);

    *leaf_idx = SPX_bytes_to_ull(bufp as *const u8, SPX_LEAF_BYTES as u32) as u32;
    /* C: *leaf_idx &= (~(uint32_t)0) >> (32 - SPX_LEAF_BITS); */
    *leaf_idx &= (!0u32).wrapping_shr((32 - SPX_LEAF_BITS) as u32);
}
