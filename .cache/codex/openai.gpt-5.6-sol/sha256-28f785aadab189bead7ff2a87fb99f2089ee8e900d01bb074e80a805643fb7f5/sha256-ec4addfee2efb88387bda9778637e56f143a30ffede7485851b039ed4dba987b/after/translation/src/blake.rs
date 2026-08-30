const SIGMA: [[usize; 16]; 10] = [
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
];

const C32: [u32; 16] = [
    0x243f6a88, 0x85a308d3, 0x13198a2e, 0x03707344,
    0xa4093822, 0x299f31d0, 0x082efa98, 0xec4e6c89,
    0x452821e6, 0x38d01377, 0xbe5466cf, 0x34e90c6c,
    0xc0ac29b7, 0xc97c50dd, 0x3f84d5b5, 0xb5470917,
];

#[unsafe(no_mangle)]
pub static cst: [u64; 16] = [
    0x243f6a8885a308d3, 0x13198a2e03707344,
    0xa4093822299f31d0, 0x082efa98ec4e6c89,
    0x452821e638d01377, 0xbe5466cf34e90c6c,
    0xc0ac29b7c97c50dd, 0x3f84d5b5b5470917,
    0x9216d5d98979fb1b, 0xd1310ba698dfb5ac,
    0x2ffd72dbd01adfb7, 0xb8e1afed6a267e96,
    0xba7c9045f12c7f99, 0x24a19947b3916cf7,
    0x0801f2e2858efc16, 0x636920d871574e69,
];

const C64: [u64; 16] = cst;

#[repr(C)]
#[derive(Clone)]
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
        Self {
            h: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
                0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
            ],
            s: [0; 4],
            t: [0; 2],
            buflen: 0,
            nullt: 0,
            buf: [0; 64],
        }
    }

    pub fn compress(&mut self, block: &[u8]) {
        let mut m = [0u32; 16];
        for (word, bytes) in m.iter_mut().zip(block.chunks_exact(4)) {
            *word = u32::from_be_bytes(bytes.try_into().unwrap());
        }
        let mut v = [0u32; 16];
        v[..8].copy_from_slice(&self.h);
        for i in 0..4 {
            v[8 + i] = self.s[i] ^ C32[i];
        }
        v[12..16].copy_from_slice(&C32[4..8]);
        if self.nullt == 0 {
            v[12] ^= self.t[0];
            v[13] ^= self.t[0];
            v[14] ^= self.t[1];
            v[15] ^= self.t[1];
        }
        for round in 0..14 {
            let sigma = &SIGMA[round % 10];
            for i in 0..4 {
                g32(&mut v, i, i + 4, i + 8, i + 12, i, sigma, &m);
            }
            g32(&mut v, 0, 5, 10, 15, 4, sigma, &m);
            g32(&mut v, 1, 6, 11, 12, 5, sigma, &m);
            g32(&mut v, 2, 7, 8, 13, 6, sigma, &m);
            g32(&mut v, 3, 4, 9, 14, 7, sigma, &m);
        }
        for i in 0..8 {
            self.h[i] ^= v[i] ^ v[i + 8] ^ self.s[i % 4];
        }
    }

    pub fn update_bits(&mut self, mut data: &[u8], mut datalen: u64) {
        let mut left = (self.buflen >> 3) as usize;
        let fill = 64 - left;
        if left != 0 && (((datalen >> 3) & 0x3f) as usize) >= fill {
            self.buf[left..left + fill].copy_from_slice(&data[..fill]);
            self.t[0] = self.t[0].wrapping_add(512);
            if self.t[0] == 0 {
                self.t[1] = self.t[1].wrapping_add(1);
            }
            let block = self.buf;
            self.compress(&block);
            data = &data[fill..];
            datalen -= (fill * 8) as u64;
            left = 0;
        }
        while datalen >= 512 {
            self.t[0] = self.t[0].wrapping_add(512);
            if self.t[0] == 0 {
                self.t[1] = self.t[1].wrapping_add(1);
            }
            self.compress(&data[..64]);
            data = &data[64..];
            datalen -= 512;
        }
        if datalen > 0 {
            let bytes = (datalen >> 3) as usize;
            self.buf[left..left + bytes].copy_from_slice(&data[..bytes]);
            self.buflen = (left as i32 * 8) + datalen as i32;
        } else {
            self.buflen = 0;
        }
    }

    pub fn finalize(&mut self) -> [u8; 32] {
        let lo = self.t[0].wrapping_add(self.buflen as u32);
        let hi = self.t[1].wrapping_add((lo < self.buflen as u32) as u32);
        let mut msglen = [0u8; 8];
        msglen[..4].copy_from_slice(&hi.to_be_bytes());
        msglen[4..].copy_from_slice(&lo.to_be_bytes());
        let mut padding = [0u8; 64];
        padding[0] = 0x80;

        if self.buflen == 440 {
            self.t[0] = self.t[0].wrapping_sub(8);
            self.update_bits(&[0x81], 8);
        } else {
            if self.buflen < 440 {
                if self.buflen == 0 {
                    self.nullt = 1;
                }
                let bits = (440 - self.buflen) as u64;
                self.t[0] = self.t[0].wrapping_sub(bits as u32);
                self.update_bits(&padding, bits);
            } else {
                let bits = (512 - self.buflen) as u64;
                self.t[0] = self.t[0].wrapping_sub(bits as u32);
                self.update_bits(&padding, bits);
                self.t[0] = self.t[0].wrapping_sub(440);
                self.update_bits(&padding[1..], 440);
                self.nullt = 1;
            }
            self.update_bits(&[0x01], 8);
            self.t[0] = self.t[0].wrapping_sub(8);
        }
        self.t[0] = self.t[0].wrapping_sub(64);
        self.update_bits(&msglen, 64);
        let mut out = [0u8; 32];
        for (bytes, word) in out.chunks_exact_mut(4).zip(self.h) {
            bytes.copy_from_slice(&word.to_be_bytes());
        }
        out
    }
}

