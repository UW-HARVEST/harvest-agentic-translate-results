// Translated from pcre2_compile_class.c
// 8-bit code units, SUPPORT_UNICODE, SUPPORT_WIDE_CHARS, no JIT, LINK_SIZE == 2.

use crate::compile_h::*;
use crate::internal::*;
use crate::pcre2_pub::*;
use crate::tables::*;
use crate::ucd_data::*;
use crate::ucp::*;
use core::ffi::{c_char, c_int, c_uint, c_void};
use core::ptr::addr_of_mut;

#[repr(C)]
struct eclass_context {
    /* Option bits for eclass. */
    options: u32,
    xoptions: u32,
    /* Rarely used members. */
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
    /* Bitmap is needed. */
    needs_bitmap: BOOL,
}

/* ---------------- SUPPORT_WIDE_CHARS ---------------- */

/* Heapsort algorithm. */

unsafe fn do_heapify(buffer: *mut u32, size: usize, mut i: usize) {
    let mut max: usize;
    let mut left: usize;
    let mut right: usize;
    let mut tmp1: u32;
    let mut tmp2: u32;

    while TRUE != 0 {
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

const PARSE_CLASS_UTF: u32 = 0x1;
const PARSE_CLASS_CASELESS_UTF: u32 = 0x2;
const PARSE_CLASS_RESTRICTED_UTF: u32 = 0x4;
const PARSE_CLASS_TURKISH_UTF: u32 = 0x8;

/* Get the range of nocase characters which includes the
'c' character passed as argument, or directly follows 'c'. */

unsafe fn get_nocase_range(c: u32) -> *const u32 {
    let mut left: u32 = 0;
    let mut right: u32 = _pcre2_ucd_nocase_ranges_size_8;
    let mut middle: u32;
    let base: *const u32 = _pcre2_ucd_nocase_ranges_8.as_ptr();

    if c > MAX_UTF_CODE_POINT {
        return base.add(right as usize);
    }

    while TRUE != 0 {
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
    /* Not reached */
    base
}

/* Get the list of othercase characters, which belongs to the passed range.
Create ranges from these characters, and append them to the buffer argument. */

unsafe fn utf_caseless_extend(
    start: u32,
    end: u32,
    options: u32,
    mut buffer: *mut u32,
) -> usize {
    let mut new_start = start;
    let mut new_end = end;
    let mut c = start;
    let mut list: *const u32;
    let mut tmp: [u32; 3] = [0; 3];
    let tmp_ptr: *mut u32 = tmp.as_mut_ptr();
    let mut result: usize = 2;
    let mut skip_range: *const u32 = get_nocase_range(c);
    let mut skip_start: u32 = *skip_range.add(0);

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
            && UCD_ANY_I(c)
        {
            co = _pcre2_ucd_turkish_dotted_i_caseset_8 + (if UCD_DOTTED_I(c) { 0 } else { 3 });
        } else {
            co = UCD_CASESET(c);
            if co != 0
                && (options & PARSE_CLASS_RESTRICTED_UTF) != 0
                && _pcre2_ucd_caseless_sets_8[co as usize] < 128
            {
                co = 0; /* Ignore the caseless set if it's restricted. */
            }
        }

        if co != 0 {
            list = _pcre2_ucd_caseless_sets_8.as_ptr().add(co as usize);
        } else {
            co = UCD_OTHERCASE(c);
            list = tmp_ptr as *const u32;
            *tmp_ptr.add(0) = c;
            *tmp_ptr.add(1) = NOTACHAR;

            if co != c {
                *tmp_ptr.add(1) = co;
                *tmp_ptr.add(2) = NOTACHAR;
            }
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

unsafe fn append_char_list(mut p: *const u32, mut buffer: *mut u32) -> usize {
    let mut n: *const u32;
    let mut result: usize = 0;

    while *p != NOTACHAR {
        n = p;
        while *n.add(0) == (*n.add(1)).wrapping_sub(1) {
            n = n.add(1);
        }

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

unsafe fn get_highest_char(_options: u32) -> u32 {
    MAX_UTF_CODE_POINT
}

/* Add a negated character list to a buffer. */

unsafe fn append_negated_char_list(
    mut p: *const u32,
    options: u32,
    mut buffer: *mut u32,
) -> usize {
    let mut n: *const u32;
    let mut start: u32 = 0;
    let mut result: usize = 2;

    while *p != NOTACHAR {
        n = p;
        while *n.add(0) == (*n.add(1)).wrapping_sub(1) {
            n = n.add(1);
        }

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

unsafe fn append_non_ascii_range(options: u32, buffer: *mut u32) -> *mut u32 {
    if buffer.is_null() {
        return core::ptr::null_mut();
    }

    *buffer.add(0) = 0x100;
    *buffer.add(1) = get_highest_char(options);
    buffer.add(2)
}

unsafe fn parse_class(mut ptr: *mut u32, options: u32, mut buffer: *mut u32) -> usize {
    let mut total_size: usize = 0;
    let mut size: usize;
    let mut meta_arg: u32;
    let mut start_char: u32;

    while TRUE != 0 {
        match META_CODE(*ptr) {
            META_ESCAPE => {
                meta_arg = META_DATA(*ptr);
                match meta_arg as c_int {
                    ESC_D | ESC_W | ESC_S => {
                        buffer = append_non_ascii_range(options, buffer);
                        total_size += 2;
                    }

                    ESC_h => {
                        size = append_char_list(_pcre2_hspace_list_8.as_ptr(), buffer);
                        total_size += size;
                        if !buffer.is_null() {
                            buffer = buffer.add(size);
                        }
                    }

                    ESC_H => {
                        size = append_negated_char_list(
                            _pcre2_hspace_list_8.as_ptr(),
                            options,
                            buffer,
                        );
                        total_size += size;
                        if !buffer.is_null() {
                            buffer = buffer.add(size);
                        }
                    }

                    ESC_v => {
                        size = append_char_list(_pcre2_vspace_list_8.as_ptr(), buffer);
                        total_size += size;
                        if !buffer.is_null() {
                            buffer = buffer.add(size);
                        }
                    }

                    ESC_V => {
                        size = append_negated_char_list(
                            _pcre2_vspace_list_8.as_ptr(),
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
                        if meta_arg as c_int == ESC_p && (*ptr >> 16) == PT_ANY {
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
            META_POSIX_NEG => {
                buffer = append_non_ascii_range(options, buffer);
                total_size += 2;
                ptr = ptr.add(2);
                continue;
            }
            META_POSIX => {
                ptr = ptr.add(2);
                continue;
            }
            META_BIGVALUE => {
                /* Character literal */
                ptr = ptr.add(1);
            }
            _ => {
                if *ptr >= META_END {
                    return total_size;
                }
            }
        }

        start_char = *ptr;

        if *ptr.add(1) == META_RANGE_LITERAL || *ptr.add(1) == META_RANGE_ESCAPED {
            ptr = ptr.add(2);

            if *ptr == META_BIGVALUE {
                ptr = ptr.add(1);
            }
        }

        if (options & PARSE_CLASS_CASELESS_UTF) != 0 {
            let endc = *ptr;
            ptr = ptr.add(1);
            size = utf_caseless_extend(start_char, endc, options, buffer);
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

    total_size
}

/* Extra uint32_t values for storing the lengths of range lists in
the worst case. Two uint32_t lengths and a range end for a range
starting before 255 */
const CHAR_LIST_EXTRA_SIZE: usize = 3;

/* Starting character values for each character list. */

static char_list_starts: [u32; 3] = [
    XCL_CHAR_LIST_LOW_32_START,
    XCL_CHAR_LIST_HIGH_16_START,
    /* Must be terminated by XCL_CHAR_LIST_LOW_16_START,
    which also represents the end of the bitset. */
    XCL_CHAR_LIST_LOW_16_START,
];

unsafe fn compile_optimize_class(
    start_ptr: *mut u32,
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
) -> *mut class_ranges {
    let cranges: *mut class_ranges;
    let mut ptr: *mut u32;
    let buffer: *mut u32;
    let mut dst: *mut u32;
    let mut class_options: u32 = 0;
    let mut range_list_size: usize;
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

    total_size = range_list_size + (if range_list_size >= 2 { CHAR_LIST_EXTRA_SIZE } else { 0 });

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
    while TRUE != 0 {
        do_heapify(buffer, range_list_size, i);
        if i == 0 {
            break;
        }
        i -= 2;
    }

    i = range_list_size - 2;
    while TRUE != 0 {
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
    if dst.offset_from(ptr) < (2 * (6 - 1)) {
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
    tmp2 = ((char_list_starts.len() as u32) - 1) * XCL_TYPE_BIT_LEN;
    range_start = *dst.add(0);
    range_end = *dst.add(1);

    while TRUE != 0 {
        if range_start >= char_list_start {
            if range_start == range_end || range_end < char_list_end {
                tmp1 += 1;
                next_char = next_char.sub(1);

                if char_list_start < XCL_CHAR_LIST_LOW_32_START {
                    *next_char = ((range_end << XCL_CHAR_SHIFT) | XCL_CHAR_END) as u16;
                } else {
                    next_char = next_char.sub(1);
                    *(next_char as *mut u32) = (range_end << XCL_CHAR_SHIFT) | XCL_CHAR_END;
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
                        *(next_char as *mut u32) = range_start << XCL_CHAR_SHIFT;
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
                    *(next_char as *mut u32) = (range_end << XCL_CHAR_SHIFT) | XCL_CHAR_END;
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
                *(next_char as *mut u32) = tmp1;
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
        (buffer.add(total_size) as *const u8).offset_from(next_char as *const u8) as usize;
    (*cranges).char_lists_start =
        (next_char as *const u8).offset_from(buffer as *const u8) as usize;
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
    let mut c: c_int;
    let mut chartype: c_int;
    let mut prop: &'static ucd_record;
    let mut gentype: u32;
    let mut set_bit: BOOL;

    if ptype == PT_ANY {
        if negated == 0 {
            memset(classbits as *mut c_void, 0xff, 32);
        }
        return;
    }

    c = 0;
    while c < 256 {
        prop = GET_UCD(c as u32);
        set_bit = FALSE;
        let _ = set_bit;

        match ptype {
            PT_LAMP => {
                chartype = prop.chartype as c_int;
                set_bit = (chartype as u32 == ucp_Lu
                    || chartype as u32 == ucp_Ll
                    || chartype as u32 == ucp_Lt) as BOOL;
            }

            PT_GC => {
                set_bit = (_pcre2_ucp_gentype_8[prop.chartype as usize] == pdata) as BOOL;
            }

            PT_PC => {
                set_bit = (prop.chartype as u32 == pdata) as BOOL;
            }

            PT_SC => {
                set_bit = (prop.script as u32 == pdata) as BOOL;
            }

            PT_SCX => {
                set_bit = (prop.script as u32 == pdata
                    || script_set_bit(UCD_SCRIPTX_PROP(prop) as usize, pdata))
                    as BOOL;
            }

            PT_ALNUM => {
                gentype = _pcre2_ucp_gentype_8[prop.chartype as usize];
                set_bit = (gentype == ucp_L || gentype == ucp_N) as BOOL;
            }

            /* PT_SPACE: Perl space; PT_PXSPACE: POSIX space */
            PT_SPACE | PT_PXSPACE => {
                match c as u32 {
                    CHAR_HT | CHAR_SPACE | CHAR_NBSP | CHAR_LF | CHAR_VT | CHAR_FF
                    | CHAR_CR | CHAR_NEL => {
                        set_bit = TRUE;
                    }

                    _ => {
                        set_bit =
                            (_pcre2_ucp_gentype_8[prop.chartype as usize] == ucp_Z) as BOOL;
                    }
                }
            }

            PT_WORD => {
                chartype = prop.chartype as c_int;
                gentype = _pcre2_ucp_gentype_8[chartype as usize];
                set_bit = (gentype == ucp_L
                    || gentype == ucp_N
                    || chartype as u32 == ucp_Mn
                    || chartype as u32 == ucp_Pc) as BOOL;
            }

            PT_UCNC => {
                set_bit = (c as u32 == CHAR_DOLLAR_SIGN
                    || c as u32 == CHAR_COMMERCIAL_AT
                    || c as u32 == CHAR_GRAVE_ACCENT
                    || c >= 0xa0) as BOOL;
            }

            PT_BIDICL => {
                set_bit = (UCD_BIDICLASS_PROP(prop) == pdata) as BOOL;
            }

            PT_BOOL => {
                set_bit = boolprop_set_bit(UCD_BPROPS_PROP(prop) as usize, pdata) as BOOL;
            }

            PT_PXGRAPH => {
                chartype = prop.chartype as c_int;
                gentype = _pcre2_ucp_gentype_8[chartype as usize];
                set_bit = (gentype != ucp_Z
                    && (gentype != ucp_C || chartype as u32 == ucp_Cf)) as BOOL;
            }

            PT_PXPRINT => {
                chartype = prop.chartype as c_int;
                set_bit = (chartype as u32 != ucp_Zl
                    && chartype as u32 != ucp_Zp
                    && (_pcre2_ucp_gentype_8[chartype as usize] != ucp_C
                        || chartype as u32 == ucp_Cf)) as BOOL;
            }

            PT_PXPUNCT => {
                gentype = _pcre2_ucp_gentype_8[prop.chartype as usize];
                set_bit = (gentype == ucp_P || (c < 128 && gentype == ucp_S)) as BOOL;
            }

            _ => {
                set_bit = ((c as u32 >= CHAR_0 && c as u32 <= CHAR_9)
                    || (c as u32 >= CHAR_A && c as u32 <= CHAR_F)
                    || (c as u32 >= CHAR_a && c as u32 <= CHAR_f)) as BOOL;
            }
        }

        if negated != 0 {
            set_bit = (set_bit == 0) as BOOL;
        }
        if set_bit != 0 {
            *classbits |= (1u32 << (c & 0x7)) as u8;
        }
        if (c & 0x7) == 0x7 {
            classbits = classbits.add(1);
        }

        c += 1;
    }
}

/* ---------------- XClass related properties ---------------- */

/* XClass needs to be generated. */
const XCLASS_REQUIRED: u32 = 0x1;
/* XClass has 8 bit character. */
const XCLASS_HAS_8BIT_CHARS: u32 = 0x2;
/* XClass has properties. */
const XCLASS_HAS_PROPS: u32 = 0x4;
/* XClass has character lists. */
const XCLASS_HAS_CHAR_LISTS: u32 = 0x8;
/* XClass matches to all >= 256 characters. */
const XCLASS_HIGH_ANY: u32 = 0x10;

/*************************************************
*   Internal entry point for add range to class  *
*************************************************/

unsafe fn add_to_class(
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
    start: u32,
    end: u32,
) {
    let classbits: *mut u8 = addr_of_mut!((*cb).classbits) as *mut u8;
    let mut c: u32;
    let mut byte_start: u32;
    let mut byte_end: u32;
    let classbits_end: u32 = if end <= 0xff { end } else { 0xff };

    /* If caseless matching is required, scan the range and process alternate
    cases. In Unicode, there are 8-bit characters that have alternate cases that
    are greater than 255 and vice-versa (though these may be ignored if caseless
    restriction is in force). Sometimes we can just extend the original range. */

    if (options & PCRE2_CASELESS) != 0 {
        /* UTF mode. This branch is taken if we don't support wide characters (e.g.
        8-bit library, without UTF), but we do treat those characters as Unicode
        (if UCP flag is set). In this case, we only need to expand the character class
        set to include the case pairs which are in the 0-255 codepoint range. */
        if (options & (PCRE2_UTF | PCRE2_UCP)) != 0 {
            let turkish_i: BOOL = ((xoptions
                & (PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT))
                == PCRE2_EXTRA_TURKISH_CASING) as BOOL;
            if start < 128 {
                let lo_end: u32 = if classbits_end < 127 { classbits_end } else { 127 };
                c = start;
                while c <= lo_end {
                    if turkish_i != 0 && UCD_ANY_I(c) {
                        c += 1;
                        continue;
                    }
                    SETBIT(classbits, *(*cb).fcc.add(c as usize) as u32);
                    c += 1;
                }
            }
            if classbits_end >= 128 {
                let hi_start: u32 = if start > 128 { start } else { 128 };
                c = hi_start;
                while c <= classbits_end {
                    let co: u32 = UCD_OTHERCASE(c);
                    if co <= 0xff {
                        SETBIT(classbits, co);
                    }
                    c += 1;
                }
            }
        }
        /* Not UTF mode */
        else {
            c = start;
            while c <= classbits_end {
                SETBIT(classbits, *(*cb).fcc.add(c as usize) as u32);
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

/*************************************************
*   Internal entry point for add list to class   *
*************************************************/

unsafe fn add_list_to_class(
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
    mut p: *const u32,
) {
    while *p.add(0) < 256 {
        let mut n: c_uint = 0;

        while *p.add((n + 1) as usize) == (*p.add(0)).wrapping_add(n).wrapping_add(1) {
            n += 1;
        }
        add_to_class(options, xoptions, cb, *p.add(0), *p.add(n as usize));

        p = p.add((n + 1) as usize);
    }
}

/*************************************************
*    Add characters not in a list to a class     *
*************************************************/

unsafe fn add_not_list_to_class(
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
    mut p: *const u32,
) {
    if *p.add(0) > 0 {
        add_to_class(options, xoptions, cb, 0, (*p.add(0)).wrapping_sub(1));
    }
    while *p.add(0) < 256 {
        while *p.add(1) == (*p.add(0)).wrapping_add(1) {
            p = p.add(1);
        }
        add_to_class(
            options,
            xoptions,
            cb,
            (*p.add(0)).wrapping_add(1),
            if *p.add(1) > 255 { 255 } else { (*p.add(1)).wrapping_sub(1) },
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
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
) -> *mut u32 {
    let mut pptr: *mut u32 = start_ptr;
    let mut code: *mut PCRE2_UCHAR = *pcode;
    let mut should_flip_negation: BOOL;
    let cbits: *const u8 = (*cb).cbits;
    /* Some functions such as add_to_class() or eclass processing
    expects that the bitset is stored in cb->classbits.classbits. */
    let classbits: *mut u8 = addr_of_mut!((*cb).classbits) as *mut u8;

    let utf: BOOL = ((options & PCRE2_UTF) != 0) as BOOL;

    /* Helper variables for OP_XCLASS opcode (for characters > 255). */

    let mut xclass_props: u32;
    let mut class_uchardata: *mut PCRE2_UCHAR;
    let mut cranges: *mut class_ranges;

    /* If an XClass contains a negative special such as \S, we need to flip the
    negation flag at the end, so that support for characters > 255 works correctly
    (they are all included in the class). An XClass may need to insert specific
    matching or non-matching code for wide characters. */

    should_flip_negation = FALSE;

    /* XClass will be used when characters > 255 might match. */

    xclass_props = 0;

    cranges = core::ptr::null_mut();

    if utf != 0 {
        if !lengthptr.is_null() {
            cranges = compile_optimize_class(pptr, options, xoptions, cb);

            if cranges.is_null() {
                *errorcodeptr = ERR(21);
                return core::ptr::null_mut();
            }

            /* Caching the pre-processed character ranges. */
            if !(*cb).last_data.is_null() {
                (*(*cb).last_data).next = addr_of_mut!((*cranges).header);
            } else {
                (*cb).first_data = addr_of_mut!((*cranges).header);
            }

            (*cb).last_data = addr_of_mut!((*cranges).header);
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

            if *ranges.add((*cranges).range_list_size as usize - 1)
                == GET_MAX_CHAR_VALUE(utf != 0)
                && *ranges.add((*cranges).range_list_size as usize - 2) <= 256
            {
                xclass_props |= XCLASS_HIGH_ANY;
            }
        }
    }

    class_uchardata = code.add(LINK_SIZE + 2); /* For XCLASS items */

    /* Initialize the 256-bit (32-byte) bit map to all zeros. We build the map
    in a temporary bit of memory, in case the class contains fewer than two
    8-bit characters because in that case the compiled code doesn't use the bit
    map. */

    memset(classbits as *mut c_void, 0, 32);

    /* Process items until end_ptr is reached. */

    'main: loop {
        let mut meta: u32 = *pptr;
        pptr = pptr.add(1);
        let local_negate: BOOL;
        let mut posix_class: c_int;
        let taboffset: c_int;
        let mut tabopt: c_int;
        let mut pbits: class_bits_storage = core::mem::zeroed();
        let escape: u32;
        let c: u32;

        /* Handle POSIX classes such as [:alpha:] etc. */
        match META_CODE(meta) {
            META_POSIX | META_POSIX_NEG => {
                local_negate = (meta == META_POSIX_NEG) as BOOL;
                posix_class = *pptr as c_int;
                pptr = pptr.add(1);

                if local_negate != 0 {
                    should_flip_negation = TRUE; /* Note negative special */
                }

                /* If matching is caseless, upper and lower are converted to alpha.
                This relies on the fact that the class table starts with alpha,
                lower, upper as the first 3 entries. */

                if (options & PCRE2_CASELESS) != 0 && posix_class <= 2 {
                    posix_class = 0;
                }

                /* When PCRE2_UCP is set, some of the POSIX classes are converted to
                different escape sequences that use Unicode properties \p or \P.
                Others that are not available via \p or \P have to generate
                XCL_PROP/XCL_NOTPROP directly, which is done here. */

                if (options & PCRE2_UCP) != 0 && (xoptions & PCRE2_EXTRA_ASCII_POSIX) == 0 {
                    let ptype: u32;

                    match posix_class as usize {
                        PC_GRAPH | PC_PRINT | PC_PUNCT => {
                            ptype = if posix_class as usize == PC_GRAPH {
                                PT_PXGRAPH
                            } else if posix_class as usize == PC_PRINT {
                                PT_PXPRINT
                            } else {
                                PT_PXPUNCT
                            };

                            _pcre2_update_classbits_8(ptype, 0, local_negate, classbits);

                            if (xclass_props & XCLASS_HIGH_ANY) == 0 {
                                if !lengthptr.is_null() {
                                    *lengthptr += 3;
                                } else {
                                    *class_uchardata = if local_negate != 0 {
                                        XCL_NOTPROP as PCRE2_UCHAR
                                    } else {
                                        XCL_PROP as PCRE2_UCHAR
                                    };
                                    class_uchardata = class_uchardata.add(1);
                                    *class_uchardata = ptype as PCRE2_UCHAR;
                                    class_uchardata = class_uchardata.add(1);
                                    *class_uchardata = 0;
                                    class_uchardata = class_uchardata.add(1);
                                }
                                xclass_props |= XCLASS_REQUIRED | XCLASS_HAS_PROPS;
                            }
                            continue 'main;
                        }

                        /* For the other POSIX classes (ex: ascii) we are going to
                        fall through to the non-UCP case and build a bit map for
                        characters with code points less than 256. */
                        _ => {}
                    }
                }

                /* In the non-UCP case, or when UCP makes no difference, we build the
                bit map for the POSIX class in a chunk of local store because we may
                be adding and subtracting from it, and we don't want to subtract bits
                that may be in the main map already. At the end we or the result into
                the bit map that is being built. */

                posix_class *= 3;

                /* Copy in the first table (always present) */

                memcpy(
                    addr_of_mut!(pbits) as *mut c_void,
                    cbits.add(_pcre2_posix_class_maps8[posix_class as usize] as usize)
                        as *const c_void,
                    32,
                );

                /* If there is a second table, add or remove it as required. */

                taboffset = _pcre2_posix_class_maps8[(posix_class + 1) as usize];
                tabopt = _pcre2_posix_class_maps8[(posix_class + 2) as usize];

                if taboffset >= 0 {
                    let pb: *mut u8 = addr_of_mut!(pbits) as *mut u8;
                    if tabopt >= 0 {
                        for i in 0..32usize {
                            *pb.add(i) |= *cbits.add(i + taboffset as usize);
                        }
                    } else {
                        for i in 0..32usize {
                            *pb.add(i) &= !*cbits.add(i + taboffset as usize);
                        }
                    }
                }

                /* Now see if we need to remove any special characters. An option
                value of 1 removes vertical space and 2 removes underscore. */

                if tabopt < 0 {
                    tabopt = -tabopt;
                }
                {
                    let pb: *mut u8 = addr_of_mut!(pbits) as *mut u8;
                    if tabopt == 1 {
                        *pb.add(1) &= !0x3cu8;
                    } else if tabopt == 2 {
                        *pb.add(11) &= 0x7f;
                    }
                }

                /* Add the POSIX table or its complement into the main table that is
                being built and we are done. */

                {
                    let classwords: *mut u32 = addr_of_mut!((*cb).classbits) as *mut u32;
                    let pw: *mut u32 = addr_of_mut!(pbits) as *mut u32;

                    if local_negate != 0 {
                        for i in 0..8usize {
                            *classwords.add(i) |= !*pw.add(i);
                        }
                    } else {
                        for i in 0..8usize {
                            *classwords.add(i) |= *pw.add(i);
                        }
                    }
                }

                /* Every class contains at least one < 256 character. */
                xclass_props |= XCLASS_HAS_8BIT_CHARS;
                continue 'main; /* End of POSIX handling */
            }

            /* Other than POSIX classes, the only items we should encounter are
            \d-type escapes and literal characters (possibly as ranges). */
            META_BIGVALUE => {
                meta = *pptr;
                pptr = pptr.add(1);
            }

            META_ESCAPE => {
                escape = META_DATA(meta);

                match escape as c_int {
                    ESC_d => {
                        for i in 0..32usize {
                            *classbits.add(i) |= *cbits.add(i + cbit_digit);
                        }
                    }

                    ESC_D => {
                        should_flip_negation = TRUE;
                        for i in 0..32usize {
                            *classbits.add(i) |= !*cbits.add(i + cbit_digit);
                        }
                    }

                    ESC_w => {
                        for i in 0..32usize {
                            *classbits.add(i) |= *cbits.add(i + cbit_word);
                        }
                    }

                    ESC_W => {
                        should_flip_negation = TRUE;
                        for i in 0..32usize {
                            *classbits.add(i) |= !*cbits.add(i + cbit_word);
                        }
                    }

                    /* Perl 5.004 onwards omitted VT from \s, but restored it at Perl
                    5.18. From PCRE 8.34 we no longer treat \s and \S specially. */

                    ESC_s => {
                        for i in 0..32usize {
                            *classbits.add(i) |= *cbits.add(i + cbit_space);
                        }
                    }

                    ESC_S => {
                        should_flip_negation = TRUE;
                        for i in 0..32usize {
                            *classbits.add(i) |= !*cbits.add(i + cbit_space);
                        }
                    }

                    /* When adding the horizontal or vertical space lists to a class, or
                    their complements, disable PCRE2_CASELESS, because it justs wastes
                    time, and in the "not-x" UTF cases can create unwanted duplicates in
                    the XCLASS list. */

                    ESC_h => {
                        if !cranges.is_null() {
                            /* break */
                        } else {
                            add_list_to_class(
                                options & !PCRE2_CASELESS,
                                xoptions,
                                cb,
                                _pcre2_hspace_list_8.as_ptr(),
                            );
                        }
                    }

                    ESC_H => {
                        if !cranges.is_null() {
                            /* break */
                        } else {
                            add_not_list_to_class(
                                options & !PCRE2_CASELESS,
                                xoptions,
                                cb,
                                _pcre2_hspace_list_8.as_ptr(),
                            );
                        }
                    }

                    ESC_v => {
                        if !cranges.is_null() {
                            /* break */
                        } else {
                            add_list_to_class(
                                options & !PCRE2_CASELESS,
                                xoptions,
                                cb,
                                _pcre2_vspace_list_8.as_ptr(),
                            );
                        }
                    }

                    ESC_V => {
                        if !cranges.is_null() {
                            /* break */
                        } else {
                            add_not_list_to_class(
                                options & !PCRE2_CASELESS,
                                xoptions,
                                cb,
                                _pcre2_vspace_list_8.as_ptr(),
                            );
                        }
                    }

                    /* If Unicode is not supported, \P and \p are not allowed and are
                    faulted at parse time, so will never appear here. */

                    ESC_p | ESC_P => {
                        let ptype: u32 = *pptr >> 16;
                        let pdata: u32 = *pptr & 0xffff;
                        pptr = pptr.add(1);

                        /* The "Any" is processed by PRIV(update_classbits)(). */
                        if ptype == PT_ANY {
                            if utf == 0 && escape as c_int == ESC_p {
                                memset(classbits as *mut c_void, 0xff, 32);
                            }
                            continue 'main;
                        }

                        _pcre2_update_classbits_8(
                            ptype,
                            pdata,
                            (escape as c_int == ESC_P) as BOOL,
                            classbits,
                        );

                        if (xclass_props & XCLASS_HIGH_ANY) == 0 {
                            if !lengthptr.is_null() {
                                *lengthptr += 3;
                            } else {
                                *class_uchardata = if escape as c_int == ESC_p {
                                    XCL_PROP as PCRE2_UCHAR
                                } else {
                                    XCL_NOTPROP as PCRE2_UCHAR
                                };
                                class_uchardata = class_uchardata.add(1);
                                *class_uchardata = ptype as PCRE2_UCHAR;
                                class_uchardata = class_uchardata.add(1);
                                *class_uchardata = pdata as PCRE2_UCHAR;
                                class_uchardata = class_uchardata.add(1);
                            }
                            xclass_props |= XCLASS_REQUIRED | XCLASS_HAS_PROPS;
                        }
                        continue 'main;
                    }

                    _ => {}
                }

                /* Every non-property class contains at least one < 256 character. */
                xclass_props |= XCLASS_HAS_8BIT_CHARS;
                /* End handling \d-type escapes */
                continue 'main;
            }

            _ => {
                /* Literals. */
                if meta < META_END {
                    /* fall through to literal handling */
                } else {
                    /* Non-literals: end of class contents. */
                    break 'main;
                }
            }
        }

        /* A literal character may be followed by a range meta. At parse time
        there are checks for out-of-order characters, for ranges where the two
        characters are equal, and for hyphens that cannot indicate a range. At
        this point, therefore, no checking is needed. */

        c = meta;

        /* Remember if \r or \n were explicitly used */

        if c == CHAR_CR || c == CHAR_NL {
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

            if d == CHAR_CR || d == CHAR_NL {
                (*cb).external_flags |= PCRE2_HASCRORLF;
            }

            if !cranges.is_null() {
                continue 'main;
            }
            xclass_props |= XCLASS_HAS_8BIT_CHARS;

            /* Not an EBCDIC special range */

            add_to_class(options, xoptions, cb, c, d);
            continue 'main;
        } /* End of range handling */

        /* Character ranges are ignored when class_ranges is present. */
        if !cranges.is_null() {
            continue 'main;
        }
        xclass_props |= XCLASS_HAS_8BIT_CHARS;

        /* Handle a single character. */

        add_to_class(options, xoptions, cb, meta, meta);
    } /* End of main class-processing loop */

    /* END_PROCESSING: */

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
                            *lengthptr += crate::ord2utf::_pcre2_ord2utf_8(
                                range_start,
                                class_uchardata,
                            ) as usize;
                        }

                        *lengthptr +=
                            crate::ord2utf::_pcre2_ord2utf_8(range_end, class_uchardata)
                                as usize;
                        continue;
                    }

                    *lengthptr += if range_start < range_end { 3 } else { 2 };
                    continue;
                }

                if utf != 0 {
                    if range_start < range_end {
                        *class_uchardata = XCL_RANGE as PCRE2_UCHAR;
                        class_uchardata = class_uchardata.add(1);
                        class_uchardata = class_uchardata.add(
                            crate::ord2utf::_pcre2_ord2utf_8(range_start, class_uchardata)
                                as usize,
                        );
                    } else {
                        *class_uchardata = XCL_SINGLE as PCRE2_UCHAR;
                        class_uchardata = class_uchardata.add(1);
                    }

                    class_uchardata = class_uchardata.add(
                        crate::ord2utf::_pcre2_ord2utf_8(range_end, class_uchardata) as usize,
                    );
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
    (\p or \P), we have to compile an extended class, with its own opcode,
    unless there were no property settings and there was a negated special such
    as \S in the class, and PCRE2_UCP is not set, because in that case all
    characters > 255 are in or not in the class, so any that were explicitly
    given as well can be ignored.

    If, when generating an xclass, there are no characters < 256, we can omit
    the bitmap in the actual compiled code. */

    'done: {
        if (xclass_props & XCLASS_REQUIRED) != 0 {
            let previous: *mut PCRE2_UCHAR = code;

            if (xclass_props & XCLASS_HAS_CHAR_LISTS) == 0 {
                *class_uchardata = XCL_END as PCRE2_UCHAR; /* Marks the end of extra data */
                class_uchardata = class_uchardata.add(1);
            }
            *code = OP_XCLASS as PCRE2_UCHAR;
            code = code.add(1);
            code = code.add(LINK_SIZE);
            *code = if negate_class != 0 {
                XCL_NOT as PCRE2_UCHAR
            } else {
                0
            };
            if (xclass_props & XCLASS_HAS_PROPS) != 0 {
                *code |= XCL_HASPROP as PCRE2_UCHAR;
            }

            /* If the map is required, move up the extra data to make room for it;
            otherwise just move the code pointer to the end of the extra data. */

            if (xclass_props & XCLASS_HAS_8BIT_CHARS) != 0 || !has_bitmap.is_null() {
                if negate_class != 0 {
                    let classwords: *mut u32 = addr_of_mut!((*cb).classbits) as *mut u32;
                    for i in 0..8usize {
                        *classwords.add(i) = !*classwords.add(i);
                    }
                }

                if has_bitmap.is_null() {
                    *code |= XCL_MAP as PCRE2_UCHAR;
                    code = code.add(1);
                    memmove(
                        code.add(32) as *mut c_void,
                        code as *const c_void,
                        CU2BYTES(class_uchardata.offset_from(code) as usize),
                    );
                    memcpy(code as *mut c_void, classbits as *const c_void, 32);
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
                        CLIST_ALIGN_TO(char_lists_size, core::mem::size_of::<u32>());

                    *lengthptr += 2 + LINK_SIZE;

                    (*cb).char_lists_size += char_lists_size;

                    char_lists_size /= core::mem::size_of::<PCRE2_UCHAR>();

                    /* Storage space for character lists is included
                    in the maximum pattern size. */
                    if *lengthptr > MAX_PATTERN_SIZE
                        || MAX_PATTERN_SIZE - *lengthptr < char_lists_size
                    {
                        *errorcodeptr = ERR(20); /* Pattern is too large */
                        return core::ptr::null_mut();
                    }
                } else {
                    let data: *mut u8;

                    /* Encode as high / low bytes. */
                    *code.add(0) =
                        (XCL_LIST | ((*cranges).char_lists_types as u32 >> 8)) as u8;
                    *code.add(1) = (*cranges).char_lists_types as u8;
                    code = code.add(2);

                    /* Character lists are stored in backwards direction from
                    byte code start. */

                    (*cb).char_lists_size += char_lists_size;
                    data = ((*cb).start_code as *mut u8).sub((*cb).char_lists_size);

                    memcpy(
                        data as *mut c_void,
                        (cranges.add(1) as *const u8).add((*cranges).char_lists_start)
                            as *const c_void,
                        char_lists_size,
                    );

                    /* Since character lists total size is less than MAX_PATTERN_SIZE,
                    their starting offset fits into a value which size is LINK_SIZE. */

                    char_lists_size = (*cb).char_lists_size;
                    PUT(code, 0, (char_lists_size >> 1) as u32);
                    code = code.add(LINK_SIZE);

                    /* If we added padding to align the list, initialize the bytes to
                    defined values, so the library is valgrind-clean. */

                    if (char_lists_size & 0x2) != 0 {
                        *(data as *mut u16).offset(-1) = 0xdead;
                    }

                    (*cb).char_lists_size =
                        CLIST_ALIGN_TO(char_lists_size, core::mem::size_of::<u32>());

                    ((*(*cb).cx).memctl.free.unwrap())(
                        cranges as *mut c_void,
                        (*(*cb).cx).memctl.memory_data,
                    );
                }
            }

            /* Now fill in the complete length of the item */

            PUT(previous, 1, code.offset_from(previous) as u32);
            break 'done; /* End of class handling */
        }

        /* If there are no characters > 255, or they are all to be included or
        excluded, set the opcode to OP_CLASS or OP_NCLASS, depending on whether the
        whole class was negated and whether there were negative specials such as \S
        (non-UCP) in the class. Then copy the 32-byte map into the code vector,
        negating it if necessary. */

        if negate_class != 0 {
            let classwords: *mut u32 = addr_of_mut!((*cb).classbits) as *mut u32;

            for i in 0..8usize {
                *classwords.add(i) = !*classwords.add(i);
            }
        }

        if (utf == 0 || negate_class != should_flip_negation)
            && *(addr_of_mut!((*cb).classbits) as *const u32).add(0) == !0u32
        {
            let classwords: *const u32 = addr_of_mut!((*cb).classbits) as *const u32;
            let mut i: c_int;

            i = 0;
            while i < 8 {
                if *classwords.add(i as usize) != !0u32 {
                    break;
                }
                i += 1;
            }

            if i == 8 {
                *code = OP_ALLANY as PCRE2_UCHAR;
                code = code.add(1);
                break 'done; /* End of class handling */
            }
        }

        *code = if negate_class == should_flip_negation {
            OP_CLASS as PCRE2_UCHAR
        } else {
            OP_NCLASS as PCRE2_UCHAR
        };
        code = code.add(1);
        memcpy(code as *mut c_void, classbits as *const c_void, 32);
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

unsafe fn fold_negation(
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
            *(*pop_info).code_start.add((*pop_info).length) = ECL_NOT as PCRE2_UCHAR;
        }
        (*pop_info).length += 1;
    }
    /* Otherwise, it's a nice single-op item, so we can easily fold in the negation
    without needing to produce an ECL_NOT. */
    else if (*pop_info).op_single_type as u32 == ECL_ANY
        || (*pop_info).op_single_type as u32 == ECL_NONE
    {
        (*pop_info).op_single_type = (if (*pop_info).op_single_type as u32 == ECL_NONE {
            ECL_ANY
        } else {
            ECL_NONE
        }) as u8;
        if lengthptr.is_null() {
            *((*pop_info).code_start) = (*pop_info).op_single_type;
        }
    } else {
        if lengthptr.is_null() {
            *(*pop_info).code_start.add(1 + LINK_SIZE) ^= XCL_NOT as PCRE2_UCHAR;
        }
    }

    if preserve_classbits == 0 {
        let cw: *mut u32 = addr_of_mut!((*pop_info).bits) as *mut u32;
        for i in 0..8usize {
            *cw.add(i) = !*cw.add(i);
        }
    }
}

/* This function folds together two operands using a binary operator.
The new, combined chunk of stack code is written out to *lhs_op_info. */

unsafe fn fold_binary(
    op: c_int,
    lhs_op_info: *mut eclass_op_info,
    rhs_op_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) {
    match op as u32 {
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
                    memmove(
                        (*lhs_op_info).code_start as *mut c_void,
                        (*rhs_op_info).code_start as *const c_void,
                        CU2BYTES((*rhs_op_info).length),
                    );
                }
                (*lhs_op_info).length = (*rhs_op_info).length;
                (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
            } else if (*rhs_op_info).op_single_type as u32 == ECL_NONE {
                /* the result is ECL_NONE: write into the LHS */
                if lengthptr.is_null() {
                    *(*lhs_op_info).code_start.add(0) = ECL_NONE as PCRE2_UCHAR;
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
                    *(*rhs_op_info).code_start.add((*rhs_op_info).length) =
                        ECL_AND as PCRE2_UCHAR;
                }
                (*lhs_op_info).length += (*rhs_op_info).length + 1;
                (*lhs_op_info).op_single_type = 0;
            }

            let lw: *mut u32 = addr_of_mut!((*lhs_op_info).bits) as *mut u32;
            let rw: *const u32 = addr_of_mut!((*rhs_op_info).bits) as *const u32;
            for i in 0..8usize {
                *lw.add(i) &= *rw.add(i);
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
                    memmove(
                        (*lhs_op_info).code_start as *mut c_void,
                        (*rhs_op_info).code_start as *const c_void,
                        CU2BYTES((*rhs_op_info).length),
                    );
                }
                (*lhs_op_info).length = (*rhs_op_info).length;
                (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
            } else if (*rhs_op_info).op_single_type as u32 == ECL_ANY {
                /* the result is ECL_ANY: write into the LHS */
                if lengthptr.is_null() {
                    *(*lhs_op_info).code_start.add(0) = ECL_ANY as PCRE2_UCHAR;
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
                    *(*rhs_op_info).code_start.add((*rhs_op_info).length) =
                        ECL_OR as PCRE2_UCHAR;
                }
                (*lhs_op_info).length += (*rhs_op_info).length + 1;
                (*lhs_op_info).op_single_type = 0;
            }

            let lw: *mut u32 = addr_of_mut!((*lhs_op_info).bits) as *mut u32;
            let rw: *const u32 = addr_of_mut!((*rhs_op_info).bits) as *const u32;
            for i in 0..8usize {
                *lw.add(i) |= *rw.add(i);
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
                    memmove(
                        (*lhs_op_info).code_start as *mut c_void,
                        (*rhs_op_info).code_start as *const c_void,
                        CU2BYTES((*rhs_op_info).length),
                    );
                }
                (*lhs_op_info).length = (*rhs_op_info).length;
                (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
            } else if (*rhs_op_info).op_single_type as u32 == ECL_ANY {
                /* the result is !LHS: fold in the negation, and drop the RHS */
                /* Preserve the classbits, because we promise to deal with them later. */
                fold_negation(lhs_op_info, lengthptr, TRUE);
            } else if (*lhs_op_info).op_single_type as u32 == ECL_ANY {
                /* the result is !RHS: drop the LHS, memmove the RHS into its place, and
                fold in the negation */
                if lengthptr.is_null() {
                    memmove(
                        (*lhs_op_info).code_start as *mut c_void,
                        (*rhs_op_info).code_start as *const c_void,
                        CU2BYTES((*rhs_op_info).length),
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
                    *(*rhs_op_info).code_start.add((*rhs_op_info).length) =
                        ECL_XOR as PCRE2_UCHAR;
                }
                (*lhs_op_info).length += (*rhs_op_info).length + 1;
                (*lhs_op_info).op_single_type = 0;
            }

            let lw: *mut u32 = addr_of_mut!((*lhs_op_info).bits) as *mut u32;
            let rw: *const u32 = addr_of_mut!((*rhs_op_info).bits) as *const u32;
            for i in 0..8usize {
                *lw.add(i) ^= *rw.add(i);
            }
        }

        _ => {}
    }
}

/* This function consumes a group of implicitly-unioned class elements.
These can be characters, ranges, properties, or nested classes, as long
as they are all joined by being placed adjacently. */

unsafe fn compile_class_operand(
    context: *mut eclass_context,
    negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    let mut ptr: *mut u32 = *pptr;
    let prev_ptr: *mut u32;
    let mut code: *mut PCRE2_UCHAR = *pcode;
    let code_start: *mut PCRE2_UCHAR = code;
    let prev_length: PCRE2_SIZE = if !lengthptr.is_null() { *lengthptr } else { 0 };
    let extra_length: PCRE2_SIZE;
    let meta: u32 = META_CODE(*ptr);

    match meta {
        META_CLASS_EMPTY_NOT | META_CLASS_EMPTY => {
            ptr = ptr.add(1);
            (*pop_info).length = 1;
            if ((meta == META_CLASS_EMPTY) as BOOL) == negated {
                (*pop_info).op_single_type = ECL_ANY as u8;
                *code = (*pop_info).op_single_type;
                code = code.add(1);
                memset(addr_of_mut!((*pop_info).bits) as *mut c_void, 0xff, 32);
            } else {
                (*pop_info).op_single_type = ECL_NONE as u8;
                *code = (*pop_info).op_single_type;
                code = code.add(1);
                memset(addr_of_mut!((*pop_info).bits) as *mut c_void, 0, 32);
            }
        }

        _ => {
            if meta == META_CLASS || meta == META_CLASS_NOT {
                if (*ptr & CLASS_IS_ECLASS) != 0 {
                    if compile_eclass_nested(
                        context,
                        negated,
                        &mut ptr,
                        &mut code,
                        pop_info,
                        lengthptr,
                    ) == 0
                    {
                        return FALSE;
                    }

                    ptr = ptr.add(1);
                    /* goto DONE */
                    *pptr = ptr;
                    *pcode = code;
                    return TRUE;
                }

                ptr = ptr.add(1);
                /* Fall through */
            }

            /* Scan forward characters, ranges, and properties.
            For example: inside [a-z_ -- m] we don't have brackets around "a-z_" but
            we still need to collect that fragment up into a "leaf" OP_CLASS. */

            prev_ptr = ptr;
            ptr = _pcre2_compile_class_not_nested_8(
                (*context).options,
                (*context).xoptions,
                ptr,
                &mut code,
                (((meta != META_CLASS_NOT) as BOOL) == negated) as BOOL,
                addr_of_mut!((*context).needs_bitmap),
                (*context).errorcodeptr,
                (*context).cb,
                lengthptr,
            );
            if ptr.is_null() {
                return FALSE;
            }

            /* We must have a 100% guarantee that ptr increases when
            compile_class_operand() returns, even on Release builds, so that we can
            statically prove our loops terminate. */
            if ptr <= prev_ptr {
                return FALSE;
            }

            /* If we fell through above, consume the closing ']'. */
            if meta == META_CLASS || meta == META_CLASS_NOT {
                ptr = ptr.add(1);
            }

            /* Regardless of whether (lengthptr == NULL), some data will still be
            written out to *pcode, which we need: we have to peek at it, to transform
            the opcode into the ECLASS version (since we need to hoist up the
            bitmaps). */
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
                memset(addr_of_mut!((*pop_info).bits) as *mut c_void, 0xff, 32);
            }
            /* For OP_CLASS and OP_NCLASS, we hoist out the bitmap and convert to
            ECL_NONE / ECL_ANY respectively. */
            else if *code_start as u32 == OP_CLASS || *code_start as u32 == OP_NCLASS {
                (*pop_info).length = 1;
                let newv: u8 = (if *code_start as u32 == OP_CLASS {
                    ECL_NONE
                } else {
                    ECL_ANY
                }) as u8;
                (*pop_info).op_single_type = newv;
                *code_start = newv;
                memcpy(
                    addr_of_mut!((*pop_info).bits) as *mut c_void,
                    code_start.add(1) as *const c_void,
                    32,
                );
                /* Rewind the code pointer, but make sure we adjust *lengthptr, because
                we do need to reserve that space (even though we only use it
                temporarily). */
                if !lengthptr.is_null() {
                    *lengthptr += code.offset_from(code_start.add(1)) as usize;
                }
                code = code_start.add(1);

                if (*context).needs_bitmap == 0 && *code_start as u32 == ECL_NONE {
                    let classwords: *const u32 =
                        addr_of_mut!((*pop_info).bits) as *const u32;

                    for i in 0..8usize {
                        if *classwords.add(i) != 0 {
                            (*context).needs_bitmap = TRUE;
                            break;
                        }
                    }
                } else {
                    (*context).needs_bitmap = TRUE;
                }
            }
            /* Finally, for OP_XCLASS we hoist out the bitmap (if any), and convert to
            ECL_XCLASS. */
            else {
                (*pop_info).op_single_type = ECL_XCLASS as u8;
                *code_start = (*pop_info).op_single_type;

                memcpy(
                    addr_of_mut!((*pop_info).bits) as *mut c_void,
                    addr_of_mut!((*(*context).cb).classbits) as *const c_void,
                    32,
                );
                (*pop_info).length =
                    (code.offset_from(code_start) as usize) + extra_length;
            }
        }
    } /* End of switch(meta) */

    (*pop_info).code_start = if lengthptr.is_null() {
        code_start
    } else {
        core::ptr::null_mut()
    };

    if !lengthptr.is_null() {
        *lengthptr += code.offset_from(code_start) as usize;
        code = code_start;
    }

    /* DONE: */
    *pptr = ptr;
    *pcode = code;
    TRUE
}

/* This function consumes a group of implicitly-unioned class elements.
These can be characters, ranges, properties, or nested classes, as long
as they are all joined by being placed adjacently. */

unsafe fn compile_class_juxtaposition(
    context: *mut eclass_context,
    negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    let mut ptr: *mut u32 = *pptr;
    let mut code: *mut PCRE2_UCHAR = *pcode;

    /* Because it's a non-empty class, there must be an operand at the start. */
    if compile_class_operand(context, negated, &mut ptr, &mut code, pop_info, lengthptr) == 0
    {
        return FALSE;
    }

    while *ptr != META_CLASS_END && !(*ptr >= META_ECLASS_AND && *ptr <= META_ECLASS_NOT) {
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
        ) == 0
        {
            return FALSE;
        }

        /* Convert infix to postfix (RPN). */
        fold_binary(op as c_int, pop_info, &mut rhs_op_info, lengthptr);
        if lengthptr.is_null() {
            code = (*pop_info).code_start.add((*pop_info).length);
        }
    }

    *pptr = ptr;
    *pcode = code;
    TRUE
}

/* This function consumes unary prefix operators. */

unsafe fn compile_class_unary(
    context: *mut eclass_context,
    mut negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    let mut ptr: *mut u32 = *pptr;

    while *ptr == META_ECLASS_NOT {
        ptr = ptr.add(1);
        negated = (negated == 0) as BOOL;
    }

    *pptr = ptr;
    /* Because it's a non-empty class, there must be an operand. */
    if compile_class_juxtaposition(context, negated, pptr, pcode, pop_info, lengthptr) == 0 {
        return FALSE;
    }

    TRUE
}

/* This function consumes tightly-binding binary operators. */

unsafe fn compile_class_binary_tight(
    context: *mut eclass_context,
    negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    let mut ptr: *mut u32 = *pptr;
    let mut code: *mut PCRE2_UCHAR = *pcode;

    /* Because it's a non-empty class, there must be an operand at the start. */
    if compile_class_unary(context, negated, &mut ptr, &mut code, pop_info, lengthptr) == 0 {
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
        ) == 0
        {
            return FALSE;
        }

        /* Convert infix to postfix (RPN). */
        fold_binary(op as c_int, pop_info, &mut rhs_op_info, lengthptr);
        if lengthptr.is_null() {
            code = (*pop_info).code_start.add((*pop_info).length);
        }
    }

    *pptr = ptr;
    *pcode = code;
    TRUE
}

/* This function consumes loosely-binding binary operators. */

unsafe fn compile_class_binary_loose(
    context: *mut eclass_context,
    negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    let mut ptr: *mut u32 = *pptr;
    let mut code: *mut PCRE2_UCHAR = *pcode;

    /* Because it's a non-empty class, there must be an operand at the start. */
    if compile_class_binary_tight(context, negated, &mut ptr, &mut code, pop_info, lengthptr)
        == 0
    {
        return FALSE;
    }

    while *ptr >= META_ECLASS_OR && *ptr <= META_ECLASS_XOR {
        let op: u32;
        let op_neg: BOOL;
        let rhs_negated: BOOL;
        let mut rhs_op_info: eclass_op_info = core::mem::zeroed();

        if negated != 0 {
            /* The whole expression is being negated; we respond by unconditionally
            negating the LHS A, before seeing what follows. */
            /* !(A || B)   ->  !A && !B                     */
            /* !(A -- B)   ->  !(A && !B)    ->  !A || B    */
            /* !(A XOR B)  ->  !(!A XOR !B)  ->  !A XNOR !B */
            op = if *ptr == META_ECLASS_OR {
                ECL_AND
            } else if *ptr == META_ECLASS_SUB {
                ECL_OR
            } else {
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
        ) == 0
        {
            return FALSE;
        }

        /* Convert infix to postfix (RPN). */
        fold_binary(op as c_int, pop_info, &mut rhs_op_info, lengthptr);
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

unsafe fn compile_eclass_nested(
    context: *mut eclass_context,
    mut negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    let mut ptr: *mut u32 = *pptr;

    /* The CLASS_IS_ECLASS bit must be set since it is a nested class. */

    let v: u32 = *ptr;
    ptr = ptr.add(1);
    let _ = ptr;
    if v == (META_CLASS_NOT | CLASS_IS_ECLASS) {
        negated = (negated == 0) as BOOL;
    }

    *pptr = (*pptr).add(1);

    /* Because it's a non-empty class, there must be an operand at the start. */
    if compile_class_binary_loose(context, negated, pptr, pcode, pop_info, lengthptr) == 0 {
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
    errorcodeptr: *mut c_int,
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
    *code = OP_ECLASS as PCRE2_UCHAR;
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
    ) == 0
    {
        return FALSE;
    }

    if !lengthptr.is_null() {
        *lengthptr += code.offset_from(previous) as usize;
        code = previous;
        /* (*lengthptr - previous_length) now holds the amount of buffer that
        we require to make the call to compile_class_nested() with
        lengthptr = NULL, and including the (1+LINK_SIZE+1) that we write out
        before that call. */
    }

    /* Do some useful counting of what's in the bitmap. */
    {
        let classwords: *const u32 = addr_of_mut!(op_info.bits) as *const u32;
        for i in 0..8usize {
            if *classwords.add(i) != 0xffffffff {
                allbitsone = FALSE;
                break;
            }
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
            *code = OP_ALLANY as PCRE2_UCHAR;
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
            *code = if op_info.op_single_type as u32 == ECL_ANY {
                OP_NCLASS as PCRE2_UCHAR
            } else {
                OP_CLASS as PCRE2_UCHAR
            };
            code = code.add(1);
            memcpy(
                code as *mut c_void,
                addr_of_mut!(op_info.bits) as *const c_void,
                32,
            );
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
                it will peek backwards. So we do have to write out a (truncated)
                OP_XCLASS, even on this branch. */
                *lengthptr -= 1 + LINK_SIZE + 1;
                *code = OP_XCLASS as PCRE2_UCHAR;
                code = code.add(1);
                PUT(code, 0, (1 + LINK_SIZE + 1) as u32);
                code = code.add(LINK_SIZE);
                *code = 0;
                code = code.add(1);
            } else {
                let rest: *mut PCRE2_UCHAR;
                let rest_len: PCRE2_SIZE;
                let flags: PCRE2_UCHAR;

                /* 1 unit: OP_XCLASS | LINK_SIZE units | 1 unit: flags | ...rest */
                rest = op_info.code_start.add(1 + LINK_SIZE + 1);
                rest_len = (op_info.code_start.add(op_info.length)).offset_from(rest) as usize;

                /* First read any data we use, before memmove splats it. */
                flags = *op_info.code_start.add(1 + LINK_SIZE);

                /* Next do the memmove before any writes. */
                memmove(
                    code.add(1 + LINK_SIZE + 1 + (if need_map != 0 { 32 } else { 0 }))
                        as *mut c_void,
                    rest as *const c_void,
                    CU2BYTES(rest_len),
                );

                /* Finally write the header data. */
                *code = OP_XCLASS as PCRE2_UCHAR;
                code = code.add(1);
                PUT(code, 0, required_len as u32);
                code = code.add(LINK_SIZE);
                *code = flags | (if need_map != 0 { XCL_MAP as PCRE2_UCHAR } else { 0 });
                code = code.add(1);
                if need_map != 0 {
                    memcpy(
                        code as *mut c_void,
                        addr_of_mut!(op_info.bits) as *const c_void,
                        32,
                    );
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
            OP_ECLASS, because of the backwards peek by the quantifier code. Write
            out a (truncated) OP_ECLASS, even on this branch. */
            *lengthptr -= 1 + LINK_SIZE + 1;
            *code = OP_ECLASS as PCRE2_UCHAR;
            code = code.add(1);
            PUT(code, 0, (1 + LINK_SIZE + 1) as u32);
            code = code.add(LINK_SIZE);
            *code = 0;
            code = code.add(1);
        } else {
            if need_map != 0 {
                let map_start: *mut PCRE2_UCHAR = previous.add(1 + LINK_SIZE + 1);
                *previous.add(1 + LINK_SIZE) |= ECL_MAP as PCRE2_UCHAR;
                memmove(
                    map_start.add(32) as *mut c_void,
                    map_start as *const c_void,
                    CU2BYTES(code.offset_from(map_start) as usize),
                );
                memcpy(
                    map_start as *mut c_void,
                    addr_of_mut!(op_info.bits) as *const c_void,
                    32,
                );
                code = code.add(32);
            }
            PUT(previous, 1, code.offset_from(previous) as u32);
        }
    }

    *pcode = code;
    TRUE
}

/* End of pcre2_compile_class.c */
