//! core_h2c (core_h2c.c): expand_message for hash-to-curve using SHA256/SHA512.
use crate::ed25519::sha512::{
    crypto_hash_sha512_final, crypto_hash_sha512_init, crypto_hash_sha512_update,
    crypto_hash_sha512_state,
};
use core::ffi::c_int;

pub const CORE_H2C_SHA256: c_int = 1;
pub const CORE_H2C_SHA512: c_int = 2;

#[repr(C)]
struct crypto_hash_sha256_state {
    state: [u32; 8],
    count: u64,
    buf: [u8; 64],
}

extern "C" {
    fn crypto_hash_sha256_init(state: *mut crypto_hash_sha256_state) -> c_int;
    fn crypto_hash_sha256_update(
        state: *mut crypto_hash_sha256_state,
        input: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha256_final(state: *mut crypto_hash_sha256_state, out: *mut u8) -> c_int;
}

const SHA256_BYTES: usize = 32;
const SHA512_BYTES: usize = 64;

unsafe fn string_to_hash_sha256(
    h: *mut u8,
    h_len: usize,
    mut ctx: *const u8,
    mut ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
) -> c_int {
    let empty_block = [0u8; 64];
    let mut u0 = [0u8; SHA256_BYTES];
    let mut ux = [0u8; SHA256_BYTES];
    let mut t: [u8; 3] = [0, h_len as u8, 0];

    let mut st = crypto_hash_sha256_state {
        state: [0; 8],
        count: 0,
        buf: [0; 64],
    };
    if ctx_len > 0xff {
        crypto_hash_sha256_init(&mut st);
        let prefix = b"H2C-OVERSIZE-DST-";
        crypto_hash_sha256_update(&mut st, prefix.as_ptr(), prefix.len() as u64);
        crypto_hash_sha256_update(&mut st, ctx, ctx_len as u64);
        crypto_hash_sha256_final(&mut st, u0.as_mut_ptr());
        ctx = u0.as_ptr();
        ctx_len = SHA256_BYTES;
    }
    let ctx_len_u8 = ctx_len as u8;
    crypto_hash_sha256_init(&mut st);
    crypto_hash_sha256_update(&mut st, empty_block.as_ptr(), empty_block.len() as u64);
    crypto_hash_sha256_update(&mut st, msg, msg_len as u64);
    crypto_hash_sha256_update(&mut st, t.as_ptr(), 3);
    crypto_hash_sha256_update(&mut st, ctx, ctx_len as u64);
    crypto_hash_sha256_update(&mut st, &ctx_len_u8, 1);
    crypto_hash_sha256_final(&mut st, u0.as_mut_ptr());

    let mut i = 0usize;
    while i < h_len {
        for j in 0..SHA256_BYTES {
            ux[j] ^= u0[j];
        }
        t[2] = t[2].wrapping_add(1);
        crypto_hash_sha256_init(&mut st);
        crypto_hash_sha256_update(&mut st, ux.as_ptr(), SHA256_BYTES as u64);
        crypto_hash_sha256_update(&mut st, &t[2], 1);
        crypto_hash_sha256_update(&mut st, ctx, ctx_len as u64);
        crypto_hash_sha256_update(&mut st, &ctx_len_u8, 1);
        crypto_hash_sha256_final(&mut st, ux.as_mut_ptr());
        let n = if h_len - i >= SHA256_BYTES {
            SHA256_BYTES
        } else {
            h_len - i
        };
        core::ptr::copy_nonoverlapping(ux.as_ptr(), h.add(i), n);
        i += SHA256_BYTES;
    }
    0
}

unsafe fn string_to_hash_sha512(
    h: *mut u8,
    h_len: usize,
    mut ctx: *const u8,
    mut ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
) -> c_int {
    let empty_block = [0u8; 128];
    let mut u0 = [0u8; SHA512_BYTES];
    let mut ux = [0u8; SHA512_BYTES];
    let mut t: [u8; 3] = [0, h_len as u8, 0];

    let mut st = crypto_hash_sha512_state {
        state: [0; 8],
        count: [0; 2],
        buf: [0; 128],
    };
    if ctx_len > 0xff {
        crypto_hash_sha512_init(&mut st);
        let prefix = b"H2C-OVERSIZE-DST-";
        crypto_hash_sha512_update(&mut st, prefix.as_ptr(), prefix.len() as u64);
        crypto_hash_sha512_update(&mut st, ctx, ctx_len as u64);
        crypto_hash_sha512_final(&mut st, u0.as_mut_ptr());
        ctx = u0.as_ptr();
        ctx_len = SHA512_BYTES;
    }
    let ctx_len_u8 = ctx_len as u8;
    crypto_hash_sha512_init(&mut st);
    crypto_hash_sha512_update(&mut st, empty_block.as_ptr(), empty_block.len() as u64);
    crypto_hash_sha512_update(&mut st, msg, msg_len as u64);
    crypto_hash_sha512_update(&mut st, t.as_ptr(), 3);
    crypto_hash_sha512_update(&mut st, ctx, ctx_len as u64);
    crypto_hash_sha512_update(&mut st, &ctx_len_u8, 1);
    crypto_hash_sha512_final(&mut st, u0.as_mut_ptr());

    let mut i = 0usize;
    while i < h_len {
        for j in 0..SHA512_BYTES {
            ux[j] ^= u0[j];
        }
        t[2] = t[2].wrapping_add(1);
        crypto_hash_sha512_init(&mut st);
        crypto_hash_sha512_update(&mut st, ux.as_ptr(), SHA512_BYTES as u64);
        crypto_hash_sha512_update(&mut st, &t[2], 1);
        crypto_hash_sha512_update(&mut st, ctx, ctx_len as u64);
        crypto_hash_sha512_update(&mut st, &ctx_len_u8, 1);
        crypto_hash_sha512_final(&mut st, ux.as_mut_ptr());
        let n = if h_len - i >= SHA512_BYTES {
            SHA512_BYTES
        } else {
            h_len - i
        };
        core::ptr::copy_nonoverlapping(ux.as_ptr(), h.add(i), n);
        i += SHA512_BYTES;
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
        CORE_H2C_SHA256 => string_to_hash_sha256(h, h_len, ctx, ctx_len, msg, msg_len),
        CORE_H2C_SHA512 => string_to_hash_sha512(h, h_len, ctx, ctx_len, msg, msg_len),
        _ => -1,
    }
}

/// Internal callable name (renamed to _sodium_ prefix in C).
pub unsafe fn core_h2c_string_to_hash(
    h: *mut u8,
    h_len: usize,
    ctx: *const u8,
    ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
    hash_alg: c_int,
) -> c_int {
    _sodium_core_h2c_string_to_hash(h, h_len, ctx, ctx_len, msg, msg_len, hash_alg)
}
