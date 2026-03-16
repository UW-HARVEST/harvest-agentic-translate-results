#![allow(
    non_snake_case,
    non_upper_case_globals,
    clippy::needless_range_loop,
    clippy::identity_op,
    clippy::manual_memcpy,
    clippy::too_many_arguments
)]

use std::ptr;

// ============================================================
// Parameters (SHA2-128f)
// ============================================================
const SPX_N: usize = 16;
const SPX_FULL_HEIGHT: usize = 66;
const SPX_D: usize = 22;
const SPX_FORS_HEIGHT: usize = 6;
const SPX_FORS_TREES: usize = 33;
const SPX_WOTS_W: usize = 16;
const SPX_WOTS_LOGW: usize = 4;
const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW; // 32
const SPX_WOTS_LEN2: usize = 3;
const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2; // 35
const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;
const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D; // 3
const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8; // 25
const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N; // 3696
const SPX_BYTES: usize =
    SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N; // 17088
const SPX_PK_BYTES: usize = 2 * SPX_N;
const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;
const SPX_ADDR_BYTES: usize = 32;

const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
const CRYPTO_BYTES: usize = SPX_BYTES;
const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

// SHA2 offsets
const SPX_OFFSET_LAYER: usize = 0;
const SPX_OFFSET_TREE: usize = 1;
const SPX_OFFSET_TYPE: usize = 9;
const SPX_OFFSET_KP_ADDR: usize = 10;
const SPX_OFFSET_CHAIN_ADDR: usize = 17;
const SPX_OFFSET_HASH_ADDR: usize = 21;
const SPX_OFFSET_TREE_HGT: usize = 17;
const SPX_OFFSET_TREE_INDEX: usize = 18;

// SHA2 constants
const SPX_SHA256_BLOCK_BYTES: usize = 64;
const SPX_SHA256_OUTPUT_BYTES: usize = 32;
const SPX_SHA256_ADDR_BYTES: usize = 22;

// Address types
const SPX_ADDR_TYPE_WOTS: u32 = 0;
const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
const SPX_ADDR_TYPE_FORSPK: u32 = 4;
const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

// For SHA2-128f: SPX_N < 24, so we use SHA-256 as shaX
const SPX_SHAX_OUTPUT_BYTES: usize = SPX_SHA256_OUTPUT_BYTES;
const SPX_SHAX_BLOCK_BYTES: usize = SPX_SHA256_BLOCK_BYTES;

// hash_message derived constants
const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1); // 63
const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8; // 8
const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT; // 3
const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8; // 1
const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES; // 34
const SPX_INBLOCKS: usize =
    ((SPX_N + SPX_PK_BYTES + SPX_SHAX_BLOCK_BYTES - 1) & !(SPX_SHAX_BLOCK_BYTES - 1))
        / SPX_SHAX_BLOCK_BYTES; // 1

// ============================================================
// Context struct
// ============================================================
#[repr(C)]
struct SpxCtx {
    pub_seed: [u8; SPX_N],
    sk_seed: [u8; SPX_N],
    state_seeded: [u8; 40],
}

// ============================================================
// SHA-256 implementation
// ============================================================
fn load_bigendian_32(x: &[u8]) -> u32 {
    (x[3] as u32) | ((x[2] as u32) << 8) | ((x[1] as u32) << 16) | ((x[0] as u32) << 24)
}

fn load_bigendian_64(x: &[u8]) -> u64 {
    (x[7] as u64)
        | ((x[6] as u64) << 8)
        | ((x[5] as u64) << 16)
        | ((x[4] as u64) << 24)
        | ((x[3] as u64) << 32)
        | ((x[2] as u64) << 40)
        | ((x[1] as u64) << 48)
        | ((x[0] as u64) << 56)
}

fn store_bigendian_32(x: &mut [u8], mut u: u64) {
    x[3] = u as u8;
    u >>= 8;
    x[2] = u as u8;
    u >>= 8;
    x[1] = u as u8;
    u >>= 8;
    x[0] = u as u8;
}

fn store_bigendian_64(x: &mut [u8], mut u: u64) {
    x[7] = u as u8;
    u >>= 8;
    x[6] = u as u8;
    u >>= 8;
    x[5] = u as u8;
    u >>= 8;
    x[4] = u as u8;
    u >>= 8;
    x[3] = u as u8;
    u >>= 8;
    x[2] = u as u8;
    u >>= 8;
    x[1] = u as u8;
    u >>= 8;
    x[0] = u as u8;
}

