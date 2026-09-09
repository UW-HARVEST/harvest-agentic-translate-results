//! Translated from pcre2_compile_cgroup.c.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use core::ffi::{c_char, c_void};

use crate::string_utils::_pcre2_strncmp_8;

/* NAMED_GROUP_GET_HASH(ng) */
macro_rules! NAMED_GROUP_GET_HASH {
    ($ng:expr) => {
        ((*$ng).hash_dup & NAMED_GROUP_HASH_MASK)
    };
}

/* Local replacement for C's memcmp(). */
pub(crate) unsafe fn cgroup_memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i: usize = 0;
    while i < n {
        let c1 = *s1.add(i) as i32;
        let c2 = *s2.add(i) as i32;
        if c1 != c2 {
            return c1 - c2;
        }
        i += 1;
    }
    0
}

/*************************************************
*   Compute the hash code from a capture name    *
*************************************************/

/* This function returns with a simple hash code
computed from the name of a capture group.

Arguments:
  name         name of the capture group
  length       the length of the name

Returns:       hash code
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_get_hash_from_name8(name: PCRE2_SPTR, length: u32) -> u16 {
    let hash: u16;

    /* PCRE2_ASSERT(length > 0); */

    hash = (((*name.add(0) as i32) & 0x7f)
        | (((*name.add(length.wrapping_sub(1) as usize) as i32) & 0xff) << 7)) as u16;
    /* PCRE2_ASSERT(hash <= NAMED_GROUP_HASH_MASK); */
    hash
}


/*************************************************
*   Get the descriptor of a known named capture  *
*************************************************/

/* This function returns the descriptor in the
named group list of a known capture group.

Arguments:
  name         name of the capture group
  length       the length of the name

Returns:       pointer to the descriptor when found,
               NULL otherwise
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_find_named_group8(
    name: PCRE2_SPTR,
    length: u32,
    cb: *mut compile_block,
) -> *mut named_group {
    let hash: u16 = _pcre2_compile_get_hash_from_name8(name, length);
    let mut ng: *mut named_group;
    let end: *mut named_group = (*cb).named_groups.add((*cb).names_found as usize);

    ng = (*cb).named_groups;
    while ng < end {
        if length == (*ng).length as u32
            && hash == NAMED_GROUP_GET_HASH!(ng)
            && _pcre2_strncmp_8(name, (*ng).name, length as usize) == 0
        {
            return ng;
        }
        ng = ng.add(1);
    }

    core::ptr::null_mut()
}


/*************************************************
*     Add an entry to the name/number table      *
*************************************************/

/* This function is called between compiling passes to add an entry to the
name/number table, maintaining alphabetical order. Checking for permitted
and forbidden duplicates has already been done.

Arguments:
  cb           the compile data block
  nb           named group entry
  tablecount   the count of names in the table so far

Returns:       new tablecount
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_add_name_to_table8(
    cb: *mut compile_block,
    ng: *mut named_group,
    tablecount: u32,
) -> u32 {
    let mut ng = ng;
    let mut tablecount = tablecount;
    let mut i: u32;
    let name: PCRE2_SPTR = (*ng).name;
    let length: i32 = (*ng).length as i32;
    let mut duplicate_count: u32 = 1;

    let mut slot: *mut PCRE2_UCHAR = (*cb).name_table;

    /* PCRE2_ASSERT(length > 0); */

    if ((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME) != 0 {
        let mut ng_it: *mut named_group;
        let end: *mut named_group = (*cb).named_groups.add((*cb).names_found as usize);

        ng_it = ng.add(1);
        while ng_it < end {
            if (*ng_it).name == name {
                duplicate_count += 1;
            }
            ng_it = ng_it.add(1);
        }
    }

    i = 0;
    while i < tablecount {
        let mut crc: i32 = cgroup_memcmp(name, slot.add(IMM2_SIZE), CU2BYTES!(length) as usize);
        if crc == 0 && *slot.add(IMM2_SIZE + length as usize) != 0 {
            crc = -1; /* Current name is a substring */
        }

        /* Make space in the table and break the loop for an earlier name. For a
        duplicate or later name, carry on. We do this for duplicates so that in the
        simple case (when ?(| is not used) they are in order of their numbers. In all
        cases they are in the order in which they appear in the pattern. */

        if crc < 0 {
            core::ptr::copy(
                slot as *const u8,
                slot.add(((*cb).name_entry_size as u32 * duplicate_count) as usize),
                CU2BYTES!((tablecount - i) * (*cb).name_entry_size as u32) as usize,
            );
            break;
        }

        /* Continue the loop for a later or duplicate name */

        slot = slot.add((*cb).name_entry_size as usize);
        i += 1;
    }

    tablecount += duplicate_count;

    while TRUE != 0 {
        PUT2!(slot, 0, (*ng).number);
        core::ptr::copy_nonoverlapping(
            name as *const u8,
            slot.add(IMM2_SIZE) as *mut u8,
            CU2BYTES!(length) as usize,
        );

        /* Add a terminating zero and fill the rest of the slot with zeroes so that
        the memory is all initialized. Otherwise valgrind moans about uninitialized
        memory when saving serialized compiled patterns. */

        core::ptr::write_bytes(
            slot.add(IMM2_SIZE + length as usize) as *mut u8,
            0u8,
            CU2BYTES!((*cb).name_entry_size as i32 - length - IMM2_SIZE as i32) as usize,
        );

        duplicate_count -= 1;
        if duplicate_count == 0 {
            break;
        }

        while TRUE != 0 {
            ng = ng.add(1);
            if (*ng).name == name {
                break;
            }
        }

        slot = slot.add((*cb).name_entry_size as usize);
    }

    tablecount
}


