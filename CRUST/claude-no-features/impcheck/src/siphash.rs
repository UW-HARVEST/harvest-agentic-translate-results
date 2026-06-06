pub const C_ROUNDS: u32 = 2;
pub const D_ROUNDS: u32 = 4;
pub const OUTLEN: usize = 16;

pub struct SipHash {
    kk: Vec<u8>,
    out: Vec<u8>,
    buf: Vec<u8>,
    buflen: u8,
    inlen: u64,
    v0: u64,
    v1: u64,
    v2: u64,
    v3: u64,
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
    p[0] = (v & 0xff) as u8;
    p[1] = ((v >> 8) & 0xff) as u8;
    p[2] = ((v >> 16) & 0xff) as u8;
    p[3] = ((v >> 24) & 0xff) as u8;
    p[4] = ((v >> 32) & 0xff) as u8;
    p[5] = ((v >> 40) & 0xff) as u8;
    p[6] = ((v >> 48) & 0xff) as u8;
    p[7] = ((v >> 56) & 0xff) as u8;
}

impl SipHash {
    fn sipround(&mut self) {
        self.v0 = self.v0.wrapping_add(self.v1);
        self.v1 = rotl(self.v1, 13);
        self.v1 ^= self.v0;
        self.v0 = rotl(self.v0, 32);
        self.v2 = self.v2.wrapping_add(self.v3);
        self.v3 = rotl(self.v3, 16);
        self.v3 ^= self.v2;
        self.v0 = self.v0.wrapping_add(self.v3);
        self.v3 = rotl(self.v3, 21);
        self.v3 ^= self.v0;
        self.v2 = self.v2.wrapping_add(self.v1);
        self.v1 = rotl(self.v1, 17);
        self.v1 ^= self.v2;
        self.v2 = rotl(self.v2, 32);
    }

    pub fn siphash_update(&mut self, data: &[u8], nb_bytes: u64) {
        let mut datapos: usize = 0;
        let n = nb_bytes as usize;
        loop {
            while (self.buflen as usize) < 8 && datapos < n {
                self.buf[self.buflen as usize] = data[datapos];
                self.buflen += 1;
                datapos += 1;
            }
            if (self.buflen as usize) < 8 {
                break;
            }
            self.process_next_block();
            self.buflen = 0;
        }
        self.inlen = self.inlen.wrapping_add(nb_bytes);
    }

    pub fn process_next_block(&mut self) {
        let m = u8to64_le(&self.buf);
        self.v3 ^= m;
        for _ in 0..C_ROUNDS {
            self.sipround();
        }
        self.v0 ^= m;
    }

    pub fn process_final_block(&mut self) {
        let left = (self.inlen & 7) as usize;
        debug_assert!(left == self.buflen as usize);
        let mut b: u64 = (self.inlen as u64) << 56;
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
        self.v3 ^= b;
        for _ in 0..C_ROUNDS {
            self.sipround();
        }
        self.v0 ^= b;
        if OUTLEN == 16 {
            self.v2 ^= 0xee;
        } else {
            self.v2 ^= 0xff;
        }
        for _ in 0..D_ROUNDS {
            self.sipround();
        }
        let b1 = self.v0 ^ self.v1 ^ self.v2 ^ self.v3;
        u64to8_le(&mut self.out[0..8], b1);
        self.v1 ^= 0xdd;
        for _ in 0..D_ROUNDS {
            self.sipround();
        }
        let b2 = self.v0 ^ self.v1 ^ self.v2 ^ self.v3;
        u64to8_le(&mut self.out[8..16], b2);
    }

    pub fn siphash_pad(&mut self, nb_bytes: u64) {
        let zero = [0u8; 1];
        for _ in 0..nb_bytes {
            self.siphash_update(&zero, 1);
        }
    }

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
        let k0 = u8to64_le(&self.kk[0..8]);
        let k1 = u8to64_le(&self.kk[8..16]);
        self.v3 ^= k1;
        self.v2 ^= k0;
        self.v1 ^= k1;
        self.v0 ^= k0;
        self.inlen = 0;
        self.buflen = 0;
        if OUTLEN == 16 {
            self.v1 ^= 0xee;
        }
    }

    pub fn siphash_digest(&self) -> Vec<u8> {
        // Caller passes immutable reference; clone state to compute digest.
        let mut tmp = SipHash {
            kk: self.kk.clone(),
            out: self.out.clone(),
            buf: self.buf.clone(),
            buflen: self.buflen,
            inlen: self.inlen,
            v0: self.v0,
            v1: self.v1,
            v2: self.v2,
            v3: self.v3,
        };
        tmp.process_final_block();
        tmp.out
    }

    pub fn siphash_free(&mut self) {
        self.buf.clear();
        self.out.clear();
        self.kk.clear();
    }
}
