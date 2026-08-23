//! Rust translation of the LZ4 library (lz4, lz4hc, lz4frame, lz4file, xxhash).
//!
//! The translation is intentionally a faithful, mechanical transliteration of the
//! original C sources found in `c_src/`, including quirks and edge-case behaviour.
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(clippy::all)]

pub mod common;
pub mod lz4;
pub mod lz4file;
pub mod lz4frame;
pub mod lz4hc;
pub mod xxhash;
