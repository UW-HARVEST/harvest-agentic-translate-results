//! Translation of `app/src/rng.c` and `app/include/rng.h` (NIST-provided
//! deterministic CTR_DRBG).
//!
//! `AES256_ECB` is implemented with the pure-Rust `aes` crate instead of
//! OpenSSL's EVP interface; both compute a single AES-256 ECB block encryption,
//! so the DRBG output is bit-identical.

use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes256;
use core::ffi::{c_int, c_ulong, c_ulonglong};

pub const RNG_SUCCESS: c_int = 0;
pub const RNG_BAD_MAXLEN: c_int = -1;
pub const RNG_BAD_OUTBUF: c_int = -2;
pub const RNG_BAD_REQ_LEN: c_int = -3;

/// `AES_XOF_struct`
#[repr(C)]
pub struct AesXofStruct {
    pub buffer: [u8; 16],
    pub buffer_pos: c_ulong,
    pub length_remaining: c_ulong,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

/// `AES256_CTR_DRBG_struct`
#[repr(C)]
pub struct Aes256CtrDrbgStruct {
    pub Key: [u8; 32],
    pub V: [u8; 16],
    pub reseed_counter: c_int,
}

/// `AES256_CTR_DRBG_struct DRBG_ctx;`
#[unsafe(no_mangle)]
pub static mut DRBG_ctx: Aes256CtrDrbgStruct = Aes256CtrDrbgStruct {
    Key: [0u8; 32],
    V: [0u8; 16],
    reseed_counter: 0,
};

#[inline]
fn drbg() -> &'static mut Aes256CtrDrbgStruct {
    unsafe { &mut *core::ptr::addr_of_mut!(DRBG_ctx) }
}

/// Use whatever AES implementation you have.
///
/// * `key` - 256-bit AES key
/// * `ctr` - a 128-bit plaintext value
/// * `buffer` - a 128-bit ciphertext value
pub fn aes256_ecb(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    let cipher = Aes256::new(key.into());
    let mut block = *ctr;
    cipher.encrypt_block((&mut block).into());
    *buffer = block;
}

/// `seedexpander_init()`
///
/// * `ctx` - stores the current state of an instance of the seed expander
/// * `seed` - a 32 byte random value
/// * `diversifier` - an 8 byte diversifier
/// * `maxlen` - maximum number of bytes (less than 2**32) generated under this
///   seed and diversifier
pub fn seedexpander_init_rs(
    ctx: &mut AesXofStruct,
    seed: &[u8],
    diversifier: &[u8],
    maxlen: u64,
) -> c_int {
    if maxlen >= 0x1_0000_0000u64 {
        return RNG_BAD_MAXLEN;
    }

    ctx.length_remaining = maxlen as c_ulong;

    ctx.key.copy_from_slice(&seed[..32]);

    ctx.ctr[..8].copy_from_slice(&diversifier[..8]);
    let mut maxlen = maxlen;
    ctx.ctr[11] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[10] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[9] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[8] = (maxlen % 256) as u8;
    for b in ctx.ctr[12..16].iter_mut() {
        *b = 0;
    }

    ctx.buffer_pos = 16;
    ctx.buffer = [0u8; 16];

    RNG_SUCCESS
}

/// `seedexpander()`
///
/// * `ctx` - stores the current state of an instance of the seed expander
/// * `x` - returns the XOF data
pub fn seedexpander_rs(ctx: &mut AesXofStruct, x: &mut [u8], xlen: u64) -> c_int {
    if xlen >= ctx.length_remaining as u64 {
        return RNG_BAD_REQ_LEN;
    }

    ctx.length_remaining -= xlen as c_ulong;

    let mut xlen = xlen as usize;
    let mut offset = 0usize;
    while xlen > 0 {
        let pos = ctx.buffer_pos as usize;
        if xlen <= 16 - pos {
            // buffer has what we need
            x[offset..offset + xlen].copy_from_slice(&ctx.buffer[pos..pos + xlen]);
            ctx.buffer_pos += xlen as c_ulong;

            return RNG_SUCCESS;
        }

        // take what's in the buffer
        let take = 16 - pos;
        x[offset..offset + take].copy_from_slice(&ctx.buffer[pos..16]);
        xlen -= take;
        offset += take;

        let key = ctx.key;
        let ctr = ctx.ctr;
        aes256_ecb(&key, &ctr, &mut ctx.buffer);
        ctx.buffer_pos = 0;

        // increment the counter
        for i in (12..16).rev() {
            if ctx.ctr[i] == 0xff {
                ctx.ctr[i] = 0x00;
            } else {
                ctx.ctr[i] += 1;
                break;
            }
        }
    }

    RNG_SUCCESS
}

