use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::bytes_to_ull;

use super::blake256::{blake256, blake256_init, blake256_update, blake256_final,
                      Blakestate256, SPX_BLAKE256_OUTPUT_BYTES};
use super::blake512::{blake512_init, blake512_update, blake512_final,
                      Blakestate512, SPX_BLAKE512_OUTPUT_BYTES};

// Select blakeX variant based on SPX_N
// SPX_N >= 24 => blake512, else blake256

const SPX_BLAKEX_OUTPUT_BYTES: usize = if SPX_N >= 24 {
    SPX_BLAKE512_OUTPUT_BYTES
} else {
    SPX_BLAKE256_OUTPUT_BYTES
};

unsafe fn blakex_mgf1(out: *mut u8, outlen: usize, in_: *const u8, inlen: usize) {
    if SPX_N >= 24 {
        super::blake512::SPX_blake512_mgf1(out, outlen as u64, in_, inlen as u64);
    } else {
        super::blake256::SPX_blake256_mgf1(out, outlen as u64, in_, inlen as u64);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(
    _ctx: *mut SpxCtx,
) {
    // No-op for BLAKE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(
    out: *mut u8,
    ctx: *const SpxCtx,
    addr: *const u32,
) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    core::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);
    core::ptr::copy_nonoverlapping((*ctx).sk_seed.as_ptr(), buf.as_mut_ptr().add(SPX_N + SPX_ADDR_BYTES), SPX_N);

    blake256(outbuf.as_mut_ptr(), buf.as_ptr(), (SPX_N + SPX_ADDR_BYTES) as u64);

    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_gen_message_random(
    r: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: u64,
    _ctx: *const SpxCtx,
) {
    if SPX_N >= 24 {
        let mut s = core::mem::MaybeUninit::<Blakestate512>::uninit();
        let s = s.assume_init_mut();
        blake512_init(s);
        blake512_update(s, sk_prf, SPX_N as u64);
        blake512_update(s, optrand, SPX_N as u64);
        blake512_update(s, m, mlen);
        blake512_final(s, r);
    } else {
        let mut s = core::mem::MaybeUninit::<Blakestate256>::uninit();
        let s = s.assume_init_mut();
        blake256_init(s);
        blake256_update(s, sk_prf, SPX_N as u64);
        blake256_update(s, optrand, SPX_N as u64);
        blake256_update(s, m, mlen);
        blake256_final(s, r);
    }
}

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
    let mut seed = [0u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES];

    if SPX_N >= 24 {
        let mut s = core::mem::MaybeUninit::<Blakestate512>::uninit();
        let s = s.assume_init_mut();
        blake512_init(s);
        blake512_update(s, r, SPX_N as u64);
        blake512_update(s, pk, SPX_PK_BYTES as u64);
        blake512_update(s, m, mlen);
        blake512_final(s, seed.as_mut_ptr().add(2 * SPX_N));
    } else {
        let mut s = core::mem::MaybeUninit::<Blakestate256>::uninit();
        let s = s.assume_init_mut();
        blake256_init(s);
        blake256_update(s, r, SPX_N as u64);
        blake256_update(s, pk, SPX_PK_BYTES as u64);
        blake256_update(s, m, mlen);
        blake256_final(s, seed.as_mut_ptr().add(2 * SPX_N));
    }

    core::ptr::copy_nonoverlapping(r, seed.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(pk, seed.as_mut_ptr().add(SPX_N), SPX_N);

    blakex_mgf1(buf.as_mut_ptr(), SPX_DGST_BYTES, seed.as_ptr(), 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES);

    core::ptr::copy_nonoverlapping(buf.as_ptr(), digest, SPX_FORS_MSG_BYTES);
    let mut bufp = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(buf.as_ptr().add(bufp), SPX_TREE_BYTES as u32);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(buf.as_ptr().add(bufp), SPX_LEAF_BYTES as u32) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
