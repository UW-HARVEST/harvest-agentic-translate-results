//! Translation of c_src/libsodium/crypto_shorthash/siphash24/shorthash_siphash24.c

const CRYPTO_SHORTHASH_SIPHASH24_BYTES: usize = 8;
const CRYPTO_SHORTHASH_SIPHASH24_KEYBYTES: usize = 16;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_siphash24_bytes() -> usize {
    CRYPTO_SHORTHASH_SIPHASH24_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_siphash24_keybytes() -> usize {
    CRYPTO_SHORTHASH_SIPHASH24_KEYBYTES
}
