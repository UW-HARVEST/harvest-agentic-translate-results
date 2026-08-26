//! A translation of libpng 1.6.59.git into Rust.
//!
//! The crate is a mechanical, behaviour-preserving translation of the C
//! sources in `c_src/`.  It exports exactly the same public ABI as the C
//! shared library.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_parens)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(unused_assignments)]
#![allow(unused_unsafe)]
#![allow(unused_imports)]
#![allow(unused_labels)]
#![allow(dead_code)]
#![allow(clippy::all)]

pub mod ffi;
pub mod pngtypes;
pub mod util;

pub mod png_c;
pub mod pngerror;
pub mod pngget;
pub mod pngmem;
pub mod pngpread;
pub mod pngread;
pub mod pngrio;
pub mod pngrtran;
pub mod pngrutil;
pub mod pngset;
pub mod pngtrans;
pub mod pngwio;
pub mod pngwrite;
pub mod pngwtran;
pub mod pngwutil;

pub use ffi::*;
pub use pngtypes::*;
pub use util::*;

pub use png_c::*;
pub use pngerror::*;
pub use pngget::*;
pub use pngmem::*;
pub use pngpread::*;
pub use pngread::*;
pub use pngrio::*;
pub use pngrtran::*;
pub use pngrutil::*;
pub use pngset::*;
pub use pngtrans::*;
pub use pngwio::*;
pub use pngwrite::*;
pub use pngwtran::*;
pub use pngwutil::*;

/// `PNG_ABORT()`
#[inline]
pub unsafe fn PNG_ABORT() -> ! {
    crate::ffi::abort()
}
