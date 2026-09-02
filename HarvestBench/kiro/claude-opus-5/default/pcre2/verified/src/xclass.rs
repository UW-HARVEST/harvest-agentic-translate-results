//! Translation of `pcre2_xclass.c`.
//!
//! This module contains two internal functions that are used to match
//! `OP_XCLASS` and `OP_ECLASS`. It is used by `pcre2_auto_possessify()` and by
//! both `pcre2_match()` and `pcre2_dfa_match()`.
//!
//! This is the 8-bit build (`PCRE2_CODE_UNIT_WIDTH == 8`) with
//! `SUPPORT_UNICODE` enabled, which implies `SUPPORT_WIDE_CHARS`.

use crate::internal::*;
use core::ffi::c_int;

// ---------------------------------------------------------------------------
// ASCII CHAR_* constants used below (this build is not EBCDIC).
// ---------------------------------------------------------------------------
const CHAR_DOLLAR_SIGN: u32 = 0x24;
const CHAR_COMMERCIAL_AT: u32 = 0x40;
const CHAR_GRAVE_ACCENT: u32 = 0x60;
const CHAR_0: u32 = 0x30;
const CHAR_9: u32 = 0x39;
const CHAR_A: u32 = 0x41;
const CHAR_F: u32 = 0x46;
const CHAR_a: u32 = 0x61;
const CHAR_f: u32 = 0x66;

/// `XCL_LIST` for 8-bit mode: `sizeof(PCRE2_UCHAR) == 1 ? 0x10 : 0x1000`.
const XCL_LIST: i64 = 0x10;

/// `true` for the code points covered by the `HSPACE_CASES` macro
/// (horizontal white space).
#[inline(always)]
fn is_hspace_case(c: u32) -> bool {
    matches!(
        c,
        0x09 | 0x20
            | 0xa0
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
    )
}

/// `true` for the code points covered by the `VSPACE_CASES` macro
/// (vertical white space).
#[inline(always)]
fn is_vspace_case(c: u32) -> bool {
    matches!(c, 0x0a | 0x0b | 0x0c | 0x0d | 0x85 | 0x2028 | 0x2029)
}

