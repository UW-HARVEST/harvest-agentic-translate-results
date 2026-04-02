#![allow(non_snake_case, unused_variables)]

use crate::context::SpxCtx;
use crate::params::*;
use crate::sha2::sha2::*;

// shaX dispatch: for N>=24 use sha512 variants, else sha256
#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
const SPX_SHAX_OUTPUT_BYTES: usize = SPX_SHA512_OUTPUT_BYTES;
#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
const SPX_SHAX_BLOCK_BYTES: usize = SPX_SHA512_BLOCK_BYTES;

#[cfg(any(feature = "128s", feature = "128f"))]
const SPX_SHAX_OUTPUT_BYTES: usize = SPX_SHA256_OUTPUT_BYTES;
#[cfg(any(feature = "128s", feature = "128f"))]
const SPX_SHAX_BLOCK_BYTES: usize = SPX_SHA256_BLOCK_BYTES;

unsafe fn shax_inc_init(state: *mut u8) {
    if SPX_N >= 24 { sha512_inc_init(state); } else { sha256_inc_init(state); }
}
unsafe fn shax_inc_blocks(state: *mut u8, inp: *const u8, inblocks: usize) {
    if SPX_N >= 24 { sha512_inc_blocks(state, inp, inblocks); } else { sha256_inc_blocks(state, inp, inblocks); }
}
unsafe fn shax_inc_finalize(out: *mut u8, state: *mut u8, inp: *const u8, inlen: usize) {
    if SPX_N >= 24 { sha512_inc_finalize(out, state, inp, inlen); } else { sha256_inc_finalize(out, state, inp, inlen); }
}
unsafe fn shax(out: *mut u8, inp: *const u8, inlen: usize) {
    if SPX_N >= 24 { sha512(out, inp, inlen); } else { sha256(out, inp, inlen); }
}
unsafe fn mgf1_x(out: *mut u8, outlen: u64, inp: *const u8, inlen: u64) {
    if SPX_N >= 24 { SPX_mgf1_512(out, outlen, inp, inlen); } else { SPX_mgf1_256(out, outlen, inp, inlen); }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    SPX_seed_state(ctx);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let mut sha2_state = [0u8; 40];
    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    core::ptr::copy_nonoverlapping((*ctx).state_seeded.as_ptr(), sha2_state.as_mut_ptr(), 40);
    core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr(), SPX_SHA256_ADDR_BYTES);
    core::ptr::copy_nonoverlapping((*ctx).sk_seed.as_ptr(), buf.as_mut_ptr().add(SPX_SHA256_ADDR_BYTES), SPX_N);

    sha256_inc_finalize(outbuf.as_mut_ptr(), sha2_state.as_mut_ptr(), buf.as_ptr(), SPX_SHA256_ADDR_BYTES + SPX_N);
    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_gen_message_random(
    R: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: u64,
    ctx: *const SpxCtx,
) {
    let mlen = mlen as usize;
    let mut buf = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    // HMAC-SHA inner: ipad XOR sk_prf
    for i in 0..SPX_N {
        buf[i] = 0x36 ^ *sk_prf.add(i);
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x36;
    }

    shax_inc_init(state.as_mut_ptr());
    shax_inc_blocks(state.as_mut_ptr(), buf.as_ptr(), 1);

    core::ptr::copy_nonoverlapping(optrand, buf.as_mut_ptr(), SPX_N);

    if SPX_N + mlen < SPX_SHAX_BLOCK_BYTES {
        core::ptr::copy_nonoverlapping(m, buf.as_mut_ptr().add(SPX_N), mlen);
        shax_inc_finalize(
            buf.as_mut_ptr().add(SPX_SHAX_BLOCK_BYTES),
            state.as_mut_ptr(),
            buf.as_ptr(),
            mlen + SPX_N,
        );
    } else {
        core::ptr::copy_nonoverlapping(m, buf.as_mut_ptr().add(SPX_N), SPX_SHAX_BLOCK_BYTES - SPX_N);
        shax_inc_blocks(state.as_mut_ptr(), buf.as_ptr(), 1);

        let m2 = m.add(SPX_SHAX_BLOCK_BYTES - SPX_N);
        let mlen2 = mlen - (SPX_SHAX_BLOCK_BYTES - SPX_N);
        shax_inc_finalize(
            buf.as_mut_ptr().add(SPX_SHAX_BLOCK_BYTES),
            state.as_mut_ptr(),
            m2,
            mlen2,
        );
    }

    // HMAC outer: opad XOR sk_prf
    for i in 0..SPX_N {
        buf[i] = 0x5c ^ *sk_prf.add(i);
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x5c;
    }

    shax(buf.as_mut_ptr(), buf.as_ptr(), SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES);
    core::ptr::copy_nonoverlapping(buf.as_ptr(), R, SPX_N);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    R: *const u8,
    pk: *const u8,
    m: *const u8,
    mlen: u64,
    ctx: *const SpxCtx,
) {
    let mlen = mlen as usize;
    let mut seed = [0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES];

    // SPX_INBLOCKS: round (SPX_N + SPX_PK_BYTES) up to multiple of SPX_SHAX_BLOCK_BYTES
    const SPX_INBLOCKS: usize = (SPX_N + SPX_PK_BYTES + SPX_SHAX_BLOCK_BYTES - 1) / SPX_SHAX_BLOCK_BYTES;
    let mut inbuf = [0u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    shax_inc_init(state.as_mut_ptr());

    core::ptr::copy_nonoverlapping(R, inbuf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(pk, inbuf.as_mut_ptr().add(SPX_N), SPX_PK_BYTES);

    if SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES {
        core::ptr::copy_nonoverlapping(m, inbuf.as_mut_ptr().add(SPX_N + SPX_PK_BYTES), mlen);
        shax_inc_finalize(
            seed.as_mut_ptr().add(2 * SPX_N),
            state.as_mut_ptr(),
            inbuf.as_ptr(),
            SPX_N + SPX_PK_BYTES + mlen,
        );
    } else {
        core::ptr::copy_nonoverlapping(
            m,
            inbuf.as_mut_ptr().add(SPX_N + SPX_PK_BYTES),
            SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES,
        );
        shax_inc_blocks(state.as_mut_ptr(), inbuf.as_ptr(), SPX_INBLOCKS);

        let m2 = m.add(SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES);
        let mlen2 = mlen - (SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES);
        shax_inc_finalize(
            seed.as_mut_ptr().add(2 * SPX_N),
            state.as_mut_ptr(),
            m2,
            mlen2,
        );
    }

    // H_msg: MGF1-SHA-X(R || PK.seed || seed)
    core::ptr::copy_nonoverlapping(R, seed.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(pk, seed.as_mut_ptr().add(SPX_N), SPX_N);

    mgf1_x(
        buf.as_mut_ptr(),
        SPX_DGST_BYTES as u64,
        seed.as_ptr(),
        (2 * SPX_N + SPX_SHAX_OUTPUT_BYTES) as u64,
    );

    core::ptr::copy_nonoverlapping(buf.as_ptr(), digest, SPX_FORS_MSG_BYTES);

    let bufp = buf.as_ptr().add(SPX_FORS_MSG_BYTES);

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = crate::utils::bytes_to_ull(bufp, SPX_TREE_BYTES as u32);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }

    let bufp = bufp.add(SPX_TREE_BYTES);
    *leaf_idx = crate::utils::bytes_to_ull(bufp, SPX_LEAF_BYTES as u32) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
