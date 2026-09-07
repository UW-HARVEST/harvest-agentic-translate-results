//! Translation of `lib/blake/src/hash_blake.c`.
//!
//! The C source uses the preprocessor to alias a family of `blakeX_*` symbols
//! to either the BLAKE-512 (`SPX_N >= 24`) or the BLAKE-256 variant.  Because
//! the two BLAKE state structs are *different* Rust types (`blakestate512` vs
//! `blakestate256`), the `#if SPX_N >= 24` branches are reproduced here as
//! pairs of `#[cfg]`-gated private helper functions:
//!
//! * `#[cfg(any(feature="192s",feature="192f",feature="256s",feature="256f"))]`
//!   -> `SPX_N >= 24`, uses BLAKE-512  (`SPX_BLAKEX_OUTPUT_BYTES = 64`)
//! * `#[cfg(not(any(...)))]`
//!   -> `SPX_N <  24`, uses BLAKE-256  (`SPX_BLAKEX_OUTPUT_BYTES = 32`)
//!
//! NOTE: `blake256_mgf1` / `blake512_mgf1` are NOT defined in `hash_blake.c`.
//! They are defined in `blake256.c` / `blake512.c` and therefore belong to the
//! `crate::blake::blake256` / `crate::blake::blake512` modules.  They are only
//! *called* from here.

use core::ffi::{c_ulong, c_ulonglong};

use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::SPX_bytes_to_ull;

use crate::blake::blake256::blake256;

/* `SPX_BLAKE256_OUTPUT_BYTES` / `SPX_BLAKE512_OUTPUT_BYTES` from blake.h. */
const SPX_BLAKE256_OUTPUT_BYTES: usize = 32; /* This does not necessarily equal SPX_N */
#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

/* `#define SPX_BLAKEX_OUTPUT_BYTES ...` (depends on `SPX_N >= 24`). */
#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
const SPX_BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE512_OUTPUT_BYTES;
#[cfg(not(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
const SPX_BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE256_OUTPUT_BYTES;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    let _ = ctx;
}

/// Computes PRF(key, addr), given a secret key of SPX_N bytes and an address
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

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

    blake256(
        outbuf.as_mut_ptr(),
        buf.as_ptr(),
        (SPX_N + SPX_ADDR_BYTES) as c_ulonglong,
    );

    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}

/// Computes the message-dependent randomness R, using a secret seed and an
/// optional randomization value as well as the message.
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
    gen_message_random_impl(R, sk_prf, optrand, m, mlen);
}

/* `blakeX_*` branch for `SPX_N >= 24` (BLAKE-512). */
#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
unsafe fn gen_message_random_impl(
    R: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: c_ulonglong,
) {
    use crate::blake::blake512::{blake512_final, blake512_init, blake512_update, blakestate512};

    let mut S = blakestate512::new();

    blake512_init(&mut S);
    blake512_update(&mut S, sk_prf, SPX_N as c_ulonglong);
    blake512_update(&mut S, optrand, SPX_N as c_ulonglong);
    blake512_update(&mut S, m, mlen);
    blake512_final(&mut S, R);
}

/* `blakeX_*` branch for `SPX_N < 24` (BLAKE-256). */
#[cfg(not(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
unsafe fn gen_message_random_impl(
    R: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: c_ulonglong,
) {
    use crate::blake::blake256::{blake256_final, blake256_init, blake256_update, blakestate256};

    let mut S = blakestate256::new();

    blake256_init(&mut S);
    blake256_update(&mut S, sk_prf, SPX_N as c_ulonglong);
    blake256_update(&mut S, optrand, SPX_N as c_ulonglong);
    blake256_update(&mut S, m, mlen);
    blake256_final(&mut S, R);
}

