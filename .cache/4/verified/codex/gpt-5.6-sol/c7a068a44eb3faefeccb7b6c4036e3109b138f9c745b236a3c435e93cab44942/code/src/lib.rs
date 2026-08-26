#![allow(clippy::missing_safety_doc)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unsafe_op_in_unsafe_fn)]

mod dtoa;
mod dump;
mod error;
mod load;
mod memory;
mod pack;
mod private;
mod types;
mod value;

pub use dtoa::*;
pub use dump::*;
pub use error::*;
pub use load::*;
pub use memory::*;
pub use pack::*;
pub use private::*;
pub use types::*;
pub use value::*;

use std::ffi::{c_char, c_int};

static VERSION: &[u8] = b"2.15.0\0";

#[unsafe(no_mangle)]
pub extern "C" fn jansson_version_str() -> *const c_char {
    VERSION.as_ptr().cast()
}

#[unsafe(no_mangle)]
pub extern "C" fn jansson_version_cmp(major: c_int, minor: c_int, micro: c_int) -> c_int {
    let diff = 2 - major;
    if diff != 0 {
        return diff;
    }
    let diff = 15 - minor;
    if diff != 0 {
        return diff;
    }
    -micro
}
