// Translated from c_src/src/pcre2_xclass.c
use crate::internal::*;

/* This module contains two internal functions that are used to match
OP_XCLASS and OP_ECLASS. It is used by pcre2_auto_possessify() and by both
pcre2_match() and pcre2_dfa_match(). */

/*************************************************
*       Match character against an XCLASS        *
*************************************************/

/* This function is called to match a character against an extended class that
might contain codepoints above 255 and/or Unicode properties.

Arguments:
  c           the character
  data        points to the flag code unit of the XCLASS data
  utf         TRUE if in UTF mode

Returns:      TRUE if character matches, else FALSE
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_xclass_8(
    mut c: u32,
    data: PCRE2_SPTR,
    char_lists_end: *const u8,
    mut utf: BOOL,
) -> BOOL {
    /* Update PRIV(update_classbits) when this function is changed. */
    let mut data: PCRE2_SPTR = data;
    let not_negated: BOOL = ((*data as u32 & XCL_NOT) == 0) as BOOL;
    let mut type_: u32;
    let mut max_index: u32;
    let mut min_index: u32;
    let mut value: u32;
    let mut next_char: *const u8;

    /* In 8 bit mode, this must always be TRUE. Help the compiler to know that. */
    utf = TRUE;

    /* Code points < 256 are matched against a bitmap, if one is present. */

    {
        let flag_unit = *data;
        data = data.add(1);
        if (flag_unit as u32 & XCL_MAP) != 0 {
            if c < 256 {
                return ((*data.add((c / 8) as usize) as u32 & (1u32 << (c & 7))) != 0) as BOOL;
            }
            /* Skip bitmap. */
            data = data.add(32);
        }
    }

    /* Match against the list of Unicode properties. We won't ever
    encounter XCL_PROP or XCL_NOTPROP when UTF support is not compiled. */

    if *data as u32 == XCL_PROP || *data as u32 == XCL_NOTPROP {
        /* The UCD record is the same for all properties. */
        let prop: *const ucd_record = GET_UCD(c);

        loop {
            let isprop: BOOL = ({
                let t = *data;
                data = data.add(1);
                t as u32
            } == XCL_PROP) as BOOL;

            match *data as u32 {
                PT_LAMP => {
                    let chartype: c_int = (*prop).chartype as c_int;
                    if ((chartype == ucp_Lu as c_int
                        || chartype == ucp_Ll as c_int
                        || chartype == ucp_Lt as c_int) as BOOL)
                        == isprop
                    {
                        return not_negated;
                    }
                }

                PT_GC => {
                    if ((*data.add(1) as u32
                        == *_pcre2_ucp_gentype_8
                            .as_ptr()
                            .add((*prop).chartype as usize)) as BOOL)
                        == isprop
                    {
                        return not_negated;
                    }
                }

                PT_PC => {
                    if ((*data.add(1) == (*prop).chartype) as BOOL) == isprop {
                        return not_negated;
                    }
                }

                PT_SC => {
                    if ((*data.add(1) == (*prop).script) as BOOL) == isprop {
                        return not_negated;
                    }
                }

                PT_SCX => {
                    let ok: BOOL = (*data.add(1) == (*prop).script
                        || MAPBIT!(
                            _pcre2_ucd_script_sets_8
                                .as_ptr()
                                .add(UCD_SCRIPTX_PROP(prop) as usize),
                            *data.add(1) as u32
                        ) != 0) as BOOL;
                    if ok == isprop {
                        return not_negated;
                    }
                }

                PT_ALNUM => {
                    let chartype: c_int = (*prop).chartype as c_int;
                    if ((*_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_L
                        || *_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_N)
                        as BOOL)
                        == isprop
                    {
                        return not_negated;
                    }
                }

                /* Perl space used to exclude VT, but from Perl 5.18 it is included,
                which means that Perl space and POSIX space are now identical. PCRE
                was changed at release 8.34. */
                PT_SPACE /* Perl space */ | PT_PXSPACE /* POSIX space */ => {
                    match c {
                        /* HSPACE_CASES */
                        CHAR_HT
                        | CHAR_SPACE
                        | CHAR_NBSP
                        | 0x1680
                        | 0x180e
                        | 0x2000
                        | 0x2001
                        | 0x2002
                        | 0x2003
                        | 0x2004
                        | 0x2005
                        | 0x2006
                        | 0x2007
                        | 0x2008
                        | 0x2009
                        | 0x200a
                        | 0x202f
                        | 0x205f
                        | 0x3000
                        /* VSPACE_CASES */
                        | CHAR_LF
                        | CHAR_VT
                        | CHAR_FF
                        | CHAR_CR
                        | CHAR_NEL
                        | 0x2028
                        | 0x2029 => {
                            if isprop != 0 {
                                return not_negated;
                            }
                        }

                        _ => {
                            if ((*_pcre2_ucp_gentype_8
                                .as_ptr()
                                .add((*prop).chartype as usize)
                                == ucp_Z) as BOOL)
                                == isprop
                            {
                                return not_negated;
                            }
                        }
                    }
                }

                PT_WORD => {
                    let chartype: c_int = (*prop).chartype as c_int;
                    if ((*_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_L
                        || *_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_N
                        || chartype == ucp_Mn as c_int
                        || chartype == ucp_Pc as c_int) as BOOL)
                        == isprop
                    {
                        return not_negated;
                    }
                }

                PT_UCNC => {
                    if c < 0xa0 {
                        if ((c == CHAR_DOLLAR_SIGN
                            || c == CHAR_COMMERCIAL_AT
                            || c == CHAR_GRAVE_ACCENT) as BOOL)
                            == isprop
                        {
                            return not_negated;
                        }
                    } else {
                        if ((c < 0xd800 || c > 0xdfff) as BOOL) == isprop {
                            return not_negated;
                        }
                    }
                }

                PT_BIDICL => {
                    if ((UCD_BIDICLASS_PROP(prop) == *data.add(1) as u32) as BOOL) == isprop {
                        return not_negated;
                    }
                }

                PT_BOOL => {
                    let ok: BOOL = (MAPBIT!(
                        _pcre2_ucd_boolprop_sets_8
                            .as_ptr()
                            .add(UCD_BPROPS_PROP(prop) as usize),
                        *data.add(1) as u32
                    ) != 0) as BOOL;
                    if ok == isprop {
                        return not_negated;
                    }
                }

                /* The following three properties can occur only in an XCLASS, as there
                is no \p or \P coding for them. */

                /* Graphic character. Implement this as not Z (space or separator) and
                not C (other), except for Cf (format) with a few exceptions. This seems
                to be what Perl does. The exceptional characters are:

                U+061C           Arabic Letter Mark
                U+180E           Mongolian Vowel Separator
                U+2066 - U+2069  Various "isolate"s
                */
                PT_PXGRAPH => {
                    let chartype: c_int = (*prop).chartype as c_int;
                    if ((*_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) != ucp_Z
                        && (*_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) != ucp_C
                            || (chartype == ucp_Cf as c_int
                                && c != 0x061c
                                && c != 0x180e
                                && (c < 0x2066 || c > 0x2069)))) as BOOL)
                        == isprop
                    {
                        return not_negated;
                    }
                }

                /* Printable character: same as graphic, with the addition of Zs, i.e.
                not Zl and not Zp, and U+180E. */
                PT_PXPRINT => {
                    let chartype: c_int = (*prop).chartype as c_int;
                    if ((chartype != ucp_Zl as c_int
                        && chartype != ucp_Zp as c_int
                        && (*_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) != ucp_C
                            || (chartype == ucp_Cf as c_int
                                && c != 0x061c
                                && (c < 0x2066 || c > 0x2069)))) as BOOL)
                        == isprop
                    {
                        return not_negated;
                    }
                }

                /* Punctuation: all Unicode punctuation, plus ASCII characters that
                Unicode treats as symbols rather than punctuation, for Perl
                compatibility (these are $+<=>^`|~). */
                PT_PXPUNCT => {
                    let chartype: c_int = (*prop).chartype as c_int;
                    if ((*_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_P
                        || (c < 128
                            && *_pcre2_ucp_gentype_8.as_ptr().add(chartype as usize) == ucp_S))
                        as BOOL)
                        == isprop
                    {
                        return not_negated;
                    }
                }

                /* Perl has two sets of hex digits */
                PT_PXXDIGIT => {
                    if (((c >= CHAR_0 && c <= CHAR_9)
                        || (c >= CHAR_A && c <= CHAR_F)
                        || (c >= CHAR_a && c <= CHAR_f)
                        || (c >= 0xff10 && c <= 0xff19) /* Fullwidth digits */
                        || (c >= 0xff21 && c <= 0xff26) /* Fullwidth letters */
                        || (c >= 0xff41 && c <= 0xff46)) as BOOL)
                        == isprop
                    {
                        return not_negated;
                    }
                }

                /* This should never occur, but compilers may mutter if there is no
                default. */
                _ => {
                    return FALSE;
                }
            }

            data = data.add(2);

            if !(*data as u32 == XCL_PROP || *data as u32 == XCL_NOTPROP) {
                break;
            }
        }
    }

    /* Match against large chars or ranges that end with a large char. */
    if (*data as u32) < XCL_LIST {
        loop {
            let t: PCRE2_UCHAR = {
                let t = *data;
                data = data.add(1);
                t
            };
            if t as u32 == XCL_END {
                break;
            }

            let mut x: u32;
            let mut y: u32;

            if utf != 0 {
                GETCHARINC!(x, data); /* macro generates multiple statements */
            } else {
                x = {
                    let t = *data;
                    data = data.add(1);
                    t as u32
                };
            }

            if t as u32 == XCL_SINGLE {
                /* Since character ranges follow the properties, and they are
                sorted, early return is possible for all characters <= x. */
                if c <= x {
                    return if c == x {
                        not_negated
                    } else {
                        (not_negated == 0) as BOOL
                    };
                }
                continue;
            }

            /* PCRE2_ASSERT(t == XCL_RANGE); */
            if utf != 0 {
                GETCHARINC!(y, data); /* macro generates multiple statements */
            } else {
                y = {
                    let t = *data;
                    data = data.add(1);
                    t as u32
                };
            }

            /* Since character ranges follow the properties, and they are
            sorted, early return is possible for all characters <= y. */
            if c <= y {
                return if c >= x {
                    not_negated
                } else {
                    (not_negated == 0) as BOOL
                };
            }
        }

        return (not_negated == 0) as BOOL; /* char did not match */
    }

    type_ = ((*data.add(0) as u32) << 8) | (*data.add(1) as u32);
    data = data.add(2);

    /* Align characters. */
    next_char = char_lists_end.sub((GET!(data, 0) << 1) as usize);
    type_ &= XCL_TYPE_MASK;

    if c >= XCL_CHAR_LIST_HIGH_16_START {
        max_index = type_ & XCL_ITEM_COUNT_MASK;
        if max_index == XCL_ITEM_COUNT_MASK {
            max_index = (next_char as *const u16).read_unaligned() as u32;
            next_char = next_char.add(2);
        }

        next_char = next_char.add((max_index << 1) as usize);
        type_ >>= XCL_TYPE_BIT_LEN;
    }

    if c < XCL_CHAR_LIST_LOW_32_START {
        max_index = type_ & XCL_ITEM_COUNT_MASK;

        c = (((c << XCL_CHAR_SHIFT) | XCL_CHAR_END) as u16) as u32;

        if max_index == XCL_ITEM_COUNT_MASK {
            max_index = (next_char as *const u16).read_unaligned() as u32;
            next_char = next_char.add(2);
        }

        if max_index == 0 || c < (next_char as *const u16).read_unaligned() as u32 {
            return ((((type_ & XCL_BEGIN_WITH_RANGE) != 0) as BOOL) == not_negated) as BOOL;
        }

        min_index = 0;
        max_index = max_index.wrapping_sub(1);
        value = (next_char as *const u16).add(max_index as usize).read_unaligned() as u32;
        if c >= value {
            return (((value == c || (value & XCL_CHAR_END) == 0) as BOOL) == not_negated) as BOOL;
        }

        max_index = max_index.wrapping_sub(1);

        /* Binary search of a range. */
        loop {
            let mid_index: u32 = (min_index.wrapping_add(max_index)) >> 1;
            value = (next_char as *const u16)
                .add(mid_index as usize)
                .read_unaligned() as u32;

            if c < value {
                max_index = mid_index.wrapping_sub(1);
            } else if ((next_char as *const u16)
                .add(mid_index as usize + 1)
                .read_unaligned() as u32)
                <= c
            {
                min_index = mid_index.wrapping_add(1);
            } else {
                return (((value == c || (value & XCL_CHAR_END) == 0) as BOOL) == not_negated)
                    as BOOL;
            }
        }
    }

    /* Skip the 16 bit ranges. */
    max_index = type_ & XCL_ITEM_COUNT_MASK;
    if max_index == XCL_ITEM_COUNT_MASK {
        max_index = (next_char as *const u16).read_unaligned() as u32;
        next_char = next_char.add(2);
    }

    next_char = next_char.add((max_index << 1) as usize);
    type_ >>= XCL_TYPE_BIT_LEN;

    max_index = type_ & XCL_ITEM_COUNT_MASK;

    c = ((c << XCL_CHAR_SHIFT) | XCL_CHAR_END) as u32;

    if max_index == XCL_ITEM_COUNT_MASK {
        max_index = (next_char as *const u32).read_unaligned();
        next_char = next_char.add(4);
    }

    if max_index == 0 || c < (next_char as *const u32).read_unaligned() {
        return ((((type_ & XCL_BEGIN_WITH_RANGE) != 0) as BOOL) == not_negated) as BOOL;
    }

    min_index = 0;
    max_index = max_index.wrapping_sub(1);
    value = (next_char as *const u32).add(max_index as usize).read_unaligned();
    if c >= value {
        return (((value == c || (value & XCL_CHAR_END) == 0) as BOOL) == not_negated) as BOOL;
    }

    max_index = max_index.wrapping_sub(1);

    /* Binary search of a range. */
    loop {
        let mid_index: u32 = (min_index.wrapping_add(max_index)) >> 1;
        value = (next_char as *const u32)
            .add(mid_index as usize)
            .read_unaligned();

        if c < value {
            max_index = mid_index.wrapping_sub(1);
        } else if (next_char as *const u32)
            .add(mid_index as usize + 1)
            .read_unaligned()
            <= c
        {
            min_index = mid_index.wrapping_add(1);
        } else {
            return (((value == c || (value & XCL_CHAR_END) == 0) as BOOL) == not_negated) as BOOL;
        }
    }
}

