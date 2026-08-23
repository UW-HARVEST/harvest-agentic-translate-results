// Translated from c_src/src/pcre2_substring.c
use crate::internal::*;

/* PCRE2_CODE_UNIT_WIDTH is 8 for this build. Note that pcre2_substring_get_bynumber()
below multiplies by PCRE2_CODE_UNIT_WIDTH (not by the size of a code unit), exactly
as the C code does. */
const PCRE2_CODE_UNIT_WIDTH: usize = 8;

/*************************************************
*   Copy named captured string to given buffer   *
*************************************************/

/* This function copies a single captured substring into a given buffer,
identifying it by name. If the regex permits duplicate names, the first
substring that is set is chosen.

Arguments:
  match_data     points to the match data
  stringname     the name of the required substring
  buffer         where to put the substring
  sizeptr        the size of the buffer, updated to the size of the substring

Returns:         if successful: zero
                 if not successful, a negative error code:
                   (1) an error from nametable_scan()
                   (2) an error from copy_bynumber()
                   (3) PCRE2_ERROR_UNAVAILABLE: no group is in ovector
                   (4) PCRE2_ERROR_UNSET: all named groups in ovector are unset
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_copy_byname_8(
    match_data: *mut pcre2_real_match_data,
    stringname: PCRE2_SPTR,
    buffer: *mut PCRE2_UCHAR,
    sizeptr: *mut PCRE2_SIZE,
) -> c_int {
    let mut first: PCRE2_SPTR = std::ptr::null();
    let mut last: PCRE2_SPTR = std::ptr::null();
    let mut entry: PCRE2_SPTR;
    let mut failrc: c_int;
    let entrysize: c_int;
    if (*match_data).matchedby as u32 == PCRE2_MATCHEDBY_DFA_INTERPRETER {
        return PCRE2_ERROR_DFA_UFUNC;
    }
    entrysize = pcre2_substring_nametable_scan_8(
        (*match_data).code,
        stringname,
        &mut first,
        &mut last,
    );
    if entrysize < 0 {
        return entrysize;
    }
    failrc = PCRE2_ERROR_UNAVAILABLE;
    entry = first;
    while entry <= last {
        let n: u32 = GET2!(entry, 0);
        if n < (*match_data).oveccount as u32 {
            if *(*match_data).ovector.as_ptr().add((n as usize) * 2) != PCRE2_UNSET {
                return pcre2_substring_copy_bynumber_8(match_data, n, buffer, sizeptr);
            }
            failrc = PCRE2_ERROR_UNSET;
        }
        entry = entry.add(entrysize as usize);
    }
    failrc
}

/*************************************************
*  Copy numbered captured string to given buffer *
*************************************************/

/* This function copies a single captured substring into a given buffer,
identifying it by number.

Arguments:
  match_data     points to the match data
  stringnumber   the number of the required substring
  buffer         where to put the substring
  sizeptr        the size of the buffer, updated to the size of the substring

Returns:         if successful: 0
                 if not successful, a negative error code:
                   PCRE2_ERROR_NOMEMORY: buffer too small
                   PCRE2_ERROR_NOSUBSTRING: no such substring
                   PCRE2_ERROR_UNAVAILABLE: ovector too small
                   PCRE2_ERROR_UNSET: substring is not set
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_copy_bynumber_8(
    match_data: *mut pcre2_real_match_data,
    stringnumber: u32,
    buffer: *mut PCRE2_UCHAR,
    sizeptr: *mut PCRE2_SIZE,
) -> c_int {
    let rc: c_int;
    let mut size: PCRE2_SIZE = 0;
    rc = pcre2_substring_length_bynumber_8(match_data, stringnumber, &mut size);
    if rc < 0 {
        return rc;
    }
    if size.wrapping_add(1) > *sizeptr {
        return PCRE2_ERROR_NOMEMORY;
    }
    if size != 0 {
        memcpy(
            buffer as *mut c_void,
            (*match_data)
                .subject
                .add(*(*match_data).ovector.as_ptr().add((stringnumber as usize) * 2))
                as *const c_void,
            CU2BYTES!(size),
        );
    }
    *buffer.add(size) = 0;
    *sizeptr = size;
    0
}

/*************************************************
*          Extract named captured string         *
*************************************************/

