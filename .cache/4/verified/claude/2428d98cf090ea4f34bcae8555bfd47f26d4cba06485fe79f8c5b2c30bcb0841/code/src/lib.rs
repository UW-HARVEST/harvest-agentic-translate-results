//! Rust translation of the C library in `c_src/` (a stb_ds.h based data
//! structure library plus its `str_dups` test entry point).
//!
//! The translation is intentionally literal: every arithmetic quirk,
//! sign-extension bug and evaluation order of the original C is reproduced so
//! that the shared library produces byte-identical output for identical input.
//!
//! Public ABI (as exported by the C `.so`):
//!   stbds_arrfreef, stbds_arrgrowf, stbds_hash_bytes, stbds_hash_string,
//!   stbds_hmdel_key, stbds_hmfree_func, stbds_hmget_key, stbds_hmget_key_ts,
//!   stbds_hmput_default, stbds_hmput_key, stbds_rand_seed, stbds_shmode_func,
//!   stbds_stralloc, stbds_strreset, str_dups, strkey

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]

pub mod ffi;
pub mod stb_ds;
pub mod str_dups;
