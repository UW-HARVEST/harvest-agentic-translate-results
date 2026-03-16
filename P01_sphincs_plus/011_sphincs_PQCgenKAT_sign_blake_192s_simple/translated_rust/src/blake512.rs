use crate::params::*;

pub struct BlakeState512 {
    pub h: [u64; 8],
    pub s: [u64; 4],
    pub t: [u64; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 128],
}

const CST512: [u64; 16] = [
    0x243F6A8885A308D3,0x13198A2E03707344,0xA4093822299F31D0,0x082EFA98EC4E6C89,
    0x452821E638D01377,0xBE5466CF34E90C6C,0xC0AC29B7C97C50DD,0x3F84D5B5B5470917,
    0x9216D5D98979FB1B,0xD1310BA698DFB5AC,0x2FFD72DBD01ADFB7,0xB8E1AFED6A267E96,
    0xBA7C9045F12C7F99,0x24A19947B3916CF7,0x0801F2E2858EFC16,0x636920D871574E69,
];

static PADDING512: [u8; 129] = {
    let mut p = [0u8; 129];
    p[0] = 0x80;
    p
};

fn u8to64(p: &[u8]) -> u64 {
    let hi = (p[0] as u64) << 24 | (p[1] as u64) << 16 | (p[2] as u64) << 8 | (p[3] as u64);
    let lo = (p[4] as u64) << 24 | (p[5] as u64) << 16 | (p[6] as u64) << 8 | (p[7] as u64);
    (hi << 32) | lo
}

fn u64to8(p: &mut [u8], v: u64) {
    p[0] = (v >> 56) as u8; p[1] = (v >> 48) as u8;
    p[2] = (v >> 40) as u8; p[3] = (v >> 32) as u8;
    p[4] = (v >> 24) as u8; p[5] = (v >> 16) as u8;
    p[6] = (v >> 8) as u8;  p[7] = v as u8;
}

fn rot512(x: u64, n: u32) -> u64 {
    (x << (64 - n)) | (x >> n)
}

pub fn blake512_compress(s: &mut BlakeState512, block: &[u8]) {
    let m: [u64; 16] = core::array::from_fn(|i| u8to64(&block[i*8..]));
    let mut v = [0u64; 16];
    for i in 0..8 { v[i] = s.h[i]; }
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

    for r in 0..16 {
        let sg = &SIGMA[r];
        macro_rules! g512 {
            ($a:expr,$b:expr,$c:expr,$d:expr,$i:expr,$j:expr) => {
                v[$a] = v[$a].wrapping_add(m[sg[$i]] ^ CST512[sg[$j]]).wrapping_add(v[$b]);
                v[$d] ^= v[$a]; v[$d] = rot512(v[$d], 32);
                v[$c] = v[$c].wrapping_add(v[$d]);
                v[$b] ^= v[$c]; v[$b] = rot512(v[$b], 25);
                v[$a] = v[$a].wrapping_add(m[sg[$j]] ^ CST512[sg[$i]]).wrapping_add(v[$b]);
                v[$d] ^= v[$a]; v[$d] = rot512(v[$d], 16);
                v[$c] = v[$c].wrapping_add(v[$d]);
                v[$b] ^= v[$c]; v[$b] = rot512(v[$b], 11);
            }
        }
        g512!(0,4,8,12,0,1);
        g512!(1,5,9,13,2,3);
        g512!(2,6,10,14,4,5);
        g512!(3,7,11,15,6,7);
        g512!(0,5,10,15,8,9);
        g512!(1,6,11,12,10,11);
        g512!(2,7,8,13,12,13);
        g512!(3,4,9,14,14,15);
    }

    for i in 0..8 { v[i] ^= v[i+8]; }
    for i in 0..4 { v[i] ^= s.s[i]; v[i+4] ^= s.s[i]; }
    for i in 0..8 { s.h[i] ^= v[i]; }
}

pub fn blake512_init(s: &mut BlakeState512) {
    s.h = [0x6A09E667F3BCC908,0xBB67AE8584CAA73B,0x3C6EF372FE94F82B,0xA54FF53A5F1D36F1,
           0x510E527FADE682D1,0x9B05688C2B3E6C1F,0x1F83D9ABFB41BD6B,0x5BE0CD19137E2179];
    s.t = [0; 2]; s.buflen = 0; s.nullt = 0; s.s = [0; 4];
}

pub fn blake512_update(s: &mut BlakeState512, data: &[u8], datalen_bits: u64) {
    let mut data = data;
    let mut datalen = datalen_bits;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 128 - left;

    if left != 0 && ((datalen >> 3) & 0x7F) >= fill as u64 {
        s.buf[left..left+fill].copy_from_slice(&data[..fill]);
        s.t[0] = s.t[0].wrapping_add(1024);
        blake512_compress(s, &s.buf.clone());
        data = &data[fill..];
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 1024 {
        s.t[0] = s.t[0].wrapping_add(1024);
        blake512_compress(s, data);
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
            blake512_update(s, &PADDING512[1..1 + 888/8], 888);
            s.nullt = 1;
        }
        blake512_update(s, &[zo], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(128);
    blake512_update(s, &msglen, 128);

    for i in 0..8 {
        u64to8(&mut digest[8*i..], s.h[i]);
    }
}

pub fn blake512(out: &mut [u8], inp: &[u8], inlen: usize) {
    let mut s = BlakeState512 { h: [0;8], s: [0;4], t: [0;2], buflen: 0, nullt: 0, buf: [0;128] };
    blake512_init(&mut s);
    blake512_update(&mut s, inp, (inlen as u64) * 8);
    blake512_final(&mut s, out);
}

pub fn blake512_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut i: usize = 0;
    while (i + 1) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake512(&mut out[i * SPX_BLAKE512_OUTPUT_BYTES..], &inbuf, inlen + 4);
        i += 1;
    }
    if outlen > i * SPX_BLAKE512_OUTPUT_BYTES {
        let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
        crate::utils::u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake512(&mut outbuf, &inbuf, inlen + 4);
        let rem = outlen - i * SPX_BLAKE512_OUTPUT_BYTES;
        out[i * SPX_BLAKE512_OUTPUT_BYTES..i * SPX_BLAKE512_OUTPUT_BYTES + rem].copy_from_slice(&outbuf[..rem]);
    }
}
