//! Rust translation of the C library in `c_src/`.
//!
//! The C build globs `c_src/src/*.c` into a single shared library which exports
//! exactly six symbols:
//!
//! | Symbol                   | Signature                                              |
//! |--------------------------|--------------------------------------------------------|
//! | `convert_double_to_int`  | `int (double)`                                         |
//! | `find_value_in_buffer`   | `int (const char *, size_t, int)`                      |
//! | `process_negation`       | `int (int)`                                            |
//! | `create_numeric_buffer`  | `void (char *, int, int)`                              |
//! | `calculate_with_doubles` | `double (int, int, int)`                               |
//! | `doubleneg`              | `int (int, int, int, int)`                             |
//!
//! There are no namespace/renaming macros in `include/lib.h`, so the linker
//! names match the source-level names one-for-one.
//!
//! Behaviour that is undefined or implementation-defined in the original is
//! reproduced rather than corrected -- see [`cvt`] for the out-of-range
//! `double`-to-`int` casts and [`buffer`] for the signed-`char` truncation.

// The crate is a C ABI shim, so the exported entry points intentionally take
// raw pointers and use C integer types.
#![allow(clippy::missing_safety_doc)]
// The crate name mirrors the C library's output name, which is derived from the
// enclosing directory and is not snake case.
#![allow(non_snake_case)]

mod buffer;
mod cvt;
mod dmath;
mod doubleneg;
mod ffi;
mod negation;

// Re-exported so the `#[unsafe(no_mangle)]` definitions are reachable from the
// crate root and are not pruned as dead code.
pub use buffer::{create_numeric_buffer, find_value_in_buffer};
pub use cvt::convert_double_to_int;
pub use dmath::calculate_with_doubles;
pub use doubleneg::doubleneg;
pub use negation::process_negation;
