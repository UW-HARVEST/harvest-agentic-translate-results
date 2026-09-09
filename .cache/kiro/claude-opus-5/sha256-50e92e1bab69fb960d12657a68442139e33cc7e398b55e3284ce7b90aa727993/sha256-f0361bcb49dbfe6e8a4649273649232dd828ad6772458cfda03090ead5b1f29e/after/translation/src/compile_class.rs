//! Translation of `pcre2_compile_class.c`.
//!
//! This module builds character-class opcodes: the class bitmap,
//! `OP_XCLASS` construction, the `XCL_LIST` character-list encoding, the
//! extended-class (`OP_ECLASS`) expression compiler, and the Unicode property
//! handling for POSIX classes.
//!
//! This is the 8-bit build (`PCRE2_CODE_UNIT_WIDTH == 8`) with
//! `SUPPORT_UNICODE` enabled (which implies `SUPPORT_WIDE_CHARS`),
//! `SUPPORT_JIT` off, and `PCRE2_DEBUG` off. EBCDIC is not supported.

use crate::compile_h::*;
use crate::consts::*;
use crate::internal::*;
// Disambiguate BOOL/TRUE/FALSE (defined in both consts and internal globs).
// The internal.rs versions are the `c_int`-typed ones we want.
use crate::internal::{BOOL, FALSE, TRUE};
use core::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// ASCII CHAR_* constants used below (this build is not EBCDIC).
// ---------------------------------------------------------------------------
const CHAR_CR: u32 = 0x0d;
const CHAR_NL_C: u32 = 0x0a;
const CHAR_DOLLAR_SIGN: u32 = 0x24;
const CHAR_COMMERCIAL_AT: u32 = 0x40;
const CHAR_GRAVE_ACCENT: u32 = 0x60;
const CHAR_0: u32 = 0x30;
const CHAR_9: u32 = 0x39;
const CHAR_A: u32 = 0x41;
const CHAR_F: u32 = 0x46;
const CHAR_a: u32 = 0x61;
const CHAR_f: u32 = 0x66;

// ---------------------------------------------------------------------------
// eclass_context
// ---------------------------------------------------------------------------

/// Local context threaded through the ECLASS-compiling functions.
struct eclass_context {
    /// Option bits for eclass.
    options: u32,
    xoptions: u32,
    /// Rarely used members.
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
    /// Bitmap is needed.
    needs_bitmap: BOOL,
}

// ---------------------------------------------------------------------------
// UCD helper macros not present in internal.rs
// ---------------------------------------------------------------------------

/// `UCD_ANY_I(ch)` — match any of 'i', 'I', U+0130, U+0131.
#[inline(always)]
fn UCD_ANY_I(ch: u32) -> bool {
    (ch | 0x20u32) == 0x69u32 || (ch | 1u32) == 0x0131u32
}

/// `UCD_DOTTED_I(ch)`.
#[inline(always)]
fn UCD_DOTTED_I(ch: u32) -> bool {
    ch == 0x69u32 || ch == 0x0130u32
}

// ---------------------------------------------------------------------------
// PARSE_CLASS flags (SUPPORT_UNICODE)
// ---------------------------------------------------------------------------

const PARSE_CLASS_UTF: u32 = 0x1;
const PARSE_CLASS_CASELESS_UTF: u32 = 0x2;
const PARSE_CLASS_RESTRICTED_UTF: u32 = 0x4;
const PARSE_CLASS_TURKISH_UTF: u32 = 0x8;

// ---------------------------------------------------------------------------
// XClass related properties
// ---------------------------------------------------------------------------

/// XClass needs to be generated.
const XCLASS_REQUIRED: u32 = 0x1;
/// XClass has 8 bit character.
const XCLASS_HAS_8BIT_CHARS: u32 = 0x2;
/// XClass has properties.
const XCLASS_HAS_PROPS: u32 = 0x4;
/// XClass has character lists.
const XCLASS_HAS_CHAR_LISTS: u32 = 0x8;
/// XClass matches to all >= 256 characters.
const XCLASS_HIGH_ANY: u32 = 0x10;

/// `XCL_LIST` for 8-bit mode: `sizeof(PCRE2_UCHAR) == 1 ? 0x10 : 0x1000`.
const XCL_LIST_VAL: u32 = 0x10;

// ---------------------------------------------------------------------------
// SUPPORT_WIDE_CHARS block
// ---------------------------------------------------------------------------

/// Heapsort helper.
unsafe fn do_heapify(buffer: *mut u32, size: usize, mut i: usize) {
    unsafe {
        loop {
            let mut max = i;
            let left = (i << 1) + 2;
            let right = left + 2;

            if left < size && *buffer.add(left) > *buffer.add(max) {
                max = left;
            }
            if right < size && *buffer.add(right) > *buffer.add(max) {
                max = right;
            }
            if i == max {
                return;
            }

            // Swap items.
            let tmp1 = *buffer.add(i);
            let tmp2 = *buffer.add(i + 1);
            *buffer.add(i) = *buffer.add(max);
            *buffer.add(i + 1) = *buffer.add(max + 1);
            *buffer.add(max) = tmp1;
            *buffer.add(max + 1) = tmp2;
            i = max;
        }
    }
}

/// Get the range of nocase characters which includes the 'c' character passed
/// as argument, or directly follows 'c'.
unsafe fn get_nocase_range(c: u32) -> *const u32 {
    unsafe {
        let base = crate::tables::_pcre2_ucd_nocase_ranges.as_ptr();
        let mut left: u32 = 0;
        let mut right: u32 = crate::tables::_pcre2_ucd_nocase_ranges_size;

        if c > MAX_UTF_CODE_POINT as u32 {
            return base.add(right as usize);
        }

        loop {
            // Range end of the middle element.
            let middle = ((left + right) >> 1) | 0x1;

            if *base.add(middle as usize) <= c {
                left = middle + 1;
            } else if middle > 1 && *base.add((middle - 2) as usize) > c {
                right = middle - 1;
            } else {
                return base.add((middle - 1) as usize);
            }
        }
    }
}

/// Get the list of othercase characters belonging to the passed range; create
/// ranges from these characters and append them to the buffer.
unsafe fn utf_caseless_extend(
    start: u32,
    end: u32,
    options: u32,
    mut buffer: *mut u32,
) -> usize {
    unsafe {
        let mut new_start = start;
        let mut new_end = end;
        let mut c = start;
        let mut list: *const u32;
        let mut tmp: [u32; 3] = [0; 3];
        let mut result: usize = 2;
        let mut skip_range = get_nocase_range(c);
        let mut skip_start = *skip_range.add(0);

        // PCRE2_ASSERT(options & PARSE_CLASS_UTF) in 8-bit mode.

        while c <= end {
            let mut co: u32;

            if c > skip_start {
                c = *skip_range.add(1);
                skip_range = skip_range.add(2);
                skip_start = *skip_range.add(0);
                continue;
            }

            // Compute caseless set.
            if (options & (PARSE_CLASS_TURKISH_UTF | PARSE_CLASS_RESTRICTED_UTF))
                == PARSE_CLASS_TURKISH_UTF
                && UCD_ANY_I(c)
            {
                co = crate::tables::_pcre2_ucd_turkish_dotted_i_caseset
                    + if UCD_DOTTED_I(c) { 0 } else { 3 };
            } else {
                co = UCD_CASESET(c);
                if co != 0
                    && (options & PARSE_CLASS_RESTRICTED_UTF) != 0
                    && crate::tables::_pcre2_ucd_caseless_sets[co as usize] < 128
                {
                    co = 0; // Ignore the caseless set if it's restricted.
                }
            }

            if co != 0 {
                list = crate::tables::_pcre2_ucd_caseless_sets.as_ptr().add(co as usize);
            } else {
                co = UCD_OTHERCASE(c);
                list = tmp.as_ptr();
                tmp[0] = c;
                tmp[1] = NOTACHAR as u32;

                if co != c {
                    tmp[1] = co;
                    tmp[2] = NOTACHAR as u32;
                }
            }
            c += 1;

            // Add characters.
            loop {
                let val = *list;

                let mut skip = false;
                if val < new_start {
                    if val + 1 == new_start {
                        new_start -= 1;
                        skip = true;
                    }
                } else if val > new_end {
                    if val - 1 == new_end {
                        new_end += 1;
                        skip = true;
                    }
                } else {
                    skip = true;
                }

                if !skip {
                    result += 2;
                    if !buffer.is_null() {
                        *buffer.add(0) = val;
                        *buffer.add(1) = val;
                        buffer = buffer.add(2);
                    }
                }

                list = list.add(1);
                if *list == NOTACHAR as u32 {
                    break;
                }
            }
        }

        if !buffer.is_null() {
            *buffer.add(0) = new_start;
            *buffer.add(1) = new_end;
            buffer = buffer.add(2);
            let _ = buffer;
        }
        result
    }
}

/// Add a character list to a buffer.
unsafe fn append_char_list(mut p: *const u32, mut buffer: *mut u32) -> usize {
    unsafe {
        let mut result: usize = 0;

        while *p != NOTACHAR as u32 {
            let mut n = p;
            while *n.add(0) == *n.add(1) - 1 {
                n = n.add(1);
            }

            // PCRE2_ASSERT(*p < 0xffff);

            if !buffer.is_null() {
                *buffer.add(0) = *p;
                *buffer.add(1) = *n;
                buffer = buffer.add(2);
            }

            result += 2;
            p = n.add(1);
        }

        result
    }
}

fn get_highest_char(options: u32) -> u32 {
    // 8-bit mode with SUPPORT_UNICODE.
    GET_MAX_CHAR_VALUE((options & PARSE_CLASS_UTF) != 0)
}

