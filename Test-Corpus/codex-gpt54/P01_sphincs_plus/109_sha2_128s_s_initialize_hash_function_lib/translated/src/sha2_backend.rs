use crate::address::addr_bytes;
use crate::context::spx_ctx;
use crate::params::*;
use crate::utils::{bytes_to_ull_impl, u32_to_bytes_into};
use cipher::generic_array::{GenericArray, typenum::{U64, U128}};
use sha2::{Digest, Sha256, Sha512, compress256, compress512};

const IV256: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];
const IV512: [u64; 8] = [
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
];

fn load_be_u32(input: &[u8]) -> u32 {
    ((input[0] as u32) << 24) | ((input[1] as u32) << 16) | ((input[2] as u32) << 8) | (input[3] as u32)
}

fn load_be_u64(input: &[u8]) -> u64 {
    ((input[0] as u64) << 56)
        | ((input[1] as u64) << 48)
        | ((input[2] as u64) << 40)
        | ((input[3] as u64) << 32)
        | ((input[4] as u64) << 24)
        | ((input[5] as u64) << 16)
        | ((input[6] as u64) << 8)
        | (input[7] as u64)
}

fn store_be_u32(out: &mut [u8], value: u32) {
    out[0] = (value >> 24) as u8;
    out[1] = (value >> 16) as u8;
    out[2] = (value >> 8) as u8;
    out[3] = value as u8;
}

fn store_be_u64(out: &mut [u8], value: u64) {
    out[0] = (value >> 56) as u8;
    out[1] = (value >> 48) as u8;
    out[2] = (value >> 40) as u8;
    out[3] = (value >> 32) as u8;
    out[4] = (value >> 24) as u8;
    out[5] = (value >> 16) as u8;
    out[6] = (value >> 8) as u8;
    out[7] = value as u8;
}

fn sha256_state_words(state: &[u8; 40]) -> ([u32; 8], u64) {
    let mut words = [0u32; 8];
    for i in 0..8 {
        words[i] = load_be_u32(&state[i * 4..i * 4 + 4]);
    }
    (words, load_be_u64(&state[32..40]))
}

fn store_sha256_state(state: &mut [u8; 40], words: &[u32; 8], bytes: u64) {
    for i in 0..8 {
        store_be_u32(&mut state[i * 4..i * 4 + 4], words[i]);
    }
    store_be_u64(&mut state[32..40], bytes);
}

fn sha512_state_words(state: &[u8; 72]) -> ([u64; 8], u64) {
    let mut words = [0u64; 8];
    for i in 0..8 {
        words[i] = load_be_u64(&state[i * 8..i * 8 + 8]);
    }
    (words, load_be_u64(&state[64..72]))
}

fn store_sha512_state(state: &mut [u8; 72], words: &[u64; 8], bytes: u64) {
    for i in 0..8 {
        store_be_u64(&mut state[i * 8..i * 8 + 8], words[i]);
    }
    store_be_u64(&mut state[64..72], bytes);
}

fn sha256_once(input: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(input);
    hasher.finalize().into()
}

fn sha512_once(input: &[u8]) -> [u8; 64] {
    let mut hasher = Sha512::new();
    hasher.update(input);
    hasher.finalize().into()
}

pub fn sha256_inc_init_rs(state: &mut [u8; 40]) {
    store_sha256_state(state, &IV256, 0);
}

pub fn sha512_inc_init_rs(state: &mut [u8; 72]) {
    store_sha512_state(state, &IV512, 0);
}

pub fn sha256_inc_blocks_rs(state: &mut [u8; 40], input: &[u8], inblocks: usize) {
    let (mut words, mut bytes) = sha256_state_words(state);
    let blocks = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const GenericArray<u8, U64>, inblocks) };
    compress256(&mut words, blocks);
    bytes += (64 * inblocks) as u64;
    store_sha256_state(state, &words, bytes);
}

