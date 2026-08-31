//! Translation of `c_src/src/pcre2_config.c`.
//!
//! Build configuration: `PCRE2_CODE_UNIT_WIDTH == 8` (only `SUPPORT_PCRE2_8`),
//! `SUPPORT_UNICODE`, no `SUPPORT_JIT`, `LINK_SIZE == 2`, `BSR_ANYCRLF` not
//! defined, `NEVER_BACKSLASH_C` not defined. Only the branches compiled under
//! that configuration are translated.

#![allow(non_snake_case)]

use core::ffi::{c_int, c_void};

use crate::internal::*;

/* The version string produced by the C preprocessor. From `pcre2.h`:
     PCRE2_MAJOR       10
     PCRE2_MINOR       48
     PCRE2_PRERELEASE  -DEV
     PCRE2_DATE        2025-10-21

The C code chooses between two forms at runtime:

     const char *v = (XSTRING(Z PCRE2_PRERELEASE)[1] == 0)?
       XSTRING(PCRE2_MAJOR.PCRE2_MINOR PCRE2_DATE) :
       XSTRING(PCRE2_MAJOR.PCRE2_MINOR) XSTRING(PCRE2_PRERELEASE PCRE2_DATE);

`XSTRING(Z PCRE2_PRERELEASE)` stringifies to "Z -DEV"; index [1] is a space,
not NUL, so the second alternative is used:

     "10.48" "-DEV 2025-10-21"  ->  "10.48-DEV 2025-10-21"
*/
static PCRE2_VERSION_STRING: &[u8] = b"10.48-DEV 2025-10-21";

/* The Unicode version string, PRIV(unicode_version). */
static UNICODE_VERSION_STRING: &[u8] = UNICODE_VERSION;

/* `PRIV(strcpy_c8)`: copy a NUL-terminated 8-bit string into `str1`, returning
the number of code units used (excluding the trailing zero). Implemented
locally to mirror `pcre2_string_utils.c`; it is exported from that module. */
#[inline]
unsafe fn strcpy_c8(str1: *mut PCRE2_UCHAR, str2: &[u8]) -> PCRE2_SIZE {
    unsafe {
        let mut t = str1;
        let mut s = str2.as_ptr();
        while *s != 0 {
            *t = *s;
            t = t.add(1);
            s = s.add(1);
        }
        *t = 0;
        t.offset_from(str1) as PCRE2_SIZE
    }
}

/*************************************************
* Return info about what features are configured *
*************************************************/

/* If where is NULL, the length of memory required is returned.

Arguments:
  what             what information is required
  where            where to put the information

Returns:           0 if a numerical value is returned
                   >= 0 if a string value
                   PCRE2_ERROR_BADOPTION if "where" not recognized
                     or JIT target requested when JIT not enabled
*/

pub unsafe fn config(what: u32, where_: *mut c_void) -> c_int {
    unsafe {
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
                | PCRE2_CONFIG_STACKRECURSE /* Obsolete */
                | PCRE2_CONFIG_TABLES_LENGTH
                | PCRE2_CONFIG_UNICODE => {
                    return core::mem::size_of::<u32>() as c_int;
                }

                /* These are handled below */
                PCRE2_CONFIG_JITTARGET
                | PCRE2_CONFIG_UNICODE_VERSION
                | PCRE2_CONFIG_VERSION => {}

                _ => return PCRE2_ERROR_BADOPTION,
            }
        }

        match what {
            PCRE2_CONFIG_BSR => {
                /* BSR_ANYCRLF not defined */
                *(where_ as *mut u32) = PCRE2_BSR_UNICODE;
            }

            PCRE2_CONFIG_COMPILED_WIDTHS => {
                /* This build defines none of SUPPORT_PCRE2_8/16/32 (the CMake
                build does not set them), so the value is zero. */
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
                /* SUPPORT_JIT not defined */
                *(where_ as *mut u32) = 0;
            }

            PCRE2_CONFIG_JITTARGET => {
                /* SUPPORT_JIT not defined */
                return PCRE2_ERROR_BADOPTION;
            }

            PCRE2_CONFIG_LINKSIZE => {
                *(where_ as *mut u32) = CONFIGURED_LINK_SIZE as u32;
            }

            PCRE2_CONFIG_MATCHLIMIT => {
                *(where_ as *mut u32) = MATCH_LIMIT;
            }

            PCRE2_CONFIG_NEWLINE => {
                *(where_ as *mut u32) = NEWLINE_DEFAULT;
            }

            PCRE2_CONFIG_NEVER_BACKSLASH_C => {
                /* NEVER_BACKSLASH_C not defined */
                *(where_ as *mut u32) = 0;
            }

            PCRE2_CONFIG_PARENSLIMIT => {
                *(where_ as *mut u32) = PARENS_NEST_LIMIT;
            }

            /* This is now obsolete. The stack is no longer used via recursion for
            handling backtracking in pcre2_match(). */
            PCRE2_CONFIG_STACKRECURSE => {
                *(where_ as *mut u32) = 0;
            }

            PCRE2_CONFIG_TABLES_LENGTH => {
                *(where_ as *mut u32) = TABLES_LENGTH as u32;
            }

            PCRE2_CONFIG_UNICODE_VERSION => {
                /* SUPPORT_UNICODE defined */
                let v = UNICODE_VERSION_STRING;
                return (1 + if where_.is_null() {
                    strlen_bytes(v)
                } else {
                    strcpy_c8(where_ as *mut PCRE2_UCHAR, v)
                }) as c_int;
            }

            PCRE2_CONFIG_UNICODE => {
                /* SUPPORT_UNICODE defined */
                *(where_ as *mut u32) = 1;
            }

            /* The hackery in setting "v" in the C source is to cope with the case
            when PCRE2_PRERELEASE is an empty string. Here PCRE2_PRERELEASE is
            "-DEV", so the produced string is "10.48-DEV 2025-10-21". */
            PCRE2_CONFIG_VERSION => {
                let v = PCRE2_VERSION_STRING;
                return (1 + if where_.is_null() {
                    strlen_bytes(v)
                } else {
                    strcpy_c8(where_ as *mut PCRE2_UCHAR, v)
                }) as c_int;
            }

            _ => return PCRE2_ERROR_BADOPTION,
        }

        0
    }
}

/// `CONFIGURED_LINK_SIZE` from `pcre2_intmodedep.h` (LINK_SIZE == 2).
const CONFIGURED_LINK_SIZE: c_int = 2;

/// `strlen` of a static byte string terminated by a NUL byte (the trailing NUL
/// in the `b"..."` literal is not present, so count until NUL or slice end).
#[inline]
fn strlen_bytes(s: &[u8]) -> PCRE2_SIZE {
    let mut i = 0;
    while i < s.len() && s[i] != 0 {
        i += 1;
    }
    i
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_config_8(what: u32, where_: *mut c_void) -> c_int {
    unsafe { config(what, where_) }
}
