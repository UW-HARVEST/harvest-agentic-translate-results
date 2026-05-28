// Translation of blake256.c / blake512.c (the bespoke BLAKE-256 and BLAKE-512
// implementations used by SPHINCS+ reference). Kept faithful so output bytes
// match the C exactly.

#[derive(Clone)]
pub struct Blakestate256 {
    pub h: [u32; 8],
    pub s: [u32; 4],
    pub t: [u32; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 64],
}

impl Blakestate256 {
    pub fn new() -> Self {
        Blakestate256 {
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
pub struct Blakestate512 {
    pub h: [u64; 8],
    pub s: [u64; 4],
    pub t: [u64; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 128],
}

impl Blakestate512 {
    pub fn new() -> Self {
        Blakestate512 {
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

const CST32: [u32; 16] = [
    0x243F6A88, 0x85A308D3, 0x13198A2E, 0x03707344,
    0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    0x452821E6, 0x38D01377, 0xBE5466CF, 0x34E90C6C,
    0xC0AC29B7, 0xC97C50DD, 0x3F84D5B5, 0xB5470917,
];

const CST64: [u64; 16] = [
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
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

#[inline]
fn rot32(x: u32, n: u32) -> u32 {
    (x.wrapping_shl(32 - n)) | (x.wrapping_shr(n))
}

#[inline]
fn rot64(x: u64, n: u32) -> u64 {
    (x.wrapping_shl(64 - n)) | (x.wrapping_shr(n))
}

// Sigma permutation table for round indexing. Each round uses a permutation.
// 14 rounds for BLAKE-256, 16 for BLAKE-512. Rows 10-15 reuse rows 0-5.
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
    [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15],
    [14,10,4,8,9,15,13,6,1,12,0,2,11,7,5,3],
    [11,8,12,0,5,2,15,13,10,14,3,6,7,1,9,4],
    [7,9,3,1,13,12,11,14,2,6,5,10,4,0,15,8],
    [9,0,5,7,2,4,10,15,14,1,11,12,6,8,3,13],
    [2,12,6,10,0,11,8,3,4,13,7,5,15,14,1,9],
];

// BLAKE-256 G function (one quarter-round). Operates on a 16-element state.
#[inline]
fn g256(v: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, m: &[u32; 16], r: usize, i: usize) {
    let s = SIGMA[r];
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(m[s[2*i]] ^ CST32[s[2*i + 1]]);
    v[d] = rot32(v[d] ^ v[a], 16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = rot32(v[b] ^ v[c], 12);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(m[s[2*i + 1]] ^ CST32[s[2*i]]);
    v[d] = rot32(v[d] ^ v[a], 8);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = rot32(v[b] ^ v[c], 7);
}

#[inline]
fn g512(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, m: &[u64; 16], r: usize, i: usize) {
    let s = SIGMA[r];
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(m[s[2*i]] ^ CST64[s[2*i + 1]]);
    v[d] = rot64(v[d] ^ v[a], 32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = rot64(v[b] ^ v[c], 25);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(m[s[2*i + 1]] ^ CST64[s[2*i]]);
    v[d] = rot64(v[d] ^ v[a], 16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = rot64(v[b] ^ v[c], 11);
}

pub fn blake256_compress(s: &mut Blakestate256, block: &[u8]) {
    let mut m = [0u32; 16];
    for i in 0..16 {
        m[i] = u8to32(&block[4 * i..]);
    }
    let mut v = [0u32; 16];
    for i in 0..8 {
        v[i] = s.h[i];
    }
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
    for r in 0..14 {
        g256(&mut v, 0, 4, 8, 12, &m, r, 0);
        g256(&mut v, 1, 5, 9, 13, &m, r, 1);
        g256(&mut v, 2, 6, 10, 14, &m, r, 2);
        g256(&mut v, 3, 7, 11, 15, &m, r, 3);
        g256(&mut v, 0, 5, 10, 15, &m, r, 4);
        g256(&mut v, 1, 6, 11, 12, &m, r, 5);
        g256(&mut v, 2, 7, 8, 13, &m, r, 6);
        g256(&mut v, 3, 4, 9, 14, &m, r, 7);
    }
    for i in 0..8 {
        s.h[i] ^= v[i] ^ v[i + 8] ^ s.s[i % 4];
    }
}

pub fn blake512_compress(s: &mut Blakestate512, block: &[u8]) {
    let mut m = [0u64; 16];
    for i in 0..16 {
        m[i] = u8to64(&block[8 * i..]);
    }
    let mut v = [0u64; 16];
    for i in 0..8 {
        v[i] = s.h[i];
    }
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
    for r in 0..16 {
        g512(&mut v, 0, 4, 8, 12, &m, r, 0);
        g512(&mut v, 1, 5, 9, 13, &m, r, 1);
        g512(&mut v, 2, 6, 10, 14, &m, r, 2);
        g512(&mut v, 3, 7, 11, 15, &m, r, 3);
        g512(&mut v, 0, 5, 10, 15, &m, r, 4);
        g512(&mut v, 1, 6, 11, 12, &m, r, 5);
        g512(&mut v, 2, 7, 8, 13, &m, r, 6);
        g512(&mut v, 3, 4, 9, 14, &m, r, 7);
    }
    for i in 0..8 {
        s.h[i] ^= v[i] ^ v[i + 8] ^ s.s[i % 4];
    }
}

pub fn blake256_init(s: &mut Blakestate256) {
    s.h = [0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
           0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19];
    s.t = [0, 0];
    s.buflen = 0;
    s.nullt = 0;
    s.s = [0; 4];
}

pub fn blake256_update(s: &mut Blakestate256, mut data: &[u8], mut datalen_bits: u64) {
    let mut left = (s.buflen >> 3) as usize;
    let fill = 64 - left;

    if left != 0 && (((datalen_bits >> 3) & 0x3F) as usize) >= fill {
        s.buf[left..left + fill].copy_from_slice(&data[..fill]);
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 {
            s.t[1] = s.t[1].wrapping_add(1);
        }
        let buf_copy = s.buf;
        blake256_compress(s, &buf_copy);
        data = &data[fill..];
        datalen_bits -= (fill as u64) << 3;
        left = 0;
    }

    while datalen_bits >= 512 {
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 {
            s.t[1] = s.t[1].wrapping_add(1);
        }
        blake256_compress(s, &data[..64]);
        data = &data[64..];
        datalen_bits -= 512;
    }

    if datalen_bits > 0 {
        let nbytes = (datalen_bits >> 3) as usize;
        s.buf[left..left + nbytes].copy_from_slice(&data[..nbytes]);
        s.buflen = ((left as u64) << 3) as i32 + datalen_bits as i32;
    } else {
        s.buflen = 0;
    }
}

pub fn blake256_final(s: &mut Blakestate256, digest: &mut [u8]) {
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
        let oo_buf = [oo];
        blake256_update(s, &oo_buf, 8);
    } else {
        if s.buflen < 440 {
            if s.buflen == 0 {
                s.nullt = 1;
            }
            s.t[0] = s.t[0].wrapping_sub(440 - s.buflen as u32);
            let pad_len = (440 - s.buflen) as u64;
            blake256_update(s, &PADDING_256, pad_len);
        } else {
            s.t[0] = s.t[0].wrapping_sub(512 - s.buflen as u32);
            let pad_len = (512 - s.buflen) as u64;
            blake256_update(s, &PADDING_256, pad_len);
            s.t[0] = s.t[0].wrapping_sub(440);
            blake256_update(s, &PADDING_256[1..], 440);
            s.nullt = 1;
        }
        let zo_buf = [zo];
        blake256_update(s, &zo_buf, 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(64);
    blake256_update(s, &msglen, 64);

    for i in 0..8 {
        u32to8(&mut digest[4 * i..4 * i + 4], s.h[i]);
    }
}

pub fn blake256(out: &mut [u8], input: &[u8], inlen: u64) {
    let mut s = Blakestate256::new();
    blake256_init(&mut s);
    blake256_update(&mut s, input, inlen.wrapping_mul(8));
    blake256_final(&mut s, out);
}

pub fn blake512_init(s: &mut Blakestate512) {
    s.h = [0x6A09E667F3BCC908, 0xBB67AE8584CAA73B, 0x3C6EF372FE94F82B, 0xA54FF53A5F1D36F1,
           0x510E527FADE682D1, 0x9B05688C2B3E6C1F, 0x1F83D9ABFB41BD6B, 0x5BE0CD19137E2179];
    s.t = [0, 0];
    s.buflen = 0;
    s.nullt = 0;
    s.s = [0; 4];
}

pub fn blake512_update(s: &mut Blakestate512, mut data: &[u8], mut datalen_bits: u64) {
    let mut left = (s.buflen >> 3) as usize;
    let fill = 128 - left;

    if left != 0 && (((datalen_bits >> 3) & 0x7F) as usize) >= fill {
        s.buf[left..left + fill].copy_from_slice(&data[..fill]);
        s.t[0] = s.t[0].wrapping_add(1024);
        let buf_copy = s.buf;
        blake512_compress(s, &buf_copy);
        data = &data[fill..];
        datalen_bits -= (fill as u64) << 3;
        left = 0;
    }

    while datalen_bits >= 1024 {
        s.t[0] = s.t[0].wrapping_add(1024);
        blake512_compress(s, &data[..128]);
        data = &data[128..];
        datalen_bits -= 1024;
    }

    if datalen_bits > 0 {
        let nbytes = ((datalen_bits >> 3) & 0x7F) as usize;
        s.buf[left..left + nbytes].copy_from_slice(&data[..nbytes]);
        s.buflen = ((left as u64) << 3) as i32 + datalen_bits as i32;
    } else {
        s.buflen = 0;
    }
}

pub fn blake512_final(s: &mut Blakestate512, digest: &mut [u8]) {
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
        let oo_buf = [oo];
        blake512_update(s, &oo_buf, 8);
    } else {
        if s.buflen < 888 {
            if s.buflen == 0 {
                s.nullt = 1;
            }
            s.t[0] = s.t[0].wrapping_sub((888 - s.buflen) as u64);
            let pad_len = (888 - s.buflen) as u64;
            blake512_update(s, &PADDING_512, pad_len);
        } else {
            s.t[0] = s.t[0].wrapping_sub((1024 - s.buflen) as u64);
            let pad_len = (1024 - s.buflen) as u64;
            blake512_update(s, &PADDING_512, pad_len);
            s.t[0] = s.t[0].wrapping_sub(888);
            blake512_update(s, &PADDING_512[1..], 888);
            s.nullt = 1;
        }
        let zo_buf = [zo];
        blake512_update(s, &zo_buf, 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(128);
    blake512_update(s, &msglen, 128);

    for i in 0..8 {
        u64to8(&mut digest[8 * i..8 * i + 8], s.h[i]);
    }
}

pub fn blake512(out: &mut [u8], input: &[u8], inlen: u64) {
    let mut s = Blakestate512::new();
    blake512_init(&mut s);
    blake512_update(&mut s, input, inlen.wrapping_mul(8));
    blake512_final(&mut s, out);
}

pub const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;
pub const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

pub fn blake256_mgf1(out: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&input[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut i: u64 = 0;
    let mut out_pos = 0usize;
    while ((i + 1) as usize) * SPX_BLAKE256_OUTPUT_BYTES <= outlen {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..inlen + 4], i as u32);
        blake256(&mut out[out_pos..out_pos + SPX_BLAKE256_OUTPUT_BYTES], &inbuf, (inlen + 4) as u64);
        out_pos += SPX_BLAKE256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > (i as usize) * SPX_BLAKE256_OUTPUT_BYTES {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..inlen + 4], i as u32);
        blake256(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let leftover = outlen - (i as usize) * SPX_BLAKE256_OUTPUT_BYTES;
        out[out_pos..out_pos + leftover].copy_from_slice(&outbuf[..leftover]);
    }
}

pub fn blake512_mgf1(out: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&input[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut i: u64 = 0;
    let mut out_pos = 0usize;
    while ((i + 1) as usize) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..inlen + 4], i as u32);
        blake512(&mut out[out_pos..out_pos + SPX_BLAKE512_OUTPUT_BYTES], &inbuf, (inlen + 4) as u64);
        out_pos += SPX_BLAKE512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > (i as usize) * SPX_BLAKE512_OUTPUT_BYTES {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..inlen + 4], i as u32);
        blake512(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let leftover = outlen - (i as usize) * SPX_BLAKE512_OUTPUT_BYTES;
        out[out_pos..out_pos + leftover].copy_from_slice(&outbuf[..leftover]);
    }
}
