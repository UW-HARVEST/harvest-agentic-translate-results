use crate::context::SpxCtx;
use crate::params::*;
use crate::sha2;
use crate::utils;

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    sha2::seed_state(ctx);
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);

    let addr_bytes: &[u8] =
        unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };

    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    sha2::sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + SPX_N);
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
    let mlen = mlen as usize;
    // HMAC-SHA256
    let mut buf = [0u8; SPX_SHA256_BLOCK_BYTES + SPX_SHA256_OUTPUT_BYTES];
    let mut state = [0u8; 40 + SPX_SHA256_OUTPUT_BYTES]; // 8 extra for state_len=40

    // ipad
    for i in 0..SPX_N {
        buf[i] = 0x36 ^ sk_prf[i];
    }
    for i in SPX_N..SPX_SHA256_BLOCK_BYTES {
        buf[i] = 0x36;
    }

    sha2::sha256_inc_init(&mut state[..40]);
    sha2::sha256_inc_blocks(&mut state[..40], &buf[..SPX_SHA256_BLOCK_BYTES], 1);

    buf[..SPX_N].copy_from_slice(&optrand[..SPX_N]);

    if SPX_N + mlen < SPX_SHA256_BLOCK_BYTES {
        buf[SPX_N..SPX_N + mlen].copy_from_slice(&m[..mlen]);
        let mut tmp = vec![0u8; SPX_N + mlen];
        tmp.copy_from_slice(&buf[..SPX_N + mlen]);
        sha2::sha256_inc_finalize(
            &mut buf[SPX_SHA256_BLOCK_BYTES..],
            &mut state[..40],
            &tmp,
            SPX_N + mlen,
        );
    } else {
        buf[SPX_N..SPX_SHA256_BLOCK_BYTES].copy_from_slice(&m[..SPX_SHA256_BLOCK_BYTES - SPX_N]);
        sha2::sha256_inc_blocks(&mut state[..40], &buf[..SPX_SHA256_BLOCK_BYTES], 1);

        let m_rest = &m[SPX_SHA256_BLOCK_BYTES - SPX_N..];
        let mlen_rest = mlen - (SPX_SHA256_BLOCK_BYTES - SPX_N);
        sha2::sha256_inc_finalize(
            &mut buf[SPX_SHA256_BLOCK_BYTES..],
            &mut state[..40],
            m_rest,
            mlen_rest,
        );
    }

    // opad
    for i in 0..SPX_N {
        buf[i] = 0x5c ^ sk_prf[i];
    }
    for i in SPX_N..SPX_SHA256_BLOCK_BYTES {
        buf[i] = 0x5c;
    }

    let mut final_out = [0u8; SPX_SHA256_OUTPUT_BYTES];
    sha2::sha256(
        &mut final_out,
        &buf[..SPX_SHA256_BLOCK_BYTES + SPX_SHA256_OUTPUT_BYTES],
        SPX_SHA256_BLOCK_BYTES + SPX_SHA256_OUTPUT_BYTES,
    );
    r[..SPX_N].copy_from_slice(&final_out[..SPX_N]);
}

pub fn hash_message(
    digest: &mut [u8],
    tree: &mut u64,
    leaf_idx: &mut u32,
    r_val: &[u8],
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
    const SPX_INBLOCKS: usize =
        (SPX_N + SPX_PK_BYTES + SPX_SHA256_BLOCK_BYTES - 1) / SPX_SHA256_BLOCK_BYTES;

    let mlen = mlen as usize;
    let mut seed = [0u8; 2 * SPX_N + SPX_SHA256_OUTPUT_BYTES];
    let mut inbuf = [0u8; SPX_INBLOCKS * SPX_SHA256_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; 40 + SPX_SHA256_OUTPUT_BYTES]; // state_len=40

    sha2::sha256_inc_init(&mut state[..40]);

    inbuf[..SPX_N].copy_from_slice(&r_val[..SPX_N]);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    if SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS * SPX_SHA256_BLOCK_BYTES {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen].copy_from_slice(&m[..mlen]);
        sha2::sha256_inc_finalize(
            &mut seed[2 * SPX_N..],
            &mut state[..40],
            &inbuf,
            SPX_N + SPX_PK_BYTES + mlen,
        );
    } else {
        let fill = SPX_INBLOCKS * SPX_SHA256_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + fill].copy_from_slice(&m[..fill]);
        sha2::sha256_inc_blocks(&mut state[..40], &inbuf, SPX_INBLOCKS);

        let m_rest = &m[fill..];
        let mlen_rest = mlen - fill;
        sha2::sha256_inc_finalize(&mut seed[2 * SPX_N..], &mut state[..40], m_rest, mlen_rest);
    }

    seed[..SPX_N].copy_from_slice(&r_val[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    sha2::mgf1_256(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_SHA256_OUTPUT_BYTES);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);

    let mut off = SPX_FORS_MSG_BYTES;
    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = utils::bytes_to_ull(&buf[off..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    off += SPX_TREE_BYTES;

    *leaf_idx = utils::bytes_to_ull(&buf[off..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}

pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];
    let mut sha2_state = [0u8; 40];

    let addr_bytes: &[u8] =
        unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    sha2::mgf1_256(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_SHA256_ADDR_BYTES);

    sha2_state.copy_from_slice(&ctx.state_seeded);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    sha2::sha256_inc_finalize(
        &mut outbuf,
        &mut sha2_state,
        &buf[SPX_N..],
        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
