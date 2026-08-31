//! A Rust translation of the zstd 1.5.7 C library found in `c_src/`.
//!
//! The crate is built as a `cdylib` and exports the same C ABI symbols as the
//! original library, so it can be dropped in place of `libzstd.so`.
// The translation deliberately keeps the C identifiers, so the usual Rust
// naming lints are disabled crate-wide.
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused_parens)]
#![allow(clippy::missing_safety_doc)]

pub mod allocations;
pub mod bits;
pub mod bitstream;
pub mod error;
pub mod mem;
pub mod zstd_internal;
pub mod xxhash;

pub mod entropy_common;
pub mod fse;
pub mod fse_decompress;
pub mod huf;
pub mod huf_decompress;
pub mod zstd_public;
pub mod zstd_decompress_internal;
pub mod layout_checks;

pub mod zstd_ddict;
pub mod zstd_decompress_block;
pub mod zstd_decompress;

pub mod fse_compress;
pub mod huf_compress;
