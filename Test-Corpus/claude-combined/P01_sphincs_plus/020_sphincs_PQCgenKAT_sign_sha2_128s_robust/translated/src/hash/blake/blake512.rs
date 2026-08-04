// Translation of c_src/lib/blake/src/blake512.c

#![allow(non_snake_case)]
#![allow(clippy::needless_range_loop)]

use core::slice;

use crate::params::SPX_BLAKE512_OUTPUT_BYTES;
use crate::utils::u32_to_bytes;

#[repr(C)]
pub struct BlakeState512 {
    pub h: [u64; 8],
    pub s: [u64; 4],
    pub t: [u64; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 128],
}

const CST: [u64; 16] = [
    0x243F6A8885A308D3, 0x13198A2E03707344, 0xA4093822299F31D0, 0x082EFA98EC4E6C89,
    0x452821E638D01377, 0xBE5466CF34E90C6C, 0xC0AC29B7C97C50DD, 0x3F84D5B5B5470917,
    0x9216D5D98979FB1B, 0xD1310BA698DFB5AC, 0x2FFD72DBD01ADFB7, 0xB8E1AFED6A267E96,
    0xBA7C9045F12C7F99, 0x24A19947B3916CF7, 0x0801F2E2858EFC16, 0x636920D871574E69,
];

const PADDING: [u8; 129] = [
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
fn rot(x: u64, n: u32) -> u64 {
    x.rotate_right(n)
}

fn u8to64(p: &[u8]) -> u64 {
    let hi = ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32);
    let lo = ((p[4] as u32) << 24) | ((p[5] as u32) << 16) | ((p[6] as u32) << 8) | (p[7] as u32);
    ((hi as u64) << 32) | (lo as u64)
}

