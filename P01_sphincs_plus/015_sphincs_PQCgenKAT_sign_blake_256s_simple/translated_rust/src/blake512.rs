use crate::address::u32_to_bytes;
use crate::params::SPX_BLAKE512_OUTPUT_BYTES;

pub struct BlakeState512 {
    pub h: [u64; 8],
    pub s: [u64; 4],
    pub t: [u64; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 128],
}

const CST512: [u64; 16] = [
    0x243F6A8885A308D3, 0x13198A2E03707344, 0xA4093822299F31D0, 0x082EFA98EC4E6C89,
    0x452821E638D01377, 0xBE5466CF34E90C6C, 0xC0AC29B7C97C50DD, 0x3F84D5B5B5470917,
    0x9216D5D98979FB1B, 0xD1310BA698DFB5AC, 0x2FFD72DBD01ADFB7, 0xB8E1AFED6A267E96,
    0xBA7C9045F12C7F99, 0x24A19947B3916CF7, 0x0801F2E2858EFC16, 0x636920D871574E69,
];

static PADDING512: [u8; 129] = {
    let mut p = [0u8; 129];
    p[0] = 0x80;
    p
};

fn u8to32(p: &[u8]) -> u32 {
    (p[0] as u32) << 24 | (p[1] as u32) << 16 | (p[2] as u32) << 8 | (p[3] as u32)
}

fn u8to64(p: &[u8]) -> u64 {
    ((u8to32(p) as u64) << 32) | (u8to32(&p[4..]) as u64)
}

fn u64to8(p: &mut [u8], v: u64) {
    let hi = (v >> 32) as u32;
    let lo = v as u32;
    p[0] = (hi >> 24) as u8; p[1] = (hi >> 16) as u8; p[2] = (hi >> 8) as u8; p[3] = hi as u8;
    p[4] = (lo >> 24) as u8; p[5] = (lo >> 16) as u8; p[6] = (lo >> 8) as u8; p[7] = lo as u8;
}

fn rot64(x: u64, n: u32) -> u64 {
    (x << (64 - n)) | (x >> n)
}

