//! Translation of `crypto_core/ed25519/core_h2c.c`.
//!
//! `core_h2c_string_to_hash` is renamed to `_sodium_core_h2c_string_to_hash`
//! by `private/quirks.h`.
//!
//! This is the `expand_message_xmd` construction from RFC 9380, instantiated
//! with SHA-256 and SHA-512.

use core::ffi::{c_int, c_ulonglong};

/* core_h2c.h */
const CORE_H2C_SHA256: c_int = 1;
const CORE_H2C_SHA512: c_int = 2;

/* <errno.h> value on Linux/glibc */
const EINVAL: c_int = 22;

/* crypto_hash_sha256.h */
#[repr(C)]
#[derive(Copy, Clone)]
struct crypto_hash_sha256_state {
    state: [u32; 8],
    count: u64,
    buf: [u8; 64],
}

/* crypto_hash_sha512.h */
#[repr(C)]
#[derive(Copy, Clone)]
struct crypto_hash_sha512_state {
    state: [u64; 8],
    count: [u64; 2],
    buf: [u8; 128],
}

extern "C" {
    fn __errno_location() -> *mut c_int;

    /* crypto_hash/sha256/cp/hash_sha256_cp.c */
    fn crypto_hash_sha256_init(state: *mut crypto_hash_sha256_state) -> c_int;
    fn crypto_hash_sha256_update(
        state: *mut crypto_hash_sha256_state,
        in_: *const u8,
        inlen: c_ulonglong,
    ) -> c_int;
    fn crypto_hash_sha256_final(
        state: *mut crypto_hash_sha256_state,
        out: *mut u8,
    ) -> c_int;

    /* crypto_hash/sha512/cp/hash_sha512_cp.c */
    fn crypto_hash_sha512_init(state: *mut crypto_hash_sha512_state) -> c_int;
    fn crypto_hash_sha512_update(
        state: *mut crypto_hash_sha512_state,
        in_: *const u8,
        inlen: c_ulonglong,
    ) -> c_int;
    fn crypto_hash_sha512_final(
        state: *mut crypto_hash_sha512_state,
        out: *mut u8,
    ) -> c_int;
}

#[inline(always)]
unsafe fn set_errno(e: c_int) {
    *__errno_location() = e;
}

/* -------------------------------------------------------------------------
 * SHA-256 variant:  HASH_BYTES = 32U, HASH_BLOCKBYTES = 64U
 * ------------------------------------------------------------------------- */

unsafe fn core_h2c_string_to_hash_sha256(
    h: *mut u8,
    h_len: usize,
    mut ctx: *const u8,
    mut ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
) -> c_int {
    const HASH_BYTES: usize = 32;
    const HASH_BLOCKBYTES: usize = 64;

    let mut st = crypto_hash_sha256_state {
        state: [0; 8],
        count: 0,
        buf: [0; 64],
    };
    let stp: *mut crypto_hash_sha256_state = &mut st;
    let empty_block = [0u8; HASH_BLOCKBYTES];
    let mut u0 = [0u8; HASH_BYTES];
    let mut ux = [0u8; HASH_BYTES];
    let mut t: [u8; 3] = [0u8, h_len as u8, 0u8];
    let ctx_len_u8: u8;
    let mut i: usize;
    let mut j: usize;

    /* assert(h_len <= 0xff); */
    if ctx_len > 0xff {
        crypto_hash_sha256_init(stp);
        crypto_hash_sha256_update(
            stp,
            b"H2C-OVERSIZE-DST-".as_ptr(),
            (b"H2C-OVERSIZE-DST-".len()) as c_ulonglong,
        );
        crypto_hash_sha256_update(stp, ctx, ctx_len as c_ulonglong);
        crypto_hash_sha256_final(stp, u0.as_mut_ptr());
        ctx = u0.as_ptr();
        ctx_len = HASH_BYTES;
    }
    ctx_len_u8 = ctx_len as u8;
    crypto_hash_sha256_init(stp);
    crypto_hash_sha256_update(
        stp,
        empty_block.as_ptr(),
        HASH_BLOCKBYTES as c_ulonglong,
    );
    crypto_hash_sha256_update(stp, msg, msg_len as c_ulonglong);
    crypto_hash_sha256_update(stp, t.as_ptr(), 3);
    crypto_hash_sha256_update(stp, ctx, ctx_len as c_ulonglong);
    crypto_hash_sha256_update(stp, &ctx_len_u8 as *const u8, 1);
    crypto_hash_sha256_final(stp, u0.as_mut_ptr());

    i = 0;
    while i < h_len {
        j = 0;
        while j < HASH_BYTES {
            ux[j] ^= u0[j];
            j += 1;
        }
        t[2] = t[2].wrapping_add(1);
        crypto_hash_sha256_init(stp);
        crypto_hash_sha256_update(stp, ux.as_ptr(), HASH_BYTES as c_ulonglong);
        crypto_hash_sha256_update(stp, (t.as_ptr()).add(2), 1);
        crypto_hash_sha256_update(stp, ctx, ctx_len as c_ulonglong);
        crypto_hash_sha256_update(stp, &ctx_len_u8 as *const u8, 1);
        crypto_hash_sha256_final(stp, ux.as_mut_ptr());
        core::ptr::copy_nonoverlapping(
            ux.as_ptr(),
            h.add(i),
            if h_len - i >= HASH_BYTES { HASH_BYTES } else { h_len - i },
        );
        i += HASH_BYTES;
    }
    0
}

