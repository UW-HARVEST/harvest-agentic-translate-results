//! `driver` - Rust translation of the C ASCII art drawing application
//! (`c_src/`).
//!
//! The crate builds two things:
//!
//!  * the `driver` executable (`src/main.rs`, plus `src/cio.rs`, `src/scene.rs`
//!    and `src/shape.rs`): the safe, idiomatic translation of the program, and
//!  * this library, whose [`capi`] module exports the very same C ABI as the
//!    shared object built from `c_src` (see `SYMBOLS.md`), so that external
//!    callers - including the differential tests in `tests/` - can load either
//!    shared object and observe identical behaviour.
#![allow(clippy::missing_safety_doc)]

pub mod capi;