macro_rules! round512 {
    ($v0:expr,$v1:expr,$v2:expr,$v3:expr,$v4:expr,$v5:expr,$v6:expr,$v7:expr,
     $v8:expr,$v9:expr,$v10:expr,$v11:expr,$v12:expr,$v13:expr,$v14:expr,$v15:expr,
     $m0:expr,$c0:expr,$m1:expr,$c1:expr,$m2:expr,$c2:expr,$m3:expr,$c3:expr,
     $m4:expr,$c4:expr,$m5:expr,$c5:expr,$m6:expr,$c6:expr,$m7:expr,$c7:expr,
     $m8:expr,$c8:expr,$m9:expr,$c9:expr,$m10:expr,$c10:expr,$m11:expr,$c11:expr,
     $m12:expr,$c12:expr,$m13:expr,$c13:expr,$m14:expr,$c14:expr,$m15:expr,$c15:expr) => {
        $v0 = $v0.wrapping_add($m0 ^ $c0).wrapping_add($v4);
        $v12 ^= $v0; $v12 = rot64($v12, 32);
        $v8 = $v8.wrapping_add($v12); $v4 ^= $v8; $v4 = rot64($v4, 25);
        $v1 = $v1.wrapping_add($m2 ^ $c2).wrapping_add($v5);
        $v13 ^= $v1; $v13 = rot64($v13, 32);
        $v9 = $v9.wrapping_add($v13); $v5 ^= $v9; $v5 = rot64($v5, 25);
        $v2 = $v2.wrapping_add($m4 ^ $c4).wrapping_add($v6);
        $v14 ^= $v2; $v14 = rot64($v14, 32);
        $v10 = $v10.wrapping_add($v14); $v6 ^= $v10; $v6 = rot64($v6, 25);
        $v3 = $v3.wrapping_add($m6 ^ $c6).wrapping_add($v7);
        $v15 ^= $v3; $v15 = rot64($v15, 32);
        $v11 = $v11.wrapping_add($v15); $v7 ^= $v11; $v7 = rot64($v7, 25);
        $v2 = $v2.wrapping_add($m5 ^ $c5).wrapping_add($v6);
        $v14 ^= $v2; $v14 = rot64($v14, 16);
        $v10 = $v10.wrapping_add($v14); $v6 ^= $v10; $v6 = rot64($v6, 11);
        $v3 = $v3.wrapping_add($m7 ^ $c7).wrapping_add($v7);
        $v15 ^= $v3; $v15 = rot64($v15, 16);
        $v11 = $v11.wrapping_add($v15); $v7 ^= $v11; $v7 = rot64($v7, 11);
        $v1 = $v1.wrapping_add($m3 ^ $c3).wrapping_add($v5);
        $v13 ^= $v1; $v13 = rot64($v13, 16);
        $v9 = $v9.wrapping_add($v13); $v5 ^= $v9; $v5 = rot64($v5, 11);
        $v0 = $v0.wrapping_add($m1 ^ $c1).wrapping_add($v4);
        $v12 ^= $v0; $v12 = rot64($v12, 16);
        $v8 = $v8.wrapping_add($v12); $v4 ^= $v8; $v4 = rot64($v4, 11);
        // diag
        $v0 = $v0.wrapping_add($m8 ^ $c8).wrapping_add($v5);
        $v15 ^= $v0; $v15 = rot64($v15, 32);
        $v10 = $v10.wrapping_add($v15); $v5 ^= $v10; $v5 = rot64($v5, 25);
        $v1 = $v1.wrapping_add($m10 ^ $c10).wrapping_add($v6);
        $v12 ^= $v1; $v12 = rot64($v12, 32);
        $v11 = $v11.wrapping_add($v12); $v6 ^= $v11; $v6 = rot64($v6, 25);
        $v2 = $v2.wrapping_add($m12 ^ $c12).wrapping_add($v7);
        $v13 ^= $v2; $v13 = rot64($v13, 32);
        $v8 = $v8.wrapping_add($v13); $v7 ^= $v8; $v7 = rot64($v7, 25);
        $v3 = $v3.wrapping_add($m14 ^ $c14).wrapping_add($v4);
        $v14 ^= $v3; $v14 = rot64($v14, 32);
        $v9 = $v9.wrapping_add($v14); $v4 ^= $v9; $v4 = rot64($v4, 25);
        $v2 = $v2.wrapping_add($m13 ^ $c13).wrapping_add($v7);
        $v13 ^= $v2; $v13 = rot64($v13, 16);
        $v8 = $v8.wrapping_add($v13); $v7 ^= $v8; $v7 = rot64($v7, 11);
        $v3 = $v3.wrapping_add($m15 ^ $c15).wrapping_add($v4);
        $v14 ^= $v3; $v14 = rot64($v14, 16);
        $v9 = $v9.wrapping_add($v14); $v4 ^= $v9; $v4 = rot64($v4, 11);
        $v1 = $v1.wrapping_add($m11 ^ $c11).wrapping_add($v6);
        $v12 ^= $v1; $v12 = rot64($v12, 16);
        $v11 = $v11.wrapping_add($v12); $v6 ^= $v11; $v6 = rot64($v6, 11);
        $v0 = $v0.wrapping_add($m9 ^ $c9).wrapping_add($v5);
        $v15 ^= $v0; $v15 = rot64($v15, 16);
        $v10 = $v10.wrapping_add($v15); $v5 ^= $v10; $v5 = rot64($v5, 11);
    };
}

