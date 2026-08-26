//! Rust translation of the C library in `c_src/` (an inlined, lightly modified
//! copy of Sean Barrett's `stb_ds.h` plus two demo helpers).
//!
//! Every function that the C shared library exports is re-implemented here with
//! the identical C ABI, identical linker symbol name and identical observable
//! behaviour (including quirks such as the sign-extension in the siphash byte
//! loader and the missing `temp_key` update on one of the "key already present"
//! paths of `stbds_hmput_key`).
//!
//! Exported symbols (must match `nm -D` of the C `.so`):
//!   stbds_arrgrowf, stbds_arrfreef, stbds_rand_seed, stbds_hash_string,
//!   stbds_hash_bytes, stbds_hmget_key_ts, stbds_hmget_key, stbds_hmput_default,
//!   stbds_shmode_func, stbds_hmdel_key, stbds_stralloc, stbds_hmput_key,
//!   stbds_strreset, stbds_hmfree_func, strkey, helxo

#![allow(clippy::missing_safety_doc)]

pub mod arena;
pub mod array;
pub mod demo;
pub mod ffi;
pub mod hash;
pub mod hashmap;
