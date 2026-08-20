//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (as exported by the C shared object):
//!   * `hex2bin`
//!
//! The translation reproduces the C semantics bit-for-bit, including integer
//! promotion / truncation behaviour and any quirks of the original code.

#![allow(clippy::missing_safety_doc)]

mod hex2bin;

pub use hex2bin::hex2bin;
