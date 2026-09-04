//! Rust translation of the zstd C library (`c_src/`).
//!
//! Layout mirrors the C source tree file-for-file.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_parens)]
#![allow(unused_unsafe)]
#![allow(unused_imports)]

pub mod common;
pub mod compress;
pub mod decompress;
pub mod deprecated;
pub mod dictbuilder;
pub mod legacy;
