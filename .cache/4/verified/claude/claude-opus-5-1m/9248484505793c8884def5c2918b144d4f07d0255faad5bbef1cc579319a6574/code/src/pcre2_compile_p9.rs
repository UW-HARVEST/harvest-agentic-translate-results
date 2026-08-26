/* Translated from c_src/src/pcre2_compile.c lines 9393-10278 */

/*************************************************
*             Skip in parsed pattern             *
*************************************************/

/* This function is called to skip parts of the parsed pattern when finding the
length of a lookbehind branch. It is called after (*ACCEPT) and (*FAIL) to find
the end of the branch, it is called to skip over an internal lookaround or
(DEFINE) group, and it is also called to skip to the end of a class, during
which it will never encounter nested groups (but there's no need to have
special code for that).

When called to find the end of a branch or group, pptr must point to the first
meta code inside the branch, not the branch-starting code. In other cases it
can point to the item that causes the function to be called.

Arguments:
  pptr       current pointer to skip from
  skiptype   PSKIP_CLASS when skipping to end of class
             PSKIP_ALT when META_ALT ends the skip
             PSKIP_KET when only META_KET ends the skip

Returns:     new value of pptr
             NULL if META_END is reached - should never occur
               or for an unknown meta value - likewise
*/

unsafe fn parsed_skip(mut pptr: *mut u32, skiptype: u32) -> *mut u32 {
    let mut nestlevel: u32 = 0;

    loop {
        /* The body of the C `for (;; pptr++)` loop; `continue` in the C code
        jumps to the increment, i.e. `break 'continue_outer` here. */
        'continue_outer: {
            let mut meta: u32 = META_CODE!(*pptr);

            if meta == META_END {
                /* The parsed regex is malformed; we have reached the end and did
                not find the end of the construct which we are skipping over. */
                /* PCRE2_DEBUG_UNREACHABLE(); */
                return std::ptr::null_mut();
            }
            /* The data for these items is variable in length. */
            else if meta == META_BACKREF {
                /* Offset is present only if group >= 10 */
                if META_DATA!(*pptr) >= 10 {
                    pptr = pptr.add(SIZEOFFSET);
                }
            } else if meta == META_ESCAPE {
                if (*pptr).wrapping_sub(META_ESCAPE) == ESC_P as u32
                    || (*pptr).wrapping_sub(META_ESCAPE) == ESC_p as u32
                {
                    pptr = pptr.add(1); /* Skip prop data */
                }
            } else if meta == META_MARK
                || meta == META_COMMIT_ARG
                || meta == META_PRUNE_ARG
                || meta == META_SKIP_ARG
                || meta == META_THEN_ARG
            {
                /* Add the length of the name. */
                pptr = pptr.add(*pptr.add(1) as usize);
            }
            /* These are the "active" items in this loop. */
            else if meta == META_CLASS_END {
                if skiptype == PSKIP_CLASS {
                    return pptr;
                }
            } else if meta == META_ATOMIC
                || meta == META_CAPTURE
                || meta == META_COND_ASSERT
                || meta == META_COND_DEFINE
                || meta == META_COND_NAME
                || meta == META_COND_NUMBER
                || meta == META_COND_RNAME
                || meta == META_COND_RNUMBER
                || meta == META_COND_VERSION
                || meta == META_SCS
                || meta == META_LOOKAHEAD
                || meta == META_LOOKAHEADNOT
                || meta == META_LOOKAHEAD_NA
                || meta == META_LOOKBEHIND
                || meta == META_LOOKBEHINDNOT
                || meta == META_LOOKBEHIND_NA
                || meta == META_NOCAPTURE
                || meta == META_SCRIPT_RUN
            {
                nestlevel += 1;
            } else if meta == META_ALT {
                if nestlevel == 0 && skiptype == PSKIP_ALT {
                    return pptr;
                }
            } else if meta == META_KET {
                if nestlevel == 0 {
                    return pptr;
                }
                nestlevel -= 1;
            } else {
                /* default: Just skip over most items */
                if meta < META_END {
                    break 'continue_outer; /* Literal */
                }
            }

            /* The extra data item length for each meta is in a table. */

            meta = (meta >> 16) & 0x7fff;
            if meta as usize >= meta_extra_lengths.len() {
                return std::ptr::null_mut();
            }
            pptr = pptr.add(*meta_extra_lengths.as_ptr().add(meta as usize) as usize);
        }
        pptr = pptr.add(1);
    }

    /* PCRE2_UNREACHABLE(); Control never reaches here */
}

/*************************************************
*       Find length of a parsed group            *
*************************************************/

