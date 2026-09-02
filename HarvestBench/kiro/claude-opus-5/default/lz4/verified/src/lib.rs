//! Rust translation of the LZ4 C library (v1.10.0) exposing the same C ABI.
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

pub mod common;
pub mod lz4;
pub mod lz4file;
pub mod lz4frame;
pub mod lz4hc;
pub mod xxhash;
