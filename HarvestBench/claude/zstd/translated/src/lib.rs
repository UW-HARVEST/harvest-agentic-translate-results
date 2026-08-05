//! Rust translation of the zstd v1.5.7 C library.
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused_parens)]
#![allow(dead_code)]
#![allow(unused_assignments)]
#![allow(unused_mut)]
#![allow(unused_imports)]

pub mod common;
pub mod compress;
pub mod decompress;
pub mod deprecated;
pub mod dictBuilder;
pub mod legacy;
pub mod zstd_h;