/// Add a negated character list to a buffer.
unsafe fn append_negated_char_list(
    mut p: *const u32,
    options: u32,
    mut buffer: *mut u32,
) -> usize {
    unsafe {
        let mut start: u32 = 0;
        let mut result: usize = 2;

        // PCRE2_ASSERT(*p > 0);

        while *p != NOTACHAR as u32 {
            let mut n = p;
            while *n.add(0) == *n.add(1) - 1 {
                n = n.add(1);
            }

            // PCRE2_ASSERT(*p < 0xffff);

            if !buffer.is_null() {
                *buffer.add(0) = start;
                *buffer.add(1) = *p - 1;
                buffer = buffer.add(2);
            }

            result += 2;
            start = *n + 1;
            p = n.add(1);
        }

        if !buffer.is_null() {
            *buffer.add(0) = start;
            *buffer.add(1) = get_highest_char(options);
            buffer = buffer.add(2);
            let _ = buffer;
        }

        result
    }
}

unsafe fn append_non_ascii_range(options: u32, buffer: *mut u32) -> *mut u32 {
    unsafe {
        if buffer.is_null() {
            return core::ptr::null_mut();
        }

        *buffer.add(0) = 0x100;
        *buffer.add(1) = get_highest_char(options);
        buffer.add(2)
    }
}

unsafe fn parse_class(mut ptr: *mut u32, options: u32, mut buffer: *mut u32) -> usize {
    unsafe {
        let mut total_size: usize = 0;
        let mut size: usize;
        let mut meta_arg: u32;
        let mut start_char: u32;

        loop {
            match META_CODE(*ptr) as i64 {
                x if x == META_ESCAPE => {
                    meta_arg = META_DATA(*ptr);
                    match meta_arg {
                        m if m == ESC_D || m == ESC_W || m == ESC_S => {
                            buffer = append_non_ascii_range(options, buffer);
                            total_size += 2;
                        }
                        m if m == ESC_h => {
                            size = append_char_list(
                                crate::tables::_pcre2_hspace_list.as_ptr(),
                                buffer,
                            );
                            total_size += size;
                            if !buffer.is_null() {
                                buffer = buffer.add(size);
                            }
                        }
                        m if m == ESC_H => {
                            size = append_negated_char_list(
                                crate::tables::_pcre2_hspace_list.as_ptr(),
                                options,
                                buffer,
                            );
                            total_size += size;
                            if !buffer.is_null() {
                                buffer = buffer.add(size);
                            }
                        }
                        m if m == ESC_v => {
                            size = append_char_list(
                                crate::tables::_pcre2_vspace_list.as_ptr(),
                                buffer,
                            );
                            total_size += size;
                            if !buffer.is_null() {
                                buffer = buffer.add(size);
                            }
                        }
                        m if m == ESC_V => {
                            size = append_negated_char_list(
                                crate::tables::_pcre2_vspace_list.as_ptr(),
                                options,
                                buffer,
                            );
                            total_size += size;
                            if !buffer.is_null() {
                                buffer = buffer.add(size);
                            }
                        }
                        m if m == ESC_p || m == ESC_P => {
                            ptr = ptr.add(1);
                            if meta_arg == ESC_p && (*ptr >> 16) == PT_ANY as u32 {
                                if !buffer.is_null() {
                                    *buffer.add(0) = 0;
                                    *buffer.add(1) = get_highest_char(options);
                                    buffer = buffer.add(2);
                                }
                                total_size += 2;
                            }
                        }
                        _ => {}
                    }
                    ptr = ptr.add(1);
                    continue;
                }
                x if x == META_POSIX_NEG => {
                    buffer = append_non_ascii_range(options, buffer);
                    total_size += 2;
                    ptr = ptr.add(2);
                    continue;
                }
                x if x == META_POSIX => {
                    ptr = ptr.add(2);
                    continue;
                }
                x if x == META_BIGVALUE => {
                    // Character literal.
                    ptr = ptr.add(1);
                }
                // CLASS_END_CASES: PCRE2_DEBUG off, so `default`.
                _ => {
                    if (*ptr as i64) >= META_END {
                        return total_size;
                    }
                }
            }

            start_char = *ptr;

            if *ptr.add(1) == META_RANGE_LITERAL as u32
                || *ptr.add(1) == META_RANGE_ESCAPED as u32
            {
                ptr = ptr.add(2);
                // PCRE2_ASSERT(*ptr < META_END || *ptr == META_BIGVALUE);

                if *ptr == META_BIGVALUE as u32 {
                    ptr = ptr.add(1);
                }
            }

            if options & PARSE_CLASS_CASELESS_UTF != 0 {
                let end_ch = *ptr;
                ptr = ptr.add(1);
                size = utf_caseless_extend(start_char, end_ch, options, buffer);
                if !buffer.is_null() {
                    buffer = buffer.add(size);
                }
                total_size += size;
                continue;
            }

            if !buffer.is_null() {
                *buffer.add(0) = start_char;
                *buffer.add(1) = *ptr;
                buffer = buffer.add(2);
            }

            ptr = ptr.add(1);
            total_size += 2;
        }
    }
}

/// Extra `uint32_t` values for storing the lengths of range lists in the worst
/// case.
const CHAR_LIST_EXTRA_SIZE: usize = 3;

/// Starting character values for each character list (8-bit + SUPPORT_UNICODE).
static CHAR_LIST_STARTS: [u32; 3] = [
    XCL_CHAR_LIST_LOW_32_START as u32,
    XCL_CHAR_LIST_HIGH_16_START as u32,
    XCL_CHAR_LIST_LOW_16_START as u32,
];

unsafe fn compile_optimize_class(
    start_ptr: *mut u32,
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
) -> *mut class_ranges {
    unsafe {
        let mut ptr: *mut u32;
        let buffer: *mut u32;
        let mut dst: *mut u32;
        let mut class_options: u32 = 0;
        let mut range_list_size: usize;
        let total_size: usize;
        let mut i: usize;
        let mut tmp1: u32;
        let mut tmp2: u32;

        if options & PCRE2_UTF as u32 != 0 {
            class_options |= PARSE_CLASS_UTF;
        }
        if (options & PCRE2_CASELESS as u32) != 0
            && (options & (PCRE2_UTF as u32 | PCRE2_UCP as u32)) != 0
        {
            class_options |= PARSE_CLASS_CASELESS_UTF;
        }
        if xoptions & PCRE2_EXTRA_CASELESS_RESTRICT as u32 != 0 {
            class_options |= PARSE_CLASS_RESTRICTED_UTF;
        }
        if xoptions & PCRE2_EXTRA_TURKISH_CASING as u32 != 0 {
            class_options |= PARSE_CLASS_TURKISH_UTF;
        }

        // Compute required space for the range.
        range_list_size = parse_class(start_ptr, class_options, core::ptr::null_mut());
        // PCRE2_ASSERT((range_list_size & 0x1) == 0);

        // Allocate buffer. total_size also represents the end of the buffer.
        total_size =
            range_list_size + if range_list_size >= 2 { CHAR_LIST_EXTRA_SIZE } else { 0 };

        let memctl = &raw mut (*(*cb).cx).memctl;
        let cranges = ((*memctl).malloc.unwrap())(
            core::mem::size_of::<class_ranges>() + total_size * core::mem::size_of::<u32>(),
            (*memctl).memory_data,
        ) as *mut class_ranges;

        if cranges.is_null() {
            return core::ptr::null_mut();
        }

        (*cranges).header.next = core::ptr::null_mut();
        (*cranges).range_list_size = range_list_size as u16;
        (*cranges).char_lists_types = 0;
        (*cranges).char_lists_size = 0;
        (*cranges).char_lists_start = 0;

        if range_list_size == 0 {
            return cranges;
        }

        buffer = cranges.add(1) as *mut u32;
        parse_class(start_ptr, class_options, buffer);

        // Using <= instead of == to help static analysis.
        if range_list_size <= 2 {
            return cranges;
        }

        // In-place sorting of ranges.
        i = ((range_list_size >> 2) - 1) << 1;
        loop {
            do_heapify(buffer, range_list_size, i);
            if i == 0 {
                break;
            }
            i -= 2;
        }

        i = range_list_size - 2;
        loop {
            tmp1 = *buffer.add(i);
            tmp2 = *buffer.add(i + 1);
            *buffer.add(i) = *buffer.add(0);
            *buffer.add(i + 1) = *buffer.add(1);
            *buffer.add(0) = tmp1;
            *buffer.add(1) = tmp2;

            do_heapify(buffer, i, 0);
            if i == 0 {
                break;
            }
            i -= 2;
        }

        // Merge ranges whenever possible.
        dst = buffer;
        ptr = buffer.add(2);
        range_list_size -= 2;

        while range_list_size > 0 && *dst.add(1) != !0u32 {
            if *dst.add(1) + 1 < *ptr.add(0) {
                dst = dst.add(2);
                *dst.add(0) = *ptr.add(0);
                *dst.add(1) = *ptr.add(1);
            } else if *dst.add(1) < *ptr.add(1) {
                *dst.add(1) = *ptr.add(1);
            }

            ptr = ptr.add(2);
            range_list_size -= 2;
        }

        // PCRE2_ASSERT(dst[1] <= get_highest_char(class_options));

        // When the number of ranges is less than six, they are not converted
        // to range lists.
        ptr = buffer;
        while ptr < dst && *ptr.add(1) < 0x100 {
            ptr = ptr.add(2);
        }
        if (dst as isize - ptr as isize) / (core::mem::size_of::<u32>() as isize)
            < (2 * (6 - 1))
        {
            (*cranges).range_list_size =
                (dst.add(2).offset_from(buffer)) as u16;
            return cranges;
        }

        // Compute character lists structures.
        let mut char_list_next = CHAR_LIST_STARTS.as_ptr();
        let mut char_list_start = *char_list_next;
        char_list_next = char_list_next.add(1);
        let mut char_list_end: u32 = XCL_CHAR_LIST_LOW_32_END as u32;
        let mut next_char = (buffer.add(total_size)) as *mut u16;

        tmp1 = 0;
        tmp2 = ((CHAR_LIST_STARTS.len() as u32) - 1) * (XCL_TYPE_BIT_LEN as u32);
        let mut range_start = *dst.add(0);
        let mut range_end = *dst.add(1);

        loop {
            if range_start >= char_list_start {
                if range_start == range_end || range_end < char_list_end {
                    tmp1 += 1;
                    next_char = next_char.sub(1);

                    if char_list_start < XCL_CHAR_LIST_LOW_32_START as u32 {
                        *next_char =
                            ((range_end << XCL_CHAR_SHIFT as u32) | XCL_CHAR_END as u32) as u16;
                    } else {
                        next_char = next_char.sub(1); // C: `--next_char` on a uint16_t*
                        (next_char as *mut u32).write_unaligned(
                            (range_end << XCL_CHAR_SHIFT as u32) | XCL_CHAR_END as u32,
                        );
                    }
                }

                if range_start < range_end {
                    if range_start > char_list_start {
                        tmp1 += 1;
                        next_char = next_char.sub(1);

                        if char_list_start < XCL_CHAR_LIST_LOW_32_START as u32 {
                            *next_char = (range_start << XCL_CHAR_SHIFT as u32) as u16;
                        } else {
                            next_char = next_char.sub(1); // C: `--next_char` on a uint16_t*
                            (next_char as *mut u32)
                                .write_unaligned(range_start << XCL_CHAR_SHIFT as u32);
                        }
                    } else {
                        (*cranges).char_lists_types |=
                            ((XCL_BEGIN_WITH_RANGE as u32) << tmp2) as u16;
                    }
                }

                if dst > buffer {
                    dst = dst.sub(2);
                    range_start = *dst.add(0);
                    range_end = *dst.add(1);
                    continue;
                }

                range_start = 0;
                range_end = 0;
            }

            if range_end >= char_list_start {
                // PCRE2_ASSERT(range_start < char_list_start);

                if range_end < char_list_end {
                    tmp1 += 1;
                    next_char = next_char.sub(1);

                    if char_list_start < XCL_CHAR_LIST_LOW_32_START as u32 {
                        *next_char =
                            ((range_end << XCL_CHAR_SHIFT as u32) | XCL_CHAR_END as u32) as u16;
                    } else {
                        next_char = next_char.sub(1); // C: `--next_char` on a uint16_t*
                        (next_char as *mut u32).write_unaligned(
                            (range_end << XCL_CHAR_SHIFT as u32) | XCL_CHAR_END as u32,
                        );
                    }
                }

                (*cranges).char_lists_types |= ((XCL_BEGIN_WITH_RANGE as u32) << tmp2) as u16;
            }

            if tmp1 >= XCL_ITEM_COUNT_MASK as u32 {
                (*cranges).char_lists_types |=
                    ((XCL_ITEM_COUNT_MASK as u32) << tmp2) as u16;
                next_char = next_char.sub(1);

                if char_list_start < XCL_CHAR_LIST_LOW_32_START as u32 {
                    *next_char = tmp1 as u16;
                } else {
                    next_char = next_char.sub(1); // C: `--next_char` on a uint16_t*
                    (next_char as *mut u32).write_unaligned(tmp1);
                }
            } else {
                (*cranges).char_lists_types |= (tmp1 << tmp2) as u16;
            }

            if range_end < XCL_CHAR_LIST_LOW_16_START as u32 || tmp2 == 0 {
                // PCRE2_ASSERT(range_start < XCL_CHAR_LIST_LOW_16_START);
                break;
            }

            // PCRE2_ASSERT((tmp2 % XCL_TYPE_BIT_LEN) == 0);
            char_list_end = char_list_start - 1;
            char_list_start = *char_list_next;
            char_list_next = char_list_next.add(1);
            tmp1 = 0;
            tmp2 -= XCL_TYPE_BIT_LEN as u32;
        }

        if *dst.add(0) < XCL_CHAR_LIST_LOW_16_START as u32 {
            dst = dst.add(2);
        }
        // PCRE2_ASSERT((uint16_t*)dst <= next_char);

        (*cranges).char_lists_size =
            (buffer.add(total_size) as *const u8).offset_from(next_char as *const u8) as usize;
        (*cranges).char_lists_start =
            (next_char as *const u8).offset_from(buffer as *const u8) as usize;
        (*cranges).range_list_size = dst.offset_from(buffer) as u16;
        cranges
    }
}

