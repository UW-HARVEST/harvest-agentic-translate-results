//! Rust translation of the MuJS JavaScript interpreter (`c_src/`).
//!
//! The crate is a `cdylib` that exports the exact same public ABI as the C
//! library built from `c_src/`.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(unused_assignments)]
#![allow(unreachable_patterns)]
#![allow(dead_code)]

#[macro_use]
pub mod macros;

pub mod cstd;
pub mod jsi;
pub mod vararg;

pub mod jsarray;
pub mod jsboolean;
pub mod jsbuiltin;
pub mod jscompile;
pub mod jsdate;
pub mod jsdtoa;
pub mod jserror;
pub mod jsfunction;
pub mod jsgc;
pub mod jsintern;
pub mod jslex;
pub mod jsmath;
pub mod jsnumber;
pub mod jsobject;
pub mod json;
pub mod jsparse;
pub mod jsproperty;
pub mod jsregexp;
pub mod jsrepr;
pub mod jsrun;
pub mod jsstate;
pub mod jsstring;
pub mod jsvalue;
pub mod regexp;
pub mod utf;
pub mod utfdata;
