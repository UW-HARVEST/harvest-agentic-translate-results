//! Public API types & constants from include/zdict.h
#![allow(non_snake_case, dead_code, non_upper_case_globals, non_camel_case_types)]

pub const ZDICT_DICTSIZE_MIN: usize = 256;
pub const ZDICT_CONTENTSIZE_MIN: usize = 128;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZDICT_params_t {
    pub compressionLevel: core::ffi::c_int,
    pub notificationLevel: core::ffi::c_uint,
    pub dictID: core::ffi::c_uint,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZDICT_cover_params_t {
    pub k: core::ffi::c_uint,
    pub d: core::ffi::c_uint,
    pub steps: core::ffi::c_uint,
    pub nbThreads: core::ffi::c_uint,
    pub splitPoint: core::ffi::c_double,
    pub shrinkDict: core::ffi::c_uint,
    pub shrinkDictMaxRegression: core::ffi::c_uint,
    pub zParams: ZDICT_params_t,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZDICT_fastCover_params_t {
    pub k: core::ffi::c_uint,
    pub d: core::ffi::c_uint,
    pub f: core::ffi::c_uint,
    pub steps: core::ffi::c_uint,
    pub nbThreads: core::ffi::c_uint,
    pub splitPoint: core::ffi::c_double,
    pub accel: core::ffi::c_uint,
    pub shrinkDict: core::ffi::c_uint,
    pub shrinkDictMaxRegression: core::ffi::c_uint,
    pub zParams: ZDICT_params_t,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZDICT_legacy_params_t {
    pub selectivityLevel: core::ffi::c_uint,
    pub zParams: ZDICT_params_t,
}
