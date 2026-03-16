#![allow(
    non_snake_case,
    non_upper_case_globals,
    clippy::identity_op,
    clippy::too_many_arguments,
    clippy::needless_range_loop
)]

use std::ptr;

// ============================================================================
// Parameters (SHA2-128s-simple)
// ============================================================================
const SPX_N: usize = 16;
const SPX_FULL_HEIGHT: usize = 63;
const SPX_D: usize = 7;
const SPX_FORS_HEIGHT: usize = 12;
const SPX_FORS_TREES: usize = 14;
const SPX_WOTS_W: usize = 16;
const SPX_WOTS_LOGW: usize = 4;
const SPX_ADDR_BYTES: usize = 32;
const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW; // 32
const SPX_WOTS_LEN2: usize = 3;
const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2; // 35
const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;
const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D; // 9
const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8; // 21
const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N; // 2912
const SPX_BYTES: usize = SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
const SPX_PK_BYTES: usize = 2 * SPX_N;
const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;
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

const SPX_SHA256_BLOCK_BYTES: usize = 64;
const SPX_SHA256_OUTPUT_BYTES: usize = 32;
const SPX_SHA256_ADDR_BYTES: usize = 22;
const SPX_SHAX_OUTPUT_BYTES: usize = SPX_SHA256_OUTPUT_BYTES;
const SPX_SHAX_BLOCK_BYTES: usize = SPX_SHA256_BLOCK_BYTES;

const SPX_ADDR_TYPE_WOTS: u32 = 0;
const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
const SPX_ADDR_TYPE_FORSPK: u32 = 4;
const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;
const SPX_INBLOCKS: usize =
    ((SPX_N + SPX_PK_BYTES + SPX_SHAX_BLOCK_BYTES - 1) & !(SPX_SHAX_BLOCK_BYTES - 1)) / SPX_SHAX_BLOCK_BYTES;

// ============================================================================
// Context
// ============================================================================
#[repr(C)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
    pub state_seeded: [u8; 40],
}

// ============================================================================
// SHA-256
// ============================================================================
fn load_bigendian_32(x: &[u8]) -> u32 {
    (x[3] as u32) | ((x[2] as u32) << 8) | ((x[1] as u32) << 16) | ((x[0] as u32) << 24)
}
fn load_bigendian_64(x: &[u8]) -> u64 {
    (x[7] as u64) | ((x[6] as u64) << 8) | ((x[5] as u64) << 16) | ((x[4] as u64) << 24)
        | ((x[3] as u64) << 32) | ((x[2] as u64) << 40) | ((x[1] as u64) << 48) | ((x[0] as u64) << 56)
}
fn store_bigendian_32(x: &mut [u8], mut u: u64) {
    x[3] = u as u8; u >>= 8; x[2] = u as u8; u >>= 8; x[1] = u as u8; u >>= 8; x[0] = u as u8;
}
fn store_bigendian_64(x: &mut [u8], mut u: u64) {
    x[7] = u as u8; u >>= 8; x[6] = u as u8; u >>= 8; x[5] = u as u8; u >>= 8; x[4] = u as u8;
    u >>= 8; x[3] = u as u8; u >>= 8; x[2] = u as u8; u >>= 8; x[1] = u as u8; u >>= 8; x[0] = u as u8;
}

static SHA256_K: [u32; 64] = [
    0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
    0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
    0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
    0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
    0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
    0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
    0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
    0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2,
];
static IV_256: [u8; 32] = [
    0x6a,0x09,0xe6,0x67,0xbb,0x67,0xae,0x85,0x3c,0x6e,0xf3,0x72,0xa5,0x4f,0xf5,0x3a,
    0x51,0x0e,0x52,0x7f,0x9b,0x05,0x68,0x8c,0x1f,0x83,0xd9,0xab,0x5b,0xe0,0xcd,0x19,
];

