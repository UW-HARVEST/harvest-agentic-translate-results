//! Rust translation of the Jansson JSON library (version 2.15.0).
//!
//! The public ABI mirrors the C library exactly: every symbol exported by the
//! C shared object is exported here with the same name and signature.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]
// The translation follows the C sources statement for statement, which leaves a
// number of variables that C declares up-front but only conditionally uses.
#![allow(unused_mut)]
#![allow(unused_assignments)]
#![allow(unused_variables)]

pub mod ffi;
pub mod jansson;

pub mod dtoa;
pub mod dtoa_tables;
pub mod dump;
pub mod error;
pub mod hashtable;
pub mod hashtable_seed;
pub mod load;
pub mod lookup3;
pub mod memory;
pub mod pack_unpack;
pub mod strbuffer;
pub mod strconv;
pub mod utf;
pub mod value;
pub mod varargs;
pub mod version;
