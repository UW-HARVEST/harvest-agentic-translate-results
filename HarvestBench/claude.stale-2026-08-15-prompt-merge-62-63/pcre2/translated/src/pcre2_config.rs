use crate::pcre2_internal::*;
use crate::pcre2_string_utils::*;
use core::ffi::{c_int, c_void};

// unicode_version D symbol: a pointer variable to a C string.
#[no_mangle]
pub static mut _pcre2_unicode_version_8: *const u8 = b"17.0.0\0".as_ptr();

const VERSION_STR: &[u8] = b"10.48-DEV 2025-10-21\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_config_8(what: u32, where_: *mut c_void) -> c_int {
    if where_.is_null() {
        match what {
            PCRE2_CONFIG_BSR
            | PCRE2_CONFIG_COMPILED_WIDTHS
            | PCRE2_CONFIG_DEPTHLIMIT
            | PCRE2_CONFIG_EFFECTIVE_LINKSIZE
            | PCRE2_CONFIG_HEAPLIMIT
            | PCRE2_CONFIG_JIT
            | PCRE2_CONFIG_LINKSIZE
            | PCRE2_CONFIG_MATCHLIMIT
            | PCRE2_CONFIG_NEVER_BACKSLASH_C
            | PCRE2_CONFIG_NEWLINE
            | PCRE2_CONFIG_PARENSLIMIT
            | PCRE2_CONFIG_STACKRECURSE
            | PCRE2_CONFIG_TABLES_LENGTH
            | PCRE2_CONFIG_UNICODE => return core::mem::size_of::<u32>() as c_int,

            PCRE2_CONFIG_JITTARGET | PCRE2_CONFIG_UNICODE_VERSION | PCRE2_CONFIG_VERSION => {}
            _ => return PCRE2_ERROR_BADOPTION,
        }
    }

    let w = where_ as *mut u32;
    match what {
        PCRE2_CONFIG_BSR => {
            *w = PCRE2_BSR_UNICODE;
        }
        PCRE2_CONFIG_COMPILED_WIDTHS => {
            // Only 8-bit built in this configuration.
            *w = 1 << 0;
        }
        PCRE2_CONFIG_DEPTHLIMIT => {
            *w = MATCH_LIMIT_DEPTH;
        }
        PCRE2_CONFIG_EFFECTIVE_LINKSIZE => {
            *w = (LINK_SIZE * core::mem::size_of::<PCRE2_UCHAR>()) as u32;
        }
        PCRE2_CONFIG_HEAPLIMIT => {
            *w = HEAP_LIMIT;
        }
        PCRE2_CONFIG_JIT => {
            *w = 0;
        }
        PCRE2_CONFIG_JITTARGET => {
            return PCRE2_ERROR_BADOPTION;
        }
        PCRE2_CONFIG_LINKSIZE => {
            *w = 2;
        }
        PCRE2_CONFIG_MATCHLIMIT => {
            *w = MATCH_LIMIT;
        }
        PCRE2_CONFIG_NEWLINE => {
            *w = NEWLINE_DEFAULT as u32;
        }
        PCRE2_CONFIG_NEVER_BACKSLASH_C => {
            *w = 0;
        }
        PCRE2_CONFIG_PARENSLIMIT => {
            *w = PARENS_NEST_LIMIT;
        }
        PCRE2_CONFIG_STACKRECURSE => {
            *w = 0;
        }
        PCRE2_CONFIG_TABLES_LENGTH => {
            *w = TABLES_LENGTH as u32;
        }
        PCRE2_CONFIG_UNICODE_VERSION => {
            let v = _pcre2_unicode_version_8 as *const core::ffi::c_char;
            let len = if where_.is_null() {
                crate::pcre2_internal::strlen(v)
            } else {
                _pcre2_strcpy_c8_8(where_ as *mut PCRE2_UCHAR, v)
            };
            return (1 + len) as c_int;
        }
        PCRE2_CONFIG_UNICODE => {
            *w = 1;
        }
        PCRE2_CONFIG_VERSION => {
            let v = VERSION_STR.as_ptr() as *const core::ffi::c_char;
            let len = if where_.is_null() {
                crate::pcre2_internal::strlen(v)
            } else {
                _pcre2_strcpy_c8_8(where_ as *mut PCRE2_UCHAR, v)
            };
            return (1 + len) as c_int;
        }
        _ => return PCRE2_ERROR_BADOPTION,
    }
    0
}