fn crypto_hashblocks_sha256(statebytes: &mut [u8], inp: &[u8], mut inlen: usize) -> usize {
    let mut state = [0u32; 8];
    for i in 0..8 { state[i] = load_bigendian_32(&statebytes[4*i..]); }
    let (mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut h) =
        (state[0],state[1],state[2],state[3],state[4],state[5],state[6],state[7]);
    let mut pos = 0usize;
    while inlen >= 64 {
        let mut w = [0u32; 16];
        for i in 0..16 { w[i] = load_bigendian_32(&inp[pos+4*i..]); }
        for round in 0..64 {
            if round >= 16 {
                let i = round & 15;
                w[i] = (w[(i+14)&15].rotate_right(17) ^ w[(i+14)&15].rotate_right(19) ^ (w[(i+14)&15] >> 10))
                    .wrapping_add(w[(i+9)&15])
                    .wrapping_add(w[(i+1)&15].rotate_right(7) ^ w[(i+1)&15].rotate_right(18) ^ (w[(i+1)&15] >> 3))
                    .wrapping_add(w[i]);
            }
            let t1 = h.wrapping_add(e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25))
                .wrapping_add((e & f) ^ (!e & g)).wrapping_add(SHA256_K[round]).wrapping_add(w[round & 15]);
            let t2 = (a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22))
                .wrapping_add((a & b) ^ (a & c) ^ (b & c));
            h = g; g = f; f = e; e = d.wrapping_add(t1); d = c; c = b; b = a; a = t1.wrapping_add(t2);
        }
        a = a.wrapping_add(state[0]); b = b.wrapping_add(state[1]);
        c = c.wrapping_add(state[2]); d = d.wrapping_add(state[3]);
        e = e.wrapping_add(state[4]); f = f.wrapping_add(state[5]);
        g = g.wrapping_add(state[6]); h = h.wrapping_add(state[7]);
        state = [a,b,c,d,e,f,g,h];
        pos += 64; inlen -= 64;
    }
    for i in 0..8 { store_bigendian_32(&mut statebytes[4*i..], state[i] as u64); }
    inlen
}

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
    let bytes = load_bigendian_64(&state[32..]).wrapping_add(inlen as u64);
    crypto_hashblocks_sha256(state, inp, inlen);
    let remaining = inlen & 63;
    let start = inlen - remaining;
    padded[..remaining].copy_from_slice(&inp[start..start + remaining]);
    padded[remaining] = 0x80;
    if remaining < 56 {
        for i in (remaining+1)..56 { padded[i] = 0; }
        padded[56]=(bytes>>53)as u8; padded[57]=(bytes>>45)as u8;
        padded[58]=(bytes>>37)as u8; padded[59]=(bytes>>29)as u8;
        padded[60]=(bytes>>21)as u8; padded[61]=(bytes>>13)as u8;
        padded[62]=(bytes>>5)as u8;  padded[63]=(bytes<<3)as u8;
        crypto_hashblocks_sha256(state, &padded, 64);
    } else {
        for i in (remaining+1)..120 { padded[i] = 0; }
        padded[120]=(bytes>>53)as u8; padded[121]=(bytes>>45)as u8;
        padded[122]=(bytes>>37)as u8; padded[123]=(bytes>>29)as u8;
        padded[124]=(bytes>>21)as u8; padded[125]=(bytes>>13)as u8;
        padded[126]=(bytes>>5)as u8;  padded[127]=(bytes<<3)as u8;
        crypto_hashblocks_sha256(state, &padded, 128);
    }
    out[..32].copy_from_slice(&state[..32]);
}
fn sha256(out: &mut [u8], inp: &[u8], inlen: usize) {
    let mut state = [0u8; 40];
    sha256_inc_init(&mut state);
    sha256_inc_finalize(out, &mut state, inp, inlen);
}

// ============================================================================
// Utils
// ============================================================================
fn ull_to_bytes(out: &mut [u8], outlen: usize, mut v: u64) {
    for i in (0..outlen).rev() { out[i] = (v & 0xff) as u8; v >>= 8; }
}
fn u32_to_bytes(out: &mut [u8], v: u32) {
    out[0]=(v>>24)as u8; out[1]=(v>>16)as u8; out[2]=(v>>8)as u8; out[3]=v as u8;
}
fn bytes_to_ull(inp: &[u8], inlen: usize) -> u64 {
    let mut r: u64 = 0;
    for i in 0..inlen { r |= (inp[i] as u64) << (8*(inlen-1-i)); }
    r
}
fn mgf1_256(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut i: usize = 0;
    while (i+1)*SPX_SHA256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha256(&mut out[i*SPX_SHA256_OUTPUT_BYTES..], &inbuf, inlen+4);
        i += 1;
    }
    if outlen > i*SPX_SHA256_OUTPUT_BYTES {
        let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha256(&mut outbuf, &inbuf, inlen+4);
        let rem = outlen - i*SPX_SHA256_OUTPUT_BYTES;
        out[i*SPX_SHA256_OUTPUT_BYTES..i*SPX_SHA256_OUTPUT_BYTES+rem].copy_from_slice(&outbuf[..rem]);
    }
}

// ============================================================================
// Address helpers
// ============================================================================
fn addr_bytes(addr: &[u32; 8]) -> &[u8; 32] {
    unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) }
}
fn addr_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8; 32] {
    unsafe { &mut *(addr as *mut [u32; 8] as *mut [u8; 32]) }
}
fn set_layer_addr(a: &mut [u32;8], v: u32) { addr_bytes_mut(a)[SPX_OFFSET_LAYER]=v as u8; }
fn set_tree_addr(a: &mut [u32;8], v: u64) { ull_to_bytes(&mut addr_bytes_mut(a)[SPX_OFFSET_TREE..],8,v); }
fn set_type(a: &mut [u32;8], v: u32) { addr_bytes_mut(a)[SPX_OFFSET_TYPE]=v as u8; }
fn copy_subtree_addr(o: &mut [u32;8], i: &[u32;8]) {
    addr_bytes_mut(o)[..SPX_OFFSET_TREE+8].copy_from_slice(&addr_bytes(i)[..SPX_OFFSET_TREE+8]);
}
fn set_keypair_addr(a: &mut [u32;8], v: u32) { u32_to_bytes(&mut addr_bytes_mut(a)[SPX_OFFSET_KP_ADDR..],v); }
fn copy_keypair_addr(o: &mut [u32;8], i: &[u32;8]) {
    addr_bytes_mut(o)[..SPX_OFFSET_TREE+8].copy_from_slice(&addr_bytes(i)[..SPX_OFFSET_TREE+8]);
    addr_bytes_mut(o)[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR+4].copy_from_slice(&addr_bytes(i)[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR+4]);
}
fn set_chain_addr(a: &mut [u32;8], v: u32) { addr_bytes_mut(a)[SPX_OFFSET_CHAIN_ADDR]=v as u8; }
fn set_hash_addr(a: &mut [u32;8], v: u32) { addr_bytes_mut(a)[SPX_OFFSET_HASH_ADDR]=v as u8; }
fn set_tree_height(a: &mut [u32;8], v: u32) { addr_bytes_mut(a)[SPX_OFFSET_TREE_HGT]=v as u8; }
fn set_tree_index(a: &mut [u32;8], v: u32) { u32_to_bytes(&mut addr_bytes_mut(a)[SPX_OFFSET_TREE_INDEX..],v); }

