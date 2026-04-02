// Byte-identical Rust translation of blake256.c and blake512.c
// BLAKE-256 and BLAKE-512 hash functions

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
        BlakeState256 {
            h: [0u32; 8],
            s: [0u32; 4],
            t: [0u32; 2],
            buflen: 0,
            nullt: 0,
            buf: [0u8; 64],
        }
    }
}

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
        BlakeState512 {
            h: [0u64; 8],
            s: [0u64; 4],
            t: [0u64; 2],
            buflen: 0,
            nullt: 0,
            buf: [0u8; 128],
        }
    }
}

// BLAKE-256 constants
const CST256: [u32; 16] = [
    0x243F6A88, 0x85A308D3, 0x13198A2E, 0x03707344,
    0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    0x452821E6, 0x38D01377, 0xBE5466CF, 0x34E90C6C,
    0xC0AC29B7, 0xC97C50DD, 0x3F84D5B5, 0xB5470917,
];

// BLAKE-512 constants
const CST512: [u64; 16] = [
    0x243F6A8885A308D3, 0x13198A2E03707344, 0xA4093822299F31D0, 0x082EFA98EC4E6C89,
    0x452821E638D01377, 0xBE5466CF34E90C6C, 0xC0AC29B7C97C50DD, 0x3F84D5B5B5470917,
    0x9216D5D98979FB1B, 0xD1310BA698DFB5AC, 0x2FFD72DBD01ADFB7, 0xB8E1AFED6A267E96,
    0xBA7C9045F12C7F99, 0x24A19947B3916CF7, 0x0801F2E2858EFC16, 0x636920D871574E69,
];

const SIGMA: [[usize; 16]; 14] = [
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

const SIGMA512: [[usize; 16]; 16] = [
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

static PADDING256: [u8; 64] = {
    let mut p = [0u8; 64];
    p[0] = 0x80;
    p
};

static PADDING512: [u8; 129] = {
    let mut p = [0u8; 129];
    p[0] = 0x80;
    p
};

#[inline]
fn u8to32(p: &[u8]) -> u32 {
    ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32)
}

#[inline]
fn u32to8(out: &mut [u8], v: u32) {
    out[0] = (v >> 24) as u8;
    out[1] = (v >> 16) as u8;
    out[2] = (v >> 8) as u8;
    out[3] = v as u8;
}

#[inline]
fn u8to64(p: &[u8]) -> u64 {
    ((p[0] as u64) << 56)
        | ((p[1] as u64) << 48)
        | ((p[2] as u64) << 40)
        | ((p[3] as u64) << 32)
        | ((p[4] as u64) << 24)
        | ((p[5] as u64) << 16)
        | ((p[6] as u64) << 8)
        | (p[7] as u64)
}

#[inline]
fn u64to8(out: &mut [u8], v: u64) {
    u32to8(&mut out[0..4], (v >> 32) as u32);
    u32to8(&mut out[4..8], v as u32);
}

// ============================================================
// BLAKE-256
// ============================================================

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
    s.buf = [0u8; 64];
}

#[inline]
fn blake256_g(v: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, mx: u32, cx: u32, my: u32, cy: u32) {
    v[a] = v[a].wrapping_add(mx ^ cx).wrapping_add(v[b]);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(12);
    v[a] = v[a].wrapping_add(my ^ cy).wrapping_add(v[b]);
    v[d] = (v[d] ^ v[a]).rotate_right(8);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(7);
}

