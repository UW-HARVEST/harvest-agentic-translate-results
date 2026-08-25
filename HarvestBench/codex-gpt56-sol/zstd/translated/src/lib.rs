#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]
#![allow(unused_mut)]
#![feature(c_variadic)]
#![feature(extern_types)]

#[macro_use]
extern crate c2rust_bitfields;
extern crate libc;

pub mod common {
        pub mod debug;
        pub mod entropy_common;
        pub mod error_private;
        pub mod fse_decompress;
        pub mod pool;
        pub mod threading;
        pub mod xxhash;
        pub mod zstd_common;
} // mod common
pub mod compress {
        pub mod fse_compress;
        pub mod hist;
        pub mod huf_compress;
        pub mod zstd_compress;
        pub mod zstd_compress_literals;
        pub mod zstd_compress_sequences;
        pub mod zstd_compress_superblock;
        pub mod zstd_double_fast;
        pub mod zstd_fast;
        pub mod zstd_lazy;
        pub mod zstd_ldm;
        pub mod zstd_opt;
        pub mod zstd_preSplit;
        pub mod zstdmt_compress;
} // mod compress
pub mod decompress {
        pub mod huf_decompress;
        pub mod zstd_ddict;
        pub mod zstd_decompress;
        pub mod zstd_decompress_block;
} // mod decompress
pub mod deprecated {
        pub mod zbuff_common;
        pub mod zbuff_compress;
        pub mod zbuff_decompress;
} // mod deprecated
pub mod dictBuilder {
        pub mod cover;
        pub mod divsufsort;
        pub mod fastcover;
        pub mod zdict;
} // mod dictBuilder
pub mod legacy {
        pub mod zstd_v01;
        pub mod zstd_v02;
        pub mod zstd_v03;
        pub mod zstd_v04;
        pub mod zstd_v05;
        pub mod zstd_v06;
        pub mod zstd_v07;
} // mod legacy
