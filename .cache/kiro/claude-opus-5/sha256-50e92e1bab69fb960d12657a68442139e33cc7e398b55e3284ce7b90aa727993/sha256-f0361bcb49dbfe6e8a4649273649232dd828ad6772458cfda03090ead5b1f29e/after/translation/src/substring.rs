//! Translation of `pcre2_substring.c`.

use crate::context::_pcre2_memctl_malloc_8;
use crate::internal::*;
use crate::string_utils::_pcre2_strcmp_8;
use core::ffi::{c_int, c_void};
use core::ptr;

// `PCRE2_MATCHEDBY_DFA_INTERPRETER` from pcre2_intmodedep.h.
const PCRE2_MATCHEDBY_DFA_INTERPRETER: u8 = 1;

// In 8-bit mode `PCRE2_CODE_UNIT_WIDTH == 8`; the C source uses it directly as a
// byte multiplier when sizing allocations.
const PCRE2_CODE_UNIT_WIDTH_U: usize = 8;

// ---------------------------------------------------------------------------
// Copy named captured string to given buffer
// ---------------------------------------------------------------------------

/// `pcre2_substring_copy_byname()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_copy_byname_8(
    match_data: *mut pcre2_match_data,
    stringname: PCRE2_SPTR,
    buffer: *mut PCRE2_UCHAR,
    sizeptr: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
        let mut first: PCRE2_SPTR = ptr::null();
        let mut last: PCRE2_SPTR = ptr::null();
        if (*match_data).matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER {
            return PCRE2_ERROR_DFA_UFUNC as c_int;
        }
        let entrysize = pcre2_substring_nametable_scan_8(
            (*match_data).code,
            stringname,
            &mut first,
            &mut last,
        );
        if entrysize < 0 {
            return entrysize;
        }
        let mut failrc = PCRE2_ERROR_UNAVAILABLE as c_int;
        let ovector = (*match_data).ovec();
        let mut entry = first;
        while entry <= last {
            let n = GET2(entry, 0);
            if n < (*match_data).oveccount as u32 {
                if *ovector.add(n as usize * 2) != PCRE2_UNSET {
                    return pcre2_substring_copy_bynumber_8(match_data, n, buffer, sizeptr);
                }
                failrc = PCRE2_ERROR_UNSET as c_int;
            }
            entry = entry.add(entrysize as usize);
        }
        failrc
    }
}

// ---------------------------------------------------------------------------
// Copy numbered captured string to given buffer
// ---------------------------------------------------------------------------

/// `pcre2_substring_copy_bynumber()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_copy_bynumber_8(
    match_data: *mut pcre2_match_data,
    stringnumber: u32,
    buffer: *mut PCRE2_UCHAR,
    sizeptr: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
        let mut size: PCRE2_SIZE = 0;
        let rc = pcre2_substring_length_bynumber_8(match_data, stringnumber, &mut size);
        if rc < 0 {
            return rc;
        }
        if size + 1 > *sizeptr {
            return PCRE2_ERROR_NOMEMORY as c_int;
        }
        let ovector = (*match_data).ovec();
        if size != 0 {
            c_memcpy(
                buffer as *mut c_void,
                (*match_data)
                    .subject
                    .add(*ovector.add(stringnumber as usize * 2)) as *const c_void,
                CU2BYTES(size),
            );
        }
        *buffer.add(size) = 0;
        *sizeptr = size;
        0
    }
}

// ---------------------------------------------------------------------------
// Extract named captured string
// ---------------------------------------------------------------------------