/* This function copies a single captured substring, identified by name, into
new memory. If the regex permits duplicate names, the first substring that is
set is chosen.

Arguments:
  match_data     pointer to match_data
  stringname     the name of the required substring
  stringptr      where to put the pointer to the new memory
  sizeptr        where to put the length of the substring

Returns:         if successful: zero
                 if not successful, a negative value:
                   (1) an error from nametable_scan()
                   (2) an error from get_bynumber()
                   (3) PCRE2_ERROR_UNAVAILABLE: no group is in ovector
                   (4) PCRE2_ERROR_UNSET: all named groups in ovector are unset
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_get_byname_8(
    match_data: *mut pcre2_real_match_data,
    stringname: PCRE2_SPTR,
    stringptr: *mut *mut PCRE2_UCHAR,
    sizeptr: *mut PCRE2_SIZE,
) -> c_int {
    let mut first: PCRE2_SPTR = std::ptr::null();
    let mut last: PCRE2_SPTR = std::ptr::null();
    let mut entry: PCRE2_SPTR;
    let mut failrc: c_int;
    let entrysize: c_int;
    if (*match_data).matchedby as u32 == PCRE2_MATCHEDBY_DFA_INTERPRETER {
        return PCRE2_ERROR_DFA_UFUNC;
    }
    entrysize = pcre2_substring_nametable_scan_8(
        (*match_data).code,
        stringname,
        &mut first,
        &mut last,
    );
    if entrysize < 0 {
        return entrysize;
    }
    failrc = PCRE2_ERROR_UNAVAILABLE;
    entry = first;
    while entry <= last {
        let n: u32 = GET2!(entry, 0);
        if n < (*match_data).oveccount as u32 {
            if *(*match_data).ovector.as_ptr().add((n as usize) * 2) != PCRE2_UNSET {
                return pcre2_substring_get_bynumber_8(match_data, n, stringptr, sizeptr);
            }
            failrc = PCRE2_ERROR_UNSET;
        }
        entry = entry.add(entrysize as usize);
    }
    failrc
}

/*************************************************
*      Extract captured string to new memory     *
*************************************************/

/* This function copies a single captured substring into a piece of new
memory.

Arguments:
  match_data     points to match data
  stringnumber   the number of the required substring
  stringptr      where to put a pointer to the new memory
  sizeptr        where to put the size of the substring

Returns:         if successful: 0
                 if not successful, a negative error code:
                   PCRE2_ERROR_NOMEMORY: failed to get memory
                   PCRE2_ERROR_NOSUBSTRING: no such substring
                   PCRE2_ERROR_UNAVAILABLE: ovector too small
                   PCRE2_ERROR_UNSET: substring is not set
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_get_bynumber_8(
    match_data: *mut pcre2_real_match_data,
    stringnumber: u32,
    stringptr: *mut *mut PCRE2_UCHAR,
    sizeptr: *mut PCRE2_SIZE,
) -> c_int {
    let rc: c_int;
    let mut size: PCRE2_SIZE = 0;
    let mut yield_: *mut PCRE2_UCHAR;
    rc = pcre2_substring_length_bynumber_8(match_data, stringnumber, &mut size);
    if rc < 0 {
        return rc;
    }
    yield_ = _pcre2_memctl_malloc_8(
        size_of::<pcre2_memctl>() + size.wrapping_add(1) * PCRE2_CODE_UNIT_WIDTH,
        match_data as *mut pcre2_memctl,
    ) as *mut PCRE2_UCHAR;
    if yield_.is_null() {
        return PCRE2_ERROR_NOMEMORY;
    }
    yield_ = (yield_ as *mut c_char).add(size_of::<pcre2_memctl>()) as *mut PCRE2_UCHAR;
    if size != 0 {
        memcpy(
            yield_ as *mut c_void,
            (*match_data)
                .subject
                .add(*(*match_data).ovector.as_ptr().add((stringnumber as usize) * 2))
                as *const c_void,
            CU2BYTES!(size),
        );
    }
    *yield_.add(size) = 0;
    *stringptr = yield_;
    *sizeptr = size;
    0
}

/*************************************************
*       Free memory obtained by get_substring    *
*************************************************/

