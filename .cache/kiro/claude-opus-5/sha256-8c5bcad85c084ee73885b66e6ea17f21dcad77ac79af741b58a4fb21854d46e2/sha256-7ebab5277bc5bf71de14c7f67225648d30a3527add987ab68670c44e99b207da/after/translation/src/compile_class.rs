//! Translation of `c_src/src/pcre2_compile_class.c`.
//!
//! Character-class and extended-class compilation. Built for the 8-bit library
//! with `SUPPORT_UNICODE` (hence `SUPPORT_WIDE_CHARS`), `LINK_SIZE == 2`, no
//! JIT, no EBCDIC, no `PCRE2_DEBUG`.

#![allow(non_snake_case, non_upper_case_globals, unused_parens, dead_code)]

use core::ffi::{c_int, c_void};
use core::ptr;

use crate::chars::*;
use crate::compile_internal::*;
use crate::internal::*;
use crate::opcodes::*;
use crate::ord2utf::ord2utf;
use crate::ucd::{UCD_BOOLPROP_SETS, UCD_CASELESS_SETS, UCD_NOCASE_RANGES,
    UCD_NOCASE_RANGES_SIZE, UCD_SCRIPT_SETS, UCD_TURKISH_DOTTED_I_CASESET};
use crate::ucp::*;

/*************************************************
*        POSIX class bit-map offset table        *
*************************************************/

/* Table of class bit maps for each POSIX class. Each class is formed from a
base map, with an optional addition or removal of another map. The triples in
the table consist of the base map offset, second map offset or -1 if no second
map, and a non-negative value for map addition or a negative value for map
subtraction (if there are two maps). The absolute value of the third field has
these meanings: 0 => no tweaking, 1 => remove vertical space characters, 2 =>
remove underscore.

This is defined in pcre2_compile.c in the C sources, but for this translation
it lives here so it can be exported alongside the other class helpers. */

pub static POSIX_CLASS_MAPS: [c_int; 42] = [
    cbit_word as c_int,   cbit_digit as c_int, -2,            /* alpha */
    cbit_lower as c_int,  -1,                   0,            /* lower */
    cbit_upper as c_int,  -1,                   0,            /* upper */
    cbit_word as c_int,   -1,                   2,            /* alnum */
    cbit_print as c_int,  cbit_cntrl as c_int,  0,            /* ascii */
    cbit_space as c_int,  -1,                   1,            /* blank */
    cbit_cntrl as c_int,  -1,                   0,            /* cntrl */
    cbit_digit as c_int,  -1,                   0,            /* digit */
    cbit_graph as c_int,  -1,                   0,            /* graph */
    cbit_print as c_int,  -1,                   0,            /* print */
    cbit_punct as c_int,  -1,                   0,            /* punct */
    cbit_space as c_int,  -1,                   0,            /* space */
    cbit_word as c_int,   -1,                   0,            /* word  */
    cbit_xdigit as c_int, -1,                   0,            /* xdigit */
];

/// Exported as `_pcre2_posix_class_maps8` (note: no underscore before the `8`,
/// because the C macro is `_pcre2_posix_class_maps` and `PCRE2_SUFFIX`
/// concatenates the width directly).
#[unsafe(no_mangle)]
pub static _pcre2_posix_class_maps8: [c_int; 42] = POSIX_CLASS_MAPS;

/*************************************************
*             eclass_context struct              *
*************************************************/

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

/* Codes for the constant-folded operand type. Match the ECL_* opcodes. */

/*************************************************
*               Heapsort algorithm               *
*************************************************/

