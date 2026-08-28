//! Rust translation of the C `driver` library (matrix multiply + file write).
//!
//! Behaviour, error ordering, messages and return codes mirror the original C
//! exactly, including its bugs (unchecked allocations, the `free()` instead of
//! `free_matrix()` in `driver`).

#![allow(non_camel_case_types)]

mod cutil;
mod driver;
mod matrix;
mod write;

pub use driver::driver;
pub use matrix::{
    allocate_matrix, free_matrix, initialize_matrix_from_string, matrix_to_string,
    multiply_matrices, matrix_t,
};
pub use write::write_to_file;
