use crate::address::u32_to_bytes;
use crate::params::SPX_BLAKE256_OUTPUT_BYTES;

const CST: [u32; 16] = [
    0x243F6A88, 0x85A308D3, 0x13198A2E, 0x03707344,
    0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    0x452821E6, 0x38D01377, 0xBE5466CF, 0x34E90C6C,
    0xC0AC29B7, 0xC97C50DD, 0x3F84D5B5, 0xB5470917,
];

static PADDING: [u8; 64] = {
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
pub struct Blake256State {
    h: [u32; 8],
    s: [u32; 4],
    t: [u32; 2],
    buflen: i32,
    nullt: i32,
    buf: [u8; 64],
}

impl Blake256State {
    pub fn new() -> Self {
        let mut s = Blake256State {
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

    pub fn compress(&mut self, block: &[u8]) {
        let m: [u32; 16] = [
            u8to32(&block[0..]), u8to32(&block[4..]), u8to32(&block[8..]), u8to32(&block[12..]),
            u8to32(&block[16..]), u8to32(&block[20..]), u8to32(&block[24..]), u8to32(&block[28..]),
            u8to32(&block[32..]), u8to32(&block[36..]), u8to32(&block[40..]), u8to32(&block[44..]),
            u8to32(&block[48..]), u8to32(&block[52..]), u8to32(&block[56..]), u8to32(&block[60..]),
        ];

        let mut v: [u32; 16] = [
            self.h[0], self.h[1], self.h[2], self.h[3],
            self.h[4], self.h[5], self.h[6], self.h[7],
            self.s[0] ^ 0x243F6A88, self.s[1] ^ 0x85A308D3,
            self.s[2] ^ 0x13198A2E, self.s[3] ^ 0x03707344,
            0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
        ];

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
            // rounds 10-13 repeat 0-3
            [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15],
            [14,10,4,8,9,15,13,6,1,12,0,2,11,7,5,3],
            [11,8,12,0,5,2,15,13,10,14,3,6,7,1,9,4],
            [7,9,3,1,13,12,11,14,2,6,5,10,4,0,15,8],
        ];

        macro_rules! g256 {
            ($a:expr, $b:expr, $c:expr, $d:expr, $mi:expr, $ci:expr, $mj:expr, $cj:expr) => {
                v[$a] = v[$a].wrapping_add(m[$mi] ^ CST[$ci]);
                v[$a] = v[$a].wrapping_add(v[$b]);
                v[$d] ^= v[$a];
                v[$d] = v[$d].rotate_right(16);
                v[$c] = v[$c].wrapping_add(v[$d]);
                v[$b] ^= v[$c];
                v[$b] = v[$b].rotate_right(12);
                v[$a] = v[$a].wrapping_add(m[$mj] ^ CST[$cj]);
                v[$a] = v[$a].wrapping_add(v[$b]);
                v[$d] ^= v[$a];
                v[$d] = v[$d].rotate_right(8);
                v[$c] = v[$c].wrapping_add(v[$d]);
                v[$b] ^= v[$c];
                v[$b] = v[$b].rotate_right(7);
            };
        }

        for round in 0..14 {
            let s = &SIGMA[round];
            g256!(0, 4, 8, 12, s[0], s[1], s[1], s[0]);
            g256!(1, 5, 9, 13, s[2], s[3], s[3], s[2]);
            g256!(2, 6, 10, 14, s[4], s[5], s[5], s[4]);
            g256!(3, 7, 11, 15, s[6], s[7], s[7], s[6]);
            g256!(0, 5, 10, 15, s[8], s[9], s[9], s[8]);
            g256!(1, 6, 11, 12, s[10], s[11], s[11], s[10]);
            g256!(2, 7, 8, 13, s[12], s[13], s[13], s[12]);
            g256!(3, 4, 9, 14, s[14], s[15], s[15], s[14]);
        }

        for i in 0..8 {
            v[i] ^= v[i + 8];
        }
        v[0] ^= self.s[0]; v[1] ^= self.s[1];
        v[2] ^= self.s[2]; v[3] ^= self.s[3];
        v[4] ^= self.s[0]; v[5] ^= self.s[1];
        v[6] ^= self.s[2]; v[7] ^= self.s[3];

        for i in 0..8 {
            self.h[i] ^= v[i];
        }
    }

    // datalen is in BITS
    pub fn update(&mut self, data: &[u8], mut datalen: u64) {
        let mut data = data;
        let mut left = (self.buflen >> 3) as usize;
        let fill = 64 - left;

        if left != 0 && ((datalen >> 3) & 0x3F) >= fill as u64 {
            self.buf[left..left + fill].copy_from_slice(&data[..fill]);
            self.t[0] = self.t[0].wrapping_add(512);
            if self.t[0] == 0 { self.t[1] = self.t[1].wrapping_add(1); }
            self.compress(&self.buf.clone());
            data = &data[fill..];
            datalen -= (fill as u64) << 3;
            left = 0;
        }

        while datalen >= 512 {
            self.t[0] = self.t[0].wrapping_add(512);
            if self.t[0] == 0 { self.t[1] = self.t[1].wrapping_add(1); }
            self.compress(&data[..64]);
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
        u32to8(&mut msglen[0..4], hi);
        u32to8(&mut msglen[4..8], lo);

        if self.buflen == 440 {
            self.t[0] = self.t[0].wrapping_sub(8);
            self.update(&[oo], 8);
        } else {
            if self.buflen < 440 {
                if self.buflen == 0 { self.nullt = 1; }
                self.t[0] = self.t[0].wrapping_sub((440 - self.buflen) as u32);
                let pad_len = (440 - self.buflen) as usize;
                let pad: Vec<u8> = PADDING[..pad_len / 8 + if pad_len % 8 != 0 { 1 } else { 0 }].to_vec();
                self.update(&pad, (440 - self.buflen) as u64);
            } else {
                self.t[0] = self.t[0].wrapping_sub((512 - self.buflen) as u32);
                let pad_len = (512 - self.buflen) as u64;
                let needed = (pad_len / 8) as usize + if pad_len % 8 != 0 { 1 } else { 0 };
                let pad: Vec<u8> = PADDING[..needed].to_vec();
                self.update(&pad, pad_len);
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

pub fn blake256(out: &mut [u8], input: &[u8], inlen: u64) -> i32 {
    let mut s = Blake256State::new();
    s.update(input, inlen * 8);
    s.finalize(out);
    0
}

pub fn blake256_mgf1(out: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&input[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut i: u64 = 0;

    while (i + 1) * SPX_BLAKE256_OUTPUT_BYTES as u64 <= outlen as u64 {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        let off = (i as usize) * SPX_BLAKE256_OUTPUT_BYTES;
        blake256(&mut out[off..], &inbuf, (inlen + 4) as u64);
        i += 1;
    }
    if outlen > (i as usize) * SPX_BLAKE256_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake256(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let remaining = outlen - (i as usize) * SPX_BLAKE256_OUTPUT_BYTES;
        out[(i as usize) * SPX_BLAKE256_OUTPUT_BYTES..(i as usize) * SPX_BLAKE256_OUTPUT_BYTES + remaining]
            .copy_from_slice(&outbuf[..remaining]);
    }
}
