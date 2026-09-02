//! Rust translation of the C library in `c_src/`.
//!
//! The C library is a single translation unit (`c_src/src/lib.c`) that embeds
//! Sean Barrett's `stb_ds` implementation together with two driver functions
//! (`strkey`, `str_put`).  The public ABI consists of the 16 symbols that
//! `nm -D` reports for the C shared object; each is re-created here with
//! `#[unsafe(no_mangle)] extern "C"` and the original signature.
//!
//! Behaviour — including the quirks of the original (the sign-extending
//! `int` sub-expressions in `stbds_siphash_bytes`, the `stbds_arrfreef(NULL)`
//! hazard, and the `printf("%s %d\n", struct_by_value, ...)` call in
//! `str_put`) — is reproduced as-is rather than corrected.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[macro_use]
mod c;

mod types;

mod arr;
mod hash;
mod strings;
mod testapi;

// Re-export so that the symbols are unambiguously part of the cdylib.
pub use arr::{stbds_arrfreef, stbds_arrgrowf};
pub use hash::{
    stbds_hash_bytes, stbds_hash_string, stbds_hmdel_key, stbds_hmfree_func, stbds_hmget_key,
    stbds_hmget_key_ts, stbds_hmput_default, stbds_hmput_key, stbds_rand_seed, stbds_shmode_func,
};
pub use strings::{stbds_stralloc, stbds_strreset};
pub use testapi::{str_put, strkey};
