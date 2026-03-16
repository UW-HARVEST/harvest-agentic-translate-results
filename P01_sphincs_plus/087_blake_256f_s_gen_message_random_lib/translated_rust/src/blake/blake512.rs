use crate::params::*;

fn u8to32(p: &[u8]) -> u32 {
    (p[0] as u32) << 24 | (p[1] as u32) << 16 | (p[2] as u32) << 8 | p[3] as u32
}
fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8; p[1] = (v >> 16) as u8; p[2] = (v >> 8) as u8; p[3] = v as u8;
}
fn u8to64(p: &[u8]) -> u64 {
    ((u8to32(p) as u64) << 32) | (u8to32(&p[4..]) as u64)
}
fn u64to8(p: &mut [u8], v: u64) {
    u32to8(p, (v >> 32) as u32);
    u32to8(&mut p[4..], v as u32);
}

const CST: [u64; 16] = [
    0x243F6A8885A308D3,0x13198A2E03707344,0xA4093822299F31D0,0x082EFA98EC4E6C89,
    0x452821E638D01377,0xBE5466CF34E90C6C,0xC0AC29B7C97C50DD,0x3F84D5B5B5470917,
    0x9216D5D98979FB1B,0xD1310BA698DFB5AC,0x2FFD72DBD01ADFB7,0xB8E1AFED6A267E96,
    0xBA7C9045F12C7F99,0x24A19947B3916CF7,0x0801F2E2858EFC16,0x636920D871574E69,
];

static PADDING: [u8; 129] = {
    let mut p = [0u8; 129];
    p[0] = 0x80;
    p
};

#[derive(Clone)]
pub struct Blakestate512 {
    pub h: [u64; 8],
    pub s: [u64; 4],
    pub t: [u64; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 128],
}

fn rot(x: u64, n: u32) -> u64 { (x << (64 - n)) | (x >> n) }

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

