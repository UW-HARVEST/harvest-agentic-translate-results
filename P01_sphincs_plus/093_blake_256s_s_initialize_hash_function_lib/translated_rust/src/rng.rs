// RNG using AES-256-CTR-DRBG (NIST deterministic RNG)
// This requires OpenSSL. For the Rust translation we implement the same
// AES256_ECB + CTR_DRBG logic using the openssl crate would add a dependency.
// Since this is a cdylib that must be byte-identical, we implement the DRBG
// in pure Rust using a minimal AES-256-ECB.
// However, the C code uses OpenSSL's EVP_aes_256_ecb. For a faithful translation
// we use a simple software AES-256 implementation.

// For simplicity and correctness, we use a static mutable global like the C code.

use std::sync::Mutex;
use std::sync::LazyLock;

pub const RNG_SUCCESS: i32 = 0;
pub const RNG_BAD_MAXLEN: i32 = -1;
pub const RNG_BAD_OUTBUF: i32 = -2;
pub const RNG_BAD_REQ_LEN: i32 = -3;

pub struct AesXofStruct {
    pub buffer: [u8; 16],
    pub buffer_pos: usize,
    pub length_remaining: u64,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

pub struct Aes256CtrDrbgStruct {
    pub key: [u8; 32],
    pub v: [u8; 16],
    pub reseed_counter: i32,
}

static DRBG_CTX: LazyLock<Mutex<Aes256CtrDrbgStruct>> = LazyLock::new(|| {
    Mutex::new(Aes256CtrDrbgStruct {
        key: [0u8; 32],
        v: [0u8; 16],
        reseed_counter: 0,
    })
});

// Minimal AES-256 ECB implementation
mod aes256 {
    const SBOX: [u8; 256] = [
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

    const RCON: [u8; 10] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36];

    fn sub_word(w: u32) -> u32 {
        let b = w.to_be_bytes();
        u32::from_be_bytes([SBOX[b[0] as usize], SBOX[b[1] as usize], SBOX[b[2] as usize], SBOX[b[3] as usize]])
    }

    fn key_expansion(key: &[u8; 32]) -> [[u8; 16]; 15] {
        let mut w = [0u32; 60];
        for i in 0..8 {
            w[i] = u32::from_be_bytes([key[4*i], key[4*i+1], key[4*i+2], key[4*i+3]]);
        }
        for i in 8..60 {
            let mut temp = w[i - 1];
            if i % 8 == 0 {
                temp = sub_word(temp.rotate_left(8)) ^ ((RCON[i / 8 - 1] as u32) << 24);
            } else if i % 8 == 4 {
                temp = sub_word(temp);
            }
            w[i] = w[i - 8] ^ temp;
        }
        let mut rk = [[0u8; 16]; 15];
        for r in 0..15 {
            for j in 0..4 {
                let bytes = w[r * 4 + j].to_be_bytes();
                rk[r][j * 4..j * 4 + 4].copy_from_slice(&bytes);
            }
        }
        rk
    }

    fn add_round_key(state: &mut [u8; 16], rk: &[u8; 16]) {
        for i in 0..16 { state[i] ^= rk[i]; }
    }

    fn sub_bytes(state: &mut [u8; 16]) {
        for i in 0..16 { state[i] = SBOX[state[i] as usize]; }
    }

    fn shift_rows(state: &mut [u8; 16]) {
        let mut tmp = *state;
        // Row 0: no shift
        // Row 1: shift left 1
        tmp[1] = state[5]; tmp[5] = state[9]; tmp[9] = state[13]; tmp[13] = state[1];
        // Row 2: shift left 2
        tmp[2] = state[10]; tmp[6] = state[14]; tmp[10] = state[2]; tmp[14] = state[6];
        // Row 3: shift left 3
        tmp[3] = state[15]; tmp[7] = state[3]; tmp[11] = state[7]; tmp[15] = state[11];
        *state = tmp;
    }

    fn xtime(a: u8) -> u8 {
        let r = (a as u16) << 1;
        (r ^ (if r & 0x100 != 0 { 0x11b } else { 0 })) as u8
    }

    fn mix_columns(state: &mut [u8; 16]) {
        for i in 0..4 {
            let c = i * 4;
            let a = [state[c], state[c+1], state[c+2], state[c+3]];
            let h = [xtime(a[0]), xtime(a[1]), xtime(a[2]), xtime(a[3])];
            state[c]   = h[0] ^ a[1] ^ h[1] ^ a[2] ^ a[3];
            state[c+1] = a[0] ^ h[1] ^ a[2] ^ h[2] ^ a[3];
            state[c+2] = a[0] ^ a[1] ^ h[2] ^ a[3] ^ h[3];
            state[c+3] = a[0] ^ h[0] ^ a[1] ^ a[2] ^ h[3];
        }
    }