/// `pcre2_substring_get_byname()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_get_byname_8(
    match_data: *mut pcre2_match_data,
    stringname: PCRE2_SPTR,
    stringptr: *mut *mut PCRE2_UCHAR,
    sizeptr: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
        let mut first: PCRE2_SPTR = ptr::null();
        let mut last: PCRE2_SPTR = ptr::null();
        if (*match_data).matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER {
            return PCRE2_ERROR_DFA_UFUNC as c_int;
        }
        let entrysize = pcre2_substring_nametable_scan_8(
            (*match_data).code,
            stringname,
            &mut first,
            &mut last,
        );
        if entrysize < 0 {
            return entrysize;
        }
        let mut failrc = PCRE2_ERROR_UNAVAILABLE as c_int;
        let ovector = (*match_data).ovec();
        let mut entry = first;
        while entry <= last {
            let n = GET2(entry, 0);
            if n < (*match_data).oveccount as u32 {
                if *ovector.add(n as usize * 2) != PCRE2_UNSET {
                    return pcre2_substring_get_bynumber_8(match_data, n, stringptr, sizeptr);
                }
                failrc = PCRE2_ERROR_UNSET as c_int;
            }
            entry = entry.add(entrysize as usize);
        }
        failrc
    }
}

// ---------------------------------------------------------------------------
// Extract captured string to new memory
// ---------------------------------------------------------------------------

/// `pcre2_substring_get_bynumber()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_get_bynumber_8(
    match_data: *mut pcre2_match_data,
    stringnumber: u32,
    stringptr: *mut *mut PCRE2_UCHAR,
    sizeptr: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
        let mut size: PCRE2_SIZE = 0;
        let rc = pcre2_substring_length_bynumber_8(match_data, stringnumber, &mut size);
        if rc < 0 {
            return rc;
        }
        let mut yield_ = _pcre2_memctl_malloc_8(
            core::mem::size_of::<pcre2_memctl>() + (size + 1) * PCRE2_CODE_UNIT_WIDTH_U,
            match_data as *mut pcre2_memctl,
        ) as *mut PCRE2_UCHAR;
        if yield_.is_null() {
            return PCRE2_ERROR_NOMEMORY as c_int;
        }
        yield_ = ((yield_ as *mut u8).add(core::mem::size_of::<pcre2_memctl>())) as *mut PCRE2_UCHAR;
        let ovector = (*match_data).ovec();
        if size != 0 {
            c_memcpy(
                yield_ as *mut c_void,
                (*match_data)
                    .subject
                    .add(*ovector.add(stringnumber as usize * 2)) as *const c_void,
                CU2BYTES(size),
            );
        }
        *yield_.add(size) = 0;
        *stringptr = yield_;
        *sizeptr = size;
        0
    }
}

// ---------------------------------------------------------------------------
// Free memory obtained by get_substring
// ---------------------------------------------------------------------------

/// `pcre2_substring_free()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_free_8(string: *mut PCRE2_UCHAR) {
    unsafe {
        if !string.is_null() {
            let memctl = (string as *mut u8).sub(core::mem::size_of::<pcre2_memctl>())
                as *mut pcre2_memctl;
            ((*memctl).free.unwrap())(memctl as *mut c_void, (*memctl).memory_data);
        }
    }
}

// ---------------------------------------------------------------------------
// Get length of a named substring
// ---------------------------------------------------------------------------

/// `pcre2_substring_length_byname()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_length_byname_8(
    match_data: *mut pcre2_match_data,
    stringname: PCRE2_SPTR,
    sizeptr: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
        let mut first: PCRE2_SPTR = ptr::null();
        let mut last: PCRE2_SPTR = ptr::null();
        if (*match_data).matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER {
            return PCRE2_ERROR_DFA_UFUNC as c_int;
        }
        let entrysize = pcre2_substring_nametable_scan_8(
            (*match_data).code,
            stringname,
            &mut first,
            &mut last,
        );
        if entrysize < 0 {
            return entrysize;
        }
        let mut failrc = PCRE2_ERROR_UNAVAILABLE as c_int;
        let ovector = (*match_data).ovec();
        let mut entry = first;
        while entry <= last {
            let n = GET2(entry, 0);
            if n < (*match_data).oveccount as u32 {
                if *ovector.add(n as usize * 2) != PCRE2_UNSET {
                    return pcre2_substring_length_bynumber_8(match_data, n, sizeptr);
                }
                failrc = PCRE2_ERROR_UNSET as c_int;
            }
            entry = entry.add(entrysize as usize);
        }
        failrc
    }
}

// ---------------------------------------------------------------------------
// Get length of a numbered substring
// ---------------------------------------------------------------------------

