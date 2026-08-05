use crate::pcre2_internal::*;

// GETCHAR for 8-bit UTF: decode UTF-8 char at ptr without advancing.
#[inline]
unsafe fn getchar(ptr: PCRE2_SPTR) -> u32 {
    let c = *ptr as u32;
    if c < 0xc0 {
        return c;
    }
    getutf8(c, ptr)
}

#[inline]
unsafe fn getutf8(c: u32, eptr: PCRE2_SPTR) -> u32 {
    if (c & 0x20) == 0 {
        ((c & 0x1f) << 6) | (*eptr.add(1) as u32 & 0x3f)
    } else if (c & 0x10) == 0 {
        ((c & 0x0f) << 12) | ((*eptr.add(1) as u32 & 0x3f) << 6) | (*eptr.add(2) as u32 & 0x3f)
    } else if (c & 0x08) == 0 {
        ((c & 0x07) << 18)
            | ((*eptr.add(1) as u32 & 0x3f) << 12)
            | ((*eptr.add(2) as u32 & 0x3f) << 6)
            | (*eptr.add(3) as u32 & 0x3f)
    } else if (c & 0x04) == 0 {
        ((c & 0x03) << 24)
            | ((*eptr.add(1) as u32 & 0x3f) << 18)
            | ((*eptr.add(2) as u32 & 0x3f) << 12)
            | ((*eptr.add(3) as u32 & 0x3f) << 6)
            | (*eptr.add(4) as u32 & 0x3f)
    } else {
        ((c & 0x01) << 30)
            | ((*eptr.add(1) as u32 & 0x3f) << 24)
            | ((*eptr.add(2) as u32 & 0x3f) << 18)
            | ((*eptr.add(3) as u32 & 0x3f) << 12)
            | ((*eptr.add(4) as u32 & 0x3f) << 6)
            | (*eptr.add(5) as u32 & 0x3f)
    }
}

// BACKCHAR: move ptr back over UTF-8 continuation bytes.
#[inline]
unsafe fn backchar(mut ptr: PCRE2_SPTR) -> PCRE2_SPTR {
    while (*ptr & 0xc0) == 0x80 {
        ptr = ptr.sub(1);
    }
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_is_newline_8(
    ptr: PCRE2_SPTR,
    type_: u32,
    endptr: PCRE2_SPTR,
    lenptr: *mut u32,
    utf: BOOL,
) -> BOOL {
    let c: u32 = if utf != 0 { getchar(ptr) } else { *ptr as u32 };

    if type_ == NLTYPE_ANYCRLF {
        match c {
            CHAR_LF => {
                *lenptr = 1;
                TRUE
            }
            CHAR_CR => {
                *lenptr = if ptr < endptr.sub(1) && *ptr.add(1) as u32 == CHAR_LF { 2 } else { 1 };
                TRUE
            }
            _ => FALSE,
        }
    } else {
        // NLTYPE_ANY
        match c {
            CHAR_LF | CHAR_VT | CHAR_FF => {
                *lenptr = 1;
                TRUE
            }
            CHAR_CR => {
                *lenptr = if ptr < endptr.sub(1) && *ptr.add(1) as u32 == CHAR_LF { 2 } else { 1 };
                TRUE
            }
            CHAR_NEL => {
                *lenptr = if utf != 0 { 2 } else { 1 };
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_was_newline_8(
    mut ptr: PCRE2_SPTR,
    type_: u32,
    startptr: PCRE2_SPTR,
    lenptr: *mut u32,
    utf: BOOL,
) -> BOOL {
    ptr = ptr.sub(1);
    let c: u32 = if utf != 0 {
        ptr = backchar(ptr);
        getchar(ptr)
    } else {
        *ptr as u32
    };

    if type_ == NLTYPE_ANYCRLF {
        match c {
            CHAR_LF => {
                *lenptr = if ptr > startptr && *ptr.sub(1) as u32 == CHAR_CR { 2 } else { 1 };
                TRUE
            }
            CHAR_CR => {
                *lenptr = 1;
                TRUE
            }
            _ => FALSE,
        }
    } else {
        match c {
            CHAR_LF => {
                *lenptr = if ptr > startptr && *ptr.sub(1) as u32 == CHAR_CR { 2 } else { 1 };
                TRUE
            }
            CHAR_VT | CHAR_FF | CHAR_CR => {
                *lenptr = 1;
                TRUE
            }
            CHAR_NEL => {
                *lenptr = if utf != 0 { 2 } else { 1 };
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
