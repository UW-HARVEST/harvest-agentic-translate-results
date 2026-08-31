//! Rust translation of the zstd C library (v1.5.7) as built by c_src/CMakeLists.txt
//! (ZSTD_LEGACY_SUPPORT=5, XXH_NAMESPACE=ZSTD_, DYNAMIC_BMI2=0, no ZSTD_MULTITHREAD).
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused_parens)]
#![allow(unused_assignments)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unreachable_patterns)]
#![allow(unreachable_code)]
#![allow(clippy::all)]

pub mod bits;
pub mod bitstream;
pub mod cmem;
pub mod entropy_common;
pub mod error_private;
pub mod fse;
pub mod fse_decompress;
pub mod huf;
pub mod pool;
pub mod xxhash;
pub mod zstd_common;
pub mod zstd_h;
pub mod zstd_internal;
pub mod zstd_trace;

pub mod compress;
pub mod decompress;
pub mod deprecated;
pub mod dictbuilder;
pub mod legacy;

pub mod size_checks;