pub fn sha512_inc_blocks_rs(state: &mut [u8; 72], input: &[u8], inblocks: usize) {
    let (mut words, mut bytes) = sha512_state_words(state);
    let blocks = unsafe { std::slice::from_raw_parts(input.as_ptr() as *const GenericArray<u8, U128>, inblocks) };
    compress512(&mut words, blocks);
    bytes += (128 * inblocks) as u64;
    store_sha512_state(state, &words, bytes);
}

pub fn sha256_inc_finalize_rs(out: &mut [u8; 32], state: &mut [u8; 40], input: &[u8]) {
    let (_, bytes_done) = sha256_state_words(state);
    let bytes = bytes_done + input.len() as u64;
    let full_len = input.len() & !63;
    if full_len > 0 {
        sha256_inc_blocks_rs(state, &input[..full_len], full_len / 64);
    }
    let rem = &input[full_len..];
    let mut padded = [0u8; 128];
    padded[..rem.len()].copy_from_slice(rem);
    padded[rem.len()] = 0x80;
    if rem.len() < 56 {
        padded[56] = (bytes >> 53) as u8;
        padded[57] = (bytes >> 45) as u8;
        padded[58] = (bytes >> 37) as u8;
        padded[59] = (bytes >> 29) as u8;
        padded[60] = (bytes >> 21) as u8;
        padded[61] = (bytes >> 13) as u8;
        padded[62] = (bytes >> 5) as u8;
        padded[63] = (bytes << 3) as u8;
        sha256_inc_blocks_rs(state, &padded[..64], 1);
    } else {
        padded[120] = (bytes >> 53) as u8;
        padded[121] = (bytes >> 45) as u8;
        padded[122] = (bytes >> 37) as u8;
        padded[123] = (bytes >> 29) as u8;
        padded[124] = (bytes >> 21) as u8;
        padded[125] = (bytes >> 13) as u8;
        padded[126] = (bytes >> 5) as u8;
        padded[127] = (bytes << 3) as u8;
        sha256_inc_blocks_rs(state, &padded, 2);
    }
    out.copy_from_slice(&state[..32]);
}

pub fn sha512_inc_finalize_rs(out: &mut [u8; 64], state: &mut [u8; 72], input: &[u8]) {
    let (_, bytes_done) = sha512_state_words(state);
    let bytes = bytes_done + input.len() as u64;
    let full_len = input.len() & !127;
    if full_len > 0 {
        sha512_inc_blocks_rs(state, &input[..full_len], full_len / 128);
    }
    let rem = &input[full_len..];
    let mut padded = [0u8; 256];
    padded[..rem.len()].copy_from_slice(rem);
    padded[rem.len()] = 0x80;
    if rem.len() < 112 {
        padded[119] = (bytes >> 61) as u8;
        padded[120] = (bytes >> 53) as u8;
        padded[121] = (bytes >> 45) as u8;
        padded[122] = (bytes >> 37) as u8;
        padded[123] = (bytes >> 29) as u8;
        padded[124] = (bytes >> 21) as u8;
        padded[125] = (bytes >> 13) as u8;
        padded[126] = (bytes >> 5) as u8;
        padded[127] = (bytes << 3) as u8;
        sha512_inc_blocks_rs(state, &padded[..128], 1);
    } else {
        padded[247] = (bytes >> 61) as u8;
        padded[248] = (bytes >> 53) as u8;
        padded[249] = (bytes >> 45) as u8;
        padded[250] = (bytes >> 37) as u8;
        padded[251] = (bytes >> 29) as u8;
        padded[252] = (bytes >> 21) as u8;
        padded[253] = (bytes >> 13) as u8;
        padded[254] = (bytes >> 5) as u8;
        padded[255] = (bytes << 3) as u8;
        sha512_inc_blocks_rs(state, &padded, 2);
    }
    out.copy_from_slice(&state[..64]);
}