/// `pcre2_substring_length_bynumber()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_length_bynumber_8(
    match_data: *mut pcre2_match_data,
    stringnumber: u32,
    sizeptr: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
        let left: PCRE2_SIZE;
        let right: PCRE2_SIZE;
        let mut count = (*match_data).rc;
        if count as i64 == PCRE2_ERROR_PARTIAL {
            if stringnumber > 0 {
                return PCRE2_ERROR_PARTIAL as c_int;
            }
            count = 0;
        } else if count < 0 {
            return count; // Match failed
        }

        if (*match_data).matchedby != PCRE2_MATCHEDBY_DFA_INTERPRETER {
            if stringnumber > (*(*match_data).code).top_bracket as u32 {
                return PCRE2_ERROR_NOSUBSTRING as c_int;
            }
            if stringnumber >= (*match_data).oveccount as u32 {
                return PCRE2_ERROR_UNAVAILABLE as c_int;
            }
            let ovector = (*match_data).ovec();
            if *ovector.add(stringnumber as usize * 2) == PCRE2_UNSET {
                return PCRE2_ERROR_UNSET as c_int;
            }
        } else {
            // Matched using pcre2_dfa_match()
            if stringnumber >= (*match_data).oveccount as u32 {
                return PCRE2_ERROR_UNAVAILABLE as c_int;
            }
            if count != 0 && stringnumber >= count as u32 {
                return PCRE2_ERROR_UNSET as c_int;
            }
        }

        let ovector = (*match_data).ovec();
        left = *ovector.add(stringnumber as usize * 2);
        right = *ovector.add(stringnumber as usize * 2 + 1);
        // LCOV_EXCL_START - this appears to be unreachable.
        if left > (*match_data).subject_length || right > (*match_data).subject_length {
            return PCRE2_ERROR_INVALIDOFFSET as c_int;
        }
        // LCOV_EXCL_STOP
        if !sizeptr.is_null() {
            *sizeptr = if left > right { 0 } else { right - left };
        }
        0
    }
}

// ---------------------------------------------------------------------------
// Extract all captured strings to new memory
// ---------------------------------------------------------------------------

/// `pcre2_substring_list_get()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_list_get_8(
    match_data: *mut pcre2_match_data,
    listptr: *mut *mut *mut PCRE2_UCHAR,
    lengthsptr: *mut *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
        let mut count = (*match_data).rc;
        if count < 0 {
            return count; // Match failed
        }
        if count == 0 {
            count = (*match_data).oveccount as c_int; // Ovector too small
        }

        let count2 = 2 * count;
        let ovector = (*match_data).ovec();
        let mut size: PCRE2_SIZE =
            core::mem::size_of::<pcre2_memctl>() + core::mem::size_of::<*mut PCRE2_UCHAR>(); // For final NULL
        if !lengthsptr.is_null() {
            size += core::mem::size_of::<PCRE2_SIZE>() * count as usize; // For lengths
        }

        let mut i: c_int = 0;
        while i < count2 {
            size += core::mem::size_of::<*mut PCRE2_UCHAR>() + CU2BYTES(1);
            if *ovector.add(i as usize + 1) > *ovector.add(i as usize) {
                size += CU2BYTES(*ovector.add(i as usize + 1) - *ovector.add(i as usize));
            }
            i += 2;
        }

        let memp = _pcre2_memctl_malloc_8(size, match_data as *mut pcre2_memctl) as *mut pcre2_memctl;
        if memp.is_null() {
            return PCRE2_ERROR_NOMEMORY as c_int;
        }

        let listp =
            ((memp as *mut u8).add(core::mem::size_of::<pcre2_memctl>())) as *mut *mut PCRE2_UCHAR;
        *listptr = listp;
        let mut lensp = ((listp as *mut u8)
            .add(core::mem::size_of::<*mut PCRE2_UCHAR>() * (count as usize + 1)))
            as *mut PCRE2_SIZE;

        let mut sp: *mut PCRE2_UCHAR;
        if lengthsptr.is_null() {
            sp = lensp as *mut PCRE2_UCHAR;
            lensp = ptr::null_mut();
        } else {
            *lengthsptr = lensp;
            sp = ((lensp as *mut u8).add(core::mem::size_of::<PCRE2_SIZE>() * count as usize))
                as *mut PCRE2_UCHAR;
        }

        let mut listp = listp;
        let mut i: c_int = 0;
        while i < count2 {
            let seg_size: PCRE2_SIZE = if *ovector.add(i as usize + 1) > *ovector.add(i as usize) {
                *ovector.add(i as usize + 1) - *ovector.add(i as usize)
            } else {
                0
            };

            // Size == 0 includes the case when the capture is unset. Avoid adding
            // PCRE2_UNSET to match_data->subject because it overflows, even though
            // with zero size calling memcpy() is harmless.
            if seg_size != 0 {
                c_memcpy(
                    sp as *mut c_void,
                    (*match_data).subject.add(*ovector.add(i as usize)) as *const c_void,
                    CU2BYTES(seg_size),
                );
            }
            *listp = sp;
            listp = listp.add(1);
            if !lensp.is_null() {
                *lensp = seg_size;
                lensp = lensp.add(1);
            }
            sp = sp.add(seg_size);
            *sp = 0;
            sp = sp.add(1);
            i += 2;
        }

        *listp = ptr::null_mut();
        0
    }
}

