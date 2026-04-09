use crate::context::SpxCtx;
use crate::params::*;
use crate::shake::fips202::*;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(_ctx: *mut SpxCtx) {
    // No-op for SHAKE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(
    out: *mut u8,
    ctx: *const SpxCtx,
    addr: *const u32,
) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];

    std::ptr::copy_nonoverlapping(
        (*ctx).pub_seed.as_ptr(),
        buf.as_mut_ptr(),
        SPX_N,
    );
    std::ptr::copy_nonoverlapping(
        addr as *const u8,
        buf.as_mut_ptr().add(SPX_N),
        SPX_ADDR_BYTES,
    );
    std::ptr::copy_nonoverlapping(
        (*ctx).sk_seed.as_ptr(),
        buf.as_mut_ptr().add(SPX_N + SPX_ADDR_BYTES),
        SPX_N,
    );

    shake256(out, SPX_N, buf.as_ptr(), 2 * SPX_N + SPX_ADDR_BYTES);
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
    let mut s_inc = [0u64; 26];

    shake256_inc_init(s_inc.as_mut_ptr());
    shake256_inc_absorb(s_inc.as_mut_ptr(), sk_prf, SPX_N);
    shake256_inc_absorb(s_inc.as_mut_ptr(), optrand, SPX_N);
    shake256_inc_absorb(s_inc.as_mut_ptr(), m, mlen as usize);
    shake256_inc_finalize(s_inc.as_mut_ptr());
    shake256_inc_squeeze(r, SPX_N, s_inc.as_mut_ptr());
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

    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u64; 26];

    shake256_inc_init(s_inc.as_mut_ptr());
    shake256_inc_absorb(s_inc.as_mut_ptr(), r, SPX_N);
    shake256_inc_absorb(s_inc.as_mut_ptr(), pk, SPX_PK_BYTES);
    shake256_inc_absorb(s_inc.as_mut_ptr(), m, mlen as usize);
    shake256_inc_finalize(s_inc.as_mut_ptr());
    shake256_inc_squeeze(buf.as_mut_ptr(), SPX_DGST_BYTES, s_inc.as_mut_ptr());

    std::ptr::copy_nonoverlapping(buf.as_ptr(), digest, SPX_FORS_MSG_BYTES);
    let mut bufp = buf.as_ptr().add(SPX_FORS_MSG_BYTES);

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = crate::utils::bytes_to_ull(bufp, SPX_TREE_BYTES as u32);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp = bufp.add(SPX_TREE_BYTES);

    *leaf_idx = crate::utils::bytes_to_ull(bufp, SPX_LEAF_BYTES as u32) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
