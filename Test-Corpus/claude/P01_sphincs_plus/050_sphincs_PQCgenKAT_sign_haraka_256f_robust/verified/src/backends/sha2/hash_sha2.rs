//! Translation of `lib/sha2/src/hash_sha2.c`.
//!
//! The `#if SPX_N >= 24` macro selection of the SHA-512 vs SHA-256 primitive
//! (`shaX`, `SPX_SHAX_BLOCK_BYTES`, `SPX_SHAX_OUTPUT_BYTES`, `mgf1_X`) is
//! reproduced with `const`/runtime `if` on `crate::params::SPX_SHA512`.

use super::sha2::{
    sha256, sha256_inc_blocks, sha256_inc_finalize, sha256_inc_init, sha512, sha512_inc_blocks,
    sha512_inc_finalize, sha512_inc_init, SPX_mgf1_256, SPX_mgf1_512, SPX_seed_state,
    SPX_SHA256_BLOCK_BYTES, SPX_SHA256_OUTPUT_BYTES, SPX_SHA512_BLOCK_BYTES,
    SPX_SHA512_OUTPUT_BYTES,
};
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::SPX_bytes_to_ull;
use core::ffi::c_ulong;

// #if SPX_N >= 24 ... #else ... #endif
const SPX_SHAX_OUTPUT_BYTES: usize = if SPX_SHA512 {
    SPX_SHA512_OUTPUT_BYTES
} else {
    SPX_SHA256_OUTPUT_BYTES
};
const SPX_SHAX_BLOCK_BYTES: usize = if SPX_SHA512 {
    SPX_SHA512_BLOCK_BYTES
} else {
    SPX_SHA256_BLOCK_BYTES
};

/// Size of the incremental state: `8 + SPX_SHAX_OUTPUT_BYTES`.
const SPX_SHAX_STATE_BYTES: usize = 8 + SPX_SHAX_OUTPUT_BYTES;

#[inline]
unsafe fn shaX_inc_init(state: *mut u8) {
    if SPX_SHA512 {
        sha512_inc_init(state);
    } else {
        sha256_inc_init(state);
    }
}

#[inline]
unsafe fn shaX_inc_blocks(state: *mut u8, inp: *const u8, inblocks: usize) {
    if SPX_SHA512 {
        sha512_inc_blocks(state, inp, inblocks);
    } else {
        sha256_inc_blocks(state, inp, inblocks);
    }
}

#[inline]
unsafe fn shaX_inc_finalize(out: *mut u8, state: *mut u8, inp: *const u8, inlen: usize) {
    if SPX_SHA512 {
        sha512_inc_finalize(out, state, inp, inlen);
    } else {
        sha256_inc_finalize(out, state, inp, inlen);
    }
}

#[inline]
unsafe fn shaX(out: *mut u8, inp: *const u8, inlen: usize) {
    if SPX_SHA512 {
        sha512(out, inp, inlen);
    } else {
        sha256(out, inp, inlen);
    }
}

#[inline]
unsafe fn mgf1_X(out: *mut u8, outlen: c_ulong, inp: *const u8, inlen: c_ulong) {
    if SPX_SHA512 {
        SPX_mgf1_512(out, outlen, inp, inlen);
    } else {
        SPX_mgf1_256(out, outlen, inp, inlen);
    }
}

/// For SHA, there is no immediate reason to initialize at the start, so this
/// function only seeds the precomputed states.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    SPX_seed_state(ctx);
}

/// Computes PRF(pk_seed, sk_seed, addr).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let mut sha2_state = [0u8; 40];
    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    /* Retrieve precomputed state containing pub_seed */
    core::ptr::copy_nonoverlapping((*ctx).state_seeded.as_ptr(), sha2_state.as_mut_ptr(), 40);

    /* Remainder: ADDR^c ‖ SK.seed */
    core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr(), SPX_SHA256_ADDR_BYTES);
    core::ptr::copy_nonoverlapping(
        (*ctx).sk_seed.as_ptr(),
        buf.as_mut_ptr().add(SPX_SHA256_ADDR_BYTES),
        SPX_N,
    );

    sha256_inc_finalize(
        outbuf.as_mut_ptr(),
        sha2_state.as_mut_ptr(),
        buf.as_ptr(),
        SPX_SHA256_ADDR_BYTES + SPX_N,
    );

    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}