macro_rules! rotr32 {
    ($x:expr, $c:expr) => {
        ($x >> $c) | ($x << (32 - $c))
    };
}

#[inline(always)]
fn ch32(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (!x & z)
}
#[inline(always)]
fn maj32(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (x & z) ^ (y & z)
}
#[inline(always)]
fn sigma0_32(x: u32) -> u32 {
    rotr32!(x, 2) ^ rotr32!(x, 13) ^ rotr32!(x, 22)
}
#[inline(always)]
fn sigma1_32(x: u32) -> u32 {
    rotr32!(x, 6) ^ rotr32!(x, 11) ^ rotr32!(x, 25)
}
#[inline(always)]
fn lsigma0_32(x: u32) -> u32 {
    rotr32!(x, 7) ^ rotr32!(x, 18) ^ (x >> 3)
}
#[inline(always)]
fn lsigma1_32(x: u32) -> u32 {
    rotr32!(x, 17) ^ rotr32!(x, 19) ^ (x >> 10)
}

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
    0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
    0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
    0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
    0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
    0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
    0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
    0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
    0xc67178f2,
];

fn crypto_hashblocks_sha256(statebytes: &mut [u8], input: &[u8], mut inlen: usize) -> usize {
    let mut state = [0u32; 8];
    for i in 0..8 {
        state[i] = load_bigendian_32(&statebytes[i * 4..]);
    }
    let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h) = (
        state[0], state[1], state[2], state[3], state[4], state[5], state[6], state[7],
    );

    let mut offset = 0usize;
    while inlen >= 64 {
        let mut w = [0u32; 16];
        for i in 0..16 {
            w[i] = load_bigendian_32(&input[offset + i * 4..]);
        }

        for round in 0..64 {
            if round >= 16 {
                let idx = round & 15;
                w[idx] = lsigma1_32(w[(round - 2) & 15])
                    .wrapping_add(w[(round - 7) & 15])
                    .wrapping_add(lsigma0_32(w[(round - 15) & 15]))
                    .wrapping_add(w[idx]);
            }
            let wi = w[round & 15];
            let t1 = h
                .wrapping_add(sigma1_32(e))
                .wrapping_add(ch32(e, f, g))
                .wrapping_add(SHA256_K[round])
                .wrapping_add(wi);
            let t2 = sigma0_32(a).wrapping_add(maj32(a, b, c));
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }

        a = a.wrapping_add(state[0]);
        b = b.wrapping_add(state[1]);
        c = c.wrapping_add(state[2]);
        d = d.wrapping_add(state[3]);
        e = e.wrapping_add(state[4]);
        f = f.wrapping_add(state[5]);
        g = g.wrapping_add(state[6]);
        h = h.wrapping_add(state[7]);

        state[0] = a;
        state[1] = b;
        state[2] = c;
        state[3] = d;
        state[4] = e;
        state[5] = f;
        state[6] = g;
        state[7] = h;

        offset += 64;
        inlen -= 64;
    }

    for i in 0..8 {
        store_bigendian_32(&mut statebytes[i * 4..], state[i] as u64);
    }
    inlen
}

static IV_256: [u8; 32] = [
    0x6a, 0x09, 0xe6, 0x67, 0xbb, 0x67, 0xae, 0x85, 0x3c, 0x6e, 0xf3, 0x72, 0xa5, 0x4f, 0xf5,
    0x3a, 0x51, 0x0e, 0x52, 0x7f, 0x9b, 0x05, 0x68, 0x8c, 0x1f, 0x83, 0xd9, 0xab, 0x5b, 0xe0,
    0xcd, 0x19,
];

fn sha256_inc_init(state: &mut [u8]) {
    state[..32].copy_from_slice(&IV_256);
    for i in 32..40 {
        state[i] = 0;
    }
}

fn sha256_inc_blocks(state: &mut [u8], input: &[u8], inblocks: usize) {
    let mut bytes = load_bigendian_64(&state[32..]);
    crypto_hashblocks_sha256(state, input, 64 * inblocks);
    bytes += (64 * inblocks) as u64;
    store_bigendian_64(&mut state[32..], bytes);
}

