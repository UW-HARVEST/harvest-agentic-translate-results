use crate::address::u32_to_bytes;

// ============ BLAKE-256 ============

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

#[derive(Clone)]
pub struct BlakeState256 {
    h: [u32; 8],
    s: [u32; 4],
    t: [u32; 2],
    buflen: i32,
    nullt: i32,
    buf: [u8; 64],
}

impl BlakeState256 {
    pub fn new() -> Self {
        let mut s = BlakeState256 {
            h: [0; 8], s: [0; 4], t: [0; 2],
            buflen: 0, nullt: 0, buf: [0; 64],
        };
        s.init();
        s
    }

    pub fn init(&mut self) {
        self.h = [
            0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
            0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
        ];
        self.t = [0; 2];
        self.buflen = 0;
        self.nullt = 0;
        self.s = [0; 4];
    }

    fn compress(&mut self, block: &[u8]) {
        let m: [u32; 16] = [
            u8to32(&block[0..]), u8to32(&block[4..]), u8to32(&block[8..]), u8to32(&block[12..]),
            u8to32(&block[16..]), u8to32(&block[20..]), u8to32(&block[24..]), u8to32(&block[28..]),
            u8to32(&block[32..]), u8to32(&block[36..]), u8to32(&block[40..]), u8to32(&block[44..]),
            u8to32(&block[48..]), u8to32(&block[52..]), u8to32(&block[56..]), u8to32(&block[60..]),
        ];

        let mut v = [0u32; 16];
        v[0] = self.h[0]; v[1] = self.h[1]; v[2] = self.h[2]; v[3] = self.h[3];
        v[4] = self.h[4]; v[5] = self.h[5]; v[6] = self.h[6]; v[7] = self.h[7];
        v[8] = self.s[0] ^ 0x243F6A88;
        v[9] = self.s[1] ^ 0x85A308D3;
        v[10] = self.s[2] ^ 0x13198A2E;
        v[11] = self.s[3] ^ 0x03707344;
        v[12] = 0xA4093822;
        v[13] = 0x299F31D0;
        v[14] = 0x082EFA98;
        v[15] = 0xEC4E6C89;

        if self.nullt == 0 {
            v[12] ^= self.t[0];
            v[13] ^= self.t[0];
            v[14] ^= self.t[1];
            v[15] ^= self.t[1];
        }

        // BLAKE-256 sigma permutations
        const SIGMA: [[usize; 16]; 14] = [
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
            // Repeat first 4
            [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15],
            [14,10,4,8,9,15,13,6,1,12,0,2,11,7,5,3],
            [11,8,12,0,5,2,15,13,10,14,3,6,7,1,9,4],
            [7,9,3,1,13,12,11,14,2,6,5,10,4,0,15,8],
        ];

        macro_rules! g256 {
            ($a:expr, $b:expr, $c:expr, $d:expr, $mi:expr, $ci:expr, $mj:expr, $cj:expr) => {
                v[$a] = v[$a].wrapping_add(m[$mi] ^ CST256[$ci]).wrapping_add(v[$b]);
                v[$d] ^= v[$a]; v[$d] = v[$d].rotate_right(16);
                v[$c] = v[$c].wrapping_add(v[$d]);
                v[$b] ^= v[$c]; v[$b] = v[$b].rotate_right(12);
                v[$a] = v[$a].wrapping_add(m[$mj] ^ CST256[$cj]).wrapping_add(v[$b]);
                v[$d] ^= v[$a]; v[$d] = v[$d].rotate_right(8);
                v[$c] = v[$c].wrapping_add(v[$d]);
                v[$b] ^= v[$c]; v[$b] = v[$b].rotate_right(7);
            };
        }

        for r in 0..14 {
            let s = &SIGMA[r];
            g256!(0, 4, 8, 12, s[0], s[1], s[1], s[0]);
            g256!(1, 5, 9, 13, s[2], s[3], s[3], s[2]);
            g256!(2, 6, 10, 14, s[4], s[5], s[5], s[4]);
            g256!(3, 7, 11, 15, s[6], s[7], s[7], s[6]);
            g256!(0, 5, 10, 15, s[8], s[9], s[9], s[8]);
            g256!(1, 6, 11, 12, s[10], s[11], s[11], s[10]);
            g256!(2, 7, 8, 13, s[12], s[13], s[13], s[12]);
            g256!(3, 4, 9, 14, s[14], s[15], s[15], s[14]);
        }

        for i in 0..8 { v[i] ^= v[i + 8]; }
        for i in 0..4 { v[i] ^= self.s[i]; v[i+4] ^= self.s[i]; }
        for i in 0..8 { self.h[i] ^= v[i]; }
    }

