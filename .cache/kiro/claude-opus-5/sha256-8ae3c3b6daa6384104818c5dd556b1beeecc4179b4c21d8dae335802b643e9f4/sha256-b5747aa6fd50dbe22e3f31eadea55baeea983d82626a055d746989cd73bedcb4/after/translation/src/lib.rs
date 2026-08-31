//! Rust translation of the LZ4 C library (lz4 1.10.0):
//! `lz4.c`, `lz4hc.c`, `lz4frame.c`, `lz4file.c` and the bundled `xxhash.c`
//! (namespaced with `XXH_NAMESPACE=LZ4_`).

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
#![allow(unused_assignments)]

pub mod util;

pub mod lz4;
pub mod lz4file;
pub mod lz4frame;
pub mod lz4hc;
pub mod xxhash;
