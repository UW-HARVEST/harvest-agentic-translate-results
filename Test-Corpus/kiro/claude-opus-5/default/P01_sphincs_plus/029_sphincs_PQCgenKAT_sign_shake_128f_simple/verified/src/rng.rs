//! Translation of `app/src/rng.c` and `app/include/rng.h`, the NIST
//! AES-256-CTR-DRBG used by the KAT driver.
//!
//! `rng.c` obtains AES-256-ECB from OpenSSL; here the pure-Rust `aes` crate is
//! used instead.  The DRBG output is unchanged.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use core::ffi::{c_int, c_ulong};

use aes::Aes256;
use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockEncrypt, KeyInit};

pub const RNG_SUCCESS: c_int = 0;
pub const RNG_BAD_MAXLEN: c_int = -1;
pub const RNG_BAD_OUTBUF: c_int = -2;
pub const RNG_BAD_REQ_LEN: c_int = -3;

/// `AES_XOF_struct`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AES_XOF_struct {
    pub buffer: [u8; 16],
    pub buffer_pos: c_ulong,
    pub length_remaining: c_ulong,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

/// `AES256_CTR_DRBG_struct`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct AES256_CTR_DRBG_struct {
    pub Key: [u8; 32],
    pub V: [u8; 16],
    pub reseed_counter: c_int,
}

/// `AES256_CTR_DRBG_struct DRBG_ctx;`
#[unsafe(no_mangle)]
pub static mut DRBG_ctx: AES256_CTR_DRBG_struct = AES256_CTR_DRBG_struct {
    Key: [0u8; 32],
    V: [0u8; 16],
    reseed_counter: 0,
};

#[inline]
fn drbg() -> &'static mut AES256_CTR_DRBG_struct {
    // SAFETY: mirrors the single-threaded use of the C global.
    unsafe { &mut *(&raw mut DRBG_ctx) }
}

/// Use whatever AES implementation you have.
///
/// * `key` - 256-bit AES key
/// * `ctr` - a 128-bit plaintext value
/// * `buffer` - a 128-bit ciphertext value
pub fn aes256_ecb(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    let cipher = Aes256::new(GenericArray::from_slice(key));
    let mut block = *GenericArray::from_slice(ctr);
    cipher.encrypt_block(&mut block);
    buffer.copy_from_slice(block.as_slice());
}

/// Increments the 128-bit big-endian counter in `v`, C style: the loop stops at
/// the first byte that did not wrap around.
#[inline]
fn increment(v: &mut [u8], from: usize) {
    for j in (from..v.len()).rev() {
        if v[j] == 0xff {
            v[j] = 0x00;
        } else {
            v[j] += 1;
            break;
        }
    }
}

