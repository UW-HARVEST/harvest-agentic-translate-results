// Translation of c_src/lib/haraka/src/hash_haraka.c

use core::slice;

use crate::context::SpxCtx;
use crate::params::{
    SPX_ADDR_BYTES, SPX_D, SPX_FORS_MSG_BYTES, SPX_N, SPX_PK_BYTES, SPX_TREE_HEIGHT,
};
use crate::utils::bytes_to_ull;

use super::haraka::{
    haraka512_inner, haraka_s_inc_absorb_inner, haraka_s_inc_finalize_inner,
    haraka_s_inc_init_inner, haraka_s_inc_squeeze_inner, tweak_constants_inner,
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    let ctx = unsafe { &mut *ctx };
    tweak_constants_inner(ctx);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let out = unsafe { slice::from_raw_parts_mut(out, SPX_N) };
    let ctx = unsafe { &*ctx };
    let addr = unsafe { slice::from_raw_parts(addr, 8) };
    prf_addr_inner(out, ctx, addr);
}

pub fn prf_addr_inner(out: &mut [u8], ctx: &SpxCtx, addr: &[u32]) {
    let mut outbuf = [0u8; 32];
    let mut buf = [0u8; 64];
    // Copy addr (32 bytes)
    let addr_bytes = unsafe { slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[..SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);
    buf[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);
    haraka512_inner(&mut outbuf, &buf, ctx);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_gen_message_random(
    r: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: u64,
    ctx: *const SpxCtx,
) {
    let r = unsafe { slice::from_raw_parts_mut(r, SPX_N) };
    let sk_prf = unsafe { slice::from_raw_parts(sk_prf, SPX_N) };
    let optrand = unsafe { slice::from_raw_parts(optrand, SPX_N) };
    let m = unsafe { slice::from_raw_parts(m, mlen as usize) };
    let ctx = unsafe { &*ctx };

    let mut s_inc = [0u8; 65];
    haraka_s_inc_init_inner(&mut s_inc);
    haraka_s_inc_absorb_inner(&mut s_inc, sk_prf, ctx);
    haraka_s_inc_absorb_inner(&mut s_inc, optrand, ctx);
    haraka_s_inc_absorb_inner(&mut s_inc, m, ctx);
    haraka_s_inc_finalize_inner(&mut s_inc);
    haraka_s_inc_squeeze_inner(r, &mut s_inc, ctx);
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
    ctx: *const SpxCtx,
) {
    const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

    let digest = unsafe { slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES) };
    let r_slice = unsafe { slice::from_raw_parts(r, SPX_N) };
    let pk_slice = unsafe { slice::from_raw_parts(pk, SPX_PK_BYTES) };
    let m = unsafe { slice::from_raw_parts(m, mlen as usize) };
    let ctx = unsafe { &*ctx };

    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u8; 65];

    haraka_s_inc_init_inner(&mut s_inc);
    haraka_s_inc_absorb_inner(&mut s_inc, r_slice, ctx);
    haraka_s_inc_absorb_inner(&mut s_inc, &pk_slice[SPX_N..2 * SPX_N], ctx);
    haraka_s_inc_absorb_inner(&mut s_inc, m, ctx);
    haraka_s_inc_finalize_inner(&mut s_inc);
    haraka_s_inc_squeeze_inner(&mut buf, &mut s_inc, ctx);

    digest.copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    let tree_val: u64 = if SPX_D == 1 {
        0
    } else {
        let v = bytes_to_ull(&buf[bufp..bufp + SPX_TREE_BYTES]);
        v & ((!0u64) >> (64 - SPX_TREE_BITS))
    };
    unsafe { *tree = tree_val; }
    bufp += SPX_TREE_BYTES;

    let leaf_idx_val =
        bytes_to_ull(&buf[bufp..bufp + SPX_LEAF_BYTES]) as u32 & ((!0u32) >> (32 - SPX_LEAF_BITS));
    unsafe { *leaf_idx = leaf_idx_val; }
}
