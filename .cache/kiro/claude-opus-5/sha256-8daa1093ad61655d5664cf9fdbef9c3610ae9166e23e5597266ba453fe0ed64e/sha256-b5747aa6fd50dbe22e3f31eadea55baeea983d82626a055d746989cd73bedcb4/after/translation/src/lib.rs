//! A Rust translation of jansson 2.15.0 that is ABI and byte-output compatible
//! with the C library built from `c_src/`.
//!
//! Every public C entry point is re-exported with `#[unsafe(no_mangle)]` and
//! the C calling convention. The internal (`jsonp_*`, `hashtable_*`, `utf8_*`,
//! `strbuffer_*`, `dtoa_r`, ...) symbols that the C build also exports from the
//! shared object are provided as well.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::needless_return)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod cfmt;
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
pub mod types;
pub mod utf;
pub mod value;
pub mod varargs;
pub mod version;
