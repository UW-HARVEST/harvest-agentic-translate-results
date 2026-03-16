use crate::params::*;
use crate::sha2::*;
use crate::address::*;

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    seed_state(ctx);
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);

    let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

pub fn thash(out: &mut [u8], in_: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let buf_len = SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let mut sha2_state = [0u8; 40];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    mgf1_256(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_SHA256_ADDR_BYTES);

    sha2_state.copy_from_slice(&ctx.state_seeded);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = in_[i] ^ bitmask[i];
    }

    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf[SPX_N..], SPX_SHA256_ADDR_BYTES + inblocks * SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// For SHA2-128s: SPX_N < 24, so we use SHA-256 variants
const SPX_SHAX_OUTPUT_BYTES: usize = SPX_SHA256_OUTPUT_BYTES;
const SPX_SHAX_BLOCK_BYTES: usize = SPX_SHA256_BLOCK_BYTES;

pub fn gen_message_random(
    r: &mut [u8], sk_prf: &[u8], optrand: &[u8],
    m: &[u8], mlen: u64, _ctx: &SpxCtx,
) {
    let mut buf = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    // HMAC inner
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
        let mut inner_out = [0u8; SPX_SHAX_OUTPUT_BYTES];
        sha256_inc_finalize(&mut inner_out, &mut state, &buf, mlen + SPX_N);
        buf[SPX_SHAX_BLOCK_BYTES..SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES]
            .copy_from_slice(&inner_out);
    } else {
        buf[SPX_N..SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..SPX_SHAX_BLOCK_BYTES - SPX_N]);
        sha256_inc_blocks(&mut state, &buf, 1);
        let m_rest = &m[SPX_SHAX_BLOCK_BYTES - SPX_N..];
        let mlen_rest = mlen - (SPX_SHAX_BLOCK_BYTES - SPX_N);
        let mut inner_out = [0u8; SPX_SHAX_OUTPUT_BYTES];
        sha256_inc_finalize(&mut inner_out, &mut state, m_rest, mlen_rest);
        buf[SPX_SHAX_BLOCK_BYTES..SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES]
            .copy_from_slice(&inner_out);
    }

    // HMAC outer
    for i in 0..SPX_N {
        buf[i] = 0x5c ^ sk_prf[i];
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x5c;
    }

    let total = SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES;
    let mut full_out = [0u8; SPX_SHAX_OUTPUT_BYTES]; // only need first SPX_N
    sha256(&mut full_out, &buf, total);
    // Actually the C code does sha256(buf, buf, total) writing back into buf
    // then memcpy(R, buf, SPX_N). Let's match:
    r[..SPX_N].copy_from_slice(&full_out[..SPX_N]);
}

pub fn hash_message(
    digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
    r: &[u8], pk: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx,
) {
    const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1); // 54
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8; // 7
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT; // 9
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8; // 2
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES; // 30

    const SPX_INBLOCKS: usize = (SPX_N + SPX_PK_BYTES + SPX_SHAX_BLOCK_BYTES - 1) / SPX_SHAX_BLOCK_BYTES; // 1

    let mut seed = [0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES];
    let mut inbuf = [0u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    sha256_inc_init(&mut state);

    inbuf[..SPX_N].copy_from_slice(&r[..SPX_N]);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    let mlen = mlen as usize;
    if SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen].copy_from_slice(&m[..mlen]);
        sha256_inc_finalize(
            &mut seed[2 * SPX_N..], &mut state, &inbuf,
            SPX_N + SPX_PK_BYTES + mlen,
        );
    } else {
        let fill = SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + fill].copy_from_slice(&m[..fill]);
        sha256_inc_blocks(&mut state, &inbuf, SPX_INBLOCKS);
        let m_rest = &m[fill..];
        let mlen_rest = mlen - fill;
        sha256_inc_finalize(&mut seed[2 * SPX_N..], &mut state, m_rest, mlen_rest);
    }

    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    mgf1_256(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);

    let mut bufp = SPX_FORS_MSG_BYTES;
    *tree = bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
    *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    bufp += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
