//! Raw C library bindings used to guarantee byte-identical behaviour with the
//! original C implementation (formatting, locale handling, allocation, ...).

use core::ffi::{c_char, c_int, c_void};

/// Partial mirror of glibc's `struct lconv`.  Only the first member
/// (`decimal_point`) is ever accessed by cJSON.
#[repr(C)]
pub struct Lconv {
    pub decimal_point: *mut c_char,
    pub thousands_sep: *mut c_char,
    pub grouping: *mut c_char,
    pub int_curr_symbol: *mut c_char,
    pub currency_symbol: *mut c_char,
    pub mon_decimal_point: *mut c_char,
    pub mon_thousands_sep: *mut c_char,
    pub mon_grouping: *mut c_char,
    pub positive_sign: *mut c_char,
    pub negative_sign: *mut c_char,
    pub int_frac_digits: c_char,
    pub frac_digits: c_char,
    pub p_cs_precedes: c_char,
    pub p_sep_by_space: c_char,
    pub n_cs_precedes: c_char,
    pub n_sep_by_space: c_char,
    pub p_sign_posn: c_char,
    pub n_sign_posn: c_char,
    pub int_p_cs_precedes: c_char,
    pub int_p_sep_by_space: c_char,
    pub int_n_cs_precedes: c_char,
    pub int_n_sep_by_space: c_char,
    pub int_p_sign_posn: c_char,
    pub int_n_sign_posn: c_char,
}

extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;

    pub fn strlen(s: *const c_char) -> usize;
    pub fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    pub fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int;
    pub fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char;
    pub fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memset(dst: *mut c_void, c: c_int, n: usize) -> *mut c_void;

    pub fn strtod(s: *const c_char, endptr: *mut *mut c_char) -> f64;
    pub fn tolower(c: c_int) -> c_int;

    pub fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    pub fn sscanf(s: *const c_char, fmt: *const c_char, ...) -> c_int;
    pub fn printf(fmt: *const c_char, ...) -> c_int;

    pub fn exit(status: c_int) -> !;

    pub fn localeconv() -> *mut Lconv;
}
