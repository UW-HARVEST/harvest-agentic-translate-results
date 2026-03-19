#![allow(non_snake_case)]
use aes::cipher::{BlockEncrypt, KeyInit, generic_array::GenericArray};
use aes::Aes256;

pub const RNG_SUCCESS: i32 = 0;

#[derive(Clone)]
pub struct AES256_CTR_DRBG_struct {
    pub Key: [u8; 32],
    pub V: [u8; 16],
    pub reseed_counter: i32,
}

static mut DRBG_CTX: AES256_CTR_DRBG_struct = AES256_CTR_DRBG_struct {
    Key: [0; 32],
    V: [0; 16],
    reseed_counter: 0,
};

fn AES256_ECB(key: &[u8], ctr: &[u8], buffer: &mut [u8]) {
    let cipher = Aes256::new(GenericArray::from_slice(key));
    let mut block = *GenericArray::from_slice(ctr);
    cipher.encrypt_block(&mut block);
    buffer[..16].copy_from_slice(&block);
}

pub fn AES256_CTR_DRBG_Update(provided_data: Option<&[u8]>, key: &mut [u8], v: &mut [u8]) {
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
        AES256_ECB(key, v, &mut temp[16 * i..16 * i + 16]);
    }
    if let Some(pd) = provided_data {
        for i in 0..48 {
            temp[i] ^= pd[i];
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
    unsafe {
        DRBG_CTX.Key = [0; 32];
        DRBG_CTX.V = [0; 16];
        let mut key = DRBG_CTX.Key;
        let mut v = DRBG_CTX.V;
        AES256_CTR_DRBG_Update(Some(&seed_material), &mut key, &mut v);
        DRBG_CTX.Key = key;
        DRBG_CTX.V = v;
        DRBG_CTX.reseed_counter = 1;
    }
}

pub fn randombytes(x: &mut [u8], mut xlen: u64) -> i32 {
    let mut block = [0u8; 16];
    let mut i: usize = 0;

    unsafe {
        while xlen > 0 {
            for j in (0..16).rev() {
                if DRBG_CTX.V[j] == 0xff {
                    DRBG_CTX.V[j] = 0x00;
                } else {
                    DRBG_CTX.V[j] += 1;
                    break;
                }
            }
            AES256_ECB(&DRBG_CTX.Key, &DRBG_CTX.V, &mut block);
            if xlen > 15 {
                x[i..i + 16].copy_from_slice(&block);
                i += 16;
                xlen -= 16;
            } else {
                x[i..i + xlen as usize].copy_from_slice(&block[..xlen as usize]);
                xlen = 0;
            }
        }
        let mut key = DRBG_CTX.Key;
        let mut v = DRBG_CTX.V;
        AES256_CTR_DRBG_Update(None, &mut key, &mut v);
        DRBG_CTX.Key = key;
        DRBG_CTX.V = v;
        DRBG_CTX.reseed_counter += 1;
    }
    RNG_SUCCESS
}
