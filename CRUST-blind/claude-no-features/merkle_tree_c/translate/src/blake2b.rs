pub const BLAKE2B_BLOCKBYTES: usize = 128;
pub const BLAKE2B_OUTBYTES: usize = 64;
pub const BLAKE2B_KEYBYTES: usize = 64;
pub const BLAKE2B_SALTBYTES: usize = 16;
pub const BLAKE2B_PERSONALBYTES: usize = 16;
#[derive(Debug, Clone)]
pub struct Blake2bState {
    pub h: [u64; 8],
    pub t: [u64; 2],
    pub f: [u64; 2],
    pub buf: [u8; BLAKE2B_BLOCKBYTES],
    pub buflen: usize,
    pub outlen: usize,
    pub last_node: u8,
}
#[repr(packed)]
#[derive(Debug, Clone)]
pub struct Blake2bParam {
    pub digest_length: u8,
    pub key_length: u8,
    pub fanout: u8,
    pub depth: u8,
    pub leaf_length: u32,
    pub node_offset: u32,
    pub xof_length: u32,
    pub node_depth: u8,
    pub inner_length: u8,
    pub reserved: [u8; 14],
    pub salt: [u8; BLAKE2B_SALTBYTES],
    pub personal: [u8; BLAKE2B_PERSONALBYTES],
}

const BLAKE2B_IV: [u64; 8] = [
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
];

const BLAKE2B_SIGMA: [[u8; 16]; 12] = [
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
];

const DEFAULT_PERSONAL: &[u8; 16] = b"ckb-default-hash";

#[inline]
fn load64(src: &[u8]) -> u64 {
    u64::from_le_bytes([
        src[0], src[1], src[2], src[3], src[4], src[5], src[6], src[7],
    ])
}

#[inline]
fn store64(dst: &mut [u8], w: u64) {
    let bytes = w.to_le_bytes();
    dst[..8].copy_from_slice(&bytes);
}

#[inline]
fn rotr64(w: u64, c: u32) -> u64 {
    w.rotate_right(c)
}

fn param_to_bytes(p: &Blake2bParam) -> [u8; 64] {
    let mut bytes = [0u8; 64];
    bytes[0] = p.digest_length;
    bytes[1] = p.key_length;
    bytes[2] = p.fanout;
    bytes[3] = p.depth;
    // Read packed fields safely by copying
    let leaf_length = unsafe { core::ptr::addr_of!(p.leaf_length).read_unaligned() };
    let node_offset = unsafe { core::ptr::addr_of!(p.node_offset).read_unaligned() };
    let xof_length = unsafe { core::ptr::addr_of!(p.xof_length).read_unaligned() };
    bytes[4..8].copy_from_slice(&leaf_length.to_le_bytes());
    bytes[8..12].copy_from_slice(&node_offset.to_le_bytes());
    bytes[12..16].copy_from_slice(&xof_length.to_le_bytes());
    bytes[16] = p.node_depth;
    bytes[17] = p.inner_length;
    bytes[18..32].copy_from_slice(&p.reserved);
    bytes[32..48].copy_from_slice(&p.salt);
    bytes[48..64].copy_from_slice(&p.personal);
    bytes
}

fn blake2b_init0(state: &mut Blake2bState) {
    state.h = [0; 8];
    state.t = [0; 2];
    state.f = [0; 2];
    state.buf = [0; BLAKE2B_BLOCKBYTES];
    state.buflen = 0;
    state.outlen = 0;
    state.last_node = 0;
    for i in 0..8 {
        state.h[i] = BLAKE2B_IV[i];
    }
}

fn blake2b_increment_counter(state: &mut Blake2bState, inc: u64) {
    state.t[0] = state.t[0].wrapping_add(inc);
    if state.t[0] < inc {
        state.t[1] = state.t[1].wrapping_add(1);
    }
}

fn blake2b_set_lastnode(state: &mut Blake2bState) {
    state.f[1] = u64::MAX;
}

