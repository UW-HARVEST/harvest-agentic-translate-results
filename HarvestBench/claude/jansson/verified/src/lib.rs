//! Rust translation of the Jansson 2.15 C library, exporting the same C ABI.
#![feature(c_variadic)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(unused_parens)]
#![allow(clippy::all)]

use core::ffi::c_void;

pub mod types;

pub mod dump;
pub mod dtoa;
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
pub mod version;

// Shared libc memcmp helper used by dump.rs's compare_keys.
extern "C" {
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> core::ffi::c_int;
}

#[inline]
pub(crate) unsafe fn c_memcmp(a: *const c_void, b: *const c_void, n: usize) -> core::ffi::c_int {
    memcmp(a, b, n)
}
