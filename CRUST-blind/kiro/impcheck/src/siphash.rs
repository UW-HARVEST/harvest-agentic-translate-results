pub const C_ROUNDS: u32 = 2;
pub const D_ROUNDS: u32 = 4;
pub struct SipHash {
kk: Vec<u8>,
out: Vec<u8>,
buf: Vec<u8>,
v0: u64,
v1: u64,
v2: u64,
v3: u64,
k0: u64,
k1: u64,
inlen: u64,
buflen: u8,
}

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

macro_rules! sipround {
    ($v0:expr, $v1:expr, $v2:expr, $v3:expr) => {
        $v0 = $v0.wrapping_add($v1);
        $v1 = rotl($v1, 13);
        $v1 ^= $v0;
        $v0 = rotl($v0, 32);
        $v2 = $v2.wrapping_add($v3);
        $v3 = rotl($v3, 16);
        $v3 ^= $v2;
        $v0 = $v0.wrapping_add($v3);
        $v3 = rotl($v3, 21);
        $v3 ^= $v0;
        $v2 = $v2.wrapping_add($v1);
        $v1 = rotl($v1, 17);
        $v1 ^= $v2;
        $v2 = rotl($v2, 32);
    };
}

impl SipHash {
pub fn siphash_update(&mut self, data: &[u8], nb_bytes: u64) {
    let nb = nb_bytes as usize;
    let mut datapos = 0usize;
    loop {
        while (self.buflen as usize) < 8 && datapos < nb {
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
    self.inlen += nb_bytes;
}
pub fn process_next_block(&mut self) {
    let m = u8to64_le(&self.buf);
    self.v3 ^= m;
    for _ in 0..C_ROUNDS {
        sipround!(self.v0, self.v1, self.v2, self.v3);
    }
    self.v0 ^= m;
}
pub fn process_final_block(&mut self) {
    let left = (self.inlen & 7) as u8;
    assert_eq!(left, self.buflen);
    let mut b: u64 = (self.inlen) << 56;
    match left {
        7 => { b |= (self.buf[6] as u64) << 48; b |= (self.buf[5] as u64) << 40; b |= (self.buf[4] as u64) << 32; b |= (self.buf[3] as u64) << 24; b |= (self.buf[2] as u64) << 16; b |= (self.buf[1] as u64) << 8; b |= self.buf[0] as u64; }
        6 => { b |= (self.buf[5] as u64) << 40; b |= (self.buf[4] as u64) << 32; b |= (self.buf[3] as u64) << 24; b |= (self.buf[2] as u64) << 16; b |= (self.buf[1] as u64) << 8; b |= self.buf[0] as u64; }
        5 => { b |= (self.buf[4] as u64) << 32; b |= (self.buf[3] as u64) << 24; b |= (self.buf[2] as u64) << 16; b |= (self.buf[1] as u64) << 8; b |= self.buf[0] as u64; }
        4 => { b |= (self.buf[3] as u64) << 24; b |= (self.buf[2] as u64) << 16; b |= (self.buf[1] as u64) << 8; b |= self.buf[0] as u64; }
        3 => { b |= (self.buf[2] as u64) << 16; b |= (self.buf[1] as u64) << 8; b |= self.buf[0] as u64; }
        2 => { b |= (self.buf[1] as u64) << 8; b |= self.buf[0] as u64; }
        1 => { b |= self.buf[0] as u64; }
        0 => {}
        _ => {}
    }
    self.v3 ^= b;
    for _ in 0..C_ROUNDS {
        sipround!(self.v0, self.v1, self.v2, self.v3);
    }
    self.v0 ^= b;
    // outlen is always 16
    self.v2 ^= 0xee;
    for _ in 0..D_ROUNDS {
        sipround!(self.v0, self.v1, self.v2, self.v3);
    }
    let b2 = self.v0 ^ self.v1 ^ self.v2 ^ self.v3;
    u64to8_le(&mut self.out[0..8], b2);
    self.v1 ^= 0xdd;
    for _ in 0..D_ROUNDS {
        sipround!(self.v0, self.v1, self.v2, self.v3);
    }
    let b3 = self.v0 ^ self.v1 ^ self.v2 ^ self.v3;
    u64to8_le(&mut self.out[8..16], b3);
}
pub fn siphash_pad(&mut self, nb_bytes: u64) {
    for _ in 0..nb_bytes {
        self.siphash_update(&[0u8], 1);
    }
}
pub fn siphash_init(key_128bit: &[u8]) -> Self {
    let mut s = SipHash {
        kk: key_128bit.to_vec(),
        out: vec![0u8; 16],
        buf: vec![0u8; 8],
        v0: 0, v1: 0, v2: 0, v3: 0,
        k0: 0, k1: 0,
        inlen: 0,
        buflen: 0,
    };
    if !key_128bit.is_empty() {
        s.siphash_reset();
    }
    s
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
    // outlen == 16
    self.v1 ^= 0xee;
}
pub fn siphash_digest(&self) -> Vec<u8> {
    let mut copy = SipHash {
        kk: self.kk.clone(),
        out: self.out.clone(),
        buf: self.buf.clone(),
        v0: self.v0, v1: self.v1, v2: self.v2, v3: self.v3,
        k0: self.k0, k1: self.k1,
        inlen: self.inlen,
        buflen: self.buflen,
    };
    copy.process_final_block();
    copy.out
}
pub fn siphash_free(&mut self) {
    self.buf.clear();
    self.out.clear();
}
}
