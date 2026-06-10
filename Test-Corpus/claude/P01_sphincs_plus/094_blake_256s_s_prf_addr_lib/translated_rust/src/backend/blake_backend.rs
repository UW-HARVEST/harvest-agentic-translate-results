// BLAKE backend - direct translation from c_src/lib/blake.

#![allow(non_snake_case)]

use crate::address::addr_to_bytes;
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::{bytes_to_ull_rs, u32_to_bytes_rs};

const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;
const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

// =========================
// Pure-Rust BLAKE-256 / BLAKE-512 matching the reference impl
// =========================

#[derive(Clone)]
#[repr(C)]
pub struct BlakeState256 {
    pub h: [u32; 8],
    pub s: [u32; 4],
    pub t: [u32; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 64],
}

#[derive(Clone)]
#[repr(C)]
pub struct BlakeState512 {
    pub h: [u64; 8],
    pub s: [u64; 4],
    pub t: [u64; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 128],
}

const CST256: [u32; 16] = [
    0x243F6A88, 0x85A308D3, 0x13198A2E, 0x03707344,
    0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    0x452821E6, 0x38D01377, 0xBE5466CF, 0x34E90C6C,
    0xC0AC29B7, 0xC97C50DD, 0x3F84D5B5, 0xB5470917,
];

const CST512: [u64; 16] = [
    0x243F6A8885A308D3, 0x13198A2E03707344, 0xA4093822299F31D0, 0x082EFA98EC4E6C89,
    0x452821E638D01377, 0xBE5466CF34E90C6C, 0xC0AC29B7C97C50DD, 0x3F84D5B5B5470917,
    0x9216D5D98979FB1B, 0xD1310BA698DFB5AC, 0x2FFD72DBD01ADFB7, 0xB8E1AFED6A267E96,
    0xBA7C9045F12C7F99, 0x24A19947B3916CF7, 0x0801F2E2858EFC16, 0x636920D871574E69,
];

const PADDING: [u8; 129] = {
    let mut p = [0u8; 129];
    p[0] = 0x80;
    p
};

const SIGMA: [[usize; 16]; 16] = [
    [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15],
    [14,10,4,8,9,15,13,6,1,12,0,2,11,7,5,3],
    [11,8,12,0,5,2,15,13,10,14,3,6,7,1,9,4],
    [7,9,3,1,13,12,11,14,2,6,5,10,4,0,15,8],
    [9,0,5,7,2,4,10,15,14,1,11,12,6,8,3,13],
    [2,12,6,10,0,11,8,3,4,13,7,5,15,14,1,9],
    [12,5,1,15,14,13,4,10,0,7,6,3,9,2,8,11],
    [13,11,7,14,12,1,3,9,5,0,15,4,8,6,2,10],
    [6,15,14,9,11,3,0,8,12,2,13,7,1,4,10,5],
    [10,2,8,4,7,6,1,5,15,11,9,14,3,12,13,0],
    // BLAKE-256 has 14 rounds, BLAKE-512 has 16 rounds; sigma repeats from 0,1,2,3.
    [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15],
    [14,10,4,8,9,15,13,6,1,12,0,2,11,7,5,3],
    [11,8,12,0,5,2,15,13,10,14,3,6,7,1,9,4],
    [7,9,3,1,13,12,11,14,2,6,5,10,4,0,15,8],
    [9,0,5,7,2,4,10,15,14,1,11,12,6,8,3,13],
    [2,12,6,10,0,11,8,3,4,13,7,5,15,14,1,9],
];

fn rot256(x: u32, n: u32) -> u32 {
    x.rotate_right(n)
}
fn rot512(x: u64, n: u32) -> u64 {
    x.rotate_right(n)
}

fn u8to32(p: &[u8]) -> u32 {
    ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32)
}
fn u8to64(p: &[u8]) -> u64 {
    let hi = u8to32(&p[..4]) as u64;
    let lo = u8to32(&p[4..]) as u64;
    (hi << 32) | lo
}
fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8;
    p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8;
    p[3] = v as u8;
}
fn u64to8(p: &mut [u8], v: u64) {
    u32to8(&mut p[..4], (v >> 32) as u32);
    u32to8(&mut p[4..], v as u32);
}

