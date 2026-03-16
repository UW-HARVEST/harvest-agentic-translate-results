#![allow(
    non_snake_case,
    non_upper_case_globals,
    static_mut_refs,
    clippy::needless_range_loop,
    clippy::identity_op,
    clippy::manual_memcpy
)]

use std::ptr;
use std::slice;

// ============================================================================
// Parameters for SPHINCS+ BLAKE-192s
// ============================================================================
const SPX_N: usize = 24;
const SPX_FULL_HEIGHT: usize = 63;
const SPX_D: usize = 7;
const SPX_FORS_HEIGHT: usize = 14;
const SPX_FORS_TREES: usize = 17;
const SPX_WOTS_W: usize = 16;
const SPX_WOTS_LOGW: usize = 4;
const SPX_ADDR_BYTES: usize = 32;

const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;
const SPX_WOTS_LEN2: usize = 3; // precomputed for N=24, W=16
const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2;
const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;

const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D;

const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;

const SPX_BYTES: usize = SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
const SPX_PK_BYTES: usize = 2 * SPX_N;
const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
const CRYPTO_BYTES: usize = SPX_BYTES;
const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

// Blake offsets
const SPX_OFFSET_LAYER: usize = 3;
const SPX_OFFSET_TREE: usize = 8;
const SPX_OFFSET_TYPE: usize = 19;
const SPX_OFFSET_KP_ADDR: usize = 20;
const SPX_OFFSET_CHAIN_ADDR: usize = 27;
const SPX_OFFSET_HASH_ADDR: usize = 31;
const SPX_OFFSET_TREE_HGT: usize = 27;
const SPX_OFFSET_TREE_INDEX: usize = 28;

// Address types
const SPX_ADDR_TYPE_WOTS: u32 = 0;
const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
const SPX_ADDR_TYPE_FORSPK: u32 = 4;
const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;
const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

// For N >= 24, we use blake512 as blakeX
const SPX_BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE512_OUTPUT_BYTES;

const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

// RNG constants
const RNG_SUCCESS: i32 = 0;
const RNG_BAD_MAXLEN: i32 = -1;
const RNG_BAD_OUTBUF: i32 = -2;
const RNG_BAD_REQ_LEN: i32 = -3;

// ============================================================================
// Context
// ============================================================================
#[repr(C)]
pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
}

// ============================================================================
// Utility functions
// ============================================================================
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

fn addr_as_bytes(addr: &[u32; 8]) -> &[u8; 32] {
    unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) }
}

fn addr_as_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8; 32] {
    unsafe { &mut *(addr as *mut [u32; 8] as *mut [u8; 32]) }
}

// ============================================================================
// Address functions
// ============================================================================
fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    addr_as_bytes_mut(addr)[SPX_OFFSET_LAYER] = layer as u8;
}

fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    let bytes = addr_as_bytes_mut(addr);
    let mut buf = [0u8; 8];
    ull_to_bytes(&mut buf, 8, tree);
    bytes[SPX_OFFSET_TREE..SPX_OFFSET_TREE + 8].copy_from_slice(&buf);
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
    let mut buf = [0u8; 4];
    u32_to_bytes(&mut buf, keypair);
    bytes[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4].copy_from_slice(&buf);
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
    let mut buf = [0u8; 4];
    u32_to_bytes(&mut buf, tree_index);
    bytes[SPX_OFFSET_TREE_INDEX..SPX_OFFSET_TREE_INDEX + 4].copy_from_slice(&buf);
}

// ============================================================================
// BLAKE-256
// ============================================================================
const CST256: [u32; 16] = [
    0x243F6A88, 0x85A308D3, 0x13198A2E, 0x03707344,
    0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    0x452821E6, 0x38D01377, 0xBE5466CF, 0x34E90C6C,
    0xC0AC29B7, 0xC97C50DD, 0x3F84D5B5, 0xB5470917,
];

static PADDING256: [u8; 64] = {
    let mut p = [0u8; 64];
    p[0] = 0x80;
    p
};

fn u8to32(p: &[u8]) -> u32 {
    ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32)
}

fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8;
    p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8;
    p[3] = v as u8;
}

#[derive(Clone)]
struct BlakeState256 {
    h: [u32; 8],
    s: [u32; 4],
    t: [u32; 2],
    buflen: i32,
    nullt: i32,
    buf: [u8; 64],
}

fn blake256_rot(x: u32, n: u32) -> u32 {
    (x << (32 - n)) | (x >> n)
}

