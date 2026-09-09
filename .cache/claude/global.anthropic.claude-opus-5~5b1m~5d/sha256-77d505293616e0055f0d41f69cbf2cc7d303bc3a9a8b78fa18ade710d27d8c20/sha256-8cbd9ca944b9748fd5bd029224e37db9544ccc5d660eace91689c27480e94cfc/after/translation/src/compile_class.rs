//! Translated from pcre2_compile_class.c.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use core::ffi::{c_char, c_void};

use crate::compile_tables::_pcre2_posix_class_maps8;
use crate::ord2utf::_pcre2_ord2utf_8;

/* ------------------------------------------------------------------ */

#[repr(C)]
pub(crate) struct eclass_context {
    /* Option bits for eclass. */
    pub options: u32,
    pub xoptions: u32,
    /* Rarely used members. */
    pub errorcodeptr: *mut i32,
    pub cb: *mut compile_block,
    /* Bitmap is needed. */
    pub needs_bitmap: BOOL,
}

/* Checks the allowed tokens at the end of a class structure in debug mode.
(PCRE2_DEBUG is not defined, so CLASS_END_CASES(meta) is just "default:".) */

/* ---------------- SUPPORT_WIDE_CHARS ---------------- */

/* Heapsort algorithm. */

pub(crate) unsafe fn do_heapify(buffer: *mut u32, size: usize, i: usize) {
    let mut i = i;
    let mut max: usize;
    let mut left: usize;
    let mut right: usize;
    let mut tmp1: u32;
    let mut tmp2: u32;

    loop {
        max = i;
        left = (i << 1) + 2;
        right = left + 2;

        if left < size && *buffer.add(left) > *buffer.add(max) {
            max = left;
        }
        if right < size && *buffer.add(right) > *buffer.add(max) {
            max = right;
        }
        if i == max {
            return;
        }

        /* Swap items. */
        tmp1 = *buffer.add(i);
        tmp2 = *buffer.add(i + 1);
        *buffer.add(i) = *buffer.add(max);
        *buffer.add(i + 1) = *buffer.add(max + 1);
        *buffer.add(max) = tmp1;
        *buffer.add(max + 1) = tmp2;
        i = max;
    }
}

/* ---------------- SUPPORT_UNICODE ---------------- */

pub(crate) const PARSE_CLASS_UTF: u32 = 0x1;
pub(crate) const PARSE_CLASS_CASELESS_UTF: u32 = 0x2;
pub(crate) const PARSE_CLASS_RESTRICTED_UTF: u32 = 0x4;
pub(crate) const PARSE_CLASS_TURKISH_UTF: u32 = 0x8;

/* Get the range of nocase characters which includes the
'c' character passed as argument, or directly follows 'c'. */

pub(crate) unsafe fn get_nocase_range(c: u32) -> *const u32 {
    let mut left: u32 = 0;
    let mut right: u32 = crate::ucd::_pcre2_ucd_nocase_ranges_size_8;
    let mut middle: u32;
    let base: *const u32 = crate::ucd::_pcre2_ucd_nocase_ranges_8.as_ptr();

    if c > MAX_UTF_CODE_POINT {
        return base.add(right as usize);
    }

    loop {
        /* Range end of the middle element. */
        middle = ((left + right) >> 1) | 0x1;

        if *base.add(middle as usize) <= c {
            left = middle + 1;
        } else if middle > 1 && *base.add((middle - 2) as usize) > c {
            right = middle - 1;
        } else {
            return base.add((middle - 1) as usize);
        }
    }
}

/* Get the list of othercase characters, which belongs to the passed range.
Create ranges from these characters, and append them to the buffer argument. */

