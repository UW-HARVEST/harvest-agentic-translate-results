use std::sync::Mutex;

static DRBG_CTX: Mutex<Option<Aes256CtrDrbg>> = Mutex::new(None);

struct Aes256CtrDrbg {
    key: [u8; 32],
    v: [u8; 16],
    reseed_counter: i32,
}

// AES-256 implementation (FIPS 197)
static SBOX: [u8; 256] = [
    0x63,0x7c,0x77,0x7b,0xf2,0x6b,0x6f,0xc5,0x30,0x01,0x67,0x2b,0xfe,0xd7,0xab,0x76,
    0xca,0x82,0xc9,0x7d,0xfa,0x59,0x47,0xf0,0xad,0xd4,0xa2,0xaf,0x9c,0xa4,0x72,0xc0,
    0xb7,0xfd,0x93,0x26,0x36,0x3f,0xf7,0xcc,0x34,0xa5,0xe5,0xf1,0x71,0xd8,0x31,0x15,
    0x04,0xc7,0x23,0xc3,0x18,0x96,0x05,0x9a,0x07,0x12,0x80,0xe2,0xeb,0x27,0xb2,0x75,
    0x09,0x83,0x2c,0x1a,0x1b,0x6e,0x5a,0xa0,0x52,0x3b,0xd6,0xb3,0x29,0xe3,0x2f,0x84,
    0x53,0xd1,0x00,0xed,0x20,0xfc,0xb1,0x5b,0x6a,0xcb,0xbe,0x39,0x4a,0x4c,0x58,0xcf,
    0xd0,0xef,0xaa,0xfb,0x43,0x4d,0x33,0x85,0x45,0xf9,0x02,0x7f,0x50,0x3c,0x9f,0xa8,
    0x51,0xa3,0x40,0x8f,0x92,0x9d,0x38,0xf5,0xbc,0xb6,0xda,0x21,0x10,0xff,0xf3,0xd2,
    0xcd,0x0c,0x13,0xec,0x5f,0x97,0x44,0x17,0xc4,0xa7,0x7e,0x3d,0x64,0x5d,0x19,0x73,
    0x60,0x81,0x4f,0xdc,0x22,0x2a,0x90,0x88,0x46,0xee,0xb8,0x14,0xde,0x5e,0x0b,0xdb,
    0xe0,0x32,0x3a,0x0a,0x49,0x06,0x24,0x5c,0xc2,0xd3,0xac,0x62,0x91,0x95,0xe4,0x79,
    0xe7,0xc8,0x37,0x6d,0x8d,0xd5,0x4e,0xa9,0x6c,0x56,0xf4,0xea,0x65,0x7a,0xae,0x08,
    0xba,0x78,0x25,0x2e,0x1c,0xa6,0xb4,0xc6,0xe8,0xdd,0x74,0x1f,0x4b,0xbd,0x8b,0x8a,
    0x70,0x3e,0xb5,0x66,0x48,0x03,0xf6,0x0e,0x61,0x35,0x57,0xb9,0x86,0xc1,0x1d,0x9e,
    0xe1,0xf8,0x98,0x11,0x69,0xd9,0x8e,0x94,0x9b,0x1e,0x87,0xe9,0xce,0x55,0x28,0xdf,
    0x8c,0xa1,0x89,0x0d,0xbf,0xe6,0x42,0x68,0x41,0x99,0x2d,0x0f,0xb0,0x54,0xbb,0x16,
];

static RCON: [u8; 10] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36];

fn sub_word(w: u32) -> u32 {
    let b0 = SBOX[(w >> 24) as usize] as u32;
    let b1 = SBOX[((w >> 16) & 0xff) as usize] as u32;
    let b2 = SBOX[((w >> 8) & 0xff) as usize] as u32;
    let b3 = SBOX[(w & 0xff) as usize] as u32;
    (b0 << 24) | (b1 << 16) | (b2 << 8) | b3
}

fn rot_word(w: u32) -> u32 {
    (w << 8) | (w >> 24)
}

fn key_expansion(key: &[u8; 32]) -> [u32; 60] {
    let mut w = [0u32; 60];
    for i in 0..8 {
        w[i] = u32::from_be_bytes([key[4*i], key[4*i+1], key[4*i+2], key[4*i+3]]);
    }
    for i in 8..60 {
        let mut temp = w[i - 1];
        if i % 8 == 0 {
            temp = sub_word(rot_word(temp)) ^ ((RCON[i / 8 - 1] as u32) << 24);
        } else if i % 8 == 4 {
            temp = sub_word(temp);
        }
        w[i] = w[i - 8] ^ temp;
    }
    w
}

fn xtime(x: u8) -> u8 {
    let r = (x as u16) << 1;
    (r ^ (if r & 0x100 != 0 { 0x11b } else { 0 })) as u8
}

fn mul(a: u8, b: u8) -> u8 {
    let mut p: u8 = 0;
    let mut a = a;
    let mut b = b;
    for _ in 0..8 {
        if b & 1 != 0 { p ^= a; }
        let hi = a & 0x80;
        a = (a << 1) ^ if hi != 0 { 0x1b } else { 0 };
        b >>= 1;
    }
    p
}