pub(crate) fn mgf1_256_rs(out: &mut [u8], input: &[u8]) {
    let mut inbuf = vec![0u8; input.len() + 4];
    inbuf[..input.len()].copy_from_slice(input);
    let mut i = 0u32;
    let mut off = 0usize;
    while off + SPX_SHA256_OUTPUT_BYTES <= out.len() {
        u32_to_bytes_into(&mut inbuf[input.len()..], i);
        let digest = sha256_once(&inbuf);
        out[off..off + SPX_SHA256_OUTPUT_BYTES].copy_from_slice(&digest);
        off += SPX_SHA256_OUTPUT_BYTES;
        i += 1;
    }
    if off < out.len() {
        u32_to_bytes_into(&mut inbuf[input.len()..], i);
        let digest = sha256_once(&inbuf);
        let rem = out.len() - off;
        out[off..].copy_from_slice(&digest[..rem]);
    }
}

pub(crate) fn mgf1_512_rs(out: &mut [u8], input: &[u8]) {
    let mut inbuf = vec![0u8; input.len() + 4];
    inbuf[..input.len()].copy_from_slice(input);
    let mut i = 0u32;
    let mut off = 0usize;
    while off + SPX_SHA512_OUTPUT_BYTES <= out.len() {
        u32_to_bytes_into(&mut inbuf[input.len()..], i);
        let digest = sha512_once(&inbuf);
        out[off..off + SPX_SHA512_OUTPUT_BYTES].copy_from_slice(&digest);
        off += SPX_SHA512_OUTPUT_BYTES;
        i += 1;
    }
    if off < out.len() {
        u32_to_bytes_into(&mut inbuf[input.len()..], i);
        let digest = sha512_once(&inbuf);
        let rem = out.len() - off;
        out[off..].copy_from_slice(&digest[..rem]);
    }
}

pub(crate) fn seed_state_rs(ctx: &mut spx_ctx) {
    let mut block = [0u8; SPX_SHA512_BLOCK_BYTES];
    block[..SPX_N].copy_from_slice(&ctx.pub_seed);
    sha256_inc_init_rs(&mut ctx.state_seeded);
    sha256_inc_blocks_rs(&mut ctx.state_seeded, &block[..64], 1);
    sha512_inc_init_rs(&mut ctx.state_seeded_512);
    sha512_inc_blocks_rs(&mut ctx.state_seeded_512, &block, 1);
}

pub(crate) fn initialize_hash_function_rs(ctx: &mut spx_ctx) {
    seed_state_rs(ctx);
}

fn sha256_seeded_finalize(seed_state: &[u8; 40], input: &[u8]) -> [u8; 32] {
    let mut state = *seed_state;
    let mut out = [0u8; 32];
    sha256_inc_finalize_rs(&mut out, &mut state, input);
    out
}

fn sha512_seeded_finalize(seed_state: &[u8; 72], input: &[u8]) -> [u8; 64] {
    let mut state = *seed_state;
    let mut out = [0u8; 64];
    sha512_inc_finalize_rs(&mut out, &mut state, input);
    out
}

pub(crate) fn SPX_prf_addr_rs(out: &mut [u8], ctx: &spx_ctx, addr: &[u32; 8]) {
    let mut buf = [0u8; SPX_SHA256_ADDR_BYTES + 32];
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);
    let digest = sha256_seeded_finalize(&ctx.state_seeded, &buf[..SPX_SHA256_ADDR_BYTES + SPX_N]);
    out.copy_from_slice(&digest[..SPX_N]);
}

