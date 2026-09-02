//! Rust translation of the `pow` C library (`c_src/`).
//!
//! The C library exports exactly one public symbol, `my_pow`, and links
//! against glibc's `pow`, `fprintf`, `stderr` and `__errno_location`.
//! This crate reproduces that ABI exactly.

mod ffi;
mod pow;
