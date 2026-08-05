//! Rust re-implementation of libsodium 1.0.23 (reference/portable build; no HAVE_* macros).
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_parens)]
#![allow(clippy::all)]

// Foundation (hand-translated).
pub mod common;
pub mod codecs;
pub mod core_init;
pub mod randombytes;
pub mod runtime;
pub mod utils;
pub mod verify;
pub mod version;

// Crypto families (each a directory module owned by one translation unit).
pub mod primitives;
pub mod ed25519;
pub mod aead;
pub mod pwhash;
pub mod kdf_kem;

// Shared helper macros / re-exports for cross-module linkage happen via extern "C".
