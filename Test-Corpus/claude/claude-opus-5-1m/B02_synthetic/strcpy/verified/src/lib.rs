//! Rust translation of the C driver in `c_src/`.
//!
//! * [`strcpy_fun`] is the translation of `c_src/src/lib.c`,
//! * [`ffi`] exports it under the same C ABI symbol as the C library,
//! * [`mem`] models the `main` stack frame of `c_src/src/main.c` (needed because
//!   the C code reads past the end of its buffers),
//! * [`scanf`] emulates the `scanf` conversions used by `c_src/src/main.c`.

pub mod cstr;
pub mod ffi;
pub mod frame_junk;
pub mod mem;
pub mod scanf;
pub mod strcpy_fun;
