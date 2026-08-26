// BLAKE-256 / BLAKE-512 implementation, ported from c_src/lib/blake/src/blake256.c, blake512.c
#![allow(dead_code, non_snake_case)]

use crate::utils::u32_to_bytes;

pub const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;
pub const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

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

impl BlakeState256 {
    pub fn new() -> Self {
        Self {
            h: [0; 8],
            s: [0; 4],
            t: [0; 2],
            buflen: 0,
            nullt: 0,
            buf: [0; 64],
        }
    }
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

impl BlakeState512 {
    pub fn new() -> Self {
        Self {
            h: [0; 8],
            s: [0; 4],
            t: [0; 2],
            buflen: 0,
            nullt: 0,
            buf: [0; 128],
        }
    }
}

#[inline]
fn u8to32(p: &[u8]) -> u32 {
    ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32)
}
#[inline]
fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8;
    p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8;
    p[3] = v as u8;
}
#[inline]
fn u8to64(p: &[u8]) -> u64 {
    ((u8to32(p) as u64) << 32) | (u8to32(&p[4..]) as u64)
}
#[inline]
fn u64to8(p: &mut [u8], v: u64) {
    u32to8(p, (v >> 32) as u32);
    u32to8(&mut p[4..], v as u32);
}

const CST_256: [u32; 16] = [
    0x243F6A88, 0x85A308D3, 0x13198A2E, 0x03707344, 0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    0x452821E6, 0x38D01377, 0xBE5466CF, 0x34E90C6C, 0xC0AC29B7, 0xC97C50DD, 0x3F84D5B5, 0xB5470917,
];

const CST_512: [u64; 16] = [
    0x243F6A8885A308D3, 0x13198A2E03707344, 0xA4093822299F31D0, 0x082EFA98EC4E6C89,
    0x452821E638D01377, 0xBE5466CF34E90C6C, 0xC0AC29B7C97C50DD, 0x3F84D5B5B5470917,
    0x9216D5D98979FB1B, 0xD1310BA698DFB5AC, 0x2FFD72DBD01ADFB7, 0xB8E1AFED6A267E96,
    0xBA7C9045F12C7F99, 0x24A19947B3916CF7, 0x0801F2E2858EFC16, 0x636920D871574E69,
];