fn u64to8(p: &mut [u8], v: u64) {
    let hi = (v >> 32) as u32;
    let lo = v as u32;
    u32_to_bytes(&mut p[0..4], hi);
    u32_to_bytes(&mut p[4..8], lo);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_compress(s: *mut BlakeState512, block: *const u8) {
    let s = unsafe { &mut *s };
    let block = unsafe { slice::from_raw_parts(block, 128) };
    blake512_compress_inner(s, block);
}

pub fn blake512_compress_inner(s: &mut BlakeState512, block: &[u8]) {
    let mut m = [0u64; 16];
    for i in 0..16 {
        m[i] = u8to64(&block[8 * i..]);
    }

    let mut v = [0u64; 16];
    v[0] = s.h[0]; v[1] = s.h[1]; v[2] = s.h[2]; v[3] = s.h[3];
    v[4] = s.h[4]; v[5] = s.h[5]; v[6] = s.h[6]; v[7] = s.h[7];
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

    fn g(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, m: u64, k: u64) {
        v[a] = v[a].wrapping_add(m ^ k).wrapping_add(v[b]);
        v[d] = rot(v[d] ^ v[a], 32);
        v[c] = v[c].wrapping_add(v[d]);
        v[b] = rot(v[b] ^ v[c], 25);
    }
    fn g2(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, m: u64, k: u64) {
        v[a] = v[a].wrapping_add(m ^ k).wrapping_add(v[b]);
        v[d] = rot(v[d] ^ v[a], 16);
        v[c] = v[c].wrapping_add(v[d]);
        v[b] = rot(v[b] ^ v[c], 11);
    }

    const SIGMA: [[usize; 16]; 16] = [
        [ 0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15],
        [14, 10,  4,  8,  9, 15, 13,  6,  1, 12,  0,  2, 11,  7,  5,  3],
        [11,  8, 12,  0,  5,  2, 15, 13, 10, 14,  3,  6,  7,  1,  9,  4],
        [ 7,  9,  3,  1, 13, 12, 11, 14,  2,  6,  5, 10,  4,  0, 15,  8],
        [ 9,  0,  5,  7,  2,  4, 10, 15, 14,  1, 11, 12,  6,  8,  3, 13],
        [ 2, 12,  6, 10,  0, 11,  8,  3,  4, 13,  7,  5, 15, 14,  1,  9],
        [12,  5,  1, 15, 14, 13,  4, 10,  0,  7,  6,  3,  9,  2,  8, 11],
        [13, 11,  7, 14, 12,  1,  3,  9,  5,  0, 15,  4,  8,  6,  2, 10],
        [ 6, 15, 14,  9, 11,  3,  0,  8, 12,  2, 13,  7,  1,  4, 10,  5],
        [10,  2,  8,  4,  7,  6,  1,  5, 15, 11,  9, 14,  3, 12, 13,  0],
        [ 0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15],
        [14, 10,  4,  8,  9, 15, 13,  6,  1, 12,  0,  2, 11,  7,  5,  3],
        [11,  8, 12,  0,  5,  2, 15, 13, 10, 14,  3,  6,  7,  1,  9,  4],
        [ 7,  9,  3,  1, 13, 12, 11, 14,  2,  6,  5, 10,  4,  0, 15,  8],
        [ 9,  0,  5,  7,  2,  4, 10, 15, 14,  1, 11, 12,  6,  8,  3, 13],
        [ 2, 12,  6, 10,  0, 11,  8,  3,  4, 13,  7,  5, 15, 14,  1,  9],
    ];

    for r in 0..16 {
        let p = SIGMA[r];
        g(&mut v, 0, 4,  8, 12, m[p[ 0]], CST[p[ 1]]);
        g2(&mut v, 0, 4,  8, 12, m[p[ 1]], CST[p[ 0]]);
        g(&mut v, 1, 5,  9, 13, m[p[ 2]], CST[p[ 3]]);
        g2(&mut v, 1, 5,  9, 13, m[p[ 3]], CST[p[ 2]]);
        g(&mut v, 2, 6, 10, 14, m[p[ 4]], CST[p[ 5]]);
        g2(&mut v, 2, 6, 10, 14, m[p[ 5]], CST[p[ 4]]);
        g(&mut v, 3, 7, 11, 15, m[p[ 6]], CST[p[ 7]]);
        g2(&mut v, 3, 7, 11, 15, m[p[ 7]], CST[p[ 6]]);

        g(&mut v, 0, 5, 10, 15, m[p[ 8]], CST[p[ 9]]);
        g2(&mut v, 0, 5, 10, 15, m[p[ 9]], CST[p[ 8]]);
        g(&mut v, 1, 6, 11, 12, m[p[10]], CST[p[11]]);
        g2(&mut v, 1, 6, 11, 12, m[p[11]], CST[p[10]]);
        g(&mut v, 2, 7,  8, 13, m[p[12]], CST[p[13]]);
        g2(&mut v, 2, 7,  8, 13, m[p[13]], CST[p[12]]);
        g(&mut v, 3, 4,  9, 14, m[p[14]], CST[p[15]]);
        g2(&mut v, 3, 4,  9, 14, m[p[15]], CST[p[14]]);
    }

    for i in 0..8 {
        v[i] ^= v[i + 8];
    }
    for i in 0..8 {
        v[i] ^= s.s[i & 3];
    }
    for i in 0..8 {
        s.h[i] ^= v[i];
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_init(s: *mut BlakeState512) {
    let s = unsafe { &mut *s };
    blake512_init_inner(s);
}

pub fn blake512_init_inner(s: &mut BlakeState512) {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_update(s: *mut BlakeState512, data: *const u8, datalen: u64) {
    let s = unsafe { &mut *s };
    let data_slice = unsafe { slice::from_raw_parts(data, ((datalen + 7) / 8) as usize) };
    blake512_update_inner(s, data_slice, datalen);
}

pub fn blake512_update_inner(s: &mut BlakeState512, mut data: &[u8], mut datalen: u64) {
    let mut left = (s.buflen as usize) >> 3;
    let fill = 128 - left;

    if left != 0 && (((datalen >> 3) & 0x7F) as usize) >= fill {
        s.buf[left..left + fill].copy_from_slice(&data[..fill]);
        s.t[0] = s.t[0].wrapping_add(1024);
        let buf_copy = s.buf;
        blake512_compress_inner(s, &buf_copy);
        data = &data[fill..];
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 1024 {
        s.t[0] = s.t[0].wrapping_add(1024);
        let mut block = [0u8; 128];
        block.copy_from_slice(&data[..128]);
        blake512_compress_inner(s, &block);
        data = &data[128..];
        datalen -= 1024;
    }

    if datalen > 0 {
        let nbytes = ((datalen >> 3) & 0x7F) as usize;
        s.buf[left..left + nbytes].copy_from_slice(&data[..nbytes]);
        s.buflen = ((left as i32) << 3) + datalen as i32;
    } else {
        s.buflen = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_final(s: *mut BlakeState512, digest: *mut u8) {
    let s = unsafe { &mut *s };
    let digest = unsafe { slice::from_raw_parts_mut(digest, 64) };
    blake512_final_inner(s, digest);
}

pub fn blake512_final_inner(s: &mut BlakeState512, digest: &mut [u8]) {
    let mut msglen = [0u8; 16];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo = s.t[0].wrapping_add(s.buflen as u64);
    let mut hi = s.t[1];
    if lo < (s.buflen as u64) {
        hi = hi.wrapping_add(1);
    }
    u64to8(&mut msglen[0..8], hi);
    u64to8(&mut msglen[8..16], lo);

    if s.buflen == 888 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake512_update_inner(s, &[oo], 8);
    } else {
        if s.buflen < 888 {
            if s.buflen == 0 {
                s.nullt = 1;
            }
            let pad = 888 - s.buflen as u64;
            s.t[0] = s.t[0].wrapping_sub(pad);
            blake512_update_inner(s, &PADDING[..((pad + 7) / 8) as usize], pad);
        } else {
            let pad1 = 1024 - s.buflen as u64;
            s.t[0] = s.t[0].wrapping_sub(pad1);
            blake512_update_inner(s, &PADDING[..((pad1 + 7) / 8) as usize], pad1);
            s.t[0] = s.t[0].wrapping_sub(888);
            blake512_update_inner(s, &PADDING[1..1 + ((888 + 7) / 8)], 888);
            s.nullt = 1;
        }
        blake512_update_inner(s, &[zo], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(128);
    blake512_update_inner(s, &msglen, 128);

    for i in 0..8 {
        u64to8(&mut digest[8 * i..8 * i + 8], s.h[i]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512(out: *mut u8, input: *const u8, inlen: u64) -> i32 {
    let out = unsafe { slice::from_raw_parts_mut(out, 64) };
    let input = unsafe { slice::from_raw_parts(input, inlen as usize) };
    blake512_oneshot(out, input);
    0
}

pub fn blake512_oneshot(out: &mut [u8], input: &[u8]) {
    let mut s: BlakeState512 = unsafe { core::mem::zeroed() };
    blake512_init_inner(&mut s);
    blake512_update_inner(&mut s, input, (input.len() as u64) * 8);
    blake512_final_inner(&mut s, out);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_blake512_mgf1(
    out: *mut u8,
    outlen: u64,
    input: *const u8,
    inlen: u64,
) {
    let out = unsafe { slice::from_raw_parts_mut(out, outlen as usize) };
    let input = unsafe { slice::from_raw_parts(input, inlen as usize) };
    blake512_mgf1_inner(out, input);
}

pub fn blake512_mgf1_inner(out: &mut [u8], input: &[u8]) {
    let outlen = out.len();
    let inlen = input.len();
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    inbuf[..inlen].copy_from_slice(input);
    let mut i: u32 = 0;
    let mut out_off = 0usize;
    while ((i as usize) + 1) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..inlen + 4], i);
        let mut chunk = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
        blake512_oneshot(&mut chunk, &inbuf);
        out[out_off..out_off + SPX_BLAKE512_OUTPUT_BYTES].copy_from_slice(&chunk);
        out_off += SPX_BLAKE512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > (i as usize) * SPX_BLAKE512_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..inlen + 4], i);
        blake512_oneshot(&mut outbuf, &inbuf);
        let rem = outlen - (i as usize) * SPX_BLAKE512_OUTPUT_BYTES;
        out[out_off..out_off + rem].copy_from_slice(&outbuf[..rem]);
    }
}
