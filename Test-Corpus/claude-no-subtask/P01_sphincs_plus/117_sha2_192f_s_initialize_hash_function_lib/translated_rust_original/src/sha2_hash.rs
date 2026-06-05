// SHA-2 hash implementation (PRF, message hash)
#![cfg(feature = "sha2")]
#![allow(dead_code)]

use crate::context::SpxCtx;
use crate::params::*;
use crate::sha2::*;
use crate::utils::bytes_to_ull;

const SPX_SHA256_ADDR_BYTES: usize = 22;

#[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
const SPX_SHAX_OUTPUT_BYTES: usize = SPX_SHA512_OUTPUT_BYTES;
#[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
const SPX_SHAX_BLOCK_BYTES: usize = SPX_SHA512_BLOCK_BYTES;

#[cfg(any(feature = "128f", feature = "128s"))]
const SPX_SHAX_OUTPUT_BYTES: usize = SPX_SHA256_OUTPUT_BYTES;
#[cfg(any(feature = "128f", feature = "128s"))]
const SPX_SHAX_BLOCK_BYTES: usize = SPX_SHA256_BLOCK_BYTES;

#[cfg(any(feature = "128f", feature = "128s"))]
const SHAX_STATE_LEN: usize = 40;
#[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
const SHAX_STATE_LEN: usize = 72;

#[inline]
fn shax_inc_init(state: &mut [u8]) {
    #[cfg(any(feature = "128f", feature = "128s"))]
    sha256_inc_init(state);
    #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
    sha512_inc_init(state);
}

#[inline]
fn shax_inc_blocks(state: &mut [u8], input: &[u8], inblocks: usize) {
    #[cfg(any(feature = "128f", feature = "128s"))]
    sha256_inc_blocks(state, input, inblocks);
    #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
    sha512_inc_blocks(state, input, inblocks);
}

#[inline]
fn shax_inc_finalize(out: &mut [u8], state: &mut [u8], input: &[u8], inlen: usize) {
    #[cfg(any(feature = "128f", feature = "128s"))]
    sha256_inc_finalize(out, state, input, inlen);
    #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
    sha512_inc_finalize(out, state, input, inlen);
}

#[inline]
fn shax(out: &mut [u8], input: &[u8]) {
    #[cfg(any(feature = "128f", feature = "128s"))]
    sha256(out, input);
    #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
    sha512(out, input);
}

#[inline]
fn mgf1_x(out: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    #[cfg(any(feature = "128f", feature = "128s"))]
    mgf1_256(out, outlen, input, inlen);
    #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
    mgf1_512(out, outlen, input, inlen);
}

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    seed_state(ctx);
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    sha2_state.copy_from_slice(&ctx.state_seeded);

    let addr_bytes: &[u8; 32] = unsafe { &*(addr.as_ptr() as *const [u8; 32]) };
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);

    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

pub fn gen_message_random(
    r: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    mlen: u64,
    _ctx: &SpxCtx,
) {
    let mut buf = vec![0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    let mut state = vec![0u8; 8 + SPX_SHAX_OUTPUT_BYTES]; // 40 or 72

    // HMAC inner
    for i in 0..SPX_N {
        buf[i] = 0x36 ^ sk_prf[i];
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x36;
    }
    shax_inc_init(&mut state);
    shax_inc_blocks(&mut state, &buf, 1);

    let mlen = mlen as usize;
    buf[..SPX_N].copy_from_slice(&optrand[..SPX_N]);

    if SPX_N + mlen < SPX_SHAX_BLOCK_BYTES {
        buf[SPX_N..SPX_N + mlen].copy_from_slice(&m[..mlen]);
        let in_data = buf[..SPX_N + mlen].to_vec();
        shax_inc_finalize(
            &mut buf[SPX_SHAX_BLOCK_BYTES..],
            &mut state,
            &in_data,
            mlen + SPX_N,
        );
    } else {
        buf[SPX_N..SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..SPX_SHAX_BLOCK_BYTES - SPX_N]);
        let buf_copy = buf[..SPX_SHAX_BLOCK_BYTES].to_vec();
        shax_inc_blocks(&mut state, &buf_copy, 1);

        let m_off = SPX_SHAX_BLOCK_BYTES - SPX_N;
        let m_remain = mlen - (SPX_SHAX_BLOCK_BYTES - SPX_N);
        shax_inc_finalize(
            &mut buf[SPX_SHAX_BLOCK_BYTES..],
            &mut state,
            &m[m_off..m_off + m_remain],
            m_remain,
        );
    }

    // HMAC outer
    for i in 0..SPX_N {
        buf[i] = 0x5c ^ sk_prf[i];
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x5c;
    }
    let buf_clone = buf.clone();
    shax(&mut buf, &buf_clone);
    r[..SPX_N].copy_from_slice(&buf[..SPX_N]);
}

pub fn hash_message(
    digest: &mut [u8],
    tree: &mut u64,
    leaf_idx: &mut u32,
    r: &[u8],
    pk: &[u8],
    m: &[u8],
    mlen: u64,
    _ctx: &SpxCtx,
) {
    const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

    let mut seed = vec![0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES];
    let spx_inblocks =
        ((SPX_N + SPX_PK_BYTES + SPX_SHAX_BLOCK_BYTES - 1) & !(SPX_SHAX_BLOCK_BYTES - 1))
            / SPX_SHAX_BLOCK_BYTES;
    let mut inbuf = vec![0u8; spx_inblocks * SPX_SHAX_BLOCK_BYTES];

    let mut buf = vec![0u8; SPX_DGST_BYTES];
    let mut state = vec![0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    shax_inc_init(&mut state);

    inbuf[..SPX_N].copy_from_slice(&r[..SPX_N]);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    let mlen = mlen as usize;

    if SPX_N + SPX_PK_BYTES + mlen < spx_inblocks * SPX_SHAX_BLOCK_BYTES {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen].copy_from_slice(&m[..mlen]);
        let in_data = inbuf[..SPX_N + SPX_PK_BYTES + mlen].to_vec();
        shax_inc_finalize(
            &mut seed[2 * SPX_N..],
            &mut state,
            &in_data,
            SPX_N + SPX_PK_BYTES + mlen,
        );
    } else {
        let take = spx_inblocks * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + take].copy_from_slice(&m[..take]);
        let in_blocks_copy = inbuf.clone();
        shax_inc_blocks(&mut state, &in_blocks_copy, spx_inblocks);

        let m_off = take;
        let m_remain = mlen - take;
        shax_inc_finalize(
            &mut seed[2 * SPX_N..],
            &mut state,
            &m[m_off..m_off + m_remain],
            m_remain,
        );
    }

    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    mgf1_x(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let bufp_off = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[bufp_off..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }

    let bufp_off = bufp_off + SPX_TREE_BYTES;
    *leaf_idx = bytes_to_ull(&buf[bufp_off..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
