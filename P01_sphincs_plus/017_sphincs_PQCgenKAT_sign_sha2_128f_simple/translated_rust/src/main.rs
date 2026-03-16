#![allow(non_snake_case, non_upper_case_globals, unused_assignments, clippy::needless_range_loop)]

use openssl::symm::{Cipher, Crypter, Mode};

// ============================================================
// params (params-sphincs-sha2-128f.h + sha2_offsets.h)
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
const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;
const SPX_BYTES: usize = SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
const SPX_PK_BYTES: usize = 2 * SPX_N;
const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;
const SPX_ADDR_BYTES: usize = 32;

const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
const CRYPTO_BYTES: usize = SPX_BYTES;
const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;
const CRYPTO_ALGNAME: &[u8] = b"SPHINCS+";

// SHA2 offsets
const SPX_OFFSET_LAYER: usize = 0;
const SPX_OFFSET_TREE: usize = 1;
const SPX_OFFSET_TYPE: usize = 9;
const SPX_OFFSET_KP_ADDR: usize = 10;
const SPX_OFFSET_CHAIN_ADDR: usize = 17;
const SPX_OFFSET_HASH_ADDR: usize = 21;
const SPX_OFFSET_TREE_HGT: usize = 17;
const SPX_OFFSET_TREE_INDEX: usize = 18;

// Address types
const SPX_ADDR_TYPE_WOTS: u32 = 0;
const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
const SPX_ADDR_TYPE_FORSPK: u32 = 4;
const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

// SHA2 constants
const SPX_SHA256_BLOCK_BYTES: usize = 64;
const SPX_SHA256_OUTPUT_BYTES: usize = 32;
const SPX_SHA256_ADDR_BYTES: usize = 22;
const SPX_SHA512: bool = false; // SPX_N < 24

// For SHA2-128f, SPX_N < 24, so we use SHA-256 for message hashing
const SPX_SHAX_OUTPUT_BYTES: usize = SPX_SHA256_OUTPUT_BYTES;
const SPX_SHAX_BLOCK_BYTES: usize = SPX_SHA256_BLOCK_BYTES;

// ============================================================
// spx_ctx
// ============================================================
#[derive(Clone)]
struct SpxCtx {
    pub_seed: [u8; SPX_N],
    sk_seed: [u8; SPX_N],
    state_seeded: [u8; 40],
}

// ============================================================
// SHA-256 implementation (from sha2.c)
// ============================================================
fn load_bigendian_32(x: &[u8]) -> u32 {
    (x[3] as u32) | ((x[2] as u32) << 8) | ((x[1] as u32) << 16) | ((x[0] as u32) << 24)
}

fn load_bigendian_64(x: &[u8]) -> u64 {
    (x[7] as u64) | ((x[6] as u64) << 8) | ((x[5] as u64) << 16) | ((x[4] as u64) << 24)
        | ((x[3] as u64) << 32) | ((x[2] as u64) << 40) | ((x[1] as u64) << 48) | ((x[0] as u64) << 56)
}

fn store_bigendian_32(x: &mut [u8], mut u: u64) {
    x[3] = u as u8; u >>= 8;
    x[2] = u as u8; u >>= 8;
    x[1] = u as u8; u >>= 8;
    x[0] = u as u8;
}

fn store_bigendian_64(x: &mut [u8], mut u: u64) {
    x[7] = u as u8; u >>= 8;
    x[6] = u as u8; u >>= 8;
    x[5] = u as u8; u >>= 8;
    x[4] = u as u8; u >>= 8;
    x[3] = u as u8; u >>= 8;
    x[2] = u as u8; u >>= 8;
    x[1] = u as u8; u >>= 8;
    x[0] = u as u8;
}

fn crypto_hashblocks_sha256(statebytes: &mut [u8], inp: &[u8], mut inlen: usize) -> usize {
    let mut state = [0u32; 8];
    for i in 0..8 {
        state[i] = load_bigendian_32(&statebytes[4*i..]);
    }
    let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h) =
        (state[0], state[1], state[2], state[3], state[4], state[5], state[6], state[7]);

    let mut off = 0usize;
    while inlen >= 64 {
        let data = &inp[off..];
        let mut w = [0u32; 16];
        for i in 0..16 { w[i] = load_bigendian_32(&data[4*i..]); }

        static K256: [u32; 64] = [
            0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
            0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
            0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
            0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
            0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
            0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
            0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
            0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2,
        ];

        let mut ww = [0u32; 64];
        ww[..16].copy_from_slice(&w);
        for i in 16..64 {
            let s0 = ww[i-15].rotate_right(7) ^ ww[i-15].rotate_right(18) ^ (ww[i-15] >> 3);
            let s1 = ww[i-2].rotate_right(17) ^ ww[i-2].rotate_right(19) ^ (ww[i-2] >> 10);
            ww[i] = ww[i-16].wrapping_add(s0).wrapping_add(ww[i-7]).wrapping_add(s1);
        }

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h.wrapping_add(s1).wrapping_add(ch).wrapping_add(K256[i]).wrapping_add(ww[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            h = g; g = f; f = e; e = d.wrapping_add(temp1);
            d = c; c = b; b = a; a = temp1.wrapping_add(temp2);
        }

        a = a.wrapping_add(state[0]); b = b.wrapping_add(state[1]);
        c = c.wrapping_add(state[2]); d = d.wrapping_add(state[3]);
        e = e.wrapping_add(state[4]); f = f.wrapping_add(state[5]);
        g = g.wrapping_add(state[6]); h = h.wrapping_add(state[7]);
        state = [a, b, c, d, e, f, g, h];

        off += 64;
        inlen -= 64;
    }

    for i in 0..8 {
        store_bigendian_32(&mut statebytes[4*i..], state[i] as u64);
    }
    inlen
}

