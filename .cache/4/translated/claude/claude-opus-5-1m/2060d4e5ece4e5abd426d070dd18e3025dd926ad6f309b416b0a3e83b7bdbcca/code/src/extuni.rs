// Translated from pcre2_extuni.c
use crate::internal::*;
use crate::tables::*;
use crate::ucp::*;
use core::ffi::c_int;

/*************************************************
*      Match an extended grapheme sequence       *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_extuni_8(
    c: u32,
    eptr: PCRE2_SPTR,
    start_subject: PCRE2_SPTR,
    end_subject: PCRE2_SPTR,
    utf: BOOL,
    xcount: *mut c_int,
) -> PCRE2_SPTR {
    let mut c = c;
    let mut eptr = eptr;
    let mut was_ep_ZWJ: BOOL = FALSE;
    let mut lgb: c_int = UCD_GRAPHBREAK(c) as c_int;

    while eptr < end_subject {
        let rgb: c_int;
        let mut len: c_int = 1;
        if utf == FALSE {
            c = *eptr as u32;
        } else {
            /* GETCHARLEN(c, eptr, len) */
            c = *eptr as u32;
            if c >= 0xc0 {
                len += utf8_extra(c) as c_int;
                c = getutf8(c, eptr);
            }
        }
        rgb = UCD_GRAPHBREAK(c) as c_int;
        if (_pcre2_ucp_gbtable_8[lgb as usize] & (1u32 << rgb)) == 0 {
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

        if lgb == ucp_gbRegional_Indicator as c_int && rgb == ucp_gbRegional_Indicator as c_int {
            let mut ricount: c_int = 0;
            let mut bptr: PCRE2_SPTR = eptr.sub(1);
            if utf != FALSE {
                /* BACKCHAR(bptr) */
                while (*bptr & 0xc0) == 0x80 {
                    bptr = bptr.sub(1);
                }
            }

            /* bptr is pointing to the left-hand character */

            while bptr > start_subject {
                bptr = bptr.sub(1);
                if utf != FALSE {
                    /* BACKCHAR(bptr) */
                    while (*bptr & 0xc0) == 0x80 {
                        bptr = bptr.sub(1);
                    }
                    /* GETCHAR(c, bptr) */
                    c = *bptr as u32;
                    if c >= 0xc0 {
                        c = getutf8(c, bptr);
                    }
                } else {
                    c = *bptr as u32;
                }
                if UCD_GRAPHBREAK(c) != ucp_gbRegional_Indicator {
                    break;
                }
                ricount += 1;
            }
            if (ricount & 1) != 0 {
                break; /* Grapheme break required */
            }
        }

        /* Set a flag when ZWJ follows Extended Pictographic. */

        was_ep_ZWJ = ((lgb == ucp_gbExtended_Pictographic as c_int
            && rgb == ucp_gbZWJ as c_int) as BOOL);

        /* If Extend follows Extended_Pictographic, do not update lgb. */

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
