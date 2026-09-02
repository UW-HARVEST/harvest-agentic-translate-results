//! Translation of `pcre2_config.c`.
//!
//! Return info about what features are configured.

use crate::internal::*;
use crate::string_utils::_pcre2_strcpy_c8_8;
use core::ffi::{c_char, c_int, c_void};

// The version string, assembled from PCRE2_MAJOR / PCRE2_MINOR /
// PCRE2_PRERELEASE / PCRE2_DATE in pcre2.h.
//
//   PCRE2_MAJOR      10
//   PCRE2_MINOR      48
//   PCRE2_PRERELEASE -DEV
//   PCRE2_DATE       2025-10-21
//
// PCRE2_PRERELEASE is non-empty ("-DEV"), so the C code assembles:
//   XSTRING(PCRE2_MAJOR.PCRE2_MINOR) XSTRING(PCRE2_PRERELEASE PCRE2_DATE)
//   = "10.48" "-DEV 2025-10-21"
//   = "10.48-DEV 2025-10-21"
const VERSION_STRING: &[u8] = b"10.48-DEV 2025-10-21\0";

/// `pcre2_config()` — return information about configured features.
///
/// If `where` is NULL, the length of memory required is returned.
///
/// Returns:  0 if a numerical value is returned
///           >= 0 if a string value
///           PCRE2_ERROR_BADOPTION if `where` not recognized
///             or JIT target requested when JIT not enabled
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_config_8(what: u32, where_: *mut c_void) -> c_int {
    unsafe {
        if where_.is_null() {
            // Requests a length
            match what as i64 {
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

                // These are handled below.
                PCRE2_CONFIG_JITTARGET
                | PCRE2_CONFIG_UNICODE_VERSION
                | PCRE2_CONFIG_VERSION => {}

                _ => return PCRE2_ERROR_BADOPTION as c_int,
            }
        }

        match what as i64 {
            PCRE2_CONFIG_BSR => {
                // BSR_ANYCRLF is not defined in this configuration.
                *(where_ as *mut u32) = PCRE2_BSR_UNICODE as u32;
            }

            PCRE2_CONFIG_COMPILED_WIDTHS => {
                // Only SUPPORT_PCRE2_8 is defined.
                *(where_ as *mut u32) = 1 << 0;
            }

            PCRE2_CONFIG_DEPTHLIMIT => {
                *(where_ as *mut u32) = MATCH_LIMIT_DEPTH as u32;
            }

            PCRE2_CONFIG_EFFECTIVE_LINKSIZE => {
                *(where_ as *mut u32) =
                    (LINK_SIZE as usize * core::mem::size_of::<PCRE2_UCHAR>()) as u32;
            }

            PCRE2_CONFIG_HEAPLIMIT => {
                *(where_ as *mut u32) = HEAP_LIMIT as u32;
            }

            PCRE2_CONFIG_JIT => {
                // SUPPORT_JIT is not defined.
                *(where_ as *mut u32) = 0;
            }

            PCRE2_CONFIG_JITTARGET => {
                // SUPPORT_JIT is not defined.
                return PCRE2_ERROR_BADOPTION as c_int;
            }

            PCRE2_CONFIG_LINKSIZE => {
                // CONFIGURED_LINK_SIZE == LINK_SIZE in this configuration.
                *(where_ as *mut u32) = LINK_SIZE as u32;
            }

            PCRE2_CONFIG_MATCHLIMIT => {
                *(where_ as *mut u32) = MATCH_LIMIT as u32;
            }

            PCRE2_CONFIG_NEWLINE => {
                *(where_ as *mut u32) = NEWLINE_DEFAULT as u32;
            }

            PCRE2_CONFIG_NEVER_BACKSLASH_C => {
                // NEVER_BACKSLASH_C is not defined.
                *(where_ as *mut u32) = 0;
            }

            PCRE2_CONFIG_PARENSLIMIT => {
                *(where_ as *mut u32) = PARENS_NEST_LIMIT as u32;
            }

            // This is now obsolete.
            PCRE2_CONFIG_STACKRECURSE => {
                *(where_ as *mut u32) = 0;
            }

            PCRE2_CONFIG_TABLES_LENGTH => {
                *(where_ as *mut u32) = TABLES_LENGTH as u32;
            }

            PCRE2_CONFIG_UNICODE_VERSION => {
                // SUPPORT_UNICODE is defined.
                let v: *const c_char = crate::tables::_pcre2_unicode_version_8.0;
                return (1 + (if where_.is_null() {
                    c_strlen(v)
                } else {
                    _pcre2_strcpy_c8_8(where_ as *mut PCRE2_UCHAR, v)
                })) as c_int;
            }

            PCRE2_CONFIG_UNICODE => {
                // SUPPORT_UNICODE is defined.
                *(where_ as *mut u32) = 1;
            }

            PCRE2_CONFIG_VERSION => {
                let v: *const c_char = VERSION_STRING.as_ptr() as *const c_char;
                return (1 + (if where_.is_null() {
                    c_strlen(v)
                } else {
                    _pcre2_strcpy_c8_8(where_ as *mut PCRE2_UCHAR, v)
                })) as c_int;
            }

            _ => return PCRE2_ERROR_BADOPTION as c_int,
        }

        0
    }
}

/// `strlen` for an 8-bit C string, mirroring `strlen(v)` used by the C source.
#[inline]
unsafe fn c_strlen(mut s: *const c_char) -> usize {
    unsafe {
        let mut n = 0usize;
        while *s != 0 {
            n += 1;
            s = s.add(1);
        }
        n
    }
}
