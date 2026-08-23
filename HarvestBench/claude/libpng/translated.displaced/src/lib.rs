//! libpng 1.6.59.git — a faithful Rust translation of the C library.
//!
//! Every public symbol exported by the C shared library is exported here with
//! the same name, signature and behaviour (bug for bug).

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_parens)]
#![allow(unused_assignments)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(unused_unsafe)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(clippy::all)]

pub mod ctypes;
pub mod pngconsts;
pub mod pngtypes;

/// C string literal helper: `cstr!("text")` is a NUL terminated `*const c_char`.
#[macro_export]
macro_rules! cstr {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const $crate::ctypes::c_char
    };
}

pub mod png;
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

pub use crate::ctypes::*;
pub use crate::pngconsts::*;
pub use crate::pngtypes::*;

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

#[cfg(test)]
mod layout;
