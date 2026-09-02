//! Everything every translated module needs, in one glob import.
//!
//! `use crate::prelude::*;` at the top of a module gives access to:
//!  * all libpng types (`png_structrp`, `png_bytep`, ...)
//!  * all libpng constants (`PNG_COLOR_TYPE_RGB`, `png_IHDR`, ...)
//!  * the macro-equivalent helper functions (`PNG_ROWBYTES`, ...)
//!  * the C library / zlib bindings (`memcpy`, `deflate`, `z_stream`, ...)
//!  * every *public* function of every other translated module
//!
//! Note: file-local (C `static`) functions must NOT be declared `pub`, so they
//! never leak into this prelude and can share names across modules.

pub use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

pub use crate::consts::*;
pub use crate::helpers::*;
pub use crate::readctrl::*;
pub use crate::srgb::*;
pub use crate::sys::*;
pub use crate::types::*;

pub use crate::png::*;
pub use crate::png2::*;
pub use crate::png3::*;
pub use crate::pngerror::*;
pub use crate::pngget::*;
pub use crate::pngmem::*;
pub use crate::pngpread::*;
pub use crate::pngread::*;
pub use crate::pngread2::*;
pub use crate::pngread3::*;
pub use crate::pngrio::*;
pub use crate::pngrtran::*;
pub use crate::pngrtran2::*;
pub use crate::pngrtran3::*;
pub use crate::pngrtran4::*;
pub use crate::pngrtran5::*;
pub use crate::pngrutil::*;
pub use crate::pngrutil2::*;
pub use crate::pngrutil3::*;
pub use crate::pngrutil4::*;
pub use crate::pngset::*;
pub use crate::pngset2::*;
pub use crate::pngtrans::*;
pub use crate::pngwio::*;
pub use crate::pngwrite::*;
pub use crate::pngwrite2::*;
pub use crate::pngwtran::*;
pub use crate::pngwutil::*;
pub use crate::pngwutil2::*;