// ---------------------------------------------------------------------------
// SUPPORT_UNICODE: update_classbits
// ---------------------------------------------------------------------------

/// `PRIV(update_classbits)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_update_classbits_8(
    ptype: u32,
    pdata: u32,
    negated: BOOL,
    classbits: *mut u8,
) {
    unsafe {
        // Update PRIV(xclass) when this function is changed.
        let mut classbits = classbits;
        let mut chartype: u32;
        let mut gentype: u32;
        let mut set_bit: BOOL;

        if ptype == PT_ANY as u32 {
            if negated == FALSE {
                core::ptr::write_bytes(classbits, 0xff, 32);
            }
            return;
        }

        for c in 0u32..256u32 {
            let prop = GET_UCD(c);
            set_bit = FALSE;

            match ptype as i64 {
                x if x == PT_LAMP => {
                    chartype = prop.chartype as u32;
                    set_bit = bool_to(
                        chartype == ucp_Lu || chartype == ucp_Ll || chartype == ucp_Lt,
                    );
                }
                x if x == PT_GC => {
                    set_bit = bool_to(
                        crate::tables::_pcre2_ucp_gentype[prop.chartype as usize] == pdata,
                    );
                }
                x if x == PT_PC => {
                    set_bit = bool_to(prop.chartype as u32 == pdata);
                }
                x if x == PT_SC => {
                    set_bit = bool_to(prop.script as u32 == pdata);
                }
                x if x == PT_SCX => {
                    set_bit = bool_to(
                        prop.script as u32 == pdata
                            || MAPBIT(
                                crate::tables::_pcre2_ucd_script_sets
                                    .as_ptr()
                                    .add(UCD_SCRIPTX_PROP(prop) as usize),
                                pdata,
                            ) != 0,
                    );
                }
                x if x == PT_ALNUM => {
                    gentype = crate::tables::_pcre2_ucp_gentype[prop.chartype as usize];
                    set_bit = bool_to(gentype == ucp_L || gentype == ucp_N);
                }
                x if x == PT_SPACE || x == PT_PXSPACE => {
                    // HSPACE_BYTE_CASES / VSPACE_BYTE_CASES.
                    set_bit = if is_hspace_byte(c) || is_vspace_byte(c) {
                        TRUE
                    } else {
                        bool_to(
                            crate::tables::_pcre2_ucp_gentype[prop.chartype as usize] == ucp_Z,
                        )
                    };
                }
                x if x == PT_WORD => {
                    chartype = prop.chartype as u32;
                    gentype = crate::tables::_pcre2_ucp_gentype[chartype as usize];
                    set_bit = bool_to(
                        gentype == ucp_L
                            || gentype == ucp_N
                            || chartype == ucp_Mn
                            || chartype == ucp_Pc,
                    );
                }
                x if x == PT_UCNC => {
                    set_bit = bool_to(
                        c == CHAR_DOLLAR_SIGN
                            || c == CHAR_COMMERCIAL_AT
                            || c == CHAR_GRAVE_ACCENT
                            || c >= 0xa0,
                    );
                }
                x if x == PT_BIDICL => {
                    set_bit = bool_to(UCD_BIDICLASS_PROP(prop) == pdata);
                }
                x if x == PT_BOOL => {
                    set_bit = bool_to(
                        MAPBIT(
                            crate::tables::_pcre2_ucd_boolprop_sets
                                .as_ptr()
                                .add(UCD_BPROPS_PROP(prop) as usize),
                            pdata,
                        ) != 0,
                    );
                }
                x if x == PT_PXGRAPH => {
                    chartype = prop.chartype as u32;
                    gentype = crate::tables::_pcre2_ucp_gentype[chartype as usize];
                    set_bit = bool_to(
                        gentype != ucp_Z && (gentype != ucp_C || chartype == ucp_Cf),
                    );
                }
                x if x == PT_PXPRINT => {
                    chartype = prop.chartype as u32;
                    set_bit = bool_to(
                        chartype != ucp_Zl
                            && chartype != ucp_Zp
                            && (crate::tables::_pcre2_ucp_gentype[chartype as usize] != ucp_C
                                || chartype == ucp_Cf),
                    );
                }
                x if x == PT_PXPUNCT => {
                    gentype = crate::tables::_pcre2_ucp_gentype[prop.chartype as usize];
                    set_bit = bool_to(gentype == ucp_P || (c < 128 && gentype == ucp_S));
                }
                // default: PCRE2_ASSERT(ptype == PT_PXXDIGIT);
                _ => {
                    set_bit = bool_to(
                        (c >= CHAR_0 && c <= CHAR_9)
                            || (c >= CHAR_A && c <= CHAR_F)
                            || (c >= CHAR_a && c <= CHAR_f),
                    );
                }
            }

            if negated != FALSE {
                set_bit = if set_bit == FALSE { TRUE } else { FALSE };
            }
            if set_bit != FALSE {
                *classbits |= 1u8 << (c & 0x7);
            }
            if (c & 0x7) == 0x7 {
                classbits = classbits.add(1);
            }
        }
    }
}

/// `HSPACE_BYTE_CASES` — horizontal white space with a code point < 256.
#[inline(always)]
fn is_hspace_byte(c: u32) -> bool {
    matches!(c, 0x09 | 0x20 | 0xa0)
}

/// `VSPACE_BYTE_CASES` — vertical white space with a code point < 256.
#[inline(always)]
fn is_vspace_byte(c: u32) -> bool {
    matches!(c, 0x0a | 0x0b | 0x0c | 0x0d | 0x85)
}

#[inline(always)]
fn bool_to(b: bool) -> BOOL {
    if b { TRUE } else { FALSE }
}

// ---------------------------------------------------------------------------
// add_to_class and list helpers
// ---------------------------------------------------------------------------