macro_rules! blake512_round {
    ($v:ident, $m0:expr,$c0:expr,$m1:expr,$c1:expr,$m2:expr,$c2:expr,$m3:expr,$c3:expr,
     $m4:expr,$c4:expr,$m5:expr,$c5:expr,$m6:expr,$c6:expr,$m7:expr,$c7:expr,
     $m8:expr,$c8:expr,$m9:expr,$c9:expr,$m10:expr,$c10:expr,$m11:expr,$c11:expr,
     $m12:expr,$c12:expr,$m13:expr,$c13:expr,$m14:expr,$c14:expr,$m15:expr,$c15:expr) => {
        $v[0] = $v[0].wrapping_add($m0 ^ $c0).wrapping_add($v[4]);
        $v[12] ^= $v[0]; $v[12] = rot($v[12], 32);
        $v[8] = $v[8].wrapping_add($v[12]);
        $v[4] ^= $v[8]; $v[4] = rot($v[4], 25);

        $v[1] = $v[1].wrapping_add($m2 ^ $c2).wrapping_add($v[5]);
        $v[13] ^= $v[1]; $v[13] = rot($v[13], 32);
        $v[9] = $v[9].wrapping_add($v[13]);
        $v[5] ^= $v[9]; $v[5] = rot($v[5], 25);

        $v[2] = $v[2].wrapping_add($m4 ^ $c4).wrapping_add($v[6]);
        $v[14] ^= $v[2]; $v[14] = rot($v[14], 32);
        $v[10] = $v[10].wrapping_add($v[14]);
        $v[6] ^= $v[10]; $v[6] = rot($v[6], 25);

        $v[3] = $v[3].wrapping_add($m6 ^ $c6).wrapping_add($v[7]);
        $v[15] ^= $v[3]; $v[15] = rot($v[15], 32);
        $v[11] = $v[11].wrapping_add($v[15]);
        $v[7] ^= $v[11]; $v[7] = rot($v[7], 25);

        $v[2] = $v[2].wrapping_add($m5 ^ $c5).wrapping_add($v[6]);
        $v[14] ^= $v[2]; $v[14] = rot($v[14], 16);
        $v[10] = $v[10].wrapping_add($v[14]);
        $v[6] ^= $v[10]; $v[6] = rot($v[6], 11);

        $v[3] = $v[3].wrapping_add($m7 ^ $c7).wrapping_add($v[7]);
        $v[15] ^= $v[3]; $v[15] = rot($v[15], 16);
        $v[11] = $v[11].wrapping_add($v[15]);
        $v[7] ^= $v[11]; $v[7] = rot($v[7], 11);

        $v[1] = $v[1].wrapping_add($m3 ^ $c3).wrapping_add($v[5]);
        $v[13] ^= $v[1]; $v[13] = rot($v[13], 16);
        $v[9] = $v[9].wrapping_add($v[13]);
        $v[5] ^= $v[9]; $v[5] = rot($v[5], 11);

        $v[0] = $v[0].wrapping_add($m1 ^ $c1).wrapping_add($v[4]);
        $v[12] ^= $v[0]; $v[12] = rot($v[12], 16);
        $v[8] = $v[8].wrapping_add($v[12]);
        $v[4] ^= $v[8]; $v[4] = rot($v[4], 11);

        $v[0] = $v[0].wrapping_add($m8 ^ $c8).wrapping_add($v[5]);
        $v[15] ^= $v[0]; $v[15] = rot($v[15], 32);
        $v[10] = $v[10].wrapping_add($v[15]);
        $v[5] ^= $v[10]; $v[5] = rot($v[5], 25);

        $v[1] = $v[1].wrapping_add($m10 ^ $c10).wrapping_add($v[6]);
        $v[12] ^= $v[1]; $v[12] = rot($v[12], 32);
        $v[11] = $v[11].wrapping_add($v[12]);
        $v[6] ^= $v[11]; $v[6] = rot($v[6], 25);

        $v[2] = $v[2].wrapping_add($m12 ^ $c12).wrapping_add($v[7]);
        $v[13] ^= $v[2]; $v[13] = rot($v[13], 32);
        $v[8] = $v[8].wrapping_add($v[13]);
        $v[7] ^= $v[8]; $v[7] = rot($v[7], 25);

        $v[3] = $v[3].wrapping_add($m14 ^ $c14).wrapping_add($v[4]);
        $v[14] ^= $v[3]; $v[14] = rot($v[14], 32);
        $v[9] = $v[9].wrapping_add($v[14]);
        $v[4] ^= $v[9]; $v[4] = rot($v[4], 25);

        $v[2] = $v[2].wrapping_add($m13 ^ $c13).wrapping_add($v[7]);
        $v[13] ^= $v[2]; $v[13] = rot($v[13], 16);
        $v[8] = $v[8].wrapping_add($v[13]);
        $v[7] ^= $v[8]; $v[7] = rot($v[7], 11);

        $v[3] = $v[3].wrapping_add($m15 ^ $c15).wrapping_add($v[4]);
        $v[14] ^= $v[3]; $v[14] = rot($v[14], 16);
        $v[9] = $v[9].wrapping_add($v[14]);
        $v[4] ^= $v[9]; $v[4] = rot($v[4], 11);

        $v[1] = $v[1].wrapping_add($m11 ^ $c11).wrapping_add($v[6]);
        $v[12] ^= $v[1]; $v[12] = rot($v[12], 16);
        $v[11] = $v[11].wrapping_add($v[12]);
        $v[6] ^= $v[11]; $v[6] = rot($v[6], 11);

        $v[0] = $v[0].wrapping_add($m9 ^ $c9).wrapping_add($v[5]);
        $v[15] ^= $v[0]; $v[15] = rot($v[15], 16);
        $v[10] = $v[10].wrapping_add($v[15]);
        $v[5] ^= $v[10]; $v[5] = rot($v[5], 11);
    };
}

pub fn blake512_compress(s: &mut Blakestate512, block: &[u8]) {
    let mut m = [0u64; 16];
    for i in 0..16 { m[i] = u8to64(&block[i*8..]); }

    let mut v = [0u64; 16];
    v[..8].copy_from_slice(&s.h);
    v[8] = s.s[0] ^ 0x243F6A8885A308D3;
    v[9] = s.s[1] ^ 0x13198A2E03707344;
    v[10] = s.s[2] ^ 0xA4093822299F31D0;
    v[11] = s.s[3] ^ 0x082EFA98EC4E6C89;
    v[12] = 0x452821E638D01377;
    v[13] = 0xBE5466CF34E90C6C;
    v[14] = 0xC0AC29B7C97C50DD;
    v[15] = 0x3F84D5B5B5470917;
    if s.nullt == 0 {
        v[12] ^= s.t[0]; v[13] ^= s.t[0];
        v[14] ^= s.t[1]; v[15] ^= s.t[1];
    }

    for r in 0..16 {
        let sg = &SIGMA[r];
        blake512_round!(v,
            m[sg[0]],CST[sg[1]],m[sg[1]],CST[sg[0]],
            m[sg[2]],CST[sg[3]],m[sg[3]],CST[sg[2]],
            m[sg[4]],CST[sg[5]],m[sg[5]],CST[sg[4]],
            m[sg[6]],CST[sg[7]],m[sg[7]],CST[sg[6]],
            m[sg[8]],CST[sg[9]],m[sg[9]],CST[sg[8]],
            m[sg[10]],CST[sg[11]],m[sg[11]],CST[sg[10]],
            m[sg[12]],CST[sg[13]],m[sg[13]],CST[sg[12]],
            m[sg[14]],CST[sg[15]],m[sg[15]],CST[sg[14]]
        );
    }

    for i in 0..8 { v[i] ^= v[i+8]; }
    for i in 0..4 { v[i] ^= s.s[i]; v[i+4] ^= s.s[i]; }
    for i in 0..8 { s.h[i] ^= v[i]; }
}

