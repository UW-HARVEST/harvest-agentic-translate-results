const ROUND_CONSTANTS: [u64; 24] = [
    0x0000_0000_0000_0001,
    0x0000_0000_0000_8082,
    0x8000_0000_0000_808a,
    0x8000_0000_8000_8000,
    0x0000_0000_0000_808b,
    0x0000_0000_8000_0001,
    0x8000_0000_8000_8081,
    0x8000_0000_0000_8009,
    0x0000_0000_0000_008a,
    0x0000_0000_0000_0088,
    0x0000_0000_8000_8009,
    0x0000_0000_8000_000a,
    0x0000_0000_8000_808b,
    0x8000_0000_0000_008b,
    0x8000_0000_0000_8089,
    0x8000_0000_0000_8003,
    0x8000_0000_0000_8002,
    0x8000_0000_0000_0080,
    0x0000_0000_0000_800a,
    0x8000_0000_8000_000a,
    0x8000_0000_8000_8081,
    0x8000_0000_0000_8080,
    0x0000_0000_8000_0001,
    0x8000_0000_8000_8008,
];

const ROTATIONS: [u32; 24] = [
    1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14, 27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20,
    44,
];

const PERMUTATION: [usize; 24] = [
    10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4, 15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1,
];

pub fn permute(state: &mut [u64; 25]) {
    for round_constant in ROUND_CONSTANTS {
        let mut columns = [0u64; 5];
        for x in 0..5 {
            columns[x] = state[x] ^ state[x + 5] ^ state[x + 10] ^ state[x + 15] ^ state[x + 20];
        }
        for x in 0..5 {
            let delta = columns[(x + 4) % 5] ^ columns[(x + 1) % 5].rotate_left(1);
            for y in 0..5 {
                state[x + 5 * y] ^= delta;
            }
        }

        let mut current = state[1];
        for i in 0..24 {
            let target = PERMUTATION[i];
            let previous = state[target];
            state[target] = current.rotate_left(ROTATIONS[i]);
            current = previous;
        }

        for y in 0..5 {
            let row = [
                state[5 * y],
                state[5 * y + 1],
                state[5 * y + 2],
                state[5 * y + 3],
                state[5 * y + 4],
            ];
            for x in 0..5 {
                state[5 * y + x] = row[x] ^ ((!row[(x + 1) % 5]) & row[(x + 2) % 5]);
            }
        }
        state[0] ^= round_constant;
    }
}

fn xor_byte(state: &mut [u64], offset: usize, byte: u8) {
    state[offset >> 3] ^= (byte as u64) << (8 * (offset & 7));
}

fn get_byte(state: &[u64], offset: usize) -> u8 {
    (state[offset >> 3] >> (8 * (offset & 7))) as u8
}

pub fn absorb(state: &mut [u64; 25], rate: usize, mut input: &[u8], domain: u8) {
    state.fill(0);
    while input.len() >= rate {
        for (i, chunk) in input[..rate].chunks_exact(8).enumerate() {
            state[i] ^= u64::from_le_bytes(chunk.try_into().unwrap());
        }
        permute(state);
        input = &input[rate..];
    }
    for (i, byte) in input.iter().copied().enumerate() {
        xor_byte(state, i, byte);
    }
    xor_byte(state, input.len(), domain);
    xor_byte(state, rate - 1, 0x80);
}

pub fn squeeze_blocks(output: &mut [u8], blocks: usize, state: &mut [u64; 25], rate: usize) {
    for block in 0..blocks {
        permute(state);
        let target = &mut output[block * rate..(block + 1) * rate];
        for (i, lane) in target.chunks_exact_mut(8).enumerate() {
            lane.copy_from_slice(&state[i].to_le_bytes());
        }
    }
}

pub fn inc_init(state: &mut [u64; 26]) {
    state.fill(0);
}

pub fn inc_absorb(state: &mut [u64; 26], rate: usize, mut input: &[u8]) {
    while input.len() + state[25] as usize >= rate {
        let pending = state[25] as usize;
        let take = rate - pending;
        for (i, byte) in input[..take].iter().copied().enumerate() {
            xor_byte(state, pending + i, byte);
        }
        input = &input[take..];
        state[25] = 0;
        let lanes: &mut [u64; 25] = (&mut state[..25]).try_into().unwrap();
        permute(lanes);
    }
    let pending = state[25] as usize;
    for (i, byte) in input.iter().copied().enumerate() {
        xor_byte(state, pending + i, byte);
    }
    state[25] += input.len() as u64;
}

pub fn inc_finalize(state: &mut [u64; 26], rate: usize, domain: u8) {
    let pending = state[25] as usize;
    xor_byte(state, pending, domain);
    xor_byte(state, rate - 1, 0x80);
    state[25] = 0;
}

pub fn inc_squeeze(output: &mut [u8], state: &mut [u64; 26], rate: usize) {
    let mut offset = 0usize;
    let available = state[25] as usize;
    let take = output.len().min(available);
    for i in 0..take {
        output[i] = get_byte(state, rate - available + i);
    }
    offset += take;
    state[25] -= take as u64;

    while offset < output.len() {
        let lanes: &mut [u64; 25] = (&mut state[..25]).try_into().unwrap();
        permute(lanes);
        let take = (output.len() - offset).min(rate);
        for i in 0..take {
            output[offset + i] = get_byte(state, i);
        }
        offset += take;
        state[25] = (rate - take) as u64;
    }
}

