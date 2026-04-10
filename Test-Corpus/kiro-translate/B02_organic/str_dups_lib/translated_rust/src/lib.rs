#![allow(
    non_camel_case_types,
    non_snake_case,
    unused_assignments,
    unused_variables,
    clippy::all
)]

use std::ffi::c_int;

mod stbds;

#[unsafe(no_mangle)]
pub extern "C" fn str_dups(num: c_int) {
    stbds::str_dups_impl(num);
}
