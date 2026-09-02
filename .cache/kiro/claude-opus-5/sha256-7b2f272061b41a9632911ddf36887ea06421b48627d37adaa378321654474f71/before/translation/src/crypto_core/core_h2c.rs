//! Translation of `crypto_core/ed25519/core_h2c.c` and
//! `crypto_core/ed25519/core_h2c.h`.
//!
//! `private/quirks.h` renames `core_h2c_string_to_hash` to
//! `_sodium_core_h2c_string_to_hash`, which is the only exported symbol.

use core::ffi::c_int;

use crate::common::memcpy;
use crate::crypto_hash::sha256::{
    crypto_hash_sha256_final, crypto_hash_sha256_init, crypto_hash_sha256_state,
    crypto_hash_sha256_update, crypto_hash_sha256_BYTES,
};
use crate::crypto_hash::sha512::{
    crypto_hash_sha512_final, crypto_hash_sha512_init, crypto_hash_sha512_state,
    crypto_hash_sha512_update, crypto_hash_sha512_BYTES,
};

pub const CORE_H2C_SHA256: c_int = 1;
pub const CORE_H2C_SHA512: c_int = 2;

const HASH_SHA256_BYTES: usize = crypto_hash_sha256_BYTES;
const HASH_SHA256_BLOCKBYTES: usize = 64;

unsafe fn core_h2c_string_to_hash_sha256(
    h: *mut u8,
    h_len: usize,
    mut ctx: *const u8,
    mut ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
) -> c_int {
    let mut st: crypto_hash_sha256_state = core::mem::zeroed();
    let empty_block: [u8; HASH_SHA256_BLOCKBYTES] = [0; HASH_SHA256_BLOCKBYTES];
    let mut u0: [u8; HASH_SHA256_BYTES] = [0; HASH_SHA256_BYTES];
    let mut ux: [u8; HASH_SHA256_BYTES] = [0; HASH_SHA256_BYTES];
    let mut t: [u8; 3] = [0u8, h_len as u8, 0u8];
    let ctx_len_u8: u8;
    let mut i: usize;
    let mut j: usize;

    /* assert(h_len <= 0xff); */
    if ctx_len > 0xff {
        crypto_hash_sha256_init(&mut st);
        crypto_hash_sha256_update(
            &mut st,
            b"H2C-OVERSIZE-DST-".as_ptr(),
            (b"H2C-OVERSIZE-DST-".len()) as u64,
        );
        crypto_hash_sha256_update(&mut st, ctx, ctx_len as u64);
        crypto_hash_sha256_final(&mut st, u0.as_mut_ptr());
        ctx = u0.as_ptr();
        ctx_len = HASH_SHA256_BYTES;
    }
    ctx_len_u8 = ctx_len as u8;
    crypto_hash_sha256_init(&mut st);
    crypto_hash_sha256_update(
        &mut st,
        empty_block.as_ptr(),
        core::mem::size_of_val(&empty_block) as u64,
    );
    crypto_hash_sha256_update(&mut st, msg, msg_len as u64);
    crypto_hash_sha256_update(&mut st, t.as_ptr(), 3u64);
    crypto_hash_sha256_update(&mut st, ctx, ctx_len as u64);
    crypto_hash_sha256_update(&mut st, &ctx_len_u8, 1u64);
    crypto_hash_sha256_final(&mut st, u0.as_mut_ptr());

    i = 0;
    while i < h_len {
        j = 0;
        while j < HASH_SHA256_BYTES {
            ux[j] ^= u0[j];
            j += 1;
        }
        t[2] = t[2].wrapping_add(1);
        crypto_hash_sha256_init(&mut st);
        crypto_hash_sha256_update(&mut st, ux.as_ptr(), HASH_SHA256_BYTES as u64);
        crypto_hash_sha256_update(&mut st, &t[2], 1u64);
        crypto_hash_sha256_update(&mut st, ctx, ctx_len as u64);
        crypto_hash_sha256_update(&mut st, &ctx_len_u8, 1u64);
        crypto_hash_sha256_final(&mut st, ux.as_mut_ptr());
        let n = if h_len - i >= HASH_SHA256_BYTES {
            HASH_SHA256_BYTES
        } else {
            h_len - i
        };
        memcpy(h.add(i), ux.as_ptr(), n);

        i += HASH_SHA256_BYTES;
    }
    0
}