fn blake256_compress(state: &mut BlakeState256, block: &[u8]) {
    let m: [u32; 16] = [
        u8to32(&block[0..]), u8to32(&block[4..]), u8to32(&block[8..]), u8to32(&block[12..]),
        u8to32(&block[16..]), u8to32(&block[20..]), u8to32(&block[24..]), u8to32(&block[28..]),
        u8to32(&block[32..]), u8to32(&block[36..]), u8to32(&block[40..]), u8to32(&block[44..]),
        u8to32(&block[48..]), u8to32(&block[52..]), u8to32(&block[56..]), u8to32(&block[60..]),
    ];

    let mut v = [0u32; 16];
    v[0] = state.h[0]; v[1] = state.h[1]; v[2] = state.h[2]; v[3] = state.h[3];
    v[4] = state.h[4]; v[5] = state.h[5]; v[6] = state.h[6]; v[7] = state.h[7];
    v[8] = state.s[0] ^ 0x243F6A88;
    v[9] = state.s[1] ^ 0x85A308D3;
    v[10] = state.s[2] ^ 0x13198A2E;
    v[11] = state.s[3] ^ 0x03707344;
    v[12] = 0xA4093822;
    v[13] = 0x299F31D0;
    v[14] = 0x082EFA98;
    v[15] = 0xEC4E6C89;

    if state.nullt == 0 {
        v[12] ^= state.t[0];
        v[13] ^= state.t[0];
        v[14] ^= state.t[1];
        v[15] ^= state.t[1];
    }

    // The BLAKE-256 ROUND macro, inlined for each of the 14 rounds
    // Each round takes 16 message words and 16 constants in a specific permutation
    macro_rules! blake256_round {
        ($m0:expr,$c0:expr,$m1:expr,$c1:expr,$m2:expr,$c2:expr,$m3:expr,$c3:expr,
         $m4:expr,$c4:expr,$m5:expr,$c5:expr,$m6:expr,$c6:expr,$m7:expr,$c7:expr,
         $m8:expr,$c8:expr,$m9:expr,$c9:expr,$m10:expr,$c10:expr,$m11:expr,$c11:expr,
         $m12:expr,$c12:expr,$m13:expr,$c13:expr,$m14:expr,$c14:expr,$m15:expr,$c15:expr) => {
            // Column step
            v[0] = v[0].wrapping_add($m0 ^ $c0).wrapping_add(v[4]);
            v[12] ^= v[0]; v[12] = blake256_rot(v[12], 16);
            v[8] = v[8].wrapping_add(v[12]); v[4] ^= v[8]; v[4] = blake256_rot(v[4], 12);

            v[1] = v[1].wrapping_add($m2 ^ $c2).wrapping_add(v[5]);
            v[13] ^= v[1]; v[13] = blake256_rot(v[13], 16);
            v[9] = v[9].wrapping_add(v[13]); v[5] ^= v[9]; v[5] = blake256_rot(v[5], 12);

            v[2] = v[2].wrapping_add($m4 ^ $c4).wrapping_add(v[6]);
            v[14] ^= v[2]; v[14] = blake256_rot(v[14], 16);
            v[10] = v[10].wrapping_add(v[14]); v[6] ^= v[10]; v[6] = blake256_rot(v[6], 12);

            v[3] = v[3].wrapping_add($m6 ^ $c6).wrapping_add(v[7]);
            v[15] ^= v[3]; v[15] = blake256_rot(v[15], 16);
            v[11] = v[11].wrapping_add(v[15]); v[7] ^= v[11]; v[7] = blake256_rot(v[7], 12);

            v[2] = v[2].wrapping_add($m5 ^ $c5).wrapping_add(v[6]);
            v[14] ^= v[2]; v[14] = blake256_rot(v[14], 8);
            v[10] = v[10].wrapping_add(v[14]); v[6] ^= v[10]; v[6] = blake256_rot(v[6], 7);

            v[3] = v[3].wrapping_add($m7 ^ $c7).wrapping_add(v[7]);
            v[15] ^= v[3]; v[15] = blake256_rot(v[15], 8);
            v[11] = v[11].wrapping_add(v[15]); v[7] ^= v[11]; v[7] = blake256_rot(v[7], 7);

            v[1] = v[1].wrapping_add($m3 ^ $c3).wrapping_add(v[5]);
            v[13] ^= v[1]; v[13] = blake256_rot(v[13], 8);
            v[9] = v[9].wrapping_add(v[13]); v[5] ^= v[9]; v[5] = blake256_rot(v[5], 7);

            v[0] = v[0].wrapping_add($m1 ^ $c1).wrapping_add(v[4]);
            v[12] ^= v[0]; v[12] = blake256_rot(v[12], 8);
            v[8] = v[8].wrapping_add(v[12]); v[4] ^= v[8]; v[4] = blake256_rot(v[4], 7);

            // Diagonal step
            v[0] = v[0].wrapping_add($m8 ^ $c8).wrapping_add(v[5]);
            v[15] ^= v[0]; v[15] = blake256_rot(v[15], 16);
            v[10] = v[10].wrapping_add(v[15]); v[5] ^= v[10]; v[5] = blake256_rot(v[5], 12);

            v[1] = v[1].wrapping_add($m10 ^ $c10).wrapping_add(v[6]);
            v[12] ^= v[1]; v[12] = blake256_rot(v[12], 16);
            v[11] = v[11].wrapping_add(v[12]); v[6] ^= v[11]; v[6] = blake256_rot(v[6], 12);

            v[2] = v[2].wrapping_add($m12 ^ $c12).wrapping_add(v[7]);
            v[13] ^= v[2]; v[13] = blake256_rot(v[13], 16);
            v[8] = v[8].wrapping_add(v[13]); v[7] ^= v[8]; v[7] = blake256_rot(v[7], 12);

            v[3] = v[3].wrapping_add($m14 ^ $c14).wrapping_add(v[4]);
            v[14] ^= v[3]; v[14] = blake256_rot(v[14], 16);
            v[9] = v[9].wrapping_add(v[14]); v[4] ^= v[9]; v[4] = blake256_rot(v[4], 12);

            v[2] = v[2].wrapping_add($m13 ^ $c13).wrapping_add(v[7]);
            v[13] ^= v[2]; v[13] = blake256_rot(v[13], 8);
            v[8] = v[8].wrapping_add(v[13]); v[7] ^= v[8]; v[7] = blake256_rot(v[7], 7);

            v[3] = v[3].wrapping_add($m15 ^ $c15).wrapping_add(v[4]);
            v[14] ^= v[3]; v[14] = blake256_rot(v[14], 8);
            v[9] = v[9].wrapping_add(v[14]); v[4] ^= v[9]; v[4] = blake256_rot(v[4], 7);

            v[1] = v[1].wrapping_add($m11 ^ $c11).wrapping_add(v[6]);
            v[12] ^= v[1]; v[12] = blake256_rot(v[12], 8);
            v[11] = v[11].wrapping_add(v[12]); v[6] ^= v[11]; v[6] = blake256_rot(v[6], 7);

            v[0] = v[0].wrapping_add($m9 ^ $c9).wrapping_add(v[5]);
            v[15] ^= v[0]; v[15] = blake256_rot(v[15], 8);
            v[10] = v[10].wrapping_add(v[15]); v[5] ^= v[10]; v[5] = blake256_rot(v[5], 7);
        };
    }

    let c = &CST256;
    blake256_round!(m[0],c[1],m[1],c[0],m[2],c[3],m[3],c[2],m[4],c[5],m[5],c[4],m[6],c[7],m[7],c[6],m[8],c[9],m[9],c[8],m[10],c[11],m[11],c[10],m[12],c[13],m[13],c[12],m[14],c[15],m[15],c[14]);
    blake256_round!(m[14],c[10],m[10],c[14],m[4],c[8],m[8],c[4],m[9],c[15],m[15],c[9],m[13],c[6],m[6],c[13],m[1],c[12],m[12],c[1],m[0],c[2],m[2],c[0],m[11],c[7],m[7],c[11],m[5],c[3],m[3],c[5]);
    blake256_round!(m[11],c[8],m[8],c[11],m[12],c[0],m[0],c[12],m[5],c[2],m[2],c[5],m[15],c[13],m[13],c[15],m[10],c[14],m[14],c[10],m[3],c[6],m[6],c[3],m[7],c[1],m[1],c[7],m[9],c[4],m[4],c[9]);
    blake256_round!(m[7],c[9],m[9],c[7],m[3],c[1],m[1],c[3],m[13],c[12],m[12],c[13],m[11],c[14],m[14],c[11],m[2],c[6],m[6],c[2],m[5],c[10],m[10],c[5],m[4],c[0],m[0],c[4],m[15],c[8],m[8],c[15]);
    blake256_round!(m[9],c[0],m[0],c[9],m[5],c[7],m[7],c[5],m[2],c[4],m[4],c[2],m[10],c[15],m[15],c[10],m[14],c[1],m[1],c[14],m[11],c[12],m[12],c[11],m[6],c[8],m[8],c[6],m[3],c[13],m[13],c[3]);
    blake256_round!(m[2],c[12],m[12],c[2],m[6],c[10],m[10],c[6],m[0],c[11],m[11],c[0],m[8],c[3],m[3],c[8],m[4],c[13],m[13],c[4],m[7],c[5],m[5],c[7],m[15],c[14],m[14],c[15],m[1],c[9],m[9],c[1]);
    blake256_round!(m[12],c[5],m[5],c[12],m[1],c[15],m[15],c[1],m[14],c[13],m[13],c[14],m[4],c[10],m[10],c[4],m[0],c[7],m[7],c[0],m[6],c[3],m[3],c[6],m[9],c[2],m[2],c[9],m[8],c[11],m[11],c[8]);
    blake256_round!(m[13],c[11],m[11],c[13],m[7],c[14],m[14],c[7],m[12],c[1],m[1],c[12],m[3],c[9],m[9],c[3],m[5],c[0],m[0],c[5],m[15],c[4],m[4],c[15],m[8],c[6],m[6],c[8],m[2],c[10],m[10],c[2]);
    blake256_round!(m[6],c[15],m[15],c[6],m[14],c[9],m[9],c[14],m[11],c[3],m[3],c[11],m[0],c[8],m[8],c[0],m[12],c[2],m[2],c[12],m[13],c[7],m[7],c[13],m[1],c[4],m[4],c[1],m[10],c[5],m[5],c[10]);
    blake256_round!(m[10],c[2],m[2],c[10],m[8],c[4],m[4],c[8],m[7],c[6],m[6],c[7],m[1],c[5],m[5],c[1],m[15],c[11],m[11],c[15],m[9],c[14],m[14],c[9],m[3],c[12],m[12],c[3],m[13],c[0],m[0],c[13]);
    blake256_round!(m[0],c[1],m[1],c[0],m[2],c[3],m[3],c[2],m[4],c[5],m[5],c[4],m[6],c[7],m[7],c[6],m[8],c[9],m[9],c[8],m[10],c[11],m[11],c[10],m[12],c[13],m[13],c[12],m[14],c[15],m[15],c[14]);
    blake256_round!(m[14],c[10],m[10],c[14],m[4],c[8],m[8],c[4],m[9],c[15],m[15],c[9],m[13],c[6],m[6],c[13],m[1],c[12],m[12],c[1],m[0],c[2],m[2],c[0],m[11],c[7],m[7],c[11],m[5],c[3],m[3],c[5]);
    blake256_round!(m[11],c[8],m[8],c[11],m[12],c[0],m[0],c[12],m[5],c[2],m[2],c[5],m[15],c[13],m[13],c[15],m[10],c[14],m[14],c[10],m[3],c[6],m[6],c[3],m[7],c[1],m[1],c[7],m[9],c[4],m[4],c[9]);
    blake256_round!(m[7],c[9],m[9],c[7],m[3],c[1],m[1],c[3],m[13],c[12],m[12],c[13],m[11],c[14],m[14],c[11],m[2],c[6],m[6],c[2],m[5],c[10],m[10],c[5],m[4],c[0],m[0],c[4],m[15],c[8],m[8],c[15]);

    v[0] ^= v[8]; v[1] ^= v[9]; v[2] ^= v[10]; v[3] ^= v[11];
    v[4] ^= v[12]; v[5] ^= v[13]; v[6] ^= v[14]; v[7] ^= v[15];

    v[0] ^= state.s[0]; v[1] ^= state.s[1]; v[2] ^= state.s[2]; v[3] ^= state.s[3];
    v[4] ^= state.s[0]; v[5] ^= state.s[1]; v[6] ^= state.s[2]; v[7] ^= state.s[3];

    state.h[0] ^= v[0]; state.h[1] ^= v[1]; state.h[2] ^= v[2]; state.h[3] ^= v[3];
    state.h[4] ^= v[4]; state.h[5] ^= v[5]; state.h[6] ^= v[6]; state.h[7] ^= v[7];
}

