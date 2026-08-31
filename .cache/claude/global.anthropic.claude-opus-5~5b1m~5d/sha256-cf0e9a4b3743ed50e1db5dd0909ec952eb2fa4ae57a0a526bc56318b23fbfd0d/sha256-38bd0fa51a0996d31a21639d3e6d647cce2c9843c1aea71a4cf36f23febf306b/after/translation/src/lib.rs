//! MuJS 1.3.8 -- faithful Rust translation of the C library.
#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_parens,
    unused_assignments,
    unused_variables,
    unused_mut,
    unused_unsafe,
    unused_imports,
    dead_code,
    static_mut_refs
)]

#[macro_use]
pub mod jsi;

pub mod astnames;
pub mod enums;
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
pub mod opnames;
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
pub mod varargs;

pub use enums::*;
pub use jsarray::*;
pub use jsboolean::*;
pub use jsbuiltin::*;
pub use jscompile::*;
pub use jsdate::*;
pub use jsdtoa::*;
pub use jserror::*;
pub use jsfunction::*;
pub use jsgc::*;
pub use jsi::*;
pub use jsintern::*;
pub use jslex::*;
pub use jsmath::*;
pub use jsnumber::*;
pub use jsobject::*;
pub use json::*;
pub use jsparse::*;
pub use jsproperty::*;
pub use jsregexp::*;
pub use jsrepr::*;
pub use jsrun::*;
pub use jsstate::*;
pub use jsstring::*;
pub use jsvalue::*;
pub use regexp::*;
pub use utf::*;