pub fn blake512_compress(s: &mut BlakeState512, block: &[u8]) {
    let c = &CST512;
    let m: [u64; 16] = core::array::from_fn(|i| u8to64(&block[i * 8..]));
    let (mut v0, mut v1, mut v2, mut v3) = (s.h[0], s.h[1], s.h[2], s.h[3]);
    let (mut v4, mut v5, mut v6, mut v7) = (s.h[4], s.h[5], s.h[6], s.h[7]);
    let mut v8 = s.s[0] ^ 0x243F6A8885A308D3;
    let mut v9 = s.s[1] ^ 0x13198A2E03707344;
    let mut v10 = s.s[2] ^ 0xA4093822299F31D0;
    let mut v11 = s.s[3] ^ 0x082EFA98EC4E6C89;
    let (mut v12, mut v13, mut v14, mut v15) = (0x452821E638D01377u64, 0xBE5466CF34E90C6Cu64, 0xC0AC29B7C97C50DDu64, 0x3F84D5B5B5470917u64);
    if s.nullt == 0 {
        v12 ^= s.t[0]; v13 ^= s.t[0]; v14 ^= s.t[1]; v15 ^= s.t[1];
    }

    round512!(v0,v1,v2,v3,v4,v5,v6,v7,v8,v9,v10,v11,v12,v13,v14,v15, m[0],c[1],m[1],c[0],m[2],c[3],m[3],c[2],m[4],c[5],m[5],c[4],m[6],c[7],m[7],c[6],m[8],c[9],m[9],c[8],m[10],c[11],m[11],c[10],m[12],c[13],m[13],c[12],m[14],c[15],m[15],c[14]);
    round512!(v0,v1,v2,v3,v4,v5,v6,v7,v8,v9,v10,v11,v12,v13,v14,v15, m[14],c[10],m[10],c[14],m[4],c[8],m[8],c[4],m[9],c[15],m[15],c[9],m[13],c[6],m[6],c[13],m[1],c[12],m[12],c[1],m[0],c[2],m[2],c[0],m[11],c[7],m[7],c[11],m[5],c[3],m[3],c[5]);
    round512!(v0,v1,v2,v3,v4,v5,v6,v7,v8,v9,v10,v11,v12,v13,v14,v15, m[11],c[8],m[8],c[11],m[12],c[0],m[0],c[12],m[5],c[2],m[2],c[5],m[15],c[13],m[13],c[15],m[10],c[14],m[14],c[10],m[3],c[6],m[6],c[3],m[7],c[1],m[1],c[7],m[9],c[4],m[4],c[9]);
    round512!(v0,v1,v2,v3,v4,v5,v6,v7,v8,v9,v10,v11,v12,v13,v14,v15, m[7],c[9],m[9],c[7],m[3],c[1],m[1],c[3],m[13],c[12],m[12],c[13],m[11],c[14],m[14],c[11],m[2],c[6],m[6],c[2],m[5],c[10],m[10],c[5],m[4],c[0],m[0],c[4],m[15],c[8],m[8],c[15]);
    round512!(v0,v1,v2,v3,v4,v5,v6,v7,v8,v9,v10,v11,v12,v13,v14,v15, m[9],c[0],m[0],c[9],m[5],c[7],m[7],c[5],m[2],c[4],m[4],c[2],m[10],c[15],m[15],c[10],m[14],c[1],m[1],c[14],m[11],c[12],m[12],c[11],m[6],c[8],m[8],c[6],m[3],c[13],m[13],c[3]);
    round512!(v0,v1,v2,v3,v4,v5,v6,v7,v8,v9,v10,v11,v12,v13,v14,v15, m[2],c[12],m[12],c[2],m[6],c[10],m[10],c[6],m[0],c[11],m[11],c[0],m[8],c[3],m[3],c[8],m[4],c[13],m[13],c[4],m[7],c[5],m[5],c[7],m[15],c[14],m[14],c[15],m[1],c[9],m[9],c[1]);
    round512!(v0,v1,v2,v3,v4,v5,v6,v7,v8,v9,v10,v11,v12,v13,v14,v15, m[12],c[5],m[5],c[12],m[1],c[15],m[15],c[1],m[14],c[13],m[13],c[14],m[4],c[10],m[10],c[4],m[0],c[7],m[7],c[0],m[6],c[3],m[3],c[6],m[9],c[2],m[2],c[9],m[8],c[11],m[11],c[8]);
    round512!(v0,v1,v2,v3,v4,v5,v6,v7,v8,v9,v10,v11,v12,v13,v14,v15, m[13],c[11],m[11],c[13],m[7],c[14],m[14],c[7],m[12],c[1],m[1],c[12],m[3],c[9],m[9],c[3],m[5],c[0],m[0],c[5],m[15],c[4],m[4],c[15],m[8],c[6],m[6],c[8],m[2],c[10],m[10],c[2]);
    round512!(v0,v1,v2,v3,v4,v5,v6,v7,v8,v9,v10,v11,v12,v13,v14,v15, m[6],c[15],m[15],c[6],m[14],c[9],m[9],c[14],m[11],c[3],m[3],c[11],m[0],c[8],m[8],c[0],m[12],c[2],m[2],c[12],m[13],c[7],m[7],c[13],m[1],c[4],m[4],c[1],m[10],c[5],m[5],c[10]);
    round512!(v0,v1,v2,v3,v4,v5,v6,v7,v8,v9,v10,v11,v12,v13,v14,v15, m[10],c[2],m[2],c[10],m[8],c[4],m[4],c[8],m[7],c[6],m[6],c[7],m[1],c[5],m[5],c[1],m[15],c[11],m[11],c[15],m[9],c[14],m[14],c[9],m[3],c[12],m[12],c[3],m[13],c[0],m[0],c[13]);
    // rounds 11-16 repeat 0-5
    round512!(v0,v1,v2,v3,v4,v5,v6,v7,v8,v9,v10,v11,v12,v13,v14,v15, m[0],c[1],m[1],c[0],m[2],c[3],m[3],c[2],m[4],c[5],m[5],c[4],m[6],c[7],m[7],c[6],m[8],c[9],m[9],c[8],m[10],c[11],m[11],c[10],m[12],c[13],m[13],c[12],m[14],c[15],m[15],c[14]);
    round512!(v0,v1,v2,v3,v4,v5,v6,v7,v8,v9,v10,v11,v12,v13,v14,v15, m[14],c[10],m[10],c[14],m[4],c[8],m[8],c[4],m[9],c[15],m[15],c[9],m[13],c[6],m[6],c[13],m[1],c[12],m[12],c[1],m[0],c[2],m[2],c[0],m[11],c[7],m[7],c[11],m[5],c[3],m[3],c[5]);
    round512!(v0,v1,v2,v3,v4,v5,v6,v7,v8,v9,v10,v11,v12,v13,v14,v15, m[11],c[8],m[8],c[11],m[12],c[0],m[0],c[12],m[5],c[2],m[2],c[5],m[15],c[13],m[13],c[15],m[10],c[14],m[14],c[10],m[3],c[6],m[6],c[3],m[7],c[1],m[1],c[7],m[9],c[4],m[4],c[9]);
    round512!(v0,v1,v2,v3,v4,v5,v6,v7,v8,v9,v10,v11,v12,v13,v14,v15, m[7],c[9],m[9],c[7],m[3],c[1],m[1],c[3],m[13],c[12],m[12],c[13],m[11],c[14],m[14],c[11],m[2],c[6],m[6],c[2],m[5],c[10],m[10],c[5],m[4],c[0],m[0],c[4],m[15],c[8],m[8],c[15]);
    round512!(v0,v1,v2,v3,v4,v5,v6,v7,v8,v9,v10,v11,v12,v13,v14,v15, m[9],c[0],m[0],c[9],m[5],c[7],m[7],c[5],m[2],c[4],m[4],c[2],m[10],c[15],m[15],c[10],m[14],c[1],m[1],c[14],m[11],c[12],m[12],c[11],m[6],c[8],m[8],c[6],m[3],c[13],m[13],c[3]);
    round512!(v0,v1,v2,v3,v4,v5,v6,v7,v8,v9,v10,v11,v12,v13,v14,v15, m[2],c[12],m[12],c[2],m[6],c[10],m[10],c[6],m[0],c[11],m[11],c[0],m[8],c[3],m[3],c[8],m[4],c[13],m[13],c[4],m[7],c[5],m[5],c[7],m[15],c[14],m[14],c[15],m[1],c[9],m[9],c[1]);

    v0 ^= v8; v1 ^= v9; v2 ^= v10; v3 ^= v11;
    v4 ^= v12; v5 ^= v13; v6 ^= v14; v7 ^= v15;
    v0 ^= s.s[0]; v1 ^= s.s[1]; v2 ^= s.s[2]; v3 ^= s.s[3];
    v4 ^= s.s[0]; v5 ^= s.s[1]; v6 ^= s.s[2]; v7 ^= s.s[3];
    s.h[0] ^= v0; s.h[1] ^= v1; s.h[2] ^= v2; s.h[3] ^= v3;
    s.h[4] ^= v4; s.h[5] ^= v5; s.h[6] ^= v6; s.h[7] ^= v7;
}