fn sha256_inc_finalize(out: &mut [u8], state: &mut [u8], input: &[u8], inlen: usize) {
    let mut padded = [0u8; 128];
    let bytes = load_bigendian_64(&state[32..]) + inlen as u64;

    crypto_hashblocks_sha256(state, input, inlen);
    let remaining = inlen & 63;
    let start = inlen - remaining;

    for i in 0..remaining {
        padded[i] = input[start + i];
    }
    padded[remaining] = 0x80;

    if remaining < 56 {
        for i in (remaining + 1)..56 {
            padded[i] = 0;
        }
        padded[56] = (bytes >> 53) as u8;
        padded[57] = (bytes >> 45) as u8;
        padded[58] = (bytes >> 37) as u8;
        padded[59] = (bytes >> 29) as u8;
        padded[60] = (bytes >> 21) as u8;
        padded[61] = (bytes >> 13) as u8;
        padded[62] = (bytes >> 5) as u8;
        padded[63] = (bytes << 3) as u8;
        crypto_hashblocks_sha256(state, &padded, 64);
    } else {
        for i in (remaining + 1)..120 {
            padded[i] = 0;
        }
        padded[120] = (bytes >> 53) as u8;
        padded[121] = (bytes >> 45) as u8;
        padded[122] = (bytes >> 37) as u8;
        padded[123] = (bytes >> 29) as u8;
        padded[124] = (bytes >> 21) as u8;
        padded[125] = (bytes >> 13) as u8;
        padded[126] = (bytes >> 5) as u8;
        padded[127] = (bytes << 3) as u8;
        crypto_hashblocks_sha256(state, &padded, 128);
    }

    out[..32].copy_from_slice(&state[..32]);
}

fn sha256(out: &mut [u8], input: &[u8], inlen: usize) {
    let mut state = [0u8; 40];
    sha256_inc_init(&mut state);
    sha256_inc_finalize(out, &mut state, input, inlen);
}

// ============================================================
// Utility functions
// ============================================================
fn ull_to_bytes(out: &mut [u8], outlen: usize, mut val: u64) {
    for i in (0..outlen).rev() {
        out[i] = (val & 0xff) as u8;
        val >>= 8;
    }
}

fn u32_to_bytes(out: &mut [u8], val: u32) {
    out[0] = (val >> 24) as u8;
    out[1] = (val >> 16) as u8;
    out[2] = (val >> 8) as u8;
    out[3] = val as u8;
}

fn bytes_to_ull(input: &[u8], inlen: usize) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (input[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

// ============================================================
// MGF1-SHA256
// ============================================================
fn mgf1_256(out: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    inbuf[..inlen].copy_from_slice(&input[..inlen]);

    let mut i: usize = 0;
    let mut off = 0usize;
    while (i + 1) * SPX_SHA256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha256(&mut out[off..], &inbuf, inlen + 4);
        off += SPX_SHA256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_SHA256_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha256(&mut outbuf, &inbuf, inlen + 4);
        let rem = outlen - i * SPX_SHA256_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

// ============================================================
// seed_state
// ============================================================
fn seed_state(ctx: &mut SpxCtx) {
    let mut block = [0u8; 128]; // SPX_SHA512_BLOCK_BYTES but we only use 64
    block[..SPX_N].copy_from_slice(&ctx.pub_seed);
    // rest already zero

    sha256_inc_init(&mut ctx.state_seeded);
    sha256_inc_blocks(&mut ctx.state_seeded, &block, 1);
}

// ============================================================
// Address functions
// ============================================================
fn addr_as_bytes(addr: &[u32; 8]) -> &[u8; SPX_ADDR_BYTES] {
    unsafe { &*(addr as *const [u32; 8] as *const [u8; SPX_ADDR_BYTES]) }
}

fn addr_as_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8; SPX_ADDR_BYTES] {
    unsafe { &mut *(addr as *mut [u32; 8] as *mut [u8; SPX_ADDR_BYTES]) }
}

fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_LAYER] = layer as u8;
}

fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    let bytes = addr_as_bytes_mut(addr);
    ull_to_bytes(&mut bytes[SPX_OFFSET_TREE..], 8, tree);
}

fn set_type(addr: &mut [u32; 8], type_val: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_TYPE] = type_val as u8;
}

fn copy_subtree_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let src = addr_as_bytes(inp);
    let dst = addr_as_bytes_mut(out);
    dst[..SPX_OFFSET_TREE + 8].copy_from_slice(&src[..SPX_OFFSET_TREE + 8]);
}

fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    let bytes = addr_as_bytes_mut(addr);
    u32_to_bytes(&mut bytes[SPX_OFFSET_KP_ADDR..], keypair);
}

fn copy_keypair_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let src = addr_as_bytes(inp);
    let dst = addr_as_bytes_mut(out);
    dst[..SPX_OFFSET_TREE + 8].copy_from_slice(&src[..SPX_OFFSET_TREE + 8]);
    dst[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&src[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

fn set_chain_addr(addr: &mut [u32; 8], chain: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

fn set_hash_addr(addr: &mut [u32; 8], hash: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

fn set_tree_height(addr: &mut [u32; 8], tree_height: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

fn set_tree_index(addr: &mut [u32; 8], tree_index: u32) {
    let bytes = addr_as_bytes_mut(addr);
    u32_to_bytes(&mut bytes[SPX_OFFSET_TREE_INDEX..], tree_index);
}

// ============================================================
// Hash functions (hash_sha2.c)
// ============================================================
fn initialize_hash_function(ctx: &mut SpxCtx) {
    seed_state(ctx);
}

fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    sha2_state.copy_from_slice(&ctx.state_seeded);

    let addr_bytes = addr_as_bytes(addr);
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

fn gen_message_random(
    r_out: &mut [u8],
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

    sha256_inc_init(&mut state);
    sha256_inc_blocks(&mut state, &buf, 1);

    buf[..SPX_N].copy_from_slice(&optrand[..SPX_N]);

    let mlen_usize = mlen as usize;
    if SPX_N + mlen_usize < SPX_SHAX_BLOCK_BYTES {
        buf[SPX_N..SPX_N + mlen_usize].copy_from_slice(&m[..mlen_usize]);
        let input_copy: Vec<u8> = buf[..SPX_N + mlen_usize].to_vec();
        sha256_inc_finalize(
            &mut buf[SPX_SHAX_BLOCK_BYTES..],
            &mut state,
            &input_copy,
            mlen_usize + SPX_N,
        );
    } else {
        let fill = SPX_SHAX_BLOCK_BYTES - SPX_N;
        buf[SPX_N..SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..fill]);
        // Need a copy for sha256_inc_blocks since buf is borrowed
        let block_copy: Vec<u8> = buf[..SPX_SHAX_BLOCK_BYTES].to_vec();
        sha256_inc_blocks(&mut state, &block_copy, 1);

        let m_rest = &m[fill..];
        let mlen_rest = mlen_usize - fill;
        sha256_inc_finalize(
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
    sha256(
        &mut buf,
        &buf_copy,
        SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES,
    );
    r_out[..SPX_N].copy_from_slice(&buf[..SPX_N]);
}

fn hash_message(
    digest: &mut [u8],
    tree: &mut u64,
    leaf_idx: &mut u32,
    r: &[u8],
    pk: &[u8],
    m: &[u8],
    mlen: u64,
    _ctx: &SpxCtx,
) {
    let mut seed = [0u8; 2 * SPX_N + SPX_SHAX_OUTPUT_BYTES];
    let mut inbuf = [0u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    sha256_inc_init(&mut state);

    inbuf[..SPX_N].copy_from_slice(&r[..SPX_N]);
    inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);

    let mlen_usize = mlen as usize;
    if SPX_N + SPX_PK_BYTES + mlen_usize < SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES {
        inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + mlen_usize]
            .copy_from_slice(&m[..mlen_usize]);
        sha256_inc_finalize(
            &mut seed[2 * SPX_N..],
            &mut state,
            &inbuf,
            SPX_N + SPX_PK_BYTES + mlen_usize,
        );
    } else {
        let fill = SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N + SPX_PK_BYTES..SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES]
            .copy_from_slice(&m[..fill]);
        sha256_inc_blocks(&mut state, &inbuf, SPX_INBLOCKS);

        let m_rest = &m[fill..];
        let mlen_rest = mlen_usize - fill;
        sha256_inc_finalize(&mut seed[2 * SPX_N..], &mut state, m_rest, mlen_rest);
    }

    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    mgf1_256(
        &mut buf,
        SPX_DGST_BYTES,
        &seed,
        2 * SPX_N + SPX_SHAX_OUTPUT_BYTES,
    );

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);

    let bufp = &buf[SPX_FORS_MSG_BYTES..];
    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(bufp, SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }

    let bufp2 = &buf[SPX_FORS_MSG_BYTES + SPX_TREE_BYTES..];
    *leaf_idx = bytes_to_ull(bufp2, SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}

// ============================================================
// thash (robust, SHA-256 only since SPX_SHA512=0)
// ============================================================
fn thash(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_OUTPUT_BYTES + inblocks * SPX_N];
    let mut sha2_state = [0u8; 40];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = addr_as_bytes(addr);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES]
        .copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    mgf1_256(
        &mut bitmask,
        inblocks * SPX_N,
        &buf,
        SPX_N + SPX_SHA256_ADDR_BYTES,
    );

    sha2_state.copy_from_slice(&ctx.state_seeded);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = input[i] ^ bitmask[i];
    }

    sha256_inc_finalize(
        &mut outbuf,
        &mut sha2_state,
        &buf[SPX_N..],
        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// ============================================================
// WOTS
// ============================================================
fn gen_chain(
    out: &mut [u8],
    input: &[u8],
    start: u32,
    steps: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    out[..SPX_N].copy_from_slice(&input[..SPX_N]);
    let mut i = start;
    while i < start + steps && i < SPX_WOTS_W as u32 {
        set_hash_addr(addr, i);
        let mut tmp = [0u8; SPX_N];
        tmp.copy_from_slice(&out[..SPX_N]);
        thash(out, &tmp, 1, ctx, addr);
        i += 1;
    }
}

fn base_w(output: &mut [u32], out_len: usize, input: &[u8]) {
    let mut in_idx = 0usize;
    let mut out_idx = 0usize;
    let mut total: u8 = 0;
    let mut bits: i32 = 0;

    for _ in 0..out_len {
        if bits == 0 {
            total = input[in_idx];
            in_idx += 1;
            bits += 8;
        }
        bits -= SPX_WOTS_LOGW as i32;
        output[out_idx] = ((total >> bits) & (SPX_WOTS_W as u8 - 1)) as u32;
        out_idx += 1;
    }
}

fn wots_checksum(csum_base_w: &mut [u32], msg_base_w: &[u32]) {
    let mut csum: u32 = 0;
    let csum_bytes_len = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;
    let mut csum_bytes = [0u8; 4]; // enough for our params

    for i in 0..SPX_WOTS_LEN1 {
        csum += (SPX_WOTS_W as u32) - 1 - msg_base_w[i];
    }

    csum <<= (8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8;
    ull_to_bytes(&mut csum_bytes, csum_bytes_len, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}

fn chain_lengths(lengths: &mut [u32; SPX_WOTS_LEN], msg: &[u8]) {
    base_w(&mut lengths[..], SPX_WOTS_LEN1, msg);
    let (msg_part, csum_part) = lengths.split_at_mut(SPX_WOTS_LEN1);
    wots_checksum(csum_part, msg_part);
}

fn wots_pk_from_sig(pk: &mut [u8], sig: &[u8], msg: &[u8], ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut lengths = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut lengths, msg);

    for i in 0..SPX_WOTS_LEN {
        set_chain_addr(addr, i as u32);
        gen_chain(
            &mut pk[i * SPX_N..],
            &sig[i * SPX_N..],
            lengths[i],
            (SPX_WOTS_W as u32) - 1 - lengths[i],
            ctx,
            addr,
        );
    }
}

// ============================================================
// WOTS x1 (wotsx1.c)
// ============================================================
struct LeafInfoX1 {
    wots_sig: *mut u8,
    wots_sign_leaf: u32,
    wots_steps: *const u32,
    leaf_addr: [u32; 8],
    pk_addr: [u32; 8],
}

fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, info: &mut LeafInfoX1) {
    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf {
        0
    } else {
        !0u32
    };

    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);

    for i in 0..SPX_WOTS_LEN {
        let wots_k = unsafe { *info.wots_steps.add(i) } | wots_k_mask;
        let buffer = &mut pk_buffer[i * SPX_N..(i + 1) * SPX_N];

        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        prf_addr(buffer, ctx, &info.leaf_addr);

        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0u32.. {
            if k == wots_k {
                if !info.wots_sig.is_null() {
                    unsafe {
                        ptr::copy_nonoverlapping(
                            buffer.as_ptr(),
                            info.wots_sig.add(i * SPX_N),
                            SPX_N,
                        );
                    }
                }
            }
            if k == (SPX_WOTS_W as u32) - 1 {
                break;
            }
            set_hash_addr(&mut info.leaf_addr, k);
            let mut tmp = [0u8; SPX_N];
            tmp.copy_from_slice(buffer);
            thash(buffer, &tmp, 1, ctx, &mut info.leaf_addr);
        }
    }

    thash(dest, &pk_buffer, SPX_WOTS_LEN, ctx, &mut info.pk_addr);
}

// ============================================================
// FORS
// ============================================================
fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, fors_leaf_addr: &[u32; 8]) {
    prf_addr(sk, ctx, fors_leaf_addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, fors_leaf_addr: &mut [u32; 8]) {
    thash(leaf, sk, 1, ctx, fors_leaf_addr);
}

#[repr(C)]
struct ForsGenLeafInfo {
    leaf_addrx: [u32; 8],
}

fn fors_gen_leafx1(
    leaf: &mut [u8],
    ctx: &SpxCtx,
    addr_idx: u32,
    info: &mut ForsGenLeafInfo,
) {
    set_tree_index(&mut info.leaf_addrx, addr_idx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf, ctx, &info.leaf_addrx);

    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSTREE);
    let tmp: Vec<u8> = leaf[..SPX_N].to_vec();
    fors_sk_to_leaf(leaf, &tmp, ctx, &mut info.leaf_addrx);
}

fn message_to_indices(indices: &mut [u32; SPX_FORS_TREES], m: &[u8]) {
    let mut offset: usize = 0;
    for i in 0..SPX_FORS_TREES {
        indices[i] = 0;
        for j in 0..SPX_FORS_HEIGHT {
            indices[i] ^= (((m[offset >> 3] >> (offset & 0x7)) & 1) as u32) << j;
            offset += 1;
        }
    }
}

fn fors_sign(
    sig: &mut [u8],
    pk: &mut [u8],
    m: &[u8],
    ctx: &SpxCtx,
    fors_addr: &[u32; 8],
) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_info = ForsGenLeafInfo {
        leaf_addrx: [0u32; 8],
    };
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr(&mut fors_info.leaf_addrx, fors_addr);
    copy_keypair_addr(&mut fors_pk_addr, fors_addr);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_off = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSPRF);

        fors_gen_sk(&mut sig[sig_off..], ctx, &fors_tree_addr);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
        sig_off += SPX_N;

        fors_treehashx1(
            &mut roots[i * SPX_N..],
            &mut sig[sig_off..],
            ctx,
            indices[i],
            idx_offset,
            SPX_FORS_HEIGHT as u32,
            &mut fors_tree_addr,
            &mut fors_info,
        );

        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    thash(pk, &roots, SPX_FORS_TREES, ctx, &mut fors_pk_addr);
}

