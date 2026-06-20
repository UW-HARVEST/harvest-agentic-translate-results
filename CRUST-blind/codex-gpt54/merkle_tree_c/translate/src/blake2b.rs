pub const BLAKE2B_BLOCKBYTES: usize = 128;
pub const BLAKE2B_OUTBYTES: usize = 64;
pub const BLAKE2B_KEYBYTES: usize = 64;
pub const BLAKE2B_SALTBYTES: usize = 16;
pub const BLAKE2B_PERSONALBYTES: usize = 16;
const DEFAULT_PERSONAL: &[u8; BLAKE2B_PERSONALBYTES] = b"ckb-default-hash";
const BLAKE2B_IV: [u64; 8] = [
    0x6a09_e667_f3bc_c908,
    0xbb67_ae85_84ca_a73b,
    0x3c6e_f372_fe94_f82b,
    0xa54f_f53a_5f1d_36f1,
    0x510e_527f_ade6_82d1,
    0x9b05_688c_2b3e_6c1f,
    0x1f83_d9ab_fb41_bd6b,
    0x5be0_cd19_137e_2179,
];
const BLAKE2B_SIGMA: [[usize; 16]; 12] = [
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
// Streaming API
pub fn blake2b_init(state: &mut Blake2bState, outlen: usize) -> i32 {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES {
        return -1;
    }

    let mut personal = [0u8; BLAKE2B_PERSONALBYTES];
    personal.copy_from_slice(DEFAULT_PERSONAL);
    let param = Blake2bParam {
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
        personal,
    };
    blake2b_init_param(state, &param)
}
pub fn blake2b_init_key(state: &mut Blake2bState, outlen: usize, key: &[u8]) -> i32 {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES {
        return -1;
    }
    if key.is_empty() || key.len() > BLAKE2B_KEYBYTES {
        return -1;
    }

    let param = Blake2bParam {
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

    if blake2b_init_param(state, &param) < 0 {
        return -1;
    }

    let mut block = [0u8; BLAKE2B_BLOCKBYTES];
    block[..key.len()].copy_from_slice(key);
    let ret = blake2b_update(state, &block);
    secure_zero_memory(&mut block);
    ret
}
pub fn blake2b_init_param(state: &mut Blake2bState, param: &Blake2bParam) -> i32 {
    *state = blake2b_state_init0();
    let param_bytes = blake2b_param_bytes(param);
    for i in 0..8 {
        state.h[i] ^= load64(&param_bytes[i * 8..(i + 1) * 8]);
    }
    state.outlen = param.digest_length as usize;
    0
}
pub fn blake2b_update(state: &mut Blake2bState, input: &[u8]) -> i32 {
    if input.is_empty() {
        return 0;
    }

    let mut in_offset = 0usize;
    let mut inlen = input.len();
    let left = state.buflen;
    let fill = BLAKE2B_BLOCKBYTES - left;

    if inlen > fill {
        state.buf[left..left + fill].copy_from_slice(&input[..fill]);
        state.buflen = 0;
        blake2b_increment_counter(state, BLAKE2B_BLOCKBYTES as u64);
        let block = state.buf;
        blake2b_compress(state, &block);
        in_offset += fill;
        inlen -= fill;

        while inlen > BLAKE2B_BLOCKBYTES {
            blake2b_increment_counter(state, BLAKE2B_BLOCKBYTES as u64);
            let end = in_offset + BLAKE2B_BLOCKBYTES;
            let mut block = [0u8; BLAKE2B_BLOCKBYTES];
            block.copy_from_slice(&input[in_offset..end]);
            blake2b_compress(state, &block);
            in_offset = end;
            inlen -= BLAKE2B_BLOCKBYTES;
        }
    }

    state.buf[state.buflen..state.buflen + inlen]
        .copy_from_slice(&input[in_offset..in_offset + inlen]);
    state.buflen += inlen;
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
    for byte in &mut state.buf[state.buflen..] {
        *byte = 0;
    }
    let block = state.buf;
    blake2b_compress(state, &block);

    let mut buffer = [0u8; BLAKE2B_OUTBYTES];
    for i in 0..8 {
        store64(&mut buffer[i * 8..(i + 1) * 8], state.h[i]);
    }
    out[..state.outlen].copy_from_slice(&buffer[..state.outlen]);
    secure_zero_memory(&mut buffer);
    0
}
// Simple API (key may be provided as Option)
pub fn blake2b(out: &mut [u8], input: &[u8], key: Option<&[u8]>) -> i32 {
    if out.is_empty() || out.len() > BLAKE2B_OUTBYTES {
        return -1;
    }

    let mut state = blake2b_state_init0();
    let init_ret = match key {
        Some(key_bytes) if !key_bytes.is_empty() => {
            if key_bytes.len() > BLAKE2B_KEYBYTES {
                return -1;
            }
            blake2b_init_key(&mut state, out.len(), key_bytes)
        }
        _ => blake2b_init(&mut state, out.len()),
    };
    if init_ret < 0 {
        return -1;
    }
    if blake2b_update(&mut state, input) < 0 {
        return -1;
    }
    blake2b_final(&mut state, out)
}
// This is simply an alias for blake2b
pub fn blake2(out: &mut [u8], input: &[u8], key: Option<&[u8]>) -> i32 {
    blake2b(out, input, key)
}

fn blake2b_state_init0() -> Blake2bState {
    Blake2bState {
        h: BLAKE2B_IV,
        t: [0; 2],
        f: [0; 2],
        buf: [0; BLAKE2B_BLOCKBYTES],
        buflen: 0,
        outlen: 0,
        last_node: 0,
    }
}

fn blake2b_param_bytes(param: &Blake2bParam) -> [u8; BLAKE2B_OUTBYTES] {
    let mut bytes = [0u8; BLAKE2B_OUTBYTES];
    bytes[0] = param.digest_length;
    bytes[1] = param.key_length;
    bytes[2] = param.fanout;
    bytes[3] = param.depth;

    let leaf_length = param.leaf_length;
    let node_offset = param.node_offset;
    let xof_length = param.xof_length;
    let node_depth = param.node_depth;
    let inner_length = param.inner_length;
    let reserved = param.reserved;
    let salt = param.salt;
    let personal = param.personal;

    bytes[4..8].copy_from_slice(&leaf_length.to_le_bytes());
    bytes[8..12].copy_from_slice(&node_offset.to_le_bytes());
    bytes[12..16].copy_from_slice(&xof_length.to_le_bytes());
    bytes[16] = node_depth;
    bytes[17] = inner_length;
    bytes[18..32].copy_from_slice(&reserved);
    bytes[32..48].copy_from_slice(&salt);
    bytes[48..64].copy_from_slice(&personal);
    bytes
}

fn load64(src: &[u8]) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&src[..8]);
    u64::from_le_bytes(bytes)
}

