//! Shared-library half of the translation of `c_src/`.
//!
//! The CMake project compiles `src/mdcore.c` and `src/mdmain.c`; this crate root
//! exposes the C ABI surface that `mdcore.c` provides so the resulting `cdylib`
//! can be linked against exactly like the C object would be. The `driver`
//! executable (`src/main.rs`) is the translation of `mdmain.c`.
//!
//! Build configuration mirrors the CMake cache variables `OP` and `REPEAT` via
//! Cargo features — see `Cargo.toml` and `mdmacros.rs`.

pub mod mdcore;
pub mod mdmacros;

// Re-exported for Rust consumers; the exported linker symbols come from the
// `#[unsafe(no_mangle)]` definitions in `mdcore`.
pub use mdcore::{G_OP, G_OP_NAME, helper_call, helper_ptr, op_add, op_mul, op_sub, use_generated};
pub use mdmacros::{INIT, OP, OP_NAME, REPEAT};
