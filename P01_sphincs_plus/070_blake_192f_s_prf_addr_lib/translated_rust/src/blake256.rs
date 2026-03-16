use crate::address::u32_to_bytes;
use crate::params::SPX_BLAKE256_OUTPUT_BYTES;

const CST: [u32; 16] = [
    0x243F6A88, 0x85A308D3, 0x13198A2E, 0x03707344,
    0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    0x452821E6, 0x38D01377, 0xBE5466CF, 0x34E90C6C,
    0xC0AC29B7, 0xC97C50DD, 0x3F84D5B5, 0xB5470917,
];

static PADDING: [u8; 64] = [
    0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

fn u8to32(p: &[u8]) -> u32 {
    (p[0] as u32) << 24 | (p[1] as u32) << 16 | (p[2] as u32) << 8 | (p[3] as u32)
}

fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8;
    p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8;
    p[3] = v as u8;
}

fn rot32(x: u32, n: u32) -> u32 {
    (x << (32 - n)) | (x >> n)
}

pub struct Blake256State {
    pub h: [u32; 8],
    pub s: [u32; 4],
    pub t: [u32; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 64],
}

impl Blake256State {
    pub fn new() -> Self {
        let mut s = Blake256State {
            h: [0; 8],
            s: [0; 4],
            t: [0; 2],
            buflen: 0,
            nullt: 0,
            buf: [0; 64],
        };
        s.init();
        s
    }

    pub fn init(&mut self) {
        self.h[0] = 0x6A09E667;
        self.h[1] = 0xBB67AE85;
        self.h[2] = 0x3C6EF372;
        self.h[3] = 0xA54FF53A;
        self.h[4] = 0x510E527F;
        self.h[5] = 0x9B05688C;
        self.h[6] = 0x1F83D9AB;
        self.h[7] = 0x5BE0CD19;
        self.t = [0; 2];
        self.buflen = 0;
        self.nullt = 0;
        self.s = [0; 4];
    }

    pub fn compress(&mut self, block: &[u8]) {
        let mut m = [0u32; 16];
        for i in 0..16 {
            m[i] = u8to32(&block[i * 4..]);
        }

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

        // 14 rounds - using the BLAKE-256 sigma permutations
        // The C code uses a ROUND macro that does column+diagonal steps
        // We reproduce the exact same round structure
        blake256_rounds(&mut v, &m);

        for i in 0..8 {
            v[i] ^= v[i + 8];
        }
        v[0] ^= self.s[0]; v[1] ^= self.s[1]; v[2] ^= self.s[2]; v[3] ^= self.s[3];
        v[4] ^= self.s[0]; v[5] ^= self.s[1]; v[6] ^= self.s[2]; v[7] ^= self.s[3];

        for i in 0..8 {
            self.h[i] ^= v[i];
        }
    }

