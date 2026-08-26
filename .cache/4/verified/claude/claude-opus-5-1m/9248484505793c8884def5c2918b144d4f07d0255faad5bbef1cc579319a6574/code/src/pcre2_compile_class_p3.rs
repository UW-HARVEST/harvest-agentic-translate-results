/* Translated from c_src/src/pcre2_compile_class.c lines 1072-1868 */

/*************************************************
*           XClass related properties            *
*************************************************/
/* (c_src/src/pcre2_compile_class.c lines 874-887; used only here.) */

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
    let classbits: *mut u8 = (*cb).classbits.classbits.as_mut_ptr();

    let utf: BOOL = if (options & PCRE2_UTF) != 0 { TRUE } else { FALSE };

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

    cranges = std::ptr::null_mut();

    if utf != 0 {
        if !lengthptr.is_null() {
            cranges = compile_optimize_class(pptr, options, xoptions, cb);

            if cranges.is_null() {
                *errorcodeptr = ERR21;
                return std::ptr::null_mut();
            }

            /* Caching the pre-processed character ranges. */
            if !(*cb).last_data.is_null() {
                (*(*cb).last_data).next = std::ptr::addr_of_mut!((*cranges).header);
            } else {
                (*cb).first_data = std::ptr::addr_of_mut!((*cranges).header);
            }

            (*cb).last_data = std::ptr::addr_of_mut!((*cranges).header);
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

            if *ranges.add((*cranges).range_list_size as usize - 1) == GET_MAX_CHAR_VALUE!(utf)
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

    'done: {
        'end_processing: {
            'main_loop: loop {
                let mut meta: u32 = {
                    let t = *pptr;
                    pptr = pptr.add(1);
                    t
                };
                let local_negate: BOOL;
                let mut posix_class: c_int;
                let taboffset: c_int;
                let mut tabopt: c_int;
                let c: u32;

                /* Handle POSIX classes such as [:alpha:] etc. */
                'switch_meta: {
                    let meta_code = META_CODE!(meta);

                    if meta_code == META_POSIX || meta_code == META_POSIX_NEG {
                        local_negate = if meta == META_POSIX_NEG { TRUE } else { FALSE };
                        posix_class = {
                            let t = *pptr;
                            pptr = pptr.add(1);
                            t
                        } as c_int;

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

                        /* TODO This entire block of code here appears to be unreachable!? I
                        simply can't see how it can be hit, given that the frontend parser
                        doesn't emit META_POSIX for GRAPH/PRINT/PUNCT when UCP is set. */
                        if (options & PCRE2_UCP) != 0 && (xoptions & PCRE2_EXTRA_ASCII_POSIX) == 0
                        {
                            if posix_class == PC_GRAPH as c_int
                                || posix_class == PC_PRINT as c_int
                                || posix_class == PC_PUNCT as c_int
                            {
                                let ptype: u32 = if posix_class == PC_GRAPH as c_int {
                                    PT_PXGRAPH
                                } else if posix_class == PC_PRINT as c_int {
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
                                        }) as PCRE2_UCHAR;
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

                            /* For the other POSIX classes (ex: ascii) we are going to
                            fall through to the non-UCP case and build a bit map for
                            characters with code points less than 256. However, if we are in
                            a negated POSIX class, characters with code points greater than
                            255 must either all match or all not match, depending on whether
                            the whole class is not or is negated. For example, for
                            [[:^ascii:]... they must all match, whereas for [^[:^ascii:]...
                            they must not.

                            In the special case where there are no xclass items, this is
                            automatically handled by the use of OP_CLASS or OP_NCLASS, but an
                            explicit range is needed for OP_XCLASS. Setting a flag here
                            causes the range to be generated later when it is known that
                            OP_XCLASS is required. In the 8-bit library this is relevant only
                            in utf mode, since no wide characters can exist otherwise. */
                        }

                        /* In the non-UCP case, or when UCP makes no difference, we build the
                        bit map for the POSIX class in a chunk of local store because we may
                        be adding and subtracting from it, and we don't want to subtract bits
                        that may be in the main map already. At the end we or the result into
                        the bit map that is being built. */

                        let mut pbits: class_bits_storage = class_bits_storage {
                            classbits: [0; 32],
                        };

                        posix_class *= 3;

                        /* Copy in the first table (always present) */

                        memcpy(
                            pbits.classbits.as_mut_ptr() as *mut c_void,
                            cbits.offset(
                                *_pcre2_posix_class_maps8.as_ptr().add(posix_class as usize)
                                    as isize,
                            ) as *const c_void,
                            32,
                        );

                        /* If there is a second table, add or remove it as required. */

                        taboffset =
                            *_pcre2_posix_class_maps8.as_ptr().add(posix_class as usize + 1);
                        tabopt = *_pcre2_posix_class_maps8.as_ptr().add(posix_class as usize + 2);

                        if taboffset >= 0 {
                            if tabopt >= 0 {
                                for i in 0..32 {
                                    pbits.classbits[i] |=
                                        *cbits.offset(i as isize + taboffset as isize);
                                }
                            } else {
                                for i in 0..32 {
                                    pbits.classbits[i] &=
                                        !*cbits.offset(i as isize + taboffset as isize);
                                }
                            }
                        }

                        /* Now see if we need to remove any special characters. An option
                        value of 1 removes vertical space and 2 removes underscore. */

                        if tabopt < 0 {
                            tabopt = -tabopt;
                        }
                        if tabopt == 1 {
                            pbits.classbits[1] &= !0x3c;
                        } else if tabopt == 2 {
                            pbits.classbits[11] &= 0x7f;
                        }

                        /* Add the POSIX table or its complement into the main table that is
                        being built and we are done. */

                        {
                            let classwords: *mut u32 = (*cb).classbits.classwords.as_mut_ptr();

                            if local_negate != 0 {
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
                    else if meta_code == META_BIGVALUE {
                        meta = {
                            let t = *pptr;
                            pptr = pptr.add(1);
                            t
                        };
                        break 'switch_meta;
                    } else if meta_code == META_ESCAPE {
                        let escape: u32 = META_DATA!(meta);

                        'switch_escape: {
                            if escape == ESC_d as u32 {
                                for i in 0..32 {
                                    *classbits.add(i) |= *cbits.add(i + cbit_digit);
                                }
                            } else if escape == ESC_D as u32 {
                                should_flip_negation = TRUE;
                                for i in 0..32 {
                                    *classbits.add(i) |= !*cbits.add(i + cbit_digit);
                                }
                            } else if escape == ESC_w as u32 {
                                for i in 0..32 {
                                    *classbits.add(i) |= *cbits.add(i + cbit_word);
                                }
                            } else if escape == ESC_W as u32 {
                                should_flip_negation = TRUE;
                                for i in 0..32 {
                                    *classbits.add(i) |= !*cbits.add(i + cbit_word);
                                }
                            }
                            /* Perl 5.004 onwards omitted VT from \s, but restored it at Perl
                            5.18. Before PCRE 8.34, we had to preserve the VT bit if it was
                            previously set by something earlier in the character class.
                            Luckily, the value of CHAR_VT is 0x0b in both ASCII and EBCDIC, so
                            we could just adjust the appropriate bit. From PCRE 8.34 we no
                            longer treat \s and \S specially. */
                            else if escape == ESC_s as u32 {
                                for i in 0..32 {
                                    *classbits.add(i) |= *cbits.add(i + cbit_space);
                                }
                            } else if escape == ESC_S as u32 {
                                should_flip_negation = TRUE;
                                for i in 0..32 {
                                    *classbits.add(i) |= !*cbits.add(i + cbit_space);
                                }
                            }
                            /* When adding the horizontal or vertical space lists to a class,
                            or their complements, disable PCRE2_CASELESS, because it justs
                            wastes time, and in the "not-x" UTF cases can create unwanted
                            duplicates in the XCLASS list (provoked by characters that have
                            more than one other case and by both cases being in the same
                            "not-x" sublist). */
                            else if escape == ESC_h as u32 {
                                if !cranges.is_null() {
                                    break 'switch_escape;
                                }
                                add_list_to_class(
                                    options & !PCRE2_CASELESS,
                                    xoptions,
                                    cb,
                                    _pcre2_hspace_list_8.as_ptr(),
                                );
                            } else if escape == ESC_H as u32 {
                                if !cranges.is_null() {
                                    break 'switch_escape;
                                }
                                add_not_list_to_class(
                                    options & !PCRE2_CASELESS,
                                    xoptions,
                                    cb,
                                    _pcre2_hspace_list_8.as_ptr(),
                                );
                            } else if escape == ESC_v as u32 {
                                if !cranges.is_null() {
                                    break 'switch_escape;
                                }
                                add_list_to_class(
                                    options & !PCRE2_CASELESS,
                                    xoptions,
                                    cb,
                                    _pcre2_vspace_list_8.as_ptr(),
                                );
                            } else if escape == ESC_V as u32 {
                                if !cranges.is_null() {
                                    break 'switch_escape;
                                }
                                add_not_list_to_class(
                                    options & !PCRE2_CASELESS,
                                    xoptions,
                                    cb,
                                    _pcre2_vspace_list_8.as_ptr(),
                                );
                            }
                            /* If Unicode is not supported, \P and \p are not allowed and are
                            faulted at parse time, so will never appear here. */
                            else if escape == ESC_p as u32 || escape == ESC_P as u32 {
                                let ptype: u32 = *pptr >> 16;
                                let pdata: u32 = {
                                    let t = *pptr;
                                    pptr = pptr.add(1);
                                    t
                                } & 0xffff;

                                /* The "Any" is processed by PRIV(update_classbits)(). */
                                if ptype == PT_ANY {
                                    if utf == 0 && escape == ESC_p as u32 {
                                        memset(classbits as *mut c_void, 0xff, 32);
                                    }
                                    continue 'main_loop;
                                }

                                _pcre2_update_classbits_8(
                                    ptype,
                                    pdata,
                                    if escape == ESC_P as u32 { TRUE } else { FALSE },
                                    classbits,
                                );

                                if (xclass_props & XCLASS_HIGH_ANY) == 0 {
                                    if !lengthptr.is_null() {
                                        *lengthptr += 3;
                                    } else {
                                        *class_uchardata = (if escape == ESC_p as u32 {
                                            XCL_PROP
                                        } else {
                                            XCL_NOTPROP
                                        }) as PCRE2_UCHAR;
                                        class_uchardata = class_uchardata.add(1);
                                        *class_uchardata = ptype as PCRE2_UCHAR;
                                        class_uchardata = class_uchardata.add(1);
                                        *class_uchardata = pdata as PCRE2_UCHAR;
                                        class_uchardata = class_uchardata.add(1);
                                    }
                                    xclass_props |= XCLASS_REQUIRED | XCLASS_HAS_PROPS;
                                }
                                continue 'main_loop;
                            }
                        }

                        /* Every non-property class contains at least one < 256 character. */
                        xclass_props |= XCLASS_HAS_8BIT_CHARS;
                        /* End handling \d-type escapes */
                        continue 'main_loop;
                    } else {
                        /* CLASS_END_CASES(meta) */
                        /* Literals. */
                        if meta < META_END {
                            break 'switch_meta;
                        }
                        /* Non-literals: end of class contents. */
                        break 'end_processing; /* goto END_PROCESSING */
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
                    d = {
                        let t = *pptr;
                        pptr = pptr.add(1);
                        t
                    };
                    if d == META_BIGVALUE {
                        d = {
                            let t = *pptr;
                            pptr = pptr.add(1);
                            t
                        };
                    }

                    /* Remember an explicit \r or \n, and add the range to the class. */

                    if d == CHAR_CR || d == CHAR_NL {
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
        }

        /* END_PROCESSING: */

        /* PCRE2_ASSERT((xclass_props & XCLASS_HAS_PROPS) == 0 ||
                        (xclass_props & XCLASS_HIGH_ANY) == 0); */

        if !cranges.is_null() {
            let mut range: *mut u32 = cranges.add(1) as *mut u32;
            let end: *mut u32 = range.add((*cranges).range_list_size as usize);

            while range < end && *range.add(0) < 256 {
                /* Add range to bitset. If we are in UTF or UCP mode, then clear the
                caseless bit, because the cranges handle caselessness (only) in this
                condition; see the condition for PARSE_CLASS_CASELESS_UTF in
                compile_optimize_class(). */
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

                            *lengthptr += _pcre2_ord2utf_8(range_end, class_uchardata) as usize;
                            continue;
                        }

                        *lengthptr += if range_start < range_end { 3 } else { 2 };
                        continue;
                    }

                    if utf != 0 {
                        if range_start < range_end {
                            *class_uchardata = XCL_RANGE as PCRE2_UCHAR;
                            class_uchardata = class_uchardata.add(1);
                            class_uchardata = class_uchardata
                                .add(_pcre2_ord2utf_8(range_start, class_uchardata) as usize);
                        } else {
                            *class_uchardata = XCL_SINGLE as PCRE2_UCHAR;
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
        (\p or \P), we have to compile an extended class, with its own opcode,
        unless there were no property settings and there was a negated special such
        as \S in the class, and PCRE2_UCP is not set, because in that case all
        characters > 255 are in or not in the class, so any that were explicitly
        given as well can be ignored.

        In the UCP case, if certain negated POSIX classes (ex: [:^ascii:]) were
        were present in a class, we either have to match or not match all wide
        characters (depending on whether the whole class is or is not negated).
        This requirement is indicated by match_all_or_no_wide_chars being true.
        We do this by including an explicit range, which works in both cases.
        This applies only in UTF and 16-bit and 32-bit non-UTF modes, since there
        cannot be any wide characters in 8-bit non-UTF mode.

        When there *are* properties in a positive UTF-8 or any 16-bit or 32_bit
        class where \S etc is present without PCRE2_UCP, causing an extended class
        to be compiled, we make sure that all characters > 255 are included by
        forcing match_all_or_no_wide_chars to be true.

        If, when generating an xclass, there are no characters < 256, we can omit
        the bitmap in the actual compiled code. */

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
                    let classwords: *mut u32 = (*cb).classbits.classwords.as_mut_ptr();
                    for i in 0..8 {
                        *classwords.add(i) = !*classwords.add(i);
                    }
                }

                if has_bitmap.is_null() {
                    *code |= XCL_MAP as PCRE2_UCHAR;
                    code = code.add(1);
                    memmove(
                        code.add(32 / size_of::<PCRE2_UCHAR>()) as *mut c_void,
                        code as *const c_void,
                        CU2BYTES!(class_uchardata.offset_from(code) as usize),
                    );
                    memcpy(code as *mut c_void, classbits as *const c_void, 32);
                    code = class_uchardata.add(32 / size_of::<PCRE2_UCHAR>());
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
                /* PCRE2_ASSERT((char_lists_size & 0x1) == 0 &&
                                (cb->char_lists_size & 0x3) == 0); */

                if !lengthptr.is_null() {
                    char_lists_size = CLIST_ALIGN_TO!(char_lists_size, size_of::<u32>());

                    *lengthptr += 2 + LINK_SIZE;

                    (*cb).char_lists_size += char_lists_size;

                    char_lists_size /= size_of::<PCRE2_UCHAR>();

                    /* Storage space for character lists is included
                    in the maximum pattern size. */
                    if *lengthptr > MAX_PATTERN_SIZE
                        || MAX_PATTERN_SIZE - *lengthptr < char_lists_size
                    {
                        *errorcodeptr = ERR20; /* Pattern is too large */
                        return std::ptr::null_mut();
                    }
                } else {
                    let data: *mut u8;

                    /* PCRE2_ASSERT(cranges->char_lists_types <= XCL_TYPE_MASK); */
                    /* Encode as high / low bytes. */
                    *code.add(0) =
                        (XCL_LIST | ((*cranges).char_lists_types as u32 >> 8)) as PCRE2_UCHAR;
                    *code.add(1) = (*cranges).char_lists_types as PCRE2_UCHAR;
                    code = code.add(2);

                    /* Character lists are stored in backwards direction from
                    byte code start. The non-dfa/dfa matchers can access these
                    lists using the byte code start stored in match blocks.
                    Each list is aligned to 32 bit with an optional unused
                    16 bit value at the beginning of the character list. */

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
                    PUT!(code, 0, (char_lists_size >> 1) as u32);
                    code = code.add(LINK_SIZE);

                    /* If we added padding to align the list, initialize the bytes to
                    defined values, so the library is valgrind-clean. It could also
                    be a security concern for clients calling into PCRE2 via bindings
                    from a memory-safe language, if pcre2_serialize_encode() exposes
                    uninitialized memory that may contain sensitive information. */

                    if (char_lists_size & 0x2) != 0 {
                        *(data as *mut u16).offset(-1) = 0xdead;
                    }

                    (*cb).char_lists_size =
                        CLIST_ALIGN_TO!(char_lists_size, size_of::<u32>());

                    ((*(*cb).cx).memctl.free.unwrap())(
                        cranges as *mut c_void,
                        (*(*cb).cx).memctl.memory_data,
                    );
                }
            }

            /* Now fill in the complete length of the item */

            PUT!(previous, 1, code.offset_from(previous) as c_int);
            break 'done; /* End of class handling */
        }

        /* If there are no characters > 255, or they are all to be included or
        excluded, set the opcode to OP_CLASS or OP_NCLASS, depending on whether the
        whole class was negated and whether there were negative specials such as \S
        (non-UCP) in the class. Then copy the 32-byte map into the code vector,
        negating it if necessary. */

        if negate_class != 0 {
            let classwords: *mut u32 = (*cb).classbits.classwords.as_mut_ptr();

            for i in 0..8 {
                *classwords.add(i) = !*classwords.add(i);
            }
        }

        if (SELECT_VALUE8!(utf == 0, false) || negate_class != should_flip_negation)
            && (*cb).classbits.classwords[0] == !0u32
        {
            let classwords: *const u32 = (*cb).classbits.classwords.as_ptr();
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

        *code = (if negate_class == should_flip_negation {
            OP_CLASS
        } else {
            OP_NCLASS
        }) as PCRE2_UCHAR;
        code = code.add(1);
        memcpy(code as *mut c_void, classbits as *const c_void, 32);
        code = code.add(32 / size_of::<PCRE2_UCHAR>());
    }

    /* DONE: */
    *pcode = code;
    return pptr.sub(1);
}