pub fn blake256_compress(s: &mut BlakeState256, block: &[u8]) {
    let mut v: [u32; 16] = [0; 16];
    let mut m: [u32; 16] = [0; 16];

    for i in 0..16 {
        m[i] = u8to32(&block[4 * i..]);
    }

    v[0] = s.h[0];
    v[1] = s.h[1];
    v[2] = s.h[2];
    v[3] = s.h[3];
    v[4] = s.h[4];
    v[5] = s.h[5];
    v[6] = s.h[6];
    v[7] = s.h[7];
    v[8] = s.s[0] ^ CST256[0];
    v[9] = s.s[1] ^ CST256[1];
    v[10] = s.s[2] ^ CST256[2];
    v[11] = s.s[3] ^ CST256[3];
    v[12] = CST256[4];
    v[13] = CST256[5];
    v[14] = CST256[6];
    v[15] = CST256[7];

    if s.nullt == 0 {
        v[12] ^= s.t[0];
        v[13] ^= s.t[0];
        v[14] ^= s.t[1];
        v[15] ^= s.t[1];
    }

    for round in 0..14 {
        let sig = &SIGMA[round];
        // Column step
        blake256_g(&mut v, 0, 4, 8,  12, m[sig[0]], CST256[sig[1]], m[sig[1]], CST256[sig[0]]);
        blake256_g(&mut v, 1, 5, 9,  13, m[sig[2]], CST256[sig[3]], m[sig[3]], CST256[sig[2]]);
        blake256_g(&mut v, 2, 6, 10, 14, m[sig[4]], CST256[sig[5]], m[sig[5]], CST256[sig[4]]);
        blake256_g(&mut v, 3, 7, 11, 15, m[sig[6]], CST256[sig[7]], m[sig[7]], CST256[sig[6]]);
        // Diagonal step
        blake256_g(&mut v, 0, 5, 10, 15, m[sig[8]],  CST256[sig[9]],  m[sig[9]],  CST256[sig[8]]);
        blake256_g(&mut v, 1, 6, 11, 12, m[sig[10]], CST256[sig[11]], m[sig[11]], CST256[sig[10]]);
        blake256_g(&mut v, 2, 7, 8,  13, m[sig[12]], CST256[sig[13]], m[sig[13]], CST256[sig[12]]);
        blake256_g(&mut v, 3, 4, 9,  14, m[sig[14]], CST256[sig[15]], m[sig[15]], CST256[sig[14]]);
    }

    for i in 0..16 {
        s.h[i % 8] ^= v[i];
    }
    for i in 0..8 {
        s.h[i] ^= s.s[i % 4];
    }
}

pub fn blake256_update(s: &mut BlakeState256, data: &[u8], datalen: u64) {
    let mut left: i32 = s.buflen >> 3;
    let fill: i32 = 64 - left;
    let mut datalen_remaining = datalen;
    let mut offset: usize = 0;

    if left != 0 && ((datalen_remaining >> 3) & 0x3F) as i32 >= fill {
        s.buf[left as usize..64].copy_from_slice(&data[offset..offset + fill as usize]);
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 {
            s.t[1] = s.t[1].wrapping_add(1);
        }
        blake256_compress(s, &s.buf.clone());
        offset += fill as usize;
        datalen_remaining -= (fill as u64) << 3;
        left = 0;
    }

    while datalen_remaining >= 512 {
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 {
            s.t[1] = s.t[1].wrapping_add(1);
        }
        blake256_compress(s, &data[offset..]);
        offset += 64;
        datalen_remaining -= 512;
    }

    if datalen_remaining > 0 {
        let bytes = (datalen_remaining >> 3) as usize;
        s.buf[left as usize..left as usize + bytes].copy_from_slice(&data[offset..offset + bytes]);
        s.buflen = (left << 3) + datalen_remaining as i32;
    } else {
        s.buflen = 0;
    }
}

pub fn blake256_final(s: &mut BlakeState256, digest: &mut [u8]) {
    let mut msglen: [u8; 8] = [0u8; 8];
    let lo: u32 = s.t[0].wrapping_add(s.buflen as u32);
    let mut hi: u32 = s.t[1];
    if lo < s.buflen as u32 {
        hi = hi.wrapping_add(1);
    }

    u32to8(&mut msglen[0..4], hi);
    u32to8(&mut msglen[4..8], lo);

    if s.buflen == 440 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake256_update(s, &[0x81u8], 8);
    } else {
        if s.buflen < 440 {
            if s.buflen == 0 {
                s.nullt = 1;
            }
            s.t[0] = s.t[0].wrapping_sub(440 - s.buflen as u32);
            blake256_update(s, &PADDING256, 440 - s.buflen as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub(512 - s.buflen as u32);
            blake256_update(s, &PADDING256, 512 - s.buflen as u64);
            s.t[0] = s.t[0].wrapping_sub(440);
            blake256_update(s, &PADDING256[1..], 440);
            s.nullt = 1;
        }
        blake256_update(s, &[0x01u8], 8);
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

pub fn blake256(out: &mut [u8], inp: &[u8], inlen: u64) -> i32 {
    let mut s = BlakeState256 {
        h: [0u32; 8],
        s: [0u32; 4],
        t: [0u32; 2],
        buflen: 0,
        nullt: 0,
        buf: [0u8; 64],
    };
    blake256_init(&mut s);
    blake256_update(&mut s, inp, inlen * 8);
    blake256_final(&mut s, out);
    0
}

pub fn blake256_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut blk: [u8; 32] = [0u8; 32];
    let mut counter: [u8; 4] = [0u8; 4];
    let mut offset: usize = 0;
    let mut i: u32 = 0;

    while offset < outlen {
        crate::utils::u32_to_bytes(&mut counter, i);

        let mut s = BlakeState256 {
            h: [0u32; 8],
            s: [0u32; 4],
            t: [0u32; 2],
            buflen: 0,
            nullt: 0,
            buf: [0u8; 64],
        };
        blake256_init(&mut s);
        blake256_update(&mut s, &inp[..inlen], (inlen as u64) * 8);
        blake256_update(&mut s, &counter, 4 * 8);
        blake256_final(&mut s, &mut blk);

        let remaining = outlen - offset;
        let to_copy = if remaining < 32 { remaining } else { 32 };
        out[offset..offset + to_copy].copy_from_slice(&blk[..to_copy]);
        offset += to_copy;
        i += 1;
    }
}

// ============================================================
// BLAKE-512
// ============================================================

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
    s.buf = [0u8; 128];
}

