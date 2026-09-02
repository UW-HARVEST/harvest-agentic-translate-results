//! Rust translation of the jansson 2.15.0 C library.
//!
//! The crate reproduces the complete public ABI (including private/internal
//! symbols that the C shared object happens to export) and byte-identical
//! behaviour.
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(unused_parens)]
#![allow(unused_assignments)]
#![allow(clippy::all)]

pub mod cffi;
pub mod dtoa;
pub mod dtoa_tables;
pub mod dump;
pub mod error;
pub mod hashtable;
pub mod hashtable_seed;
pub mod jtypes;
pub mod load;
pub mod lookup3;
pub mod memory;
pub mod pack_unpack;
pub mod strbuffer;
pub mod strconv;
pub mod trampolines;
pub mod utf;
pub mod valist;
pub mod value;
pub mod version;
