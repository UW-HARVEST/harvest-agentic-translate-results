//! Rust translation of the Jansson JSON library (c_src/).
//!
//! The public ABI mirrors the C library exactly: every symbol exported by the
//! C shared object is exported here with the same name and signature.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]

pub mod dtoa;
pub mod dtoa_r;
pub mod dtoa_strtod;
pub mod dtoa_strtod2;
pub mod dtoa_tables;
pub mod dump;
pub mod error;
pub mod hashtable;
pub mod hashtable_seed;
pub mod jansson;
pub mod load;
pub mod libc;
pub mod memory;
pub mod pack_unpack;
pub mod strbuffer;
pub mod strconv;
pub mod utf;
pub mod value;
pub mod va;
pub mod version;