pub(crate) fn SPX_gen_message_random_rs(
    out: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
) {
    if SPX_N >= 24 {
        let mut buf = vec![0u8; SPX_SHA512_BLOCK_BYTES + SPX_SHA512_OUTPUT_BYTES];
        for i in 0..SPX_N {
            buf[i] = 0x36 ^ sk_prf[i];
        }
        buf[SPX_N..SPX_SHA512_BLOCK_BYTES].fill(0x36);
        let mut state = [0u8; 72];
        sha512_inc_init_rs(&mut state);
        sha512_inc_blocks_rs(&mut state, &buf[..SPX_SHA512_BLOCK_BYTES], 1);
        buf[..SPX_N].copy_from_slice(optrand);
        if SPX_N + m.len() < SPX_SHA512_BLOCK_BYTES {
            buf[SPX_N..SPX_N + m.len()].copy_from_slice(m);
            let mut inner = [0u8; 64];
            sha512_inc_finalize_rs(&mut inner, &mut state, &buf[..SPX_N + m.len()]);
            buf[SPX_SHA512_BLOCK_BYTES..SPX_SHA512_BLOCK_BYTES + 64].copy_from_slice(&inner);
        } else {
            let take = SPX_SHA512_BLOCK_BYTES - SPX_N;
            buf[SPX_N..SPX_SHA512_BLOCK_BYTES].copy_from_slice(&m[..take]);
            sha512_inc_blocks_rs(&mut state, &buf[..SPX_SHA512_BLOCK_BYTES], 1);
            let mut inner = [0u8; 64];
            sha512_inc_finalize_rs(&mut inner, &mut state, &m[take..]);
            buf[SPX_SHA512_BLOCK_BYTES..SPX_SHA512_BLOCK_BYTES + 64].copy_from_slice(&inner);
        }
        for i in 0..SPX_N {
            buf[i] = 0x5c ^ sk_prf[i];
        }
        buf[SPX_N..SPX_SHA512_BLOCK_BYTES].fill(0x5c);
        let digest = sha512_once(&buf[..SPX_SHA512_BLOCK_BYTES + 64]);
        out.copy_from_slice(&digest[..SPX_N]);
    } else {
        let mut buf = vec![0u8; SPX_SHA256_BLOCK_BYTES + SPX_SHA256_OUTPUT_BYTES];
        for i in 0..SPX_N {
            buf[i] = 0x36 ^ sk_prf[i];
        }
        buf[SPX_N..SPX_SHA256_BLOCK_BYTES].fill(0x36);
        let mut state = [0u8; 40];
        sha256_inc_init_rs(&mut state);
        sha256_inc_blocks_rs(&mut state, &buf[..SPX_SHA256_BLOCK_BYTES], 1);
        buf[..SPX_N].copy_from_slice(optrand);
        if SPX_N + m.len() < SPX_SHA256_BLOCK_BYTES {
            buf[SPX_N..SPX_N + m.len()].copy_from_slice(m);
            let mut inner = [0u8; 32];
            sha256_inc_finalize_rs(&mut inner, &mut state, &buf[..SPX_N + m.len()]);
            buf[SPX_SHA256_BLOCK_BYTES..SPX_SHA256_BLOCK_BYTES + 32].copy_from_slice(&inner);
        } else {
            let take = SPX_SHA256_BLOCK_BYTES - SPX_N;
            buf[SPX_N..SPX_SHA256_BLOCK_BYTES].copy_from_slice(&m[..take]);
            sha256_inc_blocks_rs(&mut state, &buf[..SPX_SHA256_BLOCK_BYTES], 1);
            let mut inner = [0u8; 32];
            sha256_inc_finalize_rs(&mut inner, &mut state, &m[take..]);
            buf[SPX_SHA256_BLOCK_BYTES..SPX_SHA256_BLOCK_BYTES + 32].copy_from_slice(&inner);
        }
        for i in 0..SPX_N {
            buf[i] = 0x5c ^ sk_prf[i];
        }
        buf[SPX_N..SPX_SHA256_BLOCK_BYTES].fill(0x5c);
        let digest = sha256_once(&buf[..SPX_SHA256_BLOCK_BYTES + 32]);
        out.copy_from_slice(&digest[..SPX_N]);
    }
}

