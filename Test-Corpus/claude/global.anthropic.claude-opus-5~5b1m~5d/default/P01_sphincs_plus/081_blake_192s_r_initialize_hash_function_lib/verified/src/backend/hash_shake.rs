//! Translation of `c_src/lib/shake/src/hash_shake.c`.

use crate::address::addr_bytes;
use crate::backend::fips202;
use crate::context::SpxCtx;
use crate::params::{
    SPX_ADDR_BYTES, SPX_D, SPX_FORS_MSG_BYTES, SPX_N, SPX_PK_BYTES, SPX_TREE_HEIGHT,
};
use crate::utils::bytes_to_ull;

/* For SHAKE256, there is no immediate reason to initialize at the start,
   so this function is an empty operation. */
pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    let _ = ctx; /* Suppress an 'unused parameter' warning. */
}

/*
 * Computes PRF(pk_seed, sk_seed, addr)
 */
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed[..SPX_N]);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_ADDR_BYTES]);
    buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + SPX_N]
        .copy_from_slice(&ctx.sk_seed[..SPX_N]);

    fips202::shake256(&mut out[..SPX_N], &buf[..2 * SPX_N + SPX_ADDR_BYTES]);
}

/**
 * Computes the message-dependent randomness R, using a secret seed and an
 * optional randomization value as well as the message.
 */
pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], ctx: &SpxCtx) {
    let _ = ctx;
    let mut s_inc = [0u64; 26];

    fips202::shake256_inc_init(&mut s_inc);
    fips202::shake256_inc_absorb(&mut s_inc, &sk_prf[..SPX_N]);
    fips202::shake256_inc_absorb(&mut s_inc, &optrand[..SPX_N]);
    fips202::shake256_inc_absorb(&mut s_inc, m);
    fips202::shake256_inc_finalize(&mut s_inc);
    fips202::shake256_inc_squeeze(&mut r[..SPX_N], &mut s_inc);
}

// #define SPX_TREE_BITS (SPX_TREE_HEIGHT * (SPX_D - 1))
const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
// #define SPX_TREE_BYTES ((SPX_TREE_BITS + 7) / 8)
const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
// #define SPX_LEAF_BITS SPX_TREE_HEIGHT
const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
// #define SPX_LEAF_BYTES ((SPX_LEAF_BITS + 7) / 8)
const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
// #define SPX_DGST_BYTES (SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES)
const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

/**
 * Computes the message hash using R, the public key, and the message.
 * Outputs the message digest and the index of the leaf. The index is split in
 * the tree index and the leaf index, for convenient copying to an address.
 */
pub fn hash_message(
    digest: &mut [u8],
    tree: &mut u64,
    leaf_idx: &mut u32,
    r: &[u8],
    pk: &[u8],
    m: &[u8],
    ctx: &SpxCtx,
) {
    let _ = ctx;

    // #if SPX_TREE_BITS > 64 -> #error
    // (statically satisfied for all supported parameter sets)

    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u64; 26];

    fips202::shake256_inc_init(&mut s_inc);
    fips202::shake256_inc_absorb(&mut s_inc, &r[..SPX_N]);
    fips202::shake256_inc_absorb(&mut s_inc, &pk[..SPX_PK_BYTES]);
    fips202::shake256_inc_absorb(&mut s_inc, m);
    fips202::shake256_inc_finalize(&mut s_inc);
    fips202::shake256_inc_squeeze(&mut buf[..SPX_DGST_BYTES], &mut s_inc);

    // unsigned char *bufp = buf;
    let mut bufp: usize = 0;

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[bufp..bufp + SPX_FORS_MSG_BYTES]);
    bufp += SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES as u32);
        // (~(uint64_t)0) >> (64 - SPX_TREE_BITS)
        *tree &= u64::MAX.wrapping_shr(64u32.wrapping_sub(SPX_TREE_BITS as u32));
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES as u32) as u32;
    // (~(uint32_t)0) >> (32 - SPX_LEAF_BITS)
    *leaf_idx &= u32::MAX.wrapping_shr(32u32.wrapping_sub(SPX_LEAF_BITS as u32));
}

// ---------------------------------------------------------------------------
// C ABI wrappers
//
// The functions declared in `lib/shake/include/fips202.h` are *not* run through
// `SPX_NAMESPACE`, so they keep their plain linker names.  The functions from
// `app/include/hash.h` are namespaced and therefore carry the `SPX_` prefix.
// ---------------------------------------------------------------------------

#[inline]
unsafe fn sl<'a>(p: *const u8, len: usize) -> &'a [u8] {
    if len == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(p, len) }
    }
}

#[inline]
unsafe fn sl_mut<'a>(p: *mut u8, len: usize) -> &'a mut [u8] {
    if len == 0 {
        &mut []
    } else {
        unsafe { core::slice::from_raw_parts_mut(p, len) }
    }
}

