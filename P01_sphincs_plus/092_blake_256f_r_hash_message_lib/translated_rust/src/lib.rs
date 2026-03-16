#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use std::ptr;

// ============================================================
// params.h constants for blake-256f
// ============================================================
const SPX_N: usize = 32;
const SPX_FULL_HEIGHT: usize = 68;
const SPX_D: usize = 17;
const SPX_FORS_HEIGHT: usize = 9;
const SPX_FORS_TREES: usize = 35;
const SPX_WOTS_W: usize = 16;
const SPX_ADDR_BYTES: usize = 32;
const SPX_WOTS_LOGW: usize = 4;
const SPX_WOTS_LEN1: usize = 8 * SPX_N / SPX_WOTS_LOGW;
const SPX_WOTS_LEN2: usize = 3;
const SPX_WOTS_LEN: usize = SPX_WOTS_LEN1 + SPX_WOTS_LEN2;
const SPX_WOTS_BYTES: usize = SPX_WOTS_LEN * SPX_N;
const SPX_WOTS_PK_BYTES: usize = SPX_WOTS_BYTES;
const SPX_TREE_HEIGHT: usize = SPX_FULL_HEIGHT / SPX_D;
const SPX_FORS_MSG_BYTES: usize = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8;
const SPX_FORS_BYTES: usize = (SPX_FORS_HEIGHT + 1) * SPX_FORS_TREES * SPX_N;
const SPX_FORS_PK_BYTES: usize = SPX_N;
const SPX_BYTES: usize = SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
const SPX_PK_BYTES: usize = 2 * SPX_N;
const SPX_SK_BYTES: usize = 2 * SPX_N + SPX_PK_BYTES;

// blake_offsets.h
const SPX_OFFSET_LAYER: usize = 3;
const SPX_OFFSET_TREE: usize = 8;
const SPX_OFFSET_TYPE: usize = 19;
const SPX_OFFSET_KP_ADDR: usize = 20;
const SPX_OFFSET_CHAIN_ADDR: usize = 27;
const SPX_OFFSET_HASH_ADDR: usize = 31;
const SPX_OFFSET_TREE_HGT: usize = 27;
const SPX_OFFSET_TREE_INDEX: usize = 28;

// address types
const SPX_ADDR_TYPE_WOTS: u32 = 0;
const SPX_ADDR_TYPE_WOTSPK: u32 = 1;
const SPX_ADDR_TYPE_HASHTREE: u32 = 2;
const SPX_ADDR_TYPE_FORSTREE: u32 = 3;
const SPX_ADDR_TYPE_FORSPK: u32 = 4;
const SPX_ADDR_TYPE_WOTSPRF: u32 = 5;
const SPX_ADDR_TYPE_FORSPRF: u32 = 6;

const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;
const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

// For blake-256f: SPX_N=32 >= 24, so we use blake512 as blakeX
const SPX_BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE512_OUTPUT_BYTES;

// ============================================================
// context.h
// ============================================================
#[repr(C)]
pub struct spx_ctx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
}

// ============================================================
// blake state types (blake.h)
// ============================================================
#[repr(C)]
#[derive(Clone)]
struct blakestate256 {
    h: [u32; 8],
    s: [u32; 4],
    t: [u32; 2],
    buflen: i32,
    nullt: i32,
    buf: [u8; 64],
}

#[repr(C)]
#[derive(Clone)]
struct blakestate512 {
    h: [u64; 8],
    s: [u64; 4],
    t: [u64; 2],
    buflen: i32,
    nullt: i32,
    buf: [u8; 128],
}

// ============================================================
// blake256 constants
// ============================================================
static CST256: [u32; 16] = [
    0x243F6A88, 0x85A308D3, 0x13198A2E, 0x03707344,
    0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    0x452821E6, 0x38D01377, 0xBE5466CF, 0x34E90C6C,
    0xC0AC29B7, 0xC97C50DD, 0x3F84D5B5, 0xB5470917,
];

