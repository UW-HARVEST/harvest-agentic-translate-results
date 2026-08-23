//! Translation of `crypto_shorthash/siphash24/shorthash_siphashx24.c`.

/* #define crypto_shorthash_siphashx24_BYTES 16U */
const crypto_shorthash_siphashx24_BYTES: usize = 16;
/* #define crypto_shorthash_siphashx24_KEYBYTES 16U */
const crypto_shorthash_siphashx24_KEYBYTES: usize = 16;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_siphashx24_bytes() -> usize {
    crypto_shorthash_siphashx24_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_siphashx24_keybytes() -> usize {
    crypto_shorthash_siphashx24_KEYBYTES
}
