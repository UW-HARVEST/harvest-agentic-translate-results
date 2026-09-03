//! Pure-Rust translation of the SPHINCS+ reference implementation found in
//! `c_src/`.
//!
//! Build-time configurability mirrors the CMake cache variables:
//!
//! | CMake variable | values (= Cargo features)                        |
//! |----------------|--------------------------------------------------|
//! | `HASH_BACKEND` | `haraka`, `sha2`, `shake`, `blake`               |
//! | `THASH`        | `robust`, `simple`                               |
//! | `SECPAR`       | `128s`, `128f`, `192s`, `192f`, `256s`, `256f`   |
//!
//! Cargo features are additive, so an explicit choice must be able to win over
//! the `default = ["haraka", "robust", "128s"]` set.  The resolution order is
//! therefore `sha2 > shake > blake > haraka`, `simple > robust` and
//! `256f > 256s > 192f > 192s > 128f > 128s`.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::too_many_arguments)]

pub mod params;

pub mod context;

/* ---- core (app/src, compiled into sphincs_obj) ---- */
pub mod address;
pub mod fors;
pub mod merkle;
pub mod sign;
pub mod utils;
pub mod utilsx1;
pub mod wots;
pub mod wotsx1;

/* ---- the two alternative randomness sources ---- */
/// `app/src/rng.c` -- the deterministic NIST AES-256-CTR DRBG used by the
/// `sphincs_core_det` library and by the `driver` executable.
pub mod rng;
/// `app/src/randombytes.c` -- the `/dev/urandom` source used by the
/// `sphincs_core` library.
pub mod randombytes;

/* ---- hash backends (lib/<backend>) ---- */

#[cfg(feature = "sha2")]
pub mod sha2;
#[cfg(feature = "sha2")]
pub use crate::sha2 as backend;

#[cfg(all(not(feature = "sha2"), feature = "shake"))]
pub mod shake;
#[cfg(all(not(feature = "sha2"), feature = "shake"))]
pub use crate::shake as backend;

#[cfg(all(not(feature = "sha2"), not(feature = "shake"), feature = "blake"))]
pub mod blake;
#[cfg(all(not(feature = "sha2"), not(feature = "shake"), feature = "blake"))]
pub use crate::blake as backend;

#[cfg(all(
    not(feature = "sha2"),
    not(feature = "shake"),
    not(feature = "blake")
))]
pub mod haraka;
#[cfg(all(
    not(feature = "sha2"),
    not(feature = "shake"),
    not(feature = "blake")
))]
pub use crate::haraka as backend;

/// The hash-backend entry points (`hash.h` / `thash.h`), re-exported from
/// whichever backend is active.
pub mod hash {
    pub use crate::backend::hash::{
        SPX_gen_message_random, SPX_hash_message, SPX_initialize_hash_function, SPX_prf_addr,
    };
    pub use crate::backend::thash::SPX_thash;
}
