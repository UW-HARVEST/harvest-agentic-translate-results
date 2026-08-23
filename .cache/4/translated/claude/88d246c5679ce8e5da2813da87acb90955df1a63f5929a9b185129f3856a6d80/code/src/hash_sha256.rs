//! Translation of `crypto_hash/sha256/hash_sha256.c`.

/* Layout of `crypto_hash_sha256_state` as declared in
 * include/sodium/crypto_hash_sha256.h -- duplicated here so this file stays
 * self-contained (sizeof == 104 on x86-64). */
#[repr(C)]
#[derive(Copy, Clone)]
pub struct crypto_hash_sha256_state {
    pub state: [u32; 8],
    pub count: u64,
    pub buf: [u8; 64],
}

/* #define crypto_hash_sha256_BYTES 32U */
const crypto_hash_sha256_BYTES: usize = 32;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256_bytes() -> usize {
    crypto_hash_sha256_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256_statebytes() -> usize {
    core::mem::size_of::<crypto_hash_sha256_state>()
}
