//! Translation of `c_src/src/pcre2_substring.c`.

#![allow(non_snake_case, non_upper_case_globals, unused_parens)]

use core::ffi::{c_int, c_void};
use core::ptr;

use crate::internal::*;
use crate::string_utils::strcmp;

/* ------------------------------------------------------------------ *
 *   Copy named captured string to given buffer                        *
 * ------------------------------------------------------------------ */

/* This function copies a single captured substring into a given buffer,
identifying it by name. If the regex permits duplicate names, the first
substring that is set is chosen. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_copy_byname_8(
    match_data: *mut pcre2_real_match_data,
    stringname: PCRE2_SPTR,
    buffer: *mut PCRE2_UCHAR,
    sizeptr: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
        let mut first: PCRE2_SPTR = ptr::null();
        let mut last: PCRE2_SPTR = ptr::null();
        let mut entry: PCRE2_SPTR;
        let entrysize: c_int;
        if (*match_data).matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER {
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
        let mut failrc = PCRE2_ERROR_UNAVAILABLE;
        entry = first;
        while entry <= last {
            let n = get2(entry, 0);
            if n < (*match_data).oveccount as u32 {
                if *(*match_data).ovector.as_ptr().add((n * 2) as usize) != PCRE2_UNSET {
                    return pcre2_substring_copy_bynumber_8(match_data, n, buffer, sizeptr);
                }
                failrc = PCRE2_ERROR_UNSET;
            }
            entry = entry.add(entrysize as usize);
        }
        failrc
    }
}

/* ------------------------------------------------------------------ *
 *  Copy numbered captured string to given buffer                      *
 * ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_copy_bynumber_8(
    match_data: *mut pcre2_real_match_data,
    stringnumber: u32,
    buffer: *mut PCRE2_UCHAR,
    sizeptr: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
        let rc: c_int;
        let mut size: PCRE2_SIZE = 0;
        rc = pcre2_substring_length_bynumber_8(match_data, stringnumber, &mut size);
        if rc < 0 {
            return rc;
        }
        if size + 1 > *sizeptr {
            return PCRE2_ERROR_NOMEMORY;
        }
        if size != 0 {
            memcpy(
                buffer,
                (*match_data)
                    .subject
                    .add(*(*match_data).ovector.as_ptr().add((stringnumber * 2) as usize)),
                cu2bytes(size),
            );
        }
        *buffer.add(size) = 0;
        *sizeptr = size;
        0
    }
}

/* ------------------------------------------------------------------ *
 *          Extract named captured string                              *
 * ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_get_byname_8(
    match_data: *mut pcre2_real_match_data,
    stringname: PCRE2_SPTR,
    stringptr: *mut *mut PCRE2_UCHAR,
    sizeptr: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
        let mut first: PCRE2_SPTR = ptr::null();
        let mut last: PCRE2_SPTR = ptr::null();
        let mut entry: PCRE2_SPTR;
        let entrysize: c_int;
        if (*match_data).matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER {
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
        let mut failrc = PCRE2_ERROR_UNAVAILABLE;
        entry = first;
        while entry <= last {
            let n = get2(entry, 0);
            if n < (*match_data).oveccount as u32 {
                if *(*match_data).ovector.as_ptr().add((n * 2) as usize) != PCRE2_UNSET {
                    return pcre2_substring_get_bynumber_8(match_data, n, stringptr, sizeptr);
                }
                failrc = PCRE2_ERROR_UNSET;
            }
            entry = entry.add(entrysize as usize);
        }
        failrc
    }
}

/* ------------------------------------------------------------------ *
 *      Extract captured string to new memory                          *
 * ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_get_bynumber_8(
    match_data: *mut pcre2_real_match_data,
    stringnumber: u32,
    stringptr: *mut *mut PCRE2_UCHAR,
    sizeptr: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
        let rc: c_int;
        let mut size: PCRE2_SIZE = 0;
        let mut yield_: *mut PCRE2_UCHAR;
        rc = pcre2_substring_length_bynumber_8(match_data, stringnumber, &mut size);
        if rc < 0 {
            return rc;
        }
        yield_ = memctl_malloc(
            core::mem::size_of::<pcre2_memctl>()
                + (size + 1) * (PCRE2_CODE_UNIT_WIDTH as usize),
            match_data as *mut pcre2_memctl,
        ) as *mut PCRE2_UCHAR;
        if yield_.is_null() {
            return PCRE2_ERROR_NOMEMORY;
        }
        yield_ = (yield_ as *mut u8).add(core::mem::size_of::<pcre2_memctl>()) as *mut PCRE2_UCHAR;
        if size != 0 {
            memcpy(
                yield_,
                (*match_data)
                    .subject
                    .add(*(*match_data).ovector.as_ptr().add((stringnumber * 2) as usize)),
                cu2bytes(size),
            );
        }
        *yield_.add(size) = 0;
        *stringptr = yield_;
        *sizeptr = size;
        0
    }
}

/* ------------------------------------------------------------------ *
 *       Free memory obtained by get_substring                         *
 * ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_free_8(string: *mut PCRE2_UCHAR) {
    unsafe {
        if !string.is_null() {
            let memctl =
                (string as *mut u8).sub(core::mem::size_of::<pcre2_memctl>()) as *mut pcre2_memctl;
            ((*memctl).free.unwrap())(memctl as *mut c_void, (*memctl).memory_data);
        }
    }
}

/* ------------------------------------------------------------------ *
 *         Get length of a named substring                             *
 * ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_length_byname_8(
    match_data: *mut pcre2_real_match_data,
    stringname: PCRE2_SPTR,
    sizeptr: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
        let mut first: PCRE2_SPTR = ptr::null();
        let mut last: PCRE2_SPTR = ptr::null();
        let mut entry: PCRE2_SPTR;
        let entrysize: c_int;
        if (*match_data).matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER {
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
        let mut failrc = PCRE2_ERROR_UNAVAILABLE;
        entry = first;
        while entry <= last {
            let n = get2(entry, 0);
            if n < (*match_data).oveccount as u32 {
                if *(*match_data).ovector.as_ptr().add((n * 2) as usize) != PCRE2_UNSET {
                    return pcre2_substring_length_bynumber_8(match_data, n, sizeptr);
                }
                failrc = PCRE2_ERROR_UNSET;
            }
            entry = entry.add(entrysize as usize);
        }
        failrc
    }
}

/* ------------------------------------------------------------------ *
 *        Get length of a numbered substring                           *
 * ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_length_bynumber_8(
    match_data: *mut pcre2_real_match_data,
    stringnumber: u32,
    sizeptr: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
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

        if (*match_data).matchedby != PCRE2_MATCHEDBY_DFA_INTERPRETER {
            if stringnumber > (*(*match_data).code).top_bracket as u32 {
                return PCRE2_ERROR_NOSUBSTRING;
            }
            if stringnumber >= (*match_data).oveccount as u32 {
                return PCRE2_ERROR_UNAVAILABLE;
            }
            if *(*match_data).ovector.as_ptr().add((stringnumber * 2) as usize) == PCRE2_UNSET {
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

        left = *(*match_data).ovector.as_ptr().add((stringnumber * 2) as usize);
        right = *(*match_data)
            .ovector
            .as_ptr()
            .add((stringnumber * 2 + 1) as usize);
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
}

/* ------------------------------------------------------------------ *
 *    Extract all captured strings to new memory                       *
 * ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_list_get_8(
    match_data: *mut pcre2_real_match_data,
    listptr: *mut *mut *mut PCRE2_UCHAR,
    lengthsptr: *mut *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
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
        size = core::mem::size_of::<pcre2_memctl>() + core::mem::size_of::<*mut PCRE2_UCHAR>(); /* For final NULL */
        if !lengthsptr.is_null() {
            size += core::mem::size_of::<PCRE2_SIZE>() * count as usize; /* For lengths */
        }

        i = 0;
        while i < count2 {
            size += core::mem::size_of::<*mut PCRE2_UCHAR>() + cu2bytes(1);
            if *ovector.add((i + 1) as usize) > *ovector.add(i as usize) {
                size += cu2bytes(*ovector.add((i + 1) as usize) - *ovector.add(i as usize));
            }
            i += 2;
        }

        memp = memctl_malloc(size, match_data as *mut pcre2_memctl) as *mut pcre2_memctl;
        if memp.is_null() {
            return PCRE2_ERROR_NOMEMORY;
        }

        listp = (memp as *mut u8).add(core::mem::size_of::<pcre2_memctl>()) as *mut *mut PCRE2_UCHAR;
        *listptr = listp;
        lensp = (listp as *mut u8)
            .add(core::mem::size_of::<*mut PCRE2_UCHAR>() * (count + 1) as usize)
            as *mut PCRE2_SIZE;

        if lengthsptr.is_null() {
            sp = lensp as *mut PCRE2_UCHAR;
            lensp = ptr::null_mut();
        } else {
            *lengthsptr = lensp;
            sp = (lensp as *mut u8).add(core::mem::size_of::<PCRE2_SIZE>() * count as usize)
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
                memcpy(
                    sp,
                    (*match_data).subject.add(*ovector.add(i as usize)),
                    cu2bytes(size),
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

        *listp = ptr::null_mut();
        0
    }
}

