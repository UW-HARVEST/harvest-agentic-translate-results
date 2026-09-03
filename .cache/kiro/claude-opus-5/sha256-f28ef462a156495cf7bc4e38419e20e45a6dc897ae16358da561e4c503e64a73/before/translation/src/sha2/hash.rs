//! `lib/sha2/src/hash_sha2.c` -- SHA-2 backend hash functions.

use core::ffi::c_ulonglong;
use core::ptr::copy_nonoverlapping;

use crate::context::SpxCtx;
use crate::params::*;
use crate::sha2::sha2::{
    sha256_inc_finalize, SPX_seed_state, SPX_SHA256_ADDR_BYTES,
    SPX_SHA256_OUTPUT_BYTES,
};
use crate::utils::SPX_bytes_to_ull;

// --- `#if SPX_N >= 24` macro aliases from hash_sha2.c -----------------------
//
// The C selects SHA-256 vs SHA-512 primitives via the `shaX_*` / `mgf1_X`
// macro aliases and the `SPX_SHAX_*` sizes, keyed on `SPX_N >= 24`
// (i.e. `SPX_SHA512`). We reproduce that with `#[cfg]`-gated aliases so the
// function bodies below stay byte-identical to the C.

#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
mod shax {
    use crate::sha2::sha2::{
        SPX_SHA512_BLOCK_BYTES, SPX_SHA512_OUTPUT_BYTES,
    };
    pub const SPX_SHAX_OUTPUT_BYTES: usize = SPX_SHA512_OUTPUT_BYTES;
    pub const SPX_SHAX_BLOCK_BYTES: usize = SPX_SHA512_BLOCK_BYTES;
    pub use crate::sha2::sha2::{
        sha512 as shaX, sha512_inc_blocks as shaX_inc_blocks,
        sha512_inc_finalize as shaX_inc_finalize, sha512_inc_init as shaX_inc_init,
        SPX_mgf1_512 as mgf1_X,
    };
}

#[cfg(not(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
mod shax {
    use crate::sha2::sha2::{
        SPX_SHA256_BLOCK_BYTES, SPX_SHA256_OUTPUT_BYTES,
    };
    pub const SPX_SHAX_OUTPUT_BYTES: usize = SPX_SHA256_OUTPUT_BYTES;
    pub const SPX_SHAX_BLOCK_BYTES: usize = SPX_SHA256_BLOCK_BYTES;
    pub use crate::sha2::sha2::{
        sha256 as shaX, sha256_inc_blocks as shaX_inc_blocks,
        sha256_inc_finalize as shaX_inc_finalize, sha256_inc_init as shaX_inc_init,
        SPX_mgf1_256 as mgf1_X,
    };
}

