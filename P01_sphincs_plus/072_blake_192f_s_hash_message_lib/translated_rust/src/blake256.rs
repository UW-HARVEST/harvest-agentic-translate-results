use crate::context::u32_to_bytes;
use crate::params::*;

const CST256: [u32; 16] = [
    0x243F6A88, 0x85A308D3, 0x13198A2E, 0x03707344,
    0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    0x452821E6, 0x38D01377, 0xBE5466CF, 0x34E90C6C,
    0xC0AC29B7, 0xC97C50DD, 0x3F84D5B5, 0xB5470917,
];

static PADDING256: [u8; 64] = {
    let mut p = [0u8; 64];
    p[0] = 0x80;
    p
};

fn u8to32(p: &[u8]) -> u32 {
    ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32)
}

fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8;
    p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8;
    p[3] = v as u8;
}

fn blake256_rot(x: u32, n: u32) -> u32 {
    (x << (32 - n)) | (x >> n)
}

pub struct Blakestate256 {
    pub h: [u32; 8],
    pub s: [u32; 4],
    pub t: [u32; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 64],
}

pub fn blake256_init(s: &mut Blakestate256) {
    s.h[0] = 0x6A09E667;
    s.h[1] = 0xBB67AE85;
    s.h[2] = 0x3C6EF372;
    s.h[3] = 0xA54FF53A;
    s.h[4] = 0x510E527F;
    s.h[5] = 0x9B05688C;
    s.h[6] = 0x1F83D9AB;
    s.h[7] = 0x5BE0CD19;
    s.t = [0; 2];
    s.buflen = 0;
    s.nullt = 0;
    s.s = [0; 4];
}

