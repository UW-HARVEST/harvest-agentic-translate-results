//! Rust translation of the C library in `c_src/`.
//!
//! The C build (`c_src/CMakeLists.txt`) compiles `src/lib.c` into a single
//! shared object. None of its functions are `static`, so all six are part of
//! the public ABI even though `include/lib.h` only declares `doubleneg`:
//!
//! * `convert_double_to_int`
//! * `find_value_in_buffer`
//! * `process_negation`
//! * `create_numeric_buffer`
//! * `calculate_with_doubles`
//! * `doubleneg`
//!
//! `include/lib.h` contains no namespace/renaming macros, so the linker names
//! match the source-level names exactly.
//!
//! Undefined behaviour in the original (out-of-range `double`→`int` casts,
//! signed overflow) is reproduced as observed on x86-64 rather than fixed.

mod buffer;
mod conv;
mod doubleneg;
mod doubles;
mod ffi;
mod negation;

pub use buffer::{create_numeric_buffer, find_value_in_buffer};
pub use conv::convert_double_to_int;
pub use doubleneg::doubleneg;
pub use doubles::calculate_with_doubles;
pub use negation::process_negation;
