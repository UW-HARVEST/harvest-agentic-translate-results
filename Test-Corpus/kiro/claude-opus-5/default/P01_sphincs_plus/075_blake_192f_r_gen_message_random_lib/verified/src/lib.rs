//! Pure-Rust translation of the SPHINCS+ reference implementation in `c_src/`.
//!
//! Build-time configurability mirrors the CMake cache variables of
//! `c_src/CMakeLists.txt`, each value being a Cargo feature of the same name in
//! lower case:
//!
//! | CMake cache variable | Cargo features                                     |
//! |----------------------|----------------------------------------------------|
//! | `HASH_BACKEND`       | `haraka`, `sha2`, `shake` (alias `shake256`), `blake` |
//! | `THASH`              | `robust`, `simple`                                 |
//! | `SECPAR`             | `128s`, `128f`, `192s`, `192f`, `256s`, `256f`      |
//!
//! The defaults match the CMake defaults (`haraka` / `robust` / `128s`).

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::all)]

pub mod address;
pub mod backend;
pub mod context;
pub mod fors;
pub mod hash;
pub mod merkle;
pub mod params;
pub mod randombytes;
pub mod rng;
pub mod sign;
pub mod thash;
pub mod utils;
pub mod utilsx1;
pub mod vla;
pub mod wots;
pub mod wotsx1;
