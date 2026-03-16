use crate::context::SpxCtx;
use crate::params::*;
use crate::sha2::*;
use crate::utils::*;

pub fn seed_state(ctx: &mut SpxCtx) {
    let mut block = [0u8; SPX_SHA512_BLOCK_BYTES];
    block[..SPX_N].copy_from_slice(&ctx.pub_seed);
    // rest is already zero

    sha256_inc_init(&mut ctx.state_seeded);
    sha256_inc_blocks(&mut ctx.state_seeded, &block, 1);

    sha512_inc_init(&mut ctx.state_seeded_512);
    sha512_inc_blocks(&mut ctx.state_seeded_512, &block, 1);
}

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    seed_state(ctx);
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    sha2_state.copy_from_slice(&ctx.state_seeded);

    let addr_bytes: &[u8] = unsafe {
        core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32)
    };
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + SPX_N]
        .copy_from_slice(&ctx.sk_seed);

    sha256_inc_finalize(
        &mut outbuf,
        &mut sha2_state,
        &buf,
        SPX_SHA256_ADDR_BYTES + SPX_N,
    );

    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// For N>=24, shaX = sha512
fn shax_inc_init(state: &mut [u8]) { sha512_inc_init(state); }
fn shax_inc_blocks(state: &mut [u8], input: &[u8], inblocks: usize) { sha512_inc_blocks(state, input, inblocks); }
fn shax_inc_finalize(out: &mut [u8], state: &mut [u8], input: &[u8], inlen: usize) { sha512_inc_finalize(out, state, input, inlen); }
fn shax(out: &mut [u8], input: &[u8], inlen: usize) { sha512(out, input, inlen); }
fn mgf1_x(out: &mut [u8], outlen: usize, input: &[u8], inlen: usize) { mgf1_512(out, outlen, input, inlen); }

pub fn gen_message_random(
    r: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    mlen: u64,
    _ctx: &SpxCtx,
) {
    let mut buf = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    // HMAC inner key pad
    for i in 0..SPX_N {
        buf[i] = 0x36 ^ sk_prf[i];
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x36;
    }

    shax_inc_init(&mut state);
    shax_inc_blocks(&mut state, &buf, 1);

    buf[..SPX_N].copy_from_slice(&optrand[..SPX_N]);

    let mlen_usize = mlen as usize;
    if SPX_N + mlen_usize < SPX_SHAX_BLOCK_BYTES {
        buf[SPX_N..SPX_N + mlen_usize].copy_from_slice(&m[..mlen_usize]);
        let buf_copy: Vec<u8> = buf[..SPX_N + mlen_usize].to_vec();
        shax_inc_finalize(
            &mut buf[SPX_SHAX_BLOCK_BYTES..],
            &mut state,
            &buf_copy,
            mlen_usize + SPX_N,
        );
    } else {
        let fill = SPX_SHAX_BLOCK_BYTES - SPX_N;
        buf[SPX_N..SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..fill]);
        shax_inc_blocks(&mut state, &buf, 1);

        let m_rest = &m[fill..];
        let mlen_rest = mlen_usize - fill;
        shax_inc_finalize(
            &mut buf[SPX_SHAX_BLOCK_BYTES..],
            &mut state,
            m_rest,
            mlen_rest,
        );
    }

    // HMAC outer key pad
    for i in 0..SPX_N {
        buf[i] = 0x5c ^ sk_prf[i];
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x5c;
    }

    let buf_copy = buf.clone();
    shax(&mut buf, &buf_copy, SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES);
    r[..SPX_N].copy_from_slice(&buf[..SPX_N]);
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
    let mut seed = [0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES];
    let mut inbuf = [0u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    shax_inc_init(&mut state);

    inbuf[..SPX_N].copy_from_slice(&r_val[..SPX_N]);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    let mlen_usize = mlen as usize;
    if SPX_N + SPX_PK_BYTES + mlen_usize < SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen_usize]
            .copy_from_slice(&m[..mlen_usize]);
        shax_inc_finalize(
            &mut seed[2 * SPX_N..],
            &mut state,
            &inbuf,
            SPX_N + SPX_PK_BYTES + mlen_usize,
        );
    } else {
        let fill = SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES]
            .copy_from_slice(&m[..fill]);
        shax_inc_blocks(&mut state, &inbuf, SPX_INBLOCKS);

        let m_rest = &m[fill..];
        let mlen_rest = mlen_usize - fill;
        shax_inc_finalize(&mut seed[2 * SPX_N..], &mut state, m_rest, mlen_rest);
    }

    seed[..SPX_N].copy_from_slice(&r_val[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    mgf1_x(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES);

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