    pub fn update(&mut self, data: &[u8], mut datalen: u64) {
        let mut data = data;
        let mut left = (self.buflen >> 3) as usize;
        let fill = 64 - left;

        if left != 0 && ((datalen >> 3) & 0x3F) as usize >= fill {
            self.buf[left..left + fill].copy_from_slice(&data[..fill]);
            self.t[0] = self.t[0].wrapping_add(512);
            if self.t[0] == 0 {
                self.t[1] = self.t[1].wrapping_add(1);
            }
            let buf_copy = self.buf;
            self.compress(&buf_copy);
            data = &data[fill..];
            datalen -= (fill as u64) << 3;
            left = 0;
        }

        while datalen >= 512 {
            self.t[0] = self.t[0].wrapping_add(512);
            if self.t[0] == 0 {
                self.t[1] = self.t[1].wrapping_add(1);
            }
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
        let mut msglen = [0u8; 8];
        let lo = self.t[0].wrapping_add(self.buflen as u32);
        let mut hi = self.t[1];
        if lo < self.buflen as u32 {
            hi = hi.wrapping_add(1);
        }
        u32to8(&mut msglen[0..4], hi);
        u32to8(&mut msglen[4..8], lo);

        if self.buflen == 440 {
            self.t[0] = self.t[0].wrapping_sub(8);
            self.update(&[oo], 8);
        } else {
            if self.buflen < 440 {
                if self.buflen == 0 {
                    self.nullt = 1;
                }
                self.t[0] = self.t[0].wrapping_sub((440 - self.buflen) as u32);
                self.update(&PADDING[..(440 - self.buflen) as usize / 8], (440 - self.buflen) as u64);
            } else {
                self.t[0] = self.t[0].wrapping_sub((512 - self.buflen) as u32);
                self.update(&PADDING[..(512 - self.buflen) as usize / 8], (512 - self.buflen) as u64);
                self.t[0] = self.t[0].wrapping_sub(440);
                self.update(&PADDING[1..1 + 440 / 8], 440);
                self.nullt = 1;
            }
            self.update(&[zo], 8);
            self.t[0] = self.t[0].wrapping_sub(8);
        }
        self.t[0] = self.t[0].wrapping_sub(64);
        self.update(&msglen, 64);

        for i in 0..8 {
            u32to8(&mut digest[i * 4..], self.h[i]);
        }
    }
}

// The 14 rounds of BLAKE-256 compression, matching the C ROUND macro calls exactly
fn blake256_rounds(v: &mut [u32; 16], m: &[u32; 16]) {
    // The sigma permutations for BLAKE-256 (10 permutations, repeated for rounds 11-14)
    const SIGMA: [[usize; 16]; 14] = [
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
    ];

    // The C code's ROUND macro uses pairs: (m[sigma[2i]], cst[sigma[2i+1]]) and (m[sigma[2i+1]], cst[sigma[2i]])
    // Actually looking at the C ROUND macro more carefully:
    // ROUND(m0,c0,m1,c1,...) where the args are actual message words and constants
    // The C code passes: ROUND(m0,cst[1],m1,cst[0],...) for round 0
    // This means: first G uses m[sigma[0]] ^ cst[sigma[1]], then m[sigma[1]] ^ cst[sigma[0]]
    // Let me just implement each round directly matching the C

    for round in 0..14 {
        let s = &SIGMA[round];

        // Column step: G(0,4,8,12), G(1,5,9,13), G(2,6,10,14), G(3,7,11,15)
        // G(a,b,c,d) with (m[s[2i]], cst[s[2i+1]]) then (m[s[2i+1]], cst[s[2i]])
        // Column 0: uses s[0],s[1]
        v[0] = v[0].wrapping_add(m[s[0]] ^ CST[s[1]]).wrapping_add(v[4]);
        v[12] ^= v[0]; v[12] = rot32(v[12], 16);
        v[8] = v[8].wrapping_add(v[12]);
        v[4] ^= v[8]; v[4] = rot32(v[4], 12);
        v[0] = v[0].wrapping_add(m[s[1]] ^ CST[s[0]]).wrapping_add(v[4]);
        v[12] ^= v[0]; v[12] = rot32(v[12], 8);
        v[8] = v[8].wrapping_add(v[12]);
        v[4] ^= v[8]; v[4] = rot32(v[4], 7);

        // Column 1: uses s[2],s[3]
        v[1] = v[1].wrapping_add(m[s[2]] ^ CST[s[3]]).wrapping_add(v[5]);
        v[13] ^= v[1]; v[13] = rot32(v[13], 16);
        v[9] = v[9].wrapping_add(v[13]);
        v[5] ^= v[9]; v[5] = rot32(v[5], 12);
        v[1] = v[1].wrapping_add(m[s[3]] ^ CST[s[2]]).wrapping_add(v[5]);
        v[13] ^= v[1]; v[13] = rot32(v[13], 8);
        v[9] = v[9].wrapping_add(v[13]);
        v[5] ^= v[9]; v[5] = rot32(v[5], 7);

        // Column 2: uses s[4],s[5]
        v[2] = v[2].wrapping_add(m[s[4]] ^ CST[s[5]]).wrapping_add(v[6]);
        v[14] ^= v[2]; v[14] = rot32(v[14], 16);
        v[10] = v[10].wrapping_add(v[14]);
        v[6] ^= v[10]; v[6] = rot32(v[6], 12);
        v[2] = v[2].wrapping_add(m[s[5]] ^ CST[s[4]]).wrapping_add(v[6]);
        v[14] ^= v[2]; v[14] = rot32(v[14], 8);
        v[10] = v[10].wrapping_add(v[14]);
        v[6] ^= v[10]; v[6] = rot32(v[6], 7);

        // Column 3: uses s[6],s[7]
        v[3] = v[3].wrapping_add(m[s[6]] ^ CST[s[7]]).wrapping_add(v[7]);
        v[15] ^= v[3]; v[15] = rot32(v[15], 16);
        v[11] = v[11].wrapping_add(v[15]);
        v[7] ^= v[11]; v[7] = rot32(v[7], 12);
        v[3] = v[3].wrapping_add(m[s[7]] ^ CST[s[6]]).wrapping_add(v[7]);
        v[15] ^= v[3]; v[15] = rot32(v[15], 8);
        v[11] = v[11].wrapping_add(v[15]);
        v[7] ^= v[11]; v[7] = rot32(v[7], 7);

        // Diagonal step: G(0,5,10,15), G(1,6,11,12), G(2,7,8,13), G(3,4,9,14)
        // Diagonal 0: uses s[8],s[9]
        v[0] = v[0].wrapping_add(m[s[8]] ^ CST[s[9]]).wrapping_add(v[5]);
        v[15] ^= v[0]; v[15] = rot32(v[15], 16);
        v[10] = v[10].wrapping_add(v[15]);
        v[5] ^= v[10]; v[5] = rot32(v[5], 12);
        v[0] = v[0].wrapping_add(m[s[9]] ^ CST[s[8]]).wrapping_add(v[5]);
        v[15] ^= v[0]; v[15] = rot32(v[15], 8);
        v[10] = v[10].wrapping_add(v[15]);
        v[5] ^= v[10]; v[5] = rot32(v[5], 7);

        // Diagonal 1: uses s[10],s[11]
        v[1] = v[1].wrapping_add(m[s[10]] ^ CST[s[11]]).wrapping_add(v[6]);
        v[12] ^= v[1]; v[12] = rot32(v[12], 16);
        v[11] = v[11].wrapping_add(v[12]);
        v[6] ^= v[11]; v[6] = rot32(v[6], 12);
        v[1] = v[1].wrapping_add(m[s[11]] ^ CST[s[10]]).wrapping_add(v[6]);
        v[12] ^= v[1]; v[12] = rot32(v[12], 8);
        v[11] = v[11].wrapping_add(v[12]);
        v[6] ^= v[11]; v[6] = rot32(v[6], 7);

        // Diagonal 2: uses s[12],s[13]
        v[2] = v[2].wrapping_add(m[s[12]] ^ CST[s[13]]).wrapping_add(v[7]);
        v[13] ^= v[2]; v[13] = rot32(v[13], 16);
        v[8] = v[8].wrapping_add(v[13]);
        v[7] ^= v[8]; v[7] = rot32(v[7], 12);
        v[2] = v[2].wrapping_add(m[s[13]] ^ CST[s[12]]).wrapping_add(v[7]);
        v[13] ^= v[2]; v[13] = rot32(v[13], 8);
        v[8] = v[8].wrapping_add(v[13]);
        v[7] ^= v[8]; v[7] = rot32(v[7], 7);

        // Diagonal 3: uses s[14],s[15]
        v[3] = v[3].wrapping_add(m[s[14]] ^ CST[s[15]]).wrapping_add(v[4]);
        v[14] ^= v[3]; v[14] = rot32(v[14], 16);
        v[9] = v[9].wrapping_add(v[14]);
        v[4] ^= v[9]; v[4] = rot32(v[4], 12);
        v[3] = v[3].wrapping_add(m[s[15]] ^ CST[s[14]]).wrapping_add(v[4]);
        v[14] ^= v[3]; v[14] = rot32(v[14], 8);
        v[9] = v[9].wrapping_add(v[14]);
        v[4] ^= v[9]; v[4] = rot32(v[4], 7);
    }
}

pub fn blake256(out: &mut [u8], data: &[u8], inlen: u64) -> i32 {
    let mut s = Blake256State::new();
    s.update(data, inlen.wrapping_mul(8));
    s.finalize(out);
    0
}

pub fn blake256_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut i: usize = 0;
    let mut out_off = 0usize;

    while (i + 1) * SPX_BLAKE256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake256(&mut out[out_off..], &inbuf, (inlen + 4) as u64);
        out_off += SPX_BLAKE256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_BLAKE256_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake256(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let remaining = outlen - i * SPX_BLAKE256_OUTPUT_BYTES;
        out[out_off..out_off + remaining].copy_from_slice(&outbuf[..remaining]);
    }
}
