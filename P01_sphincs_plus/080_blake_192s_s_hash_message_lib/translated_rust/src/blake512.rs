use crate::params::SPX_BLAKE512_OUTPUT_BYTES;
use crate::utils::u32_to_bytes;

fn u8to32(p: &[u8]) -> u32 {
    ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32)
}
fn u8to64(p: &[u8]) -> u64 {
    ((u8to32(p) as u64) << 32) | (u8to32(&p[4..]) as u64)
}
fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8; p[1] = (v >> 16) as u8; p[2] = (v >> 8) as u8; p[3] = v as u8;
}
fn u64to8(p: &mut [u8], v: u64) {
    u32to8(&mut p[0..4], (v >> 32) as u32);
    u32to8(&mut p[4..8], v as u32);
}

static CST: [u64; 16] = [
    0x243F6A8885A308D3, 0x13198A2E03707344, 0xA4093822299F31D0, 0x082EFA98EC4E6C89,
    0x452821E638D01377, 0xBE5466CF34E90C6C, 0xC0AC29B7C97C50DD, 0x3F84D5B5B5470917,
    0x9216D5D98979FB1B, 0xD1310BA698DFB5AC, 0x2FFD72DBD01ADFB7, 0xB8E1AFED6A267E96,
    0xBA7C9045F12C7F99, 0x24A19947B3916CF7, 0x0801F2E2858EFC16, 0x636920D871574E69,
];

static PADDING: [u8; 129] = {
    let mut p = [0u8; 129];
    p[0] = 0x80;
    p
};

fn rot(x: u64, n: u32) -> u64 { (x << (64 - n)) | (x >> n) }

pub struct Blake512State {
    pub h: [u64; 8], pub s: [u64; 4], pub t: [u64; 2],
    pub buflen: i32, pub nullt: i32, pub buf: [u8; 128],
}

impl Blake512State {
    pub fn new() -> Self {
        let mut s = Blake512State { h: [0;8], s: [0;4], t: [0;2], buflen: 0, nullt: 0, buf: [0;128] };
        blake512_init(&mut s); s
    }
}

pub fn blake512_init(s: &mut Blake512State) {
    s.h = [0x6A09E667F3BCC908, 0xBB67AE8584CAA73B, 0x3C6EF372FE94F82B, 0xA54FF53A5F1D36F1,
           0x510E527FADE682D1, 0x9B05688C2B3E6C1F, 0x1F83D9ABFB41BD6B, 0x5BE0CD19137E2179];
    s.t = [0;2]; s.buflen = 0; s.nullt = 0; s.s = [0;4];
}

