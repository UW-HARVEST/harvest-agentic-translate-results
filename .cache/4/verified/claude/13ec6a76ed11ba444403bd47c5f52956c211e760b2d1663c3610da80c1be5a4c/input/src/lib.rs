//! Rust translation of the LZ4 library (lz4.c, lz4hc.c, lz4frame.c, lz4file.c, xxhash.c).
//!
//! The translation intentionally mirrors the original C sources instruction by
//! instruction (including pointer arithmetic and integer wrapping semantics) so
//! that the produced output is byte-identical to the C implementation.
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused_parens)]
#![allow(dead_code)]
#![allow(unused_assignments)]
#![allow(clippy::all)]

pub mod common;
pub mod lz4;
pub mod lz4file;
pub mod lz4frame;
pub mod lz4hc;
pub mod xxhash;