/*
Argument:     the result of a previous pcre2_substring_get_byxxx()
Returns:      nothing
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_free_8(string: *mut PCRE2_UCHAR) {
    if !string.is_null() {
        let memctl: *mut pcre2_memctl =
            (string as *mut c_char).sub(size_of::<pcre2_memctl>()) as *mut pcre2_memctl;
        ((*memctl).free.unwrap())(memctl as *mut c_void, (*memctl).memory_data);
    }
}

/*************************************************
*         Get length of a named substring        *
*************************************************/

/* This function returns the length of a named captured substring. If the regex
permits duplicate names, the first substring that is set is chosen.

Arguments:
  match_data      pointer to match data
  stringname      the name of the required substring
  sizeptr         where to put the length, if not NULL

Returns:          0 if successful, else a negative error number
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_length_byname_8(
    match_data: *mut pcre2_real_match_data,
    stringname: PCRE2_SPTR,
    sizeptr: *mut PCRE2_SIZE,
) -> c_int {
    let mut first: PCRE2_SPTR = std::ptr::null();
    let mut last: PCRE2_SPTR = std::ptr::null();
    let mut entry: PCRE2_SPTR;
    let mut failrc: c_int;
    let entrysize: c_int;
    if (*match_data).matchedby as u32 == PCRE2_MATCHEDBY_DFA_INTERPRETER {
        return PCRE2_ERROR_DFA_UFUNC;
    }
    entrysize = pcre2_substring_nametable_scan_8(
        (*match_data).code,
        stringname,
        &mut first,
        &mut last,
    );
    if entrysize < 0 {
        return entrysize;
    }
    failrc = PCRE2_ERROR_UNAVAILABLE;
    entry = first;
    while entry <= last {
        let n: u32 = GET2!(entry, 0);
        if n < (*match_data).oveccount as u32 {
            if *(*match_data).ovector.as_ptr().add((n as usize) * 2) != PCRE2_UNSET {
                return pcre2_substring_length_bynumber_8(match_data, n, sizeptr);
            }
            failrc = PCRE2_ERROR_UNSET;
        }
        entry = entry.add(entrysize as usize);
    }
    failrc
}

/*************************************************
*        Get length of a numbered substring      *
*************************************************/

/* This function returns the length of a captured substring. If the start is
beyond the end (which can happen when \K is used in an assertion), it sets the
length to zero.

Arguments:
  match_data      pointer to match data
  stringnumber    the number of the required substring
  sizeptr         where to put the length, if not NULL

Returns:         if successful: 0
                 if not successful, a negative error code:
                   PCRE2_ERROR_NOSUBSTRING: no such substring
                   PCRE2_ERROR_UNAVAILABLE: ovector is too small
                   PCRE2_ERROR_UNSET: substring is not set
                   PCRE2_ERROR_INVALIDOFFSET: internal error, should not occur
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_length_bynumber_8(
    match_data: *mut pcre2_real_match_data,
    stringnumber: u32,
    sizeptr: *mut PCRE2_SIZE,
) -> c_int {
    let left: PCRE2_SIZE;
    let right: PCRE2_SIZE;
    let mut count: c_int = (*match_data).rc;
    if count == PCRE2_ERROR_PARTIAL {
        if stringnumber > 0 {
            return PCRE2_ERROR_PARTIAL;
        }
        count = 0;
    } else if count < 0 {
        return count; /* Match failed */
    }

    if (*match_data).matchedby as u32 != PCRE2_MATCHEDBY_DFA_INTERPRETER {
        if stringnumber > (*(*match_data).code).top_bracket as u32 {
            return PCRE2_ERROR_NOSUBSTRING;
        }
        if stringnumber >= (*match_data).oveccount as u32 {
            return PCRE2_ERROR_UNAVAILABLE;
        }
        if *(*match_data)
            .ovector
            .as_ptr()
            .add((stringnumber as usize) * 2)
            == PCRE2_UNSET
        {
            return PCRE2_ERROR_UNSET;
        }
    } else
    /* Matched using pcre2_dfa_match() */
    {
        if stringnumber >= (*match_data).oveccount as u32 {
            return PCRE2_ERROR_UNAVAILABLE;
        }
        if count != 0 && stringnumber >= count as u32 {
            return PCRE2_ERROR_UNSET;
        }
    }

    left = *(*match_data)
        .ovector
        .as_ptr()
        .add((stringnumber as usize) * 2);
    right = *(*match_data)
        .ovector
        .as_ptr()
        .add((stringnumber as usize) * 2 + 1);
    /* LCOV_EXCL_START - this appears to be unreachable, as the ovector and
    subject_length should always be set consistently, no matter what misbehaviour
    the caller has committed. */
    if left > (*match_data).subject_length || right > (*match_data).subject_length {
        return PCRE2_ERROR_INVALIDOFFSET;
    }
    /* LCOV_EXCL_STOP */
    if !sizeptr.is_null() {
        *sizeptr = if left > right { 0 } else { right - left };
    }
    0
}