    pub fn aes256_ecb(key: &[u8; 32], input: &[u8; 16], output: &mut [u8; 16]) {
        let rk = key_expansion(key);
        let mut state = *input;
        add_round_key(&mut state, &rk[0]);
        for r in 1..14 {
            sub_bytes(&mut state);
            shift_rows(&mut state);
            mix_columns(&mut state);
            add_round_key(&mut state, &rk[r]);
        }
        sub_bytes(&mut state);
        shift_rows(&mut state);
        add_round_key(&mut state, &rk[14]);
        *output = state;
    }
}

fn aes256_ecb(key: &[u8], ctr: &[u8], buffer: &mut [u8]) {
    let k: &[u8; 32] = key[..32].try_into().unwrap();
    let c: &[u8; 16] = ctr[..16].try_into().unwrap();
    let b: &mut [u8; 16] = (&mut buffer[..16]).try_into().unwrap();
    aes256::aes256_ecb(k, c, b);
}

pub fn seedexpander_init(
    ctx: &mut AesXofStruct,
    seed: &[u8],
    diversifier: &[u8],
    maxlen: u64,
) -> i32 {
    if maxlen >= 0x100000000 {
        return RNG_BAD_MAXLEN;
    }
    ctx.length_remaining = maxlen;
    ctx.key.copy_from_slice(&seed[..32]);
    ctx.ctr[..8].copy_from_slice(&diversifier[..8]);
    let mut ml = maxlen;
    ctx.ctr[11] = (ml % 256) as u8; ml >>= 8;
    ctx.ctr[10] = (ml % 256) as u8; ml >>= 8;
    ctx.ctr[9] = (ml % 256) as u8; ml >>= 8;
    ctx.ctr[8] = (ml % 256) as u8;
    ctx.ctr[12..16].fill(0);
    ctx.buffer_pos = 16;
    ctx.buffer.fill(0);
    RNG_SUCCESS
}

pub fn seedexpander(ctx: &mut AesXofStruct, x: &mut [u8], mut xlen: usize) -> i32 {
    if x.is_empty() {
        return RNG_BAD_OUTBUF;
    }
    if xlen as u64 >= ctx.length_remaining {
        return RNG_BAD_REQ_LEN;
    }
    ctx.length_remaining -= xlen as u64;
    let mut offset = 0usize;
    while xlen > 0 {
        if xlen <= 16 - ctx.buffer_pos {
            x[offset..offset + xlen].copy_from_slice(&ctx.buffer[ctx.buffer_pos..ctx.buffer_pos + xlen]);
            ctx.buffer_pos += xlen;
            return RNG_SUCCESS;
        }
        let take = 16 - ctx.buffer_pos;
        x[offset..offset + take].copy_from_slice(&ctx.buffer[ctx.buffer_pos..16]);
        xlen -= take;
        offset += take;
        aes256_ecb(&ctx.key, &ctx.ctr.clone(), &mut ctx.buffer);
        ctx.buffer_pos = 0;
        for i in (12..=15).rev() {
            if ctx.ctr[i] == 0xff {
                ctx.ctr[i] = 0x00;
            } else {
                ctx.ctr[i] += 1;
                break;
            }
        }
    }
    RNG_SUCCESS
}

pub fn aes256_ctr_drbg_update(provided_data: Option<&[u8]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
    let mut temp = [0u8; 48];
    for i in 0..3 {
        for j in (0..=15).rev() {
            if v[j] == 0xff {
                v[j] = 0x00;
            } else {
                v[j] += 1;
                break;
            }
        }
        aes256_ecb(key, v, &mut temp[16 * i..]);
    }
    if let Some(pd) = provided_data {
        for i in 0..48 {
            temp[i] ^= pd[i];
        }
    }
    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..48]);
}

pub fn randombytes_init(entropy_input: &[u8], personalization_string: Option<&[u8]>) {
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(&entropy_input[..48]);
    if let Some(ps) = personalization_string {
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }
    let mut ctx = DRBG_CTX.lock().unwrap();
    ctx.key.fill(0);
    ctx.v.fill(0);
    let mut key = ctx.key;
    let mut v = ctx.v;
    aes256_ctr_drbg_update(Some(&seed_material), &mut key, &mut v);
    ctx.key = key;
    ctx.v = v;
    ctx.reseed_counter = 1;
}

pub fn randombytes(x: &mut [u8], mut xlen: u64) -> i32 {
    let mut ctx = DRBG_CTX.lock().unwrap();
    let mut block = [0u8; 16];
    let mut i: usize = 0;
    while xlen > 0 {
        for j in (0..=15).rev() {
            if ctx.v[j] == 0xff {
                ctx.v[j] = 0x00;
            } else {
                ctx.v[j] += 1;
                break;
            }
        }
        aes256_ecb(&ctx.key, &ctx.v.clone(), &mut block);
        if xlen > 15 {
            x[i..i + 16].copy_from_slice(&block);
            i += 16;
            xlen -= 16;
        } else {
            x[i..i + xlen as usize].copy_from_slice(&block[..xlen as usize]);
            xlen = 0;
        }
    }
    let mut key = ctx.key;
    let mut v = ctx.v;
    aes256_ctr_drbg_update(None, &mut key, &mut v);
    ctx.key = key;
    ctx.v = v;
    ctx.reseed_counter += 1;
    RNG_SUCCESS
}
