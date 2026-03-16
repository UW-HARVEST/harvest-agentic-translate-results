use crate::address::*;
use crate::context::SpxCtx;
use crate::params::*;
use crate::sha2::*;

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    seed_state(ctx);
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    sha2_state.copy_from_slice(&ctx.state_seeded);

    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_as_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);

    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8],
                          m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    let mut buf = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    // HMAC-SHA outer key pad
    for i in 0..SPX_N {
        buf[i] = 0x36 ^ sk_prf[i];
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x36;
    }

    sha512_inc_init(&mut state);
    sha512_inc_blocks(&mut state, &buf, 1);

    buf[..SPX_N].copy_from_slice(&optrand[..SPX_N]);

    let mlen = mlen as usize;
    if SPX_N + mlen < SPX_SHAX_BLOCK_BYTES {
        buf[SPX_N..SPX_N + mlen].copy_from_slice(&m[..mlen]);
        let buf_copy: Vec<u8> = buf[..SPX_N + mlen].to_vec();
        sha512_inc_finalize(&mut buf[SPX_SHAX_BLOCK_BYTES..], &mut state, &buf_copy, mlen + SPX_N);
    } else {
        buf[SPX_N..SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..SPX_SHAX_BLOCK_BYTES - SPX_N]);
        sha512_inc_blocks(&mut state, &buf, 1);

        let m_offset = SPX_SHAX_BLOCK_BYTES - SPX_N;
        let remaining = mlen - m_offset;
        sha512_inc_finalize(&mut buf[SPX_SHAX_BLOCK_BYTES..], &mut state, &m[m_offset..], remaining);
    }

    for i in 0..SPX_N {
        buf[i] = 0x5c ^ sk_prf[i];
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x5c;
    }

    // The C code does sha512(buf, buf, ...) - reads and writes same buffer.
    let input_copy = buf;
    sha512(&mut buf, &input_copy, SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES);
    r[..SPX_N].copy_from_slice(&buf[..SPX_N]);
}

pub fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
                    r_val: &[u8], pk: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    let mlen = mlen as usize;
    let mut seed = [0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES];
    let mut inbuf = [0u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    sha512_inc_init(&mut state);

    inbuf[..SPX_N].copy_from_slice(&r_val[..SPX_N]);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    if SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen].copy_from_slice(&m[..mlen]);
        sha512_inc_finalize(&mut seed[2 * SPX_N..], &mut state, &inbuf, SPX_N + SPX_PK_BYTES + mlen);
    } else {
        let fill = SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + fill].copy_from_slice(&m[..fill]);
        sha512_inc_blocks(&mut state, &inbuf, SPX_INBLOCKS);

        let m_offset = fill;
        let remaining = mlen - m_offset;
        sha512_inc_finalize(&mut seed[2 * SPX_N..], &mut state, &m[m_offset..], remaining);
    }

    seed[..SPX_N].copy_from_slice(&r_val[..SPX_N]);
    seed[SPX_N..SPX_N + SPX_N].copy_from_slice(&pk[..SPX_N]);

    mgf1_512(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES);

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