// ---------------------------------------------------------------------------
// Free memory obtained by substring_list_get
// ---------------------------------------------------------------------------

/// `pcre2_substring_list_free()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_list_free_8(list: *mut *mut PCRE2_UCHAR) {
    unsafe {
        if !list.is_null() {
            let memctl = (list as *mut u8).sub(core::mem::size_of::<pcre2_memctl>())
                as *mut pcre2_memctl;
            ((*memctl).free.unwrap())(memctl as *mut c_void, (*memctl).memory_data);
        }
    }
}

// ---------------------------------------------------------------------------
// Find (multiple) entries for named string
// ---------------------------------------------------------------------------

/// `pcre2_substring_nametable_scan()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_nametable_scan_8(
    code: *const pcre2_code,
    stringname: PCRE2_SPTR,
    firstptr: *mut PCRE2_SPTR,
    lastptr: *mut PCRE2_SPTR,
) -> c_int {
    unsafe {
        let mut bot: u16 = 0;
        let mut top: u16 = (*code).name_count;
        let entrysize: u16 = (*code).name_entry_size;
        let nametable: PCRE2_SPTR =
            (code as *const u8).add(core::mem::size_of::<pcre2_real_code>());

        while top > bot {
            let mid: u16 = (top + bot) / 2;
            let entry: PCRE2_SPTR = nametable.add(entrysize as usize * mid as usize);
            let c = _pcre2_strcmp_8(stringname, entry.add(IMM2_SIZE_U));
            if c == 0 {
                let mut first: PCRE2_SPTR;
                let mut last: PCRE2_SPTR;
                let lastentry: PCRE2_SPTR =
                    nametable.add(entrysize as usize * ((*code).name_count as usize - 1));
                first = entry;
                last = entry;
                while first > nametable {
                    if _pcre2_strcmp_8(stringname, first.sub(entrysize as usize).add(IMM2_SIZE_U))
                        != 0
                    {
                        break;
                    }
                    first = first.sub(entrysize as usize);
                }
                while last < lastentry {
                    if _pcre2_strcmp_8(stringname, last.add(entrysize as usize).add(IMM2_SIZE_U))
                        != 0
                    {
                        break;
                    }
                    last = last.add(entrysize as usize);
                }
                if firstptr.is_null() {
                    return if first == last {
                        GET2(entry, 0) as c_int
                    } else {
                        PCRE2_ERROR_NOUNIQUESUBSTRING as c_int
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

        PCRE2_ERROR_NOSUBSTRING as c_int
    }
}

// ---------------------------------------------------------------------------
// Find number for named string
// ---------------------------------------------------------------------------

/// `pcre2_substring_number_from_name()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_number_from_name_8(
    code: *const pcre2_code,
    stringname: PCRE2_SPTR,
) -> c_int {
    unsafe {
        pcre2_substring_nametable_scan_8(code, stringname, ptr::null_mut(), ptr::null_mut())
    }
}
