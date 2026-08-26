// Translation of c_src/lib/shake/src/hash_shake.c

use core::slice;

use crate::context::SpxCtx;
use crate::params::{
    SPX_ADDR_BYTES, SPX_D, SPX_FORS_MSG_BYTES, SPX_N, SPX_PK_BYTES, SPX_TREE_HEIGHT,
};
use crate::utils::bytes_to_ull;

use super::fips202::{
    shake256_inc_absorb_inner, shake256_inc_finalize_inner, shake256_inc_init_inner,
    shake256_inc_squeeze_inner, shake256_inner,
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(_ctx: *mut SpxCtx) {
    // no-op
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let out = unsafe { slice::from_raw_parts_mut(out, SPX_N) };
    let ctx = unsafe { &*ctx };
    let addr = unsafe { slice::from_raw_parts(addr, 8) };
    let mut buf = vec![0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe { slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);
    buf[SPX_N + SPX_ADDR_BYTES..2 * SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ctx.sk_seed);
    shake256_inner(out, &buf);
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
    let r = unsafe { slice::from_raw_parts_mut(r, SPX_N) };
    let sk_prf = unsafe { slice::from_raw_parts(sk_prf, SPX_N) };
    let optrand = unsafe { slice::from_raw_parts(optrand, SPX_N) };
    let m = unsafe { slice::from_raw_parts(m, mlen as usize) };

    let mut s_inc = [0u64; 26];
    shake256_inc_init_inner(&mut s_inc);
    shake256_inc_absorb_inner(&mut s_inc, sk_prf);
    shake256_inc_absorb_inner(&mut s_inc, optrand);
    shake256_inc_absorb_inner(&mut s_inc, m);
    shake256_inc_finalize_inner(&mut s_inc);
    shake256_inc_squeeze_inner(r, &mut s_inc);
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
    const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

    let digest = unsafe { slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES) };
    let r_slice = unsafe { slice::from_raw_parts(r, SPX_N) };
    let pk_slice = unsafe { slice::from_raw_parts(pk, SPX_PK_BYTES) };
    let m = unsafe { slice::from_raw_parts(m, mlen as usize) };

    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u64; 26];

    shake256_inc_init_inner(&mut s_inc);
    shake256_inc_absorb_inner(&mut s_inc, r_slice);
    shake256_inc_absorb_inner(&mut s_inc, pk_slice);
    shake256_inc_absorb_inner(&mut s_inc, m);
    shake256_inc_finalize_inner(&mut s_inc);
    shake256_inc_squeeze_inner(&mut buf, &mut s_inc);

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
