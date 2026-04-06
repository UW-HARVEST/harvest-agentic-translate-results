use crate::context::SpxCtx;
use crate::params::*;
use crate::utils;

// ===== SHAKE backend =====
#[cfg(feature = "shake")]
pub fn initialize_hash_function(_ctx: &mut SpxCtx) {}

#[cfg(feature = "shake")]
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..2 * SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ctx.sk_seed);
    crate::fips202::shake256(out, SPX_N, &buf, 2 * SPX_N + SPX_ADDR_BYTES);
}

#[cfg(feature = "shake")]
pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    let mut s_inc = [0u64; 26];
    crate::fips202::shake256_inc_init(&mut s_inc);
    crate::fips202::shake256_inc_absorb(&mut s_inc, sk_prf, SPX_N);
    crate::fips202::shake256_inc_absorb(&mut s_inc, optrand, SPX_N);
    crate::fips202::shake256_inc_absorb(&mut s_inc, m, mlen as usize);
    crate::fips202::shake256_inc_finalize(&mut s_inc);
    crate::fips202::shake256_inc_squeeze(r, SPX_N, &mut s_inc);
}

#[cfg(feature = "shake")]
pub fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32, r: &[u8], pk: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u64; 26];
    crate::fips202::shake256_inc_init(&mut s_inc);
    crate::fips202::shake256_inc_absorb(&mut s_inc, r, SPX_N);
    crate::fips202::shake256_inc_absorb(&mut s_inc, pk, SPX_PK_BYTES);
    crate::fips202::shake256_inc_absorb(&mut s_inc, m, mlen as usize);
    crate::fips202::shake256_inc_finalize(&mut s_inc);
    crate::fips202::shake256_inc_squeeze(&mut buf, SPX_DGST_BYTES, &mut s_inc);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = utils::bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = utils::bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}

// ===== SHA2 backend =====
#[cfg(feature = "sha2")]
pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    crate::sha2_impl::seed_state(ctx);
}

#[cfg(feature = "sha2")]
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    sha2_state.copy_from_slice(&ctx.state_seeded);

    let addr_bytes = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_SHA256_ADDR_BYTES) };
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);

    crate::sha2_impl::sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(feature = "sha2")]
pub fn gen_message_random(r_out: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    let mlen = mlen as usize;
    let mut buf = vec![0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    let mut state = vec![0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    // HMAC-SHA
    for i in 0..SPX_N {
        buf[i] = 0x36 ^ sk_prf[i];
    }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
        buf[i] = 0x36;
    }

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        let state_arr: &mut [u8; 72] = state.as_mut_slice().try_into().unwrap();
        crate::sha2_impl::sha512_inc_init(state_arr);
        crate::sha2_impl::sha512_inc_blocks(state_arr, &buf[..SPX_SHAX_BLOCK_BYTES], 1);

        buf[..SPX_N].copy_from_slice(optrand);
        if SPX_N + mlen < SPX_SHAX_BLOCK_BYTES {
            buf[SPX_N..SPX_N + mlen].copy_from_slice(&m[..mlen]);
            let tmp_in = buf[..SPX_N + mlen].to_vec();
            crate::sha2_impl::sha512_inc_finalize(
                &mut buf[SPX_SHAX_BLOCK_BYTES..],
                state_arr,
                &tmp_in,
                mlen + SPX_N,
            );
        } else {
            buf[SPX_N..SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..SPX_SHAX_BLOCK_BYTES - SPX_N]);
            crate::sha2_impl::sha512_inc_blocks(state_arr, &buf[..SPX_SHAX_BLOCK_BYTES], 1);
            let m_rest = &m[SPX_SHAX_BLOCK_BYTES - SPX_N..];
            let mlen_rest = mlen - (SPX_SHAX_BLOCK_BYTES - SPX_N);
            crate::sha2_impl::sha512_inc_finalize(
                &mut buf[SPX_SHAX_BLOCK_BYTES..],
                state_arr,
                m_rest,
                mlen_rest,
            );
        }

        for i in 0..SPX_N {
            buf[i] = 0x5c ^ sk_prf[i];
        }
        for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
            buf[i] = 0x5c;
        }
        let tmp = buf.clone();
        crate::sha2_impl::sha512(&mut buf, &tmp, SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES);
    }

    #[cfg(any(feature = "128s", feature = "128f"))]
    {
        let state_arr: &mut [u8; 40] = state.as_mut_slice().try_into().unwrap();
        crate::sha2_impl::sha256_inc_init(state_arr);
        crate::sha2_impl::sha256_inc_blocks(state_arr, &buf[..SPX_SHAX_BLOCK_BYTES], 1);

        buf[..SPX_N].copy_from_slice(optrand);
        if SPX_N + mlen < SPX_SHAX_BLOCK_BYTES {
            buf[SPX_N..SPX_N + mlen].copy_from_slice(&m[..mlen]);
            let tmp_in = buf[..SPX_N + mlen].to_vec();
            crate::sha2_impl::sha256_inc_finalize(
                &mut buf[SPX_SHAX_BLOCK_BYTES..],
                state_arr,
                &tmp_in,
                mlen + SPX_N,
            );
        } else {
            buf[SPX_N..SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..SPX_SHAX_BLOCK_BYTES - SPX_N]);
            crate::sha2_impl::sha256_inc_blocks(state_arr, &buf[..SPX_SHAX_BLOCK_BYTES], 1);
            let m_rest = &m[SPX_SHAX_BLOCK_BYTES - SPX_N..];
            let mlen_rest = mlen - (SPX_SHAX_BLOCK_BYTES - SPX_N);
            crate::sha2_impl::sha256_inc_finalize(
                &mut buf[SPX_SHAX_BLOCK_BYTES..],
                state_arr,
                m_rest,
                mlen_rest,
            );
        }

        for i in 0..SPX_N {
            buf[i] = 0x5c ^ sk_prf[i];
        }
        for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
            buf[i] = 0x5c;
        }
        let tmp = buf.clone();
        crate::sha2_impl::sha256(&mut buf, &tmp, SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES);
    }

    r_out[..SPX_N].copy_from_slice(&buf[..SPX_N]);
}