pub fn blake256_compress(s: &mut Blakestate256, block: &[u8]) {
    let m0 = u8to32(&block[0..]);
    let m1 = u8to32(&block[4..]);
    let m2 = u8to32(&block[8..]);
    let m3 = u8to32(&block[12..]);
    let m4 = u8to32(&block[16..]);
    let m5 = u8to32(&block[20..]);
    let m6 = u8to32(&block[24..]);
    let m7 = u8to32(&block[28..]);
    let m8 = u8to32(&block[32..]);
    let m9 = u8to32(&block[36..]);
    let m10 = u8to32(&block[40..]);
    let m11 = u8to32(&block[44..]);
    let m12 = u8to32(&block[48..]);
    let m13 = u8to32(&block[52..]);
    let m14 = u8to32(&block[56..]);
    let m15 = u8to32(&block[60..]);

    let mut v0 = s.h[0];
    let mut v1 = s.h[1];
    let mut v2 = s.h[2];
    let mut v3 = s.h[3];
    let mut v4 = s.h[4];
    let mut v5 = s.h[5];
    let mut v6 = s.h[6];
    let mut v7 = s.h[7];
    let mut v8 = s.s[0] ^ 0x243F6A88;
    let mut v9 = s.s[1] ^ 0x85A308D3;
    let mut v10 = s.s[2] ^ 0x13198A2E;
    let mut v11 = s.s[3] ^ 0x03707344;
    let mut v12: u32 = 0xA4093822;
    let mut v13: u32 = 0x299F31D0;
    let mut v14: u32 = 0x082EFA98;
    let mut v15: u32 = 0xEC4E6C89;

    if s.nullt == 0 {
        v12 ^= s.t[0];
        v13 ^= s.t[0];
        v14 ^= s.t[1];
        v15 ^= s.t[1];
    }

    macro_rules! round256 {
        ($m0:expr,$c0:expr,$m1:expr,$c1:expr,$m2:expr,$c2:expr,$m3:expr,$c3:expr,
         $m4:expr,$c4:expr,$m5:expr,$c5:expr,$m6:expr,$c6:expr,$m7:expr,$c7:expr,
         $m8:expr,$c8:expr,$m9:expr,$c9:expr,$m10:expr,$c10:expr,$m11:expr,$c11:expr,
         $m12:expr,$c12:expr,$m13:expr,$c13:expr,$m14:expr,$c14:expr,$m15:expr,$c15:expr) => {
            v0 = v0.wrapping_add($m0 ^ $c0); v0 = v0.wrapping_add(v4); v12 ^= v0; v12 = blake256_rot(v12, 16);
            v8 = v8.wrapping_add(v12); v4 ^= v8; v4 = blake256_rot(v4, 12);
            v1 = v1.wrapping_add($m2 ^ $c2); v1 = v1.wrapping_add(v5); v13 ^= v1; v13 = blake256_rot(v13, 16);
            v9 = v9.wrapping_add(v13); v5 ^= v9; v5 = blake256_rot(v5, 12);
            v2 = v2.wrapping_add($m4 ^ $c4); v2 = v2.wrapping_add(v6); v14 ^= v2; v14 = blake256_rot(v14, 16);
            v10 = v10.wrapping_add(v14); v6 ^= v10; v6 = blake256_rot(v6, 12);
            v3 = v3.wrapping_add($m6 ^ $c6); v3 = v3.wrapping_add(v7); v15 ^= v3; v15 = blake256_rot(v15, 16);
            v11 = v11.wrapping_add(v15); v7 ^= v11; v7 = blake256_rot(v7, 12);
            v2 = v2.wrapping_add($m5 ^ $c5); v2 = v2.wrapping_add(v6); v14 ^= v2; v14 = blake256_rot(v14, 8);
            v10 = v10.wrapping_add(v14); v6 ^= v10; v6 = blake256_rot(v6, 7);
            v3 = v3.wrapping_add($m7 ^ $c7); v3 = v3.wrapping_add(v7); v15 ^= v3; v15 = blake256_rot(v15, 8);
            v11 = v11.wrapping_add(v15); v7 ^= v11; v7 = blake256_rot(v7, 7);
            v1 = v1.wrapping_add($m3 ^ $c3); v1 = v1.wrapping_add(v5); v13 ^= v1; v13 = blake256_rot(v13, 8);
            v9 = v9.wrapping_add(v13); v5 ^= v9; v5 = blake256_rot(v5, 7);
            v0 = v0.wrapping_add($m1 ^ $c1); v0 = v0.wrapping_add(v4); v12 ^= v0; v12 = blake256_rot(v12, 8);
            v8 = v8.wrapping_add(v12); v4 ^= v8; v4 = blake256_rot(v4, 7);
            v0 = v0.wrapping_add($m8 ^ $c8); v0 = v0.wrapping_add(v5); v15 ^= v0; v15 = blake256_rot(v15, 16);
            v10 = v10.wrapping_add(v15); v5 ^= v10; v5 = blake256_rot(v5, 12);
            v1 = v1.wrapping_add($m10 ^ $c10); v1 = v1.wrapping_add(v6); v12 ^= v1; v12 = blake256_rot(v12, 16);
            v11 = v11.wrapping_add(v12); v6 ^= v11; v6 = blake256_rot(v6, 12);
            v2 = v2.wrapping_add($m12 ^ $c12); v2 = v2.wrapping_add(v7); v13 ^= v2; v13 = blake256_rot(v13, 16);
            v8 = v8.wrapping_add(v13); v7 ^= v8; v7 = blake256_rot(v7, 12);
            v3 = v3.wrapping_add($m14 ^ $c14); v3 = v3.wrapping_add(v4); v14 ^= v3; v14 = blake256_rot(v14, 16);
            v9 = v9.wrapping_add(v14); v4 ^= v9; v4 = blake256_rot(v4, 12);
            v2 = v2.wrapping_add($m13 ^ $c13); v2 = v2.wrapping_add(v7); v13 ^= v2; v13 = blake256_rot(v13, 8);
            v8 = v8.wrapping_add(v13); v7 ^= v8; v7 = blake256_rot(v7, 7);
            v3 = v3.wrapping_add($m15 ^ $c15); v3 = v3.wrapping_add(v4); v14 ^= v3; v14 = blake256_rot(v14, 8);
            v9 = v9.wrapping_add(v14); v4 ^= v9; v4 = blake256_rot(v4, 7);
            v1 = v1.wrapping_add($m11 ^ $c11); v1 = v1.wrapping_add(v6); v12 ^= v1; v12 = blake256_rot(v12, 8);
            v11 = v11.wrapping_add(v12); v6 ^= v11; v6 = blake256_rot(v6, 7);
            v0 = v0.wrapping_add($m9 ^ $c9); v0 = v0.wrapping_add(v5); v15 ^= v0; v15 = blake256_rot(v15, 8);
            v10 = v10.wrapping_add(v15); v5 ^= v10; v5 = blake256_rot(v5, 7);
        };
    }

    let c = &CST256;
    round256!(m0,c[1],m1,c[0],m2,c[3],m3,c[2],m4,c[5],m5,c[4],m6,c[7],m7,c[6],m8,c[9],m9,c[8],m10,c[11],m11,c[10],m12,c[13],m13,c[12],m14,c[15],m15,c[14]);
    round256!(m14,c[10],m10,c[14],m4,c[8],m8,c[4],m9,c[15],m15,c[9],m13,c[6],m6,c[13],m1,c[12],m12,c[1],m0,c[2],m2,c[0],m11,c[7],m7,c[11],m5,c[3],m3,c[5]);
    round256!(m11,c[8],m8,c[11],m12,c[0],m0,c[12],m5,c[2],m2,c[5],m15,c[13],m13,c[15],m10,c[14],m14,c[10],m3,c[6],m6,c[3],m7,c[1],m1,c[7],m9,c[4],m4,c[9]);
    round256!(m7,c[9],m9,c[7],m3,c[1],m1,c[3],m13,c[12],m12,c[13],m11,c[14],m14,c[11],m2,c[6],m6,c[2],m5,c[10],m10,c[5],m4,c[0],m0,c[4],m15,c[8],m8,c[15]);
    round256!(m9,c[0],m0,c[9],m5,c[7],m7,c[5],m2,c[4],m4,c[2],m10,c[15],m15,c[10],m14,c[1],m1,c[14],m11,c[12],m12,c[11],m6,c[8],m8,c[6],m3,c[13],m13,c[3]);
    round256!(m2,c[12],m12,c[2],m6,c[10],m10,c[6],m0,c[11],m11,c[0],m8,c[3],m3,c[8],m4,c[13],m13,c[4],m7,c[5],m5,c[7],m15,c[14],m14,c[15],m1,c[9],m9,c[1]);
    round256!(m12,c[5],m5,c[12],m1,c[15],m15,c[1],m14,c[13],m13,c[14],m4,c[10],m10,c[4],m0,c[7],m7,c[0],m6,c[3],m3,c[6],m9,c[2],m2,c[9],m8,c[11],m11,c[8]);
    round256!(m13,c[11],m11,c[13],m7,c[14],m14,c[7],m12,c[1],m1,c[12],m3,c[9],m9,c[3],m5,c[0],m0,c[5],m15,c[4],m4,c[15],m8,c[6],m6,c[8],m2,c[10],m10,c[2]);
    round256!(m6,c[15],m15,c[6],m14,c[9],m9,c[14],m11,c[3],m3,c[11],m0,c[8],m8,c[0],m12,c[2],m2,c[12],m13,c[7],m7,c[13],m1,c[4],m4,c[1],m10,c[5],m5,c[10]);
    round256!(m10,c[2],m2,c[10],m8,c[4],m4,c[8],m7,c[6],m6,c[7],m1,c[5],m5,c[1],m15,c[11],m11,c[15],m9,c[14],m14,c[9],m3,c[12],m12,c[3],m13,c[0],m0,c[13]);
    round256!(m0,c[1],m1,c[0],m2,c[3],m3,c[2],m4,c[5],m5,c[4],m6,c[7],m7,c[6],m8,c[9],m9,c[8],m10,c[11],m11,c[10],m12,c[13],m13,c[12],m14,c[15],m15,c[14]);
    round256!(m14,c[10],m10,c[14],m4,c[8],m8,c[4],m9,c[15],m15,c[9],m13,c[6],m6,c[13],m1,c[12],m12,c[1],m0,c[2],m2,c[0],m11,c[7],m7,c[11],m5,c[3],m3,c[5]);
    round256!(m11,c[8],m8,c[11],m12,c[0],m0,c[12],m5,c[2],m2,c[5],m15,c[13],m13,c[15],m10,c[14],m14,c[10],m3,c[6],m6,c[3],m7,c[1],m1,c[7],m9,c[4],m4,c[9]);
    round256!(m7,c[9],m9,c[7],m3,c[1],m1,c[3],m13,c[12],m12,c[13],m11,c[14],m14,c[11],m2,c[6],m6,c[2],m5,c[10],m10,c[5],m4,c[0],m0,c[4],m15,c[8],m8,c[15]);

    v0 ^= v8; v1 ^= v9; v2 ^= v10; v3 ^= v11;
    v4 ^= v12; v5 ^= v13; v6 ^= v14; v7 ^= v15;
    v0 ^= s.s[0]; v1 ^= s.s[1]; v2 ^= s.s[2]; v3 ^= s.s[3];
    v4 ^= s.s[0]; v5 ^= s.s[1]; v6 ^= s.s[2]; v7 ^= s.s[3];
    s.h[0] ^= v0; s.h[1] ^= v1; s.h[2] ^= v2; s.h[3] ^= v3;
    s.h[4] ^= v4; s.h[5] ^= v5; s.h[6] ^= v6; s.h[7] ^= v7;
}

