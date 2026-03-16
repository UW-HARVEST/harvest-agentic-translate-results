use aes::cipher::{BlockEncrypt, KeyInit, generic_array::GenericArray};
use aes::Aes256;
use std::sync::Mutex;

const RNG_SUCCESS: i32 = 0;
const RNG_BAD_MAXLEN: i32 = -1;
const RNG_BAD_OUTBUF: i32 = -2;
const RNG_BAD_REQ_LEN: i32 = -3;

#[repr(C)]
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

static DRBG_CTX: Mutex<Aes256CtrDrbgStruct> = Mutex::new(Aes256CtrDrbgStruct {
    key: [0u8; 32],
    v: [0u8; 16],
    reseed_counter: 0,
});

fn aes256_ecb(key: &[u8], ctr: &[u8], buffer: &mut [u8]) {
    let cipher = Aes256::new(GenericArray::from_slice(key));
    let mut block = *GenericArray::from_slice(&ctr[..16]);
    cipher.encrypt_block(&mut block);
    buffer[..16].copy_from_slice(&block);
}

pub fn aes256_ctr_drbg_update(provided_data: Option<&[u8]>, key: &mut [u8], v: &mut [u8]) {
    let mut temp = [0u8; 48];

    for i in 0..3 {
        for j in (0..16).rev() {
            if v[j] == 0xff {
                v[j] = 0x00;
            } else {
                v[j] += 1;
                break;
            }
        }
        aes256_ecb(key, v, &mut temp[16 * i..16 * i + 16]);
    }

    if let Some(data) = provided_data {
        for i in 0..48 {
            temp[i] ^= data[i];
        }
    }

    key[..32].copy_from_slice(&temp[..32]);
    v[..16].copy_from_slice(&temp[32..48]);
}

pub fn seedexpander_init(ctx: &mut AesXofStruct, seed: &[u8], diversifier: &[u8], maxlen: u64) -> i32 {
    if maxlen >= 0x100000000 {
        return RNG_BAD_MAXLEN;
    }

    ctx.length_remaining = maxlen;
    ctx.key[..32].copy_from_slice(&seed[..32]);
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

        let avail = 16 - ctx.buffer_pos;
        x[offset..offset + avail].copy_from_slice(&ctx.buffer[ctx.buffer_pos..16]);
        xlen -= avail;
        offset += avail;

        aes256_ecb(&ctx.key, &ctx.ctr, &mut ctx.buffer);
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

pub fn randombytes_init_internal(entropy_input: &[u8], personalization_string: Option<&[u8]>, drbg: &mut Aes256CtrDrbgStruct) {
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(&entropy_input[..48]);

    if let Some(ps) = personalization_string {
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }

    drbg.key.fill(0);
    drbg.v.fill(0);
    aes256_ctr_drbg_update(Some(&seed_material), &mut drbg.key, &mut drbg.v);
    drbg.reseed_counter = 1;
}

pub fn randombytes_internal(x: &mut [u8], mut xlen: usize, drbg: &mut Aes256CtrDrbgStruct) -> i32 {
    let mut block = [0u8; 16];
    let mut i = 0usize;

    while xlen > 0 {
        for j in (0..16).rev() {
            if drbg.v[j] == 0xff {
                drbg.v[j] = 0x00;
            } else {
                drbg.v[j] += 1;
                break;
            }
        }
        aes256_ecb(&drbg.key, &drbg.v, &mut block);
        if xlen > 15 {
            x[i..i + 16].copy_from_slice(&block);
            i += 16;
            xlen -= 16;
        } else {
            x[i..i + xlen].copy_from_slice(&block[..xlen]);
            xlen = 0;
        }
    }
    aes256_ctr_drbg_update(None, &mut drbg.key, &mut drbg.v);
    drbg.reseed_counter += 1;

    RNG_SUCCESS
}

pub fn randombytes_init_global(entropy_input: &[u8], personalization_string: Option<&[u8]>) {
    let mut drbg = DRBG_CTX.lock().unwrap();
    randombytes_init_internal(entropy_input, personalization_string, &mut drbg);
}

pub fn randombytes_global(x: &mut [u8], xlen: usize) -> i32 {
    let mut drbg = DRBG_CTX.lock().unwrap();
    randombytes_internal(x, xlen, &mut drbg)
}
