//! Translation of c_src/libsodium/crypto_shorthash/siphash24/shorthash_siphashx24.c

const CRYPTO_SHORTHASH_SIPHASHX24_BYTES: usize = 16;
const CRYPTO_SHORTHASH_SIPHASHX24_KEYBYTES: usize = 16;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_siphashx24_bytes() -> usize {
    CRYPTO_SHORTHASH_SIPHASHX24_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_siphashx24_keybytes() -> usize {
    CRYPTO_SHORTHASH_SIPHASHX24_KEYBYTES
}
