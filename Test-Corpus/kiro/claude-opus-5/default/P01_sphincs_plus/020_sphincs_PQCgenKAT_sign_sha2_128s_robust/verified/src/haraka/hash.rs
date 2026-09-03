//! Translation of `lib/haraka/src/hash_haraka.c`.

use core::ffi::c_ulonglong;

use crate::context::SpxCtx;
use crate::haraka::haraka::{
    SPX_haraka512, SPX_haraka_S_inc_absorb, SPX_haraka_S_inc_finalize,
    SPX_haraka_S_inc_init, SPX_haraka_S_inc_squeeze, SPX_tweak_constants,
};
use crate::params::*;
use crate::utils::SPX_bytes_to_ull;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    SPX_tweak_constants(ctx);
}

/*
 * Computes PRF(key, addr), given a secret key of SPX_N bytes and an address
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(
    out: *mut u8,
    ctx: *const SpxCtx,
    addr: *const u32,
) {
    /* Since SPX_N may be smaller than 32, we need temporary buffers. */
    let mut outbuf: [u8; 32] = [0u8; 32];
    let mut buf: [u8; 64] = [0u8; 64];

    core::ptr::copy_nonoverlapping(
        addr as *const u8,
        buf.as_mut_ptr(),
        SPX_ADDR_BYTES,
    );
    core::ptr::copy_nonoverlapping(
        (*ctx).sk_seed.as_ptr(),
        buf.as_mut_ptr().add(SPX_ADDR_BYTES),
        SPX_N,
    );

    SPX_haraka512(outbuf.as_mut_ptr(), buf.as_ptr(), ctx);
    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}

/**
 * Computes the message-dependent randomness R, using a secret seed and an
 * optional randomization value as well as the message.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_gen_message_random(
    R: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: c_ulonglong,
    ctx: *const SpxCtx,
) {
    let mut s_inc: [u8; 65] = [0u8; 65];

    SPX_haraka_S_inc_init(s_inc.as_mut_ptr());
    SPX_haraka_S_inc_absorb(s_inc.as_mut_ptr(), sk_prf, SPX_N, ctx);
    SPX_haraka_S_inc_absorb(s_inc.as_mut_ptr(), optrand, SPX_N, ctx);
    SPX_haraka_S_inc_absorb(s_inc.as_mut_ptr(), m, mlen as usize, ctx);
    SPX_haraka_S_inc_finalize(s_inc.as_mut_ptr());
    SPX_haraka_S_inc_squeeze(R, SPX_N, s_inc.as_mut_ptr(), ctx);
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
    m: *const u8,
    mlen: c_ulonglong,
    ctx: *const SpxCtx,
) {
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

    let mut buf: [u8; SPX_DGST_BYTES] = [0u8; SPX_DGST_BYTES];
    let mut bufp: *mut u8 = buf.as_mut_ptr();
    let mut s_inc: [u8; 65] = [0u8; 65];

    SPX_haraka_S_inc_init(s_inc.as_mut_ptr());
    SPX_haraka_S_inc_absorb(s_inc.as_mut_ptr(), R, SPX_N, ctx);
    // Only absorb root part of pk
    SPX_haraka_S_inc_absorb(s_inc.as_mut_ptr(), pk.add(SPX_N), SPX_N, ctx);
    SPX_haraka_S_inc_absorb(s_inc.as_mut_ptr(), m, mlen as usize, ctx);
    SPX_haraka_S_inc_finalize(s_inc.as_mut_ptr());
    SPX_haraka_S_inc_squeeze(buf.as_mut_ptr(), SPX_DGST_BYTES, s_inc.as_mut_ptr(), ctx);

    core::ptr::copy_nonoverlapping(bufp, digest, SPX_FORS_MSG_BYTES);
    bufp = bufp.add(SPX_FORS_MSG_BYTES);

    // #if SPX_TREE_BITS > 64 -> #error
    const _: () = assert!(SPX_TREE_BITS <= 64);

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = SPX_bytes_to_ull(bufp, SPX_TREE_BYTES as core::ffi::c_uint);
        *tree &= (!(0u64)) >> (64 - SPX_TREE_BITS);
    }
    bufp = bufp.add(SPX_TREE_BYTES);

    *leaf_idx =
        SPX_bytes_to_ull(bufp, SPX_LEAF_BYTES as core::ffi::c_uint) as u32;
    *leaf_idx &= (!(0u32)) >> (32 - SPX_LEAF_BITS);
}