static PADDING256: [u8; 64] = [
    0x80,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
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

// ============================================================
// blake256_compress - macro ROUND expanded inline
// ============================================================
fn blake256_compress(s: &mut blakestate256, block: &[u8]) {
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

    macro_rules! round256 {
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
    round256!(m0,c[1],m1,c[0],m2,c[3],m3,c[2],m4,c[5],m5,c[4],m6,c[7],m7,c[6],m8,c[9],m9,c[8],m10,c[11],m11,c[10],m12,c[13],m13,c[12],m14,c[15],m15,c[14]);
    round256!(m14,c[10],m10,c[14],m4,c[8],m8,c[4],m9,c[15],m15,c[9],m13,c[6],m6,c[13],m1,c[12],m12,c[1],m0,c[2],m2,c[0],m11,c[7],m7,c[11],m5,c[3],m3,c[5]);
    round256!(m11,c[8],m8,c[11],m12,c[0],m0,c[12],m5,c[2],m2,c[5],m15,c[13],m13,c[15],m10,c[14],m14,c[10],m3,c[6],m6,c[3],m7,c[1],m1,c[7],m9,c[4],m4,c[9]);
    round256!(m7,c[9],m9,c[7],m3,c[1],m1,c[3],m13,c[12],m12,c[13],m11,c[14],m14,c[11],m2,c[6],m6,c[2],m5,c[10],m10,c[5],m4,c[0],m0,c[4],m15,c[8],m8,c[15]);
    round256!(m9,c[0],m0,c[9],m5,c[7],m7,c[5],m2,c[4],m4,c[2],m10,c[15],m15,c[10],m14,c[1],m1,c[14],m11,c[12],m12,c[11],m6,c[8],m8,c[6],m3,c[13],m13,c[3]);
    round256!(m2,c[12],m12,c[2],m6,c[10],m10,c[6],m0,c[11],m11,c[0],m8,c[3],m3,c[8],m4,c[13],m13,c[4],m7,c[5],m5,c[7],m15,c[14],m14,c[15],m1,c[9],m9,c[1]);
    round256!(m12,c[5],m5,c[12],m1,c[15],m15,c[1],m14,c[13],m13,c[14],m4,c[10],m10,c[4],m0,c[7],m7,c[0],m6,c[3],m3,c[6],m9,c[2],m2,c[9],m8,c[11],m11,c[8]);
    round256!(m13,c[11],m11,c[13],m7,c[14],m14,c[7],m12,c[1],m1,c[12],m3,c[9],m9,c[3],m5,c[0],m0,c[5],m15,c[4],m4,c[15],m8,c[6],m6,c[8],m2,c[10],m10,c[2]);
    round256!(m6,c[15],m15,c[6],m14,c[9],m9,c[14],m11,c[3],m3,c[11],m0,c[8],m8,c[0],m12,c[2],m2,c[12],m13,c[7],m7,c[13],m1,c[4],m4,c[1],m10,c[5],m5,c[10]);
    round256!(m10,c[2],m2,c[10],m8,c[4],m4,c[8],m7,c[6],m6,c[7],m1,c[5],m5,c[1],m15,c[11],m11,c[15],m9,c[14],m14,c[9],m3,c[12],m12,c[3],m13,c[0],m0,c[13]);
    round256!(m0,c[1],m1,c[0],m2,c[3],m3,c[2],m4,c[5],m5,c[4],m6,c[7],m7,c[6],m8,c[9],m9,c[8],m10,c[11],m11,c[10],m12,c[13],m13,c[12],m14,c[15],m15,c[14]);
    round256!(m14,c[10],m10,c[14],m4,c[8],m8,c[4],m9,c[15],m15,c[9],m13,c[6],m6,c[13],m1,c[12],m12,c[1],m0,c[2],m2,c[0],m11,c[7],m7,c[11],m5,c[3],m3,c[5]);
    round256!(m11,c[8],m8,c[11],m12,c[0],m0,c[12],m5,c[2],m2,c[5],m15,c[13],m13,c[15],m10,c[14],m14,c[10],m3,c[6],m6,c[3],m7,c[1],m1,c[7],m9,c[4],m4,c[9]);
    round256!(m7,c[9],m9,c[7],m3,c[1],m1,c[3],m13,c[12],m12,c[13],m11,c[14],m14,c[11],m2,c[6],m6,c[2],m5,c[10],m10,c[5],m4,c[0],m0,c[4],m15,c[8],m8,c[15]);

    v0 ^= v8;  v1 ^= v9;  v2 ^= v10; v3 ^= v11;
    v4 ^= v12; v5 ^= v13; v6 ^= v14; v7 ^= v15;
    v0 ^= s.s[0]; v1 ^= s.s[1]; v2 ^= s.s[2]; v3 ^= s.s[3];
    v4 ^= s.s[0]; v5 ^= s.s[1]; v6 ^= s.s[2]; v7 ^= s.s[3];
    s.h[0] ^= v0; s.h[1] ^= v1; s.h[2] ^= v2; s.h[3] ^= v3;
    s.h[4] ^= v4; s.h[5] ^= v5; s.h[6] ^= v6; s.h[7] ^= v7;
}

fn blake256_init(s: &mut blakestate256) {
    s.h[0] = 0x6A09E667; s.h[1] = 0xBB67AE85;
    s.h[2] = 0x3C6EF372; s.h[3] = 0xA54FF53A;
    s.h[4] = 0x510E527F; s.h[5] = 0x9B05688C;
    s.h[6] = 0x1F83D9AB; s.h[7] = 0x5BE0CD19;
    s.t[0] = 0; s.t[1] = 0; s.buflen = 0; s.nullt = 0;
    s.s[0] = 0; s.s[1] = 0; s.s[2] = 0; s.s[3] = 0;
}

fn blake256_update(s: &mut blakestate256, data: &[u8], mut datalen: u64) {
    let mut off = 0usize;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 64 - left;

    if left != 0 && ((datalen >> 3) & 0x3F) >= fill as u64 {
        s.buf[left..left + fill].copy_from_slice(&data[off..off + fill]);
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 { s.t[1] = s.t[1].wrapping_add(1); }
        let buf_copy = s.buf;
        blake256_compress(s, &buf_copy);
        off += fill;
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 512 {
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 { s.t[1] = s.t[1].wrapping_add(1); }
        blake256_compress(s, &data[off..]);
        off += 64;
        datalen -= 512;
    }

    if datalen > 0 {
        let bytes = (datalen >> 3) as usize;
        s.buf[left..left + bytes].copy_from_slice(&data[off..off + bytes]);
        s.buflen = ((left << 3) as u64 + datalen) as i32;
    } else {
        s.buflen = 0;
    }
}

fn blake256_final(s: &mut blakestate256, digest: &mut [u8]) {
    let mut msglen = [0u8; 8];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo = s.t[0].wrapping_add(s.buflen as u32);
    let mut hi = s.t[1];
    if lo < s.buflen as u32 { hi = hi.wrapping_add(1); }
    u32to8(&mut msglen[0..4], hi);
    u32to8(&mut msglen[4..8], lo);

    if s.buflen == 440 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake256_update(s, &[oo], 8);
    } else {
        if s.buflen < 440 {
            if s.buflen == 0 { s.nullt = 1; }
            s.t[0] = s.t[0].wrapping_sub((440 - s.buflen) as u32);
            blake256_update(s, &PADDING256, (440 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((512 - s.buflen) as u32);
            blake256_update(s, &PADDING256, (512 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(440);
            blake256_update(s, &PADDING256[1..], 440);
            s.nullt = 1;
        }
        blake256_update(s, &[zo], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(64);
    blake256_update(s, &msglen, 64);

    u32to8(&mut digest[0..4], s.h[0]);
    u32to8(&mut digest[4..8], s.h[1]);
    u32to8(&mut digest[8..12], s.h[2]);
    u32to8(&mut digest[12..16], s.h[3]);
    u32to8(&mut digest[16..20], s.h[4]);
    u32to8(&mut digest[20..24], s.h[5]);
    u32to8(&mut digest[24..28], s.h[6]);
    u32to8(&mut digest[28..32], s.h[7]);
}

fn blake256_hash(out: &mut [u8], inp: &[u8], inlen: u64) -> i32 {
    let mut s = blakestate256 { h: [0;8], s: [0;4], t: [0;2], buflen: 0, nullt: 0, buf: [0;64] };
    blake256_init(&mut s);
    blake256_update(&mut s, inp, inlen.wrapping_mul(8));
    blake256_final(&mut s, out);
    0
}

// ============================================================
// blake512
// ============================================================
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
    u32to8(&mut p[0..4], (v >> 32) as u32);
    u32to8(&mut p[4..8], v as u32);
}

fn blake512_rot(x: u64, n: u32) -> u64 {
    (x << (64 - n)) | (x >> n)
}

fn blake512_compress(s: &mut blakestate512, block: &[u8]) {
    let m0 = u8to64(&block[0..]);   let m1 = u8to64(&block[8..]);
    let m2 = u8to64(&block[16..]);  let m3 = u8to64(&block[24..]);
    let m4 = u8to64(&block[32..]);  let m5 = u8to64(&block[40..]);
    let m6 = u8to64(&block[48..]);  let m7 = u8to64(&block[56..]);
    let m8 = u8to64(&block[64..]);  let m9 = u8to64(&block[72..]);
    let m10 = u8to64(&block[80..]); let m11 = u8to64(&block[88..]);
    let m12 = u8to64(&block[96..]); let m13 = u8to64(&block[104..]);
    let m14 = u8to64(&block[112..]); let m15 = u8to64(&block[120..]);

    let mut v0 = s.h[0]; let mut v1 = s.h[1];
    let mut v2 = s.h[2]; let mut v3 = s.h[3];
    let mut v4 = s.h[4]; let mut v5 = s.h[5];
    let mut v6 = s.h[6]; let mut v7 = s.h[7];
    let mut v8 = s.s[0] ^ 0x243F6A8885A308D3u64;
    let mut v9 = s.s[1] ^ 0x13198A2E03707344u64;
    let mut v10 = s.s[2] ^ 0xA4093822299F31D0u64;
    let mut v11 = s.s[3] ^ 0x082EFA98EC4E6C89u64;
    let mut v12: u64 = 0x452821E638D01377;
    let mut v13: u64 = 0xBE5466CF34E90C6C;
    let mut v14: u64 = 0xC0AC29B7C97C50DD;
    let mut v15: u64 = 0x3F84D5B5B5470917;

    if s.nullt == 0 {
        v12 ^= s.t[0]; v13 ^= s.t[0];
        v14 ^= s.t[1]; v15 ^= s.t[1];
    }

    macro_rules! round512 {
        ($m0:expr,$c0:expr,$m1:expr,$c1:expr,$m2:expr,$c2:expr,$m3:expr,$c3:expr,
         $m4:expr,$c4:expr,$m5:expr,$c5:expr,$m6:expr,$c6:expr,$m7:expr,$c7:expr,
         $m8:expr,$c8:expr,$m9:expr,$c9:expr,$m10:expr,$c10:expr,$m11:expr,$c11:expr,
         $m12:expr,$c12:expr,$m13:expr,$c13:expr,$m14:expr,$c14:expr,$m15:expr,$c15:expr) => {
            v0 = v0.wrapping_add($m0 ^ $c0); v0 = v0.wrapping_add(v4);
            v12 ^= v0; v12 = blake512_rot(v12, 32);
            v8 = v8.wrapping_add(v12); v4 ^= v8; v4 = blake512_rot(v4, 25);
            v1 = v1.wrapping_add($m2 ^ $c2); v1 = v1.wrapping_add(v5);
            v13 ^= v1; v13 = blake512_rot(v13, 32);
            v9 = v9.wrapping_add(v13); v5 ^= v9; v5 = blake512_rot(v5, 25);
            v2 = v2.wrapping_add($m4 ^ $c4); v2 = v2.wrapping_add(v6);
            v14 ^= v2; v14 = blake512_rot(v14, 32);
            v10 = v10.wrapping_add(v14); v6 ^= v10; v6 = blake512_rot(v6, 25);
            v3 = v3.wrapping_add($m6 ^ $c6); v3 = v3.wrapping_add(v7);
            v15 ^= v3; v15 = blake512_rot(v15, 32);
            v11 = v11.wrapping_add(v15); v7 ^= v11; v7 = blake512_rot(v7, 25);
            v2 = v2.wrapping_add($m5 ^ $c5); v2 = v2.wrapping_add(v6);
            v14 ^= v2; v14 = blake512_rot(v14, 16);
            v10 = v10.wrapping_add(v14); v6 ^= v10; v6 = blake512_rot(v6, 11);
            v3 = v3.wrapping_add($m7 ^ $c7); v3 = v3.wrapping_add(v7);
            v15 ^= v3; v15 = blake512_rot(v15, 16);
            v11 = v11.wrapping_add(v15); v7 ^= v11; v7 = blake512_rot(v7, 11);
            v1 = v1.wrapping_add($m3 ^ $c3); v1 = v1.wrapping_add(v5);
            v13 ^= v1; v13 = blake512_rot(v13, 16);
            v9 = v9.wrapping_add(v13); v5 ^= v9; v5 = blake512_rot(v5, 11);
            v0 = v0.wrapping_add($m1 ^ $c1); v0 = v0.wrapping_add(v4);
            v12 ^= v0; v12 = blake512_rot(v12, 16);
            v8 = v8.wrapping_add(v12); v4 ^= v8; v4 = blake512_rot(v4, 11);
            // diagonal
            v0 = v0.wrapping_add($m8 ^ $c8); v0 = v0.wrapping_add(v5);
            v15 ^= v0; v15 = blake512_rot(v15, 32);
            v10 = v10.wrapping_add(v15); v5 ^= v10; v5 = blake512_rot(v5, 25);
            v1 = v1.wrapping_add($m10 ^ $c10); v1 = v1.wrapping_add(v6);
            v12 ^= v1; v12 = blake512_rot(v12, 32);
            v11 = v11.wrapping_add(v12); v6 ^= v11; v6 = blake512_rot(v6, 25);
            v2 = v2.wrapping_add($m12 ^ $c12); v2 = v2.wrapping_add(v7);
            v13 ^= v2; v13 = blake512_rot(v13, 32);
            v8 = v8.wrapping_add(v13); v7 ^= v8; v7 = blake512_rot(v7, 25);
            v3 = v3.wrapping_add($m14 ^ $c14); v3 = v3.wrapping_add(v4);
            v14 ^= v3; v14 = blake512_rot(v14, 32);
            v9 = v9.wrapping_add(v14); v4 ^= v9; v4 = blake512_rot(v4, 25);
            v2 = v2.wrapping_add($m13 ^ $c13); v2 = v2.wrapping_add(v7);
            v13 ^= v2; v13 = blake512_rot(v13, 16);
            v8 = v8.wrapping_add(v13); v7 ^= v8; v7 = blake512_rot(v7, 11);
            v3 = v3.wrapping_add($m15 ^ $c15); v3 = v3.wrapping_add(v4);
            v14 ^= v3; v14 = blake512_rot(v14, 16);
            v9 = v9.wrapping_add(v14); v4 ^= v9; v4 = blake512_rot(v4, 11);
            v1 = v1.wrapping_add($m11 ^ $c11); v1 = v1.wrapping_add(v6);
            v12 ^= v1; v12 = blake512_rot(v12, 16);
            v11 = v11.wrapping_add(v12); v6 ^= v11; v6 = blake512_rot(v6, 11);
            v0 = v0.wrapping_add($m9 ^ $c9); v0 = v0.wrapping_add(v5);
            v15 ^= v0; v15 = blake512_rot(v15, 16);
            v10 = v10.wrapping_add(v15); v5 ^= v10; v5 = blake512_rot(v5, 11);
        };
    }

    let c = &CST512;
    round512!(m0,c[1],m1,c[0],m2,c[3],m3,c[2],m4,c[5],m5,c[4],m6,c[7],m7,c[6],m8,c[9],m9,c[8],m10,c[11],m11,c[10],m12,c[13],m13,c[12],m14,c[15],m15,c[14]);
    round512!(m14,c[10],m10,c[14],m4,c[8],m8,c[4],m9,c[15],m15,c[9],m13,c[6],m6,c[13],m1,c[12],m12,c[1],m0,c[2],m2,c[0],m11,c[7],m7,c[11],m5,c[3],m3,c[5]);
    round512!(m11,c[8],m8,c[11],m12,c[0],m0,c[12],m5,c[2],m2,c[5],m15,c[13],m13,c[15],m10,c[14],m14,c[10],m3,c[6],m6,c[3],m7,c[1],m1,c[7],m9,c[4],m4,c[9]);
    round512!(m7,c[9],m9,c[7],m3,c[1],m1,c[3],m13,c[12],m12,c[13],m11,c[14],m14,c[11],m2,c[6],m6,c[2],m5,c[10],m10,c[5],m4,c[0],m0,c[4],m15,c[8],m8,c[15]);
    round512!(m9,c[0],m0,c[9],m5,c[7],m7,c[5],m2,c[4],m4,c[2],m10,c[15],m15,c[10],m14,c[1],m1,c[14],m11,c[12],m12,c[11],m6,c[8],m8,c[6],m3,c[13],m13,c[3]);
    round512!(m2,c[12],m12,c[2],m6,c[10],m10,c[6],m0,c[11],m11,c[0],m8,c[3],m3,c[8],m4,c[13],m13,c[4],m7,c[5],m5,c[7],m15,c[14],m14,c[15],m1,c[9],m9,c[1]);
    round512!(m12,c[5],m5,c[12],m1,c[15],m15,c[1],m14,c[13],m13,c[14],m4,c[10],m10,c[4],m0,c[7],m7,c[0],m6,c[3],m3,c[6],m9,c[2],m2,c[9],m8,c[11],m11,c[8]);
    round512!(m13,c[11],m11,c[13],m7,c[14],m14,c[7],m12,c[1],m1,c[12],m3,c[9],m9,c[3],m5,c[0],m0,c[5],m15,c[4],m4,c[15],m8,c[6],m6,c[8],m2,c[10],m10,c[2]);
    round512!(m6,c[15],m15,c[6],m14,c[9],m9,c[14],m11,c[3],m3,c[11],m0,c[8],m8,c[0],m12,c[2],m2,c[12],m13,c[7],m7,c[13],m1,c[4],m4,c[1],m10,c[5],m5,c[10]);
    round512!(m10,c[2],m2,c[10],m8,c[4],m4,c[8],m7,c[6],m6,c[7],m1,c[5],m5,c[1],m15,c[11],m11,c[15],m9,c[14],m14,c[9],m3,c[12],m12,c[3],m13,c[0],m0,c[13]);
    round512!(m0,c[1],m1,c[0],m2,c[3],m3,c[2],m4,c[5],m5,c[4],m6,c[7],m7,c[6],m8,c[9],m9,c[8],m10,c[11],m11,c[10],m12,c[13],m13,c[12],m14,c[15],m15,c[14]);
    round512!(m14,c[10],m10,c[14],m4,c[8],m8,c[4],m9,c[15],m15,c[9],m13,c[6],m6,c[13],m1,c[12],m12,c[1],m0,c[2],m2,c[0],m11,c[7],m7,c[11],m5,c[3],m3,c[5]);
    round512!(m11,c[8],m8,c[11],m12,c[0],m0,c[12],m5,c[2],m2,c[5],m15,c[13],m13,c[15],m10,c[14],m14,c[10],m3,c[6],m6,c[3],m7,c[1],m1,c[7],m9,c[4],m4,c[9]);
    round512!(m7,c[9],m9,c[7],m3,c[1],m1,c[3],m13,c[12],m12,c[13],m11,c[14],m14,c[11],m2,c[6],m6,c[2],m5,c[10],m10,c[5],m4,c[0],m0,c[4],m15,c[8],m8,c[15]);
    round512!(m9,c[0],m0,c[9],m5,c[7],m7,c[5],m2,c[4],m4,c[2],m10,c[15],m15,c[10],m14,c[1],m1,c[14],m11,c[12],m12,c[11],m6,c[8],m8,c[6],m3,c[13],m13,c[3]);
    round512!(m2,c[12],m12,c[2],m6,c[10],m10,c[6],m0,c[11],m11,c[0],m8,c[3],m3,c[8],m4,c[13],m13,c[4],m7,c[5],m5,c[7],m15,c[14],m14,c[15],m1,c[9],m9,c[1]);

    v0 ^= v8;  v1 ^= v9;  v2 ^= v10; v3 ^= v11;
    v4 ^= v12; v5 ^= v13; v6 ^= v14; v7 ^= v15;
    v0 ^= s.s[0]; v1 ^= s.s[1]; v2 ^= s.s[2]; v3 ^= s.s[3];
    v4 ^= s.s[0]; v5 ^= s.s[1]; v6 ^= s.s[2]; v7 ^= s.s[3];
    s.h[0] ^= v0; s.h[1] ^= v1; s.h[2] ^= v2; s.h[3] ^= v3;
    s.h[4] ^= v4; s.h[5] ^= v5; s.h[6] ^= v6; s.h[7] ^= v7;
}

fn blake512_init(s: &mut blakestate512) {
    s.h[0] = 0x6A09E667F3BCC908; s.h[1] = 0xBB67AE8584CAA73B;
    s.h[2] = 0x3C6EF372FE94F82B; s.h[3] = 0xA54FF53A5F1D36F1;
    s.h[4] = 0x510E527FADE682D1; s.h[5] = 0x9B05688C2B3E6C1F;
    s.h[6] = 0x1F83D9ABFB41BD6B; s.h[7] = 0x5BE0CD19137E2179;
    s.t[0] = 0; s.t[1] = 0; s.buflen = 0; s.nullt = 0;
    s.s[0] = 0; s.s[1] = 0; s.s[2] = 0; s.s[3] = 0;
}

fn blake512_update(s: &mut blakestate512, data: &[u8], mut datalen: u64) {
    let mut off = 0usize;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 128 - left;

    if left != 0 && ((datalen >> 3) & 0x7F) >= fill as u64 {
        s.buf[left..left + fill].copy_from_slice(&data[off..off + fill]);
        s.t[0] = s.t[0].wrapping_add(1024);
        let buf_copy = s.buf;
        blake512_compress(s, &buf_copy);
        off += fill;
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 1024 {
        s.t[0] = s.t[0].wrapping_add(1024);
        blake512_compress(s, &data[off..]);
        off += 128;
        datalen -= 1024;
    }

    if datalen > 0 {
        let bytes = ((datalen >> 3) & 0x7F) as usize;
        s.buf[left..left + bytes].copy_from_slice(&data[off..off + bytes]);
        s.buflen = ((left << 3) as u64 + datalen) as i32;
    } else {
        s.buflen = 0;
    }
}

fn blake512_final(s: &mut blakestate512, digest: &mut [u8]) {
    let mut msglen = [0u8; 16];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo = s.t[0].wrapping_add(s.buflen as u64);
    let mut hi = s.t[1];
    if lo < s.buflen as u64 { hi = hi.wrapping_add(1); }
    u64to8(&mut msglen[0..8], hi);
    u64to8(&mut msglen[8..16], lo);

    if s.buflen == 888 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake512_update(s, &[oo], 8);
    } else {
        if s.buflen < 888 {
            if s.buflen == 0 { s.nullt = 1; }
            s.t[0] = s.t[0].wrapping_sub((888 - s.buflen) as u64);
            blake512_update(s, &PADDING512, (888 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((1024 - s.buflen) as u64);
            blake512_update(s, &PADDING512, (1024 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(888);
            blake512_update(s, &PADDING512[1..], 888);
            s.nullt = 1;
        }
        blake512_update(s, &[zo], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(128);
    blake512_update(s, &msglen, 128);

    u64to8(&mut digest[0..8], s.h[0]);   u64to8(&mut digest[8..16], s.h[1]);
    u64to8(&mut digest[16..24], s.h[2]); u64to8(&mut digest[24..32], s.h[3]);
    u64to8(&mut digest[32..40], s.h[4]); u64to8(&mut digest[40..48], s.h[5]);
    u64to8(&mut digest[48..56], s.h[6]); u64to8(&mut digest[56..64], s.h[7]);
}

fn blake512_hash(out: &mut [u8], inp: &[u8], inlen: u64) -> i32 {
    let mut s = blakestate512 { h: [0;8], s: [0;4], t: [0;2], buflen: 0, nullt: 0, buf: [0;128] };
    blake512_init(&mut s);
    blake512_update(&mut s, inp, inlen.wrapping_mul(8));
    blake512_final(&mut s, out);
    0
}

// ============================================================
// MGF1 functions
// ============================================================
fn blake256_mgf1_internal(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut i: usize = 0;
    let mut off = 0usize;
    while (i + 1) * SPX_BLAKE256_OUTPUT_BYTES <= outlen {
        u32_to_bytes_internal(&mut inbuf[inlen..], i as u32);
        blake256_hash(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += SPX_BLAKE256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_BLAKE256_OUTPUT_BYTES {
        u32_to_bytes_internal(&mut inbuf[inlen..], i as u32);
        blake256_hash(&mut outbuf, &inbuf, (inlen + 4) as u64);
        out[off..off + (outlen - i * SPX_BLAKE256_OUTPUT_BYTES)]
            .copy_from_slice(&outbuf[..outlen - i * SPX_BLAKE256_OUTPUT_BYTES]);
    }
}

fn blake512_mgf1_internal(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut i: usize = 0;
    let mut off = 0usize;
    while (i + 1) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        u32_to_bytes_internal(&mut inbuf[inlen..], i as u32);
        blake512_hash(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += SPX_BLAKE512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_BLAKE512_OUTPUT_BYTES {
        u32_to_bytes_internal(&mut inbuf[inlen..], i as u32);
        blake512_hash(&mut outbuf, &inbuf, (inlen + 4) as u64);
        out[off..off + (outlen - i * SPX_BLAKE512_OUTPUT_BYTES)]
            .copy_from_slice(&outbuf[..outlen - i * SPX_BLAKE512_OUTPUT_BYTES]);
    }
}

// ============================================================
// utils.c
// ============================================================
fn ull_to_bytes_internal(out: &mut [u8], outlen: usize, mut val: u64) {
    let mut i = outlen as isize - 1;
    while i >= 0 {
        out[i as usize] = (val & 0xff) as u8;
        val >>= 8;
        i -= 1;
    }
}

fn u32_to_bytes_internal(out: &mut [u8], val: u32) {
    out[0] = (val >> 24) as u8;
    out[1] = (val >> 16) as u8;
    out[2] = (val >> 8) as u8;
    out[3] = val as u8;
}

fn bytes_to_ull_internal(inp: &[u8], inlen: usize) -> u64 {
    let mut retval: u64 = 0;
    for i in 0..inlen {
        retval |= (inp[i] as u64) << (8 * (inlen - 1 - i));
    }
    retval
}

// ============================================================
// address.c
// ============================================================
fn addr_bytes(addr: &[u32; 8]) -> &[u8; 32] {
    unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) }
}

fn addr_bytes_mut(addr: &mut [u32; 8]) -> &mut [u8; 32] {
    unsafe { &mut *(addr as *mut [u32; 8] as *mut [u8; 32]) }
}

fn set_layer_addr_internal(addr: &mut [u32; 8], layer: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_LAYER] = layer as u8;
}

fn set_tree_addr_internal(addr: &mut [u32; 8], tree: u64) {
    ull_to_bytes_internal(&mut addr_bytes_mut(addr)[SPX_OFFSET_TREE..], 8, tree);
}

fn set_type_internal(addr: &mut [u32; 8], type_val: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_TYPE] = type_val as u8;
}

fn copy_subtree_addr_internal(out: &mut [u32; 8], inp: &[u32; 8]) {
    let src = addr_bytes(inp);
    let dst = addr_bytes_mut(out);
    dst[..SPX_OFFSET_TREE + 8].copy_from_slice(&src[..SPX_OFFSET_TREE + 8]);
}

fn set_keypair_addr_internal(addr: &mut [u32; 8], keypair: u32) {
    u32_to_bytes_internal(&mut addr_bytes_mut(addr)[SPX_OFFSET_KP_ADDR..], keypair);
}

fn copy_keypair_addr_internal(out: &mut [u32; 8], inp: &[u32; 8]) {
    let src = addr_bytes(inp);
    let dst = addr_bytes_mut(out);
    dst[..SPX_OFFSET_TREE + 8].copy_from_slice(&src[..SPX_OFFSET_TREE + 8]);
    dst[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]
        .copy_from_slice(&src[SPX_OFFSET_KP_ADDR..SPX_OFFSET_KP_ADDR + 4]);
}

fn set_chain_addr_internal(addr: &mut [u32; 8], chain: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_CHAIN_ADDR] = chain as u8;
}

fn set_hash_addr_internal(addr: &mut [u32; 8], hash: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_HASH_ADDR] = hash as u8;
}

fn set_tree_height_internal(addr: &mut [u32; 8], tree_height: u32) {
    addr_bytes_mut(addr)[SPX_OFFSET_TREE_HGT] = tree_height as u8;
}

fn set_tree_index_internal(addr: &mut [u32; 8], tree_index: u32) {
    u32_to_bytes_internal(&mut addr_bytes_mut(addr)[SPX_OFFSET_TREE_INDEX..], tree_index);
}

// ============================================================
// thash_blake_robust.c
// ============================================================
fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &spx_ctx, addr: &mut [u32; 8]) {
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes(addr));

    blake512_mgf1_internal(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    blake512_hash(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + inblocks * SPX_N) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn thash_internal(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &spx_ctx, addr: &mut [u32; 8]) {
    // SPX_BLAKE512 is 1 for blake-256f
    if inblocks > 1 {
        thash_512(out, inp, inblocks, ctx, addr);
        return;
    }
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes(addr));

    blake256_mgf1_internal(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    blake256_hash(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + inblocks * SPX_N) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// ============================================================
// compute_root and treehash from utils.c
// ============================================================
fn compute_root_internal(
    root: &mut [u8], leaf: &[u8], mut leaf_idx: u32, mut idx_offset: u32,
    auth_path: &[u8], tree_height: u32, ctx: &spx_ctx, addr: &mut [u32; 8],
) {
    let mut buffer = [0u8; 2 * SPX_N];
    let mut ap_off = 0usize;

    if leaf_idx & 1 != 0 {
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[..SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
    } else {
        buffer[..SPX_N].copy_from_slice(&leaf[..SPX_N]);
        buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
    }
    ap_off += SPX_N;

    for i in 0..tree_height - 1 {
        leaf_idx >>= 1;
        idx_offset >>= 1;
        set_tree_height_internal(addr, i + 1);
        set_tree_index_internal(addr, leaf_idx + idx_offset);

        if leaf_idx & 1 != 0 {
            let tmp = buffer;
            thash_internal(&mut buffer[SPX_N..], &tmp, 2, ctx, addr);
            buffer[..SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
        } else {
            let tmp = buffer;
            thash_internal(&mut buffer[..SPX_N], &tmp, 2, ctx, addr);
            buffer[SPX_N..2 * SPX_N].copy_from_slice(&auth_path[ap_off..ap_off + SPX_N]);
        }
        ap_off += SPX_N;
    }

    leaf_idx >>= 1;
    idx_offset >>= 1;
    set_tree_height_internal(addr, tree_height);
    set_tree_index_internal(addr, leaf_idx + idx_offset);
    thash_internal(root, &buffer, 2, ctx, addr);
}

// ============================================================
// Public extern "C" API
// ============================================================

// --- utils ---
#[unsafe(no_mangle)]
pub extern "C" fn SPX_ull_to_bytes(out: *mut u8, outlen: u32, val: u64) {
    let s = unsafe { std::slice::from_raw_parts_mut(out, outlen as usize) };
    ull_to_bytes_internal(s, outlen as usize, val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_u32_to_bytes(out: *mut u8, val: u32) {
    let s = unsafe { std::slice::from_raw_parts_mut(out, 4) };
    u32_to_bytes_internal(s, val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_bytes_to_ull(inp: *const u8, inlen: u32) -> u64 {
    let s = unsafe { std::slice::from_raw_parts(inp, inlen as usize) };
    bytes_to_ull_internal(s, inlen as usize)
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_compute_root(
    root: *mut u8, leaf: *const u8, leaf_idx: u32, idx_offset: u32,
    auth_path: *const u8, tree_height: u32, ctx: *const spx_ctx, addr: *mut u32,
) {
    unsafe {
        let ctx_ref = &*ctx;
        let addr_ref = &mut *(addr as *mut [u32; 8]);
        let leaf_s = std::slice::from_raw_parts(leaf, SPX_N);
        let ap_s = std::slice::from_raw_parts(auth_path, tree_height as usize * SPX_N);
        let root_s = std::slice::from_raw_parts_mut(root, SPX_N);
        compute_root_internal(root_s, leaf_s, leaf_idx, idx_offset, ap_s, tree_height, ctx_ref, addr_ref);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_treehash(
    root: *mut u8, auth_path: *mut u8, ctx: *const spx_ctx,
    leaf_idx: u32, idx_offset: u32, tree_height: u32,
    gen_leaf: unsafe extern "C" fn(*mut u8, *const spx_ctx, u32, *const u32),
    tree_addr: *mut u32,
) {
    unsafe {
        let ctx_ref = &*ctx;
        let tree_addr_ref = &mut *(tree_addr as *mut [u32; 8]);
        let num_leaves = 1u32 << tree_height;
        let mut stack = vec![0u8; (tree_height as usize + 1) * SPX_N];
        let mut heights = vec![0u32; tree_height as usize + 1];
        let mut offset: usize = 0;

        for idx in 0..num_leaves {
            gen_leaf(stack.as_mut_ptr().add(offset * SPX_N), ctx, idx + idx_offset, tree_addr as *const u32);
            offset += 1;
            heights[offset - 1] = 0;

            if (leaf_idx ^ 0x1) == idx {
                ptr::copy_nonoverlapping(stack.as_ptr().add((offset - 1) * SPX_N), auth_path, SPX_N);
            }

            while offset >= 2 && heights[offset - 1] == heights[offset - 2] {
                let tree_idx = idx >> (heights[offset - 1] + 1);
                set_tree_height_internal(tree_addr_ref, heights[offset - 1] + 1);
                set_tree_index_internal(tree_addr_ref, tree_idx + (idx_offset >> (heights[offset - 1] + 1)));
                let src_off = (offset - 2) * SPX_N;
                let mut tmp = vec![0u8; 2 * SPX_N];
                tmp.copy_from_slice(&stack[src_off..src_off + 2 * SPX_N]);
                thash_internal(&mut stack[src_off..src_off + SPX_N], &tmp, 2, ctx_ref, tree_addr_ref);
                offset -= 1;
                heights[offset - 1] += 1;

                if ((leaf_idx >> heights[offset - 1]) ^ 0x1) == tree_idx {
                    ptr::copy_nonoverlapping(
                        stack.as_ptr().add((offset - 1) * SPX_N),
                        auth_path.add(heights[offset - 1] as usize * SPX_N),
                        SPX_N,
                    );
                }
            }
        }
        ptr::copy_nonoverlapping(stack.as_ptr(), root, SPX_N);
    }
}

// --- address ---
#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_layer_addr(addr: *mut u32, layer: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_layer_addr_internal(a, layer);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_addr(addr: *mut u32, tree: u64) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_tree_addr_internal(a, tree);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_type(addr: *mut u32, type_val: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_type_internal(a, type_val);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_subtree_addr(out: *mut u32, inp: *const u32) {
    let o = unsafe { &mut *(out as *mut [u32; 8]) };
    let i = unsafe { &*(inp as *const [u32; 8]) };
    copy_subtree_addr_internal(o, i);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_keypair_addr(addr: *mut u32, keypair: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_keypair_addr_internal(a, keypair);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_copy_keypair_addr(out: *mut u32, inp: *const u32) {
    let o = unsafe { &mut *(out as *mut [u32; 8]) };
    let i = unsafe { &*(inp as *const [u32; 8]) };
    copy_keypair_addr_internal(o, i);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_chain_addr(addr: *mut u32, chain: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_chain_addr_internal(a, chain);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_hash_addr(addr: *mut u32, hash: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_hash_addr_internal(a, hash);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_height(addr: *mut u32, tree_height: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_tree_height_internal(a, tree_height);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_set_tree_index(addr: *mut u32, tree_index: u32) {
    let a = unsafe { &mut *(addr as *mut [u32; 8]) };
    set_tree_index_internal(a, tree_index);
}

// --- thash ---
#[unsafe(no_mangle)]
pub extern "C" fn SPX_thash(
    out: *mut u8, inp: *const u8, inblocks: u32, ctx: *const spx_ctx, addr: *mut u32,
) {
    unsafe {
        let ctx_ref = &*ctx;
        let addr_ref = &mut *(addr as *mut [u32; 8]);
        let in_s = std::slice::from_raw_parts(inp, inblocks as usize * SPX_N);
        let out_s = std::slice::from_raw_parts_mut(out, SPX_N);
        thash_internal(out_s, in_s, inblocks as usize, ctx_ref, addr_ref);
    }
}

// --- hash_blake (initialize_hash_function, prf_addr, gen_message_random, hash_message) ---
#[unsafe(no_mangle)]
pub extern "C" fn SPX_initialize_hash_function(_ctx: *mut spx_ctx) {
    // no-op for blake
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const spx_ctx, addr: *const u32) {
    unsafe {
        let ctx_ref = &*ctx;
        let addr_ref = &*(addr as *const [u32; 8]);
        let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
        let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

        buf[..SPX_N].copy_from_slice(&ctx_ref.pub_seed);
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes(addr_ref));
        buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + SPX_N].copy_from_slice(&ctx_ref.sk_seed);

        blake256_hash(&mut outbuf, &buf, (SPX_N + SPX_ADDR_BYTES) as u64);

        let out_s = std::slice::from_raw_parts_mut(out, SPX_N);
        out_s.copy_from_slice(&outbuf[..SPX_N]);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_gen_message_random(
    r: *mut u8, sk_prf: *const u8, optrand: *const u8,
    m: *const u8, mlen: u64, _ctx: *const spx_ctx,
) {
    unsafe {
        let sk_prf_s = std::slice::from_raw_parts(sk_prf, SPX_N);
        let optrand_s = std::slice::from_raw_parts(optrand, SPX_N);
        let m_s = std::slice::from_raw_parts(m, mlen as usize);
        let r_s = std::slice::from_raw_parts_mut(r, SPX_BLAKEX_OUTPUT_BYTES);

        // blakeX = blake512 for SPX_N >= 24
        let mut s = blakestate512 { h: [0;8], s: [0;4], t: [0;2], buflen: 0, nullt: 0, buf: [0;128] };
        blake512_init(&mut s);
        blake512_update(&mut s, sk_prf_s, (SPX_N as u64) * 8);
        blake512_update(&mut s, optrand_s, (SPX_N as u64) * 8);
        blake512_update(&mut s, m_s, mlen * 8);
        blake512_final(&mut s, r_s);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_hash_message(
    digest: *mut u8, tree: *mut u64, leaf_idx: *mut u32,
    r_val: *const u8, pk: *const u8, m: *const u8, mlen: u64,
    _ctx: *const spx_ctx,
) {
    const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

    unsafe {
        let r_s = std::slice::from_raw_parts(r_val, SPX_N);
        let pk_s = std::slice::from_raw_parts(pk, SPX_PK_BYTES);
        let m_s = std::slice::from_raw_parts(m, mlen as usize);

        let mut buf = [0u8; SPX_DGST_BYTES];
        let mut seed = [0u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES];

        // blakeX = blake512
        let mut s = blakestate512 { h: [0;8], s: [0;4], t: [0;2], buflen: 0, nullt: 0, buf: [0;128] };
        blake512_init(&mut s);
        blake512_update(&mut s, r_s, (SPX_N as u64) * 8);
        blake512_update(&mut s, pk_s, (SPX_PK_BYTES as u64) * 8);
        blake512_update(&mut s, m_s, mlen * 8);
        blake512_final(&mut s, &mut seed[2 * SPX_N..]);

        seed[..SPX_N].copy_from_slice(r_s);
        seed[SPX_N..2 * SPX_N].copy_from_slice(&pk_s[..SPX_N]);

        // blakeX_mgf1 = blake512_mgf1
        blake512_mgf1_internal(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES);

        let digest_s = std::slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES);
        digest_s.copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);

        let mut bufp = SPX_FORS_MSG_BYTES;

        if SPX_D == 1 {
            *tree = 0;
        } else {
            *tree = bytes_to_ull_internal(&buf[bufp..], SPX_TREE_BYTES);
            *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
        }
        bufp += SPX_TREE_BYTES;

        *leaf_idx = bytes_to_ull_internal(&buf[bufp..], SPX_LEAF_BYTES) as u32;
        *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
    }
}

// --- blake MGF1 extern C wrappers ---
#[unsafe(no_mangle)]
pub extern "C" fn SPX_blake256_mgf1(
    out: *mut u8, outlen: u64, inp: *const u8, inlen: u64,
) {
    let out_s = unsafe { std::slice::from_raw_parts_mut(out, outlen as usize) };
    let in_s = unsafe { std::slice::from_raw_parts(inp, inlen as usize) };
    blake256_mgf1_internal(out_s, outlen as usize, in_s, inlen as usize);
}

#[unsafe(no_mangle)]
pub extern "C" fn SPX_blake512_mgf1(
    out: *mut u8, outlen: u64, inp: *const u8, inlen: u64,
) {
    let out_s = unsafe { std::slice::from_raw_parts_mut(out, outlen as usize) };
    let in_s = unsafe { std::slice::from_raw_parts(inp, inlen as usize) };
    blake512_mgf1_internal(out_s, outlen as usize, in_s, inlen as usize);
}

// --- blake hash extern C wrappers ---
#[unsafe(no_mangle)]
pub extern "C" fn blake256(out: *mut u8, inp: *const u8, inlen: u64) -> i32 {
    let out_s = unsafe { std::slice::from_raw_parts_mut(out, SPX_BLAKE256_OUTPUT_BYTES) };
    let in_s = unsafe { std::slice::from_raw_parts(inp, inlen as usize) };
    blake256_hash(out_s, in_s, inlen)
}

#[unsafe(no_mangle)]
pub extern "C" fn blake512(out: *mut u8, inp: *const u8, inlen: u64) -> i32 {
    let out_s = unsafe { std::slice::from_raw_parts_mut(out, SPX_BLAKE512_OUTPUT_BYTES) };
    let in_s = unsafe { std::slice::from_raw_parts(inp, inlen as usize) };
    blake512_hash(out_s, in_s, inlen)
}

// --- blake init/compress/update/final extern C wrappers ---
#[unsafe(no_mangle)]
pub extern "C" fn blake256_init_ext(s: *mut blakestate256) {
    blake256_init(unsafe { &mut *s });
}

#[unsafe(no_mangle)]
pub extern "C" fn blake256_compress_ext(s: *mut blakestate256, block: *const u8) {
    let blk = unsafe { std::slice::from_raw_parts(block, 64) };
    blake256_compress(unsafe { &mut *s }, blk);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake256_update_ext(s: *mut blakestate256, data: *const u8, datalen: u64) {
    let d = unsafe { std::slice::from_raw_parts(data, (datalen / 8 + 1) as usize) };
    blake256_update(unsafe { &mut *s }, d, datalen);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake256_final_ext(s: *mut blakestate256, digest: *mut u8) {
    let d = unsafe { std::slice::from_raw_parts_mut(digest, SPX_BLAKE256_OUTPUT_BYTES) };
    blake256_final(unsafe { &mut *s }, d);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake512_init_ext(s: *mut blakestate512) {
    blake512_init(unsafe { &mut *s });
}

#[unsafe(no_mangle)]
pub extern "C" fn blake512_compress_ext(s: *mut blakestate512, block: *const u8) {
    let blk = unsafe { std::slice::from_raw_parts(block, 128) };
    blake512_compress(unsafe { &mut *s }, blk);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake512_update_ext(s: *mut blakestate512, data: *const u8, datalen: u64) {
    let d = unsafe { std::slice::from_raw_parts(data, (datalen / 8 + 1) as usize) };
    blake512_update(unsafe { &mut *s }, d, datalen);
}

#[unsafe(no_mangle)]
pub extern "C" fn blake512_final_ext(s: *mut blakestate512, digest: *mut u8) {
    let d = unsafe { std::slice::from_raw_parts_mut(digest, SPX_BLAKE512_OUTPUT_BYTES) };
    blake512_final(unsafe { &mut *s }, d);
}
