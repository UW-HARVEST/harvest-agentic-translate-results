use crate::params::SPX_BLAKE512_OUTPUT_BYTES;
use crate::utils_impl::u32_to_bytes_internal;

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

fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8;
    p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8;
    p[3] = v as u8;
}

fn u64to8(p: &mut [u8], v: u64) {
    u32to8(&mut p[0..4], (v >> 32) as u32);
    u32to8(&mut p[4..8], v as u32);
}

#[inline(always)]
fn rot512(x: u64, n: u32) -> u64 {
    (x << (64 - n)) | (x >> n)
}

macro_rules! g512 {
    ($va:expr, $vb:expr, $vc:expr, $vd:expr, $m_a:expr, $c_a:expr, $m_b:expr, $c_b:expr) => {
        $va = $va.wrapping_add($m_a ^ $c_a);
        $va = $va.wrapping_add($vb);
        $vd ^= $va;
        $vd = rot512($vd, 32);
        $vc = $vc.wrapping_add($vd);
        $vb ^= $vc;
        $vb = rot512($vb, 25);
        $va = $va.wrapping_add($m_b ^ $c_b);
        $va = $va.wrapping_add($vb);
        $vd ^= $va;
        $vd = rot512($vd, 16);
        $vc = $vc.wrapping_add($vd);
        $vb ^= $vc;
        $vb = rot512($vb, 11);
    };
}

macro_rules! round512 {
    ($v:expr, $m:expr,
     $s0:expr,$s1:expr,$s2:expr,$s3:expr,$s4:expr,$s5:expr,$s6:expr,$s7:expr,
     $s8:expr,$s9:expr,$s10:expr,$s11:expr,$s12:expr,$s13:expr,$s14:expr,$s15:expr) => {
        g512!($v[0], $v[4], $v[8],  $v[12], $m[$s0], CST512[$s1], $m[$s1], CST512[$s0]);
        g512!($v[1], $v[5], $v[9],  $v[13], $m[$s2], CST512[$s3], $m[$s3], CST512[$s2]);
        g512!($v[2], $v[6], $v[10], $v[14], $m[$s4], CST512[$s5], $m[$s5], CST512[$s4]);
        g512!($v[3], $v[7], $v[11], $v[15], $m[$s6], CST512[$s7], $m[$s7], CST512[$s6]);
        g512!($v[0], $v[5], $v[10], $v[15], $m[$s8], CST512[$s9], $m[$s9], CST512[$s8]);
        g512!($v[1], $v[6], $v[11], $v[12], $m[$s10], CST512[$s11], $m[$s11], CST512[$s10]);
        g512!($v[2], $v[7], $v[8],  $v[13], $m[$s12], CST512[$s13], $m[$s13], CST512[$s12]);
        g512!($v[3], $v[4], $v[9],  $v[14], $m[$s14], CST512[$s15], $m[$s15], CST512[$s14]);
    };
}

pub fn blake512_compress(s: &mut BlakeState512, block: &[u8]) {
    let mut m = [0u64; 16];
    for i in 0..16 {
        m[i] = u8to64(&block[i * 8..]);
    }

    let mut v = [0u64; 16];
    v[0] = s.h[0]; v[1] = s.h[1]; v[2] = s.h[2]; v[3] = s.h[3];
    v[4] = s.h[4]; v[5] = s.h[5]; v[6] = s.h[6]; v[7] = s.h[7];
    v[8]  = s.s[0] ^ 0x243F6A8885A308D3;
    v[9]  = s.s[1] ^ 0x13198A2E03707344;
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

    round512!(v, m, 0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15);
    round512!(v, m, 14,10,4,8,9,15,13,6,1,12,0,2,11,7,5,3);
    round512!(v, m, 11,8,12,0,5,2,15,13,10,14,3,6,7,1,9,4);
    round512!(v, m, 7,9,3,1,13,12,11,14,2,6,5,10,4,0,15,8);
    round512!(v, m, 9,0,5,7,2,4,10,15,14,1,11,12,6,8,3,13);
    round512!(v, m, 2,12,6,10,0,11,8,3,4,13,7,5,15,14,1,9);
    round512!(v, m, 12,5,1,15,14,13,4,10,0,7,6,3,9,2,8,11);
    round512!(v, m, 13,11,7,14,12,1,3,9,5,0,15,4,8,6,2,10);
    round512!(v, m, 6,15,14,9,11,3,0,8,12,2,13,7,1,4,10,5);
    round512!(v, m, 10,2,8,4,7,6,1,5,15,11,9,14,3,12,13,0);
    round512!(v, m, 0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15);
    round512!(v, m, 14,10,4,8,9,15,13,6,1,12,0,2,11,7,5,3);
    round512!(v, m, 11,8,12,0,5,2,15,13,10,14,3,6,7,1,9,4);
    round512!(v, m, 7,9,3,1,13,12,11,14,2,6,5,10,4,0,15,8);
    round512!(v, m, 9,0,5,7,2,4,10,15,14,1,11,12,6,8,3,13);
    round512!(v, m, 2,12,6,10,0,11,8,3,4,13,7,5,15,14,1,9);

    for i in 0..8 { v[i] ^= v[i + 8]; }
    for i in 0..4 { v[i] ^= s.s[i]; v[i+4] ^= s.s[i]; }
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

pub fn blake512_update(s: &mut BlakeState512, data: &[u8], datalen_bits: u64) {
    let mut datalen = datalen_bits;
    let mut offset = 0usize;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 128 - left;

    if left != 0 && ((datalen >> 3) & 0x7F) >= fill as u64 {
        s.buf[left..left + fill].copy_from_slice(&data[offset..offset + fill]);
        s.t[0] = s.t[0].wrapping_add(1024);
        let buf_copy = s.buf;
        blake512_compress(s, &buf_copy);
        offset += fill;
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 1024 {
        s.t[0] = s.t[0].wrapping_add(1024);
        blake512_compress(s, &data[offset..]);
        offset += 128;
        datalen -= 1024;
    }

    if datalen > 0 {
        let bytes = ((datalen >> 3) & 0x7F) as usize;
        s.buf[left..left + bytes].copy_from_slice(&data[offset..offset + bytes]);
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

pub fn blake512_hash(out: &mut [u8], inp: &[u8], inlen: u64) -> i32 {
    let mut s = BlakeState512 { h: [0;8], s: [0;4], t: [0;2], buflen: 0, nullt: 0, buf: [0;128] };
    blake512_init(&mut s);
    blake512_update(&mut s, inp, inlen.wrapping_mul(8));
    blake512_final(&mut s, out);
    0
}

pub fn blake512_mgf1_internal(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];

    let mut i: u32 = 0;
    let mut off = 0usize;
    while (i as usize + 1) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        u32_to_bytes_internal(&mut inbuf[inlen..], i);
        blake512_hash(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += SPX_BLAKE512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i as usize * SPX_BLAKE512_OUTPUT_BYTES {
        u32_to_bytes_internal(&mut inbuf[inlen..], i);
        blake512_hash(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - i as usize * SPX_BLAKE512_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}
