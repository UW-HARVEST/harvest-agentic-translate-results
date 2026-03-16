use crate::params::*;
use crate::context::SpxCtx;
use crate::address::*;
use crate::sha2::*;
use crate::utils::*;

pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    seed_state(ctx);
}

fn seed_state(ctx: &mut SpxCtx) {
    let mut block = [0u8; SPX_SHA256_BLOCK_BYTES]; // 64 is enough, sha512 block not needed
    block[..SPX_N].copy_from_slice(&ctx.pub_seed);
    // rest already zero

    sha256_inc_init(&mut ctx.state_seeded);
    sha256_inc_blocks(&mut ctx.state_seeded, &block, 1);
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);

    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_as_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

pub fn gen_message_random(
    r: &mut [u8], sk_prf: &[u8], optrand: &[u8],
    m: &[u8], mlen: u64, _ctx: &SpxCtx,
) {
    // SPX_N < 24 so uses sha256 variants
    const SHAX_OUTPUT_BYTES: usize = SPX_SHA256_OUTPUT_BYTES;
    const SHAX_BLOCK_BYTES: usize = SPX_SHA256_BLOCK_BYTES;

    let mut buf = [0u8; SHAX_BLOCK_BYTES + SHAX_OUTPUT_BYTES];
    let mut state = [0u8; 8 + SHAX_OUTPUT_BYTES]; // 40

    // HMAC-SHA inner
    for i in 0..SPX_N {
        buf[i] = 0x36 ^ sk_prf[i];
    }
    for i in SPX_N..SHAX_BLOCK_BYTES {
        buf[i] = 0x36;
    }

    sha256_inc_init(&mut state);
    sha256_inc_blocks(&mut state, &buf[..SHAX_BLOCK_BYTES], 1);

    buf[..SPX_N].copy_from_slice(&optrand[..SPX_N]);

    let mlen = mlen as usize;
    if SPX_N + mlen < SHAX_BLOCK_BYTES {
        buf[SPX_N..SPX_N + mlen].copy_from_slice(&m[..mlen]);
        let tmp = buf[..SPX_N + mlen].to_vec();
        sha256_inc_finalize(
            &mut buf[SHAX_BLOCK_BYTES..],
            &mut state,
            &tmp,
            mlen + SPX_N,
        );
    } else {
        buf[SPX_N..SHAX_BLOCK_BYTES].copy_from_slice(&m[..SHAX_BLOCK_BYTES - SPX_N]);
        sha256_inc_blocks(&mut state, &buf[..SHAX_BLOCK_BYTES], 1);

        let m_rest = &m[SHAX_BLOCK_BYTES - SPX_N..];
        let mlen_rest = mlen - (SHAX_BLOCK_BYTES - SPX_N);
        sha256_inc_finalize(
            &mut buf[SHAX_BLOCK_BYTES..],
            &mut state,
            m_rest,
            mlen_rest,
        );
    }

    // HMAC-SHA outer
    for i in 0..SPX_N {
        buf[i] = 0x5c ^ sk_prf[i];
    }
    for i in SPX_N..SHAX_BLOCK_BYTES {
        buf[i] = 0x5c;
    }

    let mut tmp = vec![0u8; SHAX_BLOCK_BYTES + SHAX_OUTPUT_BYTES];
    tmp.copy_from_slice(&buf[..SHAX_BLOCK_BYTES + SHAX_OUTPUT_BYTES]);
    sha256(&mut buf, &tmp, SHAX_BLOCK_BYTES + SHAX_OUTPUT_BYTES);

    r[..SPX_N].copy_from_slice(&buf[..SPX_N]);
}

pub fn hash_message(
    digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
    r: &[u8], pk: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx,
) {
    const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;
    const SHAX_OUTPUT_BYTES: usize = SPX_SHA256_OUTPUT_BYTES;
    const SHAX_BLOCK_BYTES: usize = SPX_SHA256_BLOCK_BYTES;
    const SPX_INBLOCKS: usize = (SPX_N + SPX_PK_BYTES + SHAX_BLOCK_BYTES - 1) / SHAX_BLOCK_BYTES;

    let mut seed = [0u8; 2 * SPX_N + SHAX_OUTPUT_BYTES];
    let mut inbuf = [0u8; SPX_INBLOCKS * SHAX_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; 8 + SHAX_OUTPUT_BYTES]; // 40

    sha256_inc_init(&mut state);

    inbuf[..SPX_N].copy_from_slice(&r[..SPX_N]);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    let mlen = mlen as usize;
    if SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS * SHAX_BLOCK_BYTES {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen].copy_from_slice(&m[..mlen]);
        sha256_inc_finalize(
            &mut seed[2 * SPX_N..],
            &mut state,
            &inbuf,
            SPX_N + SPX_PK_BYTES + mlen,
        );
    } else {
        let fill = SPX_INBLOCKS * SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..SPX_INBLOCKS * SHAX_BLOCK_BYTES].copy_from_slice(&m[..fill]);
        sha256_inc_blocks(&mut state, &inbuf, SPX_INBLOCKS);

        let m_rest = &m[fill..];
        let mlen_rest = mlen - fill;
        sha256_inc_finalize(&mut seed[2 * SPX_N..], &mut state, m_rest, mlen_rest);
    }

    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    mgf1_256(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SHAX_OUTPUT_BYTES);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);

    let bufp = &buf[SPX_FORS_MSG_BYTES..];
    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(bufp, SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }

    let bufp = &buf[SPX_FORS_MSG_BYTES + SPX_TREE_BYTES..];
    *leaf_idx = bytes_to_ull(bufp, SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
