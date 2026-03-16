#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    unused_assignments,
    unused_mut,
    clippy::all
)]

use std::ffi::c_int;

// ============================================================
// params (params-sphincs-blake-256s.h + blake_offsets.h)
// ============================================================
const SPX_N: usize = 32;
const SPX_FULL_HEIGHT: usize = 64;
const SPX_D: usize = 8;
const SPX_FORS_HEIGHT: usize = 14;
const SPX_FORS_TREES: usize = 22;
const SPX_WOTS_W: usize = 16;
const SPX_WOTS_LOGW: usize = 4;
const SPX_ADDR_BYTES: usize = 32;

const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW; // 64
const SPX_WOTS_LEN2: usize = 3;
const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2; // 67
const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;
const SPX_WOTS_PK_BYTES: usize = SPX_WOTS_BYTES;

const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D; // 8

const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8; // 38 (actually (14*22+7)/8 = 38)
const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;
const SPX_FORS_PK_BYTES: usize = SPX_N;

const SPX_BYTES: usize = SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
const SPX_PK_BYTES: usize = 2 * SPX_N;
const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;
const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

// blake_offsets.h
const SPX_OFFSET_LAYER: usize = 3;
const SPX_OFFSET_TREE: usize = 8;
const SPX_OFFSET_TYPE: usize = 19;
const SPX_OFFSET_KP_ADDR: usize = 20;
const SPX_OFFSET_CHAIN_ADDR: usize = 27;
const SPX_OFFSET_HASH_ADDR: usize = 31;
const SPX_OFFSET_TREE_HGT: usize = 27;
const SPX_OFFSET_TREE_INDEX: usize = 28;

// api.h
const CRYPTO_SECRETKEYBYTES: usize = SPX_SK_BYTES;
const CRYPTO_PUBLICKEYBYTES: usize = SPX_PK_BYTES;
const CRYPTO_BYTES: usize = SPX_BYTES;
const CRYPTO_SEEDBYTES: usize = 3 * SPX_N;
const CRYPTO_ALGNAME: &[u8] = b"SPHINCS+";

// For hash_message: since SPX_N=32 >= 24, we use blake512 as blakeX
const SPX_BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE512_OUTPUT_BYTES;

const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1); // 56
const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8; // 7
const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT; // 8
const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8; // 1
const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

// rng.h
const RNG_SUCCESS: c_int = 0;
const RNG_BAD_MAXLEN: c_int = -1;
const RNG_BAD_OUTBUF: c_int = -2;
const RNG_BAD_REQ_LEN: c_int = -3;

// address types
const SPX_ADDR_TYPE_WOTS: u32 = 0;
const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
const SPX_ADDR_TYPE_FORSPK: u32 = 4;
const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

// ============================================================
// context.h
// ============================================================
#[repr(C)]
#[derive(Clone)]
struct SpxCtx {
    pub_seed: [u8; SPX_N],
    sk_seed: [u8; SPX_N],
}

// ============================================================
// Helper: addr as byte slice
// ============================================================
fn addr_as_bytes(addr: &[u32; 8]) -> &[u8; 32] {
    unsafe { &*(addr.as_ptr() as *const [u8; 32]) }
}

fn addr_as_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8; 32] {
    unsafe { &mut *(addr.as_mut_ptr() as *mut [u8; 32]) }
}