/* ------------------------------------------------------------------ *
 *   Free memory obtained by substring_list_get                        *
 * ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_list_free_8(list: *mut *mut PCRE2_UCHAR) {
    unsafe {
        if !list.is_null() {
            let memctl =
                (list as *mut u8).sub(core::mem::size_of::<pcre2_memctl>()) as *mut pcre2_memctl;
            ((*memctl).free.unwrap())(memctl as *mut c_void, (*memctl).memory_data);
        }
    }
}

/* ------------------------------------------------------------------ *
 *     Find (multiple) entries for named string                        *
 * ------------------------------------------------------------------ */

/* This function scans the nametable for a given name, using binary chop. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_nametable_scan_8(
    code: *const pcre2_real_code,
    stringname: PCRE2_SPTR,
    firstptr: *mut PCRE2_SPTR,
    lastptr: *mut PCRE2_SPTR,
) -> c_int {
    unsafe {
        let mut bot: u16 = 0;
        let mut top: u16 = (*code).name_count;
        let entrysize: u16 = (*code).name_entry_size;
        let nametable: PCRE2_SPTR =
            (code as *const u8).add(core::mem::size_of::<pcre2_real_code>()) as PCRE2_SPTR;

        while top > bot {
            let mid: u16 = (top + bot) / 2;
            let entry: PCRE2_SPTR = nametable.add((entrysize * mid) as usize);
            let c = strcmp(stringname, entry.add(IMM2_SIZE));
            if c == 0 {
                let mut first: PCRE2_SPTR;
                let mut last: PCRE2_SPTR;
                let lastentry: PCRE2_SPTR;
                lastentry = nametable.add((entrysize * ((*code).name_count - 1)) as usize);
                first = entry;
                last = entry;
                while first > nametable {
                    if strcmp(stringname, first.sub(entrysize as usize).add(IMM2_SIZE)) != 0 {
                        break;
                    }
                    first = first.sub(entrysize as usize);
                }
                while last < lastentry {
                    if strcmp(stringname, last.add(entrysize as usize).add(IMM2_SIZE)) != 0 {
                        break;
                    }
                    last = last.add(entrysize as usize);
                }
                if firstptr.is_null() {
                    return if first == last {
                        get2(entry, 0) as c_int
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
}

/* ------------------------------------------------------------------ *
 *           Find number for named string                              *
 * ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_number_from_name_8(
    code: *const pcre2_real_code,
    stringname: PCRE2_SPTR,
) -> c_int {
    unsafe { pcre2_substring_nametable_scan_8(code, stringname, ptr::null_mut(), ptr::null_mut()) }
}
