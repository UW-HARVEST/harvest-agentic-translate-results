use aes::Aes256;
use aes::cipher::{BlockEncrypt, KeyInit, generic_array::GenericArray};

const RNG_SUCCESS: i32 = 0;
const RNG_BAD_MAXLEN: i32 = -1;
const RNG_BAD_OUTBUF: i32 = -2;
const RNG_BAD_REQ_LEN: i32 = -3;

pub struct AesXofStruct {
    buffer: [u8; 16],
    buffer_pos: usize,
    length_remaining: u64,
    key: [u8; 32],
    ctr: [u8; 16],
}

struct Aes256CtrDrbgStruct {
    key: [u8; 32],
    v: [u8; 16],
    reseed_counter: i32,
}

static mut DRBG_CTX: Aes256CtrDrbgStruct = Aes256CtrDrbgStruct {
    key: [0u8; 32],
    v: [0u8; 16],
    reseed_counter: 0,
};

pub fn aes256_ecb(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    let cipher = Aes256::new(GenericArray::from_slice(key));
    let mut block = GenericArray::clone_from_slice(ctr);
    cipher.encrypt_block(&mut block);
    buffer.copy_from_slice(&block);
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
    ctx.ctr[11] = (ml % 256) as u8;
    ml >>= 8;
    ctx.ctr[10] = (ml % 256) as u8;
    ml >>= 8;
    ctx.ctr[9] = (ml % 256) as u8;
    ml >>= 8;
    ctx.ctr[8] = (ml % 256) as u8;

    ctx.ctr[12..16].fill(0x00);
    ctx.buffer_pos = 16;
    ctx.buffer.fill(0x00);

    RNG_SUCCESS
}

pub fn seedexpander(ctx: &mut AesXofStruct, x: &mut [u8], mut xlen: u64) -> i32 {
    if xlen >= ctx.length_remaining {
        return RNG_BAD_REQ_LEN;
    }
    ctx.length_remaining -= xlen;

    let mut offset: usize = 0;

    while xlen > 0 {
        let remaining_in_buf = 16 - ctx.buffer_pos;
        if xlen <= remaining_in_buf as u64 {
            let xlen_usize = xlen as usize;
            x[offset..offset + xlen_usize]
                .copy_from_slice(&ctx.buffer[ctx.buffer_pos..ctx.buffer_pos + xlen_usize]);
            ctx.buffer_pos += xlen_usize;
            return RNG_SUCCESS;
        }
        x[offset..offset + remaining_in_buf]
            .copy_from_slice(&ctx.buffer[ctx.buffer_pos..16]);
        xlen -= remaining_in_buf as u64;
        offset += remaining_in_buf;

        let mut buf = [0u8; 16];
        let ctr: [u8; 16] = ctx.ctr;
        let key: [u8; 32] = ctx.key;
        aes256_ecb(&key, &ctr, &mut buf);
        ctx.buffer = buf;
        ctx.buffer_pos = 0;

        // Increment counter bytes 12..15 (big-endian increment of last 4 bytes)
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
        // Increment V
        for j in (0..=15).rev() {
            if v[j] == 0xff {
                v[j] = 0x00;
            } else {
                v[j] += 1;
                break;
            }
        }
        let mut block = [0u8; 16];
        aes256_ecb(key, v, &mut block);
        temp[16 * i..16 * i + 16].copy_from_slice(&block);
    }

    if let Some(data) = provided_data {
        for i in 0..48 {
            temp[i] ^= data[i];
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

    unsafe {
        DRBG_CTX.key.fill(0x00);
        DRBG_CTX.v.fill(0x00);
        aes256_ctr_drbg_update(Some(&seed_material), &mut DRBG_CTX.key, &mut DRBG_CTX.v);
        DRBG_CTX.reseed_counter = 1;
    }
}

pub fn randombytes(x: &mut [u8], mut xlen: u64) -> i32 {
    let mut block = [0u8; 16];
    let mut i: usize = 0;

    unsafe {
        while xlen > 0 {
            // Increment V
            for j in (0..=15).rev() {
                if DRBG_CTX.v[j] == 0xff {
                    DRBG_CTX.v[j] = 0x00;
                } else {
                    DRBG_CTX.v[j] += 1;
                    break;
                }
            }

            aes256_ecb(&DRBG_CTX.key, &DRBG_CTX.v, &mut block);

            if xlen > 15 {
                x[i..i + 16].copy_from_slice(&block);
                i += 16;
                xlen -= 16;
            } else {
                let len = xlen as usize;
                x[i..i + len].copy_from_slice(&block[..len]);
                xlen = 0;
            }
        }

        aes256_ctr_drbg_update(None, &mut DRBG_CTX.key, &mut DRBG_CTX.v);
        DRBG_CTX.reseed_counter += 1;
    }

    0
}

impl AesXofStruct {
    pub fn new() -> Self {
        AesXofStruct {
            buffer: [0u8; 16],
            buffer_pos: 0,
            length_remaining: 0,
            key: [0u8; 32],
            ctr: [0u8; 16],
        }
    }
}