fn fors_pk_from_sig(
    pk: &mut [u8],
    sig: &[u8],
    m: &[u8],
    ctx: &SpxCtx,
    fors_addr: &[u32; 8],
) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_pk_addr = [0u32; 8];

    copy_keypair_addr(&mut fors_tree_addr, fors_addr);
    copy_keypair_addr(&mut fors_pk_addr, fors_addr);

    set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
    set_type(&mut fors_pk_addr, SPX_ADDR_TYPE_FORSPK);

    message_to_indices(&mut indices, m);

    let mut sig_off = 0usize;
    for i in 0..SPX_FORS_TREES {
        let idx_offset = (i as u32) * (1u32 << SPX_FORS_HEIGHT);

        set_tree_height(&mut fors_tree_addr, 0);
        set_tree_index(&mut fors_tree_addr, indices[i] + idx_offset);

        fors_sk_to_leaf(&mut leaf, &sig[sig_off..], ctx, &mut fors_tree_addr);
        sig_off += SPX_N;

        compute_root(
            &mut roots[i * SPX_N..],
            &leaf,
            indices[i],
            idx_offset,
            &sig[sig_off..],
            SPX_TREE_HEIGHT as u32,
            ctx,
            &mut fors_tree_addr,
        );
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    thash(pk, &roots, SPX_FORS_TREES, ctx, &mut fors_pk_addr);
}

