// Deterministic AES-256-CTR DRBG (rng.c). The driver binary uses this for both
// `randombytes_init` and `randombytes`. The non-deterministic variant from
// randombytes.c is intentionally not used by the driver.

use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes256;

pub const RNG_SUCCESS: i32 = 0;
#[allow(dead_code)]
pub const RNG_BAD_MAXLEN: i32 = -1;
#[allow(dead_code)]
pub const RNG_BAD_OUTBUF: i32 = -2;
#[allow(dead_code)]
pub const RNG_BAD_REQ_LEN: i32 = -3;

#[repr(C)]
pub struct AES256CTRDRBG {
    pub key: [u8; 32],
    pub v: [u8; 16],
    pub reseed_counter: i32,
}

static mut DRBG_CTX: AES256CTRDRBG = AES256CTRDRBG {
    key: [0; 32],
    v: [0; 16],
    reseed_counter: 0,
};

fn aes256_ecb(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    let cipher = Aes256::new(key.into());
    let mut block = aes::cipher::generic_array::GenericArray::clone_from_slice(ctr);
    cipher.encrypt_block(&mut block);
    buffer.copy_from_slice(&block);
}

fn increment_v(v: &mut [u8; 16]) {
    for j in (0..16).rev() {
        if v[j] == 0xff {
            v[j] = 0;
        } else {
            v[j] += 1;
            return;
        }
    }
}

fn drbg_update(provided_data: Option<&[u8]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
    let mut temp = [0u8; 48];
    for i in 0..3 {
        increment_v(v);
        let mut block = [0u8; 16];
        aes256_ecb(key, v, &mut block);
        temp[16 * i..16 * i + 16].copy_from_slice(&block);
    }
    if let Some(provided) = provided_data {
        for i in 0..48 {
            temp[i] ^= provided[i];
        }
    }
    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..48]);
}

pub fn randombytes_init_internal(entropy_input: &[u8], personalization_string: Option<&[u8]>) {
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(&entropy_input[..48]);
    if let Some(ps) = personalization_string {
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }
    unsafe {
        DRBG_CTX.key = [0; 32];
        DRBG_CTX.v = [0; 16];
        let mut k = DRBG_CTX.key;
        let mut v = DRBG_CTX.v;
        drbg_update(Some(&seed_material), &mut k, &mut v);
        DRBG_CTX.key = k;
        DRBG_CTX.v = v;
        DRBG_CTX.reseed_counter = 1;
    }
}

pub fn randombytes(out: &mut [u8], xlen: u64) -> i32 {
    let mut block = [0u8; 16];
    let mut i: usize = 0;
    let mut xlen = xlen;
    unsafe {
        let mut k = DRBG_CTX.key;
        let mut v = DRBG_CTX.v;
        while xlen > 0 {
            increment_v(&mut v);
            aes256_ecb(&k, &v, &mut block);
            if xlen > 15 {
                out[i..i + 16].copy_from_slice(&block);
                i += 16;
                xlen -= 16;
            } else {
                out[i..i + xlen as usize].copy_from_slice(&block[..xlen as usize]);
                xlen = 0;
            }
        }
        drbg_update(None, &mut k, &mut v);
        DRBG_CTX.key = k;
        DRBG_CTX.v = v;
        DRBG_CTX.reseed_counter += 1;
    }
    RNG_SUCCESS
}

// ---------- C-ABI exports ----------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(
    entropy_input: *mut u8,
    personalization_string: *mut u8,
) {
    let entropy = unsafe { core::slice::from_raw_parts(entropy_input, 48) };
    let ps = if personalization_string.is_null() {
        None
    } else {
        Some(unsafe { core::slice::from_raw_parts(personalization_string, 48) })
    };
    randombytes_init_internal(entropy, ps);
}

#[unsafe(export_name = "randombytes")]
pub unsafe extern "C" fn c_randombytes(
    out: *mut u8,
    xlen: core::ffi::c_ulonglong,
) -> core::ffi::c_int {
    let slice = unsafe { core::slice::from_raw_parts_mut(out, xlen as usize) };
    randombytes(slice, xlen)
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    key: *mut u8,
    v: *mut u8,
) {
    let provided = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { core::slice::from_raw_parts(provided_data, 48) })
    };
    let key_arr: &mut [u8; 32] = unsafe { &mut *(key as *mut [u8; 32]) };
    let v_arr: &mut [u8; 16] = unsafe { &mut *(v as *mut [u8; 16]) };
    drbg_update(provided, key_arr, v_arr);
}
