/* Static helper functions of c_src/src/pcre2_match.c. This file is textually
included by src/pcre2_match.rs, so it contains item definitions only.
The #ifdef DEBUG_FRAMES_DISPLAY function display_frames() is omitted because
that macro is not defined in this build. */

/*************************************************
*                Process a callout               *
*************************************************/

/* This function is called for all callouts, whether "standalone" or at the
start of a conditional group. Feptr will be pointing to either OP_CALLOUT or
OP_CALLOUT_STR. A callout block is allocated in pcre2_match() and initialized
with fixed values.

Arguments:
  F          points to the current backtracking frame
  mb         points to the match block
  lengthptr  where to return the length of the callout item

Returns:     the return from the callout
             or 0 if no callout function exists
*/

unsafe fn do_callout(
    F: *mut heapframe,
    mb: *mut match_block,
    lengthptr: *mut PCRE2_SIZE,
) -> c_int {
    let rc: c_int;
    let save0: PCRE2_SIZE;
    let save1: PCRE2_SIZE;
    let callout_ovector: *mut PCRE2_SIZE;
    let cb: *mut pcre2_callout_block;

    *lengthptr = if *(*F).ecode as u32 == OP_CALLOUT {
        _pcre2_OP_lengths_8[OP_CALLOUT as usize] as PCRE2_SIZE
    } else {
        GET!((*F).ecode, 1 + 2 * LINK_SIZE) as PCRE2_SIZE
    };

    if (*mb).callout.is_none() {
        return 0; /* No callout function provided */
    }

    /* The original matching code (pre 10.30) worked directly with the ovector
    passed by the user, and this was passed to callouts. Now that the working
    ovector is in the backtracking frame, it no longer needs to reserve space for
    the overall match offsets (which would waste space in the frame). For backward
    compatibility, however, we pass capture_top and offset_vector to the callout as
    if for the extended ovector, and we ensure that the first two slots are unset
    by preserving and restoring their current contents. */

    callout_ovector = (*F).ovector.as_mut_ptr().sub(2);

    /* The cb->version, cb->subject, cb->subject_length, and cb->start_match fields
    are set externally. The first 3 never change; the last is updated for each
    bumpalong. */

    cb = (*mb).cb;
    (*cb).capture_top = ((*F).offset_top as u32) / 2 + 1;
    (*cb).capture_last = (*F).capture_last;
    (*cb).offset_vector = callout_ovector;
    (*cb).mark = (*mb).nomatch_mark;
    (*cb).current_position = (*F).eptr.offset_from((*mb).start_subject) as PCRE2_SIZE;
    (*cb).pattern_position = GET!((*F).ecode, 1) as PCRE2_SIZE;
    (*cb).next_item_length = GET!((*F).ecode, 1 + LINK_SIZE) as PCRE2_SIZE;

    if *(*F).ecode as u32 == OP_CALLOUT
    /* Numerical callout */
    {
        (*cb).callout_number = *(*F).ecode.add(1 + 2 * LINK_SIZE) as u32;
        (*cb).callout_string_offset = 0;
        (*cb).callout_string = core::ptr::null();
        (*cb).callout_string_length = 0;
    } else
    /* String callout */
    {
        (*cb).callout_number = 0;
        (*cb).callout_string_offset = GET!((*F).ecode, 1 + 3 * LINK_SIZE) as PCRE2_SIZE;
        (*cb).callout_string = (*F).ecode.add(1 + 4 * LINK_SIZE).add(1);
        (*cb).callout_string_length = *lengthptr - (1 + 4 * LINK_SIZE) - 2;
    }

    save0 = *callout_ovector.add(0);
    save1 = *callout_ovector.add(1);
    *callout_ovector.add(0) = PCRE2_UNSET;
    *callout_ovector.add(1) = PCRE2_UNSET;
    rc = ((*mb).callout.unwrap())(cb, (*mb).callout_data);
    *callout_ovector.add(0) = save0;
    *callout_ovector.add(1) = save1;
    (*cb).callout_flags = 0;
    rc
}

/*************************************************
*          Match a back-reference                *
*************************************************/

