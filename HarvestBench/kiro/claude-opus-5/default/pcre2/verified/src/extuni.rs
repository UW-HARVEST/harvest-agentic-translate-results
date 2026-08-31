//! Translation of `c_src/src/pcre2_extuni.c`.
//!
//! Internal function used to match a Unicode extended grapheme sequence. It is
//! used by both `pcre2_match()` and `pcre2_dfa_match()`. `SUPPORT_UNICODE` is
//! defined, so the real implementation (not the dummy) is translated.

#![allow(non_snake_case, non_upper_case_globals)]

use core::ffi::c_int;

use crate::internal::*;
use crate::ucp::*;

/// `PRIV(extuni)`
///
/// Match an extended grapheme sequence.
///
/// NOTE: The logic contained in this function is replicated in three special-
/// purpose functions in the pcre2_jit_compile.c module (not translated here).
///
/// Arguments:
/// * `c`             - the first character
/// * `eptr`          - pointer to next character
/// * `start_subject` - pointer to start of subject
/// * `end_subject`   - pointer to end of subject
/// * `utf`           - TRUE if in UTF mode
/// * `xcount`        - pointer to count of additional characters, or NULL if
///                     count not needed
///
/// Returns: pointer after the end of the sequence.
pub unsafe fn extuni(
    mut c: u32,
    mut eptr: PCRE2_SPTR,
    start_subject: PCRE2_SPTR,
    end_subject: PCRE2_SPTR,
    utf: BOOL,
    xcount: *mut c_int,
) -> PCRE2_SPTR {
    unsafe {
        let mut was_ep_ZWJ: BOOL = FALSE;
        let mut lgb: c_int = ucd_graphbreak(c) as c_int;

        while eptr < end_subject {
            let rgb: c_int;
            let mut len: u32 = 1;
            if utf == FALSE {
                c = *eptr as u32;
            } else {
                let (ch, extra) = getcharlen(eptr);
                c = ch;
                len += extra;
            }
            rgb = ucd_graphbreak(c) as c_int;
            if (UCP_GBTABLE[lgb as usize] & (1u32 << rgb)) == 0 {
                break;
            }

            /* ZWJ followed by Extended Pictographic is allowed only if the ZWJ was
            preceded by Extended Pictographic. */

            if lgb == ucp_gbZWJ as c_int
                && rgb == ucp_gbExtended_Pictographic as c_int
                && was_ep_ZWJ == FALSE
            {
                break;
            }

            /* Not breaking between Regional Indicators is allowed only if there
            are an even number of preceding RIs. */

            if lgb == ucp_gbRegional_Indicator as c_int
                && rgb == ucp_gbRegional_Indicator as c_int
            {
                let mut ricount = 0;
                let mut bptr: PCRE2_SPTR = eptr.sub(1);
                if utf != FALSE {
                    backchar(&mut bptr);
                }

                /* bptr is pointing to the left-hand character */

                while bptr > start_subject {
                    bptr = bptr.sub(1);
                    if utf != FALSE {
                        backchar(&mut bptr);
                        c = getchar_(bptr);
                    } else {
                        c = *bptr as u32;
                    }
                    if ucd_graphbreak(c) != ucp_gbRegional_Indicator {
                        break;
                    }
                    ricount += 1;
                }
                if (ricount & 1) != 0 {
                    break; /* Grapheme break required */
                }
            }

            /* Set a flag when ZWJ follows Extended Pictographic (with optional Extend
            in between; see next statement). */

            was_ep_ZWJ = if lgb == ucp_gbExtended_Pictographic as c_int
                && rgb == ucp_gbZWJ as c_int
            {
                TRUE
            } else {
                FALSE
            };

            /* If Extend follows Extended_Pictographic, do not update lgb; this allows
            any number of them before a following ZWJ. */

            if rgb != ucp_gbExtend as c_int || lgb != ucp_gbExtended_Pictographic as c_int {
                lgb = rgb;
            }

            eptr = eptr.add(len as usize);
            if !xcount.is_null() {
                *xcount += 1;
            }
        }

        eptr
    }
}

/// Exported as `_pcre2_extuni_8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_extuni_8(
    c: u32,
    eptr: PCRE2_SPTR,
    start_subject: PCRE2_SPTR,
    end_subject: PCRE2_SPTR,
    utf: BOOL,
    xcount: *mut c_int,
) -> PCRE2_SPTR {
    unsafe { extuni(c, eptr, start_subject, end_subject, utf, xcount) }
}