// ============================================================================
// seed_state / initialize_hash_function
// ============================================================================
fn seed_state(ctx: &mut SpxCtx) {
    let mut block = [0u8; SPX_SHA256_BLOCK_BYTES];
    block[..SPX_N].copy_from_slice(&ctx.pub_seed);
    sha256_inc_init(&mut ctx.state_seeded);
    sha256_inc_blocks(&mut ctx.state_seeded, &block, 1);
}
fn initialize_hash_function_internal(ctx: &mut SpxCtx) { seed_state(ctx); }

// ============================================================================
// Hash functions (prf_addr, gen_message_random, hash_message)
// ============================================================================
fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut sha2_state = [0u8; 40];
    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + SPX_N];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    sha2_state.copy_from_slice(&ctx.state_seeded);
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, SPX_SHA256_ADDR_BYTES + SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn gen_message_random(r_out: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    let mlen = mlen as usize;
    let mut buf = [0u8; SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];
    for i in 0..SPX_N { buf[i] = 0x36 ^ sk_prf[i]; }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES { buf[i] = 0x36; }
    sha256_inc_init(&mut state);
    sha256_inc_blocks(&mut state, &buf, 1);
    buf[..SPX_N].copy_from_slice(&optrand[..SPX_N]);
    if SPX_N + mlen < SPX_SHAX_BLOCK_BYTES {
        buf[SPX_N..SPX_N+mlen].copy_from_slice(&m[..mlen]);
        let tmp = buf[..SPX_N+mlen].to_vec();
        sha256_inc_finalize(&mut buf[SPX_SHAX_BLOCK_BYTES..], &mut state, &tmp, mlen+SPX_N);
    } else {
        buf[SPX_N..SPX_SHAX_BLOCK_BYTES].copy_from_slice(&m[..SPX_SHAX_BLOCK_BYTES-SPX_N]);
        let block_copy = buf[..SPX_SHAX_BLOCK_BYTES].to_vec();
        sha256_inc_blocks(&mut state, &block_copy, 1);
        let off = SPX_SHAX_BLOCK_BYTES - SPX_N;
        sha256_inc_finalize(&mut buf[SPX_SHAX_BLOCK_BYTES..], &mut state, &m[off..], mlen - off);
    }
    for i in 0..SPX_N { buf[i] = 0x5c ^ sk_prf[i]; }
    for i in SPX_N..SPX_SHAX_BLOCK_BYTES { buf[i] = 0x5c; }
    let total = SPX_SHAX_BLOCK_BYTES + SPX_SHAX_OUTPUT_BYTES;
    let buf_copy = buf[..total].to_vec();
    sha256(&mut buf, &buf_copy, total);
    r_out[..SPX_N].copy_from_slice(&buf[..SPX_N]);
}

fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
                r: &[u8], pk: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    let mlen = mlen as usize;
    let mut seed = [0u8; 2*SPX_N + SPX_SHAX_OUTPUT_BYTES];
    let mut inbuf = [0u8; SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES];
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut state = [0u8; 8 + SPX_SHAX_OUTPUT_BYTES];
    sha256_inc_init(&mut state);
    inbuf[..SPX_N].copy_from_slice(&r[..SPX_N]);
    inbuf[SPX_N..SPX_N+SPX_PK_BYTES].copy_from_slice(&pk[..SPX_PK_BYTES]);
    if SPX_N + SPX_PK_BYTES + mlen < SPX_INBLOCKS * SPX_SHAX_BLOCK_BYTES {
        inbuf[SPX_N+SPX_PK_BYTES..SPX_N+SPX_PK_BYTES+mlen].copy_from_slice(&m[..mlen]);
        sha256_inc_finalize(&mut seed[2*SPX_N..], &mut state, &inbuf, SPX_N+SPX_PK_BYTES+mlen);
    } else {
        let fill = SPX_INBLOCKS*SPX_SHAX_BLOCK_BYTES - SPX_N - SPX_PK_BYTES;
        inbuf[SPX_N+SPX_PK_BYTES..].copy_from_slice(&m[..fill]);
        sha256_inc_blocks(&mut state, &inbuf, SPX_INBLOCKS);
        sha256_inc_finalize(&mut seed[2*SPX_N..], &mut state, &m[fill..], mlen-fill);
    }
    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2*SPX_N].copy_from_slice(&pk[..SPX_N]);
    mgf1_256(&mut buf, SPX_DGST_BYTES, &seed, 2*SPX_N+SPX_SHAX_OUTPUT_BYTES);
    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bp = SPX_FORS_MSG_BYTES;
    if SPX_D == 1 { *tree = 0; } else {
        *tree = bytes_to_ull(&buf[bp..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bp += SPX_TREE_BYTES;
    *leaf_idx = bytes_to_ull(&buf[bp..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}

// ============================================================================
// thash (simple, SHA-256 only)
// ============================================================================
fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 40];
    let blen = SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; blen];
    sha2_state.copy_from_slice(&ctx.state_seeded);
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..blen].copy_from_slice(&inp[..inblocks*SPX_N]);
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, blen);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// ============================================================================
// WOTS
// ============================================================================
fn gen_chain(out: &mut [u8], inp: &[u8], start: u32, steps: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    out[..SPX_N].copy_from_slice(&inp[..SPX_N]);
    for i in start..std::cmp::min(start+steps, SPX_WOTS_W as u32) {
        set_hash_addr(addr, i);
        let tmp: Vec<u8> = out[..SPX_N].to_vec();
        thash(out, &tmp, 1, ctx, addr);
    }
}
fn base_w(output: &mut [u32], out_len: usize, input: &[u8]) {
    let (mut in_i, mut out_i, mut bits) = (0usize, 0usize, 0i32);
    let mut total: u8 = 0;
    for _ in 0..out_len {
        if bits == 0 { total = input[in_i]; in_i += 1; bits += 8; }
        bits -= SPX_WOTS_LOGW as i32;
        output[out_i] = ((total >> bits) & (SPX_WOTS_W as u8 - 1)) as u32;
        out_i += 1;
    }
}
fn wots_checksum(csum_base_w: &mut [u32], msg_base_w: &[u32]) {
    let mut csum: u32 = 0;
    for i in 0..SPX_WOTS_LEN1 { csum += SPX_WOTS_W as u32 - 1 - msg_base_w[i]; }
    csum <<= (8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8;
    let csum_bytes_len = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;
    let mut csum_bytes = [0u8; 4];
    ull_to_bytes(&mut csum_bytes, csum_bytes_len, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}
fn chain_lengths(lengths: &mut [u32; SPX_WOTS_LEN], msg: &[u8]) {
    base_w(lengths, SPX_WOTS_LEN1, msg);
    let mut csum = [0u32; SPX_WOTS_LEN2];
    wots_checksum(&mut csum, lengths);
    lengths[SPX_WOTS_LEN1..].copy_from_slice(&csum);
}
fn wots_pk_from_sig(pk: &mut [u8], sig: &[u8], msg: &[u8], ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut lengths = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut lengths, msg);
    for i in 0..SPX_WOTS_LEN {
        set_chain_addr(addr, i as u32);
        gen_chain(&mut pk[i*SPX_N..], &sig[i*SPX_N..], lengths[i], SPX_WOTS_W as u32 -1-lengths[i], ctx, addr);
    }
}

// ============================================================================
// WOTS x1
// ============================================================================
struct LeafInfoX1 {
    wots_sig: *mut u8,
    wots_sign_leaf: u32,
    wots_steps: *const u32,
    leaf_addr: [u32; 8],
    pk_addr: [u32; 8],
}

fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, info: &mut LeafInfoX1) {
    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf { 0 } else { !0u32 };
    set_keypair_addr(&mut info.leaf_addr, leaf_idx);
    set_keypair_addr(&mut info.pk_addr, leaf_idx);
    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];
    for i in 0..SPX_WOTS_LEN {
        let wots_k = unsafe { *info.wots_steps.add(i) } | wots_k_mask;
        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);
        prf_addr(&mut pk_buffer[i*SPX_N..(i+1)*SPX_N], ctx, &info.leaf_addr);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);
        for k in 0u32.. {
            if k == wots_k {
                unsafe { ptr::copy_nonoverlapping(pk_buffer[i*SPX_N..].as_ptr(), info.wots_sig.add(i*SPX_N), SPX_N); }
            }
            if k == SPX_WOTS_W as u32 - 1 { break; }
            set_hash_addr(&mut info.leaf_addr, k);
            let tmp: Vec<u8> = pk_buffer[i*SPX_N..(i+1)*SPX_N].to_vec();
            thash(&mut pk_buffer[i*SPX_N..], &tmp, 1, ctx, &mut info.leaf_addr);
        }
    }
    thash(dest, &pk_buffer, SPX_WOTS_LEN, ctx, &mut info.pk_addr);
}

// ============================================================================
// Treehash (utilsx1.c) - wots and fors variants
// ============================================================================
fn wots_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                   leaf_idx: u32, idx_offset: u32, tree_height: u32,
                   tree_addr: &mut [u32; 8], info: &mut LeafInfoX1) {
    let th = tree_height as usize;
    let mut stack = vec![0u8; th * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;
    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[SPX_N..], ctx, idx.wrapping_add(idx_offset), info);
        let mut iio = idx_offset;
        let mut ii = idx;
        let mut il = leaf_idx;
        let mut h = 0u32;
        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2*SPX_N]);
                return;
            }
            if (ii ^ il) == 0x01 {
                auth_path[h as usize*SPX_N..(h as usize+1)*SPX_N].copy_from_slice(&current[SPX_N..2*SPX_N]);
            }
            if (ii & 1) == 0 && idx < max_idx { break; }
            iio >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, ii / 2 + iio);
            current[..SPX_N].copy_from_slice(&stack[h as usize*SPX_N..(h as usize+1)*SPX_N]);
            let tmp = current[..2*SPX_N].to_vec();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);
            h += 1; ii >>= 1; il >>= 1;
        }
        stack[h as usize*SPX_N..(h as usize+1)*SPX_N].copy_from_slice(&current[SPX_N..2*SPX_N]);
    }
}

struct ForsGenLeafInfo { leaf_addrx: [u32; 8] }

