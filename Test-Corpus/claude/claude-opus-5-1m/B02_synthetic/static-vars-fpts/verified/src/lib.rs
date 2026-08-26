//! Library form of the translation of `c_src/`.
//!
//! The `ffi` module re-exports every non-static function of the C program with
//! the C ABI and the C symbol names, backed by process-global state that
//! mirrors the file-scope `static` variables of the C translation units.

pub mod analyzer;
pub mod cio;
pub mod driver;
pub mod ffi;
pub mod tokenizer;