// ============================================================
// Tree hash functions (utilsx1.c / utils.c)
// ============================================================
fn compute_root(
    root: &mut [u8],
    leaf: &[u8],
    mut leaf_idx: u32,
    mut idx_offset: u32,
    auth_path: &[u8],
    tree_height: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let mut buffer = [0u8; 2 * SPX_N];
    let mut auth_off = 0usize;

    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
    }
    auth_off += SPX_N;

    for i in 0..(tree_height - 1) {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height(addr, i + 1);
        set_tree_index(addr, leaf_idx + idx_offset);

        if leaf_idx & 1 != 0 {
            let tmp = buffer.clone();
            thash(&mut buffer[SPX_N..], &tmp, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
        } else {
            let tmp = buffer.clone();
            thash(&mut buffer[..SPX_N], &tmp, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N]
                .copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
        }
        auth_off += SPX_N;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height(addr, tree_height);
    set_tree_index(addr, leaf_idx + idx_offset);
    thash(root, &buffer, 2, ctx, addr);
}

fn wots_treehashx1(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    info: &mut LeafInfoX1,
) {
    let th = tree_height as usize;
    let mut stack = vec![0u8; th * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    let mut idx: u32 = 0;
    loop {
        let mut current = [0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        let mut h: u32 = 0;
        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..SPX_N * 2]);
                return;
            }

            if (internal_idx ^ internal_leaf) == 0x01 {
                auth_path[h as usize * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..SPX_N * 2]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            current[..SPX_N].copy_from_slice(&stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]);
            let tmp = current.clone();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]
            .copy_from_slice(&current[SPX_N..SPX_N * 2]);
        idx += 1;
    }
}