fn g32(
    v: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize,
    e: usize, sigma: &[usize; 16], m: &[u32; 16],
) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(m[sigma[2 * e]] ^ C32[sigma[2 * e + 1]]);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(12);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(m[sigma[2 * e + 1]] ^ C32[sigma[2 * e]]);
    v[d] = (v[d] ^ v[a]).rotate_right(8);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(7);
}

#[repr(C)]
#[derive(Clone)]
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
        Self {
            h: [
                0x6a09e667f3bcc908, 0xbb67ae8584caa73b,
                0x3c6ef372fe94f82b, 0xa54ff53a5f1d36f1,
                0x510e527fade682d1, 0x9b05688c2b3e6c1f,
                0x1f83d9abfb41bd6b, 0x5be0cd19137e2179,
            ],
            s: [0; 4],
            t: [0; 2],
            buflen: 0,
            nullt: 0,
            buf: [0; 128],
        }
    }

    pub fn compress(&mut self, block: &[u8]) {
        let mut m = [0u64; 16];
        for (word, bytes) in m.iter_mut().zip(block.chunks_exact(8)) {
            *word = u64::from_be_bytes(bytes.try_into().unwrap());
        }
        let mut v = [0u64; 16];
        v[..8].copy_from_slice(&self.h);
        for i in 0..4 {
            v[8 + i] = self.s[i] ^ C64[i];
        }
        v[12..16].copy_from_slice(&C64[4..8]);
        if self.nullt == 0 {
            v[12] ^= self.t[0];
            v[13] ^= self.t[0];
            v[14] ^= self.t[1];
            v[15] ^= self.t[1];
        }
        for round in 0..16 {
            let sigma = &SIGMA[round % 10];
            for i in 0..4 {
                g64(&mut v, i, i + 4, i + 8, i + 12, i, sigma, &m);
            }
            g64(&mut v, 0, 5, 10, 15, 4, sigma, &m);
            g64(&mut v, 1, 6, 11, 12, 5, sigma, &m);
            g64(&mut v, 2, 7, 8, 13, 6, sigma, &m);
            g64(&mut v, 3, 4, 9, 14, 7, sigma, &m);
        }
        for i in 0..8 {
            self.h[i] ^= v[i] ^ v[i + 8] ^ self.s[i % 4];
        }
    }

    pub fn update_bits(&mut self, mut data: &[u8], mut datalen: u64) {
        let mut left = (self.buflen >> 3) as usize;
        let fill = 128 - left;
        if left != 0 && (((datalen >> 3) & 0x7f) as usize) >= fill {
            self.buf[left..left + fill].copy_from_slice(&data[..fill]);
            self.t[0] = self.t[0].wrapping_add(1024);
            let block = self.buf;
            self.compress(&block);
            data = &data[fill..];
            datalen -= (fill * 8) as u64;
            left = 0;
        }
        while datalen >= 1024 {
            self.t[0] = self.t[0].wrapping_add(1024);
            self.compress(&data[..128]);
            data = &data[128..];
            datalen -= 1024;
        }
        if datalen > 0 {
            let bytes = ((datalen >> 3) & 0x7f) as usize;
            self.buf[left..left + bytes].copy_from_slice(&data[..bytes]);
            self.buflen = (left as i32 * 8) + datalen as i32;
        } else {
            self.buflen = 0;
        }
    }

    pub fn finalize(&mut self) -> [u8; 64] {
        let lo = self.t[0].wrapping_add(self.buflen as u64);
        let hi = self.t[1].wrapping_add((lo < self.buflen as u64) as u64);
        let mut msglen = [0u8; 16];
        msglen[..8].copy_from_slice(&hi.to_be_bytes());
        msglen[8..].copy_from_slice(&lo.to_be_bytes());
        let mut padding = [0u8; 129];
        padding[0] = 0x80;

        if self.buflen == 888 {
            self.t[0] = self.t[0].wrapping_sub(8);
            self.update_bits(&[0x81], 8);
        } else {
            if self.buflen < 888 {
                if self.buflen == 0 {
                    self.nullt = 1;
                }
                let bits = (888 - self.buflen) as u64;
                self.t[0] = self.t[0].wrapping_sub(bits);
                self.update_bits(&padding, bits);
            } else {
                let bits = (1024 - self.buflen) as u64;
                self.t[0] = self.t[0].wrapping_sub(bits);
                self.update_bits(&padding, bits);
                self.t[0] = self.t[0].wrapping_sub(888);
                self.update_bits(&padding[1..], 888);
                self.nullt = 1;
            }
            self.update_bits(&[0x01], 8);
            self.t[0] = self.t[0].wrapping_sub(8);
        }
        self.t[0] = self.t[0].wrapping_sub(128);
        self.update_bits(&msglen, 128);
        let mut out = [0u8; 64];
        for (bytes, word) in out.chunks_exact_mut(8).zip(self.h) {
            bytes.copy_from_slice(&word.to_be_bytes());
        }
        out
    }
}