fn fors_gen_leafx1(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, info: &mut ForsGenLeafInfo) {
    set_tree_index(&mut info.leaf_addrx, addr_idx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSPRF);
    prf_addr(leaf, ctx, &info.leaf_addrx);
    set_type(&mut info.leaf_addrx, SPX_ADDR_TYPE_FORSTREE);
    let tmp: Vec<u8> = leaf[..SPX_N].to_vec();
    thash(leaf, &tmp, 1, ctx, &mut info.leaf_addrx);
}

fn fors_treehashx1(root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
                   leaf_idx: u32, idx_offset: u32, tree_height: u32,
                   tree_addr: &mut [u32; 8], info: &mut ForsGenLeafInfo) {
    let th = tree_height as usize;
    let mut stack = vec![0u8; th * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;
    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        fors_gen_leafx1(&mut current[SPX_N..], ctx, idx.wrapping_add(idx_offset), info);
        let mut iio = idx_offset;
        let mut ii = idx;
        let mut il = leaf_idx;
        let mut h = 0u32;
        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2*SPX_N]);
                return;
            }
            if (ii ^ il) == 0x01 {
                auth_path[h as usize*SPX_N..(h as usize+1)*SPX_N].copy_from_slice(&current[SPX_N..2*SPX_N]);
            }
            if (ii & 1) == 0 && idx < max_idx { break; }
            iio >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, ii / 2 + iio);
            current[..SPX_N].copy_from_slice(&stack[h as usize*SPX_N..(h as usize+1)*SPX_N]);
            let tmp = current[..2*SPX_N].to_vec();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);
            h += 1; ii >>= 1; il >>= 1;
        }
        stack[h as usize*SPX_N..(h as usize+1)*SPX_N].copy_from_slice(&current[SPX_N..2*SPX_N]);
    }
}

// ============================================================================
// compute_root (utils.c)
// ============================================================================
fn compute_root(root: &mut [u8], leaf: &[u8], mut leaf_idx: u32, mut idx_offset: u32,
                auth_path: &[u8], tree_height: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut buffer = [0u8; 2 * SPX_N];
    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2*SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[..SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2*SPX_N].copy_from_slice(&auth_path[..SPX_N]);
    }
    let mut ap = SPX_N;
    for i in 0..(tree_height - 1) as usize {
        leaf_idx >>= 1; idx_offset >>= 1;
        set_tree_height(addr, i as u32 + 1);
        set_tree_index(addr, leaf_idx + idx_offset);
        if leaf_idx & 1 != 0 {
            let tmp = buffer.to_vec();
            thash(&mut buffer[SPX_N..], &tmp, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[ap..ap+SPX_N]);
        } else {
            let tmp = buffer.to_vec();
            thash(&mut buffer[..SPX_N], &tmp, 2, ctx, addr);
            buffer[SPX_N..2*SPX_N].copy_from_slice(&auth_path[ap..ap+SPX_N]);
        }
        ap += SPX_N;
    }
    leaf_idx >>= 1; idx_offset >>= 1;
    set_tree_height(addr, tree_height);
    set_tree_index(addr, leaf_idx + idx_offset);
    let tmp = buffer.to_vec();
    thash(root, &tmp, 2, ctx, addr);
}

// ============================================================================
// FORS
// ============================================================================
fn message_to_indices(indices: &mut [u32], m: &[u8]) {
    let mut offset: usize = 0;
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
        prf_addr(&mut sig[sig_off..], ctx, &fors_tree_addr);
        set_type(&mut fors_tree_addr, SPX_ADDR_TYPE_FORSTREE);
        sig_off += SPX_N;
        fors_treehashx1(&mut roots[i*SPX_N..], &mut sig[sig_off..], ctx,
                        indices[i], idx_offset, SPX_FORS_HEIGHT as u32,
                        &mut fors_tree_addr, &mut fors_info);
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }
    thash(pk, &roots, SPX_FORS_TREES, ctx, &mut fors_pk_addr);
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
        // fors_sk_to_leaf
        thash(&mut leaf, &sig[sig_off..], 1, ctx, &mut fors_tree_addr);
        sig_off += SPX_N;
        compute_root(&mut roots[i*SPX_N..], &leaf, indices[i], idx_offset,
                     &sig[sig_off..], SPX_FORS_HEIGHT as u32, ctx, &mut fors_tree_addr);
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }
    thash(pk, &roots, SPX_FORS_TREES, ctx, &mut fors_pk_addr);
}

// ============================================================================
// Merkle
// ============================================================================
fn merkle_sign(sig: &mut [u8], root: &mut [u8], ctx: &SpxCtx,
               wots_addr: &mut [u32; 8], tree_addr: &mut [u32; 8], idx_leaf: u32) {
    let auth_path = SPX_WOTS_BYTES;
    let mut info = LeafInfoX1 {
        wots_sig: sig.as_mut_ptr(),
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
    wots_treehashx1(root, &mut sig[auth_path..], ctx, idx_leaf, 0, SPX_TREE_HEIGHT as u32, tree_addr, &mut info);
}

fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];
    set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr(&mut wots_addr, (SPX_D - 1) as u32);
    merkle_sign(&mut auth_path, root, ctx, &mut wots_addr, &mut top_tree_addr, !0u32);
}

// ============================================================================
// randombytes (from /dev/urandom)
// ============================================================================
fn randombytes(x: &mut [u8], xlen: usize) {
    use std::fs::File;
    use std::io::Read;
    let mut f = File::open("/dev/urandom").expect("open /dev/urandom");
    let mut remaining = xlen;
    let mut off = 0;
    while remaining > 0 {
        let n = f.read(&mut x[off..off+remaining]).unwrap_or(0);
        if n == 0 { std::thread::sleep(std::time::Duration::from_secs(1)); continue; }
        off += n; remaining -= n;
    }
}

