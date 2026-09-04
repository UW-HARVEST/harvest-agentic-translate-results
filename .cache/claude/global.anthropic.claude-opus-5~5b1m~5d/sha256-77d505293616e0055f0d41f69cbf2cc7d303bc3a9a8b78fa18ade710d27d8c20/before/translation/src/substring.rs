//! Translated from pcre2_substring.c.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use core::ffi::{c_char, c_void};

use crate::context::_pcre2_memctl_malloc_8;
use crate::string_utils::_pcre2_strcmp_8;

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
pub unsafe extern "C" fn pcre2_substring_copy_byname_8(match_data: *mut pcre2_real_match_data, stringname: PCRE2_SPTR, buffer: *mut PCRE2_UCHAR, sizeptr: *mut PCRE2_SIZE) -> i32 {
    let mut first: PCRE2_SPTR = core::ptr::null();
    let mut last: PCRE2_SPTR = core::ptr::null();
    let mut entry: PCRE2_SPTR;
    let mut failrc: i32;
    let entrysize: i32;
    if (*match_data).matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER {
        return PCRE2_ERROR_DFA_UFUNC;
    }
    entrysize = pcre2_substring_nametable_scan_8(
        (*match_data).code,
        stringname,
        &mut first as *mut PCRE2_SPTR,
        &mut last as *mut PCRE2_SPTR,
    );
    if entrysize < 0 {
        return entrysize;
    }
    failrc = PCRE2_ERROR_UNAVAILABLE;
    entry = first;
    while entry <= last {
        let n: u32 = GET2!(entry, 0);
        if n < (*match_data).oveccount as u32 {
            if *(*match_data).ovector.as_ptr().add((n * 2) as usize) != PCRE2_UNSET {
                return pcre2_substring_copy_bynumber_8(match_data, n, buffer, sizeptr);
            }
            failrc = PCRE2_ERROR_UNSET;
        }
        entry = entry.offset(entrysize as isize);
    }
    failrc
}

/*************************************************
*  Copy numbered captured string to given buffer *
*************************************************/

/* This function copies a single captured substring into a given buffer,
identifying it by number.

Returns:         if successful: 0
                 if not successful, a negative error code:
                   PCRE2_ERROR_NOMEMORY: buffer too small
                   PCRE2_ERROR_NOSUBSTRING: no such substring
                   PCRE2_ERROR_UNAVAILABLE: ovector too small
                   PCRE2_ERROR_UNSET: substring is not set
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_copy_bynumber_8(match_data: *mut pcre2_real_match_data, stringnumber: u32, buffer: *mut PCRE2_UCHAR, sizeptr: *mut PCRE2_SIZE) -> i32 {
    let rc: i32;
    let mut size: PCRE2_SIZE = 0;
    rc = pcre2_substring_length_bynumber_8(match_data, stringnumber, &mut size as *mut PCRE2_SIZE);
    if rc < 0 {
        return rc;
    }
    if size + 1 > *sizeptr {
        return PCRE2_ERROR_NOMEMORY;
    }
    if size != 0 {
        core::ptr::copy_nonoverlapping(
            (*match_data)
                .subject
                .add(*(*match_data).ovector.as_ptr().add((stringnumber * 2) as usize)),
            buffer,
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
set is chosen. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_get_byname_8(match_data: *mut pcre2_real_match_data, stringname: PCRE2_SPTR, stringptr: *mut *mut PCRE2_UCHAR, sizeptr: *mut PCRE2_SIZE) -> i32 {
    let mut first: PCRE2_SPTR = core::ptr::null();
    let mut last: PCRE2_SPTR = core::ptr::null();
    let mut entry: PCRE2_SPTR;
    let mut failrc: i32;
    let entrysize: i32;
    if (*match_data).matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER {
        return PCRE2_ERROR_DFA_UFUNC;
    }
    entrysize = pcre2_substring_nametable_scan_8(
        (*match_data).code,
        stringname,
        &mut first as *mut PCRE2_SPTR,
        &mut last as *mut PCRE2_SPTR,
    );
    if entrysize < 0 {
        return entrysize;
    }
    failrc = PCRE2_ERROR_UNAVAILABLE;
    entry = first;
    while entry <= last {
        let n: u32 = GET2!(entry, 0);
        if n < (*match_data).oveccount as u32 {
            if *(*match_data).ovector.as_ptr().add((n * 2) as usize) != PCRE2_UNSET {
                return pcre2_substring_get_bynumber_8(match_data, n, stringptr, sizeptr);
            }
            failrc = PCRE2_ERROR_UNSET;
        }
        entry = entry.offset(entrysize as isize);
    }
    failrc
}

/*************************************************
*      Extract captured string to new memory     *
*************************************************/

