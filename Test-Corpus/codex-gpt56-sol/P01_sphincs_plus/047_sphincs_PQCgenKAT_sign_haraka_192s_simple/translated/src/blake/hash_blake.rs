//! Translation of `lib/blake/src/hash_blake.c`.

use core::ptr::{addr_of, copy_nonoverlapping};

use crate::blake::blake256::*;
use crate::blake::blake512::*;
use crate::context::SpxCtx;
use crate::params::{
    SPX_ADDR_BYTES, SPX_BLAKE512, SPX_D, SPX_FORS_MSG_BYTES, SPX_N, SPX_PK_BYTES, SPX_TREE_HEIGHT,
};
use crate::utils::SPX_bytes_to_ull;

/* From `lib/blake/include/blake.h` (kept locally so that this module does not
 * depend on them being re-exported):
 *   #define SPX_BLAKE256_OUTPUT_BYTES 32
 *   #define SPX_BLAKE512_OUTPUT_BYTES 64
 */
const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;
const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

/* C:
 *   #if SPX_N >= 24
 *   #define SPX_BLAKEX_OUTPUT_BYTES SPX_BLAKE512_OUTPUT_BYTES
 *   #define blakeX        blake512
 *   #define blakestateX   blakestate512
 *   #define blakeX_init   blake512_init
 *   #define blakeX_update blake512_update
 *   #define blakeX_final  blake512_final
 *   #define blakeX_mgf1   blake512_mgf1
 *   #else  (the BLAKE-256 variants)
 *   #endif
 * `SPX_N >= 24` is exactly `SPX_BLAKE512`, so the selection becomes a
 * compile-time constant plus runtime dispatch in the wrappers below.
 */
const SPX_BLAKEX_OUTPUT_BYTES: usize = if SPX_BLAKE512 {
    SPX_BLAKE512_OUTPUT_BYTES
} else {
    SPX_BLAKE256_OUTPUT_BYTES
};

/* Both possible `blakestateX` incarnations are kept side by side; only the one
 * selected by `SPX_BLAKE512` is ever touched. */
#[inline]
fn blakestateX_256() -> blakestate256 {
    blakestate256 {
        h: [0; 8],
        s: [0; 4],
        t: [0; 2],
        buflen: 0,
        nullt: 0,
        buf: [0; 64],
    }
}

#[inline]
fn blakestateX_512() -> blakestate512 {
    blakestate512 {
        h: [0; 8],
        s: [0; 4],
        t: [0; 2],
        buflen: 0,
        nullt: 0,
        buf: [0; 128],
    }
}

#[inline]
unsafe fn blakeX_init(s256: *mut blakestate256, s512: *mut blakestate512) {
    if SPX_BLAKE512 {
        blake512_init(s512);
    } else {
        blake256_init(s256);
    }
}

#[inline]
unsafe fn blakeX_update(
    s256: *mut blakestate256,
    s512: *mut blakestate512,
    in_: *const u8,
    inlen: u64,
) {
    if SPX_BLAKE512 {
        blake512_update(s512, in_, inlen);
    } else {
        blake256_update(s256, in_, inlen);
    }
}

#[inline]
unsafe fn blakeX_final(s256: *mut blakestate256, s512: *mut blakestate512, out: *mut u8) {
    if SPX_BLAKE512 {
        blake512_final(s512, out);
    } else {
        blake256_final(s256, out);
    }
}

#[inline]
unsafe fn blakeX_mgf1(out: *mut u8, outlen: u64, in_: *const u8, inlen: u64) {
    if SPX_BLAKE512 {
        SPX_blake512_mgf1(out, outlen, in_, inlen);
    } else {
        SPX_blake256_mgf1(out, outlen, in_, inlen);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    let _ = ctx; /* C: (void)ctx; */
}

/**
 * Computes PRF(key, addr), given a secret key of SPX_N bytes and an address
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let buf_ptr: *mut u8 = buf.as_mut_ptr();
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    copy_nonoverlapping(addr_of!((*ctx).pub_seed) as *const u8, buf_ptr, SPX_N);
    copy_nonoverlapping(addr as *const u8, buf_ptr.add(SPX_N), SPX_ADDR_BYTES);
    copy_nonoverlapping(
        addr_of!((*ctx).sk_seed) as *const u8,
        buf_ptr.add(SPX_N + SPX_ADDR_BYTES),
        SPX_N,
    );

    blake256(
        outbuf.as_mut_ptr(),
        buf_ptr as *const u8,
        (SPX_N + SPX_ADDR_BYTES) as u64,
    );

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
    let _ = ctx; /* C: (void)ctx; */
    let mut s256 = blakestateX_256();
    let mut s512 = blakestateX_512();
    let (s256, s512): (*mut blakestate256, *mut blakestate512) = (&mut s256, &mut s512);

    blakeX_init(s256, s512);
    blakeX_update(s256, s512, sk_prf, SPX_N as u64);
    blakeX_update(s256, s512, optrand, SPX_N as u64);
    blakeX_update(s256, s512, m, mlen);
    blakeX_final(s256, s512, R);
}

/* The `#define`s that hash_blake.c places inside hash_message(); as
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
    let _ = ctx; /* C: (void)ctx; */

    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut bufp: *mut u8 = buf.as_mut_ptr();
    let mut seed = [0u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES];
    let seed_ptr: *mut u8 = seed.as_mut_ptr();

    let mut s256 = blakestateX_256();
    let mut s512 = blakestateX_512();
    let (s256, s512): (*mut blakestate256, *mut blakestate512) = (&mut s256, &mut s512);

    blakeX_init(s256, s512);

    blakeX_update(s256, s512, R, SPX_N as u64);
    blakeX_update(s256, s512, pk, SPX_PK_BYTES as u64);
    blakeX_update(s256, s512, m, mlen);

    blakeX_final(s256, s512, seed_ptr.add(2 * SPX_N));

    copy_nonoverlapping(R, seed_ptr, SPX_N);
    copy_nonoverlapping(pk, seed_ptr.add(SPX_N), SPX_N);

    blakeX_mgf1(
        bufp,
        SPX_DGST_BYTES as u64,
        seed_ptr as *const u8,
        (2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES) as u64,
    );

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