fn blake256_init(state: &mut BlakeState256) {
    state.h[0] = 0x6A09E667; state.h[1] = 0xBB67AE85;
    state.h[2] = 0x3C6EF372; state.h[3] = 0xA54FF53A;
    state.h[4] = 0x510E527F; state.h[5] = 0x9B05688C;
    state.h[6] = 0x1F83D9AB; state.h[7] = 0x5BE0CD19;
    state.t = [0; 2]; state.buflen = 0; state.nullt = 0;
    state.s = [0; 4]; state.buf = [0; 64];
}

fn blake256_update(state: &mut BlakeState256, data: &[u8], mut datalen: u64) {
    let mut offset = 0usize;
    let mut left = (state.buflen >> 3) as usize;
    let fill = 64 - left;

    if left != 0 && ((datalen >> 3) & 0x3F) >= fill as u64 {
        state.buf[left..left + fill].copy_from_slice(&data[offset..offset + fill]);
        state.t[0] = state.t[0].wrapping_add(512);
        if state.t[0] == 0 { state.t[1] = state.t[1].wrapping_add(1); }
        let buf_copy = state.buf;
        blake256_compress(state, &buf_copy);
        offset += fill;
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 512 {
        state.t[0] = state.t[0].wrapping_add(512);
        if state.t[0] == 0 { state.t[1] = state.t[1].wrapping_add(1); }
        blake256_compress(state, &data[offset..]);
        offset += 64;
        datalen -= 512;
    }

    if datalen > 0 {
        let bytes = (datalen >> 3) as usize;
        state.buf[left..left + bytes].copy_from_slice(&data[offset..offset + bytes]);
        state.buflen = ((left << 3) as u64 + datalen) as i32;
    } else {
        state.buflen = 0;
    }
}

fn blake256_final(state: &mut BlakeState256, digest: &mut [u8]) {
    let mut msglen = [0u8; 8];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo = state.t[0].wrapping_add(state.buflen as u32);
    let mut hi = state.t[1];
    if lo < state.buflen as u32 { hi = hi.wrapping_add(1); }
    u32to8(&mut msglen[0..4], hi);
    u32to8(&mut msglen[4..8], lo);

    if state.buflen == 440 {
        state.t[0] = state.t[0].wrapping_sub(8);
        blake256_update(state, &[oo], 8);
    } else {
        if state.buflen < 440 {
            if state.buflen == 0 { state.nullt = 1; }
            state.t[0] = state.t[0].wrapping_sub((440 - state.buflen) as u32);
            blake256_update(state, &PADDING256, (440 - state.buflen) as u64);
        } else {
            state.t[0] = state.t[0].wrapping_sub((512 - state.buflen) as u32);
            blake256_update(state, &PADDING256, (512 - state.buflen) as u64);
            state.t[0] = state.t[0].wrapping_sub(440);
            blake256_update(state, &PADDING256[1..], 440);
            state.nullt = 1;
        }
        blake256_update(state, &[zo], 8);
        state.t[0] = state.t[0].wrapping_sub(8);
    }
    state.t[0] = state.t[0].wrapping_sub(64);
    blake256_update(state, &msglen, 64);

    u32to8(&mut digest[0..4], state.h[0]);
    u32to8(&mut digest[4..8], state.h[1]);
    u32to8(&mut digest[8..12], state.h[2]);
    u32to8(&mut digest[12..16], state.h[3]);
    u32to8(&mut digest[16..20], state.h[4]);
    u32to8(&mut digest[20..24], state.h[5]);
    u32to8(&mut digest[24..28], state.h[6]);
    u32to8(&mut digest[28..32], state.h[7]);
}

fn blake256(out: &mut [u8], data: &[u8], inlen: u64) -> i32 {
    let mut s = BlakeState256 { h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 64] };
    blake256_init(&mut s);
    blake256_update(&mut s, data, inlen.wrapping_mul(8));
    blake256_final(&mut s, out);
    0
}