/*************************************************
*    Find details of duplicate group names       *
*************************************************/

/* This is called from compile_branch() when it needs to know the index and
count of duplicates in the names table when processing named backreferences,
either directly, or as conditions.

Arguments:
  name          points to the name
  length        the length of the name
  indexptr      where to put the index
  countptr      where to put the count of duplicates
  errorcodeptr  where to put an error code
  cb            the compile block

Returns:        TRUE if OK, FALSE if not, error code set
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_find_dupname_details8(
    name: PCRE2_SPTR,
    length: u32,
    indexptr: *mut i32,
    countptr: *mut i32,
    errorcodeptr: *mut i32,
    cb: *mut compile_block,
) -> BOOL {
    let mut i: u32;
    let mut groupnumber: u32;
    let mut count: i32;
    let mut slot: *mut PCRE2_UCHAR = (*cb).name_table;

    /* Find the first entry in the table */

    i = 0;
    while i < (*cb).names_found as u32 {
        if _pcre2_strncmp_8(name, slot.add(IMM2_SIZE) as PCRE2_SPTR, length as usize) == 0
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
        /* PCRE2_DEBUG_UNREACHABLE(); */
        *errorcodeptr = ERR53;
        (*cb).erroroffset = name.offset_from((*cb).start_pattern) as usize;
        return FALSE;
    }
    /* LCOV_EXCL_STOP */

    /* Record the index and then see how many duplicates there are, updating the
    backref map and maximum back reference as we do. */

    *indexptr = i as i32;
    count = 0;

    loop {
        count += 1;
        groupnumber = GET2!(slot, 0);
        (*cb).backref_map |= if groupnumber < 32 { 1u32 << groupnumber } else { 1 };
        if groupnumber > (*cb).top_backref {
            (*cb).top_backref = groupnumber;
        }
        i += 1;
        if i >= (*cb).names_found as u32 {
            break;
        }
        slot = slot.add((*cb).name_entry_size as usize);
        if _pcre2_strncmp_8(name, slot.add(IMM2_SIZE) as PCRE2_SPTR, length as usize) != 0
            || *slot.add(IMM2_SIZE).add(length as usize) != 0
        {
            break;
        }
    }

    *countptr = count;
    TRUE
}


/* Process the capture list of scan substring and recurse
operations. Since at least one argument must be present,
a 0 return value represents error. */

