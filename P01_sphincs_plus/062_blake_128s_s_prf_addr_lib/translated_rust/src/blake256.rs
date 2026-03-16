use crate::params::*;

pub struct BlakeState256 {
    pub h: [u32; 8],
    pub s: [u32; 4],
    pub t: [u32; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 64],
}

fn u8to32(p: &[u8]) -> u32 {
    (u32::from(p[0]) << 24) | (u32::from(p[1]) << 16) | (u32::from(p[2]) << 8) | u32::from(p[3])
}

fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8;
    p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8;
    p[3] = v as u8;
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

    // The ROUND macro from C, inlined as a function
    macro_rules! blake256_round {
        ($m0:expr,$c0:expr,$m1:expr,$c1:expr,$m2:expr,$c2:expr,$m3:expr,$c3:expr,
         $m4:expr,$c4:expr,$m5:expr,$c5:expr,$m6:expr,$c6:expr,$m7:expr,$c7:expr,
         $m8:expr,$c8:expr,$m9:expr,$c9:expr,$m10:expr,$c10:expr,$m11:expr,$c11:expr,
         $m12:expr,$c12:expr,$m13:expr,$c13:expr,$m14:expr,$c14:expr,$m15:expr,$c15:expr) => {
            // Column step
            v[0] = v[0].wrapping_add($m0 ^ $c0).wrapping_add(v[4]);
            v[12] ^= v[0]; v[12] = rot(v[12], 16);
            v[8] = v[8].wrapping_add(v[12]);
            v[4] ^= v[8]; v[4] = rot(v[4], 12);

            v[1] = v[1].wrapping_add($m2 ^ $c2).wrapping_add(v[5]);
            v[13] ^= v[1]; v[13] = rot(v[13], 16);
            v[9] = v[9].wrapping_add(v[13]);
            v[5] ^= v[9]; v[5] = rot(v[5], 12);

            v[2] = v[2].wrapping_add($m4 ^ $c4).wrapping_add(v[6]);
            v[14] ^= v[2]; v[14] = rot(v[14], 16);
            v[10] = v[10].wrapping_add(v[14]);
            v[6] ^= v[10]; v[6] = rot(v[6], 12);

            v[3] = v[3].wrapping_add($m6 ^ $c6).wrapping_add(v[7]);
            v[15] ^= v[3]; v[15] = rot(v[15], 16);
            v[11] = v[11].wrapping_add(v[15]);
            v[7] ^= v[11]; v[7] = rot(v[7], 12);

            v[2] = v[2].wrapping_add($m5 ^ $c5).wrapping_add(v[6]);
            v[14] ^= v[2]; v[14] = rot(v[14], 8);
            v[10] = v[10].wrapping_add(v[14]);
            v[6] ^= v[10]; v[6] = rot(v[6], 7);

            v[3] = v[3].wrapping_add($m7 ^ $c7).wrapping_add(v[7]);
            v[15] ^= v[3]; v[15] = rot(v[15], 8);
            v[11] = v[11].wrapping_add(v[15]);
            v[7] ^= v[11]; v[7] = rot(v[7], 7);

            v[1] = v[1].wrapping_add($m3 ^ $c3).wrapping_add(v[5]);
            v[13] ^= v[1]; v[13] = rot(v[13], 8);
            v[9] = v[9].wrapping_add(v[13]);
            v[5] ^= v[9]; v[5] = rot(v[5], 7);

            v[0] = v[0].wrapping_add($m1 ^ $c1).wrapping_add(v[4]);
            v[12] ^= v[0]; v[12] = rot(v[12], 8);
            v[8] = v[8].wrapping_add(v[12]);
            v[4] ^= v[8]; v[4] = rot(v[4], 7);

            // Diagonal step
            v[0] = v[0].wrapping_add($m8 ^ $c8).wrapping_add(v[5]);
            v[15] ^= v[0]; v[15] = rot(v[15], 16);
            v[10] = v[10].wrapping_add(v[15]);
            v[5] ^= v[10]; v[5] = rot(v[5], 12);

            v[1] = v[1].wrapping_add($m10 ^ $c10).wrapping_add(v[6]);
            v[12] ^= v[1]; v[12] = rot(v[12], 16);
            v[11] = v[11].wrapping_add(v[12]);
            v[6] ^= v[11]; v[6] = rot(v[6], 12);

            v[2] = v[2].wrapping_add($m12 ^ $c12).wrapping_add(v[7]);
            v[13] ^= v[2]; v[13] = rot(v[13], 16);
            v[8] = v[8].wrapping_add(v[13]);
            v[7] ^= v[8]; v[7] = rot(v[7], 12);

            v[3] = v[3].wrapping_add($m14 ^ $c14).wrapping_add(v[4]);
            v[14] ^= v[3]; v[14] = rot(v[14], 16);
            v[9] = v[9].wrapping_add(v[14]);
            v[4] ^= v[9]; v[4] = rot(v[4], 12);

            v[2] = v[2].wrapping_add($m13 ^ $c13).wrapping_add(v[7]);
            v[13] ^= v[2]; v[13] = rot(v[13], 8);
            v[8] = v[8].wrapping_add(v[13]);
            v[7] ^= v[8]; v[7] = rot(v[7], 7);

            v[3] = v[3].wrapping_add($m15 ^ $c15).wrapping_add(v[4]);
            v[14] ^= v[3]; v[14] = rot(v[14], 8);
            v[9] = v[9].wrapping_add(v[14]);
            v[4] ^= v[9]; v[4] = rot(v[4], 7);

            v[1] = v[1].wrapping_add($m11 ^ $c11).wrapping_add(v[6]);
            v[12] ^= v[1]; v[12] = rot(v[12], 8);
            v[11] = v[11].wrapping_add(v[12]);
            v[6] ^= v[11]; v[6] = rot(v[6], 7);

            v[0] = v[0].wrapping_add($m9 ^ $c9).wrapping_add(v[5]);
            v[15] ^= v[0]; v[15] = rot(v[15], 8);
            v[10] = v[10].wrapping_add(v[15]);
            v[5] ^= v[10]; v[5] = rot(v[5], 7);
        };
    }

    blake256_round!(m[0],CST[1],m[1],CST[0],m[2],CST[3],m[3],CST[2],m[4],CST[5],m[5],CST[4],m[6],CST[7],m[7],CST[6],m[8],CST[9],m[9],CST[8],m[10],CST[11],m[11],CST[10],m[12],CST[13],m[13],CST[12],m[14],CST[15],m[15],CST[14]);
    blake256_round!(m[14],CST[10],m[10],CST[14],m[4],CST[8],m[8],CST[4],m[9],CST[15],m[15],CST[9],m[13],CST[6],m[6],CST[13],m[1],CST[12],m[12],CST[1],m[0],CST[2],m[2],CST[0],m[11],CST[7],m[7],CST[11],m[5],CST[3],m[3],CST[5]);
    blake256_round!(m[11],CST[8],m[8],CST[11],m[12],CST[0],m[0],CST[12],m[5],CST[2],m[2],CST[5],m[15],CST[13],m[13],CST[15],m[10],CST[14],m[14],CST[10],m[3],CST[6],m[6],CST[3],m[7],CST[1],m[1],CST[7],m[9],CST[4],m[4],CST[9]);
    blake256_round!(m[7],CST[9],m[9],CST[7],m[3],CST[1],m[1],CST[3],m[13],CST[12],m[12],CST[13],m[11],CST[14],m[14],CST[11],m[2],CST[6],m[6],CST[2],m[5],CST[10],m[10],CST[5],m[4],CST[0],m[0],CST[4],m[15],CST[8],m[8],CST[15]);
    blake256_round!(m[9],CST[0],m[0],CST[9],m[5],CST[7],m[7],CST[5],m[2],CST[4],m[4],CST[2],m[10],CST[15],m[15],CST[10],m[14],CST[1],m[1],CST[14],m[11],CST[12],m[12],CST[11],m[6],CST[8],m[8],CST[6],m[3],CST[13],m[13],CST[3]);
    blake256_round!(m[2],CST[12],m[12],CST[2],m[6],CST[10],m[10],CST[6],m[0],CST[11],m[11],CST[0],m[8],CST[3],m[3],CST[8],m[4],CST[13],m[13],CST[4],m[7],CST[5],m[5],CST[7],m[15],CST[14],m[14],CST[15],m[1],CST[9],m[9],CST[1]);
    blake256_round!(m[12],CST[5],m[5],CST[12],m[1],CST[15],m[15],CST[1],m[14],CST[13],m[13],CST[14],m[4],CST[10],m[10],CST[4],m[0],CST[7],m[7],CST[0],m[6],CST[3],m[3],CST[6],m[9],CST[2],m[2],CST[9],m[8],CST[11],m[11],CST[8]);
    blake256_round!(m[13],CST[11],m[11],CST[13],m[7],CST[14],m[14],CST[7],m[12],CST[1],m[1],CST[12],m[3],CST[9],m[9],CST[3],m[5],CST[0],m[0],CST[5],m[15],CST[4],m[4],CST[15],m[8],CST[6],m[6],CST[8],m[2],CST[10],m[10],CST[2]);
    blake256_round!(m[6],CST[15],m[15],CST[6],m[14],CST[9],m[9],CST[14],m[11],CST[3],m[3],CST[11],m[0],CST[8],m[8],CST[0],m[12],CST[2],m[2],CST[12],m[13],CST[7],m[7],CST[13],m[1],CST[4],m[4],CST[1],m[10],CST[5],m[5],CST[10]);
    blake256_round!(m[10],CST[2],m[2],CST[10],m[8],CST[4],m[4],CST[8],m[7],CST[6],m[6],CST[7],m[1],CST[5],m[5],CST[1],m[15],CST[11],m[11],CST[15],m[9],CST[14],m[14],CST[9],m[3],CST[12],m[12],CST[3],m[13],CST[0],m[0],CST[13]);
    blake256_round!(m[0],CST[1],m[1],CST[0],m[2],CST[3],m[3],CST[2],m[4],CST[5],m[5],CST[4],m[6],CST[7],m[7],CST[6],m[8],CST[9],m[9],CST[8],m[10],CST[11],m[11],CST[10],m[12],CST[13],m[13],CST[12],m[14],CST[15],m[15],CST[14]);
    blake256_round!(m[14],CST[10],m[10],CST[14],m[4],CST[8],m[8],CST[4],m[9],CST[15],m[15],CST[9],m[13],CST[6],m[6],CST[13],m[1],CST[12],m[12],CST[1],m[0],CST[2],m[2],CST[0],m[11],CST[7],m[7],CST[11],m[5],CST[3],m[3],CST[5]);
    blake256_round!(m[11],CST[8],m[8],CST[11],m[12],CST[0],m[0],CST[12],m[5],CST[2],m[2],CST[5],m[15],CST[13],m[13],CST[15],m[10],CST[14],m[14],CST[10],m[3],CST[6],m[6],CST[3],m[7],CST[1],m[1],CST[7],m[9],CST[4],m[4],CST[9]);
    blake256_round!(m[7],CST[9],m[9],CST[7],m[3],CST[1],m[1],CST[3],m[13],CST[12],m[12],CST[13],m[11],CST[14],m[14],CST[11],m[2],CST[6],m[6],CST[2],m[5],CST[10],m[10],CST[5],m[4],CST[0],m[0],CST[4],m[15],CST[8],m[8],CST[15]);

    for i in 0..8 { v[i] ^= v[i + 8]; }
    for i in 0..4 { v[i] ^= s.s[i]; v[i + 4] ^= s.s[i]; }
    for i in 0..8 { s.h[i] ^= v[i]; }
}

