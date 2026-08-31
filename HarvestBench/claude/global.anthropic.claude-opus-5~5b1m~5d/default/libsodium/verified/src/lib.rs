//! Rust translation of libsodium 1.0.23 (`c_src/libsodium`).
//!
//! The reference C build defines no `HAVE_*` feature macros, so every
//! `#ifdef HAVE_*` selects the portable fallback. This translation mirrors
//! that configuration exactly.
#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut,
    unused_unsafe,
    unused_variables,
    unused_parens,
    clippy::all
)]
#![no_builtins]

// ---- shared infrastructure ----
pub mod common;
pub mod csys;
pub mod types;

// ---- sodium/ ----
pub mod sodium_codecs;
pub mod sodium_core;
pub mod sodium_runtime;
pub mod sodium_utils;
pub mod sodium_version;

// ---- randombytes/ ----
pub mod randombytes;
pub mod randombytes_internal;
pub mod randombytes_sysrandom;

// ---- crypto_verify/ ----
pub mod verify;

// ---- crypto_core/ ----
pub mod core_ed25519;
pub mod core_hchacha20;
pub mod core_hsalsa20;
pub mod core_salsa;
pub mod ed25519_ref10_fe;
pub mod ed25519_ref10_ge;
pub mod ed25519_ref10_ristretto;
pub mod ed25519_ref10_sc;
pub mod ed25519_ref10_sc_mul;
pub mod ed25519_ref10_sc_muladd;
pub mod ed25519_ref10_sc_reduce;
pub mod ed25519_ref10_tables;
pub mod keccak1600;
pub mod softaes;

// ---- crypto_hash/ ----
pub mod crypto_hash;
pub mod hash_sha256;
pub mod hash_sha3;
pub mod hash_sha512;

// ---- crypto_xof/ ----
pub mod xof;

// ---- crypto_auth/ ----
pub mod auth_hmac;

// ---- crypto_generichash/ ----
pub mod blake2b;

// ---- crypto_onetimeauth/ ----
pub mod poly1305;

// ---- crypto_stream/ ----
pub mod crypto_stream;
pub mod stream_chacha20;
pub mod stream_salsa20;

// ---- crypto_shorthash/ ----
pub mod shorthash;

// ---- crypto_aead/ ----
pub mod aead_aegis128l;
pub mod aead_aegis256;
pub mod aead_aes256gcm;
pub mod aead_chacha20poly1305;
pub mod aead_xchacha20poly1305;

// ---- crypto_secretbox / secretstream ----
pub mod secretbox;
pub mod secretstream;

// ---- crypto_scalarmult / sign / box / kx ----
pub mod box_;
pub mod kx;
pub mod scalarmult;
pub mod sign_ed25519;
pub mod x25519_ref10;

// ---- crypto_kdf/ ----
pub mod kdf;

// ---- crypto_pwhash/ ----
pub mod argon2;
pub mod argon2_encoding;
pub mod pwhash_argon2;
pub mod pwhash_scrypt;
pub mod scrypt;

// ---- crypto_kem/ ----
pub mod kem;
pub mod kem_mlkem768_ref;

// ---- crypto_ipcrypt/ ----
pub mod ipcrypt;
