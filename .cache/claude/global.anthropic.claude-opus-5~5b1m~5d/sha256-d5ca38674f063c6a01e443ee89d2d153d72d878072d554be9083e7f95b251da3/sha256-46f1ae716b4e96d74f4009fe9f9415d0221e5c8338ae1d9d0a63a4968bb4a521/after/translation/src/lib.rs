//! Pure-Rust translation of the SPHINCS+ reference implementation found in `c_src/`.
//!
//! Build-time configurability mirrors the CMake cache variables:
//!   * `HASH_BACKEND` -> features `haraka`, `sha2`, `shake` (alias `shake256`), `blake`
//!   * `THASH`        -> features `robust`, `simple`
//!   * `SECPAR`       -> features `128s`, `128f`, `192s`, `192f`, `256s`, `256f`

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]

pub mod address;
pub mod backend;
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

pub use backend::{gen_message_random, hash_message, initialize_hash_function, prf_addr, thash};
pub use context::SpxCtx;
