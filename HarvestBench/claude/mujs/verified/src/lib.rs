//! MuJS 1.3.8 — faithful Rust transliteration exporting the full C ABI.
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(unused_parens, unused_assignments, unused_mut, dead_code)]
#![allow(clippy::all)]

pub mod types;
pub mod except;
pub mod cutil;
pub mod utfdata;
pub mod shim;

pub mod utf;
pub mod regexp;
pub mod jsdtoa;
pub mod jsintern;
pub mod jsvalue;
pub mod jsproperty;
pub mod jsrun;
pub mod jsgc;
pub mod jsstate;
pub mod jslex;
pub mod jsparse;
pub mod jscompile;
pub mod jserror;
pub mod jsbuiltin;
pub mod jsobject;
pub mod jsarray;
pub mod jsboolean;
pub mod jsnumber;
pub mod jsmath;
pub mod jsstring;
pub mod jsregexp;
pub mod jsfunction;
pub mod jsdate;
pub mod json;
pub mod jsrepr;