pub fn blake256_init(s: &mut BlakeState256) {
    s.h = [
        0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
        0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
    ];
    s.t = [0; 2];
    s.buflen = 0;
    s.nullt = 0;
    s.s = [0; 4];
    s.buf = [0u8; 64];
}

pub fn blake512_init(s: &mut BlakeState512) {
    s.h = [
        0x6A09E667F3BCC908, 0xBB67AE8584CAA73B, 0x3C6EF372FE94F82B, 0xA54FF53A5F1D36F1,
        0x510E527FADE682D1, 0x9B05688C2B3E6C1F, 0x1F83D9ABFB41BD6B, 0x5BE0CD19137E2179,
    ];
    s.t = [0; 2];
    s.buflen = 0;
    s.nullt = 0;
    s.s = [0; 4];
    s.buf = [0u8; 128];
}

fn blake256_compress(state: &mut BlakeState256, block: &[u8]) {
    let mut m = [0u32; 16];
    for i in 0..16 {
        m[i] = u8to32(&block[4 * i..]);
    }
    let mut v = [0u32; 16];
    for i in 0..8 {
        v[i] = state.h[i];
    }
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
    fn g256(v: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, mi: u32, ki: u32) {
        v[a] = v[a].wrapping_add(v[b]).wrapping_add(mi ^ ki);
        v[d] = rot256(v[d] ^ v[a], 16);
        v[c] = v[c].wrapping_add(v[d]);
        v[b] = rot256(v[b] ^ v[c], 12);
    }
    fn g256_2(v: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, mi: u32, ki: u32) {
        v[a] = v[a].wrapping_add(v[b]).wrapping_add(mi ^ ki);
        v[d] = rot256(v[d] ^ v[a], 8);
        v[c] = v[c].wrapping_add(v[d]);
        v[b] = rot256(v[b] ^ v[c], 7);
    }
    for r in 0..14 {
        let s = SIGMA[r];
        // Column step
        g256(&mut v, 0, 4, 8, 12, m[s[0]], CST256[s[1]]);
        g256_2(&mut v, 0, 4, 8, 12, m[s[1]], CST256[s[0]]);
        g256(&mut v, 1, 5, 9, 13, m[s[2]], CST256[s[3]]);
        g256_2(&mut v, 1, 5, 9, 13, m[s[3]], CST256[s[2]]);
        g256(&mut v, 2, 6, 10, 14, m[s[4]], CST256[s[5]]);
        g256_2(&mut v, 2, 6, 10, 14, m[s[5]], CST256[s[4]]);
        g256(&mut v, 3, 7, 11, 15, m[s[6]], CST256[s[7]]);
        g256_2(&mut v, 3, 7, 11, 15, m[s[7]], CST256[s[6]]);
        // Diagonal step
        g256(&mut v, 0, 5, 10, 15, m[s[8]], CST256[s[9]]);
        g256_2(&mut v, 0, 5, 10, 15, m[s[9]], CST256[s[8]]);
        g256(&mut v, 1, 6, 11, 12, m[s[10]], CST256[s[11]]);
        g256_2(&mut v, 1, 6, 11, 12, m[s[11]], CST256[s[10]]);
        g256(&mut v, 2, 7, 8, 13, m[s[12]], CST256[s[13]]);
        g256_2(&mut v, 2, 7, 8, 13, m[s[13]], CST256[s[12]]);
        g256(&mut v, 3, 4, 9, 14, m[s[14]], CST256[s[15]]);
        g256_2(&mut v, 3, 4, 9, 14, m[s[15]], CST256[s[14]]);
    }
    for i in 0..8 {
        state.h[i] ^= v[i] ^ v[i + 8] ^ state.s[i % 4];
    }
}