fn blake256_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    let mut i: usize = 0;
    let mut off = 0usize;
    while (i + 1) * SPX_BLAKE256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake256(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += SPX_BLAKE256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_BLAKE256_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake256(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - i * SPX_BLAKE256_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

// ============================================================================
// BLAKE-512
// ============================================================================
const CST512: [u64; 16] = [
    0x243F6A8885A308D3, 0x13198A2E03707344, 0xA4093822299F31D0, 0x082EFA98EC4E6C89,
    0x452821E638D01377, 0xBE5466CF34E90C6C, 0xC0AC29B7C97C50DD, 0x3F84D5B5B5470917,
    0x9216D5D98979FB1B, 0xD1310BA698DFB5AC, 0x2FFD72DBD01ADFB7, 0xB8E1AFED6A267E96,
    0xBA7C9045F12C7F99, 0x24A19947B3916CF7, 0x0801F2E2858EFC16, 0x636920D871574E69,
];

static PADDING512: [u8; 129] = {
    let mut p = [0u8; 129];
    p[0] = 0x80;
    p
};

fn u8to64(p: &[u8]) -> u64 {
    ((u8to32(p) as u64) << 32) | (u8to32(&p[4..]) as u64)
}

fn u64to8(p: &mut [u8], v: u64) {
    u32to8(p, (v >> 32) as u32);
    u32to8(&mut p[4..], v as u32);
}

#[derive(Clone)]
struct BlakeState512 {
    h: [u64; 8],
    s: [u64; 4],
    t: [u64; 2],
    buflen: i32,
    nullt: i32,
    buf: [u8; 128],
}

fn blake512_rot(x: u64, n: u32) -> u64 {
    (x << (64 - n)) | (x >> n)
}

fn blake512_compress(state: &mut BlakeState512, block: &[u8]) {
    let m: [u64; 16] = [
        u8to64(&block[0..]), u8to64(&block[8..]), u8to64(&block[16..]), u8to64(&block[24..]),
        u8to64(&block[32..]), u8to64(&block[40..]), u8to64(&block[48..]), u8to64(&block[56..]),
        u8to64(&block[64..]), u8to64(&block[72..]), u8to64(&block[80..]), u8to64(&block[88..]),
        u8to64(&block[96..]), u8to64(&block[104..]), u8to64(&block[112..]), u8to64(&block[120..]),
    ];

    let mut v = [0u64; 16];
    v[0] = state.h[0]; v[1] = state.h[1]; v[2] = state.h[2]; v[3] = state.h[3];
    v[4] = state.h[4]; v[5] = state.h[5]; v[6] = state.h[6]; v[7] = state.h[7];
    v[8] = state.s[0] ^ 0x243F6A8885A308D3;
    v[9] = state.s[1] ^ 0x13198A2E03707344;
    v[10] = state.s[2] ^ 0xA4093822299F31D0;
    v[11] = state.s[3] ^ 0x082EFA98EC4E6C89;
    v[12] = 0x452821E638D01377;
    v[13] = 0xBE5466CF34E90C6C;
    v[14] = 0xC0AC29B7C97C50DD;
    v[15] = 0x3F84D5B5B5470917;

    if state.nullt == 0 {
        v[12] ^= state.t[0];
        v[13] ^= state.t[0];
        v[14] ^= state.t[1];
        v[15] ^= state.t[1];
    }

    macro_rules! blake512_round {
        ($m0:expr,$c0:expr,$m1:expr,$c1:expr,$m2:expr,$c2:expr,$m3:expr,$c3:expr,
         $m4:expr,$c4:expr,$m5:expr,$c5:expr,$m6:expr,$c6:expr,$m7:expr,$c7:expr,
         $m8:expr,$c8:expr,$m9:expr,$c9:expr,$m10:expr,$c10:expr,$m11:expr,$c11:expr,
         $m12:expr,$c12:expr,$m13:expr,$c13:expr,$m14:expr,$c14:expr,$m15:expr,$c15:expr) => {
            v[0] = v[0].wrapping_add($m0 ^ $c0).wrapping_add(v[4]);
            v[12] ^= v[0]; v[12] = blake512_rot(v[12], 32);
            v[8] = v[8].wrapping_add(v[12]); v[4] ^= v[8]; v[4] = blake512_rot(v[4], 25);

            v[1] = v[1].wrapping_add($m2 ^ $c2).wrapping_add(v[5]);
            v[13] ^= v[1]; v[13] = blake512_rot(v[13], 32);
            v[9] = v[9].wrapping_add(v[13]); v[5] ^= v[9]; v[5] = blake512_rot(v[5], 25);

            v[2] = v[2].wrapping_add($m4 ^ $c4).wrapping_add(v[6]);
            v[14] ^= v[2]; v[14] = blake512_rot(v[14], 32);
            v[10] = v[10].wrapping_add(v[14]); v[6] ^= v[10]; v[6] = blake512_rot(v[6], 25);

            v[3] = v[3].wrapping_add($m6 ^ $c6).wrapping_add(v[7]);
            v[15] ^= v[3]; v[15] = blake512_rot(v[15], 32);
            v[11] = v[11].wrapping_add(v[15]); v[7] ^= v[11]; v[7] = blake512_rot(v[7], 25);

            v[2] = v[2].wrapping_add($m5 ^ $c5).wrapping_add(v[6]);
            v[14] ^= v[2]; v[14] = blake512_rot(v[14], 16);
            v[10] = v[10].wrapping_add(v[14]); v[6] ^= v[10]; v[6] = blake512_rot(v[6], 11);

            v[3] = v[3].wrapping_add($m7 ^ $c7).wrapping_add(v[7]);
            v[15] ^= v[3]; v[15] = blake512_rot(v[15], 16);
            v[11] = v[11].wrapping_add(v[15]); v[7] ^= v[11]; v[7] = blake512_rot(v[7], 11);

            v[1] = v[1].wrapping_add($m3 ^ $c3).wrapping_add(v[5]);
            v[13] ^= v[1]; v[13] = blake512_rot(v[13], 16);
            v[9] = v[9].wrapping_add(v[13]); v[5] ^= v[9]; v[5] = blake512_rot(v[5], 11);

            v[0] = v[0].wrapping_add($m1 ^ $c1).wrapping_add(v[4]);
            v[12] ^= v[0]; v[12] = blake512_rot(v[12], 16);
            v[8] = v[8].wrapping_add(v[12]); v[4] ^= v[8]; v[4] = blake512_rot(v[4], 11);

            v[0] = v[0].wrapping_add($m8 ^ $c8).wrapping_add(v[5]);
            v[15] ^= v[0]; v[15] = blake512_rot(v[15], 32);
            v[10] = v[10].wrapping_add(v[15]); v[5] ^= v[10]; v[5] = blake512_rot(v[5], 25);

            v[1] = v[1].wrapping_add($m10 ^ $c10).wrapping_add(v[6]);
            v[12] ^= v[1]; v[12] = blake512_rot(v[12], 32);
            v[11] = v[11].wrapping_add(v[12]); v[6] ^= v[11]; v[6] = blake512_rot(v[6], 25);

            v[2] = v[2].wrapping_add($m12 ^ $c12).wrapping_add(v[7]);
            v[13] ^= v[2]; v[13] = blake512_rot(v[13], 32);
            v[8] = v[8].wrapping_add(v[13]); v[7] ^= v[8]; v[7] = blake512_rot(v[7], 25);

            v[3] = v[3].wrapping_add($m14 ^ $c14).wrapping_add(v[4]);
            v[14] ^= v[3]; v[14] = blake512_rot(v[14], 32);
            v[9] = v[9].wrapping_add(v[14]); v[4] ^= v[9]; v[4] = blake512_rot(v[4], 25);

            v[2] = v[2].wrapping_add($m13 ^ $c13).wrapping_add(v[7]);
            v[13] ^= v[2]; v[13] = blake512_rot(v[13], 16);
            v[8] = v[8].wrapping_add(v[13]); v[7] ^= v[8]; v[7] = blake512_rot(v[7], 11);

            v[3] = v[3].wrapping_add($m15 ^ $c15).wrapping_add(v[4]);
            v[14] ^= v[3]; v[14] = blake512_rot(v[14], 16);
            v[9] = v[9].wrapping_add(v[14]); v[4] ^= v[9]; v[4] = blake512_rot(v[4], 11);

            v[1] = v[1].wrapping_add($m11 ^ $c11).wrapping_add(v[6]);
            v[12] ^= v[1]; v[12] = blake512_rot(v[12], 16);
            v[11] = v[11].wrapping_add(v[12]); v[6] ^= v[11]; v[6] = blake512_rot(v[6], 11);

            v[0] = v[0].wrapping_add($m9 ^ $c9).wrapping_add(v[5]);
            v[15] ^= v[0]; v[15] = blake512_rot(v[15], 16);
            v[10] = v[10].wrapping_add(v[15]); v[5] ^= v[10]; v[5] = blake512_rot(v[5], 11);
        };
    }

    let c = &CST512;
    blake512_round!(m[0],c[1],m[1],c[0],m[2],c[3],m[3],c[2],m[4],c[5],m[5],c[4],m[6],c[7],m[7],c[6],m[8],c[9],m[9],c[8],m[10],c[11],m[11],c[10],m[12],c[13],m[13],c[12],m[14],c[15],m[15],c[14]);
    blake512_round!(m[14],c[10],m[10],c[14],m[4],c[8],m[8],c[4],m[9],c[15],m[15],c[9],m[13],c[6],m[6],c[13],m[1],c[12],m[12],c[1],m[0],c[2],m[2],c[0],m[11],c[7],m[7],c[11],m[5],c[3],m[3],c[5]);
    blake512_round!(m[11],c[8],m[8],c[11],m[12],c[0],m[0],c[12],m[5],c[2],m[2],c[5],m[15],c[13],m[13],c[15],m[10],c[14],m[14],c[10],m[3],c[6],m[6],c[3],m[7],c[1],m[1],c[7],m[9],c[4],m[4],c[9]);
    blake512_round!(m[7],c[9],m[9],c[7],m[3],c[1],m[1],c[3],m[13],c[12],m[12],c[13],m[11],c[14],m[14],c[11],m[2],c[6],m[6],c[2],m[5],c[10],m[10],c[5],m[4],c[0],m[0],c[4],m[15],c[8],m[8],c[15]);
    blake512_round!(m[9],c[0],m[0],c[9],m[5],c[7],m[7],c[5],m[2],c[4],m[4],c[2],m[10],c[15],m[15],c[10],m[14],c[1],m[1],c[14],m[11],c[12],m[12],c[11],m[6],c[8],m[8],c[6],m[3],c[13],m[13],c[3]);
    blake512_round!(m[2],c[12],m[12],c[2],m[6],c[10],m[10],c[6],m[0],c[11],m[11],c[0],m[8],c[3],m[3],c[8],m[4],c[13],m[13],c[4],m[7],c[5],m[5],c[7],m[15],c[14],m[14],c[15],m[1],c[9],m[9],c[1]);
    blake512_round!(m[12],c[5],m[5],c[12],m[1],c[15],m[15],c[1],m[14],c[13],m[13],c[14],m[4],c[10],m[10],c[4],m[0],c[7],m[7],c[0],m[6],c[3],m[3],c[6],m[9],c[2],m[2],c[9],m[8],c[11],m[11],c[8]);
    blake512_round!(m[13],c[11],m[11],c[13],m[7],c[14],m[14],c[7],m[12],c[1],m[1],c[12],m[3],c[9],m[9],c[3],m[5],c[0],m[0],c[5],m[15],c[4],m[4],c[15],m[8],c[6],m[6],c[8],m[2],c[10],m[10],c[2]);
    blake512_round!(m[6],c[15],m[15],c[6],m[14],c[9],m[9],c[14],m[11],c[3],m[3],c[11],m[0],c[8],m[8],c[0],m[12],c[2],m[2],c[12],m[13],c[7],m[7],c[13],m[1],c[4],m[4],c[1],m[10],c[5],m[5],c[10]);
    blake512_round!(m[10],c[2],m[2],c[10],m[8],c[4],m[4],c[8],m[7],c[6],m[6],c[7],m[1],c[5],m[5],c[1],m[15],c[11],m[11],c[15],m[9],c[14],m[14],c[9],m[3],c[12],m[12],c[3],m[13],c[0],m[0],c[13]);
    blake512_round!(m[0],c[1],m[1],c[0],m[2],c[3],m[3],c[2],m[4],c[5],m[5],c[4],m[6],c[7],m[7],c[6],m[8],c[9],m[9],c[8],m[10],c[11],m[11],c[10],m[12],c[13],m[13],c[12],m[14],c[15],m[15],c[14]);
    blake512_round!(m[14],c[10],m[10],c[14],m[4],c[8],m[8],c[4],m[9],c[15],m[15],c[9],m[13],c[6],m[6],c[13],m[1],c[12],m[12],c[1],m[0],c[2],m[2],c[0],m[11],c[7],m[7],c[11],m[5],c[3],m[3],c[5]);
    blake512_round!(m[11],c[8],m[8],c[11],m[12],c[0],m[0],c[12],m[5],c[2],m[2],c[5],m[15],c[13],m[13],c[15],m[10],c[14],m[14],c[10],m[3],c[6],m[6],c[3],m[7],c[1],m[1],c[7],m[9],c[4],m[4],c[9]);
    blake512_round!(m[7],c[9],m[9],c[7],m[3],c[1],m[1],c[3],m[13],c[12],m[12],c[13],m[11],c[14],m[14],c[11],m[2],c[6],m[6],c[2],m[5],c[10],m[10],c[5],m[4],c[0],m[0],c[4],m[15],c[8],m[8],c[15]);
    blake512_round!(m[9],c[0],m[0],c[9],m[5],c[7],m[7],c[5],m[2],c[4],m[4],c[2],m[10],c[15],m[15],c[10],m[14],c[1],m[1],c[14],m[11],c[12],m[12],c[11],m[6],c[8],m[8],c[6],m[3],c[13],m[13],c[3]);
    blake512_round!(m[2],c[12],m[12],c[2],m[6],c[10],m[10],c[6],m[0],c[11],m[11],c[0],m[8],c[3],m[3],c[8],m[4],c[13],m[13],c[4],m[7],c[5],m[5],c[7],m[15],c[14],m[14],c[15],m[1],c[9],m[9],c[1]);

    v[0] ^= v[8]; v[1] ^= v[9]; v[2] ^= v[10]; v[3] ^= v[11];
    v[4] ^= v[12]; v[5] ^= v[13]; v[6] ^= v[14]; v[7] ^= v[15];

    v[0] ^= state.s[0]; v[1] ^= state.s[1]; v[2] ^= state.s[2]; v[3] ^= state.s[3];
    v[4] ^= state.s[0]; v[5] ^= state.s[1]; v[6] ^= state.s[2]; v[7] ^= state.s[3];

    state.h[0] ^= v[0]; state.h[1] ^= v[1]; state.h[2] ^= v[2]; state.h[3] ^= v[3];
    state.h[4] ^= v[4]; state.h[5] ^= v[5]; state.h[6] ^= v[6]; state.h[7] ^= v[7];
}

fn blake512_init(state: &mut BlakeState512) {
    state.h[0] = 0x6A09E667F3BCC908; state.h[1] = 0xBB67AE8584CAA73B;
    state.h[2] = 0x3C6EF372FE94F82B; state.h[3] = 0xA54FF53A5F1D36F1;
    state.h[4] = 0x510E527FADE682D1; state.h[5] = 0x9B05688C2B3E6C1F;
    state.h[6] = 0x1F83D9ABFB41BD6B; state.h[7] = 0x5BE0CD19137E2179;
    state.t = [0; 2]; state.buflen = 0; state.nullt = 0;
    state.s = [0; 4]; state.buf = [0; 128];
}

fn blake512_update(state: &mut BlakeState512, data: &[u8], mut datalen: u64) {
    let mut offset = 0usize;
    let mut left = (state.buflen >> 3) as usize;
    let fill = 128 - left;

    if left != 0 && ((datalen >> 3) & 0x7F) >= fill as u64 {
        state.buf[left..left + fill].copy_from_slice(&data[offset..offset + fill]);
        state.t[0] = state.t[0].wrapping_add(1024);
        let buf_copy = state.buf;
        blake512_compress(state, &buf_copy);
        offset += fill;
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 1024 {
        state.t[0] = state.t[0].wrapping_add(1024);
        blake512_compress(state, &data[offset..]);
        offset += 128;
        datalen -= 1024;
    }

    if datalen > 0 {
        let bytes = ((datalen >> 3) & 0x7F) as usize;
        state.buf[left..left + bytes].copy_from_slice(&data[offset..offset + bytes]);
        state.buflen = ((left << 3) as u64 + datalen) as i32;
    } else {
        state.buflen = 0;
    }
}

fn blake512_final(state: &mut BlakeState512, digest: &mut [u8]) {
    let mut msglen = [0u8; 16];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo = state.t[0].wrapping_add(state.buflen as u64);
    let mut hi = state.t[1];
    if lo < state.buflen as u64 { hi = hi.wrapping_add(1); }
    u64to8(&mut msglen[0..8], hi);
    u64to8(&mut msglen[8..16], lo);

    if state.buflen == 888 {
        state.t[0] = state.t[0].wrapping_sub(8);
        blake512_update(state, &[oo], 8);
    } else {
        if state.buflen < 888 {
            if state.buflen == 0 { state.nullt = 1; }
            state.t[0] = state.t[0].wrapping_sub((888 - state.buflen) as u64);
            blake512_update(state, &PADDING512, (888 - state.buflen) as u64);
        } else {
            state.t[0] = state.t[0].wrapping_sub((1024 - state.buflen) as u64);
            blake512_update(state, &PADDING512, (1024 - state.buflen) as u64);
            state.t[0] = state.t[0].wrapping_sub(888);
            blake512_update(state, &PADDING512[1..], 888);
            state.nullt = 1;
        }
        blake512_update(state, &[zo], 8);
        state.t[0] = state.t[0].wrapping_sub(8);
    }
    state.t[0] = state.t[0].wrapping_sub(128);
    blake512_update(state, &msglen, 128);

    u64to8(&mut digest[0..8], state.h[0]);
    u64to8(&mut digest[8..16], state.h[1]);
    u64to8(&mut digest[16..24], state.h[2]);
    u64to8(&mut digest[24..32], state.h[3]);
    u64to8(&mut digest[32..40], state.h[4]);
    u64to8(&mut digest[40..48], state.h[5]);
    u64to8(&mut digest[48..56], state.h[6]);
    u64to8(&mut digest[56..64], state.h[7]);
}

fn blake512(out: &mut [u8], data: &[u8], inlen: u64) -> i32 {
    let mut s = BlakeState512 { h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128] };
    blake512_init(&mut s);
    blake512_update(&mut s, data, inlen.wrapping_mul(8));
    blake512_final(&mut s, out);
    0
}

fn blake512_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    let mut i: usize = 0;
    let mut off = 0usize;
    while (i + 1) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake512(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += SPX_BLAKE512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_BLAKE512_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake512(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - i * SPX_BLAKE512_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

// ============================================================================
// Hash functions (hash_blake.c) - using blake512 as blakeX since N >= 24
// ============================================================================
fn initialize_hash_function(_ctx: &mut SpxCtx) {}

fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_as_bytes(addr));
    buf[SPX_N + SPX_ADDR_BYTES..2 * SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ctx.sk_seed);
    blake256(&mut outbuf, &buf, (SPX_N + SPX_ADDR_BYTES) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    let mut s = BlakeState512 { h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128] };
    blake512_init(&mut s);
    blake512_update(&mut s, &sk_prf[..SPX_N], (SPX_N as u64) * 8);
    blake512_update(&mut s, &optrand[..SPX_N], (SPX_N as u64) * 8);
    blake512_update(&mut s, &m[..mlen as usize], mlen * 8);
    blake512_final(&mut s, r);
}

fn hash_message(
    digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
    r: &[u8], pk: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx,
) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut seed = [0u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES];

    let mut s = BlakeState512 { h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128] };
    blake512_init(&mut s);
    blake512_update(&mut s, &r[..SPX_N], (SPX_N as u64) * 8);
    blake512_update(&mut s, &pk[..SPX_PK_BYTES], (SPX_PK_BYTES as u64) * 8);
    blake512_update(&mut s, &m[..mlen as usize], mlen * 8);
    blake512_final(&mut s, &mut seed[2 * SPX_N..]);

    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    blake512_mgf1(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES);

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

// ============================================================================
// thash_blake_robust.c
// ============================================================================
fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    if inblocks > 1 {
        thash_512(out, inp, inblocks, ctx, addr);
        return;
    }
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_as_bytes(addr));
    blake256_mgf1(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    blake256(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + inblocks * SPX_N) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_as_bytes(addr));
    blake512_mgf1(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    blake512(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + inblocks * SPX_N) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// ============================================================================
// WOTS
// ============================================================================
fn gen_chain(out: &mut [u8], inp: &[u8], start: u32, steps: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
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
        output[out_idx] = ((total >> bits) & (SPX_WOTS_W as u8 - 1)) as u32;
        out_idx += 1;
    }
}