pub fn AES256_CTR_DRBG_Update_impl(
    provided_data: Option<&[u8; 48]>,
    key: &mut [u8; 32],
    v: &mut [u8; 16],
) {
    let mut temp = [0u8; 48];

    for i in 0..3 {
        /* increment V */
        increment(&mut v[..], 0);

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
    v.copy_from_slice(&temp[32..]);
}

/// `seedexpander_init()`
///
/// * `ctx` - stores the current state of an instance of the seed expander
/// * `seed` - a 32 byte random value
/// * `diversifier` - an 8 byte diversifier
/// * `maxlen` - maximum number of bytes (less than 2**32) generated under this
///   seed and diversifier
pub fn seedexpander_init_impl(
    ctx: &mut AES_XOF_struct,
    seed: &[u8; 32],
    diversifier: &[u8; 8],
    maxlen: c_ulong,
) -> c_int {
    if maxlen >= 0x1_0000_0000 {
        return RNG_BAD_MAXLEN;
    }

    ctx.length_remaining = maxlen;

    ctx.key.copy_from_slice(seed);

    ctx.ctr[..8].copy_from_slice(diversifier);
    let mut maxlen = maxlen;
    ctx.ctr[11] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[10] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[9] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[8] = (maxlen % 256) as u8;
    ctx.ctr[12..16].fill(0x00);

    ctx.buffer_pos = 16;
    ctx.buffer.fill(0x00);

    RNG_SUCCESS
}

/// `seedexpander()`
///
/// * `ctx` - stores the current state of an instance of the seed expander
/// * `x` - returns the XOF data
pub fn seedexpander_impl(ctx: &mut AES_XOF_struct, x: &mut [u8]) -> c_int {
    let mut xlen = x.len() as c_ulong;

    if xlen >= ctx.length_remaining {
        return RNG_BAD_REQ_LEN;
    }

    ctx.length_remaining -= xlen;

    let mut offset: usize = 0;
    while xlen > 0 {
        let avail = 16 - ctx.buffer_pos as usize;
        if xlen as usize <= avail {
            /* buffer has what we need */
            let pos = ctx.buffer_pos as usize;
            x[offset..offset + xlen as usize]
                .copy_from_slice(&ctx.buffer[pos..pos + xlen as usize]);
            ctx.buffer_pos += xlen;

            return RNG_SUCCESS;
        }

        /* take what's in the buffer */
        let pos = ctx.buffer_pos as usize;
        x[offset..offset + avail].copy_from_slice(&ctx.buffer[pos..16]);
        xlen -= avail as c_ulong;
        offset += avail;

        let key = ctx.key;
        let ctr = ctx.ctr;
        aes256_ecb(&key, &ctr, &mut ctx.buffer);
        ctx.buffer_pos = 0;

        /* increment the counter */
        increment(&mut ctx.ctr[..], 12);
    }

    RNG_SUCCESS
}

pub fn randombytes_init_impl(entropy_input: &[u8; 48], personalization_string: Option<&[u8; 48]>) {
    let mut seed_material = *entropy_input;

    if let Some(ps) = personalization_string {
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }
    let ctx = drbg();
    ctx.Key.fill(0x00);
    ctx.V.fill(0x00);
    let mut key = ctx.Key;
    let mut v = ctx.V;
    AES256_CTR_DRBG_Update_impl(Some(&seed_material), &mut key, &mut v);
    ctx.Key = key;
    ctx.V = v;
    ctx.reseed_counter = 1;
}

/// The deterministic `randombytes()` from `rng.c`.
pub fn randombytes_drbg(x: &mut [u8]) -> c_int {
    let ctx = drbg();
    let mut block = [0u8; 16];
    let mut i: usize = 0;
    let mut xlen = x.len();

    while xlen > 0 {
        /* increment V */
        increment(&mut ctx.V[..], 0);

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
    let mut key = ctx.Key;
    let mut v = ctx.V;
    AES256_CTR_DRBG_Update_impl(None, &mut key, &mut v);
    ctx.Key = key;
    ctx.V = v;
    ctx.reseed_counter = ctx.reseed_counter.wrapping_add(1);

    RNG_SUCCESS
}

// ---------------------------------------------------------------------------
// C ABI.  `rng.h` does not rename anything.
// ---------------------------------------------------------------------------

/// `void AES256_ECB(unsigned char *key, unsigned char *ctr, unsigned char *buffer)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_ECB(key: *mut u8, ctr: *mut u8, buffer: *mut u8) {
    unsafe {
        aes256_ecb(
            &*(key as *const [u8; 32]),
            &*(ctr as *const [u8; 16]),
            &mut *(buffer as *mut [u8; 16]),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    Key: *mut u8,
    V: *mut u8,
) {
    unsafe {
        let pd = if provided_data.is_null() {
            None
        } else {
            Some(&*(provided_data as *const [u8; 48]))
        };
        AES256_CTR_DRBG_Update_impl(pd, &mut *(Key as *mut [u8; 32]), &mut *(V as *mut [u8; 16]));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander_init(
    ctx: *mut AES_XOF_struct,
    seed: *mut u8,
    diversifier: *mut u8,
    maxlen: c_ulong,
) -> c_int {
    unsafe {
        seedexpander_init_impl(
            &mut *ctx,
            &*(seed as *const [u8; 32]),
            &*(diversifier as *const [u8; 8]),
            maxlen,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander(
    ctx: *mut AES_XOF_struct,
    x: *mut u8,
    xlen: c_ulong,
) -> c_int {
    unsafe {
        if x.is_null() {
            return RNG_BAD_OUTBUF;
        }
        // The length check happens before any write, exactly as in C, so the
        // slice is only built once the request is known to be in range.
        if xlen >= (*ctx).length_remaining {
            return RNG_BAD_REQ_LEN;
        }
        seedexpander_impl(
            &mut *ctx,
            core::slice::from_raw_parts_mut(x, xlen as usize),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(
    entropy_input: *mut u8,
    personalization_string: *mut u8,
) {
    unsafe {
        let ps = if personalization_string.is_null() {
            None
        } else {
            Some(&*(personalization_string as *const [u8; 48]))
        };
        randombytes_init_impl(&*(entropy_input as *const [u8; 48]), ps);
    }
}

/// `int randombytes(unsigned char *x, unsigned long long xlen)` from `rng.c`.
///
/// Exported unless the `urandom` feature selects the `randombytes.c`
/// implementation instead (the C project ships the two in separate shared
/// libraries, which cannot both be one Rust cdylib).
#[cfg(not(feature = "urandom"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut u8, xlen: core::ffi::c_ulonglong) -> c_int {
    unsafe { randombytes_drbg(core::slice::from_raw_parts_mut(x, xlen as usize)) }
}
