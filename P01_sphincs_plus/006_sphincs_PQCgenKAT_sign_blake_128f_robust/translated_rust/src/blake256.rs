use crate::params::*;
use crate::context::*;

const CST: [u32; 16] = [
    0x243F6A88,0x85A308D3,0x13198A2E,0x03707344,
    0xA4093822,0x299F31D0,0x082EFA98,0xEC4E6C89,
    0x452821E6,0x38D01377,0xBE5466CF,0x34E90C6C,
    0xC0AC29B7,0xC97C50DD,0x3F84D5B5,0xB5470917,
];

static PADDING: [u8; 64] = {
    let mut p = [0u8; 64];
    p[0] = 0x80;
    p
};

const SIGMA: [[usize; 16]; 14] = [
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
];

fn u8to32(p: &[u8]) -> u32 {
    ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32)
}

fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8; p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8; p[3] = v as u8;
}

fn rot(x: u32, n: u32) -> u32 { (x << (32 - n)) | (x >> n) }

#[derive(Clone)]
pub struct BlakeState256 {
    pub h: [u32; 8], pub s: [u32; 4], pub t: [u32; 2],
    pub buflen: i32, pub nullt: i32, pub buf: [u8; 64],
}

impl BlakeState256 {
    pub fn new() -> Self {
        BlakeState256 { h: [0;8], s: [0;4], t: [0;2], buflen: 0, nullt: 0, buf: [0;64] }
    }
}

fn g(v: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, m: &[u32; 16], si: usize, sj: usize) {
    v[a] = v[a].wrapping_add(m[si] ^ CST[sj]).wrapping_add(v[b]);
    v[d] = rot(v[d] ^ v[a], 16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = rot(v[b] ^ v[c], 12);
    v[a] = v[a].wrapping_add(m[sj] ^ CST[si]).wrapping_add(v[b]);
    v[d] = rot(v[d] ^ v[a], 8);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = rot(v[b] ^ v[c], 7);
}

pub fn blake256_compress(st: &mut BlakeState256, block: &[u8]) {
    let m: [u32; 16] = std::array::from_fn(|i| u8to32(&block[i*4..]));
    let mut v = [0u32; 16];
    for i in 0..8 { v[i] = st.h[i]; }
    v[8] = st.s[0] ^ CST[0]; v[9] = st.s[1] ^ CST[1];
    v[10] = st.s[2] ^ CST[2]; v[11] = st.s[3] ^ CST[3];
    v[12] = CST[4]; v[13] = CST[5]; v[14] = CST[6]; v[15] = CST[7];
    if st.nullt == 0 {
        v[12] ^= st.t[0]; v[13] ^= st.t[0];
        v[14] ^= st.t[1]; v[15] ^= st.t[1];
    }
    for r in 0..14 {
        let s = &SIGMA[r];
        g(&mut v, 0,4,8,12, &m, s[0], s[1]);
        g(&mut v, 1,5,9,13, &m, s[2], s[3]);
        g(&mut v, 2,6,10,14, &m, s[4], s[5]);
        g(&mut v, 3,7,11,15, &m, s[6], s[7]);
        g(&mut v, 0,5,10,15, &m, s[8], s[9]);
        g(&mut v, 1,6,11,12, &m, s[10], s[11]);
        g(&mut v, 2,7,8,13, &m, s[12], s[13]);
        g(&mut v, 3,4,9,14, &m, s[14], s[15]);
    }
    for i in 0..8 { v[i] ^= v[i+8]; }
    for i in 0..4 { v[i] ^= st.s[i]; v[i+4] ^= st.s[i]; }
    for i in 0..8 { st.h[i] ^= v[i]; }
}

pub fn blake256_init(s: &mut BlakeState256) {
    s.h = [0x6A09E667,0xBB67AE85,0x3C6EF372,0xA54FF53A,
           0x510E527F,0x9B05688C,0x1F83D9AB,0x5BE0CD19];
    s.t = [0;2]; s.buflen = 0; s.nullt = 0; s.s = [0;4];
}

pub fn blake256_update(s: &mut BlakeState256, data: &[u8], mut datalen: u64) {
    let mut left = (s.buflen >> 3) as usize;
    let fill = 64 - left;
    let mut off = 0usize;
    if left != 0 && ((datalen >> 3) & 0x3F) as usize >= fill {
        s.buf[left..left+fill].copy_from_slice(&data[off..off+fill]);
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 { s.t[1] = s.t[1].wrapping_add(1); }
        let buf = s.buf;
        blake256_compress(s, &buf);
        off += fill; datalen -= (fill as u64) << 3; left = 0;
    }
    while datalen >= 512 {
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 { s.t[1] = s.t[1].wrapping_add(1); }
        blake256_compress(s, &data[off..]);
        off += 64; datalen -= 512;
    }
    if datalen > 0 {
        let bytes = (datalen >> 3) as usize;
        s.buf[left..left+bytes].copy_from_slice(&data[off..off+bytes]);
        s.buflen = ((left << 3) as u64 + datalen) as i32;
    } else { s.buflen = 0; }
}

pub fn blake256_final(s: &mut BlakeState256, digest: &mut [u8]) {
    let lo = s.t[0].wrapping_add(s.buflen as u32);
    let mut hi = s.t[1];
    if lo < s.buflen as u32 { hi = hi.wrapping_add(1); }
    let mut msglen = [0u8; 8];
    u32to8(&mut msglen[0..4], hi);
    u32to8(&mut msglen[4..8], lo);
    if s.buflen == 440 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake256_update(s, &[0x81u8], 8);
    } else {
        if s.buflen < 440 {
            if s.buflen == 0 { s.nullt = 1; }
            s.t[0] = s.t[0].wrapping_sub((440 - s.buflen) as u32);
            let pad_bytes = ((440 - s.buflen) / 8) as usize;
            let pad: Vec<u8> = PADDING[..pad_bytes].to_vec();
            blake256_update(s, &pad, (440 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((512 - s.buflen) as u32);
            let pad_bytes = ((512 - s.buflen) / 8) as usize;
            let pad: Vec<u8> = PADDING[..pad_bytes].to_vec();
            blake256_update(s, &pad, (512 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(440);
            let pad2: Vec<u8> = PADDING[1..1+55].to_vec();
            blake256_update(s, &pad2, 440);
            s.nullt = 1;
        }
        blake256_update(s, &[0x01u8], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(64);
    let ml = msglen;
    blake256_update(s, &ml, 64);
    for i in 0..8 { u32to8(&mut digest[i*4..], s.h[i]); }
}

pub fn blake256(out: &mut [u8], inp: &[u8], inlen: u64) {
    let mut s = BlakeState256::new();
    blake256_init(&mut s);
    blake256_update(&mut s, inp, inlen * 8);
    blake256_final(&mut s, out);
}

pub fn blake256_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut i = 0u32;
    let mut off = 0usize;
    while ((i as usize) + 1) * SPX_BLAKE256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i);
        blake256(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += SPX_BLAKE256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > (i as usize) * SPX_BLAKE256_OUTPUT_BYTES {
        let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
        u32_to_bytes(&mut inbuf[inlen..], i);
        blake256(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - (i as usize) * SPX_BLAKE256_OUTPUT_BYTES;
        out[off..off+rem].copy_from_slice(&outbuf[..rem]);
    }
}
