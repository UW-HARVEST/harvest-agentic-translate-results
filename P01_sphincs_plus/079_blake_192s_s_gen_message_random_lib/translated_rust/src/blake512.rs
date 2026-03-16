pub struct BlakeState512 {
    pub h: [u64; 8],
    pub s: [u64; 4],
    pub t: [u64; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 128],
}

const CST: [u64; 16] = [
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

fn u8to64(p: &[u8]) -> u64 {
    ((p[0] as u64) << 56) | ((p[1] as u64) << 48) | ((p[2] as u64) << 40) | ((p[3] as u64) << 32)
    | ((p[4] as u64) << 24) | ((p[5] as u64) << 16) | ((p[6] as u64) << 8) | (p[7] as u64)
}

fn u64to8(p: &mut [u8], v: u64) {
    p[0] = (v >> 56) as u8; p[1] = (v >> 48) as u8;
    p[2] = (v >> 40) as u8; p[3] = (v >> 32) as u8;
    p[4] = (v >> 24) as u8; p[5] = (v >> 16) as u8;
    p[6] = (v >> 8) as u8;  p[7] = v as u8;
}

fn rot(x: u64, n: u32) -> u64 { x.rotate_right(n) }

pub fn blake512_compress(s: &mut BlakeState512, block: &[u8]) {
    let mut m = [0u64; 16];
    for i in 0..16 { m[i] = u8to64(&block[i*8..]); }
    let mut v = [0u64; 16];
    for i in 0..8 { v[i] = s.h[i]; }
    v[8]  = s.s[0] ^ CST[0]; v[9]  = s.s[1] ^ CST[1];
    v[10] = s.s[2] ^ CST[2]; v[11] = s.s[3] ^ CST[3];
    v[12] = CST[4]; v[13] = CST[5]; v[14] = CST[6]; v[15] = CST[7];
    if s.nullt == 0 {
        v[12] ^= s.t[0]; v[13] ^= s.t[0];
        v[14] ^= s.t[1]; v[15] ^= s.t[1];
    }

    macro_rules! g {
        ($a:expr,$b:expr,$c:expr,$d:expr,$mi:expr,$ci:expr,$mj:expr,$cj:expr,$r1:expr,$r2:expr) => {
            v[$a] = v[$a].wrapping_add(m[$mi] ^ CST[$ci]).wrapping_add(v[$b]);
            v[$d] ^= v[$a]; v[$d] = rot(v[$d], $r1);
            v[$c] = v[$c].wrapping_add(v[$d]);
            v[$b] ^= v[$c]; v[$b] = rot(v[$b], $r2);
        };
    }

    for r in 0..16 {
        let p = &SIGMA[r];
        g!(0,4,8, 12,p[0], p[1], p[1], p[0], 32,25);
        g!(1,5,9, 13,p[2], p[3], p[3], p[2], 32,25);
        g!(2,6,10,14,p[4], p[5], p[5], p[4], 32,25);
        g!(3,7,11,15,p[6], p[7], p[7], p[6], 32,25);
        g!(2,6,10,14,p[5], p[4], p[4], p[5], 16,11);
        g!(3,7,11,15,p[7], p[6], p[6], p[7], 16,11);
        g!(1,5,9, 13,p[3], p[2], p[2], p[3], 16,11);
        g!(0,4,8, 12,p[1], p[0], p[0], p[1], 16,11);
        g!(0,5,10,15,p[8], p[9], p[9], p[8], 32,25);
        g!(1,6,11,12,p[10],p[11],p[11],p[10],32,25);
        g!(2,7,8, 13,p[12],p[13],p[13],p[12],32,25);
        g!(3,4,9, 14,p[14],p[15],p[15],p[14],32,25);
        g!(2,7,8, 13,p[13],p[12],p[12],p[13],16,11);
        g!(3,4,9, 14,p[15],p[14],p[14],p[15],16,11);
        g!(1,6,11,12,p[11],p[10],p[10],p[11],16,11);
        g!(0,5,10,15,p[9], p[8], p[8], p[9], 16,11);
    }

    for i in 0..8 { v[i] ^= v[i+8]; }
    for i in 0..4 { v[i] ^= s.s[i]; v[i+4] ^= s.s[i]; }
    for i in 0..8 { s.h[i] ^= v[i]; }
}

pub fn blake512_init(s: &mut BlakeState512) {
    s.h = [0x6A09E667F3BCC908, 0xBB67AE8584CAA73B,
           0x3C6EF372FE94F82B, 0xA54FF53A5F1D36F1,
           0x510E527FADE682D1, 0x9B05688C2B3E6C1F,
           0x1F83D9ABFB41BD6B, 0x5BE0CD19137E2179];
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
        let buf_copy = s.buf;
        blake512_compress(s, &buf_copy);
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
        let bytes = ((datalen >> 3) & 0x7F) as usize;
        s.buf[left..left+bytes].copy_from_slice(&data[..bytes]);
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
        u64to8(&mut digest[i*8..], s.h[i]);
    }
}

pub fn blake512(out: &mut [u8], inp: &[u8], inlen: u64) -> i32 {
    let mut s = BlakeState512 {
        h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128],
    };
    blake512_init(&mut s);
    blake512_update(&mut s, inp, inlen * 8);
    blake512_final(&mut s, out);
    0
}

pub fn blake512_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; crate::params::SPX_BLAKE512_OUTPUT_BYTES];
    let block = crate::params::SPX_BLAKE512_OUTPUT_BYTES;
    let mut i: u64 = 0;
    let mut off = 0usize;
    while (i + 1) * block as u64 <= outlen as u64 {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..inlen+4], i as u32);
        blake512(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += block;
        i += 1;
    }
    if outlen > i as usize * block {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..inlen+4], i as u32);
        blake512(&mut outbuf, &inbuf, (inlen + 4) as u64);
        out[off..off + (outlen - i as usize * block)]
            .copy_from_slice(&outbuf[..outlen - i as usize * block]);
    }
}
