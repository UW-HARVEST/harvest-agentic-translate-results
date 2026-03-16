pub struct Blake512State {
    pub h: [u64; 8],
    pub s: [u64; 4],
    pub t: [u64; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 128],
}

impl Blake512State {
    pub fn new() -> Self {
        Blake512State { h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128] }
    }
}

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

#[inline(always)]
fn rot(x: u64, n: u32) -> u64 { (x << (64 - n)) | (x >> n) }

macro_rules! g512 {
    ($v:expr, $a:expr, $b:expr, $c:expr, $d:expr, $mx:expr, $cx:expr, $my:expr, $cy:expr, $r1:expr, $r2:expr, $r3:expr, $r4:expr) => {
        $v[$a] = $v[$a].wrapping_add($mx ^ $cx).wrapping_add($v[$b]);
        $v[$d] ^= $v[$a]; $v[$d] = rot($v[$d], $r1);
        $v[$c] = $v[$c].wrapping_add($v[$d]); $v[$b] ^= $v[$c]; $v[$b] = rot($v[$b], $r2);
        $v[$a] = $v[$a].wrapping_add($my ^ $cy).wrapping_add($v[$b]);
        $v[$d] ^= $v[$a]; $v[$d] = rot($v[$d], $r3);
        $v[$c] = $v[$c].wrapping_add($v[$d]); $v[$b] ^= $v[$c]; $v[$b] = rot($v[$b], $r4);
    };
}

fn blake512_round(v: &mut [u64; 16], m: &[u64; 16], s: &[usize; 16]) {
    g512!(v, 0, 4, 8, 12, m[s[0]], CST[s[1]], m[s[1]], CST[s[0]], 32, 25, 16, 11);
    g512!(v, 1, 5, 9, 13, m[s[2]], CST[s[3]], m[s[3]], CST[s[2]], 32, 25, 16, 11);
    g512!(v, 2, 6, 10, 14, m[s[4]], CST[s[5]], m[s[5]], CST[s[4]], 32, 25, 16, 11);
    g512!(v, 3, 7, 11, 15, m[s[6]], CST[s[7]], m[s[7]], CST[s[6]], 32, 25, 16, 11);
    g512!(v, 0, 5, 10, 15, m[s[8]], CST[s[9]], m[s[9]], CST[s[8]], 32, 25, 16, 11);
    g512!(v, 1, 6, 11, 12, m[s[10]], CST[s[11]], m[s[11]], CST[s[10]], 32, 25, 16, 11);
    g512!(v, 2, 7, 8, 13, m[s[12]], CST[s[13]], m[s[13]], CST[s[12]], 32, 25, 16, 11);
    g512!(v, 3, 4, 9, 14, m[s[14]], CST[s[15]], m[s[15]], CST[s[14]], 32, 25, 16, 11);
}

static SIGMA: [[usize; 16]; 16] = [
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

pub fn blake512_compress(s: &mut Blake512State, block: &[u8]) {
    let mut m = [0u64; 16];
    for i in 0..16 { m[i] = u8to64(&block[i * 8..]); }

    let mut v = [0u64; 16];
    for i in 0..8 { v[i] = s.h[i]; }
    v[8] = s.s[0] ^ CST[0]; v[9] = s.s[1] ^ CST[1];
    v[10] = s.s[2] ^ CST[2]; v[11] = s.s[3] ^ CST[3];
    v[12] = CST[4]; v[13] = CST[5]; v[14] = CST[6]; v[15] = CST[7];

    if s.nullt == 0 {
        v[12] ^= s.t[0]; v[13] ^= s.t[0];
        v[14] ^= s.t[1]; v[15] ^= s.t[1];
    }

    for i in 0..16 { blake512_round(&mut v, &m, &SIGMA[i]); }

    for i in 0..8 { v[i] ^= v[i + 8]; }
    for i in 0..4 { v[i] ^= s.s[i]; v[i + 4] ^= s.s[i]; }
    for i in 0..8 { s.h[i] ^= v[i]; }
}

pub fn blake512_init(s: &mut Blake512State) {
    s.h = [
        0x6A09E667F3BCC908, 0xBB67AE8584CAA73B,
        0x3C6EF372FE94F82B, 0xA54FF53A5F1D36F1,
        0x510E527FADE682D1, 0x9B05688C2B3E6C1F,
        0x1F83D9ABFB41BD6B, 0x5BE0CD19137E2179,
    ];
    s.t = [0; 2]; s.buflen = 0; s.nullt = 0; s.s = [0; 4];
}

pub fn blake512_update(s: &mut Blake512State, data: &[u8], datalen_bits: u64) {
    let mut datalen = datalen_bits;
    let mut data = data;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 128 - left;

    if left != 0 && (((datalen >> 3) & 0x7F) as usize) >= fill {
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
        blake512_compress(s, data);
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

pub fn blake512_final(s: &mut Blake512State, digest: &mut [u8]) {
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
            let pad_bits = (888 - s.buflen) as u64;
            let pad_bytes = (pad_bits / 8) as usize + if pad_bits % 8 != 0 { 1 } else { 0 };
            blake512_update(s, &PADDING[..pad_bytes], pad_bits);
        } else {
            s.t[0] = s.t[0].wrapping_sub((1024 - s.buflen) as u64);
            let pad_bits = (1024 - s.buflen) as u64;
            let pad_bytes = (pad_bits / 8) as usize + if pad_bits % 8 != 0 { 1 } else { 0 };
            blake512_update(s, &PADDING[..pad_bytes], pad_bits);
            s.t[0] = s.t[0].wrapping_sub(888);
            blake512_update(s, &PADDING[1..1 + 888 / 8], 888);
            s.nullt = 1;
        }
        blake512_update(s, &[zo], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(128);
    blake512_update(s, &msglen, 128);

    for i in 0..8 { u64to8(&mut digest[i * 8..i * 8 + 8], s.h[i]); }
}

pub fn blake512(out: &mut [u8], data: &[u8], inlen: u64) {
    let mut s = Blake512State::new();
    blake512_init(&mut s);
    blake512_update(&mut s, data, inlen * 8);
    blake512_final(&mut s, out);
}

pub fn blake512_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; 64];
    let mut i: usize = 0;
    let mut off = 0usize;
    while (i + 1) * 64 <= outlen {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake512(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += 64;
        i += 1;
    }
    if outlen > i * 64 {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake512(&mut outbuf, &inbuf, (inlen + 4) as u64);
        out[off..off + (outlen - i * 64)].copy_from_slice(&outbuf[..outlen - i * 64]);
    }
}
