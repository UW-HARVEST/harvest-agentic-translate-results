//! Rust translation of the C library in `c_src/`.
//!
//! Exports the same public ABI as the C shared library built by
//! `c_src/CMakeLists.txt` (`libdriver.so`): `driver`, `forward_goto_example`,
//! and `open_with_cleanup`. No preprocessor namespace macros are involved, so
//! the linker symbol names are identical to the source-level names.

mod cstdio;
mod goto;

pub use goto::{driver, forward_goto_example, open_with_cleanup};
