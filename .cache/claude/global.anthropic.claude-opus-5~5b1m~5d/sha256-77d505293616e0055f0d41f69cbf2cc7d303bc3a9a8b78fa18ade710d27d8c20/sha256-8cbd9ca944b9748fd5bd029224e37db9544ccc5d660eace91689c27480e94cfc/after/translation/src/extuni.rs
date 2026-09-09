//! Translated from pcre2_extuni.c.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use core::ffi::{c_char, c_void};

/*************************************************
*      Match an extended grapheme sequence       *
*************************************************/

/* NOTE: The logic contained in this function is replicated in three special-
purpose functions in the pcre2_jit_compile.c module. If the logic below is
changed, they must be kept in step so that the interpreter and the JIT have the
same behaviour.

Arguments:
  c              the first character
  eptr           pointer to next character
  start_subject  pointer to start of subject
  end_subject    pointer to end of subject
  utf            TRUE if in UTF mode
  xcount         pointer to count of additional characters,
                   or NULL if count not needed

Returns:         pointer after the end of the sequence
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_extuni_8(c: u32, eptr: PCRE2_SPTR, start_subject: PCRE2_SPTR, end_subject: PCRE2_SPTR, utf: BOOL, xcount: *mut i32) -> PCRE2_SPTR {
    let mut c = c;
    let mut eptr = eptr;

    let mut was_ep_ZWJ: BOOL = FALSE;
    let mut lgb: i32 = UCD_GRAPHBREAK!(c) as i32;

    while eptr < end_subject {
        let rgb: i32;
        let mut len: i32 = 1;
        if utf == 0 {
            c = *eptr as u32;
        } else {
            GETCHARLEN!(c, eptr, len);
        }
        rgb = UCD_GRAPHBREAK!(c) as i32;
        if (*crate::tables::_pcre2_ucp_gbtable_8.as_ptr().add(lgb as usize) & (1u32 << rgb)) == 0 {
            break;
        }

        /* ZWJ followed by Extended Pictographic is allowed only if the ZWJ was
        preceded by Extended Pictographic. */

        if lgb == ucp_gbZWJ as i32
            && rgb == ucp_gbExtended_Pictographic as i32
            && was_ep_ZWJ == 0
        {
            break;
        }

        /* Not breaking between Regional Indicators is allowed only if there
        are an even number of preceding RIs. */

        if lgb == ucp_gbRegional_Indicator as i32 && rgb == ucp_gbRegional_Indicator as i32 {
            let mut ricount: i32 = 0;
            let mut bptr: PCRE2_SPTR = eptr.wrapping_sub(1);
            if utf != 0 {
                BACKCHAR!(bptr);
            }

            /* bptr is pointing to the left-hand character */

            while bptr > start_subject {
                bptr = bptr.wrapping_sub(1);
                if utf != 0 {
                    BACKCHAR!(bptr);
                    GETCHAR!(c, bptr);
                } else {
                    c = *bptr as u32;
                }
                if UCD_GRAPHBREAK!(c) as i32 != ucp_gbRegional_Indicator as i32 {
                    break;
                }
                ricount += 1;
            }
            if (ricount & 1) != 0 {
                break; /* Grapheme break required */
            }
        }

        /* Set a flag when ZWJ follows Extended Pictographic (with optional Extend in
        between; see next statement). */

        was_ep_ZWJ =
            (lgb == ucp_gbExtended_Pictographic as i32 && rgb == ucp_gbZWJ as i32) as BOOL;

        /* If Extend follows Extended_Pictographic, do not update lgb; this allows
        any number of them before a following ZWJ. */

        if rgb != ucp_gbExtend as i32 || lgb != ucp_gbExtended_Pictographic as i32 {
            lgb = rgb;
        }

        eptr = eptr.add(len as usize);
        if xcount != core::ptr::null_mut() {
            *xcount += 1;
        }
    }

    eptr
}

