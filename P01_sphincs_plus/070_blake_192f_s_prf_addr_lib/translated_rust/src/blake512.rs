use crate::address::u32_to_bytes;
use crate::params::SPX_BLAKE512_OUTPUT_BYTES;

const CST: [u64; 16] = [
    0x243F6A8885A308D3, 0x13198A2E03707344, 0xA4093822299F31D0, 0x082EFA98EC4E6C89,
    0x452821E638D01377, 0xBE5466CF34E90C6C, 0xC0AC29B7C97C50DD, 0x3F84D5B5B5470917,
    0x9216D5D98979FB1B, 0xD1310BA698DFB5AC, 0x2FFD72DBD01ADFB7, 0xB8E1AFED6A267E96,
    0xBA7C9045F12C7F99, 0x24A19947B3916CF7, 0x0801F2E2858EFC16, 0x636920D871574E69,
];

static PADDING: [u8; 129] = [
    0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0,
];

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
    u32to8(p, (v >> 32) as u32);
    u32to8(&mut p[4..], v as u32);
}

fn rot64(x: u64, n: u32) -> u64 {
    (x << (64 - n)) | (x >> n)
}

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
        let mut s = Blake512State {
            h: [0; 8],
            s: [0; 4],
            t: [0; 2],
            buflen: 0,
            nullt: 0,
            buf: [0; 128],
        };
        s.init();
        s
    }

    pub fn init(&mut self) {
        self.h[0] = 0x6A09E667F3BCC908;
        self.h[1] = 0xBB67AE8584CAA73B;
        self.h[2] = 0x3C6EF372FE94F82B;
        self.h[3] = 0xA54FF53A5F1D36F1;
        self.h[4] = 0x510E527FADE682D1;
        self.h[5] = 0x9B05688C2B3E6C1F;
        self.h[6] = 0x1F83D9ABFB41BD6B;
        self.h[7] = 0x5BE0CD19137E2179;
        self.t = [0; 2];
        self.buflen = 0;
        self.nullt = 0;
        self.s = [0; 4];
    }

    pub fn compress(&mut self, block: &[u8]) {
        let mut m = [0u64; 16];
        for i in 0..16 {
            m[i] = u8to64(&block[i * 8..]);
        }

        let mut v = [0u64; 16];
        for i in 0..8 { v[i] = self.h[i]; }
        v[8] = self.s[0] ^ 0x243F6A8885A308D3;
        v[9] = self.s[1] ^ 0x13198A2E03707344;
        v[10] = self.s[2] ^ 0xA4093822299F31D0;
        v[11] = self.s[3] ^ 0x082EFA98EC4E6C89;
        v[12] = 0x452821E638D01377;
        v[13] = 0xBE5466CF34E90C6C;
        v[14] = 0xC0AC29B7C97C50DD;
        v[15] = 0x3F84D5B5B5470917;

        if self.nullt == 0 {
            v[12] ^= self.t[0];
            v[13] ^= self.t[0];
            v[14] ^= self.t[1];
            v[15] ^= self.t[1];
        }

        blake512_rounds(&mut v, &m);

        for i in 0..8 { v[i] ^= v[i + 8]; }
        v[0] ^= self.s[0]; v[1] ^= self.s[1]; v[2] ^= self.s[2]; v[3] ^= self.s[3];
        v[4] ^= self.s[0]; v[5] ^= self.s[1]; v[6] ^= self.s[2]; v[7] ^= self.s[3];
        for i in 0..8 { self.h[i] ^= v[i]; }
    }

    pub fn update(&mut self, data: &[u8], mut datalen: u64) {
        let mut data = data;
        let mut left = (self.buflen >> 3) as usize;
        let fill = 128 - left;

        if left != 0 && ((datalen >> 3) & 0x7F) as usize >= fill {
            self.buf[left..left + fill].copy_from_slice(&data[..fill]);
            self.t[0] = self.t[0].wrapping_add(1024);
            let buf_copy = self.buf;
            self.compress(&buf_copy);
            data = &data[fill..];
            datalen -= (fill as u64) << 3;
            left = 0;
        }

        while datalen >= 1024 {
            self.t[0] = self.t[0].wrapping_add(1024);
            self.compress(data);
            data = &data[128..];
            datalen -= 1024;
        }

        if datalen > 0 {
            let bytes = ((datalen >> 3) & 0x7F) as usize;
            self.buf[left..left + bytes].copy_from_slice(&data[..bytes]);
            self.buflen = ((left << 3) as u64 + datalen) as i32;
        } else {
            self.buflen = 0;
        }
    }

    pub fn finalize(&mut self, digest: &mut [u8]) {
        let zo: u8 = 0x01;
        let oo: u8 = 0x81;
        let mut msglen = [0u8; 16];
        let lo = self.t[0].wrapping_add(self.buflen as u64);
        let mut hi = self.t[1];
        if lo < self.buflen as u64 {
            hi = hi.wrapping_add(1);
        }
        u64to8(&mut msglen[0..8], hi);
        u64to8(&mut msglen[8..16], lo);

        if self.buflen == 888 {
            self.t[0] = self.t[0].wrapping_sub(8);
            self.update(&[oo], 8);
        } else {
            if self.buflen < 888 {
                if self.buflen == 0 {
                    self.nullt = 1;
                }
                self.t[0] = self.t[0].wrapping_sub((888 - self.buflen) as u64);
                self.update(&PADDING[..(888 - self.buflen) as usize / 8], (888 - self.buflen) as u64);
            } else {
                self.t[0] = self.t[0].wrapping_sub((1024 - self.buflen) as u64);
                self.update(&PADDING[..(1024 - self.buflen) as usize / 8], (1024 - self.buflen) as u64);
                self.t[0] = self.t[0].wrapping_sub(888);
                self.update(&PADDING[1..1 + 888 / 8], 888);
                self.nullt = 1;
            }
            self.update(&[zo], 8);
            self.t[0] = self.t[0].wrapping_sub(8);
        }
        self.t[0] = self.t[0].wrapping_sub(128);
        self.update(&msglen, 128);

        for i in 0..8 {
            u64to8(&mut digest[i * 8..], self.h[i]);
        }
    }
}