static IV_256: [u8; 32] = [
    0x6a,0x09,0xe6,0x67,0xbb,0x67,0xae,0x85,
    0x3c,0x6e,0xf3,0x72,0xa5,0x4f,0xf5,0x3a,
    0x51,0x0e,0x52,0x7f,0x9b,0x05,0x68,0x8c,
    0x1f,0x83,0xd9,0xab,0x5b,0xe0,0xcd,0x19,
];

fn sha256_inc_init(state: &mut [u8]) {
    state[..32].copy_from_slice(&IV_256);
    for i in 32..40 { state[i] = 0; }
}

fn sha256_inc_blocks(state: &mut [u8], inp: &[u8], inblocks: usize) {
    let mut bytes = load_bigendian_64(&state[32..]);
    crypto_hashblocks_sha256(state, inp, 64 * inblocks);
    bytes += (64 * inblocks) as u64;
    store_bigendian_64(&mut state[32..], bytes);
}

fn sha256_inc_finalize(out: &mut [u8], state: &mut [u8], inp: &[u8], inlen: usize) {
    let mut padded = [0u8; 128];
    let bytes = load_bigendian_64(&state[32..]) + inlen as u64;

    let remaining = crypto_hashblocks_sha256(state, inp, inlen);
    let start = inlen - remaining;
    let inp_tail = &inp[start..start + remaining];

    padded[..remaining].copy_from_slice(inp_tail);
    padded[remaining] = 0x80;

    if remaining < 56 {
        for i in remaining + 1..56 { padded[i] = 0; }
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
        for i in remaining + 1..120 { padded[i] = 0; }
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

fn sha256(out: &mut [u8], inp: &[u8], inlen: usize) {
    let mut state = [0u8; 40];
    sha256_inc_init(&mut state);
    sha256_inc_finalize(out, &mut state, inp, inlen);
}

fn mgf1_256(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut i = 0usize;
    let mut off = 0usize;
    while (i + 1) * SPX_SHA256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha256(&mut out[off..], &inbuf, inlen + 4);
        off += SPX_SHA256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_SHA256_OUTPUT_BYTES {
        let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha256(&mut outbuf, &inbuf, inlen + 4);
        let rem = outlen - i * SPX_SHA256_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

fn seed_state(ctx: &mut SpxCtx) {
    let mut block = [0u8; 128]; // SPX_SHA512_BLOCK_BYTES but we only use 64
    block[..SPX_N].copy_from_slice(&ctx.pub_seed);
    sha256_inc_init(&mut ctx.state_seeded);
    sha256_inc_blocks(&mut ctx.state_seeded, &block, 1);
}

// ============================================================
// utils (utils.c)
// ============================================================
fn ull_to_bytes(out: &mut [u8], outlen: usize, mut inp: u64) {
    for i in (0..outlen).rev() {
        out[i] = (inp & 0xff) as u8;
        inp >>= 8;
    }
}

fn u32_to_bytes(out: &mut [u8], inp: u32) {
    out[0] = (inp >> 24) as u8;
    out[1] = (inp >> 16) as u8;
    out[2] = (inp >> 8) as u8;
    out[3] = inp as u8;
}

fn bytes_to_ull(inp: &[u8], inlen: usize) -> u64 {
    let mut retval = 0u64;
    for i in 0..inlen {
        retval |= (inp[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

// ============================================================
// address (address.c)
// ============================================================
fn addr_bytes(addr: &[u32; 8]) -> [u8; 32] {
    let mut b = [0u8; 32];
    for i in 0..8 {
        let v = addr[i];
        b[4*i] = (v & 0xff) as u8;
        b[4*i+1] = ((v >> 8) & 0xff) as u8;
        b[4*i+2] = ((v >> 16) & 0xff) as u8;
        b[4*i+3] = ((v >> 24) & 0xff) as u8;
    }
    b
}

fn addr_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8; 32] {
    unsafe { &mut *(addr as *mut [u32; 8] as *mut [u8; 32]) }
}

fn addr_as_bytes(addr: &[u32; 8]) -> &[u8; 32] {
    unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) }
}

fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_LAYER] = layer as u8;
}

fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    let b = addr_bytes_mut(addr);
    ull_to_bytes(&mut b[SPX_OFFSET_TREE..], 8, tree);
}

fn set_type(addr: &mut [u32; 8], type_val: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_TYPE] = type_val as u8;
}

fn copy_subtree_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let src = addr_as_bytes(inp);
    let dst = addr_bytes_mut(out);
    dst[..SPX_OFFSET_TREE + 8].copy_from_slice(&src[..SPX_OFFSET_TREE + 8]);
}

fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    let b = addr_bytes_mut(addr);
    u32_to_bytes(&mut b[SPX_OFFSET_KP_ADDR..], keypair);
}

fn copy_keypair_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let src = addr_as_bytes(inp);
    let dst = addr_bytes_mut(out);
    dst[..SPX_OFFSET_TREE + 8].copy_from_slice(&src[..SPX_OFFSET_TREE + 8]);
    dst[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&src[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

fn set_chain_addr(addr: &mut [u32; 8], chain: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

fn set_hash_addr(addr: &mut [u32; 8], hash: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

fn set_tree_height(addr: &mut [u32; 8], tree_height: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

fn set_tree_index(addr: &mut [u32; 8], tree_index: u32) {
    let b = addr_bytes_mut(addr);
    u32_to_bytes(&mut b[SPX_OFFSET_TREE_INDEX..], tree_index);
}

// ============================================================
// hash_sha2 (hash_sha2.c) - initialize, prf_addr, gen_message_random, hash_message
// ============================================================
fn initialize_hash_function(ctx: &mut SpxCtx) {
    seed_state(ctx);
}

fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);
    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_as_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn gen_message_random(r_out: &mut [u8], sk_prf: &[u8], optrand: &[u8],
                      m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    let mut buf = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];

    // HMAC inner
    for i in 0..SPX_N { buf[i] = 0x36 ^ sk_prf[i]; }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES { buf[i] = 0x36; }

    sha256_inc_init(&mut state);
    sha256_inc_blocks(&mut state, &buf, 1);

    buf[..SPX_N].copy_from_slice(optrand);

    let mlen = mlen as usize;
    if SPX_N + mlen < SPX_SHAX_BLOCK_BYTES {
        buf[SPX_N..SPX_N + mlen].copy_from_slice(&m[..mlen]);
        let buf_copy = buf.clone();
        sha256_inc_finalize(&mut buf[SPX_SHAX_BLOCK_BYTES..], &mut state, &buf_copy, mlen + SPX_N);
    } else {
        buf[SPX_N..SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..SPX_SHAX_BLOCK_BYTES - SPX_N]);
        let buf_copy = buf.clone();
        sha256_inc_blocks(&mut state, &buf_copy, 1);
        let m_rest = &m[SPX_SHAX_BLOCK_BYTES - SPX_N..];
        let mlen_rest = mlen - (SPX_SHAX_BLOCK_BYTES - SPX_N);
        sha256_inc_finalize(&mut buf[SPX_SHAX_BLOCK_BYTES..], &mut state, m_rest, mlen_rest);
    }

    // HMAC outer
    for i in 0..SPX_N { buf[i] = 0x5c ^ sk_prf[i]; }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES { buf[i] = 0x5c; }

    let buf_copy = buf.clone();
    sha256(&mut buf, &buf_copy, SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES);
    r_out[..SPX_N].copy_from_slice(&buf[..SPX_N]);
}

fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
                r: &[u8], pk: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;
    const SPX_INBLOCKS: usize = ((SPX_N + SPX_PK_BYTES + SPX_SHAX_BLOCK_BYTES - 1)
        & (0usize.wrapping_sub(SPX_SHAX_BLOCK_BYTES))) / SPX_SHAX_BLOCK_BYTES;

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
        sha256_inc_finalize(&mut seed[2 * SPX_N..], &mut state, &inbuf, SPX_N + SPX_PK_BYTES + mlen);
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

// ============================================================
// thash_sha2_simple (thash_sha2_simple.c)
// ============================================================
fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);
    let buflen = SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buflen];
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_as_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..buflen].copy_from_slice(&inp[..inblocks * SPX_N]);
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, buflen);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// ============================================================
// RNG (rng.c) - AES-256-CTR-DRBG
// ============================================================
struct Aes256CtrDrbg {
    key: [u8; 32],
    v: [u8; 16],
    reseed_counter: i32,
}

static mut DRBG_CTX: Aes256CtrDrbg = Aes256CtrDrbg {
    key: [0u8; 32],
    v: [0u8; 16],
    reseed_counter: 0,
};

fn aes256_ecb(key: &[u8], ctr: &[u8], buffer: &mut [u8]) {
    let cipher = Cipher::aes_256_ecb();
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, None).unwrap();
    crypter.pad(false);
    let mut out = [0u8; 32]; // need space for block + potential padding
    let count = crypter.update(ctr, &mut out).unwrap();
    // ECB with pad(false) on exactly 16 bytes gives 16 bytes
    buffer[..16].copy_from_slice(&out[..16]);
    let _ = count;
}

