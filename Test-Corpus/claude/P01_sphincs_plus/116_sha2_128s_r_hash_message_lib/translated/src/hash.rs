use crate::context::SpxCtx;
use crate::params::{SPX_ADDR_BYTES, SPX_D, SPX_FORS_HEIGHT, SPX_FORS_MSG_BYTES, SPX_FORS_TREES,
                    SPX_N, SPX_PK_BYTES, SPX_TREE_HEIGHT};

#[cfg(feature = "sha2")]
mod backend {
    use super::*;
    use crate::sha2_impl::*;

    const SPX_SHA256_ADDR_BYTES: usize = 22;

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    const SPX_SHAX_OUTPUT_BYTES: usize = SPX_SHA512_OUTPUT_BYTES;
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    const SPX_SHAX_BLOCK_BYTES: usize = 128;

    #[cfg(any(feature = "128s", feature = "128f"))]
    const SPX_SHAX_OUTPUT_BYTES: usize = SPX_SHA256_OUTPUT_BYTES;
    #[cfg(any(feature = "128s", feature = "128f"))]
    const SPX_SHAX_BLOCK_BYTES: usize = 64;

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    const STATE_LEN: usize = 72;
    #[cfg(any(feature = "128s", feature = "128f"))]
    const STATE_LEN: usize = 40;

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    fn shax_inc_init(s: &mut [u8]) { sha512_inc_init(s); }
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    fn shax_inc_blocks(s: &mut [u8], input: &[u8], n: usize) { sha512_inc_blocks(s, input, n); }
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    fn shax_inc_finalize(out: &mut [u8], s: &mut [u8], input: &[u8], inlen: usize) { sha512_inc_finalize(out, s, input, inlen); }
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    fn shax(out: &mut [u8], input: &[u8]) { sha512(out, input); }
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    fn mgf1_x(out: &mut [u8], outlen: usize, input: &[u8], inlen: usize) { mgf1_512(out, outlen, input, inlen); }

    #[cfg(any(feature = "128s", feature = "128f"))]
    fn shax_inc_init(s: &mut [u8]) { sha256_inc_init(s); }
    #[cfg(any(feature = "128s", feature = "128f"))]
    fn shax_inc_blocks(s: &mut [u8], input: &[u8], n: usize) { sha256_inc_blocks(s, input, n); }
    #[cfg(any(feature = "128s", feature = "128f"))]
    fn shax_inc_finalize(out: &mut [u8], s: &mut [u8], input: &[u8], inlen: usize) { sha256_inc_finalize(out, s, input, inlen); }
    #[cfg(any(feature = "128s", feature = "128f"))]
    fn shax(out: &mut [u8], input: &[u8]) { sha256(out, input); }
    #[cfg(any(feature = "128s", feature = "128f"))]
    fn mgf1_x(out: &mut [u8], outlen: usize, input: &[u8], inlen: usize) { mgf1_256(out, outlen, input, inlen); }

    /// Absorb pub_seed using one round of SHA-256 (and SHA-512 if applicable).
    pub fn seed_state(ctx: &mut SpxCtx) {
        let mut block = [0u8; 128];
        for i in 0..SPX_N {
            block[i] = ctx.pub_seed[i];
        }
        // The rest is already zero.
        sha256_inc_init(&mut ctx.state_seeded);
        sha256_inc_blocks(&mut ctx.state_seeded, &block, 1);
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        {
            sha512_inc_init(&mut ctx.state_seeded_512);
            sha512_inc_blocks(&mut ctx.state_seeded_512, &block, 1);
        }
    }

    pub fn initialize_hash_function(ctx: &mut SpxCtx) {
        seed_state(ctx);
    }

    pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
        let mut sha2_state = [0u8; 40];
        sha2_state.copy_from_slice(&ctx.state_seeded);
        let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
        let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
        let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
        buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
        buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);
        sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + SPX_N);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }

    pub fn gen_message_random(
        r: &mut [u8],
        sk_prf: &[u8],
        optrand: &[u8],
        m: &[u8],
        mut mlen: u64,
        _ctx: &SpxCtx,
    ) {
        let mut buf = vec![0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
        let mut state = vec![0u8; STATE_LEN];

        for i in 0..SPX_N {
            buf[i] = 0x36 ^ sk_prf[i];
        }
        for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
            buf[i] = 0x36;
        }

        shax_inc_init(&mut state);
        let buf_block = buf[..SPX_SHAX_BLOCK_BYTES].to_vec();
        shax_inc_blocks(&mut state, &buf_block, 1);

        buf[..SPX_N].copy_from_slice(&optrand[..SPX_N]);

        let mut m_pos = 0usize;
        if (SPX_N as u64) + mlen < SPX_SHAX_BLOCK_BYTES as u64 {
            buf[SPX_N..SPX_N + mlen as usize].copy_from_slice(&m[..mlen as usize]);
            let buf_finalize_in = buf[..SPX_N + mlen as usize].to_vec();
            shax_inc_finalize(&mut buf[SPX_SHAX_BLOCK_BYTES..], &mut state, &buf_finalize_in, mlen as usize + SPX_N);
        } else {
            let take = SPX_SHAX_BLOCK_BYTES - SPX_N;
            buf[SPX_N..SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..take]);
            let buf_block = buf[..SPX_SHAX_BLOCK_BYTES].to_vec();
            shax_inc_blocks(&mut state, &buf_block, 1);

            m_pos += take;
            mlen -= take as u64;
            shax_inc_finalize(&mut buf[SPX_SHAX_BLOCK_BYTES..], &mut state, &m[m_pos..m_pos + mlen as usize], mlen as usize);
        }

        for i in 0..SPX_N {
            buf[i] = 0x5c ^ sk_prf[i];
        }
        for i in SPX_N..SPX_SHAX_BLOCK_BYTES {
            buf[i] = 0x5c;
        }

        let buf_copy = buf.clone();
        shax(&mut buf, &buf_copy);
        r[..SPX_N].copy_from_slice(&buf[..SPX_N]);
    }

    pub fn hash_message(
        digest: &mut [u8],
        tree: &mut u64,
        leaf_idx: &mut u32,
        r: &[u8],
        pk: &[u8],
        m: &[u8],
        mut mlen: u64,
        _ctx: &SpxCtx,
    ) {
        const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
        const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
        const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
        const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
        const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

        let mut seed = vec![0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES];

        // SPX_INBLOCKS = ceil((SPX_N + SPX_PK_BYTES) / SPX_SHAX_BLOCK_BYTES)
        let inblocks = (SPX_N + SPX_PK_BYTES + SPX_SHAX_BLOCK_BYTES - 1) / SPX_SHAX_BLOCK_BYTES;
        let mut inbuf = vec![0u8; inblocks * SPX_SHAX_BLOCK_BYTES];

        let mut buf = vec![0u8; SPX_DGST_BYTES];
        let mut state = vec![0u8; STATE_LEN];
        shax_inc_init(&mut state);

        inbuf[..SPX_N].copy_from_slice(&r[..SPX_N]);
        inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

        let mut m_pos = 0usize;
        if (SPX_N + SPX_PK_BYTES) as u64 + mlen < (inblocks * SPX_SHAX_BLOCK_BYTES) as u64 {
            inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen as usize]
                .copy_from_slice(&m[..mlen as usize]);
            shax_inc_finalize(&mut seed[2 * SPX_N..], &mut state, &inbuf, SPX_N + SPX_PK_BYTES + mlen as usize);
        } else {
            let take = inblocks * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
            inbuf[SPX_N + SPX_PK_BYTES..].copy_from_slice(&m[..take]);
            shax_inc_blocks(&mut state, &inbuf, inblocks);
            m_pos += take;
            mlen -= take as u64;
            shax_inc_finalize(&mut seed[2 * SPX_N..], &mut state, &m[m_pos..m_pos + mlen as usize], mlen as usize);
        }

        seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
        seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

        mgf1_x(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES);

        digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
        let mut bufp = SPX_FORS_MSG_BYTES;

        if SPX_D == 1 {
            *tree = 0;
        } else {
            *tree = crate::utils::bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
            *tree &= u64::MAX >> (64 - SPX_TREE_BITS);
        }
        bufp += SPX_TREE_BYTES;

        *leaf_idx = crate::utils::bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
        *leaf_idx &= u32::MAX >> (32 - SPX_LEAF_BITS);
    }
}

