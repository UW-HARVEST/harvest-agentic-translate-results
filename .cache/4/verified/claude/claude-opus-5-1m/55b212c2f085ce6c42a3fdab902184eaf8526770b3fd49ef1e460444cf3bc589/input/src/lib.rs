//! Rust translation of the Jansson JSON library (version 2.15.0).
//!
//! Every public symbol exported by the original C shared object is exported
//! here with the same name, signature and behaviour.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(static_mut_refs)]
#![allow(unused_unsafe)]
#![allow(clippy::missing_safety_doc)]

pub mod types;

pub mod dtoa;
pub mod dtoa_strtod;
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
pub mod vararg;
pub mod version;
