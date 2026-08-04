// Library entry point for the driver crate.
//
// Exposes both the safe Rust modules (used by the binary) and the
// C-ABI FFI layer (used to build a `cdylib` that mirrors the C library).

#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

pub mod hashmap;
pub mod tree;
pub mod ffi;
