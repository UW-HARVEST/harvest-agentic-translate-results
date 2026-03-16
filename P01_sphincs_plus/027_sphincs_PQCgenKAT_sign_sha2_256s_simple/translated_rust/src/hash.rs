use crate::params::*;
use crate::context::SpxCtx;
use crate::sha2;
use crate::address::{addr_as_bytes, u32_to_bytes};

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    sha2::seed_state(ctx);
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);

    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let ab = addr_as_bytes(addr);
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&ab[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    sha2::sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// SHA-512 based for SPX_N >= 24
pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8],
                          m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    let mlen = mlen as usize;
    let mut buf = [0u8; SPX_SHA512_BLOCK_BYTES + SPX_SHA512_OUTPUT_BYTES];
    let mut state = [0u8; 8 + SPX_SHA512_OUTPUT_BYTES]; // 72

    // HMAC-SHA512
    for i in 0..SPX_N { buf[i] = 0x36 ^ sk_prf[i]; }
    for i in SPX_N..SPX_SHA512_BLOCK_BYTES { buf[i] = 0x36; }

    sha2::sha512_inc_init(&mut state);
    sha2::sha512_inc_blocks(&mut state, &buf, 1);

    buf[..SPX_N].copy_from_slice(optrand);

    if SPX_N + mlen < SPX_SHA512_BLOCK_BYTES {
        buf[SPX_N..SPX_N+mlen].copy_from_slice(&m[..mlen]);
        let tmp: Vec<u8> = buf[..SPX_N+mlen].to_vec();
        sha2::sha512_inc_finalize(&mut buf[SPX_SHA512_BLOCK_BYTES..], &mut state,
                                  &tmp, mlen + SPX_N);
    } else {
        buf[SPX_N..SPX_SHA512_BLOCK_BYTES].copy_from_slice(&m[..SPX_SHA512_BLOCK_BYTES - SPX_N]);
        sha2::sha512_inc_blocks(&mut state, &buf, 1);
        let m_off = SPX_SHA512_BLOCK_BYTES - SPX_N;
        let mlen_rem = mlen - m_off;
        sha2::sha512_inc_finalize(&mut buf[SPX_SHA512_BLOCK_BYTES..], &mut state,
                                  &m[m_off..], mlen_rem);
    }

    for i in 0..SPX_N { buf[i] = 0x5c ^ sk_prf[i]; }
    for i in SPX_N..SPX_SHA512_BLOCK_BYTES { buf[i] = 0x5c; }

    let total = SPX_SHA512_BLOCK_BYTES + SPX_SHA512_OUTPUT_BYTES;
    let mut tmp = [0u8; SPX_SHA512_BLOCK_BYTES + SPX_SHA512_OUTPUT_BYTES];
    tmp[..total].copy_from_slice(&buf[..total]);
    sha2::sha512(&mut buf, &tmp, total);
    r[..SPX_N].copy_from_slice(&buf[..SPX_N]);
}

pub fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
                    r_val: &[u8], pk: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    let mlen = mlen as usize;
    const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;
    const SPX_INBLOCKS: usize = (SPX_N + SPX_PK_BYTES + SPX_SHA512_BLOCK_BYTES - 1) / SPX_SHA512_BLOCK_BYTES;

    let mut seed = [0u8; 2 * SPX_N + SPX_SHA512_OUTPUT_BYTES];
    let mut inbuf = [0u8; SPX_INBLOCKS * SPX_SHA512_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; 8 + SPX_SHA512_OUTPUT_BYTES]; // 72

    sha2::sha512_inc_init(&mut state);

    inbuf[..SPX_N].copy_from_slice(&r_val[..SPX_N]);
    inbuf[SPX_N..SPX_N+SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    if SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS * SPX_SHA512_BLOCK_BYTES {
        inbuf[SPX_N+SPX_PK_BYTES..SPX_N+SPX_PK_BYTES+mlen].copy_from_slice(&m[..mlen]);
        sha2::sha512_inc_finalize(&mut seed[2*SPX_N..], &mut state, &inbuf, SPX_N + SPX_PK_BYTES + mlen);
    } else {
        let fill = SPX_INBLOCKS * SPX_SHA512_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N+SPX_PK_BYTES..SPX_N+SPX_PK_BYTES+fill].copy_from_slice(&m[..fill]);
        sha2::sha512_inc_blocks(&mut state, &inbuf, SPX_INBLOCKS);
        let m_off = fill;
        let mlen_rem = mlen - m_off;
        sha2::sha512_inc_finalize(&mut seed[2*SPX_N..], &mut state, &m[m_off..], mlen_rem);
    }

    seed[..SPX_N].copy_from_slice(&r_val[..SPX_N]);
    seed[SPX_N..2*SPX_N].copy_from_slice(&pk[..SPX_N]);

    sha2::mgf1_512(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_SHA512_OUTPUT_BYTES);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);

    let mut bufp = SPX_FORS_MSG_BYTES;
    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = crate::address::bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = crate::address::bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
