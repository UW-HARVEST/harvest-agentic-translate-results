//! Translation of `c_src/src/pcre2_compile_cgroup.c`.
//!
//! Named-capture-group helpers used by the compiler: hash computation, named
//! group lookup, building the name/number table, finding duplicate-name
//! details, and parsing the capture lists of scan-substring and recurse
//! operations.

#![allow(non_snake_case, non_upper_case_globals, unused_parens)]

use core::ffi::{c_int, c_void};
use core::ptr;

use crate::internal::*;
use crate::compile_internal::{
    ERR15, ERR21, ERR53, META_CAPTURE_NAME, META_CAPTURE_NUMBER, META_OFFSET,
    NAMED_GROUP_IS_DUPNAME, getplusoffset, meta_code, meta_data, named_group_get_hash,
    readplusoffset, skipoffset,
};
use crate::string_utils::strncmp;

/* Compute the hash code from a capture name.

This function returns with a simple hash code computed from the name of a
capture group. */
pub unsafe fn get_hash_from_name(name: PCRE2_SPTR, length: u32) -> u16 {
    unsafe {
        debug_assert!(length > 0);

        let hash = ((*name.add(0) as u16) & 0x7f)
            | (((*name.add((length - 1) as usize) as u16) & 0xff) << 7);
        debug_assert!(hash <= crate::compile_internal::NAMED_GROUP_HASH_MASK);
        hash
    }
}

/// Exported as `_pcre2_compile_get_hash_from_name8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_get_hash_from_name8(name: PCRE2_SPTR, length: u32) -> u16 {
    unsafe { get_hash_from_name(name, length) }
}

/* Get the descriptor of a known named capture.

This function returns the descriptor in the named group list of a known capture
group.

Returns: pointer to the descriptor when found, NULL otherwise. */
pub unsafe fn find_named_group(
    name: PCRE2_SPTR,
    length: u32,
    cb: *mut compile_block,
) -> *mut named_group {
    unsafe {
        let hash = get_hash_from_name(name, length);
        let end = (*cb).named_groups.add((*cb).names_found as usize);

        let mut ng = (*cb).named_groups;
        while ng < end {
            if length as u16 == (*ng).length
                && hash == named_group_get_hash(ng)
                && strncmp(name, (*ng).name, length as usize) == 0
            {
                return ng;
            }
            ng = ng.add(1);
        }

        ptr::null_mut()
    }
}

/// Exported as `_pcre2_compile_find_named_group8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_find_named_group8(
    name: PCRE2_SPTR,
    length: u32,
    cb: *mut compile_block,
) -> *mut named_group {
    unsafe { find_named_group(name, length, cb) }
}

/* Add an entry to the name/number table.

This function is called between compiling passes to add an entry to the
name/number table, maintaining alphabetical order. Checking for permitted and
forbidden duplicates has already been done.

Returns: new tablecount. */
pub unsafe fn add_name_to_table(
    cb: *mut compile_block,
    ng: *mut named_group,
    mut tablecount: u32,
) -> u32 {
    unsafe {
        let mut ng = ng;
        let name = (*ng).name;
        let length = (*ng).length as c_int;
        let mut duplicate_count: u32 = 1;

        let mut slot = (*cb).name_table;

        debug_assert!(length > 0);

        if ((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME) != 0 {
            let end = (*cb).named_groups.add((*cb).names_found as usize);

            let mut ng_it = ng.add(1);
            while ng_it < end {
                if (*ng_it).name == name {
                    duplicate_count += 1;
                }
                ng_it = ng_it.add(1);
            }
        }

        let mut i: u32 = 0;
        while i < tablecount {
            let mut crc = memcmp(
                name as *const c_void,
                slot.add(IMM2_SIZE) as *const c_void,
                cu2bytes(length as usize),
            );
            if crc == 0 && *slot.add(IMM2_SIZE + length as usize) != 0 {
                crc = -1; /* Current name is a substring */
            }

            /* Make space in the table and break the loop for an earlier name. For a
            duplicate or later name, carry on. We do this for duplicates so that in the
            simple case (when ?(| is not used) they are in order of their numbers. In all
            cases they are in the order in which they appear in the pattern. */

            if crc < 0 {
                ptr::copy(
                    slot,
                    slot.add((*cb).name_entry_size as usize * duplicate_count as usize),
                    cu2bytes((tablecount - i) as usize * (*cb).name_entry_size as usize),
                );
                break;
            }

            /* Continue the loop for a later or duplicate name */

            slot = slot.add((*cb).name_entry_size as usize);
            i += 1;
        }

        tablecount += duplicate_count;

        loop {
            put2(slot, 0, (*ng).number);
            ptr::copy_nonoverlapping(name, slot.add(IMM2_SIZE), cu2bytes(length as usize));

            /* Add a terminating zero and fill the rest of the slot with zeroes so that
            the memory is all initialized. Otherwise valgrind moans about uninitialized
            memory when saving serialized compiled patterns. */

            ptr::write_bytes(
                slot.add(IMM2_SIZE + length as usize),
                0,
                cu2bytes((*cb).name_entry_size as usize - length as usize - IMM2_SIZE),
            );

            duplicate_count -= 1;
            if duplicate_count == 0 {
                break;
            }

            loop {
                ng = ng.add(1);
                if (*ng).name == name {
                    break;
                }
            }

            slot = slot.add((*cb).name_entry_size as usize);
        }

        tablecount
    }
}

/// Exported as `_pcre2_compile_add_name_to_table8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_add_name_to_table8(
    cb: *mut compile_block,
    ng: *mut named_group,
    tablecount: u32,
) -> u32 {
    unsafe { add_name_to_table(cb, ng, tablecount) }
}

