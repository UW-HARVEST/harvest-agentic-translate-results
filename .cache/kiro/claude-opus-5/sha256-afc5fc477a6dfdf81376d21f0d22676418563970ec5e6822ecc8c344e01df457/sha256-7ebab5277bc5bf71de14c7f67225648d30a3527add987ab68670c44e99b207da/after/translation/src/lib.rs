//! libsodium 1.0.23 translated from C to Rust.
//!
//! Layout mirrors `c_src/libsodium/`: one Rust module per C source file.
//! Directories named `ref` are renamed `ref_` (`ref` is a Rust keyword).

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(unused_imports)]
#![allow(unused_parens)]
#![allow(unused_assignments)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(static_mut_refs)]
#![allow(clippy::all)]

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
pub mod common;
pub mod fe25519;
pub mod plat;
