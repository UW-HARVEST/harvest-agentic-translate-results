//! Translation of `lib/blake/src/hash_blake.c`.

use crate::address::addr_bytes;
use crate::backend::blake256::{BlakeState256, SPX_BLAKE256_OUTPUT_BYTES};
use crate::backend::blake512::BlakeState512;
use crate::context::SpxCtx;
use crate::params::{
    SPX_ADDR_BYTES, SPX_D, SPX_FORS_MSG_BYTES, SPX_N, SPX_PK_BYTES, SPX_TREE_HEIGHT,
};
use crate::utils::bytes_to_ull;

// ---------------------------------------------------------------------------
// #if SPX_N >= 24  ->  blakeX == blake512, else blakeX == blake256
// ---------------------------------------------------------------------------
#[cfg(any(
    feature = "192s",
    feature = "192f",
    feature = "256s",
    feature = "256f"
))]
mod blakex {
    pub use crate::backend::blake512::{
        blake512_final as blakex_final, blake512_init as blakex_init,
        blake512_mgf1 as blakex_mgf1, blake512_update_bits as blakex_update_bits,
        BlakeState512 as BlakestateX, SPX_BLAKE512_OUTPUT_BYTES as SPX_BLAKEX_OUTPUT_BYTES,
    };
}

#[cfg(not(any(
    feature = "192s",
    feature = "192f",
    feature = "256s",
    feature = "256f"
)))]
mod blakex {
    pub use crate::backend::blake256::{
        blake256_final as blakex_final, blake256_init as blakex_init,
        blake256_mgf1 as blakex_mgf1, blake256_update_bits as blakex_update_bits,
        BlakeState256 as BlakestateX, SPX_BLAKE256_OUTPUT_BYTES as SPX_BLAKEX_OUTPUT_BYTES,
    };
}

use blakex::*;

pub fn initialize_hash_function(_ctx: &mut SpxCtx) {
    /* (void)ctx; */
}

/// Computes PRF(key, addr), given a secret key of SPX_N bytes and an address.
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_ADDR_BYTES]);
    buf[SPX_N + SPX_ADDR_BYTES..2 * SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ctx.sk_seed);

    /* NOTE: as in the C reference, only the first SPX_N + SPX_ADDR_BYTES bytes
     * are hashed (the appended sk_seed is not covered). */
    crate::backend::blake256::blake256(&mut outbuf, &buf[..SPX_N + SPX_ADDR_BYTES]);

    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

/// Computes the message-dependent randomness R, using a secret seed and an
/// optional randomization value as well as the message.
///
/// NOTE: `blakeX_update` takes its length in **bits**, but the C reference hands
/// it *byte* counts (`SPX_N`, `mlen`). This is faithfully reproduced: only
/// `len / 8` bytes of each input are actually absorbed.
pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], _ctx: &SpxCtx) {
    let mut s = BlakestateX::new();

    blakex_init(&mut s);
    blakex_update_bits(&mut s, &sk_prf[..SPX_N], SPX_N as u64);
    blakex_update_bits(&mut s, &optrand[..SPX_N], SPX_N as u64);
    blakex_update_bits(&mut s, m, m.len() as u64);

    /* `blakeX_final` unconditionally emits SPX_BLAKEX_OUTPUT_BYTES bytes, which
     * is more than SPX_N whenever SPX_N < SPX_BLAKEX_OUTPUT_BYTES. The C
     * reference passes `R` (a pointer into the signature buffer) directly, so
     * the surplus bytes spill past R into the following region of the
     * signature. Those bytes are subsequently overwritten in full by the FORS
     * signature, so they never reach the output. Finalizing into a local buffer
     * and copying back only what fits in `r` is therefore byte-identical to the
     * C original, while staying inside Rust's bounds. */
    let mut outbuf = [0u8; SPX_BLAKEX_OUTPUT_BYTES];
    blakex_final(&mut s, &mut outbuf);

    let n = core::cmp::min(r.len(), SPX_BLAKEX_OUTPUT_BYTES);
    r[..n].copy_from_slice(&outbuf[..n]);
}

