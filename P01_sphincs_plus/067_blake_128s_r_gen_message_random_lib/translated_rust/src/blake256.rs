use crate::params::SPX_BLAKE256_OUTPUT_BYTES;
use crate::utils::u32_to_bytes;

#[derive(Clone)]
pub struct BlakeState256 {
    pub h: [u32; 8],
    pub s: [u32; 4],
    pub t: [u32; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 64],
}

const CST: [u32; 16] = [
    0x243F6A88, 0x85A308D3, 0x13198A2E, 0x03707344,
    0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    0x452821E6, 0x38D01377, 0xBE5466CF, 0x34E90C6C,
    0xC0AC29B7, 0xC97C50DD, 0x3F84D5B5, 0xB5470917,
];

static PADDING: [u8; 64] = [
    0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

fn u8to32(p: &[u8]) -> u32 {
    (p[0] as u32) << 24 | (p[1] as u32) << 16 | (p[2] as u32) << 8 | (p[3] as u32)
}

fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8;
    p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8;
    p[3] = v as u8;
}

#[inline(always)]
fn rot(x: u32, n: u32) -> u32 {
    (x << (32 - n)) | (x >> n)
}

pub fn blake256_compress(s: &mut BlakeState256, block: &[u8]) {
    let m: [u32; 16] = [
        u8to32(&block[0..]), u8to32(&block[4..]), u8to32(&block[8..]), u8to32(&block[12..]),
        u8to32(&block[16..]), u8to32(&block[20..]), u8to32(&block[24..]), u8to32(&block[28..]),
        u8to32(&block[32..]), u8to32(&block[36..]), u8to32(&block[40..]), u8to32(&block[44..]),
        u8to32(&block[48..]), u8to32(&block[52..]), u8to32(&block[56..]), u8to32(&block[60..]),
    ];

    let mut v: [u32; 16] = [
        s.h[0], s.h[1], s.h[2], s.h[3],
        s.h[4], s.h[5], s.h[6], s.h[7],
        s.s[0] ^ 0x243F6A88, s.s[1] ^ 0x85A308D3,
        s.s[2] ^ 0x13198A2E, s.s[3] ^ 0x03707344,
        0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    ];

    if s.nullt == 0 {
        v[12] ^= s.t[0];
        v[13] ^= s.t[0];
        v[14] ^= s.t[1];
        v[15] ^= s.t[1];
    }

    // The C ROUND macro takes pairs (message_word, constant_index) for each of 8 G calls.
    // Each G call: G(a,b,c,d, m_i^c_j, m_k^c_l)
    // The ROUND macro parameter order is:
    //   m0,c0, m1,c1, m2,c2, m3,c3, m4,c4, m5,c5, m6,c6, m7,c7,
    //   m8,c8, m9,c9, m10,c10, m11,c11, m12,c12, m13,c13, m14,c14, m15,c15
    // Column step:
    //   G(0,4,8,12):  first_add = m0^c0, second_add = m1^c1
    //   G(1,5,9,13):  first_add = m2^c2, second_add = m3^c3
    //   G(2,6,10,14): first_add = m4^c4, second_add = m5^c5
    //   G(3,7,11,15): first_add = m6^c6, second_add = m7^c7
    // Diagonal step:
    //   G(0,5,10,15): first_add = m8^c8, second_add = m9^c9
    //   G(1,6,11,12): first_add = m10^c10, second_add = m11^c11
    //   G(2,7,8,13):  first_add = m12^c12, second_add = m13^c13
    //   G(3,4,9,14):  first_add = m14^c14, second_add = m15^c15

    // Each round is specified as 16 pairs: (msg_idx, cst_idx)
    // Extracted from the 14 ROUND macro invocations in the C code:
    #[rustfmt::skip]
    const ROUNDS: [[(usize, usize); 16]; 14] = [
        // ROUND 0: m0,cst[1], m1,cst[0], m2,cst[3], m3,cst[2], m4,cst[5], m5,cst[4], m6,cst[7], m7,cst[6], m8,cst[9], m9,cst[8], m10,cst[11], m11,cst[10], m12,cst[13], m13,cst[12], m14,cst[15], m15,cst[14]
        [(0,1),(1,0),(2,3),(3,2),(4,5),(5,4),(6,7),(7,6),(8,9),(9,8),(10,11),(11,10),(12,13),(13,12),(14,15),(15,14)],
        // ROUND 1: m14,cst[10], m10,cst[14], m4,cst[8], m8,cst[4], m9,cst[15], m15,cst[9], m13,cst[6], m6,cst[13], m1,cst[12], m12,cst[1], m0,cst[2], m2,cst[0], m11,cst[7], m7,cst[11], m5,cst[3], m3,cst[5]
        [(14,10),(10,14),(4,8),(8,4),(9,15),(15,9),(13,6),(6,13),(1,12),(12,1),(0,2),(2,0),(11,7),(7,11),(5,3),(3,5)],
        // ROUND 2
        [(11,8),(8,11),(12,0),(0,12),(5,2),(2,5),(15,13),(13,15),(10,14),(14,10),(3,6),(6,3),(7,1),(1,7),(9,4),(4,9)],
        // ROUND 3
        [(7,9),(9,7),(3,1),(1,3),(13,12),(12,13),(11,14),(14,11),(2,6),(6,2),(5,10),(10,5),(4,0),(0,4),(15,8),(8,15)],
        // ROUND 4
        [(9,0),(0,9),(5,7),(7,5),(2,4),(4,2),(10,15),(15,10),(14,1),(1,14),(11,12),(12,11),(6,8),(8,6),(3,13),(13,3)],
        // ROUND 5
        [(2,12),(12,2),(6,10),(10,6),(0,11),(11,0),(8,3),(3,8),(4,13),(13,4),(7,5),(5,7),(15,14),(14,15),(1,9),(9,1)],
        // ROUND 6
        [(12,5),(5,12),(1,15),(15,1),(14,13),(13,14),(4,10),(10,4),(0,7),(7,0),(6,3),(3,6),(9,2),(2,9),(8,11),(11,8)],
        // ROUND 7
        [(13,11),(11,13),(7,14),(14,7),(12,1),(1,12),(3,9),(9,3),(5,0),(0,5),(15,4),(4,15),(8,6),(6,8),(2,10),(10,2)],
        // ROUND 8
        [(6,15),(15,6),(14,9),(9,14),(11,3),(3,11),(0,8),(8,0),(12,2),(2,12),(13,7),(7,13),(1,4),(4,1),(10,5),(5,10)],
        // ROUND 9
        [(10,2),(2,10),(8,4),(4,8),(7,6),(6,7),(1,5),(5,1),(15,11),(11,15),(9,14),(14,9),(3,12),(12,3),(13,0),(0,13)],
        // ROUND 10 = ROUND 0
        [(0,1),(1,0),(2,3),(3,2),(4,5),(5,4),(6,7),(7,6),(8,9),(9,8),(10,11),(11,10),(12,13),(13,12),(14,15),(15,14)],
        // ROUND 11 = ROUND 1
        [(14,10),(10,14),(4,8),(8,4),(9,15),(15,9),(13,6),(6,13),(1,12),(12,1),(0,2),(2,0),(11,7),(7,11),(5,3),(3,5)],
        // ROUND 12 = ROUND 2
        [(11,8),(8,11),(12,0),(0,12),(5,2),(2,5),(15,13),(13,15),(10,14),(14,10),(3,6),(6,3),(7,1),(1,7),(9,4),(4,9)],
        // ROUND 13 = ROUND 3
        [(7,9),(9,7),(3,1),(1,3),(13,12),(12,13),(11,14),(14,11),(2,6),(6,2),(5,10),(10,5),(4,0),(0,4),(15,8),(8,15)],
    ];

    for round in 0..14 {
        let r = &ROUNDS[round];
        // Column step
        // G(0,4,8,12)
        v[0] = v[0].wrapping_add(m[r[0].0] ^ CST[r[0].1]).wrapping_add(v[4]);
        v[12] ^= v[0]; v[12] = rot(v[12], 16);
        v[8] = v[8].wrapping_add(v[12]);
        v[4] ^= v[8]; v[4] = rot(v[4], 12);
        v[0] = v[0].wrapping_add(m[r[1].0] ^ CST[r[1].1]).wrapping_add(v[4]);
        v[12] ^= v[0]; v[12] = rot(v[12], 8);
        v[8] = v[8].wrapping_add(v[12]);
        v[4] ^= v[8]; v[4] = rot(v[4], 7);
        // G(1,5,9,13)
        v[1] = v[1].wrapping_add(m[r[2].0] ^ CST[r[2].1]).wrapping_add(v[5]);
        v[13] ^= v[1]; v[13] = rot(v[13], 16);
        v[9] = v[9].wrapping_add(v[13]);
        v[5] ^= v[9]; v[5] = rot(v[5], 12);
        v[1] = v[1].wrapping_add(m[r[3].0] ^ CST[r[3].1]).wrapping_add(v[5]);
        v[13] ^= v[1]; v[13] = rot(v[13], 8);
        v[9] = v[9].wrapping_add(v[13]);
        v[5] ^= v[9]; v[5] = rot(v[5], 7);
        // G(2,6,10,14)
        v[2] = v[2].wrapping_add(m[r[4].0] ^ CST[r[4].1]).wrapping_add(v[6]);
        v[14] ^= v[2]; v[14] = rot(v[14], 16);
        v[10] = v[10].wrapping_add(v[14]);
        v[6] ^= v[10]; v[6] = rot(v[6], 12);
        v[2] = v[2].wrapping_add(m[r[5].0] ^ CST[r[5].1]).wrapping_add(v[6]);
        v[14] ^= v[2]; v[14] = rot(v[14], 8);
        v[10] = v[10].wrapping_add(v[14]);
        v[6] ^= v[10]; v[6] = rot(v[6], 7);
        // G(3,7,11,15)
        v[3] = v[3].wrapping_add(m[r[6].0] ^ CST[r[6].1]).wrapping_add(v[7]);
        v[15] ^= v[3]; v[15] = rot(v[15], 16);
        v[11] = v[11].wrapping_add(v[15]);
        v[7] ^= v[11]; v[7] = rot(v[7], 12);
        v[3] = v[3].wrapping_add(m[r[7].0] ^ CST[r[7].1]).wrapping_add(v[7]);
        v[15] ^= v[3]; v[15] = rot(v[15], 8);
        v[11] = v[11].wrapping_add(v[15]);
        v[7] ^= v[11]; v[7] = rot(v[7], 7);
        // Diagonal step
        // G(0,5,10,15)
        v[0] = v[0].wrapping_add(m[r[8].0] ^ CST[r[8].1]).wrapping_add(v[5]);
        v[15] ^= v[0]; v[15] = rot(v[15], 16);
        v[10] = v[10].wrapping_add(v[15]);
        v[5] ^= v[10]; v[5] = rot(v[5], 12);
        v[0] = v[0].wrapping_add(m[r[9].0] ^ CST[r[9].1]).wrapping_add(v[5]);
        v[15] ^= v[0]; v[15] = rot(v[15], 8);
        v[10] = v[10].wrapping_add(v[15]);
        v[5] ^= v[10]; v[5] = rot(v[5], 7);
        // G(1,6,11,12)
        v[1] = v[1].wrapping_add(m[r[10].0] ^ CST[r[10].1]).wrapping_add(v[6]);
        v[12] ^= v[1]; v[12] = rot(v[12], 16);
        v[11] = v[11].wrapping_add(v[12]);
        v[6] ^= v[11]; v[6] = rot(v[6], 12);
        v[1] = v[1].wrapping_add(m[r[11].0] ^ CST[r[11].1]).wrapping_add(v[6]);
        v[12] ^= v[1]; v[12] = rot(v[12], 8);
        v[11] = v[11].wrapping_add(v[12]);
        v[6] ^= v[11]; v[6] = rot(v[6], 7);
        // G(2,7,8,13)
        v[2] = v[2].wrapping_add(m[r[12].0] ^ CST[r[12].1]).wrapping_add(v[7]);
        v[13] ^= v[2]; v[13] = rot(v[13], 16);
        v[8] = v[8].wrapping_add(v[13]);
        v[7] ^= v[8]; v[7] = rot(v[7], 12);
        v[2] = v[2].wrapping_add(m[r[13].0] ^ CST[r[13].1]).wrapping_add(v[7]);
        v[13] ^= v[2]; v[13] = rot(v[13], 8);
        v[8] = v[8].wrapping_add(v[13]);
        v[7] ^= v[8]; v[7] = rot(v[7], 7);
        // G(3,4,9,14)
        v[3] = v[3].wrapping_add(m[r[14].0] ^ CST[r[14].1]).wrapping_add(v[4]);
        v[14] ^= v[3]; v[14] = rot(v[14], 16);
        v[9] = v[9].wrapping_add(v[14]);
        v[4] ^= v[9]; v[4] = rot(v[4], 12);
        v[3] = v[3].wrapping_add(m[r[15].0] ^ CST[r[15].1]).wrapping_add(v[4]);
        v[14] ^= v[3]; v[14] = rot(v[14], 8);
        v[9] = v[9].wrapping_add(v[14]);
        v[4] ^= v[9]; v[4] = rot(v[4], 7);
    }

    // Finalization
    for i in 0..8 {
        v[i] ^= v[i + 8];
    }
    for i in 0..8 {
        v[i] ^= s.s[i % 4];
    }
    for i in 0..8 {
        s.h[i] ^= v[i];
    }
}

