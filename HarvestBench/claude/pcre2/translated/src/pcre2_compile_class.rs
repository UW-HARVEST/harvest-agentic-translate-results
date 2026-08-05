#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    unused_assignments,
    unused_parens,
    unused_mut,
    unused_variables,
    dead_code
)]

use core::ffi::{c_int, c_void};

use crate::pcre2_internal::*;

/* File-local definitions. */

const MAX_UTF_CODE_POINT: u32 = 0x0010_ffff;

/* eclass context, file-local struct. */
#[repr(C)]
struct eclass_context {
    /* Option bits for eclass. */
    options: u32,
    xoptions: u32,
    /* Rarely used members. */
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
    /* Bitmap is needed. */
    needs_bitmap: bool,
}

/* Parse class option flags (file-local). */
const PARSE_CLASS_UTF: u32 = 0x1;
const PARSE_CLASS_CASELESS_UTF: u32 = 0x2;
const PARSE_CLASS_RESTRICTED_UTF: u32 = 0x4;
const PARSE_CLASS_TURKISH_UTF: u32 = 0x8;

/* Heapsort algorithm. */

unsafe fn do_heapify(buffer: *mut u32, size: usize, mut i: usize) {
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

/* Get the range of nocase characters which includes the
'c' character passed as argument, or directly follows 'c'. */

unsafe fn get_nocase_range(c: u32) -> *const u32 {
    let mut left: u32 = 0;
    let mut right: u32 = _pcre2_ucd_nocase_ranges_size_8;
    let mut middle: u32;

    if c > MAX_UTF_CODE_POINT {
        return _pcre2_ucd_nocase_ranges_8.as_ptr().add(right as usize);
    }

    loop {
        /* Range end of the middle element. */
        middle = ((left + right) >> 1) | 0x1;

        if _pcre2_ucd_nocase_ranges_8[middle as usize] <= c {
            left = middle + 1;
        } else if middle > 1 && _pcre2_ucd_nocase_ranges_8[(middle - 2) as usize] > c {
            right = middle - 1;
        } else {
            return _pcre2_ucd_nocase_ranges_8.as_ptr().add((middle - 1) as usize);
        }
    }
}

/* Get the list of othercase characters, which belongs to the passed range.
Create ranges from these characters, and append them to the buffer argument. */

unsafe fn utf_caseless_extend(
    start: u32,
    end: u32,
    options: u32,
    mut buffer: *mut u32,
) -> usize {
    let mut new_start: u32 = start;
    let mut new_end: u32 = end;
    let mut c: u32 = start;
    let mut list: *const u32;
    let mut tmp: [u32; 3] = [0; 3];
    let mut result: usize = 2;
    let mut skip_range: *const u32 = get_nocase_range(c);
    let mut skip_start: u32 = *skip_range;

    while c <= end {
        let mut co: u32;

        if c > skip_start {
            c = *skip_range.add(1);
            skip_range = skip_range.add(2);
            skip_start = *skip_range;
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
            list = tmp.as_ptr();
            tmp[0] = c;
            tmp[1] = NOTACHAR;

            if co != c {
                tmp[1] = co;
                tmp[2] = NOTACHAR;
            }
        }
        c += 1;

        /* Add characters. */
        loop {
            let add: bool;

            if *list < new_start {
                if *list + 1 == new_start {
                    new_start -= 1;
                    add = false;
                } else {
                    add = true;
                }
            } else if *list > new_end {
                if *list - 1 == new_end {
                    new_end += 1;
                    add = false;
                } else {
                    add = true;
                }
            } else {
                add = false;
            }

            if add {
                result += 2;
                if !buffer.is_null() {
                    *buffer = *list;
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
        *buffer = new_start;
        *buffer.add(1) = new_end;
        buffer = buffer.add(2);
        let _ = buffer;
    }
    result
}

/* Add a character list to a buffer. */

unsafe fn append_char_list(p: *const u32, mut buffer: *mut u32) -> usize {
    let mut p = p;
    let mut n: *const u32;
    let mut result: usize = 0;

    while *p != NOTACHAR {
        n = p;
        while *n == *n.add(1) - 1 {
            n = n.add(1);
        }

        if !buffer.is_null() {
            *buffer = *p;
            *buffer.add(1) = *n;
            buffer = buffer.add(2);
        }

        result += 2;
        p = n.add(1);
    }

    result
}

unsafe fn get_highest_char(options: u32) -> u32 {
    let _ = options;
    MAX_UTF_CODE_POINT
}

/* Add a negated character list to a buffer. */
unsafe fn append_negated_char_list(p: *const u32, options: u32, mut buffer: *mut u32) -> usize {
    let mut p = p;
    let mut n: *const u32;
    let mut start: u32 = 0;
    let mut result: usize = 2;

    while *p != NOTACHAR {
        n = p;
        while *n == *n.add(1) - 1 {
            n = n.add(1);
        }

        if !buffer.is_null() {
            *buffer = start;
            *buffer.add(1) = *p - 1;
            buffer = buffer.add(2);
        }

        result += 2;
        start = *n + 1;
        p = n.add(1);
    }

    if !buffer.is_null() {
        *buffer = start;
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

    *buffer = 0x100;
    *buffer.add(1) = get_highest_char(options);
    buffer.add(2)
}

unsafe fn parse_class(ptr: *mut u32, options: u32, buffer: *mut u32) -> usize {
    let mut ptr = ptr;
    let mut buffer = buffer;
    let mut total_size: usize = 0;
    let mut size: usize;
    let mut meta_arg: c_int;
    let mut start_char: u32;

    loop {
        match META_CODE(*ptr) {
            META_ESCAPE => {
                meta_arg = META_DATA(*ptr) as c_int;
                match meta_arg {
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
                        if meta_arg == ESC_p && (*ptr >> 16) == PT_ANY {
                            if !buffer.is_null() {
                                *buffer = 0;
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

        if options & PARSE_CLASS_CASELESS_UTF != 0 {
            let end_char = *ptr;
            ptr = ptr.add(1);
            size = utf_caseless_extend(start_char, end_char, options, buffer);
            if !buffer.is_null() {
                buffer = buffer.add(size);
            }
            total_size += size;
            continue;
        }

        if !buffer.is_null() {
            *buffer = start_char;
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

    if options & PCRE2_UTF != 0 {
        class_options |= PARSE_CLASS_UTF;
    }

    if (options & PCRE2_CASELESS) != 0 && (options & (PCRE2_UTF | PCRE2_UCP)) != 0 {
        class_options |= PARSE_CLASS_CASELESS_UTF;
    }

    if xoptions & PCRE2_EXTRA_CASELESS_RESTRICT != 0 {
        class_options |= PARSE_CLASS_RESTRICTED_UTF;
    }

    if xoptions & PCRE2_EXTRA_TURKISH_CASING != 0 {
        class_options |= PARSE_CLASS_TURKISH_UTF;
    }

    /* Compute required space for the range. */

    range_list_size = parse_class(start_ptr, class_options, core::ptr::null_mut());

    /* Allocate buffer. The total_size also represents the end of the buffer. */

    total_size = range_list_size + (if range_list_size >= 2 { CHAR_LIST_EXTRA_SIZE } else { 0 });

    cranges = (*(*cb).cx).memctl.malloc.unwrap()(
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
        *buffer.add(i) = *buffer;
        *buffer.add(i + 1) = *buffer.add(1);
        *buffer = tmp1;
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
        if *dst.add(1) + 1 < *ptr {
            dst = dst.add(2);
            *dst = *ptr;
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
    if (dst as isize - ptr as isize) / (core::mem::size_of::<u32>() as isize) < (2 * (6 - 1)) {
        (*cranges).range_list_size = (dst.add(2).offset_from(buffer)) as u16;
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
    range_start = *dst;
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
                range_start = *dst;
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

    if *dst < XCL_CHAR_LIST_LOW_16_START {
        dst = dst.add(2);
    }

    (*cranges).char_lists_size =
        (buffer.add(total_size) as *const u8).offset_from(next_char as *const u8) as usize;
    (*cranges).char_lists_start =
        (next_char as *const u8).offset_from(buffer as *const u8) as usize;
    (*cranges).range_list_size = dst.offset_from(buffer) as u16;
    cranges
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_update_classbits_8(
    ptype: u32,
    pdata: u32,
    negated: c_int,
    classbits: *mut u8,
) {
    /* Update PRIV(xclass) when this function is changed. */
    let mut c: c_int;
    let mut chartype: c_int;
    let mut prop: *const ucd_record;
    let mut gentype: u32;
    let mut set_bit: bool;
    let mut classbits = classbits;
    let negated = negated != 0;

    if ptype == PT_ANY {
        if !negated {
            crate::pcre2_internal::memset(classbits as *mut c_void, 0xff, 32);
        }
        return;
    }

    c = 0;
    while c < 256 {
        prop = GET_UCD(c as u32);
        set_bit = false;

        match ptype {
            PT_LAMP => {
                chartype = (*prop).chartype as c_int;
                set_bit = chartype == ucp_Lu as c_int
                    || chartype == ucp_Ll as c_int
                    || chartype == ucp_Lt as c_int;
            }

            PT_GC => {
                set_bit = _pcre2_ucp_gentype_8[(*prop).chartype as usize] as u32 == pdata;
            }

            PT_PC => {
                set_bit = (*prop).chartype as u32 == pdata;
            }

            PT_SC => {
                set_bit = (*prop).script as u32 == pdata;
            }

            PT_SCX => {
                set_bit = (*prop).script as u32 == pdata
                    || {
                        let base = _pcre2_ucd_script_sets_8
                            .as_ptr()
                            .add(UCD_SCRIPTX_PROP(&*prop) as usize);
                        (*base.add((pdata / 32) as usize) & (1u32 << (pdata % 32))) != 0
                    };
            }

            PT_ALNUM => {
                gentype = _pcre2_ucp_gentype_8[(*prop).chartype as usize] as u32;
                set_bit = gentype == ucp_L as u32 || gentype == ucp_N as u32;
            }

            PT_SPACE | PT_PXSPACE => {
                set_bit = match c {
                    0x09 | 0x20 | 0xa0 => true,
                    0x0a | 0x0b | 0x0c | 0x0d | 0x85 => true,
                    _ => _pcre2_ucp_gentype_8[(*prop).chartype as usize] as u32 == ucp_Z as u32,
                };
            }

            PT_WORD => {
                chartype = (*prop).chartype as c_int;
                gentype = _pcre2_ucp_gentype_8[chartype as usize] as u32;
                set_bit = gentype == ucp_L as u32
                    || gentype == ucp_N as u32
                    || chartype == ucp_Mn as c_int
                    || chartype == ucp_Pc as c_int;
            }

            PT_UCNC => {
                set_bit = c == CHAR_DOLLAR_SIGN as c_int
                    || c == CHAR_COMMERCIAL_AT as c_int
                    || c == CHAR_GRAVE_ACCENT as c_int
                    || c >= 0xa0;
            }

            PT_BIDICL => {
                set_bit = UCD_BIDICLASS_PROP(&*prop) as u32 == pdata;
            }

            PT_BOOL => {
                set_bit = {
                    let base = _pcre2_ucd_boolprop_sets_8
                        .as_ptr()
                        .add(UCD_BPROPS_PROP(&*prop) as usize);
                    (*base.add((pdata / 32) as usize) & (1u32 << (pdata % 32))) != 0
                };
            }

            PT_PXGRAPH => {
                chartype = (*prop).chartype as c_int;
                gentype = _pcre2_ucp_gentype_8[chartype as usize] as u32;
                set_bit = gentype != ucp_Z as u32
                    && (gentype != ucp_C as u32 || chartype == ucp_Cf as c_int);
            }

            PT_PXPRINT => {
                chartype = (*prop).chartype as c_int;
                set_bit = chartype != ucp_Zl as c_int
                    && chartype != ucp_Zp as c_int
                    && (_pcre2_ucp_gentype_8[chartype as usize] as u32 != ucp_C as u32
                        || chartype == ucp_Cf as c_int);
            }

            PT_PXPUNCT => {
                gentype = _pcre2_ucp_gentype_8[(*prop).chartype as usize] as u32;
                set_bit = gentype == ucp_P as u32 || (c < 128 && gentype == ucp_S as u32);
            }

            _ => {
                /* PT_PXXDIGIT */
                set_bit = (c >= CHAR_0 as c_int && c <= CHAR_9 as c_int)
                    || (c >= CHAR_A as c_int && c <= CHAR_F as c_int)
                    || (c >= CHAR_a as c_int && c <= CHAR_f as c_int);
            }
        }

        if negated {
            set_bit = !set_bit;
        }
        if set_bit {
            *classbits |= (1 << (c & 0x7)) as u8;
        }
        if (c & 0x7) == 0x7 {
            classbits = classbits.add(1);
        }
        c += 1;
    }
}

/*************************************************
*           XClass related properties            *
*************************************************/

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

/* This function sets the overall range for characters < 256.
It also handles non-utf case folding. */

unsafe fn add_to_class(
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
        /* UTF mode / UCP mode. */
        if (options & (PCRE2_UTF | PCRE2_UCP)) != 0 {
            let turkish_i: bool =
                (xoptions & (PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT))
                    == PCRE2_EXTRA_TURKISH_CASING;
            if start < 128 {
                let lo_end: u32 = if classbits_end < 127 { classbits_end } else { 127 };
                c = start;
                while c <= lo_end {
                    if turkish_i && UCD_ANY_I(c) {
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

/* This function is used for adding a list of horizontal or vertical whitespace
characters to a class. */

unsafe fn add_list_to_class(
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
    p: *const u32,
) {
    let mut p = p;
    while *p < 256 {
        let mut n: u32 = 0;

        while *p.add((n + 1) as usize) == *p + n + 1 {
            n += 1;
        }
        add_to_class(options, xoptions, cb, *p, *p.add(n as usize));

        p = p.add((n + 1) as usize);
    }
}

/*************************************************
*    Add characters not in a list to a class     *
*************************************************/

/* This function is used for adding the complement of a list of horizontal or
vertical whitespace to a class. */

unsafe fn add_not_list_to_class(
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
    p: *const u32,
) {
    let mut p = p;
    if *p > 0 {
        add_to_class(options, xoptions, cb, 0, *p - 1);
    }
    while *p < 256 {
        while *p.add(1) == *p + 1 {
            p = p.add(1);
        }
        add_to_class(
            options,
            xoptions,
            cb,
            *p + 1,
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
    pcode: *mut *mut u8,
    negate_class: c_int,
    has_bitmap: *mut c_int,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
    lengthptr: *mut usize,
) -> *mut u32 {
    let negate_class = negate_class != 0;
    let mut pptr: *mut u32 = start_ptr;
    let mut code: *mut u8 = *pcode;
    let mut should_flip_negation: bool;
    let cbits: *const u8 = (*cb).cbits;
    /* Some functions such as add_to_class() or eclass processing
    expects that the bitset is stored in cb->classbits.classbits. */
    let classbits: *mut u8 = (*cb).classbits.classbits.as_mut_ptr();

    let utf: bool = (options & PCRE2_UTF) != 0;

    /* Helper variables for OP_XCLASS opcode (for characters > 255). */

    let mut xclass_props: u32;
    let mut class_uchardata: *mut u8;
    let mut cranges: *mut class_ranges;

    /* If an XClass contains a negative special such as \S, we need to flip the
    negation flag at the end. */

    should_flip_negation = false;

    /* XClass will be used when characters > 255 might match. */

    xclass_props = 0;

    cranges = core::ptr::null_mut();

    if utf {
        if !lengthptr.is_null() {
            cranges = compile_optimize_class(pptr, options, xoptions, cb);

            if cranges.is_null() {
                *errorcodeptr = ERR21;
                return core::ptr::null_mut();
            }

            /* Caching the pre-processed character ranges. */
            if !(*cb).last_data.is_null() {
                (*(*cb).last_data).next = &mut (*cranges).header;
            } else {
                (*cb).first_data = &mut (*cranges).header;
            }

            (*cb).last_data = &mut (*cranges).header;
        } else {
            /* Reuse the pre-processed character ranges. */
            cranges = (*cb).first_data as *mut class_ranges;
            (*cb).first_data = (*cranges).header.next;
        }

        if (*cranges).range_list_size > 0 {
            let ranges: *const u32 = cranges.add(1) as *const u32;

            if *ranges <= 255 {
                xclass_props |= XCLASS_HAS_8BIT_CHARS;
            }

            if *ranges.add(((*cranges).range_list_size - 1) as usize) == GET_MAX_CHAR_VALUE(utf)
                && *ranges.add(((*cranges).range_list_size - 2) as usize) <= 256
            {
                xclass_props |= XCLASS_HIGH_ANY;
            }
        }
    }

    class_uchardata = code.add(LINK_SIZE + 2); /* For XCLASS items */

    /* Initialize the 256-bit (32-byte) bit map to all zeros. */

    crate::pcre2_internal::memset(classbits as *mut c_void, 0, 32);

    /* Process items until end_ptr is reached. */

    'main_loop: loop {
        let mut meta: u32 = *pptr;
        pptr = pptr.add(1);
        let local_negate: bool;
        let mut posix_class: c_int;
        let mut taboffset: c_int;
        let mut tabopt: c_int;
        let mut pbits: class_bits_storage = core::mem::zeroed();
        let escape: c_int;
        let c: u32;

        /* Handle POSIX classes such as [:alpha:] etc. */
        match META_CODE(meta) {
            META_POSIX | META_POSIX_NEG => {
                local_negate = meta == META_POSIX_NEG;
                posix_class = *pptr as c_int;
                pptr = pptr.add(1);

                if local_negate {
                    should_flip_negation = true; /* Note negative special */
                }

                /* If matching is caseless, upper and lower are converted to alpha. */

                if (options & PCRE2_CASELESS) != 0 && posix_class <= 2 {
                    posix_class = 0;
                }

                /* When PCRE2_UCP is set, some POSIX classes are converted. */

                if (options & PCRE2_UCP) != 0 && (xoptions & PCRE2_EXTRA_ASCII_POSIX) == 0 {
                    let ptype: u32;

                    if posix_class == PC_GRAPH as c_int
                        || posix_class == PC_PRINT as c_int
                        || posix_class == PC_PUNCT as c_int
                    {
                        ptype = if posix_class == PC_GRAPH as c_int {
                            PT_PXGRAPH
                        } else if posix_class == PC_PRINT as c_int {
                            PT_PXPRINT
                        } else {
                            PT_PXPUNCT
                        };

                        _pcre2_update_classbits_8(ptype, 0, local_negate as c_int, classbits);

                        if (xclass_props & XCLASS_HIGH_ANY) == 0 {
                            if !lengthptr.is_null() {
                                *lengthptr += 3;
                            } else {
                                *class_uchardata =
                                    if local_negate { XCL_NOTPROP } else { XCL_PROP };
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
                }

                /* In the non-UCP case, we build the bit map for the POSIX class
                in a chunk of local store. */

                posix_class *= 3;

                /* Copy in the first table (always present) */

                crate::pcre2_internal::memcpy(
                    pbits.classbits.as_mut_ptr() as *mut c_void,
                    cbits.add(_pcre2_posix_class_maps8[posix_class as usize] as usize)
                        as *const c_void,
                    32,
                );

                /* If there is a second table, add or remove it as required. */

                taboffset = _pcre2_posix_class_maps8[(posix_class + 1) as usize] as c_int;
                tabopt = _pcre2_posix_class_maps8[(posix_class + 2) as usize] as c_int;

                if taboffset >= 0 {
                    if tabopt >= 0 {
                        for i in 0..32 {
                            pbits.classbits[i] |= *cbits.add((i as c_int + taboffset) as usize);
                        }
                    } else {
                        for i in 0..32 {
                            pbits.classbits[i] &=
                                !*cbits.add((i as c_int + taboffset) as usize);
                        }
                    }
                }

                /* Now see if we need to remove any special characters. */

                if tabopt < 0 {
                    tabopt = -tabopt;
                }
                if tabopt == 1 {
                    pbits.classbits[1] &= !0x3c;
                } else if tabopt == 2 {
                    pbits.classbits[11] &= 0x7f;
                }

                /* Add the POSIX table or its complement into the main table. */

                {
                    let classwords: *mut u32 = (*cb).classbits.classwords.as_mut_ptr();

                    if local_negate {
                        for i in 0..8 {
                            *classwords.add(i) |= !pbits.classwords[i];
                        }
                    } else {
                        for i in 0..8 {
                            *classwords.add(i) |= pbits.classwords[i];
                        }
                    }
                }

                /* Every class contains at least one < 256 character. */
                xclass_props |= XCLASS_HAS_8BIT_CHARS;
                continue 'main_loop; /* End of POSIX handling */
            }

            /* Other than POSIX classes, the only items we should encounter are
            \d-type escapes and literal characters (possibly as ranges). */
            META_BIGVALUE => {
                meta = *pptr;
                pptr = pptr.add(1);
            }

            META_ESCAPE => {
                escape = META_DATA(meta) as c_int;

                match escape {
                    ESC_d => {
                        for i in 0..32 {
                            *classbits.add(i) |= *cbits.add(i + cbit_digit);
                        }
                    }

                    ESC_D => {
                        should_flip_negation = true;
                        for i in 0..32 {
                            *classbits.add(i) |= !*cbits.add(i + cbit_digit);
                        }
                    }

                    ESC_w => {
                        for i in 0..32 {
                            *classbits.add(i) |= *cbits.add(i + cbit_word);
                        }
                    }

                    ESC_W => {
                        should_flip_negation = true;
                        for i in 0..32 {
                            *classbits.add(i) |= !*cbits.add(i + cbit_word);
                        }
                    }

                    ESC_s => {
                        for i in 0..32 {
                            *classbits.add(i) |= *cbits.add(i + cbit_space);
                        }
                    }

                    ESC_S => {
                        should_flip_negation = true;
                        for i in 0..32 {
                            *classbits.add(i) |= !*cbits.add(i + cbit_space);
                        }
                    }

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

                    ESC_p | ESC_P => {
                        let ptype: u32 = *pptr >> 16;
                        let pdata: u32 = *pptr & 0xffff;
                        pptr = pptr.add(1);

                        /* The "Any" is processed by PRIV(update_classbits)(). */
                        if ptype == PT_ANY {
                            if !utf && escape == ESC_p {
                                crate::pcre2_internal::memset(classbits as *mut c_void, 0xff, 32);
                            }
                            continue 'main_loop;
                        }

                        _pcre2_update_classbits_8(
                            ptype,
                            pdata,
                            (escape == ESC_P) as c_int,
                            classbits,
                        );

                        if (xclass_props & XCLASS_HIGH_ANY) == 0 {
                            if !lengthptr.is_null() {
                                *lengthptr += 3;
                            } else {
                                *class_uchardata =
                                    if escape == ESC_p { XCL_PROP } else { XCL_NOTPROP };
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

                /* Every non-property class contains at least one < 256 character. */
                xclass_props |= XCLASS_HAS_8BIT_CHARS;
                /* End handling \d-type escapes */
                continue 'main_loop;
            }

            _ => {
                /* Literals. */
                if meta < META_END {
                    /* break out of match, fall through to literal handling */
                } else {
                    /* Non-literals: end of class contents. */
                    break 'main_loop;
                }
            }
        }

        /* A literal character may be followed by a range meta. */

        c = meta;

        /* Remember if \r or \n were explicitly used */

        if c == CHAR_CR as u32 || c == CHAR_NL as u32 {
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

            if d == CHAR_CR as u32 || d == CHAR_NL as u32 {
                (*cb).external_flags |= PCRE2_HASCRORLF;
            }

            if !cranges.is_null() {
                continue 'main_loop;
            }
            xclass_props |= XCLASS_HAS_8BIT_CHARS;

            /* Not an EBCDIC special range */

            add_to_class(options, xoptions, cb, c, d);
            continue 'main_loop;
        } /* End of range handling */

        /* Character ranges are ignored when class_ranges is present. */
        if !cranges.is_null() {
            continue 'main_loop;
        }
        xclass_props |= XCLASS_HAS_8BIT_CHARS;
        /* Handle a single character. */

        add_to_class(options, xoptions, cb, meta, meta);
    } /* End of main class-processing loop */

    /* END_PROCESSING: */

    if !cranges.is_null() {
        let mut range: *mut u32 = cranges.add(1) as *mut u32;
        let end: *mut u32 = range.add((*cranges).range_list_size as usize);

        while range < end && *range < 256 {
            /* Add range to bitset. */
            add_to_class(
                if (options & (PCRE2_UTF | PCRE2_UCP)) != 0 {
                    options & !PCRE2_CASELESS
                } else {
                    options
                },
                xoptions,
                cb,
                *range,
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
                should_flip_negation = true;
                range = end;
            }

            while range < end {
                let mut range_start: u32 = *range;
                let range_end: u32 = *range.add(1);

                range = range.add(2);
                xclass_props |= XCLASS_REQUIRED;

                if range_start < 256 {
                    range_start = 256;
                }

                if !lengthptr.is_null() {
                    if utf {
                        *lengthptr += 1;

                        if range_start < range_end {
                            *lengthptr +=
                                crate::pcre2_ord2utf::_pcre2_ord2utf_8(range_start, class_uchardata)
                                    as usize;
                        }

                        *lengthptr +=
                            crate::pcre2_ord2utf::_pcre2_ord2utf_8(range_end, class_uchardata)
                                as usize;
                        continue;
                    }

                    *lengthptr += if range_start < range_end { 3 } else { 2 };
                    continue;
                }

                if utf {
                    if range_start < range_end {
                        *class_uchardata = XCL_RANGE;
                        class_uchardata = class_uchardata.add(1);
                        class_uchardata = class_uchardata.add(
                            crate::pcre2_ord2utf::_pcre2_ord2utf_8(range_start, class_uchardata)
                                as usize,
                        );
                    } else {
                        *class_uchardata = XCL_SINGLE;
                        class_uchardata = class_uchardata.add(1);
                    }

                    class_uchardata = class_uchardata.add(
                        crate::pcre2_ord2utf::_pcre2_ord2utf_8(range_end, class_uchardata) as usize,
                    );
                    continue;
                }
                /* 8-bit non-UTF: no wide chars. */
            }

            if lengthptr.is_null() {
                (*(*cb).cx).memctl.free.unwrap()(
                    cranges as *mut c_void,
                    (*(*cb).cx).memctl.memory_data,
                );
            }
        }
    }

    'done: {
        /* If there are characters with values > 255, or Unicode property settings,
        we have to compile an extended class. */

        if (xclass_props & XCLASS_REQUIRED) != 0 {
            let previous: *mut u8 = code;

            if (xclass_props & XCLASS_HAS_CHAR_LISTS) == 0 {
                *class_uchardata = XCL_END; /* Marks the end of extra data */
                class_uchardata = class_uchardata.add(1);
            }
            *code = OP_XCLASS;
            code = code.add(1);
            code = code.add(LINK_SIZE);
            *code = if negate_class { XCL_NOT } else { 0 };
            if (xclass_props & XCLASS_HAS_PROPS) != 0 {
                *code |= XCL_HASPROP;
            }

            /* If the map is required, move up the extra data to make room for it. */

            if (xclass_props & XCLASS_HAS_8BIT_CHARS) != 0 || !has_bitmap.is_null() {
                if negate_class {
                    let classwords: *mut u32 = (*cb).classbits.classwords.as_mut_ptr();
                    for i in 0..8 {
                        *classwords.add(i) = !*classwords.add(i);
                    }
                }

                if has_bitmap.is_null() {
                    *code |= XCL_MAP;
                    code = code.add(1);
                    crate::pcre2_internal::memmove(
                        code.add(32) as *mut c_void,
                        code as *const c_void,
                        class_uchardata.offset_from(code) as usize,
                    );
                    crate::pcre2_internal::memcpy(code as *mut c_void, classbits as *const c_void, 32);
                    code = class_uchardata.add(32);
                } else {
                    code = class_uchardata;
                    if (xclass_props & XCLASS_HAS_8BIT_CHARS) != 0 {
                        *has_bitmap = 1;
                    }
                }
            } else {
                code = class_uchardata;
            }

            if (xclass_props & XCLASS_HAS_CHAR_LISTS) != 0 {
                /* Char lists size is an even number. */
                let mut char_lists_size: usize = (*cranges).char_lists_size;

                if !lengthptr.is_null() {
                    char_lists_size =
                        CLIST_ALIGN_TO(char_lists_size, core::mem::size_of::<u32>());

                    *lengthptr += 2 + LINK_SIZE;

                    (*cb).char_lists_size += char_lists_size;

                    char_lists_size /= core::mem::size_of::<u8>();

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
                    data = ((*cb).start_code as *mut u8).sub((*cb).char_lists_size);

                    crate::pcre2_internal::memcpy(
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

                    /* If we added padding to align the list, initialize the bytes. */

                    if (char_lists_size & 0x2) != 0 {
                        *(data as *mut u16).sub(1) = 0xdead;
                    }

                    (*cb).char_lists_size =
                        CLIST_ALIGN_TO(char_lists_size, core::mem::size_of::<u32>());

                    (*(*cb).cx).memctl.free.unwrap()(
                        cranges as *mut c_void,
                        (*(*cb).cx).memctl.memory_data,
                    );
                }
            }

            /* Now fill in the complete length of the item */

            PUT(previous, 1, code.offset_from(previous) as u32);
            break 'done; /* End of class handling */
        }

        /* If there are no characters > 255, set the opcode to OP_CLASS or
        OP_NCLASS. */

        if negate_class {
            let classwords: *mut u32 = (*cb).classbits.classwords.as_mut_ptr();

            for i in 0..8 {
                *classwords.add(i) = !*classwords.add(i);
            }
        }

        if (!utf || negate_class != should_flip_negation)
            && (*cb).classbits.classwords[0] == !0u32
        {
            let classwords: *const u32 = (*cb).classbits.classwords.as_ptr();
            let mut i: usize = 0;

            while i < 8 {
                if *classwords.add(i) != !0u32 {
                    break;
                }
                i += 1;
            }

            if i == 8 {
                *code = OP_ALLANY;
                code = code.add(1);
                break 'done; /* End of class handling */
            }
        }

        *code = if negate_class == should_flip_negation {
            OP_CLASS
        } else {
            OP_NCLASS
        };
        code = code.add(1);
        crate::pcre2_internal::memcpy(code as *mut c_void, classbits as *const c_void, 32);
        code = code.add(32);
    } /* DONE */

    *pcode = code;
    pptr.sub(1)
}

/* ===================================================================*/
/* Here follows a block of ECLASS-compiling functions. */

/* This function folds one operand using the negation operator. */

unsafe fn fold_negation(
    pop_info: *mut eclass_op_info,
    lengthptr: *mut usize,
    preserve_classbits: bool,
) {
    /* If the chunk of stack code is already composed of multiple ops, we won't
    descend in. */

    if (*pop_info).op_single_type == 0 {
        if !lengthptr.is_null() {
            *lengthptr += 1;
        } else {
            *(*pop_info).code_start.add((*pop_info).length) = ECL_NOT;
        }
        (*pop_info).length += 1;
    }
    /* Otherwise, it's a nice single-op item. */
    else if (*pop_info).op_single_type == ECL_ANY || (*pop_info).op_single_type == ECL_NONE {
        (*pop_info).op_single_type = if (*pop_info).op_single_type == ECL_NONE {
            ECL_ANY
        } else {
            ECL_NONE
        };
        if lengthptr.is_null() {
            *(*pop_info).code_start = (*pop_info).op_single_type;
        }
    } else {
        if lengthptr.is_null() {
            *(*pop_info).code_start.add(1 + LINK_SIZE) ^= XCL_NOT;
        }
    }

    if !preserve_classbits {
        for i in 0..8 {
            (*pop_info).bits.classwords[i] = !(*pop_info).bits.classwords[i];
        }
    }
}

/* This function folds together two operands using a binary operator. */

unsafe fn fold_binary(
    op: c_int,
    lhs_op_info: *mut eclass_op_info,
    rhs_op_info: *mut eclass_op_info,
    lengthptr: *mut usize,
) {
    match op {
        /* ECL_AND truth table. */
        x if x == ECL_AND as c_int => {
            if (*rhs_op_info).op_single_type == ECL_ANY {
                /* no-op: drop the RHS */
            } else if (*lhs_op_info).op_single_type == ECL_ANY {
                /* no-op: drop the LHS, and memmove the RHS into its place */
                if lengthptr.is_null() {
                    crate::pcre2_internal::memmove(
                        (*lhs_op_info).code_start as *mut c_void,
                        (*rhs_op_info).code_start as *const c_void,
                        (*rhs_op_info).length,
                    );
                }
                (*lhs_op_info).length = (*rhs_op_info).length;
                (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
            } else if (*rhs_op_info).op_single_type == ECL_NONE {
                /* the result is ECL_NONE: write into the LHS */
                if lengthptr.is_null() {
                    *(*lhs_op_info).code_start = ECL_NONE;
                }
                (*lhs_op_info).length = 1;
                (*lhs_op_info).op_single_type = ECL_NONE;
            } else if (*lhs_op_info).op_single_type == ECL_NONE {
                /* the result is ECL_NONE: drop the RHS */
            } else {
                /* Both are either ECL_XCLASS, or compound operations. */
                if !lengthptr.is_null() {
                    *lengthptr += 1;
                } else {
                    *(*rhs_op_info).code_start.add((*rhs_op_info).length) = ECL_AND;
                }
                (*lhs_op_info).length += (*rhs_op_info).length + 1;
                (*lhs_op_info).op_single_type = 0;
            }

            for i in 0..8 {
                (*lhs_op_info).bits.classwords[i] &= (*rhs_op_info).bits.classwords[i];
            }
        }

        /* ECL_OR truth table. */
        x if x == ECL_OR as c_int => {
            if (*rhs_op_info).op_single_type == ECL_NONE {
                /* no-op: drop the RHS */
            } else if (*lhs_op_info).op_single_type == ECL_NONE {
                /* no-op: drop the LHS, and memmove the RHS into its place */
                if lengthptr.is_null() {
                    crate::pcre2_internal::memmove(
                        (*lhs_op_info).code_start as *mut c_void,
                        (*rhs_op_info).code_start as *const c_void,
                        (*rhs_op_info).length,
                    );
                }
                (*lhs_op_info).length = (*rhs_op_info).length;
                (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
            } else if (*rhs_op_info).op_single_type == ECL_ANY {
                /* the result is ECL_ANY: write into the LHS */
                if lengthptr.is_null() {
                    *(*lhs_op_info).code_start = ECL_ANY;
                }
                (*lhs_op_info).length = 1;
                (*lhs_op_info).op_single_type = ECL_ANY;
            } else if (*lhs_op_info).op_single_type == ECL_ANY {
                /* the result is ECL_ANY: drop the RHS */
            } else {
                /* Both are either ECL_XCLASS, or compound operations. */
                if !lengthptr.is_null() {
                    *lengthptr += 1;
                } else {
                    *(*rhs_op_info).code_start.add((*rhs_op_info).length) = ECL_OR;
                }
                (*lhs_op_info).length += (*rhs_op_info).length + 1;
                (*lhs_op_info).op_single_type = 0;
            }

            for i in 0..8 {
                (*lhs_op_info).bits.classwords[i] |= (*rhs_op_info).bits.classwords[i];
            }
        }

        /* ECL_XOR truth table. */
        x if x == ECL_XOR as c_int => {
            if (*rhs_op_info).op_single_type == ECL_NONE {
                /* no-op: drop the RHS */
            } else if (*lhs_op_info).op_single_type == ECL_NONE {
                /* no-op: drop the LHS, and memmove the RHS into its place */
                if lengthptr.is_null() {
                    crate::pcre2_internal::memmove(
                        (*lhs_op_info).code_start as *mut c_void,
                        (*rhs_op_info).code_start as *const c_void,
                        (*rhs_op_info).length,
                    );
                }
                (*lhs_op_info).length = (*rhs_op_info).length;
                (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
            } else if (*rhs_op_info).op_single_type == ECL_ANY {
                /* the result is !LHS: fold in the negation, and drop the RHS */
                fold_negation(lhs_op_info, lengthptr, true);
            } else if (*lhs_op_info).op_single_type == ECL_ANY {
                /* the result is !RHS. */
                if lengthptr.is_null() {
                    crate::pcre2_internal::memmove(
                        (*lhs_op_info).code_start as *mut c_void,
                        (*rhs_op_info).code_start as *const c_void,
                        (*rhs_op_info).length,
                    );
                }
                (*lhs_op_info).length = (*rhs_op_info).length;
                (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;

                fold_negation(lhs_op_info, lengthptr, true);
            } else {
                /* Both are either ECL_XCLASS, or compound operations. */
                if !lengthptr.is_null() {
                    *lengthptr += 1;
                } else {
                    *(*rhs_op_info).code_start.add((*rhs_op_info).length) = ECL_XOR;
                }
                (*lhs_op_info).length += (*rhs_op_info).length + 1;
                (*lhs_op_info).op_single_type = 0;
            }

            for i in 0..8 {
                (*lhs_op_info).bits.classwords[i] ^= (*rhs_op_info).bits.classwords[i];
            }
        }

        _ => {
            /* LCOV_EXCL_START */
            /* LCOV_EXCL_STOP */
        }
    }
}

/* This function consumes a group of implicitly-unioned class elements. */

unsafe fn compile_class_operand(
    context: *mut eclass_context,
    negated: bool,
    pptr: *mut *mut u32,
    pcode: *mut *mut u8,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut usize,
) -> bool {
    let mut ptr: *mut u32 = *pptr;
    let mut prev_ptr: *mut u32;
    let mut code: *mut u8 = *pcode;
    let code_start: *mut u8 = code;
    let prev_length: usize = if !lengthptr.is_null() { *lengthptr } else { 0 };
    let extra_length: usize;
    let meta: u32 = META_CODE(*ptr);

    'outer: {
        match meta {
            META_CLASS_EMPTY_NOT | META_CLASS_EMPTY => {
                ptr = ptr.add(1);
                (*pop_info).length = 1;
                if (meta == META_CLASS_EMPTY) == negated {
                    (*pop_info).op_single_type = ECL_ANY;
                    *code = ECL_ANY;
                    code = code.add(1);
                    crate::pcre2_internal::memset((*pop_info).bits.classbits.as_mut_ptr() as *mut c_void, 0xff, 32);
                } else {
                    (*pop_info).op_single_type = ECL_NONE;
                    *code = ECL_NONE;
                    code = code.add(1);
                    crate::pcre2_internal::memset((*pop_info).bits.classbits.as_mut_ptr() as *mut c_void, 0, 32);
                }
            }

            META_CLASS | META_CLASS_NOT => {
                if (*ptr & CLASS_IS_ECLASS) != 0 {
                    if !compile_eclass_nested(context, negated, &mut ptr, &mut code, pop_info, lengthptr)
                    {
                        return false;
                    }

                    ptr = ptr.add(1);
                    break 'outer;
                }

                ptr = ptr.add(1);
                /* Fall through to default. */
                if compile_class_operand_default(
                    context, negated, meta, &mut ptr, &mut code, code_start, prev_length,
                    pop_info, lengthptr,
                )
                .is_none()
                {
                    return false;
                }
            }

            _ => {
                if compile_class_operand_default(
                    context, negated, meta, &mut ptr, &mut code, code_start, prev_length,
                    pop_info, lengthptr,
                )
                .is_none()
                {
                    return false;
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
    } /* DONE */

    *pptr = ptr;
    *pcode = code;
    true
}

/* Helper: the "default" fall-through case of compile_class_operand.
Returns None to signal that the caller should return false. */

unsafe fn compile_class_operand_default(
    context: *mut eclass_context,
    negated: bool,
    meta: u32,
    ptr: &mut *mut u32,
    code: &mut *mut u8,
    code_start: *mut u8,
    prev_length: usize,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut usize,
) -> Option<()> {
    let prev_ptr: *mut u32;
    let extra_length: usize;

    /* Scan forward characters, ranges, and properties. */

    prev_ptr = *ptr;
    *ptr = _pcre2_compile_class_not_nested_8(
        (*context).options,
        (*context).xoptions,
        *ptr,
        code,
        ((meta != META_CLASS_NOT) == negated) as c_int,
        &mut (*context).needs_bitmap as *mut bool as *mut c_int,
        (*context).errorcodeptr,
        (*context).cb,
        lengthptr,
    );
    if (*ptr).is_null() {
        return None;
    }

    /* We must have a 100% guarantee that ptr increases. */
    if *ptr <= prev_ptr {
        return None;
    }

    /* If we fell through above, consume the closing ']'. */
    if meta == META_CLASS || meta == META_CLASS_NOT {
        *ptr = (*ptr).add(1);
    }

    extra_length = if !lengthptr.is_null() {
        *lengthptr - prev_length
    } else {
        0
    };

    /* Easiest case: convert OP_ALLANY to ECL_ANY */

    if *code_start == OP_ALLANY {
        (*pop_info).length = 1;
        (*pop_info).op_single_type = ECL_ANY;
        *code_start = ECL_ANY;
        crate::pcre2_internal::memset((*pop_info).bits.classbits.as_mut_ptr() as *mut c_void, 0xff, 32);
    }
    /* For OP_CLASS and OP_NCLASS, hoist out the bitmap. */
    else if *code_start == OP_CLASS || *code_start == OP_NCLASS {
        (*pop_info).length = 1;
        (*pop_info).op_single_type = if *code_start == OP_CLASS { ECL_NONE } else { ECL_ANY };
        *code_start = (*pop_info).op_single_type;
        crate::pcre2_internal::memcpy(
            (*pop_info).bits.classbits.as_mut_ptr() as *mut c_void,
            code_start.add(1) as *const c_void,
            32,
        );
        /* Rewind the code pointer, but adjust *lengthptr. */
        if !lengthptr.is_null() {
            *lengthptr += code.offset_from(code_start.add(1)) as usize;
        }
        *code = code_start.add(1);

        if !(*context).needs_bitmap && *code_start == ECL_NONE {
            let classwords: *mut u32 = (*pop_info).bits.classwords.as_mut_ptr();

            for i in 0..8 {
                if *classwords.add(i) != 0 {
                    (*context).needs_bitmap = true;
                    break;
                }
            }
        } else {
            (*context).needs_bitmap = true;
        }
    }
    /* Finally, for OP_XCLASS we hoist out the bitmap (if any). */
    else {
        *code_start = ECL_XCLASS;
        (*pop_info).op_single_type = ECL_XCLASS;

        crate::pcre2_internal::memcpy(
            (*pop_info).bits.classbits.as_mut_ptr() as *mut c_void,
            (*(*context).cb).classbits.classbits.as_ptr() as *const c_void,
            32,
        );
        (*pop_info).length = code.offset_from(code_start) as usize + extra_length;
    }

    Some(())
}

/* This function consumes a group of implicitly-unioned class elements. */

unsafe fn compile_class_juxtaposition(
    context: *mut eclass_context,
    negated: bool,
    pptr: *mut *mut u32,
    pcode: *mut *mut u8,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut usize,
) -> bool {
    let mut ptr: *mut u32 = *pptr;
    let mut code: *mut u8 = *pcode;

    /* Because it's a non-empty class, there must be an operand at the start. */
    if !compile_class_operand(context, negated, &mut ptr, &mut code, pop_info, lengthptr) {
        return false;
    }

    while *ptr != META_CLASS_END && !(*ptr >= META_ECLASS_AND && *ptr <= META_ECLASS_NOT) {
        let op: c_int;
        let rhs_negated: bool;
        let mut rhs_op_info: eclass_op_info = core::mem::zeroed();

        if negated {
            /* !(A juxtapose B)  ->  !A && !B */
            op = ECL_AND as c_int;
            rhs_negated = true;
        } else {
            /* A juxtapose B  ->  A || B */
            op = ECL_OR as c_int;
            rhs_negated = false;
        }

        /* An operand must follow the operator. */
        if !compile_class_operand(context, rhs_negated, &mut ptr, &mut code, &mut rhs_op_info, lengthptr)
        {
            return false;
        }

        /* Convert infix to postfix (RPN). */
        fold_binary(op, pop_info, &mut rhs_op_info, lengthptr);
        if lengthptr.is_null() {
            code = (*pop_info).code_start.add((*pop_info).length);
        }
    }

    *pptr = ptr;
    *pcode = code;
    true
}

/* This function consumes unary prefix operators. */

unsafe fn compile_class_unary(
    context: *mut eclass_context,
    mut negated: bool,
    pptr: *mut *mut u32,
    pcode: *mut *mut u8,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut usize,
) -> bool {
    let mut ptr: *mut u32 = *pptr;

    while *ptr == META_ECLASS_NOT {
        ptr = ptr.add(1);
        negated = !negated;
    }

    *pptr = ptr;
    /* Because it's a non-empty class, there must be an operand. */
    if !compile_class_juxtaposition(context, negated, pptr, pcode, pop_info, lengthptr) {
        return false;
    }

    true
}

/* This function consumes tightly-binding binary operators. */

unsafe fn compile_class_binary_tight(
    context: *mut eclass_context,
    negated: bool,
    pptr: *mut *mut u32,
    pcode: *mut *mut u8,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut usize,
) -> bool {
    let mut ptr: *mut u32 = *pptr;
    let mut code: *mut u8 = *pcode;

    /* Because it's a non-empty class, there must be an operand at the start. */
    if !compile_class_unary(context, negated, &mut ptr, &mut code, pop_info, lengthptr) {
        return false;
    }

    while *ptr == META_ECLASS_AND {
        let op: c_int;
        let rhs_negated: bool;
        let mut rhs_op_info: eclass_op_info = core::mem::zeroed();

        if negated {
            /* !(A && B)  ->  !A || !B */
            op = ECL_OR as c_int;
            rhs_negated = true;
        } else {
            /* A && B  ->  A && B */
            op = ECL_AND as c_int;
            rhs_negated = false;
        }

        ptr = ptr.add(1);

        /* An operand must follow the operator. */
        if !compile_class_unary(context, rhs_negated, &mut ptr, &mut code, &mut rhs_op_info, lengthptr)
        {
            return false;
        }

        /* Convert infix to postfix (RPN). */
        fold_binary(op, pop_info, &mut rhs_op_info, lengthptr);
        if lengthptr.is_null() {
            code = (*pop_info).code_start.add((*pop_info).length);
        }
    }

    *pptr = ptr;
    *pcode = code;
    true
}

/* This function consumes loosely-binding binary operators. */

unsafe fn compile_class_binary_loose(
    context: *mut eclass_context,
    negated: bool,
    pptr: *mut *mut u32,
    pcode: *mut *mut u8,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut usize,
) -> bool {
    let mut ptr: *mut u32 = *pptr;
    let mut code: *mut u8 = *pcode;

    /* Because it's a non-empty class, there must be an operand at the start. */
    if !compile_class_binary_tight(context, negated, &mut ptr, &mut code, pop_info, lengthptr) {
        return false;
    }

    while *ptr >= META_ECLASS_OR && *ptr <= META_ECLASS_XOR {
        let op: c_int;
        let op_neg: bool;
        let rhs_negated: bool;
        let mut rhs_op_info: eclass_op_info = core::mem::zeroed();

        if negated {
            /* The whole expression is being negated. */
            op = if *ptr == META_ECLASS_OR {
                ECL_AND as c_int
            } else if *ptr == META_ECLASS_SUB {
                ECL_OR as c_int
            } else {
                ECL_XOR as c_int
            };
            op_neg = *ptr == META_ECLASS_XOR;
            rhs_negated = *ptr != META_ECLASS_SUB;
        } else {
            op = if *ptr == META_ECLASS_OR {
                ECL_OR as c_int
            } else if *ptr == META_ECLASS_SUB {
                ECL_AND as c_int
            } else {
                ECL_XOR as c_int
            };
            op_neg = false;
            rhs_negated = *ptr == META_ECLASS_SUB;
        }

        ptr = ptr.add(1);

        /* An operand must follow the operator. */
        if !compile_class_binary_tight(
            context,
            rhs_negated,
            &mut ptr,
            &mut code,
            &mut rhs_op_info,
            lengthptr,
        ) {
            return false;
        }

        /* Convert infix to postfix (RPN). */
        fold_binary(op, pop_info, &mut rhs_op_info, lengthptr);
        if op_neg {
            fold_negation(pop_info, lengthptr, false);
        }
        if lengthptr.is_null() {
            code = (*pop_info).code_start.add((*pop_info).length);
        }
    }

    *pptr = ptr;
    *pcode = code;
    true
}

/* This function converts the META codes in pptr into opcodes written to
pcode. The pptr must start at a META_CLASS or META_CLASS_NOT. */

unsafe fn compile_eclass_nested(
    context: *mut eclass_context,
    mut negated: bool,
    pptr: *mut *mut u32,
    pcode: *mut *mut u8,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut usize,
) -> bool {
    let ptr: *mut u32 = *pptr;

    if *ptr == (META_CLASS_NOT | CLASS_IS_ECLASS) {
        negated = !negated;
    }

    (*pptr) = (*pptr).add(1);

    /* Because it's a non-empty class, there must be an operand at the start. */
    if !compile_class_binary_loose(context, negated, pptr, pcode, pop_info, lengthptr) {
        return false;
    }

    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_class_nested_8(
    options: u32,
    xoptions: u32,
    pptr: *mut *mut u32,
    pcode: *mut *mut u8,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
    lengthptr: *mut usize,
) -> c_int {
    let mut context: eclass_context = core::mem::zeroed();
    let mut op_info: eclass_op_info = core::mem::zeroed();
    let previous_length: usize = if !lengthptr.is_null() { *lengthptr } else { 0 };
    let mut code: *mut u8 = *pcode;
    let previous: *mut u8;
    let mut allbitsone: bool = true;

    context.needs_bitmap = false;
    context.options = options;
    context.xoptions = xoptions;
    context.errorcodeptr = errorcodeptr;
    context.cb = cb;

    previous = code;
    *code = OP_ECLASS;
    code = code.add(1);
    code = code.add(LINK_SIZE);
    *code = 0; /* Flags, currently zero. */
    code = code.add(1);
    if !compile_eclass_nested(&mut context, false, pptr, &mut code, &mut op_info, lengthptr) {
        return 0;
    }

    if !lengthptr.is_null() {
        *lengthptr += code.offset_from(previous) as usize;
        code = previous;
    }

    /* Do some useful counting of what's in the bitmap. */
    for i in 0..8 {
        if op_info.bits.classwords[i] != 0xffffffff {
            allbitsone = false;
            break;
        }
    }

    /* After constant-folding, it may turn out to be a simple class after all. */

    if op_info.op_single_type != 0 {
        /* Rewind back over the OP_ECLASS. */
        code = previous;

        /* If the bits are all ones, and the "high characters" are all matched
        too, we use a special-cased encoding of OP_ALLANY. */

        if op_info.op_single_type == ECL_ANY && allbitsone {
            if !lengthptr.is_null() {
                *lengthptr -= 1;
            }
            *code = OP_ALLANY;
            code = code.add(1);
        }
        /* If the high bits are all matched / all not-matched. */
        else if op_info.op_single_type == ECL_ANY || op_info.op_single_type == ECL_NONE {
            let required_len: usize = 1 + 32;

            if !lengthptr.is_null() {
                if required_len > (*lengthptr - previous_length) {
                    *lengthptr = previous_length + required_len;
                }
            }

            if !lengthptr.is_null() {
                *lengthptr -= required_len;
            }
            *code = if op_info.op_single_type == ECL_ANY {
                OP_NCLASS
            } else {
                OP_CLASS
            };
            code = code.add(1);
            crate::pcre2_internal::memcpy(
                code as *mut c_void,
                op_info.bits.classbits.as_ptr() as *const c_void,
                32,
            );
            code = code.add(32);
        }
        /* Otherwise, we have an ECL_XCLASS. */
        else {
            let need_map: bool = context.needs_bitmap;
            let required_len: usize;

            required_len = op_info.length + (if need_map { 32 } else { 0 });

            if !lengthptr.is_null() {
                /* Don't unconditionally request all the space we need. */
                if required_len > (*lengthptr - previous_length) {
                    *lengthptr = previous_length + required_len;
                }

                *lengthptr -= 1 + LINK_SIZE + 1;
                *code = OP_XCLASS;
                code = code.add(1);
                PUT(code, 0, (1 + LINK_SIZE + 1) as u32);
                code = code.add(LINK_SIZE);
                *code = 0;
                code = code.add(1);
            } else {
                let rest: *mut u8;
                let rest_len: usize;
                let flags: u8;

                /* 1 unit: OP_XCLASS | LINK_SIZE units | 1 unit: flags | ...rest */
                rest = op_info.code_start.add(1 + LINK_SIZE + 1);
                rest_len = op_info.code_start.add(op_info.length).offset_from(rest) as usize;

                /* First read any data we use, before memmove splats it. */
                flags = *op_info.code_start.add(1 + LINK_SIZE);

                /* Next do the memmove before any writes. */
                crate::pcre2_internal::memmove(
                    code.add(1 + LINK_SIZE + 1 + (if need_map { 32 } else { 0 })) as *mut c_void,
                    rest as *const c_void,
                    rest_len,
                );

                /* Finally write the header data. */
                *code = OP_XCLASS;
                code = code.add(1);
                PUT(code, 0, required_len as u32);
                code = code.add(LINK_SIZE);
                *code = flags | (if need_map { XCL_MAP } else { 0 });
                code = code.add(1);
                if need_map {
                    crate::pcre2_internal::memcpy(
                        code as *mut c_void,
                        op_info.bits.classbits.as_ptr() as *const c_void,
                        32,
                    );
                    code = code.add(32);
                }
                code = code.add(rest_len);
            }
        }
    }
    /* Otherwise, we're going to keep the OP_ECLASS. */
    else {
        let need_map: bool = context.needs_bitmap;
        let required_len: usize =
            1 + LINK_SIZE + 1 + (if need_map { 32 } else { 0 }) + op_info.length;

        if !lengthptr.is_null() {
            if required_len > (*lengthptr - previous_length) {
                *lengthptr = previous_length + required_len;
            }

            *lengthptr -= 1 + LINK_SIZE + 1;
            *code = OP_ECLASS;
            code = code.add(1);
            PUT(code, 0, (1 + LINK_SIZE + 1) as u32);
            code = code.add(LINK_SIZE);
            *code = 0;
            code = code.add(1);
        } else {
            if need_map {
                let map_start: *mut u8 = previous.add(1 + LINK_SIZE + 1);
                *previous.add(1 + LINK_SIZE) |= ECL_MAP;
                crate::pcre2_internal::memmove(
                    map_start.add(32) as *mut c_void,
                    map_start as *const c_void,
                    code.offset_from(map_start) as usize,
                );
                crate::pcre2_internal::memcpy(
                    map_start as *mut c_void,
                    op_info.bits.classbits.as_ptr() as *const c_void,
                    32,
                );
                code = code.add(32);
            }
            PUT(previous, 1, code.offset_from(previous) as u32);
        }
    }

    *pcode = code;
    1
}

/* End of pcre2_compile_class.rs */