/*************************************************
*       Match character against an ECLASS        *
*************************************************/

/* This function is called to match a character against an extended class
used for describing characters using boolean operations on sets.

Arguments:
  c           the character
  data_start  points to the start of the ECLASS data
  data_end    points one-past-the-last of the ECLASS data
  utf         TRUE if in UTF mode

Returns:      TRUE if character matches, else FALSE
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_eclass_8(
    c: u32,
    data_start: PCRE2_SPTR,
    data_end: PCRE2_SPTR,
    char_lists_end: *const u8,
    utf: BOOL,
) -> BOOL {
    let mut ptr: PCRE2_SPTR = data_start;
    let flags: PCRE2_UCHAR;
    let mut stack: u32 = 0;
    let mut stack_depth: c_int = 0;

    flags = *ptr;
    ptr = ptr.add(1);

    /* Code points < 256 are matched against a bitmap, if one is present.
    Otherwise all codepoints are checked later. */

    if (flags as u32 & ECL_MAP) != 0 {
        if c < 256 {
            return ((*ptr.add((c / 8) as usize) as u32 & (1u32 << (c & 7))) != 0) as BOOL;
        }

        /* Skip the bitmap. */
        ptr = ptr.add(32);
    }

    /* Do a little loop, until we reach the end of the ECLASS. */
    while ptr < data_end {
        let op = *ptr as u32;

        if op == ECL_AND {
            ptr = ptr.add(1);
            stack = (stack >> 1) & (stack | !(1u32));
            stack_depth -= 1;
        } else if op == ECL_OR {
            ptr = ptr.add(1);
            stack = (stack >> 1) | (stack & 1u32);
            stack_depth -= 1;
        } else if op == ECL_XOR {
            ptr = ptr.add(1);
            stack = (stack >> 1) ^ (stack & 1u32);
            stack_depth -= 1;
        } else if op == ECL_NOT {
            ptr = ptr.add(1);
            stack ^= 1u32;
        } else if op == ECL_XCLASS {
            let matched: u32 =
                _pcre2_xclass_8(c, ptr.add(1 + LINK_SIZE), char_lists_end, utf) as u32;

            ptr = ptr.add(GET!(ptr, 1) as usize);
            stack = (stack << 1) | matched;
            stack_depth += 1;
        } else {
            /* This should never occur, but compilers may mutter if there is no
            default. */
            return FALSE;
        }
    }

    let _ = stack_depth; /* Ignore unused variable, if assertions are disabled. */

    /* The final bit left on the stack now holds the match result. */
    ((stack & 1u32) != 0) as BOOL
}

/* End of pcre2_xclass.c */
