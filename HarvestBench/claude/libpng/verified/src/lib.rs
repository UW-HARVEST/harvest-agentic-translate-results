//! Pure-Rust translation of libpng 1.6.59, preserving the complete public ABI.
//!
//! Every public C function is exported with `#[unsafe(no_mangle)] extern "C"`
//! and the exact C signature.  Internal libpng "extern" functions
//! (PNG_INTERNAL_FUNCTION) are also exported so that behaviour and the symbol
//! table match the reference C shared library.
//!
//! DEFLATE/INFLATE is delegated to the system zlib (linked via build.rs) so
//! that compressed output is byte-identical to the C reference build.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_parens)]
#![allow(unused_assignments)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(clippy::all)]

#[macro_use]
mod cffi;
pub mod consts;
pub mod helpers;
pub mod pstruct;
pub mod ptypes;

// Translated libpng source modules.
mod png;
mod pngerror;
mod pngget;
mod pngmem;
mod pngpread;
mod pngread;
mod pngrio;
mod pngrtran;
mod pngrutil;
mod pngset;
mod pngtrans;
mod pngwio;
mod pngwrite;
mod pngwtran;
mod pngwutil;

/// Re-exports of every translated module's functions so that cross-module
/// calls are ordinary (checked) Rust calls.  All the `png_*` functions are
/// defined once (with `#[unsafe(no_mangle)]`) in their home module and made
/// callable everywhere through this facade.
pub(crate) mod api {
    pub use crate::png::*;
    pub use crate::pngerror::*;
    pub use crate::pngget::*;
    pub use crate::pngmem::*;
    pub use crate::pngpread::*;
    pub use crate::pngread::*;
    pub use crate::pngrio::*;
    pub use crate::pngrtran::*;
    pub use crate::pngrutil::*;
    pub use crate::pngset::*;
    pub use crate::pngtrans::*;
    pub use crate::pngwio::*;
    pub use crate::pngwrite::*;
    pub use crate::pngwtran::*;
    pub use crate::pngwutil::*;
}

/// Prelude of shared items imported by each translated module.
pub(crate) mod prelude {
    pub use crate::api::*;
    pub use crate::cffi::*;
    pub use crate::consts::*;
    pub use crate::helpers::*;
    pub use crate::pstruct::*;
    pub use crate::ptypes::*;
    pub use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};
    pub use core::ptr;
}