pub(crate) fn SPX_hash_message_rs(
    digest: &mut [u8],
    tree: &mut u64,
    leaf_idx: &mut u32,
    r: &[u8],
    pk: &[u8],
    m: &[u8],
) {
    const fn tree_bits() -> usize {
        SPX_TREE_HEIGHT * (SPX_D - 1)
    }
    const fn tree_bytes() -> usize {
        tree_bits().div_ceil(8)
    }
    const fn leaf_bits() -> usize {
        SPX_TREE_HEIGHT
    }
    const fn leaf_bytes() -> usize {
        leaf_bits().div_ceil(8)
    }
    const fn dgst_bytes() -> usize {
        SPX_FORS_MSG_BYTES + tree_bytes() + leaf_bytes()
    }
    if SPX_N >= 24 {
        const INBLOCKS: usize = (SPX_N + SPX_PK_BYTES + SPX_SHA512_BLOCK_BYTES - 1) / SPX_SHA512_BLOCK_BYTES;
        let mut seed = [0u8; 2 * 32 + 64];
        let mut inbuf = vec![0u8; INBLOCKS * SPX_SHA512_BLOCK_BYTES];
        let mut state = [0u8; 72];
        sha512_inc_init_rs(&mut state);
        inbuf[..SPX_N].copy_from_slice(r);
        inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(pk);
        if SPX_N + SPX_PK_BYTES + m.len() < inbuf.len() {
            inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + m.len()].copy_from_slice(m);
            let mut digest_seed = [0u8; 64];
            sha512_inc_finalize_rs(&mut digest_seed, &mut state, &inbuf[..SPX_N + SPX_PK_BYTES + m.len()]);
            seed[2 * SPX_N..2 * SPX_N + 64].copy_from_slice(&digest_seed);
        } else {
            let take = inbuf.len() - SPX_N - SPX_PK_BYTES;
            inbuf[SPX_N + SPX_PK_BYTES..].copy_from_slice(&m[..take]);
            sha512_inc_blocks_rs(&mut state, &inbuf, INBLOCKS);
            let mut digest_seed = [0u8; 64];
            sha512_inc_finalize_rs(&mut digest_seed, &mut state, &m[take..]);
            seed[2 * SPX_N..2 * SPX_N + 64].copy_from_slice(&digest_seed);
        }
        seed[..SPX_N].copy_from_slice(r);
        seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);
        let mut buf = vec![0u8; dgst_bytes()];
        mgf1_512_rs(&mut buf, &seed[..2 * SPX_N + 64]);
        digest.copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
        let mut off = SPX_FORS_MSG_BYTES;
        *tree = if SPX_D == 1 {
            0
        } else {
            bytes_to_ull_impl(&buf[off..off + tree_bytes()]) & (u64::MAX >> (64 - tree_bits()))
        };
        off += tree_bytes();
        *leaf_idx = bytes_to_ull_impl(&buf[off..off + leaf_bytes()]) as u32
            & (u32::MAX >> (32 - leaf_bits()));
    } else {
        const INBLOCKS: usize = (SPX_N + SPX_PK_BYTES + SPX_SHA256_BLOCK_BYTES - 1) / SPX_SHA256_BLOCK_BYTES;
        let mut seed = [0u8; 2 * 32 + 64];
        let mut inbuf = vec![0u8; INBLOCKS * SPX_SHA256_BLOCK_BYTES];
        let mut state = [0u8; 40];
        sha256_inc_init_rs(&mut state);
        inbuf[..SPX_N].copy_from_slice(r);
        inbuf[SPX_N..SPX_N + SPX_PK_BYTES].copy_from_slice(pk);
        if SPX_N + SPX_PK_BYTES + m.len() < inbuf.len() {
            inbuf[SPX_N + SPX_PK_BYTES..SPX_N + SPX_PK_BYTES + m.len()].copy_from_slice(m);
            let mut digest_seed = [0u8; 32];
            sha256_inc_finalize_rs(&mut digest_seed, &mut state, &inbuf[..SPX_N + SPX_PK_BYTES + m.len()]);
            seed[2 * SPX_N..2 * SPX_N + 32].copy_from_slice(&digest_seed);
        } else {
            let take = inbuf.len() - SPX_N - SPX_PK_BYTES;
            inbuf[SPX_N + SPX_PK_BYTES..].copy_from_slice(&m[..take]);
            sha256_inc_blocks_rs(&mut state, &inbuf, INBLOCKS);
            let mut digest_seed = [0u8; 32];
            sha256_inc_finalize_rs(&mut digest_seed, &mut state, &m[take..]);
            seed[2 * SPX_N..2 * SPX_N + 32].copy_from_slice(&digest_seed);
        }
        seed[..SPX_N].copy_from_slice(r);
        seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);
        let mut buf = vec![0u8; dgst_bytes()];
        mgf1_256_rs(&mut buf, &seed[..2 * SPX_N + 32]);
        digest.copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
        let mut off = SPX_FORS_MSG_BYTES;
        *tree = if SPX_D == 1 {
            0
        } else {
            bytes_to_ull_impl(&buf[off..off + tree_bytes()]) & (u64::MAX >> (64 - tree_bits()))
        };
        off += tree_bytes();
        *leaf_idx = bytes_to_ull_impl(&buf[off..off + leaf_bytes()]) as u32
            & (u32::MAX >> (32 - leaf_bits()));
    }
}

