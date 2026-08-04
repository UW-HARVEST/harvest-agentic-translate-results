pub const C_ROUNDS: u32 = 2;
pub const D_ROUNDS: u32 = 4;

pub struct SipHash {
    kk: Vec<u8>,
    out: Vec<u8>,
    buf: Vec<u8>,
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

fn u64_to_le_bytes(v: u64, out: &mut [u8]) {
    out[..8].copy_from_slice(&v.to_le_bytes());
}

fn sip_round(v0: &mut u64, v1: &mut u64, v2: &mut u64, v3: &mut u64) {
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

fn compute_siphash_128(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut v0 = 0x736f6d6570736575_u64;
    let mut v1 = 0x646f72616e646f6d_u64;
    let mut v2 = 0x6c7967656e657261_u64;
    let mut v3 = 0x7465646279746573_u64;
    let k0 = u8to64_le(&key[..8]);
    let k1 = u8to64_le(&key[8..16]);
    v3 ^= k1;
    v2 ^= k0;
    v1 ^= k1;
    v0 ^= k0;
    v1 ^= 0xee;

    let mut chunks = data.chunks_exact(8);
    for chunk in &mut chunks {
        let m = u8to64_le(chunk);
        v3 ^= m;
        for _ in 0..C_ROUNDS {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= m;
    }

    let rem = chunks.remainder();
    let mut b = (data.len() as u64) << 56;
    for (idx, byte) in rem.iter().enumerate() {
        b |= (*byte as u64) << (idx * 8);
    }

    v3 ^= b;
    for _ in 0..C_ROUNDS {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= b;
    v2 ^= 0xee;
    for _ in 0..D_ROUNDS {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }

    let mut out = vec![0_u8; 16];
    let mut digest = v0 ^ v1 ^ v2 ^ v3;
    u64_to_le_bytes(digest, &mut out[..8]);

    v1 ^= 0xdd;
    for _ in 0..D_ROUNDS {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    digest = v0 ^ v1 ^ v2 ^ v3;
    u64_to_le_bytes(digest, &mut out[8..]);
    out
}

impl SipHash {
pub fn siphash_update(&mut self, data: &[u8], nb_bytes: u64) {
    self.buf.extend_from_slice(&data[..data.len().min(nb_bytes as usize)]);
}
pub fn process_next_block(&mut self) {
    self.out = compute_siphash_128(&self.kk, &self.buf);
}
pub fn process_final_block(&mut self) {
    self.out = compute_siphash_128(&self.kk, &self.buf);
}
pub fn siphash_pad(&mut self, nb_bytes: u64) {
    self.buf
        .extend(std::iter::repeat_n(0_u8, nb_bytes as usize));
}
pub fn siphash_init(key_128bit: &[u8]) -> Self {
    let mut state = Self {
        kk: key_128bit[..key_128bit.len().min(16)].to_vec(),
        out: vec![0_u8; 16],
        buf: Vec::new(),
    };
    if state.kk.len() < 16 {
        state.kk.resize(16, 0);
    }
    state.siphash_reset();
    state
}
pub fn siphash_reset(&mut self) {
    self.buf.clear();
    self.out.fill(0);
}
pub fn siphash_digest(&self) -> Vec<u8> {
    compute_siphash_128(&self.kk, &self.buf)
}
pub fn siphash_free(&mut self) {
    self.kk.clear();
    self.out.clear();
    self.buf.clear();
}
}
