#![allow(static_mut_refs)]

use aes::cipher::{BlockEncrypt, KeyInit, generic_array::GenericArray};
use aes::Aes256;

pub struct Aes256CtrDrbg {
    pub key: [u8; 32],
    pub v: [u8; 16],
    pub reseed_counter: i32,
}

static mut DRBG_CTX: Aes256CtrDrbg = Aes256CtrDrbg {
    key: [0u8; 32],
    v: [0u8; 16],
    reseed_counter: 0,
};

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
            if v[j] == 0xff { v[j] = 0x00; }
            else { v[j] += 1; break; }
        }
        aes256_ecb(key, v, &mut temp[16 * i..]);
    }
    if let Some(data) = provided_data {
        for i in 0..48 { temp[i] ^= data[i]; }
    }
    key[..32].copy_from_slice(&temp[..32]);
    v[..16].copy_from_slice(&temp[32..48]);
}

pub fn randombytes_init(entropy_input: &[u8], personalization_string: Option<&[u8]>) {
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(&entropy_input[..48]);
    if let Some(ps) = personalization_string {
        for i in 0..48 { seed_material[i] ^= ps[i]; }
    }
    unsafe {
        DRBG_CTX.key = [0u8; 32];
        DRBG_CTX.v = [0u8; 16];
        aes256_ctr_drbg_update(Some(&seed_material), &mut DRBG_CTX.key, &mut DRBG_CTX.v);
        DRBG_CTX.reseed_counter = 1;
    }
}

pub fn randombytes(x: &mut [u8], mut xlen: u64) -> i32 {
    let mut block = [0u8; 16];
    let mut i = 0usize;
    unsafe {
        while xlen > 0 {
            for j in (0..16).rev() {
                if DRBG_CTX.v[j] == 0xff { DRBG_CTX.v[j] = 0x00; }
                else { DRBG_CTX.v[j] += 1; break; }
            }
            aes256_ecb(&DRBG_CTX.key, &DRBG_CTX.v, &mut block);
            if xlen > 15 {
                x[i..i + 16].copy_from_slice(&block);
                i += 16; xlen -= 16;
            } else {
                x[i..i + xlen as usize].copy_from_slice(&block[..xlen as usize]);
                xlen = 0;
            }
        }
        aes256_ctr_drbg_update(None, &mut DRBG_CTX.key, &mut DRBG_CTX.v);
        DRBG_CTX.reseed_counter += 1;
    }
    0
}