fn wots_checksum(csum_base_w: &mut [u32], msg_base_w: &[u32]) {
    let mut csum: u32 = 0;
    let csum_bytes_len = (SPX_WOTS_LEN2 * SPX_WOTS_LOGW + 7) / 8;
    let mut csum_bytes = vec![0u8; csum_bytes_len];
    for i in 0..SPX_WOTS_LEN1 {
        csum += (SPX_WOTS_W as u32) - 1 - msg_base_w[i];
    }
    csum <<= (8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8;
    ull_to_bytes(&mut csum_bytes, csum_bytes_len, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}

fn chain_lengths(lengths: &mut [u32; SPX_WOTS_LEN], msg: &[u8]) {
    base_w(&mut lengths[..SPX_WOTS_LEN1], SPX_WOTS_LEN1, msg);
    let (left, right) = lengths.split_at_mut(SPX_WOTS_LEN1);
    wots_checksum(right, left);
}

fn wots_pk_from_sig(pk: &mut [u8], sig: &[u8], msg: &[u8], ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut lengths = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut lengths, msg);
    for i in 0..SPX_WOTS_LEN {
        set_chain_addr(addr, i as u32);
        gen_chain(
            &mut pk[i * SPX_N..(i + 1) * SPX_N],
            &sig[i * SPX_N..],
            lengths[i],
            (SPX_WOTS_W as u32) - 1 - lengths[i],
            ctx, addr,
        );
    }
}

// ============================================================================
// WOTS x1 / tree hash
// ============================================================================
struct LeafInfoX1 {
    wots_sig: *mut u8,
    wots_sign_leaf: u32,
    wots_steps: *const u32,
    leaf_addr: [u32; 8],
    pk_addr: [u32; 8],
}

fn wots_gen_leafx1(dest: &mut [u8], ctx: &SpxCtx, leaf_idx: u32, info: &mut LeafInfoX1) {
    let leaf_addr = &mut info.leaf_addr;
    let pk_addr = &mut info.pk_addr;
    let mut pk_buffer = [0u8; SPX_WOTS_BYTES];

    let wots_k_mask: u32 = if leaf_idx == info.wots_sign_leaf { 0 } else { !0u32 };

    set_keypair_addr(leaf_addr, leaf_idx);
    set_keypair_addr(pk_addr, leaf_idx);

    for i in 0..SPX_WOTS_LEN {
        let wots_k = unsafe { *info.wots_steps.add(i) } | wots_k_mask;
        let buffer = &mut pk_buffer[i * SPX_N..(i + 1) * SPX_N];

        set_chain_addr(leaf_addr, i as u32);
        set_hash_addr(leaf_addr, 0);
        set_type(leaf_addr, SPX_ADDR_TYPE_WOTSPRF);
        prf_addr(buffer, ctx, leaf_addr);
        set_type(leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0u32.. {
            if k == wots_k {
                if !info.wots_sig.is_null() {
                    unsafe {
                        ptr::copy_nonoverlapping(buffer.as_ptr(), info.wots_sig.add(i * SPX_N), SPX_N);
                    }
                }
            }
            if k == (SPX_WOTS_W as u32) - 1 { break; }
            set_hash_addr(leaf_addr, k);
            let mut tmp = [0u8; SPX_N];
            tmp.copy_from_slice(buffer);
            thash(buffer, &tmp, 1, ctx, leaf_addr);
        }
    }

    thash(dest, &pk_buffer, SPX_WOTS_LEN, ctx, pk_addr);
}

fn wots_treehashx1(
    root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8], info: &mut LeafInfoX1,
) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    let mut idx = 0u32;
    loop {
        let mut current = [0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[SPX_N..], ctx, idx.wrapping_add(idx_offset), info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h = 0u32;
        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                let off = h as usize * SPX_N;
                auth_path[off..off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let soff = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[soff..soff + SPX_N]);
            let tmp = current;
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let soff = h as usize * SPX_N;
        stack[soff..soff + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
        idx += 1;
    }
}

// ============================================================================
// FORS
// ============================================================================
#[repr(C)]
struct ForsGenLeafInfo {
    leaf_addrx: [u32; 8],
}

fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, fors_leaf_addr: &[u32; 8]) {
    prf_addr(sk, ctx, fors_leaf_addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, fors_leaf_addr: &mut [u32; 8]) {
    thash(leaf, sk, 1, ctx, fors_leaf_addr);
}

fn fors_gen_leafx1(leaf: &mut [u8], ctx: &SpxCtx, addr_idx: u32, info: &mut ForsGenLeafInfo) {
    let fors_leaf_addr = &mut info.leaf_addrx;
    set_tree_index(fors_leaf_addr, addr_idx);
    set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSPRF);
    fors_gen_sk(leaf, ctx, fors_leaf_addr);
    set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSTREE);
    let mut tmp = [0u8; SPX_N];
    tmp.copy_from_slice(&leaf[..SPX_N]);
    fors_sk_to_leaf(leaf, &tmp, ctx, fors_leaf_addr);
}