pub fn blake512_init(s: &mut Blakestate512) {
    s.h = [0x6A09E667F3BCC908,0xBB67AE8584CAA73B,0x3C6EF372FE94F82B,0xA54FF53A5F1D36F1,
           0x510E527FADE682D1,0x9B05688C2B3E6C1F,0x1F83D9ABFB41BD6B,0x5BE0CD19137E2179];
    s.t = [0; 2]; s.buflen = 0; s.nullt = 0; s.s = [0; 4];
    s.buf = [0; 128];
}

pub fn blake512_update(s: &mut Blakestate512, data: &[u8], datalen: u64) {
    let mut datalen = datalen;
    let mut data = data;
    let left = (s.buflen >> 3) as usize;
    let fill = 128 - left;

    if left != 0 && ((datalen >> 3) & 0x7F) >= fill as u64 {
        s.buf[left..left + fill].copy_from_slice(&data[..fill]);
        s.t[0] = s.t[0].wrapping_add(1024);
        blake512_compress(s, &s.buf.clone());
        data = &data[fill..];
        datalen -= (fill as u64) << 3;
    }

    while datalen >= 1024 {
        s.t[0] = s.t[0].wrapping_add(1024);
        blake512_compress(s, &data[..128]);
        data = &data[128..];
        datalen -= 1024;
    }

    if datalen > 0 {
        let left2 = (s.buflen >> 3) as usize;
        let nbytes = ((datalen >> 3) & 0x7F) as usize;
        s.buf[left2..left2 + nbytes].copy_from_slice(&data[..nbytes]);
        s.buflen = ((left2 as i32) << 3) + datalen as i32;
    } else {
        s.buflen = 0;
    }
}

pub fn blake512_final(s: &mut Blakestate512, digest: &mut [u8]) {
    let mut msglen = [0u8; 16];
    let lo = s.t[0].wrapping_add(s.buflen as u64);
    let mut hi = s.t[1];
    if lo < s.buflen as u64 { hi = hi.wrapping_add(1); }
    u64to8(&mut msglen[0..8], hi);
    u64to8(&mut msglen[8..16], lo);

    if s.buflen == 888 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake512_update(s, &[0x81u8], 8);
    } else {
        if s.buflen < 888 {
            if s.buflen == 0 { s.nullt = 1; }
            s.t[0] = s.t[0].wrapping_sub((888 - s.buflen) as u64);
            let pad_len = (888 - s.buflen) as u64;
            let nbytes = (pad_len >> 3) as usize;
            let mut pad = vec![0u8; nbytes];
            let copy_len = nbytes.min(PADDING.len());
            pad[..copy_len].copy_from_slice(&PADDING[..copy_len]);
            blake512_update(s, &pad, pad_len);
        } else {
            s.t[0] = s.t[0].wrapping_sub((1024 - s.buflen) as u64);
            let pad_len = (1024 - s.buflen) as u64;
            let nbytes = (pad_len >> 3) as usize;
            let mut pad = vec![0u8; nbytes];
            let copy_len = nbytes.min(PADDING.len());
            pad[..copy_len].copy_from_slice(&PADDING[..copy_len]);
            blake512_update(s, &pad, pad_len);
            s.t[0] = s.t[0].wrapping_sub(888);
            blake512_update(s, &PADDING[1..112], 888);
            s.nullt = 1;
        }
        blake512_update(s, &[0x01u8], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(128);
    blake512_update(s, &msglen, 128);

    for i in 0..8 {
        u64to8(&mut digest[i*8..i*8+8], s.h[i]);
    }
}

pub fn blake512(out: &mut [u8], inp: &[u8], inlen: u64) -> i32 {
    let mut s = Blakestate512 {
        h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128],
    };
    blake512_init(&mut s);
    blake512_update(&mut s, inp, inlen.wrapping_mul(8));
    blake512_final(&mut s, out);
    0
}

pub fn blake512_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut i: u64 = 0;
    let mut off = 0usize;
    while (i + 1) * SPX_BLAKE512_OUTPUT_BYTES as u64 <= outlen as u64 {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..inlen + 4], i as u32);
        blake512(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += SPX_BLAKE512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i as usize * SPX_BLAKE512_OUTPUT_BYTES {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..inlen + 4], i as u32);
        blake512(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - i as usize * SPX_BLAKE512_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}