pub fn blake256_init(s: &mut BlakeState256) {
    s.h = [0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
           0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19];
    s.t = [0, 0];
    s.buflen = 0;
    s.nullt = 0;
    s.s = [0, 0, 0, 0];
}

pub fn blake256_update(s: &mut BlakeState256, data: &[u8], datalen: u64) {
    let mut data = data;
    let mut datalen = datalen;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 64 - left;

    if left != 0 && ((datalen >> 3) & 0x3F) as usize >= fill {
        s.buf[left..left + fill].copy_from_slice(&data[..fill]);
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 { s.t[1] = s.t[1].wrapping_add(1); }
        let buf_copy = s.buf;
        blake256_compress(s, &buf_copy);
        data = &data[fill..];
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 512 {
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 { s.t[1] = s.t[1].wrapping_add(1); }
        blake256_compress(s, &data[..64]);
        data = &data[64..];
        datalen -= 512;
    }

    if datalen > 0 {
        let bytes = (datalen >> 3) as usize;
        s.buf[left..left + bytes].copy_from_slice(&data[..bytes]);
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
            blake256_update(s, &PADDING[..(440 - s.buflen) as usize / 8], (440 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((512 - s.buflen) as u32);
            blake256_update(s, &PADDING[..(512 - s.buflen) as usize / 8], (512 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(440);
            blake256_update(s, &PADDING[1..1 + 440 / 8], 440);
            s.nullt = 1;
        }
        blake256_update(s, &[zo], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(64);
    blake256_update(s, &msglen, 64);

    for i in 0..8 {
        u32to8(&mut digest[i * 4..i * 4 + 4], s.h[i]);
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
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    let mut i: u32 = 0;
    while ((i as usize) + 1) * SPX_BLAKE256_OUTPUT_BYTES <= outlen {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..inlen + 4], i);
        let start = i as usize * SPX_BLAKE256_OUTPUT_BYTES;
        blake256(&mut out[start..], &inbuf, (inlen + 4) as u64);
        i += 1;
    }
    if outlen > i as usize * SPX_BLAKE256_OUTPUT_BYTES {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..inlen + 4], i);
        blake256(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let start = i as usize * SPX_BLAKE256_OUTPUT_BYTES;
        let remaining = outlen - start;
        out[start..start + remaining].copy_from_slice(&outbuf[..remaining]);
    }
}
