/* Translated from c_src/src/pcre2_compile_class.c lines 45-750 */

#[repr(C)]
#[derive(Copy, Clone)]
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

/* Checks the allowed tokens at the end of a class structure in debug mode.
CLASS_END_CASES(meta) expands to just `default:' when PCRE2_DEBUG is not
defined, which is the case for this build. */

/* ------------------------------ SUPPORT_WIDE_CHARS ------------------------ */

/* Heapsort algorithm. */

unsafe fn do_heapify(buffer: *mut u32, size: usize, i: usize) {
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

/* ------------------------------- SUPPORT_UNICODE -------------------------- */

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

    if c > MAX_UTF_CODE_POINT {
        return _pcre2_ucd_nocase_ranges_8.as_ptr().add(right as usize);
    }

    loop {
        /* Range end of the middle element. */
        middle = ((left + right) >> 1) | 0x1;

        if *_pcre2_ucd_nocase_ranges_8.as_ptr().add(middle as usize) <= c {
            left = middle + 1;
        } else if middle > 1 && *_pcre2_ucd_nocase_ranges_8.as_ptr().add((middle - 2) as usize) > c {
            right = middle - 1;
        } else {
            return _pcre2_ucd_nocase_ranges_8.as_ptr().add((middle - 1) as usize);
        }
    }
}

/* Get the list of othercase characters, which belongs to the passed range.
Create ranges from these characters, and append them to the buffer argument. */

