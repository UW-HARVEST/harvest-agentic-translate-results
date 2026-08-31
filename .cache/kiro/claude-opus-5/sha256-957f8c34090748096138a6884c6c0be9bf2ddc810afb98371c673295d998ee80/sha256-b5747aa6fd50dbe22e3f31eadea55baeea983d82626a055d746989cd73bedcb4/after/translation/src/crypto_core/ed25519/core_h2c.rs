//! Translation of c_src/libsodium/crypto_core/ed25519/core_h2c.c

use core::ffi::c_int;

// CORE_H2C_* from core_h2c.h
const CORE_H2C_SHA256: c_int = 1;
const CORE_H2C_SHA512: c_int = 2;

// crypto_hash_sha256_BYTES / crypto_hash_sha512_BYTES
const crypto_hash_sha256_BYTES: usize = 32;
const crypto_hash_sha512_BYTES: usize = 64;

// Local repr(C) copies of the hash state structs (rule 4).
#[repr(C)]
struct crypto_hash_sha256_state {
    state: [u32; 8],
    count: u64,
    buf: [u8; 64],
}

#[repr(C)]
struct crypto_hash_sha512_state {
    state: [u64; 8],
    count: [u64; 2],
    buf: [u8; 128],
}

extern "C" {
    fn crypto_hash_sha256_init(state: *mut crypto_hash_sha256_state) -> c_int;
    fn crypto_hash_sha256_update(
        state: *mut crypto_hash_sha256_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha256_final(
        state: *mut crypto_hash_sha256_state,
        out: *mut u8,
    ) -> c_int;

    fn crypto_hash_sha512_init(state: *mut crypto_hash_sha512_state) -> c_int;
    fn crypto_hash_sha512_update(
        state: *mut crypto_hash_sha512_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_hash_sha512_final(
        state: *mut crypto_hash_sha512_state,
        out: *mut u8,
    ) -> c_int;
}

// #define HASH_BYTES      crypto_hash_sha256_BYTES
// #define HASH_BLOCKBYTES 64U
const SHA256_HASH_BYTES: usize = crypto_hash_sha256_BYTES;
const SHA256_HASH_BLOCKBYTES: usize = 64;

unsafe fn core_h2c_string_to_hash_sha256(
    h: *mut u8,
    h_len: usize,
    mut ctx: *const u8,
    mut ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
) -> c_int {
    let mut st = core::mem::MaybeUninit::<crypto_hash_sha256_state>::uninit();
    let st = st.as_mut_ptr();
    let empty_block: [u8; SHA256_HASH_BLOCKBYTES] = [0; SHA256_HASH_BLOCKBYTES];
    let mut u0: [u8; SHA256_HASH_BYTES] = [0; SHA256_HASH_BYTES];
    let mut ux: [u8; SHA256_HASH_BYTES] = [0; SHA256_HASH_BYTES];
    let mut t: [u8; 3] = [0u8, h_len as u8, 0u8];
    let ctx_len_u8: u8;
    let mut i: usize;
    let mut j: usize;

    // assert(h_len <= 0xff);
    if ctx_len > 0xff {
        crypto_hash_sha256_init(st);
        let s = b"H2C-OVERSIZE-DST-";
        crypto_hash_sha256_update(st, s.as_ptr(), (s.len()) as u64);
        crypto_hash_sha256_update(st, ctx, ctx_len as u64);
        crypto_hash_sha256_final(st, u0.as_mut_ptr());
        ctx = u0.as_ptr();
        ctx_len = SHA256_HASH_BYTES;
        // COMPILER_ASSERT(HASH_BYTES <= (size_t) 0xff);
    }
    ctx_len_u8 = ctx_len as u8;
    crypto_hash_sha256_init(st);
    crypto_hash_sha256_update(st, empty_block.as_ptr(), core::mem::size_of::<[u8; SHA256_HASH_BLOCKBYTES]>() as u64);
    crypto_hash_sha256_update(st, msg, msg_len as u64);
    crypto_hash_sha256_update(st, t.as_ptr(), 3);
    crypto_hash_sha256_update(st, ctx, ctx_len as u64);
    crypto_hash_sha256_update(st, &ctx_len_u8, 1);
    crypto_hash_sha256_final(st, u0.as_mut_ptr());

    i = 0;
    while i < h_len {
        j = 0;
        while j < SHA256_HASH_BYTES {
            ux[j] ^= u0[j];
            j += 1;
        }
        t[2] = t[2].wrapping_add(1);
        crypto_hash_sha256_init(st);
        crypto_hash_sha256_update(st, ux.as_ptr(), SHA256_HASH_BYTES as u64);
        crypto_hash_sha256_update(st, &t[2], 1);
        crypto_hash_sha256_update(st, ctx, ctx_len as u64);
        crypto_hash_sha256_update(st, &ctx_len_u8, 1);
        crypto_hash_sha256_final(st, ux.as_mut_ptr());
        let n = if h_len - i >= core::mem::size_of::<[u8; SHA256_HASH_BYTES]>() {
            core::mem::size_of::<[u8; SHA256_HASH_BYTES]>()
        } else {
            h_len - i
        };
        core::ptr::copy_nonoverlapping(ux.as_ptr(), h.add(i), n);
        i += SHA256_HASH_BYTES;
    }
    0
}

// #define HASH_BYTES      crypto_hash_sha512_BYTES
// #define HASH_BLOCKBYTES 128U
const SHA512_HASH_BYTES: usize = crypto_hash_sha512_BYTES;
const SHA512_HASH_BLOCKBYTES: usize = 128;

unsafe fn core_h2c_string_to_hash_sha512(
    h: *mut u8,
    h_len: usize,
    mut ctx: *const u8,
    mut ctx_len: usize,
    msg: *const u8,
    msg_len: usize,
) -> c_int {
    let mut st = core::mem::MaybeUninit::<crypto_hash_sha512_state>::uninit();
    let st = st.as_mut_ptr();
    let empty_block: [u8; SHA512_HASH_BLOCKBYTES] = [0; SHA512_HASH_BLOCKBYTES];
    let mut u0: [u8; SHA512_HASH_BYTES] = [0; SHA512_HASH_BYTES];
    let mut ux: [u8; SHA512_HASH_BYTES] = [0; SHA512_HASH_BYTES];
    let mut t: [u8; 3] = [0u8, h_len as u8, 0u8];
    let ctx_len_u8: u8;
    let mut i: usize;
    let mut j: usize;

    // assert(h_len <= 0xff);
    if ctx_len > 0xff {
        crypto_hash_sha512_init(st);
        let s = b"H2C-OVERSIZE-DST-";
        crypto_hash_sha512_update(st, s.as_ptr(), (s.len()) as u64);
        crypto_hash_sha512_update(st, ctx, ctx_len as u64);
        crypto_hash_sha512_final(st, u0.as_mut_ptr());
        ctx = u0.as_ptr();
        ctx_len = SHA512_HASH_BYTES;
        // COMPILER_ASSERT(HASH_BYTES <= (size_t) 0xff);
    }
    ctx_len_u8 = ctx_len as u8;
    crypto_hash_sha512_init(st);
    crypto_hash_sha512_update(st, empty_block.as_ptr(), core::mem::size_of::<[u8; SHA512_HASH_BLOCKBYTES]>() as u64);
    crypto_hash_sha512_update(st, msg, msg_len as u64);
    crypto_hash_sha512_update(st, t.as_ptr(), 3);
    crypto_hash_sha512_update(st, ctx, ctx_len as u64);
    crypto_hash_sha512_update(st, &ctx_len_u8, 1);
    crypto_hash_sha512_final(st, u0.as_mut_ptr());

    i = 0;
    while i < h_len {
        j = 0;
        while j < SHA512_HASH_BYTES {
            ux[j] ^= u0[j];
            j += 1;
        }
        t[2] = t[2].wrapping_add(1);
        crypto_hash_sha512_init(st);
        crypto_hash_sha512_update(st, ux.as_ptr(), SHA512_HASH_BYTES as u64);
        crypto_hash_sha512_update(st, &t[2], 1);
        crypto_hash_sha512_update(st, ctx, ctx_len as u64);
        crypto_hash_sha512_update(st, &ctx_len_u8, 1);
        crypto_hash_sha512_final(st, ux.as_mut_ptr());
        let n = if h_len - i >= core::mem::size_of::<[u8; SHA512_HASH_BYTES]>() {
            core::mem::size_of::<[u8; SHA512_HASH_BYTES]>()
        } else {
            h_len - i
        };
        core::ptr::copy_nonoverlapping(ux.as_ptr(), h.add(i), n);
        i += SHA512_HASH_BYTES;
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
            crate::plat::set_errno(crate::plat::EINVAL);
            -1
        }
    }
}