/* This function is called only when it is known that the offset lies within
the offsets that have so far been used in the match. Note that in caseless
UTF-8 mode, the number of subject bytes matched may be different to the number
of reference bytes.

Arguments:
  offset      index into the offset vector
  caseless    TRUE if caseless
  caseopts    bitmask of REFI_FLAG_XYZ values
  F           the current backtracking frame pointer
  mb          points to match block
  lengthptr   pointer for returning the length matched

Returns:      = 0 sucessful match; number of code units matched is set
              < 0 no match
              > 0 partial match
*/

unsafe fn match_ref(
    offset: PCRE2_SIZE,
    caseless: BOOL,
    caseopts: c_int,
    F: *mut heapframe,
    mb: *mut match_block,
    lengthptr: *mut PCRE2_SIZE,
) -> c_int {
    let mut p: PCRE2_SPTR;
    let mut length: PCRE2_SIZE;
    let mut eptr: PCRE2_SPTR;
    let eptr_start: PCRE2_SPTR;

    /* Deal with an unset group. The default is no match, but there is an option to
    match an empty string. */

    if offset >= (*F).offset_top || *(*F).ovector.as_mut_ptr().add(offset) == PCRE2_UNSET {
        if ((*mb).poptions & PCRE2_MATCH_UNSET_BACKREF) != 0 {
            *lengthptr = 0;
            return 0; /* Match */
        } else {
            return -1; /* No match */
        }
    }

    /* Separate the caseless and UTF cases for speed. */

    eptr = (*F).eptr;
    eptr_start = eptr;
    p = (*mb)
        .start_subject
        .add(*(*F).ovector.as_mut_ptr().add(offset));
    length = *(*F).ovector.as_mut_ptr().add(offset + 1) - *(*F).ovector.as_mut_ptr().add(offset);

    if caseless != 0 {
        let utf: BOOL = (((*mb).poptions & PCRE2_UTF) != 0) as BOOL;
        let caseless_restrict: BOOL =
            ((caseopts as u32 & REFI_FLAG_CASELESS_RESTRICT) != 0) as BOOL;
        let turkish_casing: BOOL = (caseless_restrict == 0
            && (caseopts as u32 & REFI_FLAG_TURKISH_CASING) != 0) as BOOL;

        if utf != 0 || ((*mb).poptions & PCRE2_UCP) != 0 {
            let endptr: PCRE2_SPTR = p.add(length);

            /* Match characters up to the end of the reference. NOTE: the number of
            code units matched may differ, because in UTF-8 there are some characters
            whose upper and lower case codes have different numbers of bytes. It is
            important, therefore, to check the length along the reference, not along
            the subject. UCP uses Unicode properties but without UTF encoding. */

            while p < endptr {
                let mut c: u32;
                let mut d: u32;
                let mut ur: *const ucd_record = core::ptr::null();
                if eptr >= (*mb).end_subject {
                    return 1; /* Partial match */
                }

                if utf != 0 {
                    GETCHARINC!(c, eptr);
                    GETCHARINC!(d, p);
                } else {
                    c = *eptr as u32;
                    eptr = eptr.add(1);
                    d = *p as u32;
                    p = p.add(1);
                }

                if turkish_casing != 0 && UCD_ANY_I(d) {
                    c = UCD_FOLD_I_TURKISH(c);
                    d = UCD_FOLD_I_TURKISH(d);
                    if c != d {
                        return -1; /* No match */
                    }
                } else if c != d
                    && {
                        ur = GET_UCD(d);
                        c != ((d as c_int).wrapping_add((*ur).other_case) as u32)
                    }
                {
                    let mut pp: *const u32 = _pcre2_ucd_caseless_sets_8
                        .as_ptr()
                        .add((*ur).caseset as usize);

                    /* When PCRE2_EXTRA_CASELESS_RESTRICT is set, ignore any caseless sets
                    that start with an ASCII character. */
                    if caseless_restrict != 0 && *pp < 128 {
                        return -1; /* No match */
                    }

                    loop {
                        if c < *pp {
                            return -1; /* No match */
                        }
                        let t = *pp;
                        pp = pp.add(1);
                        if c == t {
                            break;
                        }
                    }
                }
            }
        }
        /* Not in UTF or UCP mode */
        else {
            while length > 0 {
                let cc: u32;
                let cp: u32;
                if eptr >= (*mb).end_subject {
                    return 1; /* Partial match */
                }
                cc = *eptr as u32;
                cp = *p as u32;
                if TABLE_GET!(cp, (*mb).lcc, cp) != TABLE_GET!(cc, (*mb).lcc, cc) {
                    return -1; /* No match */
                }
                p = p.add(1);
                eptr = eptr.add(1);
                length -= 1;
            }
        }
    }
    /* In the caseful case, we can just compare the code units, whether or not we
    are in UTF and/or UCP mode. When partial matching, we have to do this unit by
    unit. */
    else {
        if (*mb).partial != 0 {
            while length > 0 {
                if eptr >= (*mb).end_subject {
                    return 1; /* Partial match */
                }
                let pc = *p;
                p = p.add(1);
                let ec = *eptr;
                eptr = eptr.add(1);
                if pc != ec {
                    return -1; /* No match */
                }
                length -= 1;
            }
        }
        /* Not partial matching */
        else {
            if ((*mb).end_subject.offset_from(eptr) as PCRE2_SIZE) < length
                || memcmp(
                    p as *const c_void,
                    eptr as *const c_void,
                    CU2BYTES!(length),
                ) != 0
            {
                return -1; /* No match */
            }
            eptr = eptr.add(length);
        }
    }

    *lengthptr = eptr.offset_from(eptr_start) as PCRE2_SIZE;
    0 /* Match */
}

