//! Rust translation of the C `driver` library in `c_src/`.
//!
//! Exports the same public ABI as the CMake-built `libdriver.so`:
//! `allocate_matrix`, `free_matrix`, `initialize_matrix_from_string`,
//! `multiply_matrices`, `matrix_to_string`, `write_to_file`, `driver`.
//!
//! Behaviour — including diagnostics on `stderr`, `errno` return values,
//! allocation via libc `malloc`/`free` (so callers may `free()` returned
//! pointers), and the original code's quirks — is reproduced exactly.

#![allow(non_camel_case_types)]

mod cstd;
mod driver;
mod matrix;
mod write;

pub use driver::driver;
pub use matrix::{
    allocate_matrix, free_matrix, initialize_matrix_from_string, matrix_t, matrix_to_string,
    multiply_matrices,
};
pub use write::write_to_file;
