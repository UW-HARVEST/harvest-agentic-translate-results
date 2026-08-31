//! Translation of c_src/libsodium/crypto_core/hsalsa20/core_hsalsa20.c

// crypto_core_hsalsa20_* constants from crypto_core_hsalsa20.h
const CRYPTO_CORE_HSALSA20_OUTPUTBYTES: usize = 32;
const CRYPTO_CORE_HSALSA20_INPUTBYTES: usize = 16;
const CRYPTO_CORE_HSALSA20_KEYBYTES: usize = 32;
const CRYPTO_CORE_HSALSA20_CONSTBYTES: usize = 16;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_hsalsa20_outputbytes() -> usize {
    CRYPTO_CORE_HSALSA20_OUTPUTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_hsalsa20_inputbytes() -> usize {
    CRYPTO_CORE_HSALSA20_INPUTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_hsalsa20_keybytes() -> usize {
    CRYPTO_CORE_HSALSA20_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_hsalsa20_constbytes() -> usize {
    CRYPTO_CORE_HSALSA20_CONSTBYTES
}
