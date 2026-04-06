// hash_shake.rs - SHAKE-256 based hash functions for SPHINCS+
// Translated from c_src/lib/shake/src/hash_shake.c

use core::ptr;
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::bytes_to_ull;
use super::fips202::*;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    let _ = ctx; // no-op for SHAKE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);
    ptr::copy_nonoverlapping((*ctx).sk_seed.as_ptr(), buf.as_mut_ptr().add(SPX_N + SPX_ADDR_BYTES), SPX_N);

    let mut out_buf = [0u8; SPX_N];
    shake256(&mut out_buf, SPX_N, &buf);
    ptr::copy_nonoverlapping(out_buf.as_ptr(), out, SPX_N);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_gen_message_random(
    R: *mut u8, sk_prf: *const u8, optrand: *const u8,
    m: *const u8, mlen: u64, ctx: *const SpxCtx,
) {
    let _ = ctx;
    let mut s_inc = [0u64; 26];
    shake256_inc_init(&mut s_inc);
    shake256_inc_absorb(&mut s_inc, core::slice::from_raw_parts(sk_prf, SPX_N));
    shake256_inc_absorb(&mut s_inc, core::slice::from_raw_parts(optrand, SPX_N));
    shake256_inc_absorb(&mut s_inc, core::slice::from_raw_parts(m, mlen as usize));
    shake256_inc_finalize(&mut s_inc);
    let mut out_buf = [0u8; SPX_N];
    shake256_inc_squeeze(&mut out_buf, SPX_N, &mut s_inc);
    ptr::copy_nonoverlapping(out_buf.as_ptr(), R, SPX_N);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_hash_message(
    digest: *mut u8, tree: *mut u64, leaf_idx: *mut u32,
    R: *const u8, pk: *const u8, m: *const u8, mlen: u64,
    ctx: *const SpxCtx,
) {
    let _ = ctx;
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u64; 26];

    shake256_inc_init(&mut s_inc);
    shake256_inc_absorb(&mut s_inc, core::slice::from_raw_parts(R, SPX_N));
    shake256_inc_absorb(&mut s_inc, core::slice::from_raw_parts(pk, SPX_PK_BYTES));
    shake256_inc_absorb(&mut s_inc, core::slice::from_raw_parts(m, mlen as usize));
    shake256_inc_finalize(&mut s_inc);
    shake256_inc_squeeze(&mut buf, SPX_DGST_BYTES, &mut s_inc);

    ptr::copy_nonoverlapping(buf.as_ptr(), digest, SPX_FORS_MSG_BYTES);

    let bufp = &buf[SPX_FORS_MSG_BYTES..];
    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(bufp.as_ptr(), SPX_TREE_BYTES as u32);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }

    let bufp = &buf[SPX_FORS_MSG_BYTES + SPX_TREE_BYTES..];
    *leaf_idx = bytes_to_ull(bufp.as_ptr(), SPX_LEAF_BYTES as u32) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
