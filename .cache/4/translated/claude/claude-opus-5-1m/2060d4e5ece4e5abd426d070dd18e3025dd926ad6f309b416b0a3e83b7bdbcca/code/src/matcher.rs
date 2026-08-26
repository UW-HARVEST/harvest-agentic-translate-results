// Translated from pcre2_match.c (everything except the match() function itself,
// which lives in src/matcher_core.rs + src/matcher_arms/*.rs).
use crate::internal::*;
use crate::matcher_core::*;
use crate::pcre2_pub::*;
use crate::tables::*;
use crate::ucd_data::*;
use crate::ucp::*;
use core::ffi::{c_char, c_int, c_uint, c_void};

/* NLBLOCK is mb, PSSTART is start_subject, PSEND is end_subject. */

pub(crate) const RECURSE_UNSET: u32 = 0xffffffff; /* Bigger than max group number */

pub(crate) const PUBLIC_MATCH_OPTIONS: u32 = PCRE2_ANCHORED
    | PCRE2_ENDANCHORED
    | PCRE2_NOTBOL
    | PCRE2_NOTEOL
    | PCRE2_NOTEMPTY
    | PCRE2_NOTEMPTY_ATSTART
    | PCRE2_NO_UTF_CHECK
    | PCRE2_PARTIAL_HARD
    | PCRE2_PARTIAL_SOFT
    | PCRE2_NO_JIT
    | PCRE2_COPY_MATCHED_SUBJECT
    | PCRE2_DISABLE_RECURSELOOP_CHECK;

pub(crate) const PUBLIC_JIT_MATCH_OPTIONS: u32 = PCRE2_NO_UTF_CHECK
    | PCRE2_NOTBOL
    | PCRE2_NOTEOL
    | PCRE2_NOTEMPTY
    | PCRE2_NOTEMPTY_ATSTART
    | PCRE2_PARTIAL_SOFT
    | PCRE2_PARTIAL_HARD
    | PCRE2_COPY_MATCHED_SUBJECT;

pub(crate) const MATCH_MATCH: c_int = 1;
pub(crate) const MATCH_NOMATCH: c_int = 0;

pub(crate) const MATCH_ACCEPT: c_int = -999;
pub(crate) const MATCH_KETRPOS: c_int = -998;
pub(crate) const MATCH_COMMIT: c_int = -997;
pub(crate) const MATCH_PRUNE: c_int = -996;
pub(crate) const MATCH_SKIP: c_int = -995;
pub(crate) const MATCH_SKIP_ARG: c_int = -994;
pub(crate) const MATCH_THEN: c_int = -993;
pub(crate) const MATCH_BACKTRACK_MAX: c_int = MATCH_THEN;
pub(crate) const MATCH_BACKTRACK_MIN: c_int = MATCH_COMMIT;

pub(crate) const GF_CAPTURE: u32 = 0x00010000;
pub(crate) const GF_NOCAPTURE: u32 = 0x00020000;
pub(crate) const GF_CONDASSERT: u32 = 0x00030000;
pub(crate) const GF_RECURSE: u32 = 0x00040000;

#[inline(always)]
pub(crate) fn GF_IDMASK(a: u32) -> u32 {
    a & 0xffff0000
}
#[inline(always)]
pub(crate) fn GF_DATAMASK(a: u32) -> u32 {
    a & 0x0000ffff
}

/* Repetition types */
pub(crate) const REPTYPE_MIN: u32 = 0;
pub(crate) const REPTYPE_MAX: u32 = 1;
pub(crate) const REPTYPE_POS: u32 = 2;

pub(crate) static rep_min: [u32; 11] = [
    0, 0, /* * and *? */
    1, 1, /* + and +? */
    0, 0, /* ? and ?? */
    0, 0, /* dummy placefillers for OP_CR[MIN]RANGE */
    0, 1, 0, /* OP_CRPOS{STAR, PLUS, QUERY} */
];

pub(crate) static rep_max: [u32; 11] = [
    u32::MAX,
    u32::MAX, /* * and *? */
    u32::MAX,
    u32::MAX, /* + and +? */
    1,
    1, /* ? and ?? */
    0,
    0, /* dummy placefillers for OP_CR[MIN]RANGE */
    u32::MAX,
    u32::MAX,
    1, /* OP_CRPOS{STAR, PLUS, QUERY} */
];

pub(crate) static rep_typ: [u32; 12] = [
    REPTYPE_MAX, REPTYPE_MIN, /* * and *? */
    REPTYPE_MAX, REPTYPE_MIN, /* + and +? */
    REPTYPE_MAX, REPTYPE_MIN, /* ? and ?? */
    REPTYPE_MAX, REPTYPE_MIN, /* OP_CRRANGE and OP_CRMINRANGE */
    REPTYPE_POS, REPTYPE_POS, /* OP_CRPOSSTAR, OP_CRPOSPLUS */
    REPTYPE_POS, REPTYPE_POS, /* OP_CRPOSQUERY, OP_CRPOSRANGE */
];

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

pub(crate) unsafe fn do_callout(
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
        GET((*F).ecode, 1 + 2 * LINK_SIZE) as PCRE2_SIZE
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
    by preserving and restoring their current contents. Picky compilers complain if
    references such as Fovector[-2] are use directly, so we set up a separate
    pointer. */

    callout_ovector = ovec(F).sub(2);

    /* The cb->version, cb->subject, cb->subject_length, and cb->start_match fields
    are set externally. The first 3 never change; the last is updated for each
    bumpalong. */

    cb = (*mb).cb;
    (*cb).capture_top = ((*F).offset_top as u32) / 2 + 1;
    (*cb).capture_last = (*F).capture_last;
    (*cb).offset_vector = callout_ovector;
    (*cb).mark = (*mb).nomatch_mark;
    (*cb).current_position = (*F).eptr.offset_from((*mb).start_subject) as PCRE2_SIZE;
    (*cb).pattern_position = GET((*F).ecode, 1) as PCRE2_SIZE;
    (*cb).next_item_length = GET((*F).ecode, 1 + LINK_SIZE) as PCRE2_SIZE;

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
        (*cb).callout_string_offset = GET((*F).ecode, 1 + 3 * LINK_SIZE) as PCRE2_SIZE;
        (*cb).callout_string = (*F).ecode.add(1 + 4 * LINK_SIZE).add(1);
        (*cb).callout_string_length = (*lengthptr)
            .wrapping_sub(1 + 4 * LINK_SIZE)
            .wrapping_sub(2);
    }

    save0 = *callout_ovector.add(0);
    save1 = *callout_ovector.add(1);
    *callout_ovector.add(1) = PCRE2_UNSET;
    *callout_ovector.add(0) = *callout_ovector.add(1);
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