const PADDING_256: [u8; 64] = [
    0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

const PADDING_512: [u8; 129] = [
    0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0,
];

#[inline]
fn rot32(x: u32, n: u32) -> u32 {
    // C macro is: ((x<<(32-n))|(x>>n))  — that is, rotate-left by (32-n)
    // which is rotate-right by n
    x.rotate_right(n)
}

#[inline]
fn rot64(x: u64, n: u32) -> u64 {
    x.rotate_right(n)
}

pub fn blake256_init(s: &mut BlakeState256) {
    s.h[0] = 0x6A09E667;
    s.h[1] = 0xBB67AE85;
    s.h[2] = 0x3C6EF372;
    s.h[3] = 0xA54FF53A;
    s.h[4] = 0x510E527F;
    s.h[5] = 0x9B05688C;
    s.h[6] = 0x1F83D9AB;
    s.h[7] = 0x5BE0CD19;
    s.t[0] = 0;
    s.t[1] = 0;
    s.buflen = 0;
    s.nullt = 0;
    s.s[0] = 0;
    s.s[1] = 0;
    s.s[2] = 0;
    s.s[3] = 0;
}

pub fn blake256_compress(s: &mut BlakeState256, block: &[u8]) {
    let m: [u32; 16] = [
        u8to32(&block[0..]),
        u8to32(&block[4..]),
        u8to32(&block[8..]),
        u8to32(&block[12..]),
        u8to32(&block[16..]),
        u8to32(&block[20..]),
        u8to32(&block[24..]),
        u8to32(&block[28..]),
        u8to32(&block[32..]),
        u8to32(&block[36..]),
        u8to32(&block[40..]),
        u8to32(&block[44..]),
        u8to32(&block[48..]),
        u8to32(&block[52..]),
        u8to32(&block[56..]),
        u8to32(&block[60..]),
    ];
    let mut v = [0u32; 16];
    v[0] = s.h[0];
    v[1] = s.h[1];
    v[2] = s.h[2];
    v[3] = s.h[3];
    v[4] = s.h[4];
    v[5] = s.h[5];
    v[6] = s.h[6];
    v[7] = s.h[7];
    v[8] = s.s[0] ^ 0x243F6A88;
    v[9] = s.s[1] ^ 0x85A308D3;
    v[10] = s.s[2] ^ 0x13198A2E;
    v[11] = s.s[3] ^ 0x03707344;
    v[12] = 0xA4093822;
    v[13] = 0x299F31D0;
    v[14] = 0x082EFA98;
    v[15] = 0xEC4E6C89;
    if s.nullt == 0 {
        v[12] ^= s.t[0];
        v[13] ^= s.t[0];
        v[14] ^= s.t[1];
        v[15] ^= s.t[1];
    }

    // Define G - using full BLAKE-256 round
    macro_rules! g {
        ($v:expr, $a:expr, $b:expr, $c:expr, $d:expr, $mx:expr, $cx:expr, $my:expr, $cy:expr) => {{
            $v[$a] = $v[$a].wrapping_add($v[$b]).wrapping_add($mx ^ $cx);
            $v[$d] = rot32($v[$d] ^ $v[$a], 16);
            $v[$c] = $v[$c].wrapping_add($v[$d]);
            $v[$b] = rot32($v[$b] ^ $v[$c], 12);
            $v[$a] = $v[$a].wrapping_add($v[$b]).wrapping_add($my ^ $cy);
            $v[$d] = rot32($v[$d] ^ $v[$a], 8);
            $v[$c] = $v[$c].wrapping_add($v[$d]);
            $v[$b] = rot32($v[$b] ^ $v[$c], 7);
        }};
    }

    let sigma: [[usize; 16]; 14] = [
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
        [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
        [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
        [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
        [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
        [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
        [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
        [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
        [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
        [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
        [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    ];

    for i in 0..14 {
        let sg = &sigma[i];
        g!(v, 0, 4, 8, 12, m[sg[0]], CST_256[sg[1]], m[sg[1]], CST_256[sg[0]]);
        g!(v, 1, 5, 9, 13, m[sg[2]], CST_256[sg[3]], m[sg[3]], CST_256[sg[2]]);
        g!(v, 2, 6, 10, 14, m[sg[4]], CST_256[sg[5]], m[sg[5]], CST_256[sg[4]]);
        g!(v, 3, 7, 11, 15, m[sg[6]], CST_256[sg[7]], m[sg[7]], CST_256[sg[6]]);
        g!(v, 0, 5, 10, 15, m[sg[8]], CST_256[sg[9]], m[sg[9]], CST_256[sg[8]]);
        g!(v, 1, 6, 11, 12, m[sg[10]], CST_256[sg[11]], m[sg[11]], CST_256[sg[10]]);
        g!(v, 2, 7, 8, 13, m[sg[12]], CST_256[sg[13]], m[sg[13]], CST_256[sg[12]]);
        g!(v, 3, 4, 9, 14, m[sg[14]], CST_256[sg[15]], m[sg[15]], CST_256[sg[14]]);
    }

    s.h[0] ^= v[0] ^ v[8] ^ s.s[0];
    s.h[1] ^= v[1] ^ v[9] ^ s.s[1];
    s.h[2] ^= v[2] ^ v[10] ^ s.s[2];
    s.h[3] ^= v[3] ^ v[11] ^ s.s[3];
    s.h[4] ^= v[4] ^ v[12] ^ s.s[0];
    s.h[5] ^= v[5] ^ v[13] ^ s.s[1];
    s.h[6] ^= v[6] ^ v[14] ^ s.s[2];
    s.h[7] ^= v[7] ^ v[15] ^ s.s[3];
}

pub fn blake256_update(s: &mut BlakeState256, data: &[u8], mut datalen: u64) {
    // datalen is bit length
    let mut left = (s.buflen >> 3) as usize;
    let fill = 64 - left;
    let mut data_off = 0;

    if left != 0 && (((datalen >> 3) & 0x3F) >= fill as u64) {
        s.buf[left..left + fill].copy_from_slice(&data[data_off..data_off + fill]);
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 {
            s.t[1] = s.t[1].wrapping_add(1);
        }
        let buf_copy = s.buf;
        blake256_compress(s, &buf_copy);
        data_off += fill;
        datalen -= (fill << 3) as u64;
        left = 0;
    }

    while datalen >= 512 {
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 {
            s.t[1] = s.t[1].wrapping_add(1);
        }
        blake256_compress(s, &data[data_off..data_off + 64]);
        data_off += 64;
        datalen -= 512;
    }

    if datalen > 0 {
        let n = (datalen >> 3) as usize;
        s.buf[left..left + n].copy_from_slice(&data[data_off..data_off + n]);
        s.buflen = ((left << 3) as u64 + datalen) as i32;
    } else {
        s.buflen = 0;
    }
}

pub fn blake256_final(s: &mut BlakeState256, digest: &mut [u8]) {
    let mut msglen = [0u8; 8];
    let zo = [0x01u8];
    let oo = [0x81u8];
    let lo = s.t[0].wrapping_add(s.buflen as u32);
    let mut hi = s.t[1];
    if lo < s.buflen as u32 {
        hi = hi.wrapping_add(1);
    }
    u32to8(&mut msglen[0..], hi);
    u32to8(&mut msglen[4..], lo);

    if s.buflen == 440 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake256_update(s, &oo, 8);
    } else {
        if s.buflen < 440 {
            if s.buflen == 0 {
                s.nullt = 1;
            }
            s.t[0] = s.t[0].wrapping_sub((440 - s.buflen) as u32);
            blake256_update(s, &PADDING_256, (440 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((512 - s.buflen) as u32);
            blake256_update(s, &PADDING_256, (512 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(440);
            blake256_update(s, &PADDING_256[1..], 440);
            s.nullt = 1;
        }
        blake256_update(s, &zo, 8);
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

pub fn blake256(out: &mut [u8], input: &[u8], inlen: u64) {
    let mut s = BlakeState256::new();
    blake256_init(&mut s);
    blake256_update(&mut s, input, inlen * 8);
    blake256_final(&mut s, out);
}

pub fn blake256_mgf1(out: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&input[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut i: u32 = 0;
    let mut out_off = 0usize;
    while ((i as usize) + 1) * SPX_BLAKE256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..inlen + 4], i);
        blake256(
            &mut out[out_off..out_off + SPX_BLAKE256_OUTPUT_BYTES],
            &inbuf,
            (inlen + 4) as u64,
        );
        out_off += SPX_BLAKE256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > (i as usize) * SPX_BLAKE256_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..inlen + 4], i);
        blake256(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - (i as usize) * SPX_BLAKE256_OUTPUT_BYTES;
        out[out_off..out_off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

// ===== BLAKE-512 =====

pub fn blake512_init(s: &mut BlakeState512) {
    s.h[0] = 0x6A09E667F3BCC908;
    s.h[1] = 0xBB67AE8584CAA73B;
    s.h[2] = 0x3C6EF372FE94F82B;
    s.h[3] = 0xA54FF53A5F1D36F1;
    s.h[4] = 0x510E527FADE682D1;
    s.h[5] = 0x9B05688C2B3E6C1F;
    s.h[6] = 0x1F83D9ABFB41BD6B;
    s.h[7] = 0x5BE0CD19137E2179;
    s.t[0] = 0;
    s.t[1] = 0;
    s.buflen = 0;
    s.nullt = 0;
    s.s[0] = 0;
    s.s[1] = 0;
    s.s[2] = 0;
    s.s[3] = 0;
}

pub fn blake512_compress(s: &mut BlakeState512, block: &[u8]) {
    let m: [u64; 16] = [
        u8to64(&block[0..]),
        u8to64(&block[8..]),
        u8to64(&block[16..]),
        u8to64(&block[24..]),
        u8to64(&block[32..]),
        u8to64(&block[40..]),
        u8to64(&block[48..]),
        u8to64(&block[56..]),
        u8to64(&block[64..]),
        u8to64(&block[72..]),
        u8to64(&block[80..]),
        u8to64(&block[88..]),
        u8to64(&block[96..]),
        u8to64(&block[104..]),
        u8to64(&block[112..]),
        u8to64(&block[120..]),
    ];
    let mut v = [0u64; 16];
    v[0] = s.h[0];
    v[1] = s.h[1];
    v[2] = s.h[2];
    v[3] = s.h[3];
    v[4] = s.h[4];
    v[5] = s.h[5];
    v[6] = s.h[6];
    v[7] = s.h[7];
    v[8] = s.s[0] ^ 0x243F6A8885A308D3;
    v[9] = s.s[1] ^ 0x13198A2E03707344;
    v[10] = s.s[2] ^ 0xA4093822299F31D0;
    v[11] = s.s[3] ^ 0x082EFA98EC4E6C89;
    v[12] = 0x452821E638D01377;
    v[13] = 0xBE5466CF34E90C6C;
    v[14] = 0xC0AC29B7C97C50DD;
    v[15] = 0x3F84D5B5B5470917;
    if s.nullt == 0 {
        v[12] ^= s.t[0];
        v[13] ^= s.t[0];
        v[14] ^= s.t[1];
        v[15] ^= s.t[1];
    }

    macro_rules! g512 {
        ($v:expr, $a:expr, $b:expr, $c:expr, $d:expr, $mx:expr, $cx:expr, $my:expr, $cy:expr) => {{
            $v[$a] = $v[$a].wrapping_add($v[$b]).wrapping_add($mx ^ $cx);
            $v[$d] = rot64($v[$d] ^ $v[$a], 32);
            $v[$c] = $v[$c].wrapping_add($v[$d]);
            $v[$b] = rot64($v[$b] ^ $v[$c], 25);
            $v[$a] = $v[$a].wrapping_add($v[$b]).wrapping_add($my ^ $cy);
            $v[$d] = rot64($v[$d] ^ $v[$a], 16);
            $v[$c] = $v[$c].wrapping_add($v[$d]);
            $v[$b] = rot64($v[$b] ^ $v[$c], 11);
        }};
    }

    let sigma: [[usize; 16]; 16] = [
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
        [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
        [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
        [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
        [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
        [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
        [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
        [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
        [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
        [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
        [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
        [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
        [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    ];

    for i in 0..16 {
        let sg = &sigma[i];
        g512!(v, 0, 4, 8, 12, m[sg[0]], CST_512[sg[1]], m[sg[1]], CST_512[sg[0]]);
        g512!(v, 1, 5, 9, 13, m[sg[2]], CST_512[sg[3]], m[sg[3]], CST_512[sg[2]]);
        g512!(v, 2, 6, 10, 14, m[sg[4]], CST_512[sg[5]], m[sg[5]], CST_512[sg[4]]);
        g512!(v, 3, 7, 11, 15, m[sg[6]], CST_512[sg[7]], m[sg[7]], CST_512[sg[6]]);
        g512!(v, 0, 5, 10, 15, m[sg[8]], CST_512[sg[9]], m[sg[9]], CST_512[sg[8]]);
        g512!(v, 1, 6, 11, 12, m[sg[10]], CST_512[sg[11]], m[sg[11]], CST_512[sg[10]]);
        g512!(v, 2, 7, 8, 13, m[sg[12]], CST_512[sg[13]], m[sg[13]], CST_512[sg[12]]);
        g512!(v, 3, 4, 9, 14, m[sg[14]], CST_512[sg[15]], m[sg[15]], CST_512[sg[14]]);
    }

    s.h[0] ^= v[0] ^ v[8] ^ s.s[0];
    s.h[1] ^= v[1] ^ v[9] ^ s.s[1];
    s.h[2] ^= v[2] ^ v[10] ^ s.s[2];
    s.h[3] ^= v[3] ^ v[11] ^ s.s[3];
    s.h[4] ^= v[4] ^ v[12] ^ s.s[0];
    s.h[5] ^= v[5] ^ v[13] ^ s.s[1];
    s.h[6] ^= v[6] ^ v[14] ^ s.s[2];
    s.h[7] ^= v[7] ^ v[15] ^ s.s[3];
}

pub fn blake512_update(s: &mut BlakeState512, data: &[u8], mut datalen: u64) {
    let mut left = (s.buflen >> 3) as usize;
    let fill = 128 - left;
    let mut data_off = 0;

    if left != 0 && (((datalen >> 3) & 0x7F) >= fill as u64) {
        s.buf[left..left + fill].copy_from_slice(&data[data_off..data_off + fill]);
        s.t[0] = s.t[0].wrapping_add(1024);
        let buf_copy = s.buf;
        blake512_compress(s, &buf_copy);
        data_off += fill;
        datalen -= (fill << 3) as u64;
        left = 0;
    }

    while datalen >= 1024 {
        s.t[0] = s.t[0].wrapping_add(1024);
        blake512_compress(s, &data[data_off..data_off + 128]);
        data_off += 128;
        datalen -= 1024;
    }

    if datalen > 0 {
        let n = ((datalen >> 3) & 0x7F) as usize;
        s.buf[left..left + n].copy_from_slice(&data[data_off..data_off + n]);
        s.buflen = ((left << 3) as u64 + datalen) as i32;
    } else {
        s.buflen = 0;
    }
}

pub fn blake512_final(s: &mut BlakeState512, digest: &mut [u8]) {
    let mut msglen = [0u8; 16];
    let zo = [0x01u8];
    let oo = [0x81u8];
    let lo = s.t[0].wrapping_add(s.buflen as u64);
    let mut hi = s.t[1];
    if lo < s.buflen as u64 {
        hi = hi.wrapping_add(1);
    }
    u64to8(&mut msglen[0..], hi);
    u64to8(&mut msglen[8..], lo);

    if s.buflen == 888 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake512_update(s, &oo, 8);
    } else {
        if s.buflen < 888 {
            if s.buflen == 0 {
                s.nullt = 1;
            }
            s.t[0] = s.t[0].wrapping_sub((888 - s.buflen) as u64);
            blake512_update(s, &PADDING_512, (888 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((1024 - s.buflen) as u64);
            blake512_update(s, &PADDING_512, (1024 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(888);
            blake512_update(s, &PADDING_512[1..], 888);
            s.nullt = 1;
        }
        blake512_update(s, &zo, 8);
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

pub fn blake512(out: &mut [u8], input: &[u8], inlen: u64) {
    let mut s = BlakeState512::new();
    blake512_init(&mut s);
    blake512_update(&mut s, input, inlen * 8);
    blake512_final(&mut s, out);
}

pub fn blake512_mgf1(out: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&input[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut i: u32 = 0;
    let mut out_off = 0usize;
    while ((i as usize) + 1) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..inlen + 4], i);
        blake512(
            &mut out[out_off..out_off + SPX_BLAKE512_OUTPUT_BYTES],
            &inbuf,
            (inlen + 4) as u64,
        );
        out_off += SPX_BLAKE512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > (i as usize) * SPX_BLAKE512_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..inlen + 4], i);
        blake512(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - (i as usize) * SPX_BLAKE512_OUTPUT_BYTES;
        out[out_off..out_off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

// C-ABI exports
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake256_init_c(s: *mut BlakeState256) {
    blake256_init(unsafe { &mut *s });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake256_update_c(s: *mut BlakeState256, data: *const u8, datalen: u64) {
    let r = unsafe { &mut *s };
    let bits = datalen;
    let bytes = (bits >> 3) as usize;
    let extra = (bits & 7) != 0;
    let len = bytes + if extra { 1 } else { 0 };
    let d = unsafe { std::slice::from_raw_parts(data, len) };
    blake256_update(r, d, datalen);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake256_final_c(s: *mut BlakeState256, digest: *mut u8) {
    let r = unsafe { &mut *s };
    let d = unsafe { std::slice::from_raw_parts_mut(digest, 32) };
    blake256_final(r, d);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake256_c(out: *mut u8, input: *const u8, inlen: u64) -> i32 {
    let o = unsafe { std::slice::from_raw_parts_mut(out, 32) };
    let i = unsafe { std::slice::from_raw_parts(input, inlen as usize) };
    blake256(o, i, inlen);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_init_c(s: *mut BlakeState512) {
    blake512_init(unsafe { &mut *s });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_update_c(s: *mut BlakeState512, data: *const u8, datalen: u64) {
    let r = unsafe { &mut *s };
    let bits = datalen;
    let bytes = (bits >> 3) as usize;
    let extra = (bits & 7) != 0;
    let len = bytes + if extra { 1 } else { 0 };
    let d = unsafe { std::slice::from_raw_parts(data, len) };
    blake512_update(r, d, datalen);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_final_c(s: *mut BlakeState512, digest: *mut u8) {
    let r = unsafe { &mut *s };
    let d = unsafe { std::slice::from_raw_parts_mut(digest, 64) };
    blake512_final(r, d);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_c(out: *mut u8, input: *const u8, inlen: u64) -> i32 {
    let o = unsafe { std::slice::from_raw_parts_mut(out, 64) };
    let i = unsafe { std::slice::from_raw_parts(input, inlen as usize) };
    blake512(o, i, inlen);
    0
}
