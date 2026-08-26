// Translated from pcre2_newline.c
use crate::internal::*;

/*************************************************
*      Check for newline at given position       *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_is_newline_8(
    ptr: PCRE2_SPTR,
    type_: u32,
    endptr: PCRE2_SPTR,
    lenptr: *mut u32,
    utf: BOOL,
) -> BOOL {
    let c: u32;

    if utf != FALSE {
        /* GETCHAR(c, ptr) */
        let mut cc = *ptr as u32;
        if cc >= 0xc0 {
            cc = getutf8(cc, ptr);
        }
        c = cc;
    } else {
        c = *ptr as u32;
    }

    if type_ == NLTYPE_ANYCRLF {
        match c {
            CHAR_LF => {
                *lenptr = 1;
                TRUE
            }
            CHAR_CR => {
                *lenptr = if ptr < endptr.sub(1) && *ptr.add(1) as u32 == CHAR_LF {
                    2
                } else {
                    1
                };
                TRUE
            }
            _ => FALSE,
        }
    } else {
        /* NLTYPE_ANY */
        match c {
            CHAR_LF | CHAR_VT | CHAR_FF => {
                *lenptr = 1;
                TRUE
            }
            CHAR_CR => {
                *lenptr = if ptr < endptr.sub(1) && *ptr.add(1) as u32 == CHAR_LF {
                    2
                } else {
                    1
                };
                TRUE
            }
            CHAR_NEL => {
                *lenptr = if utf != FALSE { 2 } else { 1 };
                TRUE
            }
            0x2028 | 0x2029 => {
                *lenptr = 3;
                TRUE
            }
            _ => FALSE,
        }
    }
}

/*************************************************
*     Check for newline at previous position     *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_was_newline_8(
    ptr: PCRE2_SPTR,
    type_: u32,
    startptr: PCRE2_SPTR,
    lenptr: *mut u32,
    utf: BOOL,
) -> BOOL {
    let c: u32;
    let mut ptr = ptr.sub(1);

    if utf != FALSE {
        /* BACKCHAR(ptr) */
        while (*ptr & 0xc0) == 0x80 {
            ptr = ptr.sub(1);
        }
        /* GETCHAR(c, ptr) */
        let mut cc = *ptr as u32;
        if cc >= 0xc0 {
            cc = getutf8(cc, ptr);
        }
        c = cc;
    } else {
        c = *ptr as u32;
    }

    if type_ == NLTYPE_ANYCRLF {
        match c {
            CHAR_LF => {
                *lenptr = if ptr > startptr && *ptr.offset(-1) as u32 == CHAR_CR {
                    2
                } else {
                    1
                };
                TRUE
            }
            CHAR_CR => {
                *lenptr = 1;
                TRUE
            }
            _ => FALSE,
        }
    } else {
        /* NLTYPE_ANY */
        match c {
            CHAR_LF => {
                *lenptr = if ptr > startptr && *ptr.offset(-1) as u32 == CHAR_CR {
                    2
                } else {
                    1
                };
                TRUE
            }
            CHAR_VT | CHAR_FF | CHAR_CR => {
                *lenptr = 1;
                TRUE
            }
            CHAR_NEL => {
                *lenptr = if utf != FALSE { 2 } else { 1 };
                TRUE
            }
            0x2028 | 0x2029 => {
                *lenptr = 3;
                TRUE
            }
            _ => FALSE,
        }
    }
}
