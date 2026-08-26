//! Translation of `lib/blake/src/hash_blake.c`.
//!
//! The C file selects between BLAKE-256 and BLAKE-512 with `#if SPX_N >= 24`
//! (the `blakeX` macro family). Here that choice is the runtime-constant
//! `crate::params::SPX_BLAKE512`, which the optimiser folds away.

use super::blake256::{
    blake256, blake256_final, blake256_init, blake256_update, blakestate256, SPX_blake256_mgf1,
    SPX_BLAKE256_OUTPUT_BYTES,
};
use super::blake512::{
    blake512_final, blake512_init, blake512_update, blakestate512, SPX_blake512_mgf1,
    SPX_BLAKE512_OUTPUT_BYTES,
};
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::SPX_bytes_to_ull;
use core::ffi::c_ulong;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(_ctx: *mut SpxCtx) {}

/// Computes PRF(key, addr), given a secret key of SPX_N bytes and an address.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    core::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);
    core::ptr::copy_nonoverlapping(
        (*ctx).sk_seed.as_ptr(),
        buf.as_mut_ptr().add(SPX_N + SPX_ADDR_BYTES),
        SPX_N,
    );

    blake256(
        outbuf.as_mut_ptr(),
        buf.as_ptr(),
        (SPX_N + SPX_ADDR_BYTES) as u64,
    );

    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}

/// Computes the message-dependent randomness R, using a secret seed and an
/// optional randomization value as well as the message.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_gen_message_random(
    r: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: u64,
    _ctx: *const SpxCtx,
) {
    if SPX_BLAKE512 {
        let mut s = blakestate512::new_zeroed();

        blake512_init(&mut s);
        blake512_update(&mut s, sk_prf, SPX_N as u64);
        blake512_update(&mut s, optrand, SPX_N as u64);
        blake512_update(&mut s, m, mlen);
        blake512_final(&mut s, r);
    } else {
        let mut s = blakestate256::new_zeroed();

        blake256_init(&mut s);
        blake256_update(&mut s, sk_prf, SPX_N as u64);
        blake256_update(&mut s, optrand, SPX_N as u64);
        blake256_update(&mut s, m, mlen);
        blake256_final(&mut s, r);
    }
}

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
    m: *const u8,
    mlen: u64,
    _ctx: *const SpxCtx,
) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let bufp0 = buf.as_mut_ptr();

    if SPX_BLAKE512 {
        let seedlen = 2 * SPX_N + SPX_BLAKE512_OUTPUT_BYTES;
        let mut seed = vec![0u8; seedlen];

        let mut s = blakestate512::new_zeroed();
        blake512_init(&mut s);

        blake512_update(&mut s, r, SPX_N as u64);
        blake512_update(&mut s, pk, SPX_PK_BYTES as u64);
        blake512_update(&mut s, m, mlen);

        blake512_final(&mut s, seed.as_mut_ptr().add(2 * SPX_N));

        core::ptr::copy_nonoverlapping(r, seed.as_mut_ptr(), SPX_N);
        core::ptr::copy_nonoverlapping(pk, seed.as_mut_ptr().add(SPX_N), SPX_N);

        SPX_blake512_mgf1(
            bufp0,
            SPX_DGST_BYTES as c_ulong,
            seed.as_ptr(),
            seedlen as c_ulong,
        );
    } else {
        let seedlen = 2 * SPX_N + SPX_BLAKE256_OUTPUT_BYTES;
        let mut seed = vec![0u8; seedlen];

        let mut s = blakestate256::new_zeroed();
        blake256_init(&mut s);

        blake256_update(&mut s, r, SPX_N as u64);
        blake256_update(&mut s, pk, SPX_PK_BYTES as u64);
        blake256_update(&mut s, m, mlen);

        blake256_final(&mut s, seed.as_mut_ptr().add(2 * SPX_N));

        core::ptr::copy_nonoverlapping(r, seed.as_mut_ptr(), SPX_N);
        core::ptr::copy_nonoverlapping(pk, seed.as_mut_ptr().add(SPX_N), SPX_N);

        SPX_blake256_mgf1(
            bufp0,
            SPX_DGST_BYTES as c_ulong,
            seed.as_ptr(),
            seedlen as c_ulong,
        );
    }

    let mut bufp: *const u8 = bufp0;
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
