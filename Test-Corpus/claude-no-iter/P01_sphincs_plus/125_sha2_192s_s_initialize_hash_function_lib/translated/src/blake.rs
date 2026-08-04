// Translation of c_src/lib/blake/src/blake256.c and blake512.c
// (J.-P. Aumasson reference BLAKE implementation.)
//
// We model the C structs `blakestate256` and `blakestate512` directly. The
// `update` functions take a bit-length (datalen *in bits*), preserving the
// original C calling convention.

use crate::utils::u32_to_bytes;

pub const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;
pub const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

#[derive(Clone, Copy)]
pub struct BlakeState256 {
    pub h: [u32; 8],
    pub s: [u32; 4],
    pub t: [u32; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 64],
}

#[derive(Clone, Copy)]
pub struct BlakeState512 {
    pub h: [u64; 8],
    pub s: [u64; 4],
    pub t: [u64; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 128],
}

impl BlakeState256 {
    pub const fn zero() -> Self {
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
impl BlakeState512 {
    pub const fn zero() -> Self {
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

const PADDING_256: [u8; 64] = {
    let mut p = [0u8; 64];
    p[0] = 0x80;
    p
};
const PADDING_512: [u8; 129] = {
    let mut p = [0u8; 129];
    p[0] = 0x80;
    p
};

const SIGMA: [[usize; 16]; 16] = [
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
    // BLAKE-512 has 16 rounds; the last two re-use sigma rows 4 and 5.
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
];

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

#[inline]
fn rot256(x: u32, n: u32) -> u32 {
    (x << (32 - n)) | (x >> n)
}
#[inline]
fn rot512(x: u64, n: u32) -> u64 {
    (x << (64 - n)) | (x >> n)
}

pub fn blake256_compress(state: &mut BlakeState256, block: &[u8]) {
    let mut m = [0u32; 16];
    for i in 0..16 {
        m[i] = u8to32(&block[4 * i..]);
    }
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

    // BLAKE-256 G mixing function. Takes two pairs of message word and
    // constant (matching the BLAKE-256 spec).
    fn g256(
        v: &mut [u32; 16],
        a: usize, b: usize, c: usize, d: usize,
        m0: u32, k0: u32, m1: u32, k1: u32,
    ) {
        v[a] = v[a].wrapping_add(m0 ^ k0).wrapping_add(v[b]);
        v[d] = rot256(v[d] ^ v[a], 16);
        v[c] = v[c].wrapping_add(v[d]);
        v[b] = rot256(v[b] ^ v[c], 12);
        v[a] = v[a].wrapping_add(m1 ^ k1).wrapping_add(v[b]);
        v[d] = rot256(v[d] ^ v[a], 8);
        v[c] = v[c].wrapping_add(v[d]);
        v[b] = rot256(v[b] ^ v[c], 7);
    }

    fn round(v: &mut [u32; 16], m: &[u32; 16], r: usize) {
        let s = SIGMA[r];
        g256(v, 0, 4, 8, 12, m[s[0]], CST256[s[1]], m[s[1]], CST256[s[0]]);
        g256(v, 1, 5, 9, 13, m[s[2]], CST256[s[3]], m[s[3]], CST256[s[2]]);
        g256(v, 2, 6, 10, 14, m[s[4]], CST256[s[5]], m[s[5]], CST256[s[4]]);
        g256(v, 3, 7, 11, 15, m[s[6]], CST256[s[7]], m[s[7]], CST256[s[6]]);
        g256(v, 0, 5, 10, 15, m[s[8]], CST256[s[9]], m[s[9]], CST256[s[8]]);
        g256(v, 1, 6, 11, 12, m[s[10]], CST256[s[11]], m[s[11]], CST256[s[10]]);
        g256(v, 2, 7, 8, 13, m[s[12]], CST256[s[13]], m[s[13]], CST256[s[12]]);
        g256(v, 3, 4, 9, 14, m[s[14]], CST256[s[15]], m[s[15]], CST256[s[14]]);
    }

    for r in 0..14 {
        round(&mut v, &m, r);
    }

    v[0] ^= v[8]; v[1] ^= v[9]; v[2] ^= v[10]; v[3] ^= v[11];
    v[4] ^= v[12]; v[5] ^= v[13]; v[6] ^= v[14]; v[7] ^= v[15];

    v[0] ^= state.s[0]; v[1] ^= state.s[1]; v[2] ^= state.s[2]; v[3] ^= state.s[3];
    v[4] ^= state.s[0]; v[5] ^= state.s[1]; v[6] ^= state.s[2]; v[7] ^= state.s[3];

    state.h[0] ^= v[0]; state.h[1] ^= v[1]; state.h[2] ^= v[2]; state.h[3] ^= v[3];
    state.h[4] ^= v[4]; state.h[5] ^= v[5]; state.h[6] ^= v[6]; state.h[7] ^= v[7];
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
    s.t[0] = 0; s.t[1] = 0;
    s.buflen = 0;
    s.nullt = 0;
    s.s[0] = 0; s.s[1] = 0; s.s[2] = 0; s.s[3] = 0;
}

pub fn blake256_update(state: &mut BlakeState256, mut data: &[u8], mut datalen: u64) {
    let mut left = (state.buflen >> 3) as usize;
    let fill = 64 - left;

    if left != 0 && (((datalen >> 3) & 0x3F) >= fill as u64) {
        state.buf[left..left + fill].copy_from_slice(&data[..fill]);
        state.t[0] = state.t[0].wrapping_add(512);
        if state.t[0] == 0 {
            state.t[1] = state.t[1].wrapping_add(1);
        }
        let buf_copy = state.buf;
        blake256_compress(state, &buf_copy);
        data = &data[fill..];
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 512 {
        state.t[0] = state.t[0].wrapping_add(512);
        if state.t[0] == 0 {
            state.t[1] = state.t[1].wrapping_add(1);
        }
        blake256_compress(state, &data[..64]);
        data = &data[64..];
        datalen -= 512;
    }

    if datalen > 0 {
        let nbytes = (datalen >> 3) as usize;
        state.buf[left..left + nbytes].copy_from_slice(&data[..nbytes]);
        state.buflen = (left as i32) * 8 + datalen as i32;
    } else {
        state.buflen = 0;
    }
}

pub fn blake256_final(state: &mut BlakeState256, digest: &mut [u8]) {
    let mut msglen = [0u8; 8];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo = state.t[0].wrapping_add(state.buflen as u32);
    let mut hi = state.t[1];
    if lo < state.buflen as u32 {
        hi = hi.wrapping_add(1);
    }
    u32to8(&mut msglen[..4], hi);
    u32to8(&mut msglen[4..], lo);

    if state.buflen == 440 {
        state.t[0] = state.t[0].wrapping_sub(8);
        let arr = [oo];
        blake256_update(state, &arr, 8);
    } else {
        if state.buflen < 440 {
            if state.buflen == 0 {
                state.nullt = 1;
            }
            state.t[0] = state.t[0].wrapping_sub(440 - state.buflen as u32);
            let nbytes = ((440 - state.buflen) / 8) as usize;
            let pad = PADDING_256[..nbytes].to_vec();
            blake256_update(state, &pad, (440 - state.buflen) as u64);
        } else {
            state.t[0] = state.t[0].wrapping_sub(512 - state.buflen as u32);
            let nbytes_a = ((512 - state.buflen) / 8) as usize;
            let pad_a = PADDING_256[..nbytes_a].to_vec();
            blake256_update(state, &pad_a, (512 - state.buflen) as u64);
            state.t[0] = state.t[0].wrapping_sub(440);
            let pad_b = PADDING_256[1..1 + 55].to_vec();
            blake256_update(state, &pad_b, 440);
            state.nullt = 1;
        }
        let arr = [zo];
        blake256_update(state, &arr, 8);
        state.t[0] = state.t[0].wrapping_sub(8);
    }
    state.t[0] = state.t[0].wrapping_sub(64);
    blake256_update(state, &msglen, 64);

    u32to8(&mut digest[0..], state.h[0]);
    u32to8(&mut digest[4..], state.h[1]);
    u32to8(&mut digest[8..], state.h[2]);
    u32to8(&mut digest[12..], state.h[3]);
    u32to8(&mut digest[16..], state.h[4]);
    u32to8(&mut digest[20..], state.h[5]);
    u32to8(&mut digest[24..], state.h[6]);
    u32to8(&mut digest[28..], state.h[7]);
}

pub fn blake256_mgf1(out: &mut [u8], outlen: usize, in_buf: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    inbuf[..inlen].copy_from_slice(&in_buf[..inlen]);

    let mut out_off = 0usize;
    let mut i = 0u32;
    loop {
        if (i as usize + 1) * SPX_BLAKE256_OUTPUT_BYTES > outlen {
            break;
        }
        u32_to_bytes(&mut inbuf[inlen..], i);
        blake256(&mut out[out_off..], &inbuf, (inlen + 4) as u64);
        out_off += SPX_BLAKE256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > (i as usize) * SPX_BLAKE256_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i);
        blake256(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let want = outlen - (i as usize) * SPX_BLAKE256_OUTPUT_BYTES;
        out[out_off..out_off + want].copy_from_slice(&outbuf[..want]);
    }
}

pub fn blake256(out: &mut [u8], in_buf: &[u8], inlen: u64) -> i32 {
    let mut s = BlakeState256::zero();
    blake256_init(&mut s);
    blake256_update(&mut s, in_buf, inlen * 8);
    blake256_final(&mut s, out);
    0
}

// ----------------------------- BLAKE-512 -----------------------------------

pub fn blake512_compress(state: &mut BlakeState512, block: &[u8]) {
    let mut m = [0u64; 16];
    for i in 0..16 {
        m[i] = u8to64(&block[8 * i..]);
    }
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

    fn g512(
        v: &mut [u64; 16],
        a: usize, b: usize, c: usize, d: usize,
        m0: u64, k0: u64, m1: u64, k1: u64,
    ) {
        v[a] = v[a].wrapping_add(m0 ^ k0).wrapping_add(v[b]);
        v[d] = rot512(v[d] ^ v[a], 32);
        v[c] = v[c].wrapping_add(v[d]);
        v[b] = rot512(v[b] ^ v[c], 25);
        v[a] = v[a].wrapping_add(m1 ^ k1).wrapping_add(v[b]);
        v[d] = rot512(v[d] ^ v[a], 16);
        v[c] = v[c].wrapping_add(v[d]);
        v[b] = rot512(v[b] ^ v[c], 11);
    }

    fn round(v: &mut [u64; 16], m: &[u64; 16], r: usize) {
        let s = SIGMA[r];
        g512(v, 0, 4, 8, 12, m[s[0]], CST512[s[1]], m[s[1]], CST512[s[0]]);
        g512(v, 1, 5, 9, 13, m[s[2]], CST512[s[3]], m[s[3]], CST512[s[2]]);
        g512(v, 2, 6, 10, 14, m[s[4]], CST512[s[5]], m[s[5]], CST512[s[4]]);
        g512(v, 3, 7, 11, 15, m[s[6]], CST512[s[7]], m[s[7]], CST512[s[6]]);
        g512(v, 0, 5, 10, 15, m[s[8]], CST512[s[9]], m[s[9]], CST512[s[8]]);
        g512(v, 1, 6, 11, 12, m[s[10]], CST512[s[11]], m[s[11]], CST512[s[10]]);
        g512(v, 2, 7, 8, 13, m[s[12]], CST512[s[13]], m[s[13]], CST512[s[12]]);
        g512(v, 3, 4, 9, 14, m[s[14]], CST512[s[15]], m[s[15]], CST512[s[14]]);
    }

    for r in 0..16 {
        round(&mut v, &m, r);
    }

    v[0] ^= v[8]; v[1] ^= v[9]; v[2] ^= v[10]; v[3] ^= v[11];
    v[4] ^= v[12]; v[5] ^= v[13]; v[6] ^= v[14]; v[7] ^= v[15];

    v[0] ^= state.s[0]; v[1] ^= state.s[1]; v[2] ^= state.s[2]; v[3] ^= state.s[3];
    v[4] ^= state.s[0]; v[5] ^= state.s[1]; v[6] ^= state.s[2]; v[7] ^= state.s[3];

    state.h[0] ^= v[0]; state.h[1] ^= v[1]; state.h[2] ^= v[2]; state.h[3] ^= v[3];
    state.h[4] ^= v[4]; state.h[5] ^= v[5]; state.h[6] ^= v[6]; state.h[7] ^= v[7];
}

pub fn blake512_init(s: &mut BlakeState512) {
    s.h[0] = 0x6A09E667F3BCC908;
    s.h[1] = 0xBB67AE8584CAA73B;
    s.h[2] = 0x3C6EF372FE94F82B;
    s.h[3] = 0xA54FF53A5F1D36F1;
    s.h[4] = 0x510E527FADE682D1;
    s.h[5] = 0x9B05688C2B3E6C1F;
    s.h[6] = 0x1F83D9ABFB41BD6B;
    s.h[7] = 0x5BE0CD19137E2179;
    s.t[0] = 0; s.t[1] = 0;
    s.buflen = 0;
    s.nullt = 0;
    s.s[0] = 0; s.s[1] = 0; s.s[2] = 0; s.s[3] = 0;
}

pub fn blake512_update(state: &mut BlakeState512, mut data: &[u8], mut datalen: u64) {
    let mut left = (state.buflen >> 3) as usize;
    let fill = 128 - left;

    if left != 0 && (((datalen >> 3) & 0x7F) >= fill as u64) {
        state.buf[left..left + fill].copy_from_slice(&data[..fill]);
        state.t[0] = state.t[0].wrapping_add(1024);
        let buf_copy = state.buf;
        blake512_compress(state, &buf_copy);
        data = &data[fill..];
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 1024 {
        state.t[0] = state.t[0].wrapping_add(1024);
        blake512_compress(state, &data[..128]);
        data = &data[128..];
        datalen -= 1024;
    }

    if datalen > 0 {
        let nbytes = ((datalen >> 3) & 0x7F) as usize;
        state.buf[left..left + nbytes].copy_from_slice(&data[..nbytes]);
        state.buflen = (left as i32) * 8 + datalen as i32;
    } else {
        state.buflen = 0;
    }
}

pub fn blake512_final(state: &mut BlakeState512, digest: &mut [u8]) {
    let mut msglen = [0u8; 16];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo = state.t[0].wrapping_add(state.buflen as u64);
    let mut hi = state.t[1];
    if lo < state.buflen as u64 {
        hi = hi.wrapping_add(1);
    }
    u64to8(&mut msglen[..8], hi);
    u64to8(&mut msglen[8..], lo);

    if state.buflen == 888 {
        state.t[0] = state.t[0].wrapping_sub(8);
        let arr = [oo];
        blake512_update(state, &arr, 8);
    } else {
        if state.buflen < 888 {
            if state.buflen == 0 {
                state.nullt = 1;
            }
            state.t[0] = state.t[0].wrapping_sub(888 - state.buflen as u64);
            let nbytes = ((888 - state.buflen) / 8) as usize;
            let pad = PADDING_512[..nbytes].to_vec();
            blake512_update(state, &pad, (888 - state.buflen) as u64);
        } else {
            state.t[0] = state.t[0].wrapping_sub(1024 - state.buflen as u64);
            let nbytes_a = ((1024 - state.buflen) / 8) as usize;
            let pad_a = PADDING_512[..nbytes_a].to_vec();
            blake512_update(state, &pad_a, (1024 - state.buflen) as u64);
            state.t[0] = state.t[0].wrapping_sub(888);
            let pad_b = PADDING_512[1..1 + 111].to_vec();
            blake512_update(state, &pad_b, 888);
            state.nullt = 1;
        }
        let arr = [zo];
        blake512_update(state, &arr, 8);
        state.t[0] = state.t[0].wrapping_sub(8);
    }
    state.t[0] = state.t[0].wrapping_sub(128);
    blake512_update(state, &msglen, 128);

    u64to8(&mut digest[0..], state.h[0]);
    u64to8(&mut digest[8..], state.h[1]);
    u64to8(&mut digest[16..], state.h[2]);
    u64to8(&mut digest[24..], state.h[3]);
    u64to8(&mut digest[32..], state.h[4]);
    u64to8(&mut digest[40..], state.h[5]);
    u64to8(&mut digest[48..], state.h[6]);
    u64to8(&mut digest[56..], state.h[7]);
}

pub fn blake512_mgf1(out: &mut [u8], outlen: usize, in_buf: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    inbuf[..inlen].copy_from_slice(&in_buf[..inlen]);

    let mut out_off = 0usize;
    let mut i = 0u32;
    loop {
        if (i as usize + 1) * SPX_BLAKE512_OUTPUT_BYTES > outlen {
            break;
        }
        u32_to_bytes(&mut inbuf[inlen..], i);
        blake512(&mut out[out_off..], &inbuf, (inlen + 4) as u64);
        out_off += SPX_BLAKE512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > (i as usize) * SPX_BLAKE512_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i);
        blake512(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let want = outlen - (i as usize) * SPX_BLAKE512_OUTPUT_BYTES;
        out[out_off..out_off + want].copy_from_slice(&outbuf[..want]);
    }
}

pub fn blake512(out: &mut [u8], in_buf: &[u8], inlen: u64) -> i32 {
    let mut s = BlakeState512::zero();
    blake512_init(&mut s);
    blake512_update(&mut s, in_buf, inlen * 8);
    blake512_final(&mut s, out);
    0
}