unsafe fn utf_caseless_extend(start: u32, end: u32, options: u32, buffer: *mut u32) -> usize {
    let mut buffer = buffer;
    let mut new_start: u32 = start;
    let mut new_end: u32 = end;
    let mut c: u32 = start;
    let mut list: *const u32;
    let mut tmp: [u32; 3] = [0; 3];
    let tmp_ptr: *mut u32 = tmp.as_mut_ptr();
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
            && UCD_ANY_I(c)
        {
            co = _pcre2_ucd_turkish_dotted_i_caseset_8 + (if UCD_DOTTED_I(c) { 0 } else { 3 });
        } else {
            co = UCD_CASESET(c);
            if co != 0
                && (options & PARSE_CLASS_RESTRICTED_UTF) != 0
                && *_pcre2_ucd_caseless_sets_8.as_ptr().add(co as usize) < 128
            {
                co = 0; /* Ignore the caseless set if it's restricted. */
            }
        }

        if co != 0 {
            list = _pcre2_ucd_caseless_sets_8.as_ptr().add(co as usize);
        } else {
            co = UCD_OTHERCASE(c);
            list = tmp_ptr;
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
            'next_item: {
                if *list < new_start {
                    if (*list).wrapping_add(1) == new_start {
                        new_start -= 1;
                        break 'next_item;
                    }
                } else if *list > new_end {
                    if (*list).wrapping_sub(1) == new_end {
                        new_end += 1;
                        break 'next_item;
                    }
                } else {
                    break 'next_item;
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

unsafe fn append_char_list(p: *const u32, buffer: *mut u32) -> usize {
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

unsafe fn get_highest_char(options: u32) -> u32 {
    let _ = options; /* Avoid compiler warning. */

    MAX_UTF_CODE_POINT
}

/* Add a negated character list to a buffer. */

unsafe fn append_negated_char_list(p: *const u32, options: u32, buffer: *mut u32) -> usize {
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

unsafe fn append_non_ascii_range(options: u32, buffer: *mut u32) -> *mut u32 {
    if buffer.is_null() {
        return std::ptr::null_mut();
    }

    *buffer.add(0) = 0x100;
    *buffer.add(1) = get_highest_char(options);
    buffer.add(2)
}

unsafe fn parse_class(ptr: *mut u32, options: u32, buffer: *mut u32) -> usize {
    let mut ptr = ptr;
    let mut buffer = buffer;
    let mut total_size: usize = 0;
    let mut size: usize;
    let mut meta_arg: u32;
    let mut start_char: u32;

    loop {
        let meta_code = META_CODE!(*ptr);

        if meta_code == META_ESCAPE {
            meta_arg = META_DATA!(*ptr);

            if meta_arg == ESC_D as u32 || meta_arg == ESC_W as u32 || meta_arg == ESC_S as u32 {
                buffer = append_non_ascii_range(options, buffer);
                total_size += 2;
            } else if meta_arg == ESC_h as u32 {
                size = append_char_list(_pcre2_hspace_list_8.as_ptr(), buffer);
                total_size += size;
                if !buffer.is_null() {
                    buffer = buffer.add(size);
                }
            } else if meta_arg == ESC_H as u32 {
                size = append_negated_char_list(_pcre2_hspace_list_8.as_ptr(), options, buffer);
                total_size += size;
                if !buffer.is_null() {
                    buffer = buffer.add(size);
                }
            } else if meta_arg == ESC_v as u32 {
                size = append_char_list(_pcre2_vspace_list_8.as_ptr(), buffer);
                total_size += size;
                if !buffer.is_null() {
                    buffer = buffer.add(size);
                }
            } else if meta_arg == ESC_V as u32 {
                size = append_negated_char_list(_pcre2_vspace_list_8.as_ptr(), options, buffer);
                total_size += size;
                if !buffer.is_null() {
                    buffer = buffer.add(size);
                }
            } else if meta_arg == ESC_p as u32 || meta_arg == ESC_P as u32 {
                ptr = ptr.add(1);
                if meta_arg == ESC_p as u32 && (*ptr >> 16) == PT_ANY {
                    if !buffer.is_null() {
                        *buffer.add(0) = 0;
                        *buffer.add(1) = get_highest_char(options);
                        buffer = buffer.add(2);
                    }
                    total_size += 2;
                }
            }

            ptr = ptr.add(1);
            continue;
        } else if meta_code == META_POSIX_NEG {
            buffer = append_non_ascii_range(options, buffer);
            total_size += 2;
            ptr = ptr.add(2);
            continue;
        } else if meta_code == META_POSIX {
            ptr = ptr.add(2);
            continue;
        } else if meta_code == META_BIGVALUE {
            /* Character literal */
            ptr = ptr.add(1);
        } else {
            /* CLASS_END_CASES(*ptr) */
            if *ptr >= META_END {
                return total_size;
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
            let range_end = {
                let t = *ptr;
                ptr = ptr.add(1);
                t
            };
            size = utf_caseless_extend(start_char, range_end, options, buffer);
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
    let mut buffer: *mut u32;
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

    range_list_size = parse_class(start_ptr, class_options, std::ptr::null_mut());
    /* PCRE2_ASSERT((range_list_size & 0x1) == 0); */

    /* Allocate buffer. The total_size also represents the end of the buffer. */

    total_size = range_list_size + (if range_list_size >= 2 { CHAR_LIST_EXTRA_SIZE } else { 0 });

    cranges = ((*(*cb).cx).memctl.malloc.unwrap())(
        size_of::<class_ranges>() + total_size * size_of::<u32>(),
        (*(*cb).cx).memctl.memory_data,
    ) as *mut class_ranges;

    if cranges.is_null() {
        return std::ptr::null_mut();
    }

    (*cranges).header.next = std::ptr::null_mut();
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

    while range_list_size > 0 && *dst.add(1) != !(0u32) {
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
    char_list_start = {
        let t = *char_list_next;
        char_list_next = char_list_next.add(1);
        t
    };
    char_list_end = XCL_CHAR_LIST_LOW_32_END;
    next_char = buffer.add(total_size) as *mut u16;

    tmp1 = 0;
    tmp2 = ((char_list_starts.len() - 1) as u32) * XCL_TYPE_BIT_LEN;
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
                    (next_char as *mut u32)
                        .write_unaligned((range_end << XCL_CHAR_SHIFT) | XCL_CHAR_END);
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
                        (next_char as *mut u32).write_unaligned(range_start << XCL_CHAR_SHIFT);
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
                    (next_char as *mut u32)
                        .write_unaligned((range_end << XCL_CHAR_SHIFT) | XCL_CHAR_END);
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
                (next_char as *mut u32).write_unaligned(tmp1);
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
        char_list_start = {
            let t = *char_list_next;
            char_list_next = char_list_next.add(1);
            t
        };
        tmp1 = 0;
        tmp2 -= XCL_TYPE_BIT_LEN;
    }

    if *dst.add(0) < XCL_CHAR_LIST_LOW_16_START {
        dst = dst.add(2);
    }
    /* PCRE2_ASSERT((uint16_t*)dst <= next_char); */

    (*cranges).char_lists_size =
        (buffer.add(total_size) as *const u8).offset_from(next_char as *const u8) as usize;
    (*cranges).char_lists_start =
        (next_char as *const u8).offset_from(buffer as *const u8) as usize;
    (*cranges).range_list_size = dst.offset_from(buffer) as u16;
    cranges
}
