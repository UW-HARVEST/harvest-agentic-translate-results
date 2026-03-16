use crate::params::*;

pub struct BlakeState512 {
    pub h: [u64; 8],
    pub s: [u64; 4],
    pub t: [u64; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 128],
}

fn u8to32(p: &[u8]) -> u32 {
    (u32::from(p[0]) << 24) | (u32::from(p[1]) << 16) | (u32::from(p[2]) << 8) | u32::from(p[3])
}

fn u8to64(p: &[u8]) -> u64 {
    ((u8to32(p) as u64) << 32) | (u8to32(&p[4..]) as u64)
}

fn u64to8(p: &mut [u8], v: u64) {
    p[0] = (v >> 56) as u8;
    p[1] = (v >> 48) as u8;
    p[2] = (v >> 40) as u8;
    p[3] = (v >> 32) as u8;
    p[4] = (v >> 24) as u8;
    p[5] = (v >> 16) as u8;
    p[6] = (v >> 8) as u8;
    p[7] = v as u8;
}

#[allow(dead_code)]
const CST: [u64; 16] = [
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

#[inline(always)]
fn rot(x: u64, n: u32) -> u64 {
    (x << (64 - n)) | (x >> n)
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

    macro_rules! blake512_round {
        ($m0:expr,$c0:expr,$m1:expr,$c1:expr,$m2:expr,$c2:expr,$m3:expr,$c3:expr,
         $m4:expr,$c4:expr,$m5:expr,$c5:expr,$m6:expr,$c6:expr,$m7:expr,$c7:expr,
         $m8:expr,$c8:expr,$m9:expr,$c9:expr,$m10:expr,$c10:expr,$m11:expr,$c11:expr,
         $m12:expr,$c12:expr,$m13:expr,$c13:expr,$m14:expr,$c14:expr,$m15:expr,$c15:expr) => {
            v[0] = v[0].wrapping_add($m0 ^ $c0).wrapping_add(v[4]);
            v[12] ^= v[0]; v[12] = rot(v[12], 32);
            v[8] = v[8].wrapping_add(v[12]);
            v[4] ^= v[8]; v[4] = rot(v[4], 25);

            v[1] = v[1].wrapping_add($m2 ^ $c2).wrapping_add(v[5]);
            v[13] ^= v[1]; v[13] = rot(v[13], 32);
            v[9] = v[9].wrapping_add(v[13]);
            v[5] ^= v[9]; v[5] = rot(v[5], 25);

            v[2] = v[2].wrapping_add($m4 ^ $c4).wrapping_add(v[6]);
            v[14] ^= v[2]; v[14] = rot(v[14], 32);
            v[10] = v[10].wrapping_add(v[14]);
            v[6] ^= v[10]; v[6] = rot(v[6], 25);

            v[3] = v[3].wrapping_add($m6 ^ $c6).wrapping_add(v[7]);
            v[15] ^= v[3]; v[15] = rot(v[15], 32);
            v[11] = v[11].wrapping_add(v[15]);
            v[7] ^= v[11]; v[7] = rot(v[7], 25);

            v[2] = v[2].wrapping_add($m5 ^ $c5).wrapping_add(v[6]);
            v[14] ^= v[2]; v[14] = rot(v[14], 16);
            v[10] = v[10].wrapping_add(v[14]);
            v[6] ^= v[10]; v[6] = rot(v[6], 11);

            v[3] = v[3].wrapping_add($m7 ^ $c7).wrapping_add(v[7]);
            v[15] ^= v[3]; v[15] = rot(v[15], 16);
            v[11] = v[11].wrapping_add(v[15]);
            v[7] ^= v[11]; v[7] = rot(v[7], 11);

            v[1] = v[1].wrapping_add($m3 ^ $c3).wrapping_add(v[5]);
            v[13] ^= v[1]; v[13] = rot(v[13], 16);
            v[9] = v[9].wrapping_add(v[13]);
            v[5] ^= v[9]; v[5] = rot(v[5], 11);

            v[0] = v[0].wrapping_add($m1 ^ $c1).wrapping_add(v[4]);
            v[12] ^= v[0]; v[12] = rot(v[12], 16);
            v[8] = v[8].wrapping_add(v[12]);
            v[4] ^= v[8]; v[4] = rot(v[4], 11);

            // Diagonal
            v[0] = v[0].wrapping_add($m8 ^ $c8).wrapping_add(v[5]);
            v[15] ^= v[0]; v[15] = rot(v[15], 32);
            v[10] = v[10].wrapping_add(v[15]);
            v[5] ^= v[10]; v[5] = rot(v[5], 25);

            v[1] = v[1].wrapping_add($m10 ^ $c10).wrapping_add(v[6]);
            v[12] ^= v[1]; v[12] = rot(v[12], 32);
            v[11] = v[11].wrapping_add(v[12]);
            v[6] ^= v[11]; v[6] = rot(v[6], 25);

            v[2] = v[2].wrapping_add($m12 ^ $c12).wrapping_add(v[7]);
            v[13] ^= v[2]; v[13] = rot(v[13], 32);
            v[8] = v[8].wrapping_add(v[13]);
            v[7] ^= v[8]; v[7] = rot(v[7], 25);

            v[3] = v[3].wrapping_add($m14 ^ $c14).wrapping_add(v[4]);
            v[14] ^= v[3]; v[14] = rot(v[14], 32);
            v[9] = v[9].wrapping_add(v[14]);
            v[4] ^= v[9]; v[4] = rot(v[4], 25);

            v[2] = v[2].wrapping_add($m13 ^ $c13).wrapping_add(v[7]);
            v[13] ^= v[2]; v[13] = rot(v[13], 16);
            v[8] = v[8].wrapping_add(v[13]);
            v[7] ^= v[8]; v[7] = rot(v[7], 11);

            v[3] = v[3].wrapping_add($m15 ^ $c15).wrapping_add(v[4]);
            v[14] ^= v[3]; v[14] = rot(v[14], 16);
            v[9] = v[9].wrapping_add(v[14]);
            v[4] ^= v[9]; v[4] = rot(v[4], 11);

            v[1] = v[1].wrapping_add($m11 ^ $c11).wrapping_add(v[6]);
            v[12] ^= v[1]; v[12] = rot(v[12], 16);
            v[11] = v[11].wrapping_add(v[12]);
            v[6] ^= v[11]; v[6] = rot(v[6], 11);

            v[0] = v[0].wrapping_add($m9 ^ $c9).wrapping_add(v[5]);
            v[15] ^= v[0]; v[15] = rot(v[15], 16);
            v[10] = v[10].wrapping_add(v[15]);
            v[5] ^= v[10]; v[5] = rot(v[5], 11);
        };
    }

    blake512_round!(m[0],CST[1],m[1],CST[0],m[2],CST[3],m[3],CST[2],m[4],CST[5],m[5],CST[4],m[6],CST[7],m[7],CST[6],m[8],CST[9],m[9],CST[8],m[10],CST[11],m[11],CST[10],m[12],CST[13],m[13],CST[12],m[14],CST[15],m[15],CST[14]);
    blake512_round!(m[14],CST[10],m[10],CST[14],m[4],CST[8],m[8],CST[4],m[9],CST[15],m[15],CST[9],m[13],CST[6],m[6],CST[13],m[1],CST[12],m[12],CST[1],m[0],CST[2],m[2],CST[0],m[11],CST[7],m[7],CST[11],m[5],CST[3],m[3],CST[5]);
    blake512_round!(m[11],CST[8],m[8],CST[11],m[12],CST[0],m[0],CST[12],m[5],CST[2],m[2],CST[5],m[15],CST[13],m[13],CST[15],m[10],CST[14],m[14],CST[10],m[3],CST[6],m[6],CST[3],m[7],CST[1],m[1],CST[7],m[9],CST[4],m[4],CST[9]);
    blake512_round!(m[7],CST[9],m[9],CST[7],m[3],CST[1],m[1],CST[3],m[13],CST[12],m[12],CST[13],m[11],CST[14],m[14],CST[11],m[2],CST[6],m[6],CST[2],m[5],CST[10],m[10],CST[5],m[4],CST[0],m[0],CST[4],m[15],CST[8],m[8],CST[15]);
    blake512_round!(m[9],CST[0],m[0],CST[9],m[5],CST[7],m[7],CST[5],m[2],CST[4],m[4],CST[2],m[10],CST[15],m[15],CST[10],m[14],CST[1],m[1],CST[14],m[11],CST[12],m[12],CST[11],m[6],CST[8],m[8],CST[6],m[3],CST[13],m[13],CST[3]);
    blake512_round!(m[2],CST[12],m[12],CST[2],m[6],CST[10],m[10],CST[6],m[0],CST[11],m[11],CST[0],m[8],CST[3],m[3],CST[8],m[4],CST[13],m[13],CST[4],m[7],CST[5],m[5],CST[7],m[15],CST[14],m[14],CST[15],m[1],CST[9],m[9],CST[1]);
    blake512_round!(m[12],CST[5],m[5],CST[12],m[1],CST[15],m[15],CST[1],m[14],CST[13],m[13],CST[14],m[4],CST[10],m[10],CST[4],m[0],CST[7],m[7],CST[0],m[6],CST[3],m[3],CST[6],m[9],CST[2],m[2],CST[9],m[8],CST[11],m[11],CST[8]);
    blake512_round!(m[13],CST[11],m[11],CST[13],m[7],CST[14],m[14],CST[7],m[12],CST[1],m[1],CST[12],m[3],CST[9],m[9],CST[3],m[5],CST[0],m[0],CST[5],m[15],CST[4],m[4],CST[15],m[8],CST[6],m[6],CST[8],m[2],CST[10],m[10],CST[2]);
    blake512_round!(m[6],CST[15],m[15],CST[6],m[14],CST[9],m[9],CST[14],m[11],CST[3],m[3],CST[11],m[0],CST[8],m[8],CST[0],m[12],CST[2],m[2],CST[12],m[13],CST[7],m[7],CST[13],m[1],CST[4],m[4],CST[1],m[10],CST[5],m[5],CST[10]);
    blake512_round!(m[10],CST[2],m[2],CST[10],m[8],CST[4],m[4],CST[8],m[7],CST[6],m[6],CST[7],m[1],CST[5],m[5],CST[1],m[15],CST[11],m[11],CST[15],m[9],CST[14],m[14],CST[9],m[3],CST[12],m[12],CST[3],m[13],CST[0],m[0],CST[13]);
    blake512_round!(m[0],CST[1],m[1],CST[0],m[2],CST[3],m[3],CST[2],m[4],CST[5],m[5],CST[4],m[6],CST[7],m[7],CST[6],m[8],CST[9],m[9],CST[8],m[10],CST[11],m[11],CST[10],m[12],CST[13],m[13],CST[12],m[14],CST[15],m[15],CST[14]);
    blake512_round!(m[14],CST[10],m[10],CST[14],m[4],CST[8],m[8],CST[4],m[9],CST[15],m[15],CST[9],m[13],CST[6],m[6],CST[13],m[1],CST[12],m[12],CST[1],m[0],CST[2],m[2],CST[0],m[11],CST[7],m[7],CST[11],m[5],CST[3],m[3],CST[5]);
    blake512_round!(m[11],CST[8],m[8],CST[11],m[12],CST[0],m[0],CST[12],m[5],CST[2],m[2],CST[5],m[15],CST[13],m[13],CST[15],m[10],CST[14],m[14],CST[10],m[3],CST[6],m[6],CST[3],m[7],CST[1],m[1],CST[7],m[9],CST[4],m[4],CST[9]);
    blake512_round!(m[7],CST[9],m[9],CST[7],m[3],CST[1],m[1],CST[3],m[13],CST[12],m[12],CST[13],m[11],CST[14],m[14],CST[11],m[2],CST[6],m[6],CST[2],m[5],CST[10],m[10],CST[5],m[4],CST[0],m[0],CST[4],m[15],CST[8],m[8],CST[15]);
    blake512_round!(m[9],CST[0],m[0],CST[9],m[5],CST[7],m[7],CST[5],m[2],CST[4],m[4],CST[2],m[10],CST[15],m[15],CST[10],m[14],CST[1],m[1],CST[14],m[11],CST[12],m[12],CST[11],m[6],CST[8],m[8],CST[6],m[3],CST[13],m[13],CST[3]);
    blake512_round!(m[2],CST[12],m[12],CST[2],m[6],CST[10],m[10],CST[6],m[0],CST[11],m[11],CST[0],m[8],CST[3],m[3],CST[8],m[4],CST[13],m[13],CST[4],m[7],CST[5],m[5],CST[7],m[15],CST[14],m[14],CST[15],m[1],CST[9],m[9],CST[1]);

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
    s.s = [0, 0, 0, 0];
}

pub fn blake512_update(s: &mut BlakeState512, data: &[u8], datalen: u64) {
    let mut data = data;
    let mut datalen = datalen;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 128 - left;

    if left != 0 && ((datalen >> 3) & 0x7F) as usize >= fill {
        s.buf[left..left + fill].copy_from_slice(&data[..fill]);
        s.t[0] = s.t[0].wrapping_add(1024);
        let buf_copy = s.buf;
        blake512_compress(s, &buf_copy);
        data = &data[fill..];
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 1024 {
        s.t[0] = s.t[0].wrapping_add(1024);
        blake512_compress(s, &data[..128]);
        data = &data[128..];
        datalen -= 1024;
    }

    if datalen > 0 {
        let bytes = ((datalen >> 3) & 0x7F) as usize;
        s.buf[left..left + bytes].copy_from_slice(&data[..bytes]);
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
    u64to8(&mut msglen[0..8], hi);
    u64to8(&mut msglen[8..16], lo);

    if s.buflen == 888 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake512_update(s, &[oo], 8);
    } else {
        if s.buflen < 888 {
            if s.buflen == 0 { s.nullt = 1; }
            s.t[0] = s.t[0].wrapping_sub((888 - s.buflen) as u64);
            blake512_update(s, &PADDING[..(888 - s.buflen) as usize / 8], (888 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((1024 - s.buflen) as u64);
            blake512_update(s, &PADDING[..(1024 - s.buflen) as usize / 8], (1024 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(888);
            blake512_update(s, &PADDING[1..1 + 888 / 8], 888);
            s.nullt = 1;
        }
        blake512_update(s, &[zo], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(128);
    blake512_update(s, &msglen, 128);

    for i in 0..8 {
        u64to8(&mut digest[i * 8..i * 8 + 8], s.h[i]);
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

    let mut i: u32 = 0;
    while ((i as usize) + 1) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..inlen + 4], i);
        let start = i as usize * SPX_BLAKE512_OUTPUT_BYTES;
        blake512(&mut out[start..], &inbuf, (inlen + 4) as u64);
        i += 1;
    }
    if outlen > i as usize * SPX_BLAKE512_OUTPUT_BYTES {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..inlen + 4], i);
        blake512(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let start = i as usize * SPX_BLAKE512_OUTPUT_BYTES;
        let remaining = outlen - start;
        out[start..start + remaining].copy_from_slice(&outbuf[..remaining]);
    }
}