/*************************************************
*     Restore offsets after a recurse            *
*************************************************/

/* This function restores the ovector values when a recursive block reaches its
end, and the triggering recurse has an argument list.

Arguments:
  F           the current backtracking frame pointer
  P           the previous backtracking frame pointer
*/

unsafe fn recurse_update_offsets(F: *mut heapframe, P: *mut heapframe) {
    let mut dst: *mut PCRE2_SIZE = (*F).ovector.as_mut_ptr();
    let mut src: *mut PCRE2_SIZE = (*P).ovector.as_mut_ptr();
    /* The first bracket has offset 2, because
    offset 0 is reserved for the full match. */
    let mut offset: PCRE2_SIZE = 2;
    let offset_top: PCRE2_SIZE = (*F).offset_top + 2;
    let mut diff: PCRE2_SIZE;
    let mut ecode: PCRE2_SPTR = (*F).ecode;

    loop {
        diff = ((GET2!(ecode, 1) << 1) as PCRE2_SIZE).wrapping_sub(offset);
        ecode = ecode.add(1 + IMM2_SIZE);

        if offset.wrapping_add(diff) >= offset_top {
            /* Some OP_CREF opcodes are not
            processed, they must be skipped. */
            while *ecode as u32 == OP_CREF {
                ecode = ecode.add(1 + IMM2_SIZE);
            }
            break;
        }

        if diff == 2 {
            *dst.add(0) = *src.add(0);
            *dst.add(1) = *src.add(1);
        } else if diff >= 4 {
            memcpy(
                dst as *mut c_void,
                src as *const c_void,
                diff * size_of::<PCRE2_SIZE>(),
            );
        }

        /* Skip the unmodified entry. */
        diff = diff.wrapping_add(2);
        offset = offset.wrapping_add(diff);
        dst = dst.add(diff);
        src = src.add(diff);

        if *ecode as u32 != OP_CREF {
            break;
        }
    }

    diff = offset_top.wrapping_sub(offset);
    if diff == 2 {
        *dst.add(0) = *src.add(0);
        *dst.add(1) = *src.add(1);
    } else if diff >= 4 {
        memcpy(
            dst as *mut c_void,
            src as *const c_void,
            diff * size_of::<PCRE2_SIZE>(),
        );
    }

    (*F).ecode = ecode;
    (*F).offset_top = if offset <= (*P).offset_top {
        (*P).offset_top
    } else {
        offset - 2
    };
}
