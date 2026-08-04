#![allow(
    non_camel_case_types,
    non_snake_case,
    unused_assignments,
    unused_variables,
    clippy::all
)]

use std::ffi::c_int;

pub mod stbds;

#[unsafe(no_mangle)]
pub extern "C" fn str_dups(num: c_int) {
    stbds::str_dups_impl(num);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strkey(n: c_int) -> *const u8 {
    stbds::strkey(n)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_rand_seed(seed: usize) {
    stbds::rand_seed(seed);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut u8, len: usize, seed: usize) -> usize {
    stbds::hash_bytes(p, len, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_string(s: *mut u8, seed: usize) -> usize {
    stbds::hash_string_export(s, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(a: *mut u8, elemsize: usize, addlen: usize, min_cap: usize) -> *mut u8 {
    stbds::arrgrowf(a, elemsize, addlen, min_cap)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut u8) {
    stbds::arrfreef(a);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmfree_func(a: *mut u8, elemsize: usize) {
    stbds::hmfree_func(a, elemsize);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key(a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, mode: c_int) -> *mut u8 {
    stbds::hmget_key_export(a, elemsize, key, keysize, mode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmget_key_ts(a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, temp: *mut isize, mode: c_int) -> *mut u8 {
    stbds::hmget_key_ts_export(a, elemsize, key, keysize, temp, mode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_default(a: *mut u8, elemsize: usize) -> *mut u8 {
    stbds::hmput_default_export(a, elemsize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmput_key(a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, mode: c_int) -> *mut u8 {
    stbds::hmput_key(a, elemsize, key, keysize, mode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hmdel_key(a: *mut u8, elemsize: usize, key: *mut u8, keysize: usize, keyoffset: usize, mode: c_int) -> *mut u8 {
    stbds::hmdel_key_export(a, elemsize, key, keysize, keyoffset, mode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_shmode_func(elemsize: usize, mode: c_int) -> *mut u8 {
    stbds::shmode_func(elemsize, mode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_stralloc(a: *mut stbds::StringArena, s: *mut u8) -> *mut u8 {
    stbds::stralloc(&mut *a, s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_strreset(a: *mut stbds::StringArena) {
    stbds::strreset(&mut *a);
}
