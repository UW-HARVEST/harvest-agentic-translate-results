//! Rust translation of tree/hashmap C library.
//!
//! Designed to produce byte-identical output and to be ABI-compatible with the
//! original C library (same struct layout, same exported symbols).

#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

pub mod hashmap;
pub mod tree;
