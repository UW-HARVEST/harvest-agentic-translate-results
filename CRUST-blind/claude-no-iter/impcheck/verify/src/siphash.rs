pub const C_ROUNDS: u32 = 2;
pub const D_ROUNDS: u32 = 4;

const OUTLEN: usize = 16;

fn rotl(x: u64, b: u32) -> u64 {
    (x << b) | (x >> (64 - b))
}

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

pub struct SipHash {
    kk: Vec<u8>,
    out: Vec<u8>,
    buf: Vec<u8>,
    // Internal state
    v0: u64,
    v1: u64,
    v2: u64,
    v3: u64,
    inlen: u64,
    buflen: u8,
}

impl SipHash {
    pub fn siphash_update(&mut self, data: &[u8], nb_bytes: u64) {
        let n = nb_bytes as usize;
        let mut datapos: usize = 0;
        loop {
            while self.buflen < 8 && datapos < n {
                let bl = self.buflen as usize;
                self.buf[bl] = data[datapos];
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

    pub fn process_next_block(&mut self) {
        let m = u8to64_le(&self.buf);
        self.v3 ^= m;
        for _ in 0..C_ROUNDS {
            let mut v0 = self.v0;
            let mut v1 = self.v1;
            let mut v2 = self.v2;
            let mut v3 = self.v3;
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
            self.v0 = v0;
            self.v1 = v1;
            self.v2 = v2;
            self.v3 = v3;
        }
        self.v0 ^= m;
    }

    pub fn process_final_block(&mut self) {
        let left = (self.inlen & 7) as usize;
        debug_assert!(left == self.buflen as usize);
        let mut b: u64 = (self.inlen as u64) << 56;
        let ni = &self.buf;
        if left >= 7 {
            b |= (ni[6] as u64) << 48;
        }
        if left >= 6 {
            b |= (ni[5] as u64) << 40;
        }
        if left >= 5 {
            b |= (ni[4] as u64) << 32;
        }
        if left >= 4 {
            b |= (ni[3] as u64) << 24;
        }
        if left >= 3 {
            b |= (ni[2] as u64) << 16;
        }
        if left >= 2 {
            b |= (ni[1] as u64) << 8;
        }
        if left >= 1 {
            b |= ni[0] as u64;
        }

        self.v3 ^= b;
        for _ in 0..C_ROUNDS {
            let mut v0 = self.v0;
            let mut v1 = self.v1;
            let mut v2 = self.v2;
            let mut v3 = self.v3;
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
            self.v0 = v0;
            self.v1 = v1;
            self.v2 = v2;
            self.v3 = v3;
        }
        self.v0 ^= b;

        if OUTLEN == 16 {
            self.v2 ^= 0xee;
        } else {
            self.v2 ^= 0xff;
        }

        for _ in 0..D_ROUNDS {
            let mut v0 = self.v0;
            let mut v1 = self.v1;
            let mut v2 = self.v2;
            let mut v3 = self.v3;
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
            self.v0 = v0;
            self.v1 = v1;
            self.v2 = v2;
            self.v3 = v3;
        }

        let bb = self.v0 ^ self.v1 ^ self.v2 ^ self.v3;
        u64to8_le(&mut self.out[0..8], bb);

        self.v1 ^= 0xdd;
        for _ in 0..D_ROUNDS {
            let mut v0 = self.v0;
            let mut v1 = self.v1;
            let mut v2 = self.v2;
            let mut v3 = self.v3;
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
            self.v0 = v0;
            self.v1 = v1;
            self.v2 = v2;
            self.v3 = v3;
        }

        let bb = self.v0 ^ self.v1 ^ self.v2 ^ self.v3;
        u64to8_le(&mut self.out[8..16], bb);
    }

    pub fn siphash_pad(&mut self, nb_bytes: u64) {
        let c: [u8; 1] = [0];
        for _ in 0..nb_bytes {
            self.siphash_update(&c, 1);
        }
    }

    pub fn siphash_init(key_128bit: &[u8]) -> Self {
        let mut sh = SipHash {
            kk: key_128bit.to_vec(),
            out: vec![0u8; 16],
            buf: vec![0u8; 8],
            v0: 0,
            v1: 0,
            v2: 0,
            v3: 0,
            inlen: 0,
            buflen: 0,
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
        // Mutate a clone since the function takes &self.
        let mut tmp = SipHash {
            kk: self.kk.clone(),
            out: self.out.clone(),
            buf: self.buf.clone(),
            v0: self.v0,
            v1: self.v1,
            v2: self.v2,
            v3: self.v3,
            inlen: self.inlen,
            buflen: self.buflen,
        };
        tmp.process_final_block();
        tmp.out
    }

    pub fn siphash_free(&mut self) {
        self.buf.clear();
        self.out.clear();
    }
}
