//! Rust translation of the zstd C library (v1.5.7), built as a cdylib that
//! exposes the same public ABI as the original shared object.
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused_parens)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(unused_assignments)]
#![allow(unused_unsafe)]
#![allow(dead_code)]
#![allow(static_mut_refs)]
#![allow(clippy::all)]

pub mod libc;
pub mod zstd_h;

pub mod common {
    pub mod bits;
    pub mod bitstream;
    pub mod debug;
    pub mod entropy_common;
    pub mod error_private;
    pub mod fse;
    pub mod fse_decompress;
    pub mod huf;
    pub mod mem;
    pub mod pool;
    pub mod threading;
    pub mod xxhash;
    pub mod zstd_common;
    pub mod zstd_internal;
}

pub mod compress {
    pub mod clevels;
    pub mod fse_compress;
    pub mod hist;
    pub mod huf_compress;
    pub mod zstd_compress;
    pub mod zstd_compress_internal;
    pub mod zstd_compress_literals;
    pub mod zstd_compress_sequences;
    pub mod zstd_compress_superblock;
    pub mod zstd_cwksp;
    pub mod zstd_double_fast;
    pub mod zstd_fast;
    pub mod zstd_lazy;
    pub mod zstd_ldm;
    pub mod zstd_ldm_geartab;
    pub mod zstd_opt;
    pub mod zstd_preSplit;
    pub mod zstdmt_compress;
}

pub mod decompress {
    pub mod huf_decompress;
    pub mod zstd_ddict;
    pub mod zstd_decompress;
    pub mod zstd_decompress_block;
    pub mod zstd_decompress_internal;
}

pub mod dictbuilder {
    pub mod cover;
    pub mod divsufsort;
    pub mod fastcover;
    pub mod zdict;
}

pub mod deprecated {
    pub mod zbuff_common;
    pub mod zbuff_compress;
    pub mod zbuff_decompress;
}

pub mod legacy {
    pub mod zstd_legacy;
    pub mod zstd_v01;
    pub mod zstd_v02;
    pub mod zstd_v03;
    pub mod zstd_v04;
    pub mod zstd_v05;
    pub mod zstd_v06;
    pub mod zstd_v07;
}

