pub const SHAKE256_RATE: usize = 136;

const RC: [u64; 24] = [
    0x0000000000000001, 0x0000000000008082, 0x800000000000808a,
    0x8000000080008000, 0x000000000000808b, 0x0000000080000001,
    0x8000000080008081, 0x8000000000008009, 0x000000000000008a,
    0x0000000000000088, 0x0000000080008009, 0x000000008000000a,
    0x000000008000808b, 0x800000000000008b, 0x8000000000008089,
    0x8000000000008003, 0x8000000000008002, 0x8000000000000080,
    0x000000000000800a, 0x800000008000000a, 0x8000000080008081,
    0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
];

const RHO: [[u32; 5]; 5] = [
    [0, 36, 3, 41, 18],
    [1, 44, 10, 45, 2],
    [62, 6, 43, 15, 61],
    [28, 55, 25, 21, 56],
    [27, 20, 39, 8, 14],
];

pub fn keccak_f1600(state: &mut [u64]) {
    for rc in RC {
        let mut c = [0u64; 5];
        for x in 0..5 {
            c[x] = state[x] ^ state[x + 5] ^ state[x + 10]
                ^ state[x + 15] ^ state[x + 20];
        }
        let mut d = [0u64; 5];
        for x in 0..5 {
            d[x] = c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1);
        }
        for y in 0..5 {
            for x in 0..5 {
                state[x + 5 * y] ^= d[x];
            }
        }
        let mut b = [0u64; 25];
        for y in 0..5 {
            for x in 0..5 {
                b[y + 5 * ((2 * x + 3 * y) % 5)] =
                    state[x + 5 * y].rotate_left(RHO[x][y]);
            }
        }
        for y in 0..5 {
            for x in 0..5 {
                state[x + 5 * y] =
                    b[x + 5 * y] ^ ((!b[(x + 1) % 5 + 5 * y]) & b[(x + 2) % 5 + 5 * y]);
            }
        }
        state[0] ^= rc;
    }
}

fn xor_byte(state: &mut [u64], offset: usize, byte: u8) {
    state[offset >> 3] ^= (byte as u64) << (8 * (offset & 7));
}

fn get_byte(state: &[u64], offset: usize) -> u8 {
    (state[offset >> 3] >> (8 * (offset & 7))) as u8
}

pub fn shake256_inc_init(state: &mut [u64; 26]) {
    state.fill(0);
}

pub fn shake256_inc_absorb(state: &mut [u64; 26], mut input: &[u8]) {
    while input.len() + state[25] as usize >= SHAKE256_RATE {
        let take = SHAKE256_RATE - state[25] as usize;
        let state_offset = state[25] as usize;
        for (i, byte) in input[..take].iter().enumerate() {
            xor_byte(state, state_offset + i, *byte);
        }
        input = &input[take..];
        state[25] = 0;
        keccak_f1600(state);
    }
    let offset = state[25] as usize;
    for (i, byte) in input.iter().enumerate() {
        xor_byte(state, offset + i, *byte);
    }
    state[25] += input.len() as u64;
}

pub fn shake256_inc_finalize(state: &mut [u64; 26]) {
    let state_offset = state[25] as usize;
    xor_byte(state, state_offset, 0x1f);
    xor_byte(state, SHAKE256_RATE - 1, 0x80);
    state[25] = 0;
}

pub fn shake256_inc_squeeze(out: &mut [u8], state: &mut [u64; 26]) {
    let mut offset = 0;
    while offset < out.len() && state[25] > 0 {
        out[offset] = get_byte(state, SHAKE256_RATE - state[25] as usize);
        state[25] -= 1;
        offset += 1;
    }
    while offset < out.len() {
        keccak_f1600(state);
        let take = (out.len() - offset).min(SHAKE256_RATE);
        for i in 0..take {
            out[offset + i] = get_byte(state, i);
        }
        state[25] = (SHAKE256_RATE - take) as u64;
        offset += take;
    }
}

pub fn shake256_absorb(state: &mut [u64; 25], input: &[u8]) {
    state.fill(0);
    let mut offset = 0;
    while input.len() - offset >= SHAKE256_RATE {
        for i in 0..SHAKE256_RATE {
            xor_byte(state, i, input[offset + i]);
        }
        keccak_f1600(state);
        offset += SHAKE256_RATE;
    }
    for (i, byte) in input[offset..].iter().enumerate() {
        xor_byte(state, i, *byte);
    }
    xor_byte(state, input.len() - offset, 0x1f);
    xor_byte(state, SHAKE256_RATE - 1, 0x80);
}

pub fn shake256_squeezeblocks(out: &mut [u8], state: &mut [u64; 25]) {
    for block in out.chunks_exact_mut(SHAKE256_RATE) {
        keccak_f1600(state);
        for (i, byte) in block.iter_mut().enumerate() {
            *byte = get_byte(state, i);
        }
    }
}

pub fn shake256(out: &mut [u8], input: &[u8]) {
    let mut state = [0u64; 25];
    shake256_absorb(&mut state, input);
    let full = out.len() / SHAKE256_RATE * SHAKE256_RATE;
    shake256_squeezeblocks(&mut out[..full], &mut state);
    if full < out.len() {
        let mut block = [0u8; SHAKE256_RATE];
        shake256_squeezeblocks(&mut block, &mut state);
        let remainder = out.len() - full;
        out[full..].copy_from_slice(&block[..remainder]);
    }
}