pub(crate) fn SPX_thash_rs(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &spx_ctx, addr: &mut [u32; 8]) {
    if THASH_ROBUST {
        if SPX_SHA512 && inblocks > 1 {
            let mut bitmask = vec![0u8; inblocks as usize * SPX_N];
            let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + inblocks as usize * SPX_N];
            buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
            buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
            mgf1_512_rs(&mut bitmask, &buf[..SPX_N + SPX_SHA256_ADDR_BYTES]);
            for i in 0..bitmask.len() {
                buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = input[i] ^ bitmask[i];
            }
            let digest = sha512_seeded_finalize(&ctx.state_seeded_512, &buf[SPX_N..]);
            out.copy_from_slice(&digest[..SPX_N]);
        } else {
            let mut bitmask = vec![0u8; inblocks as usize * SPX_N];
            let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + inblocks as usize * SPX_N];
            buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
            buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
            mgf1_256_rs(&mut bitmask, &buf[..SPX_N + SPX_SHA256_ADDR_BYTES]);
            for i in 0..bitmask.len() {
                buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = input[i] ^ bitmask[i];
            }
            let digest = sha256_seeded_finalize(&ctx.state_seeded, &buf[SPX_N..]);
            out.copy_from_slice(&digest[..SPX_N]);
        }
    } else if SPX_SHA512 && inblocks > 1 {
        let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + inblocks as usize * SPX_N];
        buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
        buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(input);
        let digest = sha512_seeded_finalize(&ctx.state_seeded_512, &buf);
        out.copy_from_slice(&digest[..SPX_N]);
    } else {
        let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + inblocks as usize * SPX_N];
        buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
        buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(input);
        let digest = sha256_seeded_finalize(&ctx.state_seeded, &buf);
        out.copy_from_slice(&digest[..SPX_N]);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn sha256_inc_init(state: *mut u8) {
    sha256_inc_init_rs(unsafe { &mut *(state as *mut [u8; 40]) });
}