const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

/// Computes the message hash using R, the public key, and the message.
/// Outputs the message digest and the index of the leaf. The index is split in
/// the tree index and the leaf index, for convenient copying to an address.
pub fn hash_message(
    digest: &mut [u8],
    tree: &mut u64,
    leaf_idx: &mut u32,
    r: &[u8],
    pk: &[u8],
    m: &[u8],
    _ctx: &SpxCtx,
) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut seed = [0u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES];

    let mut s = BlakestateX::new();
    blakex_init(&mut s);

    /* NOTE: byte counts handed to a bit-count API, exactly as in the C. */
    blakex_update_bits(&mut s, &r[..SPX_N], SPX_N as u64);
    blakex_update_bits(&mut s, &pk[..SPX_PK_BYTES], SPX_PK_BYTES as u64);
    blakex_update_bits(&mut s, m, m.len() as u64);

    blakex_final(&mut s, &mut seed[2 * SPX_N..]);

    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    blakex_mgf1(&mut buf[..SPX_DGST_BYTES], &seed);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    /* #if SPX_TREE_BITS > 64 -> #error (checked at compile time in C) */

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES as u32);
        *tree &= u64::MAX.wrapping_shr((64 - SPX_TREE_BITS) as u32);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES as u32) as u32;
    *leaf_idx &= u32::MAX.wrapping_shr((32 - SPX_LEAF_BITS) as u32);
}

// ===========================================================================
// C ABI wrappers
// ===========================================================================

// --- lib/blake/include/blake.h (non-namespaced) -----------------------------

/// `int blake256(unsigned char *out, const unsigned char *in, unsigned long long inlen)`
#[unsafe(no_mangle)]
pub extern "C" fn blake256(out: *mut u8, input: *const u8, inlen: u64) -> i32 {
    unsafe {
        let out = core::slice::from_raw_parts_mut(out, SPX_BLAKE256_OUTPUT_BYTES);
        let input = core::slice::from_raw_parts(input, inlen as usize);
        crate::backend::blake256::blake256(out, input)
    }
}

/// `void blake256_init(blakestate256 *S)`
#[unsafe(no_mangle)]
pub extern "C" fn blake256_init(s: *mut BlakeState256) {
    unsafe { crate::backend::blake256::blake256_init(&mut *s) }
}

/// `void blake256_compress(blakestate256 *S, const unsigned char *block)`
#[unsafe(no_mangle)]
pub extern "C" fn blake256_compress(s: *mut BlakeState256, block: *const u8) {
    unsafe {
        let block = core::slice::from_raw_parts(block, 64);
        crate::backend::blake256::blake256_compress(&mut *s, block)
    }
}

/// `void blake256_update(blakestate256 *S, const unsigned char *in, unsigned long long inlen)`
/// (`inlen` is a *bit* count, as in the C source.)
#[unsafe(no_mangle)]
pub extern "C" fn blake256_update(s: *mut BlakeState256, input: *const u8, inlen: u64) {
    unsafe {
        let input = core::slice::from_raw_parts(input, (inlen >> 3) as usize);
        crate::backend::blake256::blake256_update_bits(&mut *s, input, inlen)
    }
}

/// `void blake256_final(blakestate256 *S, unsigned char *out)`
#[unsafe(no_mangle)]
pub extern "C" fn blake256_final(s: *mut BlakeState256, out: *mut u8) {
    unsafe {
        let out = core::slice::from_raw_parts_mut(out, SPX_BLAKE256_OUTPUT_BYTES);
        crate::backend::blake256::blake256_final(&mut *s, out)
    }
}

/// `int blake512(uint8_t *out, const unsigned char *in, unsigned long long inlen)`
#[unsafe(no_mangle)]
pub extern "C" fn blake512(out: *mut u8, input: *const u8, inlen: u64) -> i32 {
    unsafe {
        let out = core::slice::from_raw_parts_mut(
            out,
            crate::backend::blake512::SPX_BLAKE512_OUTPUT_BYTES,
        );
        let input = core::slice::from_raw_parts(input, inlen as usize);
        crate::backend::blake512::blake512(out, input)
    }
}

