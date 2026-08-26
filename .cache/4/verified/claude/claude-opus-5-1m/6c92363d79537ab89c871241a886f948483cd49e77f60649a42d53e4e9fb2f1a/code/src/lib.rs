//! `cdylib` view of the translation: exports the ten `dag_lib.h` symbols with
//! the C calling convention, data layout and allocator, so that the Rust build
//! can be dropped in for `libdag_c.so` and compared symbol by symbol.
//!
//! The translated *program* lives in `src/main.rs` (`src/dag_lib.rs`,
//! `src/cio.rs`); see the module documentation in `src/ffi.rs` for why the two
//! representations of `node_t *` differ.

pub mod ffi;
