#![allow(
    non_snake_case,
    non_upper_case_globals,
    clippy::needless_range_loop,
    clippy::identity_op,
    clippy::too_many_arguments,
    clippy::manual_memcpy
)]

use std::ptr;

// ============================================================================
// Parameters (blake-256s)
// ============================================================================
const SPX_N: usize = 32;
const SPX_FULL_HEIGHT: usize = 64;
const SPX_D: usize = 8;
const SPX_FORS_HEIGHT: usize = 14;
const SPX_FORS_TREES: usize = 22;
const SPX_WOTS_W: usize = 16;
const SPX_WOTS_LOGW: usize = 4;
const SPX_ADDR_BYTES: usize = 32;

const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW; // 64
const SPX_WOTS_LEN2: usize = 3; // precomputed for W=16, N=32
const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2; // 67
const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;

const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D; // 8

const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;

const SPX_BYTES: usize =
    SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
const SPX_PK_BYTES: usize = 2 * SPX_N;
const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
const CRYPTO_BYTES: usize = SPX_BYTES;
const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;

const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;
const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

// Since SPX_N >= 24, we use blake512 as blakeX
const SPX_BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE512_OUTPUT_BYTES;

// Address offsets (blake)
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

// Tree bits for hash_message
const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1); // 56
const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8; // 7
const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT; // 8
const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8; // 1
const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

// ============================================================================
// Context
// ============================================================================
#[repr(C)]
struct SpxCtx {
    pub_seed: [u8; SPX_N],
    sk_seed: [u8; SPX_N],
}

// ============================================================================
// Utility functions
// ============================================================================
fn addr_bytes(addr: &[u32; 8]) -> &[u8; 32] {
    unsafe { &*(addr.as_ptr() as *const [u8; 32]) }
}

fn addr_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8; 32] {
    unsafe { &mut *(addr.as_mut_ptr() as *mut [u8; 32]) }
}

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

fn bytes_to_ull(inp: &[u8], inlen: usize) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (inp[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

// ============================================================================
// Address functions
// ============================================================================
fn set_layer_addr(addr: &mut [u32; 8], layer: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_LAYER] = layer as u8;
}

fn set_tree_addr(addr: &mut [u32; 8], tree: u64) {
    let ab = addr_bytes_mut(addr);
    let mut buf = [0u8; 8];
    ull_to_bytes(&mut buf, 8, tree);
    ab[SPX_OFFSET_TREE..SPX_OFFSET_TREE + 8].copy_from_slice(&buf);
}

fn set_type(addr: &mut [u32; 8], type_val: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_TYPE] = type_val as u8;
}

fn copy_subtree_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let src = addr_bytes(inp);
    let dst = addr_bytes_mut(out);
    dst[..SPX_OFFSET_TREE + 8].copy_from_slice(&src[..SPX_OFFSET_TREE + 8]);
}

fn set_keypair_addr(addr: &mut [u32; 8], keypair: u32) {
    let ab = addr_bytes_mut(addr);
    let mut buf = [0u8; 4];
    u32_to_bytes(&mut buf, keypair);
    ab[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4].copy_from_slice(&buf);
}