/// Sets the overall range for characters < 256; also handles non-utf case
/// folding.
unsafe fn add_to_class(
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
    start: u32,
    end: u32,
) {
    unsafe {
        let classbits = (*cb).classbits.classbits.as_mut_ptr();
        let mut c: u32;
        let mut byte_start: u32;
        let mut byte_end: u32;
        let classbits_end: u32 = if end <= 0xff { end } else { 0xff };
        let fcc = (*cb).fcc;

        if (options & PCRE2_CASELESS as u32) != 0 {
            if (options & (PCRE2_UTF as u32 | PCRE2_UCP as u32)) != 0 {
                let turkish_i = (xoptions
                    & (PCRE2_EXTRA_TURKISH_CASING as u32 | PCRE2_EXTRA_CASELESS_RESTRICT as u32))
                    == PCRE2_EXTRA_TURKISH_CASING as u32;
                if start < 128 {
                    let lo_end = if classbits_end < 127 { classbits_end } else { 127 };
                    c = start;
                    while c <= lo_end {
                        if !(turkish_i && UCD_ANY_I(c)) {
                            SETBIT(classbits, *fcc.add(c as usize) as u32);
                        }
                        c += 1;
                    }
                }
                if classbits_end >= 128 {
                    let hi_start = if start > 128 { start } else { 128 };
                    c = hi_start;
                    while c <= classbits_end {
                        let co = UCD_OTHERCASE(c);
                        if co <= 0xff {
                            SETBIT(classbits, co);
                        }
                        c += 1;
                    }
                }
            } else {
                // Not UTF mode.
                c = start;
                while c <= classbits_end {
                    SETBIT(classbits, *fcc.add(c as usize) as u32);
                    c += 1;
                }
            }
        }

        // Use the bitmap for characters < 256.
        byte_start = (start + 7) >> 3;
        byte_end = (classbits_end + 1) >> 3;

        if byte_start >= byte_end {
            c = start;
            while c <= classbits_end {
                // Regardless of start, c will always be <= 255.
                SETBIT(classbits, c);
                c += 1;
            }
            return;
        }

        c = byte_start;
        while c < byte_end {
            *classbits.add(c as usize) = 0xff;
            c += 1;
        }

        byte_start <<= 3;
        byte_end <<= 3;

        c = start;
        while c < byte_start {
            SETBIT(classbits, c);
            c += 1;
        }

        c = byte_end;
        while c <= classbits_end {
            SETBIT(classbits, c);
            c += 1;
        }
    }
}

/// Adds a list of horizontal or vertical whitespace characters to a class.
unsafe fn add_list_to_class(
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
    mut p: *const u32,
) {
    unsafe {
        while *p.add(0) < 256 {
            let mut n: usize = 0;

            while *p.add(n + 1) == *p.add(0) + (n as u32) + 1 {
                n += 1;
            }
            add_to_class(options, xoptions, cb, *p.add(0), *p.add(n));

            p = p.add(n + 1);
        }
    }
}

/// Adds the complement of a list of horizontal or vertical whitespace to a
/// class.
unsafe fn add_not_list_to_class(
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
    mut p: *const u32,
) {
    unsafe {
        if *p.add(0) > 0 {
            add_to_class(options, xoptions, cb, 0, *p.add(0) - 1);
        }
        while *p.add(0) < 256 {
            while *p.add(1) == *p.add(0) + 1 {
                p = p.add(1);
            }
            add_to_class(
                options,
                xoptions,
                cb,
                *p.add(0) + 1,
                if *p.add(1) > 255 { 255 } else { *p.add(1) - 1 },
            );
            p = p.add(1);
        }
    }
}

// ---------------------------------------------------------------------------
// compile_class_not_nested — main entry point to compile a character class
// ---------------------------------------------------------------------------

