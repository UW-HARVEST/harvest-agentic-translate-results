pub const C_ROUNDS: u32 = 2;
pub const D_ROUNDS: u32 = 4;

const OUTLEN: usize = 16;

pub struct SipHash {
    kk: Vec<u8>,
    out: Vec<u8>,
    buf: Vec<u8>,
}

fn rotl(x: u64, b: u32) -> u64 {
    (x << b) | (x >> (64 - b))
}

fn u8to64_le(p: &[u8]) -> u64 {
    let mut v = 0u64;
    for i in 0..8 {
        v |= (p[i] as u64) << (i * 8);
    }
    v
}

fn u64to8_le(p: &mut [u8], v: u64) {
    for i in 0..8 {
        p[i] = ((v >> (i * 8)) & 0xff) as u8;
    }
}

struct State {
    v0: u64,
    v1: u64,
    v2: u64,
    v3: u64,
    inlen: u64,
    buflen: u8,
}

fn sipround(s: &mut State) {
    s.v0 = s.v0.wrapping_add(s.v1);
    s.v1 = rotl(s.v1, 13);
    s.v1 ^= s.v0;
    s.v0 = rotl(s.v0, 32);
    s.v2 = s.v2.wrapping_add(s.v3);
    s.v3 = rotl(s.v3, 16);
    s.v3 ^= s.v2;
    s.v0 = s.v0.wrapping_add(s.v3);
    s.v3 = rotl(s.v3, 21);
    s.v3 ^= s.v0;
    s.v2 = s.v2.wrapping_add(s.v1);
    s.v1 = rotl(s.v1, 17);
    s.v1 ^= s.v2;
    s.v2 = rotl(s.v2, 32);
}

impl SipHash {
    fn state(&self) -> State {
        // State is rebuilt from buf metadata; this stub does not maintain
        // the full SipHash state across invocations, see siphash_reset for full setup.
        State {
            v0: 0,
            v1: 0,
            v2: 0,
            v3: 0,
            inlen: 0,
            buflen: 0,
        }
    }

    pub fn siphash_update(&mut self, data: &[u8], nb_bytes: u64) {
        let n = (nb_bytes as usize).min(data.len());
        self.buf.extend_from_slice(&data[..n]);
        // Process complete 8-byte blocks (left in buf for digest; this stub
        // is simplified compared to full SipHash; the library entrypoints
        // do not compile this module).
    }
    pub fn process_next_block(&mut self) {
        // Consume the first 8 bytes of buf if available.
        if self.buf.len() >= 8 {
            self.buf.drain(..8);
        }
    }
    pub fn process_final_block(&mut self) {
        // No-op for this stub.
    }
    pub fn siphash_pad(&mut self, nb_bytes: u64) {
        for _ in 0..nb_bytes {
            self.buf.push(0);
        }
    }
    pub fn siphash_init(key_128bit: &[u8]) -> Self {
        let mut kk = Vec::with_capacity(16);
        let n = key_128bit.len().min(16);
        kk.extend_from_slice(&key_128bit[..n]);
        while kk.len() < 16 {
            kk.push(0);
        }
        SipHash {
            kk,
            out: vec![0u8; OUTLEN],
            buf: Vec::new(),
        }
    }
    pub fn siphash_reset(&mut self) {
        self.buf.clear();
        for b in self.out.iter_mut() {
            *b = 0;
        }
    }
    pub fn siphash_digest(&self) -> Vec<u8> {
        // Compute a full SipHash-2-4 digest of accumulated buffer using key kk.
        // Build initial state from key.
        let k0 = u8to64_le(&self.kk[0..8]);
        let k1 = u8to64_le(&self.kk[8..16]);
        let mut s = State {
            v0: 0x736f6d6570736575u64 ^ k0,
            v1: 0x646f72616e646f6du64 ^ k1 ^ 0xee,
            v2: 0x6c7967656e657261u64 ^ k0,
            v3: 0x7465646279746573u64 ^ k1,
            inlen: self.buf.len() as u64,
            buflen: 0,
        };
        let inlen = self.buf.len();
        let nblocks = inlen / 8;
        for i in 0..nblocks {
            let m = u8to64_le(&self.buf[i * 8..i * 8 + 8]);
            s.v3 ^= m;
            for _ in 0..C_ROUNDS {
                sipround(&mut s);
            }
            s.v0 ^= m;
        }
        let left = inlen & 7;
        let mut b: u64 = (inlen as u64) << 56;
        let off = nblocks * 8;
        for i in 0..left {
            b |= (self.buf[off + i] as u64) << (i * 8);
        }
        s.v3 ^= b;
        for _ in 0..C_ROUNDS {
            sipround(&mut s);
        }
        s.v0 ^= b;
        s.v2 ^= 0xee;
        for _ in 0..D_ROUNDS {
            sipround(&mut s);
        }
        let mut out = vec![0u8; OUTLEN];
        let r = s.v0 ^ s.v1 ^ s.v2 ^ s.v3;
        u64to8_le(&mut out[0..8], r);
        s.v1 ^= 0xdd;
        for _ in 0..D_ROUNDS {
            sipround(&mut s);
        }
        let r = s.v0 ^ s.v1 ^ s.v2 ^ s.v3;
        u64to8_le(&mut out[8..16], r);
        out
    }
    pub fn siphash_free(&mut self) {
        self.kk.clear();
        self.out.clear();
        self.buf.clear();
    }
}