pub(crate) unsafe fn compile_process_capture_list(
    pptr: *mut u32,
    offset: PCRE2_SIZE,
    errorcodeptr: *mut i32,
    cb: *mut compile_block,
) -> usize {
    let mut pptr = pptr;
    let mut offset = offset;
    let mut i: usize;
    let mut size: usize = 0;
    let mut ng: *mut named_group;
    let mut name: PCRE2_SPTR;
    let mut length: u32;
    let end: *mut named_group = (*cb).named_groups.add((*cb).names_found as usize);

    loop {
        pptr = pptr.add(1);

        match META_CODE!(*pptr) {
            META_OFFSET => {
                GETPLUSOFFSET!(offset, pptr);
                continue;
            }

            META_CAPTURE_NAME => {
                offset += META_DATA!(*pptr) as usize;
                pptr = pptr.add(1);
                length = *pptr;
                name = (*cb).start_pattern.add(offset);

                ng = _pcre2_compile_find_named_group8(name, length, cb);

                if ng.is_null() {
                    *errorcodeptr = ERR15;
                    (*cb).erroroffset = offset;
                    return 0;
                }

                if ((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME) == 0 {
                    *pptr.offset(-1) = META_CAPTURE_NUMBER;
                    *pptr.add(0) = (*ng).number;
                    size += 1;
                    continue;
                }

                /* Remains only for duplicated names. */
                *pptr.offset(-1) = META_CAPTURE_NAME;
                *pptr.add(0) = ng.offset_from((*cb).named_groups) as u32;
                size += 1;
                name = (*ng).name;

                loop {
                    ng = ng.add(1);
                    if !(ng < end) {
                        break;
                    }
                    if (*ng).name == name {
                        size += 1;
                    }
                }
                continue;
            }

            META_CAPTURE_NUMBER => {
                offset += META_DATA!(*pptr) as usize;

                pptr = pptr.add(1);
                i = *pptr as usize;
                if i > (*cb).bracount as usize {
                    *errorcodeptr = ERR15;
                    (*cb).erroroffset = offset;
                    return 0;
                }
                if i > (*cb).top_backref as usize {
                    (*cb).top_backref = (i as u16) as u32;
                }
                size += 1;
                continue;
            }

            _ => {}
        }

        /* PCRE2_ASSERT(size > 0); */
        return size;
    }
}


/*******************************************************
*   Parse the arguments of scan substring operations   *
********************************************************/

/* This function parses the arguments of scan substring operations.

Arguments:
  pptr_start    points to the current parsed pattern pointer
  offset        argument starting offset in the pattern
  errorcodeptr  where to put an error code
  cb            the compile block
  lengthptr     NULL during the real compile phase
                points to length accumulator during pre-compile phase

Returns:        TRUE if OK, FALSE if not, error code set
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_parse_scan_substr_args8(
    pptr: *mut u32,
    errorcodeptr: *mut i32,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
) -> *mut u32 {
    let mut pptr = pptr;
    let captures: *mut u8;
    let mut capture_ptr: *mut u8;
    let mut bit: u8;
    let mut name: PCRE2_SPTR;
    let mut ng: *mut named_group;
    let end: *mut named_group = (*cb).named_groups.add((*cb).names_found as usize);
    let mut all_found: BOOL;
    let size: usize;

    /* PCRE2_ASSERT(*pptr == META_OFFSET); */
    if compile_process_capture_list(pptr.offset(-1), 0, errorcodeptr, cb) == 0 {
        return core::ptr::null_mut();
    }

    /* Align to bytes. Since the highest capture can
    be equal to bracount, +1 is added before the aligning. */
    size = (((*cb).bracount + 1 + 7) >> 3) as usize;
    captures = ((*(*cb).cx).memctl.malloc.unwrap())(size, (*(*cb).cx).memctl.memory_data) as *mut u8;
    if captures.is_null() {
        *errorcodeptr = ERR21;
        READPLUSOFFSET!((*cb).erroroffset, pptr);
        return core::ptr::null_mut();
    }

    core::ptr::write_bytes(captures, 0u8, size);

    loop {
        match META_CODE!(*pptr) {
            META_OFFSET => {
                pptr = pptr.add(1);
                SKIPOFFSET!(pptr);
                continue;
            }

            META_CAPTURE_NAME => {
                ng = (*cb).named_groups.add(*pptr.add(1) as usize);
                /* PCRE2_ASSERT((ng->hash_dup & NAMED_GROUP_IS_DUPNAME) != 0); */
                pptr = pptr.add(2);
                name = (*ng).name;

                all_found = TRUE;
                loop {
                    'cont: {
                        if (*ng).name != name {
                            break 'cont; /* goto do-while condition */
                        }

                        capture_ptr = captures.add(((*ng).number >> 3) as usize);
                        /* PCRE2_ASSERT(capture_ptr < captures + size); */
                        bit = (1u32 << ((*ng).number & 0x7)) as u8;

                        if (*capture_ptr & bit) == 0 {
                            *capture_ptr |= bit;
                            all_found = FALSE;
                        }
                    }
                    ng = ng.add(1);
                    if !(ng < end) {
                        break;
                    }
                }

                if all_found == 0 {
                    *lengthptr += 1 + 2 * IMM2_SIZE;
                    continue;
                }

                *pptr.offset(-2) = META_CAPTURE_NUMBER;
                *pptr.offset(-1) = 0;
                continue;
            }

            META_CAPTURE_NUMBER => {
                pptr = pptr.add(2);

                capture_ptr = captures.add((*pptr.offset(-1) >> 3) as usize);
                /* PCRE2_ASSERT(capture_ptr < captures + size); */
                bit = (1u32 << (*pptr.offset(-1) & 0x7)) as u8;

                if (*capture_ptr & bit) != 0 {
                    *pptr.offset(-1) = 0;
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
    pptr.offset(-1)
}


/* Implement heapsort heapify algorithm. */

pub(crate) unsafe fn do_heapify_u16(captures: *mut u16, size: usize, i: usize) {
    let mut i = i;
    let mut max: usize;
    let mut left: usize;
    let mut right: usize;
    let mut tmp: u16;

    while TRUE != 0 {
        max = i;
        left = (i << 1) + 1;
        right = left + 1;

        if left < size && *captures.add(left) > *captures.add(max) {
            max = left;
        }
        if right < size && *captures.add(right) > *captures.add(max) {
            max = right;
        }
        if i == max {
            return;
        }

        tmp = *captures.add(i);
        *captures.add(i) = *captures.add(max);
        *captures.add(max) = tmp;
        i = max;
    }
}


/*************************************************
*   Parse the arguments of recurse operations    *
*************************************************/

/* This function parses the arguments of recurse operations.

Arguments:
  pptr_start    the current parsed pattern pointer
  offset        argument starting offset in the pattern
  errorcodeptr  where to put an error code
  cb            the compile block

Returns:        TRUE if OK, FALSE if not, error code set
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_parse_recurse_args8(
    pptr_start: *mut u32,
    offset: PCRE2_SIZE,
    errorcodeptr: *mut i32,
    cb: *mut compile_block,
) -> BOOL {
    let mut pptr: *mut u32 = pptr_start;
    let mut i: usize;
    let mut size: usize;
    let mut name: PCRE2_SPTR;
    let mut ng: *mut named_group;
    let end: *mut named_group = (*cb).named_groups.add((*cb).names_found as usize);
    let args: *mut recurse_arguments;
    let mut captures: *mut u16;
    let mut current: *mut u16;
    let captures_end: *mut u16;
    let mut tmp: u16;

    /* Process all arguments, compute the required size. */

    size = compile_process_capture_list(pptr, offset, errorcodeptr, cb);
    if size == 0 {
        return FALSE;
    }

    args = ((*(*cb).cx).memctl.malloc.unwrap())(
        core::mem::size_of::<recurse_arguments>() + size * core::mem::size_of::<u16>(),
        (*(*cb).cx).memctl.memory_data,
    ) as *mut recurse_arguments;

    if args.is_null() {
        *errorcodeptr = ERR21;
        (*cb).erroroffset = offset;
        return FALSE;
    }

    (*args).header.next = core::ptr::null_mut();
    (*args).size = size;

    /* Caching the pre-processed capture list. */
    if !(*cb).last_data.is_null() {
        (*(*cb).last_data).next = &mut (*args).header;
    } else {
        (*cb).first_data = &mut (*args).header;
    }

    (*cb).last_data = &mut (*args).header;

    /* Create the capture list size. */

    captures = args.add(1) as *mut u16;

    loop {
        pptr = pptr.add(1);

        match META_CODE!(*pptr) {
            META_OFFSET => {
                SKIPOFFSET!(pptr);
                continue;
            }

            META_CAPTURE_NAME => {
                pptr = pptr.add(1);
                ng = (*cb).named_groups.add(*pptr as usize);
                /* PCRE2_ASSERT((ng->hash_dup & NAMED_GROUP_IS_DUPNAME) != 0); */
                *captures = (*ng).number as u16;
                captures = captures.add(1);

                name = (*ng).name;

                loop {
                    ng = ng.add(1);
                    if !(ng < end) {
                        break;
                    }
                    if (*ng).name == name {
                        *captures = (*ng).number as u16;
                        captures = captures.add(1);
                    }
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

    /* PCRE2_ASSERT(size == (size_t)(captures - (uint16_t*)(args + 1))); */
    (*args).skip_size = (pptr.offset_from(pptr_start) as usize) - 1;

    if size == 1 {
        return TRUE;
    }

    /* Sort captures. */

    captures = args.add(1) as *mut u16;
    i = (size >> 1) - 1;
    while TRUE != 0 {
        do_heapify_u16(captures, size, i);
        if i == 0 {
            break;
        }
        i -= 1;
    }

    i = size - 1;
    while i > 0 {
        tmp = *captures.add(0);
        *captures.add(0) = *captures.add(i);
        *captures.add(i) = tmp;

        do_heapify_u16(captures, i, 0);
        i -= 1;
    }

    /* Remove duplicates. */

    captures_end = captures.add(size);
    tmp = *captures;
    captures = captures.add(1);
    current = captures;

    while current < captures_end {
        if *current != tmp {
            tmp = *current;
            *captures = tmp;
            captures = captures.add(1);
        }

        current = current.add(1);
    }

    (*args).size = (captures as usize - (args.add(1) as *mut u16) as usize)
        / core::mem::size_of::<u16>();
    TRUE
}

/* End of pcre2_compile_cgroup.c */