pub fn blake512_compress(s: &mut Blake512State, block: &[u8]) {
    let m: Vec<u64> = (0..16).map(|i| u8to64(&block[i*8..])).collect();
    let (m0,m1,m2,m3,m4,m5,m6,m7) = (m[0],m[1],m[2],m[3],m[4],m[5],m[6],m[7]);
    let (m8,m9,m10,m11,m12,m13,m14,m15) = (m[8],m[9],m[10],m[11],m[12],m[13],m[14],m[15]);

    let (mut v0,mut v1,mut v2,mut v3) = (s.h[0],s.h[1],s.h[2],s.h[3]);
    let (mut v4,mut v5,mut v6,mut v7) = (s.h[4],s.h[5],s.h[6],s.h[7]);
    let mut v8 = s.s[0] ^ 0x243F6A8885A308D3u64;
    let mut v9 = s.s[1] ^ 0x13198A2E03707344u64;
    let mut v10 = s.s[2] ^ 0xA4093822299F31D0u64;
    let mut v11 = s.s[3] ^ 0x082EFA98EC4E6C89u64;
    let (mut v12,mut v13,mut v14,mut v15) = (0x452821E638D01377u64,0xBE5466CF34E90C6Cu64,0xC0AC29B7C97C50DDu64,0x3F84D5B5B5470917u64);

    if s.nullt == 0 { v12 ^= s.t[0]; v13 ^= s.t[0]; v14 ^= s.t[1]; v15 ^= s.t[1]; }

    macro_rules! R {
        ($m0:expr,$c0:expr,$m1:expr,$c1:expr,$m2:expr,$c2:expr,$m3:expr,$c3:expr,
         $m4:expr,$c4:expr,$m5:expr,$c5:expr,$m6:expr,$c6:expr,$m7:expr,$c7:expr,
         $m8:expr,$c8:expr,$m9:expr,$c9:expr,$m10:expr,$c10:expr,$m11:expr,$c11:expr,
         $m12:expr,$c12:expr,$m13:expr,$c13:expr,$m14:expr,$c14:expr,$m15:expr,$c15:expr) => {
            v0=v0.wrapping_add($m0^$c0); v0=v0.wrapping_add(v4); v12^=v0; v12=rot(v12,32); v8=v8.wrapping_add(v12); v4^=v8; v4=rot(v4,25);
            v1=v1.wrapping_add($m2^$c2); v1=v1.wrapping_add(v5); v13^=v1; v13=rot(v13,32); v9=v9.wrapping_add(v13); v5^=v9; v5=rot(v5,25);
            v2=v2.wrapping_add($m4^$c4); v2=v2.wrapping_add(v6); v14^=v2; v14=rot(v14,32); v10=v10.wrapping_add(v14); v6^=v10; v6=rot(v6,25);
            v3=v3.wrapping_add($m6^$c6); v3=v3.wrapping_add(v7); v15^=v3; v15=rot(v15,32); v11=v11.wrapping_add(v15); v7^=v11; v7=rot(v7,25);
            v2=v2.wrapping_add($m5^$c5); v2=v2.wrapping_add(v6); v14^=v2; v14=rot(v14,16); v10=v10.wrapping_add(v14); v6^=v10; v6=rot(v6,11);
            v3=v3.wrapping_add($m7^$c7); v3=v3.wrapping_add(v7); v15^=v3; v15=rot(v15,16); v11=v11.wrapping_add(v15); v7^=v11; v7=rot(v7,11);
            v1=v1.wrapping_add($m3^$c3); v1=v1.wrapping_add(v5); v13^=v1; v13=rot(v13,16); v9=v9.wrapping_add(v13); v5^=v9; v5=rot(v5,11);
            v0=v0.wrapping_add($m1^$c1); v0=v0.wrapping_add(v4); v12^=v0; v12=rot(v12,16); v8=v8.wrapping_add(v12); v4^=v8; v4=rot(v4,11);
            v0=v0.wrapping_add($m8^$c8); v0=v0.wrapping_add(v5); v15^=v0; v15=rot(v15,32); v10=v10.wrapping_add(v15); v5^=v10; v5=rot(v5,25);
            v1=v1.wrapping_add($m10^$c10); v1=v1.wrapping_add(v6); v12^=v1; v12=rot(v12,32); v11=v11.wrapping_add(v12); v6^=v11; v6=rot(v6,25);
            v2=v2.wrapping_add($m12^$c12); v2=v2.wrapping_add(v7); v13^=v2; v13=rot(v13,32); v8=v8.wrapping_add(v13); v7^=v8; v7=rot(v7,25);
            v3=v3.wrapping_add($m14^$c14); v3=v3.wrapping_add(v4); v14^=v3; v14=rot(v14,32); v9=v9.wrapping_add(v14); v4^=v9; v4=rot(v4,25);
            v2=v2.wrapping_add($m13^$c13); v2=v2.wrapping_add(v7); v13^=v2; v13=rot(v13,16); v8=v8.wrapping_add(v13); v7^=v8; v7=rot(v7,11);
            v3=v3.wrapping_add($m15^$c15); v3=v3.wrapping_add(v4); v14^=v3; v14=rot(v14,16); v9=v9.wrapping_add(v14); v4^=v9; v4=rot(v4,11);
            v1=v1.wrapping_add($m11^$c11); v1=v1.wrapping_add(v6); v12^=v1; v12=rot(v12,16); v11=v11.wrapping_add(v12); v6^=v11; v6=rot(v6,11);
            v0=v0.wrapping_add($m9^$c9); v0=v0.wrapping_add(v5); v15^=v0; v15=rot(v15,16); v10=v10.wrapping_add(v15); v5^=v10; v5=rot(v5,11);
        };
    }

    R!(m0,CST[1],m1,CST[0],m2,CST[3],m3,CST[2],m4,CST[5],m5,CST[4],m6,CST[7],m7,CST[6],m8,CST[9],m9,CST[8],m10,CST[11],m11,CST[10],m12,CST[13],m13,CST[12],m14,CST[15],m15,CST[14]);
    R!(m14,CST[10],m10,CST[14],m4,CST[8],m8,CST[4],m9,CST[15],m15,CST[9],m13,CST[6],m6,CST[13],m1,CST[12],m12,CST[1],m0,CST[2],m2,CST[0],m11,CST[7],m7,CST[11],m5,CST[3],m3,CST[5]);
    R!(m11,CST[8],m8,CST[11],m12,CST[0],m0,CST[12],m5,CST[2],m2,CST[5],m15,CST[13],m13,CST[15],m10,CST[14],m14,CST[10],m3,CST[6],m6,CST[3],m7,CST[1],m1,CST[7],m9,CST[4],m4,CST[9]);
    R!(m7,CST[9],m9,CST[7],m3,CST[1],m1,CST[3],m13,CST[12],m12,CST[13],m11,CST[14],m14,CST[11],m2,CST[6],m6,CST[2],m5,CST[10],m10,CST[5],m4,CST[0],m0,CST[4],m15,CST[8],m8,CST[15]);
    R!(m9,CST[0],m0,CST[9],m5,CST[7],m7,CST[5],m2,CST[4],m4,CST[2],m10,CST[15],m15,CST[10],m14,CST[1],m1,CST[14],m11,CST[12],m12,CST[11],m6,CST[8],m8,CST[6],m3,CST[13],m13,CST[3]);
    R!(m2,CST[12],m12,CST[2],m6,CST[10],m10,CST[6],m0,CST[11],m11,CST[0],m8,CST[3],m3,CST[8],m4,CST[13],m13,CST[4],m7,CST[5],m5,CST[7],m15,CST[14],m14,CST[15],m1,CST[9],m9,CST[1]);
    R!(m12,CST[5],m5,CST[12],m1,CST[15],m15,CST[1],m14,CST[13],m13,CST[14],m4,CST[10],m10,CST[4],m0,CST[7],m7,CST[0],m6,CST[3],m3,CST[6],m9,CST[2],m2,CST[9],m8,CST[11],m11,CST[8]);
    R!(m13,CST[11],m11,CST[13],m7,CST[14],m14,CST[7],m12,CST[1],m1,CST[12],m3,CST[9],m9,CST[3],m5,CST[0],m0,CST[5],m15,CST[4],m4,CST[15],m8,CST[6],m6,CST[8],m2,CST[10],m10,CST[2]);
    R!(m6,CST[15],m15,CST[6],m14,CST[9],m9,CST[14],m11,CST[3],m3,CST[11],m0,CST[8],m8,CST[0],m12,CST[2],m2,CST[12],m13,CST[7],m7,CST[13],m1,CST[4],m4,CST[1],m10,CST[5],m5,CST[10]);
    R!(m10,CST[2],m2,CST[10],m8,CST[4],m4,CST[8],m7,CST[6],m6,CST[7],m1,CST[5],m5,CST[1],m15,CST[11],m11,CST[15],m9,CST[14],m14,CST[9],m3,CST[12],m12,CST[3],m13,CST[0],m0,CST[13]);
    R!(m0,CST[1],m1,CST[0],m2,CST[3],m3,CST[2],m4,CST[5],m5,CST[4],m6,CST[7],m7,CST[6],m8,CST[9],m9,CST[8],m10,CST[11],m11,CST[10],m12,CST[13],m13,CST[12],m14,CST[15],m15,CST[14]);
    R!(m14,CST[10],m10,CST[14],m4,CST[8],m8,CST[4],m9,CST[15],m15,CST[9],m13,CST[6],m6,CST[13],m1,CST[12],m12,CST[1],m0,CST[2],m2,CST[0],m11,CST[7],m7,CST[11],m5,CST[3],m3,CST[5]);
    R!(m11,CST[8],m8,CST[11],m12,CST[0],m0,CST[12],m5,CST[2],m2,CST[5],m15,CST[13],m13,CST[15],m10,CST[14],m14,CST[10],m3,CST[6],m6,CST[3],m7,CST[1],m1,CST[7],m9,CST[4],m4,CST[9]);
    R!(m7,CST[9],m9,CST[7],m3,CST[1],m1,CST[3],m13,CST[12],m12,CST[13],m11,CST[14],m14,CST[11],m2,CST[6],m6,CST[2],m5,CST[10],m10,CST[5],m4,CST[0],m0,CST[4],m15,CST[8],m8,CST[15]);
    R!(m9,CST[0],m0,CST[9],m5,CST[7],m7,CST[5],m2,CST[4],m4,CST[2],m10,CST[15],m15,CST[10],m14,CST[1],m1,CST[14],m11,CST[12],m12,CST[11],m6,CST[8],m8,CST[6],m3,CST[13],m13,CST[3]);
    R!(m2,CST[12],m12,CST[2],m6,CST[10],m10,CST[6],m0,CST[11],m11,CST[0],m8,CST[3],m3,CST[8],m4,CST[13],m13,CST[4],m7,CST[5],m5,CST[7],m15,CST[14],m14,CST[15],m1,CST[9],m9,CST[1]);

    v0^=v8; v1^=v9; v2^=v10; v3^=v11; v4^=v12; v5^=v13; v6^=v14; v7^=v15;
    v0^=s.s[0]; v1^=s.s[1]; v2^=s.s[2]; v3^=s.s[3];
    v4^=s.s[0]; v5^=s.s[1]; v6^=s.s[2]; v7^=s.s[3];
    s.h[0]^=v0; s.h[1]^=v1; s.h[2]^=v2; s.h[3]^=v3;
    s.h[4]^=v4; s.h[5]^=v5; s.h[6]^=v6; s.h[7]^=v7;
}