fn aes256_encrypt_block(key: &[u8; 32], input: &[u8; 16], output: &mut [u8; 16]) {
    let rk = key_expansion(key);
    let mut state = [[0u8; 4]; 4];
    for c in 0..4 {
        for r in 0..4 {
            state[r][c] = input[c * 4 + r];
        }
    }
    // AddRoundKey 0
    for c in 0..4 {
        let k = rk[c].to_be_bytes();
        for r in 0..4 { state[r][c] ^= k[r]; }
    }
    for round in 1..14 {
        // SubBytes
        for r in 0..4 { for c in 0..4 { state[r][c] = SBOX[state[r][c] as usize]; } }
        // ShiftRows
        let tmp = state[1][0]; state[1][0] = state[1][1]; state[1][1] = state[1][2]; state[1][2] = state[1][3]; state[1][3] = tmp;
        let tmp0 = state[2][0]; let tmp1 = state[2][1]; state[2][0] = state[2][2]; state[2][1] = state[2][3]; state[2][2] = tmp0; state[2][3] = tmp1;
        let tmp = state[3][3]; state[3][3] = state[3][2]; state[3][2] = state[3][1]; state[3][1] = state[3][0]; state[3][0] = tmp;
        // MixColumns
        for c in 0..4 {
            let s0 = state[0][c]; let s1 = state[1][c]; let s2 = state[2][c]; let s3 = state[3][c];
            state[0][c] = mul(2, s0) ^ mul(3, s1) ^ s2 ^ s3;
            state[1][c] = s0 ^ mul(2, s1) ^ mul(3, s2) ^ s3;
            state[2][c] = s0 ^ s1 ^ mul(2, s2) ^ mul(3, s3);
            state[3][c] = mul(3, s0) ^ s1 ^ s2 ^ mul(2, s3);
        }
        // AddRoundKey
        for c in 0..4 {
            let k = rk[round * 4 + c].to_be_bytes();
            for r in 0..4 { state[r][c] ^= k[r]; }
        }
    }
    // Final round (no MixColumns)
    for r in 0..4 { for c in 0..4 { state[r][c] = SBOX[state[r][c] as usize]; } }
    let tmp = state[1][0]; state[1][0] = state[1][1]; state[1][1] = state[1][2]; state[1][2] = state[1][3]; state[1][3] = tmp;
    let tmp0 = state[2][0]; let tmp1 = state[2][1]; state[2][0] = state[2][2]; state[2][1] = state[2][3]; state[2][2] = tmp0; state[2][3] = tmp1;
    let tmp = state[3][3]; state[3][3] = state[3][2]; state[3][2] = state[3][1]; state[3][1] = state[3][0]; state[3][0] = tmp;
    for c in 0..4 {
        let k = rk[56 + c].to_be_bytes();
        for r in 0..4 { state[r][c] ^= k[r]; }
    }
    for c in 0..4 {
        for r in 0..4 {
            output[c * 4 + r] = state[r][c];
        }
    }
}

fn aes256_ecb(key: &[u8], ctr: &[u8], buffer: &mut [u8]) {
    let mut k = [0u8; 32];
    let mut inp = [0u8; 16];
    let mut out = [0u8; 16];
    k.copy_from_slice(&key[..32]);
    inp.copy_from_slice(&ctr[..16]);
    aes256_encrypt_block(&k, &inp, &mut out);
    buffer[..16].copy_from_slice(&out);
}

pub fn aes256_ctr_drbg_update(provided_data: Option<&[u8]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
    let mut temp = [0u8; 48];
    for i in 0..3 {
        for j in (0..16).rev() {
            if v[j] == 0xff { v[j] = 0x00; } else { v[j] += 1; break; }
        }
        aes256_ecb(key, v, &mut temp[16 * i..16 * i + 16]);
    }
    if let Some(data) = provided_data {
        for i in 0..48 { temp[i] ^= data[i]; }
    }
    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..48]);
}

pub fn randombytes_init(entropy_input: &[u8], personalization_string: Option<&[u8]>) {
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(&entropy_input[..48]);
    if let Some(ps) = personalization_string {
        for i in 0..48 { seed_material[i] ^= ps[i]; }
    }
    let mut key = [0u8; 32];
    let mut v = [0u8; 16];
    aes256_ctr_drbg_update(Some(&seed_material), &mut key, &mut v);

    let mut ctx = DRBG_CTX.lock().unwrap();
    *ctx = Some(Aes256CtrDrbg { key, v, reseed_counter: 1 });
}

pub fn randombytes_rng(x: &mut [u8], mut xlen: u64) -> i32 {
    let mut ctx = DRBG_CTX.lock().unwrap();
    let drbg = ctx.as_mut().unwrap();
    let mut block = [0u8; 16];
    let mut i: usize = 0;

    while xlen > 0 {
        for j in (0..16).rev() {
            if drbg.v[j] == 0xff { drbg.v[j] = 0x00; } else { drbg.v[j] += 1; break; }
        }
        aes256_ecb(&drbg.key, &drbg.v, &mut block);
        if xlen > 15 {
            x[i..i + 16].copy_from_slice(&block);
            i += 16;
            xlen -= 16;
        } else {
            x[i..i + xlen as usize].copy_from_slice(&block[..xlen as usize]);
            xlen = 0;
        }
    }
    aes256_ctr_drbg_update(None, &mut drbg.key, &mut drbg.v);
    drbg.reseed_counter += 1;
    0
}
