//! Rust translation of the C library in `../c_src`.
//!
//! The C build globs `src/lib.c` into one shared object exporting ten symbols:
//! `increment_counter`, `decrement_counter`, `multiply_counter`,
//! `reset_counter`, `is_string_empty`, `find_char_in_buffer`, `create_buffer`,
//! `validate_uint16_range`, `apply_operation` and `charinbuf`. All ten are
//! re-exported here with the same names and signatures; `include/lib.h` defines
//! no namespace macros, so no symbol renaming is involved.
//!
//! Guiding rules for this translation:
//! * Observable behaviour is preserved, bugs included. Nothing is "fixed".
//! * Error checks stay in their original order.
//! * Formatted output goes through libc `printf` with the original format
//!   strings, and heap buffers come from libc `malloc`, so stdout bytes and the
//!   `free()`-ability of returned pointers are unchanged.

mod charinbuf;
mod counter;
mod cruntime;
mod helpers;

use core::ffi::c_int;

/// `UINT16_MAX` from `<stdint.h>`, which the preprocessor expands to `65535`.
///
/// In the C source it appears both in an `int` comparison
/// (`value > UINT16_MAX`) and as a `%u` argument, so it is kept as a `c_int`
/// and cast at the `printf` call sites.
pub(crate) const UINT16_MAX: c_int = 65535;