fn fors_treehashx1(
    root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8], info: &mut ForsGenLeafInfo,
) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    let mut idx = 0u32;
    loop {
        let mut current = [0u8; 2 * SPX_N];
        fors_gen_leafx1(&mut current[SPX_N..], ctx, idx.wrapping_add(idx_offset), info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h = 0u32;
        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                let off = h as usize * SPX_N;
                auth_path[off..off + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            let soff = h as usize * SPX_N;
            current[..SPX_N].copy_from_slice(&stack[soff..soff + SPX_N]);
            let tmp = current;
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            h += 1;
            internal_idx >>= 1;
            internal_leaf >>= 1;
        }

        let soff = h as usize * SPX_N;
        stack[soff..soff + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
        idx += 1;
    }
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

        fors_treehashx1(
            &mut roots[i * SPX_N..], &mut sig[sig_off..], ctx,
            indices[i], idx_offset, SPX_FORS_HEIGHT as u32,
            &mut fors_tree_addr, &mut fors_info,
        );
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

        fors_sk_to_leaf(&mut leaf, &sig[sig_off..], ctx, &mut fors_tree_addr);
        sig_off += SPX_N;

        compute_root(
            &mut roots[i * SPX_N..], &leaf, indices[i], idx_offset,
            &sig[sig_off..], SPX_FORS_HEIGHT as u32, ctx, &mut fors_tree_addr,
        );
        sig_off += SPX_N * SPX_FORS_HEIGHT;
    }

    thash(pk, &roots, SPX_FORS_TREES, ctx, &mut fors_pk_addr);
}

