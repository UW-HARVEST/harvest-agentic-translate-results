//! Pure-Rust translation of the SPHINCS+ reference implementation.
//!
//! Build-time configurability (CMake cache variables `HASH_BACKEND`, `THASH`,
//! `SECPAR`) is preserved via Cargo features with the exact same lowercase
//! names. Exactly one hash backend module is compiled, selected with the
//! priority `sha2` > `shake` > `blake` > `haraka` (the last also being the
//! default when no backend feature is enabled), matching the CMake default of
//! `haraka`.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_range_loop)]

pub mod address;
pub mod context;
pub mod fors;
pub mod merkle;
pub mod params;
pub mod randombytes;
pub mod rng;
pub mod sign;
pub mod utils;
pub mod utilsx1;
pub mod wots;
pub mod wotsx1;

// --- Active hash backend (exactly one is compiled) --------------------------

#[cfg(feature = "sha2")]
#[path = "backends/sha2/mod.rs"]
pub mod backend;

#[cfg(all(feature = "shake", not(feature = "sha2")))]
#[path = "backends/shake/mod.rs"]
pub mod backend;

#[cfg(all(feature = "blake", not(feature = "sha2"), not(feature = "shake")))]
#[path = "backends/blake/mod.rs"]
pub mod backend;

#[cfg(all(
    not(feature = "sha2"),
    not(feature = "shake"),
    not(feature = "blake")
))]
#[path = "backends/haraka/mod.rs"]
pub mod backend;

pub use context::SpxCtx;
