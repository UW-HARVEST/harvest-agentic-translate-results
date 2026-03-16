use crate::params::SPX_BLAKE512_OUTPUT_BYTES;
use crate::address::u32_to_bytes;

pub struct BlakeState512 {
    pub h: [u64; 8],
    pub s: [u64; 4],
    pub t: [u64; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 128],
}

// Note: in the C code this is non-static (global). We replicate as a module-level const.
static CST: [u64; 16] = [
    0x243F6A8885A308D3, 0x13198A2E03707344, 0xA4093822299F31D0, 0x082EFA98EC4E6C89,
    0x452821E638D01377, 0xBE5466CF34E90C6C, 0xC0AC29B7C97C50DD, 0x3F84D5B5B5470917,
    0x9216D5D98979FB1B, 0xD1310BA698DFB5AC, 0x2FFD72DBD01ADFB7, 0xB8E1AFED6A267E96,
    0xBA7C9045F12C7F99, 0x24A19947B3916CF7, 0x0801F2E2858EFC16, 0x636920D871574E69,
];

static PADDING: [u8; 129] = [
    0x80,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
];

static SIGMA: [[usize; 16]; 10] = [
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
];

fn u8to32(p: &[u8]) -> u32 {
    ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32)
}

fn u8to64(p: &[u8]) -> u64 {
    ((u8to32(p) as u64) << 32) | (u8to32(&p[4..]) as u64)
}

fn u64to8(p: &mut [u8], v: u64) {
    let hi = (v >> 32) as u32;
    let lo = v as u32;
    p[0] = (hi >> 24) as u8; p[1] = (hi >> 16) as u8;
    p[2] = (hi >> 8) as u8; p[3] = hi as u8;
    p[4] = (lo >> 24) as u8; p[5] = (lo >> 16) as u8;
    p[6] = (lo >> 8) as u8; p[7] = lo as u8;
}

#[inline(always)]
fn g512(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize,
        mx: u64, cx: u64, my: u64, cy: u64) {
    v[a] = v[a].wrapping_add(mx ^ cx).wrapping_add(v[b]);
    v[d] ^= v[a];
    v[d] = v[d].rotate_right(32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] ^= v[c];
    v[b] = v[b].rotate_right(25);
    v[a] = v[a].wrapping_add(my ^ cy).wrapping_add(v[b]);
    v[d] ^= v[a];
    v[d] = v[d].rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] ^= v[c];
    v[b] = v[b].rotate_right(11);
}

pub fn blake512_compress(s: &mut BlakeState512, block: &[u8]) {
    let m: [u64; 16] = [
        u8to64(&block[0..]), u8to64(&block[8..]), u8to64(&block[16..]), u8to64(&block[24..]),
        u8to64(&block[32..]), u8to64(&block[40..]), u8to64(&block[48..]), u8to64(&block[56..]),
        u8to64(&block[64..]), u8to64(&block[72..]), u8to64(&block[80..]), u8to64(&block[88..]),
        u8to64(&block[96..]), u8to64(&block[104..]), u8to64(&block[112..]), u8to64(&block[120..]),
    ];

    let mut v: [u64; 16] = [
        s.h[0], s.h[1], s.h[2], s.h[3],
        s.h[4], s.h[5], s.h[6], s.h[7],
        s.s[0] ^ 0x243F6A8885A308D3, s.s[1] ^ 0x13198A2E03707344,
        s.s[2] ^ 0xA4093822299F31D0, s.s[3] ^ 0x082EFA98EC4E6C89,
        0x452821E638D01377, 0xBE5466CF34E90C6C,
        0xC0AC29B7C97C50DD, 0x3F84D5B5B5470917,
    ];

    if s.nullt == 0 {
        v[12] ^= s.t[0];
        v[13] ^= s.t[0];
        v[14] ^= s.t[1];
        v[15] ^= s.t[1];
    }

    // 16 rounds for BLAKE-512 (sigma wraps around after 10)
    for round in 0..16 {
        let sg = &SIGMA[round % 10];
        g512(&mut v, 0, 4, 8, 12, m[sg[0]], CST[sg[1]], m[sg[1]], CST[sg[0]]);
        g512(&mut v, 1, 5, 9, 13, m[sg[2]], CST[sg[3]], m[sg[3]], CST[sg[2]]);
        g512(&mut v, 2, 6, 10, 14, m[sg[4]], CST[sg[5]], m[sg[5]], CST[sg[4]]);
        g512(&mut v, 3, 7, 11, 15, m[sg[6]], CST[sg[7]], m[sg[7]], CST[sg[6]]);
        g512(&mut v, 0, 5, 10, 15, m[sg[8]], CST[sg[9]], m[sg[9]], CST[sg[8]]);
        g512(&mut v, 1, 6, 11, 12, m[sg[10]], CST[sg[11]], m[sg[11]], CST[sg[10]]);
        g512(&mut v, 2, 7, 8, 13, m[sg[12]], CST[sg[13]], m[sg[13]], CST[sg[12]]);
        g512(&mut v, 3, 4, 9, 14, m[sg[14]], CST[sg[15]], m[sg[15]], CST[sg[14]]);
    }

    for i in 0..8 { v[i] ^= v[i + 8]; }
    for i in 0..4 { v[i] ^= s.s[i]; v[i + 4] ^= s.s[i]; }
    for i in 0..8 { s.h[i] ^= v[i]; }
}

pub fn blake512_init(s: &mut BlakeState512) {
    s.h = [
        0x6A09E667F3BCC908, 0xBB67AE8584CAA73B,
        0x3C6EF372FE94F82B, 0xA54FF53A5F1D36F1,
        0x510E527FADE682D1, 0x9B05688C2B3E6C1F,
        0x1F83D9ABFB41BD6B, 0x5BE0CD19137E2179,
    ];
    s.t = [0, 0];
    s.buflen = 0;
    s.nullt = 0;
    s.s = [0; 4];
}

pub fn blake512_update(s: &mut BlakeState512, data: &[u8], mut datalen: u64) {
    let mut offset = 0usize;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 128 - left;

    if left != 0 && (((datalen >> 3) & 0x7F) as usize) >= fill {
        s.buf[left..left + fill].copy_from_slice(&data[offset..offset + fill]);
        s.t[0] = s.t[0].wrapping_add(1024);
        blake512_compress(s, &s.buf.clone());
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

pub fn blake512_final(s: &mut BlakeState512, digest: &mut [u8]) {
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
            blake512_update(s, &PADDING, (888 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((1024 - s.buflen) as u64);
            blake512_update(s, &PADDING, (1024 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(888);
            blake512_update(s, &PADDING[1..], 888);
            s.nullt = 1;
        }
        blake512_update(s, &[zo], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(128);
    blake512_update(s, &msglen, 128);

    for i in 0..8 {
        u64to8(&mut digest[i * 8..], s.h[i]);
    }
}

pub fn blake512(out: &mut [u8], inp: &[u8], inlen: u64) -> i32 {
    let mut s = BlakeState512 {
        h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128],
    };
    blake512_init(&mut s);
    blake512_update(&mut s, inp, inlen.wrapping_mul(8));
    blake512_final(&mut s, out);
    0
}

pub fn blake512_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    let mut i: usize = 0;
    let mut out_off = 0usize;
    while (i + 1) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake512(&mut out[out_off..], &inbuf, (inlen + 4) as u64);
        out_off += SPX_BLAKE512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_BLAKE512_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake512(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - i * SPX_BLAKE512_OUTPUT_BYTES;
        out[out_off..out_off + rem].copy_from_slice(&outbuf[..rem]);
    }
}
