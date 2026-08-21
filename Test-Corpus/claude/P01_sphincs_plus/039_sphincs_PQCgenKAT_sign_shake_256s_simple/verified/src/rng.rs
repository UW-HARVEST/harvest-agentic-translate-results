//! Translation of `app/src/rng.c` — the NIST AES-256 CTR_DRBG used by the
//! deterministic `sphincs_core_det` library and the driver.
//!
//! The OpenSSL AES-256-ECB block encryption is replaced with the pure-Rust
//! `aes` crate (byte-identical, standard AES-256).
//!
//! The global `DRBG_ctx` is exported exactly as the C file does (a
//! `.bss` object of type `AES256_CTR_DRBG_struct`), so that callers observing
//! or mutating it see identical behaviour.

use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes256;
use core::ffi::{c_int, c_ulong};

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

/// `AES256_CTR_DRBG_struct DRBG_ctx;` from `rng.c` (a `.bss` global).
#[unsafe(no_mangle)]
pub static mut DRBG_ctx: AES256_CTR_DRBG_struct = AES256_CTR_DRBG_struct {
    Key: [0u8; 32],
    V: [0u8; 16],
    reseed_counter: 0,
};

/// `seedexpander_init()`
///
/// * `ctx`         - stores the current state of an instance of the seed expander
/// * `seed`        - a 32 byte random value
/// * `diversifier` - an 8 byte diversifier
/// * `maxlen`      - maximum number of bytes (less than 2**32) generated under
///   this seed and diversifier
#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander_init(
    ctx: *mut AES_XOF_struct,
    seed: *mut u8,
    diversifier: *mut u8,
    mut maxlen: c_ulong,
) -> c_int {
    if maxlen >= 0x1_0000_0000 {
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

/// `seedexpander()`
///
/// * `ctx`  - stores the current state of an instance of the seed expander
/// * `x`    - returns the XOF data
/// * `xlen` - number of bytes to return
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

    let mut offset: c_ulong = 0;
    while xlen > 0 {
        // `16 - ctx->buffer_pos` in C is unsigned long arithmetic and wraps.
        let avail: c_ulong = (16 as c_ulong).wrapping_sub(ctx.buffer_pos);
        if xlen <= avail {
            // buffer has what we need
            core::ptr::copy_nonoverlapping(
                ctx.buffer.as_ptr().add(ctx.buffer_pos as usize),
                x.add(offset as usize),
                xlen as usize,
            );
            ctx.buffer_pos += xlen;
            return RNG_SUCCESS;
        }

        // take what's in the buffer
        core::ptr::copy_nonoverlapping(
            ctx.buffer.as_ptr().add(ctx.buffer_pos as usize),
            x.add(offset as usize),
            avail as usize,
        );
        xlen = xlen.wrapping_sub(avail);
        offset = offset.wrapping_add(avail);

        AES256_ECB(
            ctx.key.as_mut_ptr(),
            ctx.ctr.as_mut_ptr(),
            ctx.buffer.as_mut_ptr(),
        );
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

/// AES-256-ECB single block encryption.
///
/// * `key`    - 256-bit AES key
/// * `ctr`    - a 128-bit plaintext value
/// * `buffer` - a 128-bit ciphertext value
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_ECB(key: *mut u8, ctr: *mut u8, buffer: *mut u8) {
    let mut k = [0u8; 32];
    k.copy_from_slice(core::slice::from_raw_parts(key, 32));
    let cipher = Aes256::new(GenericArray::from_slice(&k));
    let mut block = *GenericArray::<u8, aes::cipher::consts::U16>::from_slice(
        core::slice::from_raw_parts(ctr, 16),
    );
    cipher.encrypt_block(&mut block);
    core::ptr::copy_nonoverlapping(block.as_ptr(), buffer, 16);
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
    let ctx = &raw mut DRBG_ctx;
    (*ctx).Key = [0u8; 32];
    (*ctx).V = [0u8; 16];
    AES256_CTR_DRBG_Update(
        seed_material.as_mut_ptr(),
        (*ctx).Key.as_mut_ptr(),
        (*ctx).V.as_mut_ptr(),
    );
    (*ctx).reseed_counter = 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut u8, mut xlen: u64) -> c_int {
    let mut block = [0u8; 16];
    let mut i: usize = 0;
    let ctx = &raw mut DRBG_ctx;

    while xlen > 0 {
        // increment V
        for j in (0..16).rev() {
            if (*ctx).V[j] == 0xff {
                (*ctx).V[j] = 0x00;
            } else {
                (*ctx).V[j] += 1;
                break;
            }
        }
        AES256_ECB(
            (*ctx).Key.as_mut_ptr(),
            (*ctx).V.as_mut_ptr(),
            block.as_mut_ptr(),
        );
        if xlen > 15 {
            core::ptr::copy_nonoverlapping(block.as_ptr(), x.add(i), 16);
            i += 16;
            xlen -= 16;
        } else {
            core::ptr::copy_nonoverlapping(block.as_ptr(), x.add(i), xlen as usize);
            xlen = 0;
        }
    }
    AES256_CTR_DRBG_Update(
        core::ptr::null_mut(),
        (*ctx).Key.as_mut_ptr(),
        (*ctx).V.as_mut_ptr(),
    );
    (*ctx).reseed_counter += 1;

    RNG_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    key: *mut u8,
    v: *mut u8,
) {
    let mut temp = [0u8; 48];

    for i in 0..3usize {
        // increment V
        for j in (0..16isize).rev() {
            if *v.offset(j) == 0xff {
                *v.offset(j) = 0x00;
            } else {
                *v.offset(j) += 1;
                break;
            }
        }

        AES256_ECB(key, v, temp.as_mut_ptr().add(16 * i));
    }
    if !provided_data.is_null() {
        for i in 0..48usize {
            temp[i] ^= *provided_data.add(i);
        }
    }
    core::ptr::copy_nonoverlapping(temp.as_ptr(), key, 32);
    core::ptr::copy_nonoverlapping(temp.as_ptr().add(32), v, 16);
}