/// `PRIV(xclass)` — match a character against an extended class that might
/// contain code points above 255 and/or Unicode properties.
///
/// Arguments:
///   c               the character
///   data            points to the flag code unit of the XCLASS data
///   char_lists_end  points one-past-the-end of the character lists
///   utf             TRUE if in UTF mode
///
/// Returns:          TRUE if character matches, else FALSE
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_xclass_8(
    mut c: u32,
    data: PCRE2_SPTR,
    char_lists_end: *const u8,
    utf: BOOL,
) -> BOOL {
    unsafe {
        // Update PRIV(update_classbits) when this function is changed.
        let mut t: PCRE2_UCHAR;
        let not_negated: BOOL = if (*data & (XCL_NOT as u8)) == 0 { TRUE } else { FALSE };
        let mut type_: u32;
        let mut max_index: u32;
        let mut min_index: u32;
        let mut value: u32;
        let mut next_char: *const u8;

        let mut data = data;

        // In 8 bit mode, this must always be TRUE. Help the compiler to know
        // that.
        let _ = utf;
        let utf = true;

        // Code points < 256 are matched against a bitmap, if one is present.
        let first = *data;
        data = data.add(1);
        if (first & (XCL_MAP as u8)) != 0 {
            if c < 256 {
                return if ((*(data as *const u8).add((c / 8) as usize)) & (1u8 << (c & 7))) != 0 {
                    TRUE
                } else {
                    FALSE
                };
            }
            // Skip bitmap. (32 / sizeof(PCRE2_UCHAR) == 32 in 8-bit mode.)
            data = data.add(32 / core::mem::size_of::<PCRE2_UCHAR>());
        }

        // Match against the list of Unicode properties. SUPPORT_UNICODE is
        // defined in this configuration.
        if *data == (XCL_PROP as u8) || *data == (XCL_NOTPROP as u8) {
            // The UCD record is the same for all properties.
            let prop: &UcdRecord = GET_UCD(c);

            loop {
                let mut chartype: u32;
                let d0 = *data;
                data = data.add(1);
                let isprop: BOOL = if d0 == (XCL_PROP as u8) { TRUE } else { FALSE };
                let ok: BOOL;

                let ptype = *data as i64;
                match ptype {
                    x if x == PT_LAMP => {
                        chartype = prop.chartype as u32;
                        if (chartype == ucp_Lu || chartype == ucp_Ll || chartype == ucp_Lt)
                            == (isprop != FALSE)
                        {
                            return not_negated;
                        }
                    }

                    x if x == PT_GC => {
                        if (*data.add(1) as u32
                            == crate::tables::_pcre2_ucp_gentype[prop.chartype as usize])
                            == (isprop != FALSE)
                        {
                            return not_negated;
                        }
                    }

                    x if x == PT_PC => {
                        if (*data.add(1) as u32 == prop.chartype as u32) == (isprop != FALSE) {
                            return not_negated;
                        }
                    }

                    x if x == PT_SC => {
                        if (*data.add(1) as u32 == prop.script as u32) == (isprop != FALSE) {
                            return not_negated;
                        }
                    }

                    x if x == PT_SCX => {
                        let dv = *data.add(1) as u32;
                        let okb = dv == prop.script as u32
                            || MAPBIT(
                                (crate::tables::_pcre2_ucd_script_sets)
                                    .as_ptr()
                                    .add(UCD_SCRIPTX_PROP(prop) as usize),
                                dv,
                            ) != 0;
                        if okb == (isprop != FALSE) {
                            return not_negated;
                        }
                    }

                    x if x == PT_ALNUM => {
                        chartype = prop.chartype as u32;
                        if (crate::tables::_pcre2_ucp_gentype[chartype as usize] == ucp_L
                            || crate::tables::_pcre2_ucp_gentype[chartype as usize] == ucp_N)
                            == (isprop != FALSE)
                        {
                            return not_negated;
                        }
                    }

                    // Perl space and POSIX space are now identical (since PCRE
                    // 8.34 / Perl 5.18).
                    x if x == PT_SPACE || x == PT_PXSPACE => {
                        if is_hspace_case(c) || is_vspace_case(c) {
                            if isprop != FALSE {
                                return not_negated;
                            }
                        } else if (crate::tables::_pcre2_ucp_gentype[prop.chartype as usize]
                            == ucp_Z)
                            == (isprop != FALSE)
                        {
                            return not_negated;
                        }
                    }

                    x if x == PT_WORD => {
                        chartype = prop.chartype as u32;
                        if (crate::tables::_pcre2_ucp_gentype[chartype as usize] == ucp_L
                            || crate::tables::_pcre2_ucp_gentype[chartype as usize] == ucp_N
                            || chartype == ucp_Mn
                            || chartype == ucp_Pc)
                            == (isprop != FALSE)
                        {
                            return not_negated;
                        }
                    }

                    x if x == PT_UCNC => {
                        if c < 0xa0 {
                            if ((c == CHAR_DOLLAR_SIGN
                                || c == CHAR_COMMERCIAL_AT
                                || c == CHAR_GRAVE_ACCENT)
                                == (isprop != FALSE))
                            {
                                return not_negated;
                            }
                        } else if ((c < 0xd800 || c > 0xdfff) == (isprop != FALSE)) {
                            return not_negated;
                        }
                    }

                    x if x == PT_BIDICL => {
                        if (UCD_BIDICLASS_PROP(prop) == *data.add(1) as u32) == (isprop != FALSE) {
                            return not_negated;
                        }
                    }

                    x if x == PT_BOOL => {
                        ok = if MAPBIT(
                            (crate::tables::_pcre2_ucd_boolprop_sets)
                                .as_ptr()
                                .add(UCD_BPROPS_PROP(prop) as usize),
                            *data.add(1) as u32,
                        ) != 0
                        {
                            TRUE
                        } else {
                            FALSE
                        };
                        if (ok != FALSE) == (isprop != FALSE) {
                            return not_negated;
                        }
                    }

                    // Graphic character.
                    x if x == PT_PXGRAPH => {
                        chartype = prop.chartype as u32;
                        if (crate::tables::_pcre2_ucp_gentype[chartype as usize] != ucp_Z
                            && (crate::tables::_pcre2_ucp_gentype[chartype as usize] != ucp_C
                                || (chartype == ucp_Cf
                                    && c != 0x061c
                                    && c != 0x180e
                                    && (c < 0x2066 || c > 0x2069))))
                            == (isprop != FALSE)
                        {
                            return not_negated;
                        }
                    }

                    // Printable character.
                    x if x == PT_PXPRINT => {
                        chartype = prop.chartype as u32;
                        if (chartype != ucp_Zl
                            && chartype != ucp_Zp
                            && (crate::tables::_pcre2_ucp_gentype[chartype as usize] != ucp_C
                                || (chartype == ucp_Cf
                                    && c != 0x061c
                                    && (c < 0x2066 || c > 0x2069))))
                            == (isprop != FALSE)
                        {
                            return not_negated;
                        }
                    }

                    // Punctuation.
                    x if x == PT_PXPUNCT => {
                        chartype = prop.chartype as u32;
                        if (crate::tables::_pcre2_ucp_gentype[chartype as usize] == ucp_P
                            || (c < 128
                                && crate::tables::_pcre2_ucp_gentype[chartype as usize] == ucp_S))
                            == (isprop != FALSE)
                        {
                            return not_negated;
                        }
                    }

                    // Perl has two sets of hex digits.
                    x if x == PT_PXXDIGIT => {
                        if (((c >= CHAR_0 && c <= CHAR_9)
                            || (c >= CHAR_A && c <= CHAR_F)
                            || (c >= CHAR_a && c <= CHAR_f)
                            || (c >= 0xff10 && c <= 0xff19)
                            || (c >= 0xff21 && c <= 0xff26)
                            || (c >= 0xff41 && c <= 0xff46))
                            == (isprop != FALSE))
                        {
                            return not_negated;
                        }
                    }

                    // This should never occur.
                    _ => {
                        return FALSE;
                    }
                }

                data = data.add(2);

                if !(*data == (XCL_PROP as u8) || *data == (XCL_NOTPROP as u8)) {
                    break;
                }
            }
        }

        // Match against large chars or ranges that end with a large char.
        if (*data as i64) < XCL_LIST {
            loop {
                t = *data;
                data = data.add(1);
                if t as i64 == XCL_END {
                    break;
                }

                let x: u32;
                let y: u32;

                // utf is always true in 8-bit mode.
                x = GETCHARINC(&mut data);

                if t as i64 == XCL_SINGLE {
                    // Since character ranges follow the properties, and they are
                    // sorted, early return is possible for all characters <= x.
                    if c <= x {
                        return if c == x { not_negated } else { not_not(not_negated) };
                    }
                    continue;
                }

                // PCRE2_ASSERT(t == XCL_RANGE);
                y = GETCHARINC(&mut data);

                // Since character ranges follow the properties, and they are
                // sorted, early return is possible for all characters <= y.
                if c <= y {
                    return if c >= x { not_negated } else { not_not(not_negated) };
                }
            }

            return not_not(not_negated); // char did not match
        }

        // CODE_UNIT_WIDTH == 8: type is stored big-endian in two code units.
        type_ = ((*data.add(0) as u32) << 8) | (*data.add(1) as u32);
        data = data.add(2);

        // Align characters.
        next_char = char_lists_end.offset(-((GET(data, 0) as isize) << 1));
        type_ &= XCL_TYPE_MASK as u32;

        // Alignment check (PCRE2_ASSERT).

        if c >= XCL_CHAR_LIST_HIGH_16_START as u32 {
            max_index = type_ & (XCL_ITEM_COUNT_MASK as u32);
            if max_index == XCL_ITEM_COUNT_MASK as u32 {
                max_index = (next_char as *const u16).read_unaligned() as u32;
                next_char = next_char.add(2);
            }

            next_char = next_char.add((max_index as usize) << 1);
            type_ >>= XCL_TYPE_BIT_LEN as u32;
        }

        if c < XCL_CHAR_LIST_LOW_32_START as u32 {
            max_index = type_ & (XCL_ITEM_COUNT_MASK as u32);

            c = (((c << (XCL_CHAR_SHIFT as u32)) | (XCL_CHAR_END as u32)) & 0xffff) as u32;

            if max_index == XCL_ITEM_COUNT_MASK as u32 {
                max_index = (next_char as *const u16).read_unaligned() as u32;
                next_char = next_char.add(2);
            }

            if max_index == 0 || c < (next_char as *const u16).read_unaligned() as u32 {
                return if ((type_ & (XCL_BEGIN_WITH_RANGE as u32)) != 0) == (not_negated != FALSE) {
                    TRUE
                } else {
                    FALSE
                };
            }

            min_index = 0;
            max_index -= 1;
            value = (next_char as *const u16).add(max_index as usize).read_unaligned() as u32;
            if c >= value {
                return if (value == c || (value & (XCL_CHAR_END as u32)) == 0)
                    == (not_negated != FALSE)
                {
                    TRUE
                } else {
                    FALSE
                };
            }

            max_index -= 1;

            // Binary search of a range.
            loop {
                let mid_index = (min_index + max_index) >> 1;
                value = (next_char as *const u16).add(mid_index as usize).read_unaligned() as u32;

                if c < value {
                    max_index = mid_index - 1;
                } else if ((next_char as *const u16).add((mid_index + 1) as usize).read_unaligned() as u32) <= c {
                    min_index = mid_index + 1;
                } else {
                    return if (value == c || (value & (XCL_CHAR_END as u32)) == 0)
                        == (not_negated != FALSE)
                    {
                        TRUE
                    } else {
                        FALSE
                    };
                }
            }
        }

        // Skip the 16 bit ranges.
        max_index = type_ & (XCL_ITEM_COUNT_MASK as u32);
        if max_index == XCL_ITEM_COUNT_MASK as u32 {
            max_index = (next_char as *const u16).read_unaligned() as u32;
            next_char = next_char.add(2);
        }

        next_char = next_char.add((max_index as usize) << 1);
        type_ >>= XCL_TYPE_BIT_LEN as u32;

        // Alignment check (PCRE2_ASSERT).

        max_index = type_ & (XCL_ITEM_COUNT_MASK as u32);

        // The `#if PCRE2_CODE_UNIT_WIDTH == 32` HIGH_32 skip block is not
        // compiled in this 8-bit build.

        c = ((c << (XCL_CHAR_SHIFT as u32)) | (XCL_CHAR_END as u32)) as u32;

        if max_index == XCL_ITEM_COUNT_MASK as u32 {
            max_index = (next_char as *const u32).read_unaligned();
            next_char = next_char.add(4);
        }

        if max_index == 0 || c < (next_char as *const u32).read_unaligned() {
            return if ((type_ & (XCL_BEGIN_WITH_RANGE as u32)) != 0) == (not_negated != FALSE) {
                TRUE
            } else {
                FALSE
            };
        }

        min_index = 0;
        max_index -= 1;
        value = (next_char as *const u32).add(max_index as usize).read_unaligned();
        if c >= value {
            return if (value == c || (value & (XCL_CHAR_END as u32)) == 0) == (not_negated != FALSE)
            {
                TRUE
            } else {
                FALSE
            };
        }

        max_index -= 1;

        // Binary search of a range.
        loop {
            let mid_index = (min_index + max_index) >> 1;
            value = (next_char as *const u32).add(mid_index as usize).read_unaligned();

            if c < value {
                max_index = mid_index - 1;
            } else if (next_char as *const u32).add((mid_index + 1) as usize).read_unaligned() <= c {
                min_index = mid_index + 1;
            } else {
                return if (value == c || (value & (XCL_CHAR_END as u32)) == 0)
                    == (not_negated != FALSE)
                {
                    TRUE
                } else {
                    FALSE
                };
            }
        }
    }
}

