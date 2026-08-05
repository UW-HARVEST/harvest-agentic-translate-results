// P3 AEAD & high-level boxes
pub mod softaes;

pub mod chacha20poly1305;
pub mod xchacha20poly1305;
pub mod aes256gcm;
pub mod aegis128l;
pub mod aegis256;

pub mod core_hsalsa20;
pub mod generichash_dispatch;

pub mod secretbox;
pub mod cryptobox;
pub mod auth;
pub mod secretstream;
