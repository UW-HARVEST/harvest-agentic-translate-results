//! Translation of `app/src/rng.c` — the NIST AES-256 CTR_DRBG used by the
//! deterministic `sphincs_core_det` library and the driver.
//!
//! The OpenSSL AES-256-ECB block encryption is replaced with the pure-Rust
//! `aes` crate (byte-identical, standard AES-256).

use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes256;
use core::ffi::{c_int, c_ulong};
use std::sync::Mutex;

pub const RNG_SUCCESS: c_int = 0;
pub const RNG_BAD_MAXLEN: c_int = -1;
pub const RNG_BAD_OUTBUF: c_int = -2;
pub const RNG_BAD_REQ_LEN: c_int = -3;

#[repr(C)]
pub struct AES_XOF_struct {
    pub buffer: [u8; 16],
    pub buffer_pos: c_ulong,
    pub length_remaining: c_ulong,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

#[repr(C)]
pub struct AES256_CTR_DRBG_struct {
    pub Key: [u8; 32],
    pub V: [u8; 16],
    pub reseed_counter: c_int,
}

struct Drbg {
    key: [u8; 32],
    v: [u8; 16],
    reseed_counter: c_int,
}

static DRBG_CTX: Mutex<Drbg> = Mutex::new(Drbg {
    key: [0u8; 32],
    v: [0u8; 16],
    reseed_counter: 0,
});

/// AES-256-ECB single-block encryption: `buffer = AES256_enc(key, ctr)`.
fn aes256_ecb(key: &[u8; 32], ctr: &[u8; 16], buffer: &mut [u8; 16]) {
    let cipher = Aes256::new(GenericArray::from_slice(key));
    let mut block = *GenericArray::from_slice(ctr);
    cipher.encrypt_block(&mut block);
    buffer.copy_from_slice(block.as_slice());
}

/// The CTR_DRBG update function operating on a key/V pair.
fn drbg_update(provided_data: Option<&[u8; 48]>, key: &mut [u8; 32], v: &mut [u8; 16]) {
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
    key.copy_from_slice(&temp[0..32]);
    v.copy_from_slice(&temp[32..48]);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    key: *mut u8,
    v: *mut u8,
) {
    let mut k = [0u8; 32];
    let mut vv = [0u8; 16];
    k.copy_from_slice(core::slice::from_raw_parts(key, 32));
    vv.copy_from_slice(core::slice::from_raw_parts(v, 16));

    let pd_storage;
    let pd = if provided_data.is_null() {
        None
    } else {
        let mut buf = [0u8; 48];
        buf.copy_from_slice(core::slice::from_raw_parts(provided_data, 48));
        pd_storage = buf;
        Some(&pd_storage)
    };

    drbg_update(pd, &mut k, &mut vv);

    core::slice::from_raw_parts_mut(key, 32).copy_from_slice(&k);
    core::slice::from_raw_parts_mut(v, 16).copy_from_slice(&vv);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander_init(
    ctx: *mut AES_XOF_struct,
    seed: *mut u8,
    diversifier: *mut u8,
    mut maxlen: c_ulong,
) -> c_int {
    if maxlen >= 0x100000000 {
        return RNG_BAD_MAXLEN;
    }

    let ctx = &mut *ctx;
    ctx.length_remaining = maxlen;

    ctx.key
        .copy_from_slice(core::slice::from_raw_parts(seed, 32));

    ctx.ctr[0..8].copy_from_slice(core::slice::from_raw_parts(diversifier, 8));
    ctx.ctr[11] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[10] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[9] = (maxlen % 256) as u8;
    maxlen >>= 8;
    ctx.ctr[8] = (maxlen % 256) as u8;
    for b in ctx.ctr[12..16].iter_mut() {
        *b = 0x00;
    }

    ctx.buffer_pos = 16;
    ctx.buffer = [0u8; 16];

    RNG_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander(
    ctx: *mut AES_XOF_struct,
    x: *mut u8,
    mut xlen: c_ulong,
) -> c_int {
    if x.is_null() {
        return RNG_BAD_OUTBUF;
    }
    let ctx = &mut *ctx;
    if xlen >= ctx.length_remaining {
        return RNG_BAD_REQ_LEN;
    }

    ctx.length_remaining -= xlen;

    let mut offset: usize = 0;
    while xlen > 0 {
        let avail = 16 - ctx.buffer_pos as usize;
        if xlen as usize <= avail {
            // buffer has what we need
            core::ptr::copy_nonoverlapping(
                ctx.buffer.as_ptr().add(ctx.buffer_pos as usize),
                x.add(offset),
                xlen as usize,
            );
            ctx.buffer_pos += xlen;
            return RNG_SUCCESS;
        }

        // take what's in the buffer
        core::ptr::copy_nonoverlapping(
            ctx.buffer.as_ptr().add(ctx.buffer_pos as usize),
            x.add(offset),
            avail,
        );
        xlen -= avail as c_ulong;
        offset += avail;

        let key = ctx.key;
        let ctr = ctx.ctr;
        let mut buf = [0u8; 16];
        aes256_ecb(&key, &ctr, &mut buf);
        ctx.buffer = buf;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(
    entropy_input: *mut u8,
    personalization_string: *mut u8,
) {
    let mut seed_material = [0u8; 48];
    seed_material.copy_from_slice(core::slice::from_raw_parts(entropy_input, 48));
    if !personalization_string.is_null() {
        let ps = core::slice::from_raw_parts(personalization_string, 48);
        for i in 0..48 {
            seed_material[i] ^= ps[i];
        }
    }

    let mut drbg = DRBG_CTX.lock().unwrap();
    drbg.key = [0u8; 32];
    drbg.v = [0u8; 16];
    let mut k = drbg.key;
    let mut v = drbg.v;
    drbg_update(Some(&seed_material), &mut k, &mut v);
    drbg.key = k;
    drbg.v = v;
    drbg.reseed_counter = 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut u8, mut xlen: u64) -> c_int {
    let mut drbg = DRBG_CTX.lock().unwrap();
    let mut i: usize = 0;

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
        let key = drbg.key;
        let v = drbg.v;
        let mut block = [0u8; 16];
        aes256_ecb(&key, &v, &mut block);

        if xlen > 15 {
            core::ptr::copy_nonoverlapping(block.as_ptr(), x.add(i), 16);
            i += 16;
            xlen -= 16;
        } else {
            core::ptr::copy_nonoverlapping(block.as_ptr(), x.add(i), xlen as usize);
            xlen = 0;
        }
    }

    let mut k = drbg.key;
    let mut v = drbg.v;
    drbg_update(None, &mut k, &mut v);
    drbg.key = k;
    drbg.v = v;
    drbg.reseed_counter += 1;

    RNG_SUCCESS
}
