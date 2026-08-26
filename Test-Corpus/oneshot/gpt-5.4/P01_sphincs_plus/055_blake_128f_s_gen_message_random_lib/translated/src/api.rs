use crate::sign::*;

pub use crate::params::{CRYPTO_ALGNAME, CRYPTO_BYTES, CRYPTO_PUBLICKEYBYTES, CRYPTO_SECRETKEYBYTES, CRYPTO_SEEDBYTES};

pub fn crypto_sign_secretkeybytes() -> u64 {
    crate::params::CRYPTO_SECRETKEYBYTES as u64
}

pub fn crypto_sign_publickeybytes() -> u64 {
    crate::params::CRYPTO_PUBLICKEYBYTES as u64
}

pub fn crypto_sign_bytes() -> u64 {
    crate::params::CRYPTO_BYTES as u64
}

pub fn crypto_sign_seedbytes() -> u64 {
    crate::params::CRYPTO_SEEDBYTES as u64
}
