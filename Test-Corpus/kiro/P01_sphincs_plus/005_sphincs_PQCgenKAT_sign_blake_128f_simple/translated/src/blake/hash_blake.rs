use crate::context::SpxCtx;
use crate::params::*;
use crate::blake::blake256::*;
use crate::blake::blake512::*;

// blakeX selection: N>=24 uses blake512, N<24 uses blake256
#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
const BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE512_OUTPUT_BYTES;
#[cfg(any(feature = "128s", feature = "128f"))]
const BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE256_OUTPUT_BYTES;

#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
unsafe fn blakex(out: *mut u8, inp: *const u8, inlen: u64) {
    blake512(out, inp, inlen);
}
#[cfg(any(feature = "128s", feature = "128f"))]
unsafe fn blakex(out: *mut u8, inp: *const u8, inlen: u64) {
    blake256(out, inp, inlen);
}

#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
unsafe fn blakex_init(s: *mut Blakestate512) { blake512_init(s); }
#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
unsafe fn blakex_update(s: *mut Blakestate512, data: *const u8, datalen: u64) { blake512_update(s, data, datalen); }
#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
unsafe fn blakex_final(s: *mut Blakestate512, digest: *mut u8) { blake512_final(s, digest); }
#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
unsafe fn blakex_mgf1(out: *mut u8, outlen: usize, inp: *const u8, inlen: usize) {
    SPX_blake512_mgf1(out, outlen as u64, inp, inlen as u64);
}

#[cfg(any(feature = "128s", feature = "128f"))]
unsafe fn blakex_init(s: *mut Blakestate256) { blake256_init(s); }
#[cfg(any(feature = "128s", feature = "128f"))]
unsafe fn blakex_update(s: *mut Blakestate256, data: *const u8, datalen: u64) { blake256_update(s, data, datalen); }
#[cfg(any(feature = "128s", feature = "128f"))]
unsafe fn blakex_final(s: *mut Blakestate256, digest: *mut u8) { blake256_final(s, digest); }
#[cfg(any(feature = "128s", feature = "128f"))]
unsafe fn blakex_mgf1(out: *mut u8, outlen: usize, inp: *const u8, inlen: usize) {
    SPX_blake256_mgf1(out, outlen as u64, inp, inlen as u64);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(_ctx: *mut SpxCtx) {
    // no-op for BLAKE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(
    out: *mut u8, ctx: *const SpxCtx, addr: *const u32,
) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    std::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    std::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);
    std::ptr::copy_nonoverlapping((*ctx).sk_seed.as_ptr(), buf.as_mut_ptr().add(SPX_N + SPX_ADDR_BYTES), SPX_N);

    blake256(outbuf.as_mut_ptr(), buf.as_ptr(), (SPX_N + SPX_ADDR_BYTES) as u64);
    std::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
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
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        let mut state = std::mem::MaybeUninit::<Blakestate512>::uninit();
        let s = state.as_mut_ptr();
        blakex_init(s);
        blakex_update(s, sk_prf, (SPX_N as u64) * 8);
        blakex_update(s, optrand, (SPX_N as u64) * 8);
        blakex_update(s, m, mlen * 8);
        blakex_final(s, r);
    }
    #[cfg(any(feature = "128s", feature = "128f"))]
    {
        let mut state = std::mem::MaybeUninit::<Blakestate256>::uninit();
        let s = state.as_mut_ptr();
        blakex_init(s);
        blakex_update(s, sk_prf, (SPX_N as u64) * 8);
        blakex_update(s, optrand, (SPX_N as u64) * 8);
        blakex_update(s, m, mlen * 8);
        blakex_final(s, r);
    }
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
    let mut seed = [0u8; 2 * SPX_N + BLAKEX_OUTPUT_BYTES];

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        let mut state = std::mem::MaybeUninit::<Blakestate512>::uninit();
        let s = state.as_mut_ptr();
        blakex_init(s);
        blakex_update(s, r, (SPX_N as u64) * 8);
        blakex_update(s, pk, (SPX_PK_BYTES as u64) * 8);
        blakex_update(s, m, mlen * 8);
        blakex_final(s, seed.as_mut_ptr().add(2 * SPX_N));
    }
    #[cfg(any(feature = "128s", feature = "128f"))]
    {
        let mut state = std::mem::MaybeUninit::<Blakestate256>::uninit();
        let s = state.as_mut_ptr();
        blakex_init(s);
        blakex_update(s, r, (SPX_N as u64) * 8);
        blakex_update(s, pk, (SPX_PK_BYTES as u64) * 8);
        blakex_update(s, m, mlen * 8);
        blakex_final(s, seed.as_mut_ptr().add(2 * SPX_N));
    }

    std::ptr::copy_nonoverlapping(r, seed.as_mut_ptr(), SPX_N);
    std::ptr::copy_nonoverlapping(pk, seed.as_mut_ptr().add(SPX_N), SPX_N);

    blakex_mgf1(buf.as_mut_ptr(), SPX_DGST_BYTES, seed.as_ptr(), 2 * SPX_N + BLAKEX_OUTPUT_BYTES);

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