/* This is called for nested groups within a branch of a lookbehind whose
length is being computed. On entry, the pointer must be at the first element
after the group initializing code. On exit it points to OP_KET. Caching is used
to improve processing speed when the same capturing group occurs many times.

Arguments:
  pptrptr     pointer to pointer in the parsed pattern
  minptr      where to return the minimum length
  isinline    FALSE if a reference or recursion; TRUE for inline group
  errcodeptr  pointer to the errorcode
  lcptr       pointer to the loop counter
  group       number of captured group or -1 for a non-capturing group
  recurses    chain of recurse_check to catch mutual recursion
  cb          pointer to the compile data

Returns:      the maximum group length or a negative number
*/

unsafe fn get_grouplength(
    pptrptr: *mut *mut u32,
    minptr: *mut c_int,
    isinline: BOOL,
    errcodeptr: *mut c_int,
    lcptr: *mut c_int,
    group: c_int,
    recurses: *mut parsed_recurse_check,
    cb: *mut compile_block,
) -> c_int {
    /* wrapping_offset because C computes cb->groupinfo + 2*group even when
    group is negative (the result is then never dereferenced). */
    let gi: *mut u32 = (*cb).groupinfo.wrapping_offset((2 * group) as isize);
    let mut branchlength: c_int;
    let mut branchminlength: c_int = 0;
    let mut grouplength: c_int = -1;
    let mut groupminlength: c_int = i32::MAX; /* INT_MAX */

    'isnotfixed: {
        /* The cache can be used only if there is no possibility of there being two
        groups with the same number. We do not need to set the end pointer for a group
        that is being processed as a back reference or recursion, but we must do so for
        an inline group. */

        if group > 0 && ((*cb).external_flags & PCRE2_DUPCAPUSED) == 0 {
            let groupinfo: u32 = *gi;
            if (groupinfo & GI_NOT_FIXED_LENGTH) != 0 {
                return -1;
            }
            if (groupinfo & GI_SET_FIXED_LENGTH) != 0 {
                if isinline != 0 {
                    *pptrptr = parsed_skip(*pptrptr, PSKIP_KET);
                }
                *minptr = *gi.add(1) as c_int;
                return (groupinfo & GI_FIXED_LENGTH_MASK) as c_int;
            }
        }

        /* Scan the group. In this case we find the end pointer of necessity. */

        loop {
            branchlength = get_branchlength(
                pptrptr,
                &mut branchminlength as *mut c_int,
                errcodeptr,
                lcptr,
                recurses,
                cb,
            );
            if branchlength < 0 {
                break 'isnotfixed;
            }
            if branchlength > grouplength {
                grouplength = branchlength;
            }
            if branchminlength < groupminlength {
                groupminlength = branchminlength;
            }
            if **pptrptr == META_KET {
                break;
            }
            *pptrptr = (*pptrptr).add(1); /* Skip META_ALT */
        }

        if group > 0 {
            *gi |= GI_SET_FIXED_LENGTH | grouplength as u32;
            *gi.add(1) = groupminlength as u32;
        }

        *minptr = groupminlength;
        return grouplength;
    }

    /* ISNOTFIXED: */
    if group > 0 {
        *gi |= GI_NOT_FIXED_LENGTH;
    }
    return -1;
}

/*************************************************
*        Find length of a parsed branch          *
*************************************************/

/* Return fixed maximum and minimum lengths for a branch in a lookbehind,
giving an error if the length is not limited. On entry, *pptrptr points to the
first element inside the branch. On exit it is set to point to the ALT or KET.

Arguments:
  pptrptr     pointer to pointer in the parsed pattern
  minptr      where to return the minimum length
  errcodeptr  pointer to error code
  lcptr       pointer to loop counter
  recurses    chain of recurse_check to catch mutual recursion
  cb          pointer to compile block

Returns:      the maximum length, or a negative value on error
*/