pub(crate) unsafe fn utf_caseless_extend(
    start: u32,
    end: u32,
    options: u32,
    buffer: *mut u32,
) -> usize {
    let mut buffer = buffer;
    let mut new_start: u32 = start;
    let mut new_end: u32 = end;
    let mut c: u32 = start;
    let mut list: *const u32;
    let mut tmp: [u32; 3] = [0; 3];
    let mut result: usize = 2;
    let mut skip_range: *const u32 = get_nocase_range(c);
    let mut skip_start: u32 = *skip_range.add(0);

    /* PCRE2_ASSERT(options & PARSE_CLASS_UTF); */

    let end: u32 = end;

    while c <= end {
        let mut co: u32;

        if c > skip_start {
            c = *skip_range.add(1);
            skip_range = skip_range.add(2);
            skip_start = *skip_range.add(0);
            continue;
        }

        /* Compute caseless set. */

        if (options & (PARSE_CLASS_TURKISH_UTF | PARSE_CLASS_RESTRICTED_UTF))
            == PARSE_CLASS_TURKISH_UTF
            && UCD_ANY_I!(c)
        {
            co = crate::ucd::_pcre2_ucd_turkish_dotted_i_caseset_8
                + (if UCD_DOTTED_I!(c) { 0 } else { 3 });
        } else {
            co = UCD_CASESET!(c);
            if co != 0
                && (options & PARSE_CLASS_RESTRICTED_UTF) != 0
                && *crate::ucd::_pcre2_ucd_caseless_sets_8
                    .as_ptr()
                    .add(co as usize)
                    < 128
            {
                co = 0; /* Ignore the caseless set if it's restricted. */
            }
        }

        if co != 0 {
            list = crate::ucd::_pcre2_ucd_caseless_sets_8.as_ptr().add(co as usize);
        } else {
            co = UCD_OTHERCASE!(c);
            /* list = tmp; */
            tmp[0] = c;
            tmp[1] = NOTACHAR;

            if co != c {
                tmp[1] = co;
                tmp[2] = NOTACHAR;
            }
            list = tmp.as_ptr();
        }
        c += 1;

        /* Add characters. */
        loop {
            'cont: {
                if *list < new_start {
                    if (*list).wrapping_add(1) == new_start {
                        new_start -= 1;
                        break 'cont;
                    }
                } else if *list > new_end {
                    if (*list).wrapping_sub(1) == new_end {
                        new_end += 1;
                        break 'cont;
                    }
                } else {
                    break 'cont;
                }

                result += 2;
                if !buffer.is_null() {
                    *buffer.add(0) = *list;
                    *buffer.add(1) = *list;
                    buffer = buffer.add(2);
                }
            }

            list = list.add(1);
            if *list == NOTACHAR {
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

/* Add a character list to a buffer. */

pub(crate) unsafe fn append_char_list(p: *const u32, buffer: *mut u32) -> usize {
    let mut p = p;
    let mut buffer = buffer;
    let mut n: *const u32;
    let mut result: usize = 0;

    while *p != NOTACHAR {
        n = p;
        while *n.add(0) == (*n.add(1)).wrapping_sub(1) {
            n = n.add(1);
        }

        /* PCRE2_ASSERT(*p < 0xffff); */

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

pub(crate) unsafe fn get_highest_char(options: u32) -> u32 {
    let _ = options; /* Avoid compiler warning. */

    MAX_UTF_CODE_POINT
}

/* Add a negated character list to a buffer. */

pub(crate) unsafe fn append_negated_char_list(
    p: *const u32,
    options: u32,
    buffer: *mut u32,
) -> usize {
    let mut p = p;
    let mut buffer = buffer;
    let mut n: *const u32;
    let mut start: u32 = 0;
    let mut result: usize = 2;

    /* PCRE2_ASSERT(*p > 0); */

    while *p != NOTACHAR {
        n = p;
        while *n.add(0) == (*n.add(1)).wrapping_sub(1) {
            n = n.add(1);
        }

        /* PCRE2_ASSERT(*p < 0xffff); */

        if !buffer.is_null() {
            *buffer.add(0) = start;
            *buffer.add(1) = (*p).wrapping_sub(1);
            buffer = buffer.add(2);
        }

        result += 2;
        start = (*n).wrapping_add(1);
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

pub(crate) unsafe fn append_non_ascii_range(options: u32, buffer: *mut u32) -> *mut u32 {
    if buffer.is_null() {
        return core::ptr::null_mut();
    }

    *buffer.add(0) = 0x100;
    *buffer.add(1) = get_highest_char(options);
    buffer.add(2)
}

pub(crate) unsafe fn parse_class(ptr: *mut u32, options: u32, buffer: *mut u32) -> usize {
    let mut ptr = ptr;
    let mut buffer = buffer;
    let mut total_size: usize = 0;
    let mut size: usize;
    let mut meta_arg: u32;
    let mut start_char: u32;

    'ploop: loop {
        'sw: {
            match META_CODE!(*ptr) {
                META_ESCAPE => {
                    meta_arg = META_DATA!(*ptr);
                    match meta_arg {
                        ESC_D | ESC_W | ESC_S => {
                            buffer = append_non_ascii_range(options, buffer);
                            total_size += 2;
                        }

                        ESC_h => {
                            size = append_char_list(
                                crate::tables::_pcre2_hspace_list_8.as_ptr(),
                                buffer,
                            );
                            total_size += size;
                            if !buffer.is_null() {
                                buffer = buffer.add(size);
                            }
                        }

                        ESC_H => {
                            size = append_negated_char_list(
                                crate::tables::_pcre2_hspace_list_8.as_ptr(),
                                options,
                                buffer,
                            );
                            total_size += size;
                            if !buffer.is_null() {
                                buffer = buffer.add(size);
                            }
                        }

                        ESC_v => {
                            size = append_char_list(
                                crate::tables::_pcre2_vspace_list_8.as_ptr(),
                                buffer,
                            );
                            total_size += size;
                            if !buffer.is_null() {
                                buffer = buffer.add(size);
                            }
                        }

                        ESC_V => {
                            size = append_negated_char_list(
                                crate::tables::_pcre2_vspace_list_8.as_ptr(),
                                options,
                                buffer,
                            );
                            total_size += size;
                            if !buffer.is_null() {
                                buffer = buffer.add(size);
                            }
                        }

                        ESC_p | ESC_P => {
                            ptr = ptr.add(1);
                            if meta_arg == ESC_p && (*ptr >> 16) == PT_ANY {
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
                    continue 'ploop;
                }
                META_POSIX_NEG => {
                    buffer = append_non_ascii_range(options, buffer);
                    total_size += 2;
                    ptr = ptr.add(2);
                    continue 'ploop;
                }
                META_POSIX => {
                    ptr = ptr.add(2);
                    continue 'ploop;
                }
                META_BIGVALUE => {
                    /* Character literal */
                    ptr = ptr.add(1);
                    break 'sw;
                }
                _ => {
                    if *ptr >= META_END {
                        return total_size;
                    }
                    break 'sw;
                }
            }
        }

        start_char = *ptr;

        if *ptr.add(1) == META_RANGE_LITERAL || *ptr.add(1) == META_RANGE_ESCAPED {
            ptr = ptr.add(2);
            /* PCRE2_ASSERT(*ptr < META_END || *ptr == META_BIGVALUE); */

            if *ptr == META_BIGVALUE {
                ptr = ptr.add(1);
            }
        }

        if (options & PARSE_CLASS_CASELESS_UTF) != 0 {
            let t = *ptr;
            ptr = ptr.add(1);
            size = utf_caseless_extend(start_char, t, options, buffer);
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

/* Extra uint32_t values for storing the lengths of range lists in
the worst case. Two uint32_t lengths and a range end for a range
starting before 255 */
pub(crate) const CHAR_LIST_EXTRA_SIZE: usize = 3;

/* Starting character values for each character list. */

static char_list_starts: [u32; 3] = [
    XCL_CHAR_LIST_LOW_32_START,
    XCL_CHAR_LIST_HIGH_16_START,
    /* Must be terminated by XCL_CHAR_LIST_LOW_16_START,
    which also represents the end of the bitset. */
    XCL_CHAR_LIST_LOW_16_START,
];

pub(crate) unsafe fn compile_optimize_class(
    start_ptr: *mut u32,
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
) -> *mut class_ranges {
    let cranges: *mut class_ranges;
    let mut ptr: *mut u32;
    let mut buffer: *mut u32;
    let mut dst: *mut u32;
    let mut class_options: u32 = 0;
    let mut range_list_size: usize = 0;
    let total_size: usize;
    let mut i: usize;
    let mut tmp1: u32;
    let mut tmp2: u32;
    let mut char_list_next: *const u32;
    let mut next_char: *mut u16;
    let mut char_list_start: u32;
    let mut char_list_end: u32;
    let mut range_start: u32;
    let mut range_end: u32;

    if (options & PCRE2_UTF) != 0 {
        class_options |= PARSE_CLASS_UTF;
    }

    if (options & PCRE2_CASELESS) != 0 && (options & (PCRE2_UTF | PCRE2_UCP)) != 0 {
        class_options |= PARSE_CLASS_CASELESS_UTF;
    }

    if (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0 {
        class_options |= PARSE_CLASS_RESTRICTED_UTF;
    }

    if (xoptions & PCRE2_EXTRA_TURKISH_CASING) != 0 {
        class_options |= PARSE_CLASS_TURKISH_UTF;
    }

    /* Compute required space for the range. */

    range_list_size = parse_class(start_ptr, class_options, core::ptr::null_mut());

    /* Allocate buffer. The total_size also represents the end of the buffer. */

    total_size = range_list_size
        + (if range_list_size >= 2 {
            CHAR_LIST_EXTRA_SIZE
        } else {
            0
        });

    cranges = ((*(*cb).cx).memctl.malloc.unwrap())(
        core::mem::size_of::<class_ranges>() + total_size * core::mem::size_of::<u32>(),
        (*(*cb).cx).memctl.memory_data,
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

    /* Using <= instead of == to help static analysis. */
    if range_list_size <= 2 {
        return cranges;
    }

    /* In-place sorting of ranges. */

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

    /* Merge ranges whenever possible. */
    dst = buffer;
    ptr = buffer.add(2);
    range_list_size -= 2;

    /* The second condition is a very rare corner case, where the end of the last
    range is the maximum character. This range cannot be extended further. */

    while range_list_size > 0 && *dst.add(1) != !0u32 {
        if (*dst.add(1)).wrapping_add(1) < *ptr.add(0) {
            dst = dst.add(2);
            *dst.add(0) = *ptr.add(0);
            *dst.add(1) = *ptr.add(1);
        } else if *dst.add(1) < *ptr.add(1) {
            *dst.add(1) = *ptr.add(1);
        }

        ptr = ptr.add(2);
        range_list_size -= 2;
    }

    /* When the number of ranges are less than six,
    they are not converted to range lists. */

    ptr = buffer;
    while ptr < dst && *ptr.add(1) < 0x100 {
        ptr = ptr.add(2);
    }
    if dst.offset_from(ptr) < (2 * (6 - 1)) as isize {
        (*cranges).range_list_size = dst.add(2).offset_from(buffer) as u16;
        return cranges;
    }

    /* Compute character lists structures. */

    char_list_next = char_list_starts.as_ptr();
    char_list_start = *char_list_next;
    char_list_next = char_list_next.add(1);
    char_list_end = XCL_CHAR_LIST_LOW_32_END;
    next_char = buffer.add(total_size) as *mut u16;

    tmp1 = 0;
    tmp2 = ((core::mem::size_of::<[u32; 3]>() / core::mem::size_of::<u32>()) as u32 - 1)
        * XCL_TYPE_BIT_LEN;
    range_start = *dst.add(0);
    range_end = *dst.add(1);

    loop {
        if range_start >= char_list_start {
            if range_start == range_end || range_end < char_list_end {
                tmp1 += 1;
                next_char = next_char.sub(1);

                if char_list_start < XCL_CHAR_LIST_LOW_32_START {
                    *next_char = ((range_end << XCL_CHAR_SHIFT) | XCL_CHAR_END) as u16;
                } else {
                    next_char = next_char.sub(1);
                    core::ptr::write_unaligned(
                        next_char as *mut u32,
                        (range_end << XCL_CHAR_SHIFT) | XCL_CHAR_END,
                    );
                }
            }

            if range_start < range_end {
                if range_start > char_list_start {
                    tmp1 += 1;
                    next_char = next_char.sub(1);

                    if char_list_start < XCL_CHAR_LIST_LOW_32_START {
                        *next_char = (range_start << XCL_CHAR_SHIFT) as u16;
                    } else {
                        next_char = next_char.sub(1);
                        core::ptr::write_unaligned(
                            next_char as *mut u32,
                            range_start << XCL_CHAR_SHIFT,
                        );
                    }
                } else {
                    (*cranges).char_lists_types |= (XCL_BEGIN_WITH_RANGE << tmp2) as u16;
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
            if range_end < char_list_end {
                tmp1 += 1;
                next_char = next_char.sub(1);

                if char_list_start < XCL_CHAR_LIST_LOW_32_START {
                    *next_char = ((range_end << XCL_CHAR_SHIFT) | XCL_CHAR_END) as u16;
                } else {
                    next_char = next_char.sub(1);
                    core::ptr::write_unaligned(
                        next_char as *mut u32,
                        (range_end << XCL_CHAR_SHIFT) | XCL_CHAR_END,
                    );
                }
            }

            (*cranges).char_lists_types |= (XCL_BEGIN_WITH_RANGE << tmp2) as u16;
        }

        if tmp1 >= XCL_ITEM_COUNT_MASK {
            (*cranges).char_lists_types |= (XCL_ITEM_COUNT_MASK << tmp2) as u16;
            next_char = next_char.sub(1);

            if char_list_start < XCL_CHAR_LIST_LOW_32_START {
                *next_char = tmp1 as u16;
            } else {
                next_char = next_char.sub(1);
                core::ptr::write_unaligned(next_char as *mut u32, tmp1);
            }
        } else {
            (*cranges).char_lists_types |= (tmp1 << tmp2) as u16;
        }

        if range_end < XCL_CHAR_LIST_LOW_16_START || tmp2 == 0 {
            break;
        }

        char_list_end = char_list_start - 1;
        char_list_start = *char_list_next;
        char_list_next = char_list_next.add(1);
        tmp1 = 0;
        tmp2 -= XCL_TYPE_BIT_LEN;
    }

    if *dst.add(0) < XCL_CHAR_LIST_LOW_16_START {
        dst = dst.add(2);
    }

    (*cranges).char_lists_size =
        (buffer.add(total_size) as *mut u8 as usize) - (next_char as *mut u8 as usize);
    (*cranges).char_lists_start =
        (next_char as *mut u8 as usize) - (buffer as *mut u8 as usize);
    (*cranges).range_list_size = dst.offset_from(buffer) as u16;
    cranges
}

/* ---------------- SUPPORT_UNICODE ---------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_update_classbits_8(
    ptype: u32,
    pdata: u32,
    negated: BOOL,
    classbits: *mut u8,
) {
    /* Update PRIV(xclass) when this function is changed. */
    let mut classbits = classbits;
    let mut c: i32;
    let mut chartype: i32;
    let mut prop: *const ucd_record;
    let mut gentype: u32;
    let mut set_bit: BOOL;

    if ptype == PT_ANY {
        if negated == 0 {
            core::ptr::write_bytes(classbits, 0xff, 32);
        }
        return;
    }

    c = 0;
    while c < 256 {
        prop = GET_UCD!(c);
        set_bit = FALSE;
        let _ = set_bit;

        match ptype {
            PT_LAMP => {
                chartype = (*prop).chartype as i32;
                set_bit = (chartype == ucp_Lu as i32
                    || chartype == ucp_Ll as i32
                    || chartype == ucp_Lt as i32) as BOOL;
            }

            PT_GC => {
                set_bit = (crate::tables::_pcre2_ucp_gentype_8[(*prop).chartype as usize]
                    == pdata) as BOOL;
            }

            PT_PC => {
                set_bit = ((*prop).chartype as u32 == pdata) as BOOL;
            }

            PT_SC => {
                set_bit = ((*prop).script as u32 == pdata) as BOOL;
            }

            PT_SCX => {
                set_bit = ((*prop).script as u32 == pdata
                    || MAPBIT!(
                        crate::ucd::_pcre2_ucd_script_sets_8
                            .as_ptr()
                            .add(UCD_SCRIPTX_PROP!(prop) as usize),
                        pdata
                    ) != 0) as BOOL;
            }

            PT_ALNUM => {
                gentype = crate::tables::_pcre2_ucp_gentype_8[(*prop).chartype as usize];
                set_bit = (gentype == ucp_L || gentype == ucp_N) as BOOL;
            }

            /* PT_SPACE = Perl space, PT_PXSPACE = POSIX space */
            PT_SPACE | PT_PXSPACE => {
                match c {
                    /* HSPACE_BYTE_CASES */
                    0x09 | 0x20 | 0xa0 |
                    /* VSPACE_BYTE_CASES */
                    0x0a | 0x0b | 0x0c | 0x0d | 0x85 => {
                        set_bit = TRUE;
                    }

                    _ => {
                        set_bit = (crate::tables::_pcre2_ucp_gentype_8
                            [(*prop).chartype as usize]
                            == ucp_Z) as BOOL;
                    }
                }
            }

            PT_WORD => {
                chartype = (*prop).chartype as i32;
                gentype = crate::tables::_pcre2_ucp_gentype_8[chartype as usize];
                set_bit = (gentype == ucp_L
                    || gentype == ucp_N
                    || chartype == ucp_Mn as i32
                    || chartype == ucp_Pc as i32) as BOOL;
            }

            PT_UCNC => {
                set_bit = (c == 0x24 /* CHAR_DOLLAR_SIGN */
                    || c == 0x40 /* CHAR_COMMERCIAL_AT */
                    || c == 0x60 /* CHAR_GRAVE_ACCENT */
                    || c >= 0xa0) as BOOL;
            }

            PT_BIDICL => {
                set_bit = (UCD_BIDICLASS_PROP!(prop) == pdata) as BOOL;
            }

            PT_BOOL => {
                set_bit = (MAPBIT!(
                    crate::ucd::_pcre2_ucd_boolprop_sets_8
                        .as_ptr()
                        .add(UCD_BPROPS_PROP!(prop) as usize),
                    pdata
                ) != 0) as BOOL;
            }

            PT_PXGRAPH => {
                chartype = (*prop).chartype as i32;
                gentype = crate::tables::_pcre2_ucp_gentype_8[chartype as usize];
                set_bit = (gentype != ucp_Z
                    && (gentype != ucp_C || chartype == ucp_Cf as i32)) as BOOL;
            }

            PT_PXPRINT => {
                chartype = (*prop).chartype as i32;
                set_bit = (chartype != ucp_Zl as i32
                    && chartype != ucp_Zp as i32
                    && (crate::tables::_pcre2_ucp_gentype_8[chartype as usize] != ucp_C
                        || chartype == ucp_Cf as i32)) as BOOL;
            }

            PT_PXPUNCT => {
                gentype = crate::tables::_pcre2_ucp_gentype_8[(*prop).chartype as usize];
                set_bit = (gentype == ucp_P || (c < 128 && gentype == ucp_S)) as BOOL;
            }

            _ => {
                /* PCRE2_ASSERT(ptype == PT_PXXDIGIT); */
                set_bit = ((c >= 0x30 && c <= 0x39)
                    || (c >= 0x41 && c <= 0x46)
                    || (c >= 0x61 && c <= 0x66)) as BOOL;
            }
        }

        if negated != 0 {
            set_bit = if set_bit != 0 { FALSE } else { TRUE };
        }
        if set_bit != 0 {
            *classbits |= (1i32 << (c & 0x7)) as u8;
        }
        if (c & 0x7) == 0x7 {
            classbits = classbits.add(1);
        }

        c += 1;
    }
}

/* ---------------- XClass related properties ---------------- */

/* XClass needs to be generated. */
pub(crate) const XCLASS_REQUIRED: u32 = 0x1;
/* XClass has 8 bit character. */
pub(crate) const XCLASS_HAS_8BIT_CHARS: u32 = 0x2;
/* XClass has properties. */
pub(crate) const XCLASS_HAS_PROPS: u32 = 0x4;
/* XClass has character lists. */
pub(crate) const XCLASS_HAS_CHAR_LISTS: u32 = 0x8;
/* XClass matches to all >= 256 characters. */
pub(crate) const XCLASS_HIGH_ANY: u32 = 0x10;

/*************************************************
*   Internal entry point for add range to class  *
*************************************************/

/* This function sets the overall range for characters < 256.
It also handles non-utf case folding. */

pub(crate) unsafe fn add_to_class(
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
    start: u32,
    end: u32,
) {
    let classbits: *mut u8 = (*cb).classbits.classbits.as_mut_ptr();
    let mut c: u32;
    let mut byte_start: u32;
    let mut byte_end: u32;
    let classbits_end: u32 = if end <= 0xff { end } else { 0xff };

    /* If caseless matching is required, scan the range and process alternate
    cases. */

    if (options & PCRE2_CASELESS) != 0 {
        /* UTF mode (or UCP). */
        if (options & (PCRE2_UTF | PCRE2_UCP)) != 0 {
            let turkish_i: BOOL = ((xoptions
                & (PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT))
                == PCRE2_EXTRA_TURKISH_CASING) as BOOL;
            if start < 128 {
                let lo_end: u32 = if classbits_end < 127 { classbits_end } else { 127 };
                c = start;
                while c <= lo_end {
                    if turkish_i != 0 && UCD_ANY_I!(c) {
                        c += 1;
                        continue;
                    }
                    SETBIT!(classbits, *(*cb).fcc.add(c as usize));
                    c += 1;
                }
            }
            if classbits_end >= 128 {
                let hi_start: u32 = if start > 128 { start } else { 128 };
                c = hi_start;
                while c <= classbits_end {
                    let co: u32 = UCD_OTHERCASE!(c);
                    if co <= 0xff {
                        SETBIT!(classbits, co);
                    }
                    c += 1;
                }
            }
        }
        /* Not UTF mode */
        else {
            c = start;
            while c <= classbits_end {
                SETBIT!(classbits, *(*cb).fcc.add(c as usize));
                c += 1;
            }
        }
    }

    /* Use the bitmap for characters < 256. Otherwise use extra data. */

    byte_start = (start + 7) >> 3;
    byte_end = (classbits_end + 1) >> 3;

    if byte_start >= byte_end {
        c = start;
        while c <= classbits_end {
            /* Regardless of start, c will always be <= 255. */
            SETBIT!(classbits, c);
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
        SETBIT!(classbits, c);
        c += 1;
    }

    c = byte_end;
    while c <= classbits_end {
        SETBIT!(classbits, c);
        c += 1;
    }
}

/*************************************************
*   Internal entry point for add list to class   *
*************************************************/

pub(crate) unsafe fn add_list_to_class(
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
    p: *const u32,
) {
    let mut p = p;
    while *p.add(0) < 256 {
        let mut n: u32 = 0;

        while *p.add((n + 1) as usize) == *p.add(0) + n + 1 {
            n += 1;
        }
        add_to_class(options, xoptions, cb, *p.add(0), *p.add(n as usize));

        p = p.add((n + 1) as usize);
    }
}

/*************************************************
*    Add characters not in a list to a class     *
*************************************************/

pub(crate) unsafe fn add_not_list_to_class(
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
    p: *const u32,
) {
    let mut p = p;
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

/*************************************************
*  Main entry-point to compile a character class *
*************************************************/

/* This function consumes a "leaf", which is a set of characters that will
become a single OP_CLASS OP_NCLASS, OP_XCLASS, or OP_ALLANY. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_class_not_nested_8(
    options: u32,
    xoptions: u32,
    start_ptr: *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    negate_class: BOOL,
    has_bitmap: *mut BOOL,
    errorcodeptr: *mut i32,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
) -> *mut u32 {
    let mut pptr: *mut u32 = start_ptr;
    let mut code: *mut PCRE2_UCHAR = *pcode;
    let mut should_flip_negation: BOOL;
    let cbits: *const u8 = (*cb).cbits;
    /* Some functions such as add_to_class() or eclass processing
    expects that the bitset is stored in cb->classbits.classbits. */
    let classbits: *mut u8 = (*cb).classbits.classbits.as_mut_ptr();

    let utf: BOOL = ((options & PCRE2_UTF) != 0) as BOOL;

    /* Helper variables for OP_XCLASS opcode (for characters > 255). */

    let mut xclass_props: u32;
    let mut class_uchardata: *mut PCRE2_UCHAR;
    let mut cranges: *mut class_ranges;

    /* If an XClass contains a negative special such as \S, we need to flip the
    negation flag at the end. */

    should_flip_negation = FALSE;

    /* XClass will be used when characters > 255 might match. */

    xclass_props = 0;

    cranges = core::ptr::null_mut();

    if utf != 0 {
        if !lengthptr.is_null() {
            cranges = compile_optimize_class(pptr, options, xoptions, cb);

            if cranges.is_null() {
                *errorcodeptr = ERR21;
                return core::ptr::null_mut();
            }

            /* Caching the pre-processed character ranges. */
            if !(*cb).last_data.is_null() {
                (*(*cb).last_data).next = &mut (*cranges).header as *mut compile_data;
            } else {
                (*cb).first_data = &mut (*cranges).header as *mut compile_data;
            }

            (*cb).last_data = &mut (*cranges).header as *mut compile_data;
        } else {
            /* Reuse the pre-processed character ranges. */
            cranges = (*cb).first_data as *mut class_ranges;
            (*cb).first_data = (*cranges).header.next;
        }

        if (*cranges).range_list_size > 0 {
            let ranges: *const u32 = cranges.add(1) as *const u32;

            if *ranges.add(0) <= 255 {
                xclass_props |= XCLASS_HAS_8BIT_CHARS;
            }

            if *ranges.add(((*cranges).range_list_size - 1) as usize)
                == GET_MAX_CHAR_VALUE!(utf)
                && *ranges.add(((*cranges).range_list_size - 2) as usize) <= 256
            {
                xclass_props |= XCLASS_HIGH_ANY;
            }
        }
    }

    class_uchardata = code.add(LINK_SIZE + 2); /* For XCLASS items */

    /* Initialize the 256-bit (32-byte) bit map to all zeros. */

    core::ptr::write_bytes(classbits, 0, 32);

    /* Process items until end_ptr is reached. */

    'mainloop: loop {
        let mut meta: u32 = {
            let t = *pptr;
            pptr = pptr.add(1);
            t
        };
        let mut local_negate: BOOL;
        let mut posix_class: i32;
        let mut taboffset: i32;
        let mut tabopt: i32;
        let mut pbits: class_bits_storage = class_bits_storage { classbits: [0; 32] };
        let mut escape: u32;
        let c: u32;

        /* Handle POSIX classes such as [:alpha:] etc. */
        'sw: {
            match META_CODE!(meta) {
                META_POSIX | META_POSIX_NEG => {
                    local_negate = (meta == META_POSIX_NEG) as BOOL;
                    posix_class = {
                        let t = *pptr;
                        pptr = pptr.add(1);
                        t as i32
                    };

                    if local_negate != 0 {
                        should_flip_negation = TRUE; /* Note negative special */
                    }

                    /* If matching is caseless, upper and lower are converted to alpha. */

                    if (options & PCRE2_CASELESS) != 0 && posix_class <= 2 {
                        posix_class = 0;
                    }

                    /* When PCRE2_UCP is set, some of the POSIX classes are converted to
                    different escape sequences that use Unicode properties \p or \P. */

                    if (options & PCRE2_UCP) != 0 && (xoptions & PCRE2_EXTRA_ASCII_POSIX) == 0
                    {
                        let ptype: u32;

                        if posix_class == PC_GRAPH as i32
                            || posix_class == PC_PRINT as i32
                            || posix_class == PC_PUNCT as i32
                        {
                            ptype = if posix_class == PC_GRAPH as i32 {
                                PT_PXGRAPH
                            } else if posix_class == PC_PRINT as i32 {
                                PT_PXPRINT
                            } else {
                                PT_PXPUNCT
                            };

                            _pcre2_update_classbits_8(ptype, 0, local_negate, classbits);

                            if (xclass_props & XCLASS_HIGH_ANY) == 0 {
                                if !lengthptr.is_null() {
                                    *lengthptr += 3;
                                } else {
                                    *class_uchardata = (if local_negate != 0 {
                                        XCL_NOTPROP
                                    } else {
                                        XCL_PROP
                                    }) as u8;
                                    class_uchardata = class_uchardata.add(1);
                                    *class_uchardata = ptype as u8;
                                    class_uchardata = class_uchardata.add(1);
                                    *class_uchardata = 0;
                                    class_uchardata = class_uchardata.add(1);
                                }
                                xclass_props |= XCLASS_REQUIRED | XCLASS_HAS_PROPS;
                            }
                            continue 'mainloop;
                        }
                        /* default: break -- fall through to the non-UCP case */
                    }

                    /* In the non-UCP case, or when UCP makes no difference, we build the
                    bit map for the POSIX class in a chunk of local store. */

                    posix_class *= 3;

                    /* Copy in the first table (always present) */

                    core::ptr::copy_nonoverlapping(
                        cbits.offset(_pcre2_posix_class_maps8[posix_class as usize] as isize),
                        pbits.classbits.as_mut_ptr(),
                        32,
                    );

                    /* If there is a second table, add or remove it as required. */

                    taboffset = _pcre2_posix_class_maps8[(posix_class + 1) as usize];
                    tabopt = _pcre2_posix_class_maps8[(posix_class + 2) as usize];

                    if taboffset >= 0 {
                        if tabopt >= 0 {
                            for i in 0..32 {
                                pbits.classbits[i as usize] |=
                                    *cbits.offset(i as isize + taboffset as isize);
                            }
                        } else {
                            for i in 0..32 {
                                pbits.classbits[i as usize] &=
                                    !(*cbits.offset(i as isize + taboffset as isize));
                            }
                        }
                    }

                    /* Now see if we need to remove any special characters. An option
                    value of 1 removes vertical space and 2 removes underscore. */

                    if tabopt < 0 {
                        tabopt = -tabopt;
                    }
                    if tabopt == 1 {
                        pbits.classbits[1] &= !0x3cu8;
                    } else if tabopt == 2 {
                        pbits.classbits[11] &= 0x7f;
                    }

                    /* Add the POSIX table or its complement into the main table that is
                    being built and we are done. */

                    {
                        let classwords: *mut u32 = (*cb).classbits.classwords.as_mut_ptr();

                        if local_negate != 0 {
                            for i in 0..8 {
                                *classwords.add(i as usize) |= !pbits.classwords[i as usize];
                            }
                        } else {
                            for i in 0..8 {
                                *classwords.add(i as usize) |= pbits.classwords[i as usize];
                            }
                        }
                    }

                    /* Every class contains at least one < 256 character. */
                    xclass_props |= XCLASS_HAS_8BIT_CHARS;
                    continue 'mainloop; /* End of POSIX handling */
                }

                /* Other than POSIX classes, the only items we should encounter are
                \d-type escapes and literal characters (possibly as ranges). */
                META_BIGVALUE => {
                    meta = *pptr;
                    pptr = pptr.add(1);
                    break 'sw;
                }

                META_ESCAPE => {
                    escape = META_DATA!(meta);

                    'esc: {
                        match escape {
                            ESC_d => {
                                for i in 0..32 {
                                    *classbits.add(i as usize) |=
                                        *cbits.add(i as usize + cbit_digit);
                                }
                            }

                            ESC_D => {
                                should_flip_negation = TRUE;
                                for i in 0..32 {
                                    *classbits.add(i as usize) |=
                                        !(*cbits.add(i as usize + cbit_digit));
                                }
                            }

                            ESC_w => {
                                for i in 0..32 {
                                    *classbits.add(i as usize) |=
                                        *cbits.add(i as usize + cbit_word);
                                }
                            }

                            ESC_W => {
                                should_flip_negation = TRUE;
                                for i in 0..32 {
                                    *classbits.add(i as usize) |=
                                        !(*cbits.add(i as usize + cbit_word));
                                }
                            }

                            ESC_s => {
                                for i in 0..32 {
                                    *classbits.add(i as usize) |=
                                        *cbits.add(i as usize + cbit_space);
                                }
                            }

                            ESC_S => {
                                should_flip_negation = TRUE;
                                for i in 0..32 {
                                    *classbits.add(i as usize) |=
                                        !(*cbits.add(i as usize + cbit_space));
                                }
                            }

                            /* When adding the horizontal or vertical space lists to a
                            class, or their complements, disable PCRE2_CASELESS. */

                            ESC_h => {
                                if !cranges.is_null() {
                                    break 'esc;
                                }
                                add_list_to_class(
                                    options & !PCRE2_CASELESS,
                                    xoptions,
                                    cb,
                                    crate::tables::_pcre2_hspace_list_8.as_ptr(),
                                );
                            }

                            ESC_H => {
                                if !cranges.is_null() {
                                    break 'esc;
                                }
                                add_not_list_to_class(
                                    options & !PCRE2_CASELESS,
                                    xoptions,
                                    cb,
                                    crate::tables::_pcre2_hspace_list_8.as_ptr(),
                                );
                            }

                            ESC_v => {
                                if !cranges.is_null() {
                                    break 'esc;
                                }
                                add_list_to_class(
                                    options & !PCRE2_CASELESS,
                                    xoptions,
                                    cb,
                                    crate::tables::_pcre2_vspace_list_8.as_ptr(),
                                );
                            }

                            ESC_V => {
                                if !cranges.is_null() {
                                    break 'esc;
                                }
                                add_not_list_to_class(
                                    options & !PCRE2_CASELESS,
                                    xoptions,
                                    cb,
                                    crate::tables::_pcre2_vspace_list_8.as_ptr(),
                                );
                            }

                            /* If Unicode is not supported, \P and \p are not allowed and
                            are faulted at parse time, so will never appear here. */

                            ESC_p | ESC_P => {
                                let ptype: u32 = *pptr >> 16;
                                let pdata: u32 = {
                                    let t = *pptr;
                                    pptr = pptr.add(1);
                                    t & 0xffff
                                };

                                /* The "Any" is processed by PRIV(update_classbits)(). */
                                if ptype == PT_ANY {
                                    if utf == 0 && escape == ESC_p {
                                        core::ptr::write_bytes(classbits, 0xff, 32);
                                    }
                                    continue 'mainloop;
                                }

                                _pcre2_update_classbits_8(
                                    ptype,
                                    pdata,
                                    (escape == ESC_P) as BOOL,
                                    classbits,
                                );

                                if (xclass_props & XCLASS_HIGH_ANY) == 0 {
                                    if !lengthptr.is_null() {
                                        *lengthptr += 3;
                                    } else {
                                        *class_uchardata = (if escape == ESC_p {
                                            XCL_PROP
                                        } else {
                                            XCL_NOTPROP
                                        }) as u8;
                                        class_uchardata = class_uchardata.add(1);
                                        *class_uchardata = ptype as u8;
                                        class_uchardata = class_uchardata.add(1);
                                        *class_uchardata = pdata as u8;
                                        class_uchardata = class_uchardata.add(1);
                                    }
                                    xclass_props |= XCLASS_REQUIRED | XCLASS_HAS_PROPS;
                                }
                                continue 'mainloop;
                            }

                            _ => {}
                        }
                    }

                    /* Every non-property class contains at least one < 256 character. */
                    xclass_props |= XCLASS_HAS_8BIT_CHARS;
                    /* End handling \d-type escapes */
                    continue 'mainloop;
                }

                _ => {
                    /* Literals. */
                    if meta < META_END {
                        break 'sw;
                    }
                    /* Non-literals: end of class contents. */
                    break 'mainloop; /* goto END_PROCESSING */
                }
            }
        }

        /* A literal character may be followed by a range meta. */

        c = meta;

        /* Remember if \r or \n were explicitly used */

        if c == 0x0d /* CHAR_CR */ || c == 0x0a
        /* CHAR_NL */
        {
            (*cb).external_flags |= PCRE2_HASCRORLF;
        }

        /* Process a character range */

        if *pptr == META_RANGE_LITERAL || *pptr == META_RANGE_ESCAPED {
            let mut d: u32;

            pptr = pptr.add(1);
            d = *pptr;
            pptr = pptr.add(1);
            if d == META_BIGVALUE {
                d = *pptr;
                pptr = pptr.add(1);
            }

            /* Remember an explicit \r or \n, and add the range to the class. */

            if d == 0x0d /* CHAR_CR */ || d == 0x0a
            /* CHAR_NL */
            {
                (*cb).external_flags |= PCRE2_HASCRORLF;
            }

            if !cranges.is_null() {
                continue 'mainloop;
            }
            xclass_props |= XCLASS_HAS_8BIT_CHARS;

            /* Not an EBCDIC special range */

            add_to_class(options, xoptions, cb, c, d);
            continue 'mainloop;
        } /* End of range handling */

        /* Character ranges are ignored when class_ranges is present. */
        if !cranges.is_null() {
            continue 'mainloop;
        }
        xclass_props |= XCLASS_HAS_8BIT_CHARS;
        /* Handle a single character. */

        add_to_class(options, xoptions, cb, meta, meta);
    } /* End of main class-processing loop */

    /* END_PROCESSING: */

    'done: {
        if !cranges.is_null() {
            let mut range: *mut u32 = cranges.add(1) as *mut u32;
            let end: *mut u32 = range.add((*cranges).range_list_size as usize);

            while range < end && *range.add(0) < 256 {
                /* Add range to bitset. If we are in UTF or UCP mode, then clear the
                caseless bit, because the cranges handle caselessness (only) in this
                condition. */
                add_to_class(
                    if (options & (PCRE2_UTF | PCRE2_UCP)) != 0 {
                        options & !PCRE2_CASELESS
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
                /* The cranges structure is still used and freed later. */
                xclass_props |= XCLASS_REQUIRED | XCLASS_HAS_CHAR_LISTS;
            } else {
                if (xclass_props & XCLASS_HIGH_ANY) != 0 {
                    should_flip_negation = TRUE;
                    range = end;
                }

                while range < end {
                    let mut range_start: u32 = *range.add(0);
                    let range_end: u32 = *range.add(1);

                    range = range.add(2);
                    xclass_props |= XCLASS_REQUIRED;

                    if range_start < 256 {
                        range_start = 256;
                    }

                    if !lengthptr.is_null() {
                        if utf != 0 {
                            *lengthptr += 1;

                            if range_start < range_end {
                                *lengthptr +=
                                    _pcre2_ord2utf_8(range_start, class_uchardata) as usize;
                            }

                            *lengthptr +=
                                _pcre2_ord2utf_8(range_end, class_uchardata) as usize;
                            continue;
                        }

                        *lengthptr += if range_start < range_end { 3 } else { 2 };
                        continue;
                    }

                    if utf != 0 {
                        if range_start < range_end {
                            *class_uchardata = XCL_RANGE as u8;
                            class_uchardata = class_uchardata.add(1);
                            class_uchardata = class_uchardata
                                .add(_pcre2_ord2utf_8(range_start, class_uchardata) as usize);
                        } else {
                            *class_uchardata = XCL_SINGLE as u8;
                            class_uchardata = class_uchardata.add(1);
                        }

                        class_uchardata = class_uchardata
                            .add(_pcre2_ord2utf_8(range_end, class_uchardata) as usize);
                        continue;
                    }

                    /* Without UTF support, character values are constrained
                    by the bit length, and can only be > 256 for 16-bit and
                    32-bit libraries. */
                }

                if lengthptr.is_null() {
                    ((*(*cb).cx).memctl.free.unwrap())(
                        cranges as *mut c_void,
                        (*(*cb).cx).memctl.memory_data,
                    );
                }
            }
        }

        /* If there are characters with values > 255, or Unicode property settings
        (\p or \P), we have to compile an extended class, with its own opcode. */

        if (xclass_props & XCLASS_REQUIRED) != 0 {
            let previous: *mut PCRE2_UCHAR = code;

            if (xclass_props & XCLASS_HAS_CHAR_LISTS) == 0 {
                *class_uchardata = XCL_END as u8; /* Marks the end of extra data */
                class_uchardata = class_uchardata.add(1);
            }
            *code = OP_XCLASS as u8;
            code = code.add(1);
            code = code.add(LINK_SIZE);
            *code = if negate_class != 0 { XCL_NOT as u8 } else { 0 };
            if (xclass_props & XCLASS_HAS_PROPS) != 0 {
                *code |= XCL_HASPROP as u8;
            }

            /* If the map is required, move up the extra data to make room for it;
            otherwise just move the code pointer to the end of the extra data. */

            if (xclass_props & XCLASS_HAS_8BIT_CHARS) != 0 || !has_bitmap.is_null() {
                if negate_class != 0 {
                    let classwords: *mut u32 = (*cb).classbits.classwords.as_mut_ptr();
                    for i in 0..8 {
                        *classwords.add(i as usize) = !*classwords.add(i as usize);
                    }
                }

                if has_bitmap.is_null() {
                    *code |= XCL_MAP as u8;
                    code = code.add(1);
                    core::ptr::copy(
                        code as *const u8,
                        code.add(32),
                        (class_uchardata as usize) - (code as usize),
                    );
                    core::ptr::copy_nonoverlapping(classbits as *const u8, code, 32);
                    code = class_uchardata.add(32);
                } else {
                    code = class_uchardata;
                    if (xclass_props & XCLASS_HAS_8BIT_CHARS) != 0 {
                        *has_bitmap = TRUE;
                    }
                }
            } else {
                code = class_uchardata;
            }

            if (xclass_props & XCLASS_HAS_CHAR_LISTS) != 0 {
                /* Char lists size is an even number, because all items are 16 or 32
                bit values. The character list data is always aligned to 32 bits. */
                let mut char_lists_size: usize = (*cranges).char_lists_size;

                if !lengthptr.is_null() {
                    char_lists_size =
                        CLIST_ALIGN_TO!(char_lists_size, core::mem::size_of::<u32>());

                    *lengthptr += 2 + LINK_SIZE;

                    (*cb).char_lists_size += char_lists_size;

                    char_lists_size /= core::mem::size_of::<PCRE2_UCHAR>();

                    /* Storage space for character lists is included
                    in the maximum pattern size. */
                    if *lengthptr > MAX_PATTERN_SIZE
                        || MAX_PATTERN_SIZE - *lengthptr < char_lists_size
                    {
                        *errorcodeptr = ERR20; /* Pattern is too large */
                        return core::ptr::null_mut();
                    }
                } else {
                    let data: *mut u8;

                    /* Encode as high / low bytes. */
                    *code.add(0) = (XCL_LIST | ((*cranges).char_lists_types as u32 >> 8)) as u8;
                    *code.add(1) = (*cranges).char_lists_types as u8;
                    code = code.add(2);

                    /* Character lists are stored in backwards direction from
                    byte code start. */

                    (*cb).char_lists_size += char_lists_size;
                    data = ((*cb).start_code as *mut u8).offset(-((*cb).char_lists_size as isize));

                    core::ptr::copy_nonoverlapping(
                        (cranges.add(1) as *const u8).add((*cranges).char_lists_start),
                        data,
                        char_lists_size,
                    );

                    /* Since character lists total size is less than MAX_PATTERN_SIZE,
                    their starting offset fits into a value which size is LINK_SIZE. */

                    char_lists_size = (*cb).char_lists_size;
                    PUT!(code, 0, (char_lists_size >> 1) as u32);
                    code = code.add(LINK_SIZE);

                    /* If we added padding to align the list, initialize the bytes to
                    defined values. */

                    if (char_lists_size & 0x2) != 0 {
                        core::ptr::write_unaligned((data as *mut u16).offset(-1), 0xdead);
                    }

                    (*cb).char_lists_size =
                        CLIST_ALIGN_TO!(char_lists_size, core::mem::size_of::<u32>());

                    ((*(*cb).cx).memctl.free.unwrap())(
                        cranges as *mut c_void,
                        (*(*cb).cx).memctl.memory_data,
                    );
                }
            }

            /* Now fill in the complete length of the item */

            PUT!(previous, 1, code.offset_from(previous) as i32);
            break 'done; /* goto DONE -- End of class handling */
        }

        /* If there are no characters > 255, or they are all to be included or
        excluded, set the opcode to OP_CLASS or OP_NCLASS. */

        if negate_class != 0 {
            let classwords: *mut u32 = (*cb).classbits.classwords.as_mut_ptr();

            for i in 0..8 {
                *classwords.add(i as usize) = !*classwords.add(i as usize);
            }
        }

        if (SELECT_VALUE8!(utf == 0, false) || negate_class != should_flip_negation)
            && (*cb).classbits.classwords[0] == !0u32
        {
            let classwords: *const u32 = (*cb).classbits.classwords.as_ptr();
            let mut i: i32;

            i = 0;
            while i < 8 {
                if *classwords.add(i as usize) != !0u32 {
                    break;
                }
                i += 1;
            }

            if i == 8 {
                *code = OP_ALLANY as u8;
                code = code.add(1);
                break 'done; /* goto DONE -- End of class handling */
            }
        }

        *code = (if negate_class == should_flip_negation {
            OP_CLASS
        } else {
            OP_NCLASS
        }) as u8;
        code = code.add(1);
        core::ptr::copy_nonoverlapping(classbits as *const u8, code, 32);
        code = code.add(32);
    }

    /* DONE: */
    *pcode = code;
    pptr.sub(1)
}

/* ===================================================================*/
/* Here follows a block of ECLASS-compiling functions. */

/* This function folds one operand using the negation operator.
The new, combined chunk of stack code is written out to *pop_info. */

pub(crate) unsafe fn fold_negation(
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
    preserve_classbits: BOOL,
) {
    /* If the chunk of stack code is already composed of multiple ops, we won't
    descend in and try and propagate the negation down the tree. */

    if (*pop_info).op_single_type == 0 {
        if !lengthptr.is_null() {
            *lengthptr += 1;
        } else {
            *(*pop_info).code_start.add((*pop_info).length) = ECL_NOT as u8;
        }
        (*pop_info).length += 1;
    }
    /* Otherwise, it's a nice single-op item, so we can easily fold in the
    negation without needing to produce an ECL_NOT. */
    else if (*pop_info).op_single_type as u32 == ECL_ANY
        || (*pop_info).op_single_type as u32 == ECL_NONE
    {
        (*pop_info).op_single_type = (if (*pop_info).op_single_type as u32 == ECL_NONE {
            ECL_ANY
        } else {
            ECL_NONE
        }) as u8;
        if lengthptr.is_null() {
            *(*pop_info).code_start = (*pop_info).op_single_type;
        }
    } else {
        if lengthptr.is_null() {
            *(*pop_info).code_start.add(1 + LINK_SIZE) ^= XCL_NOT as u8;
        }
    }

    if preserve_classbits == 0 {
        for i in 0..8 {
            (*pop_info).bits.classwords[i as usize] = !(*pop_info).bits.classwords[i as usize];
        }
    }
}

/* This function folds together two operands using a binary operator.
The new, combined chunk of stack code is written out to *lhs_op_info. */

pub(crate) unsafe fn fold_binary(
    op: u32,
    lhs_op_info: *mut eclass_op_info,
    rhs_op_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) {
    match op {
        /* ECL_AND truth table:

           LHS  RHS  RESULT
           ----------------
           ANY  *    RHS
           *    ANY  LHS
           NONE *    NONE
           *    NONE NONE
           X    Y    X & Y
        */
        ECL_AND => {
            if (*rhs_op_info).op_single_type as u32 == ECL_ANY {
                /* no-op: drop the RHS */
            } else if (*lhs_op_info).op_single_type as u32 == ECL_ANY {
                /* no-op: drop the LHS, and memmove the RHS into its place */
                if lengthptr.is_null() {
                    core::ptr::copy(
                        (*rhs_op_info).code_start as *const u8,
                        (*lhs_op_info).code_start,
                        (*rhs_op_info).length,
                    );
                }
                (*lhs_op_info).length = (*rhs_op_info).length;
                (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
            } else if (*rhs_op_info).op_single_type as u32 == ECL_NONE {
                /* the result is ECL_NONE: write into the LHS */
                if lengthptr.is_null() {
                    *(*lhs_op_info).code_start.add(0) = ECL_NONE as u8;
                }
                (*lhs_op_info).length = 1;
                (*lhs_op_info).op_single_type = ECL_NONE as u8;
            } else if (*lhs_op_info).op_single_type as u32 == ECL_NONE {
                /* the result is ECL_NONE: drop the RHS */
            } else {
                /* Both of LHS & RHS are either ECL_XCLASS, or compound operations. */
                if !lengthptr.is_null() {
                    *lengthptr += 1;
                } else {
                    *(*rhs_op_info).code_start.add((*rhs_op_info).length) = ECL_AND as u8;
                }
                (*lhs_op_info).length += (*rhs_op_info).length + 1;
                (*lhs_op_info).op_single_type = 0;
            }

            for i in 0..8 {
                (*lhs_op_info).bits.classwords[i as usize] &=
                    (*rhs_op_info).bits.classwords[i as usize];
            }
        }

        /* ECL_OR truth table:

           LHS  RHS  RESULT
           ----------------
           ANY  *    ANY
           *    ANY  ANY
           NONE *    RHS
           *    NONE LHS
           X    Y    X | Y
        */
        ECL_OR => {
            if (*rhs_op_info).op_single_type as u32 == ECL_NONE {
                /* no-op: drop the RHS */
            } else if (*lhs_op_info).op_single_type as u32 == ECL_NONE {
                /* no-op: drop the LHS, and memmove the RHS into its place */
                if lengthptr.is_null() {
                    core::ptr::copy(
                        (*rhs_op_info).code_start as *const u8,
                        (*lhs_op_info).code_start,
                        (*rhs_op_info).length,
                    );
                }
                (*lhs_op_info).length = (*rhs_op_info).length;
                (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
            } else if (*rhs_op_info).op_single_type as u32 == ECL_ANY {
                /* the result is ECL_ANY: write into the LHS */
                if lengthptr.is_null() {
                    *(*lhs_op_info).code_start.add(0) = ECL_ANY as u8;
                }
                (*lhs_op_info).length = 1;
                (*lhs_op_info).op_single_type = ECL_ANY as u8;
            } else if (*lhs_op_info).op_single_type as u32 == ECL_ANY {
                /* the result is ECL_ANY: drop the RHS */
            } else {
                /* Both of LHS & RHS are either ECL_XCLASS, or compound operations. */
                if !lengthptr.is_null() {
                    *lengthptr += 1;
                } else {
                    *(*rhs_op_info).code_start.add((*rhs_op_info).length) = ECL_OR as u8;
                }
                (*lhs_op_info).length += (*rhs_op_info).length + 1;
                (*lhs_op_info).op_single_type = 0;
            }

            for i in 0..8 {
                (*lhs_op_info).bits.classwords[i as usize] |=
                    (*rhs_op_info).bits.classwords[i as usize];
            }
        }

        /* ECL_XOR truth table:

           LHS  RHS  RESULT
           ----------------
           ANY  *    !RHS
           *    ANY  !LHS
           NONE *    RHS
           *    NONE LHS
           X    Y    X ^ Y
        */
        ECL_XOR => {
            if (*rhs_op_info).op_single_type as u32 == ECL_NONE {
                /* no-op: drop the RHS */
            } else if (*lhs_op_info).op_single_type as u32 == ECL_NONE {
                /* no-op: drop the LHS, and memmove the RHS into its place */
                if lengthptr.is_null() {
                    core::ptr::copy(
                        (*rhs_op_info).code_start as *const u8,
                        (*lhs_op_info).code_start,
                        (*rhs_op_info).length,
                    );
                }
                (*lhs_op_info).length = (*rhs_op_info).length;
                (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
            } else if (*rhs_op_info).op_single_type as u32 == ECL_ANY {
                /* the result is !LHS: fold in the negation, and drop the RHS */
                /* Preserve the classbits, because we promise to deal with them later. */
                fold_negation(lhs_op_info, lengthptr, TRUE);
            } else if (*lhs_op_info).op_single_type as u32 == ECL_ANY {
                /* the result is !RHS: drop the LHS, memmove the RHS into its place,
                and fold in the negation */
                if lengthptr.is_null() {
                    core::ptr::copy(
                        (*rhs_op_info).code_start as *const u8,
                        (*lhs_op_info).code_start,
                        (*rhs_op_info).length,
                    );
                }
                (*lhs_op_info).length = (*rhs_op_info).length;
                (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;

                /* Preserve the classbits, because we promise to deal with them later. */
                fold_negation(lhs_op_info, lengthptr, TRUE);
            } else {
                /* Both of LHS & RHS are either ECL_XCLASS, or compound operations. */
                if !lengthptr.is_null() {
                    *lengthptr += 1;
                } else {
                    *(*rhs_op_info).code_start.add((*rhs_op_info).length) = ECL_XOR as u8;
                }
                (*lhs_op_info).length += (*rhs_op_info).length + 1;
                (*lhs_op_info).op_single_type = 0;
            }

            for i in 0..8 {
                (*lhs_op_info).bits.classwords[i as usize] ^=
                    (*rhs_op_info).bits.classwords[i as usize];
            }
        }

        _ => {}
    }
}

/* This function consumes a group of implicitly-unioned class elements.
These can be characters, ranges, properties, or nested classes, as long
as they are all joined by being placed adjacently. */

pub(crate) unsafe fn compile_class_operand(
    context: *mut eclass_context,
    negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    let mut ptr: *mut u32 = *pptr;
    let mut prev_ptr: *mut u32;
    let mut code: *mut PCRE2_UCHAR = *pcode;
    let code_start: *mut PCRE2_UCHAR = code;
    let prev_length: PCRE2_SIZE = if !lengthptr.is_null() { *lengthptr } else { 0 };
    let mut extra_length: PCRE2_SIZE;
    let meta: u32 = META_CODE!(*ptr);

    'done: {
        let mut do_default: bool = true;

        match meta {
            META_CLASS_EMPTY_NOT | META_CLASS_EMPTY => {
                ptr = ptr.add(1);
                (*pop_info).length = 1;
                if ((meta == META_CLASS_EMPTY) as BOOL) == negated {
                    (*pop_info).op_single_type = ECL_ANY as u8;
                    *code = (*pop_info).op_single_type;
                    code = code.add(1);
                    core::ptr::write_bytes((*pop_info).bits.classbits.as_mut_ptr(), 0xff, 32);
                } else {
                    (*pop_info).op_single_type = ECL_NONE as u8;
                    *code = (*pop_info).op_single_type;
                    code = code.add(1);
                    core::ptr::write_bytes((*pop_info).bits.classbits.as_mut_ptr(), 0, 32);
                }
                do_default = false;
            }

            META_CLASS | META_CLASS_NOT => {
                if (*ptr & CLASS_IS_ECLASS) != 0 {
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

                    ptr = ptr.add(1);
                    break 'done; /* goto DONE */
                }

                ptr = ptr.add(1);
                /* Fall through */
            }

            _ => {}
        }

        if do_default {
            /* Scan forward characters, ranges, and properties. */

            prev_ptr = ptr;
            ptr = _pcre2_compile_class_not_nested_8(
                (*context).options,
                (*context).xoptions,
                ptr,
                &mut code,
                (((meta != META_CLASS_NOT) as BOOL) == negated) as BOOL,
                &mut (*context).needs_bitmap as *mut BOOL,
                (*context).errorcodeptr,
                (*context).cb,
                lengthptr,
            );
            if ptr.is_null() {
                return FALSE;
            }

            /* We must have a 100% guarantee that ptr increases when
            compile_class_operand() returns. */
            if ptr <= prev_ptr {
                return FALSE;
            }

            /* If we fell through above, consume the closing ']'. */
            if meta == META_CLASS || meta == META_CLASS_NOT {
                ptr = ptr.add(1);
            }

            /* Regardless of whether (lengthptr == NULL), some data will still be
            written out to *pcode, which we need. */
            extra_length = if !lengthptr.is_null() {
                *lengthptr - prev_length
            } else {
                0
            };

            /* Easiest case: convert OP_ALLANY to ECL_ANY */

            if *code_start as u32 == OP_ALLANY {
                (*pop_info).length = 1;
                (*pop_info).op_single_type = ECL_ANY as u8;
                *code_start = (*pop_info).op_single_type;
                core::ptr::write_bytes((*pop_info).bits.classbits.as_mut_ptr(), 0xff, 32);
            }
            /* For OP_CLASS and OP_NCLASS, we hoist out the bitmap and convert to
            ECL_NONE / ECL_ANY respectively. */
            else if *code_start as u32 == OP_CLASS || *code_start as u32 == OP_NCLASS {
                (*pop_info).length = 1;
                (*pop_info).op_single_type = (if *code_start as u32 == OP_CLASS {
                    ECL_NONE
                } else {
                    ECL_ANY
                }) as u8;
                *code_start = (*pop_info).op_single_type;
                core::ptr::copy_nonoverlapping(
                    code_start.add(1) as *const u8,
                    (*pop_info).bits.classbits.as_mut_ptr(),
                    32,
                );
                /* Rewind the code pointer, but make sure we adjust *lengthptr,
                because we do need to reserve that space. */
                if !lengthptr.is_null() {
                    *lengthptr += (code as usize) - (code_start.add(1) as usize);
                }
                code = code_start.add(1);

                if (*context).needs_bitmap == 0 && *code_start as u32 == ECL_NONE {
                    let classwords: *mut u32 = (*pop_info).bits.classwords.as_mut_ptr();

                    for i in 0..8 {
                        if *classwords.add(i as usize) != 0 {
                            (*context).needs_bitmap = TRUE;
                            break;
                        }
                    }
                } else {
                    (*context).needs_bitmap = TRUE;
                }
            }
            /* Finally, for OP_XCLASS we hoist out the bitmap (if any), and convert
            to ECL_XCLASS. */
            else {
                (*pop_info).op_single_type = ECL_XCLASS as u8;
                *code_start = (*pop_info).op_single_type;

                core::ptr::copy_nonoverlapping(
                    (*(*context).cb).classbits.classbits.as_ptr(),
                    (*pop_info).bits.classbits.as_mut_ptr(),
                    32,
                );
                (*pop_info).length =
                    ((code as usize) - (code_start as usize)) + extra_length;
            }
        } /* End of switch(meta) */

        (*pop_info).code_start = if lengthptr.is_null() {
            code_start
        } else {
            core::ptr::null_mut()
        };

        if !lengthptr.is_null() {
            *lengthptr += (code as usize) - (code_start as usize);
            code = code_start;
        }
    }

    /* DONE: */
    *pptr = ptr;
    *pcode = code;
    TRUE
}

/* This function consumes a group of implicitly-unioned class elements. */

pub(crate) unsafe fn compile_class_juxtaposition(
    context: *mut eclass_context,
    negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    let mut ptr: *mut u32 = *pptr;
    let mut code: *mut PCRE2_UCHAR = *pcode;

    /* See compile_class_binary_loose() for comments on compile-time folding of
    the "negated" flag. */

    /* Because it's a non-empty class, there must be an operand at the start. */
    if compile_class_operand(context, negated, &mut ptr, &mut code, pop_info, lengthptr)
        == FALSE
    {
        return FALSE;
    }

    while *ptr != META_CLASS_END
        && !(*ptr >= META_ECLASS_AND && *ptr <= META_ECLASS_NOT)
    {
        let op: u32;
        let rhs_negated: BOOL;
        let mut rhs_op_info: eclass_op_info = core::mem::zeroed();

        if negated != 0 {
            /* !(A juxtapose B)  ->  !A && !B */
            op = ECL_AND;
            rhs_negated = TRUE;
        } else {
            /* A juxtapose B  ->  A || B */
            op = ECL_OR;
            rhs_negated = FALSE;
        }

        /* An operand must follow the operator. */
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

        /* Convert infix to postfix (RPN). */
        fold_binary(op, pop_info, &mut rhs_op_info, lengthptr);
        if lengthptr.is_null() {
            code = (*pop_info).code_start.add((*pop_info).length);
        }
    }

    *pptr = ptr;
    *pcode = code;
    TRUE
}

/* This function consumes unary prefix operators. */

pub(crate) unsafe fn compile_class_unary(
    context: *mut eclass_context,
    negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    let mut negated = negated;
    let mut ptr: *mut u32 = *pptr;

    while *ptr == META_ECLASS_NOT {
        ptr = ptr.add(1);
        negated = if negated != 0 { FALSE } else { TRUE };
    }

    *pptr = ptr;
    /* Because it's a non-empty class, there must be an operand. */
    if compile_class_juxtaposition(context, negated, pptr, pcode, pop_info, lengthptr)
        == FALSE
    {
        return FALSE;
    }

    TRUE
}

/* This function consumes tightly-binding binary operators. */

pub(crate) unsafe fn compile_class_binary_tight(
    context: *mut eclass_context,
    negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    let mut ptr: *mut u32 = *pptr;
    let mut code: *mut PCRE2_UCHAR = *pcode;

    /* See compile_class_binary_loose() for comments on compile-time folding of
    the "negated" flag. */

    /* Because it's a non-empty class, there must be an operand at the start. */
    if compile_class_unary(context, negated, &mut ptr, &mut code, pop_info, lengthptr) == FALSE
    {
        return FALSE;
    }

    while *ptr == META_ECLASS_AND {
        let op: u32;
        let rhs_negated: BOOL;
        let mut rhs_op_info: eclass_op_info = core::mem::zeroed();

        if negated != 0 {
            /* !(A && B)  ->  !A || !B */
            op = ECL_OR;
            rhs_negated = TRUE;
        } else {
            /* A && B  ->  A && B */
            op = ECL_AND;
            rhs_negated = FALSE;
        }

        ptr = ptr.add(1);

        /* An operand must follow the operator. */
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

        /* Convert infix to postfix (RPN). */
        fold_binary(op, pop_info, &mut rhs_op_info, lengthptr);
        if lengthptr.is_null() {
            code = (*pop_info).code_start.add((*pop_info).length);
        }
    }

    *pptr = ptr;
    *pcode = code;
    TRUE
}

/* This function consumes loosely-binding binary operators. */

pub(crate) unsafe fn compile_class_binary_loose(
    context: *mut eclass_context,
    negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    let mut ptr: *mut u32 = *pptr;
    let mut code: *mut PCRE2_UCHAR = *pcode;

    /* We really want to fold the negation operator, if at all possible, so that
    simple cases can be reduced down. */

    /* Because it's a non-empty class, there must be an operand at the start. */
    if compile_class_binary_tight(context, negated, &mut ptr, &mut code, pop_info, lengthptr)
        == FALSE
    {
        return FALSE;
    }

    while *ptr >= META_ECLASS_OR && *ptr <= META_ECLASS_XOR {
        let op: u32;
        let op_neg: BOOL;
        let rhs_negated: BOOL;
        let mut rhs_op_info: eclass_op_info = core::mem::zeroed();

        if negated != 0 {
            /* The whole expression is being negated. */
            /* !(A || B)   ->  !A && !B                     */
            /* !(A -- B)   ->  !(A && !B)    ->  !A || B    */
            /* !(A XOR B)  ->  !(!A XOR !B)  ->  !A XNOR !B */
            op = if *ptr == META_ECLASS_OR {
                ECL_AND
            } else if *ptr == META_ECLASS_SUB {
                ECL_OR
            } else {
                /* *ptr == META_ECLASS_XOR */
                ECL_XOR
            };
            op_neg = (*ptr == META_ECLASS_XOR) as BOOL;
            rhs_negated = (*ptr != META_ECLASS_SUB) as BOOL;
        } else {
            /* A || B   ->  A || B  */
            /* A -- B   ->  A && !B */
            /* A XOR B  ->  A XOR B */
            op = if *ptr == META_ECLASS_OR {
                ECL_OR
            } else if *ptr == META_ECLASS_SUB {
                ECL_AND
            } else {
                /* *ptr == META_ECLASS_XOR */
                ECL_XOR
            };
            op_neg = FALSE;
            rhs_negated = (*ptr == META_ECLASS_SUB) as BOOL;
        }

        ptr = ptr.add(1);

        /* An operand must follow the operator. */
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

        /* Convert infix to postfix (RPN). */
        fold_binary(op, pop_info, &mut rhs_op_info, lengthptr);
        if op_neg != 0 {
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

/* This function converts the META codes in pptr into opcodes written to
pcode. The pptr must start at a META_CLASS or META_CLASS_NOT.

The class is compiled as a left-associative sequence of operator
applications.

The pptr will be left pointing at the matching META_CLASS_END. */

pub(crate) unsafe fn compile_eclass_nested(
    context: *mut eclass_context,
    negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    let mut negated = negated;
    let mut ptr: *mut u32 = *pptr;

    /* The CLASS_IS_ECLASS bit must be set since it is a nested class. */

    if {
        let t = *ptr;
        ptr = ptr.add(1);
        t
    } == (META_CLASS_NOT | CLASS_IS_ECLASS)
    {
        negated = if negated != 0 { FALSE } else { TRUE };
    }

    *pptr = (*pptr).add(1);

    /* Because it's a non-empty class, there must be an operand at the start. */
    if compile_class_binary_loose(context, negated, pptr, pcode, pop_info, lengthptr) == FALSE {
        return FALSE;
    }

    TRUE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_class_nested_8(
    options: u32,
    xoptions: u32,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    errorcodeptr: *mut i32,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    let mut context: eclass_context = core::mem::zeroed();
    let mut op_info: eclass_op_info = core::mem::zeroed();
    let previous_length: PCRE2_SIZE = if !lengthptr.is_null() { *lengthptr } else { 0 };
    let mut code: *mut PCRE2_UCHAR = *pcode;
    let previous: *mut PCRE2_UCHAR;
    let mut allbitsone: BOOL = TRUE;

    context.needs_bitmap = FALSE;
    context.options = options;
    context.xoptions = xoptions;
    context.errorcodeptr = errorcodeptr;
    context.cb = cb;

    previous = code;
    *code = OP_ECLASS as u8;
    code = code.add(1);
    code = code.add(LINK_SIZE);
    *code = 0; /* Flags, currently zero. */
    code = code.add(1);
    if compile_eclass_nested(
        &mut context,
        FALSE,
        pptr,
        &mut code,
        &mut op_info,
        lengthptr,
    ) == FALSE
    {
        return FALSE;
    }

    if !lengthptr.is_null() {
        *lengthptr += (code as usize) - (previous as usize);
        code = previous;
        /* (*lengthptr - previous_length) now holds the amount of buffer that
        we require to make the call to compile_class_nested() with
        lengthptr = NULL, and including the (1+LINK_SIZE+1) that we write out
        before that call. */
    }

    /* Do some useful counting of what's in the bitmap. */
    for i in 0..8 {
        if op_info.bits.classwords[i as usize] != 0xffffffff {
            allbitsone = FALSE;
            break;
        }
    }

    /* After constant-folding the extended class syntax, it may turn out to be
    a simple class after all. In that case, we can unwrap it from the
    OP_ECLASS container. */

    if op_info.op_single_type != 0 {
        /* Rewind back over the OP_ECLASS. */
        code = previous;

        /* If the bits are all ones, and the "high characters" are all matched
        too, we use a special-cased encoding of OP_ALLANY. */

        if op_info.op_single_type as u32 == ECL_ANY && allbitsone != 0 {
            /* Advancing code means rewinding lengthptr, at this point. */
            if !lengthptr.is_null() {
                *lengthptr -= 1;
            }
            *code = OP_ALLANY as u8;
            code = code.add(1);
        }
        /* If the high bits are all matched / all not-matched, then we emit an
        OP_NCLASS/OP_CLASS respectively. */
        else if op_info.op_single_type as u32 == ECL_ANY
            || op_info.op_single_type as u32 == ECL_NONE
        {
            let required_len: PCRE2_SIZE = 1 + 32;

            if !lengthptr.is_null() {
                if required_len > (*lengthptr - previous_length) {
                    *lengthptr = previous_length + required_len;
                }
            }

            /* Advancing code means rewinding lengthptr, at this point. */
            if !lengthptr.is_null() {
                *lengthptr -= required_len;
            }
            *code = (if op_info.op_single_type as u32 == ECL_ANY {
                OP_NCLASS
            } else {
                OP_CLASS
            }) as u8;
            code = code.add(1);
            core::ptr::copy_nonoverlapping(op_info.bits.classbits.as_ptr(), code, 32);
            code = code.add(32);
        }
        /* Otherwise, we have an ECL_XCLASS, so we have the OP_XCLASS data
        there, but, we pulled out its bitmap into op_info, so now we have to
        put that back into the OP_XCLASS. */
        else {
            let need_map: BOOL = context.needs_bitmap;
            let required_len: PCRE2_SIZE;

            required_len = op_info.length + (if need_map != 0 { 32 } else { 0 });

            if !lengthptr.is_null() {
                /* Don't unconditionally request all the space we need - we may
                already have asked for more during processing of the ECLASS. */
                if required_len > (*lengthptr - previous_length) {
                    *lengthptr = previous_length + required_len;
                }

                /* The code we write out here won't be ignored, even during the
                (lengthptr != NULL) phase, because if there's a following quantifier
                it will peek backwards. */
                *lengthptr -= 1 + LINK_SIZE + 1;
                *code = OP_XCLASS as u8;
                code = code.add(1);
                PUT!(code, 0, (1 + LINK_SIZE + 1) as u32);
                code = code.add(LINK_SIZE);
                *code = 0;
                code = code.add(1);
            } else {
                let rest: *mut PCRE2_UCHAR;
                let rest_len: PCRE2_SIZE;
                let flags: PCRE2_UCHAR;

                /* 1 unit: OP_XCLASS | LINK_SIZE units | 1 unit: flags | ...rest */
                rest = op_info.code_start.add(1 + LINK_SIZE + 1);
                rest_len = (op_info.code_start.add(op_info.length) as usize)
                    - (rest as usize);

                /* First read any data we use, before memmove splats it. */
                flags = *op_info.code_start.add(1 + LINK_SIZE);

                /* Next do the memmove before any writes. */
                core::ptr::copy(
                    rest as *const u8,
                    code.add(1 + LINK_SIZE + 1 + (if need_map != 0 { 32 } else { 0 })),
                    rest_len,
                );

                /* Finally write the header data. */
                *code = OP_XCLASS as u8;
                code = code.add(1);
                PUT!(code, 0, required_len as i32);
                code = code.add(LINK_SIZE);
                *code = flags | (if need_map != 0 { XCL_MAP as u8 } else { 0 });
                code = code.add(1);
                if need_map != 0 {
                    core::ptr::copy_nonoverlapping(op_info.bits.classbits.as_ptr(), code, 32);
                    code = code.add(32);
                }
                code = code.add(rest_len);
            }
        }
    }
    /* Otherwise, we're going to keep the OP_ECLASS. However, again we need
    to do some adjustment to insert the bitmap if we have one. */
    else {
        let need_map: BOOL = context.needs_bitmap;
        let required_len: PCRE2_SIZE = 1
            + LINK_SIZE
            + 1
            + (if need_map != 0 { 32 } else { 0 })
            + op_info.length;

        if !lengthptr.is_null() {
            if required_len > (*lengthptr - previous_length) {
                *lengthptr = previous_length + required_len;
            }

            /* As for the XCLASS branch above, we do have to write out a dummy
            OP_ECLASS, because of the backwards peek by the quantifier code. */
            *lengthptr -= 1 + LINK_SIZE + 1;
            *code = OP_ECLASS as u8;
            code = code.add(1);
            PUT!(code, 0, (1 + LINK_SIZE + 1) as u32);
            code = code.add(LINK_SIZE);
            *code = 0;
            code = code.add(1);
        } else {
            if need_map != 0 {
                let map_start: *mut PCRE2_UCHAR = previous.add(1 + LINK_SIZE + 1);
                *previous.add(1 + LINK_SIZE) |= ECL_MAP as u8;
                core::ptr::copy(
                    map_start as *const u8,
                    map_start.add(32),
                    (code as usize) - (map_start as usize),
                );
                core::ptr::copy_nonoverlapping(
                    op_info.bits.classbits.as_ptr(),
                    map_start,
                    32,
                );
                code = code.add(32);
            }
            PUT!(previous, 1, code.offset_from(previous) as i32);
        }
    }

    *pcode = code;
    TRUE
}

/* End of pcre2_compile_class.c */
