// Translation of c_src/src/driver.c and c_src/include/driver.h
//
// Original C library:
//   Copyright 2025 MIT Lincoln Laboratory (MIT licence, see c_src/)
//
// The C library exports exactly three public symbols:
//   fma_array, call_fma, driver
// There are no namespace-renaming macros in the public header, so the Rust
// linker names are identical to the C source-level names.

#![allow(clippy::missing_safety_doc)]

pub mod cstdio;
pub mod driver;