use shax::*;

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
pub unsafe extern "C" fn SPX_prf_addr(
    out: *mut u8,
    ctx: *const SpxCtx,
    addr: *const u32,
) {
    let mut sha2_state: [u8; 40] = [0u8; 40];
    let mut buf: [u8; SPX_SHA256_ADDR_BYTES + SPX_N] = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let mut outbuf: [u8; SPX_SHA256_OUTPUT_BYTES] = [0u8; SPX_SHA256_OUTPUT_BYTES];

    /* Retrieve precomputed state containing pub_seed */
    copy_nonoverlapping(
        (*ctx).state_seeded.as_ptr(),
        sha2_state.as_mut_ptr(),
        40,
    );

    /* Remainder: ADDR^c ‖ SK.seed */
    copy_nonoverlapping(
        addr as *const u8,
        buf.as_mut_ptr(),
        SPX_SHA256_ADDR_BYTES,
    );
    copy_nonoverlapping(
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

    copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}

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
    mut m: *const u8,
    mut mlen: c_ulonglong,
    ctx: *const SpxCtx,
) {
    let _ = ctx;

    let mut buf: [u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES] =
        [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    let mut state: [u8; 8 + SPX_SHAX_OUTPUT_BYTES] = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];
    let mut i: usize;

    /* This implements HMAC-SHA */
    i = 0;
    while i < SPX_N {
        buf[i] = 0x36 ^ *sk_prf.add(i);
        i += 1;
    }
    core::ptr::write_bytes(
        buf.as_mut_ptr().add(SPX_N),
        0x36,
        SPX_SHAX_BLOCK_BYTES - SPX_N,
    );

    shaX_inc_init(state.as_mut_ptr());
    shaX_inc_blocks(state.as_mut_ptr(), buf.as_ptr(), 1);

    copy_nonoverlapping(optrand, buf.as_mut_ptr(), SPX_N);

    /* If optrand + message cannot fill up an entire block */
    if (SPX_N as c_ulonglong) + mlen < SPX_SHAX_BLOCK_BYTES as c_ulonglong {
        copy_nonoverlapping(m, buf.as_mut_ptr().add(SPX_N), mlen as usize);
        shaX_inc_finalize(
            buf.as_mut_ptr().add(SPX_SHAX_BLOCK_BYTES),
            state.as_mut_ptr(),
            buf.as_ptr(),
            (mlen + SPX_N as c_ulonglong) as usize,
        );
    }
    /* Otherwise first fill a block, so that finalize only uses the message */
    else {
        copy_nonoverlapping(
            m,
            buf.as_mut_ptr().add(SPX_N),
            SPX_SHAX_BLOCK_BYTES - SPX_N,
        );
        shaX_inc_blocks(state.as_mut_ptr(), buf.as_ptr(), 1);

        m = m.add(SPX_SHAX_BLOCK_BYTES - SPX_N);
        mlen -= (SPX_SHAX_BLOCK_BYTES - SPX_N) as c_ulonglong;
        shaX_inc_finalize(
            buf.as_mut_ptr().add(SPX_SHAX_BLOCK_BYTES),
            state.as_mut_ptr(),
            m,
            mlen as usize,
        );
    }

    i = 0;
    while i < SPX_N {
        buf[i] = 0x5c ^ *sk_prf.add(i);
        i += 1;
    }
    core::ptr::write_bytes(
        buf.as_mut_ptr().add(SPX_N),
        0x5c,
        SPX_SHAX_BLOCK_BYTES - SPX_N,
    );

    shaX(
        buf.as_mut_ptr(),
        buf.as_ptr(),
        SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES,
    );
    copy_nonoverlapping(buf.as_ptr(), R, SPX_N);
}

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
    mut m: *const u8,
    mut mlen: c_ulonglong,
    ctx: *const SpxCtx,
) {
    let _ = ctx;

    // #define SPX_TREE_BITS (SPX_TREE_HEIGHT * (SPX_D - 1))
    const SPX_TREE_BITS: u32 = SPX_TREE_HEIGHT * (SPX_D - 1);
    // #define SPX_TREE_BYTES ((SPX_TREE_BITS + 7) / 8)
    const SPX_TREE_BYTES: usize = ((SPX_TREE_BITS + 7) / 8) as usize;
    // #define SPX_LEAF_BITS SPX_TREE_HEIGHT
    const SPX_LEAF_BITS: u32 = SPX_TREE_HEIGHT;
    // #define SPX_LEAF_BYTES ((SPX_LEAF_BITS + 7) / 8)
    const SPX_LEAF_BYTES: usize = ((SPX_LEAF_BITS + 7) / 8) as usize;
    // #define SPX_DGST_BYTES (SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES)
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

    let mut seed: [u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES] =
        [0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES];

    /* Round to nearest multiple of SPX_SHAX_BLOCK_BYTES */
    // #define SPX_INBLOCKS (((SPX_N + SPX_PK_BYTES + SPX_SHAX_BLOCK_BYTES - 1) &
    //                        -SPX_SHAX_BLOCK_BYTES) / SPX_SHAX_BLOCK_BYTES)
    const SPX_INBLOCKS: usize = ((SPX_N + SPX_PK_BYTES + SPX_SHAX_BLOCK_BYTES - 1)
        & (SPX_SHAX_BLOCK_BYTES.wrapping_neg()))
        / SPX_SHAX_BLOCK_BYTES;
    let mut inbuf: [u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES] =
        [0u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES];

    let mut buf: [u8; SPX_DGST_BYTES] = [0u8; SPX_DGST_BYTES];
    let mut bufp: *mut u8 = buf.as_mut_ptr();
    let mut state: [u8; 8 + SPX_SHAX_OUTPUT_BYTES] = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    shaX_inc_init(state.as_mut_ptr());

    // seed: SHA-X(R ‖ PK.seed ‖ PK.root ‖ M)
    copy_nonoverlapping(R, inbuf.as_mut_ptr(), SPX_N);
    copy_nonoverlapping(pk, inbuf.as_mut_ptr().add(SPX_N), SPX_PK_BYTES);

    /* If R + pk + message cannot fill up an entire block */
    if (SPX_N as c_ulonglong) + SPX_PK_BYTES as c_ulonglong + mlen
        < (SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES) as c_ulonglong
    {
        copy_nonoverlapping(
            m,
            inbuf.as_mut_ptr().add(SPX_N + SPX_PK_BYTES),
            mlen as usize,
        );
        shaX_inc_finalize(
            seed.as_mut_ptr().add(2 * SPX_N),
            state.as_mut_ptr(),
            inbuf.as_ptr(),
            (SPX_N as c_ulonglong + SPX_PK_BYTES as c_ulonglong + mlen) as usize,
        );
    }
    /* Otherwise first fill a block, so that finalize only uses the message */
    else {
        copy_nonoverlapping(
            m,
            inbuf.as_mut_ptr().add(SPX_N + SPX_PK_BYTES),
            SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES,
        );
        shaX_inc_blocks(state.as_mut_ptr(), inbuf.as_ptr(), SPX_INBLOCKS);

        m = m.add(SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES);
        mlen -= (SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES) as c_ulonglong;
        shaX_inc_finalize(
            seed.as_mut_ptr().add(2 * SPX_N),
            state.as_mut_ptr(),
            m,
            mlen as usize,
        );
    }

    // H_msg: MGF1-SHA-X(R ‖ PK.seed ‖ seed)
    copy_nonoverlapping(R, seed.as_mut_ptr(), SPX_N);
    copy_nonoverlapping(pk, seed.as_mut_ptr().add(SPX_N), SPX_N);

    /* By doing this in two steps, we prevent hashing the message twice;
       otherwise each iteration in MGF1 would hash the message again. */
    mgf1_X(
        bufp,
        SPX_DGST_BYTES as core::ffi::c_ulong,
        seed.as_ptr(),
        (2 * SPX_N + SPX_SHAX_OUTPUT_BYTES) as core::ffi::c_ulong,
    );

    copy_nonoverlapping(bufp, digest, SPX_FORS_MSG_BYTES);
    bufp = bufp.add(SPX_FORS_MSG_BYTES);

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = SPX_bytes_to_ull(bufp, SPX_TREE_BYTES as core::ffi::c_uint);
        *tree &= (!(0u64)) >> (64 - SPX_TREE_BITS);
    }
    bufp = bufp.add(SPX_TREE_BYTES);

    *leaf_idx = SPX_bytes_to_ull(bufp, SPX_LEAF_BYTES as core::ffi::c_uint) as u32;
    *leaf_idx &= (!(0u32)) >> (32 - SPX_LEAF_BITS);
}
