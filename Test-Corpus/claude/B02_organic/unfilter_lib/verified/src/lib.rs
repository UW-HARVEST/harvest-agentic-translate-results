//! Rust translation of the C library in `c_src/` (a cut-down `cute_png`
//! derivative: DEFLATE inflate + PNG scanline unfiltering).
//!
//! The shared library exports exactly the same public ABI as the C build:
//!
//! ```text
//! FUNC   cp_inflate
//! FUNC   unfilter
//! OBJECT cp_error_reason
//! OBJECT cp_fixed_table
//! OBJECT cp_permutation_order
//! OBJECT cp_len_extra_bits
//! OBJECT cp_len_base
//! OBJECT cp_dist_extra_bits
//! OBJECT cp_dist_base
//! ```
//!
//! Behaviour (including the bugs and the `assert()`s, which are live because
//! the CMake build never defines `NDEBUG`) is reproduced as-is.

#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]

#[macro_use]
mod cassert;

mod inflate;
mod misc;
mod tables;
mod unfilter;

// Re-exported so that the `#[unsafe(no_mangle)]` items are unmistakably part of
// the crate's surface.
pub use inflate::cp_inflate;
pub use tables::{
    cp_dist_base, cp_dist_extra_bits, cp_error_reason, cp_fixed_table, cp_len_base,
    cp_len_extra_bits, cp_permutation_order,
};
pub use unfilter::unfilter;