/* This function copies a single captured substring into a piece of new
memory.

Returns:         if successful: 0
                 if not successful, a negative error code:
                   PCRE2_ERROR_NOMEMORY: failed to get memory
                   PCRE2_ERROR_NOSUBSTRING: no such substring
                   PCRE2_ERROR_UNAVAILABLE: ovector too small
                   PCRE2_ERROR_UNSET: substring is not set
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_get_bynumber_8(match_data: *mut pcre2_real_match_data, stringnumber: u32, stringptr: *mut *mut PCRE2_UCHAR, sizeptr: *mut PCRE2_SIZE) -> i32 {
    let rc: i32;
    let mut size: PCRE2_SIZE = 0;
    let mut yield_: *mut PCRE2_UCHAR;
    rc = pcre2_substring_length_bynumber_8(match_data, stringnumber, &mut size as *mut PCRE2_SIZE);
    if rc < 0 {
        return rc;
    }
    yield_ = _pcre2_memctl_malloc_8(
        core::mem::size_of::<pcre2_memctl>() + (size + 1) * 8, /* PCRE2_CODE_UNIT_WIDTH */
        match_data as *mut pcre2_memctl,
    ) as *mut PCRE2_UCHAR;
    if yield_.is_null() {
        return PCRE2_ERROR_NOMEMORY;
    }
    yield_ = (yield_ as *mut c_char).add(core::mem::size_of::<pcre2_memctl>()) as *mut PCRE2_UCHAR;
    if size != 0 {
        core::ptr::copy_nonoverlapping(
            (*match_data)
                .subject
                .add(*(*match_data).ovector.as_ptr().add((stringnumber * 2) as usize)),
            yield_,
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_free_8(string: *mut PCRE2_UCHAR) {
    if !string.is_null() {
        let memctl: *mut pcre2_memctl =
            (string as *mut c_char).sub(core::mem::size_of::<pcre2_memctl>()) as *mut pcre2_memctl;
        ((*memctl).free.unwrap())(memctl as *mut c_void, (*memctl).memory_data);
    }
}

/*************************************************
*         Get length of a named substring        *
*************************************************/

/* This function returns the length of a named captured substring. If the regex
permits duplicate names, the first substring that is set is chosen. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_length_byname_8(match_data: *mut pcre2_real_match_data, stringname: PCRE2_SPTR, lengthptr: *mut PCRE2_SIZE) -> i32 {
    let mut first: PCRE2_SPTR = core::ptr::null();
    let mut last: PCRE2_SPTR = core::ptr::null();
    let mut entry: PCRE2_SPTR;
    let mut failrc: i32;
    let entrysize: i32;
    if (*match_data).matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER {
        return PCRE2_ERROR_DFA_UFUNC;
    }
    entrysize = pcre2_substring_nametable_scan_8(
        (*match_data).code,
        stringname,
        &mut first as *mut PCRE2_SPTR,
        &mut last as *mut PCRE2_SPTR,
    );
    if entrysize < 0 {
        return entrysize;
    }
    failrc = PCRE2_ERROR_UNAVAILABLE;
    entry = first;
    while entry <= last {
        let n: u32 = GET2!(entry, 0);
        if n < (*match_data).oveccount as u32 {
            if *(*match_data).ovector.as_ptr().add((n * 2) as usize) != PCRE2_UNSET {
                return pcre2_substring_length_bynumber_8(match_data, n, lengthptr);
            }
            failrc = PCRE2_ERROR_UNSET;
        }
        entry = entry.offset(entrysize as isize);
    }
    failrc
}

/*************************************************
*        Get length of a numbered substring      *
*************************************************/

/* This function returns the length of a captured substring. If the start is
beyond the end (which can happen when \K is used in an assertion), it sets the
length to zero.

Returns:         if successful: 0
                 if not successful, a negative error code:
                   PCRE2_ERROR_NOSUBSTRING: no such substring
                   PCRE2_ERROR_UNAVAILABLE: ovector is too small
                   PCRE2_ERROR_UNSET: substring is not set
                   PCRE2_ERROR_INVALIDOFFSET: internal error, should not occur
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_length_bynumber_8(match_data: *mut pcre2_real_match_data, stringnumber: u32, lengthptr: *mut PCRE2_SIZE) -> i32 {
    let left: PCRE2_SIZE;
    let right: PCRE2_SIZE;
    let mut count: i32 = (*match_data).rc;
    if count == PCRE2_ERROR_PARTIAL {
        if stringnumber > 0 {
            return PCRE2_ERROR_PARTIAL;
        }
        count = 0;
    } else if count < 0 {
        return count; /* Match failed */
    }

    if (*match_data).matchedby != PCRE2_MATCHEDBY_DFA_INTERPRETER {
        if stringnumber > (*(*match_data).code).top_bracket as u32 {
            return PCRE2_ERROR_NOSUBSTRING;
        }
        if stringnumber >= (*match_data).oveccount as u32 {
            return PCRE2_ERROR_UNAVAILABLE;
        }
        if *(*match_data)
            .ovector
            .as_ptr()
            .add((stringnumber * 2) as usize)
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
        .add((stringnumber * 2) as usize);
    right = *(*match_data)
        .ovector
        .as_ptr()
        .add((stringnumber * 2 + 1) as usize);
    /* LCOV_EXCL_START - this appears to be unreachable, as the ovector and
    subject_length should always be set consistently, no matter what misbehaviour
    the caller has committed. */
    if left > (*match_data).subject_length || right > (*match_data).subject_length {
        /* PCRE2_DEBUG_UNREACHABLE */
        return PCRE2_ERROR_INVALIDOFFSET;
    }
    /* LCOV_EXCL_STOP */
    if !lengthptr.is_null() {
        *lengthptr = if left > right { 0 } else { right - left };
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

Returns:         if successful: 0
                 if not successful, a negative error code:
                   PCRE2_ERROR_NOMEMORY: failed to get memory,
                   or a match failure code
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_list_get_8(match_data: *mut pcre2_real_match_data, listptr: *mut *mut *mut PCRE2_UCHAR, lengthsptr: *mut *mut PCRE2_SIZE) -> i32 {
    let mut i: i32;
    let mut count: i32;
    let count2: i32;
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
        count = (*match_data).oveccount as i32; /* Ovector too small */
    }

    count2 = 2 * count;
    ovector = (*match_data).ovector.as_mut_ptr();
    size = core::mem::size_of::<pcre2_memctl>() + core::mem::size_of::<*mut PCRE2_UCHAR>(); /* For final NULL */
    if !lengthsptr.is_null() {
        size += core::mem::size_of::<PCRE2_SIZE>() * count as usize; /* For lengths */
    }

    i = 0;
    while i < count2 {
        size += core::mem::size_of::<*mut PCRE2_UCHAR>() + CU2BYTES!(1);
        if *ovector.add((i + 1) as usize) > *ovector.add(i as usize) {
            size += CU2BYTES!(*ovector.add((i + 1) as usize) - *ovector.add(i as usize));
        }
        i += 2;
    }

    memp = _pcre2_memctl_malloc_8(size, match_data as *mut pcre2_memctl) as *mut pcre2_memctl;
    if memp.is_null() {
        return PCRE2_ERROR_NOMEMORY;
    }

    listp = (memp as *mut c_char).add(core::mem::size_of::<pcre2_memctl>()) as *mut *mut PCRE2_UCHAR;
    *listptr = listp;
    lensp = (listp as *mut c_char)
        .add(core::mem::size_of::<*mut PCRE2_UCHAR>() * (count + 1) as usize)
        as *mut PCRE2_SIZE;

    if lengthsptr.is_null() {
        sp = lensp as *mut PCRE2_UCHAR;
        lensp = core::ptr::null_mut();
    } else {
        *lengthsptr = lensp;
        sp = (lensp as *mut c_char).add(core::mem::size_of::<PCRE2_SIZE>() * count as usize)
            as *mut PCRE2_UCHAR;
    }

    i = 0;
    while i < count2 {
        size = if *ovector.add((i + 1) as usize) > *ovector.add(i as usize) {
            *ovector.add((i + 1) as usize) - *ovector.add(i as usize)
        } else {
            0
        };

        /* Size == 0 includes the case when the capture is unset. Avoid adding
        PCRE2_UNSET to match_data->subject because it overflows, even though with
        zero size calling memcpy() is harmless. */

        if size != 0 {
            core::ptr::copy_nonoverlapping(
                (*match_data).subject.add(*ovector.add(i as usize)),
                sp,
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

    *listp = core::ptr::null_mut();
    0
}

/*************************************************
*   Free memory obtained by substring_list_get   *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_list_free_8(list: *mut *mut PCRE2_UCHAR) {
    if !list.is_null() {
        let memctl: *mut pcre2_memctl =
            (list as *mut c_char).sub(core::mem::size_of::<pcre2_memctl>()) as *mut pcre2_memctl;
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

Returns:      PCRE2_ERROR_NOSUBSTRING if the name is not found
              otherwise, if firstptr and lastptr are NULL:
                a group number for a unique substring
                else PCRE2_ERROR_NOUNIQUESUBSTRING
              otherwise:
                the length of each entry, having set firstptr and lastptr
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_nametable_scan_8(code: *const pcre2_real_code, stringname: PCRE2_SPTR, firstptr: *mut PCRE2_SPTR, lastptr: *mut PCRE2_SPTR) -> i32 {
    let mut bot: u16 = 0;
    let mut top: u16 = (*code).name_count;
    let entrysize: u16 = (*code).name_entry_size;
    let nametable: PCRE2_SPTR = (code as *const c_char)
        .add(core::mem::size_of::<pcre2_real_code>()) as PCRE2_SPTR;

    while top > bot {
        let mid: u16 = ((top as i32 + bot as i32) / 2) as u16;
        let entry: PCRE2_SPTR = nametable.offset((entrysize as i32 * mid as i32) as isize);
        let c: i32 = _pcre2_strcmp_8(stringname, entry.add(IMM2_SIZE));
        if c == 0 {
            let mut first: PCRE2_SPTR;
            let mut last: PCRE2_SPTR;
            let lastentry: PCRE2_SPTR;
            lastentry =
                nametable.offset((entrysize as i32 * ((*code).name_count as i32 - 1)) as isize);
            last = entry;
            first = last;
            while first > nametable {
                if _pcre2_strcmp_8(
                    stringname,
                    first
                        .offset(-(entrysize as isize))
                        .add(IMM2_SIZE),
                ) != 0
                {
                    break;
                }
                first = first.offset(-(entrysize as isize));
            }
            while last < lastentry {
                if _pcre2_strcmp_8(stringname, last.offset(entrysize as isize).add(IMM2_SIZE)) != 0
                {
                    break;
                }
                last = last.offset(entrysize as isize);
            }
            if firstptr.is_null() {
                return if first == last {
                    GET2!(entry, 0) as i32
                } else {
                    PCRE2_ERROR_NOUNIQUESUBSTRING
                };
            }
            *firstptr = first;
            *lastptr = last;
            return entrysize as i32;
        }
        if c > 0 {
            bot = (mid as i32 + 1) as u16;
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
defined which number is returned. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_number_from_name_8(code: *const pcre2_real_code, stringname: PCRE2_SPTR) -> i32 {
    pcre2_substring_nametable_scan_8(
        code,
        stringname,
        core::ptr::null_mut(),
        core::ptr::null_mut(),
    )
}

/* End of pcre2_substring.c */