/// Computes the message hash using R, the public key, and the message.
/// Outputs the message digest and the index of the leaf. The index is split in
/// the tree index and the leaf index, for convenient copying to an address.
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

    /* #define SPX_TREE_BITS  (SPX_TREE_HEIGHT * (SPX_D - 1))  (u32 arithmetic) */
    const SPX_TREE_BITS: u32 = SPX_TREE_HEIGHT * (SPX_D - 1);
    /* #define SPX_TREE_BYTES ((SPX_TREE_BITS + 7) / 8) */
    const SPX_TREE_BYTES: usize = ((SPX_TREE_BITS + 7) / 8) as usize;
    /* #define SPX_LEAF_BITS  SPX_TREE_HEIGHT */
    const SPX_LEAF_BITS: u32 = SPX_TREE_HEIGHT;
    /* #define SPX_LEAF_BYTES ((SPX_LEAF_BITS + 7) / 8) */
    const SPX_LEAF_BYTES: usize = ((SPX_LEAF_BITS + 7) / 8) as usize;
    /* #define SPX_DGST_BYTES (SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES) */
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

    let mut buf = [0u8; SPX_DGST_BYTES];
    // bufp is an index into buf.
    let mut bufp: usize = 0;
    let mut seed = [0u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES];

    /* blakeX_init / update / final over (R, pk, m), writing to seed + 2*SPX_N */
    hash_message_seed(&mut seed, R, pk, m, mlen);

    core::ptr::copy_nonoverlapping(R, seed.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(pk, seed.as_mut_ptr().add(SPX_N), SPX_N);

    /* blakeX_mgf1(bufp, SPX_DGST_BYTES, seed, 2*SPX_N + SPX_BLAKEX_OUTPUT_BYTES) */
    hash_message_mgf1(
        buf.as_mut_ptr(),
        SPX_DGST_BYTES as c_ulong,
        seed.as_ptr(),
        (2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES) as c_ulong,
    );

    core::ptr::copy_nonoverlapping(buf.as_ptr().add(bufp), digest, SPX_FORS_MSG_BYTES);
    bufp += SPX_FORS_MSG_BYTES;

    /* #if SPX_TREE_BITS > 64 -> compile-time error in C; N/A at runtime. */

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = SPX_bytes_to_ull(buf.as_ptr().add(bufp), SPX_TREE_BYTES as core::ffi::c_uint);
        *tree &= (!(0u64)) >> (64 - SPX_TREE_BITS);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx =
        SPX_bytes_to_ull(buf.as_ptr().add(bufp), SPX_LEAF_BYTES as core::ffi::c_uint) as u32;
    *leaf_idx &= (!(0u32)) >> (32 - SPX_LEAF_BITS);
}

/* `blakeX_*` seed computation, BLAKE-512 branch (`SPX_N >= 24`). */
#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
unsafe fn hash_message_seed(
    seed: &mut [u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES],
    R: *const u8,
    pk: *const u8,
    m: *const u8,
    mlen: c_ulonglong,
) {
    use crate::blake::blake512::{blake512_final, blake512_init, blake512_update, blakestate512};

    let mut S = blakestate512::new();
    blake512_init(&mut S);

    blake512_update(&mut S, R, SPX_N as c_ulonglong);
    blake512_update(&mut S, pk, SPX_PK_BYTES as c_ulonglong);
    blake512_update(&mut S, m, mlen);

    blake512_final(&mut S, seed.as_mut_ptr().add(2 * SPX_N));
}

/* `blakeX_*` seed computation, BLAKE-256 branch (`SPX_N < 24`). */
#[cfg(not(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
unsafe fn hash_message_seed(
    seed: &mut [u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES],
    R: *const u8,
    pk: *const u8,
    m: *const u8,
    mlen: c_ulonglong,
) {
    use crate::blake::blake256::{blake256_final, blake256_init, blake256_update, blakestate256};

    let mut S = blakestate256::new();
    blake256_init(&mut S);

    blake256_update(&mut S, R, SPX_N as c_ulonglong);
    blake256_update(&mut S, pk, SPX_PK_BYTES as c_ulonglong);
    blake256_update(&mut S, m, mlen);

    blake256_final(&mut S, seed.as_mut_ptr().add(2 * SPX_N));
}

/* `blakeX_mgf1`, BLAKE-512 branch (`SPX_N >= 24`). */
#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
unsafe fn hash_message_mgf1(out: *mut u8, outlen: c_ulong, in_: *const u8, inlen: c_ulong) {
    crate::blake::blake512::SPX_blake512_mgf1(out, outlen, in_, inlen);
}

/* `blakeX_mgf1`, BLAKE-256 branch (`SPX_N < 24`). */
#[cfg(not(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
unsafe fn hash_message_mgf1(out: *mut u8, outlen: c_ulong, in_: *const u8, inlen: c_ulong) {
    crate::blake::blake256::SPX_blake256_mgf1(out, outlen, in_, inlen);
}