fn fors_treehashx1(
    root: &mut [u8],
    auth_path: &mut [u8],
    ctx: &SpxCtx,
    leaf_idx: u32,
    idx_offset: u32,
    tree_height: u32,
    tree_addr: &mut [u32; 8],
    info: &mut ForsGenLeafInfo,
) {
    let th = tree_height as usize;
    let mut stack = vec![0u8; th * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    let mut idx: u32 = 0;
    loop {
        let mut current = [0u8; 2 * SPX_N];
        fors_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        let mut h: u32 = 0;
        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..SPX_N * 2]);
                return;
            }

            if (internal_idx ^ internal_leaf) == 0x01 {
                auth_path[h as usize * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..SPX_N * 2]);
            }

            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }

            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            current[..SPX_N].copy_from_slice(&stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]);
            let tmp = current.clone();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]
            .copy_from_slice(&current[SPX_N..SPX_N * 2]);
        idx += 1;
    }
}

// ============================================================
// Merkle
// ============================================================
fn merkle_sign(
    sig: &mut [u8],
    root: &mut [u8],
    ctx: &SpxCtx,
    wots_addr: &mut [u32; 8],
    tree_addr: &mut [u32; 8],
    idx_leaf: u32,
) {
    let sig_ptr = sig.as_mut_ptr();
    let auth_path_ptr = unsafe { sig_ptr.add(SPX_WOTS_BYTES) };
    let auth_path_len = sig.len() - SPX_WOTS_BYTES;
    let auth_path = unsafe { std::slice::from_raw_parts_mut(auth_path_ptr, auth_path_len) };
    let mut info = LeafInfoX1 {
        wots_sig: sig_ptr,
        wots_sign_leaf: idx_leaf,
        wots_steps: ptr::null(),
        leaf_addr: [0u32; 8],
        pk_addr: [0u32; 8],
    };
    let mut steps = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut steps, root);
    info.wots_steps = steps.as_ptr();

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr(&mut info.pk_addr, wots_addr);

    wots_treehashx1(
        root,
        auth_path,
        ctx,
        idx_leaf,
        0,
        SPX_TREE_HEIGHT as u32,
        tree_addr,
        &mut info,
    );
}

fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];

    set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr(&mut wots_addr, (SPX_D - 1) as u32);

    merkle_sign(
        &mut auth_path,
        root,
        ctx,
        &mut wots_addr,
        &mut top_tree_addr,
        !0u32,
    );
}

// ============================================================
// randombytes
// ============================================================
fn randombytes(x: &mut [u8], xlen: usize) {
    use std::fs::File;
    use std::io::Read;
    let mut f = File::open("/dev/urandom").expect("Failed to open /dev/urandom");
    f.read_exact(&mut x[..xlen]).expect("Failed to read random bytes");
}

