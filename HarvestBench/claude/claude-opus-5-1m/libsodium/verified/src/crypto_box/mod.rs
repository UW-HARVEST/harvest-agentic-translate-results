pub mod curve25519xchacha20poly1305;
pub mod curve25519xsalsa20poly1305;
pub mod easy;
pub mod seal;
pub mod seal_curve25519xchacha20poly1305;

// Translation of `crypto_box/crypto_box.c`.

use core::ffi::{c_char, c_int};

use self::curve25519xsalsa20poly1305::{
    crypto_box_curve25519xsalsa20poly1305, crypto_box_curve25519xsalsa20poly1305_BEFORENMBYTES,
    crypto_box_curve25519xsalsa20poly1305_BOXZEROBYTES,
    crypto_box_curve25519xsalsa20poly1305_MACBYTES,
    crypto_box_curve25519xsalsa20poly1305_MESSAGEBYTES_MAX,
    crypto_box_curve25519xsalsa20poly1305_NONCEBYTES,
    crypto_box_curve25519xsalsa20poly1305_PUBLICKEYBYTES,
    crypto_box_curve25519xsalsa20poly1305_SECRETKEYBYTES,
    crypto_box_curve25519xsalsa20poly1305_SEEDBYTES,
    crypto_box_curve25519xsalsa20poly1305_ZEROBYTES, crypto_box_curve25519xsalsa20poly1305_afternm,
    crypto_box_curve25519xsalsa20poly1305_beforenm, crypto_box_curve25519xsalsa20poly1305_keypair,
    crypto_box_curve25519xsalsa20poly1305_open,
    crypto_box_curve25519xsalsa20poly1305_open_afternm,
    crypto_box_curve25519xsalsa20poly1305_seed_keypair,
};

// ---------------------------------------------------------------------------
// Constants from include/sodium/crypto_box.h
// ---------------------------------------------------------------------------

pub const crypto_box_SEEDBYTES: usize = crypto_box_curve25519xsalsa20poly1305_SEEDBYTES;
pub const crypto_box_PUBLICKEYBYTES: usize = crypto_box_curve25519xsalsa20poly1305_PUBLICKEYBYTES;
pub const crypto_box_SECRETKEYBYTES: usize = crypto_box_curve25519xsalsa20poly1305_SECRETKEYBYTES;
pub const crypto_box_NONCEBYTES: usize = crypto_box_curve25519xsalsa20poly1305_NONCEBYTES;
pub const crypto_box_MACBYTES: usize = crypto_box_curve25519xsalsa20poly1305_MACBYTES;
pub const crypto_box_MESSAGEBYTES_MAX: usize =
    crypto_box_curve25519xsalsa20poly1305_MESSAGEBYTES_MAX;
pub const crypto_box_BEFORENMBYTES: usize = crypto_box_curve25519xsalsa20poly1305_BEFORENMBYTES;
pub const crypto_box_ZEROBYTES: usize = crypto_box_curve25519xsalsa20poly1305_ZEROBYTES;
pub const crypto_box_BOXZEROBYTES: usize = crypto_box_curve25519xsalsa20poly1305_BOXZEROBYTES;
/// `#define crypto_box_SEALBYTES (crypto_box_PUBLICKEYBYTES + crypto_box_MACBYTES)`
pub const crypto_box_SEALBYTES: usize = crypto_box_PUBLICKEYBYTES + crypto_box_MACBYTES;

/// `#define crypto_box_PRIMITIVE "curve25519xsalsa20poly1305"`
static crypto_box_PRIMITIVE: [u8; 27] = *b"curve25519xsalsa20poly1305\0";

// ---------------------------------------------------------------------------
// Size accessors
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_seedbytes() -> usize {
    crypto_box_SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_publickeybytes() -> usize {
    crypto_box_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_secretkeybytes() -> usize {
    crypto_box_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_beforenmbytes() -> usize {
    crypto_box_BEFORENMBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_noncebytes() -> usize {
    crypto_box_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_zerobytes() -> usize {
    crypto_box_ZEROBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_boxzerobytes() -> usize {
    crypto_box_BOXZEROBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_macbytes() -> usize {
    crypto_box_MACBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_messagebytes_max() -> usize {
    crypto_box_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_primitive() -> *const c_char {
    crypto_box_PRIMITIVE.as_ptr() as *const c_char
}

// ---------------------------------------------------------------------------
// Thin wrappers over the default primitive
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    unsafe { crypto_box_curve25519xsalsa20poly1305_seed_keypair(pk, sk, seed) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    unsafe { crypto_box_curve25519xsalsa20poly1305_keypair(pk, sk) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_beforenm(
    k: *mut u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    unsafe { crypto_box_curve25519xsalsa20poly1305_beforenm(k, pk, sk) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_afternm(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    unsafe { crypto_box_curve25519xsalsa20poly1305_afternm(c, m, mlen, n, k) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_open_afternm(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    unsafe { crypto_box_curve25519xsalsa20poly1305_open_afternm(m, c, clen, n, k) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    unsafe { crypto_box_curve25519xsalsa20poly1305(c, m, mlen, n, pk, sk) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_open(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    unsafe { crypto_box_curve25519xsalsa20poly1305_open(m, c, clen, n, pk, sk) }
}
