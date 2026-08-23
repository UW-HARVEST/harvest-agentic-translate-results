/* Translated from c_src/src/pcre2_study.c lines 788-1091 */

/*************************************************
*      Set a bit and maybe its alternate case    *
*************************************************/

/* Given a character, set its first code unit's bit in the table, and also the
corresponding bit for the other version of a letter if we are caseless.

Arguments:
  re            points to the regex block
  p             points to the first code unit of the character
  caseless      TRUE if caseless
  utf           TRUE for UTF mode
  ucp           TRUE for UCP mode

Returns:        pointer after the character
*/

unsafe fn set_table_bit(
    re: *mut pcre2_real_code,
    mut p: PCRE2_SPTR,
    caseless: BOOL,
    utf: BOOL,
    ucp: BOOL,
) -> PCRE2_SPTR {
    let mut c: u32 = {
        let t = *p as u32;
        p = p.add(1);
        t
    }; /* First code unit */

    /* SET_BIT(c) */
    *(*re).start_bitmap.as_mut_ptr().add((c / 8) as usize) |= (1u32 << (c & 7)) as u8;

    /* In UTF-8 or UTF-16 mode, pick up the remaining code units in order to find
    the end of the character, even when caseless. */

    if utf != 0 {
        if c >= 0xc0 {
            GETUTF8INC!(c, p);
        }
    }

    /* If caseless, handle the other case of the character. */

    if caseless != 0 {
        if utf != 0 || ucp != 0 {
            c = UCD_OTHERCASE(c);
            if utf != 0 {
                let mut buff: [PCRE2_UCHAR; 6] = [0; 6];
                _pcre2_ord2utf_8(c, buff.as_mut_ptr());
                let b: u32 = buff[0] as u32;
                /* SET_BIT(buff[0]) */
                *(*re).start_bitmap.as_mut_ptr().add((b / 8) as usize) |= (1u32 << (b & 7)) as u8;
            } else if c < 256 {
                /* SET_BIT(c) */
                *(*re).start_bitmap.as_mut_ptr().add((c / 8) as usize) |= (1u32 << (c & 7)) as u8;
            }
        }
        /* Not UTF or UCP */
        else if MAX_255!(c) != 0 {
            /* SET_BIT(re->tables[fcc_offset + c]) */
            let b: u32 = *(*re).tables.add(fcc_offset + c as usize) as u32;
            *(*re).start_bitmap.as_mut_ptr().add((b / 8) as usize) |= (1u32 << (b & 7)) as u8;
        }
    }

    return p;
}

/*************************************************
*     Set bits for a positive character type     *
*************************************************/

/* This function sets starting bits for a character type. In UTF-8 mode, we can
only do a direct setting for bytes less than 128, as otherwise there can be
confusion with bytes in the middle of UTF-8 characters. In a "traditional"
environment, the tables will only recognize ASCII characters anyway, but in at
least one Windows environment, some higher bytes bits were set in the tables.
So we deal with that case by considering the UTF-8 encoding.

Arguments:
  re             the regex block
  cbit type      the type of character wanted
  table_limit    32 for non-UTF-8; 16 for UTF-8

Returns:         nothing
*/

unsafe fn set_type_bits(re: *mut pcre2_real_code, cbit_type: c_int, table_limit: c_uint) {
    let mut c: u32;
    c = 0;
    while c < table_limit {
        *(*re).start_bitmap.as_mut_ptr().add(c as usize) |=
            *(*re).tables.add(c as usize + cbits_offset + cbit_type as usize);
        c += 1;
    }
    if table_limit == 32 {
        return;
    }
    c = 128;
    while c < 256 {
        if (*(*re).tables.add(cbits_offset + (c / 8) as usize) as u32 & (1u32 << (c & 7))) != 0 {
            let mut buff: [PCRE2_UCHAR; 6] = [0; 6];
            _pcre2_ord2utf_8(c, buff.as_mut_ptr());
            let b: u32 = buff[0] as u32;
            /* SET_BIT(buff[0]) */
            *(*re).start_bitmap.as_mut_ptr().add((b / 8) as usize) |= (1u32 << (b & 7)) as u8;
        }
        c += 1;
    }
}

/*************************************************
*     Set bits for a negative character type     *
*************************************************/

/* This function sets starting bits for a negative character type such as \D.
In UTF-8 mode, we can only do a direct setting for bytes less than 128, as
otherwise there can be confusion with bytes in the middle of UTF-8 characters.
Unlike in the positive case, where we can set appropriate starting bits for
specific high-valued UTF-8 characters, in this case we have to set the bits for
all high-valued characters. The lowest is 0xc2, but we overkill by starting at
0xc0 (192) for simplicity.

Arguments:
  re             the regex block
  cbit type      the type of character wanted
  table_limit    32 for non-UTF-8; 16 for UTF-8

Returns:         nothing
*/

unsafe fn set_nottype_bits(re: *mut pcre2_real_code, cbit_type: c_int, table_limit: c_uint) {
    let mut c: u32;
    c = 0;
    while c < table_limit {
        *(*re).start_bitmap.as_mut_ptr().add(c as usize) |=
            !(*(*re).tables.add(c as usize + cbits_offset + cbit_type as usize));
        c += 1;
    }
    if table_limit != 32 {
        c = 24;
        while c < 32 {
            *(*re).start_bitmap.as_mut_ptr().add(c as usize) = 0xff;
            c += 1;
        }
    }
}