// ============================================================================
// RNG (NIST DRBG - uses OpenSSL AES via C FFI)
// ============================================================================
extern "C" {
    fn EVP_CIPHER_CTX_new() -> *mut std::ffi::c_void;
    fn EVP_CIPHER_CTX_free(ctx: *mut std::ffi::c_void);
    fn EVP_aes_256_ecb() -> *const std::ffi::c_void;
    fn EVP_EncryptInit_ex(ctx: *mut std::ffi::c_void, cipher: *const std::ffi::c_void,
                          engine: *const std::ffi::c_void, key: *const u8, iv: *const u8) -> i32;
    fn EVP_EncryptUpdate(ctx: *mut std::ffi::c_void, out: *mut u8, outl: *mut i32,
                         inp: *const u8, inl: i32) -> i32;
}

fn aes256_ecb(key: &[u8], ctr: &[u8], buffer: &mut [u8]) {
    unsafe {
        let ctx = EVP_CIPHER_CTX_new();
        assert!(!ctx.is_null());
        EVP_EncryptInit_ex(ctx, EVP_aes_256_ecb(), ptr::null(), key.as_ptr(), ptr::null());
        let mut len: i32 = 0;
        EVP_EncryptUpdate(ctx, buffer.as_mut_ptr(), &mut len, ctr.as_ptr(), 16);
        EVP_CIPHER_CTX_free(ctx);
    }
}

static mut DRBG_CTX: AES256CtrDrbg = AES256CtrDrbg { key: [0u8; 32], v: [0u8; 16], reseed_counter: 0 };

struct AES256CtrDrbg { key: [u8; 32], v: [u8; 16], reseed_counter: i32 }

fn aes256_ctr_drbg_update(provided_data: Option<&[u8]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
    let mut temp = [0u8; 48];
    for i in 0..3 {
        for j in (0..16).rev() {
            if v[j] == 0xff { v[j] = 0x00; } else { v[j] += 1; break; }
        }
        aes256_ecb(key, v, &mut temp[16*i..16*i+16]);
    }
    if let Some(pd) = provided_data {
        for i in 0..48 { temp[i] ^= pd[i]; }
    }
    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..48]);
}

fn randombytes_init_internal(entropy_input: &[u8], personalization_string: Option<&[u8]>) {
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(&entropy_input[..48]);
    if let Some(ps) = personalization_string {
        for i in 0..48 { seed_material[i] ^= ps[i]; }
    }
    unsafe {
        DRBG_CTX.key = [0u8; 32];
        DRBG_CTX.v = [0u8; 16];
        aes256_ctr_drbg_update(Some(&seed_material), &mut DRBG_CTX.key, &mut DRBG_CTX.v);
        DRBG_CTX.reseed_counter = 1;
    }
}

fn randombytes_drbg(x: &mut [u8], mut xlen: usize) -> i32 {
    let mut block = [0u8; 16];
    let mut i = 0usize;
    unsafe {
        while xlen > 0 {
            for j in (0..16).rev() {
                if DRBG_CTX.v[j] == 0xff { DRBG_CTX.v[j] = 0x00; } else { DRBG_CTX.v[j] += 1; break; }
            }
            aes256_ecb(&DRBG_CTX.key, &DRBG_CTX.v, &mut block);
            if xlen > 15 {
                x[i..i+16].copy_from_slice(&block);
                i += 16; xlen -= 16;
            } else {
                x[i..i+xlen].copy_from_slice(&block[..xlen]);
                xlen = 0;
            }
        }
        aes256_ctr_drbg_update(None, &mut DRBG_CTX.key, &mut DRBG_CTX.v);
        DRBG_CTX.reseed_counter += 1;
    }
    0
}

