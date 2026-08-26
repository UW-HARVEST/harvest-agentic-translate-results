// Translated from pcre2_config.c
use crate::internal::*;
use crate::pcre2_pub::*;
use core::ffi::{c_char, c_int, c_void};

/* PCRE2_PRERELEASE is "-DEV" (non-empty), so the runtime test in the C code
selects XSTRING(PCRE2_MAJOR.PCRE2_MINOR) XSTRING(PCRE2_PRERELEASE PCRE2_DATE). */
static VERSION_STRING: &[u8] = b"10.48-DEV 2025-10-21\0";

/*************************************************
* Return info about what features are configured *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_config_8(what: u32, where_: *mut c_void) -> c_int {
    if where_.is_null()
    /* Requests a length */
    {
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
            | PCRE2_CONFIG_UNICODE => {
                return core::mem::size_of::<u32>() as c_int;
            }

            /* These are handled below */
            PCRE2_CONFIG_JITTARGET | PCRE2_CONFIG_UNICODE_VERSION | PCRE2_CONFIG_VERSION => {}

            _ => return PCRE2_ERROR_BADOPTION,
        }
    }

    match what {
        PCRE2_CONFIG_BSR => {
            *(where_ as *mut u32) = PCRE2_BSR_UNICODE;
        }

        PCRE2_CONFIG_COMPILED_WIDTHS => {
            /* SUPPORT_PCRE2_8/16/32 are not defined in this build's config.h */
            *(where_ as *mut u32) = 0;
        }

        PCRE2_CONFIG_DEPTHLIMIT => {
            *(where_ as *mut u32) = MATCH_LIMIT_DEPTH;
        }

        PCRE2_CONFIG_EFFECTIVE_LINKSIZE => {
            *(where_ as *mut u32) =
                (LINK_SIZE * core::mem::size_of::<PCRE2_UCHAR>()) as u32;
        }

        PCRE2_CONFIG_HEAPLIMIT => {
            *(where_ as *mut u32) = HEAP_LIMIT;
        }

        PCRE2_CONFIG_JIT => {
            *(where_ as *mut u32) = 0;
        }

        PCRE2_CONFIG_JITTARGET => {
            return PCRE2_ERROR_BADOPTION;
        }

        PCRE2_CONFIG_LINKSIZE => {
            *(where_ as *mut u32) = 2u32; /* CONFIGURED_LINK_SIZE */
        }

        PCRE2_CONFIG_MATCHLIMIT => {
            *(where_ as *mut u32) = MATCH_LIMIT;
        }

        PCRE2_CONFIG_NEWLINE => {
            *(where_ as *mut u32) = NEWLINE_DEFAULT;
        }

        PCRE2_CONFIG_NEVER_BACKSLASH_C => {
            *(where_ as *mut u32) = 0;
        }

        PCRE2_CONFIG_PARENSLIMIT => {
            *(where_ as *mut u32) = PARENS_NEST_LIMIT;
        }

        PCRE2_CONFIG_STACKRECURSE => {
            *(where_ as *mut u32) = 0;
        }

        PCRE2_CONFIG_TABLES_LENGTH => {
            *(where_ as *mut u32) = TABLES_LENGTH as u32;
        }

        PCRE2_CONFIG_UNICODE_VERSION => {
            let v: *const c_char = crate::ucd_data::_pcre2_unicode_version_8.0;
            return (1 + (if where_.is_null() {
                strlen(v)
            } else {
                crate::string_utils::_pcre2_strcpy_c8_8(where_ as *mut PCRE2_UCHAR, v)
            })) as c_int;
        }

        PCRE2_CONFIG_UNICODE => {
            *(where_ as *mut u32) = 1;
        }

        PCRE2_CONFIG_VERSION => {
            let v: *const c_char = VERSION_STRING.as_ptr() as *const c_char;
            return (1 + (if where_.is_null() {
                strlen(v)
            } else {
                crate::string_utils::_pcre2_strcpy_c8_8(where_ as *mut PCRE2_UCHAR, v)
            })) as c_int;
        }

        _ => return PCRE2_ERROR_BADOPTION,
    }

    0
}
