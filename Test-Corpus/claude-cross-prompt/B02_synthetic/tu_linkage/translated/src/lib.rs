// Translated from C to Rust. This is a library (cdylib).
// All public C symbols are exposed via #[unsafe(no_mangle)] extern "C".

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

pub mod util;
pub mod lib_target;
pub mod a;
pub mod b;
pub mod engine;
