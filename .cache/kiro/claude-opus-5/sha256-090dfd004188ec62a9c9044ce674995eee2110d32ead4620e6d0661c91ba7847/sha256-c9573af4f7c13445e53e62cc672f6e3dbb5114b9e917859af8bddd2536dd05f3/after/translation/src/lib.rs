//! libpng 1.6.59.git — Rust translation of the complete C library.
//!
//! Structure mirrors the C sources one module per `c_src/src/*.c` file.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_parens)]
#![allow(unused_unsafe)]
#![allow(unused_variables)]
#![allow(unused_assignments)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
#![allow(unused_comparisons)]
#![allow(clippy::all)]

pub mod consts;
pub mod helpers;
pub mod readctrl;
pub mod srgb;
pub mod sys;
pub mod types;

pub(crate) mod prelude;

mod png;
mod png2;
mod png3;
mod pngerror;
mod pngget;
mod pngmem;
mod pngpread;
mod pngread;
mod pngread2;
mod pngread3;
mod pngrio;
mod pngrtran;
mod pngrtran2;
mod pngrtran3;
mod pngrtran4;
mod pngrtran5;
mod pngrutil;
mod pngrutil2;
mod pngrutil3;
mod pngrutil4;
mod pngset;
mod pngset2;
mod pngtrans;
mod pngwio;
mod pngwrite;
mod pngwrite2;
mod pngwtran;
mod pngwutil;
mod pngwutil2;