fn blake512_rounds(v: &mut [u64; 16], m: &[u64; 16]) {
    const SIGMA: [[usize; 16]; 16] = [
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
        [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
        [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
        [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
        [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
        [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
        [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
        [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
        [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
        [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
        [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
        [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
        [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    ];

    for round in 0..16 {
        let s = &SIGMA[round];

        // Column step
        v[0] = v[0].wrapping_add(m[s[0]] ^ CST[s[1]]).wrapping_add(v[4]);
        v[12] ^= v[0]; v[12] = rot64(v[12], 32);
        v[8] = v[8].wrapping_add(v[12]);
        v[4] ^= v[8]; v[4] = rot64(v[4], 25);
        v[0] = v[0].wrapping_add(m[s[1]] ^ CST[s[0]]).wrapping_add(v[4]);
        v[12] ^= v[0]; v[12] = rot64(v[12], 16);
        v[8] = v[8].wrapping_add(v[12]);
        v[4] ^= v[8]; v[4] = rot64(v[4], 11);

        v[1] = v[1].wrapping_add(m[s[2]] ^ CST[s[3]]).wrapping_add(v[5]);
        v[13] ^= v[1]; v[13] = rot64(v[13], 32);
        v[9] = v[9].wrapping_add(v[13]);
        v[5] ^= v[9]; v[5] = rot64(v[5], 25);
        v[1] = v[1].wrapping_add(m[s[3]] ^ CST[s[2]]).wrapping_add(v[5]);
        v[13] ^= v[1]; v[13] = rot64(v[13], 16);
        v[9] = v[9].wrapping_add(v[13]);
        v[5] ^= v[9]; v[5] = rot64(v[5], 11);

        v[2] = v[2].wrapping_add(m[s[4]] ^ CST[s[5]]).wrapping_add(v[6]);
        v[14] ^= v[2]; v[14] = rot64(v[14], 32);
        v[10] = v[10].wrapping_add(v[14]);
        v[6] ^= v[10]; v[6] = rot64(v[6], 25);
        v[2] = v[2].wrapping_add(m[s[5]] ^ CST[s[4]]).wrapping_add(v[6]);
        v[14] ^= v[2]; v[14] = rot64(v[14], 16);
        v[10] = v[10].wrapping_add(v[14]);
        v[6] ^= v[10]; v[6] = rot64(v[6], 11);

        v[3] = v[3].wrapping_add(m[s[6]] ^ CST[s[7]]).wrapping_add(v[7]);
        v[15] ^= v[3]; v[15] = rot64(v[15], 32);
        v[11] = v[11].wrapping_add(v[15]);
        v[7] ^= v[11]; v[7] = rot64(v[7], 25);
        v[3] = v[3].wrapping_add(m[s[7]] ^ CST[s[6]]).wrapping_add(v[7]);
        v[15] ^= v[3]; v[15] = rot64(v[15], 16);
        v[11] = v[11].wrapping_add(v[15]);
        v[7] ^= v[11]; v[7] = rot64(v[7], 11);

        // Diagonal step
        v[0] = v[0].wrapping_add(m[s[8]] ^ CST[s[9]]).wrapping_add(v[5]);
        v[15] ^= v[0]; v[15] = rot64(v[15], 32);
        v[10] = v[10].wrapping_add(v[15]);
        v[5] ^= v[10]; v[5] = rot64(v[5], 25);
        v[0] = v[0].wrapping_add(m[s[9]] ^ CST[s[8]]).wrapping_add(v[5]);
        v[15] ^= v[0]; v[15] = rot64(v[15], 16);
        v[10] = v[10].wrapping_add(v[15]);
        v[5] ^= v[10]; v[5] = rot64(v[5], 11);

        v[1] = v[1].wrapping_add(m[s[10]] ^ CST[s[11]]).wrapping_add(v[6]);
        v[12] ^= v[1]; v[12] = rot64(v[12], 32);
        v[11] = v[11].wrapping_add(v[12]);
        v[6] ^= v[11]; v[6] = rot64(v[6], 25);
        v[1] = v[1].wrapping_add(m[s[11]] ^ CST[s[10]]).wrapping_add(v[6]);
        v[12] ^= v[1]; v[12] = rot64(v[12], 16);
        v[11] = v[11].wrapping_add(v[12]);
        v[6] ^= v[11]; v[6] = rot64(v[6], 11);

        v[2] = v[2].wrapping_add(m[s[12]] ^ CST[s[13]]).wrapping_add(v[7]);
        v[13] ^= v[2]; v[13] = rot64(v[13], 32);
        v[8] = v[8].wrapping_add(v[13]);
        v[7] ^= v[8]; v[7] = rot64(v[7], 25);
        v[2] = v[2].wrapping_add(m[s[13]] ^ CST[s[12]]).wrapping_add(v[7]);
        v[13] ^= v[2]; v[13] = rot64(v[13], 16);
        v[8] = v[8].wrapping_add(v[13]);
        v[7] ^= v[8]; v[7] = rot64(v[7], 11);

        v[3] = v[3].wrapping_add(m[s[14]] ^ CST[s[15]]).wrapping_add(v[4]);
        v[14] ^= v[3]; v[14] = rot64(v[14], 32);
        v[9] = v[9].wrapping_add(v[14]);
        v[4] ^= v[9]; v[4] = rot64(v[4], 25);
        v[3] = v[3].wrapping_add(m[s[15]] ^ CST[s[14]]).wrapping_add(v[4]);
        v[14] ^= v[3]; v[14] = rot64(v[14], 16);
        v[9] = v[9].wrapping_add(v[14]);
        v[4] ^= v[9]; v[4] = rot64(v[4], 11);
    }
}

pub fn blake512(out: &mut [u8], data: &[u8], inlen: u64) -> i32 {
    let mut s = Blake512State::new();
    s.update(data, inlen.wrapping_mul(8));
    s.finalize(out);
    0
}

pub fn blake512_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut i: usize = 0;
    let mut out_off = 0usize;

    while (i + 1) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake512(&mut out[out_off..], &inbuf, (inlen + 4) as u64);
        out_off += SPX_BLAKE512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_BLAKE512_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake512(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let remaining = outlen - i * SPX_BLAKE512_OUTPUT_BYTES;
        out[out_off..out_off + remaining].copy_from_slice(&outbuf[..remaining]);
    }
}
