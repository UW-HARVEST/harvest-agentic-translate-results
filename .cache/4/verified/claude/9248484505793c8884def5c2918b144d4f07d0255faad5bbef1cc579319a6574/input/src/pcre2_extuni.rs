// Translated from c_src/src/pcre2_extuni.c
use crate::internal::*;

/* This module contains an internal function that is used to match a Unicode
extended grapheme sequence. It is used by both pcre2_match() and
pcre2_dfa_match(). However, it is called only when Unicode support is being
compiled. Nevertheless, we provide a dummy function when there is no Unicode
support, because some compilers do not like functionless source files. */

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
pub unsafe extern "C" fn _pcre2_extuni_8(
    mut c: u32,
    mut eptr: PCRE2_SPTR,
    start_subject: PCRE2_SPTR,
    end_subject: PCRE2_SPTR,
    utf: BOOL,
    xcount: *mut c_int,
) -> PCRE2_SPTR {
    let mut was_ep_ZWJ: BOOL = FALSE;
    let mut lgb: c_int = UCD_GRAPHBREAK(c) as c_int;

    'outer: while eptr < end_subject {
        let rgb: c_int;
        let mut len: c_int = 1;
        if utf == 0 {
            c = *eptr as u32;
        } else {
            GETCHARLEN!(c, eptr, len);
        }
        rgb = UCD_GRAPHBREAK(c) as c_int;
        if (*_pcre2_ucp_gbtable_8.as_ptr().add(lgb as usize) & (1u32 << rgb)) == 0 {
            break;
        }

        /* ZWJ followed by Extended Pictographic is allowed only if the ZWJ was
        preceded by Extended Pictographic. */

        if lgb == ucp_gbZWJ as c_int
            && rgb == ucp_gbExtended_Pictographic as c_int
            && was_ep_ZWJ == 0
        {
            break;
        }

        /* Not breaking between Regional Indicators is allowed only if there
        are an even number of preceding RIs. */

        if lgb == ucp_gbRegional_Indicator as c_int && rgb == ucp_gbRegional_Indicator as c_int {
            let mut ricount: c_int = 0;
            let mut bptr: PCRE2_SPTR = eptr.sub(1);
            if utf != 0 {
                BACKCHAR!(bptr);
            }

            /* bptr is pointing to the left-hand character */

            while bptr > start_subject {
                bptr = bptr.sub(1);
                if utf != 0 {
                    BACKCHAR!(bptr);
                    GETCHAR!(c, bptr);
                } else {
                    c = *bptr as u32;
                }
                if UCD_GRAPHBREAK(c) as c_int != ucp_gbRegional_Indicator as c_int {
                    break;
                }
                ricount += 1;
            }
            if (ricount & 1) != 0 {
                break 'outer;
            } /* Grapheme break required */
        }

        /* Set a flag when ZWJ follows Extended Pictographic (with optional Extend in
        between; see next statement). */

        was_ep_ZWJ = if lgb == ucp_gbExtended_Pictographic as c_int && rgb == ucp_gbZWJ as c_int {
            TRUE
        } else {
            FALSE
        };

        /* If Extend follows Extended_Pictographic, do not update lgb; this allows
        any number of them before a following ZWJ. */

        if rgb != ucp_gbExtend as c_int || lgb != ucp_gbExtended_Pictographic as c_int {
            lgb = rgb;
        }

        eptr = eptr.offset(len as isize);
        if !xcount.is_null() {
            *xcount += 1;
        }
    }

    eptr
}

/* End of pcre2_extuni.c */
