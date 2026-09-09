//! Translation of `compress/zstd_compress.c`, split across `p1.rs` .. `p5.rs`
//! purely for manageability. All items are re-exported here so that the rest of
//! the crate can refer to them as `crate::compress::zstd_compress::NAME`.
#![allow(dead_code)]

pub mod p1;
pub mod p2;
pub mod p3;
pub mod p4;
pub mod p5;

pub use p1::*;
pub use p2::*;
pub use p3::*;
pub use p4::*;
pub use p5::*;
