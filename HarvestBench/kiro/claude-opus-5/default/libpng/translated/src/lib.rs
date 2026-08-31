//! libpng 1.6.59, translated from C to Rust.
//!
//! The public interface, the linker symbol names, the error and warning
//! strings, the chunk encoding and the compressed byte streams are all
//! identical to the reference C build.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_parens)]
#![allow(unused_assignments)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_unsafe)]
#![allow(clippy::all)]

pub mod pngpriv;

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

pub use pngpriv::*;

pub use png::*;
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