fn aes256_ctr_drbg_update(provided_data: Option<&[u8]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
    let mut temp = [0u8; 48];
    for i in 0..3 {
        // increment V
        for j in (0..16).rev() {
            if v[j] == 0xff { v[j] = 0x00; } else { v[j] += 1; break; }
        }
        aes256_ecb(key, v, &mut temp[16 * i..]);
    }
    if let Some(pd) = provided_data {
        for i in 0..48 { temp[i] ^= pd[i]; }
    }
    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..48]);
}

fn randombytes_init(entropy_input: &[u8], personalization_string: Option<&[u8]>) {
    unsafe {
        let mut seed_material = [0u8; 48];
        seed_material.copy_from_slice(&entropy_input[..48]);
        if let Some(ps) = personalization_string {
            for i in 0..48 { seed_material[i] ^= ps[i]; }
        }
        DRBG_CTX.key = [0u8; 32];
        DRBG_CTX.v = [0u8; 16];
        aes256_ctr_drbg_update(Some(&seed_material), &mut DRBG_CTX.key, &mut DRBG_CTX.v);
        DRBG_CTX.reseed_counter = 1;
    }
}

fn randombytes(x: &mut [u8], mut xlen: usize) {
    unsafe {
        let mut block = [0u8; 16];
        let mut i = 0usize;
        while xlen > 0 {
            for j in (0..16).rev() {
                if DRBG_CTX.v[j] == 0xff { DRBG_CTX.v[j] = 0x00; } else { DRBG_CTX.v[j] += 1; break; }
            }
            aes256_ecb(&DRBG_CTX.key, &DRBG_CTX.v, &mut block);
            if xlen > 15 {
                x[i..i + 16].copy_from_slice(&block);
                i += 16;
                xlen -= 16;
            } else {
                x[i..i + xlen].copy_from_slice(&block[..xlen]);
                xlen = 0;
            }
        }
        aes256_ctr_drbg_update(None, &mut DRBG_CTX.key, &mut DRBG_CTX.v);
        DRBG_CTX.reseed_counter += 1;
    }
}

// ============================================================
// WOTS (wots.c)
// ============================================================
fn gen_chain(out: &mut [u8], inp: &[u8], start: u32, steps: u32,
             ctx: &SpxCtx, addr: &mut [u32; 8]) {
    out[..SPX_N].copy_from_slice(&inp[..SPX_N]);
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
        output[out_idx] = ((total >> bits) as u32) & (SPX_WOTS_W as u32 - 1);
        out_idx += 1;
    }
}

fn wots_checksum(csum_base_w: &mut [u32], msg_base_w: &[u32]) {
    let mut csum = 0u32;
    for i in 0..SPX_WOTS_LEN1 {
        csum += SPX_WOTS_W as u32 - 1 - msg_base_w[i];
    }
    csum <<= (8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8;
    let csum_bytes_len = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;
    let mut csum_bytes = [0u8; 4]; // max needed
    ull_to_bytes(&mut csum_bytes, csum_bytes_len, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}

fn chain_lengths(lengths: &mut [u32; SPX_WOTS_LEN], msg: &[u8]) {
    base_w(lengths, SPX_WOTS_LEN1, msg);
    let mut csum = [0u32; SPX_WOTS_LEN2];
    wots_checksum(&mut csum, lengths);
    lengths[SPX_WOTS_LEN1..SPX_WOTS_LEN].copy_from_slice(&csum);
}

fn wots_pk_from_sig(pk: &mut [u8], sig: &[u8], msg: &[u8],
                    ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut lengths = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut lengths, msg);
    for i in 0..SPX_WOTS_LEN {
        set_chain_addr(addr, i as u32);
        gen_chain(&mut pk[i * SPX_N..], &sig[i * SPX_N..],
                  lengths[i], SPX_WOTS_W as u32 - 1 - lengths[i], ctx, addr);
    }
}

// ============================================================
// wotsx1 (wotsx1.c)
// ============================================================
struct LeafInfoX1 {
    wots_sig: Vec<u8>,
    wots_sign_leaf: u32,
    wots_steps: [u32; SPX_WOTS_LEN],
    leaf_addr: [u32; 8],
    pk_addr: [u32; 8],
}

fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, info: &mut LeafInfoX1) {
    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf { 0 } else { !0u32 };

    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);

    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

    for i in 0..SPX_WOTS_LEN {
        let wots_k = info.wots_steps[i] | wots_k_mask;
        let buffer = &mut pk_buffer[i * SPX_N..(i + 1) * SPX_N];

        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        prf_addr(buffer, ctx, &info.leaf_addr);

        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0u32.. {
            if k == wots_k {
                info.wots_sig[i * SPX_N..(i + 1) * SPX_N].copy_from_slice(buffer);
            }
            if k == SPX_WOTS_W as u32 - 1 { break; }
            set_hash_addr(&mut info.leaf_addr, k);
            let mut tmp = [0u8; SPX_N];
            tmp.copy_from_slice(buffer);
            thash(buffer, &tmp, 1, ctx, &info.leaf_addr);
        }
    }

    thash(dest, &pk_buffer, SPX_WOTS_LEN, ctx, &info.pk_addr);
}