#[cfg(feature = "shake")]
mod backend {
    use super::*;
    use crate::fips202::*;

    pub fn initialize_hash_function(_ctx: &mut SpxCtx) {}

    pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
        let mut buf = vec![0u8; 2 * SPX_N + SPX_ADDR_BYTES];
        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);
        buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);
        let buf_copy = buf.clone();
        shake256(out, SPX_N, &buf_copy, 2 * SPX_N + SPX_ADDR_BYTES);
    }

    pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
        let mut s_inc = [0u64; 26];
        shake256_inc_init(&mut s_inc);
        shake256_inc_absorb(&mut s_inc, sk_prf, SPX_N);
        shake256_inc_absorb(&mut s_inc, optrand, SPX_N);
        shake256_inc_absorb(&mut s_inc, m, mlen as usize);
        shake256_inc_finalize(&mut s_inc);
        shake256_inc_squeeze(r, SPX_N, &mut s_inc);
    }

    pub fn hash_message(
        digest: &mut [u8],
        tree: &mut u64,
        leaf_idx: &mut u32,
        r: &[u8],
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

        let mut buf = vec![0u8; SPX_DGST_BYTES];
        let mut s_inc = [0u64; 26];

        shake256_inc_init(&mut s_inc);
        shake256_inc_absorb(&mut s_inc, r, SPX_N);
        shake256_inc_absorb(&mut s_inc, pk, SPX_PK_BYTES);
        shake256_inc_absorb(&mut s_inc, m, mlen as usize);
        shake256_inc_finalize(&mut s_inc);
        shake256_inc_squeeze(&mut buf, SPX_DGST_BYTES, &mut s_inc);

        digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
        let mut bufp = SPX_FORS_MSG_BYTES;

        if SPX_D == 1 {
            *tree = 0;
        } else {
            *tree = crate::utils::bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
            *tree &= u64::MAX >> (64 - SPX_TREE_BITS);
        }
        bufp += SPX_TREE_BYTES;

        *leaf_idx = crate::utils::bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
        *leaf_idx &= u32::MAX >> (32 - SPX_LEAF_BITS);
    }
}

#[cfg(feature = "blake")]
mod backend {
    use super::*;
    use crate::blake::*;

    pub fn initialize_hash_function(_ctx: &mut SpxCtx) {}

    pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
        let mut buf = vec![0u8; 2 * SPX_N + SPX_ADDR_BYTES];
        let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);
        buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);
        // C calls blake256(outbuf, buf, SPX_N + SPX_ADDR_BYTES) — so it does NOT
        // include the appended sk_seed in the digest input length.
        blake256(&mut outbuf, &buf, (SPX_N + SPX_ADDR_BYTES) as u64);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }

    // NOTE: The reference C code passes byte counts to blake*_update, but
    // blake*_update interprets its `datalen` argument as a BIT count. We
    // faithfully reproduce that behavior for byte-identical output.
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
        let mut s = Blakestate512::new();
        blake512_init(&mut s);
        blake512_update(&mut s, sk_prf, SPX_N as u64);
        blake512_update(&mut s, optrand, SPX_N as u64);
        blake512_update(&mut s, m, mlen);
        let mut digest = [0u8; 64];
        blake512_final(&mut s, &mut digest);
        r[..SPX_N].copy_from_slice(&digest[..SPX_N]);
    }
    #[cfg(any(feature = "128s", feature = "128f"))]
    pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
        let mut s = Blakestate256::new();
        blake256_init(&mut s);
        blake256_update(&mut s, sk_prf, SPX_N as u64);
        blake256_update(&mut s, optrand, SPX_N as u64);
        blake256_update(&mut s, m, mlen);
        let mut digest = [0u8; 32];
        blake256_final(&mut s, &mut digest);
        r[..SPX_N].copy_from_slice(&digest[..SPX_N]);
    }

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    const BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE512_OUTPUT_BYTES;
    #[cfg(any(feature = "128s", feature = "128f"))]
    const BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE256_OUTPUT_BYTES;

    pub fn hash_message(
        digest: &mut [u8],
        tree: &mut u64,
        leaf_idx: &mut u32,
        r: &[u8],
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

        let mut buf = vec![0u8; SPX_DGST_BYTES];
        let mut seed = vec![0u8; 2 * SPX_N + BLAKEX_OUTPUT_BYTES];

        // NOTE: same C bug as in gen_message_random — the datalen is treated
        // as bits internally but byte counts are passed.
        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        {
            let mut s = Blakestate512::new();
            blake512_init(&mut s);
            blake512_update(&mut s, r, SPX_N as u64);
            blake512_update(&mut s, pk, SPX_PK_BYTES as u64);
            blake512_update(&mut s, m, mlen);
            blake512_final(&mut s, &mut seed[2 * SPX_N..]);
        }
        #[cfg(any(feature = "128s", feature = "128f"))]
        {
            let mut s = Blakestate256::new();
            blake256_init(&mut s);
            blake256_update(&mut s, r, SPX_N as u64);
            blake256_update(&mut s, pk, SPX_PK_BYTES as u64);
            blake256_update(&mut s, m, mlen);
            blake256_final(&mut s, &mut seed[2 * SPX_N..]);
        }

        seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
        seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

        #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
        blake512_mgf1(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + BLAKEX_OUTPUT_BYTES);
        #[cfg(any(feature = "128s", feature = "128f"))]
        blake256_mgf1(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + BLAKEX_OUTPUT_BYTES);

        digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
        let mut bufp = SPX_FORS_MSG_BYTES;

        if SPX_D == 1 {
            *tree = 0;
        } else {
            *tree = crate::utils::bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
            *tree &= u64::MAX >> (64 - SPX_TREE_BITS);
        }
        bufp += SPX_TREE_BYTES;

        *leaf_idx = crate::utils::bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
        *leaf_idx &= u32::MAX >> (32 - SPX_LEAF_BITS);
    }
}

#[cfg(feature = "haraka")]
mod backend {
    use super::*;
    use crate::haraka::*;

    pub fn initialize_hash_function(ctx: &mut SpxCtx) {
        tweak_constants(ctx);
    }

    pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
        let mut outbuf = [0u8; 32];
        let mut buf = [0u8; 64];
        let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
        buf[..SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);
        buf[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);
        haraka512(&mut outbuf, &buf, ctx);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }

    pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], mlen: u64, ctx: &SpxCtx) {
        let mut s_inc = [0u8; 65];
        haraka_s_inc_init(&mut s_inc);
        haraka_s_inc_absorb(&mut s_inc, sk_prf, SPX_N, ctx);
        haraka_s_inc_absorb(&mut s_inc, optrand, SPX_N, ctx);
        haraka_s_inc_absorb(&mut s_inc, m, mlen as usize, ctx);
        haraka_s_inc_finalize(&mut s_inc);
        haraka_s_inc_squeeze(r, SPX_N, &mut s_inc, ctx);
    }

    pub fn hash_message(
        digest: &mut [u8],
        tree: &mut u64,
        leaf_idx: &mut u32,
        r: &[u8],
        pk: &[u8],
        m: &[u8],
        mlen: u64,
        ctx: &SpxCtx,
    ) {
        const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
        const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
        const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
        const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
        const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

        let mut buf = vec![0u8; SPX_DGST_BYTES];
        let mut s_inc = [0u8; 65];

        haraka_s_inc_init(&mut s_inc);
        haraka_s_inc_absorb(&mut s_inc, r, SPX_N, ctx);
        haraka_s_inc_absorb(&mut s_inc, &pk[SPX_N..], SPX_N, ctx);
        haraka_s_inc_absorb(&mut s_inc, m, mlen as usize, ctx);
        haraka_s_inc_finalize(&mut s_inc);
        haraka_s_inc_squeeze(&mut buf, SPX_DGST_BYTES, &mut s_inc, ctx);

        digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
        let mut bufp = SPX_FORS_MSG_BYTES;

        if SPX_D == 1 {
            *tree = 0;
        } else {
            *tree = crate::utils::bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
            *tree &= u64::MAX >> (64 - SPX_TREE_BITS);
        }
        bufp += SPX_TREE_BYTES;

        *leaf_idx = crate::utils::bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
        *leaf_idx &= u32::MAX >> (32 - SPX_LEAF_BITS);
    }
}

pub use backend::{gen_message_random, hash_message, initialize_hash_function, prf_addr};

// Note: we intentionally use `SPX_FORS_TREES` to silence the unused-import lint
// triggered for some feature combinations that don't reference it directly.
#[allow(dead_code)]
const _USES_FORS_TREES: usize = SPX_FORS_TREES;

// ---------- C-ABI exports ----------

#[unsafe(export_name = "SPX_initialize_hash_function")]
pub unsafe extern "C" fn spx_initialize_hash_function(ctx: *mut SpxCtx) {
    initialize_hash_function(unsafe { &mut *ctx });
}

#[unsafe(export_name = "SPX_prf_addr")]
pub unsafe extern "C" fn spx_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let out_slice = unsafe { core::slice::from_raw_parts_mut(out, SPX_N) };
    let addr_ref = unsafe { &*(addr as *const [u32; 8]) };
    prf_addr(out_slice, unsafe { &*ctx }, addr_ref);
}

#[unsafe(export_name = "SPX_gen_message_random")]
pub unsafe extern "C" fn spx_gen_message_random(
    r: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: core::ffi::c_ulonglong,
    ctx: *const SpxCtx,
) {
    let r_slice = unsafe { core::slice::from_raw_parts_mut(r, SPX_N) };
    let sk_prf_slice = unsafe { core::slice::from_raw_parts(sk_prf, SPX_N) };
    let optrand_slice = unsafe { core::slice::from_raw_parts(optrand, SPX_N) };
    let m_slice = unsafe { core::slice::from_raw_parts(m, mlen as usize) };
    gen_message_random(r_slice, sk_prf_slice, optrand_slice, m_slice, mlen, unsafe { &*ctx });
}

#[unsafe(export_name = "SPX_hash_message")]
pub unsafe extern "C" fn spx_hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    r: *const u8,
    pk: *const u8,
    m: *const u8,
    mlen: core::ffi::c_ulonglong,
    ctx: *const SpxCtx,
) {
    let digest_slice = unsafe { core::slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES) };
    let r_slice = unsafe { core::slice::from_raw_parts(r, SPX_N) };
    let pk_slice = unsafe { core::slice::from_raw_parts(pk, SPX_PK_BYTES) };
    let m_slice = unsafe { core::slice::from_raw_parts(m, mlen as usize) };
    hash_message(
        digest_slice,
        unsafe { &mut *tree },
        unsafe { &mut *leaf_idx },
        r_slice,
        pk_slice,
        m_slice,
        mlen,
        unsafe { &*ctx },
    );
}
