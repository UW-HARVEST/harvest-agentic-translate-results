use crate::pcre2_internal::*;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_extuni_8(
    mut c: u32,
    mut eptr: PCRE2_SPTR,
    start_subject: PCRE2_SPTR,
    end_subject: PCRE2_SPTR,
    utf: BOOL,
    xcount: *mut i32,
) -> PCRE2_SPTR {
    let mut was_ep_zwj: BOOL = FALSE;
    let mut lgb = UCD_GRAPHBREAK(c) as i32;

    while eptr < end_subject {
        let mut len: usize = 1;
        if utf == 0 {
            c = *eptr as u32;
        } else {
            let (cc, extra) = GETCHARLEN(eptr);
            c = cc;
            len += extra as usize;
        }
        let rgb = UCD_GRAPHBREAK(c) as i32;
        if (_pcre2_ucp_gbtable_8[lgb as usize] & (1u32 << rgb)) == 0 {
            break;
        }

        if lgb == ucp_gbZWJ as i32
            && rgb == ucp_gbExtended_Pictographic as i32
            && was_ep_zwj == FALSE
        {
            break;
        }

        if lgb == ucp_gbRegional_Indicator as i32 && rgb == ucp_gbRegional_Indicator as i32 {
            let mut ricount = 0;
            let mut bptr = eptr.sub(1);
            if utf != 0 {
                bptr = BACKCHAR(bptr);
            }

            while bptr > start_subject {
                bptr = bptr.sub(1);
                if utf != 0 {
                    bptr = BACKCHAR(bptr);
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
                break;
            }
        }

        was_ep_zwj = (lgb == ucp_gbExtended_Pictographic as i32 && rgb == ucp_gbZWJ as i32) as BOOL;

        if rgb != ucp_gbExtend as i32 || lgb != ucp_gbExtended_Pictographic as i32 {
            lgb = rgb;
        }

        eptr = eptr.add(len);
        if !xcount.is_null() {
            *xcount += 1;
        }
    }

    eptr
}