// ============================================================
// FORS (fors.c)
// ============================================================
fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, fors_leaf_addr: &[u32; 8]) {
    prf_addr(sk, ctx, fors_leaf_addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, fors_leaf_addr: &[u32; 8]) {
    thash(leaf, sk, 1, ctx, fors_leaf_addr);
}

struct ForsGenLeafInfo {
    leaf_addrx: [u32; 8],
}

fn fors_gen_leafx1(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, info: &mut ForsGenLeafInfo) {
    set_tree_index(&mut info.leaf_addrx, addr_idx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf, ctx, &info.leaf_addrx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSTREE);
    let tmp = leaf[..SPX_N].to_vec();
    fors_sk_to_leaf(leaf, &tmp, ctx, &info.leaf_addrx);
}

fn message_to_indices(indices: &mut [u32; SPX_FORS_TREES], m: &[u8]) {
    let mut offset = 0usize;
    for i in 0..SPX_FORS_TREES {
        indices[i] = 0;
        for j in 0..SPX_FORS_HEIGHT {
            indices[i] ^= (((m[offset >> 3] >> (offset & 0x7)) & 1) as u32) << j;
            offset += 1;
        }
    }
}

fn fors_sign(sig: &mut [u8], pk: &mut [u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32; 8]) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = [0u8; SPX_FORS_TREES * SPX_N];
    let mut fors_tree_addr = [0u32; 8];
    let mut fors_info = ForsGenLeafInfo { leaf_addrx: [0u32; 8] };
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

        fors_treehashx1(&mut roots[i * SPX_N..], &mut sig[sig_off..], ctx,
                         indices[i], idx_offset, SPX_FORS_HEIGHT as u32,
                         &mut fors_tree_addr, &mut fors_info);
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    thash(pk, &roots, SPX_FORS_TREES, ctx, &fors_pk_addr);
}

fn fors_pk_from_sig(pk: &mut [u8], sig: &[u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32; 8]) {
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

        fors_sk_to_leaf(&mut leaf, &sig[sig_off..], ctx, &fors_tree_addr);
        sig_off += SPX_N;

        compute_root(&mut roots[i * SPX_N..], &leaf, indices[i], idx_offset,
                     &sig[sig_off..], SPX_FORS_HEIGHT as u32, ctx, &mut fors_tree_addr);
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    thash(pk, &roots, SPX_FORS_TREES, ctx, &fors_pk_addr);
}

// ============================================================
// utils - compute_root, treehash (utils.c)
// ============================================================
fn compute_root(root: &mut [u8], leaf: &[u8], mut leaf_idx: u32, mut idx_offset: u32,
                auth_path: &[u8], tree_height: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut buffer = [0u8; 2 * SPX_N];
    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[..SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[..SPX_N]);
    }
    let mut auth_off = SPX_N;

    for i in 0..tree_height - 1 {
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
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[auth_off..auth_off + SPX_N]);
        }
        auth_off += SPX_N;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height(addr, tree_height);
    set_tree_index(addr, leaf_idx + idx_offset);
    thash(root, &buffer, 2, ctx, addr);
}