    pub fn update(&mut self, data: &[u8], datalen_bits: u64) {
        let mut data = data;
        let mut datalen = datalen_bits;
        let mut left = (self.buflen >> 3) as usize;
        let fill = 64 - left;

        if left != 0 && ((datalen >> 3) & 0x3F) >= fill as u64 {
            self.buf[left..left + fill].copy_from_slice(&data[..fill]);
            self.t[0] = self.t[0].wrapping_add(512);
            if self.t[0] == 0 { self.t[1] = self.t[1].wrapping_add(1); }
            let buf_copy = self.buf;
            self.compress(&buf_copy);
            data = &data[fill..];
            datalen -= (fill as u64) << 3;
            left = 0;
        }

        while datalen >= 512 {
            self.t[0] = self.t[0].wrapping_add(512);
            if self.t[0] == 0 { self.t[1] = self.t[1].wrapping_add(1); }
            self.compress(data);
            data = &data[64..];
            datalen -= 512;
        }

        if datalen > 0 {
            let bytes = (datalen >> 3) as usize;
            self.buf[left..left + bytes].copy_from_slice(&data[..bytes]);
            self.buflen = ((left << 3) as u64 + datalen) as i32;
        } else {
            self.buflen = 0;
        }
    }

    pub fn finalize(&mut self, digest: &mut [u8]) {
        let zo: u8 = 0x01;
        let oo: u8 = 0x81;
        let lo = self.t[0].wrapping_add(self.buflen as u32);
        let mut hi = self.t[1];
        if lo < self.buflen as u32 { hi = hi.wrapping_add(1); }
        let mut msglen = [0u8; 8];
        u32to8(&mut msglen[0..], hi);
        u32to8(&mut msglen[4..], lo);

        if self.buflen == 440 {
            self.t[0] = self.t[0].wrapping_sub(8);
            self.update(&[oo], 8);
        } else {
            if self.buflen < 440 {
                if self.buflen == 0 { self.nullt = 1; }
                self.t[0] = self.t[0].wrapping_sub((440 - self.buflen) as u32);
                self.update(&PADDING256[..], (440 - self.buflen) as u64);
            } else {
                self.t[0] = self.t[0].wrapping_sub((512 - self.buflen) as u32);
                self.update(&PADDING256[..], (512 - self.buflen) as u64);
                self.t[0] = self.t[0].wrapping_sub(440);
                self.update(&PADDING256[1..], 440);
                self.nullt = 1;
            }
            self.update(&[zo], 8);
            self.t[0] = self.t[0].wrapping_sub(8);
        }
        self.t[0] = self.t[0].wrapping_sub(64);
        self.update(&msglen, 64);

        for i in 0..8 {
            u32to8(&mut digest[4 * i..], self.h[i]);
        }
    }
}

pub fn blake256(out: &mut [u8], inp: &[u8], inlen: u64) {
    let mut s = BlakeState256::new();
    s.update(inp, inlen * 8);
    s.finalize(out);
}

