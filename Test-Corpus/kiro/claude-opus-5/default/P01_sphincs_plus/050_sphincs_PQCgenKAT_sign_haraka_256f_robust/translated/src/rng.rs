//
//  rng.rs
//
//  Translated from app/src/rng.c (originally by Bassham, Lawrence E (Fed)).
//
//  The C original uses OpenSSL EVP for AES-256-ECB. Here we replace that with
//  the pure-Rust `aes` crate (0.8). No OpenSSL or other C library is used.
//

use core::ffi::{c_int, c_ulong, c_ulonglong};

use aes::Aes256;
use aes::cipher::{BlockEncrypt, KeyInit};
use aes::cipher::generic_array::GenericArray;

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

// C global `AES256_CTR_DRBG_struct DRBG_ctx;` is zero-initialised.
#[unsafe(no_mangle)]
pub static mut DRBG_ctx: AES256_CTR_DRBG_struct = AES256_CTR_DRBG_struct {
    Key: [0u8; 32],
    V: [0u8; 16],
    reseed_counter: 0,
};

/*
 seedexpander_init()
 ctx            - stores the current state of an instance of the seed expander
 seed           - a 32 byte random value
 diversifier    - an 8 byte diversifier
 maxlen         - maximum number of bytes (less than 2**32) generated under this seed and diversifier
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander_init(
    ctx: *mut AES_XOF_struct,
    seed: *mut u8,
    diversifier: *mut u8,
    maxlen: c_ulong,
) -> c_int {
    let mut maxlen = maxlen;

    if maxlen >= 0x100000000 {
        return RNG_BAD_MAXLEN;
    }

    (*ctx).length_remaining = maxlen;

    core::ptr::copy_nonoverlapping(seed, (*ctx).key.as_mut_ptr(), 32);

    core::ptr::copy_nonoverlapping(diversifier, (*ctx).ctr.as_mut_ptr(), 8);
    (*ctx).ctr[11] = (maxlen % 256) as u8;
    maxlen >>= 8;
    (*ctx).ctr[10] = (maxlen % 256) as u8;
    maxlen >>= 8;
    (*ctx).ctr[9] = (maxlen % 256) as u8;
    maxlen >>= 8;
    (*ctx).ctr[8] = (maxlen % 256) as u8;
    core::ptr::write_bytes((*ctx).ctr.as_mut_ptr().add(12), 0x00, 4);

    (*ctx).buffer_pos = 16;
    core::ptr::write_bytes((*ctx).buffer.as_mut_ptr(), 0x00, 16);

    RNG_SUCCESS
}

/*
 seedexpander()
    ctx  - stores the current state of an instance of the seed expander
    x    - returns the XOF data
    xlen - number of bytes to return
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander(
    ctx: *mut AES_XOF_struct,
    x: *mut u8,
    xlen: c_ulong,
) -> c_int {
    let mut xlen = xlen;
    let mut offset: c_ulong;

    if x.is_null() {
        return RNG_BAD_OUTBUF;
    }
    // Faithful reproduction of the C off-by-one behaviour.
    if xlen >= (*ctx).length_remaining {
        return RNG_BAD_REQ_LEN;
    }

    (*ctx).length_remaining -= xlen;

    offset = 0;
    while xlen > 0 {
        if xlen <= (16 - (*ctx).buffer_pos) {
            // buffer has what we need
            core::ptr::copy_nonoverlapping(
                (*ctx).buffer.as_ptr().add((*ctx).buffer_pos as usize),
                x.add(offset as usize),
                xlen as usize,
            );
            (*ctx).buffer_pos += xlen;

            return RNG_SUCCESS;
        }

        // take what's in the buffer
        core::ptr::copy_nonoverlapping(
            (*ctx).buffer.as_ptr().add((*ctx).buffer_pos as usize),
            x.add(offset as usize),
            (16 - (*ctx).buffer_pos) as usize,
        );
        xlen -= 16 - (*ctx).buffer_pos;
        offset += 16 - (*ctx).buffer_pos;

        let c = ctx;
        AES256_ECB(
            (*c).key.as_mut_ptr(),
            (*c).ctr.as_mut_ptr(),
            (*c).buffer.as_mut_ptr(),
        );
        (*ctx).buffer_pos = 0;

        // increment the counter
        let mut i: i32 = 15;
        while i >= 12 {
            if (*ctx).ctr[i as usize] == 0xff {
                (*ctx).ctr[i as usize] = 0x00;
            } else {
                (*ctx).ctr[i as usize] += 1;
                break;
            }
            i -= 1;
        }
    }

    RNG_SUCCESS
}

// NOTE: The C `handleErrors()` function reports an OpenSSL error and aborts.
// With a pure-Rust AES implementation there is no OpenSSL error path, so this
// function is unreachable and has been intentionally omitted.

// Use whatever AES implementation you have. This uses AES from the pure-Rust
// `aes` crate.
//    key - 256-bit AES key
//    ctr - a 128-bit plaintext value
//    buffer - a 128-bit ciphertext value
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_ECB(key: *mut u8, ctr: *mut u8, buffer: *mut u8) {
    let key_arr = core::slice::from_raw_parts(key, 32);
    let cipher = Aes256::new(GenericArray::from_slice(key_arr));
    let mut block = GenericArray::clone_from_slice(core::slice::from_raw_parts(ctr, 16));
    cipher.encrypt_block(&mut block);
    core::ptr::copy_nonoverlapping(block.as_ptr(), buffer, 16);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(
    entropy_input: *mut u8,
    personalization_string: *mut u8,
) {
    let mut seed_material: [u8; 48] = [0u8; 48];

    core::ptr::copy_nonoverlapping(entropy_input, seed_material.as_mut_ptr(), 48);
    if !personalization_string.is_null() {
        for i in 0..48 {
            seed_material[i] ^= *personalization_string.add(i);
        }
    }
    let d = core::ptr::addr_of_mut!(DRBG_ctx);
    core::ptr::write_bytes((*d).Key.as_mut_ptr(), 0x00, 32);
    core::ptr::write_bytes((*d).V.as_mut_ptr(), 0x00, 16);
    let key = core::ptr::addr_of_mut!((*d).Key) as *mut u8;
    let v = core::ptr::addr_of_mut!((*d).V) as *mut u8;
    AES256_CTR_DRBG_Update(seed_material.as_mut_ptr(), key, v);
    (*d).reseed_counter = 1;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut u8, xlen: c_ulonglong) -> c_int {
    let mut xlen = xlen;
    let mut block: [u8; 16] = [0u8; 16];
    let mut i: c_int = 0;

    let d = core::ptr::addr_of_mut!(DRBG_ctx);

    while xlen > 0 {
        // increment V
        let mut j: i32 = 15;
        while j >= 0 {
            if (*d).V[j as usize] == 0xff {
                (*d).V[j as usize] = 0x00;
            } else {
                (*d).V[j as usize] += 1;
                break;
            }
            j -= 1;
        }
        let key = core::ptr::addr_of_mut!((*d).Key) as *mut u8;
        let v = core::ptr::addr_of_mut!((*d).V) as *mut u8;
        AES256_ECB(key, v, block.as_mut_ptr());
        if xlen > 15 {
            core::ptr::copy_nonoverlapping(block.as_ptr(), x.add(i as usize), 16);
            i += 16;
            xlen -= 16;
        } else {
            core::ptr::copy_nonoverlapping(block.as_ptr(), x.add(i as usize), xlen as usize);
            xlen = 0;
        }
    }
    let key = core::ptr::addr_of_mut!((*d).Key) as *mut u8;
    let v = core::ptr::addr_of_mut!((*d).V) as *mut u8;
    AES256_CTR_DRBG_Update(core::ptr::null_mut(), key, v);
    (*d).reseed_counter += 1;

    RNG_SUCCESS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    Key: *mut u8,
    V: *mut u8,
) {
    let mut temp: [u8; 48] = [0u8; 48];

    for i in 0..3 {
        // increment V
        let mut j: i32 = 15;
        while j >= 0 {
            if *V.add(j as usize) == 0xff {
                *V.add(j as usize) = 0x00;
            } else {
                *V.add(j as usize) += 1;
                break;
            }
            j -= 1;
        }

        AES256_ECB(Key, V, temp.as_mut_ptr().add(16 * i));
    }
    if !provided_data.is_null() {
        for i in 0..48 {
            temp[i] ^= *provided_data.add(i);
        }
    }
    core::ptr::copy_nonoverlapping(temp.as_ptr(), Key, 32);
    core::ptr::copy_nonoverlapping(temp.as_ptr().add(32), V, 16);
}
