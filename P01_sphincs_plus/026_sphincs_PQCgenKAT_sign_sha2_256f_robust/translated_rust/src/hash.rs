use crate::params::*;
use crate::context::SpxCtx;
use crate::sha2::*;
use crate::address;

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    seed_state(ctx);
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);

    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&address::addr_as_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8],
                          m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    // HMAC-SHA-512 (since SPX_N >= 24)
    const SHAX_BLOCK: usize = SPX_SHA512_BLOCK_BYTES;
    const SHAX_OUT: usize = SPX_SHA512_OUTPUT_BYTES;

    let mut buf = [0u8; SHAX_BLOCK + SHAX_OUT];
    let mut state = [0u8; 8 + SHAX_OUT];

    // ipad
    for i in 0..SPX_N { buf[i] = 0x36 ^ sk_prf[i]; }
    for i in SPX_N..SHAX_BLOCK { buf[i] = 0x36; }

    sha512_inc_init(&mut state);
    sha512_inc_blocks(&mut state, &buf[..SHAX_BLOCK], 1);

    buf[..SPX_N].copy_from_slice(optrand);

    let mlen = mlen as usize;
    if SPX_N + mlen < SHAX_BLOCK {
        buf[SPX_N..SPX_N + mlen].copy_from_slice(&m[..mlen]);
        let tmp: Vec<u8> = buf[..SPX_N + mlen].to_vec();
        sha512_inc_finalize(&mut buf[SHAX_BLOCK..], &mut state, &tmp, SPX_N + mlen);
    } else {
        buf[SPX_N..SHAX_BLOCK].copy_from_slice(&m[..SHAX_BLOCK - SPX_N]);
        sha512_inc_blocks(&mut state, &buf[..SHAX_BLOCK], 1);
        let m_rest = &m[SHAX_BLOCK - SPX_N..];
        let mlen_rest = mlen - (SHAX_BLOCK - SPX_N);
        sha512_inc_finalize(&mut buf[SHAX_BLOCK..], &mut state, m_rest, mlen_rest);
    }

    // opad
    for i in 0..SPX_N { buf[i] = 0x5c ^ sk_prf[i]; }
    for i in SPX_N..SHAX_BLOCK { buf[i] = 0x5c; }

    let tmp: Vec<u8> = buf.to_vec();
    sha512(&mut buf, &tmp, SHAX_BLOCK + SHAX_OUT);
    r[..SPX_N].copy_from_slice(&buf[..SPX_N]);
}

pub fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
                    r: &[u8], pk: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    const SHAX_BLOCK: usize = SPX_SHA512_BLOCK_BYTES;
    const SHAX_OUT: usize = SPX_SHA512_OUTPUT_BYTES;
    const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;
    const SPX_INBLOCKS: usize = ((SPX_N + SPX_PK_BYTES + SHAX_BLOCK - 1) & !(SHAX_BLOCK - 1)) / SHAX_BLOCK;

    let mlen = mlen as usize;
    let mut seed = [0u8; 2 * SPX_N + SHAX_OUT];
    let mut inbuf = [0u8; SPX_INBLOCKS * SHAX_BLOCK];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; 8 + SHAX_OUT];

    sha512_inc_init(&mut state);

    inbuf[..SPX_N].copy_from_slice(&r[..SPX_N]);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    if SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS * SHAX_BLOCK {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen].copy_from_slice(&m[..mlen]);
        sha512_inc_finalize(&mut seed[2 * SPX_N..], &mut state, &inbuf, SPX_N + SPX_PK_BYTES + mlen);
    } else {
        let fill = SPX_INBLOCKS * SHAX_BLOCK - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..SPX_INBLOCKS * SHAX_BLOCK].copy_from_slice(&m[..fill]);
        sha512_inc_blocks(&mut state, &inbuf[..SPX_INBLOCKS * SHAX_BLOCK], SPX_INBLOCKS);
        let m_rest = &m[fill..];
        let mlen_rest = mlen - fill;
        sha512_inc_finalize(&mut seed[2 * SPX_N..], &mut state, m_rest, mlen_rest);
    }

    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    mgf1_512(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SHAX_OUT);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = crate::utils::bytes_to_ull(&buf[SPX_FORS_MSG_BYTES..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }

    *leaf_idx = crate::utils::bytes_to_ull(&buf[SPX_FORS_MSG_BYTES + SPX_TREE_BYTES..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