/// `PRIV(compile_class_not_nested)`. Consumes a "leaf" (a set of characters
/// that will become a single `OP_CLASS`, `OP_NCLASS`, `OP_XCLASS`, or
/// `OP_ALLANY`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_class_not_nested_8(
    options: u32,
    xoptions: u32,
    start_ptr: *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    negate_class: BOOL,
    has_bitmap: *mut BOOL,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
) -> *mut u32 {
    unsafe {
        let mut pptr: *mut u32 = start_ptr;
        let mut code: *mut PCRE2_UCHAR = *pcode;
        let mut should_flip_negation: BOOL;
        let cbits = (*cb).cbits;
        let classbits = (*cb).classbits.classbits.as_mut_ptr();

        let utf: BOOL = bool_to((options & PCRE2_UTF as u32) != 0);

        // Helper variables for OP_XCLASS opcode (for characters > 255).
        let mut xclass_props: u32;
        let mut class_uchardata: *mut PCRE2_UCHAR;
        let mut cranges: *mut class_ranges;

        should_flip_negation = FALSE;

        // XClass will be used when characters > 255 might match.
        xclass_props = 0;
        cranges = core::ptr::null_mut();

        if utf != FALSE {
            if !lengthptr.is_null() {
                cranges = compile_optimize_class(pptr, options, xoptions, cb);

                if cranges.is_null() {
                    *errorcodeptr = ERR21;
                    return core::ptr::null_mut();
                }

                // Caching the pre-processed character ranges.
                if !(*cb).last_data.is_null() {
                    (*(*cb).last_data).next = &raw mut (*cranges).header;
                } else {
                    (*cb).first_data = &raw mut (*cranges).header;
                }

                (*cb).last_data = &raw mut (*cranges).header;
            } else {
                // Reuse the pre-processed character ranges.
                cranges = (*cb).first_data as *mut class_ranges;
                // PCRE2_ASSERT(cranges != NULL ...);
                (*cb).first_data = (*cranges).header.next;
            }

            if (*cranges).range_list_size > 0 {
                let ranges = cranges.add(1) as *const u32;

                if *ranges.add(0) <= 255 {
                    xclass_props |= XCLASS_HAS_8BIT_CHARS;
                }

                if *ranges.add((*cranges).range_list_size as usize - 1)
                    == GET_MAX_CHAR_VALUE(utf != FALSE)
                    && *ranges.add((*cranges).range_list_size as usize - 2) <= 256
                {
                    xclass_props |= XCLASS_HIGH_ANY;
                }
            }
        }

        class_uchardata = code.add(LINK_SIZE_U + 2); // For XCLASS items.

        // Initialize the 256-bit (32-byte) bit map to all zeros.
        core::ptr::write_bytes(classbits, 0, 32);

        // Process items until end_ptr is reached.
        'main_loop: loop {
            let mut meta = *pptr;
            pptr = pptr.add(1);
            let local_negate: BOOL;
            let mut posix_class: c_int;
            let mut taboffset: c_int;
            let mut tabopt: c_int;
            let mut pbits: class_bits_storage;
            let escape: u32;
            let c: u32;

            // Handle POSIX classes such as [:alpha:] etc.
            match META_CODE(meta) as i64 {
                x if x == META_POSIX || x == META_POSIX_NEG => {
                    local_negate = bool_to(meta == META_POSIX_NEG as u32);
                    posix_class = *pptr as c_int;
                    pptr = pptr.add(1);

                    if local_negate != FALSE {
                        should_flip_negation = TRUE;
                    }

                    if (options & PCRE2_CASELESS as u32) != 0 && posix_class <= 2 {
                        posix_class = 0;
                    }

                    if (options & PCRE2_UCP as u32) != 0
                        && (xoptions & PCRE2_EXTRA_ASCII_POSIX as u32) == 0
                    {
                        match posix_class as i64 {
                            p if p == PC_GRAPH || p == PC_PRINT || p == PC_PUNCT => {
                                let ptype: u32 = if posix_class as i64 == PC_GRAPH {
                                    PT_PXGRAPH as u32
                                } else if posix_class as i64 == PC_PRINT {
                                    PT_PXPRINT as u32
                                } else {
                                    PT_PXPUNCT as u32
                                };

                                _pcre2_update_classbits_8(ptype, 0, local_negate, classbits);

                                if (xclass_props & XCLASS_HIGH_ANY) == 0 {
                                    if !lengthptr.is_null() {
                                        *lengthptr += 3;
                                    } else {
                                        *class_uchardata = if local_negate != FALSE {
                                            XCL_NOTPROP as u8
                                        } else {
                                            XCL_PROP as u8
                                        };
                                        class_uchardata = class_uchardata.add(1);
                                        *class_uchardata = ptype as u8;
                                        class_uchardata = class_uchardata.add(1);
                                        *class_uchardata = 0;
                                        class_uchardata = class_uchardata.add(1);
                                    }
                                    xclass_props |= XCLASS_REQUIRED | XCLASS_HAS_PROPS;
                                }
                                continue 'main_loop;
                            }
                            _ => {}
                        }
                    }

                    // Build the bit map for the POSIX class in local store.
                    posix_class *= 3;

                    // Copy in the first table (always present).
                    pbits = core::mem::zeroed();
                    core::ptr::copy_nonoverlapping(
                        cbits.add(
                            crate::compile_h::_pcre2_posix_class_maps8[posix_class as usize]
                                as usize,
                        ),
                        pbits.classbits.as_mut_ptr(),
                        32,
                    );

                    // If there is a second table, add or remove it as required.
                    taboffset =
                        crate::compile_h::_pcre2_posix_class_maps8[posix_class as usize + 1];
                    tabopt =
                        crate::compile_h::_pcre2_posix_class_maps8[posix_class as usize + 2];

                    if taboffset >= 0 {
                        if tabopt >= 0 {
                            for i in 0..32usize {
                                pbits.classbits[i] |= *cbits.add(i + taboffset as usize);
                            }
                        } else {
                            for i in 0..32usize {
                                pbits.classbits[i] &= !*cbits.add(i + taboffset as usize);
                            }
                        }
                    }

                    if tabopt < 0 {
                        tabopt = -tabopt;
                    }
                    if tabopt == 1 {
                        pbits.classbits[1] &= !0x3c;
                    } else if tabopt == 2 {
                        pbits.classbits[11] &= 0x7f;
                    }

                    // Add the POSIX table or its complement into the main table.
                    {
                        let classwords = (*cb).classbits.classwords.as_mut_ptr();
                        if local_negate != FALSE {
                            for i in 0..8usize {
                                *classwords.add(i) |= !pbits.classwords[i];
                            }
                        } else {
                            for i in 0..8usize {
                                *classwords.add(i) |= pbits.classwords[i];
                            }
                        }
                    }

                    // Every class contains at least one < 256 character.
                    xclass_props |= XCLASS_HAS_8BIT_CHARS;
                    continue 'main_loop; // End of POSIX handling.
                }

                x if x == META_BIGVALUE => {
                    meta = *pptr;
                    pptr = pptr.add(1);
                    // Fall through to literal handling below.
                }

                x if x == META_ESCAPE => {
                    escape = META_DATA(meta);

                    match escape {
                        e if e == ESC_d => {
                            for i in 0..32usize {
                                *classbits.add(i) |= *cbits.add(i + cbit_digit as usize);
                            }
                        }
                        e if e == ESC_D => {
                            should_flip_negation = TRUE;
                            for i in 0..32usize {
                                *classbits.add(i) |= !*cbits.add(i + cbit_digit as usize);
                            }
                        }
                        e if e == ESC_w => {
                            for i in 0..32usize {
                                *classbits.add(i) |= *cbits.add(i + cbit_word as usize);
                            }
                        }
                        e if e == ESC_W => {
                            should_flip_negation = TRUE;
                            for i in 0..32usize {
                                *classbits.add(i) |= !*cbits.add(i + cbit_word as usize);
                            }
                        }
                        e if e == ESC_s => {
                            for i in 0..32usize {
                                *classbits.add(i) |= *cbits.add(i + cbit_space as usize);
                            }
                        }
                        e if e == ESC_S => {
                            should_flip_negation = TRUE;
                            for i in 0..32usize {
                                *classbits.add(i) |= !*cbits.add(i + cbit_space as usize);
                            }
                        }
                        e if e == ESC_h => {
                            if cranges.is_null() {
                                add_list_to_class(
                                    options & !(PCRE2_CASELESS as u32),
                                    xoptions,
                                    cb,
                                    crate::tables::_pcre2_hspace_list.as_ptr(),
                                );
                            }
                        }
                        e if e == ESC_H => {
                            if cranges.is_null() {
                                add_not_list_to_class(
                                    options & !(PCRE2_CASELESS as u32),
                                    xoptions,
                                    cb,
                                    crate::tables::_pcre2_hspace_list.as_ptr(),
                                );
                            }
                        }
                        e if e == ESC_v => {
                            if cranges.is_null() {
                                add_list_to_class(
                                    options & !(PCRE2_CASELESS as u32),
                                    xoptions,
                                    cb,
                                    crate::tables::_pcre2_vspace_list.as_ptr(),
                                );
                            }
                        }
                        e if e == ESC_V => {
                            if cranges.is_null() {
                                add_not_list_to_class(
                                    options & !(PCRE2_CASELESS as u32),
                                    xoptions,
                                    cb,
                                    crate::tables::_pcre2_vspace_list.as_ptr(),
                                );
                            }
                        }
                        e if e == ESC_p || e == ESC_P => {
                            let ptype: u32 = *pptr >> 16;
                            let pdata: u32 = *pptr & 0xffff;
                            pptr = pptr.add(1);

                            // The "Any" is processed by update_classbits().
                            if ptype == PT_ANY as u32 {
                                if utf == FALSE && escape == ESC_p {
                                    core::ptr::write_bytes(classbits, 0xff, 32);
                                }
                                continue 'main_loop;
                            }

                            _pcre2_update_classbits_8(
                                ptype,
                                pdata,
                                bool_to(escape == ESC_P),
                                classbits,
                            );

                            if (xclass_props & XCLASS_HIGH_ANY) == 0 {
                                if !lengthptr.is_null() {
                                    *lengthptr += 3;
                                } else {
                                    *class_uchardata = if escape == ESC_p {
                                        XCL_PROP as u8
                                    } else {
                                        XCL_NOTPROP as u8
                                    };
                                    class_uchardata = class_uchardata.add(1);
                                    *class_uchardata = ptype as u8;
                                    class_uchardata = class_uchardata.add(1);
                                    *class_uchardata = pdata as u8;
                                    class_uchardata = class_uchardata.add(1);
                                }
                                xclass_props |= XCLASS_REQUIRED | XCLASS_HAS_PROPS;
                            }
                            continue 'main_loop;
                        }
                        _ => {}
                    }

                    // Every non-property class contains at least one < 256 char.
                    xclass_props |= XCLASS_HAS_8BIT_CHARS;
                    continue 'main_loop; // End handling \d-type escapes.
                }

                // CLASS_END_CASES: PCRE2_DEBUG off, so `default`.
                _ => {
                    // Literals.
                    if (meta as i64) < META_END {
                        // break — fall through to literal handling.
                    } else {
                        // Non-literals: end of class contents.
                        break 'main_loop;
                    }
                }
            }

            // A literal character may be followed by a range meta.
            c = meta;

            // Remember if \r or \n were explicitly used.
            if c == CHAR_CR || c == CHAR_NL_C {
                (*cb).external_flags |= PCRE2_HASCRORLF as u32;
            }

            // Process a character range.
            if *pptr == META_RANGE_LITERAL as u32 || *pptr == META_RANGE_ESCAPED as u32 {
                let mut d: u32;

                pptr = pptr.add(1);
                d = *pptr;
                pptr = pptr.add(1);
                if d == META_BIGVALUE as u32 {
                    d = *pptr;
                    pptr = pptr.add(1);
                }

                if d == CHAR_CR || d == CHAR_NL_C {
                    (*cb).external_flags |= PCRE2_HASCRORLF as u32;
                }

                if cranges.is_null() {
                    xclass_props |= XCLASS_HAS_8BIT_CHARS;
                    add_to_class(options, xoptions, cb, c, d);
                }
                continue 'main_loop;
            } // End of range handling.

            // Character ranges are ignored when class_ranges is present.
            if cranges.is_null() {
                xclass_props |= XCLASS_HAS_8BIT_CHARS;
                // Handle a single character.
                add_to_class(options, xoptions, cb, meta, meta);
            }
        } // End of main class-processing loop.

        // END_PROCESSING:

        // PCRE2_ASSERT((xclass_props & XCLASS_HAS_PROPS) == 0 || ...);

        if !cranges.is_null() {
            let mut range = cranges.add(1) as *mut u32;
            let end = range.add((*cranges).range_list_size as usize);

            while range < end && *range.add(0) < 256 {
                add_to_class(
                    if (options & (PCRE2_UTF as u32 | PCRE2_UCP as u32)) != 0 {
                        options & !(PCRE2_CASELESS as u32)
                    } else {
                        options
                    },
                    xoptions,
                    cb,
                    *range.add(0),
                    *range.add(1),
                );

                if *range.add(1) > 255 {
                    break;
                }
                range = range.add(2);
            }

            if (*cranges).char_lists_size > 0 {
                // The cranges structure is still used and freed later.
                xclass_props |= XCLASS_REQUIRED | XCLASS_HAS_CHAR_LISTS;
            } else {
                if (xclass_props & XCLASS_HIGH_ANY) != 0 {
                    should_flip_negation = TRUE;
                    range = end;
                }

                while range < end {
                    let mut range_start = *range.add(0);
                    let range_end = *range.add(1);

                    range = range.add(2);
                    xclass_props |= XCLASS_REQUIRED;

                    if range_start < 256 {
                        range_start = 256;
                    }

                    if !lengthptr.is_null() {
                        if utf != FALSE {
                            *lengthptr += 1;

                            if range_start < range_end {
                                *lengthptr += crate::ord2utf::_pcre2_ord2utf_8(
                                    range_start,
                                    class_uchardata,
                                ) as usize;
                            }

                            *lengthptr += crate::ord2utf::_pcre2_ord2utf_8(
                                range_end,
                                class_uchardata,
                            ) as usize;
                            continue;
                        }

                        *lengthptr += if range_start < range_end { 3 } else { 2 };
                        continue;
                    }

                    if utf != FALSE {
                        if range_start < range_end {
                            *class_uchardata = XCL_RANGE as u8;
                            class_uchardata = class_uchardata.add(1);
                            class_uchardata = class_uchardata.add(
                                crate::ord2utf::_pcre2_ord2utf_8(range_start, class_uchardata)
                                    as usize,
                            );
                        } else {
                            *class_uchardata = XCL_SINGLE as u8;
                            class_uchardata = class_uchardata.add(1);
                        }

                        class_uchardata = class_uchardata.add(
                            crate::ord2utf::_pcre2_ord2utf_8(range_end, class_uchardata) as usize,
                        );
                        continue;
                    }
                    // Without UTF support, no wide chars can exist in 8-bit mode.
                }

                if lengthptr.is_null() {
                    let memctl = &raw mut (*(*cb).cx).memctl;
                    ((*memctl).free.unwrap())(cranges as *mut c_void, (*memctl).memory_data);
                }
            }
        }

        // Extended class (OP_XCLASS) construction.
        if (xclass_props & XCLASS_REQUIRED) != 0 {
            let previous: *mut PCRE2_UCHAR = code;

            if (xclass_props & XCLASS_HAS_CHAR_LISTS) == 0 {
                *class_uchardata = XCL_END as u8; // Marks the end of extra data.
                class_uchardata = class_uchardata.add(1);
            }
            *code = OP_XCLASS as u8;
            code = code.add(1);
            code = code.add(LINK_SIZE_U);
            *code = if negate_class != FALSE { XCL_NOT as u8 } else { 0 };
            if (xclass_props & XCLASS_HAS_PROPS) != 0 {
                *code |= XCL_HASPROP as u8;
            }

            if (xclass_props & XCLASS_HAS_8BIT_CHARS) != 0 || !has_bitmap.is_null() {
                if negate_class != FALSE {
                    let classwords = (*cb).classbits.classwords.as_mut_ptr();
                    for i in 0..8usize {
                        *classwords.add(i) = !*classwords.add(i);
                    }
                }

                if has_bitmap.is_null() {
                    *code |= XCL_MAP as u8;
                    code = code.add(1);
                    core::ptr::copy(
                        code,
                        code.add(32 / core::mem::size_of::<PCRE2_UCHAR>()),
                        CU2BYTES(class_uchardata.offset_from(code) as usize),
                    );
                    core::ptr::copy_nonoverlapping(classbits, code, 32);
                    code = class_uchardata.add(32 / core::mem::size_of::<PCRE2_UCHAR>());
                } else {
                    code = code.add(1);
                    code = class_uchardata;
                    if (xclass_props & XCLASS_HAS_8BIT_CHARS) != 0 {
                        *has_bitmap = TRUE;
                    }
                }
            } else {
                code = code.add(1);
                code = class_uchardata;
            }

            if (xclass_props & XCLASS_HAS_CHAR_LISTS) != 0 {
                let mut char_lists_size = (*cranges).char_lists_size;
                // PCRE2_ASSERT((char_lists_size & 0x1) == 0 && ...);

                if !lengthptr.is_null() {
                    char_lists_size =
                        CLIST_ALIGN_TO(char_lists_size, core::mem::size_of::<u32>());

                    *lengthptr += 2 + LINK_SIZE_U;

                    (*cb).char_lists_size += char_lists_size;

                    char_lists_size /= core::mem::size_of::<PCRE2_UCHAR>();

                    if *lengthptr > MAX_PATTERN_SIZE_U
                        || MAX_PATTERN_SIZE_U - *lengthptr < char_lists_size
                    {
                        *errorcodeptr = ERR20; // Pattern is too large.
                        return core::ptr::null_mut();
                    }
                } else {
                    // PCRE2_ASSERT(cranges->char_lists_types <= XCL_TYPE_MASK);
                    // Encode as high / low bytes.
                    *code.add(0) =
                        (XCL_LIST_VAL | ((*cranges).char_lists_types as u32 >> 8)) as u8;
                    *code.add(1) = (*cranges).char_lists_types as u8;
                    code = code.add(2);

                    (*cb).char_lists_size += char_lists_size;
                    let data = ((*cb).start_code as *mut u8).sub((*cb).char_lists_size);

                    core::ptr::copy_nonoverlapping(
                        (cranges.add(1) as *const u8).add((*cranges).char_lists_start),
                        data,
                        char_lists_size,
                    );

                    char_lists_size = (*cb).char_lists_size;
                    PUT(code, 0, (char_lists_size >> 1) as i32);
                    code = code.add(LINK_SIZE_U);

                    if (char_lists_size & 0x2) != 0 {
                        *(data as *mut u16).sub(1) = 0xdead;
                    }

                    (*cb).char_lists_size =
                        CLIST_ALIGN_TO(char_lists_size, core::mem::size_of::<u32>());

                    let memctl = &raw mut (*(*cb).cx).memctl;
                    ((*memctl).free.unwrap())(cranges as *mut c_void, (*memctl).memory_data);
                }
            }

            // Now fill in the complete length of the item.
            PUT(previous, 1, code.offset_from(previous) as i32);
            *pcode = code;
            return pptr.sub(1); // DONE
        }

        // OP_CLASS / OP_NCLASS / OP_ALLANY.
        if negate_class != FALSE {
            let classwords = (*cb).classbits.classwords.as_mut_ptr();
            for i in 0..8usize {
                *classwords.add(i) = !*classwords.add(i);
            }
        }

        if (SELECT_VALUE8((utf == FALSE) as i32, 0) != 0
            || (negate_class != should_flip_negation))
            && (*cb).classbits.classwords[0] == !0u32
        {
            let classwords = (*cb).classbits.classwords.as_ptr();
            let mut i = 0usize;
            while i < 8 {
                if *classwords.add(i) != !0u32 {
                    break;
                }
                i += 1;
            }

            if i == 8 {
                *code = OP_ALLANY as u8;
                code = code.add(1);
                *pcode = code;
                return pptr.sub(1); // DONE
            }
        }

        *code = if negate_class == should_flip_negation {
            OP_CLASS as u8
        } else {
            OP_NCLASS as u8
        };
        code = code.add(1);
        core::ptr::copy_nonoverlapping(classbits, code, 32);
        code = code.add(32 / core::mem::size_of::<PCRE2_UCHAR>());

        // DONE:
        *pcode = code;
        pptr.sub(1)
    }
}