// ============================================================
// utils.c
// ============================================================
fn ull_to_bytes(out: &mut [u8], outlen: usize, mut val: u64) {
    let mut i = outlen as isize - 1;
    while i >= 0 {
        out[i as usize] = (val & 0xff) as u8;
        val >>= 8;
        i -= 1;
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

// ============================================================
// address.c
// ============================================================
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
// blake256.c
// ============================================================
#[derive(Clone)]
struct Blakestate256 {
    h: [u32; 8],
    s: [u32; 4],
    t: [u32; 2],
    buflen: i32,
    nullt: i32,
    buf: [u8; 64],
}

static CST256: [u32; 16] = [
    0x243F6A88, 0x85A308D3, 0x13198A2E, 0x03707344,
    0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    0x452821E6, 0x38D01377, 0xBE5466CF, 0x34E90C6C,
    0xC0AC29B7, 0xC97C50DD, 0x3F84D5B5, 0xB5470917,
];

static PADDING256: [u8; 64] = [
    0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

fn u8to32(p: &[u8]) -> u32 {
    ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32)
}

fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8;
    p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8;
    p[3] = v as u8;
}

fn blake256_rot(x: u32, n: u32) -> u32 {
    (x << (32 - n)) | (x >> n)
}

fn blake256_compress(s: &mut Blakestate256, block: &[u8]) {
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

    let mut v0 = s.h[0];
    let mut v1 = s.h[1];
    let mut v2 = s.h[2];
    let mut v3 = s.h[3];
    let mut v4 = s.h[4];
    let mut v5 = s.h[5];
    let mut v6 = s.h[6];
    let mut v7 = s.h[7];
    let mut v8 = s.s[0] ^ 0x243F6A88;
    let mut v9 = s.s[1] ^ 0x85A308D3;
    let mut v10 = s.s[2] ^ 0x13198A2E;
    let mut v11 = s.s[3] ^ 0x03707344;
    let mut v12: u32 = 0xA4093822;
    let mut v13: u32 = 0x299F31D0;
    let mut v14: u32 = 0x082EFA98;
    let mut v15: u32 = 0xEC4E6C89;

    if s.nullt == 0 {
        v12 ^= s.t[0];
        v13 ^= s.t[0];
        v14 ^= s.t[1];
        v15 ^= s.t[1];
    }

    macro_rules! blake256_round {
        ($m0:expr,$c0:expr,$m1:expr,$c1:expr,$m2:expr,$c2:expr,$m3:expr,$c3:expr,
         $m4:expr,$c4:expr,$m5:expr,$c5:expr,$m6:expr,$c6:expr,$m7:expr,$c7:expr,
         $m8:expr,$c8:expr,$m9:expr,$c9:expr,$m10:expr,$c10:expr,$m11:expr,$c11:expr,
         $m12:expr,$c12:expr,$m13:expr,$c13:expr,$m14:expr,$c14:expr,$m15:expr,$c15:expr) => {
            v0 = v0.wrapping_add($m0 ^ $c0);
            v0 = v0.wrapping_add(v4);
            v12 ^= v0;
            v12 = blake256_rot(v12, 16);
            v8 = v8.wrapping_add(v12);
            v4 ^= v8;
            v4 = blake256_rot(v4, 12);
              v1 = v1.wrapping_add($m2 ^ $c2);
              v1 = v1.wrapping_add(v5);
              v13 ^= v1;
              v13 = blake256_rot(v13, 16);
              v9 = v9.wrapping_add(v13);
              v5 ^= v9;
              v5 = blake256_rot(v5, 12);
                v2 = v2.wrapping_add($m4 ^ $c4);
                v2 = v2.wrapping_add(v6);
                v14 ^= v2;
                v14 = blake256_rot(v14, 16);
                v10 = v10.wrapping_add(v14);
                v6 ^= v10;
                v6 = blake256_rot(v6, 12);
                  v3 = v3.wrapping_add($m6 ^ $c6);
                  v3 = v3.wrapping_add(v7);
                  v15 ^= v3;
                  v15 = blake256_rot(v15, 16);
                  v11 = v11.wrapping_add(v15);
                  v7 ^= v11;
                  v7 = blake256_rot(v7, 12);
                v2 = v2.wrapping_add($m5 ^ $c5);
                v2 = v2.wrapping_add(v6);
                v14 ^= v2;
                v14 = blake256_rot(v14, 8);
                v10 = v10.wrapping_add(v14);
                v6 ^= v10;
                v6 = blake256_rot(v6, 7);
                  v3 = v3.wrapping_add($m7 ^ $c7);
                  v3 = v3.wrapping_add(v7);
                  v15 ^= v3;
                  v15 = blake256_rot(v15, 8);
                  v11 = v11.wrapping_add(v15);
                  v7 ^= v11;
                  v7 = blake256_rot(v7, 7);
              v1 = v1.wrapping_add($m3 ^ $c3);
              v1 = v1.wrapping_add(v5);
              v13 ^= v1;
              v13 = blake256_rot(v13, 8);
              v9 = v9.wrapping_add(v13);
              v5 ^= v9;
              v5 = blake256_rot(v5, 7);
            v0 = v0.wrapping_add($m1 ^ $c1);
            v0 = v0.wrapping_add(v4);
            v12 ^= v0;
            v12 = blake256_rot(v12, 8);
            v8 = v8.wrapping_add(v12);
            v4 ^= v8;
            v4 = blake256_rot(v4, 7);
            // diagonal
            v0 = v0.wrapping_add($m8 ^ $c8);
            v0 = v0.wrapping_add(v5);
            v15 ^= v0;
            v15 = blake256_rot(v15, 16);
            v10 = v10.wrapping_add(v15);
            v5 ^= v10;
            v5 = blake256_rot(v5, 12);
              v1 = v1.wrapping_add($m10 ^ $c10);
              v1 = v1.wrapping_add(v6);
              v12 ^= v1;
              v12 = blake256_rot(v12, 16);
              v11 = v11.wrapping_add(v12);
              v6 ^= v11;
              v6 = blake256_rot(v6, 12);
                v2 = v2.wrapping_add($m12 ^ $c12);
                v2 = v2.wrapping_add(v7);
                v13 ^= v2;
                v13 = blake256_rot(v13, 16);
                v8 = v8.wrapping_add(v13);
                v7 ^= v8;
                v7 = blake256_rot(v7, 12);
                  v3 = v3.wrapping_add($m14 ^ $c14);
                  v3 = v3.wrapping_add(v4);
                  v14 ^= v3;
                  v14 = blake256_rot(v14, 16);
                  v9 = v9.wrapping_add(v14);
                  v4 ^= v9;
                  v4 = blake256_rot(v4, 12);
                v2 = v2.wrapping_add($m13 ^ $c13);
                v2 = v2.wrapping_add(v7);
                v13 ^= v2;
                v13 = blake256_rot(v13, 8);
                v8 = v8.wrapping_add(v13);
                v7 ^= v8;
                v7 = blake256_rot(v7, 7);
                  v3 = v3.wrapping_add($m15 ^ $c15);
                  v3 = v3.wrapping_add(v4);
                  v14 ^= v3;
                  v14 = blake256_rot(v14, 8);
                  v9 = v9.wrapping_add(v14);
                  v4 ^= v9;
                  v4 = blake256_rot(v4, 7);
              v1 = v1.wrapping_add($m11 ^ $c11);
              v1 = v1.wrapping_add(v6);
              v12 ^= v1;
              v12 = blake256_rot(v12, 8);
              v11 = v11.wrapping_add(v12);
              v6 ^= v11;
              v6 = blake256_rot(v6, 7);
            v0 = v0.wrapping_add($m9 ^ $c9);
            v0 = v0.wrapping_add(v5);
            v15 ^= v0;
            v15 = blake256_rot(v15, 8);
            v10 = v10.wrapping_add(v15);
            v5 ^= v10;
            v5 = blake256_rot(v5, 7);
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
    v0 ^= s.s[0]; v1 ^= s.s[1]; v2 ^= s.s[2]; v3 ^= s.s[3];
    v4 ^= s.s[0]; v5 ^= s.s[1]; v6 ^= s.s[2]; v7 ^= s.s[3];
    s.h[0] ^= v0; s.h[1] ^= v1; s.h[2] ^= v2; s.h[3] ^= v3;
    s.h[4] ^= v4; s.h[5] ^= v5; s.h[6] ^= v6; s.h[7] ^= v7;
}

fn blake256_init(s: &mut Blakestate256) {
    s.h[0] = 0x6A09E667; s.h[1] = 0xBB67AE85;
    s.h[2] = 0x3C6EF372; s.h[3] = 0xA54FF53A;
    s.h[4] = 0x510E527F; s.h[5] = 0x9B05688C;
    s.h[6] = 0x1F83D9AB; s.h[7] = 0x5BE0CD19;
    s.t[0] = 0; s.t[1] = 0; s.buflen = 0; s.nullt = 0;
    s.s = [0; 4];
    s.buf = [0; 64];
}

fn blake256_update(s: &mut Blakestate256, data: &[u8], mut datalen: u64) {
    let mut offset = 0usize;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 64 - left;

    if left != 0 && ((datalen >> 3) & 0x3F) as usize >= fill {
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

fn blake256_final_fn(s: &mut Blakestate256, digest: &mut [u8]) {
    let mut msglen = [0u8; 8];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo = s.t[0].wrapping_add(s.buflen as u32);
    let mut hi = s.t[1];
    if lo < s.buflen as u32 { hi = hi.wrapping_add(1); }
    u32to8(&mut msglen[0..], hi);
    u32to8(&mut msglen[4..], lo);

    if s.buflen == 440 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake256_update(s, &[oo], 8);
    } else {
        if s.buflen < 440 {
            if s.buflen == 0 { s.nullt = 1; }
            s.t[0] = s.t[0].wrapping_sub((440 - s.buflen) as u32);
            blake256_update(s, &PADDING256[..(440 - s.buflen) as usize / 8 + if (440 - s.buflen) % 8 != 0 { 1 } else { 0 }], (440 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((512 - s.buflen) as u32);
            let pad_bits = (512 - s.buflen) as u64;
            blake256_update(s, &PADDING256[..((pad_bits >> 3) as usize).min(PADDING256.len())], pad_bits);
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

fn blake256_hash(out: &mut [u8], inp: &[u8], inlen: u64) -> c_int {
    let mut s = Blakestate256 { h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 64] };
    blake256_init(&mut s);
    blake256_update(&mut s, inp, inlen.wrapping_mul(8));
    blake256_final_fn(&mut s, out);
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

// ============================================================
// blake512.c
// ============================================================
#[derive(Clone)]
struct Blakestate512 {
    h: [u64; 8],
    s: [u64; 4],
    t: [u64; 2],
    buflen: i32,
    nullt: i32,
    buf: [u8; 128],
}

static CST512: [u64; 16] = [
    0x243F6A8885A308D3, 0x13198A2E03707344, 0xA4093822299F31D0, 0x082EFA98EC4E6C89,
    0x452821E638D01377, 0xBE5466CF34E90C6C, 0xC0AC29B7C97C50DD, 0x3F84D5B5B5470917,
    0x9216D5D98979FB1B, 0xD1310BA698DFB5AC, 0x2FFD72DBD01ADFB7, 0xB8E1AFED6A267E96,
    0xBA7C9045F12C7F99, 0x24A19947B3916CF7, 0x0801F2E2858EFC16, 0x636920D871574E69,
];

static PADDING512: [u8; 129] = [
    0x80,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
];

fn u8to64(p: &[u8]) -> u64 {
    ((u8to32(p) as u64) << 32) | (u8to32(&p[4..]) as u64)
}

fn u64to8(p: &mut [u8], v: u64) {
    u32to8(p, (v >> 32) as u32);
    u32to8(&mut p[4..], v as u32);
}

fn blake512_rot(x: u64, n: u32) -> u64 {
    (x << (64 - n)) | (x >> n)
}

fn blake512_compress(s: &mut Blakestate512, block: &[u8]) {
    let m0 = u8to64(&block[0..]);
    let m1 = u8to64(&block[8..]);
    let m2 = u8to64(&block[16..]);
    let m3 = u8to64(&block[24..]);
    let m4 = u8to64(&block[32..]);
    let m5 = u8to64(&block[40..]);
    let m6 = u8to64(&block[48..]);
    let m7 = u8to64(&block[56..]);
    let m8 = u8to64(&block[64..]);
    let m9 = u8to64(&block[72..]);
    let m10 = u8to64(&block[80..]);
    let m11 = u8to64(&block[88..]);
    let m12 = u8to64(&block[96..]);
    let m13 = u8to64(&block[104..]);
    let m14 = u8to64(&block[112..]);
    let m15 = u8to64(&block[120..]);

    let mut v0 = s.h[0];
    let mut v1 = s.h[1];
    let mut v2 = s.h[2];
    let mut v3 = s.h[3];
    let mut v4 = s.h[4];
    let mut v5 = s.h[5];
    let mut v6 = s.h[6];
    let mut v7 = s.h[7];
    let mut v8 = s.s[0] ^ 0x243F6A8885A308D3u64;
    let mut v9 = s.s[1] ^ 0x13198A2E03707344u64;
    let mut v10 = s.s[2] ^ 0xA4093822299F31D0u64;
    let mut v11 = s.s[3] ^ 0x082EFA98EC4E6C89u64;
    let mut v12: u64 = 0x452821E638D01377;
    let mut v13: u64 = 0xBE5466CF34E90C6C;
    let mut v14: u64 = 0xC0AC29B7C97C50DD;
    let mut v15: u64 = 0x3F84D5B5B5470917;

    if s.nullt == 0 {
        v12 ^= s.t[0];
        v13 ^= s.t[0];
        v14 ^= s.t[1];
        v15 ^= s.t[1];
    }

    macro_rules! blake512_round {
        ($m0:expr,$c0:expr,$m1:expr,$c1:expr,$m2:expr,$c2:expr,$m3:expr,$c3:expr,
         $m4:expr,$c4:expr,$m5:expr,$c5:expr,$m6:expr,$c6:expr,$m7:expr,$c7:expr,
         $m8:expr,$c8:expr,$m9:expr,$c9:expr,$m10:expr,$c10:expr,$m11:expr,$c11:expr,
         $m12:expr,$c12:expr,$m13:expr,$c13:expr,$m14:expr,$c14:expr,$m15:expr,$c15:expr) => {
            v0 = v0.wrapping_add($m0 ^ $c0);
            v0 = v0.wrapping_add(v4);
            v12 ^= v0;
            v12 = blake512_rot(v12, 32);
            v8 = v8.wrapping_add(v12);
            v4 ^= v8;
            v4 = blake512_rot(v4, 25);
              v1 = v1.wrapping_add($m2 ^ $c2);
              v1 = v1.wrapping_add(v5);
              v13 ^= v1;
              v13 = blake512_rot(v13, 32);
              v9 = v9.wrapping_add(v13);
              v5 ^= v9;
              v5 = blake512_rot(v5, 25);
                v2 = v2.wrapping_add($m4 ^ $c4);
                v2 = v2.wrapping_add(v6);
                v14 ^= v2;
                v14 = blake512_rot(v14, 32);
                v10 = v10.wrapping_add(v14);
                v6 ^= v10;
                v6 = blake512_rot(v6, 25);
                  v3 = v3.wrapping_add($m6 ^ $c6);
                  v3 = v3.wrapping_add(v7);
                  v15 ^= v3;
                  v15 = blake512_rot(v15, 32);
                  v11 = v11.wrapping_add(v15);
                  v7 ^= v11;
                  v7 = blake512_rot(v7, 25);
                v2 = v2.wrapping_add($m5 ^ $c5);
                v2 = v2.wrapping_add(v6);
                v14 ^= v2;
                v14 = blake512_rot(v14, 16);
                v10 = v10.wrapping_add(v14);
                v6 ^= v10;
                v6 = blake512_rot(v6, 11);
                  v3 = v3.wrapping_add($m7 ^ $c7);
                  v3 = v3.wrapping_add(v7);
                  v15 ^= v3;
                  v15 = blake512_rot(v15, 16);
                  v11 = v11.wrapping_add(v15);
                  v7 ^= v11;
                  v7 = blake512_rot(v7, 11);
              v1 = v1.wrapping_add($m3 ^ $c3);
              v1 = v1.wrapping_add(v5);
              v13 ^= v1;
              v13 = blake512_rot(v13, 16);
              v9 = v9.wrapping_add(v13);
              v5 ^= v9;
              v5 = blake512_rot(v5, 11);
            v0 = v0.wrapping_add($m1 ^ $c1);
            v0 = v0.wrapping_add(v4);
            v12 ^= v0;
            v12 = blake512_rot(v12, 16);
            v8 = v8.wrapping_add(v12);
            v4 ^= v8;
            v4 = blake512_rot(v4, 11);
            // diagonal
            v0 = v0.wrapping_add($m8 ^ $c8);
            v0 = v0.wrapping_add(v5);
            v15 ^= v0;
            v15 = blake512_rot(v15, 32);
            v10 = v10.wrapping_add(v15);
            v5 ^= v10;
            v5 = blake512_rot(v5, 25);
              v1 = v1.wrapping_add($m10 ^ $c10);
              v1 = v1.wrapping_add(v6);
              v12 ^= v1;
              v12 = blake512_rot(v12, 32);
              v11 = v11.wrapping_add(v12);
              v6 ^= v11;
              v6 = blake512_rot(v6, 25);
                v2 = v2.wrapping_add($m12 ^ $c12);
                v2 = v2.wrapping_add(v7);
                v13 ^= v2;
                v13 = blake512_rot(v13, 32);
                v8 = v8.wrapping_add(v13);
                v7 ^= v8;
                v7 = blake512_rot(v7, 25);
                  v3 = v3.wrapping_add($m14 ^ $c14);
                  v3 = v3.wrapping_add(v4);
                  v14 ^= v3;
                  v14 = blake512_rot(v14, 32);
                  v9 = v9.wrapping_add(v14);
                  v4 ^= v9;
                  v4 = blake512_rot(v4, 25);
                v2 = v2.wrapping_add($m13 ^ $c13);
                v2 = v2.wrapping_add(v7);
                v13 ^= v2;
                v13 = blake512_rot(v13, 16);
                v8 = v8.wrapping_add(v13);
                v7 ^= v8;
                v7 = blake512_rot(v7, 11);
                  v3 = v3.wrapping_add($m15 ^ $c15);
                  v3 = v3.wrapping_add(v4);
                  v14 ^= v3;
                  v14 = blake512_rot(v14, 16);
                  v9 = v9.wrapping_add(v14);
                  v4 ^= v9;
                  v4 = blake512_rot(v4, 11);
              v1 = v1.wrapping_add($m11 ^ $c11);
              v1 = v1.wrapping_add(v6);
              v12 ^= v1;
              v12 = blake512_rot(v12, 16);
              v11 = v11.wrapping_add(v12);
              v6 ^= v11;
              v6 = blake512_rot(v6, 11);
            v0 = v0.wrapping_add($m9 ^ $c9);
            v0 = v0.wrapping_add(v5);
            v15 ^= v0;
            v15 = blake512_rot(v15, 16);
            v10 = v10.wrapping_add(v15);
            v5 ^= v10;
            v5 = blake512_rot(v5, 11);
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
    v0 ^= s.s[0]; v1 ^= s.s[1]; v2 ^= s.s[2]; v3 ^= s.s[3];
    v4 ^= s.s[0]; v5 ^= s.s[1]; v6 ^= s.s[2]; v7 ^= s.s[3];
    s.h[0] ^= v0; s.h[1] ^= v1; s.h[2] ^= v2; s.h[3] ^= v3;
    s.h[4] ^= v4; s.h[5] ^= v5; s.h[6] ^= v6; s.h[7] ^= v7;
}

fn blake512_init(s: &mut Blakestate512) {
    s.h[0] = 0x6A09E667F3BCC908; s.h[1] = 0xBB67AE8584CAA73B;
    s.h[2] = 0x3C6EF372FE94F82B; s.h[3] = 0xA54FF53A5F1D36F1;
    s.h[4] = 0x510E527FADE682D1; s.h[5] = 0x9B05688C2B3E6C1F;
    s.h[6] = 0x1F83D9ABFB41BD6B; s.h[7] = 0x5BE0CD19137E2179;
    s.t = [0; 2]; s.buflen = 0; s.nullt = 0;
    s.s = [0; 4]; s.buf = [0; 128];
}

fn blake512_update(s: &mut Blakestate512, data: &[u8], mut datalen: u64) {
    let mut offset = 0usize;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 128 - left;

    if left != 0 && ((datalen >> 3) & 0x7F) as usize >= fill {
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

fn blake512_final_fn(s: &mut Blakestate512, digest: &mut [u8]) {
    let mut msglen = [0u8; 16];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo = s.t[0].wrapping_add(s.buflen as u64);
    let mut hi = s.t[1];
    if lo < s.buflen as u64 { hi = hi.wrapping_add(1); }
    u64to8(&mut msglen[0..], hi);
    u64to8(&mut msglen[8..], lo);

    if s.buflen == 888 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake512_update(s, &[oo], 8);
    } else {
        if s.buflen < 888 {
            if s.buflen == 0 { s.nullt = 1; }
            s.t[0] = s.t[0].wrapping_sub((888 - s.buflen) as u64);
            blake512_update(s, &PADDING512[..(888 - s.buflen) as usize / 8 + if (888 - s.buflen) % 8 != 0 { 1 } else { 0 }], (888 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((1024 - s.buflen) as u64);
            let pad_bits = (1024 - s.buflen) as u64;
            blake512_update(s, &PADDING512[..((pad_bits >> 3) as usize).min(PADDING512.len())], pad_bits);
            s.t[0] = s.t[0].wrapping_sub(888);
            blake512_update(s, &PADDING512[1..1 + 888 / 8], 888);
            s.nullt = 1;
        }
        blake512_update(s, &[zo], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(128);
    blake512_update(s, &msglen, 128);

    u64to8(&mut digest[0..], s.h[0]);
    u64to8(&mut digest[8..], s.h[1]);
    u64to8(&mut digest[16..], s.h[2]);
    u64to8(&mut digest[24..], s.h[3]);
    u64to8(&mut digest[32..], s.h[4]);
    u64to8(&mut digest[40..], s.h[5]);
    u64to8(&mut digest[48..], s.h[6]);
    u64to8(&mut digest[56..], s.h[7]);
}

fn blake512_hash(out: &mut [u8], inp: &[u8], inlen: u64) -> c_int {
    let mut s = Blakestate512 { h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128] };
    blake512_init(&mut s);
    blake512_update(&mut s, inp, inlen.wrapping_mul(8));
    blake512_final_fn(&mut s, out);
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

// ============================================================
// thash_blake_simple.c (SPX_BLAKE512=1)
// ============================================================
fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let buflen = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buflen];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_as_bytes(addr));
    buf[SPX_N + SPX_ADDR_BYTES..buflen].copy_from_slice(&inp[..inblocks * SPX_N]);
    blake512_hash(&mut outbuf, &buf, buflen as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    // SPX_BLAKE512 = 1: use blake512 for inblocks > 1
    if inblocks > 1 {
        thash_512(out, inp, inblocks, ctx, addr);
        return;
    }
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let buflen = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buflen];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_as_bytes(addr));
    buf[SPX_N + SPX_ADDR_BYTES..buflen].copy_from_slice(&inp[..inblocks * SPX_N]);
    blake256_hash(&mut outbuf, &buf, buflen as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// ============================================================
// hash_blake.c  (SPX_N >= 24 => blakeX = blake512)
// ============================================================
fn initialize_hash_function(_ctx: &mut SpxCtx) {}

fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_as_bytes(addr));
    buf[SPX_N + SPX_ADDR_BYTES..2 * SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ctx.sk_seed);
    blake256_hash(&mut outbuf, &buf, (SPX_N + SPX_ADDR_BYTES) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    let mut s = Blakestate512 { h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128] };
    blake512_init(&mut s);
    blake512_update(&mut s, &sk_prf[..SPX_N], (SPX_N as u64) * 8);
    blake512_update(&mut s, &optrand[..SPX_N], (SPX_N as u64) * 8);
    blake512_update(&mut s, m, mlen * 8);
    blake512_final_fn(&mut s, r);
}

fn hash_message(
    digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
    r: &[u8], pk: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx,
) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut seed = [0u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES];

    let mut s = Blakestate512 { h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128] };
    blake512_init(&mut s);
    blake512_update(&mut s, &r[..SPX_N], (SPX_N as u64) * 8);
    blake512_update(&mut s, &pk[..SPX_PK_BYTES], (SPX_PK_BYTES as u64) * 8);
    blake512_update(&mut s, m, mlen * 8);
    blake512_final_fn(&mut s, &mut seed[2 * SPX_N..]);

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

// ============================================================
// wots.c
// ============================================================
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
    let mut inp_idx = 0usize;
    let mut out_idx = 0usize;
    let mut total: u8 = 0;
    let mut bits: i32 = 0;
    for _ in 0..out_len {
        if bits == 0 {
            total = input[inp_idx];
            inp_idx += 1;
            bits += 8;
        }
        bits -= SPX_WOTS_LOGW as i32;
        output[out_idx] = ((total >> bits) as u32) & (SPX_WOTS_W as u32 - 1);
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

fn chain_lengths(lengths: &mut [u32; SPX_WOTS_LEN], msg: &[u8]) {
    base_w(&mut lengths[..SPX_WOTS_LEN1], SPX_WOTS_LEN1, msg);
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
            ctx, addr,
        );
    }
}

// ============================================================
// wotsx1.c
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

    let mut pk_buffer = vec![0u8; SPX_WOTS_BYTES];
    for i in 0..SPX_WOTS_LEN {
        let wots_k = info.wots_steps[i] | wots_k_mask;
        set_chain_addr(&mut info.leaf_addr, i as u32);
        set_hash_addr(&mut info.leaf_addr, 0);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTSPRF);
        prf_addr(&mut pk_buffer[i * SPX_N..], ctx, &info.leaf_addr);
        set_type(&mut info.leaf_addr, SPX_ADDR_TYPE_WOTS);

        for k in 0u32.. {
            if k == wots_k {
                info.wots_sig[i * SPX_N..(i + 1) * SPX_N]
                    .copy_from_slice(&pk_buffer[i * SPX_N..(i + 1) * SPX_N]);
            }
            if k == (SPX_WOTS_W as u32) - 1 { break; }
            set_hash_addr(&mut info.leaf_addr, k);
            let mut tmp = [0u8; SPX_N];
            tmp.copy_from_slice(&pk_buffer[i * SPX_N..(i + 1) * SPX_N]);
            thash(&mut pk_buffer[i * SPX_N..], &tmp, 1, ctx, &mut info.leaf_addr);
        }
    }
    thash(dest, &pk_buffer, SPX_WOTS_LEN, ctx, &mut info.pk_addr);
}

// ============================================================
// fors.c
// ============================================================
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

fn fors_sign(sig: &mut [u8], pk: &mut [u8], m: &[u8], ctx: &SpxCtx, fors_addr: &[u32; 8]) {
    let mut indices = [0u32; SPX_FORS_TREES];
    let mut roots = vec![0u8; SPX_FORS_TREES * SPX_N];
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
    let mut roots = vec![0u8; SPX_FORS_TREES * SPX_N];
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

// ============================================================
// utils.c: compute_root, treehash
// ============================================================
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
            let tmp: [u8; 2 * SPX_N] = buffer;
            thash(&mut buffer[SPX_N..], &tmp, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
        } else {
            let tmp: [u8; 2 * SPX_N] = buffer;
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

// Note: treehash from utils.c is not used in this build (utilsx1 versions are used instead)

// ============================================================
// utilsx1.c: wots_treehashx1, fors_treehashx1
// ============================================================
fn wots_treehashx1(
    root: &mut [u8], auth_path: &mut [u8], ctx: &SpxCtx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    tree_addr: &mut [u32; 8], info: &mut LeafInfoX1,
) {
    let mut stack = vec![0u8; tree_height as usize * SPX_N];
    let max_idx = (1u32 << tree_height) - 1;

    for idx in 0u32.. {
        let mut current = vec![0u8; 2 * SPX_N];
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
                auth_path[h as usize * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            current[..SPX_N].copy_from_slice(&stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]);
            let tmp: Vec<u8> = current[..2 * SPX_N].to_vec();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }
        // Save to stack - need to figure out h
        let mut h = 0u32;
        { // recalculate h
            let mut ii = idx;
            let mut il = leaf_idx;
            loop {
                if h == tree_height { break; }
                if (ii ^ il) == 0x01 { }
                if (ii & 1) == 0 && idx < max_idx { break; }
                ii >>= 1; il >>= 1; h += 1;
            }
        }
        stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]
            .copy_from_slice(&current[SPX_N..2 * SPX_N]);
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
        let mut current = vec![0u8; 2 * SPX_N];
        fors_gen_leafx1(&mut current[SPX_N..], ctx, idx + idx_offset, info);

        let mut internal_idx_offset = idx_offset;
        let mut internal_idx = idx;
        let mut internal_leaf = leaf_idx;

        for h in 0u32.. {
            if h == tree_height {
                root[..SPX_N].copy_from_slice(&current[SPX_N..2 * SPX_N]);
                return;
            }
            if (internal_idx ^ internal_leaf) == 0x01 {
                auth_path[h as usize * SPX_N..(h as usize + 1) * SPX_N]
                    .copy_from_slice(&current[SPX_N..2 * SPX_N]);
            }
            if (internal_idx & 1) == 0 && idx < max_idx {
                break;
            }
            internal_idx_offset >>= 1;
            set_tree_height(tree_addr, h + 1);
            set_tree_index(tree_addr, internal_idx / 2 + internal_idx_offset);

            current[..SPX_N].copy_from_slice(&stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]);
            let tmp: Vec<u8> = current[..2 * SPX_N].to_vec();
            thash(&mut current[SPX_N..], &tmp, 2, ctx, tree_addr);

            internal_idx >>= 1;
            internal_leaf >>= 1;
        }
        let mut h = 0u32;
        {
            let mut ii = idx;
            let mut il = leaf_idx;
            loop {
                if h == tree_height { break; }
                if (ii & 1) == 0 && idx < max_idx { break; }
                ii >>= 1; il >>= 1; h += 1;
            }
        }
        stack[h as usize * SPX_N..(h as usize + 1) * SPX_N]
            .copy_from_slice(&current[SPX_N..2 * SPX_N]);
    }
}

// ============================================================
// merkle.c
// ============================================================
fn merkle_sign(
    sig: &mut [u8], root: &mut [u8], ctx: &SpxCtx,
    wots_addr: &mut [u32; 8], tree_addr: &mut [u32; 8], idx_leaf: u32,
) {
    let auth_path_off = SPX_WOTS_BYTES;
    let mut info = LeafInfoX1 {
        wots_sig: vec![0u8; SPX_WOTS_BYTES],
        wots_sign_leaf: idx_leaf,
        wots_steps: [0u32; SPX_WOTS_LEN],
        leaf_addr: [0u32; 8],
        pk_addr: [0u32; 8],
    };
    chain_lengths(&mut info.wots_steps, root);
    info.wots_sig = sig[..SPX_WOTS_BYTES].to_vec();

    set_type(tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut info.pk_addr, SPX_ADDR_TYPE_WOTSPK);
    copy_subtree_addr(&mut info.leaf_addr, wots_addr);
    copy_subtree_addr(&mut info.pk_addr, wots_addr);

    let mut auth_buf = vec![0u8; SPX_TREE_HEIGHT * SPX_N];
    wots_treehashx1(root, &mut auth_buf, ctx, idx_leaf, 0, SPX_TREE_HEIGHT as u32, tree_addr, &mut info);

    // Copy wots_sig back
    sig[..SPX_WOTS_BYTES].copy_from_slice(&info.wots_sig);
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
// rng.c (AES256-CTR-DRBG using OpenSSL-compatible pure Rust)
// ============================================================
// We implement AES-256-ECB in pure Rust for the DRBG
// This is a minimal AES-256 implementation for KAT generation

struct Aes256Key {
    round_keys: [[u8; 16]; 15],
}

static SBOX: [u8; 256] = [
    0x63,0x7c,0x77,0x7b,0xf2,0x6b,0x6f,0xc5,0x30,0x01,0x67,0x2b,0xfe,0xd7,0xab,0x76,
    0xca,0x82,0xc9,0x7d,0xfa,0x59,0x47,0xf0,0xad,0xd4,0xa2,0xaf,0x9c,0xa4,0x72,0xc0,
    0xb7,0xfd,0x93,0x26,0x36,0x3f,0xf7,0xcc,0x34,0xa5,0xe5,0xf1,0x71,0xd8,0x31,0x15,
    0x04,0xc7,0x23,0xc3,0x18,0x96,0x05,0x9a,0x07,0x12,0x80,0xe2,0xeb,0x27,0xb2,0x75,
    0x09,0x83,0x2c,0x1a,0x1b,0x6e,0x5a,0xa0,0x52,0x3b,0xd6,0xb3,0x29,0xe3,0x2f,0x84,
    0x53,0xd1,0x00,0xed,0x20,0xfc,0xb1,0x5b,0x6a,0xcb,0xbe,0x39,0x4a,0x4c,0x58,0xcf,
    0xd0,0xef,0xaa,0xfb,0x43,0x4d,0x33,0x85,0x45,0xf9,0x02,0x7f,0x50,0x3c,0x9f,0xa8,
    0x51,0xa3,0x40,0x8f,0x92,0x9d,0x38,0xf5,0xbc,0xb6,0xda,0x21,0x10,0xff,0xf3,0xd2,
    0xcd,0x0c,0x13,0xec,0x5f,0x97,0x44,0x17,0xc4,0xa7,0x7e,0x3d,0x64,0x5d,0x19,0x73,
    0x60,0x81,0x4f,0xdc,0x22,0x2a,0x90,0x88,0x46,0xee,0xb8,0x14,0xde,0x5e,0x0b,0xdb,
    0xe0,0x32,0x3a,0x0a,0x49,0x06,0x24,0x5c,0xc2,0xd3,0xac,0x62,0x91,0x95,0xe4,0x79,
    0xe7,0xc8,0x37,0x6d,0x8d,0xd5,0x4e,0xa9,0x6c,0x56,0xf4,0xea,0x65,0x7a,0xae,0x08,
    0xba,0x78,0x25,0x2e,0x1c,0xa6,0xb4,0xc6,0xe8,0xdd,0x74,0x1f,0x4b,0xbd,0x8b,0x8a,
    0x70,0x3e,0xb5,0x66,0x48,0x03,0xf6,0x0e,0x61,0x35,0x57,0xb9,0x86,0xc1,0x1d,0x9e,
    0xe1,0xf8,0x98,0x11,0x69,0xd9,0x8e,0x94,0x9b,0x1e,0x87,0xe9,0xce,0x55,0x28,0xdf,
    0x8c,0xa1,0x89,0x0d,0xbf,0xe6,0x42,0x68,0x41,0x99,0x2d,0x0f,0xb0,0x54,0xbb,0x16,
];

static RCON: [u8; 10] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36];

fn sub_word(w: u32) -> u32 {
    let b0 = SBOX[(w >> 24) as usize] as u32;
    let b1 = SBOX[((w >> 16) & 0xff) as usize] as u32;
    let b2 = SBOX[((w >> 8) & 0xff) as usize] as u32;
    let b3 = SBOX[(w & 0xff) as usize] as u32;
    (b0 << 24) | (b1 << 16) | (b2 << 8) | b3
}

fn rot_word(w: u32) -> u32 {
    (w << 8) | (w >> 24)
}

fn aes256_key_expansion(key: &[u8; 32]) -> Aes256Key {
    let mut rk = [[0u8; 16]; 15];
    let mut w = [0u32; 60];
    for i in 0..8 {
        w[i] = ((key[4*i] as u32) << 24) | ((key[4*i+1] as u32) << 16)
             | ((key[4*i+2] as u32) << 8) | (key[4*i+3] as u32);
    }
    for i in 8..60 {
        let mut temp = w[i - 1];
        if i % 8 == 0 {
            temp = sub_word(rot_word(temp)) ^ ((RCON[i / 8 - 1] as u32) << 24);
        } else if i % 8 == 4 {
            temp = sub_word(temp);
        }
        w[i] = w[i - 8] ^ temp;
    }
    for r in 0..15 {
        for j in 0..4 {
            rk[r][4*j] = (w[4*r+j] >> 24) as u8;
            rk[r][4*j+1] = (w[4*r+j] >> 16) as u8;
            rk[r][4*j+2] = (w[4*r+j] >> 8) as u8;
            rk[r][4*j+3] = w[4*r+j] as u8;
        }
    }
    Aes256Key { round_keys: rk }
}

fn xtime(a: u8) -> u8 {
    let r = (a as u16) << 1;
    (r ^ (if a & 0x80 != 0 { 0x1b } else { 0 })) as u8
}

fn gmul(mut a: u8, mut b: u8) -> u8 {
    let mut p: u8 = 0;
    for _ in 0..8 {
        if b & 1 != 0 { p ^= a; }
        let hi = a & 0x80;
        a <<= 1;
        if hi != 0 { a ^= 0x1b; }
        b >>= 1;
    }
    p
}

fn aes256_ecb_encrypt(aes_key: &Aes256Key, input: &[u8; 16], output: &mut [u8; 16]) {
    let mut state = *input;
    // AddRoundKey 0
    for i in 0..16 { state[i] ^= aes_key.round_keys[0][i]; }
    for round in 1..14 {
        // SubBytes
        for i in 0..16 { state[i] = SBOX[state[i] as usize]; }
        // ShiftRows
        let tmp = state[1]; state[1] = state[5]; state[5] = state[9]; state[9] = state[13]; state[13] = tmp;
        let tmp = state[2]; state[2] = state[10]; state[10] = tmp;
        let tmp = state[6]; state[6] = state[14]; state[14] = tmp;
        let tmp = state[3]; state[3] = state[15]; state[15] = state[11]; state[11] = state[7]; state[7] = tmp;
        // MixColumns
        for c in 0..4 {
            let s0 = state[4*c]; let s1 = state[4*c+1]; let s2 = state[4*c+2]; let s3 = state[4*c+3];
            state[4*c]   = gmul(2,s0) ^ gmul(3,s1) ^ s2 ^ s3;
            state[4*c+1] = s0 ^ gmul(2,s1) ^ gmul(3,s2) ^ s3;
            state[4*c+2] = s0 ^ s1 ^ gmul(2,s2) ^ gmul(3,s3);
            state[4*c+3] = gmul(3,s0) ^ s1 ^ s2 ^ gmul(2,s3);
        }
        // AddRoundKey
        for i in 0..16 { state[i] ^= aes_key.round_keys[round][i]; }
    }
    // Final round (no MixColumns)
    for i in 0..16 { state[i] = SBOX[state[i] as usize]; }
    let tmp = state[1]; state[1] = state[5]; state[5] = state[9]; state[9] = state[13]; state[13] = tmp;
    let tmp = state[2]; state[2] = state[10]; state[10] = tmp;
    let tmp = state[6]; state[6] = state[14]; state[14] = tmp;
    let tmp = state[3]; state[3] = state[15]; state[15] = state[11]; state[11] = state[7]; state[7] = tmp;
    for i in 0..16 { state[i] ^= aes_key.round_keys[14][i]; }
    *output = state;
}

fn aes256_ecb(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    let aes_key = aes256_key_expansion(key);
    aes256_ecb_encrypt(&aes_key, ctr, buffer);
}

struct AesXofStruct {
    buffer: [u8; 16],
    buffer_pos: usize,
    length_remaining: u64,
    key: [u8; 32],
    ctr: [u8; 16],
}

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

fn aes256_ctr_drbg_update(provided_data: Option<&[u8; 48]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
    let mut temp = [0u8; 48];
    for i in 0..3 {
        // increment V
        for j in (0..16).rev() {
            if v[j] == 0xff {
                v[j] = 0x00;
            } else {
                v[j] += 1;
                break;
            }
        }
        let mut block = [0u8; 16];
        let mut ctr_arr = [0u8; 16];
        ctr_arr.copy_from_slice(v);
        aes256_ecb(key, &ctr_arr, &mut block);
        temp[16 * i..16 * i + 16].copy_from_slice(&block);
    }
    if let Some(pd) = provided_data {
        for i in 0..48 {
            temp[i] ^= pd[i];
        }
    }
    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..48]);
}

fn seedexpander_init(ctx: &mut AesXofStruct, seed: &[u8], diversifier: &[u8], maxlen: u64) -> c_int {
    if maxlen >= 0x100000000 {
        return RNG_BAD_MAXLEN;
    }
    ctx.length_remaining = maxlen;
    ctx.key.copy_from_slice(&seed[..32]);
    ctx.ctr[..8].copy_from_slice(&diversifier[..8]);
    ctx.ctr[11] = (maxlen % 256) as u8;
    let mut ml = maxlen >> 8;
    ctx.ctr[10] = (ml % 256) as u8;
    ml >>= 8;
    ctx.ctr[9] = (ml % 256) as u8;
    ml >>= 8;
    ctx.ctr[8] = (ml % 256) as u8;
    ctx.ctr[12..16].fill(0);
    ctx.buffer_pos = 16;
    ctx.buffer.fill(0);
    RNG_SUCCESS
}

fn seedexpander(ctx: &mut AesXofStruct, x: &mut [u8], mut xlen: usize) -> c_int {
    if x.is_empty() && xlen > 0 {
        return RNG_BAD_OUTBUF;
    }
    if xlen as u64 >= ctx.length_remaining {
        return RNG_BAD_REQ_LEN;
    }
    ctx.length_remaining -= xlen as u64;
    let mut offset = 0usize;
    while xlen > 0 {
        if xlen <= 16 - ctx.buffer_pos {
            x[offset..offset + xlen].copy_from_slice(&ctx.buffer[ctx.buffer_pos..ctx.buffer_pos + xlen]);
            ctx.buffer_pos += xlen;
            return RNG_SUCCESS;
        }
        let take = 16 - ctx.buffer_pos;
        x[offset..offset + take].copy_from_slice(&ctx.buffer[ctx.buffer_pos..16]);
        xlen -= take;
        offset += take;

        let mut ctr_arr = [0u8; 16];
        ctr_arr.copy_from_slice(&ctx.ctr);
        aes256_ecb(&ctx.key, &ctr_arr, &mut ctx.buffer);
        ctx.buffer_pos = 0;

        for i in (12..=15).rev() {
            if ctx.ctr[i] == 0xff {
                ctx.ctr[i] = 0x00;
            } else {
                ctx.ctr[i] += 1;
                break;
            }
        }
    }
    RNG_SUCCESS
}

fn randombytes_init(entropy_input: &[u8; 48], personalization_string: Option<&[u8; 48]>) {
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(entropy_input);
    if let Some(ps) = personalization_string {
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }
    unsafe {
        DRBG_CTX.key.fill(0);
        DRBG_CTX.v.fill(0);
        let mut key_copy = DRBG_CTX.key;
        let mut v_copy = DRBG_CTX.v;
        let sm: [u8; 48] = seed_material;
        aes256_ctr_drbg_update(Some(&sm), &mut key_copy, &mut v_copy);
        DRBG_CTX.key = key_copy;
        DRBG_CTX.v = v_copy;
        DRBG_CTX.reseed_counter = 1;
    }
}

fn rng_randombytes(x: &mut [u8], mut xlen: u64) -> c_int {
    let mut block = [0u8; 16];
    let mut i = 0usize;
    unsafe {
        while xlen > 0 {
            for j in (0..16).rev() {
                if DRBG_CTX.v[j] == 0xff {
                    DRBG_CTX.v[j] = 0x00;
                } else {
                    DRBG_CTX.v[j] += 1;
                    break;
                }
            }
            let mut ctr_arr = [0u8; 16];
            ctr_arr.copy_from_slice(&DRBG_CTX.v);
            aes256_ecb(&DRBG_CTX.key, &ctr_arr, &mut block);
            if xlen > 15 {
                x[i..i + 16].copy_from_slice(&block);
                i += 16;
                xlen -= 16;
            } else {
                x[i..i + xlen as usize].copy_from_slice(&block[..xlen as usize]);
                xlen = 0;
            }
        }
        let mut key_copy = DRBG_CTX.key;
        let mut v_copy = DRBG_CTX.v;
        aes256_ctr_drbg_update(None, &mut key_copy, &mut v_copy);
        DRBG_CTX.key = key_copy;
        DRBG_CTX.v = v_copy;
        DRBG_CTX.reseed_counter += 1;
    }
    RNG_SUCCESS
}

// randombytes wrapper used by sign.c (uses the DRBG)
fn randombytes_fn(x: &mut [u8], xlen: u64) {
    rng_randombytes(x, xlen);
}

// ============================================================
// sign.c
// ============================================================
fn crypto_sign_secretkeybytes_inner() -> u64 { CRYPTO_SECRETKEYBYTES as u64 }
fn crypto_sign_publickeybytes_inner() -> u64 { CRYPTO_PUBLICKEYBYTES as u64 }
fn crypto_sign_bytes_inner() -> u64 { CRYPTO_BYTES as u64 }
fn crypto_sign_seedbytes_inner() -> u64 { CRYPTO_SEEDBYTES as u64 }

fn crypto_sign_seed_keypair_inner(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) -> c_int {
    let mut ctx = SpxCtx { pub_seed: [0; SPX_N], sk_seed: [0; SPX_N] };
    sk[..CRYPTO_SEEDBYTES].copy_from_slice(&seed[..CRYPTO_SEEDBYTES]);
    pk[..SPX_N].copy_from_slice(&sk[2 * SPX_N..3 * SPX_N]);
    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    ctx.sk_seed.copy_from_slice(&sk[..SPX_N]);
    initialize_hash_function(&mut ctx);
    merkle_gen_root(&mut sk[3 * SPX_N..], &ctx);
    pk[SPX_N..2 * SPX_N].copy_from_slice(&sk[3 * SPX_N..4 * SPX_N]);
    0
}

fn crypto_sign_keypair_inner(pk: &mut [u8], sk: &mut [u8]) -> c_int {
    let mut seed = [0u8; CRYPTO_SEEDBYTES];
    randombytes_fn(&mut seed, CRYPTO_SEEDBYTES as u64);
    crypto_sign_seed_keypair_inner(pk, sk, &seed);
    0
}

fn crypto_sign_signature_inner(sig: &mut [u8], siglen: &mut usize, m: &[u8], mlen: usize, sk: &[u8]) -> c_int {
    let mut ctx = SpxCtx { pub_seed: [0; SPX_N], sk_seed: [0; SPX_N] };
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
    randombytes_fn(&mut optrand, SPX_N as u64);
    gen_message_random(sig, sk_prf, &optrand, m, mlen as u64, &ctx);

    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut root = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;

    hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig, pk, m, mlen as u64, &ctx);
    let mut sig_off = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_sign(&mut sig[sig_off..], &mut root, &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);
        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);

        merkle_sign(&mut sig[sig_off..], &mut root, &ctx, &mut wots_addr, &mut tree_addr, idx_leaf);
        sig_off += SPX_WOTS_BYTES + SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    *siglen = SPX_BYTES;
    0
}

fn crypto_sign_verify_inner(sig: &[u8], siglen: usize, m: &[u8], mlen: usize, pk: &[u8]) -> c_int {
    let mut ctx = SpxCtx { pub_seed: [0; SPX_N], sk_seed: [0; SPX_N] };
    let pub_root = &pk[SPX_N..];
    let mut mhash = [0u8; SPX_FORS_MSG_BYTES];
    let mut wots_pk = vec![0u8; SPX_WOTS_BYTES];
    let mut root = [0u8; SPX_N];
    let mut leaf = [0u8; SPX_N];
    let mut tree: u64 = 0;
    let mut idx_leaf: u32 = 0;
    let mut wots_addr = [0u32; 8];
    let mut tree_addr = [0u32; 8];
    let mut wots_pk_addr = [0u32; 8];

    if siglen != SPX_BYTES { return -1; }

    ctx.pub_seed.copy_from_slice(&pk[..SPX_N]);
    initialize_hash_function(&mut ctx);

    set_type(&mut wots_addr, SPX_ADDR_TYPE_WOTS);
    set_type(&mut tree_addr, SPX_ADDR_TYPE_HASHTREE);
    set_type(&mut wots_pk_addr, SPX_ADDR_TYPE_WOTSPK);

    hash_message(&mut mhash, &mut tree, &mut idx_leaf, sig, pk, m, mlen as u64, &ctx);
    let mut sig_off = SPX_N;

    set_tree_addr(&mut wots_addr, tree);
    set_keypair_addr(&mut wots_addr, idx_leaf);

    fors_pk_from_sig(&mut root, &sig[sig_off..], &mhash, &ctx, &wots_addr);
    sig_off += SPX_FORS_BYTES;

    for i in 0..SPX_D {
        set_layer_addr(&mut tree_addr, i as u32);
        set_tree_addr(&mut tree_addr, tree);
        copy_subtree_addr(&mut wots_addr, &tree_addr);
        set_keypair_addr(&mut wots_addr, idx_leaf);
        copy_keypair_addr(&mut wots_pk_addr, &wots_addr);

        wots_pk_from_sig(&mut wots_pk, &sig[sig_off..], &root, &ctx, &mut wots_addr);
        sig_off += SPX_WOTS_BYTES;

        thash(&mut leaf, &wots_pk, SPX_WOTS_LEN, &ctx, &mut wots_pk_addr);
        compute_root(&mut root, &leaf, idx_leaf, 0, &sig[sig_off..], SPX_TREE_HEIGHT as u32, &ctx, &mut tree_addr);
        sig_off += SPX_TREE_HEIGHT * SPX_N;

        idx_leaf = (tree & ((1 << SPX_TREE_HEIGHT) - 1)) as u32;
        tree >>= SPX_TREE_HEIGHT;
    }

    if root[..SPX_N] != pub_root[..SPX_N] { return -1; }
    0
}

fn crypto_sign_inner(sm: &mut [u8], smlen: &mut u64, m: &[u8], mlen: u64, sk: &[u8]) -> c_int {
    let mut siglen: usize = 0;
    crypto_sign_signature_inner(sm, &mut siglen, m, mlen as usize, sk);
    // memmove sm + SPX_BYTES, m, mlen
    let mlen_usize = mlen as usize;
    sm.copy_within(0..0, 0); // no-op, just for clarity
    // We need to copy m into sm[SPX_BYTES..] but sm might overlap with m in C.
    // In Rust, they're separate, so just copy.
    sm[SPX_BYTES..SPX_BYTES + mlen_usize].copy_from_slice(&m[..mlen_usize]);
    *smlen = (siglen as u64) + mlen;
    0
}

fn crypto_sign_open_inner(m_out: &mut [u8], mlen: &mut u64, sm: &[u8], smlen: u64, pk: &[u8]) -> c_int {
    if smlen < SPX_BYTES as u64 {
        m_out[..smlen as usize].fill(0);
        *mlen = 0;
        return -1;
    }
    *mlen = smlen - SPX_BYTES as u64;
    let mlen_usize = *mlen as usize;

    if crypto_sign_verify_inner(sm, SPX_BYTES, &sm[SPX_BYTES..SPX_BYTES + mlen_usize], mlen_usize, pk) != 0 {
        m_out[..smlen as usize].fill(0);
        *mlen = 0;
        return -1;
    }
    m_out[..mlen_usize].copy_from_slice(&sm[SPX_BYTES..SPX_BYTES + mlen_usize]);
    0
}

// ============================================================
// Extern "C" exports
// ============================================================
#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> libc::c_ulonglong {
    crypto_sign_secretkeybytes_inner() as libc::c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> libc::c_ulonglong {
    crypto_sign_publickeybytes_inner() as libc::c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> libc::c_ulonglong {
    crypto_sign_bytes_inner() as libc::c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> libc::c_ulonglong {
    crypto_sign_seedbytes_inner() as libc::c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8, sk: *mut u8, seed: *const u8,
) -> c_int {
    unsafe {
        let pk_s = std::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES);
        let sk_s = std::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES);
        let seed_s = std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES);
        crypto_sign_seed_keypair_inner(pk_s, sk_s, seed_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    unsafe {
        let pk_s = std::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES);
        let sk_s = std::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES);
        crypto_sign_keypair_inner(pk_s, sk_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8, siglen: *mut libc::size_t,
    m: *const u8, mlen: libc::size_t, sk: *const u8,
) -> c_int {
    unsafe {
        let sig_s = std::slice::from_raw_parts_mut(sig, SPX_BYTES);
        let m_s = std::slice::from_raw_parts(m, mlen);
        let sk_s = std::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES);
        let mut sl: usize = 0;
        let ret = crypto_sign_signature_inner(sig_s, &mut sl, m_s, mlen, sk_s);
        *siglen = sl;
        ret
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8, siglen: libc::size_t,
    m: *const u8, mlen: libc::size_t, pk: *const u8,
) -> c_int {
    unsafe {
        let sig_s = std::slice::from_raw_parts(sig, siglen);
        let m_s = std::slice::from_raw_parts(m, mlen);
        let pk_s = std::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES);
        crypto_sign_verify_inner(sig_s, siglen, m_s, mlen, pk_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8, smlen: *mut libc::c_ulonglong,
    m: *const u8, mlen: libc::c_ulonglong, sk: *const u8,
) -> c_int {
    unsafe {
        let total = SPX_BYTES + mlen as usize;
        let sm_s = std::slice::from_raw_parts_mut(sm, total);
        let m_s = std::slice::from_raw_parts(m, mlen as usize);
        let sk_s = std::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES);
        let mut sl: u64 = 0;
        let ret = crypto_sign_inner(sm_s, &mut sl, m_s, mlen as u64, sk_s);
        *smlen = sl as libc::c_ulonglong;
        ret
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut u8, mlen: *mut libc::c_ulonglong,
    sm: *const u8, smlen: libc::c_ulonglong, pk: *const u8,
) -> c_int {
    unsafe {
        let sm_s = std::slice::from_raw_parts(sm, smlen as usize);
        let m_s = std::slice::from_raw_parts_mut(m, smlen as usize);
        let pk_s = std::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES);
        let mut ml: u64 = 0;
        let ret = crypto_sign_open_inner(m_s, &mut ml, sm_s, smlen as u64, pk_s);
        *mlen = ml as libc::c_ulonglong;
        ret
    }
}

// ============================================================
// PQCgenKAT_sign (BLAKE_TR variant)
// ============================================================
const BASE_MLEN: usize = 33;
const LOOP_COUNT: usize = 7;

struct KatTrCtx {
    inner: Blakestate512,
}

fn kat_tr_init(ctx: &mut KatTrCtx) {
    blake512_init(&mut ctx.inner);
    let tag = b"KAT-TRANSCRIPT-v1-BLAKE";
    blake512_update(&mut ctx.inner, tag, (tag.len() as u64) * 8);
    let sep: u8 = 0x00;
    blake512_update(&mut ctx.inner, &[sep], 8);
}

fn kat_tr_absorb_label(ctx: &mut KatTrCtx, label: &[u8]) {
    blake512_update(&mut ctx.inner, label, (label.len() as u64) * 8);
    let sep: u8 = 0x00;
    blake512_update(&mut ctx.inner, &[sep], 8);
}

fn kat_tr_absorb_u64(ctx: &mut KatTrCtx, x: u64) {
    let mut le = [0u8; 8];
    for i in 0..8 { le[i] = ((x >> (8 * i)) & 0xFF) as u8; }
    let mut lenle = [0u8; 8];
    let l: u64 = 8;
    for i in 0..8 { lenle[i] = ((l >> (8 * i)) & 0xFF) as u8; }
    blake512_update(&mut ctx.inner, &lenle, 64);
    blake512_update(&mut ctx.inner, &le, 64);
}

fn kat_tr_absorb_bytes(ctx: &mut KatTrCtx, buf: &[u8], len: usize) {
    let mut lenle = [0u8; 8];
    let l = len as u64;
    for i in 0..8 { lenle[i] = ((l >> (8 * i)) & 0xFF) as u8; }
    blake512_update(&mut ctx.inner, &lenle, 64);
    if len > 0 {
        blake512_update(&mut ctx.inner, &buf[..len], (len as u64) * 8);
    }
}

fn kat_tr_final(ctx: &mut KatTrCtx, out32: &mut [u8; 32]) {
    let mut outbuf = [0u8; 64];
    blake512_final_fn(&mut ctx.inner, &mut outbuf);
    out32.copy_from_slice(&outbuf[..32]);
}

#[unsafe(no_mangle)]
pub extern "C" fn PQCgenKAT_sign() -> c_int {
    let mut m = vec![0u8; BASE_MLEN * LOOP_COUNT];
    let mut sm = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut m1 = vec![0u8; BASE_MLEN * LOOP_COUNT + CRYPTO_BYTES];
    let mut pk = vec![0u8; CRYPTO_PUBLICKEYBYTES];
    let mut sk = vec![0u8; CRYPTO_SECRETKEYBYTES];
    let mut seed = [0u8; 48];
    let mut entropy_input = [0u8; 48];
    let mut msg = vec![0u8; BASE_MLEN * LOOP_COUNT];

    for i in 0..48 { entropy_input[i] = i as u8; }
    randombytes_init(&entropy_input, None);

    let mut tctx = KatTrCtx {
        inner: Blakestate512 { h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128] },
    };
    kat_tr_init(&mut tctx);
    kat_tr_absorb_label(&mut tctx, b"CRYPTO_ALGNAME");
    kat_tr_absorb_bytes(&mut tctx, CRYPTO_ALGNAME, CRYPTO_ALGNAME.len());
    kat_tr_absorb_label(&mut tctx, b"SKBYTES"); kat_tr_absorb_u64(&mut tctx, CRYPTO_SECRETKEYBYTES as u64);
    kat_tr_absorb_label(&mut tctx, b"PKBYTES"); kat_tr_absorb_u64(&mut tctx, CRYPTO_PUBLICKEYBYTES as u64);
    kat_tr_absorb_label(&mut tctx, b"SIGBYTES"); kat_tr_absorb_u64(&mut tctx, CRYPTO_BYTES as u64);

    for i in 0..LOOP_COUNT {
        rng_randombytes(&mut seed, 48);

        kat_tr_absorb_label(&mut tctx, b"count"); kat_tr_absorb_u64(&mut tctx, i as u64);
        kat_tr_absorb_label(&mut tctx, b"seed"); kat_tr_absorb_bytes(&mut tctx, &seed, 48);

        let mlen = BASE_MLEN * (i + 1);
        kat_tr_absorb_label(&mut tctx, b"mlen"); kat_tr_absorb_u64(&mut tctx, mlen as u64);

        rng_randombytes(&mut msg[..mlen], mlen as u64);
        kat_tr_absorb_label(&mut tctx, b"msg"); kat_tr_absorb_bytes(&mut tctx, &msg, mlen);

        m[..mlen].fill(0);
        m1[..mlen + CRYPTO_BYTES].fill(0);
        sm[..mlen + CRYPTO_BYTES].fill(0);
        m[..mlen].copy_from_slice(&msg[..mlen]);

        let ret = crypto_sign_keypair_inner(&mut pk, &mut sk);
        if ret != 0 { return -2; }
        kat_tr_absorb_label(&mut tctx, b"pk"); kat_tr_absorb_bytes(&mut tctx, &pk, CRYPTO_PUBLICKEYBYTES);
        kat_tr_absorb_label(&mut tctx, b"sk"); kat_tr_absorb_bytes(&mut tctx, &sk, CRYPTO_SECRETKEYBYTES);

        let mut smlen: u64 = 0;
        let ret = crypto_sign_inner(&mut sm, &mut smlen, &m[..mlen], mlen as u64, &sk);
        if ret != 0 { return -2; }
        kat_tr_absorb_label(&mut tctx, b"smlen"); kat_tr_absorb_u64(&mut tctx, smlen);
        kat_tr_absorb_label(&mut tctx, b"sm"); kat_tr_absorb_bytes(&mut tctx, &sm, smlen as usize);

        let mut mlen1: u64 = 0;
        let ret = crypto_sign_open_inner(&mut m1, &mut mlen1, &sm[..smlen as usize], smlen, &pk);
        if ret != 0 { return -2; }
        if mlen1 != mlen as u64 { return -2; }
        if m[..mlen] != m1[..mlen] { return -2; }
    }

    let mut digest = [0u8; 32];
    kat_tr_final(&mut tctx, &mut digest);

    // Print digest
    print!("KAT transcript digest = ");
    for b in &digest { print!("{:02X}", b); }
    println!();

    0
}
