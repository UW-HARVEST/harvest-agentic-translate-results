use openssl::symm::{Cipher, Crypter, Mode};
use std::sync::Mutex;

struct Aes256CtrDrbg {
    key: [u8; 32],
    v: [u8; 16],
    reseed_counter: i32,
}

impl Aes256CtrDrbg {
    const fn new() -> Self {
        Aes256CtrDrbg {
            key: [0u8; 32],
            v: [0u8; 16],
            reseed_counter: 0,
        }
    }
}

static DRBG_CTX: Mutex<Aes256CtrDrbg> = Mutex::new(Aes256CtrDrbg::new());

fn aes256_ecb(key: &[u8], ctr: &[u8], buffer: &mut [u8]) {
    let cipher = Cipher::aes_256_ecb();
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, None).unwrap();
    crypter.pad(false);
    let count = crypter.update(ctr, buffer).unwrap();
    let _ = crypter.finalize(&mut buffer[count..]).unwrap();
}

fn aes256_ctr_drbg_update(provided_data: Option<&[u8]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
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
    ctx.key = [0u8; 32];
    ctx.v = [0u8; 16];
    // Work around borrow checker by copying out, updating, copying back
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
