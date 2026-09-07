//! Pure-Rust translation of the SPHINCS+ reference project.
//!
//! Build-time configurability mirrors the CMake cache variables via Cargo
//! features (resolved into `spx_backend` / `spx_thash` / `spx_secpar` cfgs by
//! `build.rs`).

#![allow(clippy::needless_range_loop)]
#![allow(clippy::missing_safety_doc)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

pub mod params;
pub mod context;

pub mod address;
pub mod utils;
pub mod utilsx1;
pub mod wots;
pub mod wotsx1;
pub mod fors;
pub mod merkle;
pub mod sign;
pub mod rng;
pub mod randombytes;

// -------- Hash backends (exactly one is active at a time) --------
#[cfg(spx_backend = "sha2")]
pub mod sha2;
#[cfg(spx_backend = "sha2")]
pub mod sha2_hash;
#[cfg(spx_backend = "sha2")]
pub mod sha2_thash;

#[cfg(spx_backend = "shake")]
pub mod fips202;
#[cfg(spx_backend = "shake")]
pub mod shake_hash;
#[cfg(spx_backend = "shake")]
pub mod shake_thash;

#[cfg(spx_backend = "haraka")]
pub mod haraka;
#[cfg(spx_backend = "haraka")]
pub mod haraka_hash;
#[cfg(spx_backend = "haraka")]
pub mod haraka_thash;

#[cfg(spx_backend = "blake")]
pub mod blake256;
#[cfg(spx_backend = "blake")]
pub mod blake512;
#[cfg(spx_backend = "blake")]
pub mod blake_hash;
#[cfg(spx_backend = "blake")]
pub mod blake_thash;

// Backend-agnostic facades used by the core code.
pub mod hash;
pub mod thash;