/* -------------------------------------------------------------------------
 * SHA-512 variant:  HASH_BYTES = 64U, HASH_BLOCKBYTES = 128U
 * ------------------------------------------------------------------------- */

unsafe fn core_h2c_string_to_hash_sha512(
    h: *mut u8,
    h_len: usize,
    mut ctx: *const u8,
    mut ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
) -> c_int {
    const HASH_BYTES: usize = 64;
    const HASH_BLOCKBYTES: usize = 128;

    let mut st = crypto_hash_sha512_state {
        state: [0; 8],
        count: [0; 2],
        buf: [0; 128],
    };
    let stp: *mut crypto_hash_sha512_state = &mut st;
    let empty_block = [0u8; HASH_BLOCKBYTES];
    let mut u0 = [0u8; HASH_BYTES];
    let mut ux = [0u8; HASH_BYTES];
    let mut t: [u8; 3] = [0u8, h_len as u8, 0u8];
    let ctx_len_u8: u8;
    let mut i: usize;
    let mut j: usize;

    /* assert(h_len <= 0xff); */
    if ctx_len > 0xff {
        crypto_hash_sha512_init(stp);
        crypto_hash_sha512_update(
            stp,
            b"H2C-OVERSIZE-DST-".as_ptr(),
            (b"H2C-OVERSIZE-DST-".len()) as c_ulonglong,
        );
        crypto_hash_sha512_update(stp, ctx, ctx_len as c_ulonglong);
        crypto_hash_sha512_final(stp, u0.as_mut_ptr());
        ctx = u0.as_ptr();
        ctx_len = HASH_BYTES;
    }
    ctx_len_u8 = ctx_len as u8;
    crypto_hash_sha512_init(stp);
    crypto_hash_sha512_update(
        stp,
        empty_block.as_ptr(),
        HASH_BLOCKBYTES as c_ulonglong,
    );
    crypto_hash_sha512_update(stp, msg, msg_len as c_ulonglong);
    crypto_hash_sha512_update(stp, t.as_ptr(), 3);
    crypto_hash_sha512_update(stp, ctx, ctx_len as c_ulonglong);
    crypto_hash_sha512_update(stp, &ctx_len_u8 as *const u8, 1);
    crypto_hash_sha512_final(stp, u0.as_mut_ptr());

    i = 0;
    while i < h_len {
        j = 0;
        while j < HASH_BYTES {
            ux[j] ^= u0[j];
            j += 1;
        }
        t[2] = t[2].wrapping_add(1);
        crypto_hash_sha512_init(stp);
        crypto_hash_sha512_update(stp, ux.as_ptr(), HASH_BYTES as c_ulonglong);
        crypto_hash_sha512_update(stp, (t.as_ptr()).add(2), 1);
        crypto_hash_sha512_update(stp, ctx, ctx_len as c_ulonglong);
        crypto_hash_sha512_update(stp, &ctx_len_u8 as *const u8, 1);
        crypto_hash_sha512_final(stp, ux.as_mut_ptr());
        core::ptr::copy_nonoverlapping(
            ux.as_ptr(),
            h.add(i),
            if h_len - i >= HASH_BYTES { HASH_BYTES } else { h_len - i },
        );
        i += HASH_BYTES;
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
            set_errno(EINVAL);
            -1
        }
    }
}
