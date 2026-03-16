use crate::params::SPX_BLAKE256_OUTPUT_BYTES;
use crate::utils_impl::{u32_to_bytes_internal};

pub struct BlakeState256 {
    pub h: [u32; 8],
    pub s: [u32; 4],
    pub t: [u32; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 64],
}

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
    (p[0] as u32) << 24 | (p[1] as u32) << 16 | (p[2] as u32) << 8 | (p[3] as u32)
}

fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8;
    p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8;
    p[3] = v as u8;
}

#[inline(always)]
fn rot256(x: u32, n: u32) -> u32 {
    (x << (32 - n)) | (x >> n)
}

macro_rules! g256 {
    ($va:expr, $vb:expr, $vc:expr, $vd:expr, $m_a:expr, $c_a:expr, $m_b:expr, $c_b:expr) => {
        $va = $va.wrapping_add($m_a ^ $c_a);
        $va = $va.wrapping_add($vb);
        $vd ^= $va;
        $vd = rot256($vd, 16);
        $vc = $vc.wrapping_add($vd);
        $vb ^= $vc;
        $vb = rot256($vb, 12);
        $va = $va.wrapping_add($m_b ^ $c_b);
        $va = $va.wrapping_add($vb);
        $vd ^= $va;
        $vd = rot256($vd, 8);
        $vc = $vc.wrapping_add($vd);
        $vb ^= $vc;
        $vb = rot256($vb, 7);
    };
}

macro_rules! round256 {
    ($v:expr, $m:expr,
     $s0:expr,$s1:expr,$s2:expr,$s3:expr,$s4:expr,$s5:expr,$s6:expr,$s7:expr,
     $s8:expr,$s9:expr,$s10:expr,$s11:expr,$s12:expr,$s13:expr,$s14:expr,$s15:expr) => {
        // columns
        g256!($v[0], $v[4], $v[8],  $v[12], $m[$s0], CST256[$s1], $m[$s1], CST256[$s0]);
        g256!($v[1], $v[5], $v[9],  $v[13], $m[$s2], CST256[$s3], $m[$s3], CST256[$s2]);
        g256!($v[2], $v[6], $v[10], $v[14], $m[$s4], CST256[$s5], $m[$s5], CST256[$s4]);
        g256!($v[3], $v[7], $v[11], $v[15], $m[$s6], CST256[$s7], $m[$s7], CST256[$s6]);
        // diagonals
        g256!($v[0], $v[5], $v[10], $v[15], $m[$s8], CST256[$s9], $m[$s9], CST256[$s8]);
        g256!($v[1], $v[6], $v[11], $v[12], $m[$s10], CST256[$s11], $m[$s11], CST256[$s10]);
        g256!($v[2], $v[7], $v[8],  $v[13], $m[$s12], CST256[$s13], $m[$s13], CST256[$s12]);
        g256!($v[3], $v[4], $v[9],  $v[14], $m[$s14], CST256[$s15], $m[$s15], CST256[$s14]);
    };
}

pub fn blake256_compress(s: &mut BlakeState256, block: &[u8]) {
    let mut m = [0u32; 16];
    for i in 0..16 {
        m[i] = u8to32(&block[i * 4..]);
    }

    let mut v = [0u32; 16];
    v[0] = s.h[0]; v[1] = s.h[1]; v[2] = s.h[2]; v[3] = s.h[3];
    v[4] = s.h[4]; v[5] = s.h[5]; v[6] = s.h[6]; v[7] = s.h[7];
    v[8]  = s.s[0] ^ 0x243F6A88;
    v[9]  = s.s[1] ^ 0x85A308D3;
    v[10] = s.s[2] ^ 0x13198A2E;
    v[11] = s.s[3] ^ 0x03707344;
    v[12] = 0xA4093822;
    v[13] = 0x299F31D0;
    v[14] = 0x082EFA98;
    v[15] = 0xEC4E6C89;

    if s.nullt == 0 {
        v[12] ^= s.t[0];
        v[13] ^= s.t[0];
        v[14] ^= s.t[1];
        v[15] ^= s.t[1];
    }

    round256!(v, m, 0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15);
    round256!(v, m, 14,10,4,8,9,15,13,6,1,12,0,2,11,7,5,3);
    round256!(v, m, 11,8,12,0,5,2,15,13,10,14,3,6,7,1,9,4);
    round256!(v, m, 7,9,3,1,13,12,11,14,2,6,5,10,4,0,15,8);
    round256!(v, m, 9,0,5,7,2,4,10,15,14,1,11,12,6,8,3,13);
    round256!(v, m, 2,12,6,10,0,11,8,3,4,13,7,5,15,14,1,9);
    round256!(v, m, 12,5,1,15,14,13,4,10,0,7,6,3,9,2,8,11);
    round256!(v, m, 13,11,7,14,12,1,3,9,5,0,15,4,8,6,2,10);
    round256!(v, m, 6,15,14,9,11,3,0,8,12,2,13,7,1,4,10,5);
    round256!(v, m, 10,2,8,4,7,6,1,5,15,11,9,14,3,12,13,0);
    round256!(v, m, 0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15);
    round256!(v, m, 14,10,4,8,9,15,13,6,1,12,0,2,11,7,5,3);
    round256!(v, m, 11,8,12,0,5,2,15,13,10,14,3,6,7,1,9,4);
    round256!(v, m, 7,9,3,1,13,12,11,14,2,6,5,10,4,0,15,8);

    for i in 0..8 { v[i] ^= v[i + 8]; }
    for i in 0..4 { v[i] ^= s.s[i]; v[i+4] ^= s.s[i]; }
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
            blake256_update(s, &PADDING256[..(440 - s.buflen) as usize / 8], (440 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((512 - s.buflen) as u32);
            blake256_update(s, &PADDING256[..(512 - s.buflen) as usize / 8], (512 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(440);
            blake256_update(s, &PADDING256[1..1 + 440 / 8], 440);
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

pub fn blake256_hash(out: &mut [u8], inp: &[u8], inlen: u64) -> i32 {
    let mut s = BlakeState256 { h: [0;8], s: [0;4], t: [0;2], buflen: 0, nullt: 0, buf: [0;64] };
    blake256_init(&mut s);
    blake256_update(&mut s, inp, inlen.wrapping_mul(8));
    blake256_final(&mut s, out);
    0
}

pub fn blake256_mgf1_internal(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    let mut i: u32 = 0;
    let mut off = 0usize;
    while (i as usize + 1) * SPX_BLAKE256_OUTPUT_BYTES <= outlen {
        u32_to_bytes_internal(&mut inbuf[inlen..], i);
        blake256_hash(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += SPX_BLAKE256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i as usize * SPX_BLAKE256_OUTPUT_BYTES {
        u32_to_bytes_internal(&mut inbuf[inlen..], i);
        blake256_hash(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - i as usize * SPX_BLAKE256_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}