const HASH_SHA512_BYTES: usize = crypto_hash_sha512_BYTES;
const HASH_SHA512_BLOCKBYTES: usize = 128;

unsafe fn core_h2c_string_to_hash_sha512(
    h: *mut u8,
    h_len: usize,
    mut ctx: *const u8,
    mut ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
) -> c_int {
    let mut st: crypto_hash_sha512_state = core::mem::zeroed();
    let empty_block: [u8; HASH_SHA512_BLOCKBYTES] = [0; HASH_SHA512_BLOCKBYTES];
    let mut u0: [u8; HASH_SHA512_BYTES] = [0; HASH_SHA512_BYTES];
    let mut ux: [u8; HASH_SHA512_BYTES] = [0; HASH_SHA512_BYTES];
    let mut t: [u8; 3] = [0u8, h_len as u8, 0u8];
    let ctx_len_u8: u8;
    let mut i: usize;
    let mut j: usize;

    /* assert(h_len <= 0xff); */
    if ctx_len > 0xff {
        crypto_hash_sha512_init(&mut st);
        crypto_hash_sha512_update(
            &mut st,
            b"H2C-OVERSIZE-DST-".as_ptr(),
            (b"H2C-OVERSIZE-DST-".len()) as u64,
        );
        crypto_hash_sha512_update(&mut st, ctx, ctx_len as u64);
        crypto_hash_sha512_final(&mut st, u0.as_mut_ptr());
        ctx = u0.as_ptr();
        ctx_len = HASH_SHA512_BYTES;
    }
    ctx_len_u8 = ctx_len as u8;
    crypto_hash_sha512_init(&mut st);
    crypto_hash_sha512_update(
        &mut st,
        empty_block.as_ptr(),
        core::mem::size_of_val(&empty_block) as u64,
    );
    crypto_hash_sha512_update(&mut st, msg, msg_len as u64);
    crypto_hash_sha512_update(&mut st, t.as_ptr(), 3u64);
    crypto_hash_sha512_update(&mut st, ctx, ctx_len as u64);
    crypto_hash_sha512_update(&mut st, &ctx_len_u8, 1u64);
    crypto_hash_sha512_final(&mut st, u0.as_mut_ptr());

    i = 0;
    while i < h_len {
        j = 0;
        while j < HASH_SHA512_BYTES {
            ux[j] ^= u0[j];
            j += 1;
        }
        t[2] = t[2].wrapping_add(1);
        crypto_hash_sha512_init(&mut st);
        crypto_hash_sha512_update(&mut st, ux.as_ptr(), HASH_SHA512_BYTES as u64);
        crypto_hash_sha512_update(&mut st, &t[2], 1u64);
        crypto_hash_sha512_update(&mut st, ctx, ctx_len as u64);
        crypto_hash_sha512_update(&mut st, &ctx_len_u8, 1u64);
        crypto_hash_sha512_final(&mut st, ux.as_mut_ptr());
        let n = if h_len - i >= HASH_SHA512_BYTES {
            HASH_SHA512_BYTES
        } else {
            h_len - i
        };
        memcpy(h.add(i), ux.as_ptr(), n);

        i += HASH_SHA512_BYTES;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_core_h2c_string_to_hash(
    h: *mut u8,
    h_len: usize,
    ctx: *const u8,
    ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
    hash_alg: c_int,
) -> c_int {
    match hash_alg {
        CORE_H2C_SHA256 => {
            core_h2c_string_to_hash_sha256(h, h_len, ctx, ctx_len, msg, msg_len)
        }
        CORE_H2C_SHA512 => {
            core_h2c_string_to_hash_sha512(h, h_len, ctx, ctx_len, msg, msg_len)
        }
        _ => {
            crate::set_errno(crate::EINVAL);
            -1
        }
    }
}