#[cfg(feature = "sha2")]
pub fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32, r: &[u8], pk: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    let mlen = mlen as usize;
    let mut seed = vec![0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES];

    const SPX_INBLOCKS: usize = ((SPX_N + SPX_PK_BYTES + SPX_SHAX_BLOCK_BYTES - 1) & (0usize.wrapping_sub(SPX_SHAX_BLOCK_BYTES))) / SPX_SHAX_BLOCK_BYTES;
    let mut inbuf = vec![0u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = vec![0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    inbuf[..SPX_N].copy_from_slice(r);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        let state_arr: &mut [u8; 72] = state.as_mut_slice().try_into().unwrap();
        crate::sha2_impl::sha512_inc_init(state_arr);

        if SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES {
            inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen].copy_from_slice(&m[..mlen]);
            crate::sha2_impl::sha512_inc_finalize(&mut seed[2 * SPX_N..], state_arr, &inbuf, SPX_N + SPX_PK_BYTES + mlen);
        } else {
            inbuf[SPX_N + SPX_PK_BYTES..SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES]);
            crate::sha2_impl::sha512_inc_blocks(state_arr, &inbuf, SPX_INBLOCKS);
            let m_off = SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
            crate::sha2_impl::sha512_inc_finalize(&mut seed[2 * SPX_N..], state_arr, &m[m_off..], mlen - m_off);
        }

        seed[..SPX_N].copy_from_slice(r);
        seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);
        crate::sha2_impl::mgf1_512(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES);
    }

    #[cfg(any(feature = "128s", feature = "128f"))]
    {
        let state_arr: &mut [u8; 40] = state.as_mut_slice().try_into().unwrap();
        crate::sha2_impl::sha256_inc_init(state_arr);

        if SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES {
            inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen].copy_from_slice(&m[..mlen]);
            crate::sha2_impl::sha256_inc_finalize(&mut seed[2 * SPX_N..], state_arr, &inbuf, SPX_N + SPX_PK_BYTES + mlen);
        } else {
            inbuf[SPX_N + SPX_PK_BYTES..SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES]);
            crate::sha2_impl::sha256_inc_blocks(state_arr, &inbuf, SPX_INBLOCKS);
            let m_off = SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
            crate::sha2_impl::sha256_inc_finalize(&mut seed[2 * SPX_N..], state_arr, &m[m_off..], mlen - m_off);
        }

        seed[..SPX_N].copy_from_slice(r);
        seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);
        crate::sha2_impl::mgf1_256(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES);
    }

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = utils::bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = utils::bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}

// ===== BLAKE backend =====
#[cfg(feature = "blake")]
pub fn initialize_hash_function(_ctx: &mut SpxCtx) {}

#[cfg(feature = "blake")]
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let addr_bytes = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES) };

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..2 * SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ctx.sk_seed);

    crate::blake_impl::blake256(&mut outbuf, &buf, (SPX_N + SPX_ADDR_BYTES) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(feature = "blake")]
