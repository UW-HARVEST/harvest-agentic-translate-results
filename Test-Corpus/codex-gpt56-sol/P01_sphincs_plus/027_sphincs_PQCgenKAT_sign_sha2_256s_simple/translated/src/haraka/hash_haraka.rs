//! Translation of `lib/haraka/src/hash_haraka.c`.

use core::ptr::{addr_of, copy_nonoverlapping};

use crate::context::SpxCtx;
use crate::haraka::haraka::{
    SPX_haraka512, SPX_haraka_S_inc_absorb, SPX_haraka_S_inc_finalize, SPX_haraka_S_inc_init,
    SPX_haraka_S_inc_squeeze, SPX_tweak_constants,
};
use crate::params::{SPX_ADDR_BYTES, SPX_D, SPX_FORS_MSG_BYTES, SPX_N, SPX_TREE_HEIGHT};
use crate::utils::SPX_bytes_to_ull;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    SPX_tweak_constants(ctx);
}

/*
 * Computes PRF(key, addr), given a secret key of SPX_N bytes and an address
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    /* Since SPX_N may be smaller than 32, we need temporary buffers. */
    let mut outbuf = [0u8; 32];
    let mut buf = [0u8; 64];
    let buf_ptr: *mut u8 = buf.as_mut_ptr();

    copy_nonoverlapping(addr as *const u8, buf_ptr, SPX_ADDR_BYTES);
    copy_nonoverlapping(
        addr_of!((*ctx).sk_seed) as *const u8,
        buf_ptr.add(SPX_ADDR_BYTES),
        SPX_N,
    );

    SPX_haraka512(outbuf.as_mut_ptr(), buf_ptr, ctx);
    copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
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
    mlen: u64,
    ctx: *const SpxCtx,
) {
    let mut s_inc = [0u8; 65];
    let s_inc_ptr: *mut u8 = s_inc.as_mut_ptr();

    SPX_haraka_S_inc_init(s_inc_ptr);
    SPX_haraka_S_inc_absorb(s_inc_ptr, sk_prf, SPX_N, ctx);
    SPX_haraka_S_inc_absorb(s_inc_ptr, optrand, SPX_N, ctx);
    SPX_haraka_S_inc_absorb(s_inc_ptr, m, mlen as usize, ctx);
    SPX_haraka_S_inc_finalize(s_inc_ptr);
    SPX_haraka_S_inc_squeeze(R, SPX_N, s_inc_ptr, ctx);
}

/* The `#define`s that hash_haraka.c places inside hash_message(); as
 * compile-time constants they adapt to the selected parameter set. */
const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

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
    let mut buf = [0u8; SPX_DGST_BYTES];
    let buf_ptr: *mut u8 = buf.as_mut_ptr();
    let mut bufp: *mut u8 = buf_ptr;
    let mut s_inc = [0u8; 65];
    let s_inc_ptr: *mut u8 = s_inc.as_mut_ptr();

    SPX_haraka_S_inc_init(s_inc_ptr);
    SPX_haraka_S_inc_absorb(s_inc_ptr, R, SPX_N, ctx);
    // Only absorb root part of pk
    SPX_haraka_S_inc_absorb(s_inc_ptr, pk.add(SPX_N), SPX_N, ctx);
    SPX_haraka_S_inc_absorb(s_inc_ptr, m, mlen as usize, ctx);
    SPX_haraka_S_inc_finalize(s_inc_ptr);
    SPX_haraka_S_inc_squeeze(buf_ptr, SPX_DGST_BYTES, s_inc_ptr, ctx);

    copy_nonoverlapping(bufp as *const u8, digest, SPX_FORS_MSG_BYTES);
    bufp = bufp.add(SPX_FORS_MSG_BYTES);

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = SPX_bytes_to_ull(bufp as *const u8, SPX_TREE_BYTES as u32);
        /* C: *tree &= (~(uint64_t)0) >> (64 - SPX_TREE_BITS);
         * `wrapping_shr` keeps the shift well-defined when SPX_TREE_BITS == 0
         * (i.e. SPX_D == 1), in which case this branch is never taken anyway. */
        *tree &= (!0u64).wrapping_shr((64 - SPX_TREE_BITS) as u32);
    }
    bufp = bufp.add(SPX_TREE_BYTES);

    *leaf_idx = SPX_bytes_to_ull(bufp as *const u8, SPX_LEAF_BYTES as u32) as u32;
    /* C: *leaf_idx &= (~(uint32_t)0) >> (32 - SPX_LEAF_BITS); */
    *leaf_idx &= (!0u32).wrapping_shr((32 - SPX_LEAF_BITS) as u32);
}
