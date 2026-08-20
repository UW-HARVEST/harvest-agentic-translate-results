//! Rust translation of the C library in `c_src/` (an inlined copy of
//! `stb_ds.h` plus the `strkey`/`intput` entry points).
//!
//! The crate is a `cdylib` and exports exactly the same public symbols as the
//! C shared object, with identical signatures and identical behaviour —
//! including the C code's integer-promotion quirks and its `assert()` aborts.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_unsafe)]
#![allow(clippy::missing_safety_doc)]

/// Reproduce `STBDS_ASSERT(expr)` == `assert(expr)`.
///
/// `$expr_str` is the stringified condition, `$line` the line in
/// `c_src/src/lib.c` and `$func` the enclosing function name — the three pieces
/// glibc prints before calling `abort()`.
#[macro_export]
macro_rules! stbds_assert {
    ($cond:expr, $expr_str:expr, $line:expr, $func:expr) => {
        if !($cond) {
            unsafe { $crate::cffi::assert_fail($expr_str, $line, $func) }
        }
    };
}

pub mod cffi;
pub mod types;

pub mod arena;
pub mod arr;
pub mod hash;
pub mod hmap;

pub mod api;