#[inline]
fn blake512_g(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, mx: u64, cx: u64, my: u64, cy: u64) {
    v[a] = v[a].wrapping_add(mx ^ cx).wrapping_add(v[b]);
    v[d] = (v[d] ^ v[a]).rotate_right(32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(25);
    v[a] = v[a].wrapping_add(my ^ cy).wrapping_add(v[b]);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(11);
}

pub fn blake512_compress(s: &mut BlakeState512, block: &[u8]) {
    let mut v: [u64; 16] = [0; 16];
    let mut m: [u64; 16] = [0; 16];

    for i in 0..16 {
        m[i] = u8to64(&block[8 * i..]);
    }

    v[0] = s.h[0];
    v[1] = s.h[1];
    v[2] = s.h[2];
    v[3] = s.h[3];
    v[4] = s.h[4];
    v[5] = s.h[5];
    v[6] = s.h[6];
    v[7] = s.h[7];
    v[8] = s.s[0] ^ CST512[0];
    v[9] = s.s[1] ^ CST512[1];
    v[10] = s.s[2] ^ CST512[2];
    v[11] = s.s[3] ^ CST512[3];
    v[12] = CST512[4];
    v[13] = CST512[5];
    v[14] = CST512[6];
    v[15] = CST512[7];

    if s.nullt == 0 {
        v[12] ^= s.t[0];
        v[13] ^= s.t[0];
        v[14] ^= s.t[1];
        v[15] ^= s.t[1];
    }

    for round in 0..16 {
        let sig = &SIGMA512[round];
        // Column step
        blake512_g(&mut v, 0, 4, 8,  12, m[sig[0]], CST512[sig[1]], m[sig[1]], CST512[sig[0]]);
        blake512_g(&mut v, 1, 5, 9,  13, m[sig[2]], CST512[sig[3]], m[sig[3]], CST512[sig[2]]);
        blake512_g(&mut v, 2, 6, 10, 14, m[sig[4]], CST512[sig[5]], m[sig[5]], CST512[sig[4]]);
        blake512_g(&mut v, 3, 7, 11, 15, m[sig[6]], CST512[sig[7]], m[sig[7]], CST512[sig[6]]);
        // Diagonal step
        blake512_g(&mut v, 0, 5, 10, 15, m[sig[8]],  CST512[sig[9]],  m[sig[9]],  CST512[sig[8]]);
        blake512_g(&mut v, 1, 6, 11, 12, m[sig[10]], CST512[sig[11]], m[sig[11]], CST512[sig[10]]);
        blake512_g(&mut v, 2, 7, 8,  13, m[sig[12]], CST512[sig[13]], m[sig[13]], CST512[sig[12]]);
        blake512_g(&mut v, 3, 4, 9,  14, m[sig[14]], CST512[sig[15]], m[sig[15]], CST512[sig[14]]);
    }

    for i in 0..16 {
        s.h[i % 8] ^= v[i];
    }
    for i in 0..8 {
        s.h[i] ^= s.s[i % 4];
    }
}

pub fn blake512_update(s: &mut BlakeState512, data: &[u8], datalen: u64) {
    let mut left: i32 = s.buflen >> 3;
    let fill: i32 = 128 - left;
    let mut datalen_remaining = datalen;
    let mut offset: usize = 0;

    if left != 0 && ((datalen_remaining >> 3) & 0x7F) as i32 >= fill {
        s.buf[left as usize..128].copy_from_slice(&data[offset..offset + fill as usize]);
        s.t[0] = s.t[0].wrapping_add(1024);
        blake512_compress(s, &s.buf.clone());
        offset += fill as usize;
        datalen_remaining -= (fill as u64) << 3;
        left = 0;
    }

    while datalen_remaining >= 1024 {
        s.t[0] = s.t[0].wrapping_add(1024);
        blake512_compress(s, &data[offset..]);
        offset += 128;
        datalen_remaining -= 1024;
    }

    if datalen_remaining > 0 {
        let bytes = ((datalen_remaining >> 3) & 0x7F) as usize;
        s.buf[left as usize..left as usize + bytes].copy_from_slice(&data[offset..offset + bytes]);
        s.buflen = (left << 3) + datalen_remaining as i32;
    } else {
        s.buflen = 0;
    }
}

pub fn blake512_final(s: &mut BlakeState512, digest: &mut [u8]) {
    let mut msglen: [u8; 16] = [0u8; 16];
    let lo: u64 = s.t[0].wrapping_add(s.buflen as u64);
    let mut hi: u64 = s.t[1];
    if lo < s.buflen as u64 {
        hi = hi.wrapping_add(1);
    }

    u64to8(&mut msglen[0..8], hi);
    u64to8(&mut msglen[8..16], lo);

    if s.buflen == 888 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake512_update(s, &[0x81u8], 8);
    } else {
        if s.buflen < 888 {
            if s.buflen == 0 {
                s.nullt = 1;
            }
            s.t[0] = s.t[0].wrapping_sub(888 - s.buflen as u64);
            blake512_update(s, &PADDING512, 888 - s.buflen as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub(1024 - s.buflen as u64);
            blake512_update(s, &PADDING512, 1024 - s.buflen as u64);
            s.t[0] = s.t[0].wrapping_sub(888);
            blake512_update(s, &PADDING512[1..], 888);
            s.nullt = 1;
        }
        blake512_update(s, &[0x01u8], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(128);
    blake512_update(s, &msglen, 128);

    u64to8(&mut digest[0..8], s.h[0]);
    u64to8(&mut digest[8..16], s.h[1]);
    u64to8(&mut digest[16..24], s.h[2]);
    u64to8(&mut digest[24..32], s.h[3]);
    u64to8(&mut digest[32..40], s.h[4]);
    u64to8(&mut digest[40..48], s.h[5]);
    u64to8(&mut digest[48..56], s.h[6]);
    u64to8(&mut digest[56..64], s.h[7]);
}

pub fn blake512(out: &mut [u8], inp: &[u8], inlen: u64) -> i32 {
    let mut s = BlakeState512 {
        h: [0u64; 8],
        s: [0u64; 4],
        t: [0u64; 2],
        buflen: 0,
        nullt: 0,
        buf: [0u8; 128],
    };
    blake512_init(&mut s);
    blake512_update(&mut s, inp, inlen * 8);
    blake512_final(&mut s, out);
    0
}

pub fn blake512_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut blk: [u8; 64] = [0u8; 64];
    let mut counter: [u8; 4] = [0u8; 4];
    let mut offset: usize = 0;
    let mut i: u32 = 0;

    while offset < outlen {
        crate::utils::u32_to_bytes(&mut counter, i);

        let mut s = BlakeState512 {
            h: [0u64; 8],
            s: [0u64; 4],
            t: [0u64; 2],
            buflen: 0,
            nullt: 0,
            buf: [0u8; 128],
        };
        blake512_init(&mut s);
        blake512_update(&mut s, &inp[..inlen], (inlen as u64) * 8);
        blake512_update(&mut s, &counter, 4 * 8);
        blake512_final(&mut s, &mut blk);

        let remaining = outlen - offset;
        let to_copy = if remaining < 64 { remaining } else { 64 };
        out[offset..offset + to_copy].copy_from_slice(&blk[..to_copy]);
        offset += to_copy;
        i += 1;
    }
}