pub fn blake512_update(s: &mut Blake512State, data: &[u8], mut datalen: u64) {
    let mut off = 0usize;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 128 - left;

    if left != 0 && ((datalen >> 3) & 0x7F) as usize >= fill {
        s.buf[left..left+fill].copy_from_slice(&data[off..off+fill]);
        s.t[0] = s.t[0].wrapping_add(1024);
        let buf = s.buf;
        blake512_compress(s, &buf);
        off += fill; datalen -= (fill as u64) << 3; left = 0;
    }
    while datalen >= 1024 {
        s.t[0] = s.t[0].wrapping_add(1024);
        blake512_compress(s, &data[off..]);
        off += 128; datalen -= 1024;
    }
    if datalen > 0 {
        let bytes = ((datalen >> 3) & 0x7F) as usize;
        s.buf[left..left+bytes].copy_from_slice(&data[off..off+bytes]);
        s.buflen = ((left << 3) as u64 + datalen) as i32;
    } else { s.buflen = 0; }
}

pub fn blake512_final(s: &mut Blake512State, digest: &mut [u8]) {
    let mut msglen = [0u8; 16];
    let zo: u8 = 0x01; let oo: u8 = 0x81;
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
            blake512_update(s, &PADDING[1..1+888/8], 888);
            s.nullt = 1;
        }
        blake512_update(s, &[zo], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(128);
    blake512_update(s, &msglen, 128);

    for i in 0..8 { u64to8(&mut digest[i*8..i*8+8], s.h[i]); }
}

pub fn blake512(out: &mut [u8], inp: &[u8], inlen: u64) -> i32 {
    let mut s = Blake512State::new();
    blake512_update(&mut s, inp, inlen.wrapping_mul(8));
    blake512_final(&mut s, out);
    0
}

pub fn blake512_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    let mut i: usize = 0;
    let mut out_off = 0usize;
    while (i + 1) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..inlen+4], i as u32);
        blake512(&mut out[out_off..], &inbuf, (inlen + 4) as u64);
        out_off += SPX_BLAKE512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_BLAKE512_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..inlen+4], i as u32);
        blake512(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - i * SPX_BLAKE512_OUTPUT_BYTES;
        out[out_off..out_off+rem].copy_from_slice(&outbuf[..rem]);
    }
}