fn blake512_compress(state: &mut BlakeState512, block: &[u8]) {
    let mut m = [0u64; 16];
    for i in 0..16 {
        m[i] = u8to64(&block[8 * i..]);
    }
    let mut v = [0u64; 16];
    for i in 0..8 {
        v[i] = state.h[i];
    }
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
    fn g512(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, mi: u64, ki: u64) {
        v[a] = v[a].wrapping_add(v[b]).wrapping_add(mi ^ ki);
        v[d] = rot512(v[d] ^ v[a], 32);
        v[c] = v[c].wrapping_add(v[d]);
        v[b] = rot512(v[b] ^ v[c], 25);
    }
    fn g512_2(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, mi: u64, ki: u64) {
        v[a] = v[a].wrapping_add(v[b]).wrapping_add(mi ^ ki);
        v[d] = rot512(v[d] ^ v[a], 16);
        v[c] = v[c].wrapping_add(v[d]);
        v[b] = rot512(v[b] ^ v[c], 11);
    }
    for r in 0..16 {
        let s = SIGMA[r];
        g512(&mut v, 0, 4, 8, 12, m[s[0]], CST512[s[1]]);
        g512_2(&mut v, 0, 4, 8, 12, m[s[1]], CST512[s[0]]);
        g512(&mut v, 1, 5, 9, 13, m[s[2]], CST512[s[3]]);
        g512_2(&mut v, 1, 5, 9, 13, m[s[3]], CST512[s[2]]);
        g512(&mut v, 2, 6, 10, 14, m[s[4]], CST512[s[5]]);
        g512_2(&mut v, 2, 6, 10, 14, m[s[5]], CST512[s[4]]);
        g512(&mut v, 3, 7, 11, 15, m[s[6]], CST512[s[7]]);
        g512_2(&mut v, 3, 7, 11, 15, m[s[7]], CST512[s[6]]);
        g512(&mut v, 0, 5, 10, 15, m[s[8]], CST512[s[9]]);
        g512_2(&mut v, 0, 5, 10, 15, m[s[9]], CST512[s[8]]);
        g512(&mut v, 1, 6, 11, 12, m[s[10]], CST512[s[11]]);
        g512_2(&mut v, 1, 6, 11, 12, m[s[11]], CST512[s[10]]);
        g512(&mut v, 2, 7, 8, 13, m[s[12]], CST512[s[13]]);
        g512_2(&mut v, 2, 7, 8, 13, m[s[13]], CST512[s[12]]);
        g512(&mut v, 3, 4, 9, 14, m[s[14]], CST512[s[15]]);
        g512_2(&mut v, 3, 4, 9, 14, m[s[15]], CST512[s[14]]);
    }
    for i in 0..8 {
        state.h[i] ^= v[i] ^ v[i + 8] ^ state.s[i % 4];
    }
}