fn copy_keypair_addr(out: &mut [u32; 8], inp: &[u32; 8]) {
    let src = addr_bytes(inp);
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
    let ab = addr_bytes_mut(addr);
    let mut buf = [0u8; 4];
    u32_to_bytes(&mut buf, tree_index);
    ab[SPX_OFFSET_TREE_INDEX..SPX_OFFSET_TREE_INDEX + 4].copy_from_slice(&buf);
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
    let m0 = u8to32(&block[0..]);
    let m1 = u8to32(&block[4..]);
    let m2 = u8to32(&block[8..]);
    let m3 = u8to32(&block[12..]);
    let m4 = u8to32(&block[16..]);
    let m5 = u8to32(&block[20..]);
    let m6 = u8to32(&block[24..]);
    let m7 = u8to32(&block[28..]);
    let m8 = u8to32(&block[32..]);
    let m9 = u8to32(&block[36..]);
    let m10 = u8to32(&block[40..]);
    let m11 = u8to32(&block[44..]);
    let m12 = u8to32(&block[48..]);
    let m13 = u8to32(&block[52..]);
    let m14 = u8to32(&block[56..]);
    let m15 = u8to32(&block[60..]);

    let mut v0 = state.h[0];
    let mut v1 = state.h[1];
    let mut v2 = state.h[2];
    let mut v3 = state.h[3];
    let mut v4 = state.h[4];
    let mut v5 = state.h[5];
    let mut v6 = state.h[6];
    let mut v7 = state.h[7];
    let mut v8 = state.s[0] ^ 0x243F6A88;
    let mut v9 = state.s[1] ^ 0x85A308D3;
    let mut v10 = state.s[2] ^ 0x13198A2E;
    let mut v11 = state.s[3] ^ 0x03707344;
    let mut v12: u32 = 0xA4093822;
    let mut v13: u32 = 0x299F31D0;
    let mut v14: u32 = 0x082EFA98;
    let mut v15: u32 = 0xEC4E6C89;

    if state.nullt == 0 {
        v12 ^= state.t[0];
        v13 ^= state.t[0];
        v14 ^= state.t[1];
        v15 ^= state.t[1];
    }

    macro_rules! blake256_round {
        ($m0:expr,$c0:expr,$m1:expr,$c1:expr,$m2:expr,$c2:expr,$m3:expr,$c3:expr,
         $m4:expr,$c4:expr,$m5:expr,$c5:expr,$m6:expr,$c6:expr,$m7:expr,$c7:expr,
         $m8:expr,$c8:expr,$m9:expr,$c9:expr,$m10:expr,$c10:expr,$m11:expr,$c11:expr,
         $m12:expr,$c12:expr,$m13:expr,$c13:expr,$m14:expr,$c14:expr,$m15:expr,$c15:expr) => {
            v0 = v0.wrapping_add($m0 ^ $c0); v0 = v0.wrapping_add(v4); v12 ^= v0; v12 = blake256_rot(v12, 16); v8 = v8.wrapping_add(v12); v4 ^= v8; v4 = blake256_rot(v4, 12);
            v1 = v1.wrapping_add($m2 ^ $c2); v1 = v1.wrapping_add(v5); v13 ^= v1; v13 = blake256_rot(v13, 16); v9 = v9.wrapping_add(v13); v5 ^= v9; v5 = blake256_rot(v5, 12);
            v2 = v2.wrapping_add($m4 ^ $c4); v2 = v2.wrapping_add(v6); v14 ^= v2; v14 = blake256_rot(v14, 16); v10 = v10.wrapping_add(v14); v6 ^= v10; v6 = blake256_rot(v6, 12);
            v3 = v3.wrapping_add($m6 ^ $c6); v3 = v3.wrapping_add(v7); v15 ^= v3; v15 = blake256_rot(v15, 16); v11 = v11.wrapping_add(v15); v7 ^= v11; v7 = blake256_rot(v7, 12);
            v2 = v2.wrapping_add($m5 ^ $c5); v2 = v2.wrapping_add(v6); v14 ^= v2; v14 = blake256_rot(v14, 8); v10 = v10.wrapping_add(v14); v6 ^= v10; v6 = blake256_rot(v6, 7);
            v3 = v3.wrapping_add($m7 ^ $c7); v3 = v3.wrapping_add(v7); v15 ^= v3; v15 = blake256_rot(v15, 8); v11 = v11.wrapping_add(v15); v7 ^= v11; v7 = blake256_rot(v7, 7);
            v1 = v1.wrapping_add($m3 ^ $c3); v1 = v1.wrapping_add(v5); v13 ^= v1; v13 = blake256_rot(v13, 8); v9 = v9.wrapping_add(v13); v5 ^= v9; v5 = blake256_rot(v5, 7);
            v0 = v0.wrapping_add($m1 ^ $c1); v0 = v0.wrapping_add(v4); v12 ^= v0; v12 = blake256_rot(v12, 8); v8 = v8.wrapping_add(v12); v4 ^= v8; v4 = blake256_rot(v4, 7);
            v0 = v0.wrapping_add($m8 ^ $c8); v0 = v0.wrapping_add(v5); v15 ^= v0; v15 = blake256_rot(v15, 16); v10 = v10.wrapping_add(v15); v5 ^= v10; v5 = blake256_rot(v5, 12);
            v1 = v1.wrapping_add($m10 ^ $c10); v1 = v1.wrapping_add(v6); v12 ^= v1; v12 = blake256_rot(v12, 16); v11 = v11.wrapping_add(v12); v6 ^= v11; v6 = blake256_rot(v6, 12);
            v2 = v2.wrapping_add($m12 ^ $c12); v2 = v2.wrapping_add(v7); v13 ^= v2; v13 = blake256_rot(v13, 16); v8 = v8.wrapping_add(v13); v7 ^= v8; v7 = blake256_rot(v7, 12);
            v3 = v3.wrapping_add($m14 ^ $c14); v3 = v3.wrapping_add(v4); v14 ^= v3; v14 = blake256_rot(v14, 16); v9 = v9.wrapping_add(v14); v4 ^= v9; v4 = blake256_rot(v4, 12);
            v2 = v2.wrapping_add($m13 ^ $c13); v2 = v2.wrapping_add(v7); v13 ^= v2; v13 = blake256_rot(v13, 8); v8 = v8.wrapping_add(v13); v7 ^= v8; v7 = blake256_rot(v7, 7);
            v3 = v3.wrapping_add($m15 ^ $c15); v3 = v3.wrapping_add(v4); v14 ^= v3; v14 = blake256_rot(v14, 8); v9 = v9.wrapping_add(v14); v4 ^= v9; v4 = blake256_rot(v4, 7);
            v1 = v1.wrapping_add($m11 ^ $c11); v1 = v1.wrapping_add(v6); v12 ^= v1; v12 = blake256_rot(v12, 8); v11 = v11.wrapping_add(v12); v6 ^= v11; v6 = blake256_rot(v6, 7);
            v0 = v0.wrapping_add($m9 ^ $c9); v0 = v0.wrapping_add(v5); v15 ^= v0; v15 = blake256_rot(v15, 8); v10 = v10.wrapping_add(v15); v5 ^= v10; v5 = blake256_rot(v5, 7);
        };
    }

    let c = &CST256;
    blake256_round!(m0,c[1],m1,c[0],m2,c[3],m3,c[2],m4,c[5],m5,c[4],m6,c[7],m7,c[6],m8,c[9],m9,c[8],m10,c[11],m11,c[10],m12,c[13],m13,c[12],m14,c[15],m15,c[14]);
    blake256_round!(m14,c[10],m10,c[14],m4,c[8],m8,c[4],m9,c[15],m15,c[9],m13,c[6],m6,c[13],m1,c[12],m12,c[1],m0,c[2],m2,c[0],m11,c[7],m7,c[11],m5,c[3],m3,c[5]);
    blake256_round!(m11,c[8],m8,c[11],m12,c[0],m0,c[12],m5,c[2],m2,c[5],m15,c[13],m13,c[15],m10,c[14],m14,c[10],m3,c[6],m6,c[3],m7,c[1],m1,c[7],m9,c[4],m4,c[9]);
    blake256_round!(m7,c[9],m9,c[7],m3,c[1],m1,c[3],m13,c[12],m12,c[13],m11,c[14],m14,c[11],m2,c[6],m6,c[2],m5,c[10],m10,c[5],m4,c[0],m0,c[4],m15,c[8],m8,c[15]);
    blake256_round!(m9,c[0],m0,c[9],m5,c[7],m7,c[5],m2,c[4],m4,c[2],m10,c[15],m15,c[10],m14,c[1],m1,c[14],m11,c[12],m12,c[11],m6,c[8],m8,c[6],m3,c[13],m13,c[3]);
    blake256_round!(m2,c[12],m12,c[2],m6,c[10],m10,c[6],m0,c[11],m11,c[0],m8,c[3],m3,c[8],m4,c[13],m13,c[4],m7,c[5],m5,c[7],m15,c[14],m14,c[15],m1,c[9],m9,c[1]);
    blake256_round!(m12,c[5],m5,c[12],m1,c[15],m15,c[1],m14,c[13],m13,c[14],m4,c[10],m10,c[4],m0,c[7],m7,c[0],m6,c[3],m3,c[6],m9,c[2],m2,c[9],m8,c[11],m11,c[8]);
    blake256_round!(m13,c[11],m11,c[13],m7,c[14],m14,c[7],m12,c[1],m1,c[12],m3,c[9],m9,c[3],m5,c[0],m0,c[5],m15,c[4],m4,c[15],m8,c[6],m6,c[8],m2,c[10],m10,c[2]);
    blake256_round!(m6,c[15],m15,c[6],m14,c[9],m9,c[14],m11,c[3],m3,c[11],m0,c[8],m8,c[0],m12,c[2],m2,c[12],m13,c[7],m7,c[13],m1,c[4],m4,c[1],m10,c[5],m5,c[10]);
    blake256_round!(m10,c[2],m2,c[10],m8,c[4],m4,c[8],m7,c[6],m6,c[7],m1,c[5],m5,c[1],m15,c[11],m11,c[15],m9,c[14],m14,c[9],m3,c[12],m12,c[3],m13,c[0],m0,c[13]);
    blake256_round!(m0,c[1],m1,c[0],m2,c[3],m3,c[2],m4,c[5],m5,c[4],m6,c[7],m7,c[6],m8,c[9],m9,c[8],m10,c[11],m11,c[10],m12,c[13],m13,c[12],m14,c[15],m15,c[14]);
    blake256_round!(m14,c[10],m10,c[14],m4,c[8],m8,c[4],m9,c[15],m15,c[9],m13,c[6],m6,c[13],m1,c[12],m12,c[1],m0,c[2],m2,c[0],m11,c[7],m7,c[11],m5,c[3],m3,c[5]);
    blake256_round!(m11,c[8],m8,c[11],m12,c[0],m0,c[12],m5,c[2],m2,c[5],m15,c[13],m13,c[15],m10,c[14],m14,c[10],m3,c[6],m6,c[3],m7,c[1],m1,c[7],m9,c[4],m4,c[9]);
    blake256_round!(m7,c[9],m9,c[7],m3,c[1],m1,c[3],m13,c[12],m12,c[13],m11,c[14],m14,c[11],m2,c[6],m6,c[2],m5,c[10],m10,c[5],m4,c[0],m0,c[4],m15,c[8],m8,c[15]);

    v0 ^= v8; v1 ^= v9; v2 ^= v10; v3 ^= v11;
    v4 ^= v12; v5 ^= v13; v6 ^= v14; v7 ^= v15;
    v0 ^= state.s[0]; v1 ^= state.s[1]; v2 ^= state.s[2]; v3 ^= state.s[3];
    v4 ^= state.s[0]; v5 ^= state.s[1]; v6 ^= state.s[2]; v7 ^= state.s[3];
    state.h[0] ^= v0; state.h[1] ^= v1; state.h[2] ^= v2; state.h[3] ^= v3;
    state.h[4] ^= v4; state.h[5] ^= v5; state.h[6] ^= v6; state.h[7] ^= v7;
}

