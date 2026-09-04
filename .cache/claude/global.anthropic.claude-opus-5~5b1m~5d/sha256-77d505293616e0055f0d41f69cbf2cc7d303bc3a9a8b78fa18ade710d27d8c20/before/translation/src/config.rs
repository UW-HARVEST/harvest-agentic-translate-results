//! Translated from pcre2_config.c.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use core::ffi::{c_char, c_void};

use crate::string_utils::_pcre2_strcpy_c8_8;

/* From pcre2_intmodedep.h: with LINK_SIZE == 2, CONFIGURED_LINK_SIZE is 2. */
const CONFIGURED_LINK_SIZE: u32 = 2;

/* These macros are the standard way of turning unquoted text into C strings.
They allow macros like PCRE2_MAJOR to be defined without quotes, which is
convenient for user programs that want to test their values.

  #define STRING(a)  # a
  #define XSTRING(s) STRING(s)

The stringified forms that are used below are precomputed here, with
PCRE2_MAJOR = 10, PCRE2_MINOR = 48, PCRE2_PRERELEASE = -DEV and
PCRE2_DATE = 2025-10-21. */

/* XSTRING(Z PCRE2_PRERELEASE) */
static XSTRING_Z_PCRE2_PRERELEASE: [u8; 7] = *b"Z -DEV\0";

/* XSTRING(PCRE2_MAJOR.PCRE2_MINOR PCRE2_DATE) */
static XSTRING_MAJOR_MINOR_DATE: [u8; 17] = *b"10.48 2025-10-21\0";

/* XSTRING(PCRE2_MAJOR.PCRE2_MINOR) XSTRING(PCRE2_PRERELEASE PCRE2_DATE) */
static XSTRING_MAJOR_MINOR_PRERELEASE_DATE: [u8; 21] = *b"10.48-DEV 2025-10-21\0";

unsafe fn strlen(s: *const c_char) -> usize {
    let mut p = s;
    while *p != 0 {
        p = p.add(1);
    }
    p as usize - s as usize
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_config_8(what: u32, where_: *mut c_void) -> i32 {
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
                return core::mem::size_of::<u32>() as i32;
            }

            /* These are handled below */
            PCRE2_CONFIG_JITTARGET | PCRE2_CONFIG_UNICODE_VERSION | PCRE2_CONFIG_VERSION => {}

            _ => {
                return PCRE2_ERROR_BADOPTION;
            }
        }
    }

    match what {
        PCRE2_CONFIG_BSR => {
            /* BSR_ANYCRLF is not defined */
            *(where_ as *mut u32) = PCRE2_BSR_UNICODE;
        }

        PCRE2_CONFIG_COMPILED_WIDTHS => {
            /* SUPPORT_PCRE2_8, SUPPORT_PCRE2_16 and SUPPORT_PCRE2_32 are not defined */
            *(where_ as *mut u32) = 0;
        }

        PCRE2_CONFIG_DEPTHLIMIT => {
            *(where_ as *mut u32) = MATCH_LIMIT_DEPTH;
        }

        PCRE2_CONFIG_EFFECTIVE_LINKSIZE => {
            *(where_ as *mut u32) =
                (LINK_SIZE as u32) * (core::mem::size_of::<PCRE2_UCHAR>() as u32);
        }

        PCRE2_CONFIG_HEAPLIMIT => {
            *(where_ as *mut u32) = HEAP_LIMIT;
        }

        PCRE2_CONFIG_JIT => {
            /* SUPPORT_JIT is not defined */
            *(where_ as *mut u32) = 0;
        }

        PCRE2_CONFIG_JITTARGET => {
            /* SUPPORT_JIT is not defined */
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
            /* NEVER_BACKSLASH_C is not defined */
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
            /* SUPPORT_UNICODE is defined */
            let v: *const c_char = crate::ucd::_pcre2_unicode_version_8.0;
            return (1 + (if where_.is_null() {
                strlen(v)
            } else {
                _pcre2_strcpy_c8_8(where_ as *mut PCRE2_UCHAR, v)
            })) as i32;
        }

        PCRE2_CONFIG_UNICODE => {
            /* SUPPORT_UNICODE is defined */
            *(where_ as *mut u32) = 1;
        }

        /* The hackery in setting "v" below is to cope with the case when
        PCRE2_PRERELEASE is set to an empty string (which it is for real releases).
        If the second alternative is used in this case, it does not leave a space
        before the date. On the other hand, if all four macros are put into a single
        XSTRING when PCRE2_PRERELEASE is not empty, an unwanted space is inserted.
        There are problems using an "obvious" approach like this:

           XSTRING(PCRE2_MAJOR) "." XSTRING(PCRE2_MINOR)
           XSTRING(PCRE2_PRERELEASE) " " XSTRING(PCRE2_DATE)

        because, when PCRE2_PRERELEASE is empty, this leads to an attempted expansion
        of STRING(). The C standard states: "If (before argument substitution) any
        argument consists of no preprocessing tokens, the behavior is undefined." It
        turns out the gcc treats this case as a single empty string - which is what
        we really want - but Visual C grumbles about the lack of an argument for the
        macro. Unfortunately, both are within their rights. As there seems to be no
        way to test for a macro's value being empty at compile time, we have to
        resort to a runtime test. */
        PCRE2_CONFIG_VERSION => {
            let v: *const c_char = if XSTRING_Z_PCRE2_PRERELEASE[1] == 0 {
                XSTRING_MAJOR_MINOR_DATE.as_ptr() as *const c_char
            } else {
                XSTRING_MAJOR_MINOR_PRERELEASE_DATE.as_ptr() as *const c_char
            };
            return (1 + (if where_.is_null() {
                strlen(v)
            } else {
                _pcre2_strcpy_c8_8(where_ as *mut PCRE2_UCHAR, v)
            })) as i32;
        }

        _ => {
            return PCRE2_ERROR_BADOPTION;
        }
    }

    0
}

/* End of pcre2_config.c */
