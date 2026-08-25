//! Rust translation of the SPHINCS+ reference implementation found in `c_src/`.
//!
//! Build-time configurability mirrors `c_src/CMakeLists.txt`:
//!
//! | CMake cache variable | values                                   | cargo features                        |
//! |----------------------|------------------------------------------|---------------------------------------|
//! | `HASH_BACKEND`       | haraka, sha2, shake, blake               | `haraka`, `sha2`, `shake`, `blake`    |
//! | `THASH`              | robust, simple                           | `robust`, `simple`                    |
//! | `SECPAR`             | 128s, 128f, 192s, 192f, 256s, 256f       | `128s` .. `256f`                      |
//!
//! Defaults: `haraka`, `robust`, `128s` (identical to the CMake defaults).
//! If several features of one group are enabled, a fixed priority decides,
//! so that *every* feature combination compiles.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]

pub mod context;
pub mod params;

// ---- app/src: the SPHINCS+ core -------------------------------------------
pub mod address;
pub mod fors;
pub mod merkle;
pub mod randombytes;
pub mod rng;
pub mod sign;
pub mod utils;
pub mod utilsx1;
pub mod wots;
pub mod wotsx1;

// ---- lib/<HASH_BACKEND>: the hash backends -------------------------------
#[cfg(any(
    feature = "haraka",
    not(any(feature = "sha2", feature = "shake", feature = "blake"))
))]
pub mod haraka;

#[cfg(all(feature = "sha2", not(feature = "haraka")))]
pub mod sha2;

#[cfg(all(feature = "shake", not(any(feature = "haraka", feature = "sha2"))))]
pub mod shake;

#[cfg(all(
    feature = "blake",
    not(any(feature = "haraka", feature = "sha2", feature = "shake"))
))]
pub mod blake;

/// Uniform internal view of whichever hash backend was selected (this is what
/// linking against a different `hash_*.c` / `thash_*.c` achieved in C).
pub(crate) mod backend {
    #[cfg(any(
        feature = "haraka",
        not(any(feature = "sha2", feature = "shake", feature = "blake"))
    ))]
    pub(crate) use crate::haraka::{
        SPX_gen_message_random as gen_message_random, SPX_hash_message as hash_message,
        SPX_initialize_hash_function as initialize_hash_function, SPX_prf_addr as prf_addr,
        SPX_thash as thash,
    };

    #[cfg(all(feature = "sha2", not(feature = "haraka")))]
    pub(crate) use crate::sha2::{
        SPX_gen_message_random as gen_message_random, SPX_hash_message as hash_message,
        SPX_initialize_hash_function as initialize_hash_function, SPX_prf_addr as prf_addr,
        SPX_thash as thash,
    };

    #[cfg(all(feature = "shake", not(any(feature = "haraka", feature = "sha2"))))]
    pub(crate) use crate::shake::{
        SPX_gen_message_random as gen_message_random, SPX_hash_message as hash_message,
        SPX_initialize_hash_function as initialize_hash_function, SPX_prf_addr as prf_addr,
        SPX_thash as thash,
    };

    #[cfg(all(
        feature = "blake",
        not(any(feature = "haraka", feature = "sha2", feature = "shake"))
    ))]
    pub(crate) use crate::blake::{
        SPX_gen_message_random as gen_message_random, SPX_hash_message as hash_message,
        SPX_initialize_hash_function as initialize_hash_function, SPX_prf_addr as prf_addr,
        SPX_thash as thash,
    };
}
