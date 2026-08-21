//! Translation of `lib/shake/src/hash_shake.c`.

use super::fips202::{
    shake256, shake256_inc_absorb, shake256_inc_finalize, shake256_inc_init, shake256_inc_squeeze,
};
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::SPX_bytes_to_ull;

/// For SHAKE256, there is no immediate reason to initialize at the start.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(_ctx: *mut SpxCtx) {}

/// Computes PRF(pk_seed, sk_seed, addr).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];

    core::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);
    core::ptr::copy_nonoverlapping(
        (*ctx).sk_seed.as_ptr(),
        buf.as_mut_ptr().add(SPX_N + SPX_ADDR_BYTES),
        SPX_N,
    );

    shake256(out, SPX_N, buf.as_ptr(), 2 * SPX_N + SPX_ADDR_BYTES);
}

/// Computes the message-dependent randomness R.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_gen_message_random(
    r: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: u64,
    _ctx: *const SpxCtx,
) {
    let mut s_inc = [0u64; 26];

    shake256_inc_init(s_inc.as_mut_ptr());
    shake256_inc_absorb(s_inc.as_mut_ptr(), sk_prf, SPX_N);
    shake256_inc_absorb(s_inc.as_mut_ptr(), optrand, SPX_N);
    shake256_inc_absorb(s_inc.as_mut_ptr(), m, mlen as usize);
    shake256_inc_finalize(s_inc.as_mut_ptr());
    shake256_inc_squeeze(r, SPX_N, s_inc.as_mut_ptr());
}

/// Computes the message hash using R, the public key, and the message.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    r: *const u8,
    pk: *const u8,
    m: *const u8,
    mlen: u64,
    _ctx: *const SpxCtx,
) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u64; 26];

    shake256_inc_init(s_inc.as_mut_ptr());
    shake256_inc_absorb(s_inc.as_mut_ptr(), r, SPX_N);
    shake256_inc_absorb(s_inc.as_mut_ptr(), pk, SPX_PK_BYTES);
    shake256_inc_absorb(s_inc.as_mut_ptr(), m, mlen as usize);
    shake256_inc_finalize(s_inc.as_mut_ptr());
    shake256_inc_squeeze(buf.as_mut_ptr(), SPX_DGST_BYTES, s_inc.as_mut_ptr());

    let mut bufp = buf.as_ptr();
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
