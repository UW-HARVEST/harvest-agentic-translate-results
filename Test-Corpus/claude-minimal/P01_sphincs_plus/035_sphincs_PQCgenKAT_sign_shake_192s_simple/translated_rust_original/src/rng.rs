// Deterministic NIST AES-256 CTR DRBG implementation.
// Translated from rng.c using a pure-Rust AES-256 ECB.

pub const RNG_SUCCESS: i32 = 0;

pub struct Aes256CtrDrbgState {
    pub key: [u8; 32],
    pub v: [u8; 16],
    pub reseed_counter: i32,
}

impl Aes256CtrDrbgState {
    pub const fn new() -> Self {
        Aes256CtrDrbgState {
            key: [0u8; 32],
            v: [0u8; 16],
            reseed_counter: 0,
        }
    }
}

// AES-256 implementation
mod aes {
    // S-box
    const SBOX: [u8; 256] = [
        0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab,
        0x76, 0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4,
        0x72, 0xc0, 0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71,
        0xd8, 0x31, 0x15, 0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2,
        0xeb, 0x27, 0xb2, 0x75, 0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6,
        0xb3, 0x29, 0xe3, 0x2f, 0x84, 0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb,
        0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf, 0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45,
        0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8, 0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5,
        0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2, 0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44,
        0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73, 0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a,
        0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb, 0xe0, 0x32, 0x3a, 0x0a, 0x49,
        0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79, 0xe7, 0xc8, 0x37, 0x6d,
        0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08, 0xba, 0x78, 0x25,
        0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a, 0x70, 0x3e,
        0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e, 0xe1,
        0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
        0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb,
        0x16,
    ];

    const RCON: [u8; 11] = [
        0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36,
    ];

    // Number of rounds for AES-256
    const NR: usize = 14;
    // Number of 32-bit words in key
    const NK: usize = 8;
    // Number of columns in state
    const NB: usize = 4;

    fn xtime(x: u8) -> u8 {
        ((x << 1) as u8) ^ (((x >> 7) & 1) * 0x1b)
    }

    fn sub_word(w: u32) -> u32 {
        let b0 = SBOX[(w & 0xff) as usize] as u32;
        let b1 = SBOX[((w >> 8) & 0xff) as usize] as u32;
        let b2 = SBOX[((w >> 16) & 0xff) as usize] as u32;
        let b3 = SBOX[((w >> 24) & 0xff) as usize] as u32;
        b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    }

    fn rot_word(w: u32) -> u32 {
        (w >> 8) | (w << 24)
    }

    pub fn key_expansion(key: &[u8; 32]) -> [u32; NB * (NR + 1)] {
        let mut w = [0u32; NB * (NR + 1)];
        for i in 0..NK {
            w[i] = (key[4 * i] as u32)
                | ((key[4 * i + 1] as u32) << 8)
                | ((key[4 * i + 2] as u32) << 16)
                | ((key[4 * i + 3] as u32) << 24);
        }
        for i in NK..NB * (NR + 1) {
            let mut temp = w[i - 1];
            if i % NK == 0 {
                temp = sub_word(rot_word(temp)) ^ (RCON[i / NK] as u32);
            } else if NK > 6 && i % NK == 4 {
                temp = sub_word(temp);
            }
            w[i] = w[i - NK] ^ temp;
        }
        w
    }

    fn add_round_key(state: &mut [u8; 16], w: &[u32], round: usize) {
        for c in 0..NB {
            let word = w[round * NB + c];
            state[4 * c] ^= (word & 0xff) as u8;
            state[4 * c + 1] ^= ((word >> 8) & 0xff) as u8;
            state[4 * c + 2] ^= ((word >> 16) & 0xff) as u8;
            state[4 * c + 3] ^= ((word >> 24) & 0xff) as u8;
        }
    }

    fn sub_bytes(state: &mut [u8; 16]) {
        for b in state.iter_mut() {
            *b = SBOX[*b as usize];
        }
    }