// blake256_update: datalen is in bits.
pub fn blake256_update(s: &mut BlakeState256, data: &[u8], mut datalen: u64) {
    let mut left: i32 = s.buflen >> 3;
    let mut fill: i32 = 64 - left;
    let mut data_idx: usize = 0;

    if left != 0 && (((datalen >> 3) & 0x3F) >= fill as u64) {
        s.buf[left as usize..(left + fill) as usize]
            .copy_from_slice(&data[data_idx..data_idx + fill as usize]);
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 {
            s.t[1] = s.t[1].wrapping_add(1);
        }
        let buf_copy = s.buf;
        blake256_compress(s, &buf_copy);
        data_idx += fill as usize;
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 512 {
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 {
            s.t[1] = s.t[1].wrapping_add(1);
        }
        let block: [u8; 64] = data[data_idx..data_idx + 64].try_into().unwrap();
        blake256_compress(s, &block);
        data_idx += 64;
        datalen -= 512;
    }

    if datalen > 0 {
        let nbytes = (datalen >> 3) as usize;
        s.buf[left as usize..left as usize + nbytes]
            .copy_from_slice(&data[data_idx..data_idx + nbytes]);
        s.buflen = (left << 3) + datalen as i32;
    } else {
        s.buflen = 0;
    }

    let _ = fill;
}

pub fn blake256_final(s: &mut BlakeState256, digest: &mut [u8]) {
    let mut msglen = [0u8; 8];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo = s.t[0].wrapping_add(s.buflen as u32);
    let mut hi = s.t[1];
    if lo < s.buflen as u32 {
        hi = hi.wrapping_add(1);
    }
    u32to8(&mut msglen[0..4], hi);
    u32to8(&mut msglen[4..8], lo);

    if s.buflen == 440 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake256_update(s, &[oo], 8);
    } else {
        if s.buflen < 440 {
            if s.buflen == 0 {
                s.nullt = 1;
            }
            let n = (440 - s.buflen) as u64;
            s.t[0] = s.t[0].wrapping_sub(440 - s.buflen as u32);
            blake256_update(s, &PADDING[..n as usize / 8 + if n % 8 != 0 { 1 } else { 0 }], n);
        } else {
            let n1 = (512 - s.buflen) as u64;
            s.t[0] = s.t[0].wrapping_sub(512 - s.buflen as u32);
            blake256_update(s, &PADDING[..n1 as usize / 8 + if n1 % 8 != 0 { 1 } else { 0 }], n1);
            s.t[0] = s.t[0].wrapping_sub(440);
            blake256_update(s, &PADDING[1..56], 440);
            s.nullt = 1;
        }
        blake256_update(s, &[zo], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(64);
    blake256_update(s, &msglen, 64);

    for i in 0..8 {
        u32to8(&mut digest[4 * i..4 * i + 4], s.h[i]);
    }
}

pub fn blake512_update(s: &mut BlakeState512, data: &[u8], mut datalen: u64) {
    let mut left: i32 = s.buflen >> 3;
    let fill: i32 = 128 - left;
    let mut data_idx: usize = 0;

    if left != 0 && (((datalen >> 3) & 0x7F) >= fill as u64) {
        s.buf[left as usize..(left + fill) as usize]
            .copy_from_slice(&data[data_idx..data_idx + fill as usize]);
        s.t[0] = s.t[0].wrapping_add(1024);
        let buf_copy = s.buf;
        blake512_compress(s, &buf_copy);
        data_idx += fill as usize;
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 1024 {
        s.t[0] = s.t[0].wrapping_add(1024);
        let block: [u8; 128] = data[data_idx..data_idx + 128].try_into().unwrap();
        blake512_compress(s, &block);
        data_idx += 128;
        datalen -= 1024;
    }

    if datalen > 0 {
        let nbytes = ((datalen >> 3) & 0x7F) as usize;
        s.buf[left as usize..left as usize + nbytes]
            .copy_from_slice(&data[data_idx..data_idx + nbytes]);
        s.buflen = (left << 3) + datalen as i32;
    } else {
        s.buflen = 0;
    }
}

pub fn blake512_final(s: &mut BlakeState512, digest: &mut [u8]) {
    let mut msglen = [0u8; 16];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo = s.t[0].wrapping_add(s.buflen as u64);
    let mut hi = s.t[1];
    if lo < s.buflen as u64 {
        hi = hi.wrapping_add(1);
    }
    u64to8(&mut msglen[0..8], hi);
    u64to8(&mut msglen[8..16], lo);

    if s.buflen == 888 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake512_update(s, &[oo], 8);
    } else {
        if s.buflen < 888 {
            if s.buflen == 0 {
                s.nullt = 1;
            }
            let n = (888 - s.buflen) as u64;
            s.t[0] = s.t[0].wrapping_sub(888 - s.buflen as u64);
            let nbytes = (n >> 3) as usize;
            blake512_update(s, &PADDING[..nbytes + if n % 8 != 0 { 1 } else { 0 }], n);
        } else {
            let n1 = (1024 - s.buflen) as u64;
            s.t[0] = s.t[0].wrapping_sub(1024 - s.buflen as u64);
            let nbytes1 = (n1 >> 3) as usize;
            blake512_update(s, &PADDING[..nbytes1 + if n1 % 8 != 0 { 1 } else { 0 }], n1);
            s.t[0] = s.t[0].wrapping_sub(888);
            blake512_update(s, &PADDING[1..1 + 111], 888);
            s.nullt = 1;
        }
        blake512_update(s, &[zo], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(128);
    blake512_update(s, &msglen, 128);

    for i in 0..8 {
        u64to8(&mut digest[8 * i..8 * i + 8], s.h[i]);
    }
}

pub fn blake256(out: &mut [u8], inp: &[u8]) {
    let mut s = BlakeState256 {
        h: [0; 8],
        s: [0; 4],
        t: [0; 2],
        buflen: 0,
        nullt: 0,
        buf: [0u8; 64],
    };
    blake256_init(&mut s);
    blake256_update(&mut s, inp, (inp.len() as u64) * 8);
    blake256_final(&mut s, out);
}

pub fn blake512(out: &mut [u8], inp: &[u8]) {
    let mut s = BlakeState512 {
        h: [0; 8],
        s: [0; 4],
        t: [0; 2],
        buflen: 0,
        nullt: 0,
        buf: [0u8; 128],
    };
    blake512_init(&mut s);
    blake512_update(&mut s, inp, (inp.len() as u64) * 8);
    blake512_final(&mut s, out);
}

pub fn blake256_mgf1(out: &mut [u8], outlen: usize, inp: &[u8]) {
    let inlen = inp.len();
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    inbuf[..inlen].copy_from_slice(inp);
    let mut i: usize = 0;
    let mut written = 0;
    while (i + 1) * SPX_BLAKE256_OUTPUT_BYTES <= outlen {
        u32_to_bytes_rs(&mut inbuf[inlen..inlen + 4], i as u32);
        blake256(&mut out[written..written + 32], &inbuf);
        written += SPX_BLAKE256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_BLAKE256_OUTPUT_BYTES {
        u32_to_bytes_rs(&mut inbuf[inlen..inlen + 4], i as u32);
        blake256(&mut outbuf, &inbuf);
        let rem = outlen - i * SPX_BLAKE256_OUTPUT_BYTES;
        out[written..written + rem].copy_from_slice(&outbuf[..rem]);
    }
}

pub fn blake512_mgf1(out: &mut [u8], outlen: usize, inp: &[u8]) {
    let inlen = inp.len();
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    inbuf[..inlen].copy_from_slice(inp);
    let mut i: usize = 0;
    let mut written = 0;
    while (i + 1) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        u32_to_bytes_rs(&mut inbuf[inlen..inlen + 4], i as u32);
        blake512(&mut out[written..written + 64], &inbuf);
        written += SPX_BLAKE512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_BLAKE512_OUTPUT_BYTES {
        u32_to_bytes_rs(&mut inbuf[inlen..inlen + 4], i as u32);
        blake512(&mut outbuf, &inbuf);
        let rem = outlen - i * SPX_BLAKE512_OUTPUT_BYTES;
        out[written..written + rem].copy_from_slice(&outbuf[..rem]);
    }
}

// =========================
// Backend impls
// =========================

#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
const BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE512_OUTPUT_BYTES;
#[cfg(any(feature = "128s", feature = "128f"))]
const BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE256_OUTPUT_BYTES;

fn blakeX(out: &mut [u8], inp: &[u8]) {
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    blake512(out, inp);
    #[cfg(any(feature = "128s", feature = "128f"))]
    blake256(out, inp);
}

fn blakeX_mgf1(out: &mut [u8], outlen: usize, inp: &[u8]) {
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    blake512_mgf1(out, outlen, inp);
    #[cfg(any(feature = "128s", feature = "128f"))]
    blake256_mgf1(out, outlen, inp);
}

pub fn initialize_hash_function_impl(_ctx: &mut SpxCtx) {}

pub fn prf_addr_impl(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = vec![0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = addr_to_bytes(addr);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..2 * SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ctx.sk_seed);

    // Note: matches C code which uses inlen = SPX_N + SPX_ADDR_BYTES (NOT the full buffer)
    blake256(&mut outbuf, &buf[..SPX_N + SPX_ADDR_BYTES]);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

pub fn gen_message_random_impl(
    R: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    _ctx: &SpxCtx,
) {
    // Reproduce a quirk in the reference C: blake_update is documented as
    // taking the data length in *bits*, but gen_message_random passes a
    // *byte* count. We mirror that exactly so output bytes match.
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        let mut s = BlakeState512 {
            h: [0; 8],
            s: [0; 4],
            t: [0; 2],
            buflen: 0,
            nullt: 0,
            buf: [0u8; 128],
        };
        blake512_init(&mut s);
        blake512_update(&mut s, sk_prf, sk_prf.len() as u64);
        blake512_update(&mut s, optrand, optrand.len() as u64);
        blake512_update(&mut s, m, m.len() as u64);
        let mut full = [0u8; 64];
        blake512_final(&mut s, &mut full);
        R[..SPX_N].copy_from_slice(&full[..SPX_N]);
    }
    #[cfg(any(feature = "128s", feature = "128f"))]
    {
        let mut s = BlakeState256 {
            h: [0; 8],
            s: [0; 4],
            t: [0; 2],
            buflen: 0,
            nullt: 0,
            buf: [0u8; 64],
        };
        blake256_init(&mut s);
        blake256_update(&mut s, sk_prf, sk_prf.len() as u64);
        blake256_update(&mut s, optrand, optrand.len() as u64);
        blake256_update(&mut s, m, m.len() as u64);
        let mut full = [0u8; 32];
        blake256_final(&mut s, &mut full);
        R[..SPX_N].copy_from_slice(&full[..SPX_N]);
    }
}

pub fn hash_message_impl(
    digest: &mut [u8],
    R: &[u8],
    pk: &[u8],
    m: &[u8],
    _ctx: &SpxCtx,
) -> (u64, u32) {
    let spx_tree_bits = SPX_TREE_HEIGHT * (SPX_D - 1);
    let spx_tree_bytes = (spx_tree_bits + 7) / 8;
    let spx_leaf_bits = SPX_TREE_HEIGHT;
    let spx_leaf_bytes = (spx_leaf_bits + 7) / 8;
    let spx_dgst_bytes = SPX_FORS_MSG_BYTES + spx_tree_bytes + spx_leaf_bytes;

    let mut buf = vec![0u8; spx_dgst_bytes];
    let mut seed = vec![0u8; 2 * SPX_N + BLAKEX_OUTPUT_BYTES];

    // Reproduce the same byte-count-as-bit-count quirk from the reference C.
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        let mut s = BlakeState512 {
            h: [0; 8],
            s: [0; 4],
            t: [0; 2],
            buflen: 0,
            nullt: 0,
            buf: [0u8; 128],
        };
        blake512_init(&mut s);
        blake512_update(&mut s, R, R.len() as u64);
        blake512_update(&mut s, pk, pk.len() as u64);
        blake512_update(&mut s, m, m.len() as u64);
        blake512_final(&mut s, &mut seed[2 * SPX_N..]);
    }
    #[cfg(any(feature = "128s", feature = "128f"))]
    {
        let mut s = BlakeState256 {
            h: [0; 8],
            s: [0; 4],
            t: [0; 2],
            buflen: 0,
            nullt: 0,
            buf: [0u8; 64],
        };
        blake256_init(&mut s);
        blake256_update(&mut s, R, R.len() as u64);
        blake256_update(&mut s, pk, pk.len() as u64);
        blake256_update(&mut s, m, m.len() as u64);
        blake256_final(&mut s, &mut seed[2 * SPX_N..]);
    }

    seed[..SPX_N].copy_from_slice(R);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    blakeX_mgf1(&mut buf, spx_dgst_bytes, &seed);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    let tree = if SPX_D == 1 {
        0u64
    } else {
        let mut t = bytes_to_ull_rs(&buf[bufp..bufp + spx_tree_bytes]);
        t &= !0u64 >> (64 - spx_tree_bits);
        t
    };
    bufp += spx_tree_bytes;
    let mut leaf_idx = bytes_to_ull_rs(&buf[bufp..bufp + spx_leaf_bytes]) as u32;
    leaf_idx &= !0u32 >> (32 - spx_leaf_bits);

    (tree, leaf_idx)
}

pub fn thash_impl(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    #[cfg(feature = "robust")]
    {
        thash_robust(out, inp, inblocks, ctx, addr);
    }
    #[cfg(feature = "simple")]
    {
        thash_simple(out, inp, inblocks, ctx, addr);
    }
}

#[cfg(feature = "robust")]
fn thash_robust(
    out: &mut [u8],
    inp: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let inblocks_us = inblocks as usize;

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    if inblocks > 1 {
        thash_robust_512(out, inp, inblocks, ctx, addr);
        return;
    }

    let mut bitmask = vec![0u8; inblocks_us * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks_us * SPX_N];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = addr_to_bytes(addr);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes);
    let buf_clone: Vec<u8> = buf[..SPX_N + SPX_ADDR_BYTES].to_vec();
    blake256_mgf1(&mut bitmask, inblocks_us * SPX_N, &buf_clone);
    for i in 0..inblocks_us * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    // C: blake256(outbuf, buf + SPX_N, SPX_ADDR_BYTES + inblocks*SPX_N);
    blake256(&mut outbuf, &buf[SPX_N..SPX_N + SPX_ADDR_BYTES + inblocks_us * SPX_N]);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(feature = "robust", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
fn thash_robust_512(
    out: &mut [u8],
    inp: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let inblocks_us = inblocks as usize;
    let mut bitmask = vec![0u8; inblocks_us * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks_us * SPX_N];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = addr_to_bytes(addr);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes);
    let buf_clone: Vec<u8> = buf[..SPX_N + SPX_ADDR_BYTES].to_vec();
    blake512_mgf1(&mut bitmask, inblocks_us * SPX_N, &buf_clone);
    for i in 0..inblocks_us * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    blake512(&mut outbuf, &buf[SPX_N..SPX_N + SPX_ADDR_BYTES + inblocks_us * SPX_N]);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(feature = "simple")]
fn thash_simple(
    out: &mut [u8],
    inp: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let inblocks_us = inblocks as usize;

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    if inblocks > 1 {
        thash_simple_512(out, inp, inblocks, ctx, addr);
        return;
    }

    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks_us * SPX_N];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = addr_to_bytes(addr);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&inp[..inblocks_us * SPX_N]);
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    blake256(&mut outbuf, &buf);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(feature = "simple", any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
fn thash_simple_512(
    out: &mut [u8],
    inp: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let inblocks_us = inblocks as usize;
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks_us * SPX_N];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = addr_to_bytes(addr);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&inp[..inblocks_us * SPX_N]);
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    blake512(&mut outbuf, &buf);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// FFI exports for the BLAKE state functions used in the driver's KAT transcript

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake256_init_c(s: *mut BlakeState256) {
    blake256_init(unsafe { &mut *s });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake256_update_c(s: *mut BlakeState256, inp: *const u8, inlen: u64) {
    let in_slice = unsafe { core::slice::from_raw_parts(inp, inlen as usize) };
    blake256_update(unsafe { &mut *s }, in_slice, inlen * 8);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake256_final_c(s: *mut BlakeState256, out: *mut u8) {
    let out_slice = unsafe { core::slice::from_raw_parts_mut(out, 32) };
    blake256_final(unsafe { &mut *s }, out_slice);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_init_c(s: *mut BlakeState512) {
    blake512_init(unsafe { &mut *s });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_update_c(s: *mut BlakeState512, inp: *const u8, inlen: u64) {
    let in_slice = unsafe { core::slice::from_raw_parts(inp, inlen as usize) };
    blake512_update(unsafe { &mut *s }, in_slice, inlen * 8);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_final_c(s: *mut BlakeState512, out: *mut u8) {
    let out_slice = unsafe { core::slice::from_raw_parts_mut(out, 64) };
    blake512_final(unsafe { &mut *s }, out_slice);
}
