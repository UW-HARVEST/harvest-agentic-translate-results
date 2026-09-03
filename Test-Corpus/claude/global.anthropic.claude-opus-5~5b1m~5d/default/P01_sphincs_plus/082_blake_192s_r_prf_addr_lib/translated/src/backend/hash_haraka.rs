//! Translation of `c_src/lib/haraka/src/hash_haraka.c`

use crate::address::addr_bytes;
use crate::backend::haraka::{
    haraka512, haraka_S_inc_absorb, haraka_S_inc_finalize, haraka_S_inc_init,
    haraka_S_inc_squeeze, tweak_constants,
};
use crate::context::SpxCtx;
use crate::params::{
    SPX_ADDR_BYTES, SPX_D, SPX_FORS_MSG_BYTES, SPX_N, SPX_TREE_HEIGHT,
};
use crate::utils::bytes_to_ull;

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    tweak_constants(ctx);
}

/// Computes PRF(key, addr), given a secret key of SPX_N bytes and an address
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    /* Since SPX_N may be smaller than 32, we need temporary buffers. */
    let mut outbuf = [0u8; 32];
    let mut buf = [0u8; 64];

    buf[..SPX_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_ADDR_BYTES]);
    buf[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed[..SPX_N]);

    haraka512(&mut outbuf, &buf, ctx);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

/// Computes the message-dependent randomness R, using a secret seed and an
/// optional randomization value as well as the message.
pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], ctx: &SpxCtx) {
    let mut s_inc = [0u8; 65];

    haraka_S_inc_init(&mut s_inc);
    haraka_S_inc_absorb(&mut s_inc, &sk_prf[..SPX_N], ctx);
    haraka_S_inc_absorb(&mut s_inc, &optrand[..SPX_N], ctx);
    haraka_S_inc_absorb(&mut s_inc, m, ctx);
    haraka_S_inc_finalize(&mut s_inc);
    haraka_S_inc_squeeze(&mut r[..SPX_N], &mut s_inc, ctx);
}

const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

/// `(~(uint64_t)0) >> (64 - SPX_TREE_BITS)`; the `SPX_TREE_BITS == 0` arm is
/// only there to keep the (dead, `SPX_D == 1`) shift from being rejected by
/// rustc's constant-shift-overflow lint.
const SPX_TREE_MASK: u64 = if SPX_TREE_BITS == 0 {
    0
} else {
    (!0u64) >> (64 - SPX_TREE_BITS)
};
/// `(~(uint32_t)0) >> (32 - SPX_LEAF_BITS)`
const SPX_LEAF_MASK: u32 = if SPX_LEAF_BITS == 0 {
    0
} else {
    (!0u32) >> (32 - SPX_LEAF_BITS)
};

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
    ctx: &SpxCtx,
) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u8; 65];

    haraka_S_inc_init(&mut s_inc);
    haraka_S_inc_absorb(&mut s_inc, &r[..SPX_N], ctx);
    // Only absorb root part of pk
    haraka_S_inc_absorb(&mut s_inc, &pk[SPX_N..SPX_N + SPX_N], ctx);
    haraka_S_inc_absorb(&mut s_inc, m, ctx);
    haraka_S_inc_finalize(&mut s_inc);
    haraka_S_inc_squeeze(&mut buf[..SPX_DGST_BYTES], &mut s_inc, ctx);

    let mut bufp: usize = 0;

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[bufp..bufp + SPX_FORS_MSG_BYTES]);
    bufp += SPX_FORS_MSG_BYTES;

    // #if SPX_TREE_BITS > 64
    //     #error For given height and depth, 64 bits cannot represent all subtrees
    // #endif
    const _: () = assert!(SPX_TREE_BITS <= 64);

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES as u32);
        *tree &= SPX_TREE_MASK;
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES as u32) as u32;
    *leaf_idx &= SPX_LEAF_MASK;
}

// ---------------------------------------------------------------------------
// C ABI wrappers (app/include/hash.h)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    unsafe { initialize_hash_function(&mut *ctx) }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    unsafe {
        prf_addr(
            core::slice::from_raw_parts_mut(out, SPX_N),
            &*ctx,
            &*(addr as *const [u32; 8]),
        )
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
        gen_message_random(
            core::slice::from_raw_parts_mut(r, SPX_N),
            core::slice::from_raw_parts(sk_prf, SPX_N),
            core::slice::from_raw_parts(optrand, SPX_N),
            core::slice::from_raw_parts(m, mlen as usize),
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
            core::slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES),
            &mut *tree,
            &mut *leaf_idx,
            core::slice::from_raw_parts(r, SPX_N),
            core::slice::from_raw_parts(pk, 2 * SPX_N),
            core::slice::from_raw_parts(m, mlen as usize),
            &*ctx,
        )
    }
}