unsafe fn get_branchlength(
    pptrptr: *mut *mut u32,
    minptr: *mut c_int,
    errcodeptr: *mut c_int,
    lcptr: *mut c_int,
    recurses: *mut parsed_recurse_check,
    cb: *mut compile_block,
) -> c_int {
    let mut branchlength: c_int = 0;
    let mut branchminlength: c_int = 0;
    let mut grouplength: c_int;
    let mut groupminlength: c_int = 0;
    let mut lastitemlength: u32 = 0;
    let mut lastitemminlength: u32 = 0;
    let mut pptr: *mut u32 = *pptrptr;
    let mut offset: PCRE2_SIZE = 0;
    let mut this_recurse: parsed_recurse_check = parsed_recurse_check {
        prev: std::ptr::null_mut(),
        groupptr: std::ptr::null_mut(),
    };

    /* A large and/or complex regex can take too long to process. This can happen
    more often when (?| groups are present in the pattern because their length
    cannot be cached. */

    /* (*lcptr)++ > 2000 : the post-incremented (old) value is tested. */
    let lc_old: c_int = *lcptr;
    *lcptr = lc_old.wrapping_add(1);
    if lc_old > 2000 {
        *errcodeptr = ERR35; /* Lookbehind is too complicated */
        return -1;
    }

    /* Scan the branch, accumulating the length. */

    'parsed_skip_failed: {
        'exit: {
            loop {
                let mut r: *mut parsed_recurse_check;
                let mut gptr: *mut u32;
                let gptrend: *mut u32;
                let escape: u32;
                let mut min: u32 = 0;
                let mut max: u32 = 0;
                let mut group: u32 = 0;
                let mut itemlength: u32 = 0;
                let mut itemminlength: u32 = 0;

                if *pptr < META_END {
                    itemlength = 1;
                    itemminlength = 1;
                } else {
                    'sw: {
                        'isnotfixed: {
                            'repetition: {
                                'check_group: {
                                    'recurse_or_backref_length: {
                                        let mc: u32 = META_CODE!(*pptr);

                                        if mc == META_KET || mc == META_ALT {
                                            break 'exit;
                                        }
                                        /* (*ACCEPT) and (*FAIL) terminate the branch, but we must
                                        skip to the actual termination. */
                                        else if mc == META_ACCEPT || mc == META_FAIL {
                                            pptr = parsed_skip(pptr, PSKIP_ALT);
                                            if pptr.is_null() {
                                                break 'parsed_skip_failed;
                                            }
                                            break 'exit;
                                        } else if mc == META_MARK
                                            || mc == META_COMMIT_ARG
                                            || mc == META_PRUNE_ARG
                                            || mc == META_SKIP_ARG
                                            || mc == META_THEN_ARG
                                        {
                                            pptr =
                                                pptr.add((*pptr.add(1)).wrapping_add(1) as usize);
                                            break 'sw;
                                        } else if mc == META_CIRCUMFLEX
                                            || mc == META_COMMIT
                                            || mc == META_DOLLAR
                                            || mc == META_PRUNE
                                            || mc == META_SKIP
                                            || mc == META_THEN
                                        {
                                            break 'sw;
                                        } else if mc == META_OPTIONS {
                                            pptr = pptr.add(2);
                                            break 'sw;
                                        } else if mc == META_BIGVALUE {
                                            itemlength = 1;
                                            itemminlength = 1;
                                            pptr = pptr.add(1);
                                            break 'sw;
                                        } else if mc == META_CLASS || mc == META_CLASS_NOT {
                                            itemlength = 1;
                                            itemminlength = 1;
                                            pptr = parsed_skip(pptr, PSKIP_CLASS);
                                            if pptr.is_null() {
                                                break 'parsed_skip_failed;
                                            }
                                            break 'sw;
                                        } else if mc == META_CLASS_EMPTY_NOT || mc == META_DOT {
                                            itemlength = 1;
                                            itemminlength = 1;
                                            break 'sw;
                                        } else if mc == META_CALLOUT_NUMBER {
                                            pptr = pptr.add(3);
                                            break 'sw;
                                        } else if mc == META_CALLOUT_STRING {
                                            pptr = pptr.add(3 + SIZEOFFSET);
                                            break 'sw;
                                        }
                                        /* Only some escapes consume a character. Of those, \R can
                                        match one or two characters, but \X is never allowed because
                                        it matches an unknown number of characters. \C is allowed
                                        only in 32-bit and non-UTF 8/16-bit modes. */
                                        else if mc == META_ESCAPE {
                                            escape = META_DATA!(*pptr);
                                            if escape == ESC_X as u32 {
                                                return -1;
                                            }
                                            if escape == ESC_R as u32 {
                                                itemminlength = 1;
                                                itemlength = 2;
                                            } else if escape > ESC_b as u32 && escape < ESC_Z as u32
                                            {
                                                if ((*cb).external_options & PCRE2_UTF) != 0
                                                    && escape == ESC_C as u32
                                                {
                                                    *errcodeptr = ERR36;
                                                    return -1;
                                                }
                                                itemlength = 1;
                                                itemminlength = 1;
                                                if escape == ESC_p as u32 || escape == ESC_P as u32
                                                {
                                                    pptr = pptr.add(1); /* Skip prop data */
                                                }
                                            }
                                            break 'sw;
                                        }
                                        /* Lookaheads do not contribute to the length of this branch,
                                        but they may contain lookbehinds within them whose lengths
                                        need to be set. */
                                        else if mc == META_LOOKAHEAD
                                            || mc == META_LOOKAHEADNOT
                                            || mc == META_LOOKAHEAD_NA
                                            || mc == META_SCS
                                        {
                                            *errcodeptr = check_lookbehinds(
                                                pptr.add(1),
                                                &mut pptr as *mut *mut u32,
                                                recurses,
                                                cb,
                                                lcptr,
                                            );
                                            if *errcodeptr != 0 {
                                                return -1;
                                            }

                                            /* Ignore any qualifiers that follow a lookahead
                                            assertion. */

                                            let q: u32 = *pptr.add(1);
                                            if q == META_ASTERISK
                                                || q == META_ASTERISK_PLUS
                                                || q == META_ASTERISK_QUERY
                                                || q == META_PLUS
                                                || q == META_PLUS_PLUS
                                                || q == META_PLUS_QUERY
                                                || q == META_QUERY
                                                || q == META_QUERY_PLUS
                                                || q == META_QUERY_QUERY
                                            {
                                                pptr = pptr.add(1);
                                            } else if q == META_MINMAX
                                                || q == META_MINMAX_PLUS
                                                || q == META_MINMAX_QUERY
                                            {
                                                pptr = pptr.add(3);
                                            }
                                            break 'sw;
                                        }
                                        /* A nested lookbehind does not contribute any length to this
                                        lookbehind, but must itself be checked and have its lengths
                                        set. Note that set_lookbehind_lengths() updates pptr, leaving
                                        it pointing to the final ket of the group, so no need to
                                        update it here. */
                                        else if mc == META_LOOKBEHIND
                                            || mc == META_LOOKBEHINDNOT
                                            || mc == META_LOOKBEHIND_NA
                                        {
                                            if set_lookbehind_lengths(
                                                &mut pptr as *mut *mut u32,
                                                errcodeptr,
                                                lcptr,
                                                recurses,
                                                cb,
                                            ) == 0
                                            {
                                                return -1;
                                            }
                                            break 'sw;
                                        }
                                        /* Back references and recursions are handled by very similar
                                        code. At this stage, the names generated in the parsing pass
                                        are available, but the main name table has not yet been
                                        created. So for the named varieties, scan the list of names in
                                        order to get the number of the first one in the pattern, and
                                        whether or not this name is duplicated. */
                                        else if mc == META_BACKREF_BYNAME
                                            || mc == META_RECURSE_BYNAME
                                        {
                                            if mc == META_BACKREF_BYNAME
                                                && ((*cb).external_options
                                                    & PCRE2_MATCH_UNSET_BACKREF)
                                                    != 0
                                            {
                                                break 'isnotfixed;
                                            }
                                            /* Fall through */
                                            {
                                                let name: PCRE2_SPTR;
                                                let mut is_dupname: BOOL = FALSE;
                                                let ng: *mut named_group;
                                                let meta_code: u32 = META_CODE!(*pptr);
                                                pptr = pptr.add(1);
                                                let length: u32 = *pptr;

                                                GETPLUSOFFSET!(offset, pptr);
                                                name = (*cb).start_pattern.add(offset);
                                                ng = _pcre2_compile_find_named_group8(
                                                    name, length, cb,
                                                );

                                                if ng.is_null() {
                                                    *errcodeptr = ERR15; /* Non-existent subpattern */
                                                    (*cb).erroroffset = offset;
                                                    return -1;
                                                }

                                                group = (*ng).number;
                                                is_dupname = if ((*ng).hash_dup
                                                    & NAMED_GROUP_IS_DUPNAME)
                                                    != 0
                                                {
                                                    TRUE
                                                } else {
                                                    FALSE
                                                };

                                                /* A numerical back reference can be fixed length if
                                                duplicate capturing groups are not being used. A
                                                non-duplicate named back reference can also be
                                                handled. */

                                                if meta_code == META_RECURSE_BYNAME
                                                    || (is_dupname == FALSE
                                                        && ((*cb).external_flags
                                                            & PCRE2_DUPCAPUSED)
                                                            == 0)
                                                {
                                                    /* Handle as a numbered version. */
                                                    break 'recurse_or_backref_length;
                                                }
                                            }
                                            break 'isnotfixed; /* Duplicate name or number */
                                        }
                                        /* The offset values for back references < 10 are in a
                                        separate vector because otherwise they would use more than
                                        two parsed pattern elements on 64-bit systems. */
                                        /* A true recursion implies not fixed length, but a subroutine
                                        call may be OK. Back reference "recursions" are also
                                        failed. */
                                        else if mc == META_BACKREF || mc == META_RECURSE {
                                            if mc == META_BACKREF {
                                                if ((*cb).external_options
                                                    & PCRE2_MATCH_UNSET_BACKREF)
                                                    != 0
                                                    || ((*cb).external_flags & PCRE2_DUPCAPUSED)
                                                        != 0
                                                {
                                                    break 'isnotfixed;
                                                }
                                                group = META_DATA!(*pptr);
                                                if group < 10 {
                                                    offset = *(*cb)
                                                        .small_ref_offset
                                                        .as_ptr()
                                                        .add(group as usize);
                                                    break 'recurse_or_backref_length;
                                                }
                                                /* Fall through */
                                                /* For groups >= 10 - picking up group twice does no
                                                harm. */
                                            }
                                            group = META_DATA!(*pptr);
                                            GETPLUSOFFSET!(offset, pptr);
                                            break 'recurse_or_backref_length;
                                        }
                                        /* A (DEFINE) group is never obeyed inline and so it does not
                                        contribute to the length of this branch. Skip from the
                                        following item to the next unpaired ket. */
                                        else if mc == META_COND_DEFINE {
                                            pptr = parsed_skip(pptr.add(1), PSKIP_KET);
                                            break 'sw;
                                        }
                                        /* Check other nested groups - advance past the initial data
                                        for each type and then seek a fixed length with
                                        get_grouplength(). */
                                        else if mc == META_COND_NAME
                                            || mc == META_COND_NUMBER
                                            || mc == META_COND_RNAME
                                            || mc == META_COND_RNUMBER
                                        {
                                            pptr = pptr.add(2 + SIZEOFFSET);
                                            break 'check_group;
                                        } else if mc == META_COND_ASSERT {
                                            pptr = pptr.add(1);
                                            break 'check_group;
                                        } else if mc == META_COND_VERSION {
                                            pptr = pptr.add(4);
                                            break 'check_group;
                                        } else if mc == META_CAPTURE
                                            || mc == META_ATOMIC
                                            || mc == META_NOCAPTURE
                                            || mc == META_SCRIPT_RUN
                                        {
                                            if mc == META_CAPTURE {
                                                group = META_DATA!(*pptr);
                                                /* Fall through */
                                            }
                                            pptr = pptr.add(1);
                                            break 'check_group;
                                        } else if mc == META_QUERY
                                            || mc == META_QUERY_PLUS
                                            || mc == META_QUERY_QUERY
                                        {
                                            min = 0;
                                            max = 1;
                                            break 'repetition;
                                        }
                                        /* Exact repetition is OK; variable repetition is not. A
                                        repetition of zero must subtract the length that has already
                                        been added. */
                                        else if mc == META_MINMAX
                                            || mc == META_MINMAX_PLUS
                                            || mc == META_MINMAX_QUERY
                                        {
                                            min = *pptr.add(1);
                                            max = *pptr.add(2);
                                            pptr = pptr.add(2);
                                            break 'repetition;
                                        }
                                        /* Any other item means this branch does not have a fixed
                                        length. */
                                        else {
                                            break 'isnotfixed;
                                        }
                                    }

                                    /* RECURSE_OR_BACKREF_LENGTH: */
                                    if group > (*cb).bracount {
                                        (*cb).erroroffset = offset;
                                        *errcodeptr = ERR15; /* Non-existent subpattern */
                                        return -1;
                                    }
                                    if group == 0 {
                                        break 'isnotfixed; /* Local recursion */
                                    }
                                    gptr = (*cb).parsed_pattern;
                                    while *gptr != META_END {
                                        if META_CODE!(*gptr) == META_BIGVALUE {
                                            gptr = gptr.add(1);
                                        } else if *gptr == (META_CAPTURE | group) {
                                            break;
                                        }
                                        gptr = gptr.add(1);
                                    }

                                    /* We must start the search for the end of the group at the first
                                    meta code inside the group. Otherwise it will be treated as an
                                    enclosed group. */

                                    gptrend = parsed_skip(gptr.add(1), PSKIP_KET);
                                    if gptrend.is_null() {
                                        break 'parsed_skip_failed;
                                    }
                                    if pptr > gptr && pptr < gptrend {
                                        break 'isnotfixed; /* Local recursion */
                                    }
                                    r = recurses;
                                    while !r.is_null() {
                                        if (*r).groupptr == gptr {
                                            break;
                                        }
                                        r = (*r).prev;
                                    }
                                    if !r.is_null() {
                                        break 'isnotfixed; /* Mutual recursion */
                                    }
                                    this_recurse.prev = recurses;
                                    this_recurse.groupptr = gptr;

                                    /* We do not need to know the position of the end of the group,
                                    that is, gptr is not used after the call to get_grouplength().
                                    Setting the second argument FALSE stops it scanning for the end
                                    when the length can be found in the cache. */

                                    gptr = gptr.add(1);
                                    grouplength = get_grouplength(
                                        &mut gptr as *mut *mut u32,
                                        &mut groupminlength as *mut c_int,
                                        FALSE,
                                        errcodeptr,
                                        lcptr,
                                        group as c_int,
                                        &mut this_recurse as *mut parsed_recurse_check,
                                        cb,
                                    );
                                    if grouplength < 0 {
                                        if *errcodeptr == 0 {
                                            break 'isnotfixed;
                                        }
                                        return -1; /* Error already set */
                                    }
                                    itemlength = grouplength as u32;
                                    itemminlength = groupminlength as u32;
                                    break 'sw;
                                }

                                /* CHECK_GROUP: */
                                grouplength = get_grouplength(
                                    &mut pptr as *mut *mut u32,
                                    &mut groupminlength as *mut c_int,
                                    TRUE,
                                    errcodeptr,
                                    lcptr,
                                    group as c_int,
                                    recurses,
                                    cb,
                                );
                                if grouplength < 0 {
                                    return -1;
                                }
                                itemlength = grouplength as u32;
                                itemminlength = groupminlength as u32;
                                break 'sw;
                            }

                            /* REPETITION: */
                            if max != REPEAT_UNLIMITED {
                                if lastitemlength != 0 && /* Should not occur, but just in case */
                                   max != 0 &&
                                   ((i32::MAX.wrapping_sub(branchlength)) as u32) / lastitemlength
                                     < max.wrapping_sub(1)
                                {
                                    *errcodeptr = ERR87; /* Integer overflow; lookbehind too big */
                                    return -1;
                                }
                                if min == 0 {
                                    branchminlength = (branchminlength as u32)
                                        .wrapping_sub(lastitemminlength)
                                        as c_int;
                                } else {
                                    itemminlength =
                                        min.wrapping_sub(1).wrapping_mul(lastitemminlength);
                                }
                                if max == 0 {
                                    branchlength =
                                        (branchlength as u32).wrapping_sub(lastitemlength) as c_int;
                                } else {
                                    itemlength = max.wrapping_sub(1).wrapping_mul(lastitemlength);
                                }
                                break 'sw;
                            }
                            /* Fall through */
                        }

                        /* ISNOTFIXED: (also the switch default) */
                        *errcodeptr = ERR25; /* Not fixed length */
                        return -1;
                    }
                }

                /* Add the item length to the branchlength, checking for integer overflow
                and for the branch length exceeding the overall limit. Later, if there is
                at least one variable-length branch in the group, there is a test for the
                (smaller) variable-length branch length limit. */

                /* if (INT_MAX - branchlength < (int)itemlength ||
                   (branchlength += itemlength) > LOOKBEHIND_MAX)  - the second
                operand is evaluated only when the first one is false. */
                let mut too_big: bool = false;
                if i32::MAX.wrapping_sub(branchlength) < itemlength as c_int {
                    too_big = true;
                } else {
                    branchlength = (branchlength as u32).wrapping_add(itemlength) as c_int;
                    if branchlength > LOOKBEHIND_MAX {
                        too_big = true;
                    }
                }
                if too_big {
                    *errcodeptr = ERR87;
                    return -1;
                }

                branchminlength = (branchminlength as u32).wrapping_add(itemminlength) as c_int;

                /* Save this item length for use if the next item is a quantifier. */

                lastitemlength = itemlength;
                lastitemminlength = itemminlength;

                pptr = pptr.add(1);
            }
        }

        /* EXIT: */
        *pptrptr = pptr;
        *minptr = branchminlength;
        return branchlength;
    }

    /* PARSED_SKIP_FAILED: */
    /* PCRE2_DEBUG_UNREACHABLE(); */
    *errcodeptr = ERR90; /* Unhandled META code - internal error */
    return -1;
}

