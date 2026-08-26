//! zstd, transliterated from C to Rust.
#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    unused_mut,
    unused_variables,
    unused_assignments,
    unused_parens,
    unused_imports,
    unused_unsafe,
    unreachable_patterns,
    unreachable_code,
    overflowing_literals,
    static_mut_refs,
    invalid_value
)]

pub mod bits;
pub mod bitstream;
pub mod clevels;
pub mod compiler;
pub mod cover;
pub mod debug;
pub mod divsufsort;
pub mod divsufsort_common;
pub mod divsufsort_ss;
pub mod divsufsort_tr;
pub mod entropy_common;
pub mod error_private;
pub mod fastcover;
pub mod fse;
pub mod fse_compress;
pub mod fse_decompress;
pub mod hist;
pub mod huf;
pub mod huf_compress;
pub mod huf_decompress;
pub mod mem;
pub mod pool;
pub mod threading;
pub mod xxhash;
pub mod zbuff_common;
pub mod zbuff_compress;
pub mod zbuff_decompress;
pub mod zdict;
pub mod zdict_h;
pub mod zstd_common;
pub mod zstd_compress;
pub mod zstd_compress_internal;
pub mod zstd_compress_literals;
pub mod zstd_compress_p2;
pub mod zstd_compress_p3;
pub mod zstd_compress_p4;
pub mod zstd_compress_sequences;
pub mod zstd_compress_superblock;
pub mod zstd_cwksp;
pub mod zstd_ddict;
pub mod zstd_decompress;
pub mod zstd_decompress_block;
pub mod zstd_decompress_internal;
pub mod zstd_double_fast;
pub mod zstd_fast;
pub mod zstd_h;
pub mod zstd_internal;
pub mod zstd_lazy;
pub mod zstd_ldm;
pub mod zstd_opt;
pub mod zstd_presplit;
pub mod zstdmt_compress;

pub mod legacy {
    pub mod v01;
    pub mod v02;
    pub mod v03;
    pub mod v04;
    pub mod v05;
    pub mod v05_ent;
    pub mod v06;
    pub mod v06_ent;
    pub mod v07;
    pub mod zstd_legacy;
}
