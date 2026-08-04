use crate::context::SpxCtx;
use crate::haraka::*;
use crate::params::*;
use crate::utils::bytes_to_ull_slice;

#[unsafe(no_mangle)]
pub extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    unsafe {
        tweak_constants_safe(&mut *ctx);
    }
}

pub fn initialize_hash_function_safe(ctx: &mut SpxCtx) {
    tweak_constants_safe(ctx);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    unsafe {
        let mut buf = [0u8; 64];
        // Copy address (32 bytes)
        let addr_bytes = std::slice::from_raw_parts(addr as *const u8, SPX_ADDR_BYTES);
        buf[..SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
        // Copy sk_seed (SPX_N)
        buf[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N]
            .copy_from_slice(&(*ctx).sk_seed);

        let mut outbuf = [0u8; 32];
        haraka512_safe(&mut outbuf, &buf, &*ctx);
        std::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
    }
}

pub fn prf_addr_safe(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = [0u8; 64];
    let addr_bytes: &[u8; 32] = unsafe { &*(addr.as_ptr() as *const [u8; 32]) };
    buf[..SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);

    let mut outbuf = [0u8; 32];
    haraka512_safe(&mut outbuf, &buf, ctx);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
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
        let mut s_inc = [0u8; 65];
        haraka_s_inc_init_safe(&mut s_inc);
        haraka_s_inc_absorb_safe(
            &mut s_inc,
            std::slice::from_raw_parts(sk_prf, SPX_N),
            &*ctx,
        );
        haraka_s_inc_absorb_safe(
            &mut s_inc,
            std::slice::from_raw_parts(optrand, SPX_N),
            &*ctx,
        );
        haraka_s_inc_absorb_safe(
            &mut s_inc,
            std::slice::from_raw_parts(m, mlen as usize),
            &*ctx,
        );
        haraka_s_inc_finalize_safe(&mut s_inc);
        let r_slice = std::slice::from_raw_parts_mut(r, SPX_N);
        haraka_s_inc_squeeze_safe(r_slice, &mut s_inc, &*ctx);
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
    const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

    unsafe {
        let mut buf = [0u8; SPX_DGST_BYTES];
        let mut s_inc = [0u8; 65];
        haraka_s_inc_init_safe(&mut s_inc);
        haraka_s_inc_absorb_safe(&mut s_inc, std::slice::from_raw_parts(r, SPX_N), &*ctx);
        // Only absorb root part of pk (skip first SPX_N bytes for haraka layout: pk = [PUB_SEED || root])
        haraka_s_inc_absorb_safe(
            &mut s_inc,
            std::slice::from_raw_parts(pk.add(SPX_N), SPX_N),
            &*ctx,
        );
        haraka_s_inc_absorb_safe(
            &mut s_inc,
            std::slice::from_raw_parts(m, mlen as usize),
            &*ctx,
        );
        haraka_s_inc_finalize_safe(&mut s_inc);
        haraka_s_inc_squeeze_safe(&mut buf, &mut s_inc, &*ctx);

        let mut bufp = 0usize;
        std::ptr::copy_nonoverlapping(buf.as_ptr(), digest, SPX_FORS_MSG_BYTES);
        bufp += SPX_FORS_MSG_BYTES;

        if SPX_D == 1 {
            *tree = 0;
        } else {
            let mut t = bytes_to_ull_slice(&buf[bufp..bufp + SPX_TREE_BYTES]);
            t &= (!0u64) >> (64 - SPX_TREE_BITS);
            *tree = t;
        }
        bufp += SPX_TREE_BYTES;

        let mut l = bytes_to_ull_slice(&buf[bufp..bufp + SPX_LEAF_BYTES]) as u32;
        l &= (!0u32) >> (32 - SPX_LEAF_BITS);
        *leaf_idx = l;
    }
}

pub fn gen_message_random_safe(
    r: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    ctx: &SpxCtx,
) {
    let mut s_inc = [0u8; 65];
    haraka_s_inc_init_safe(&mut s_inc);
    haraka_s_inc_absorb_safe(&mut s_inc, sk_prf, ctx);
    haraka_s_inc_absorb_safe(&mut s_inc, optrand, ctx);
    haraka_s_inc_absorb_safe(&mut s_inc, m, ctx);
    haraka_s_inc_finalize_safe(&mut s_inc);
    haraka_s_inc_squeeze_safe(&mut r[..SPX_N], &mut s_inc, ctx);
}

pub fn hash_message_safe(
    digest: &mut [u8; SPX_FORS_MSG_BYTES],
    tree: &mut u64,
    leaf_idx: &mut u32,
    r: &[u8],
    pk: &[u8],
    m: &[u8],
    ctx: &SpxCtx,
) {
    const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u8; 65];
    haraka_s_inc_init_safe(&mut s_inc);
    haraka_s_inc_absorb_safe(&mut s_inc, &r[..SPX_N], ctx);
    haraka_s_inc_absorb_safe(&mut s_inc, &pk[SPX_N..2 * SPX_N], ctx);
    haraka_s_inc_absorb_safe(&mut s_inc, m, ctx);
    haraka_s_inc_finalize_safe(&mut s_inc);
    haraka_s_inc_squeeze_safe(&mut buf, &mut s_inc, ctx);

    let mut bufp = 0;
    digest.copy_from_slice(&buf[bufp..bufp + SPX_FORS_MSG_BYTES]);
    bufp += SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        let mut t = bytes_to_ull_slice(&buf[bufp..bufp + SPX_TREE_BYTES]);
        t &= (!0u64) >> (64 - SPX_TREE_BITS);
        *tree = t;
    }
    bufp += SPX_TREE_BYTES;

    let mut l = bytes_to_ull_slice(&buf[bufp..bufp + SPX_LEAF_BYTES]) as u32;
    l &= (!0u32) >> (32 - SPX_LEAF_BITS);
    *leaf_idx = l;
}