#[unsafe(no_mangle)]
pub extern "C" fn sha256_inc_blocks(state: *mut u8, input: *const u8, inblocks: usize) {
    sha256_inc_blocks_rs(
        unsafe { &mut *(state as *mut [u8; 40]) },
        unsafe { std::slice::from_raw_parts(input, inblocks * 64) },
        inblocks,
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn sha256_inc_finalize(out: *mut u8, state: *mut u8, input: *const u8, inlen: usize) {
    sha256_inc_finalize_rs(
        unsafe { &mut *(out as *mut [u8; 32]) },
        unsafe { &mut *(state as *mut [u8; 40]) },
        unsafe { std::slice::from_raw_parts(input, inlen) },
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn sha256(out: *mut u8, input: *const u8, inlen: usize) {
    let digest = sha256_once(unsafe { std::slice::from_raw_parts(input, inlen) });
    unsafe { std::slice::from_raw_parts_mut(out, 32) }.copy_from_slice(&digest);
}

#[unsafe(no_mangle)]
pub extern "C" fn sha512_inc_init(state: *mut u8) {
    sha512_inc_init_rs(unsafe { &mut *(state as *mut [u8; 72]) });
}

#[unsafe(no_mangle)]
pub extern "C" fn sha512_inc_blocks(state: *mut u8, input: *const u8, inblocks: usize) {
    sha512_inc_blocks_rs(
        unsafe { &mut *(state as *mut [u8; 72]) },
        unsafe { std::slice::from_raw_parts(input, inblocks * 128) },
        inblocks,
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn sha512_inc_finalize(out: *mut u8, state: *mut u8, input: *const u8, inlen: usize) {
    sha512_inc_finalize_rs(
        unsafe { &mut *(out as *mut [u8; 64]) },
        unsafe { &mut *(state as *mut [u8; 72]) },
        unsafe { std::slice::from_raw_parts(input, inlen) },
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn sha512(out: *mut u8, input: *const u8, inlen: usize) {
    let digest = sha512_once(unsafe { std::slice::from_raw_parts(input, inlen) });
    unsafe { std::slice::from_raw_parts_mut(out, 64) }.copy_from_slice(&digest);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_mgf1_256(out: *mut u8, outlen: u64, input: *const u8, inlen: u64) {
    mgf1_256_rs(
        unsafe { std::slice::from_raw_parts_mut(out, outlen as usize) },
        unsafe { std::slice::from_raw_parts(input, inlen as usize) },
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_mgf1_512(out: *mut u8, outlen: u64, input: *const u8, inlen: u64) {
    mgf1_512_rs(
        unsafe { std::slice::from_raw_parts_mut(out, outlen as usize) },
        unsafe { std::slice::from_raw_parts(input, inlen as usize) },
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_seed_state(ctx: *mut spx_ctx) {
    seed_state_rs(unsafe { &mut *ctx });
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_initialize_hash_function(ctx: *mut spx_ctx) {
    initialize_hash_function_rs(unsafe { &mut *ctx });
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const spx_ctx, addr: *const u32) {
    SPX_prf_addr_rs(
        unsafe { std::slice::from_raw_parts_mut(out, SPX_N) },
        unsafe { &*ctx },
        unsafe { &*(addr as *const [u32; 8]) },
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_gen_message_random(
    out: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: u64,
    _ctx: *const spx_ctx,
) {
    SPX_gen_message_random_rs(
        unsafe { std::slice::from_raw_parts_mut(out, SPX_N) },
        unsafe { std::slice::from_raw_parts(sk_prf, SPX_N) },
        unsafe { std::slice::from_raw_parts(optrand, SPX_N) },
        unsafe { std::slice::from_raw_parts(m, mlen as usize) },
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    r: *const u8,
    pk: *const u8,
    m: *const u8,
    mlen: u64,
    _ctx: *const spx_ctx,
) {
    SPX_hash_message_rs(
        unsafe { std::slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES) },
        unsafe { &mut *tree },
        unsafe { &mut *leaf_idx },
        unsafe { std::slice::from_raw_parts(r, SPX_N) },
        unsafe { std::slice::from_raw_parts(pk, SPX_PK_BYTES) },
        unsafe { std::slice::from_raw_parts(m, mlen as usize) },
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_thash(out: *mut u8, input: *const u8, inblocks: u32, ctx: *const spx_ctx, addr: *mut u32) {
    SPX_thash_rs(
        unsafe { std::slice::from_raw_parts_mut(out, SPX_N) },
        unsafe { std::slice::from_raw_parts(input, inblocks as usize * SPX_N) },
        inblocks,
        unsafe { &*ctx },
        unsafe { &mut *(addr as *mut [u32; 8]) },
    );
}
