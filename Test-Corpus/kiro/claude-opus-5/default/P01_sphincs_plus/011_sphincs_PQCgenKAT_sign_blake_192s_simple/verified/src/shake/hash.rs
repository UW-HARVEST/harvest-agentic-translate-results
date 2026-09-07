use core::ffi::c_ulonglong;

use crate::context::SpxCtx;
use crate::params::*;
use crate::shake::fips202::{
    shake256, shake256_inc_absorb, shake256_inc_finalize, shake256_inc_init,
    shake256_inc_squeeze,
};
use crate::utils::SPX_bytes_to_ull;

/* For SHAKE256, there is no immediate reason to initialize at the start,
   so this function is an empty operation. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    let _ = ctx; /* Suppress an 'unused parameter' warning. */
}

/*
 * Computes PRF(pk_seed, sk_seed, addr)
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];

    core::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(
        addr as *const u8,
        buf.as_mut_ptr().add(SPX_N),
        SPX_ADDR_BYTES,
    );
    core::ptr::copy_nonoverlapping(
        (*ctx).sk_seed.as_ptr(),
        buf.as_mut_ptr().add(SPX_N + SPX_ADDR_BYTES),
        SPX_N,
    );

    shake256(out, SPX_N, buf.as_ptr(), 2 * SPX_N + SPX_ADDR_BYTES);
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
    let _ = ctx;
    let mut s_inc = [0u64; 26];

    shake256_inc_init(s_inc.as_mut_ptr());
    shake256_inc_absorb(s_inc.as_mut_ptr(), sk_prf, SPX_N);
    shake256_inc_absorb(s_inc.as_mut_ptr(), optrand, SPX_N);
    shake256_inc_absorb(s_inc.as_mut_ptr(), m, mlen as usize);
    shake256_inc_finalize(s_inc.as_mut_ptr());
    shake256_inc_squeeze(R, SPX_N, s_inc.as_mut_ptr());
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
    let _ = ctx;

    const SPX_TREE_BITS: u32 = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = ((SPX_TREE_BITS + 7) / 8) as usize;
    const SPX_LEAF_BITS: u32 = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = ((SPX_LEAF_BITS + 7) / 8) as usize;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut bufp: *mut u8 = buf.as_mut_ptr();
    let mut s_inc = [0u64; 26];

    shake256_inc_init(s_inc.as_mut_ptr());
    shake256_inc_absorb(s_inc.as_mut_ptr(), R, SPX_N);
    shake256_inc_absorb(s_inc.as_mut_ptr(), pk, SPX_PK_BYTES);
    shake256_inc_absorb(s_inc.as_mut_ptr(), m, mlen as usize);
    shake256_inc_finalize(s_inc.as_mut_ptr());
    shake256_inc_squeeze(buf.as_mut_ptr(), SPX_DGST_BYTES, s_inc.as_mut_ptr());

    core::ptr::copy_nonoverlapping(bufp as *const u8, digest, SPX_FORS_MSG_BYTES);
    bufp = bufp.add(SPX_FORS_MSG_BYTES);

    // #if SPX_TREE_BITS > 64 -> compile error in C; here SPX_TREE_BITS <= 64.

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = SPX_bytes_to_ull(bufp as *const u8, SPX_TREE_BYTES as core::ffi::c_uint);
        *tree &= (!(0u64)) >> (64 - SPX_TREE_BITS);
    }
    bufp = bufp.add(SPX_TREE_BYTES);

    *leaf_idx = SPX_bytes_to_ull(bufp as *const u8, SPX_LEAF_BYTES as core::ffi::c_uint) as u32;
    *leaf_idx &= (!(0u32)) >> (32 - SPX_LEAF_BITS);
}