pub fn blake256_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut i: u32 = 0;
    let mut off = 0;
    while (i as usize + 1) * SPX_BLAKE256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i);
        blake256(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += SPX_BLAKE256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i as usize * SPX_BLAKE256_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i);
        blake256(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - i as usize * SPX_BLAKE256_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

pub const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;
pub const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

// ============ BLAKE-512 ============

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

fn u8to64(p: &[u8]) -> u64 {
    ((u8to32(p) as u64) << 32) | (u8to32(&p[4..]) as u64)
}

fn u64to8(p: &mut [u8], v: u64) {
    u32to8(p, (v >> 32) as u32);
    u32to8(&mut p[4..], v as u32);
}

#[derive(Clone)]
pub struct BlakeState512 {
    h: [u64; 8],
    s: [u64; 4],
    t: [u64; 2],
    buflen: i32,
    nullt: i32,
    buf: [u8; 128],
}

impl BlakeState512 {
    pub fn new() -> Self {
        let mut s = BlakeState512 {
            h: [0; 8], s: [0; 4], t: [0; 2],
            buflen: 0, nullt: 0, buf: [0; 128],
        };
        s.init();
        s
    }

    pub fn init(&mut self) {
        self.h = [
            0x6A09E667F3BCC908, 0xBB67AE8584CAA73B,
            0x3C6EF372FE94F82B, 0xA54FF53A5F1D36F1,
            0x510E527FADE682D1, 0x9B05688C2B3E6C1F,
            0x1F83D9ABFB41BD6B, 0x5BE0CD19137E2179,
        ];
        self.t = [0; 2];
        self.buflen = 0;
        self.nullt = 0;
        self.s = [0; 4];
    }

    fn compress(&mut self, block: &[u8]) {
        let m: [u64; 16] = [
            u8to64(&block[0..]), u8to64(&block[8..]), u8to64(&block[16..]), u8to64(&block[24..]),
            u8to64(&block[32..]), u8to64(&block[40..]), u8to64(&block[48..]), u8to64(&block[56..]),
            u8to64(&block[64..]), u8to64(&block[72..]), u8to64(&block[80..]), u8to64(&block[88..]),
            u8to64(&block[96..]), u8to64(&block[104..]), u8to64(&block[112..]), u8to64(&block[120..]),
        ];

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
            // Repeat first 6
            [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15],
            [14,10,4,8,9,15,13,6,1,12,0,2,11,7,5,3],
            [11,8,12,0,5,2,15,13,10,14,3,6,7,1,9,4],
            [7,9,3,1,13,12,11,14,2,6,5,10,4,0,15,8],
            [9,0,5,7,2,4,10,15,14,1,11,12,6,8,3,13],
            [2,12,6,10,0,11,8,3,4,13,7,5,15,14,1,9],
        ];

        macro_rules! g512 {
            ($a:expr, $b:expr, $c:expr, $d:expr, $mi:expr, $ci:expr, $mj:expr, $cj:expr) => {
                v[$a] = v[$a].wrapping_add(m[$mi] ^ CST512[$ci]).wrapping_add(v[$b]);
                v[$d] ^= v[$a]; v[$d] = v[$d].rotate_right(32);
                v[$c] = v[$c].wrapping_add(v[$d]);
                v[$b] ^= v[$c]; v[$b] = v[$b].rotate_right(25);
                v[$a] = v[$a].wrapping_add(m[$mj] ^ CST512[$cj]).wrapping_add(v[$b]);
                v[$d] ^= v[$a]; v[$d] = v[$d].rotate_right(16);
                v[$c] = v[$c].wrapping_add(v[$d]);
                v[$b] ^= v[$c]; v[$b] = v[$b].rotate_right(11);
            };
        }

        for r in 0..16 {
            let s = &SIGMA[r];
            g512!(0, 4, 8, 12, s[0], s[1], s[1], s[0]);
            g512!(1, 5, 9, 13, s[2], s[3], s[3], s[2]);
            g512!(2, 6, 10, 14, s[4], s[5], s[5], s[4]);
            g512!(3, 7, 11, 15, s[6], s[7], s[7], s[6]);
            g512!(0, 5, 10, 15, s[8], s[9], s[9], s[8]);
            g512!(1, 6, 11, 12, s[10], s[11], s[11], s[10]);
            g512!(2, 7, 8, 13, s[12], s[13], s[13], s[12]);
            g512!(3, 4, 9, 14, s[14], s[15], s[15], s[14]);
        }

        for i in 0..8 { v[i] ^= v[i + 8]; }
        for i in 0..4 { v[i] ^= self.s[i]; v[i+4] ^= self.s[i]; }
        for i in 0..8 { self.h[i] ^= v[i]; }
    }

    pub fn update(&mut self, data: &[u8], datalen_bits: u64) {
        let mut data = data;
        let mut datalen = datalen_bits;
        let mut left = (self.buflen >> 3) as usize;
        let fill = 128 - left;

        if left != 0 && ((datalen >> 3) & 0x7F) >= fill as u64 {
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
        let lo = self.t[0].wrapping_add(self.buflen as u64);
        let mut hi = self.t[1];
        if lo < self.buflen as u64 { hi = hi.wrapping_add(1); }
        let mut msglen = [0u8; 16];
        u64to8(&mut msglen[0..], hi);
        u64to8(&mut msglen[8..], lo);

        if self.buflen == 888 {
            self.t[0] = self.t[0].wrapping_sub(8);
            self.update(&[oo], 8);
        } else {
            if self.buflen < 888 {
                if self.buflen == 0 { self.nullt = 1; }
                self.t[0] = self.t[0].wrapping_sub((888 - self.buflen) as u64);
                self.update(&PADDING512[..], (888 - self.buflen) as u64);
            } else {
                self.t[0] = self.t[0].wrapping_sub((1024 - self.buflen) as u64);
                self.update(&PADDING512[..], (1024 - self.buflen) as u64);
                self.t[0] = self.t[0].wrapping_sub(888);
                self.update(&PADDING512[1..], 888);
                self.nullt = 1;
            }
            self.update(&[zo], 8);
            self.t[0] = self.t[0].wrapping_sub(8);
        }
        self.t[0] = self.t[0].wrapping_sub(128);
        self.update(&msglen, 128);

        for i in 0..8 {
            u64to8(&mut digest[8 * i..], self.h[i]);
        }
    }
}

pub fn blake512(out: &mut [u8], inp: &[u8], inlen: u64) {
    let mut s = BlakeState512::new();
    s.update(inp, inlen * 8);
    s.finalize(out);
}

pub fn blake512_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut i: u32 = 0;
    let mut off = 0;
    while (i as usize + 1) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i);
        blake512(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += SPX_BLAKE512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i as usize * SPX_BLAKE512_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i);
        blake512(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - i as usize * SPX_BLAKE512_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}