pub fn blake256_update(s: &mut Blakestate256, mut data: &[u8], mut datalen: u64) {
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
        blake256_compress(s, data);
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

pub fn blake256_final(s: &mut Blakestate256, digest: &mut [u8]) {
    let mut msglen = [0u8; 8];
    let lo = s.t[0].wrapping_add(s.buflen as u32);
    let mut hi = s.t[1];
    if lo < s.buflen as u32 { hi = hi.wrapping_add(1); }
    u32to8(&mut msglen[0..4], hi);
    u32to8(&mut msglen[4..8], lo);

    if s.buflen == 440 {
        s.t[0] = s.t[0].wrapping_sub(8);
        let oo: u8 = 0x81;
        blake256_update(s, &[oo], 8);
    } else {
        if s.buflen < 440 {
            if s.buflen == 0 { s.nullt = 1; }
            s.t[0] = s.t[0].wrapping_sub((440 - s.buflen) as u32);
            blake256_update(s, &PADDING256[..(440 - s.buflen) as usize / 8], (440 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((512 - s.buflen) as u32);
            blake256_update(s, &PADDING256[..(512 - s.buflen) as usize / 8], (512 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(440);
            blake256_update(s, &PADDING256[1..1 + 440 / 8], 440);
            s.nullt = 1;
        }
        let zo: u8 = 0x01;
        blake256_update(s, &[zo], 8);
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
    let mut s = Blakestate256 {
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

    let mut i: usize = 0;
    while (i + 1) * SPX_BLAKE256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..inlen + 4], i as u32);
        blake256(&mut out[i * SPX_BLAKE256_OUTPUT_BYTES..], &inbuf, (inlen + 4) as u64);
        i += 1;
    }
    if outlen > i * SPX_BLAKE256_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..inlen + 4], i as u32);
        blake256(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - i * SPX_BLAKE256_OUTPUT_BYTES;
        out[i * SPX_BLAKE256_OUTPUT_BYTES..i * SPX_BLAKE256_OUTPUT_BYTES + rem]
            .copy_from_slice(&outbuf[..rem]);
    }
}
