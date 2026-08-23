//! Translation of `crypto_core/ed25519/core_h2c.c`.
//!
//! `core_h2c_string_to_hash` is renamed to `_sodium_core_h2c_string_to_hash`
//! by `include/sodium/private/quirks.h`.
//!
//! `NDEBUG` is set in the reference build, so `assert(h_len <= 0xff)` is a
//! no-op and is not translated.

#![allow(unsafe_op_in_unsafe_fn)]

use core::ffi::c_int;

use crate::common::{memcpy, set_errno};

/// `#define CORE_H2C_SHA256 1`
pub const CORE_H2C_SHA256: c_int = 1;
/// `#define CORE_H2C_SHA512 2`
pub const CORE_H2C_SHA512: c_int = 2;

/// `EINVAL` on Linux.
const EINVAL: c_int = 22;

/// ```c
/// typedef struct crypto_hash_sha256_state {
///     uint32_t state[8];
///     uint64_t count;
///     uint8_t  buf[64];
/// } crypto_hash_sha256_state;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
struct crypto_hash_sha256_state {
    state: [u32; 8],
    count: u64,
    buf: [u8; 64],
}

/// ```c
/// typedef struct crypto_hash_sha512_state {
///     uint64_t state[8];
///     uint64_t count[2];
///     uint8_t  buf[128];
/// } crypto_hash_sha512_state;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
struct crypto_hash_sha512_state {
    state: [u64; 8],
    count: [u64; 2],
    buf: [u8; 128],
}