fn store64(dst: &mut [u8], word: u64) {
    dst[..8].copy_from_slice(&word.to_le_bytes());
}

fn rotr64(word: u64, count: u32) -> u64 {
    word.rotate_right(count)
}

fn secure_zero_memory(buf: &mut [u8]) {
    buf.fill(0);
}

fn blake2b_set_lastnode(state: &mut Blake2bState) {
    state.f[1] = u64::MAX;
}

fn blake2b_is_lastblock(state: &Blake2bState) -> bool {
    state.f[0] != 0
}

fn blake2b_set_lastblock(state: &mut Blake2bState) {
    if state.last_node != 0 {
        blake2b_set_lastnode(state);
    }
    state.f[0] = u64::MAX;
}

fn blake2b_increment_counter(state: &mut Blake2bState, inc: u64) {
    state.t[0] = state.t[0].wrapping_add(inc);
    if state.t[0] < inc {
        state.t[1] = state.t[1].wrapping_add(1);
    }
}

fn g(
    round: usize,
    idx: usize,
    v: &mut [u64; 16],
    m: &[u64; 16],
    a: usize,
    b: usize,
    c: usize,
    d: usize,
) {
    v[a] = v[a]
        .wrapping_add(v[b])
        .wrapping_add(m[BLAKE2B_SIGMA[round][2 * idx]]);
    v[d] = rotr64(v[d] ^ v[a], 32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = rotr64(v[b] ^ v[c], 24);
    v[a] = v[a]
        .wrapping_add(v[b])
        .wrapping_add(m[BLAKE2B_SIGMA[round][2 * idx + 1]]);
    v[d] = rotr64(v[d] ^ v[a], 16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = rotr64(v[b] ^ v[c], 63);
}

fn blake2b_round(round: usize, v: &mut [u64; 16], m: &[u64; 16]) {
    g(round, 0, v, m, 0, 4, 8, 12);
    g(round, 1, v, m, 1, 5, 9, 13);
    g(round, 2, v, m, 2, 6, 10, 14);
    g(round, 3, v, m, 3, 7, 11, 15);
    g(round, 4, v, m, 0, 5, 10, 15);
    g(round, 5, v, m, 1, 6, 11, 12);
    g(round, 6, v, m, 2, 7, 8, 13);
    g(round, 7, v, m, 3, 4, 9, 14);
}

fn blake2b_compress(state: &mut Blake2bState, block: &[u8; BLAKE2B_BLOCKBYTES]) {
    let mut m = [0u64; 16];
    let mut v = [0u64; 16];

    for i in 0..16 {
        m[i] = load64(&block[i * 8..(i + 1) * 8]);
    }

    v[..8].copy_from_slice(&state.h);
    v[8] = BLAKE2B_IV[0];
    v[9] = BLAKE2B_IV[1];
    v[10] = BLAKE2B_IV[2];
    v[11] = BLAKE2B_IV[3];
    v[12] = BLAKE2B_IV[4] ^ state.t[0];
    v[13] = BLAKE2B_IV[5] ^ state.t[1];
    v[14] = BLAKE2B_IV[6] ^ state.f[0];
    v[15] = BLAKE2B_IV[7] ^ state.f[1];

    for round in 0..12 {
        blake2b_round(round, &mut v, &m);
    }

    for i in 0..8 {
        state.h[i] ^= v[i] ^ v[i + 8];
    }
}
