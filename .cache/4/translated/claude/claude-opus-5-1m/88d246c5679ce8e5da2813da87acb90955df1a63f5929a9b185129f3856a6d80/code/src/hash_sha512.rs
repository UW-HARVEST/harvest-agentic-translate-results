//! Translation of `crypto_hash/sha512/hash_sha512.c`.

/* Layout of `crypto_hash_sha512_state` as declared in
 * include/sodium/crypto_hash_sha512.h -- duplicated here so this file stays
 * self-contained (sizeof == 208 on x86-64). */
#[repr(C)]
#[derive(Copy, Clone)]
pub struct crypto_hash_sha512_state {
    pub state: [u64; 8],
    pub count: [u64; 2],
    pub buf: [u8; 128],
}

/* #define crypto_hash_sha512_BYTES 64U */
const crypto_hash_sha512_BYTES: usize = 64;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha512_bytes() -> usize {
    crypto_hash_sha512_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha512_statebytes() -> usize {
    core::mem::size_of::<crypto_hash_sha512_state>()
}
