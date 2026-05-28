//! Pure-Rust port of the SPHINCS+ reference implementation.
//!
//! Cargo features select the build-time configuration that the C CMake build
//! exposed via `HASH_BACKEND`, `THASH`, and `SECPAR`. All combinations of
//! { haraka | sha2 | shake | blake } x { robust | simple } x
//! { 128s | 128f | 192s | 192f | 256s | 256f } compile.

#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(static_mut_refs)]

pub mod address;
#[cfg(feature = "blake")]
pub mod blake;
pub mod context;
#[cfg(feature = "shake")]
pub mod fips202;
pub mod fors;
pub mod forsx1;
#[cfg(feature = "haraka")]
pub mod haraka;
pub mod hash;
pub mod merkle;
pub mod params;
pub mod rng;
#[cfg(feature = "sha2")]
pub mod sha2_impl;
pub mod sign;
pub mod thash;
pub mod utils;
pub mod utilsx1;
pub mod wots;
pub mod wotsx1;