// ============================================================
// Public C API (sign.c)
// ============================================================
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> u64 {
    CRYPTO_SECRETKEYBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> u64 {
    CRYPTO_PUBLICKEYBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> u64 {
    CRYPTO_BYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> u64 {
    CRYPTO_SEEDBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> i32 {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES) };
    let seed = unsafe { std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };

    let mut ctx = SpxCtx {
        pub_seed: [0u8; SPX_N],
        sk_seed: [0u8; SPX_N],
        state_seeded: [0u8; 40],
    };

    sk[..CRYPTO_SEEDBYTES].copy_from_slice(seed);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);

    initialize_hash_function(&mut ctx);

    merkle_gen_root(&mut sk[3 * SPX_N..], &ctx);

    pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    randombytes(&mut seed, CRYPTO_SEEDBYTES);
    crypto_sign_seed_keypair(pk, sk, seed.as_ptr());
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> i32 {
    let sig_slice = unsafe { std::slice::from_raw_parts_mut(sig, SPX_BYTES) };
    let m_slice = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sk_slice = unsafe { std::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES) };

    let mut ctx = SpxCtx {
        pub_seed: [0u8; SPX_N],
        sk_seed: [0u8; SPX_N],
        state_seeded: [0u8; 40],
    };

    let sk_prf = &sk_slice[SPX_N..2 * SPX_N];
    let pk = &sk_slice[2 * SPX_N..];

    let mut optrand = [0u8; SPX_N];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut root = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];

    ctx.sk_seed.copy_from_slice(&sk_slice[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);

    initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    randombytes(&mut optrand, SPX_N);
    gen_message_random(sig_slice, sk_prf, &optrand, m_slice, mlen as u64, &ctx);

    hash_message(
        &mut mhash,
        &mut tree,
        &mut idx_leaf,
        sig_slice,
        pk,
        m_slice,
        mlen as u64,
        &ctx,
    );

    let mut sig_off = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_sign(&mut sig_slice[sig_off..], &mut root, &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        merkle_sign(
            &mut sig_slice[sig_off..],
            &mut root,
            &ctx,
            &mut wots_addr,
            &mut tree_addr,
            idx_leaf,
        );
        sig_off += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    unsafe {
        *siglen = SPX_BYTES;
    }

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> i32 {
    let pk_slice = unsafe { std::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES) };

    if siglen != SPX_BYTES {
        return -1;
    }

    let sig_slice = unsafe { std::slice::from_raw_parts(sig, SPX_BYTES) };
    let m_slice = unsafe { std::slice::from_raw_parts(m, mlen) };

    let mut ctx = SpxCtx {
        pub_seed: [0u8; SPX_N],
        sk_seed: [0u8; SPX_N],
        state_seeded: [0u8; 40],
    };

    let pub_root = &pk_slice[SPX_N..];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk = [0u8; SPX_WOTS_BYTES];
    let mut root = [0u8; SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];

    ctx.pub_seed.copy_from_slice(&pk_slice[..SPX_N]);

    initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    hash_message(
        &mut mhash,
        &mut tree,
        &mut idx_leaf,
        sig_slice,
        pk_slice,
        m_slice,
        mlen as u64,
        &ctx,
    );

    let mut sig_off = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_pk_from_sig(&mut root, &sig_slice[sig_off..], &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);

        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        wots_pk_from_sig(
            &mut wots_pk,
            &sig_slice[sig_off..],
            &root,
            &ctx,
            &mut wots_addr,
        );
        sig_off += SPX_WOTS_BYTES;

        thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &mut wots_pk_addr);

        compute_root(
            &mut root,
            &leaf,
            idx_leaf,
            0,
            &sig_slice[sig_off..],
            SPX_TREE_HEIGHT as u32,
            &ctx,
            &mut tree_addr,
        );
        sig_off += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    if root[..SPX_N] != pub_root[..SPX_N] {
        return -1;
    }

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> i32 {
    let mut siglen: usize = 0;

    crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);

    unsafe {
        ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
        *smlen = siglen as u64 + mlen;
    }

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> i32 {
    if (smlen as usize) < SPX_BYTES {
        unsafe {
            ptr::write_bytes(m, 0, smlen as usize);
            *mlen = 0;
        }
        return -1;
    }

    let msg_len = smlen as usize - SPX_BYTES;
    unsafe {
        *mlen = msg_len as u64;
    }

    if crypto_sign_verify(sm, SPX_BYTES, unsafe { sm.add(SPX_BYTES) }, msg_len, pk) != 0 {
        unsafe {
            ptr::write_bytes(m, 0, smlen as usize);
            *mlen = 0;
        }
        return -1;
    }

    unsafe {
        ptr::copy(sm.add(SPX_BYTES), m, msg_len);
    }

    0
}
