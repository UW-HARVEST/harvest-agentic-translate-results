//! Translation of `pcre2_extuni.c` (8-bit, `SUPPORT_UNICODE` on).

use crate::internal::*;
use crate::tables;

/// `PRIV(extuni)` — match an extended grapheme sequence.
///
/// Returns a pointer after the end of the sequence.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_extuni_8(
    c: u32,
    eptr: PCRE2_SPTR,
    start_subject: PCRE2_SPTR,
    end_subject: PCRE2_SPTR,
    utf: BOOL,
    xcount: *mut i32,
) -> PCRE2_SPTR {
    unsafe {
        let mut c = c;
        let mut eptr = eptr;
        let mut was_ep_zwj: BOOL = FALSE;
        let mut lgb = UCD_GRAPHBREAK(c);

        while eptr < end_subject {
            let rgb;
            let mut len: u32 = 1;
            if utf == 0 {
                c = *eptr as u32;
            } else {
                c = GETCHARLEN(eptr, &mut len);
            }
            rgb = UCD_GRAPHBREAK(c);
            if (tables::_pcre2_ucp_gbtable_8[lgb as usize] & (1u32 << rgb)) == 0 {
                break;
            }

            // ZWJ followed by Extended Pictographic is allowed only if the ZWJ
            // was preceded by Extended Pictographic.
            if lgb == ucp_gbZWJ && rgb == ucp_gbExtended_Pictographic && was_ep_zwj == 0 {
                break;
            }

            // Not breaking between Regional Indicators is allowed only if there
            // are an even number of preceding RIs.
            if lgb == ucp_gbRegional_Indicator && rgb == ucp_gbRegional_Indicator {
                let mut ricount = 0;
                let mut bptr = eptr.offset(-1);
                if utf != 0 {
                    BACKCHAR(&mut bptr);
                }

                // bptr is pointing to the left-hand character
                while bptr > start_subject {
                    bptr = bptr.offset(-1);
                    if utf != 0 {
                        BACKCHAR(&mut bptr);
                        c = GETCHAR(bptr);
                    } else {
                        c = *bptr as u32;
                    }
                    if UCD_GRAPHBREAK(c) != ucp_gbRegional_Indicator {
                        break;
                    }
                    ricount += 1;
                }
                if (ricount & 1) != 0 {
                    break; // Grapheme break required
                }
            }

            // Set a flag when ZWJ follows Extended Pictographic (with optional
            // Extend in between; see next statement).
            was_ep_zwj = if lgb == ucp_gbExtended_Pictographic && rgb == ucp_gbZWJ {
                TRUE
            } else {
                FALSE
            };

            // If Extend follows Extended_Pictographic, do not update lgb; this
            // allows any number of them before a following ZWJ.
            if rgb != ucp_gbExtend || lgb != ucp_gbExtended_Pictographic {
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
