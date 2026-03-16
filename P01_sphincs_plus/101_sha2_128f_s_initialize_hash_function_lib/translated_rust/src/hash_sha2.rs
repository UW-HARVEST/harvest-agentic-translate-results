use crate::context::SpxCtx;
use crate::params::*;
use crate::sha2::*;

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    seed_state(ctx);
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);

    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let addr_bytes = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

pub fn gen_message_random(
    r: &mut [u8], sk_prf: &[u8], optrand: &[u8],
    m: &[u8], mlen: usize, _ctx: &SpxCtx,
) {
    let mut buf = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    // HMAC inner key
    for i in 0..SPX_N {
        buf[i] = 0x36 ^ sk_prf[i];
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x36;
    }

    sha256_inc_init(&mut state);
    sha256_inc_blocks(&mut state, &buf, 1);

    buf[..SPX_N].copy_from_slice(&optrand[..SPX_N]);

    if SPX_N + mlen < SPX_SHAX_BLOCK_BYTES {
        buf[SPX_N..SPX_N + mlen].copy_from_slice(&m[..mlen]);
        let input_copy = buf[..SPX_N + mlen].to_vec();
        sha256_inc_finalize(
            &mut buf[SPX_SHAX_BLOCK_BYTES..],
            &mut state,
            &input_copy,
            mlen + SPX_N,
        );
    } else {
        let copy_len = SPX_SHAX_BLOCK_BYTES - SPX_N;
        buf[SPX_N..SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..copy_len]);
        sha256_inc_blocks(&mut state, &buf, 1);

        let m_rest = &m[copy_len..];
        let mlen_rest = mlen - copy_len;
        sha256_inc_finalize(
            &mut buf[SPX_SHAX_BLOCK_BYTES..],
            &mut state,
            m_rest,
            mlen_rest,
        );
    }

    // HMAC outer key
    for i in 0..SPX_N {
        buf[i] = 0x5c ^ sk_prf[i];
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x5c;
    }

    let buf_copy = buf.clone();
    sha256(&mut buf, &buf_copy, SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES);
    r[..SPX_N].copy_from_slice(&buf[..SPX_N]);
}

pub fn hash_message(
    digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
    r_val: &[u8], pk: &[u8], m: &[u8], mlen: usize, _ctx: &SpxCtx,
) {
    let mut seed = [0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES];
    let mut inbuf = [0u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    sha256_inc_init(&mut state);

    inbuf[..SPX_N].copy_from_slice(&r_val[..SPX_N]);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    if SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen].copy_from_slice(&m[..mlen]);
        sha256_inc_finalize(
            &mut seed[2 * SPX_N..],
            &mut state,
            &inbuf,
            SPX_N + SPX_PK_BYTES + mlen,
        );
    } else {
        let copy_len = SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + copy_len]
            .copy_from_slice(&m[..copy_len]);
        sha256_inc_blocks(&mut state, &inbuf, SPX_INBLOCKS);

        let m_rest = &m[copy_len..];
        let mlen_rest = mlen - copy_len;
        sha256_inc_finalize(&mut seed[2 * SPX_N..], &mut state, m_rest, mlen_rest);
    }

    seed[..SPX_N].copy_from_slice(&r_val[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    mgf1_256(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);

    let mut bufp = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = crate::utils::bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES as u32);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = crate::utils::bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES as u32) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
