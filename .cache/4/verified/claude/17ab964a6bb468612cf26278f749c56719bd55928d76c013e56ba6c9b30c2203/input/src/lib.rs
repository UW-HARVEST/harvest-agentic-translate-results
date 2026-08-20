/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Rust translation of the `driver` C shared library found in `c_src/`.
//!
//! The library exports the following public ABI (identical to the symbols
//! exported by `libdriver.so` built from the C sources):
//!
//! * `allocate_matrix`
//! * `free_matrix`
//! * `initialize_matrix_from_string`
//! * `multiply_matrices`
//! * `matrix_to_string`
//! * `write_to_file`
//! * `driver`

pub mod cffi;
pub mod driver;
pub mod matrix;
pub mod write;

pub use crate::driver::driver;
pub use crate::matrix::{
    allocate_matrix, free_matrix, initialize_matrix_from_string, matrix_to_string,
    multiply_matrices, matrix_t,
};
pub use crate::write::write_to_file;
