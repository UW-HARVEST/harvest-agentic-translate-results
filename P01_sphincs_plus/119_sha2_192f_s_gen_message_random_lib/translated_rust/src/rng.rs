use openssl::symm::{Cipher, Crypter, Mode};
use std::sync::Mutex;

pub struct AesXofStruct {
    pub buffer: [u8; 16],
    pub buffer_pos: usize,
    pub length_remaining: u64,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

struct Aes256CtrDrbgStruct {
    key: [u8; 32],
    v: [u8; 16],
    reseed_counter: i32,
}

static DRBG_CTX: Mutex<Aes256CtrDrbgStruct> = Mutex::new(Aes256CtrDrbgStruct {
    key: [0u8; 32],
    v: [0u8; 16],
    reseed_counter: 0,
});

fn aes256_ecb(key: &[u8], ctr: &[u8], buffer: &mut [u8]) {
    let cipher = Cipher::aes_256_ecb();
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, None).unwrap();
    crypter.pad(false);
    let count = crypter.update(ctr, buffer).unwrap();
    let _ = crypter.finalize(&mut buffer[count..]).unwrap();
}

fn aes256_ctr_drbg_update(provided_data: Option<&[u8]>, key: &mut [u8], v: &mut [u8]) {
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

pub fn randombytes(x: &mut [u8], mut xlen: u64) {
    let mut ctx = DRBG_CTX.lock().unwrap();
    let mut block = [0u8; 16];
    let mut i = 0usize;

    while xlen > 0 {
        for j in (0..16).rev() {
            if ctx.v[j] == 0xff {
                ctx.v[j] = 0x00;
            } else {
                ctx.v[j] += 1;
                break;
            }
        }
        aes256_ecb(&ctx.key, &ctx.v, &mut block);
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
}

pub fn aes256_ctr_drbg_update_export(provided_data: Option<&[u8]>, key: &mut [u8], v: &mut [u8]) {
    aes256_ctr_drbg_update(provided_data, key, v);
}