unsafe extern "C" {
    fn crypto_hash_sha256_init(state: *mut crypto_hash_sha256_state) -> c_int;
    fn crypto_hash_sha256_update(
        state: *mut crypto_hash_sha256_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha256_final(state: *mut crypto_hash_sha256_state, out: *mut u8) -> c_int;

    fn crypto_hash_sha512_init(state: *mut crypto_hash_sha512_state) -> c_int;
    fn crypto_hash_sha512_update(
        state: *mut crypto_hash_sha512_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha512_final(state: *mut crypto_hash_sha512_state, out: *mut u8) -> c_int;
}

/// `H2C-OVERSIZE-DST-` without the terminating NUL.
const OVERSIZE_DST: &[u8; 17] = b"H2C-OVERSIZE-DST-";

/// ```c
/// static int
/// core_h2c_string_to_hash_sha256(unsigned char *h, const size_t h_len,
///                                const unsigned char *ctx, size_t ctx_len,
///                                const unsigned char *msg, size_t msg_len)
/// ```
///
/// `HASH_BYTES = crypto_hash_sha256_BYTES = 32`, `HASH_BLOCKBYTES = 64`.
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

    let mut st = core::mem::MaybeUninit::<crypto_hash_sha256_state>::uninit();
    let st = st.as_mut_ptr();
    let empty_block: [u8; HASH_BLOCKBYTES] = [0; HASH_BLOCKBYTES];
    let mut u0: [u8; HASH_BYTES] = [0; HASH_BYTES];
    let mut ux: [u8; HASH_BYTES] = [0; HASH_BYTES];
    let mut t: [u8; 3] = [0u8, h_len as u8, 0u8];
    let ctx_len_u8: u8;
    let mut i: usize;
    let mut j: usize;

    /* assert(h_len <= 0xff); -- NDEBUG */
    if ctx_len > 0xff_usize {
        crypto_hash_sha256_init(st);
        crypto_hash_sha256_update(st, OVERSIZE_DST.as_ptr(), OVERSIZE_DST.len() as u64);
        crypto_hash_sha256_update(st, ctx, ctx_len as u64);
        crypto_hash_sha256_final(st, u0.as_mut_ptr());
        ctx = u0.as_ptr();
        ctx_len = HASH_BYTES;
    }
    ctx_len_u8 = ctx_len as u8;
    crypto_hash_sha256_init(st);
    crypto_hash_sha256_update(st, empty_block.as_ptr(), empty_block.len() as u64);
    crypto_hash_sha256_update(st, msg, msg_len as u64);
    crypto_hash_sha256_update(st, t.as_ptr(), 3u64);
    crypto_hash_sha256_update(st, ctx, ctx_len as u64);
    crypto_hash_sha256_update(st, &ctx_len_u8 as *const u8, 1u64);
    crypto_hash_sha256_final(st, u0.as_mut_ptr());

    i = 0;
    while i < h_len {
        j = 0;
        while j < HASH_BYTES {
            ux[j] ^= u0[j];
            j = j.wrapping_add(1);
        }
        t[2] = t[2].wrapping_add(1);
        crypto_hash_sha256_init(st);
        crypto_hash_sha256_update(st, ux.as_ptr(), HASH_BYTES as u64);
        crypto_hash_sha256_update(st, &t[2] as *const u8, 1u64);
        crypto_hash_sha256_update(st, ctx, ctx_len as u64);
        crypto_hash_sha256_update(st, &ctx_len_u8 as *const u8, 1u64);
        crypto_hash_sha256_final(st, ux.as_mut_ptr());
        memcpy(
            h.add(i),
            ux.as_ptr(),
            if h_len - i >= HASH_BYTES {
                HASH_BYTES
            } else {
                h_len - i
            },
        );
        i = i.wrapping_add(HASH_BYTES);
    }
    0
}

/// ```c
/// static int
/// core_h2c_string_to_hash_sha512(unsigned char *h, const size_t h_len,
///                                const unsigned char *ctx, size_t ctx_len,
///                                const unsigned char *msg, size_t msg_len)
/// ```
///
/// `HASH_BYTES = crypto_hash_sha512_BYTES = 64`, `HASH_BLOCKBYTES = 128`.
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

    let mut st = core::mem::MaybeUninit::<crypto_hash_sha512_state>::uninit();
    let st = st.as_mut_ptr();
    let empty_block: [u8; HASH_BLOCKBYTES] = [0; HASH_BLOCKBYTES];
    let mut u0: [u8; HASH_BYTES] = [0; HASH_BYTES];
    let mut ux: [u8; HASH_BYTES] = [0; HASH_BYTES];
    let mut t: [u8; 3] = [0u8, h_len as u8, 0u8];
    let ctx_len_u8: u8;
    let mut i: usize;
    let mut j: usize;

    /* assert(h_len <= 0xff); -- NDEBUG */
    if ctx_len > 0xff_usize {
        crypto_hash_sha512_init(st);
        crypto_hash_sha512_update(st, OVERSIZE_DST.as_ptr(), OVERSIZE_DST.len() as u64);
        crypto_hash_sha512_update(st, ctx, ctx_len as u64);
        crypto_hash_sha512_final(st, u0.as_mut_ptr());
        ctx = u0.as_ptr();
        ctx_len = HASH_BYTES;
    }
    ctx_len_u8 = ctx_len as u8;
    crypto_hash_sha512_init(st);
    crypto_hash_sha512_update(st, empty_block.as_ptr(), empty_block.len() as u64);
    crypto_hash_sha512_update(st, msg, msg_len as u64);
    crypto_hash_sha512_update(st, t.as_ptr(), 3u64);
    crypto_hash_sha512_update(st, ctx, ctx_len as u64);
    crypto_hash_sha512_update(st, &ctx_len_u8 as *const u8, 1u64);
    crypto_hash_sha512_final(st, u0.as_mut_ptr());

    i = 0;
    while i < h_len {
        j = 0;
        while j < HASH_BYTES {
            ux[j] ^= u0[j];
            j = j.wrapping_add(1);
        }
        t[2] = t[2].wrapping_add(1);
        crypto_hash_sha512_init(st);
        crypto_hash_sha512_update(st, ux.as_ptr(), HASH_BYTES as u64);
        crypto_hash_sha512_update(st, &t[2] as *const u8, 1u64);
        crypto_hash_sha512_update(st, ctx, ctx_len as u64);
        crypto_hash_sha512_update(st, &ctx_len_u8 as *const u8, 1u64);
        crypto_hash_sha512_final(st, ux.as_mut_ptr());
        memcpy(
            h.add(i),
            ux.as_ptr(),
            if h_len - i >= HASH_BYTES {
                HASH_BYTES
            } else {
                h_len - i
            },
        );
        i = i.wrapping_add(HASH_BYTES);
    }
    0
}

/// ```c
/// int
/// core_h2c_string_to_hash(unsigned char *h, const size_t h_len,
///                         const unsigned char *ctx, size_t ctx_len,
///                         const unsigned char *msg, size_t msg_len, int hash_alg)
/// ```
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
        CORE_H2C_SHA256 => core_h2c_string_to_hash_sha256(h, h_len, ctx, ctx_len, msg, msg_len),
        CORE_H2C_SHA512 => core_h2c_string_to_hash_sha512(h, h_len, ctx, ctx_len, msg, msg_len),
        _ => {
            set_errno(EINVAL);
            -1
        }
    }
}