pub fn gen_message_random(r_out: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        let mut s = crate::blake_impl::BlakeState512::new();
        let mut tmp = [0u8; 64];
        crate::blake_impl::blake512_init(&mut s);
        crate::blake_impl::blake512_update(&mut s, sk_prf, SPX_N as u64);
        crate::blake_impl::blake512_update(&mut s, optrand, SPX_N as u64);
        crate::blake_impl::blake512_update(&mut s, m, mlen);
        crate::blake_impl::blake512_final(&mut s, &mut tmp);
        r_out[..SPX_N].copy_from_slice(&tmp[..SPX_N]);
    }
    #[cfg(any(feature = "128s", feature = "128f"))]
    {
        let mut s = crate::blake_impl::BlakeState256::new();
        let mut tmp = [0u8; 32];
        crate::blake_impl::blake256_init(&mut s);
        crate::blake_impl::blake256_update(&mut s, sk_prf, SPX_N as u64);
        crate::blake_impl::blake256_update(&mut s, optrand, SPX_N as u64);
        crate::blake_impl::blake256_update(&mut s, m, mlen);
        crate::blake_impl::blake256_final(&mut s, &mut tmp);
        r_out[..SPX_N].copy_from_slice(&tmp[..SPX_N]);
    }
}

#[cfg(feature = "blake")]
pub fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32, r: &[u8], pk: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut seed = vec![0u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES];

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        let mut s = crate::blake_impl::BlakeState512::new();
        crate::blake_impl::blake512_init(&mut s);
        crate::blake_impl::blake512_update(&mut s, r, SPX_N as u64);
        crate::blake_impl::blake512_update(&mut s, pk, SPX_PK_BYTES as u64);
        crate::blake_impl::blake512_update(&mut s, m, mlen);
        crate::blake_impl::blake512_final(&mut s, &mut seed[2 * SPX_N..]);

        seed[..SPX_N].copy_from_slice(r);
        seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

        crate::blake_impl::blake512_mgf1(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES);
    }
    #[cfg(any(feature = "128s", feature = "128f"))]
    {
        let mut s = crate::blake_impl::BlakeState256::new();
        crate::blake_impl::blake256_init(&mut s);
        crate::blake_impl::blake256_update(&mut s, r, SPX_N as u64);
        crate::blake_impl::blake256_update(&mut s, pk, SPX_PK_BYTES as u64);
        crate::blake_impl::blake256_update(&mut s, m, mlen);
        crate::blake_impl::blake256_final(&mut s, &mut seed[2 * SPX_N..]);

        seed[..SPX_N].copy_from_slice(r);
        seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

        crate::blake_impl::blake256_mgf1(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES);
    }

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = utils::bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = utils::bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}

// ===== HARAKA backend =====
#[cfg(feature = "haraka")]
pub fn initialize_hash_function(ctx: &mut SpxCtx) {
    crate::haraka_impl::tweak_constants(ctx);
}

#[cfg(feature = "haraka")]
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut outbuf = [0u8; 32];
    let mut buf = [0u8; 64];
    let addr_bytes = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES) };

    buf[..SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);

    crate::haraka_impl::haraka512(&mut outbuf, &buf, ctx);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(feature = "haraka")]
pub fn gen_message_random(r_out: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], mlen: u64, ctx: &SpxCtx) {
    let mut s_inc = [0u8; 65];
    crate::haraka_impl::haraka_S_inc_init(&mut s_inc);
    crate::haraka_impl::haraka_S_inc_absorb(&mut s_inc, sk_prf, SPX_N, ctx);
    crate::haraka_impl::haraka_S_inc_absorb(&mut s_inc, optrand, SPX_N, ctx);
    crate::haraka_impl::haraka_S_inc_absorb(&mut s_inc, m, mlen as usize, ctx);
    crate::haraka_impl::haraka_S_inc_finalize(&mut s_inc);
    crate::haraka_impl::haraka_S_inc_squeeze(r_out, SPX_N, &mut s_inc, ctx);
}

#[cfg(feature = "haraka")]
pub fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32, r: &[u8], pk: &[u8], m: &[u8], mlen: u64, ctx: &SpxCtx) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u8; 65];

    crate::haraka_impl::haraka_S_inc_init(&mut s_inc);
    crate::haraka_impl::haraka_S_inc_absorb(&mut s_inc, r, SPX_N, ctx);
    // Only absorb root part of pk
    crate::haraka_impl::haraka_S_inc_absorb(&mut s_inc, &pk[SPX_N..], SPX_N, ctx);
    crate::haraka_impl::haraka_S_inc_absorb(&mut s_inc, m, mlen as usize, ctx);
    crate::haraka_impl::haraka_S_inc_finalize(&mut s_inc);
    crate::haraka_impl::haraka_S_inc_squeeze(&mut buf, SPX_DGST_BYTES, &mut s_inc, ctx);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = utils::bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = utils::bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