/*************************************************
*     Set starting bits for a character list.    *
*************************************************/

/* This function sets starting bits for a character list. It enumerates
all characters and character ranges in the character list, and sets
the starting bits accordingly.

Arguments:
  code           pointer to the code
  start_bitmap   pointer to the starting bitmap

Returns:         nothing
*/

unsafe fn study_char_list(
    mut code: PCRE2_SPTR,
    start_bitmap: *mut u8,
    char_lists_end: *const u8,
) {
    let mut type_: u32;
    let mut list_ind: u32;
    let mut char_list_add: u32 = XCL_CHAR_LIST_LOW_16_ADD;
    let mut range_start: u32 = !(0 as u32);
    let mut range_end: u32 = 0;
    let mut next_char: *const u8;
    let mut start_buffer: [PCRE2_UCHAR; 6] = [0; 6];
    let mut end_buffer: [PCRE2_UCHAR; 6] = [0; 6];
    let mut start: PCRE2_UCHAR;
    let mut end: PCRE2_UCHAR;

    /* Only needed in 8-bit mode at the moment. */
    type_ = ((*code.add(0) as u32) << 8) | *code.add(1) as u32;
    code = code.add(2);

    /* Align characters. */
    next_char = char_lists_end.sub((GET!(code, 0) << 1) as usize);
    type_ &= XCL_TYPE_MASK;
    list_ind = 0;

    if (type_ & XCL_BEGIN_WITH_RANGE) != 0 {
        range_start = XCL_CHAR_LIST_LOW_16_START;
    }

    while type_ > 0 {
        let mut item_count: u32 = type_ & XCL_ITEM_COUNT_MASK;

        if item_count == XCL_ITEM_COUNT_MASK {
            if list_ind <= 1 {
                item_count = std::ptr::read_unaligned(next_char as *const u16) as u32;
                next_char = next_char.add(2);
            } else {
                item_count = std::ptr::read_unaligned(next_char as *const u32);
                next_char = next_char.add(4);
            }
        }

        while item_count > 0 {
            if list_ind <= 1 {
                range_end = std::ptr::read_unaligned(next_char as *const u16) as u32;
                next_char = next_char.add(2);
            } else {
                range_end = std::ptr::read_unaligned(next_char as *const u32);
                next_char = next_char.add(4);
            }

            if (range_end & XCL_CHAR_END) != 0 {
                range_end = char_list_add + (range_end >> XCL_CHAR_SHIFT);

                _pcre2_ord2utf_8(range_end, end_buffer.as_mut_ptr());
                end = end_buffer[0];

                if range_start < range_end {
                    _pcre2_ord2utf_8(range_start, start_buffer.as_mut_ptr());
                    start = start_buffer[0];
                    while start <= end {
                        *start_bitmap.add((start / 8) as usize) |= (1u32 << (start & 7)) as u8;
                        start = start.wrapping_add(1);
                    }
                } else {
                    *start_bitmap.add((end / 8) as usize) |= (1u32 << (end & 7)) as u8;
                }

                range_start = !(0 as u32);
            } else {
                range_start = char_list_add + (range_end >> XCL_CHAR_SHIFT);
            }

            item_count -= 1;
        }

        list_ind += 1;
        type_ >>= XCL_TYPE_BIT_LEN;

        if range_start == !(0 as u32) {
            if (type_ & XCL_BEGIN_WITH_RANGE) != 0 {
                /* In 8 bit mode XCL_CHAR_LIST_HIGH_32_START is not possible. */
                if list_ind == 1 {
                    range_start = XCL_CHAR_LIST_HIGH_16_START;
                } else {
                    range_start = XCL_CHAR_LIST_LOW_32_START;
                }
            }
        } else if (type_ & XCL_BEGIN_WITH_RANGE) == 0 {
            _pcre2_ord2utf_8(range_start, start_buffer.as_mut_ptr());

            /* In 8 bit mode XCL_CHAR_LIST_LOW_32_END and
            XCL_CHAR_LIST_HIGH_32_END are not possible. */
            if list_ind == 1 {
                range_end = XCL_CHAR_LIST_LOW_16_END;
            } else {
                range_end = XCL_CHAR_LIST_HIGH_16_END;
            }

            _pcre2_ord2utf_8(range_end, end_buffer.as_mut_ptr());
            end = end_buffer[0];

            start = start_buffer[0];
            while start <= end {
                *start_bitmap.add((start / 8) as usize) |= (1u32 << (start & 7)) as u8;
                start = start.wrapping_add(1);
            }

            range_start = !(0 as u32);
        }

        /* In 8 bit mode XCL_CHAR_LIST_HIGH_32_ADD is not possible. */
        if list_ind == 1 {
            char_list_add = XCL_CHAR_LIST_HIGH_16_ADD;
        } else {
            char_list_add = XCL_CHAR_LIST_LOW_32_ADD;
        }
    }
}
