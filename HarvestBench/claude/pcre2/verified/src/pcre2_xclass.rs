use crate::pcre2_internal::*;

// HSPACE / VSPACE case sets (8-bit + Unicode). Used in PT_SPACE/PT_PXSPACE.
#[inline]
fn is_hspace(c: u32) -> bool {
    matches!(
        c,
        CHAR_HT | CHAR_SPACE | CHAR_NBSP | 0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002 | 0x2003
            | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009 | 0x200a | 0x202f | 0x205f
            | 0x3000
    )
}
#[inline]
fn is_vspace(c: u32) -> bool {
    matches!(c, CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028 | 0x2029)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_xclass_8(
    mut c: u32,
    mut data: PCRE2_SPTR,
    char_lists_end: *const u8,
    mut utf: BOOL,
) -> BOOL {
    let not_negated: BOOL = ((*data & XCL_NOT) == 0) as BOOL;
    let mut type_: u32;
    let mut max_index: u32;
    let mut min_index: u32;
    let mut value: u32;
    let mut next_char: *const u8;

    // 8-bit mode: utf is always TRUE.
    utf = TRUE;

    // Bitmap for code points < 256.
    let d0 = *data;
    data = data.add(1);
    if (d0 & XCL_MAP) != 0 {
        if c < 256 {
            return ((*(data as *const u8).add((c / 8) as usize) & (1u8 << (c & 7))) != 0) as BOOL;
        }
        data = data.add(32); // 32 / sizeof(PCRE2_UCHAR)
    }

    // Unicode property list.
    if *data == XCL_PROP || *data == XCL_NOTPROP {
        let prop = GET_UCD(c);
        loop {
            let isprop = *data == XCL_PROP;
            data = data.add(1);
            let ptype = *data;

            match ptype {
                x if x == PT_LAMP as u8 => {
                    let chartype = prop.chartype as u32;
                    if (chartype == ucp_Lu || chartype == ucp_Ll || chartype == ucp_Lt) == isprop {
                        return not_negated;
                    }
                }
                x if x == PT_GC as u8 => {
                    if ((*data.add(1) as u32) == _pcre2_ucp_gentype_8[prop.chartype as usize])
                        == isprop
                    {
                        return not_negated;
                    }
                }
                x if x == PT_PC as u8 => {
                    if ((*data.add(1) as u32) == prop.chartype as u32) == isprop {
                        return not_negated;
                    }
                }
                x if x == PT_SC as u8 => {
                    if ((*data.add(1) as u32) == prop.script as u32) == isprop {
                        return not_negated;
                    }
                }
                x if x == PT_SCX as u8 => {
                    let d1 = *data.add(1) as u32;
                    let sx = UCD_SCRIPTX_PROP(prop) as usize;
                    let ok = d1 == prop.script as u32
                        || MAPBIT(&_pcre2_ucd_script_sets_8[sx..], d1) != 0;
                    if ok == isprop {
                        return not_negated;
                    }
                }
                x if x == PT_ALNUM as u8 => {
                    let chartype = prop.chartype as usize;
                    if (_pcre2_ucp_gentype_8[chartype] == ucp_L
                        || _pcre2_ucp_gentype_8[chartype] == ucp_N)
                        == isprop
                    {
                        return not_negated;
                    }
                }
                x if x == PT_SPACE as u8 || x == PT_PXSPACE as u8 => {
                    if is_hspace(c) || is_vspace(c) {
                        if isprop {
                            return not_negated;
                        }
                    } else if (_pcre2_ucp_gentype_8[prop.chartype as usize] == ucp_Z) == isprop {
                        return not_negated;
                    }
                }
                x if x == PT_WORD as u8 => {
                    let chartype = prop.chartype as u32;
                    let ct = chartype as usize;
                    if (_pcre2_ucp_gentype_8[ct] == ucp_L
                        || _pcre2_ucp_gentype_8[ct] == ucp_N
                        || chartype == ucp_Mn
                        || chartype == ucp_Pc)
                        == isprop
                    {
                        return not_negated;
                    }
                }
                x if x == PT_UCNC as u8 => {
                    if c < 0xa0 {
                        if (c == CHAR_DOLLAR_SIGN
                            || c == CHAR_COMMERCIAL_AT
                            || c == CHAR_GRAVE_ACCENT)
                            == isprop
                        {
                            return not_negated;
                        }
                    } else if (c < 0xd800 || c > 0xdfff) == isprop {
                        return not_negated;
                    }
                }
                x if x == PT_BIDICL as u8 => {
                    if (UCD_BIDICLASS_PROP(prop) == *data.add(1) as u32) == isprop {
                        return not_negated;
                    }
                }
                x if x == PT_BOOL as u8 => {
                    let bp = UCD_BPROPS_PROP(prop) as usize;
                    let ok = MAPBIT(&_pcre2_ucd_boolprop_sets_8[bp..], *data.add(1) as u32) != 0;
                    if ok == isprop {
                        return not_negated;
                    }
                }
                x if x == PT_PXGRAPH as u8 => {
                    let chartype = prop.chartype as u32;
                    let ct = chartype as usize;
                    if (_pcre2_ucp_gentype_8[ct] != ucp_Z
                        && (_pcre2_ucp_gentype_8[ct] != ucp_C
                            || (chartype == ucp_Cf
                                && c != 0x061c
                                && c != 0x180e
                                && (c < 0x2066 || c > 0x2069))))
                        == isprop
                    {
                        return not_negated;
                    }
                }
                x if x == PT_PXPRINT as u8 => {
                    let chartype = prop.chartype as u32;
                    let ct = chartype as usize;
                    if (chartype != ucp_Zl
                        && chartype != ucp_Zp
                        && (_pcre2_ucp_gentype_8[ct] != ucp_C
                            || (chartype == ucp_Cf && c != 0x061c && (c < 0x2066 || c > 0x2069))))
                        == isprop
                    {
                        return not_negated;
                    }
                }
                x if x == PT_PXPUNCT as u8 => {
                    let chartype = prop.chartype as usize;
                    if (_pcre2_ucp_gentype_8[chartype] == ucp_P
                        || (c < 128 && _pcre2_ucp_gentype_8[chartype] == ucp_S))
                        == isprop
                    {
                        return not_negated;
                    }
                }
                x if x == PT_PXXDIGIT as u8 => {
                    if ((c >= CHAR_0 && c <= CHAR_9)
                        || (c >= CHAR_A && c <= CHAR_F)
                        || (c >= CHAR_a && c <= CHAR_f)
                        || (c >= 0xff10 && c <= 0xff19)
                        || (c >= 0xff21 && c <= 0xff26)
                        || (c >= 0xff41 && c <= 0xff46))
                        == isprop
                    {
                        return not_negated;
                    }
                }
                _ => return FALSE,
            }

            data = data.add(2);
            if !(*data == XCL_PROP || *data == XCL_NOTPROP) {
                break;
            }
        }
    }

    // Large chars / ranges ending with a large char.
    if *data < XCL_LIST as u8 {
        loop {
            let t = *data;
            data = data.add(1);
            if t == XCL_END {
                break;
            }
            let x: u32;
            if utf != 0 {
                let (v, consumed) = GETCHARINC(data);
                x = v;
                data = data.add(consumed);
            } else {
                x = *data as u32;
                data = data.add(1);
            }

            if t == XCL_SINGLE {
                if c <= x {
                    return if c == x { not_negated } else { !not_negated & 1 };
                }
                continue;
            }

            // XCL_RANGE
            let y: u32;
            if utf != 0 {
                let (v, consumed) = GETCHARINC(data);
                y = v;
                data = data.add(consumed);
            } else {
                y = *data as u32;
                data = data.add(1);
            }

            if c <= y {
                return if c >= x { not_negated } else { !not_negated & 1 };
            }
        }
        return !not_negated & 1;
    }

    // 8-bit: type is 16 bits, high/low byte.
    type_ = ((*data as u32) << 8) | *data.add(1) as u32;
    data = data.add(2);

    next_char = char_lists_end.offset(-((GET(data, 0) << 1) as isize));
    type_ &= XCL_TYPE_MASK;

    if c >= XCL_CHAR_LIST_HIGH_16_START {
        max_index = type_ & XCL_ITEM_COUNT_MASK;
        if max_index == XCL_ITEM_COUNT_MASK {
            max_index = *(next_char as *const u16) as u32;
            next_char = next_char.add(2);
        }
        next_char = next_char.add((max_index << 1) as usize);
        type_ >>= XCL_TYPE_BIT_LEN;
    }

    if c < XCL_CHAR_LIST_LOW_32_START {
        max_index = type_ & XCL_ITEM_COUNT_MASK;
        c = ((c << XCL_CHAR_SHIFT) | XCL_CHAR_END) & 0xffff;

        if max_index == XCL_ITEM_COUNT_MASK {
            max_index = *(next_char as *const u16) as u32;
            next_char = next_char.add(2);
        }

        if max_index == 0 || c < *(next_char as *const u16) as u32 {
            return (((type_ & XCL_BEGIN_WITH_RANGE) != 0) as BOOL == not_negated) as BOOL;
        }

        min_index = 0;
        max_index -= 1;
        value = *(next_char as *const u16).add(max_index as usize) as u32;
        if c >= value {
            return ((value == c || (value & XCL_CHAR_END) == 0) as BOOL == not_negated) as BOOL;
        }

        max_index -= 1;

        loop {
            let mid_index = (min_index + max_index) >> 1;
            value = *(next_char as *const u16).add(mid_index as usize) as u32;
            if c < value {
                max_index = mid_index - 1;
            } else if (*(next_char as *const u16).add((mid_index + 1) as usize) as u32) <= c {
                min_index = mid_index + 1;
            } else {
                return ((value == c || (value & XCL_CHAR_END) == 0) as BOOL == not_negated) as BOOL;
            }
        }
    }

    // Skip 16-bit ranges.
    max_index = type_ & XCL_ITEM_COUNT_MASK;
    if max_index == XCL_ITEM_COUNT_MASK {
        max_index = *(next_char as *const u16) as u32;
        next_char = next_char.add(2);
    }
    next_char = next_char.add((max_index << 1) as usize);
    type_ >>= XCL_TYPE_BIT_LEN;

    max_index = type_ & XCL_ITEM_COUNT_MASK;

    c = (c << XCL_CHAR_SHIFT) | XCL_CHAR_END;

    if max_index == XCL_ITEM_COUNT_MASK {
        max_index = *(next_char as *const u32);
        next_char = next_char.add(4);
    }

    if max_index == 0 || c < *(next_char as *const u32) {
        return (((type_ & XCL_BEGIN_WITH_RANGE) != 0) as BOOL == not_negated) as BOOL;
    }

    min_index = 0;
    max_index -= 1;
    value = *(next_char as *const u32).add(max_index as usize);
    if c >= value {
        return ((value == c || (value & XCL_CHAR_END) == 0) as BOOL == not_negated) as BOOL;
    }

    max_index -= 1;

    loop {
        let mid_index = (min_index + max_index) >> 1;
        value = *(next_char as *const u32).add(mid_index as usize);
        if c < value {
            max_index = mid_index - 1;
        } else if *(next_char as *const u32).add((mid_index + 1) as usize) <= c {
            min_index = mid_index + 1;
        } else {
            return ((value == c || (value & XCL_CHAR_END) == 0) as BOOL == not_negated) as BOOL;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_eclass_8(
    c: u32,
    data_start: PCRE2_SPTR,
    data_end: PCRE2_SPTR,
    char_lists_end: *const u8,
    utf: BOOL,
) -> BOOL {
    let mut ptr = data_start;
    let flags = *ptr;
    ptr = ptr.add(1);

    if (flags & ECL_MAP) != 0 {
        if c < 256 {
            return ((*(ptr as *const u8).add((c / 8) as usize) & (1u8 << (c & 7))) != 0) as BOOL;
        }
        ptr = ptr.add(32);
    }

    let mut stack: u32 = 0;
    while ptr < data_end {
        match *ptr {
            x if x == ECL_AND => {
                ptr = ptr.add(1);
                stack = (stack >> 1) & (stack | !1u32);
            }
            x if x == ECL_OR => {
                ptr = ptr.add(1);
                stack = (stack >> 1) | (stack & 1u32);
            }
            x if x == ECL_XOR => {
                ptr = ptr.add(1);
                stack = (stack >> 1) ^ (stack & 1u32);
            }
            x if x == ECL_NOT => {
                ptr = ptr.add(1);
                stack ^= 1u32;
            }
            x if x == ECL_XCLASS => {
                let matched =
                    _pcre2_xclass_8(c, ptr.add(1 + LINK_SIZE), char_lists_end, utf) as u32;
                ptr = ptr.add(GET(ptr, 1) as usize);
                stack = (stack << 1) | matched;
            }
            _ => return FALSE,
        }
    }

    ((stack & 1u32) != 0) as BOOL
}
