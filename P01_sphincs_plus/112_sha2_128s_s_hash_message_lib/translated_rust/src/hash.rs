// hash_sha2.rs - SHA2 hash functions for SPHINCS+

use crate::params::*;
use crate::sha2::*;
use crate::utils::{ull_to_bytes, u32_to_bytes, bytes_to_ull};

pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
    pub state_seeded: [u8; 40],
}

/// Absorb pub_seed into SHA-256 state
pub fn seed_state(ctx: &mut SpxCtx) {
    let mut block = [0u8; 128]; // SPX_SHA512_BLOCK_BYTES, but we only use 64
    block[..SPX_N].copy_from_slice(&ctx.pub_seed);
    // rest is already zero

    sha256_inc_init(&mut ctx.state_seeded);
    sha256_inc_blocks(&mut ctx.state_seeded, &block, 1);
}

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    seed_state(ctx);
}

/// PRF(pk_seed, sk_seed, addr)
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);

    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let addr_bytes = addr_to_bytes(addr);
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

/// thash - simple variant (SHA-256 only since SPX_SHA512=0)
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);

    let buf_len = SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let addr_bytes = addr_to_bytes(addr);
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..buf_len].copy_from_slice(&inp[..inblocks * SPX_N]);

    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, buf_len);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

/// gen_message_random - HMAC-SHA256 based
pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8],
                          m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    let mut buf = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    // ipad
    for i in 0..SPX_N {
        buf[i] = 0x36 ^ sk_prf[i];
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x36;
    }

    sha256_inc_init(&mut state);
    sha256_inc_blocks(&mut state, &buf, 1);

    buf[..SPX_N].copy_from_slice(optrand);

    let mlen = mlen as usize;
    if SPX_N + mlen < SPX_SHAX_BLOCK_BYTES {
        buf[SPX_N..SPX_N + mlen].copy_from_slice(&m[..mlen]);
        let tmp = buf; // copy
        sha256_inc_finalize(&mut buf[SPX_SHAX_BLOCK_BYTES..], &mut state, &tmp[..SPX_N + mlen], mlen + SPX_N);
    } else {
        buf[SPX_N..SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..SPX_SHAX_BLOCK_BYTES - SPX_N]);
        sha256_inc_blocks(&mut state, &buf, 1);

        let m_offset = SPX_SHAX_BLOCK_BYTES - SPX_N;
        let remaining = mlen - m_offset;
        sha256_inc_finalize(&mut buf[SPX_SHAX_BLOCK_BYTES..], &mut state, &m[m_offset..], remaining);
    }

    // opad
    for i in 0..SPX_N {
        buf[i] = 0x5c ^ sk_prf[i];
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x5c;
    }

    let mut hash_out = [0u8; SPX_SHAX_OUTPUT_BYTES]; // temporary, but we hash into buf
    sha256(&mut hash_out, &buf, SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES);
    r[..SPX_N].copy_from_slice(&hash_out[..SPX_N]);
}

/// MGF1-SHA256
pub fn mgf1_256(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    let mut i = 0u32;
    let mut pos = 0usize;
    while ((i as usize) + 1) * SPX_SHA256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i);
        sha256(&mut out[pos..], &inbuf, inlen + 4);
        pos += SPX_SHA256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > (i as usize) * SPX_SHA256_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i);
        sha256(&mut outbuf, &inbuf, inlen + 4);
        let remaining = outlen - (i as usize) * SPX_SHA256_OUTPUT_BYTES;
        out[pos..pos + remaining].copy_from_slice(&outbuf[..remaining]);
    }
}

/// hash_message
pub fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
                    r: &[u8], pk: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    let mlen = mlen as usize;
    let mut seed = [0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES];
    let mut inbuf = [0u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    sha256_inc_init(&mut state);

    inbuf[..SPX_N].copy_from_slice(r);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    if SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen].copy_from_slice(&m[..mlen]);
        sha256_inc_finalize(&mut seed[2 * SPX_N..], &mut state, &inbuf, SPX_N + SPX_PK_BYTES + mlen);
    } else {
        let fill = SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + fill].copy_from_slice(&m[..fill]);
        sha256_inc_blocks(&mut state, &inbuf, SPX_INBLOCKS);

        let m_remaining = &m[fill..];
        let mlen_remaining = mlen - fill;
        sha256_inc_finalize(&mut seed[2 * SPX_N..], &mut state, m_remaining, mlen_remaining);
    }

    seed[..SPX_N].copy_from_slice(r);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    mgf1_256(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);

    let mut bufp = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}

/// Helper: convert addr [u32; 8] to byte slice
pub fn addr_to_bytes(addr: &[u32; 8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for i in 0..8 {
        out[4*i]     = (addr[i] >> 24) as u8;
        out[4*i + 1] = (addr[i] >> 16) as u8;
        out[4*i + 2] = (addr[i] >> 8) as u8;
        out[4*i + 3] = addr[i] as u8;
    }
    out
}

/// Helper: convert byte slice back to addr
pub fn bytes_to_addr(bytes: &[u8]) -> [u32; 8] {
    let mut addr = [0u32; 8];
    for i in 0..8 {
        addr[i] = ((bytes[4*i] as u32) << 24) | ((bytes[4*i+1] as u32) << 16)
                 | ((bytes[4*i+2] as u32) << 8) | (bytes[4*i+3] as u32);
    }
    addr
}
