use crate::context::SpxCtx;
use crate::params::*;
use super::haraka::*;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    SPX_tweak_constants(ctx);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(
    out: *mut u8,
    ctx: *const SpxCtx,
    addr: *const u32,
) {
    let ctx = &*ctx;
    let mut outbuf = [0u8; 32];
    let mut buf = [0u8; 64];

    core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr(), SPX_ADDR_BYTES);
    core::ptr::copy_nonoverlapping(ctx.sk_seed.as_ptr(), buf.as_mut_ptr().add(SPX_ADDR_BYTES), SPX_N);

    SPX_haraka512(outbuf.as_mut_ptr(), buf.as_ptr(), ctx as *const SpxCtx);
    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
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
    let mut s_inc = [0u8; 65];

    SPX_haraka_S_inc_init(s_inc.as_mut_ptr());
    SPX_haraka_S_inc_absorb(s_inc.as_mut_ptr(), sk_prf, SPX_N, ctx);
    SPX_haraka_S_inc_absorb(s_inc.as_mut_ptr(), optrand, SPX_N, ctx);
    SPX_haraka_S_inc_absorb(s_inc.as_mut_ptr(), m, mlen as usize, ctx);
    SPX_haraka_S_inc_finalize(s_inc.as_mut_ptr());
    SPX_haraka_S_inc_squeeze(r, SPX_N, s_inc.as_mut_ptr(), ctx);
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
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u8; 65];

    SPX_haraka_S_inc_init(s_inc.as_mut_ptr());
    SPX_haraka_S_inc_absorb(s_inc.as_mut_ptr(), r, SPX_N, ctx);
    SPX_haraka_S_inc_absorb(s_inc.as_mut_ptr(), pk.add(SPX_N), SPX_N, ctx);
    SPX_haraka_S_inc_absorb(s_inc.as_mut_ptr(), m, mlen as usize, ctx);
    SPX_haraka_S_inc_finalize(s_inc.as_mut_ptr());
    SPX_haraka_S_inc_squeeze(buf.as_mut_ptr(), SPX_DGST_BYTES, s_inc.as_mut_ptr(), ctx);

    core::ptr::copy_nonoverlapping(buf.as_ptr(), digest, SPX_FORS_MSG_BYTES);
    let mut bufp = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = crate::utils::bytes_to_ull(buf.as_ptr().add(bufp), SPX_TREE_BYTES as u32);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = crate::utils::bytes_to_ull(buf.as_ptr().add(bufp), SPX_LEAF_BYTES as u32) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