/*************************************************
*    Extract all captured strings to new memory  *
*************************************************/

/* This function gets one chunk of memory and builds a list of pointers and all
the captured substrings in it. A NULL pointer is put on the end of the list.
The substrings are zero-terminated, but also, if the final argument is
non-NULL, a list of lengths is also returned. This allows binary data to be
handled.

Arguments:
  match_data     points to the match data
  listptr        set to point to the list of pointers
  lengthsptr     set to point to the list of lengths (may be NULL)

Returns:         if successful: 0
                 if not successful, a negative error code:
                   PCRE2_ERROR_NOMEMORY: failed to get memory,
                   or a match failure code
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_list_get_8(
    match_data: *mut pcre2_real_match_data,
    listptr: *mut *mut *mut PCRE2_UCHAR,
    lengthsptr: *mut *mut PCRE2_SIZE,
) -> c_int {
    let mut i: c_int;
    let mut count: c_int;
    let count2: c_int;
    let mut size: PCRE2_SIZE;
    let mut lensp: *mut PCRE2_SIZE;
    let memp: *mut pcre2_memctl;
    let mut listp: *mut *mut PCRE2_UCHAR;
    let mut sp: *mut PCRE2_UCHAR;
    let ovector: *mut PCRE2_SIZE;

    count = (*match_data).rc;
    if count < 0 {
        return count; /* Match failed */
    }
    if count == 0 {
        count = (*match_data).oveccount as c_int; /* Ovector too small */
    }

    count2 = 2 * count;
    ovector = (*match_data).ovector.as_mut_ptr();
    size = size_of::<pcre2_memctl>() + size_of::<*mut PCRE2_UCHAR>(); /* For final NULL */
    if !lengthsptr.is_null() {
        size += size_of::<PCRE2_SIZE>() * count as usize; /* For lengths */
    }

    i = 0;
    while i < count2 {
        size += size_of::<*mut PCRE2_UCHAR>() + CU2BYTES!(1);
        if *ovector.add(i as usize + 1) > *ovector.add(i as usize) {
            size += CU2BYTES!(*ovector.add(i as usize + 1) - *ovector.add(i as usize));
        }
        i += 2;
    }

    memp = _pcre2_memctl_malloc_8(size, match_data as *mut pcre2_memctl) as *mut pcre2_memctl;
    if memp.is_null() {
        return PCRE2_ERROR_NOMEMORY;
    }

    listp = (memp as *mut c_char).add(size_of::<pcre2_memctl>()) as *mut *mut PCRE2_UCHAR;
    *listptr = listp;
    lensp = (listp as *mut c_char)
        .add(size_of::<*mut PCRE2_UCHAR>() * (count as usize + 1)) as *mut PCRE2_SIZE;

    if lengthsptr.is_null() {
        sp = lensp as *mut PCRE2_UCHAR;
        lensp = std::ptr::null_mut();
    } else {
        *lengthsptr = lensp;
        sp = (lensp as *mut c_char).add(size_of::<PCRE2_SIZE>() * count as usize)
            as *mut PCRE2_UCHAR;
    }

    i = 0;
    while i < count2 {
        size = if *ovector.add(i as usize + 1) > *ovector.add(i as usize) {
            *ovector.add(i as usize + 1) - *ovector.add(i as usize)
        } else {
            0
        };

        /* Size == 0 includes the case when the capture is unset. Avoid adding
        PCRE2_UNSET to match_data->subject because it overflows, even though with
        zero size calling memcpy() is harmless. */

        if size != 0 {
            memcpy(
                sp as *mut c_void,
                (*match_data).subject.add(*ovector.add(i as usize)) as *const c_void,
                CU2BYTES!(size),
            );
        }
        *listp = sp;
        listp = listp.add(1);
        if !lensp.is_null() {
            *lensp = size;
            lensp = lensp.add(1);
        }
        sp = sp.add(size);
        *sp = 0;
        sp = sp.add(1);
        i += 2;
    }

    *listp = std::ptr::null_mut();
    0
}