// ============================================================
// utilsx1 - wots_treehashx1, fors_treehashx1 (utilsx1.c)
// ============================================================
fn wots_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                   leaf_idx: u32, idx_offset: u32, tree_height: u32,
                   tree_addr: &mut [u32; 8], info: &mut LeafInfoX1) {
    let th = tree_height as usize;
    let mut stack = vec![0u8; th * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        for h in 0u32.. {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                let ho = h as usize;
                auth_path[ho * SPX_N..(ho + 1) * SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let ho = h as usize;
            current[..SPX_N].copy_from_slice(&stack[ho * SPX_N..(ho + 1) * SPX_N]);
            let tmp = current.clone();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        // Save left child
        let h_val = {
            let mut internal_idx2 = idx;
            let mut hh = 0u32;
            // find h: count trailing ones + check break condition
            // Actually we need to know what h was when we broke out of the inner loop
            // Recompute: the inner loop breaks when (internal_idx & 1) == 0 && idx < max_idx
            // h increments each iteration, internal_idx >>= 1 each iteration
            let mut ii = idx;
            let mut il = leaf_idx;
            let mut hc = 0u32;
            loop {
                if hc == tree_height { break; }
                if (ii & 1) == 0 && idx < max_idx { break; }
                ii >>= 1;
                il >>= 1;
                hc += 1;
            }
            hc
        };
        let ho = h_val as usize;
        stack[ho * SPX_N..(ho + 1) * SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

fn fors_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                   leaf_idx: u32, idx_offset: u32, tree_height: u32,
                   tree_addr: &mut [u32; 8], fors_info: &mut ForsGenLeafInfo) {
    let th = tree_height as usize;
    let mut stack = vec![0u8; th * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        fors_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, fors_info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        for h in 0u32.. {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                let ho = h as usize;
                auth_path[ho * SPX_N..(ho + 1) * SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let ho = h as usize;
            current[..SPX_N].copy_from_slice(&stack[ho * SPX_N..(ho + 1) * SPX_N]);
            let tmp = current.clone();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let h_val = {
            let mut ii = idx;
            let mut hc = 0u32;
            loop {
                if hc == tree_height { break; }
                if (ii & 1) == 0 && idx < max_idx { break; }
                ii >>= 1;
                hc += 1;
            }
            hc
        };
        let ho = h_val as usize;
        stack[ho * SPX_N..(ho + 1) * SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

// ============================================================
// merkle (merkle.c)
// ============================================================
fn merkle_sign(sig: &mut [u8], root: &mut [u8], ctx: &SpxCtx,
               wots_addr: &mut [u32; 8], tree_addr: &mut [u32; 8], idx_leaf: u32) {
    let auth_path_off = SPX_WOTS_BYTES;
    let mut info = LeafInfoX1 {
        wots_sig: vec![0u8; SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N], // sig buffer
        wots_sign_leaf: idx_leaf,
        wots_steps: [0u32; SPX_WOTS_LEN],
        leaf_addr: [0u32; 8],
        pk_addr: [0u32; 8],
    };

    // The sig pointer in C is used as info.wots_sig
    // We'll write into info.wots_sig then copy back
    chain_lengths(&mut info.wots_steps, root);

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr(&mut info.pk_addr, wots_addr);

    // wots_treehashx1 writes wots_sig into info.wots_sig and auth_path
    let mut auth_buf = vec![0u8; SPX_TREE_HEIGHT * SPX_N];
    wots_treehashx1(root, &mut auth_buf, ctx, idx_leaf, 0, SPX_TREE_HEIGHT as u32, tree_addr, &mut info);

    // Copy wots signature and auth path to sig
    sig[..SPX_WOTS_BYTES].copy_from_slice(&info.wots_sig[..SPX_WOTS_BYTES]);
    sig[auth_path_off..auth_path_off + SPX_TREE_HEIGHT * SPX_N].copy_from_slice(&auth_buf);
}

fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];
    set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr(&mut wots_addr, (SPX_D - 1) as u32);
    merkle_sign(&mut auth_path, root, ctx, &mut wots_addr, &mut top_tree_addr, !0u32);
}

// ============================================================
// sign (sign.c)
// ============================================================
fn crypto_sign_seed_keypair(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) {
    let mut ctx = SpxCtx { pub_seed: [0; SPX_N], sk_seed: [0; SPX_N], state_seeded: [0; 40] };
    sk[..CRYPTO_SEEDBYTES].copy_from_slice(&seed[..CRYPTO_SEEDBYTES]);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    initialize_hash_function(&mut ctx);
    let mut root = [0u8; SPX_N];
    merkle_gen_root(&mut root, &ctx);
    sk[3 * SPX_N..4 * SPX_N].copy_from_slice(&root);
    pk[SPX_N..2 * SPX_N].copy_from_slice(&root);
}

fn crypto_sign_keypair(pk: &mut [u8], sk: &mut [u8]) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    randombytes(&mut seed, CRYPTO_SEEDBYTES);
    crypto_sign_seed_keypair(pk, sk, &seed);
    0
}

fn crypto_sign_signature(sig: &mut [u8], siglen: &mut usize,
                         m: &[u8], mlen: usize, sk: &[u8]) -> i32 {
    let mut ctx = SpxCtx { pub_seed: [0; SPX_N], sk_seed: [0; SPX_N], state_seeded: [0; 40] };
    let sk_prf = &sk[SPX_N..2 * SPX_N];
    let pk = &sk[2 * SPX_N..];

    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    initialize_hash_function(&mut ctx);

    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

    let mut optrand = [0u8; SPX_N];
    randombytes(&mut optrand, SPX_N);

    gen_message_random(&mut sig[..SPX_N], sk_prf, &optrand, m, mlen as u64, &ctx);

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut tree = 0u64;
    let mut idx_leaf = 0u32;
    hash_message(&mut mhash, &mut tree, &mut idx_leaf, &sig[..SPX_N], pk, m, mlen as u64, &ctx);

    let mut sig_off = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    let mut root = [0u8; SPX_N];
    fors_sign(&mut sig[sig_off..], &mut root, &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);
        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        merkle_sign(&mut sig[sig_off..], &mut root, &ctx, &mut wots_addr, &mut tree_addr, idx_leaf);
        sig_off += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    *siglen = SPX_BYTES;
    0
}

fn crypto_sign_verify(sig: &[u8], siglen: usize, m: &[u8], mlen: usize, pk: &[u8]) -> i32 {
    let mut ctx = SpxCtx { pub_seed: [0; SPX_N], sk_seed: [0; SPX_N], state_seeded: [0; 40] };
    let pub_root = &pk[SPX_N..];

    if siglen != SPX_BYTES { return -1; }

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    initialize_hash_function(&mut ctx);

    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];
    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut tree = 0u64;
    let mut idx_leaf = 0u32;
    hash_message(&mut mhash, &mut tree, &mut idx_leaf, &sig[..SPX_N], pk, m, mlen as u64, &ctx);

    let mut sig_off = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    let mut root = [0u8; SPX_N];
    fors_pk_from_sig(&mut root, &sig[sig_off..], &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    let mut wots_pk = [0u8; SPX_WOTS_BYTES];
    let mut leaf = [0u8; SPX_N];

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);
        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);
        copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        wots_pk_from_sig(&mut wots_pk, &sig[sig_off..], &root, &ctx, &mut wots_addr);
        sig_off += SPX_WOTS_BYTES;

        thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &wots_pk_addr);

        compute_root(&mut root, &leaf, idx_leaf, 0, &sig[sig_off..], SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr);
        sig_off += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1u64 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    if root[..SPX_N] != pub_root[..SPX_N] { return -1; }
    0
}

fn crypto_sign(sm: &mut [u8], smlen: &mut u64, m: &[u8], mlen: u64, sk: &[u8]) -> i32 {
    let mut siglen = 0usize;
    crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);
    // memmove sm + SPX_BYTES, m, mlen
    let mlen = mlen as usize;
    sm.copy_within(0..0, 0); // no-op, just for clarity
    // We need to copy m after the signature
    for i in (0..mlen).rev() {
        sm[SPX_BYTES + i] = m[i];
    }
    *smlen = (siglen + mlen) as u64;
    0
}

fn crypto_sign_open(m_out: &mut [u8], mlen: &mut u64, sm: &[u8], smlen: u64, pk: &[u8]) -> i32 {
    let smlen_usize = smlen as usize;
    if smlen_usize < SPX_BYTES {
        for i in 0..smlen_usize { m_out[i] = 0; }
        *mlen = 0;
        return -1;
    }
    *mlen = (smlen_usize - SPX_BYTES) as u64;
    let mlen_usize = *mlen as usize;

    if crypto_sign_verify(sm, SPX_BYTES, &sm[SPX_BYTES..], mlen_usize, pk) != 0 {
        for i in 0..smlen_usize { m_out[i] = 0; }
        *mlen = 0;
        return -1;
    }

    m_out[..mlen_usize].copy_from_slice(&sm[SPX_BYTES..SPX_BYTES + mlen_usize]);
    0
}

// ============================================================
// KAT Transcript (SHA2 variant from PQCgenKAT_sign.c)
// ============================================================
// For SHA2 with SPX_N < 24: uses sha256
const SHAX_STATE_LEN: usize = 40;
const SHAX_BLOCK_BYTES: usize = 64;
const SHAX_OUTPUT_BYTES: usize = 32;

struct KatTrCtx {
    s: [u8; SHAX_STATE_LEN],
}

fn kat_tr_init(ctx: &mut KatTrCtx) {
    let tag = b"KAT-TRANSCRIPT-v1-SHA2";
    let mut block = [0u8; SHAX_BLOCK_BYTES];
    for i in 0..tag.len() { block[i] = tag[i]; }
    // rest already zero

    sha256_inc_init(&mut ctx.s);
    sha256_inc_blocks(&mut ctx.s, &block, 1);
}

fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
    let n = label.len();
    let block_count = (n + 1 + (SHAX_BLOCK_BYTES - 1)) / SHAX_BLOCK_BYTES;

    for i in 0..block_count {
        let mut block = [0u8; SHAX_BLOCK_BYTES];
        let mut j = 0usize;
        while i * SHAX_BLOCK_BYTES + j < n && j < SHAX_BLOCK_BYTES {
            block[j] = label[i * SHAX_BLOCK_BYTES + j];
            j += 1;
        }
        if i * SHAX_BLOCK_BYTES + j == n && j < SHAX_BLOCK_BYTES {
            block[j] = 0x00;
            j += 1;
        }
        while j < SHAX_BLOCK_BYTES { block[j] = 0; j += 1; }
        sha256_inc_blocks(&mut ctx.s, &block, 1);
    }
}

fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
    let mut le = [0u8; 8];
    for i in 0..8 { le[i] = ((x >> (8 * i)) & 0xFF) as u8; }
    let mut lenle = [0u8; 8];
    let l: u64 = 8;
    for i in 0..8 { lenle[i] = ((l >> (8 * i)) & 0xFF) as u8; }

    let mut block = [0u8; SHAX_BLOCK_BYTES];
    block[..8].copy_from_slice(&lenle);
    block[8..16].copy_from_slice(&le);
    // rest zero
    sha256_inc_blocks(&mut ctx.s, &block, 1);
}

fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
    let mut lenle_block = [0u8; SHAX_BLOCK_BYTES];
    let l = len as u64;
    for i in 0..8 { lenle_block[i] = ((l >> (8 * i)) & 0xFF) as u8; }
    let block_count = (len + SHAX_BLOCK_BYTES - 1) / SHAX_BLOCK_BYTES;
    sha256_inc_blocks(&mut ctx.s, &lenle_block, 1);

    if len != 0 {
        for i in 0..block_count {
            let mut block = [0u8; SHAX_BLOCK_BYTES];
            let mut j = 0usize;
            while i * SHAX_BLOCK_BYTES + j < len && j < SHAX_BLOCK_BYTES {
                block[j] = buf[i * SHAX_BLOCK_BYTES + j];
                j += 1;
            }
            while j < SHAX_BLOCK_BYTES { block[j] = 0; j += 1; }
            sha256_inc_blocks(&mut ctx.s, &block, 1);
        }
    }
}

fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
    let mut outbuf = [0u8; SHAX_OUTPUT_BYTES];
    let final_block = [0u8; SHAX_BLOCK_BYTES];
    // C: shaX_inc_finalize(outbuf, ctx->s, final_block, 1) — 1 byte of zero input
    sha256_inc_finalize(&mut outbuf, &mut ctx.s, &final_block, 1);
    out32.copy_from_slice(&outbuf[..32]);
}