pub fn blake256_init(s: &mut BlakeState256) {
    s.h = [
        0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
        0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
    ];
    s.t = [0, 0];
    s.buflen = 0;
    s.nullt = 0;
    s.s = [0, 0, 0, 0];
}

pub fn blake256_update(s: &mut BlakeState256, data: &[u8], datalen_bits: u64) {
    let mut datalen = datalen_bits;
    let mut offset = 0usize;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 64 - left;

    if left != 0 && ((datalen >> 3) & 0x3F) >= fill as u64 {
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

pub fn blake256_final(s: &mut BlakeState256, digest: &mut [u8]) {
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
            blake256_update(s, &PADDING[..], (440 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((512 - s.buflen) as u32);
            blake256_update(s, &PADDING[..], (512 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(440);
            blake256_update(s, &PADDING[1..], 440);
            s.nullt = 1;
        }
        blake256_update(s, &[zo], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(64);
    blake256_update(s, &msglen, 64);

    for i in 0..8 {
        u32to8(&mut digest[i * 4..], s.h[i]);
    }
}

pub fn blake256(out: &mut [u8], inp: &[u8], inlen: u64) -> i32 {
    let mut s = BlakeState256 {
        h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 64],
    };
    blake256_init(&mut s);
    blake256_update(&mut s, inp, inlen.wrapping_mul(8));
    blake256_final(&mut s, out);
    0
}

pub fn blake256_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut i: usize = 0;
    let mut off = 0usize;

    while (i + 1) * SPX_BLAKE256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake256(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += SPX_BLAKE256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_BLAKE256_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake256(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - i * SPX_BLAKE256_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}
