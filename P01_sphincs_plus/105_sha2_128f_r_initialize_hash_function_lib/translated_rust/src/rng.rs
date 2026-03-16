use crate::params::*;
use aes::cipher::{BlockEncrypt, KeyInit, generic_array::GenericArray};
use aes::Aes256;
use std::sync::Mutex;

#[repr(C)]
pub struct AesXofStruct {
    pub buffer: [u8; 16],
    pub buffer_pos: u64,
    pub length_remaining: u64,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

#[repr(C)]
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

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander_init(
    ctx: *mut AesXofStruct, seed: *const u8, diversifier: *const u8, maxlen: u64,
) -> i32 {
    let ctx = unsafe { &mut *ctx };
    let seed = unsafe { std::slice::from_raw_parts(seed, 32) };
    let diversifier = unsafe { std::slice::from_raw_parts(diversifier, 8) };

    if maxlen >= 0x100000000 { return RNG_BAD_MAXLEN; }

    ctx.length_remaining = maxlen;
    ctx.key.copy_from_slice(seed);
    ctx.ctr[..8].copy_from_slice(diversifier);
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

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander(ctx: *mut AesXofStruct, x: *mut u8, mut xlen: u64) -> i32 {
    let ctx = unsafe { &mut *ctx };
    if x.is_null() { return RNG_BAD_OUTBUF; }
    if xlen >= ctx.length_remaining { return RNG_BAD_REQ_LEN; }
    let x = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };

    ctx.length_remaining -= xlen;
    let mut offset = 0usize;

    while xlen > 0 {
        let avail = 16 - ctx.buffer_pos as usize;
        if (xlen as usize) <= avail {
            x[offset..offset + xlen as usize].copy_from_slice(
                &ctx.buffer[ctx.buffer_pos as usize..ctx.buffer_pos as usize + xlen as usize],
            );
            ctx.buffer_pos += xlen;
            return RNG_SUCCESS;
        }

        x[offset..offset + avail].copy_from_slice(&ctx.buffer[ctx.buffer_pos as usize..16]);
        xlen -= avail as u64;
        offset += avail;

        aes256_ecb(&ctx.key, &ctx.ctr, &mut ctx.buffer);
        ctx.buffer_pos = 0;

        for i in (12..=15).rev() {
            if ctx.ctr[i] == 0xff { ctx.ctr[i] = 0x00; }
            else { ctx.ctr[i] += 1; break; }
        }
    }
    RNG_SUCCESS
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(entropy_input: *const u8, personalization_string: *const u8) {
    let entropy = unsafe { std::slice::from_raw_parts(entropy_input, 48) };
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(entropy);

    if !personalization_string.is_null() {
        let ps = unsafe { std::slice::from_raw_parts(personalization_string, 48) };
        for i in 0..48 { seed_material[i] ^= ps[i]; }
    }

    let mut ctx = DRBG_CTX.lock().unwrap();
    ctx.key.fill(0);
    ctx.v.fill(0);
    let mut key_copy = ctx.key;
    let mut v_copy = ctx.v;
    aes256_ctr_drbg_update_inner(&seed_material, &mut key_copy, &mut v_copy);
    ctx.key = key_copy;
    ctx.v = v_copy;
    ctx.reseed_counter = 1;
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut u8, xlen: u64) -> i32 {
    let x = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    let mut ctx = DRBG_CTX.lock().unwrap();
    let mut i = 0usize;
    let mut remaining = xlen as usize;
    let mut block = [0u8; 16];

    while remaining > 0 {
        for j in (0..16).rev() {
            if ctx.v[j] == 0xff { ctx.v[j] = 0x00; }
            else { ctx.v[j] += 1; break; }
        }
        aes256_ecb(&ctx.key, &ctx.v, &mut block);
        if remaining > 15 {
            x[i..i + 16].copy_from_slice(&block);
            i += 16; remaining -= 16;
        } else {
            x[i..i + remaining].copy_from_slice(&block[..remaining]);
            remaining = 0;
        }
    }
    let mut key_copy = ctx.key;
    let mut v_copy = ctx.v;
    aes256_ctr_drbg_update_inner(&[], &mut key_copy, &mut v_copy);
    ctx.key = key_copy;
    ctx.v = v_copy;
    ctx.reseed_counter += 1;
    RNG_SUCCESS
}

fn aes256_ctr_drbg_update_inner(provided_data: &[u8], key: &mut [u8; 32], v: &mut [u8; 16]) {
    let mut temp = [0u8; 48];
    for i in 0..3 {
        for j in (0..16).rev() {
            if v[j] == 0xff { v[j] = 0x00; }
            else { v[j] += 1; break; }
        }
        aes256_ecb(key, v, &mut temp[16 * i..]);
    }
    if !provided_data.is_empty() {
        for i in 0..48 { temp[i] ^= provided_data[i]; }
    }
    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..48]);
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(provided_data: *const u8, key: *mut u8, v: *mut u8) {
    let key = unsafe { &mut *(key as *mut [u8; 32]) };
    let v = unsafe { &mut *(v as *mut [u8; 16]) };
    if provided_data.is_null() {
        aes256_ctr_drbg_update_inner(&[], key, v);
    } else {
        let pd = unsafe { std::slice::from_raw_parts(provided_data, 48) };
        aes256_ctr_drbg_update_inner(pd, key, v);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_urandom(x: *mut u8, xlen: u64) {
    use std::io::Read;
    let x = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    let mut remaining = xlen as usize;
    let mut offset = 0;
    while remaining > 0 {
        if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
            match f.read(&mut x[offset..]) {
                Ok(n) if n > 0 => { offset += n; remaining -= n; }
                _ => { std::thread::sleep(std::time::Duration::from_secs(1)); }
            }
        } else {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