fn blake2b_set_lastblock(state: &mut Blake2bState) {
    if state.last_node != 0 {
        blake2b_set_lastnode(state);
    }
    state.f[0] = u64::MAX;
}

fn blake2b_is_lastblock(state: &Blake2bState) -> bool {
    state.f[0] != 0
}

#[inline]
fn g(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, x: u64, y: u64) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(x);
    v[d] = rotr64(v[d] ^ v[a], 32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = rotr64(v[b] ^ v[c], 24);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(y);
    v[d] = rotr64(v[d] ^ v[a], 16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = rotr64(v[b] ^ v[c], 63);
}

fn round_fn(v: &mut [u64; 16], m: &[u64; 16], r: usize) {
    let s = &BLAKE2B_SIGMA[r];
    g(v, 0, 4, 8, 12, m[s[0] as usize], m[s[1] as usize]);
    g(v, 1, 5, 9, 13, m[s[2] as usize], m[s[3] as usize]);
    g(v, 2, 6, 10, 14, m[s[4] as usize], m[s[5] as usize]);
    g(v, 3, 7, 11, 15, m[s[6] as usize], m[s[7] as usize]);
    g(v, 0, 5, 10, 15, m[s[8] as usize], m[s[9] as usize]);
    g(v, 1, 6, 11, 12, m[s[10] as usize], m[s[11] as usize]);
    g(v, 2, 7, 8, 13, m[s[12] as usize], m[s[13] as usize]);
    g(v, 3, 4, 9, 14, m[s[14] as usize], m[s[15] as usize]);
}

fn blake2b_compress(state: &mut Blake2bState, block: &[u8; BLAKE2B_BLOCKBYTES]) {
    let mut m = [0u64; 16];
    let mut v = [0u64; 16];

    for i in 0..16 {
        m[i] = load64(&block[i * 8..i * 8 + 8]);
    }

    for i in 0..8 {
        v[i] = state.h[i];
    }

    v[8] = BLAKE2B_IV[0];
    v[9] = BLAKE2B_IV[1];
    v[10] = BLAKE2B_IV[2];
    v[11] = BLAKE2B_IV[3];
    v[12] = BLAKE2B_IV[4] ^ state.t[0];
    v[13] = BLAKE2B_IV[5] ^ state.t[1];
    v[14] = BLAKE2B_IV[6] ^ state.f[0];
    v[15] = BLAKE2B_IV[7] ^ state.f[1];

    for r in 0..12 {
        round_fn(&mut v, &m, r);
    }

    for i in 0..8 {
        state.h[i] = state.h[i] ^ v[i] ^ v[i + 8];
    }
}

// Streaming API
pub fn blake2b_init(state: &mut Blake2bState, outlen: usize) -> i32 {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES {
        return -1;
    }
    let mut p = Blake2bParam {
        digest_length: outlen as u8,
        key_length: 0,
        fanout: 1,
        depth: 1,
        leaf_length: 0,
        node_offset: 0,
        xof_length: 0,
        node_depth: 0,
        inner_length: 0,
        reserved: [0; 14],
        salt: [0; BLAKE2B_SALTBYTES],
        personal: [0; BLAKE2B_PERSONALBYTES],
    };
    for i in 0..BLAKE2B_PERSONALBYTES {
        p.personal[i] = DEFAULT_PERSONAL[i];
    }
    blake2b_init_param(state, &p)
}

pub fn blake2b_init_key(state: &mut Blake2bState, outlen: usize, key: &[u8]) -> i32 {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES {
        return -1;
    }
    if key.is_empty() || key.len() > BLAKE2B_KEYBYTES {
        return -1;
    }
    let p = Blake2bParam {
        digest_length: outlen as u8,
        key_length: key.len() as u8,
        fanout: 1,
        depth: 1,
        leaf_length: 0,
        node_offset: 0,
        xof_length: 0,
        node_depth: 0,
        inner_length: 0,
        reserved: [0; 14],
        salt: [0; BLAKE2B_SALTBYTES],
        personal: [0; BLAKE2B_PERSONALBYTES],
    };
    if blake2b_init_param(state, &p) < 0 {
        return -1;
    }
    let mut block = [0u8; BLAKE2B_BLOCKBYTES];
    block[..key.len()].copy_from_slice(key);
    blake2b_update(state, &block);
    // Burn the key
    for b in block.iter_mut() {
        *b = 0;
    }
    0
}

pub fn blake2b_init_param(state: &mut Blake2bState, param: &Blake2bParam) -> i32 {
    blake2b_init0(state);
    let bytes = param_to_bytes(param);
    for i in 0..8 {
        state.h[i] ^= load64(&bytes[i * 8..i * 8 + 8]);
    }
    state.outlen = param.digest_length as usize;
    0
}

pub fn blake2b_update(state: &mut Blake2bState, input: &[u8]) -> i32 {
    let mut input = input;
    if !input.is_empty() {
        let left = state.buflen;
        let fill = BLAKE2B_BLOCKBYTES - left;
        if input.len() > fill {
            state.buflen = 0;
            state.buf[left..left + fill].copy_from_slice(&input[..fill]);
            blake2b_increment_counter(state, BLAKE2B_BLOCKBYTES as u64);
            let buf_copy = state.buf;
            blake2b_compress(state, &buf_copy);
            input = &input[fill..];
            while input.len() > BLAKE2B_BLOCKBYTES {
                blake2b_increment_counter(state, BLAKE2B_BLOCKBYTES as u64);
                let mut block = [0u8; BLAKE2B_BLOCKBYTES];
                block.copy_from_slice(&input[..BLAKE2B_BLOCKBYTES]);
                blake2b_compress(state, &block);
                input = &input[BLAKE2B_BLOCKBYTES..];
            }
        }
        let buflen = state.buflen;
        state.buf[buflen..buflen + input.len()].copy_from_slice(input);
        state.buflen += input.len();
    }
    0
}

pub fn blake2b_final(state: &mut Blake2bState, out: &mut [u8]) -> i32 {
    if out.len() < state.outlen {
        return -1;
    }
    if blake2b_is_lastblock(state) {
        return -1;
    }
    blake2b_increment_counter(state, state.buflen as u64);
    blake2b_set_lastblock(state);
    // Pad
    let buflen = state.buflen;
    for i in buflen..BLAKE2B_BLOCKBYTES {
        state.buf[i] = 0;
    }
    let buf_copy = state.buf;
    blake2b_compress(state, &buf_copy);

    let mut buffer = [0u8; BLAKE2B_OUTBYTES];
    for i in 0..8 {
        store64(&mut buffer[i * 8..i * 8 + 8], state.h[i]);
    }
    out[..state.outlen].copy_from_slice(&buffer[..state.outlen]);
    0
}

// Simple API (key may be provided as Option)
pub fn blake2b(out: &mut [u8], input: &[u8], key: Option<&[u8]>) -> i32 {
    if out.is_empty() {
        return -1;
    }
    let outlen = out.len();
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES {
        return -1;
    }
    if let Some(k) = key {
        if k.len() > BLAKE2B_KEYBYTES {
            return -1;
        }
    }
    let mut state = Blake2bState {
        h: [0; 8],
        t: [0; 2],
        f: [0; 2],
        buf: [0; BLAKE2B_BLOCKBYTES],
        buflen: 0,
        outlen: 0,
        last_node: 0,
    };
    match key {
        Some(k) if !k.is_empty() => {
            if blake2b_init_key(&mut state, outlen, k) < 0 {
                return -1;
            }
        }
        _ => {
            if blake2b_init(&mut state, outlen) < 0 {
                return -1;
            }
        }
    }
    blake2b_update(&mut state, input);
    blake2b_final(&mut state, out);
    0
}

// This is simply an alias for blake2b
pub fn blake2(out: &mut [u8], input: &[u8], key: Option<&[u8]>) -> i32 {
    blake2b(out, input, key)
}
