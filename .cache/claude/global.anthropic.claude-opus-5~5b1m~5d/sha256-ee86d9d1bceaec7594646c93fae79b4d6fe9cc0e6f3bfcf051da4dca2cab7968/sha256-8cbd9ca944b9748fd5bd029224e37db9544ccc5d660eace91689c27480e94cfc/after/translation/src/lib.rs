//! Rust translation of the `driver` C library in `c_src/`.
//!
//! The C build (`c_src/CMakeLists.txt`) compiles `src/matrix.c`, `src/write.c`
//! and `src/driver.c` into a single shared object, `libdriver.so`, exporting
//! seven functions:
//!
//! ```text
//! allocate_matrix                 (matrix.c — not in a header, but non-static)
//! free_matrix                     (matrix.c / matrix.h)
//! initialize_matrix_from_string   (matrix.c / matrix.h)
//! multiply_matrices               (matrix.c / matrix.h)
//! matrix_to_string                (matrix.c / matrix.h)
//! write_to_file                   (write.c  / write.h)
//! driver                          (driver.c)
//! ```
//!
//! There are no namespace/renaming macros in the headers, so the linker names
//! are the source-level names.

pub mod cffi;
pub mod driver;
pub mod matrix;
pub mod write;