/*************************************************
*        Set lengths in a lookbehind             *
*************************************************/

/* This function is called for each lookbehind, to set the lengths in its
branches. An error occurs if any branch does not have a limited maximum length
that is less than the limit (65535). On exit, the pointer must be left on the
final ket.

The function also maintains the max_lookbehind value. Any lookbehind branch
that contains a nested lookbehind may actually look further back than the
length of the branch. The additional amount is passed back from
get_branchlength() as an "extra" value.

Arguments:
  pptrptr     pointer to pointer in the parsed pattern
  errcodeptr  pointer to error code
  lcptr       pointer to loop counter
  recurses    chain of recurse_check to catch mutual recursion
  cb          pointer to compile block

Returns:      TRUE if all is well
              FALSE otherwise, with error code and offset set
*/

unsafe fn set_lookbehind_lengths(
    pptrptr: *mut *mut u32,
    errcodeptr: *mut c_int,
    lcptr: *mut c_int,
    recurses: *mut parsed_recurse_check,
    cb: *mut compile_block,
) -> BOOL {
    let offset: PCRE2_SIZE;
    let mut bptr: *mut u32 = *pptrptr;
    let gbptr: *mut u32 = bptr;
    let mut maxlength: c_int = 0;
    let mut minlength: c_int = i32::MAX; /* INT_MAX */
    let mut variable: BOOL = FALSE;

    READPLUSOFFSET!(offset, bptr); /* Offset for error messages */
    *pptrptr = (*pptrptr).add(SIZEOFFSET);

    /* Each branch can have a different maximum length, but we can keep only a
    single minimum for the whole group, because there's nowhere to save individual
    values in the META_ALT item. */

    loop {
        let branchlength: c_int;
        let mut branchminlength: c_int = 0;

        *pptrptr = (*pptrptr).add(1);
        branchlength = get_branchlength(
            pptrptr,
            &mut branchminlength as *mut c_int,
            errcodeptr,
            lcptr,
            recurses,
            cb,
        );

        if branchlength < 0 {
            /* The errorcode and offset may already be set from a nested lookbehind. */
            if *errcodeptr == 0 {
                *errcodeptr = ERR25;
            }
            if (*cb).erroroffset == PCRE2_UNSET {
                (*cb).erroroffset = offset;
            }
            return FALSE;
        }

        if branchlength != branchminlength {
            variable = TRUE;
        }
        if branchminlength < minlength {
            minlength = branchminlength;
        }
        if branchlength > maxlength {
            maxlength = branchlength;
        }
        if branchlength > (*cb).max_lookbehind {
            (*cb).max_lookbehind = branchlength;
        }
        *bptr |= branchlength as u32; /* branchlength never more than 65535 */
        bptr = *pptrptr;

        if !(META_CODE!(*bptr) == META_ALT) {
            break;
        }
    }

    /* If any branch is of variable length, the whole lookbehind is of variable
    length. If the maximum length of any branch exceeds the maximum for variable
    lookbehinds, give an error. Otherwise, the minimum length is set in the word
    that follows the original group META value. For a fixed-length lookbehind, this
    is set to LOOKBEHIND_MAX, to indicate that each branch is of a fixed (but
    possibly different) length. */

    if variable != 0 {
        *gbptr.add(1) = minlength as u32;
        if (maxlength as PCRE2_SIZE) > (*cb).max_varlookbehind as PCRE2_SIZE {
            *errcodeptr = ERR100;
            (*cb).erroroffset = offset;
            return FALSE;
        }
    } else {
        *gbptr.add(1) = LOOKBEHIND_MAX as u32;
    }

    return TRUE;
}