fn blake256_init(s: &mut BlakeState256) {
    s.h = [0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
           0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19];
    s.t = [0, 0]; s.buflen = 0; s.nullt = 0;
    s.s = [0, 0, 0, 0]; s.buf = [0; 64];
}

fn blake256_update(s: &mut BlakeState256, data: &[u8], mut datalen: u64) {
    let mut offset = 0usize;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 64 - left;

    if left != 0 && ((datalen >> 3) & 0x3F) >= fill as u64 {
        s.buf[left..left + fill].copy_from_slice(&data[offset..offset + fill]);
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 { s.t[1] = s.t[1].wrapping_add(1); }
        let buf_copy = s.buf;
        blake256_compress(s, &buf_copy);
        offset += fill;
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 512 {
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 { s.t[1] = s.t[1].wrapping_add(1); }
        blake256_compress(s, &data[offset..]);
        offset += 64;
        datalen -= 512;
    }

    if datalen > 0 {
        let bytes = (datalen >> 3) as usize;
        s.buf[left..left + bytes].copy_from_slice(&data[offset..offset + bytes]);
        s.buflen = ((left << 3) as u64 + datalen) as i32;
    } else {
        s.buflen = 0;
    }
}

fn blake256_final(s: &mut BlakeState256, digest: &mut [u8]) {
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo = s.t[0].wrapping_add(s.buflen as u32);
    let mut hi = s.t[1];
    if lo < s.buflen as u32 { hi = hi.wrapping_add(1); }
    let mut msglen = [0u8; 8];
    u32to8(&mut msglen[0..], hi);
    u32to8(&mut msglen[4..], lo);

    if s.buflen == 440 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake256_update(s, &[oo], 8);
    } else {
        if s.buflen < 440 {
            if s.buflen == 0 { s.nullt = 1; }
            s.t[0] = s.t[0].wrapping_sub((440 - s.buflen) as u32);
            blake256_update(s, &PADDING256[..(440 - s.buflen) as usize / 8], (440 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((512 - s.buflen) as u32);
            blake256_update(s, &PADDING256[..(512 - s.buflen) as usize / 8], (512 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(440);
            blake256_update(s, &PADDING256[1..1 + 440 / 8], 440);
            s.nullt = 1;
        }
        blake256_update(s, &[zo], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(64);
    blake256_update(s, &msglen, 64);

    u32to8(&mut digest[0..], s.h[0]);
    u32to8(&mut digest[4..], s.h[1]);
    u32to8(&mut digest[8..], s.h[2]);
    u32to8(&mut digest[12..], s.h[3]);
    u32to8(&mut digest[16..], s.h[4]);
    u32to8(&mut digest[20..], s.h[5]);
    u32to8(&mut digest[24..], s.h[6]);
    u32to8(&mut digest[28..], s.h[7]);
}

fn blake256_hash(out: &mut [u8], inp: &[u8], inlen: u64) {
    let mut s = BlakeState256 { h: [0;8], s: [0;4], t: [0;2], buflen: 0, nullt: 0, buf: [0;64] };
    blake256_init(&mut s);
    blake256_update(&mut s, inp, inlen.wrapping_mul(8));
    blake256_final(&mut s, out);
}

fn blake256_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut i: usize = 0;
    let mut off = 0usize;

    while (i + 1) * SPX_BLAKE256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake256_hash(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += SPX_BLAKE256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_BLAKE256_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake256_hash(&mut outbuf, &inbuf, (inlen + 4) as u64);
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
    let m0 = u8to64(&block[0..]); let m1 = u8to64(&block[8..]);
    let m2 = u8to64(&block[16..]); let m3 = u8to64(&block[24..]);
    let m4 = u8to64(&block[32..]); let m5 = u8to64(&block[40..]);
    let m6 = u8to64(&block[48..]); let m7 = u8to64(&block[56..]);
    let m8 = u8to64(&block[64..]); let m9 = u8to64(&block[72..]);
    let m10 = u8to64(&block[80..]); let m11 = u8to64(&block[88..]);
    let m12 = u8to64(&block[96..]); let m13 = u8to64(&block[104..]);
    let m14 = u8to64(&block[112..]); let m15 = u8to64(&block[120..]);

    let mut v0 = state.h[0]; let mut v1 = state.h[1];
    let mut v2 = state.h[2]; let mut v3 = state.h[3];
    let mut v4 = state.h[4]; let mut v5 = state.h[5];
    let mut v6 = state.h[6]; let mut v7 = state.h[7];
    let mut v8 = state.s[0] ^ 0x243F6A8885A308D3u64;
    let mut v9 = state.s[1] ^ 0x13198A2E03707344u64;
    let mut v10 = state.s[2] ^ 0xA4093822299F31D0u64;
    let mut v11 = state.s[3] ^ 0x082EFA98EC4E6C89u64;
    let mut v12: u64 = 0x452821E638D01377;
    let mut v13: u64 = 0xBE5466CF34E90C6C;
    let mut v14: u64 = 0xC0AC29B7C97C50DD;
    let mut v15: u64 = 0x3F84D5B5B5470917;

    if state.nullt == 0 {
        v12 ^= state.t[0]; v13 ^= state.t[0];
        v14 ^= state.t[1]; v15 ^= state.t[1];
    }

    macro_rules! blake512_round {
        ($m0:expr,$c0:expr,$m1:expr,$c1:expr,$m2:expr,$c2:expr,$m3:expr,$c3:expr,
         $m4:expr,$c4:expr,$m5:expr,$c5:expr,$m6:expr,$c6:expr,$m7:expr,$c7:expr,
         $m8:expr,$c8:expr,$m9:expr,$c9:expr,$m10:expr,$c10:expr,$m11:expr,$c11:expr,
         $m12:expr,$c12:expr,$m13:expr,$c13:expr,$m14:expr,$c14:expr,$m15:expr,$c15:expr) => {
            v0 = v0.wrapping_add($m0 ^ $c0); v0 = v0.wrapping_add(v4); v12 ^= v0; v12 = blake512_rot(v12, 32); v8 = v8.wrapping_add(v12); v4 ^= v8; v4 = blake512_rot(v4, 25);
            v1 = v1.wrapping_add($m2 ^ $c2); v1 = v1.wrapping_add(v5); v13 ^= v1; v13 = blake512_rot(v13, 32); v9 = v9.wrapping_add(v13); v5 ^= v9; v5 = blake512_rot(v5, 25);
            v2 = v2.wrapping_add($m4 ^ $c4); v2 = v2.wrapping_add(v6); v14 ^= v2; v14 = blake512_rot(v14, 32); v10 = v10.wrapping_add(v14); v6 ^= v10; v6 = blake512_rot(v6, 25);
            v3 = v3.wrapping_add($m6 ^ $c6); v3 = v3.wrapping_add(v7); v15 ^= v3; v15 = blake512_rot(v15, 32); v11 = v11.wrapping_add(v15); v7 ^= v11; v7 = blake512_rot(v7, 25);
            v2 = v2.wrapping_add($m5 ^ $c5); v2 = v2.wrapping_add(v6); v14 ^= v2; v14 = blake512_rot(v14, 16); v10 = v10.wrapping_add(v14); v6 ^= v10; v6 = blake512_rot(v6, 11);
            v3 = v3.wrapping_add($m7 ^ $c7); v3 = v3.wrapping_add(v7); v15 ^= v3; v15 = blake512_rot(v15, 16); v11 = v11.wrapping_add(v15); v7 ^= v11; v7 = blake512_rot(v7, 11);
            v1 = v1.wrapping_add($m3 ^ $c3); v1 = v1.wrapping_add(v5); v13 ^= v1; v13 = blake512_rot(v13, 16); v9 = v9.wrapping_add(v13); v5 ^= v9; v5 = blake512_rot(v5, 11);
            v0 = v0.wrapping_add($m1 ^ $c1); v0 = v0.wrapping_add(v4); v12 ^= v0; v12 = blake512_rot(v12, 16); v8 = v8.wrapping_add(v12); v4 ^= v8; v4 = blake512_rot(v4, 11);
            v0 = v0.wrapping_add($m8 ^ $c8); v0 = v0.wrapping_add(v5); v15 ^= v0; v15 = blake512_rot(v15, 32); v10 = v10.wrapping_add(v15); v5 ^= v10; v5 = blake512_rot(v5, 25);
            v1 = v1.wrapping_add($m10 ^ $c10); v1 = v1.wrapping_add(v6); v12 ^= v1; v12 = blake512_rot(v12, 32); v11 = v11.wrapping_add(v12); v6 ^= v11; v6 = blake512_rot(v6, 25);
            v2 = v2.wrapping_add($m12 ^ $c12); v2 = v2.wrapping_add(v7); v13 ^= v2; v13 = blake512_rot(v13, 32); v8 = v8.wrapping_add(v13); v7 ^= v8; v7 = blake512_rot(v7, 25);
            v3 = v3.wrapping_add($m14 ^ $c14); v3 = v3.wrapping_add(v4); v14 ^= v3; v14 = blake512_rot(v14, 32); v9 = v9.wrapping_add(v14); v4 ^= v9; v4 = blake512_rot(v4, 25);
            v2 = v2.wrapping_add($m13 ^ $c13); v2 = v2.wrapping_add(v7); v13 ^= v2; v13 = blake512_rot(v13, 16); v8 = v8.wrapping_add(v13); v7 ^= v8; v7 = blake512_rot(v7, 11);
            v3 = v3.wrapping_add($m15 ^ $c15); v3 = v3.wrapping_add(v4); v14 ^= v3; v14 = blake512_rot(v14, 16); v9 = v9.wrapping_add(v14); v4 ^= v9; v4 = blake512_rot(v4, 11);
            v1 = v1.wrapping_add($m11 ^ $c11); v1 = v1.wrapping_add(v6); v12 ^= v1; v12 = blake512_rot(v12, 16); v11 = v11.wrapping_add(v12); v6 ^= v11; v6 = blake512_rot(v6, 11);
            v0 = v0.wrapping_add($m9 ^ $c9); v0 = v0.wrapping_add(v5); v15 ^= v0; v15 = blake512_rot(v15, 16); v10 = v10.wrapping_add(v15); v5 ^= v10; v5 = blake512_rot(v5, 11);
        };
    }

    let c = &CST512;
    blake512_round!(m0,c[1],m1,c[0],m2,c[3],m3,c[2],m4,c[5],m5,c[4],m6,c[7],m7,c[6],m8,c[9],m9,c[8],m10,c[11],m11,c[10],m12,c[13],m13,c[12],m14,c[15],m15,c[14]);
    blake512_round!(m14,c[10],m10,c[14],m4,c[8],m8,c[4],m9,c[15],m15,c[9],m13,c[6],m6,c[13],m1,c[12],m12,c[1],m0,c[2],m2,c[0],m11,c[7],m7,c[11],m5,c[3],m3,c[5]);
    blake512_round!(m11,c[8],m8,c[11],m12,c[0],m0,c[12],m5,c[2],m2,c[5],m15,c[13],m13,c[15],m10,c[14],m14,c[10],m3,c[6],m6,c[3],m7,c[1],m1,c[7],m9,c[4],m4,c[9]);
    blake512_round!(m7,c[9],m9,c[7],m3,c[1],m1,c[3],m13,c[12],m12,c[13],m11,c[14],m14,c[11],m2,c[6],m6,c[2],m5,c[10],m10,c[5],m4,c[0],m0,c[4],m15,c[8],m8,c[15]);
    blake512_round!(m9,c[0],m0,c[9],m5,c[7],m7,c[5],m2,c[4],m4,c[2],m10,c[15],m15,c[10],m14,c[1],m1,c[14],m11,c[12],m12,c[11],m6,c[8],m8,c[6],m3,c[13],m13,c[3]);
    blake512_round!(m2,c[12],m12,c[2],m6,c[10],m10,c[6],m0,c[11],m11,c[0],m8,c[3],m3,c[8],m4,c[13],m13,c[4],m7,c[5],m5,c[7],m15,c[14],m14,c[15],m1,c[9],m9,c[1]);
    blake512_round!(m12,c[5],m5,c[12],m1,c[15],m15,c[1],m14,c[13],m13,c[14],m4,c[10],m10,c[4],m0,c[7],m7,c[0],m6,c[3],m3,c[6],m9,c[2],m2,c[9],m8,c[11],m11,c[8]);
    blake512_round!(m13,c[11],m11,c[13],m7,c[14],m14,c[7],m12,c[1],m1,c[12],m3,c[9],m9,c[3],m5,c[0],m0,c[5],m15,c[4],m4,c[15],m8,c[6],m6,c[8],m2,c[10],m10,c[2]);
    blake512_round!(m6,c[15],m15,c[6],m14,c[9],m9,c[14],m11,c[3],m3,c[11],m0,c[8],m8,c[0],m12,c[2],m2,c[12],m13,c[7],m7,c[13],m1,c[4],m4,c[1],m10,c[5],m5,c[10]);
    blake512_round!(m10,c[2],m2,c[10],m8,c[4],m4,c[8],m7,c[6],m6,c[7],m1,c[5],m5,c[1],m15,c[11],m11,c[15],m9,c[14],m14,c[9],m3,c[12],m12,c[3],m13,c[0],m0,c[13]);
    blake512_round!(m0,c[1],m1,c[0],m2,c[3],m3,c[2],m4,c[5],m5,c[4],m6,c[7],m7,c[6],m8,c[9],m9,c[8],m10,c[11],m11,c[10],m12,c[13],m13,c[12],m14,c[15],m15,c[14]);
    blake512_round!(m14,c[10],m10,c[14],m4,c[8],m8,c[4],m9,c[15],m15,c[9],m13,c[6],m6,c[13],m1,c[12],m12,c[1],m0,c[2],m2,c[0],m11,c[7],m7,c[11],m5,c[3],m3,c[5]);
    blake512_round!(m11,c[8],m8,c[11],m12,c[0],m0,c[12],m5,c[2],m2,c[5],m15,c[13],m13,c[15],m10,c[14],m14,c[10],m3,c[6],m6,c[3],m7,c[1],m1,c[7],m9,c[4],m4,c[9]);
    blake512_round!(m7,c[9],m9,c[7],m3,c[1],m1,c[3],m13,c[12],m12,c[13],m11,c[14],m14,c[11],m2,c[6],m6,c[2],m5,c[10],m10,c[5],m4,c[0],m0,c[4],m15,c[8],m8,c[15]);
    blake512_round!(m9,c[0],m0,c[9],m5,c[7],m7,c[5],m2,c[4],m4,c[2],m10,c[15],m15,c[10],m14,c[1],m1,c[14],m11,c[12],m12,c[11],m6,c[8],m8,c[6],m3,c[13],m13,c[3]);
    blake512_round!(m2,c[12],m12,c[2],m6,c[10],m10,c[6],m0,c[11],m11,c[0],m8,c[3],m3,c[8],m4,c[13],m13,c[4],m7,c[5],m5,c[7],m15,c[14],m14,c[15],m1,c[9],m9,c[1]);

    v0 ^= v8; v1 ^= v9; v2 ^= v10; v3 ^= v11;
    v4 ^= v12; v5 ^= v13; v6 ^= v14; v7 ^= v15;
    v0 ^= state.s[0]; v1 ^= state.s[1]; v2 ^= state.s[2]; v3 ^= state.s[3];
    v4 ^= state.s[0]; v5 ^= state.s[1]; v6 ^= state.s[2]; v7 ^= state.s[3];
    state.h[0] ^= v0; state.h[1] ^= v1; state.h[2] ^= v2; state.h[3] ^= v3;
    state.h[4] ^= v4; state.h[5] ^= v5; state.h[6] ^= v6; state.h[7] ^= v7;
}

fn blake512_init(s: &mut BlakeState512) {
    s.h = [0x6A09E667F3BCC908, 0xBB67AE8584CAA73B, 0x3C6EF372FE94F82B, 0xA54FF53A5F1D36F1,
           0x510E527FADE682D1, 0x9B05688C2B3E6C1F, 0x1F83D9ABFB41BD6B, 0x5BE0CD19137E2179];
    s.t = [0, 0]; s.buflen = 0; s.nullt = 0;
    s.s = [0, 0, 0, 0]; s.buf = [0; 128];
}

fn blake512_update(s: &mut BlakeState512, data: &[u8], mut datalen: u64) {
    let mut offset = 0usize;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 128 - left;

    if left != 0 && ((datalen >> 3) & 0x7F) >= fill as u64 {
        s.buf[left..left + fill].copy_from_slice(&data[offset..offset + fill]);
        s.t[0] = s.t[0].wrapping_add(1024);
        let buf_copy = s.buf;
        blake512_compress(s, &buf_copy);
        offset += fill;
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 1024 {
        s.t[0] = s.t[0].wrapping_add(1024);
        blake512_compress(s, &data[offset..]);
        offset += 128;
        datalen -= 1024;
    }

    if datalen > 0 {
        let bytes = ((datalen >> 3) & 0x7F) as usize;
        s.buf[left..left + bytes].copy_from_slice(&data[offset..offset + bytes]);
        s.buflen = ((left << 3) as u64 + datalen) as i32;
    } else {
        s.buflen = 0;
    }
}

fn blake512_final(s: &mut BlakeState512, digest: &mut [u8]) {
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo = s.t[0].wrapping_add(s.buflen as u64);
    let mut hi = s.t[1];
    if lo < s.buflen as u64 { hi = hi.wrapping_add(1); }
    let mut msglen = [0u8; 16];
    u64to8(&mut msglen[0..], hi);
    u64to8(&mut msglen[8..], lo);

    if s.buflen == 888 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake512_update(s, &[oo], 8);
    } else {
        if s.buflen < 888 {
            if s.buflen == 0 { s.nullt = 1; }
            s.t[0] = s.t[0].wrapping_sub((888 - s.buflen) as u64);
            blake512_update(s, &PADDING512[..(888 - s.buflen) as usize / 8], (888 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((1024 - s.buflen) as u64);
            blake512_update(s, &PADDING512[..(1024 - s.buflen) as usize / 8], (1024 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(888);
            blake512_update(s, &PADDING512[1..1 + 888 / 8], 888);
            s.nullt = 1;
        }
        blake512_update(s, &[zo], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(128);
    blake512_update(s, &msglen, 128);

    u64to8(&mut digest[0..], s.h[0]); u64to8(&mut digest[8..], s.h[1]);
    u64to8(&mut digest[16..], s.h[2]); u64to8(&mut digest[24..], s.h[3]);
    u64to8(&mut digest[32..], s.h[4]); u64to8(&mut digest[40..], s.h[5]);
    u64to8(&mut digest[48..], s.h[6]); u64to8(&mut digest[56..], s.h[7]);
}

fn blake512_hash(out: &mut [u8], inp: &[u8], inlen: u64) {
    let mut s = BlakeState512 { h: [0;8], s: [0;4], t: [0;2], buflen: 0, nullt: 0, buf: [0;128] };
    blake512_init(&mut s);
    blake512_update(&mut s, inp, inlen.wrapping_mul(8));
    blake512_final(&mut s, out);
}

fn blake512_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut i: usize = 0;
    let mut off = 0usize;

    while (i + 1) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake512_hash(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += SPX_BLAKE512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_BLAKE512_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake512_hash(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - i * SPX_BLAKE512_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

// ============================================================================
// Hash functions (hash_blake.c) - using blake512 as blakeX since SPX_N >= 24
// ============================================================================
fn initialize_hash_function(_ctx: &mut SpxCtx) {}

fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes(addr));
    buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);

    // C code: blake256(outbuf, buf, SPX_N + SPX_ADDR_BYTES) - only hashes first 64 bytes
    blake256_hash(&mut outbuf, &buf, (SPX_N + SPX_ADDR_BYTES) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], _ctx: &SpxCtx) {
    let mut s = BlakeState512 { h: [0;8], s: [0;4], t: [0;2], buflen: 0, nullt: 0, buf: [0;128] };
    blake512_init(&mut s);
    blake512_update(&mut s, sk_prf, (SPX_N as u64) * 8);
    blake512_update(&mut s, optrand, (SPX_N as u64) * 8);
    blake512_update(&mut s, m, (m.len() as u64) * 8);
    blake512_final(&mut s, r);
}

fn hash_message(
    digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
    r: &[u8], pk: &[u8], m: &[u8], _ctx: &SpxCtx,
) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut seed = [0u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES];

    let mut s = BlakeState512 { h: [0;8], s: [0;4], t: [0;2], buflen: 0, nullt: 0, buf: [0;128] };
    blake512_init(&mut s);
    blake512_update(&mut s, r, (SPX_N as u64) * 8);
    blake512_update(&mut s, pk, (SPX_PK_BYTES as u64) * 8);
    blake512_update(&mut s, m, (m.len() as u64) * 8);
    blake512_final(&mut s, &mut seed[2 * SPX_N..]);

    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    blake512_mgf1(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut off = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[off..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    off += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[off..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}

// ============================================================================
// thash (robust, blake) - thash_blake_robust.c
// ============================================================================
fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes(addr));

    blake512_mgf1(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    blake512_hash(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + inblocks * SPX_N) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    // SPX_BLAKE512 is 1, so use 512 for inblocks > 1
    if inblocks > 1 {
        thash_512(out, inp, inblocks, ctx, addr);
        return;
    }
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes(addr));

    blake256_mgf1(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    blake256_hash(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + inblocks * SPX_N) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// ============================================================================
// WOTS (wots.c)
// ============================================================================
fn gen_chain(out: &mut [u8], inp: &[u8], start: u32, steps: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    out[..SPX_N].copy_from_slice(&inp[..SPX_N]);
    for i in start..start + steps {
        if i >= SPX_WOTS_W as u32 { break; }
        set_hash_addr(addr, i);
        let mut tmp = [0u8; SPX_N];
        tmp.copy_from_slice(&out[..SPX_N]);
        thash(out, &tmp, 1, ctx, addr);
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
    let mut csum_bytes = [0u8; 4]; // max needed

    for i in 0..SPX_WOTS_LEN1 {
        csum += (SPX_WOTS_W as u32) - 1 - msg_base_w[i];
    }

    csum <<= (8 - ((SPX_WOTS_LEN2 * SPX_WOTS_LOGW) % 8)) % 8;
    ull_to_bytes(&mut csum_bytes, csum_bytes_len, csum as u64);
    base_w(csum_base_w, SPX_WOTS_LEN2, &csum_bytes);
}

fn chain_lengths(lengths: &mut [u32], msg: &[u8]) {
    base_w(lengths, SPX_WOTS_LEN1, msg);
    let mut csum = [0u32; SPX_WOTS_LEN2];
    wots_checksum(&mut csum, lengths);
    lengths[SPX_WOTS_LEN1..SPX_WOTS_LEN1 + SPX_WOTS_LEN2].copy_from_slice(&csum);
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
// compute_root and treehash (utils.c)
// ============================================================================
fn compute_root(
    root: &mut [u8], leaf: &[u8],
    mut leaf_idx: u32, mut idx_offset: u32,
    auth_path: &[u8], tree_height: u32,
    ctx: &SpxCtx, addr: &mut [u32; 8],
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
// FORS (fors.c)
// ============================================================================
fn fors_gen_sk(sk: &mut [u8], ctx: &SpxCtx, fors_leaf_addr: &[u32; 8]) {
    prf_addr(sk, ctx, fors_leaf_addr);
}

fn fors_sk_to_leaf(leaf: &mut [u8], sk: &[u8], ctx: &SpxCtx, fors_leaf_addr: &mut [u32; 8]) {
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
    fors_sk_to_leaf(leaf, &tmp, ctx, &mut info.leaf_addrx);
}

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

fn fors_treehashx1(
    root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8], info: &mut ForsGenLeafInfo,
) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        fors_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h: u32 = 0;

        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                let aoff = h as usize * SPX_N;
                auth_path[aoff..aoff + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
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
    }
}

// ============================================================================
// WOTS leaf generation and treehash (wotsx1.c, utilsx1.c)
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
        let buffer = &mut pk_buffer[i * SPX_N..(i + 1) * SPX_N];

        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);

        prf_addr(buffer, ctx, &info.leaf_addr);

        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0u32.. {
            if k == wots_k {
                unsafe {
                    ptr::copy_nonoverlapping(
                        buffer.as_ptr(),
                        info.wots_sig.add(i * SPX_N),
                        SPX_N,
                    );
                }
            }
            if k == (SPX_WOTS_W as u32) - 1 { break; }
            set_hash_addr(&mut info.leaf_addr, k);
            let tmp: Vec<u8> = buffer.to_vec();
            thash(buffer, &tmp, 1, ctx, &mut info.leaf_addr);
        }
    }

    thash(dest, &pk_buffer, SPX_WOTS_LEN, ctx, &mut info.pk_addr);
}

fn wots_treehashx1(
    root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8], info: &mut LeafInfoX1,
) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = [0u8; 2 * SPX_N];
        wots_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;
        let mut h: u32 = 0;

        loop {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                let aoff = h as usize * SPX_N;
                auth_path[aoff..aoff + SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
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
    }
}

// ============================================================================
// FORS sign/verify (fors.c)
// ============================================================================
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
            &mut roots[i * SPX_N..],
            &mut sig[sig_off..],
            ctx, indices[i], idx_offset, SPX_FORS_HEIGHT as u32,
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
// Merkle (merkle.c)
// ============================================================================
fn merkle_sign(
    sig: &mut [u8], root: &mut [u8], ctx: &SpxCtx,
    wots_addr: &mut [u32; 8], tree_addr: &mut [u32; 8], idx_leaf: u32,
) {
    let auth_path = SPX_WOTS_BYTES;
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
        root, &mut sig[auth_path..], ctx,
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

    merkle_sign(&mut auth_path, root, ctx, &mut wots_addr, &mut top_tree_addr, !0u32);
}

// ============================================================================
// randombytes
// ============================================================================
fn randombytes(x: &mut [u8], xlen: usize) {
    use std::fs::File;
    use std::io::Read;
    let mut f = File::open("/dev/urandom").expect("Failed to open /dev/urandom");
    let mut remaining = xlen;
    let mut offset = 0;
    while remaining > 0 {
        let chunk = if remaining < 1048576 { remaining } else { 1048576 };
        match f.read(&mut x[offset..offset + chunk]) {
            Ok(0) => continue,
            Ok(n) => { offset += n; remaining -= n; }
            Err(_) => continue,
        }
    }
}

// ============================================================================
// Public API (sign.c)
// ============================================================================
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
    pk: *mut u8, sk: *mut u8, seed: *const u8,
) -> i32 {
    unsafe {
        let seed_s = std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES);
        let sk_s = std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES);
        let pk_s = std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES);

        sk_s[..CRYPTO_SEEDBYTES].copy_from_slice(seed_s);
        pk_s[..SPX_N].copy_from_slice(&sk_s[2 * SPX_N..3 * SPX_N]);

        let mut ctx = SpxCtx { pub_seed: [0; SPX_N], sk_seed: [0; SPX_N] };
        ctx.pub_seed.copy_from_slice(&pk_s[..SPX_N]);
        ctx.sk_seed.copy_from_slice(&sk_s[..SPX_N]);

        initialize_hash_function(&mut ctx);

        let mut root = [0u8; SPX_N];
        merkle_gen_root(&mut root, &ctx);
        sk_s[3 * SPX_N..4 * SPX_N].copy_from_slice(&root);
        pk_s[SPX_N..2 * SPX_N].copy_from_slice(&root);
    }
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
    sig: *mut u8, siglen: *mut usize,
    m: *const u8, mlen: usize, sk: *const u8,
) -> i32 {
    unsafe {
        let sk_s = std::slice::from_raw_parts(sk, SPX_SK_BYTES);
        let m_s = std::slice::from_raw_parts(m, mlen);
        let sig_s = std::slice::from_raw_parts_mut(sig, SPX_BYTES);

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
        randombytes(&mut optrand, SPX_N);

        let mut r_buf = [0u8; SPX_BLAKEX_OUTPUT_BYTES];
        gen_message_random(&mut r_buf, sk_prf, &optrand, m_s, &ctx);
        sig_s[..SPX_N].copy_from_slice(&r_buf[..SPX_N]);

        let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
        let mut tree: u64 = 0;
        let mut idx_leaf: u32 = 0;
        hash_message(&mut mhash, &mut tree, &mut idx_leaf, &sig_s[..SPX_N], &pk[..SPX_PK_BYTES], m_s, &ctx);

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
    sig: *const u8, siglen: usize,
    m: *const u8, mlen: usize, pk: *const u8,
) -> i32 {
    unsafe {
        let pk_s = std::slice::from_raw_parts(pk, SPX_PK_BYTES);
        let m_s = std::slice::from_raw_parts(m, mlen);

        if siglen != SPX_BYTES {
            return -1;
        }
        let sig_s = std::slice::from_raw_parts(sig, SPX_BYTES);

        let pub_root = &pk_s[SPX_N..];
        let mut ctx = SpxCtx { pub_seed: [0; SPX_N], sk_seed: [0; SPX_N] };
        ctx.pub_seed.copy_from_slice(&pk_s[..SPX_N]);

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
        hash_message(&mut mhash, &mut tree, &mut idx_leaf, &sig_s[..SPX_N], pk_s, m_s, &ctx);

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
    sm: *mut u8, smlen: *mut u64,
    m: *const u8, mlen: u64, sk: *const u8,
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
    m: *mut u8, mlen: *mut u64,
    sm: *const u8, smlen: u64, pk: *const u8,
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
