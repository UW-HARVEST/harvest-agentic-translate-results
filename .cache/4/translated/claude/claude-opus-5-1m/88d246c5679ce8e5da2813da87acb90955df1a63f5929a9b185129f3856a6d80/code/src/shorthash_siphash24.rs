//! Translation of `crypto_shorthash/siphash24/shorthash_siphash24.c`.

/* #define crypto_shorthash_siphash24_BYTES 8U */
const crypto_shorthash_siphash24_BYTES: usize = 8;
/* #define crypto_shorthash_siphash24_KEYBYTES 16U */
const crypto_shorthash_siphash24_KEYBYTES: usize = 16;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_siphash24_bytes() -> usize {
    crypto_shorthash_siphash24_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_siphash24_keybytes() -> usize {
    crypto_shorthash_siphash24_KEYBYTES
}
