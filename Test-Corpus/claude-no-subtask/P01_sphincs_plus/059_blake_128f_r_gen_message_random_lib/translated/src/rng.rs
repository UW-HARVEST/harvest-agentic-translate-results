// NIST AES256-CTR DRBG
#![allow(non_snake_case, dead_code)]

use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes256;
use std::sync::Mutex;

#[repr(C)]
pub struct AES256CtrDrbgStruct {
    pub Key: [u8; 32],
    pub V: [u8; 16],
    pub reseed_counter: i32,
}

impl AES256CtrDrbgStruct {
    pub const fn new() -> Self {
        Self {
            Key: [0; 32],
            V: [0; 16],
            reseed_counter: 0,
        }
    }
}

static DRBG_CTX: Mutex<AES256CtrDrbgStruct> = Mutex::new(AES256CtrDrbgStruct::new());

pub fn aes256_ecb(key: &[u8], ctr: &[u8], buffer: &mut [u8]) {
    let cipher = Aes256::new_from_slice(key).expect("AES key length");
    let mut block = aes::cipher::generic_array::GenericArray::clone_from_slice(ctr);
    cipher.encrypt_block(&mut block);
    buffer[..16].copy_from_slice(&block);
}

pub fn aes256_ctr_drbg_update(provided_data: Option<&[u8]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
    let mut temp = [0u8; 48];

    for i in 0..3 {
        // increment V
        for j in (0..16).rev() {
            if v[j] == 0xff {
                v[j] = 0;
            } else {
                v[j] += 1;
                break;
            }
        }
        let mut block = [0u8; 16];
        aes256_ecb(key, v, &mut block);
        temp[16 * i..16 * i + 16].copy_from_slice(&block);
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
    ctx.Key = [0; 32];
    ctx.V = [0; 16];
    let mut k = ctx.Key;
    let mut v = ctx.V;
    aes256_ctr_drbg_update(Some(&seed_material), &mut k, &mut v);
    ctx.Key = k;
    ctx.V = v;
    ctx.reseed_counter = 1;
}

pub fn randombytes(x: &mut [u8], mut xlen: usize) -> i32 {
    let mut block = [0u8; 16];
    let mut i: usize = 0;
    let mut ctx = DRBG_CTX.lock().unwrap();
    let mut k = ctx.Key;
    let mut v = ctx.V;
    while xlen > 0 {
        for j in (0..16).rev() {
            if v[j] == 0xff {
                v[j] = 0;
            } else {
                v[j] += 1;
                break;
            }
        }
        aes256_ecb(&k, &v, &mut block);
        if xlen > 15 {
            x[i..i + 16].copy_from_slice(&block);
            i += 16;
            xlen -= 16;
        } else {
            x[i..i + xlen].copy_from_slice(&block[..xlen]);
            xlen = 0;
        }
    }
    aes256_ctr_drbg_update(None, &mut k, &mut v);
    ctx.Key = k;
    ctx.V = v;
    ctx.reseed_counter += 1;
    0
}

// C-ABI exports

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    key: *mut u8,
    v: *mut u8,
) {
    let pd = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(provided_data, 48) })
    };
    let k = unsafe { &mut *(key as *mut [u8; 32]) };
    let vv = unsafe { &mut *(v as *mut [u8; 16]) };
    aes256_ctr_drbg_update(pd, k, vv);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init_c(
    entropy_input: *mut u8,
    personalization_string: *mut u8,
) {
    let e = unsafe { std::slice::from_raw_parts(entropy_input, 48) };
    let ps = if personalization_string.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(personalization_string, 48) })
    };
    randombytes_init(e, ps);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_c(x: *mut u8, xlen: u64) -> i32 {
    let s = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    randombytes(s, xlen as usize)
}