pub fn randombytes_init_rs(entropy_input: &[u8], personalization_string: Option<&[u8]>) {
    let mut seed_material = [0u8; 48];

    seed_material.copy_from_slice(&entropy_input[..48]);
    if let Some(ps) = personalization_string {
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }
    let ctx = drbg();
    ctx.Key = [0u8; 32];
    ctx.V = [0u8; 16];
    aes256_ctr_drbg_update(Some(&seed_material), &mut ctx.Key, &mut ctx.V);
    ctx.reseed_counter = 1;
}

pub fn randombytes_drbg(x: &mut [u8]) -> c_int {
    let mut block = [0u8; 16];
    let mut i = 0usize;
    let mut xlen = x.len();

    let ctx = drbg();
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
            x[i..i + 16].copy_from_slice(&block);
            i += 16;
            xlen -= 16;
        } else {
            x[i..i + xlen].copy_from_slice(&block[..xlen]);
            xlen = 0;
        }
    }
    aes256_ctr_drbg_update(None, &mut ctx.Key, &mut ctx.V);
    ctx.reseed_counter += 1;

    RNG_SUCCESS
}

pub fn aes256_ctr_drbg_update(provided_data: Option<&[u8; 48]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
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
    if let Some(pd) = provided_data {
        for i in 0..48 {
            temp[i] ^= pd[i];
        }
    }
    key.copy_from_slice(&temp[..32]);
    v.copy_from_slice(&temp[32..48]);
}

// ---------------------------------------------------------------------------
// C ABI
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_ECB(key: *mut u8, ctr: *mut u8, buffer: *mut u8) {
    let k = &*(key as *const [u8; 32]);
    let c = &*(ctr as *const [u8; 16]);
    let b = &mut *(buffer as *mut [u8; 16]);
    aes256_ecb(k, c, b);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    Key: *mut u8,
    V: *mut u8,
) {
    let key = &mut *(Key as *mut [u8; 32]);
    let v = &mut *(V as *mut [u8; 16]);
    if provided_data.is_null() {
        aes256_ctr_drbg_update(None, key, v);
    } else {
        let pd = &*(provided_data as *const [u8; 48]);
        aes256_ctr_drbg_update(Some(pd), key, v);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander_init(
    ctx: *mut AesXofStruct,
    seed: *mut u8,
    diversifier: *mut u8,
    maxlen: c_ulong,
) -> c_int {
    let seed_s = core::slice::from_raw_parts(seed, 32);
    let div_s = core::slice::from_raw_parts(diversifier, 8);
    seedexpander_init_rs(&mut *ctx, seed_s, div_s, maxlen as u64)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander(
    ctx: *mut AesXofStruct,
    x: *mut u8,
    xlen: c_ulong,
) -> c_int {
    if x.is_null() {
        return RNG_BAD_OUTBUF;
    }
    let x_s = core::slice::from_raw_parts_mut(x, xlen as usize);
    seedexpander_rs(&mut *ctx, x_s, xlen as u64)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(
    entropy_input: *mut u8,
    personalization_string: *mut u8,
) {
    let e = core::slice::from_raw_parts(entropy_input, 48);
    if personalization_string.is_null() {
        randombytes_init_rs(e, None);
    } else {
        let p = core::slice::from_raw_parts(personalization_string, 48);
        randombytes_init_rs(e, Some(p));
    }
}

/// `int randombytes(unsigned char *x, unsigned long long xlen)`
///
/// Provided by `rng.c` (CMake target `sphincs_core_det`, used by the driver).
#[cfg(rand_drbg)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut u8, xlen: c_ulonglong) -> c_int {
    let s = core::slice::from_raw_parts_mut(x, xlen as usize);
    randombytes_drbg(s)
}

#[cfg(not(rand_drbg))]
#[allow(dead_code)]
pub unsafe fn randombytes_det(x: *mut u8, xlen: c_ulonglong) -> c_int {
    let s = core::slice::from_raw_parts_mut(x, xlen as usize);
    randombytes_drbg(s)
}