/*************************************************
*   Free memory obtained by substring_list_get   *
*************************************************/

/*
Argument:     the result of a previous pcre2_substring_list_get()
Returns:      nothing
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_list_free_8(list: *mut *mut PCRE2_UCHAR) {
    if !list.is_null() {
        let memctl: *mut pcre2_memctl =
            (list as *mut c_char).sub(size_of::<pcre2_memctl>()) as *mut pcre2_memctl;
        ((*memctl).free.unwrap())(memctl as *mut c_void, (*memctl).memory_data);
    }
}

/*************************************************
*     Find (multiple) entries for named string   *
*************************************************/

/* This function scans the nametable for a given name, using binary chop. It
returns either two pointers to the entries in the table, or, if no pointers are
given, the number of a unique group with the given name. If duplicate names are
permitted, and the name is not unique, an error is generated.

Arguments:
  code        the compiled regex
  stringname  the name whose entries required
  firstptr    where to put the pointer to the first entry
  lastptr     where to put the pointer to the last entry

Returns:      PCRE2_ERROR_NOSUBSTRING if the name is not found
              otherwise, if firstptr and lastptr are NULL:
                a group number for a unique substring
                else PCRE2_ERROR_NOUNIQUESUBSTRING
              otherwise:
                the length of each entry, having set firstptr and lastptr
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_nametable_scan_8(
    code: *const pcre2_real_code,
    stringname: PCRE2_SPTR,
    firstptr: *mut PCRE2_SPTR,
    lastptr: *mut PCRE2_SPTR,
) -> c_int {
    let mut bot: u16 = 0;
    let mut top: u16 = (*code).name_count;
    let entrysize: u16 = (*code).name_entry_size;
    let nametable: PCRE2_SPTR =
        (code as *const c_char).add(size_of::<pcre2_real_code>()) as PCRE2_SPTR;

    while top > bot {
        let mid: u16 = ((top as u32 + bot as u32) / 2) as u16;
        let entry: PCRE2_SPTR = nametable.add(entrysize as usize * mid as usize);
        let c: c_int = _pcre2_strcmp_8(stringname, entry.add(IMM2_SIZE));
        if c == 0 {
            let mut first: PCRE2_SPTR;
            let mut last: PCRE2_SPTR;
            let lastentry: PCRE2_SPTR;
            lastentry = nametable.add(entrysize as usize * ((*code).name_count as usize - 1));
            last = entry;
            first = last;
            while first > nametable {
                if _pcre2_strcmp_8(stringname, first.sub(entrysize as usize).add(IMM2_SIZE)) != 0 {
                    break;
                }
                first = first.sub(entrysize as usize);
            }
            while last < lastentry {
                if _pcre2_strcmp_8(stringname, last.add(entrysize as usize).add(IMM2_SIZE)) != 0 {
                    break;
                }
                last = last.add(entrysize as usize);
            }
            if firstptr.is_null() {
                return if first == last {
                    GET2!(entry, 0) as c_int
                } else {
                    PCRE2_ERROR_NOUNIQUESUBSTRING
                };
            }
            *firstptr = first;
            *lastptr = last;
            return entrysize as c_int;
        }
        if c > 0 {
            bot = mid + 1;
        } else {
            top = mid;
        }
    }

    PCRE2_ERROR_NOSUBSTRING
}

/*************************************************
*           Find number for named string         *
*************************************************/

/* This function is a convenience wrapper for pcre2_substring_nametable_scan()
when it is known that names are unique. If there are duplicate names, it is not
defined which number is returned.

Arguments:
  code        the compiled regex
  stringname  the name whose number is required

Returns:      the number of the named parenthesis, or a negative number
                PCRE2_ERROR_NOSUBSTRING if not found
                PCRE2_ERROR_NOUNIQUESUBSTRING if not unique
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_number_from_name_8(
    code: *const pcre2_real_code,
    stringname: PCRE2_SPTR,
) -> c_int {
    pcre2_substring_nametable_scan_8(code, stringname, std::ptr::null_mut(), std::ptr::null_mut())
}

/* End of pcre2_substring.c */
