//! Translated from pcre2_xclass.c.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use core::ffi::{c_char, c_void};

/* PRIV(ucp_gentype)[i] - table lookup without a bounds-check panic path. */
#[inline]
unsafe fn ucp_gentype(i: i32) -> u32 {
    *crate::tables::_pcre2_ucp_gentype_8.as_ptr().add(i as usize)
}

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
    mut data: PCRE2_SPTR,
    char_lists_end: *const u8,
    mut utf: BOOL,
) -> BOOL {
    /* Update PRIV(update_classbits) when this function is changed. */
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
        let d0 = *data;
        data = data.add(1);
        if (d0 as u32 & XCL_MAP) != 0 {
            if c < 256 {
                return ((*data.add((c / 8) as usize) as u32 & (1u32 << (c & 7))) != 0) as BOOL;
            }
            /* Skip bitmap. */
            data = data.add(32 / core::mem::size_of::<u8>());
        }
    }

    /* Match against the list of Unicode properties. We won't ever
    encounter XCL_PROP or XCL_NOTPROP when UTF support is not compiled. */

    if *data as u32 == XCL_PROP || *data as u32 == XCL_NOTPROP {
        /* The UCD record is the same for all properties. */
        let prop: *const ucd_record = GET_UCD!(c);

        loop {
            let mut chartype: i32;
            let isprop: BOOL = ({
                let d = *data;
                data = data.add(1);
                d
            } as u32
                == XCL_PROP) as BOOL;
            let ok: BOOL;

            match *data as u32 {
                PT_LAMP => {
                    chartype = (*prop).chartype as i32;
                    if ((chartype == ucp_Lu as i32
                        || chartype == ucp_Ll as i32
                        || chartype == ucp_Lt as i32) as BOOL)
                        == isprop
                    {
                        return not_negated;
                    }
                }

                PT_GC => {
                    if ((*data.add(1) as u32 == ucp_gentype((*prop).chartype as i32)) as BOOL)
                        == isprop
                    {
                        return not_negated;
                    }
                }

                PT_PC => {
                    if ((*data.add(1) as u32 == (*prop).chartype as u32) as BOOL) == isprop {
                        return not_negated;
                    }
                }

                PT_SC => {
                    if ((*data.add(1) as u32 == (*prop).script as u32) as BOOL) == isprop {
                        return not_negated;
                    }
                }

                PT_SCX => {
                    ok = (*data.add(1) as u32 == (*prop).script as u32
                        || MAPBIT!(
                            crate::ucd::_pcre2_ucd_script_sets_8
                                .as_ptr()
                                .add(UCD_SCRIPTX_PROP!(prop) as usize),
                            *data.add(1)
                        ) != 0) as BOOL;
                    if ok == isprop {
                        return not_negated;
                    }
                }

                PT_ALNUM => {
                    chartype = (*prop).chartype as i32;
                    if ((ucp_gentype(chartype) == ucp_L || ucp_gentype(chartype) == ucp_N) as BOOL)
                        == isprop
                    {
                        return not_negated;
                    }
                }

                /* Perl space used to exclude VT, but from Perl 5.18 it is included,
                which means that Perl space and POSIX space are now identical. PCRE
                was changed at release 8.34. */
                PT_SPACE  /* Perl space */
                | PT_PXSPACE /* POSIX space */ => {
                    match c {
                        /* HSPACE_CASES */
                        0x09 | 0x20 | 0xa0 | 0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002
                        | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009
                        | 0x200a | 0x202f | 0x205f | 0x3000
                        /* VSPACE_CASES */
                        | 0x0a | 0x0b | 0x0c | 0x0d | 0x85 | 0x2028 | 0x2029 => {
                            if isprop != 0 {
                                return not_negated;
                            }
                        }

                        _ => {
                            if ((ucp_gentype((*prop).chartype as i32) == ucp_Z) as BOOL) == isprop {
                                return not_negated;
                            }
                        }
                    }
                }

                PT_WORD => {
                    chartype = (*prop).chartype as i32;
                    if ((ucp_gentype(chartype) == ucp_L
                        || ucp_gentype(chartype) == ucp_N
                        || chartype == ucp_Mn as i32
                        || chartype == ucp_Pc as i32) as BOOL)
                        == isprop
                    {
                        return not_negated;
                    }
                }

                PT_UCNC => {
                    if c < 0xa0 {
                        if ((c == b'$' as u32 /* CHAR_DOLLAR_SIGN */
                            || c == b'@' as u32 /* CHAR_COMMERCIAL_AT */
                            || c == b'`' as u32 /* CHAR_GRAVE_ACCENT */) as BOOL)
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
                    if ((UCD_BIDICLASS_PROP!(prop) == *data.add(1) as u32) as BOOL) == isprop {
                        return not_negated;
                    }
                }

                PT_BOOL => {
                    ok = (MAPBIT!(
                        crate::ucd::_pcre2_ucd_boolprop_sets_8
                            .as_ptr()
                            .add(UCD_BPROPS_PROP!(prop) as usize),
                        *data.add(1)
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
                    chartype = (*prop).chartype as i32;
                    if ((ucp_gentype(chartype) != ucp_Z
                        && (ucp_gentype(chartype) != ucp_C
                            || (chartype == ucp_Cf as i32
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
                    chartype = (*prop).chartype as i32;
                    if ((chartype != ucp_Zl as i32
                        && chartype != ucp_Zp as i32
                        && (ucp_gentype(chartype) != ucp_C
                            || (chartype == ucp_Cf as i32
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
                    chartype = (*prop).chartype as i32;
                    if ((ucp_gentype(chartype) == ucp_P
                        || (c < 128 && ucp_gentype(chartype) == ucp_S)) as BOOL)
                        == isprop
                    {
                        return not_negated;
                    }
                }

                /* Perl has two sets of hex digits */
                PT_PXXDIGIT => {
                    if (((c >= b'0' as u32 && c <= b'9' as u32)
                        || (c >= b'A' as u32 && c <= b'F' as u32)
                        || (c >= b'a' as u32 && c <= b'f' as u32)
                        || (c >= 0xff10 && c <= 0xff19)   /* Fullwidth digits */
                        || (c >= 0xff21 && c <= 0xff26)   /* Fullwidth letters */
                        || (c >= 0xff41 && c <= 0xff46)) as BOOL)
                        == isprop
                    {
                        return not_negated;
                    }
                }

                /* This should never occur, but compilers may mutter if there is no
                default. */

                /* LCOV_EXCL_START */
                _ => {
                    /* PCRE2_DEBUG_UNREACHABLE */
                    return FALSE;
                }
                /* LCOV_EXCL_STOP */
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
            let t: u8 = {
                let d = *data;
                data = data.add(1);
                d
            };
            if t as u32 == XCL_END {
                break;
            }

            let mut x: u32 = 0;
            let mut y: u32 = 0;

            if utf != 0 {
                GETCHARINC!(x, data); /* macro generates multiple statements */
            } else {
                x = *data as u32;
                data = data.add(1);
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
                y = *data as u32;
                data = data.add(1);
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

    type_ = ((*data.add(0) as u32) << 8) | *data.add(1) as u32;
    data = data.add(2);

    /* Align characters. */
    next_char = char_lists_end.wrapping_sub((GET!(data, 0) << 1) as usize);
    type_ &= XCL_TYPE_MASK;

    /* Alignment check. */
    /* PCRE2_ASSERT(((uintptr_t)next_char & 0x1) == 0); */

    if c >= XCL_CHAR_LIST_HIGH_16_START {
        max_index = type_ & XCL_ITEM_COUNT_MASK;
        if max_index == XCL_ITEM_COUNT_MASK {
            max_index = core::ptr::read_unaligned(next_char as *const u16) as u32;
            /* PCRE2_ASSERT(max_index >= XCL_ITEM_COUNT_MASK); */
            next_char = next_char.add(2);
        }

        next_char = next_char.add((max_index << 1) as usize);
        type_ >>= XCL_TYPE_BIT_LEN;
    }

    if c < XCL_CHAR_LIST_LOW_32_START {
        max_index = type_ & XCL_ITEM_COUNT_MASK;

        c = (((c << XCL_CHAR_SHIFT) | XCL_CHAR_END) as u16) as u32;

        if max_index == XCL_ITEM_COUNT_MASK {
            max_index = core::ptr::read_unaligned(next_char as *const u16) as u32;
            /* PCRE2_ASSERT(max_index >= XCL_ITEM_COUNT_MASK); */
            next_char = next_char.add(2);
        }

        if max_index == 0 || c < core::ptr::read_unaligned(next_char as *const u16) as u32 {
            return (((type_ & XCL_BEGIN_WITH_RANGE) != 0) as BOOL == not_negated) as BOOL;
        }

        min_index = 0;
        max_index = max_index.wrapping_sub(1);
        value = core::ptr::read_unaligned((next_char as *const u16).add(max_index as usize)) as u32;
        if c >= value {
            return ((value == c || (value & XCL_CHAR_END) == 0) as BOOL == not_negated) as BOOL;
        }

        max_index = max_index.wrapping_sub(1);

        /* Binary search of a range. */
        loop {
            let mid_index: u32 = (min_index.wrapping_add(max_index)) >> 1;
            value =
                core::ptr::read_unaligned((next_char as *const u16).add(mid_index as usize)) as u32;

            if c < value {
                max_index = mid_index.wrapping_sub(1);
            } else if (core::ptr::read_unaligned(
                (next_char as *const u16).add((mid_index.wrapping_add(1)) as usize),
            ) as u32)
                <= c
            {
                min_index = mid_index.wrapping_add(1);
            } else {
                return ((value == c || (value & XCL_CHAR_END) == 0) as BOOL == not_negated) as BOOL;
            }
        }
    }

    /* Skip the 16 bit ranges. */
    max_index = type_ & XCL_ITEM_COUNT_MASK;
    if max_index == XCL_ITEM_COUNT_MASK {
        max_index = core::ptr::read_unaligned(next_char as *const u16) as u32;
        /* PCRE2_ASSERT(max_index >= XCL_ITEM_COUNT_MASK); */
        next_char = next_char.add(2);
    }

    next_char = next_char.add((max_index << 1) as usize);
    type_ >>= XCL_TYPE_BIT_LEN;

    /* Alignment check. */
    /* PCRE2_ASSERT(((uintptr_t)next_char & 0x3) == 0); */

    max_index = type_ & XCL_ITEM_COUNT_MASK;

    c = ((c << XCL_CHAR_SHIFT) | XCL_CHAR_END) as u32;

    if max_index == XCL_ITEM_COUNT_MASK {
        max_index = core::ptr::read_unaligned(next_char as *const u32);
        next_char = next_char.add(4);
    }

    if max_index == 0 || c < core::ptr::read_unaligned(next_char as *const u32) {
        return (((type_ & XCL_BEGIN_WITH_RANGE) != 0) as BOOL == not_negated) as BOOL;
    }

    min_index = 0;
    max_index = max_index.wrapping_sub(1);
    value = core::ptr::read_unaligned((next_char as *const u32).add(max_index as usize));
    if c >= value {
        return ((value == c || (value & XCL_CHAR_END) == 0) as BOOL == not_negated) as BOOL;
    }

    max_index = max_index.wrapping_sub(1);

    /* Binary search of a range. */
    loop {
        let mid_index: u32 = (min_index.wrapping_add(max_index)) >> 1;
        value = core::ptr::read_unaligned((next_char as *const u32).add(mid_index as usize));

        if c < value {
            max_index = mid_index.wrapping_sub(1);
        } else if core::ptr::read_unaligned(
            (next_char as *const u32).add((mid_index.wrapping_add(1)) as usize),
        ) <= c
        {
            min_index = mid_index.wrapping_add(1);
        } else {
            return ((value == c || (value & XCL_CHAR_END) == 0) as BOOL == not_negated) as BOOL;
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
    let flags: u8;
    let mut stack: u32 = 0;
    let mut stack_depth: i32 = 0;

    /* PCRE2_ASSERT(data_start < data_end); */
    flags = {
        let d = *ptr;
        ptr = ptr.add(1);
        d
    };

    /* Code points < 256 are matched against a bitmap, if one is present.
    Otherwise all codepoints are checked later. */

    if (flags as u32 & ECL_MAP) != 0 {
        if c < 256 {
            return ((*ptr.add((c / 8) as usize) as u32 & (1u32 << (c & 7))) != 0) as BOOL;
        }

        /* Skip the bitmap. */
        ptr = ptr.add(32 / core::mem::size_of::<u8>());
    }

    /* Do a little loop, until we reach the end of the ECLASS. */
    while ptr < data_end {
        match *ptr as u32 {
            ECL_AND => {
                ptr = ptr.add(1);
                stack = (stack >> 1) & (stack | !1u32);
                /* PCRE2_ASSERT(stack_depth >= 2); */
                stack_depth -= 1;
            }

            ECL_OR => {
                ptr = ptr.add(1);
                stack = (stack >> 1) | (stack & 1u32);
                /* PCRE2_ASSERT(stack_depth >= 2); */
                stack_depth -= 1;
            }

            ECL_XOR => {
                ptr = ptr.add(1);
                stack = (stack >> 1) ^ (stack & 1u32);
                /* PCRE2_ASSERT(stack_depth >= 2); */
                stack_depth -= 1;
            }

            ECL_NOT => {
                ptr = ptr.add(1);
                stack ^= 1u32;
                /* PCRE2_ASSERT(stack_depth >= 1); */
            }

            ECL_XCLASS => {
                let matched: u32 =
                    _pcre2_xclass_8(c, ptr.add(1 + LINK_SIZE), char_lists_end, utf) as u32;

                ptr = ptr.add(GET!(ptr, 1) as usize);
                stack = (stack << 1) | matched;
                stack_depth += 1;
            }

            /* This should never occur, but compilers may mutter if there is no
            default. */

            /* LCOV_EXCL_START */
            _ => {
                /* PCRE2_DEBUG_UNREACHABLE */
                return FALSE;
            }
            /* LCOV_EXCL_STOP */
        }
    }

    /* PCRE2_ASSERT(stack_depth == 1); */

    /* The final bit left on the stack now holds the match result. */
    return ((stack & 1u32) != 0) as BOOL;
}

/* End of pcre2_xclass.c */