unsafe fn do_heapify(buffer: *mut u32, size: usize, mut i: usize) {
    unsafe {
        loop {
            let mut max: usize = i;
            let left: usize = (i << 1) + 2;
            let right: usize = left + 2;

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

const PARSE_CLASS_UTF: u32 = 0x1;
const PARSE_CLASS_CASELESS_UTF: u32 = 0x2;
const PARSE_CLASS_RESTRICTED_UTF: u32 = 0x4;
const PARSE_CLASS_TURKISH_UTF: u32 = 0x8;

/* Get the range of nocase characters which includes the 'c' character passed
as argument, or directly follows 'c'. */

unsafe fn get_nocase_range(c: u32) -> *const u32 {
    unsafe {
        let mut left: u32 = 0;
        let mut right: u32 = UCD_NOCASE_RANGES_SIZE;
        let mut middle: u32;

        if c > MAX_UTF_CODE_POINT {
            return UCD_NOCASE_RANGES.as_ptr().add(right as usize);
        }

        loop {
            /* Range end of the middle element. */
            middle = ((left + right) >> 1) | 0x1;

            if UCD_NOCASE_RANGES[middle as usize] <= c {
                left = middle + 1;
            } else if middle > 1 && UCD_NOCASE_RANGES[(middle - 2) as usize] > c {
                right = middle - 1;
            } else {
                return UCD_NOCASE_RANGES.as_ptr().add((middle - 1) as usize);
            }
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
    unsafe {
        let mut new_start: u32 = start;
        let mut new_end: u32 = end;
        let mut c: u32 = start;
        let mut list: *const u32;
        let mut tmp: [u32; 3] = [0; 3];
        let mut result: usize = 2;
        let mut skip_range: *const u32 = get_nocase_range(c);
        let mut skip_start: u32 = *skip_range.add(0);

        /* PCRE2_ASSERT(options & PARSE_CLASS_UTF); */

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
                && ucd_any_i(c)
            {
                co = UCD_TURKISH_DOTTED_I_CASESET + (if ucd_dotted_i(c) { 0 } else { 3 });
            } else if {
                co = ucd_caseset(c);
                co != 0
            } && (options & PARSE_CLASS_RESTRICTED_UTF) != 0
                && UCD_CASELESS_SETS[co as usize] < 128
            {
                co = 0; /* Ignore the caseless set if it's restricted. */
            }

            if co != 0 {
                list = UCD_CASELESS_SETS.as_ptr().add(co as usize);
            } else {
                co = ucd_othercase(c);
                tmp[0] = c;
                tmp[1] = NOTACHAR;

                if co != c {
                    tmp[1] = co;
                    tmp[2] = NOTACHAR;
                }
                /* Take the pointer only after filling the buffer: deriving it
                from a shared borrow first would let the optimiser assume the
                writes are not observable through it. */
                list = tmp.as_ptr();
            }
            c += 1;

            /* Add characters. */
            loop {
                if *list < new_start {
                    if *list + 1 == new_start {
                        new_start -= 1;
                        list = list.add(1);
                        if *list == NOTACHAR {
                            break;
                        }
                        continue;
                    }
                } else if *list > new_end {
                    if *list - 1 == new_end {
                        new_end += 1;
                        list = list.add(1);
                        if *list == NOTACHAR {
                            break;
                        }
                        continue;
                    }
                } else {
                    list = list.add(1);
                    if *list == NOTACHAR {
                        break;
                    }
                    continue;
                }

                result += 2;
                if buffer != ptr::null_mut() {
                    *buffer.add(0) = *list;
                    *buffer.add(1) = *list;
                    buffer = buffer.add(2);
                }

                list = list.add(1);
                if *list == NOTACHAR {
                    break;
                }
            }
        }

        if buffer != ptr::null_mut() {
            *buffer.add(0) = new_start;
            *buffer.add(1) = new_end;
            buffer = buffer.add(2);
            let _ = buffer;
        }
        result
    }
}

/* Add a character list to a buffer. */

unsafe fn append_char_list(p: *const u32, mut buffer: *mut u32) -> usize {
    unsafe {
        let mut p = p;
        let mut n: *const u32;
        let mut result: usize = 0;

        while *p != NOTACHAR {
            n = p;
            while *n.add(0) == *n.add(1) - 1 {
                n = n.add(1);
            }

            /* PCRE2_ASSERT(*p < 0xffff); */

            if buffer != ptr::null_mut() {
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

fn get_highest_char(_options: u32) -> u32 {
    /* PCRE2_CODE_UNIT_WIDTH == 8 */
    MAX_UTF_CODE_POINT
}

/* Add a negated character list to a buffer. */
unsafe fn append_negated_char_list(p: *const u32, options: u32, mut buffer: *mut u32) -> usize {
    unsafe {
        let mut p = p;
        let mut n: *const u32;
        let mut start: u32 = 0;
        let mut result: usize = 2;

        /* PCRE2_ASSERT(*p > 0); */

        while *p != NOTACHAR {
            n = p;
            while *n.add(0) == *n.add(1) - 1 {
                n = n.add(1);
            }

            /* PCRE2_ASSERT(*p < 0xffff); */

            if buffer != ptr::null_mut() {
                *buffer.add(0) = start;
                *buffer.add(1) = *p - 1;
                buffer = buffer.add(2);
            }

            result += 2;
            start = *n + 1;
            p = n.add(1);
        }

        if buffer != ptr::null_mut() {
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
        if buffer == ptr::null_mut() {
            return ptr::null_mut();
        }

        *buffer.add(0) = 0x100;
        *buffer.add(1) = get_highest_char(options);
        buffer.add(2)
    }
}

unsafe fn parse_class(ptr_in: *mut u32, options: u32, buffer_in: *mut u32) -> usize {
    unsafe {
        let mut ptr = ptr_in;
        let mut buffer = buffer_in;
        let mut total_size: usize = 0;
        let mut size: usize;
        let mut meta_arg: u32;
        let mut start_char: u32;

        loop {
            match meta_code(*ptr) {
                x if x == META_ESCAPE => {
                    meta_arg = meta_data(*ptr);
                    match meta_arg as i32 {
                        ESC_D | ESC_W | ESC_S => {
                            buffer = append_non_ascii_range(options, buffer);
                            total_size += 2;
                        }

                        ESC_h => {
                            size = append_char_list(HSPACE_LIST.as_ptr(), buffer);
                            total_size += size;
                            if buffer != ptr::null_mut() {
                                buffer = buffer.add(size);
                            }
                        }

                        ESC_H => {
                            size = append_negated_char_list(HSPACE_LIST.as_ptr(), options, buffer);
                            total_size += size;
                            if buffer != ptr::null_mut() {
                                buffer = buffer.add(size);
                            }
                        }

                        ESC_v => {
                            size = append_char_list(VSPACE_LIST.as_ptr(), buffer);
                            total_size += size;
                            if buffer != ptr::null_mut() {
                                buffer = buffer.add(size);
                            }
                        }

                        ESC_V => {
                            size = append_negated_char_list(VSPACE_LIST.as_ptr(), options, buffer);
                            total_size += size;
                            if buffer != ptr::null_mut() {
                                buffer = buffer.add(size);
                            }
                        }

                        ESC_p | ESC_P => {
                            ptr = ptr.add(1);
                            if meta_arg as i32 == ESC_p && (*ptr >> 16) == PT_ANY {
                                if buffer != ptr::null_mut() {
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
                /* PCRE2_ASSERT(*ptr < META_END || *ptr == META_BIGVALUE); */

                if *ptr == META_BIGVALUE {
                    ptr = ptr.add(1);
                }
            }

            if options & PARSE_CLASS_CASELESS_UTF != 0 {
                let end = *ptr;
                ptr = ptr.add(1);
                size = utf_caseless_extend(start_char, end, options, buffer);
                if buffer != ptr::null_mut() {
                    buffer = buffer.add(size);
                }
                total_size += size;
                continue;
            }

            if buffer != ptr::null_mut() {
                *buffer.add(0) = start_char;
                *buffer.add(1) = *ptr;
                buffer = buffer.add(2);
            }

            ptr = ptr.add(1);
            total_size += 2;
        }
    }
}

/* Extra uint32_t values for storing the lengths of range lists in the worst
case. Two uint32_t lengths and a range end for a range starting before 255 */
const CHAR_LIST_EXTRA_SIZE: usize = 3;

/* Starting character values for each character list. */
static char_list_starts: [u32; 3] = [
    XCL_CHAR_LIST_LOW_32_START,
    XCL_CHAR_LIST_HIGH_16_START,
    /* Must be terminated by XCL_CHAR_LIST_LOW_16_START, which also represents
    the end of the bitset. */
    XCL_CHAR_LIST_LOW_16_START,
];

unsafe fn compile_optimize_class(
    start_ptr: *mut u32,
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
) -> *mut class_ranges {
    unsafe {
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

        range_list_size = parse_class(start_ptr, class_options, ptr::null_mut());
        /* PCRE2_ASSERT((range_list_size & 0x1) == 0); */

        /* Allocate buffer. The total_size also represents the end of the buffer. */

        total_size = range_list_size + (if range_list_size >= 2 { CHAR_LIST_EXTRA_SIZE } else { 0 });

        cranges = (*cb).cx.as_ref().unwrap().memctl.malloc.unwrap()(
            core::mem::size_of::<class_ranges>() + total_size * core::mem::size_of::<u32>(),
            (*cb).cx.as_ref().unwrap().memctl.memory_data,
        ) as *mut class_ranges;

        if cranges == ptr::null_mut() {
            return ptr::null_mut();
        }

        (*cranges).header.next = ptr::null_mut();
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

        /* The second condition is a very rare corner case, where the end of the
        last range is the maximum character. This range cannot be extended
        further. */

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

        /* PCRE2_ASSERT(dst[1] <= get_highest_char(class_options)); */

        /* When the number of ranges are less than six, they are not converted
        to range lists. */

        ptr = buffer;
        while ptr < dst && *ptr.add(1) < 0x100 {
            ptr = ptr.add(2);
        }
        if (dst.offset_from(ptr) as isize) < (2 * (6 - 1)) as isize {
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
        /* PCRE2_ASSERT(tmp2 <= 3 * XCL_TYPE_BIT_LEN && tmp2 >= XCL_TYPE_BIT_LEN); */
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

                /* PCRE2_ASSERT((uint32_t*)next_char >= dst + 2); */

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
                /* PCRE2_ASSERT(range_start < char_list_start); */

                if range_end < char_list_end {
                    tmp1 += 1;
                    next_char = next_char.sub(1);

                    if char_list_start < XCL_CHAR_LIST_LOW_32_START {
                        *next_char = ((range_end << XCL_CHAR_SHIFT) | XCL_CHAR_END) as u16;
                    } else {
                        next_char = next_char.sub(1);
                        *(next_char as *mut u32) = (range_end << XCL_CHAR_SHIFT) | XCL_CHAR_END;
                    }

                    /* PCRE2_ASSERT((uint32_t*)next_char >= dst + 2); */
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
                /* PCRE2_ASSERT(range_start < XCL_CHAR_LIST_LOW_16_START); */
                break;
            }

            /* PCRE2_ASSERT((tmp2 % XCL_TYPE_BIT_LEN) == 0); */
            char_list_end = char_list_start - 1;
            char_list_start = *char_list_next;
            char_list_next = char_list_next.add(1);
            tmp1 = 0;
            tmp2 -= XCL_TYPE_BIT_LEN;
        }

        if *dst.add(0) < XCL_CHAR_LIST_LOW_16_START {
            dst = dst.add(2);
        }
        /* PCRE2_ASSERT((uint16_t*)dst <= next_char); */

        (*cranges).char_lists_size =
            (buffer.add(total_size) as *mut u8).offset_from(next_char as *mut u8) as usize;
        (*cranges).char_lists_start =
            (next_char as *mut u8).offset_from(buffer as *mut u8) as usize;
        (*cranges).range_list_size = dst.offset_from(buffer) as u16;
        cranges
    }
}

/*************************************************
*            Update classbits for \p etc.        *
*************************************************/

/// `PRIV(update_classbits)`
pub unsafe fn update_classbits(ptype: u32, pdata: u32, negated: BOOL, classbits: *mut u8) {
    unsafe {
        /* Update PRIV(xclass) when this function is changed. */
        let mut classbits = classbits;
        let mut chartype: c_int;
        let mut gentype: u32;
        let mut set_bit: BOOL;

        if ptype == PT_ANY {
            if negated == FALSE {
                ptr::write_bytes(classbits, 0xff, 32);
            }
            return;
        }

        let mut c: c_int = 0;
        while c < 256 {
            let prop = get_ucd(c as u32);
            set_bit = FALSE;

            match ptype {
                PT_LAMP => {
                    chartype = prop.chartype as c_int;
                    set_bit = (chartype == ucp_Lu as c_int
                        || chartype == ucp_Ll as c_int
                        || chartype == ucp_Lt as c_int) as BOOL;
                }

                PT_GC => {
                    set_bit = (UCP_GENTYPE[prop.chartype as usize] == pdata) as BOOL;
                }

                PT_PC => {
                    set_bit = (prop.chartype as u32 == pdata) as BOOL;
                }

                PT_SC => {
                    set_bit = (prop.script as u32 == pdata) as BOOL;
                }

                PT_SCX => {
                    set_bit = (prop.script as u32 == pdata
                        || mapbit(
                            &UCD_SCRIPT_SETS[ucd_scriptx_prop(prop) as usize..],
                            pdata,
                        ) != 0) as BOOL;
                }

                PT_ALNUM => {
                    gentype = UCP_GENTYPE[prop.chartype as usize];
                    set_bit = (gentype == ucp_L || gentype == ucp_N) as BOOL;
                }

                PT_SPACE /* Perl space */ | PT_PXSPACE /* POSIX space */ => {
                    match c as u32 {
                        /* HSPACE_BYTE_CASES */
                        0x09 | 0x20 | 0xa0
                        /* VSPACE_BYTE_CASES */
                        | 0x0a | 0x0b | 0x0c | 0x0d | 0x85 => {
                            set_bit = TRUE;
                        }
                        _ => {
                            set_bit = (UCP_GENTYPE[prop.chartype as usize] == ucp_Z) as BOOL;
                        }
                    }
                }

                PT_WORD => {
                    chartype = prop.chartype as c_int;
                    gentype = UCP_GENTYPE[chartype as usize];
                    set_bit = (gentype == ucp_L
                        || gentype == ucp_N
                        || chartype == ucp_Mn as c_int
                        || chartype == ucp_Pc as c_int) as BOOL;
                }

                PT_UCNC => {
                    set_bit = (c as u32 == CHAR_DOLLAR_SIGN
                        || c as u32 == CHAR_COMMERCIAL_AT
                        || c as u32 == CHAR_GRAVE_ACCENT
                        || c >= 0xa0) as BOOL;
                }

                PT_BIDICL => {
                    set_bit = (ucd_bidiclass_prop(prop) == pdata) as BOOL;
                }

                PT_BOOL => {
                    set_bit = (mapbit(
                        &UCD_BOOLPROP_SETS[ucd_bprops_prop(prop) as usize..],
                        pdata,
                    ) != 0) as BOOL;
                }

                PT_PXGRAPH => {
                    chartype = prop.chartype as c_int;
                    gentype = UCP_GENTYPE[chartype as usize];
                    set_bit = (gentype != ucp_Z
                        && (gentype != ucp_C || chartype == ucp_Cf as c_int)) as BOOL;
                }

                PT_PXPRINT => {
                    chartype = prop.chartype as c_int;
                    set_bit = (chartype != ucp_Zl as c_int
                        && chartype != ucp_Zp as c_int
                        && (UCP_GENTYPE[chartype as usize] != ucp_C
                            || chartype == ucp_Cf as c_int)) as BOOL;
                }

                PT_PXPUNCT => {
                    gentype = UCP_GENTYPE[prop.chartype as usize];
                    set_bit = (gentype == ucp_P || (c < 128 && gentype == ucp_S)) as BOOL;
                }

                _ => {
                    /* PCRE2_ASSERT(ptype == PT_PXXDIGIT); */
                    set_bit = ((c as u32 >= CHAR_0 && c as u32 <= CHAR_9)
                        || (c as u32 >= CHAR_A && c as u32 <= CHAR_F)
                        || (c as u32 >= CHAR_a && c as u32 <= CHAR_f)) as BOOL;
                }
            }

            if negated != FALSE {
                set_bit = (set_bit == 0) as BOOL;
            }
            if set_bit != FALSE {
                *classbits |= (1u32 << (c & 0x7)) as u8;
            }
            if (c & 0x7) == 0x7 {
                classbits = classbits.add(1);
            }

            c += 1;
        }
    }
}

/// Exported as `_pcre2_update_classbits_8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_update_classbits_8(
    ptype: u32,
    pdata: u32,
    negated: BOOL,
    classbits: *mut u8,
) {
    unsafe { update_classbits(ptype, pdata, negated, classbits) }
}

/*************************************************
*        XClass related property flags           *
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

/* This function sets the overall range for characters < 256. It also handles
non-utf case folding. cb->classbits is updated. */

unsafe fn add_to_class(
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
    start: u32,
    end: u32,
) {
    unsafe {
        let classbits: *mut u8 = (*cb).classbits.classbits.as_mut_ptr();
        let mut c: u32;
        let mut byte_start: u32;
        let mut byte_end: u32;
        let classbits_end: u32 = if end <= 0xff { end } else { 0xff };

        /* If caseless matching is required, scan the range and process alternate
        cases. */

        if (options & PCRE2_CASELESS) != 0 {
            /* UTF/UCP mode. */
            if (options & (PCRE2_UTF | PCRE2_UCP)) != 0 {
                let turkish_i: BOOL = ((xoptions
                    & (PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT))
                    == PCRE2_EXTRA_TURKISH_CASING) as BOOL;
                if start < 128 {
                    let lo_end: u32 = if classbits_end < 127 { classbits_end } else { 127 };
                    c = start;
                    while c <= lo_end {
                        if turkish_i != FALSE && ucd_any_i(c) {
                            c += 1;
                            continue;
                        }
                        setbit(classbits, *(*cb).fcc.add(c as usize) as u32);
                        c += 1;
                    }
                }
                if classbits_end >= 128 {
                    let hi_start: u32 = if start > 128 { start } else { 128 };
                    c = hi_start;
                    while c <= classbits_end {
                        let co: u32 = ucd_othercase(c);
                        if co <= 0xff {
                            setbit(classbits, co);
                        }
                        c += 1;
                    }
                }
            }
            /* Not UTF mode */
            else {
                c = start;
                while c <= classbits_end {
                    setbit(classbits, *(*cb).fcc.add(c as usize) as u32);
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
                setbit(classbits, c);
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
            setbit(classbits, c);
            c += 1;
        }

        c = byte_end;
        while c <= classbits_end {
            setbit(classbits, c);
            c += 1;
        }
    }
}

/*************************************************
*   Internal entry point for add list to class   *
*************************************************/

/* Add a list of horizontal or vertical whitespace characters to a class. */

unsafe fn add_list_to_class(
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
    p: *const u32,
) {
    unsafe {
        let mut p = p;
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

/*************************************************
*    Add characters not in a list to a class     *
*************************************************/

/* Add the complement of a list of horizontal or vertical whitespace to a
class. */

unsafe fn add_not_list_to_class(
    options: u32,
    xoptions: u32,
    cb: *mut compile_block,
    p: *const u32,
) {
    unsafe {
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
}

/*************************************************
*  Main entry-point to compile a character class *
*************************************************/

/* This function consumes a "leaf", which is a set of characters that will
become a single OP_CLASS OP_NCLASS, OP_XCLASS, or OP_ALLANY. */

/// `PRIV(compile_class_not_nested)`
pub unsafe fn compile_class_not_nested(
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
        let cbits: *const u8 = (*cb).cbits;
        /* Some functions such as add_to_class() or eclass processing expect
        that the bitset is stored in cb->classbits.classbits. */
        let classbits: *mut u8 = (*cb).classbits.classbits.as_mut_ptr();

        let utf: BOOL = ((options & PCRE2_UTF) != 0) as BOOL;

        /* Helper variables for OP_XCLASS opcode (for characters > 255). */
        let mut xclass_props: u32;
        let mut class_uchardata: *mut PCRE2_UCHAR;
        let mut cranges: *mut class_ranges;

        /* If an XClass contains a negative special such as \S, we need to flip
        the negation flag at the end. */

        should_flip_negation = FALSE;

        /* XClass will be used when characters > 255 might match. */

        xclass_props = 0;

        cranges = ptr::null_mut();

        if utf != FALSE {
            if lengthptr != ptr::null_mut() {
                cranges = compile_optimize_class(pptr, options, xoptions, cb);

                if cranges == ptr::null_mut() {
                    *errorcodeptr = ERR21;
                    return ptr::null_mut();
                }

                /* Caching the pre-processed character ranges. */
                if (*cb).last_data != ptr::null_mut() {
                    (*(*cb).last_data).next = &mut (*cranges).header;
                } else {
                    (*cb).first_data = &mut (*cranges).header;
                }

                (*cb).last_data = &mut (*cranges).header;
            } else {
                /* Reuse the pre-processed character ranges. */
                cranges = (*cb).first_data as *mut class_ranges;
                /* PCRE2_ASSERT(cranges != NULL && cranges->header.type == CDATA_CRANGE); */
                (*cb).first_data = (*cranges).header.next;
            }

            if (*cranges).range_list_size > 0 {
                let ranges: *const u32 = cranges.add(1) as *const u32;

                if *ranges.add(0) <= 255 {
                    xclass_props |= XCLASS_HAS_8BIT_CHARS;
                }

                if *ranges.add((*cranges).range_list_size as usize - 1)
                    == get_max_char_value(utf != FALSE)
                    && *ranges.add((*cranges).range_list_size as usize - 2) <= 256
                {
                    xclass_props |= XCLASS_HIGH_ANY;
                }
            }
        }

        class_uchardata = code.add(LINK_SIZE + 2); /* For XCLASS items */

        /* Initialize the 256-bit (32-byte) bit map to all zeros. */

        ptr::write_bytes(classbits, 0, 32);

        /* Process items until end_ptr is reached. */

        'main_loop: loop {
            let mut meta: u32 = *pptr;
            pptr = pptr.add(1);
            let local_negate: BOOL;
            let mut posix_class: c_int;
            let mut taboffset: c_int;
            let mut tabopt: c_int;
            let mut pbits: class_bits_storage = class_bits_storage { classbits: [0; 32] };
            let escape: u32;
            let c: u32;

            /* Handle POSIX classes such as [:alpha:] etc. */
            match meta_code(meta) {
                x if x == META_POSIX || x == META_POSIX_NEG => {
                    local_negate = (meta == META_POSIX_NEG) as BOOL;
                    posix_class = *pptr as c_int;
                    pptr = pptr.add(1);

                    if local_negate != FALSE {
                        should_flip_negation = TRUE; /* Note negative special */
                    }

                    /* If matching is caseless, upper and lower are converted to
                    alpha. */

                    if (options & PCRE2_CASELESS) != 0 && posix_class <= 2 {
                        posix_class = 0;
                    }

                    /* When PCRE2_UCP is set, some of the POSIX classes are
                    converted to different escape sequences. Others that are not
                    available via \p or \P have to generate XCL_PROP/XCL_NOTPROP
                    directly, which is done here. */

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

                                update_classbits(ptype, 0, local_negate, classbits);

                                if (xclass_props & XCLASS_HIGH_ANY) == 0 {
                                    if lengthptr != ptr::null_mut() {
                                        *lengthptr += 3;
                                    } else {
                                        *class_uchardata =
                                            if local_negate != FALSE {
                                                XCL_NOTPROP as u8
                                            } else {
                                                XCL_PROP as u8
                                            };
                                        class_uchardata = class_uchardata.add(1);
                                        *class_uchardata = ptype as PCRE2_UCHAR;
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

                    /* In the non-UCP case, or when UCP makes no difference, we
                    build the bit map for the POSIX class in a chunk of local
                    store. */

                    posix_class *= 3;

                    /* Copy in the first table (always present) */

                    ptr::copy_nonoverlapping(
                        cbits.add(POSIX_CLASS_MAPS[posix_class as usize] as usize),
                        pbits.classbits.as_mut_ptr(),
                        32,
                    );

                    /* If there is a second table, add or remove it as required. */

                    taboffset = POSIX_CLASS_MAPS[posix_class as usize + 1];
                    tabopt = POSIX_CLASS_MAPS[posix_class as usize + 2];

                    if taboffset >= 0 {
                        if tabopt >= 0 {
                            for ii in 0..32usize {
                                pbits.classbits[ii] |= *cbits.add(ii + taboffset as usize);
                            }
                        } else {
                            for ii in 0..32usize {
                                pbits.classbits[ii] &= !*cbits.add(ii + taboffset as usize);
                            }
                        }
                    }

                    /* Now see if we need to remove any special characters. An
                    option value of 1 removes vertical space and 2 removes
                    underscore. */

                    if tabopt < 0 {
                        tabopt = -tabopt;
                    }
                    if tabopt == 1 {
                        pbits.classbits[1] &= !0x3c;
                    } else if tabopt == 2 {
                        pbits.classbits[11] &= 0x7f;
                    }

                    /* Add the POSIX table or its complement into the main table
                    that is being built and we are done. */

                    {
                        let classwords: *mut u32 = (*cb).classbits.classwords.as_mut_ptr();

                        if local_negate != FALSE {
                            for ii in 0..8usize {
                                *classwords.add(ii) |= !pbits.classwords[ii];
                            }
                        } else {
                            for ii in 0..8usize {
                                *classwords.add(ii) |= pbits.classwords[ii];
                            }
                        }
                    }

                    /* Every class contains at least one < 256 character. */
                    xclass_props |= XCLASS_HAS_8BIT_CHARS;
                    continue 'main_loop; /* End of POSIX handling */
                }

                x if x == META_BIGVALUE => {
                    meta = *pptr;
                    pptr = pptr.add(1);
                }

                x if x == META_ESCAPE => {
                    escape = meta_data(meta);

                    match escape as i32 {
                        ESC_d => {
                            for ii in 0..32usize {
                                *classbits.add(ii) |= *cbits.add(ii + cbit_digit);
                            }
                        }

                        ESC_D => {
                            should_flip_negation = TRUE;
                            for ii in 0..32usize {
                                *classbits.add(ii) |= !*cbits.add(ii + cbit_digit);
                            }
                        }

                        ESC_w => {
                            for ii in 0..32usize {
                                *classbits.add(ii) |= *cbits.add(ii + cbit_word);
                            }
                        }

                        ESC_W => {
                            should_flip_negation = TRUE;
                            for ii in 0..32usize {
                                *classbits.add(ii) |= !*cbits.add(ii + cbit_word);
                            }
                        }

                        ESC_s => {
                            for ii in 0..32usize {
                                *classbits.add(ii) |= *cbits.add(ii + cbit_space);
                            }
                        }

                        ESC_S => {
                            should_flip_negation = TRUE;
                            for ii in 0..32usize {
                                *classbits.add(ii) |= !*cbits.add(ii + cbit_space);
                            }
                        }

                        /* When adding the horizontal or vertical space lists to
                        a class, or their complements, disable PCRE2_CASELESS. */

                        ESC_h => {
                            if cranges != ptr::null_mut() {
                                /* break */
                            } else {
                                add_list_to_class(
                                    options & !PCRE2_CASELESS,
                                    xoptions,
                                    cb,
                                    HSPACE_LIST.as_ptr(),
                                );
                            }
                        }

                        ESC_H => {
                            if cranges != ptr::null_mut() {
                                /* break */
                            } else {
                                add_not_list_to_class(
                                    options & !PCRE2_CASELESS,
                                    xoptions,
                                    cb,
                                    HSPACE_LIST.as_ptr(),
                                );
                            }
                        }

                        ESC_v => {
                            if cranges != ptr::null_mut() {
                                /* break */
                            } else {
                                add_list_to_class(
                                    options & !PCRE2_CASELESS,
                                    xoptions,
                                    cb,
                                    VSPACE_LIST.as_ptr(),
                                );
                            }
                        }

                        ESC_V => {
                            if cranges != ptr::null_mut() {
                                /* break */
                            } else {
                                add_not_list_to_class(
                                    options & !PCRE2_CASELESS,
                                    xoptions,
                                    cb,
                                    VSPACE_LIST.as_ptr(),
                                );
                            }
                        }

                        ESC_p | ESC_P => {
                            let ptype: u32 = *pptr >> 16;
                            let pdata: u32 = *pptr & 0xffff;
                            pptr = pptr.add(1);

                            /* The "Any" is processed by update_classbits(). */
                            if ptype == PT_ANY {
                                if utf == FALSE && escape as i32 == ESC_p {
                                    ptr::write_bytes(classbits, 0xff, 32);
                                }
                                continue 'main_loop;
                            }

                            update_classbits(ptype, pdata, (escape as i32 == ESC_P) as BOOL, classbits);

                            if (xclass_props & XCLASS_HIGH_ANY) == 0 {
                                if lengthptr != ptr::null_mut() {
                                    *lengthptr += 3;
                                } else {
                                    *class_uchardata = if escape as i32 == ESC_p {
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

                    /* Every non-property class contains at least one < 256
                    character. */
                    xclass_props |= XCLASS_HAS_8BIT_CHARS;
                    /* End handling \d-type escapes */
                    continue 'main_loop;
                }

                _ => {
                    /* CLASS_END_CASES */
                    /* Literals. */
                    if meta < META_END {
                        /* break: fall through to literal handling below */
                    } else {
                        /* Non-literals: end of class contents. */
                        break 'main_loop;
                    }
                }
            }

            /* A literal character may be followed by a range meta. */

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

                if cranges != ptr::null_mut() {
                    continue 'main_loop;
                }
                xclass_props |= XCLASS_HAS_8BIT_CHARS;

                /* Not an EBCDIC special range */

                add_to_class(options, xoptions, cb, c, d);
                continue 'main_loop;
            } /* End of range handling */

            /* Character ranges are ignored when class_ranges is present. */
            if cranges != ptr::null_mut() {
                continue 'main_loop;
            }
            xclass_props |= XCLASS_HAS_8BIT_CHARS;
            /* Handle a single character. */

            add_to_class(options, xoptions, cb, meta, meta);
        } /* End of main class-processing loop */

        /* END_PROCESSING: */

        /* PCRE2_ASSERT((xclass_props & XCLASS_HAS_PROPS) == 0 ||
                     (xclass_props & XCLASS_HIGH_ANY) == 0); */

        if cranges != ptr::null_mut() {
            let mut range: *mut u32 = cranges.add(1) as *mut u32;
            let end: *mut u32 = range.add((*cranges).range_list_size as usize);

            while range < end && *range.add(0) < 256 {
                /* Add range to bitset. If we are in UTF or UCP mode, then clear
                the caseless bit. */
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

                    if lengthptr != ptr::null_mut() {
                        if utf != FALSE {
                            *lengthptr += 1;

                            if range_start < range_end {
                                *lengthptr += ord2utf(range_start, class_uchardata) as usize;
                            }

                            *lengthptr += ord2utf(range_end, class_uchardata) as usize;
                            continue;
                        }

                        *lengthptr += if range_start < range_end { 3 } else { 2 };
                        continue;
                    }

                    if utf != FALSE {
                        if range_start < range_end {
                            *class_uchardata = XCL_RANGE as u8;
                            class_uchardata = class_uchardata.add(1);
                            class_uchardata =
                                class_uchardata.add(ord2utf(range_start, class_uchardata) as usize);
                        } else {
                            *class_uchardata = XCL_SINGLE as u8;
                            class_uchardata = class_uchardata.add(1);
                        }

                        class_uchardata =
                            class_uchardata.add(ord2utf(range_end, class_uchardata) as usize);
                        continue;
                    }

                    /* Without UTF support, character values are constrained by
                    the bit length, and can only be > 256 for 16-bit and 32-bit
                    libraries. (8-bit branch produces nothing here.) */
                }

                if lengthptr == ptr::null_mut() {
                    (*cb).cx.as_ref().unwrap().memctl.free.unwrap()(
                        cranges as *mut c_void,
                        (*cb).cx.as_ref().unwrap().memctl.memory_data,
                    );
                }
            }
        }

        /* If there are characters with values > 255, or Unicode property
        settings (\p or \P), we have to compile an extended class. */

        if (xclass_props & XCLASS_REQUIRED) != 0 {
            let previous: *mut PCRE2_UCHAR = code;

            if (xclass_props & XCLASS_HAS_CHAR_LISTS) == 0 {
                *class_uchardata = XCL_END as u8; /* Marks the end of extra data */
                class_uchardata = class_uchardata.add(1);
            }
            *code = OP_XCLASS;
            code = code.add(1);
            code = code.add(LINK_SIZE);
            *code = if negate_class != FALSE { XCL_NOT as u8 } else { 0 };
            if (xclass_props & XCLASS_HAS_PROPS) != 0 {
                *code |= XCL_HASPROP as u8;
            }

            /* If the map is required, move up the extra data to make room for
            it; otherwise just move the code pointer to the end of the extra
            data. */

            if (xclass_props & XCLASS_HAS_8BIT_CHARS) != 0 || has_bitmap != ptr::null_mut() {
                if negate_class != FALSE {
                    let classwords: *mut u32 = (*cb).classbits.classwords.as_mut_ptr();
                    for ii in 0..8usize {
                        *classwords.add(ii) = !*classwords.add(ii);
                    }
                }

                if has_bitmap == ptr::null_mut() {
                    /* Note the C post-increment: `*code++ |= XCL_MAP;` */
                    *code |= XCL_MAP as u8;
                    code = code.add(1);
                    ptr::copy(
                        code,
                        code.add(32),
                        cu2bytes(class_uchardata.offset_from(code) as usize),
                    );
                    ptr::copy_nonoverlapping(classbits, code, 32);
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
                /* Char lists size is an even number, because all items are 16 or
                32 bit values. The character list data is always aligned to 32
                bits. */
                let mut char_lists_size: usize = (*cranges).char_lists_size;
                /* PCRE2_ASSERT((char_lists_size & 0x1) == 0 &&
                             (cb->char_lists_size & 0x3) == 0); */

                if lengthptr != ptr::null_mut() {
                    char_lists_size =
                        clist_align_to(char_lists_size, core::mem::size_of::<u32>());

                    *lengthptr += 2 + LINK_SIZE;

                    (*cb).char_lists_size += char_lists_size;

                    char_lists_size /= core::mem::size_of::<PCRE2_UCHAR>();

                    /* Storage space for character lists is included in the
                    maximum pattern size. */
                    if *lengthptr > MAX_PATTERN_SIZE
                        || MAX_PATTERN_SIZE - *lengthptr < char_lists_size
                    {
                        *errorcodeptr = ERR20; /* Pattern is too large */
                        return ptr::null_mut();
                    }
                } else {
                    let data: *mut u8;

                    /* PCRE2_ASSERT(cranges->char_lists_types <= XCL_TYPE_MASK); */
                    /* Encode as high / low bytes. */
                    *code.add(0) = (XCL_LIST | ((*cranges).char_lists_types as u32 >> 8)) as u8;
                    *code.add(1) = (*cranges).char_lists_types as u8;
                    code = code.add(2);

                    /* Character lists are stored in backwards direction from
                    byte code start. */

                    (*cb).char_lists_size += char_lists_size;
                    data = ((*cb).start_code as *mut u8).sub((*cb).char_lists_size);

                    ptr::copy_nonoverlapping(
                        (cranges.add(1) as *mut u8).add((*cranges).char_lists_start),
                        data,
                        char_lists_size,
                    );

                    /* Since character lists total size is less than
                    MAX_PATTERN_SIZE, their starting offset fits into a value
                    which size is LINK_SIZE. */

                    let char_lists_size2 = (*cb).char_lists_size;
                    put(code, 0, (char_lists_size2 >> 1) as i32);
                    code = code.add(LINK_SIZE);

                    /* If we added padding to align the list, initialize the
                    bytes to defined values. */

                    if (char_lists_size2 & 0x2) != 0 {
                        *(data as *mut u16).sub(1) = 0xdead;
                    }

                    (*cb).char_lists_size =
                        clist_align_to(char_lists_size2, core::mem::size_of::<u32>());

                    (*cb).cx.as_ref().unwrap().memctl.free.unwrap()(
                        cranges as *mut c_void,
                        (*cb).cx.as_ref().unwrap().memctl.memory_data,
                    );
                }
            }

            /* Now fill in the complete length of the item */

            put(previous, 1, code.offset_from(previous) as i32);
            /* goto DONE */
            *pcode = code;
            return pptr.sub(1);
        }

        /* If there are no characters > 255, or they are all to be included or
        excluded, set the opcode to OP_CLASS or OP_NCLASS. */

        if negate_class != FALSE {
            let classwords: *mut u32 = (*cb).classbits.classwords.as_mut_ptr();
            for ii in 0..8usize {
                *classwords.add(ii) = !*classwords.add(ii);
            }
        }

        if (select_value8(utf == FALSE, false) || (negate_class != should_flip_negation))
            && (*cb).classbits.classwords[0] == !0u32
        {
            let classwords: *const u32 = (*cb).classbits.classwords.as_ptr();
            let mut ii: usize = 0;

            while ii < 8 {
                if *classwords.add(ii) != !0u32 {
                    break;
                }
                ii += 1;
            }

            if ii == 8 {
                *code = OP_ALLANY;
                code = code.add(1);
                /* goto DONE */
                *pcode = code;
                return pptr.sub(1);
            }
        }

        *code = if negate_class == should_flip_negation {
            OP_CLASS
        } else {
            OP_NCLASS
        };
        code = code.add(1);
        ptr::copy_nonoverlapping(classbits, code, 32);
        code = code.add(32);

        /* DONE: */
        *pcode = code;
        pptr.sub(1)
    }
}

/// Exported as `_pcre2_compile_class_not_nested_8`.
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
        compile_class_not_nested(
            options,
            xoptions,
            start_ptr,
            pcode,
            negate_class,
            has_bitmap,
            errorcodeptr,
            cb,
            lengthptr,
        )
    }
}

/* ===================================================================*/
/* Here follows a block of ECLASS-compiling functions, ordered from leafmost
(at the top) to outermost parser (at the bottom of the file). */

/* This function folds one operand using the negation operator. The new,
combined chunk of stack code is written out to *pop_info. */

unsafe fn fold_negation(
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
    preserve_classbits: BOOL,
) {
    unsafe {
        /* If the chunk of stack code is already composed of multiple ops, we
        won't descend in and try and propagate the negation down the tree. */

        if (*pop_info).op_single_type == 0 {
            if lengthptr != ptr::null_mut() {
                *lengthptr += 1;
            } else {
                *(*pop_info).code_start.add((*pop_info).length) = ECL_NOT;
            }
            (*pop_info).length += 1;
        }
        /* Otherwise, it's a nice single-op item, so we can easily fold in the
        negation without needing to produce an ECL_NOT. */
        else if (*pop_info).op_single_type == ECL_ANY
            || (*pop_info).op_single_type == ECL_NONE
        {
            (*pop_info).op_single_type = if (*pop_info).op_single_type == ECL_NONE {
                ECL_ANY
            } else {
                ECL_NONE
            };
            if lengthptr == ptr::null_mut() {
                *(*pop_info).code_start = (*pop_info).op_single_type;
            }
        } else {
            /* PCRE2_ASSERT(op_single_type == ECL_XCLASS && length >= 1+LINK_SIZE+1); */
            if lengthptr == ptr::null_mut() {
                *(*pop_info).code_start.add(1 + LINK_SIZE) ^= XCL_NOT as u8;
            }
        }

        if preserve_classbits == FALSE {
            for ii in 0..8usize {
                (*pop_info).bits.classwords[ii] = !(*pop_info).bits.classwords[ii];
            }
        }
    }
}

/* This function folds together two operands using a binary operator. The new,
combined chunk of stack code is written out to *lhs_op_info. */

unsafe fn fold_binary(
    op: c_int,
    lhs_op_info: *mut eclass_op_info,
    rhs_op_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) {
    unsafe {
        match op as u8 {
            ECL_AND => {
                if (*rhs_op_info).op_single_type == ECL_ANY {
                    /* no-op: drop the RHS */
                } else if (*lhs_op_info).op_single_type == ECL_ANY {
                    /* no-op: drop the LHS, and memmove the RHS into its place */
                    if lengthptr == ptr::null_mut() {
                        ptr::copy(
                            (*rhs_op_info).code_start,
                            (*lhs_op_info).code_start,
                            cu2bytes((*rhs_op_info).length),
                        );
                    }
                    (*lhs_op_info).length = (*rhs_op_info).length;
                    (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
                } else if (*rhs_op_info).op_single_type == ECL_NONE {
                    /* the result is ECL_NONE: write into the LHS */
                    if lengthptr == ptr::null_mut() {
                        *(*lhs_op_info).code_start = ECL_NONE;
                    }
                    (*lhs_op_info).length = 1;
                    (*lhs_op_info).op_single_type = ECL_NONE;
                } else if (*lhs_op_info).op_single_type == ECL_NONE {
                    /* the result is ECL_NONE: drop the RHS */
                } else {
                    /* Both of LHS & RHS are either ECL_XCLASS, or compound. */
                    if lengthptr != ptr::null_mut() {
                        *lengthptr += 1;
                    } else {
                        *(*rhs_op_info).code_start.add((*rhs_op_info).length) = ECL_AND;
                    }
                    (*lhs_op_info).length += (*rhs_op_info).length + 1;
                    (*lhs_op_info).op_single_type = 0;
                }

                for ii in 0..8usize {
                    (*lhs_op_info).bits.classwords[ii] &= (*rhs_op_info).bits.classwords[ii];
                }
            }

            ECL_OR => {
                if (*rhs_op_info).op_single_type == ECL_NONE {
                    /* no-op: drop the RHS */
                } else if (*lhs_op_info).op_single_type == ECL_NONE {
                    /* no-op: drop the LHS, and memmove the RHS into its place */
                    if lengthptr == ptr::null_mut() {
                        ptr::copy(
                            (*rhs_op_info).code_start,
                            (*lhs_op_info).code_start,
                            cu2bytes((*rhs_op_info).length),
                        );
                    }
                    (*lhs_op_info).length = (*rhs_op_info).length;
                    (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
                } else if (*rhs_op_info).op_single_type == ECL_ANY {
                    /* the result is ECL_ANY: write into the LHS */
                    if lengthptr == ptr::null_mut() {
                        *(*lhs_op_info).code_start = ECL_ANY;
                    }
                    (*lhs_op_info).length = 1;
                    (*lhs_op_info).op_single_type = ECL_ANY;
                } else if (*lhs_op_info).op_single_type == ECL_ANY {
                    /* the result is ECL_ANY: drop the RHS */
                } else {
                    /* Both of LHS & RHS are either ECL_XCLASS, or compound. */
                    if lengthptr != ptr::null_mut() {
                        *lengthptr += 1;
                    } else {
                        *(*rhs_op_info).code_start.add((*rhs_op_info).length) = ECL_OR;
                    }
                    (*lhs_op_info).length += (*rhs_op_info).length + 1;
                    (*lhs_op_info).op_single_type = 0;
                }

                for ii in 0..8usize {
                    (*lhs_op_info).bits.classwords[ii] |= (*rhs_op_info).bits.classwords[ii];
                }
            }

            ECL_XOR => {
                if (*rhs_op_info).op_single_type == ECL_NONE {
                    /* no-op: drop the RHS */
                } else if (*lhs_op_info).op_single_type == ECL_NONE {
                    /* no-op: drop the LHS, and memmove the RHS into its place */
                    if lengthptr == ptr::null_mut() {
                        ptr::copy(
                            (*rhs_op_info).code_start,
                            (*lhs_op_info).code_start,
                            cu2bytes((*rhs_op_info).length),
                        );
                    }
                    (*lhs_op_info).length = (*rhs_op_info).length;
                    (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
                } else if (*rhs_op_info).op_single_type == ECL_ANY {
                    /* the result is !LHS: fold in the negation, and drop the RHS */
                    /* Preserve the classbits, because we deal with them later. */
                    fold_negation(lhs_op_info, lengthptr, TRUE);
                } else if (*lhs_op_info).op_single_type == ECL_ANY {
                    /* the result is !RHS: drop the LHS, memmove the RHS into its
                    place, and fold in the negation */
                    if lengthptr == ptr::null_mut() {
                        ptr::copy(
                            (*rhs_op_info).code_start,
                            (*lhs_op_info).code_start,
                            cu2bytes((*rhs_op_info).length),
                        );
                    }
                    (*lhs_op_info).length = (*rhs_op_info).length;
                    (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;

                    /* Preserve the classbits, because we deal with them later. */
                    fold_negation(lhs_op_info, lengthptr, TRUE);
                } else {
                    /* Both of LHS & RHS are either ECL_XCLASS, or compound. */
                    if lengthptr != ptr::null_mut() {
                        *lengthptr += 1;
                    } else {
                        *(*rhs_op_info).code_start.add((*rhs_op_info).length) = ECL_XOR;
                    }
                    (*lhs_op_info).length += (*rhs_op_info).length + 1;
                    (*lhs_op_info).op_single_type = 0;
                }

                for ii in 0..8usize {
                    (*lhs_op_info).bits.classwords[ii] ^= (*rhs_op_info).bits.classwords[ii];
                }
            }

            _ => {
                /* PCRE2_DEBUG_UNREACHABLE(); */
            }
        }
    }
}

/* This function consumes a group of implicitly-unioned class elements. These
can be characters, ranges, properties, or nested classes, as long as they are
all joined by being placed adjacently. */

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
        let prev_ptr: *mut u32;
        let mut code: *mut PCRE2_UCHAR = *pcode;
        let code_start: *mut PCRE2_UCHAR = code;
        let prev_length: PCRE2_SIZE = if lengthptr != ptr::null_mut() { *lengthptr } else { 0 };
        let extra_length: PCRE2_SIZE;
        let meta: u32 = meta_code(*ptr);

        match meta {
            x if x == META_CLASS_EMPTY_NOT || x == META_CLASS_EMPTY => {
                ptr = ptr.add(1);
                (*pop_info).length = 1;
                if (meta == META_CLASS_EMPTY) == (negated != FALSE) {
                    (*pop_info).op_single_type = ECL_ANY;
                    *code = ECL_ANY;
                    code = code.add(1);
                    ptr::write_bytes((*pop_info).bits.classbits.as_mut_ptr(), 0xff, 32);
                } else {
                    (*pop_info).op_single_type = ECL_NONE;
                    *code = ECL_NONE;
                    code = code.add(1);
                    ptr::write_bytes((*pop_info).bits.classbits.as_mut_ptr(), 0, 32);
                }
            }

            _ => {
                let mut fell_through_from_class = false;

                if meta == META_CLASS || meta == META_CLASS_NOT {
                    if (*ptr & CLASS_IS_ECLASS) != 0 {
                        if compile_eclass_nested(context, negated, &mut ptr, &mut code,
                                                 pop_info, lengthptr) == FALSE
                        {
                            return FALSE;
                        }

                        /* PCRE2_ASSERT(*ptr == META_CLASS_END); */
                        ptr = ptr.add(1);
                        /* goto DONE */
                        *pptr = ptr;
                        *pcode = code;
                        return TRUE;
                    }

                    ptr = ptr.add(1);
                    fell_through_from_class = true;
                }
                let _ = fell_through_from_class;

                /* default (and fall-through from META_CLASS/NOT):
                Scan forward characters, ranges, and properties. */

                prev_ptr = ptr;
                ptr = compile_class_not_nested(
                    (*context).options,
                    (*context).xoptions,
                    ptr,
                    &mut code,
                    ((meta != META_CLASS_NOT) == (negated != FALSE)) as BOOL,
                    &mut (*context).needs_bitmap,
                    (*context).errorcodeptr,
                    (*context).cb,
                    lengthptr,
                );
                if ptr == ptr::null_mut() {
                    return FALSE;
                }

                /* We must have a 100% guarantee that ptr increases. */
                if ptr <= prev_ptr {
                    return FALSE;
                }

                /* If we fell through above, consume the closing ']'. */
                if meta == META_CLASS || meta == META_CLASS_NOT {
                    /* PCRE2_ASSERT(*ptr == META_CLASS_END); */
                    ptr = ptr.add(1);
                }

                /* Regardless of whether (lengthptr == NULL), some data will
                still be written out to *pcode, which we need. */
                extra_length = if lengthptr != ptr::null_mut() {
                    *lengthptr - prev_length
                } else {
                    0
                };

                /* Easiest case: convert OP_ALLANY to ECL_ANY */

                if *code_start == OP_ALLANY {
                    (*pop_info).length = 1;
                    (*pop_info).op_single_type = ECL_ANY;
                    *code_start = ECL_ANY;
                    ptr::write_bytes((*pop_info).bits.classbits.as_mut_ptr(), 0xff, 32);
                }
                /* For OP_CLASS and OP_NCLASS, we hoist out the bitmap and
                convert to ECL_NONE / ECL_ANY respectively. */
                else if *code_start == OP_CLASS || *code_start == OP_NCLASS {
                    (*pop_info).length = 1;
                    (*pop_info).op_single_type = if *code_start == OP_CLASS {
                        ECL_NONE
                    } else {
                        ECL_ANY
                    };
                    *code_start = (*pop_info).op_single_type;
                    ptr::copy_nonoverlapping(
                        code_start.add(1),
                        (*pop_info).bits.classbits.as_mut_ptr(),
                        32,
                    );
                    /* Rewind the code pointer, but adjust *lengthptr. */
                    if lengthptr != ptr::null_mut() {
                        *lengthptr += code.offset_from(code_start.add(1)) as usize;
                    }
                    code = code_start.add(1);

                    if (*context).needs_bitmap == FALSE && *code_start == ECL_NONE {
                        let classwords: *const u32 = (*pop_info).bits.classwords.as_ptr();

                        let mut jj = 0;
                        while jj < 8 {
                            if *classwords.add(jj) != 0 {
                                (*context).needs_bitmap = TRUE;
                                break;
                            }
                            jj += 1;
                        }
                    } else {
                        (*context).needs_bitmap = TRUE;
                    }
                }
                /* Finally, for OP_XCLASS we hoist out the bitmap (if any), and
                convert to ECL_XCLASS. */
                else {
                    /* PCRE2_ASSERT(*code_start == OP_XCLASS); */
                    *code_start = ECL_XCLASS;
                    (*pop_info).op_single_type = ECL_XCLASS;

                    ptr::copy_nonoverlapping(
                        (*(*context).cb).classbits.classbits.as_ptr(),
                        (*pop_info).bits.classbits.as_mut_ptr(),
                        32,
                    );
                    (*pop_info).length =
                        (code.offset_from(code_start) as usize) + extra_length;
                }
            }
        } /* End of switch(meta) */

        (*pop_info).code_start = if lengthptr == ptr::null_mut() {
            code_start
        } else {
            ptr::null_mut()
        };

        if lengthptr != ptr::null_mut() {
            *lengthptr += code.offset_from(code_start) as usize;
            code = code_start;
        }

        /* DONE: */
        /* PCRE2_ASSERT(lengthptr == NULL || (code == code_start)); */

        *pptr = ptr;
        *pcode = code;
        TRUE
    }
}

/* This function consumes a group of implicitly-unioned class elements. */

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

            if negated != FALSE {
                /* !(A juxtapose B)  ->  !A && !B */
                op = ECL_AND as u32;
                rhs_negated = TRUE;
            } else {
                /* A juxtapose B  ->  A || B */
                op = ECL_OR as u32;
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
            fold_binary(op as c_int, pop_info, &mut rhs_op_info, lengthptr);
            if lengthptr == ptr::null_mut() {
                code = (*pop_info).code_start.add((*pop_info).length);
            }
        }

        *pptr = ptr;
        *pcode = code;
        TRUE
    }
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
    unsafe {
        let mut ptr: *mut u32 = *pptr;

        while *ptr == META_ECLASS_NOT {
            ptr = ptr.add(1);
            negated = (negated == 0) as BOOL;
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
    unsafe {
        let mut ptr: *mut u32 = *pptr;
        let mut code: *mut PCRE2_UCHAR = *pcode;

        /* Because it's a non-empty class, there must be an operand at the start. */
        if compile_class_unary(context, negated, &mut ptr, &mut code, pop_info, lengthptr)
            == FALSE
        {
            return FALSE;
        }

        while *ptr == META_ECLASS_AND {
            let op: u32;
            let rhs_negated: BOOL;
            let mut rhs_op_info: eclass_op_info = core::mem::zeroed();

            if negated != FALSE {
                /* !(A && B)  ->  !A || !B */
                op = ECL_OR as u32;
                rhs_negated = TRUE;
            } else {
                /* A && B  ->  A && B */
                op = ECL_AND as u32;
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
            fold_binary(op as c_int, pop_info, &mut rhs_op_info, lengthptr);
            if lengthptr == ptr::null_mut() {
                code = (*pop_info).code_start.add((*pop_info).length);
            }
        }

        *pptr = ptr;
        *pcode = code;
        TRUE
    }
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
    unsafe {
        let mut ptr: *mut u32 = *pptr;
        let mut code: *mut PCRE2_UCHAR = *pcode;

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

            if negated != FALSE {
                /* !(A || B)   ->  !A && !B                     */
                /* !(A -- B)   ->  !(A && !B)    ->  !A || B    */
                /* !(A XOR B)  ->  !(!A XOR !B)  ->  !A XNOR !B */
                op = if *ptr == META_ECLASS_OR {
                    ECL_AND as u32
                } else if *ptr == META_ECLASS_SUB {
                    ECL_OR as u32
                } else {
                    ECL_XOR as u32
                };
                op_neg = (*ptr == META_ECLASS_XOR) as BOOL;
                rhs_negated = (*ptr != META_ECLASS_SUB) as BOOL;
            } else {
                /* A || B   ->  A || B  */
                /* A -- B   ->  A && !B */
                /* A XOR B  ->  A XOR B */
                op = if *ptr == META_ECLASS_OR {
                    ECL_OR as u32
                } else if *ptr == META_ECLASS_SUB {
                    ECL_AND as u32
                } else {
                    ECL_XOR as u32
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
            fold_binary(op as c_int, pop_info, &mut rhs_op_info, lengthptr);
            if op_neg != FALSE {
                fold_negation(pop_info, lengthptr, FALSE);
            }
            if lengthptr == ptr::null_mut() {
                code = (*pop_info).code_start.add((*pop_info).length);
            }
        }

        *pptr = ptr;
        *pcode = code;
        TRUE
    }
}

/* This function converts the META codes in pptr into opcodes written to pcode.
The pptr must start at a META_CLASS or META_CLASS_NOT. The pptr will be left
pointing at the matching META_CLASS_END. */

unsafe fn compile_eclass_nested(
    context: *mut eclass_context,
    mut negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    unsafe {
        let mut ptr: *mut u32 = *pptr;

        /* The CLASS_IS_ECLASS bit must be set since it is a nested class. */
        /* PCRE2_ASSERT(*ptr == (META_CLASS | CLASS_IS_ECLASS) ||
                     *ptr == (META_CLASS_NOT | CLASS_IS_ECLASS)); */

        let val = *ptr;
        ptr = ptr.add(1);
        let _ = ptr;
        if val == (META_CLASS_NOT | CLASS_IS_ECLASS) {
            negated = (negated == 0) as BOOL;
        }

        *pptr = (*pptr).add(1);

        /* Because it's a non-empty class, there must be an operand at the start. */
        if compile_class_binary_loose(context, negated, pptr, pcode, pop_info, lengthptr)
            == FALSE
        {
            return FALSE;
        }

        /* PCRE2_ASSERT(**pptr == META_CLASS_END); */
        TRUE
    }
}

/// `PRIV(compile_class_nested)`
pub unsafe fn compile_class_nested(
    options: u32,
    xoptions: u32,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    unsafe {
        let mut context: eclass_context = core::mem::zeroed();
        let mut op_info: eclass_op_info = core::mem::zeroed();
        let previous_length: PCRE2_SIZE =
            if lengthptr != ptr::null_mut() { *lengthptr } else { 0 };
        let mut code: *mut PCRE2_UCHAR = *pcode;
        let previous: *mut PCRE2_UCHAR;
        let mut allbitsone: BOOL = TRUE;

        context.needs_bitmap = FALSE;
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
        if compile_eclass_nested(&mut context, FALSE, pptr, &mut code, &mut op_info, lengthptr)
            == FALSE
        {
            return FALSE;
        }

        if lengthptr != ptr::null_mut() {
            *lengthptr += code.offset_from(previous) as usize;
            code = previous;
        }

        /* Do some useful counting of what's in the bitmap. */
        for ii in 0..8usize {
            if op_info.bits.classwords[ii] != 0xffffffff {
                allbitsone = FALSE;
                break;
            }
        }

        /* After constant-folding the extended class syntax, it may turn out to
        be a simple class after all. */

        if op_info.op_single_type != 0 {
            /* Rewind back over the OP_ECLASS. */
            code = previous;

            /* If the bits are all ones, and the "high characters" are all
            matched too, we use a special-cased encoding of OP_ALLANY. */

            if op_info.op_single_type == ECL_ANY && allbitsone != FALSE {
                /* Advancing code means rewinding lengthptr, at this point. */
                if lengthptr != ptr::null_mut() {
                    *lengthptr -= 1;
                }
                *code = OP_ALLANY;
                code = code.add(1);
            }
            /* If the high bits are all matched / all not-matched, then we emit
            an OP_NCLASS/OP_CLASS respectively. */
            else if op_info.op_single_type == ECL_ANY || op_info.op_single_type == ECL_NONE {
                let required_len: PCRE2_SIZE = 1 + 32;

                if lengthptr != ptr::null_mut() {
                    if required_len > (*lengthptr - previous_length) {
                        *lengthptr = previous_length + required_len;
                    }
                }

                /* Advancing code means rewinding lengthptr, at this point. */
                if lengthptr != ptr::null_mut() {
                    *lengthptr -= required_len;
                }
                *code = if op_info.op_single_type == ECL_ANY {
                    OP_NCLASS
                } else {
                    OP_CLASS
                };
                code = code.add(1);
                ptr::copy_nonoverlapping(op_info.bits.classbits.as_ptr(), code, 32);
                code = code.add(32);
            }
            /* Otherwise, we have an ECL_XCLASS, so we have the OP_XCLASS data
            there, but we pulled out its bitmap into op_info, so now we have to
            put that back into the OP_XCLASS. */
            else {
                let need_map: BOOL = context.needs_bitmap;
                let required_len: PCRE2_SIZE;

                /* PCRE2_ASSERT(op_info.op_single_type == ECL_XCLASS); */
                required_len = op_info.length + (if need_map != FALSE { 32 } else { 0 });

                if lengthptr != ptr::null_mut() {
                    /* Don't unconditionally request all the space we need. */
                    if required_len > (*lengthptr - previous_length) {
                        *lengthptr = previous_length + required_len;
                    }

                    /* We do have to write out a (truncated) OP_XCLASS, even on
                    this branch. */
                    *lengthptr -= 1 + LINK_SIZE + 1;
                    *code = OP_XCLASS;
                    code = code.add(1);
                    put(code, 0, (1 + LINK_SIZE + 1) as i32);
                    code = code.add(LINK_SIZE);
                    *code = 0;
                    code = code.add(1);
                } else {
                    let rest: *mut PCRE2_UCHAR;
                    let rest_len: PCRE2_SIZE;
                    let flags: PCRE2_UCHAR;

                    /* 1 unit: OP_XCLASS | LINK_SIZE units | 1 unit: flags | rest */
                    /* PCRE2_ASSERT(op_info.length >= 1 + LINK_SIZE + 1); */
                    rest = op_info.code_start.add(1 + LINK_SIZE + 1);
                    rest_len = op_info
                        .code_start
                        .add(op_info.length)
                        .offset_from(rest) as usize;

                    /* First read any data we use, before memmove splats it. */
                    flags = *op_info.code_start.add(1 + LINK_SIZE);
                    /* PCRE2_ASSERT((flags & XCL_MAP) == 0); */

                    /* Next do the memmove before any writes. */
                    ptr::copy(
                        rest,
                        code.add(1 + LINK_SIZE + 1 + (if need_map != FALSE { 32 } else { 0 })),
                        cu2bytes(rest_len),
                    );

                    /* Finally write the header data. */
                    *code = OP_XCLASS;
                    code = code.add(1);
                    put(code, 0, required_len as i32);
                    code = code.add(LINK_SIZE);
                    *code = flags | (if need_map != FALSE { XCL_MAP as u8 } else { 0 });
                    code = code.add(1);
                    if need_map != FALSE {
                        ptr::copy_nonoverlapping(op_info.bits.classbits.as_ptr(), code, 32);
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
            let required_len: PCRE2_SIZE =
                1 + LINK_SIZE + 1 + (if need_map != FALSE { 32 } else { 0 }) + op_info.length;

            if lengthptr != ptr::null_mut() {
                if required_len > (*lengthptr - previous_length) {
                    *lengthptr = previous_length + required_len;
                }

                /* As for the XCLASS branch above, we do have to write out a
                dummy OP_ECLASS. */
                *lengthptr -= 1 + LINK_SIZE + 1;
                *code = OP_ECLASS;
                code = code.add(1);
                put(code, 0, (1 + LINK_SIZE + 1) as i32);
                code = code.add(LINK_SIZE);
                *code = 0;
                code = code.add(1);
            } else {
                if need_map != FALSE {
                    let map_start: *mut PCRE2_UCHAR = previous.add(1 + LINK_SIZE + 1);
                    *previous.add(1 + LINK_SIZE) |= ECL_MAP as u8;
                    ptr::copy(
                        map_start,
                        map_start.add(32),
                        cu2bytes(code.offset_from(map_start) as usize),
                    );
                    ptr::copy_nonoverlapping(op_info.bits.classbits.as_ptr(), map_start, 32);
                    code = code.add(32);
                }
                put(previous, 1, code.offset_from(previous) as i32);
            }
        }

        *pcode = code;
        TRUE
    }
}

/// Exported as `_pcre2_compile_class_nested_8`.
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
        compile_class_nested(options, xoptions, pptr, pcode, errorcodeptr, cb, lengthptr)
    }
}

/* End of pcre2_compile_class.c */