// ===========================================================================
// ECLASS-compiling functions (leafmost at top).
// ===========================================================================

/// Folds one operand using the negation operator.
unsafe fn fold_negation(
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
    preserve_classbits: BOOL,
) {
    unsafe {
        if (*pop_info).op_single_type == 0 {
            if !lengthptr.is_null() {
                *lengthptr += 1;
            } else {
                *(*pop_info).code_start.add((*pop_info).length) = ECL_NOT as u8;
            }
            (*pop_info).length += 1;
        } else if (*pop_info).op_single_type as i64 == ECL_ANY
            || (*pop_info).op_single_type as i64 == ECL_NONE
        {
            (*pop_info).op_single_type = if (*pop_info).op_single_type as i64 == ECL_NONE {
                ECL_ANY as u8
            } else {
                ECL_NONE as u8
            };
            if lengthptr.is_null() {
                *(*pop_info).code_start = (*pop_info).op_single_type;
            }
        } else {
            // PCRE2_ASSERT(op_single_type == ECL_XCLASS && ...);
            if lengthptr.is_null() {
                *(*pop_info).code_start.add(1 + LINK_SIZE_U) ^= XCL_NOT as u8;
            }
        }

        if preserve_classbits == FALSE {
            for i in 0..8usize {
                (*pop_info).bits.classwords[i] = !(*pop_info).bits.classwords[i];
            }
        }
    }
}

