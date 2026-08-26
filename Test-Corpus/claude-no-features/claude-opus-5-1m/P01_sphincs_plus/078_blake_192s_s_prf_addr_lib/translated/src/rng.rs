// NIST AES-256 CTR DRBG implementation
// Translated from rng.c (NIST KAT generator)

use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes256;
use std::ffi::c_int;
use std::sync::Mutex;

#[allow(non_snake_case)]
pub struct Aes256CtrDrbg {
    pub Key: [u8; 32],
    pub V: [u8; 16],
    pub reseed_counter: i32,
}

#[allow(non_snake_case)]
#[repr(C)]
pub struct AesXofStruct {
    pub buffer: [u8; 16],
    pub buffer_pos: u64,
    pub length_remaining: u64,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

static DRBG_CTX: Mutex<Aes256CtrDrbg> = Mutex::new(Aes256CtrDrbg {
    Key: [0u8; 32],
    V: [0u8; 16],
    reseed_counter: 0,
});

pub const RNG_SUCCESS: i32 = 0;
pub const RNG_BAD_MAXLEN: i32 = -1;
pub const RNG_BAD_OUTBUF: i32 = -2;
pub const RNG_BAD_REQ_LEN: i32 = -3;

fn aes256_ecb(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    let cipher = Aes256::new(key.into());
    let mut block = aes::Block::clone_from_slice(ctr);
    cipher.encrypt_block(&mut block);
    buffer.copy_from_slice(block.as_slice());
}

#[allow(non_snake_case)]
fn aes256_ctr_drbg_update(provided_data: Option<&[u8; 48]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
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
    v.copy_from_slice(&temp[32..]);
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    Key: *mut u8,
    V: *mut u8,
) {
    unsafe {
        let key_slice = std::slice::from_raw_parts_mut(Key, 32);
        let v_slice = std::slice::from_raw_parts_mut(V, 16);
        let mut key_arr: [u8; 32] = key_slice.try_into().unwrap();
        let mut v_arr: [u8; 16] = v_slice.try_into().unwrap();

        if !provided_data.is_null() {
            let pd_slice = std::slice::from_raw_parts(provided_data, 48);
            let pd_arr: [u8; 48] = pd_slice.try_into().unwrap();
            aes256_ctr_drbg_update(Some(&pd_arr), &mut key_arr, &mut v_arr);
        } else {
            aes256_ctr_drbg_update(None, &mut key_arr, &mut v_arr);
        }

        key_slice.copy_from_slice(&key_arr);
        v_slice.copy_from_slice(&v_arr);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(
    entropy_input: *mut u8,
    personalization_string: *mut u8,
) {
    unsafe {
        let mut seed_material = [0u8; 48];
        let entropy_slice = std::slice::from_raw_parts(entropy_input, 48);
        seed_material.copy_from_slice(entropy_slice);

        if !personalization_string.is_null() {
            let ps_slice = std::slice::from_raw_parts(personalization_string, 48);
            for i in 0..48 {
                seed_material[i] ^= ps_slice[i];
            }
        }

        let mut ctx = DRBG_CTX.lock().unwrap();
        ctx.Key = [0u8; 32];
        ctx.V = [0u8; 16];
        let mut k = ctx.Key;
        let mut v = ctx.V;
        aes256_ctr_drbg_update(Some(&seed_material), &mut k, &mut v);
        ctx.Key = k;
        ctx.V = v;
        ctx.reseed_counter = 1;
    }
}

#[allow(non_snake_case)]
pub fn SPX_randombytes(x: *mut u8, mut xlen: u64) -> c_int {
    unsafe {
        let mut block = [0u8; 16];
        let mut i: usize = 0;
        let mut ctx = DRBG_CTX.lock().unwrap();

        while xlen > 0 {
            // increment V
            for j in (0..16).rev() {
                if ctx.V[j] == 0xff {
                    ctx.V[j] = 0x00;
                } else {
                    ctx.V[j] += 1;
                    break;
                }
            }
            let key = ctx.Key;
            let v = ctx.V;
            aes256_ecb(&key, &v, &mut block);
            if xlen > 15 {
                std::ptr::copy_nonoverlapping(block.as_ptr(), x.add(i), 16);
                i += 16;
                xlen -= 16;
            } else {
                std::ptr::copy_nonoverlapping(block.as_ptr(), x.add(i), xlen as usize);
                xlen = 0;
            }
        }
        let mut k = ctx.Key;
        let mut v = ctx.V;
        aes256_ctr_drbg_update(None, &mut k, &mut v);
        ctx.Key = k;
        ctx.V = v;
        ctx.reseed_counter += 1;
    }
    RNG_SUCCESS
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut u8, xlen: u64) -> c_int {
    SPX_randombytes(x, xlen)
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander_init(
    ctx: *mut AesXofStruct,
    seed: *mut u8,
    diversifier: *mut u8,
    mut maxlen: u64,
) -> c_int {
    unsafe {
        if maxlen >= 0x100000000u64 {
            return RNG_BAD_MAXLEN;
        }
        let ctx_ref = &mut *ctx;
        ctx_ref.length_remaining = maxlen;

        let seed_slice = std::slice::from_raw_parts(seed, 32);
        ctx_ref.key.copy_from_slice(seed_slice);

        let div_slice = std::slice::from_raw_parts(diversifier, 8);
        ctx_ref.ctr[..8].copy_from_slice(div_slice);

        ctx_ref.ctr[11] = (maxlen % 256) as u8;
        maxlen >>= 8;
        ctx_ref.ctr[10] = (maxlen % 256) as u8;
        maxlen >>= 8;
        ctx_ref.ctr[9] = (maxlen % 256) as u8;
        maxlen >>= 8;
        ctx_ref.ctr[8] = (maxlen % 256) as u8;
        for i in 12..16 {
            ctx_ref.ctr[i] = 0;
        }

        ctx_ref.buffer_pos = 16;
        ctx_ref.buffer = [0u8; 16];
    }
    RNG_SUCCESS
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander(ctx: *mut AesXofStruct, x: *mut u8, mut xlen: u64) -> c_int {
    unsafe {
        if x.is_null() {
            return RNG_BAD_OUTBUF;
        }
        let ctx_ref = &mut *ctx;
        if xlen >= ctx_ref.length_remaining {
            return RNG_BAD_REQ_LEN;
        }
        ctx_ref.length_remaining -= xlen;

        let mut offset: usize = 0;
        while xlen > 0 {
            if xlen <= (16 - ctx_ref.buffer_pos) {
                std::ptr::copy_nonoverlapping(
                    ctx_ref.buffer.as_ptr().add(ctx_ref.buffer_pos as usize),
                    x.add(offset),
                    xlen as usize,
                );
                ctx_ref.buffer_pos += xlen;
                return RNG_SUCCESS;
            }

            let take = 16 - ctx_ref.buffer_pos;
            std::ptr::copy_nonoverlapping(
                ctx_ref.buffer.as_ptr().add(ctx_ref.buffer_pos as usize),
                x.add(offset),
                take as usize,
            );
            xlen -= take;
            offset += take as usize;

            let key = ctx_ref.key;
            let ctr = ctx_ref.ctr;
            let mut buf = [0u8; 16];
            aes256_ecb(&key, &ctr, &mut buf);
            ctx_ref.buffer = buf;
            ctx_ref.buffer_pos = 0;

            // increment counter
            for i in (12..16).rev() {
                if ctx_ref.ctr[i] == 0xff {
                    ctx_ref.ctr[i] = 0x00;
                } else {
                    ctx_ref.ctr[i] += 1;
                    break;
                }
            }
        }
    }
    RNG_SUCCESS
}