/* Find details of duplicate group names.

This is called from compile_branch() when it needs to know the index and count
of duplicates in the names table when processing named backreferences, either
directly, or as conditions.

Returns: TRUE if OK, FALSE if not, error code set. */
pub unsafe fn find_dupname_details(
    name: PCRE2_SPTR,
    length: u32,
    indexptr: *mut c_int,
    countptr: *mut c_int,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> BOOL {
    unsafe {
        let mut slot = (*cb).name_table;

        /* Find the first entry in the table */

        let mut i: u32 = 0;
        while i < (*cb).names_found as u32 {
            if strncmp(name, slot.add(IMM2_SIZE), length as usize) == 0
                && *slot.add(IMM2_SIZE + length as usize) == 0
            {
                break;
            }
            slot = slot.add((*cb).name_entry_size as usize);
            i += 1;
        }

        /* This should not occur, because this function is called only when we know we
        have duplicate names. Give an internal error. */

        /* LCOV_EXCL_START */
        if i >= (*cb).names_found as u32 {
            *errorcodeptr = ERR53;
            (*cb).erroroffset = name.offset_from((*cb).start_pattern) as PCRE2_SIZE;
            return FALSE;
        }
        /* LCOV_EXCL_STOP */

        /* Record the index and then see how many duplicates there are, updating the
        backref map and maximum back reference as we do. */

        *indexptr = i as c_int;
        let mut count: c_int = 0;

        loop {
            count += 1;
            let groupnumber = get2(slot, 0);
            (*cb).backref_map |= if groupnumber < 32 { 1u32 << groupnumber } else { 1 };
            if groupnumber > (*cb).top_backref {
                (*cb).top_backref = groupnumber;
            }
            i += 1;
            if i >= (*cb).names_found as u32 {
                break;
            }
            slot = slot.add((*cb).name_entry_size as usize);
            if strncmp(name, slot.add(IMM2_SIZE), length as usize) != 0
                || *(slot.add(IMM2_SIZE)).add(length as usize) != 0
            {
                break;
            }
        }

        *countptr = count;
        TRUE
    }
}

/// Exported as `_pcre2_compile_find_dupname_details8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_find_dupname_details8(
    name: PCRE2_SPTR,
    length: u32,
    indexptr: *mut c_int,
    countptr: *mut c_int,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> BOOL {
    unsafe { find_dupname_details(name, length, indexptr, countptr, errorcodeptr, cb) }
}

/* Process the capture list of scan substring and recurse operations. Since at
least one argument must be present, a 0 return value represents error. */
unsafe fn process_capture_list(
    mut pptr: *mut u32,
    mut offset: PCRE2_SIZE,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> usize {
    unsafe {
        let mut size: usize = 0;
        let end = (*cb).named_groups.add((*cb).names_found as usize);

        loop {
            pptr = pptr.add(1);

            match meta_code(*pptr) {
                META_OFFSET => {
                    offset = getplusoffset(&mut pptr);
                    continue;
                }

                META_CAPTURE_NAME => {
                    offset += meta_data(*pptr) as PCRE2_SIZE;
                    pptr = pptr.add(1);
                    let length = *pptr;
                    let name = (*cb).start_pattern.add(offset);

                    let mut ng = find_named_group(name, length, cb);

                    if ng.is_null() {
                        *errorcodeptr = ERR15;
                        (*cb).erroroffset = offset;
                        return 0;
                    }

                    if ((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME) == 0 {
                        *pptr.sub(1) = META_CAPTURE_NUMBER;
                        *pptr.add(0) = (*ng).number;
                        size += 1;
                        continue;
                    }

                    /* Remains only for duplicated names. */
                    *pptr.sub(1) = META_CAPTURE_NAME;
                    *pptr.add(0) = ng.offset_from((*cb).named_groups) as u32;
                    size += 1;
                    let name = (*ng).name;

                    ng = ng.add(1);
                    while ng < end {
                        if (*ng).name == name {
                            size += 1;
                        }
                        ng = ng.add(1);
                    }
                    continue;
                }

                META_CAPTURE_NUMBER => {
                    offset += meta_data(*pptr) as PCRE2_SIZE;

                    pptr = pptr.add(1);
                    let i = *pptr as PCRE2_SIZE;
                    if i > (*cb).bracount as PCRE2_SIZE {
                        *errorcodeptr = ERR15;
                        (*cb).erroroffset = offset;
                        return 0;
                    }
                    if i > (*cb).top_backref as PCRE2_SIZE {
                        (*cb).top_backref = (i as u16) as u32;
                    }
                    size += 1;
                    continue;
                }

                _ => {}
            }

            debug_assert!(size > 0);
            return size;
        }
    }
}

/* Parse the arguments of scan substring operations.

Returns: pointer past the processed args, or NULL on error with an error code
set. */
pub unsafe fn parse_scan_substr_args(
    mut pptr: *mut u32,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
) -> *mut u32 {
    unsafe {
        let mut bit: u8;
        let mut name: PCRE2_SPTR;
        let mut ng: *mut named_group;
        let end = (*cb).named_groups.add((*cb).names_found as usize);
        let mut all_found: BOOL;

        debug_assert!(*pptr == META_OFFSET);
        if process_capture_list(pptr.sub(1), 0, errorcodeptr, cb) == 0 {
            return ptr::null_mut();
        }

        /* Align to bytes. Since the highest capture can be equal to bracount, +1 is
        added before the aligning. */
        let size: usize = (((*cb).bracount + 1 + 7) >> 3) as usize;
        let captures = ((*(*cb).cx).memctl.malloc.unwrap())(size, (*(*cb).cx).memctl.memory_data)
            as *mut u8;
        if captures.is_null() {
            *errorcodeptr = ERR21;
            (*cb).erroroffset = readplusoffset(pptr);
            return ptr::null_mut();
        }

        ptr::write_bytes(captures, 0, size);

        loop {
            match meta_code(*pptr) {
                META_OFFSET => {
                    pptr = pptr.add(1);
                    skipoffset(&mut pptr);
                    continue;
                }

                META_CAPTURE_NAME => {
                    ng = (*cb).named_groups.add(*pptr.add(1) as usize);
                    debug_assert!(((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME) != 0);
                    pptr = pptr.add(2);
                    name = (*ng).name;

                    all_found = TRUE;
                    loop {
                        if (*ng).name != name {
                            ng = ng.add(1);
                            if !(ng < end) {
                                break;
                            }
                            continue;
                        }

                        let capture_ptr = captures.add(((*ng).number >> 3) as usize);
                        debug_assert!(capture_ptr < captures.add(size));
                        bit = (1u32 << ((*ng).number & 0x7)) as u8;

                        if (*capture_ptr & bit) == 0 {
                            *capture_ptr |= bit;
                            all_found = FALSE;
                        }

                        ng = ng.add(1);
                        if !(ng < end) {
                            break;
                        }
                    }

                    if all_found == FALSE {
                        *lengthptr += 1 + 2 * IMM2_SIZE;
                        continue;
                    }

                    *pptr.sub(2) = META_CAPTURE_NUMBER;
                    *pptr.sub(1) = 0;
                    continue;
                }

                META_CAPTURE_NUMBER => {
                    pptr = pptr.add(2);

                    let capture_ptr = captures.add((*pptr.sub(1) >> 3) as usize);
                    debug_assert!(capture_ptr < captures.add(size));
                    bit = (1u32 << (*pptr.sub(1) & 0x7)) as u8;

                    if (*capture_ptr & bit) != 0 {
                        *pptr.sub(1) = 0;
                        continue;
                    }

                    *capture_ptr |= bit;
                    *lengthptr += 1 + IMM2_SIZE;
                    continue;
                }

                _ => {}
            }

            break;
        }

        ((*(*cb).cx).memctl.free.unwrap())(captures as *mut c_void, (*(*cb).cx).memctl.memory_data);
        pptr.sub(1)
    }
}

/// Exported as `_pcre2_compile_parse_scan_substr_args8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_parse_scan_substr_args8(
    pptr: *mut u32,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
) -> *mut u32 {
    unsafe { parse_scan_substr_args(pptr, errorcodeptr, cb, lengthptr) }
}

/* Implement heapsort heapify algorithm. */
unsafe fn do_heapify_u16(captures: *mut u16, size: usize, mut i: usize) {
    unsafe {
        loop {
            let mut max = i;
            let left = (i << 1) + 1;
            let right = left + 1;

            if left < size && *captures.add(left) > *captures.add(max) {
                max = left;
            }
            if right < size && *captures.add(right) > *captures.add(max) {
                max = right;
            }
            if i == max {
                return;
            }

            let tmp = *captures.add(i);
            *captures.add(i) = *captures.add(max);
            *captures.add(max) = tmp;
            i = max;
        }
    }
}

/* Parse the arguments of recurse operations.

Returns: TRUE if OK, FALSE if not, error code set. */
pub unsafe fn parse_recurse_args(
    pptr_start: *mut u32,
    offset: PCRE2_SIZE,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> BOOL {
    unsafe {
        let mut pptr = pptr_start;
        let mut i: usize;
        let size: usize;
        let mut name: PCRE2_SPTR;
        let mut ng: *mut named_group;
        let end = (*cb).named_groups.add((*cb).names_found as usize);

        /* Process all arguments, compute the required size. */

        size = process_capture_list(pptr, offset, errorcodeptr, cb);
        if size == 0 {
            return FALSE;
        }

        let args = ((*(*cb).cx).memctl.malloc.unwrap())(
            core::mem::size_of::<recurse_arguments>() + size * core::mem::size_of::<u16>(),
            (*(*cb).cx).memctl.memory_data,
        ) as *mut recurse_arguments;

        if args.is_null() {
            *errorcodeptr = ERR21;
            (*cb).erroroffset = offset;
            return FALSE;
        }

        (*args).header.next = ptr::null_mut();
        (*args).size = size;

        /* Caching the pre-processed capture list. */
        if !(*cb).last_data.is_null() {
            (*(*cb).last_data).next = &mut (*args).header;
        } else {
            (*cb).first_data = &mut (*args).header;
        }

        (*cb).last_data = &mut (*args).header;

        /* Create the capture list size. */

        let mut captures = args.add(1) as *mut u16;

        loop {
            pptr = pptr.add(1);

            match meta_code(*pptr) {
                META_OFFSET => {
                    skipoffset(&mut pptr);
                    continue;
                }

                META_CAPTURE_NAME => {
                    pptr = pptr.add(1);
                    ng = (*cb).named_groups.add(*pptr as usize);
                    debug_assert!(((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME) != 0);
                    *captures = (*ng).number as u16;
                    captures = captures.add(1);

                    name = (*ng).name;

                    ng = ng.add(1);
                    while ng < end {
                        if (*ng).name == name {
                            *captures = (*ng).number as u16;
                            captures = captures.add(1);
                        }
                        ng = ng.add(1);
                    }
                    continue;
                }

                META_CAPTURE_NUMBER => {
                    pptr = pptr.add(1);
                    *captures = *pptr as u16;
                    captures = captures.add(1);
                    continue;
                }

                _ => {}
            }

            break;
        }

        debug_assert!(size == captures.offset_from(args.add(1) as *mut u16) as usize);
        (*args).skip_size = (pptr.offset_from(pptr_start) as usize) - 1;

        if size == 1 {
            return TRUE;
        }

        /* Sort captures. */

        captures = args.add(1) as *mut u16;
        i = (size >> 1) - 1;
        loop {
            do_heapify_u16(captures, size, i);
            if i == 0 {
                break;
            }
            i -= 1;
        }

        i = size - 1;
        while i > 0 {
            let tmp = *captures.add(0);
            *captures.add(0) = *captures.add(i);
            *captures.add(i) = tmp;

            do_heapify_u16(captures, i, 0);
            i -= 1;
        }

        /* Remove duplicates. */

        let captures_end = captures.add(size);
        let mut tmp = *captures;
        captures = captures.add(1);
        let mut current = captures;

        while current < captures_end {
            if *current != tmp {
                tmp = *current;
                *captures = tmp;
                captures = captures.add(1);
            }

            current = current.add(1);
        }

        (*args).size = captures.offset_from(args.add(1) as *mut u16) as usize;
        TRUE
    }
}

/// Exported as `_pcre2_compile_parse_recurse_args8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_parse_recurse_args8(
    pptr_start: *mut u32,
    offset: PCRE2_SIZE,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> BOOL {
    unsafe { parse_recurse_args(pptr_start, offset, errorcodeptr, cb) }
}

/* End of pcre2_compile_cgroup.c */