#[inline]
unsafe fn st<'a>(p: *mut u64, len: usize) -> &'a mut [u64] {
    unsafe { core::slice::from_raw_parts_mut(p, len) }
}

// --- fips202.h -------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn shake128_absorb(s: *mut u64, input: *const u8, inlen: usize) {
    unsafe { fips202::shake128_absorb(st(s, 25), sl(input, inlen)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn shake128_squeezeblocks(output: *mut u8, nblocks: usize, s: *mut u64) {
    unsafe {
        fips202::shake128_squeezeblocks(
            sl_mut(output, nblocks * fips202::SHAKE128_RATE),
            nblocks,
            st(s, 25),
        )
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn shake128_inc_init(s_inc: *mut u64) {
    unsafe { fips202::shake128_inc_init(st(s_inc, 26)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn shake128_inc_absorb(s_inc: *mut u64, input: *const u8, inlen: usize) {
    unsafe { fips202::shake128_inc_absorb(st(s_inc, 26), sl(input, inlen)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn shake128_inc_finalize(s_inc: *mut u64) {
    unsafe { fips202::shake128_inc_finalize(st(s_inc, 26)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn shake128_inc_squeeze(output: *mut u8, outlen: usize, s_inc: *mut u64) {
    unsafe { fips202::shake128_inc_squeeze(sl_mut(output, outlen), st(s_inc, 26)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn shake256_absorb(s: *mut u64, input: *const u8, inlen: usize) {
    unsafe { fips202::shake256_absorb(st(s, 25), sl(input, inlen)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn shake256_squeezeblocks(output: *mut u8, nblocks: usize, s: *mut u64) {
    unsafe {
        fips202::shake256_squeezeblocks(
            sl_mut(output, nblocks * fips202::SHAKE256_RATE),
            nblocks,
            st(s, 25),
        )
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn shake256_inc_init(s_inc: *mut u64) {
    unsafe { fips202::shake256_inc_init(st(s_inc, 26)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn shake256_inc_absorb(s_inc: *mut u64, input: *const u8, inlen: usize) {
    unsafe { fips202::shake256_inc_absorb(st(s_inc, 26), sl(input, inlen)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn shake256_inc_finalize(s_inc: *mut u64) {
    unsafe { fips202::shake256_inc_finalize(st(s_inc, 26)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn shake256_inc_squeeze(output: *mut u8, outlen: usize, s_inc: *mut u64) {
    unsafe { fips202::shake256_inc_squeeze(sl_mut(output, outlen), st(s_inc, 26)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn shake128(output: *mut u8, outlen: usize, input: *const u8, inlen: usize) {
    unsafe { fips202::shake128(sl_mut(output, outlen), sl(input, inlen)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn shake256(output: *mut u8, outlen: usize, input: *const u8, inlen: usize) {
    unsafe { fips202::shake256(sl_mut(output, outlen), sl(input, inlen)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha3_256_inc_init(s_inc: *mut u64) {
    unsafe { fips202::sha3_256_inc_init(st(s_inc, 26)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha3_256_inc_absorb(s_inc: *mut u64, input: *const u8, inlen: usize) {
    unsafe { fips202::sha3_256_inc_absorb(st(s_inc, 26), sl(input, inlen)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha3_256_inc_finalize(output: *mut u8, s_inc: *mut u64) {
    unsafe { fips202::sha3_256_inc_finalize(sl_mut(output, 32), st(s_inc, 26)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha3_256(output: *mut u8, input: *const u8, inlen: usize) {
    unsafe { fips202::sha3_256(sl_mut(output, 32), sl(input, inlen)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha3_512_inc_init(s_inc: *mut u64) {
    unsafe { fips202::sha3_512_inc_init(st(s_inc, 26)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha3_512_inc_absorb(s_inc: *mut u64, input: *const u8, inlen: usize) {
    unsafe { fips202::sha3_512_inc_absorb(st(s_inc, 26), sl(input, inlen)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha3_512_inc_finalize(output: *mut u8, s_inc: *mut u64) {
    unsafe { fips202::sha3_512_inc_finalize(sl_mut(output, 64), st(s_inc, 26)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha3_512(output: *mut u8, input: *const u8, inlen: usize) {
    unsafe { fips202::sha3_512(sl_mut(output, 64), sl(input, inlen)) }
}

// --- hash.h ---------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    unsafe { initialize_hash_function(&mut *ctx) }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    unsafe { prf_addr(sl_mut(out, SPX_N), &*ctx, &*(addr as *const [u32; 8])) }
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
        gen_message_random(
            sl_mut(r, SPX_N),
            sl(sk_prf, SPX_N),
            sl(optrand, SPX_N),
            sl(m, mlen as usize),
            &*ctx,
        )
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
        hash_message(
            sl_mut(digest, SPX_FORS_MSG_BYTES),
            &mut *tree,
            &mut *leaf_idx,
            sl(r, SPX_N),
            sl(pk, SPX_PK_BYTES),
            sl(m, mlen as usize),
            &*ctx,
        )
    }
}