/// Computes the message-dependent randomness R, using a secret seed as a key
/// for HMAC, and an optional randomization value prefixed to the message.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_gen_message_random(
    r: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    mut m: *const u8,
    mut mlen: u64,
    _ctx: *const SpxCtx,
) {
    let mut buf = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    let mut state = [0u8; SPX_SHAX_STATE_BYTES];

    /* This implements HMAC-SHA */
    for i in 0..SPX_N {
        buf[i] = 0x36 ^ *sk_prf.add(i);
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x36;
    }

    shaX_inc_init(state.as_mut_ptr());
    shaX_inc_blocks(state.as_mut_ptr(), buf.as_ptr(), 1);

    core::ptr::copy_nonoverlapping(optrand, buf.as_mut_ptr(), SPX_N);

    /* If optrand + message cannot fill up an entire block */
    if SPX_N as u64 + mlen < SPX_SHAX_BLOCK_BYTES as u64 {
        core::ptr::copy_nonoverlapping(m, buf.as_mut_ptr().add(SPX_N), mlen as usize);
        shaX_inc_finalize(
            buf.as_mut_ptr().add(SPX_SHAX_BLOCK_BYTES),
            state.as_mut_ptr(),
            buf.as_ptr(),
            mlen as usize + SPX_N,
        );
    }
    /* Otherwise first fill a block, so that finalize only uses the message */
    else {
        core::ptr::copy_nonoverlapping(
            m,
            buf.as_mut_ptr().add(SPX_N),
            SPX_SHAX_BLOCK_BYTES - SPX_N,
        );
        shaX_inc_blocks(state.as_mut_ptr(), buf.as_ptr(), 1);

        m = m.add(SPX_SHAX_BLOCK_BYTES - SPX_N);
        mlen -= (SPX_SHAX_BLOCK_BYTES - SPX_N) as u64;
        shaX_inc_finalize(
            buf.as_mut_ptr().add(SPX_SHAX_BLOCK_BYTES),
            state.as_mut_ptr(),
            m,
            mlen as usize,
        );
    }

    for i in 0..SPX_N {
        buf[i] = 0x5c ^ *sk_prf.add(i);
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x5c;
    }

    shaX(
        buf.as_mut_ptr(),
        buf.as_ptr(),
        SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES,
    );
    core::ptr::copy_nonoverlapping(buf.as_ptr(), r, SPX_N);
}

/// Round to nearest multiple of `SPX_SHAX_BLOCK_BYTES` (which is a power of 2).
const SPX_INBLOCKS: usize =
    ((SPX_N + SPX_PK_BYTES + SPX_SHAX_BLOCK_BYTES - 1) & (!(SPX_SHAX_BLOCK_BYTES - 1)))
        / SPX_SHAX_BLOCK_BYTES;

/// Computes the message hash using R, the public key, and the message.
/// Outputs the message digest and the index of the leaf. The index is split in
/// the tree index and the leaf index, for convenient copying to an address.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    r: *const u8,
    pk: *const u8,
    mut m: *const u8,
    mut mlen: u64,
    _ctx: *const SpxCtx,
) {
    let mut seed = [0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES];
    let mut inbuf = [0u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; SPX_SHAX_STATE_BYTES];

    shaX_inc_init(state.as_mut_ptr());

    // seed: SHA-X(R ‖ PK.seed ‖ PK.root ‖ M)
    core::ptr::copy_nonoverlapping(r, inbuf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(pk, inbuf.as_mut_ptr().add(SPX_N), SPX_PK_BYTES);

    /* If R + pk + message cannot fill up an entire block */
    if (SPX_N + SPX_PK_BYTES) as u64 + mlen < (SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES) as u64 {
        core::ptr::copy_nonoverlapping(
            m,
            inbuf.as_mut_ptr().add(SPX_N + SPX_PK_BYTES),
            mlen as usize,
        );
        shaX_inc_finalize(
            seed.as_mut_ptr().add(2 * SPX_N),
            state.as_mut_ptr(),
            inbuf.as_ptr(),
            SPX_N + SPX_PK_BYTES + mlen as usize,
        );
    }
    /* Otherwise first fill a block, so that finalize only uses the message */
    else {
        core::ptr::copy_nonoverlapping(
            m,
            inbuf.as_mut_ptr().add(SPX_N + SPX_PK_BYTES),
            SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES,
        );
        shaX_inc_blocks(state.as_mut_ptr(), inbuf.as_ptr(), SPX_INBLOCKS);

        m = m.add(SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES);
        mlen -= (SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES) as u64;
        shaX_inc_finalize(
            seed.as_mut_ptr().add(2 * SPX_N),
            state.as_mut_ptr(),
            m,
            mlen as usize,
        );
    }

    // H_msg: MGF1-SHA-X(R ‖ PK.seed ‖ seed)
    core::ptr::copy_nonoverlapping(r, seed.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(pk, seed.as_mut_ptr().add(SPX_N), SPX_N);

    /* By doing this in two steps, we prevent hashing the message twice;
       otherwise each iteration in MGF1 would hash the message again. */
    let mut bufp = buf.as_mut_ptr();
    mgf1_X(
        bufp,
        SPX_DGST_BYTES as c_ulong,
        seed.as_ptr(),
        (2 * SPX_N + SPX_SHAX_OUTPUT_BYTES) as c_ulong,
    );

    core::ptr::copy_nonoverlapping(bufp, digest, SPX_FORS_MSG_BYTES);
    bufp = bufp.add(SPX_FORS_MSG_BYTES);

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = SPX_bytes_to_ull(bufp, SPX_TREE_BYTES as u32);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp = bufp.add(SPX_TREE_BYTES);

    *leaf_idx = SPX_bytes_to_ull(bufp, SPX_LEAF_BYTES as u32) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
