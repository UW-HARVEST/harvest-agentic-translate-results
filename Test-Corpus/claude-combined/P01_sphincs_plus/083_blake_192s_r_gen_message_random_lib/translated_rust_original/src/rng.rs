// Translation of c_src/app/src/rng.c (deterministic AES-256 CTR DRBG)

use std::sync::Mutex;

use aes::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};
use aes::Aes256;

#[repr(C)]
pub struct AES256CTRDRBGStruct {
    pub key: [u8; 32],
    pub v: [u8; 16],
    pub reseed_counter: i32,
}

static DRBG_CTX: Mutex<AES256CTRDRBGStruct> = Mutex::new(AES256CTRDRBGStruct {
    key: [0u8; 32],
    v: [0u8; 16],
    reseed_counter: 0,
});

fn aes256_ecb(key: &[u8], ctr: &[u8], buffer: &mut [u8]) {
    let cipher = Aes256::new(GenericArray::from_slice(key));
    let mut block = GenericArray::clone_from_slice(ctr);
    cipher.encrypt_block(&mut block);
    buffer[..16].copy_from_slice(block.as_slice());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_ECB(key: *mut u8, ctr: *mut u8, buffer: *mut u8) {
    let key = unsafe { core::slice::from_raw_parts(key, 32) };
    let ctr = unsafe { core::slice::from_raw_parts(ctr, 16) };
    let buffer = unsafe { core::slice::from_raw_parts_mut(buffer, 16) };
    aes256_ecb(key, ctr, buffer);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    key: *mut u8,
    v: *mut u8,
) {
    let key = unsafe { core::slice::from_raw_parts_mut(key, 32) };
    let v = unsafe { core::slice::from_raw_parts_mut(v, 16) };
    let provided_data_slice = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { core::slice::from_raw_parts(provided_data, 48) })
    };
    aes256_ctr_drbg_update(provided_data_slice, key, v);
}

fn aes256_ctr_drbg_update(provided_data: Option<&[u8]>, key: &mut [u8], v: &mut [u8]) {
    let mut temp = [0u8; 48];
    for i in 0..3 {
        // Increment V
        for j in (0..16).rev() {
            if v[j] == 0xff {
                v[j] = 0x00;
            } else {
                v[j] += 1;
                break;
            }
        }
        let mut buf = [0u8; 16];
        aes256_ecb(key, v, &mut buf);
        temp[16 * i..16 * i + 16].copy_from_slice(&buf);
    }
    if let Some(pd) = provided_data {
        for i in 0..48 {
            temp[i] ^= pd[i];
        }
    }
    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..48]);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(
    entropy_input: *mut u8,
    personalization_string: *mut u8,
) {
    let entropy = unsafe { core::slice::from_raw_parts(entropy_input, 48) };
    let pers = if personalization_string.is_null() {
        None
    } else {
        Some(unsafe { core::slice::from_raw_parts(personalization_string, 48) })
    };
    randombytes_init_inner(entropy, pers);
}

pub fn randombytes_init_inner(entropy_input: &[u8], personalization_string: Option<&[u8]>) {
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(entropy_input);
    if let Some(p) = personalization_string {
        for i in 0..48 {
            seed_material[i] ^= p[i];
        }
    }
    let mut ctx = DRBG_CTX.lock().unwrap();
    ctx.key = [0u8; 32];
    ctx.v = [0u8; 16];
    let mut k = ctx.key;
    let mut v = ctx.v;
    aes256_ctr_drbg_update(Some(&seed_material), &mut k, &mut v);
    ctx.key = k;
    ctx.v = v;
    ctx.reseed_counter = 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut u8, mut xlen: u64) -> i32 {
    let mut ctx = DRBG_CTX.lock().unwrap();
    let mut block = [0u8; 16];
    let mut i: usize = 0;
    let mut k = ctx.key;
    let mut v = ctx.v;
    while xlen > 0 {
        // Increment V
        for j in (0..16).rev() {
            if v[j] == 0xff {
                v[j] = 0x00;
            } else {
                v[j] += 1;
                break;
            }
        }
        aes256_ecb(&k, &v, &mut block);
        if xlen > 15 {
            unsafe {
                core::ptr::copy_nonoverlapping(block.as_ptr(), x.add(i), 16);
            }
            i += 16;
            xlen -= 16;
        } else {
            unsafe {
                core::ptr::copy_nonoverlapping(block.as_ptr(), x.add(i), xlen as usize);
            }
            xlen = 0;
        }
    }
    aes256_ctr_drbg_update(None, &mut k, &mut v);
    ctx.key = k;
    ctx.v = v;
    ctx.reseed_counter += 1;
    0
}

#[repr(C)]
pub struct AESXOFStruct {
    pub buffer: [u8; 16],
    pub buffer_pos: u64,
    pub length_remaining: u64,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander_init(
    ctx: *mut AESXOFStruct,
    seed: *mut u8,
    diversifier: *mut u8,
    mut maxlen: u64,
) -> i32 {
    if maxlen >= 0x100000000 {
        return -1;
    }
    let ctx = unsafe { &mut *ctx };
    let seed = unsafe { core::slice::from_raw_parts(seed, 32) };
    let diversifier = unsafe { core::slice::from_raw_parts(diversifier, 8) };
    ctx.length_remaining = maxlen;
    ctx.key.copy_from_slice(seed);
    ctx.ctr[..8].copy_from_slice(diversifier);
    ctx.ctr[11] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[10] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[9] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[8] = (maxlen % 256) as u8;
    for i in 12..16 { ctx.ctr[i] = 0; }
    ctx.buffer_pos = 16;
    ctx.buffer = [0u8; 16];
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander(
    ctx: *mut AESXOFStruct,
    x: *mut u8,
    mut xlen: u64,
) -> i32 {
    if x.is_null() {
        return -2;
    }
    let ctx = unsafe { &mut *ctx };
    if xlen >= ctx.length_remaining {
        return -3;
    }
    ctx.length_remaining -= xlen;
    let mut offset: u64 = 0;
    while xlen > 0 {
        let avail = 16u64 - ctx.buffer_pos;
        if xlen <= avail {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    ctx.buffer.as_ptr().add(ctx.buffer_pos as usize),
                    x.add(offset as usize),
                    xlen as usize,
                );
            }
            ctx.buffer_pos += xlen;
            return 0;
        }
        unsafe {
            core::ptr::copy_nonoverlapping(
                ctx.buffer.as_ptr().add(ctx.buffer_pos as usize),
                x.add(offset as usize),
                avail as usize,
            );
        }
        xlen -= avail;
        offset += avail;
        let mut buf = [0u8; 16];
        aes256_ecb(&ctx.key, &ctx.ctr, &mut buf);
        ctx.buffer = buf;
        ctx.buffer_pos = 0;
        for j in (12..16).rev() {
            if ctx.ctr[j] == 0xff {
                ctx.ctr[j] = 0;
            } else {
                ctx.ctr[j] += 1;
                break;
            }
        }
    }
    0
}