// ============================================================
// main
// ============================================================
const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

fn main() {
    let mut m = vec![0u8; BASE_MLEN * LOOP_COUNT];
    let mut sm = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut m1 = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut pk = [0u8; CRYPTO_PUBLICKEYBYTES];
    let mut sk = [0u8; CRYPTO_SECRETKEYBYTES];
    let mut seed = [0u8; 48];
    let mut entropy_input = [0u8; 48];
    let mut msg = vec![0u8; BASE_MLEN * LOOP_COUNT];

    for i in 0..48 { entropy_input[i] = i as u8; }
    randombytes_init(&entropy_input, None);

    let mut tctx = KatTrCtx { s: [0u8; SHAX_STATE_LEN] };
    kat_tr_init(&mut tctx);
    kat_tr_absorb_label(&mut tctx, b"CRYPTO_ALGNAME");
    kat_tr_absorb_bytes(&mut tctx, CRYPTO_ALGNAME, CRYPTO_ALGNAME.len());
    kat_tr_absorb_label(&mut tctx, b"SKBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_SECRETKEYBYTES as u64);
    kat_tr_absorb_label(&mut tctx, b"PKBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_PUBLICKEYBYTES as u64);
    kat_tr_absorb_label(&mut tctx, b"SIGBYTES");
    kat_tr_absorb_u64(&mut tctx, CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT {
        randombytes(&mut seed, 48);

        kat_tr_absorb_label(&mut tctx, b"count");
        kat_tr_absorb_u64(&mut tctx, i as u64);
        kat_tr_absorb_label(&mut tctx, b"seed");
        kat_tr_absorb_bytes(&mut tctx, &seed, 48);

        let mlen = BASE_MLEN * (i + 1);
        if mlen > BASE_MLEN * LOOP_COUNT {
            eprintln!("mlen overflow");
            std::process::exit(-1);
        }

        kat_tr_absorb_label(&mut tctx, b"mlen");
        kat_tr_absorb_u64(&mut tctx, mlen as u64);

        randombytes(&mut msg, mlen);
        kat_tr_absorb_label(&mut tctx, b"msg");
        kat_tr_absorb_bytes(&mut tctx, &msg, mlen);

        for j in 0..mlen { m[j] = 0; }
        for j in 0..mlen + CRYPTO_BYTES { m1[j] = 0; }
        for j in 0..mlen + CRYPTO_BYTES { sm[j] = 0; }
        m[..mlen].copy_from_slice(&msg[..mlen]);

        let ret = crypto_sign_keypair(&mut pk, &mut sk);
        if ret != 0 {
            eprintln!("crypto_sign_keypair={}", ret);
            std::process::exit(-2);
        }
        kat_tr_absorb_label(&mut tctx, b"pk");
        kat_tr_absorb_bytes(&mut tctx, &pk, CRYPTO_PUBLICKEYBYTES);
        kat_tr_absorb_label(&mut tctx, b"sk");
        kat_tr_absorb_bytes(&mut tctx, &sk, CRYPTO_SECRETKEYBYTES);

        let mut smlen = 0u64;
        let ret = crypto_sign(&mut sm, &mut smlen, &m[..mlen], mlen as u64, &sk);
        if ret != 0 {
            eprintln!("crypto_sign={}", ret);
            std::process::exit(-2);
        }
        kat_tr_absorb_label(&mut tctx, b"smlen");
        kat_tr_absorb_u64(&mut tctx, smlen);
        kat_tr_absorb_label(&mut tctx, b"sm");
        kat_tr_absorb_bytes(&mut tctx, &sm, smlen as usize);

        let mut mlen1 = 0u64;
        let ret = crypto_sign_open(&mut m1, &mut mlen1, &sm, smlen, &pk);
        if ret != 0 {
            eprintln!("crypto_sign_open={}", ret);
            std::process::exit(-2);
        }
        if mlen1 as usize != mlen {
            eprintln!("mlen mismatch");
            std::process::exit(-2);
        }
        if m[..mlen] != m1[..mlen] {
            eprintln!("m mismatch");
            std::process::exit(-2);
        }
    }

    let mut digest = [0u8; 32];
    kat_tr_final(&mut tctx, &mut digest);

    print!("KAT transcript digest = ");
    for i in 0..32 { print!("{:02X}", digest[i]); }
    println!();
}