/// Folds together two operands using a binary operator.
unsafe fn fold_binary(
    op: c_int,
    lhs_op_info: *mut eclass_op_info,
    rhs_op_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) {
    unsafe {
        match op as i64 {
            o if o == ECL_AND => {
                if (*rhs_op_info).op_single_type as i64 == ECL_ANY {
                    // no-op: drop the RHS
                } else if (*lhs_op_info).op_single_type as i64 == ECL_ANY {
                    if lengthptr.is_null() {
                        core::ptr::copy(
                            (*rhs_op_info).code_start,
                            (*lhs_op_info).code_start,
                            CU2BYTES((*rhs_op_info).length),
                        );
                    }
                    (*lhs_op_info).length = (*rhs_op_info).length;
                    (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
                } else if (*rhs_op_info).op_single_type as i64 == ECL_NONE {
                    if lengthptr.is_null() {
                        *(*lhs_op_info).code_start.add(0) = ECL_NONE as u8;
                    }
                    (*lhs_op_info).length = 1;
                    (*lhs_op_info).op_single_type = ECL_NONE as u8;
                } else if (*lhs_op_info).op_single_type as i64 == ECL_NONE {
                    // the result is ECL_NONE: drop the RHS
                } else {
                    if !lengthptr.is_null() {
                        *lengthptr += 1;
                    } else {
                        *(*rhs_op_info).code_start.add((*rhs_op_info).length) = ECL_AND as u8;
                    }
                    (*lhs_op_info).length += (*rhs_op_info).length + 1;
                    (*lhs_op_info).op_single_type = 0;
                }

                for i in 0..8usize {
                    (*lhs_op_info).bits.classwords[i] &= (*rhs_op_info).bits.classwords[i];
                }
            }

            o if o == ECL_OR => {
                if (*rhs_op_info).op_single_type as i64 == ECL_NONE {
                    // no-op: drop the RHS
                } else if (*lhs_op_info).op_single_type as i64 == ECL_NONE {
                    if lengthptr.is_null() {
                        core::ptr::copy(
                            (*rhs_op_info).code_start,
                            (*lhs_op_info).code_start,
                            CU2BYTES((*rhs_op_info).length),
                        );
                    }
                    (*lhs_op_info).length = (*rhs_op_info).length;
                    (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
                } else if (*rhs_op_info).op_single_type as i64 == ECL_ANY {
                    if lengthptr.is_null() {
                        *(*lhs_op_info).code_start.add(0) = ECL_ANY as u8;
                    }
                    (*lhs_op_info).length = 1;
                    (*lhs_op_info).op_single_type = ECL_ANY as u8;
                } else if (*lhs_op_info).op_single_type as i64 == ECL_ANY {
                    // the result is ECL_ANY: drop the RHS
                } else {
                    if !lengthptr.is_null() {
                        *lengthptr += 1;
                    } else {
                        *(*rhs_op_info).code_start.add((*rhs_op_info).length) = ECL_OR as u8;
                    }
                    (*lhs_op_info).length += (*rhs_op_info).length + 1;
                    (*lhs_op_info).op_single_type = 0;
                }

                for i in 0..8usize {
                    (*lhs_op_info).bits.classwords[i] |= (*rhs_op_info).bits.classwords[i];
                }
            }

            o if o == ECL_XOR => {
                if (*rhs_op_info).op_single_type as i64 == ECL_NONE {
                    // no-op: drop the RHS
                } else if (*lhs_op_info).op_single_type as i64 == ECL_NONE {
                    if lengthptr.is_null() {
                        core::ptr::copy(
                            (*rhs_op_info).code_start,
                            (*lhs_op_info).code_start,
                            CU2BYTES((*rhs_op_info).length),
                        );
                    }
                    (*lhs_op_info).length = (*rhs_op_info).length;
                    (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
                } else if (*rhs_op_info).op_single_type as i64 == ECL_ANY {
                    // the result is !LHS: fold in the negation, drop the RHS
                    fold_negation(lhs_op_info, lengthptr, TRUE);
                } else if (*lhs_op_info).op_single_type as i64 == ECL_ANY {
                    if lengthptr.is_null() {
                        core::ptr::copy(
                            (*rhs_op_info).code_start,
                            (*lhs_op_info).code_start,
                            CU2BYTES((*rhs_op_info).length),
                        );
                    }
                    (*lhs_op_info).length = (*rhs_op_info).length;
                    (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;

                    fold_negation(lhs_op_info, lengthptr, TRUE);
                } else {
                    if !lengthptr.is_null() {
                        *lengthptr += 1;
                    } else {
                        *(*rhs_op_info).code_start.add((*rhs_op_info).length) = ECL_XOR as u8;
                    }
                    (*lhs_op_info).length += (*rhs_op_info).length + 1;
                    (*lhs_op_info).op_single_type = 0;
                }

                for i in 0..8usize {
                    (*lhs_op_info).bits.classwords[i] ^= (*rhs_op_info).bits.classwords[i];
                }
            }

            _ => {
                // PCRE2_DEBUG_UNREACHABLE();
            }
        }
    }
}

/// Consumes a group of implicitly-unioned class elements (characters, ranges,
/// properties, or nested classes).
unsafe fn compile_class_operand(
    context: *mut eclass_context,
    negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    unsafe {
        let mut ptr: *mut u32 = *pptr;
        let mut prev_ptr: *mut u32;
        let mut code: *mut PCRE2_UCHAR = *pcode;
        let code_start: *mut PCRE2_UCHAR = code;
        let prev_length: PCRE2_SIZE = if !lengthptr.is_null() { *lengthptr } else { 0 };
        let extra_length: PCRE2_SIZE;
        let meta = META_CODE(*ptr) as i64;

        let mut done = false;

        match meta {
            m if m == META_CLASS_EMPTY_NOT || m == META_CLASS_EMPTY => {
                ptr = ptr.add(1);
                (*pop_info).length = 1;
                if (meta == META_CLASS_EMPTY) == (negated != FALSE) {
                    *code = ECL_ANY as u8;
                    (*pop_info).op_single_type = ECL_ANY as u8;
                    code = code.add(1);
                    core::ptr::write_bytes((*pop_info).bits.classbits.as_mut_ptr(), 0xff, 32);
                } else {
                    *code = ECL_NONE as u8;
                    (*pop_info).op_single_type = ECL_NONE as u8;
                    code = code.add(1);
                    core::ptr::write_bytes((*pop_info).bits.classbits.as_mut_ptr(), 0, 32);
                }
            }

            _ => {
                let mut fallthrough_to_default = true;

                if meta == META_CLASS || meta == META_CLASS_NOT {
                    if (*ptr & CLASS_IS_ECLASS as u32) != 0 {
                        if compile_eclass_nested(
                            context,
                            negated,
                            &mut ptr,
                            &mut code,
                            pop_info,
                            lengthptr,
                        ) == FALSE
                        {
                            return FALSE;
                        }

                        // PCRE2_ASSERT(*ptr == META_CLASS_END);
                        ptr = ptr.add(1);
                        done = true;
                        fallthrough_to_default = false;
                    } else {
                        ptr = ptr.add(1);
                        // Fall through to default.
                    }
                }

                if fallthrough_to_default {
                    // default: scan forward characters, ranges, and properties.
                    prev_ptr = ptr;
                    ptr = _pcre2_compile_class_not_nested_8(
                        (*context).options,
                        (*context).xoptions,
                        ptr,
                        &mut code,
                        bool_to((meta != META_CLASS_NOT) == (negated != FALSE)),
                        &mut (*context).needs_bitmap,
                        (*context).errorcodeptr,
                        (*context).cb,
                        lengthptr,
                    );
                    if ptr.is_null() {
                        return FALSE;
                    }

                    if ptr <= prev_ptr {
                        return FALSE;
                    }

                    // If we fell through above, consume the closing ']'.
                    if meta == META_CLASS || meta == META_CLASS_NOT {
                        // PCRE2_ASSERT(*ptr == META_CLASS_END);
                        ptr = ptr.add(1);
                    }

                    // PCRE2_ASSERT(code > code_start);
                    extra_length =
                        if !lengthptr.is_null() { *lengthptr - prev_length } else { 0 };

                    if *code_start as u32 == OP_ALLANY {
                        // Easiest case: convert OP_ALLANY to ECL_ANY.
                        (*pop_info).length = 1;
                        *code_start = ECL_ANY as u8;
                        (*pop_info).op_single_type = ECL_ANY as u8;
                        core::ptr::write_bytes(
                            (*pop_info).bits.classbits.as_mut_ptr(),
                            0xff,
                            32,
                        );
                    } else if *code_start as u32 == OP_CLASS || *code_start as u32 == OP_NCLASS
                    {
                        (*pop_info).length = 1;
                        let is_class = *code_start as u32 == OP_CLASS;
                        *code_start = if is_class { ECL_NONE as u8 } else { ECL_ANY as u8 };
                        (*pop_info).op_single_type = *code_start;
                        core::ptr::copy_nonoverlapping(
                            code_start.add(1),
                            (*pop_info).bits.classbits.as_mut_ptr(),
                            32,
                        );
                        // Rewind the code pointer, adjust *lengthptr.
                        if !lengthptr.is_null() {
                            *lengthptr += code.offset_from(code_start.add(1)) as usize;
                        }
                        code = code_start.add(1);

                        if (*context).needs_bitmap == FALSE
                            && *code_start as i64 == ECL_NONE
                        {
                            let classwords = (*pop_info).bits.classwords;
                            let mut set = false;
                            for i in 0..8usize {
                                if classwords[i] != 0 {
                                    (*context).needs_bitmap = TRUE;
                                    set = true;
                                    break;
                                }
                            }
                            let _ = set;
                        } else {
                            (*context).needs_bitmap = TRUE;
                        }
                    } else {
                        // OP_XCLASS: hoist out the bitmap (if any).
                        // PCRE2_ASSERT(*code_start == OP_XCLASS);
                        *code_start = ECL_XCLASS as u8;
                        (*pop_info).op_single_type = ECL_XCLASS as u8;

                        core::ptr::copy_nonoverlapping(
                            (*(*context).cb).classbits.classbits.as_ptr(),
                            (*pop_info).bits.classbits.as_mut_ptr(),
                            32,
                        );
                        (*pop_info).length =
                            (code.offset_from(code_start) as usize) + extra_length;
                    }
                }
            }
        }

        if !done {
            (*pop_info).code_start =
                if lengthptr.is_null() { code_start } else { core::ptr::null_mut() };

            if !lengthptr.is_null() {
                *lengthptr += code.offset_from(code_start) as usize;
                code = code_start;
            }
        }

        // DONE:
        // PCRE2_ASSERT(lengthptr == NULL || (code == code_start));

        *pptr = ptr;
        *pcode = code;
        TRUE
    }
}

/// Consumes a group of implicitly-unioned (juxtaposed) class elements.
unsafe fn compile_class_juxtaposition(
    context: *mut eclass_context,
    negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    unsafe {
        let mut ptr: *mut u32 = *pptr;
        let mut code: *mut PCRE2_UCHAR = *pcode;

        // Because it's a non-empty class, there must be an operand at the start.
        if compile_class_operand(context, negated, &mut ptr, &mut code, pop_info, lengthptr)
            == FALSE
        {
            return FALSE;
        }

        while *ptr != META_CLASS_END as u32
            && !(*ptr >= META_ECLASS_AND as u32 && *ptr <= META_ECLASS_NOT as u32)
        {
            let op: c_int;
            let rhs_negated: BOOL;
            let mut rhs_op_info: eclass_op_info = core::mem::zeroed();

            if negated != FALSE {
                // !(A juxtapose B)  ->  !A && !B
                op = ECL_AND as c_int;
                rhs_negated = TRUE;
            } else {
                // A juxtapose B  ->  A || B
                op = ECL_OR as c_int;
                rhs_negated = FALSE;
            }

            // An operand must follow the operator.
            if compile_class_operand(
                context,
                rhs_negated,
                &mut ptr,
                &mut code,
                &mut rhs_op_info,
                lengthptr,
            ) == FALSE
            {
                return FALSE;
            }

            // Convert infix to postfix (RPN).
            fold_binary(op, pop_info, &mut rhs_op_info, lengthptr);
            if lengthptr.is_null() {
                code = (*pop_info).code_start.add((*pop_info).length);
            }
        }

        *pptr = ptr;
        *pcode = code;
        TRUE
    }
}

/// Consumes unary prefix operators.
unsafe fn compile_class_unary(
    context: *mut eclass_context,
    mut negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    unsafe {
        let mut ptr: *mut u32 = *pptr;

        while *ptr == META_ECLASS_NOT as u32 {
            ptr = ptr.add(1);
            negated = if negated == FALSE { TRUE } else { FALSE };
        }

        *pptr = ptr;
        // Because it's a non-empty class, there must be an operand.
        if compile_class_juxtaposition(context, negated, pptr, pcode, pop_info, lengthptr)
            == FALSE
        {
            return FALSE;
        }

        TRUE
    }
}

/// Consumes tightly-binding binary operators.
unsafe fn compile_class_binary_tight(
    context: *mut eclass_context,
    negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    unsafe {
        let mut ptr: *mut u32 = *pptr;
        let mut code: *mut PCRE2_UCHAR = *pcode;

        // Because it's a non-empty class, there must be an operand at the start.
        if compile_class_unary(context, negated, &mut ptr, &mut code, pop_info, lengthptr)
            == FALSE
        {
            return FALSE;
        }

        while *ptr == META_ECLASS_AND as u32 {
            let op: c_int;
            let rhs_negated: BOOL;
            let mut rhs_op_info: eclass_op_info = core::mem::zeroed();

            if negated != FALSE {
                // !(A && B)  ->  !A || !B
                op = ECL_OR as c_int;
                rhs_negated = TRUE;
            } else {
                // A && B  ->  A && B
                op = ECL_AND as c_int;
                rhs_negated = FALSE;
            }

            ptr = ptr.add(1);

            // An operand must follow the operator.
            if compile_class_unary(
                context,
                rhs_negated,
                &mut ptr,
                &mut code,
                &mut rhs_op_info,
                lengthptr,
            ) == FALSE
            {
                return FALSE;
            }

            // Convert infix to postfix (RPN).
            fold_binary(op, pop_info, &mut rhs_op_info, lengthptr);
            if lengthptr.is_null() {
                code = (*pop_info).code_start.add((*pop_info).length);
            }
        }

        *pptr = ptr;
        *pcode = code;
        TRUE
    }
}

/// Consumes loosely-binding binary operators.
unsafe fn compile_class_binary_loose(
    context: *mut eclass_context,
    negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    unsafe {
        let mut ptr: *mut u32 = *pptr;
        let mut code: *mut PCRE2_UCHAR = *pcode;

        // Because it's a non-empty class, there must be an operand at the start.
        if compile_class_binary_tight(context, negated, &mut ptr, &mut code, pop_info, lengthptr)
            == FALSE
        {
            return FALSE;
        }

        while *ptr >= META_ECLASS_OR as u32 && *ptr <= META_ECLASS_XOR as u32 {
            let op: c_int;
            let op_neg: BOOL;
            let rhs_negated: BOOL;
            let mut rhs_op_info: eclass_op_info = core::mem::zeroed();

            if negated != FALSE {
                op = if *ptr == META_ECLASS_OR as u32 {
                    ECL_AND as c_int
                } else if *ptr == META_ECLASS_SUB as u32 {
                    ECL_OR as c_int
                } else {
                    ECL_XOR as c_int
                };
                op_neg = bool_to(*ptr == META_ECLASS_XOR as u32);
                rhs_negated = bool_to(*ptr != META_ECLASS_SUB as u32);
            } else {
                op = if *ptr == META_ECLASS_OR as u32 {
                    ECL_OR as c_int
                } else if *ptr == META_ECLASS_SUB as u32 {
                    ECL_AND as c_int
                } else {
                    ECL_XOR as c_int
                };
                op_neg = FALSE;
                rhs_negated = bool_to(*ptr == META_ECLASS_SUB as u32);
            }

            ptr = ptr.add(1);

            // An operand must follow the operator.
            if compile_class_binary_tight(
                context,
                rhs_negated,
                &mut ptr,
                &mut code,
                &mut rhs_op_info,
                lengthptr,
            ) == FALSE
            {
                return FALSE;
            }

            // Convert infix to postfix (RPN).
            fold_binary(op, pop_info, &mut rhs_op_info, lengthptr);
            if op_neg != FALSE {
                fold_negation(pop_info, lengthptr, FALSE);
            }
            if lengthptr.is_null() {
                code = (*pop_info).code_start.add((*pop_info).length);
            }
        }

        *pptr = ptr;
        *pcode = code;
        TRUE
    }
}

/// Converts the META codes in `pptr` into opcodes written to `pcode`.
unsafe fn compile_eclass_nested(
    context: *mut eclass_context,
    mut negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    unsafe {
        let ptr: *mut u32 = *pptr;

        // The CLASS_IS_ECLASS bit must be set since it is a nested class.
        if *ptr == (META_CLASS_NOT as u32 | CLASS_IS_ECLASS as u32) {
            negated = if negated == FALSE { TRUE } else { FALSE };
        }

        (*pptr) = (*pptr).add(1);

        // Because it's a non-empty class, there must be an operand at the start.
        if compile_class_binary_loose(context, negated, pptr, pcode, pop_info, lengthptr)
            == FALSE
        {
            return FALSE;
        }

        // PCRE2_ASSERT(**pptr == META_CLASS_END);
        TRUE
    }
}

/// `PRIV(compile_class_nested)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_class_nested_8(
    options: u32,
    xoptions: u32,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    unsafe {
        let mut context: eclass_context = eclass_context {
            options,
            xoptions,
            errorcodeptr,
            cb,
            needs_bitmap: FALSE,
        };
        let mut op_info: eclass_op_info = core::mem::zeroed();
        let previous_length: PCRE2_SIZE = if !lengthptr.is_null() { *lengthptr } else { 0 };
        let mut code: *mut PCRE2_UCHAR = *pcode;
        let previous: *mut PCRE2_UCHAR;
        let mut allbitsone: BOOL = TRUE;

        previous = code;
        *code = OP_ECLASS as u8;
        code = code.add(1);
        code = code.add(LINK_SIZE_U);
        *code = 0; // Flags, currently zero.
        code = code.add(1);
        if compile_eclass_nested(&mut context, FALSE, pptr, &mut code, &mut op_info, lengthptr)
            == FALSE
        {
            return FALSE;
        }

        if !lengthptr.is_null() {
            *lengthptr += code.offset_from(previous) as usize;
            code = previous;
        }

        // Do some useful counting of what's in the bitmap.
        for i in 0..8usize {
            if op_info.bits.classwords[i] != 0xffffffff {
                allbitsone = FALSE;
                break;
            }
        }

        // After constant-folding, it may turn out to be a simple class.
        if op_info.op_single_type != 0 {
            // Rewind back over the OP_ECLASS.
            code = previous;

            if op_info.op_single_type as i64 == ECL_ANY && allbitsone != FALSE {
                // Special-cased encoding of OP_ALLANY.
                if !lengthptr.is_null() {
                    *lengthptr -= 1;
                }
                *code = OP_ALLANY as u8;
                code = code.add(1);
            } else if op_info.op_single_type as i64 == ECL_ANY
                || op_info.op_single_type as i64 == ECL_NONE
            {
                let required_len: PCRE2_SIZE = 1 + (32 / core::mem::size_of::<PCRE2_UCHAR>());

                if !lengthptr.is_null() {
                    if required_len > (*lengthptr - previous_length) {
                        *lengthptr = previous_length + required_len;
                    }
                }

                if !lengthptr.is_null() {
                    *lengthptr -= required_len;
                }
                *code = if op_info.op_single_type as i64 == ECL_ANY {
                    OP_NCLASS as u8
                } else {
                    OP_CLASS as u8
                };
                code = code.add(1);
                core::ptr::copy_nonoverlapping(
                    op_info.bits.classbits.as_ptr(),
                    code,
                    32,
                );
                code = code.add(32 / core::mem::size_of::<PCRE2_UCHAR>());
            } else {
                // ECL_XCLASS: put the bitmap back into the OP_XCLASS.
                let need_map: BOOL = context.needs_bitmap;
                let required_len: PCRE2_SIZE;

                // PCRE2_ASSERT(op_info.op_single_type == ECL_XCLASS);
                required_len = op_info.length
                    + if need_map != FALSE {
                        32 / core::mem::size_of::<PCRE2_UCHAR>()
                    } else {
                        0
                    };

                if !lengthptr.is_null() {
                    if required_len > (*lengthptr - previous_length) {
                        *lengthptr = previous_length + required_len;
                    }

                    *lengthptr -= 1 + LINK_SIZE_U + 1;
                    *code = OP_XCLASS as u8;
                    code = code.add(1);
                    PUT(code, 0, (1 + LINK_SIZE_U + 1) as i32);
                    code = code.add(LINK_SIZE_U);
                    *code = 0;
                    code = code.add(1);
                } else {
                    let rest: *mut PCRE2_UCHAR;
                    let rest_len: PCRE2_SIZE;
                    let flags: PCRE2_UCHAR;

                    // 1 unit: OP_XCLASS | LINK_SIZE units | 1 unit: flags | rest
                    rest = op_info.code_start.add(1 + LINK_SIZE_U + 1);
                    rest_len = op_info.code_start.add(op_info.length).offset_from(rest) as usize;

                    // First read any data we use.
                    flags = *op_info.code_start.add(1 + LINK_SIZE_U);
                    // PCRE2_ASSERT((flags & XCL_MAP) == 0);

                    // Do the memmove before any writes.
                    core::ptr::copy(
                        rest,
                        code.add(
                            1 + LINK_SIZE_U
                                + 1
                                + if need_map != FALSE {
                                    32 / core::mem::size_of::<PCRE2_UCHAR>()
                                } else {
                                    0
                                },
                        ),
                        CU2BYTES(rest_len),
                    );

                    // Finally write the header data.
                    *code = OP_XCLASS as u8;
                    code = code.add(1);
                    PUT(code, 0, required_len as i32);
                    code = code.add(LINK_SIZE_U);
                    *code = flags | if need_map != FALSE { XCL_MAP as u8 } else { 0 };
                    code = code.add(1);
                    if need_map != FALSE {
                        core::ptr::copy_nonoverlapping(
                            op_info.bits.classbits.as_ptr(),
                            code,
                            32,
                        );
                        code = code.add(32 / core::mem::size_of::<PCRE2_UCHAR>());
                    }
                    code = code.add(rest_len);
                }
            }
        } else {
            // Keep the OP_ECLASS; insert the bitmap if we have one.
            let need_map: BOOL = context.needs_bitmap;
            let required_len: PCRE2_SIZE = 1
                + LINK_SIZE_U
                + 1
                + if need_map != FALSE {
                    32 / core::mem::size_of::<PCRE2_UCHAR>()
                } else {
                    0
                }
                + op_info.length;

            if !lengthptr.is_null() {
                if required_len > (*lengthptr - previous_length) {
                    *lengthptr = previous_length + required_len;
                }

                *lengthptr -= 1 + LINK_SIZE_U + 1;
                *code = OP_ECLASS as u8;
                code = code.add(1);
                PUT(code, 0, (1 + LINK_SIZE_U + 1) as i32);
                code = code.add(LINK_SIZE_U);
                *code = 0;
                code = code.add(1);
            } else {
                if need_map != FALSE {
                    let map_start: *mut PCRE2_UCHAR = previous.add(1 + LINK_SIZE_U + 1);
                    *previous.add(1 + LINK_SIZE_U) |= ECL_MAP as u8;
                    core::ptr::copy(
                        map_start,
                        map_start.add(32 / core::mem::size_of::<PCRE2_UCHAR>()),
                        CU2BYTES(code.offset_from(map_start) as usize),
                    );
                    core::ptr::copy_nonoverlapping(
                        op_info.bits.classbits.as_ptr(),
                        map_start,
                        32,
                    );
                    code = code.add(32 / core::mem::size_of::<PCRE2_UCHAR>());
                }
                PUT(previous, 1, code.offset_from(previous) as i32);
            }
        }

        *pcode = code;
        TRUE
    }
}