pub fn blake512_init(s: &mut BlakeState512) {
    s.h = [0x6A09E667F3BCC908, 0xBB67AE8584CAA73B, 0x3C6EF372FE94F82B, 0xA54FF53A5F1D36F1,
           0x510E527FADE682D1, 0x9B05688C2B3E6C1F, 0x1F83D9ABFB41BD6B, 0x5BE0CD19137E2179];
    s.t = [0, 0]; s.buflen = 0; s.nullt = 0;
    s.s = [0, 0, 0, 0]; s.buf = [0; 128];
}

pub fn blake512_update(s: &mut BlakeState512, data: &[u8], datalen_bits: u64) {
    let mut datalen = datalen_bits;
    let mut data = data;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 128 - left;

    if left != 0 && ((datalen >> 3) & 0x7F) >= fill as u64 {
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
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo = s.t[0].wrapping_add(s.buflen as u64);
    let mut hi = s.t[1];
    if lo < s.buflen as u64 { hi = hi.wrapping_add(1); }
    let mut msglen = [0u8; 16];
    u64to8(&mut msglen[0..8], hi);
    u64to8(&mut msglen[8..16], lo);

    if s.buflen == 888 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake512_update(s, &[oo], 8);
    } else {
        if s.buflen < 888 {
            if s.buflen == 0 { s.nullt = 1; }
            s.t[0] = s.t[0].wrapping_sub((888 - s.buflen) as u64);
            blake512_update(s, &PADDING512[..(888 - s.buflen) as usize / 8], (888 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((1024 - s.buflen) as u64);
            blake512_update(s, &PADDING512[..(1024 - s.buflen) as usize / 8], (1024 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(888);
            blake512_update(s, &PADDING512[1..1 + 888 / 8], 888);
            s.nullt = 1;
        }
        blake512_update(s, &[zo], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(128);
    blake512_update(s, &msglen, 128);

    for i in 0..8 {
        u64to8(&mut digest[i * 8..], s.h[i]);
    }
}

pub fn blake512(out: &mut [u8], inp: &[u8], inlen: u64) {
    let mut s = BlakeState512 { h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128] };
    blake512_init(&mut s);
    blake512_update(&mut s, inp, inlen * 8);
    blake512_final(&mut s, out);
}

pub fn blake512_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut i: u32 = 0;
    let mut off = 0;
    while (i as usize + 1) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..inlen + 4], i);
        blake512(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += SPX_BLAKE512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i as usize * SPX_BLAKE512_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..inlen + 4], i);
        blake512(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - i as usize * SPX_BLAKE512_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}