// ============================================================================
// Utils: compute_root, treehash
// ============================================================================
fn compute_root(
    root: &mut [u8], leaf: &[u8], mut leaf_idx: u32, mut idx_offset: u32,
    auth_path: &[u8], tree_height: u32, ctx: &SpxCtx, addr: &mut [u32; 8],
) {
    let mut buffer = [0u8; 2 * SPX_N];
    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[..SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[..SPX_N]);
    }
    let mut ap_off = SPX_N;

    for i in 0..tree_height - 1 {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height(addr, i + 1);
        set_tree_index(addr, leaf_idx + idx_offset);

        if leaf_idx & 1 != 0 {
            let tmp = buffer;
            thash(&mut buffer[SPX_N..], &tmp, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
        } else {
            let tmp = buffer;
            thash(&mut buffer[..SPX_N], &tmp, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
        }
        ap_off += SPX_N;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height(addr, tree_height);
    set_tree_index(addr, leaf_idx + idx_offset);
    thash(root, &buffer, 2, ctx, addr);
}

// ============================================================================
// Merkle
// ============================================================================
fn merkle_sign(
    sig: &mut [u8], root: &mut [u8], ctx: &SpxCtx,
    wots_addr: &mut [u32; 8], tree_addr: &mut [u32; 8], idx_leaf: u32,
) {
    let auth_path_off = SPX_WOTS_BYTES;
    let mut steps = [0u32; SPX_WOTS_LEN];
    chain_lengths(&mut steps, root);

    let mut info = LeafInfoX1 {
        wots_sig: sig.as_mut_ptr(),
        wots_sign_leaf: idx_leaf,
        wots_steps: steps.as_ptr(),
        leaf_addr: [0u32; 8],
        pk_addr: [0u32; 8],
    };

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr(&mut info.pk_addr, wots_addr);

    wots_treehashx1(
        root, &mut sig[auth_path_off..], ctx,
        idx_leaf, 0, SPX_TREE_HEIGHT as u32,
        tree_addr, &mut info,
    );
}

fn merkle_gen_root(root: &mut [u8], ctx: &SpxCtx) {
    let mut auth_path = vec![0u8; SPX_TREE_HEIGHT * SPX_N + SPX_WOTS_BYTES];
    let mut top_tree_addr = [0u32; 8];
    let mut wots_addr = [0u32; 8];

    set_layer_addr(&mut top_tree_addr, (SPX_D - 1) as u32);
    set_layer_addr(&mut wots_addr, (SPX_D - 1) as u32);

    merkle_sign(
        &mut auth_path, root, ctx,
        &mut wots_addr, &mut top_tree_addr, !0u32,
    );
}

// ============================================================================
// RNG (rng.c - deterministic DRBG using AES-256-CTR)
// ============================================================================
// We use a minimal software AES-256-ECB for the DRBG, matching the OpenSSL behavior.
// For the cdylib, we link against OpenSSL via libc FFI.

extern "C" {
    fn EVP_CIPHER_CTX_new() -> *mut std::ffi::c_void;
    fn EVP_CIPHER_CTX_free(ctx: *mut std::ffi::c_void);
    fn EVP_aes_256_ecb() -> *const std::ffi::c_void;
    fn EVP_EncryptInit_ex(
        ctx: *mut std::ffi::c_void, cipher: *const std::ffi::c_void,
        engine: *const std::ffi::c_void, key: *const u8, iv: *const u8,
    ) -> i32;
    fn EVP_EncryptUpdate(
        ctx: *mut std::ffi::c_void, out: *mut u8, outl: *mut i32,
        inp: *const u8, inl: i32,
    ) -> i32;
}

#[repr(C)]
pub struct AesXofStruct {
    buffer: [u8; 16],
    buffer_pos: u64,
    length_remaining: u64,
    key: [u8; 32],
    ctr: [u8; 16],
}

#[repr(C)]
struct Aes256CtrDrbgStruct {
    key: [u8; 32],
    v: [u8; 16],
    reseed_counter: i32,
}

static mut DRBG_CTX: Aes256CtrDrbgStruct = Aes256CtrDrbgStruct {
    key: [0; 32],
    v: [0; 16],
    reseed_counter: 0,
};

unsafe fn aes256_ecb(key: *const u8, ctr: *const u8, buffer: *mut u8) {
    let ctx = EVP_CIPHER_CTX_new();
    EVP_EncryptInit_ex(ctx, EVP_aes_256_ecb(), ptr::null(), key, ptr::null());
    let mut len: i32 = 0;
    EVP_EncryptUpdate(ctx, buffer, &mut len, ctr, 16);
    EVP_CIPHER_CTX_free(ctx);
}

unsafe fn aes256_ctr_drbg_update(provided_data: *const u8, key: *mut u8, v: *mut u8) {
    let mut temp = [0u8; 48];
    for i in 0..3 {
        // increment V
        for j in (0..16).rev() {
            if *v.add(j) == 0xff {
                *v.add(j) = 0x00;
            } else {
                *v.add(j) += 1;
                break;
            }
        }
        aes256_ecb(key, v, temp.as_mut_ptr().add(16 * i));
    }
    if !provided_data.is_null() {
        for i in 0..48 {
            temp[i] ^= *provided_data.add(i);
        }
    }
    ptr::copy_nonoverlapping(temp.as_ptr(), key, 32);
    ptr::copy_nonoverlapping(temp.as_ptr().add(32), v, 16);
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander_init(
    ctx: *mut AesXofStruct, seed: *const u8, diversifier: *const u8, maxlen: u64,
) -> i32 {
    if maxlen >= 0x100000000 {
        return RNG_BAD_MAXLEN;
    }
    unsafe {
        (*ctx).length_remaining = maxlen;
        ptr::copy_nonoverlapping(seed, (*ctx).key.as_mut_ptr(), 32);
        ptr::copy_nonoverlapping(diversifier, (*ctx).ctr.as_mut_ptr(), 8);
        let mut ml = maxlen;
        (*ctx).ctr[11] = (ml % 256) as u8; ml >>= 8;
        (*ctx).ctr[10] = (ml % 256) as u8; ml >>= 8;
        (*ctx).ctr[9] = (ml % 256) as u8; ml >>= 8;
        (*ctx).ctr[8] = (ml % 256) as u8;
        (*ctx).ctr[12] = 0; (*ctx).ctr[13] = 0; (*ctx).ctr[14] = 0; (*ctx).ctr[15] = 0;
        (*ctx).buffer_pos = 16;
        (*ctx).buffer = [0u8; 16];
    }
    RNG_SUCCESS
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander(ctx: *mut AesXofStruct, x: *mut u8, mut xlen: u64) -> i32 {
    if x.is_null() {
        return RNG_BAD_OUTBUF;
    }
    unsafe {
        if xlen >= (*ctx).length_remaining {
            return RNG_BAD_REQ_LEN;
        }
        (*ctx).length_remaining -= xlen;
        let mut offset: u64 = 0;
        while xlen > 0 {
            let avail = 16 - (*ctx).buffer_pos;
            if xlen <= avail {
                ptr::copy_nonoverlapping(
                    (*ctx).buffer.as_ptr().add((*ctx).buffer_pos as usize),
                    x.add(offset as usize), xlen as usize,
                );
                (*ctx).buffer_pos += xlen;
                return RNG_SUCCESS;
            }
            ptr::copy_nonoverlapping(
                (*ctx).buffer.as_ptr().add((*ctx).buffer_pos as usize),
                x.add(offset as usize), avail as usize,
            );
            xlen -= avail;
            offset += avail;
            aes256_ecb((*ctx).key.as_ptr(), (*ctx).ctr.as_ptr(), (*ctx).buffer.as_mut_ptr());
            (*ctx).buffer_pos = 0;
            for i in (12..=15).rev() {
                if (*ctx).ctr[i] == 0xff {
                    (*ctx).ctr[i] = 0x00;
                } else {
                    (*ctx).ctr[i] += 1;
                    break;
                }
            }
        }
    }
    RNG_SUCCESS
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(entropy_input: *const u8, personalization_string: *const u8) {
    unsafe {
        let mut seed_material = [0u8; 48];
        ptr::copy_nonoverlapping(entropy_input, seed_material.as_mut_ptr(), 48);
        if !personalization_string.is_null() {
            for i in 0..48 {
                seed_material[i] ^= *personalization_string.add(i);
            }
        }
        DRBG_CTX.key.fill(0);
        DRBG_CTX.v.fill(0);
        aes256_ctr_drbg_update(
            seed_material.as_ptr(), DRBG_CTX.key.as_mut_ptr(), DRBG_CTX.v.as_mut_ptr(),
        );
        DRBG_CTX.reseed_counter = 1;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut u8, mut xlen: u64) -> i32 {
    unsafe {
        let mut block = [0u8; 16];
        let mut i: usize = 0;
        while xlen > 0 {
            for j in (0..16).rev() {
                if DRBG_CTX.v[j] == 0xff {
                    DRBG_CTX.v[j] = 0x00;
                } else {
                    DRBG_CTX.v[j] += 1;
                    break;
                }
            }
            aes256_ecb(DRBG_CTX.key.as_ptr(), DRBG_CTX.v.as_ptr(), block.as_mut_ptr());
            if xlen > 15 {
                ptr::copy_nonoverlapping(block.as_ptr(), x.add(i), 16);
                i += 16;
                xlen -= 16;
            } else {
                ptr::copy_nonoverlapping(block.as_ptr(), x.add(i), xlen as usize);
                xlen = 0;
            }
        }
        aes256_ctr_drbg_update(ptr::null(), DRBG_CTX.key.as_mut_ptr(), DRBG_CTX.v.as_mut_ptr());
        DRBG_CTX.reseed_counter += 1;
    }
    RNG_SUCCESS
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *const u8, key: *mut u8, v: *mut u8,
) {
    unsafe { aes256_ctr_drbg_update(provided_data, key, v); }
}

// ============================================================================
// Sign API (sign.c)
// ============================================================================
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> u64 { CRYPTO_SECRETKEYBYTES as u64 }

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> u64 { CRYPTO_PUBLICKEYBYTES as u64 }

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> u64 { CRYPTO_BYTES as u64 }

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> u64 { CRYPTO_SEEDBYTES as u64 }

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8, sk: *mut u8, seed: *const u8,
) -> i32 {
    unsafe {
        let sk_s = slice::from_raw_parts_mut(sk, SPX_SK_BYTES);
        let pk_s = slice::from_raw_parts_mut(pk, SPX_PK_BYTES);
        let seed_s = slice::from_raw_parts(seed, CRYPTO_SEEDBYTES);

        sk_s[..CRYPTO_SEEDBYTES].copy_from_slice(seed_s);
        pk_s[..SPX_N].copy_from_slice(&sk_s[2 * SPX_N..3 * SPX_N]);

        let mut ctx = SpxCtx { pub_seed: [0; SPX_N], sk_seed: [0; SPX_N] };
        ctx.pub_seed.copy_from_slice(&pk_s[..SPX_N]);
        ctx.sk_seed.copy_from_slice(&sk_s[..SPX_N]);

        initialize_hash_function(&mut ctx);
        merkle_gen_root(&mut sk_s[3 * SPX_N..], &ctx);
        pk_s[SPX_N..2 * SPX_N].copy_from_slice(&sk_s[3 * SPX_N..4 * SPX_N]);
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    randombytes(seed.as_mut_ptr(), CRYPTO_SEEDBYTES as u64);
    crypto_sign_seed_keypair(pk, sk, seed.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8, siglen: *mut usize, m: *const u8, mlen: usize, sk: *const u8,
) -> i32 {
    unsafe {
        let sk_s = slice::from_raw_parts(sk, SPX_SK_BYTES);
        let m_s = slice::from_raw_parts(m, mlen);
        let sig_s = slice::from_raw_parts_mut(sig, SPX_BYTES);

        let mut ctx = SpxCtx { pub_seed: [0; SPX_N], sk_seed: [0; SPX_N] };
        let sk_prf = &sk_s[SPX_N..2 * SPX_N];
        let pk = &sk_s[2 * SPX_N..];

        ctx.sk_seed.copy_from_slice(&sk_s[..SPX_N]);
        ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);

        initialize_hash_function(&mut ctx);

        let mut wots_addr = [0u32; 8];
        let mut tree_addr = [0u32; 8];
        set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
        set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);

        let mut optrand = [0u8; SPX_N];
        randombytes(optrand.as_mut_ptr(), SPX_N as u64);

        let mut sig_buf = [0u8; SPX_BLAKEX_OUTPUT_BYTES];
        gen_message_random(&mut sig_buf, sk_prf, &optrand, m_s, mlen as u64, &ctx);
        sig_s[..SPX_N].copy_from_slice(&sig_buf[..SPX_N]);

        let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
        let mut tree: u64 = 0;
        let mut idx_leaf: u32 = 0;
        hash_message(&mut mhash, &mut tree, &mut idx_leaf, &sig_s[..SPX_N], pk, m_s, mlen as u64, &ctx);

        let mut sig_off = SPX_N;

        set_tree_addr(&mut wots_addr, tree);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        let mut root = [0u8; SPX_N];
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
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8, siglen: usize, m: *const u8, mlen: usize, pk: *const u8,
) -> i32 {
    unsafe {
        let pk_s = slice::from_raw_parts(pk, SPX_PK_BYTES);
        let m_s = slice::from_raw_parts(m, mlen);

        if siglen != SPX_BYTES {
            return -1;
        }
        let sig_s = slice::from_raw_parts(sig, SPX_BYTES);

        let mut ctx = SpxCtx { pub_seed: [0; SPX_N], sk_seed: [0; SPX_N] };
        ctx.pub_seed.copy_from_slice(&pk_s[..SPX_N]);
        let pub_root = &pk_s[SPX_N..2 * SPX_N];

        initialize_hash_function(&mut ctx);

        let mut wots_addr = [0u32; 8];
        let mut tree_addr = [0u32; 8];
        let mut wots_pk_addr = [0u32; 8];
        set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
        set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
        set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

        let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
        let mut tree: u64 = 0;
        let mut idx_leaf: u32 = 0;
        hash_message(&mut mhash, &mut tree, &mut idx_leaf, &sig_s[..SPX_N], pk_s, m_s, mlen as u64, &ctx);

        let mut sig_off = SPX_N;

        set_tree_addr(&mut wots_addr, tree);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        let mut root = [0u8; SPX_N];
        fors_pk_from_sig(&mut root, &sig_s[sig_off..], &mhash, &ctx, &wots_addr);
        sig_off += SPX_FORS_BYTES;

        for i in 0..SPX_D {
            set_layer_addr(&mut tree_addr, i as u32);
            set_tree_addr(&mut tree_addr, tree);
            copy_subtree_addr(&mut wots_addr, &tree_addr);
            set_keypair_addr(&mut wots_addr, idx_leaf);
            copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

            let mut wots_pk = [0u8; SPX_WOTS_BYTES];
            wots_pk_from_sig(&mut wots_pk, &sig_s[sig_off..], &root, &ctx, &mut wots_addr);
            sig_off += SPX_WOTS_BYTES;

            let mut leaf = [0u8; SPX_N];
            thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &mut wots_pk_addr);

            compute_root(&mut root, &leaf, idx_leaf, 0, &sig_s[sig_off..], SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr);
            sig_off += SPX_TREE_HEIGHT * SPX_N;

            idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
            tree >>= SPX_TREE_HEIGHT;
        }

        if root != pub_root[..SPX_N] {
            return -1;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8, smlen: *mut u64, m: *const u8, mlen: u64, sk: *const u8,
) -> i32 {
    unsafe {
        let mut siglen: usize = 0;
        crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);
        ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
        *smlen = siglen as u64 + mlen;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut u8, mlen: *mut u64, sm: *const u8, smlen: u64, pk: *const u8,
) -> i32 {
    unsafe {
        if smlen < SPX_BYTES as u64 {
            ptr::write_bytes(m, 0, smlen as usize);
            *mlen = 0;
            return -1;
        }

        *mlen = smlen - SPX_BYTES as u64;

        if crypto_sign_verify(sm, SPX_BYTES, sm.add(SPX_BYTES), *mlen as usize, pk) != 0 {
            ptr::write_bytes(m, 0, smlen as usize);
            *mlen = 0;
            return -1;
        }

        ptr::copy(sm.add(SPX_BYTES), m, *mlen as usize);
    }
    0
}