// ============================================================================
// Public API (extern "C")
// ============================================================================
#[unsafe(no_mangle)]
pub extern "C" fn SPX_initialize_hash_function(ctx: *mut SpxCtx) {
    let ctx = unsafe { &mut *ctx };
    initialize_hash_function_internal(ctx);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    unsafe {
        let ctx = &*ctx;
        let addr = &*(addr as *const [u32; 8]);
        prf_addr(std::slice::from_raw_parts_mut(out, SPX_N), ctx, addr);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_gen_message_random(r: *mut u8, sk_prf: *const u8, optrand: *const u8,
                                         m: *const u8, mlen: u64, ctx: *const SpxCtx) {
    unsafe {
        let ctx = &*ctx;
        gen_message_random(
            std::slice::from_raw_parts_mut(r, SPX_N),
            std::slice::from_raw_parts(sk_prf, SPX_N),
            std::slice::from_raw_parts(optrand, SPX_N),
            std::slice::from_raw_parts(m, mlen as usize),
            mlen, ctx);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_hash_message(digest: *mut u8, tree: *mut u64, leaf_idx: *mut u32,
                                    r: *const u8, pk: *const u8, m: *const u8, mlen: u64,
                                    ctx: *const SpxCtx) {
    unsafe {
        let ctx = &*ctx;
        hash_message(
            std::slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES),
            &mut *tree, &mut *leaf_idx,
            std::slice::from_raw_parts(r, SPX_N),
            std::slice::from_raw_parts(pk, SPX_PK_BYTES),
            std::slice::from_raw_parts(m, mlen as usize),
            mlen, ctx);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> u64 { CRYPTO_SECRETKEYBYTES as u64 }

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> u64 { CRYPTO_PUBLICKEYBYTES as u64 }

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> u64 { CRYPTO_BYTES as u64 }

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> u64 { CRYPTO_SEEDBYTES as u64 }

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seed_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) -> i32 {
    unsafe {
        let sk_s = std::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES);
        let pk_s = std::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES);
        let seed_s = std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES);
        let mut ctx = SpxCtx { pub_seed: [0;SPX_N], sk_seed: [0;SPX_N], state_seeded: [0;40] };
        sk_s[..CRYPTO_SEEDBYTES].copy_from_slice(seed_s);
        pk_s[..SPX_N].copy_from_slice(&sk_s[2*SPX_N..3*SPX_N]);
        ctx.pub_seed.copy_from_slice(&pk_s[..SPX_N]);
        ctx.sk_seed.copy_from_slice(&sk_s[..SPX_N]);
        initialize_hash_function_internal(&mut ctx);
        merkle_gen_root(&mut sk_s[3*SPX_N..], &ctx);
        pk_s[SPX_N..2*SPX_N].copy_from_slice(&sk_s[3*SPX_N..4*SPX_N]);
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    randombytes(&mut seed, CRYPTO_SEEDBYTES);
    crypto_sign_seed_keypair(pk, sk, seed.as_ptr());
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(sig: *mut u8, siglen: *mut usize,
                                         m: *const u8, mlen: usize, sk: *const u8) -> i32 {
    unsafe {
        let sk_s = std::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES);
        let m_s = std::slice::from_raw_parts(m, mlen);
        let sig_s = std::slice::from_raw_parts_mut(sig, SPX_BYTES);
        let mut ctx = SpxCtx { pub_seed: [0;SPX_N], sk_seed: [0;SPX_N], state_seeded: [0;40] };
        let sk_prf = &sk_s[SPX_N..2*SPX_N];
        let pk = &sk_s[2*SPX_N..];
        ctx.sk_seed.copy_from_slice(&sk_s[..SPX_N]);
        ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
        initialize_hash_function_internal(&mut ctx);
        let mut wots_addr = [0u32; 8];
        let mut tree_addr = [0u32; 8];
        set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
        set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
        let mut optrand = [0u8; SPX_N];
        randombytes(&mut optrand, SPX_N);
        gen_message_random(sig_s, sk_prf, &optrand, m_s, mlen as u64, &ctx);
        let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
        let mut root = [0u8; SPX_N];
        let mut tree: u64 = 0;
        let mut idx_leaf: u32 = 0;
        hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig_s, pk, m_s, mlen as u64, &ctx);
        let mut sig_off = SPX_N;
        set_tree_addr(&mut wots_addr, tree);
        set_keypair_addr(&mut wots_addr, idx_leaf);
        fors_sign(&mut sig_s[sig_off..], &mut root, &mhash, &ctx, &wots_addr);
        sig_off += SPX_FORS_BYTES;
        for i in 0..SPX_D {
            set_layer_addr(&mut tree_addr, i as u32);
            set_tree_addr(&mut tree_addr, tree);
            copy_subtree_addr(&mut wots_addr, &tree_addr);
            set_keypair_addr(&mut wots_addr, idx_leaf);
            merkle_sign(&mut sig_s[sig_off..], &mut root, &ctx, &mut wots_addr, &mut tree_addr, idx_leaf);
            sig_off += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;
            idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
            tree >>= SPX_TREE_HEIGHT;
        }
        *siglen = SPX_BYTES;
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(sig: *const u8, siglen: usize,
                                      m: *const u8, mlen: usize, pk: *const u8) -> i32 {
    unsafe {
        let sig_s = std::slice::from_raw_parts(sig, siglen);
        let m_s = std::slice::from_raw_parts(m, mlen);
        let pk_s = std::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES);
        if siglen != SPX_BYTES { return -1; }
        let mut ctx = SpxCtx { pub_seed: [0;SPX_N], sk_seed: [0;SPX_N], state_seeded: [0;40] };
        ctx.pub_seed.copy_from_slice(&pk_s[..SPX_N]);
        initialize_hash_function_internal(&mut ctx);
        let pub_root = &pk_s[SPX_N..];
        let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
        let mut wots_pk = [0u8; SPX_WOTS_BYTES];
        let mut root = [0u8; SPX_N];
        let mut leaf = [0u8; SPX_N];
        let mut tree: u64 = 0;
        let mut idx_leaf: u32 = 0;
        let mut wots_addr = [0u32; 8];
        let mut tree_addr = [0u32; 8];
        let mut wots_pk_addr = [0u32; 8];
        set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
        set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
        set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);
        hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig_s, pk_s, m_s, mlen as u64, &ctx);
        let mut sig_off = SPX_N;
        set_tree_addr(&mut wots_addr, tree);
        set_keypair_addr(&mut wots_addr, idx_leaf);
        fors_pk_from_sig(&mut root, &sig_s[sig_off..], &mhash, &ctx, &wots_addr);
        sig_off += SPX_FORS_BYTES;
        for i in 0..SPX_D {
            set_layer_addr(&mut tree_addr, i as u32);
            set_tree_addr(&mut tree_addr, tree);
            copy_subtree_addr(&mut wots_addr, &tree_addr);
            set_keypair_addr(&mut wots_addr, idx_leaf);
            copy_keypair_addr(&mut wots_pk_addr, &wots_addr);
            wots_pk_from_sig(&mut wots_pk, &sig_s[sig_off..], &root, &ctx, &mut wots_addr);
            sig_off += SPX_WOTS_BYTES;
            thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &mut wots_pk_addr);
            compute_root(&mut root, &leaf, idx_leaf, 0, &sig_s[sig_off..], SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr);
            sig_off += SPX_TREE_HEIGHT * SPX_N;
            idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
            tree >>= SPX_TREE_HEIGHT;
        }
        if root[..SPX_N] != pub_root[..SPX_N] { return -1; }
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(sm: *mut u8, smlen: *mut u64,
                               m: *const u8, mlen: u64, sk: *const u8) -> i32 {
    unsafe {
        let mut siglen: usize = 0;
        crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);
        ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
        *smlen = siglen as u64 + mlen;
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(m: *mut u8, mlen: *mut u64,
                                    sm: *const u8, smlen: u64, pk: *const u8) -> i32 {
    unsafe {
        let smlen_usize = smlen as usize;
        if smlen_usize < SPX_BYTES {
            ptr::write_bytes(m, 0, smlen_usize);
            *mlen = 0;
            return -1;
        }
        *mlen = smlen - SPX_BYTES as u64;
        if crypto_sign_verify(sm, SPX_BYTES, sm.add(SPX_BYTES), *mlen as usize, pk) != 0 {
            ptr::write_bytes(m, 0, smlen_usize);
            *mlen = 0;
            return -1;
        }
        ptr::copy(sm.add(SPX_BYTES), m, *mlen as usize);
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(entropy_input: *const u8, personalization_string: *const u8) {
    unsafe {
        let ei = std::slice::from_raw_parts(entropy_input, 48);
        let ps = if personalization_string.is_null() { None }
                 else { Some(std::slice::from_raw_parts(personalization_string, 48)) };
        randombytes_init_internal(ei, ps);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_randombytes(x: *mut u8, xlen: u64) -> i32 {
    unsafe {
        randombytes_drbg(std::slice::from_raw_parts_mut(x, xlen as usize), xlen as usize)
    }
}

// seedexpander functions
#[repr(C)]
pub struct AES_XOF_struct {
    buffer: [u8; 16],
    buffer_pos: u64,
    length_remaining: u64,
    key: [u8; 32],
    ctr: [u8; 16],
}

const RNG_SUCCESS: i32 = 0;
const RNG_BAD_MAXLEN: i32 = -1;
const RNG_BAD_OUTBUF: i32 = -2;
const RNG_BAD_REQ_LEN: i32 = -3;

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander_init(ctx: *mut AES_XOF_struct, seed: *const u8,
                                     diversifier: *const u8, maxlen: u64) -> i32 {
    unsafe {
        let ctx = &mut *ctx;
        if maxlen >= 0x100000000 { return RNG_BAD_MAXLEN; }
        ctx.length_remaining = maxlen;
        ctx.key.copy_from_slice(std::slice::from_raw_parts(seed, 32));
        ctx.ctr[..8].copy_from_slice(std::slice::from_raw_parts(diversifier, 8));
        let mut ml = maxlen;
        ctx.ctr[11] = (ml % 256) as u8; ml >>= 8;
        ctx.ctr[10] = (ml % 256) as u8; ml >>= 8;
        ctx.ctr[9] = (ml % 256) as u8; ml >>= 8;
        ctx.ctr[8] = (ml % 256) as u8;
        ctx.ctr[12..16].fill(0);
        ctx.buffer_pos = 16;
        ctx.buffer.fill(0);
        RNG_SUCCESS
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander(ctx: *mut AES_XOF_struct, x: *mut u8, mut xlen: u64) -> i32 {
    unsafe {
        let ctx = &mut *ctx;
        if x.is_null() { return RNG_BAD_OUTBUF; }
        if xlen >= ctx.length_remaining { return RNG_BAD_REQ_LEN; }
        ctx.length_remaining -= xlen;
        let mut offset: u64 = 0;
        while xlen > 0 {
            let bp = ctx.buffer_pos as u64;
            if xlen <= 16 - bp {
                ptr::copy_nonoverlapping(ctx.buffer.as_ptr().add(bp as usize), x.add(offset as usize), xlen as usize);
                ctx.buffer_pos += xlen;
                return RNG_SUCCESS;
            }
            let take = 16 - bp;
            ptr::copy_nonoverlapping(ctx.buffer.as_ptr().add(bp as usize), x.add(offset as usize), take as usize);
            xlen -= take;
            offset += take;
            aes256_ecb(&ctx.key, &ctx.ctr, &mut ctx.buffer);
            ctx.buffer_pos = 0;
            for i in (12..16).rev() {
                if ctx.ctr[i] == 0xff { ctx.ctr[i] = 0x00; } else { ctx.ctr[i] += 1; break; }
            }
        }
        RNG_SUCCESS
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(provided_data: *const u8, key: *mut u8, v: *mut u8) {
    unsafe {
        let key_s = &mut *(key as *mut [u8; 32]);
        let v_s = &mut *(v as *mut [u8; 16]);
        let pd = if provided_data.is_null() { None }
                 else { Some(std::slice::from_raw_parts(provided_data, 48)) };
        aes256_ctr_drbg_update(pd, key_s, v_s);
    }
}