pub(crate) unsafe fn match_ref(
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

    if offset >= (*F).offset_top || *ovec(F).add(offset) == PCRE2_UNSET {
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
    p = (*mb).start_subject.add(*ovec(F).add(offset));
    length = (*ovec(F).add(offset + 1)).wrapping_sub(*ovec(F).add(offset));

    if caseless != FALSE {
        let utf: BOOL = (((*mb).poptions & PCRE2_UTF) != 0) as BOOL;
        let caseless_restrict: BOOL =
            (((caseopts as u32) & REFI_FLAG_CASELESS_RESTRICT) != 0) as BOOL;
        let turkish_casing: BOOL = ((caseless_restrict == FALSE)
            && ((caseopts as u32) & REFI_FLAG_TURKISH_CASING) != 0) as BOOL;

        if utf != FALSE || ((*mb).poptions & PCRE2_UCP) != 0 {
            let endptr: PCRE2_SPTR = p.add(length);

            /* Match characters up to the end of the reference. NOTE: the number of
            code units matched may differ, because in UTF-8 there are some characters
            whose upper and lower case codes have different numbers of bytes. */

            while p < endptr {
                let mut c: u32;
                let mut d: u32;
                if eptr >= (*mb).end_subject {
                    return 1; /* Partial match */
                }

                if utf != FALSE {
                    /* GETCHARINC(c, eptr) */
                    c = *eptr as u32;
                    eptr = eptr.add(1);
                    if c >= 0xc0 {
                        let r = getutf8inc(c, eptr);
                        c = r.0;
                        eptr = r.1;
                    }
                    /* GETCHARINC(d, p) */
                    d = *p as u32;
                    p = p.add(1);
                    if d >= 0xc0 {
                        let r = getutf8inc(d, p);
                        d = r.0;
                        p = r.1;
                    }
                } else {
                    c = *eptr as u32;
                    eptr = eptr.add(1);
                    d = *p as u32;
                    p = p.add(1);
                }

                if turkish_casing != FALSE && UCD_ANY_I(d) {
                    c = UCD_FOLD_I_TURKISH(c);
                    d = UCD_FOLD_I_TURKISH(d);
                    if c != d {
                        return -1; /* No match */
                    }
                } else if c != d {
                    let ur: &ucd_record = GET_UCD(d);
                    if c != (((d as c_int).wrapping_add(ur.other_case)) as u32) {
                        let mut pp: *const u32 =
                            _pcre2_ucd_caseless_sets_8.as_ptr().add(ur.caseset as usize);

                        /* When PCRE2_EXTRA_CASELESS_RESTRICT is set, ignore any caseless sets
                        that start with an ASCII character. */
                        if caseless_restrict != FALSE && *pp < 128 {
                            return -1; /* No match */
                        }

                        loop {
                            if c < *pp {
                                return -1; /* No match */
                            }
                            let v = *pp;
                            pp = pp.add(1);
                            if c == v {
                                break;
                            }
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
                if TABLE_GET(cp, (*mb).lcc, cp) != TABLE_GET(cc, (*mb).lcc, cc) {
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
                let a = *p;
                p = p.add(1);
                let b = *eptr;
                eptr = eptr.add(1);
                if a != b {
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
                    CU2BYTES(length),
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

pub(crate) unsafe fn recurse_update_offsets(F: *mut heapframe, P: *mut heapframe) {
    let mut dst: *mut PCRE2_SIZE = ovec(F);
    let mut src: *mut PCRE2_SIZE = ovec(P);
    /* The first bracket has offset 2, because
    offset 0 is reserved for the full match. */
    let mut offset: PCRE2_SIZE = 2;
    let offset_top: PCRE2_SIZE = (*F).offset_top.wrapping_add(2);
    let mut diff: PCRE2_SIZE;
    let mut ecode: PCRE2_SPTR = (*F).ecode;

    loop {
        diff = ((GET2(ecode, 1) << 1) as PCRE2_SIZE).wrapping_sub(offset);
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
                diff.wrapping_mul(core::mem::size_of::<PCRE2_SIZE>()),
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
            diff.wrapping_mul(core::mem::size_of::<PCRE2_SIZE>()),
        );
    }

    (*F).ecode = ecode;
    (*F).offset_top = if offset <= (*P).offset_top {
        (*P).offset_top
    } else {
        offset.wrapping_sub(2)
    };
}

/* Expansion of the IS_NEWLINE() macro with NLBLOCK == mb, PSEND ==
end_subject. */

#[inline(always)]
unsafe fn is_newline_at(p: PCRE2_SPTR, mb: *mut match_block, utf: BOOL) -> bool {
    if (*mb).nltype != NLTYPE_FIXED {
        p < (*mb).end_subject
            && crate::newline::_pcre2_is_newline_8(
                p,
                (*mb).nltype,
                (*mb).end_subject,
                core::ptr::addr_of_mut!((*mb).nllen),
                utf,
            ) != FALSE
    } else {
        p <= (*mb).end_subject.wrapping_sub((*mb).nllen as usize)
            && *p as u32 == (*mb).nl[0] as u32
            && ((*mb).nllen == 1 || *p.add(1) as u32 == (*mb).nl[1] as u32)
    }
}

/* Expansion of the WAS_NEWLINE() macro with NLBLOCK == mb, PSSTART ==
start_subject. */

#[inline(always)]
unsafe fn was_newline_at(p: PCRE2_SPTR, mb: *mut match_block, utf: BOOL) -> bool {
    if (*mb).nltype != NLTYPE_FIXED {
        p > (*mb).start_subject
            && crate::newline::_pcre2_was_newline_8(
                p,
                (*mb).nltype,
                (*mb).start_subject,
                core::ptr::addr_of_mut!((*mb).nllen),
                utf,
            ) != FALSE
    } else {
        p >= (*mb).start_subject.wrapping_add((*mb).nllen as usize)
            && *p.wrapping_sub((*mb).nllen as usize) as u32 == (*mb).nl[0] as u32
            && ((*mb).nllen == 1
                || *p.wrapping_sub((*mb).nllen as usize).add(1) as u32 == (*mb).nl[1] as u32)
    }
}

/*************************************************
*           Match a Regular Expression           *
*************************************************/

/* This function applies a compiled pattern to a subject string and picks out
portions of the string if it matches. Two elements in the vector are set for
each substring: the offsets to the start and end of the substring.

Arguments:
  code            points to the compiled expression
  subject         points to the subject string
  length          length of subject string (may contain binary zeros)
  start_offset    where to start in the subject string
  options         option bits
  match_data      points to a match_data block
  mcontext        points a PCRE2 context

Returns:          > 0 => success; value is the number of ovector pairs filled
                  = 0 => success, but ovector is not big enough
                  = -1 => failed to match (PCRE2_ERROR_NOMATCH)
                  = -2 => partial match (PCRE2_ERROR_PARTIAL)
                  < -2 => some kind of unexpected problem
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_match_8(
    code: *const pcre2_real_code,
    subject: PCRE2_SPTR,
    length: PCRE2_SIZE,
    start_offset: PCRE2_SIZE,
    options: u32,
    match_data: *mut pcre2_real_match_data,
    mcontext: *mut pcre2_real_match_context,
) -> c_int {
    let mut subject: PCRE2_SPTR = subject;
    let mut length: PCRE2_SIZE = length;
    let mut options: u32 = options;
    let mut mcontext: *mut pcre2_real_match_context = mcontext;

    let mut rc: c_int = 0;
    let mut start_bits: *const u8 = core::ptr::null();
    let re: *const pcre2_real_code = code;
    let original_options: u32 = options;

    let anchored: BOOL;
    let firstline: BOOL;
    let mut has_first_cu: BOOL = FALSE;
    let mut has_req_cu: BOOL = FALSE;
    let startline: BOOL;

    let mut memchr_found_first_cu: PCRE2_SPTR;
    let mut memchr_found_first_cu2: PCRE2_SPTR;

    let mut first_cu: PCRE2_UCHAR = 0;
    let mut first_cu2: PCRE2_UCHAR = 0;
    let mut req_cu: PCRE2_UCHAR = 0;
    let mut req_cu2: PCRE2_UCHAR = 0;

    let null_str: [PCRE2_UCHAR; 1] = [0xcd];
    let original_subject: PCRE2_SPTR = subject;
    let bumpalong_limit: PCRE2_SPTR;
    let mut end_subject: PCRE2_SPTR;
    let true_end_subject: PCRE2_SPTR;
    let mut start_match: PCRE2_SPTR;
    let mut req_cu_ptr: PCRE2_SPTR;
    let mut start_partial: PCRE2_SPTR = core::ptr::null();
    let mut match_partial: PCRE2_SPTR = core::ptr::null();

    /* This flag is needed even when Unicode is not supported for convenience
    (it is used by the IS_NEWLINE macro). */

    let utf: BOOL;

    let ucp: BOOL;
    let allow_invalid: BOOL;
    let mut fragment_options: u32 = 0;

    let frame_size: PCRE2_SIZE;
    let mut heapframes_size: PCRE2_SIZE;

    /* We need to have mb as a pointer to a match block, because the IS_NEWLINE
    macro is used below, and it expects NLBLOCK to be defined as a pointer. */

    let mut cb: pcre2_callout_block = core::mem::zeroed();
    let mut actual_match_block: match_block = core::mem::zeroed();
    let mb: *mut match_block = &mut actual_match_block;
    let cbp: *mut pcre2_callout_block = &mut cb;

    /* Recognize NULL, length 0 as an empty string. */

    if subject.is_null() && length == 0 {
        subject = null_str.as_ptr();
    }

    /* Plausibility checks */

    if match_data.is_null() {
        return PCRE2_ERROR_NULL;
    }
    if code.is_null() || subject.is_null() {
        (*match_data).rc = PCRE2_ERROR_NULL;
        return (*match_data).rc;
    }
    if (options & !PUBLIC_MATCH_OPTIONS) != 0 {
        (*match_data).rc = PCRE2_ERROR_BADOPTION;
        return (*match_data).rc;
    }

    start_match = subject.wrapping_add(start_offset);
    req_cu_ptr = start_match.wrapping_sub(1);
    if length == PCRE2_ZERO_TERMINATED {
        length = crate::string_utils::_pcre2_strlen_8(subject);
    }
    end_subject = subject.wrapping_add(length);
    true_end_subject = end_subject;

    if start_offset > length {
        (*match_data).rc = PCRE2_ERROR_BADOFFSET;
        return (*match_data).rc;
    }

    /* Check that the first field in the block is the magic number. */

    if (*re).magic_number != MAGIC_NUMBER {
        (*match_data).rc = PCRE2_ERROR_BADMAGIC;
        return (*match_data).rc;
    }

    /* Check the code unit width. */

    if ((*re).flags & PCRE2_MODE_MASK) != 8 / 8
    /* PCRE2_CODE_UNIT_WIDTH/8 */
    {
        (*match_data).rc = PCRE2_ERROR_BADMODE;
        return (*match_data).rc;
    }

    /* PCRE2_NOTEMPTY and PCRE2_NOTEMPTY_ATSTART are match-time flags in the
    options variable for this function. Transfer the corresponding pattern flag
    bits into the options for this function (see the C for the explanation of the
    Boolean trickery). */

    {
        const FF: u32 = PCRE2_NOTEMPTY_SET | PCRE2_NE_ATST_SET;
        const OO: u32 = PCRE2_NOTEMPTY | PCRE2_NOTEMPTY_ATSTART;
        options |= ((*re).flags & FF)
            / ((FF & (!FF).wrapping_add(1)) / (OO & (!OO).wrapping_add(1)));
    }

    /* Initialize UTF/UCP parameters. */

    utf = ((((*re).overall_options & PCRE2_UTF) != 0)) as BOOL;
    allow_invalid = ((((*re).overall_options & PCRE2_MATCH_INVALID_UTF) != 0)) as BOOL;
    ucp = ((((*re).overall_options & PCRE2_UCP) != 0)) as BOOL;

    /* Convert the partial matching flags into an integer. */

    (*mb).partial = if (options & PCRE2_PARTIAL_HARD) != 0 {
        2
    } else if (options & PCRE2_PARTIAL_SOFT) != 0 {
        1
    } else {
        0
    };

    /* Partial matching and PCRE2_ENDANCHORED are currently not allowed at the same
    time. */

    if (*mb).partial != 0 && (((*re).overall_options | options) & PCRE2_ENDANCHORED) != 0 {
        (*match_data).rc = PCRE2_ERROR_BADOPTION;
        return (*match_data).rc;
    }

    /* It is an error to set an offset limit without setting the flag at compile
    time. */

    if !mcontext.is_null()
        && (*mcontext).offset_limit != PCRE2_UNSET
        && ((*re).overall_options & PCRE2_USE_OFFSET_LIMIT) == 0
    {
        (*match_data).rc = PCRE2_ERROR_BADOFFSETLIMIT;
        return (*match_data).rc;
    }

    /* If the match data block was previously used with PCRE2_COPY_MATCHED_SUBJECT,
    free the memory that was obtained. Set the field to NULL for match error
    cases. */

    if ((*match_data).flags & PCRE2_MD_COPIED_SUBJECT) != 0 {
        ((*match_data).memctl.free.unwrap())(
            (*match_data).subject as *mut c_void,
            (*match_data).memctl.memory_data,
        );
        (*match_data).flags &= !PCRE2_MD_COPIED_SUBJECT;
    }
    (*match_data).subject = core::ptr::null();

    /* Zero the error offset in case the first code unit is invalid UTF. */

    (*match_data).startchar = 0;

    /* ============================= JIT matching ==============================
    Omitted: SUPPORT_JIT is not defined in this build.
    ========================= End of JIT matching ========================== */

    /* Proceed with non-JIT matching. The default is to allow lookbehinds to the
    start of the subject. A UTF check when there is a non-zero offset may change
    this. */

    (*mb).check_subject = subject;

    /* If a UTF subject string was not checked for validity, check it here, and
    handle support for invalid UTF strings. Note that support for invalid UTF
    forces a check, overriding the setting of PCRE2_NO_CHECK_UTF. */

    if utf != FALSE && ((options & PCRE2_NO_UTF_CHECK) == 0 || allow_invalid != FALSE) {
        let mut skipped_bad_start: BOOL = FALSE;

        /* For 8-bit and 16-bit UTF, check that the first code unit is a valid
        character start. If we are handling invalid UTF, just skip over such code
        units. Otherwise, give an appropriate error. */

        if allow_invalid != FALSE {
            while start_match < end_subject && NOT_FIRSTCU(*start_match as u32) {
                start_match = start_match.add(1);
                skipped_bad_start = TRUE;
            }
        } else if start_match < end_subject && NOT_FIRSTCU(*start_match as u32) {
            if start_offset > 0 {
                (*match_data).rc = PCRE2_ERROR_BADUTFOFFSET;
                return (*match_data).rc;
            }
            (*match_data).rc = PCRE2_ERROR_UTF8_ERR20; /* Isolated 0x80 byte */
            return (*match_data).rc;
        }

        /* The mb->check_subject field points to the start of UTF checking;
        lookbehinds can go back no further than this. */

        (*mb).check_subject = start_match;

        /* Move back by the maximum lookbehind, just in case it happens at the very
        start of matching, but don't do this if we skipped bad 8-bit code units
        above. */

        if skipped_bad_start == FALSE {
            let mut i: c_uint = (*re).max_lookbehind as c_uint;
            while i > 0 && (*mb).check_subject > subject {
                (*mb).check_subject = (*mb).check_subject.sub(1);
                while (*mb).check_subject > subject && (*(*mb).check_subject & 0xc0) == 0x80 {
                    (*mb).check_subject = (*mb).check_subject.sub(1);
                }
                i -= 1;
            }
        }

        /* Validate the relevant portion of the subject. There's a loop in case we
        encounter bad UTF in the characters preceding start_match which we are
        scanning because of a lookbehind. */

        loop {
            rc = crate::valid_utf::_pcre2_valid_utf_8(
                (*mb).check_subject,
                length.wrapping_sub((*mb).check_subject.offset_from(subject) as PCRE2_SIZE),
                core::ptr::addr_of_mut!((*match_data).startchar),
            );

            if rc == 0 {
                break; /* Valid UTF string */
            }

            /* Invalid UTF string. Adjust the offset to be an absolute offset in the
            whole string. */

            (*match_data).startchar = (*match_data)
                .startchar
                .wrapping_add((*mb).check_subject.offset_from(subject) as PCRE2_SIZE);
            if allow_invalid == FALSE || rc > 0 {
                (*match_data).rc = rc;
                return (*match_data).rc;
            }
            end_subject = subject.add((*match_data).startchar);

            /* If the end precedes start_match, it means there is invalid UTF in the
            extra code units we reversed over because of a lookbehind. */

            if end_subject < start_match {
                (*mb).check_subject = end_subject.add(1);
                while (*mb).check_subject < start_match
                    && NOT_FIRSTCU(*(*mb).check_subject as u32)
                {
                    (*mb).check_subject = (*mb).check_subject.add(1);
                }
                end_subject = true_end_subject;
            }
            /* Otherwise, set the not end of line option, and do the match. */
            else {
                fragment_options = PCRE2_NOTEOL;
                break;
            }
        }
    }

    /* A NULL match context means "use a default context", but we take the memory
    control functions from the pattern. */

    if mcontext.is_null() {
        mcontext = &raw mut crate::context::_pcre2_default_match_context_8;
        (*mb).memctl = (*re).memctl;
    } else {
        (*mb).memctl = (*mcontext).memctl;
    }

    anchored = (((((*re).overall_options | options) & PCRE2_ANCHORED) != 0)) as BOOL;
    firstline =
        ((anchored == FALSE) && ((*re).overall_options & PCRE2_FIRSTLINE) != 0) as BOOL;
    startline = ((((*re).flags & PCRE2_STARTLINE) != 0)) as BOOL;
    bumpalong_limit = if (*mcontext).offset_limit == PCRE2_UNSET {
        true_end_subject
    } else {
        subject.wrapping_add((*mcontext).offset_limit)
    };

    /* Initialize and set up the fixed fields in the callout block, with a pointer
    in the match block. */

    (*mb).cb = cbp;
    (*cbp).version = 2;
    (*cbp).subject = subject;
    (*cbp).subject_length = end_subject.offset_from(subject) as PCRE2_SIZE;
    (*cbp).callout_flags = 0;

    /* Fill in the remaining fields in the match block, except for moptions, which
    gets set later. */

    (*mb).callout = (*mcontext).callout;
    (*mb).callout_data = (*mcontext).callout_data;

    (*mb).start_subject = subject;
    (*mb).start_offset = start_offset;
    (*mb).end_subject = end_subject;
    (*mb).true_end_subject = true_end_subject;
    (*mb).hasthen = ((((*re).flags & PCRE2_HASTHEN) != 0)) as BOOL;
    (*mb).hasbsk = ((((*re).flags & PCRE2_HASBSK) != 0)) as BOOL;
    (*mb).allowemptypartial =
        (((*re).max_lookbehind > 0) || ((*re).flags & PCRE2_MATCH_EMPTY) != 0) as BOOL;
    (*mb).allowlookaroundbsk =
        ((((*re).extra_options & PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK) != 0)) as BOOL;
    (*mb).poptions = (*re).overall_options; /* Pattern options */
    (*mb).ignore_skip_arg = 0;
    (*mb).nomatch_mark = core::ptr::null();
    (*mb).mark = (*mb).nomatch_mark; /* In case never set */

    /* The name table is needed for finding all the numbers associated with a
    given name, for condition testing. The code follows the name table. */

    (*mb).name_table = (re as *const u8).add(core::mem::size_of::<pcre2_real_code>());
    (*mb).name_count = (*re).name_count;
    (*mb).name_entry_size = (*re).name_entry_size;
    (*mb).start_code = (re as *const u8).add((*re).code_start);

    /* Process the \R and newline settings. */

    (*mb).bsr_convention = (*re).bsr_convention;
    (*mb).nltype = NLTYPE_FIXED;
    match (*re).newline_convention as u32 {
        PCRE2_NEWLINE_CR => {
            (*mb).nllen = 1;
            (*mb).nl[0] = CHAR_CR as PCRE2_UCHAR;
        }

        PCRE2_NEWLINE_LF => {
            (*mb).nllen = 1;
            (*mb).nl[0] = CHAR_NL as PCRE2_UCHAR;
        }

        PCRE2_NEWLINE_NUL => {
            (*mb).nllen = 1;
            (*mb).nl[0] = CHAR_NUL as PCRE2_UCHAR;
        }

        PCRE2_NEWLINE_CRLF => {
            (*mb).nllen = 2;
            (*mb).nl[0] = CHAR_CR as PCRE2_UCHAR;
            (*mb).nl[1] = CHAR_NL as PCRE2_UCHAR;
        }

        PCRE2_NEWLINE_ANY => {
            (*mb).nltype = NLTYPE_ANY;
        }

        PCRE2_NEWLINE_ANYCRLF => {
            (*mb).nltype = NLTYPE_ANYCRLF;
        }

        _ => {
            (*match_data).rc = PCRE2_ERROR_INTERNAL;
            return (*match_data).rc;
        }
    }

    /* The backtracking frames have fixed data at the front, and a PCRE2_SIZE
    vector at the end, whose size depends on the number of capturing parentheses in
    the pattern. */

    frame_size = (core::mem::offset_of!(heapframe, ovector)
        + (*re).top_bracket as PCRE2_SIZE * 2 * core::mem::size_of::<PCRE2_SIZE>()
        + HEAPFRAME_ALIGNMENT
        - 1)
        & !(HEAPFRAME_ALIGNMENT - 1);

    /* Limits set in the pattern override the match context only if they are
    smaller. */

    (*mb).heap_limit = if (*mcontext).heap_limit < (*re).limit_heap {
        (*mcontext).heap_limit
    } else {
        (*re).limit_heap
    };

    (*mb).match_limit = if (*mcontext).match_limit < (*re).limit_match {
        (*mcontext).match_limit
    } else {
        (*re).limit_match
    };

    (*mb).match_limit_depth = if (*mcontext).depth_limit < (*re).limit_depth {
        (*mcontext).depth_limit
    } else {
        (*re).limit_depth
    };

    /* If a pattern has very many capturing parentheses, the frame size may be very
    large. Set the initial frame vector size to ensure that there are at least 10
    available frames, but enforce a minimum of START_FRAMES_SIZE. */

    heapframes_size = frame_size * 10;
    if heapframes_size < START_FRAMES_SIZE {
        heapframes_size = START_FRAMES_SIZE;
    }
    if heapframes_size / 1024 > (*mb).heap_limit as PCRE2_SIZE {
        let max_size: PCRE2_SIZE = 1024 * (*mb).heap_limit as PCRE2_SIZE;
        if max_size < frame_size {
            (*match_data).rc = PCRE2_ERROR_HEAPLIMIT;
            return (*match_data).rc;
        }
        heapframes_size = max_size;
    }

    /* If an existing frame vector in the match_data block is large enough, we can
    use it. Otherwise, free any pre-existing vector and get a new one. */

    if (*match_data).heapframes_size < heapframes_size {
        ((*match_data).memctl.free.unwrap())(
            (*match_data).heapframes as *mut c_void,
            (*match_data).memctl.memory_data,
        );
        (*match_data).heapframes = ((*match_data).memctl.malloc.unwrap())(
            heapframes_size,
            (*match_data).memctl.memory_data,
        ) as *mut heapframe;
        if (*match_data).heapframes.is_null() {
            (*match_data).heapframes_size = 0;
            (*match_data).rc = PCRE2_ERROR_NOMEMORY;
            return (*match_data).rc;
        }
        (*match_data).heapframes_size = heapframes_size;
    }

    /* Write to the ovector within the first frame to mark every capture unset and
    to avoid uninitialized memory read errors when it is copied to a new frame. */

    memset(
        ((*match_data).heapframes as *mut u8).add(core::mem::offset_of!(heapframe, ovector))
            as *mut c_void,
        0xff,
        frame_size - core::mem::offset_of!(heapframe, ovector),
    );

    /* Pointers to the individual character tables */

    (*mb).lcc = (*re).tables.add(lcc_offset);
    (*mb).fcc = (*re).tables.add(fcc_offset);
    (*mb).ctypes = (*re).tables.add(ctypes_offset);

    /* Set up the first code unit to match, if available. If there's no first code
    unit there may be a bitmap of possible first characters. */

    if ((*re).flags & PCRE2_FIRSTSET) != 0 {
        has_first_cu = TRUE;
        first_cu = (*re).first_codeunit as PCRE2_UCHAR;
        first_cu2 = first_cu;
        if ((*re).flags & PCRE2_FIRSTCASELESS) != 0 {
            first_cu2 = TABLE_GET(first_cu as u32, (*mb).fcc, first_cu as u32) as PCRE2_UCHAR;
            if first_cu as u32 > 127 && ucp != FALSE && utf == FALSE {
                first_cu2 = UCD_OTHERCASE(first_cu as u32) as PCRE2_UCHAR;
            }
        }
    } else if startline == FALSE && ((*re).flags & PCRE2_FIRSTMAPSET) != 0 {
        start_bits = core::ptr::addr_of!((*re).start_bitmap) as *const u8;
    }

    /* There may also be a "last known required character" set. */

    if ((*re).flags & PCRE2_LASTSET) != 0 {
        has_req_cu = TRUE;
        req_cu = (*re).last_codeunit as PCRE2_UCHAR;
        req_cu2 = req_cu;
        if ((*re).flags & PCRE2_LASTCASELESS) != 0 {
            req_cu2 = TABLE_GET(req_cu as u32, (*mb).fcc, req_cu as u32) as PCRE2_UCHAR;
            if req_cu as u32 > 127 && ucp != FALSE && utf == FALSE {
                req_cu2 = UCD_OTHERCASE(req_cu as u32) as PCRE2_UCHAR;
            }
        }
    }

    /* ==========================================================================*/

    /* Loop for handling unanchored repeated matching attempts; for anchored regexs
    the loop runs just once. */

    'FRAGMENT_RESTART: loop {
        start_partial = core::ptr::null();
        match_partial = core::ptr::null();
        (*mb).hitend = FALSE;

        memchr_found_first_cu = core::ptr::null();
        memchr_found_first_cu2 = core::ptr::null();

        'ENDLOOP: {
            'bumpalong: loop {
                let mut new_start_match: PCRE2_SPTR = core::ptr::null();

                /* ----------------- Start of match optimizations ---------------- */

                /* There are some optimizations that avoid running the match if a known
                starting point is not found, or if a known later code unit is not
                present. However, there is an option (settable at compile time) that
                disables these, for testing and for ensuring that all callouts do
                actually occur. */

                if ((*re).optimization_flags & PCRE2_OPTIM_START_OPTIMIZE) != 0 {
                    /* If firstline is TRUE, the start of the match is constrained to the
                    first line of a multiline string. */

                    if firstline != FALSE {
                        let mut t: PCRE2_SPTR = start_match;
                        if utf != FALSE {
                            while t < end_subject && !is_newline_at(t, mb, utf) {
                                t = t.add(1);
                                /* ACROSSCHAR(t < end_subject, t, t++) */
                                while t < end_subject && (*t & 0xc0) == 0x80 {
                                    t = t.add(1);
                                }
                            }
                        } else {
                            while t < end_subject && !is_newline_at(t, mb, utf) {
                                t = t.add(1);
                            }
                        }
                        end_subject = t;
                    }

                    /* Anchored: check the first code unit if one is recorded. */

                    if anchored != FALSE {
                        if has_first_cu != FALSE || !start_bits.is_null() {
                            let mut ok: BOOL = (start_match < end_subject) as BOOL;
                            if ok != FALSE {
                                let c: PCRE2_UCHAR = *start_match;
                                ok = (has_first_cu != FALSE
                                    && (c == first_cu || c == first_cu2))
                                    as BOOL;
                                if ok == FALSE && !start_bits.is_null() {
                                    ok = ((*start_bits.add((c / 8) as usize) as u32
                                        & (1u32 << ((c & 7) as u32)))
                                        != 0) as BOOL;
                                }
                            }
                            if ok == FALSE {
                                rc = MATCH_NOMATCH;
                                break 'bumpalong;
                            }
                        }
                    }
                    /* Not anchored. Advance to a unique first code unit if there is one. */
                    else {
                        if has_first_cu != FALSE {
                            if first_cu != first_cu2
                            /* Caseless */
                            {
                                /* In 8-bit mode, the use of memchr() gives a big speed up,
                                even though we have to call it twice in order to find the
                                earliest occurrence of the code unit in either of its cases.
                                Caching is used to remember the positions of previously found
                                code units. */

                                let mut pp1: PCRE2_SPTR;
                                let mut pp2: PCRE2_SPTR;
                                let searchlength: PCRE2_SIZE =
                                    end_subject.offset_from(start_match) as PCRE2_SIZE;

                                /* If we haven't got a previously found position for first_cu,
                                or if the current starting position is later, we need to do a
                                search. If the code unit is not found, set it to the end. */

                                if memchr_found_first_cu.is_null()
                                    || start_match > memchr_found_first_cu
                                {
                                    pp1 = memchr(
                                        start_match as *const c_void,
                                        first_cu as c_int,
                                        searchlength,
                                    ) as PCRE2_SPTR;
                                    memchr_found_first_cu =
                                        if pp1.is_null() { end_subject } else { pp1 };
                                }
                                /* If the start is before a previously found position, use the
                                previous position, or NULL if a previous search failed. */
                                else {
                                    pp1 = if memchr_found_first_cu == end_subject {
                                        core::ptr::null()
                                    } else {
                                        memchr_found_first_cu
                                    };
                                }

                                /* Do the same thing for the other case. */

                                if memchr_found_first_cu2.is_null()
                                    || start_match > memchr_found_first_cu2
                                {
                                    pp2 = memchr(
                                        start_match as *const c_void,
                                        first_cu2 as c_int,
                                        searchlength,
                                    ) as PCRE2_SPTR;
                                    memchr_found_first_cu2 =
                                        if pp2.is_null() { end_subject } else { pp2 };
                                } else {
                                    pp2 = if memchr_found_first_cu2 == end_subject {
                                        core::ptr::null()
                                    } else {
                                        memchr_found_first_cu2
                                    };
                                }

                                /* Set the start to the end of the subject if neither case was
                                found. Otherwise, use the earlier found point. */

                                if pp1.is_null() {
                                    start_match =
                                        if pp2.is_null() { end_subject } else { pp2 };
                                } else {
                                    start_match = if pp2.is_null() || pp1 < pp2 { pp1 } else { pp2 };
                                }
                            }
                            /* The caseful case is much simpler. */
                            else {
                                start_match = memchr(
                                    start_match as *const c_void,
                                    first_cu as c_int,
                                    end_subject.offset_from(start_match) as usize,
                                ) as PCRE2_SPTR;
                                if start_match.is_null() {
                                    start_match = end_subject;
                                }
                            }

                            /* If we can't find the required first code unit, having reached
                            the true end of the subject, break the bumpalong loop, to force a
                            match failure, except when doing partial matching. */

                            if (*mb).partial == 0 && start_match >= (*mb).end_subject {
                                rc = MATCH_NOMATCH;
                                break 'bumpalong;
                            }
                        }
                        /* If there's no first code unit, advance to just after a linebreak
                        for a multiline match if required. */
                        else if startline != FALSE {
                            if start_match > (*mb).start_subject.add(start_offset) {
                                if utf != FALSE {
                                    while start_match < end_subject
                                        && !was_newline_at(start_match, mb, utf)
                                    {
                                        start_match = start_match.add(1);
                                        /* ACROSSCHAR(start_match < end_subject, start_match,
                                        start_match++) */
                                        while start_match < end_subject
                                            && (*start_match & 0xc0) == 0x80
                                        {
                                            start_match = start_match.add(1);
                                        }
                                    }
                                } else {
                                    while start_match < end_subject
                                        && !was_newline_at(start_match, mb, utf)
                                    {
                                        start_match = start_match.add(1);
                                    }
                                }

                                /* If we have just passed a CR and the newline option is ANY
                                or ANYCRLF, and we are now at a LF, advance the match position
                                by one more code unit. */

                                if *start_match.offset(-1) as u32 == CHAR_CR
                                    && ((*mb).nltype == NLTYPE_ANY
                                        || (*mb).nltype == NLTYPE_ANYCRLF)
                                    && start_match < end_subject
                                    && *start_match as u32 == CHAR_NL
                                {
                                    start_match = start_match.add(1);
                                }
                            }
                        }
                        /* If there's no first code unit or a requirement for a multiline
                        line start, advance to a non-unique first code unit if any have been
                        identified. */
                        else if !start_bits.is_null() {
                            while start_match < end_subject {
                                let c: u32 = *start_match as u32;
                                if (*start_bits.add((c / 8) as usize) as u32
                                    & (1u32 << (c & 7)))
                                    != 0
                                {
                                    break;
                                }
                                start_match = start_match.add(1);
                            }

                            /* See comment above in first_cu checking about the next few
                            lines. */

                            if (*mb).partial == 0 && start_match >= (*mb).end_subject {
                                rc = MATCH_NOMATCH;
                                break 'bumpalong;
                            }
                        }
                    } /* End first code unit handling */

                    /* Restore fudged end_subject */

                    end_subject = (*mb).end_subject;

                    /* The following two optimizations must be disabled for partial
                    matching. */

                    if (*mb).partial == 0 {
                        let mut p: PCRE2_SPTR;

                        /* The minimum matching length is a lower bound; no string of that
                        length may actually match the pattern. */

                        if (end_subject.offset_from(start_match) as isize)
                            < (*re).minlength as isize
                        {
                            rc = MATCH_NOMATCH;
                            break 'bumpalong;
                        }

                        /* If req_cu is set, we know that that code unit must appear in the
                        subject for the (non-partial) match to succeed. */

                        p = start_match.add(if has_first_cu != FALSE { 1 } else { 0 });
                        if has_req_cu != FALSE && p > req_cu_ptr {
                            let check_length: PCRE2_SIZE =
                                end_subject.offset_from(start_match) as PCRE2_SIZE;

                            if check_length < REQ_CU_MAX
                                || (anchored == FALSE && check_length < REQ_CU_MAX * 1000)
                            {
                                if req_cu != req_cu2
                                /* Caseless */
                                {
                                    let pp: PCRE2_SPTR = p;
                                    p = memchr(
                                        pp as *const c_void,
                                        req_cu as c_int,
                                        end_subject.offset_from(pp) as usize,
                                    ) as PCRE2_SPTR;
                                    if p.is_null() {
                                        p = memchr(
                                            pp as *const c_void,
                                            req_cu2 as c_int,
                                            end_subject.offset_from(pp) as usize,
                                        ) as PCRE2_SPTR;
                                        if p.is_null() {
                                            p = end_subject;
                                        }
                                    }
                                }
                                /* The caseful case */
                                else {
                                    p = memchr(
                                        p as *const c_void,
                                        req_cu as c_int,
                                        end_subject.offset_from(p) as usize,
                                    ) as PCRE2_SPTR;
                                    if p.is_null() {
                                        p = end_subject;
                                    }
                                }

                                /* If we can't find the required code unit, break the
                                bumpalong loop, forcing a match failure. */

                                if p >= end_subject {
                                    rc = MATCH_NOMATCH;
                                    break 'bumpalong;
                                }

                                /* If we have found the required code unit, save the point
                                where we found it. */

                                req_cu_ptr = p;
                            }
                        }
                    }
                }

                /* ------------ End of start of match optimizations ------------ */

                /* Give no match if we have passed the bumpalong limit. */

                if start_match > bumpalong_limit {
                    rc = MATCH_NOMATCH;
                    break 'bumpalong;
                }

                /* OK, we can now run the match. If "hitend" is set afterwards, remember
                the first starting point for which a partial match was found. */

                (*cbp).start_match = start_match.offset_from(subject) as PCRE2_SIZE;
                (*cbp).callout_flags |= PCRE2_CALLOUT_STARTMATCH;

                (*mb).start_used_ptr = start_match;
                (*mb).last_used_ptr = start_match;
                (*mb).moptions = options | fragment_options;
                (*mb).match_call_count = 0;
                (*mb).end_offset_top = 0;
                (*mb).skip_arg_count = 0;

                rc = crate::matcher_core::match_(
                    start_match,
                    (*mb).start_code,
                    (*re).top_bracket,
                    frame_size,
                    match_data,
                    mb,
                );

                if (*mb).hitend != FALSE && start_partial.is_null() {
                    start_partial = (*mb).start_used_ptr;
                    match_partial = start_match;
                }

                /* switch(rc) */
                'SWRC: {
                    /* If MATCH_SKIP_ARG reaches this level it means that a MARK that
                    matched the SKIP's arg was not found. */

                    if rc == MATCH_SKIP_ARG {
                        new_start_match = start_match;
                        (*mb).ignore_skip_arg = (*mb).skip_arg_count;
                        break 'SWRC;
                    }

                    /* SKIP passes back the next starting point explicitly, but if it is
                    no greater than the match we have just done, treat it as NOMATCH. */

                    if rc == MATCH_SKIP {
                        if (*mb).verb_skip_ptr > start_match {
                            new_start_match = (*mb).verb_skip_ptr;
                            break 'SWRC;
                        }
                        /* Fall through */
                    }

                    /* NOMATCH and PRUNE advance by one character. THEN at this level acts
                    exactly like PRUNE. Unset ignore SKIP-with-argument. */

                    if rc == MATCH_SKIP
                        || rc == MATCH_NOMATCH
                        || rc == MATCH_PRUNE
                        || rc == MATCH_THEN
                    {
                        (*mb).ignore_skip_arg = 0;
                        new_start_match = start_match.add(1);
                        if utf != FALSE {
                            /* ACROSSCHAR(new_start_match < end_subject, new_start_match,
                            new_start_match++) */
                            while new_start_match < end_subject
                                && (*new_start_match & 0xc0) == 0x80
                            {
                                new_start_match = new_start_match.add(1);
                            }
                        }
                        break 'SWRC;
                    }

                    /* COMMIT disables the bumpalong, but otherwise behaves as NOMATCH. */

                    if rc == MATCH_COMMIT {
                        rc = MATCH_NOMATCH;
                        break 'ENDLOOP;
                    }

                    /* Any other return is either a match, or some kind of error. */

                    break 'ENDLOOP;
                }

                /* Control reaches here for the various types of "no match at this point"
                result. Reset the code to MATCH_NOMATCH for subsequent checking. */

                rc = MATCH_NOMATCH;

                /* If PCRE2_FIRSTLINE is set, the match must happen before or at the
                first newline in the subject. */

                if firstline != FALSE && is_newline_at(start_match, mb, utf) {
                    break 'bumpalong;
                }

                /* Advance to new matching position */

                start_match = new_start_match;

                /* Break the loop if the pattern is anchored or if we have passed the end
                of the subject. */

                if anchored != FALSE || start_match > end_subject {
                    break 'bumpalong;
                }

                /* If we have just passed a CR and we are now at a LF, and the pattern
                does not contain any explicit matches for \r or \n, and the newline
                option is CRLF or ANY or ANYCRLF, advance the match position by one more
                code unit. */

                if start_match > subject.add(start_offset)
                    && *start_match.offset(-1) as u32 == CHAR_CR
                    && start_match < end_subject
                    && *start_match as u32 == CHAR_NL
                    && ((*re).flags & PCRE2_HASCRORLF) == 0
                    && ((*mb).nltype == NLTYPE_ANY
                        || (*mb).nltype == NLTYPE_ANYCRLF
                        || (*mb).nllen == 2)
                {
                    start_match = start_match.add(1);
                }

                (*mb).mark = core::ptr::null(); /* Reset for start of next match attempt */
            } /* End of for(;;) "bumpalong" loop */
        } /* ENDLOOP: */

        /* ==========================================================================*/

        /* If end_subject != true_end_subject, it means we are handling invalid UTF,
        and have just processed a non-terminal fragment. If this resulted in no match
        or a partial match we must carry on to the next fragment. */

        if utf != FALSE
            && end_subject != true_end_subject
            && (rc == MATCH_NOMATCH || rc == PCRE2_ERROR_PARTIAL)
        {
            loop {
                /* Advance past the first bad code unit, and then skip invalid character
                starting code units in 8-bit mode. */

                start_match = end_subject.add(1);

                while start_match < true_end_subject && NOT_FIRSTCU(*start_match as u32) {
                    start_match = start_match.add(1);
                }

                /* If we have hit the end of the subject, there isn't another non-empty
                fragment, so give up. */

                if start_match >= true_end_subject {
                    rc = MATCH_NOMATCH; /* In case it was partial */
                    match_partial = core::ptr::null();
                    break;
                }

                /* Check the rest of the subject */

                (*mb).check_subject = start_match;
                rc = crate::valid_utf::_pcre2_valid_utf_8(
                    start_match,
                    length.wrapping_sub(start_match.offset_from(subject) as PCRE2_SIZE),
                    core::ptr::addr_of_mut!((*match_data).startchar),
                );

                /* The rest of the subject is valid UTF. */

                if rc == 0 {
                    end_subject = true_end_subject;
                    (*mb).end_subject = end_subject;
                    fragment_options = PCRE2_NOTBOL;
                    continue 'FRAGMENT_RESTART;
                }
                /* A subsequent UTF error has been found; if the next fragment is
                non-empty, set up to process it. Otherwise, let the loop advance. */
                else if rc < 0 {
                    end_subject = start_match.add((*match_data).startchar);
                    (*mb).end_subject = end_subject;
                    if end_subject > start_match {
                        fragment_options = PCRE2_NOTBOL | PCRE2_NOTEOL;
                        continue 'FRAGMENT_RESTART;
                    }
                }
            }
        }

        break;
    } /* End of 'FRAGMENT_RESTART loop */

    /* Fill in fields that are always returned in the match data. */

    (*match_data).code = re;
    (*match_data).mark = (*mb).mark;
    (*match_data).matchedby = PCRE2_MATCHEDBY_INTERPRETER as u8;
    (*match_data).options = original_options;

    /* Handle a fully successful match. */

    if rc == MATCH_MATCH {
        (*match_data).rc =
            if ((*mb).end_offset_top as c_int) >= 2 * ((*match_data).oveccount as c_int) {
                0
            } else {
                ((*mb).end_offset_top as c_int) / 2 + 1
            };
        (*match_data).subject_length = length;
        (*match_data).start_offset = start_offset;
        (*match_data).startchar = start_match.offset_from(subject) as PCRE2_SIZE;
        (*match_data).leftchar = (*mb).start_used_ptr.offset_from(subject) as PCRE2_SIZE;
        (*match_data).rightchar = (if (*mb).last_used_ptr > (*mb).end_match_ptr {
            (*mb).last_used_ptr
        } else {
            (*mb).end_match_ptr
        })
        .offset_from(subject) as PCRE2_SIZE;
        if (options & PCRE2_COPY_MATCHED_SUBJECT) != 0 {
            if length != 0 {
                (*match_data).subject = ((*match_data).memctl.malloc.unwrap())(
                    CU2BYTES(length),
                    (*match_data).memctl.memory_data,
                ) as PCRE2_SPTR;
                if (*match_data).subject.is_null() {
                    (*match_data).rc = PCRE2_ERROR_NOMEMORY;
                    return (*match_data).rc;
                }
                memcpy(
                    (*match_data).subject as *mut c_void,
                    subject as *const c_void,
                    CU2BYTES(length),
                );
            } else {
                (*match_data).subject = core::ptr::null();
            }
            (*match_data).flags |= PCRE2_MD_COPIED_SUBJECT;
        } else {
            (*match_data).subject = original_subject;
        }

        return (*match_data).rc;
    }

    /* Control gets here if there has been a partial match, an error, or if the
    overall match attempt has failed at all permitted starting positions. */

    (*match_data).mark = (*mb).nomatch_mark;

    /* For anything other than nomatch or partial match, just return the code. */

    if rc != MATCH_NOMATCH && rc != PCRE2_ERROR_PARTIAL {
        (*match_data).rc = rc;
    }
    /* Handle a partial match. */
    else if !match_partial.is_null() {
        let md_ovector: *mut PCRE2_SIZE =
            core::ptr::addr_of_mut!((*match_data).ovector) as *mut PCRE2_SIZE;
        (*match_data).subject = original_subject;
        (*match_data).subject_length = length;
        (*match_data).start_offset = start_offset;
        *md_ovector.add(0) = match_partial.offset_from(subject) as PCRE2_SIZE;
        *md_ovector.add(1) = end_subject.offset_from(subject) as PCRE2_SIZE;
        (*match_data).startchar = match_partial.offset_from(subject) as PCRE2_SIZE;
        (*match_data).leftchar = start_partial.offset_from(subject) as PCRE2_SIZE;
        (*match_data).rightchar = end_subject.offset_from(subject) as PCRE2_SIZE;
        (*match_data).rc = PCRE2_ERROR_PARTIAL;
    }
    /* Else this is the classic nomatch case. */
    else {
        (*match_data).subject = original_subject;
        (*match_data).subject_length = length;
        (*match_data).start_offset = start_offset;
        (*match_data).rc = PCRE2_ERROR_NOMATCH;
    }

    (*match_data).rc
}
