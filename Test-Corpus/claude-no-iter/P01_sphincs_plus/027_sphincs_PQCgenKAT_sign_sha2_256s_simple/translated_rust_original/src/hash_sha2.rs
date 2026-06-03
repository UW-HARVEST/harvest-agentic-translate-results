// Translation of c_src/lib/sha2/src/hash_sha2.c

use crate::context::SpxCtx;
use crate::params::{
    SPX_D, SPX_FORS_MSG_BYTES, SPX_N, SPX_PK_BYTES, SPX_TREE_HEIGHT,
};
use crate::sha2::{
    seed_state, sha256_inc_finalize, SPX_SHA256_ADDR_BYTES, SPX_SHA256_OUTPUT_BYTES,
};
use crate::utils::bytes_to_ull;

#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
use crate::sha2::{
    mgf1_512, sha512, sha512_inc_blocks, sha512_inc_finalize, sha512_inc_init,
    SPX_SHA512_BLOCK_BYTES, SPX_SHA512_OUTPUT_BYTES,
};
#[cfg(any(feature = "128s", feature = "128f"))]
use crate::sha2::{
    mgf1_256, sha256, sha256_inc_blocks, sha256_inc_finalize as _shaX_inc_finalize,
    sha256_inc_init, SPX_SHA256_BLOCK_BYTES,
};

// shaX_*: SHA-512 if N >= 24, else SHA-256
#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
const SHAX_OUTPUT_BYTES: usize = SPX_SHA512_OUTPUT_BYTES;
#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
const SHAX_BLOCK_BYTES: usize = SPX_SHA512_BLOCK_BYTES;
#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
const SHAX_STATE_LEN: usize = 72;

#[cfg(any(feature = "128s", feature = "128f"))]
const SHAX_OUTPUT_BYTES: usize = SPX_SHA256_OUTPUT_BYTES;
#[cfg(any(feature = "128s", feature = "128f"))]
const SHAX_BLOCK_BYTES: usize = SPX_SHA256_BLOCK_BYTES;
#[cfg(any(feature = "128s", feature = "128f"))]
const SHAX_STATE_LEN: usize = 40;

fn shax_inc_init(state: &mut [u8]) {
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    sha512_inc_init(state);
    #[cfg(any(feature = "128s", feature = "128f"))]
    sha256_inc_init(state);
}

fn shax_inc_blocks(state: &mut [u8], in_buf: &[u8], inblocks: usize) {
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    sha512_inc_blocks(state, in_buf, inblocks);
    #[cfg(any(feature = "128s", feature = "128f"))]
    sha256_inc_blocks(state, in_buf, inblocks);
}

fn shax_inc_finalize(out: &mut [u8], state: &mut [u8], in_buf: &[u8], inlen: usize) {
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    sha512_inc_finalize(out, state, in_buf, inlen);
    #[cfg(any(feature = "128s", feature = "128f"))]
    _shaX_inc_finalize(out, state, in_buf, inlen);
}

fn shax(out: &mut [u8], in_buf: &[u8], inlen: usize) {
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    sha512(out, in_buf, inlen);
    #[cfg(any(feature = "128s", feature = "128f"))]
    sha256(out, in_buf, inlen);
}

fn mgf1_x(out: &mut [u8], outlen: usize, in_buf: &[u8], inlen: usize) {
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    mgf1_512(out, outlen, in_buf, inlen);
    #[cfg(any(feature = "128s", feature = "128f"))]
    mgf1_256(out, outlen, in_buf, inlen);
}

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    seed_state(ctx);
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    sha2_state.copy_from_slice(&ctx.state_seeded);

    let addr_bytes =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);

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
    let mut buf = vec![0u8; SHAX_BLOCK_BYTES + SHAX_OUTPUT_BYTES];
    let mut state = vec![0u8; SHAX_STATE_LEN];

    for i in 0..SPX_N {
        buf[i] = 0x36 ^ sk_prf[i];
    }
    for i in SPX_N..SHAX_BLOCK_BYTES {
        buf[i] = 0x36;
    }

    shax_inc_init(&mut state);
    {
        let block = buf[..SHAX_BLOCK_BYTES].to_vec();
        shax_inc_blocks(&mut state, &block, 1);
    }

    buf[..SPX_N].copy_from_slice(&optrand[..SPX_N]);

    let mut m_off = 0usize;
    let mut mlen = mlen as usize;

    if SPX_N + mlen < SHAX_BLOCK_BYTES {
        buf[SPX_N..SPX_N + mlen].copy_from_slice(&m[..mlen]);
        let mlen_total = mlen + SPX_N;
        let in_data = buf[..mlen_total].to_vec();
        shax_inc_finalize(
            &mut buf[SHAX_BLOCK_BYTES..],
            &mut state,
            &in_data,
            mlen_total,
        );
    } else {
        let take = SHAX_BLOCK_BYTES - SPX_N;
        buf[SPX_N..SPX_N + take].copy_from_slice(&m[..take]);
        let block = buf[..SHAX_BLOCK_BYTES].to_vec();
        shax_inc_blocks(&mut state, &block, 1);
        m_off += take;
        mlen -= take;
        shax_inc_finalize(&mut buf[SHAX_BLOCK_BYTES..], &mut state, &m[m_off..], mlen);
    }

    for i in 0..SPX_N {
        buf[i] = 0x5c ^ sk_prf[i];
    }
    for i in SPX_N..SHAX_BLOCK_BYTES {
        buf[i] = 0x5c;
    }

    let buf_in = buf.clone();
    shax(&mut buf, &buf_in, SHAX_BLOCK_BYTES + SHAX_OUTPUT_BYTES);
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
    const SPX_TREE_BITS_VAL: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS_VAL + 7) / 8;
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

    let inblocks =
        ((SPX_N + SPX_PK_BYTES + SHAX_BLOCK_BYTES - 1) / SHAX_BLOCK_BYTES) as usize;

    let mut seed = vec![0u8; 2 * SPX_N + SHAX_OUTPUT_BYTES];
    let mut inbuf = vec![0u8; inblocks * SHAX_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = vec![0u8; SHAX_STATE_LEN];

    shax_inc_init(&mut state);

    inbuf[..SPX_N].copy_from_slice(r);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(pk);

    let mut mlen = mlen as usize;
    let mut m_off = 0usize;
    if SPX_N + SPX_PK_BYTES + mlen < inblocks * SHAX_BLOCK_BYTES {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen].copy_from_slice(&m[..mlen]);
        let total = SPX_N + SPX_PK_BYTES + mlen;
        let in_data = inbuf[..total].to_vec();
        shax_inc_finalize(&mut seed[2 * SPX_N..], &mut state, &in_data, total);
    } else {
        let take = inblocks * SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + take].copy_from_slice(&m[..take]);
        let block = inbuf.clone();
        shax_inc_blocks(&mut state, &block, inblocks);
        m_off += take;
        mlen -= take;
        shax_inc_finalize(&mut seed[2 * SPX_N..], &mut state, &m[m_off..], mlen);
    }

    seed[..SPX_N].copy_from_slice(r);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    mgf1_x(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SHAX_OUTPUT_BYTES);

    let mut bufp = 0usize;
    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[bufp..bufp + SPX_FORS_MSG_BYTES]);
    bufp += SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
        *tree &= !0u64 >> (64 - SPX_TREE_BITS_VAL);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= !0u32 >> (32 - SPX_LEAF_BITS);
}