/// C's `!not_negated` on a `BOOL`.
#[inline(always)]
fn not_not(b: BOOL) -> BOOL {
    if b == FALSE { TRUE } else { FALSE }
}

/// `PRIV(eclass)` — match a character against an extended class used for
/// describing characters using boolean operations on sets.
///
/// Arguments:
///   c               the character
///   data_start      points to the start of the ECLASS data
///   data_end        points one-past-the-last of the ECLASS data
///   char_lists_end  points one-past-the-end of the character lists
///   utf             TRUE if in UTF mode
///
/// Returns:          TRUE if character matches, else FALSE
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_eclass_8(
    c: u32,
    data_start: PCRE2_SPTR,
    data_end: PCRE2_SPTR,
    char_lists_end: *const u8,
    utf: BOOL,
) -> BOOL {
    unsafe {
        let mut ptr: PCRE2_SPTR = data_start;
        let flags: PCRE2_UCHAR;
        let mut stack: u32 = 0;
        let mut stack_depth: c_int = 0;

        // PCRE2_ASSERT(data_start < data_end);
        flags = *ptr;
        ptr = ptr.add(1);

        // Code points < 256 are matched against a bitmap, if one is present.
        // Otherwise all codepoints are checked later.
        if (flags & (ECL_MAP as u8)) != 0 {
            if c < 256 {
                return if ((*(ptr as *const u8).add((c / 8) as usize)) & (1u8 << (c & 7))) != 0 {
                    TRUE
                } else {
                    FALSE
                };
            }

            // Skip the bitmap.
            ptr = ptr.add(32 / core::mem::size_of::<PCRE2_UCHAR>());
        }

        // Do a little loop, until we reach the end of the ECLASS.
        while ptr < data_end {
            let op = *ptr as i64;
            if op == ECL_AND {
                ptr = ptr.add(1);
                stack = (stack >> 1) & (stack | !1u32);
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
                    _pcre2_xclass_8(c, ptr.add(1 + LINK_SIZE_U), char_lists_end, utf) as u32;

                ptr = ptr.add(GET(ptr, 1) as usize);
                stack = (stack << 1) | matched;
                stack_depth += 1;
            } else {
                // This should never occur.
                return FALSE;
            }
        }

        // PCRE2_ASSERT(stack_depth == 1);
        let _ = stack_depth;

        // The final bit left on the stack now holds the match result.
        if (stack & 1u32) != 0 { TRUE } else { FALSE }
    }
}

// Silence unused-import lints for c_int if not otherwise referenced.
#[allow(unused_imports)]
use c_int as _;
