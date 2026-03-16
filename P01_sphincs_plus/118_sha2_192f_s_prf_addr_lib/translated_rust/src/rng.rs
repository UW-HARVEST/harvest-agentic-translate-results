use openssl::symm::{Cipher, Crypter, Mode};
use std::sync::Mutex;

const RNG_SUCCESS: i32 = 0;
const RNG_BAD_MAXLEN: i32 = -1;
const RNG_BAD_OUTBUF: i32 = -2;
const RNG_BAD_REQ_LEN: i32 = -3;

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
    let cipher = Cipher::aes_256_ecb();
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, None).unwrap();
    crypter.pad(false);
    let count = crypter.update(ctr, buffer).unwrap();
    let _ = crypter.finalize(&mut buffer[count..]).unwrap();
}

fn aes256_ctr_drbg_update_inner(provided_data: Option<&[u8]>, key: &mut [u8], v: &mut [u8]) {
    let mut temp = [0u8; 48];
    for i in 0..3 {
        // increment V
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
    if let Some(pd) = provided_data {
        for i in 0..48 {
            temp[i] ^= pd[i];
        }
    }
    key[..32].copy_from_slice(&temp[..32]);
    v[..16].copy_from_slice(&temp[32..48]);
}

fn randombytes_inner(x: &mut [u8], mut xlen: usize) -> i32 {
    let mut drbg = DRBG_CTX.lock().unwrap();
    let mut block = [0u8; 16];
    let mut i = 0usize;

    while xlen > 0 {
        // increment V
        for j in (0..16).rev() {
            if drbg.v[j] == 0xff {
                drbg.v[j] = 0x00;
            } else {
                drbg.v[j] += 1;
                break;
            }
        }
        aes256_ecb(&drbg.key.clone(), &drbg.v, &mut block);
        if xlen > 15 {
            x[i..i + 16].copy_from_slice(&block);
            i += 16;
            xlen -= 16;
        } else {
            x[i..i + xlen].copy_from_slice(&block[..xlen]);
            xlen = 0;
        }
    }
    let mut key_copy = drbg.key;
    let mut v_copy = drbg.v;
    aes256_ctr_drbg_update_inner(None, &mut key_copy, &mut v_copy);
    drbg.key = key_copy;
    drbg.v = v_copy;
    drbg.reseed_counter += 1;
    RNG_SUCCESS
}

// Public C API

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    key: *mut u8,
    v: *mut u8,
) {
    unsafe {
        let key_slice = std::slice::from_raw_parts_mut(key, 32);
        let v_slice = std::slice::from_raw_parts_mut(v, 16);
        let pd = if provided_data.is_null() {
            None
        } else {
            Some(std::slice::from_raw_parts(provided_data, 48))
        };
        aes256_ctr_drbg_update_inner(pd, key_slice, v_slice);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander_init(
    ctx: *mut AesXofStruct,
    seed: *mut u8,
    diversifier: *mut u8,
    maxlen: u64,
) -> i32 {
    unsafe {
        if maxlen >= 0x100000000 {
            return RNG_BAD_MAXLEN;
        }
        let c = &mut *ctx;
        c.length_remaining = maxlen;
        c.key.copy_from_slice(std::slice::from_raw_parts(seed, 32));
        c.ctr[..8].copy_from_slice(std::slice::from_raw_parts(diversifier, 8));
        let mut ml = maxlen;
        c.ctr[11] = (ml % 256) as u8; ml >>= 8;
        c.ctr[10] = (ml % 256) as u8; ml >>= 8;
        c.ctr[9] = (ml % 256) as u8; ml >>= 8;
        c.ctr[8] = (ml % 256) as u8;
        c.ctr[12..16].fill(0);
        c.buffer_pos = 16;
        c.buffer.fill(0);
        RNG_SUCCESS
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander(ctx: *mut AesXofStruct, x: *mut u8, mut xlen: u64) -> i32 {
    unsafe {
        if x.is_null() {
            return RNG_BAD_OUTBUF;
        }
        let c = &mut *ctx;
        if xlen >= c.length_remaining {
            return RNG_BAD_REQ_LEN;
        }
        c.length_remaining -= xlen;
        let mut offset = 0u64;
        while xlen > 0 {
            let buf_remaining = 16 - c.buffer_pos;
            if xlen <= buf_remaining {
                std::ptr::copy_nonoverlapping(
                    c.buffer.as_ptr().add(c.buffer_pos as usize),
                    x.add(offset as usize),
                    xlen as usize,
                );
                c.buffer_pos += xlen;
                return RNG_SUCCESS;
            }
            std::ptr::copy_nonoverlapping(
                c.buffer.as_ptr().add(c.buffer_pos as usize),
                x.add(offset as usize),
                buf_remaining as usize,
            );
            xlen -= buf_remaining;
            offset += buf_remaining;
            aes256_ecb(&c.key, &c.ctr, &mut c.buffer);
            c.buffer_pos = 0;
            // increment counter
            for i in (12..=15).rev() {
                if c.ctr[i] == 0xff {
                    c.ctr[i] = 0x00;
                } else {
                    c.ctr[i] += 1;
                    break;
                }
            }
        }
        RNG_SUCCESS
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(
    entropy_input: *mut u8,
    personalization_string: *mut u8,
) {
    unsafe {
        let mut seed_material = [0u8; 48];
        seed_material.copy_from_slice(std::slice::from_raw_parts(entropy_input, 48));
        if !personalization_string.is_null() {
            let ps = std::slice::from_raw_parts(personalization_string, 48);
            for i in 0..48 {
                seed_material[i] ^= ps[i];
            }
        }
        let mut drbg = DRBG_CTX.lock().unwrap();
        drbg.key.fill(0);
        drbg.v.fill(0);
        let mut key_copy = drbg.key;
        let mut v_copy = drbg.v;
        aes256_ctr_drbg_update_inner(Some(&seed_material), &mut key_copy, &mut v_copy);
        drbg.key = key_copy;
        drbg.v = v_copy;
        drbg.reseed_counter = 1;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut u8, xlen: u64) -> i32 {
    let buf = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    randombytes_inner(buf, xlen as usize)
}

// Internal version for use within the crate
pub fn randombytes_internal(buf: &mut [u8], len: usize) {
    randombytes_inner(&mut buf[..len], len);
}
