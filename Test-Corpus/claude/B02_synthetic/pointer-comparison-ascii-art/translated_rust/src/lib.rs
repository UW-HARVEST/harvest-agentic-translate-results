// lib.rs - Library entry point. Re-exports modules and provides
// #[no_mangle] C-ABI wrappers used by the cdylib so that external
// callers (e.g. integration tests via libloading) can exercise the
// same public API exposed by the C library.

#[macro_use]
pub mod out;
pub mod shape;
pub mod scene;
pub mod util;
pub mod ffi;
