pub const C_ROUNDS: u32 = 2;
pub const D_ROUNDS: u32 = 4;

const OUTLEN: usize = 128 / 8;

pub struct SipHash {
    pub kk: Vec<u8>,
    pub out: Vec<u8>,
    pub buf: Vec<u8>,
    pub buflen: u8,
    pub inlen: u64,
    pub v0: u64,
    pub v1: u64,
    pub v2: u64,
    pub v3: u64,
    pub k0: u64,
    pub k1: u64,
}

#[inline(always)]
fn rotl(x: u64, b: u32) -> u64 {
    (x << b) | (x >> (64 - b))
}

#[inline(always)]
fn u8to64_le(p: &[u8]) -> u64 {
    (p[0] as u64)
        | ((p[1] as u64) << 8)
        | ((p[2] as u64) << 16)
        | ((p[3] as u64) << 24)
        | ((p[4] as u64) << 32)
        | ((p[5] as u64) << 40)
        | ((p[6] as u64) << 48)
        | ((p[7] as u64) << 56)
}

#[inline(always)]
fn u64to8_le(p: &mut [u8], v: u64) {
    p[0] = v as u8;
    p[1] = (v >> 8) as u8;
    p[2] = (v >> 16) as u8;
    p[3] = (v >> 24) as u8;
    p[4] = (v >> 32) as u8;
    p[5] = (v >> 40) as u8;
    p[6] = (v >> 48) as u8;
    p[7] = (v >> 56) as u8;
}

#[inline(always)]
fn sipround(v0: &mut u64, v1: &mut u64, v2: &mut u64, v3: &mut u64) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = rotl(*v1, 13);
    *v1 ^= *v0;
    *v0 = rotl(*v0, 32);
    *v2 = v2.wrapping_add(*v3);
    *v3 = rotl(*v3, 16);
    *v3 ^= *v2;
    *v0 = v0.wrapping_add(*v3);
    *v3 = rotl(*v3, 21);
    *v3 ^= *v0;
    *v2 = v2.wrapping_add(*v1);
    *v1 = rotl(*v1, 17);
    *v1 ^= *v2;
    *v2 = rotl(*v2, 32);
}

impl SipHash {
    pub fn siphash_init(key_128bit: &[u8]) -> Self {
        let mut sh = SipHash {
            kk: key_128bit.to_vec(),
            out: vec![0u8; OUTLEN],
            buf: vec![0u8; 8],
            buflen: 0,
            inlen: 0,
            v0: 0,
            v1: 0,
            v2: 0,
            v3: 0,
            k0: 0,
            k1: 0,
        };
        if !sh.kk.is_empty() {
            sh.siphash_reset();
        }
        sh
    }

    pub fn siphash_reset(&mut self) {
        self.v0 = 0x736f6d6570736575u64;
        self.v1 = 0x646f72616e646f6du64;
        self.v2 = 0x6c7967656e657261u64;
        self.v3 = 0x7465646279746573u64;
        self.k0 = u8to64_le(&self.kk[0..8]);
        self.k1 = u8to64_le(&self.kk[8..16]);
        self.v3 ^= self.k1;
        self.v2 ^= self.k0;
        self.v1 ^= self.k1;
        self.v0 ^= self.k0;
        self.inlen = 0;
        self.buflen = 0;
        if OUTLEN == 16 {
            self.v1 ^= 0xee;
        }
    }

    pub fn process_next_block(&mut self) {
        let m = u8to64_le(&self.buf);
        self.v3 ^= m;
        let mut v0 = self.v0;
        let mut v1 = self.v1;
        let mut v2 = self.v2;
        let mut v3 = self.v3;
        for _ in 0..C_ROUNDS {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= m;
        self.v0 = v0;
        self.v1 = v1;
        self.v2 = v2;
        self.v3 = v3;
    }

    pub fn process_final_block(&mut self) {
        let left = (self.inlen & 7) as usize;
        debug_assert_eq!(left, self.buflen as usize);
        let mut b: u64 = (self.inlen) << 56;
        let ni = &self.buf;
        match left {
            7 => {
                b |= (ni[6] as u64) << 48;
                b |= (ni[5] as u64) << 40;
                b |= (ni[4] as u64) << 32;
                b |= (ni[3] as u64) << 24;
                b |= (ni[2] as u64) << 16;
                b |= (ni[1] as u64) << 8;
                b |= ni[0] as u64;
            }
            6 => {
                b |= (ni[5] as u64) << 40;
                b |= (ni[4] as u64) << 32;
                b |= (ni[3] as u64) << 24;
                b |= (ni[2] as u64) << 16;
                b |= (ni[1] as u64) << 8;
                b |= ni[0] as u64;
            }
            5 => {
                b |= (ni[4] as u64) << 32;
                b |= (ni[3] as u64) << 24;
                b |= (ni[2] as u64) << 16;
                b |= (ni[1] as u64) << 8;
                b |= ni[0] as u64;
            }
            4 => {
                b |= (ni[3] as u64) << 24;
                b |= (ni[2] as u64) << 16;
                b |= (ni[1] as u64) << 8;
                b |= ni[0] as u64;
            }
            3 => {
                b |= (ni[2] as u64) << 16;
                b |= (ni[1] as u64) << 8;
                b |= ni[0] as u64;
            }
            2 => {
                b |= (ni[1] as u64) << 8;
                b |= ni[0] as u64;
            }
            1 => {
                b |= ni[0] as u64;
            }
            _ => {}
        }
        let mut v0 = self.v0;
        let mut v1 = self.v1;
        let mut v2 = self.v2;
        let mut v3 = self.v3;
        v3 ^= b;
        for _ in 0..C_ROUNDS {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= b;
        if OUTLEN == 16 {
            v2 ^= 0xee;
        } else {
            v2 ^= 0xff;
        }
        for _ in 0..D_ROUNDS {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        let mut bb = v0 ^ v1 ^ v2 ^ v3;
        u64to8_le(&mut self.out[0..8], bb);
        v1 ^= 0xdd;
        for _ in 0..D_ROUNDS {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        bb = v0 ^ v1 ^ v2 ^ v3;
        u64to8_le(&mut self.out[8..16], bb);
        self.v0 = v0;
        self.v1 = v1;
        self.v2 = v2;
        self.v3 = v3;
    }

    pub fn siphash_update(&mut self, data: &[u8], nb_bytes: u64) {
        let mut datapos: u32 = 0;
        loop {
            while self.buflen < 8 && (datapos as u64) < nb_bytes {
                self.buf[self.buflen as usize] = data[datapos as usize];
                self.buflen += 1;
                datapos += 1;
            }
            if self.buflen < 8 {
                break;
            }
            self.process_next_block();
            self.buflen = 0;
        }
        self.inlen += nb_bytes;
    }

    pub fn siphash_pad(&mut self, nb_bytes: u64) {
        let c = [0u8; 1];
        for _ in 0..nb_bytes {
            self.siphash_update(&c, 1);
        }
    }

    pub fn siphash_digest(&self) -> Vec<u8> {
        // We need a mutable digest to call process_final_block. Use interior pattern:
        // make a temporary clone to compute the result.
        let mut copy = SipHash {
            kk: self.kk.clone(),
            out: self.out.clone(),
            buf: self.buf.clone(),
            buflen: self.buflen,
            inlen: self.inlen,
            v0: self.v0,
            v1: self.v1,
            v2: self.v2,
            v3: self.v3,
            k0: self.k0,
            k1: self.k1,
        };
        copy.process_final_block();
        copy.out
    }

    pub fn siphash_free(&mut self) {
        self.buf.clear();
        self.out.clear();
    }
}
