//! Shared declarations for `randombytes.h`

use core::ffi::{c_char, c_int, c_void};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct randombytes_implementation {
    pub implementation_name: Option<extern "C" fn() -> *const c_char>,
    pub random: Option<extern "C" fn() -> u32>,
    pub stir: Option<extern "C" fn()>,
    pub uniform: Option<extern "C" fn(u32) -> u32>,
    pub buf: Option<unsafe extern "C" fn(*mut c_void, usize)>,
    pub close: Option<extern "C" fn() -> c_int>,
}

unsafe impl Sync for randombytes_implementation {}

pub const RANDOMBYTES_SEEDBYTES: usize = 32;

pub mod internal_random;
pub mod randombytes;
pub mod sysrandom;
