// Translated from c_src/src/pcre2_config.c
use crate::internal::*;

/* CONFIGURED_LINK_SIZE comes from pcre2_intmodedep.h; LINK_SIZE == 2 here. */

const CONFIGURED_LINK_SIZE: usize = 2;

/* In C, STRING(a)/XSTRING(s) are the standard way of turning unquoted text into
C strings. They allow macros like PCRE2_MAJOR to be defined without quotes,
which is convenient for user programs that want to test their values. The
preprocessor cannot be reproduced in Rust, so the expansions of the XSTRING
uses below are pre-computed for this build:

  PCRE2_MAJOR      10
  PCRE2_MINOR      48
  PCRE2_PRERELEASE -DEV
  PCRE2_DATE       2025-10-21

  XSTRING(Z PCRE2_PRERELEASE)                      -> "Z -DEV"
  XSTRING(PCRE2_MAJOR.PCRE2_MINOR PCRE2_DATE)      -> "10.48 2025-10-21"
  XSTRING(PCRE2_MAJOR.PCRE2_MINOR)
    XSTRING(PCRE2_PRERELEASE PCRE2_DATE)           -> "10.48-DEV 2025-10-21"
*/

static XSTRING_Z_PRERELEASE: [u8; 7] = *b"Z -DEV\0";
static XSTRING_VERSION_NO_PRERELEASE: [u8; 17] = *b"10.48 2025-10-21\0";
static XSTRING_VERSION_WITH_PRERELEASE: [u8; 21] = *b"10.48-DEV 2025-10-21\0";

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
            | PCRE2_CONFIG_STACKRECURSE /* Obsolete */
            | PCRE2_CONFIG_TABLES_LENGTH
            | PCRE2_CONFIG_UNICODE => return size_of::<u32>() as c_int,

            /* These are handled below */

            PCRE2_CONFIG_JITTARGET
            | PCRE2_CONFIG_UNICODE_VERSION
            | PCRE2_CONFIG_VERSION => {}

            _ => return PCRE2_ERROR_BADOPTION,
        }
    }

    match what {
        PCRE2_CONFIG_BSR => {
            *(where_ as *mut u32) = PCRE2_BSR_UNICODE;
        }

        PCRE2_CONFIG_COMPILED_WIDTHS => {
            /* SUPPORT_PCRE2_8, SUPPORT_PCRE2_16 and SUPPORT_PCRE2_32 are all
            undefined in this build, so the sum of the enabled bits is 0. */
            *(where_ as *mut u32) = 0;
        }

        PCRE2_CONFIG_DEPTHLIMIT => {
            *(where_ as *mut u32) = MATCH_LIMIT_DEPTH;
        }

        PCRE2_CONFIG_EFFECTIVE_LINKSIZE => {
            *(where_ as *mut u32) = (LINK_SIZE * size_of::<PCRE2_UCHAR>()) as u32;
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
            *(where_ as *mut u32) = CONFIGURED_LINK_SIZE as u32;
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

        /* This is now obsolete. The stack is no longer used via recursion for
        handling backtracking in pcre2_match(). */
        PCRE2_CONFIG_STACKRECURSE => {
            *(where_ as *mut u32) = 0;
        }

        PCRE2_CONFIG_TABLES_LENGTH => {
            *(where_ as *mut u32) = TABLES_LENGTH as u32;
        }

        PCRE2_CONFIG_UNICODE_VERSION => {
            let v: *const c_char = _pcre2_unicode_version_8;
            return (1 + (if where_.is_null() {
                strlen(v)
            } else {
                _pcre2_strcpy_c8_8(where_ as *mut PCRE2_UCHAR, v)
            })) as c_int;
        }

        PCRE2_CONFIG_UNICODE => {
            *(where_ as *mut u32) = 1;
        }

        /* The hackery in setting "v" below is to cope with the case when
        PCRE2_PRERELEASE is set to an empty string (which it is for real
        releases). If the second alternative is used in this case, it does not
        leave a space before the date. On the other hand, if all four macros are
        put into a single XSTRING when PCRE2_PRERELEASE is not empty, an unwanted
        space is inserted. As there seems to be no way to test for a macro's
        value being empty at compile time, C has to resort to a runtime test,
        which is reproduced here. */
        PCRE2_CONFIG_VERSION => {
            let v: *const c_char = if XSTRING_Z_PRERELEASE[1] == 0 {
                XSTRING_VERSION_NO_PRERELEASE.as_ptr() as *const c_char
            } else {
                XSTRING_VERSION_WITH_PRERELEASE.as_ptr() as *const c_char
            };
            return (1 + (if where_.is_null() {
                strlen(v)
            } else {
                _pcre2_strcpy_c8_8(where_ as *mut PCRE2_UCHAR, v)
            })) as c_int;
        }

        _ => return PCRE2_ERROR_BADOPTION,
    }

    return 0;
}

/* End of pcre2_config.c */