/// `void blake512_init(blakestate512 *S)`
#[unsafe(no_mangle)]
pub extern "C" fn blake512_init(s: *mut BlakeState512) {
    unsafe { crate::backend::blake512::blake512_init(&mut *s) }
}

/// `void blake512_compress(blakestate512 *S, const unsigned char *block)`
#[unsafe(no_mangle)]
pub extern "C" fn blake512_compress(s: *mut BlakeState512, block: *const u8) {
    unsafe {
        let block = core::slice::from_raw_parts(block, 128);
        crate::backend::blake512::blake512_compress(&mut *s, block)
    }
}

/// `void blake512_update(blakestate512 *S, const unsigned char *in, unsigned long long inlen)`
/// (`inlen` is a *bit* count, as in the C source.)
#[unsafe(no_mangle)]
pub extern "C" fn blake512_update(s: *mut BlakeState512, input: *const u8, inlen: u64) {
    unsafe {
        let input = core::slice::from_raw_parts(input, (inlen >> 3) as usize);
        crate::backend::blake512::blake512_update_bits(&mut *s, input, inlen)
    }
}

/// `void blake512_final(blakestate512 *S, unsigned char *out)`
#[unsafe(no_mangle)]
pub extern "C" fn blake512_final(s: *mut BlakeState512, out: *mut u8) {
    unsafe {
        let out = core::slice::from_raw_parts_mut(
            out,
            crate::backend::blake512::SPX_BLAKE512_OUTPUT_BYTES,
        );
        crate::backend::blake512::blake512_final(&mut *s, out)
    }
}

/// `void blake256_mgf1(unsigned char *out, unsigned long outlen,
///                     const unsigned char *in, unsigned long inlen)`
#[unsafe(no_mangle)]
pub extern "C" fn SPX_blake256_mgf1(out: *mut u8, outlen: u64, input: *const u8, inlen: u64) {
    unsafe {
        let out = core::slice::from_raw_parts_mut(out, outlen as usize);
        let input = core::slice::from_raw_parts(input, inlen as usize);
        crate::backend::blake256::blake256_mgf1(out, input)
    }
}

/// `void blake512_mgf1(unsigned char *out, unsigned long outlen,
///                     const unsigned char *in, unsigned long inlen)`
#[unsafe(no_mangle)]
pub extern "C" fn SPX_blake512_mgf1(out: *mut u8, outlen: u64, input: *const u8, inlen: u64) {
    unsafe {
        let out = core::slice::from_raw_parts_mut(out, outlen as usize);
        let input = core::slice::from_raw_parts(input, inlen as usize);
        crate::backend::blake512::blake512_mgf1(out, input)
    }
}

// --- app/include/hash.h (namespaced) ---------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    unsafe { initialize_hash_function(&mut *ctx) }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    unsafe {
        let out = core::slice::from_raw_parts_mut(out, SPX_N);
        prf_addr(out, &*ctx, &*(addr as *const [u32; 8]))
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_gen_message_random(
    r: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: u64,
    ctx: *const SpxCtx,
) {
    unsafe {
        let r = core::slice::from_raw_parts_mut(r, SPX_BLAKEX_OUTPUT_BYTES);
        let sk_prf = core::slice::from_raw_parts(sk_prf, SPX_N);
        let optrand = core::slice::from_raw_parts(optrand, SPX_N);
        let m = core::slice::from_raw_parts(m, mlen as usize);
        gen_message_random(r, sk_prf, optrand, m, &*ctx)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    r: *const u8,
    pk: *const u8,
    m: *const u8,
    mlen: u64,
    ctx: *const SpxCtx,
) {
    unsafe {
        let digest = core::slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES);
        let r = core::slice::from_raw_parts(r, SPX_N);
        let pk = core::slice::from_raw_parts(pk, SPX_PK_BYTES);
        let m = core::slice::from_raw_parts(m, mlen as usize);
        hash_message(digest, &mut *tree, &mut *leaf_idx, r, pk, m, &*ctx)
    }
}