fn g64(
    v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize,
    e: usize, sigma: &[usize; 16], m: &[u64; 16],
) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(m[sigma[2 * e]] ^ C64[sigma[2 * e + 1]]);
    v[d] = (v[d] ^ v[a]).rotate_right(32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(25);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(m[sigma[2 * e + 1]] ^ C64[sigma[2 * e]]);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(11);
}

pub fn blake256(input: &[u8]) -> [u8; 32] {
    let mut state = Blake256State::new();
    state.update_bits(input, (input.len() as u64) * 8);
    state.finalize()
}

pub fn blake512(input: &[u8]) -> [u8; 64] {
    let mut state = Blake512State::new();
    state.update_bits(input, (input.len() as u64) * 8);
    state.finalize()
}

pub fn blake256_mgf1(out: &mut [u8], input: &[u8]) {
    mgf1(out, input, false);
}

pub fn blake512_mgf1(out: &mut [u8], input: &[u8]) {
    mgf1(out, input, true);
}

fn mgf1(out: &mut [u8], input: &[u8], wide: bool) {
    let output_bytes = if wide { 64 } else { 32 };
    let mut inbuf = Vec::with_capacity(input.len() + 4);
    inbuf.extend_from_slice(input);
    inbuf.extend_from_slice(&[0; 4]);
    for (counter, chunk) in out.chunks_mut(output_bytes).enumerate() {
        inbuf[input.len()..].copy_from_slice(&(counter as u32).to_be_bytes());
        if wide {
            chunk.copy_from_slice(&blake512(&inbuf)[..chunk.len()]);
        } else {
            chunk.copy_from_slice(&blake256(&inbuf)[..chunk.len()]);
        }
    }
}

pub fn blakex_buggy(parts: &[&[u8]]) -> Vec<u8> {
    if crate::params::SPX_N >= 24 {
        let mut state = Blake512State::new();
        for part in parts {
            state.update_bits(part, part.len() as u64);
        }
        state.finalize().to_vec()
    } else {
        let mut state = Blake256State::new();
        for part in parts {
            state.update_bits(part, part.len() as u64);
        }
        state.finalize().to_vec()
    }
}