    fn shift_rows(state: &mut [u8; 16]) {
        // Note: state is column-major: state[r + 4*c] is row r col c
        // Row 1: shift by 1
        let t = state[1];
        state[1] = state[5];
        state[5] = state[9];
        state[9] = state[13];
        state[13] = t;
        // Row 2: shift by 2
        let t1 = state[2];
        let t2 = state[6];
        state[2] = state[10];
        state[6] = state[14];
        state[10] = t1;
        state[14] = t2;
        // Row 3: shift by 3
        let t = state[3];
        state[3] = state[15];
        state[15] = state[11];
        state[11] = state[7];
        state[7] = t;
    }

    fn mix_columns(state: &mut [u8; 16]) {
        for c in 0..NB {
            let s0 = state[4 * c];
            let s1 = state[4 * c + 1];
            let s2 = state[4 * c + 2];
            let s3 = state[4 * c + 3];
            let t = s0 ^ s1 ^ s2 ^ s3;
            state[4 * c] = s0 ^ t ^ xtime(s0 ^ s1);
            state[4 * c + 1] = s1 ^ t ^ xtime(s1 ^ s2);
            state[4 * c + 2] = s2 ^ t ^ xtime(s2 ^ s3);
            state[4 * c + 3] = s3 ^ t ^ xtime(s3 ^ s0);
        }
    }

    pub fn encrypt_block(key_schedule: &[u32], input: &[u8; 16], output: &mut [u8; 16]) {
        let mut state: [u8; 16] = *input;

        add_round_key(&mut state, key_schedule, 0);
        for round in 1..NR {
            sub_bytes(&mut state);
            shift_rows(&mut state);
            mix_columns(&mut state);
            add_round_key(&mut state, key_schedule, round);
        }
        sub_bytes(&mut state);
        shift_rows(&mut state);
        add_round_key(&mut state, key_schedule, NR);

        output.copy_from_slice(&state);
    }
}

fn aes256_ecb(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    let ks = aes::key_expansion(key);
    aes::encrypt_block(&ks, ctr, buffer);
}

fn aes256_ctr_drbg_update(provided_data: Option<&[u8]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
    let mut temp = [0u8; 48];

    for i in 0..3 {
        // Increment V (big-endian 128-bit counter)
        for j in (0..16).rev() {
            if v[j] == 0xff {
                v[j] = 0;
            } else {
                v[j] += 1;
                break;
            }
        }
        let mut block_out = [0u8; 16];
        let v_in = *v;
        aes256_ecb(key, &v_in, &mut block_out);
        temp[16 * i..16 * (i + 1)].copy_from_slice(&block_out);
    }

    if let Some(pd) = provided_data {
        for i in 0..48 {
            temp[i] ^= pd[i];
        }
    }

    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..48]);
}

pub fn randombytes_init(
    state: &mut Aes256CtrDrbgState,
    entropy_input: &[u8],
    personalization_string: Option<&[u8]>,
) {
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(&entropy_input[..48]);
    if let Some(ps) = personalization_string {
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }
    for b in state.key.iter_mut() {
        *b = 0;
    }
    for b in state.v.iter_mut() {
        *b = 0;
    }
    aes256_ctr_drbg_update(Some(&seed_material), &mut state.key, &mut state.v);
    state.reseed_counter = 1;
}

pub fn randombytes(state: &mut Aes256CtrDrbgState, x: &mut [u8]) -> i32 {
    let mut block = [0u8; 16];
    let mut i = 0usize;
    let mut xlen = x.len();

    while xlen > 0 {
        // Increment V
        for j in (0..16).rev() {
            if state.v[j] == 0xff {
                state.v[j] = 0;
            } else {
                state.v[j] += 1;
                break;
            }
        }
        let v_in = state.v;
        aes256_ecb(&state.key, &v_in, &mut block);

        if xlen > 15 {
            x[i..i + 16].copy_from_slice(&block);
            i += 16;
            xlen -= 16;
        } else {
            x[i..i + xlen].copy_from_slice(&block[..xlen]);
            xlen = 0;
        }
    }
    aes256_ctr_drbg_update(None, &mut state.key, &mut state.v);
    state.reseed_counter += 1;
    RNG_SUCCESS
}