/*************************************************
*         Check parsed pattern lookbehinds       *
*************************************************/

/* This function is called at the end of parsing a pattern if any lookbehinds
were encountered. It scans the parsed pattern for them, calling
set_lookbehind_lengths() for each one. At the start, the errorcode is zero and
the error offset is marked unset. The enables the functions above not to
override settings from deeper nestings.

This function is called recursively from get_branchlength() for lookaheads in
order to process any lookbehinds that they may contain. It stops when it hits a
non-nested closing parenthesis in this case, returning a pointer to it.

Arguments
  pptr      points to where to start (start of pattern or start of lookahead)
  retptr    if not NULL, return the ket pointer here
  recurses  chain of recurse_check to catch mutual recursion
  cb        points to the compile block
  lcptr     points to loop counter

Returns:    0 on success, or an errorcode (cb->erroroffset will be set)
*/

unsafe fn check_lookbehinds(
    mut pptr: *mut u32,
    retptr: *mut *mut u32,
    recurses: *mut parsed_recurse_check,
    cb: *mut compile_block,
    lcptr: *mut c_int,
) -> c_int {
    let mut errorcode: c_int = 0;
    let mut nestlevel: c_int = 0;

    (*cb).erroroffset = PCRE2_UNSET;

    while *pptr != META_END {
        'continue_outer: {
            if *pptr < META_END {
                break 'continue_outer; /* Literal */
            }

            let mc: u32 = META_CODE!(*pptr);

            if mc == META_ESCAPE {
                if (*pptr).wrapping_sub(META_ESCAPE) == ESC_P as u32
                    || (*pptr).wrapping_sub(META_ESCAPE) == ESC_p as u32
                {
                    pptr = pptr.add(1); /* Skip prop data */
                }
            } else if mc == META_KET {
                nestlevel -= 1;
                if nestlevel < 0 {
                    if !retptr.is_null() {
                        *retptr = pptr;
                    }
                    return 0;
                }
            } else if mc == META_ATOMIC
                || mc == META_CAPTURE
                || mc == META_COND_ASSERT
                || mc == META_SCS
                || mc == META_LOOKAHEAD
                || mc == META_LOOKAHEADNOT
                || mc == META_LOOKAHEAD_NA
                || mc == META_NOCAPTURE
                || mc == META_SCRIPT_RUN
            {
                nestlevel += 1;
            } else if mc == META_ACCEPT
                || mc == META_ALT
                || mc == META_ASTERISK
                || mc == META_ASTERISK_PLUS
                || mc == META_ASTERISK_QUERY
                || mc == META_BACKREF
                || mc == META_CIRCUMFLEX
                || mc == META_CLASS
                || mc == META_CLASS_EMPTY
                || mc == META_CLASS_EMPTY_NOT
                || mc == META_CLASS_END
                || mc == META_CLASS_NOT
                || mc == META_COMMIT
                || mc == META_DOLLAR
                || mc == META_DOT
                || mc == META_FAIL
                || mc == META_PLUS
                || mc == META_PLUS_PLUS
                || mc == META_PLUS_QUERY
                || mc == META_PRUNE
                || mc == META_QUERY
                || mc == META_QUERY_PLUS
                || mc == META_QUERY_QUERY
                || mc == META_RANGE_ESCAPED
                || mc == META_RANGE_LITERAL
                || mc == META_SKIP
                || mc == META_THEN
            {
                /* Nothing to do */
            } else if mc == META_OFFSET || mc == META_RECURSE {
                pptr = pptr.add(SIZEOFFSET);
            } else if mc == META_BACKREF_BYNAME || mc == META_RECURSE_BYNAME {
                pptr = pptr.add(1 + SIZEOFFSET);
            } else if mc == META_COND_DEFINE {
                pptr = pptr.add(SIZEOFFSET);
                nestlevel += 1;
            } else if mc == META_COND_NAME
                || mc == META_COND_NUMBER
                || mc == META_COND_RNAME
                || mc == META_COND_RNUMBER
            {
                pptr = pptr.add(1 + SIZEOFFSET);
                nestlevel += 1;
            } else if mc == META_COND_VERSION {
                pptr = pptr.add(3);
                nestlevel += 1;
            } else if mc == META_CALLOUT_STRING {
                pptr = pptr.add(3 + SIZEOFFSET);
            } else if mc == META_BIGVALUE
                || mc == META_POSIX
                || mc == META_POSIX_NEG
                || mc == META_CAPTURE_NAME
                || mc == META_CAPTURE_NUMBER
            {
                pptr = pptr.add(1);
            } else if mc == META_MINMAX
                || mc == META_MINMAX_QUERY
                || mc == META_MINMAX_PLUS
                || mc == META_OPTIONS
            {
                pptr = pptr.add(2);
            } else if mc == META_CALLOUT_NUMBER {
                pptr = pptr.add(3);
            } else if mc == META_MARK
                || mc == META_COMMIT_ARG
                || mc == META_PRUNE_ARG
                || mc == META_SKIP_ARG
                || mc == META_THEN_ARG
            {
                pptr = pptr.add(1u32.wrapping_add(*pptr.add(1)) as usize);
            }
            /* Note that set_lookbehind_lengths() updates pptr, leaving it pointing to
            the final ket of the group, so no need to update it here. */
            else if mc == META_LOOKBEHIND || mc == META_LOOKBEHINDNOT || mc == META_LOOKBEHIND_NA
            {
                if set_lookbehind_lengths(
                    &mut pptr as *mut *mut u32,
                    &mut errorcode as *mut c_int,
                    lcptr,
                    recurses,
                    cb,
                ) == 0
                {
                    return errorcode;
                }
            } else {
                /* default: The following erroroffset is a bogus but safe value. This
                branch should be avoided by providing a proper implementation for all
                supported cases above. */
                /* PCRE2_DEBUG_UNREACHABLE(); */
                (*cb).erroroffset = 0;
                return ERR70; /* Unrecognized meta code */
            }
        }
        pptr = pptr.add(1);
    }

    return 0;
}
