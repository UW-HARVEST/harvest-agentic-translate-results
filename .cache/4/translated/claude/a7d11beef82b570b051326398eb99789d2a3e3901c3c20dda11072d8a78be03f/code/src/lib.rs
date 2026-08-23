//! Rust translation of libsodium 1.0.23 (`c_src/libsodium`).
//!
//! The reference C build (see `c_src/CMakeLists.txt`) defines **no** `HAVE_*`
//! feature macros, so every `#ifdef HAVE_*` selects the portable fallback.
//! This crate reproduces exactly that configuration.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_parens)]
#![allow(unused_unsafe)]
#![allow(dead_code)]
#![allow(clippy::all)]

pub mod common;

pub mod crypto_aead;
pub mod crypto_auth;
pub mod crypto_box;
pub mod crypto_core;
pub mod crypto_generichash;
pub mod crypto_hash;
pub mod crypto_ipcrypt;
pub mod crypto_kdf;
pub mod crypto_kem;
pub mod crypto_kx;
pub mod crypto_onetimeauth;
pub mod crypto_pwhash;
pub mod crypto_scalarmult;
pub mod crypto_secretbox;
pub mod crypto_secretstream;
pub mod crypto_shorthash;
pub mod crypto_sign;
pub mod crypto_stream;
pub mod crypto_verify;
pub mod crypto_xof;
pub mod randombytes;
pub mod sodium;
