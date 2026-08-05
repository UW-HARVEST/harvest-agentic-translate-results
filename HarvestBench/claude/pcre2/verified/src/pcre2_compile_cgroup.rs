use crate::pcre2_internal::*;
use crate::pcre2_string_utils::_pcre2_strncmp_8;
use core::ffi::{c_int, c_void};
use core::ptr;

extern "C" {
    fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> c_int;
}

#[inline]
fn NAMED_GROUP_GET_HASH(ng: *const named_group) -> u16 {
    unsafe { (*ng).hash_dup & NAMED_GROUP_HASH_MASK }
}

/*************************************************
*   Compute the hash code from a capture name    *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_get_hash_from_name8(
    name: PCRE2_SPTR,
    length: u32,
) -> u16 {
    // PCRE2_ASSERT(length > 0);
    let hash: u16 = ((*name.add(0) as u16 & 0x7f)
        | ((*name.add((length - 1) as usize) as u16 & 0xff) << 7)) as u16;
    // PCRE2_ASSERT(hash <= NAMED_GROUP_HASH_MASK);
    hash
}

/*************************************************
*   Get the descriptor of a known named capture  *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_find_named_group8(
    name: PCRE2_SPTR,
    length: u32,
    cb: *mut compile_block,
) -> *mut named_group {
    let hash = _pcre2_compile_get_hash_from_name8(name, length);
    let end = (*cb).named_groups.add((*cb).names_found as usize);

    let mut ng = (*cb).named_groups;
    while ng < end {
        if length == (*ng).length as u32
            && hash == NAMED_GROUP_GET_HASH(ng)
            && _pcre2_strncmp_8(name, (*ng).name, length as usize) == 0
        {
            return ng;
        }
        ng = ng.add(1);
    }

    ptr::null_mut()
}

/*************************************************
*     Add an entry to the name/number table      *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_add_name_to_table8(
    cb: *mut compile_block,
    mut ng: *mut named_group,
    mut tablecount: u32,
) -> u32 {
    let name: PCRE2_SPTR = (*ng).name;
    let length: c_int = (*ng).length as c_int;
    let mut duplicate_count: u32 = 1;

    let mut slot: *mut PCRE2_UCHAR = (*cb).name_table;

    // PCRE2_ASSERT(length > 0);

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
            CU2BYTES(length as usize),
        );
        if crc == 0 && *slot.add(IMM2_SIZE + length as usize) != 0 {
            crc = -1; /* Current name is a substring */
        }

        /* Make space in the table and break the loop for an earlier name. */

        if crc < 0 {
            memmove(
                slot.add((*cb).name_entry_size as usize * duplicate_count as usize) as *mut c_void,
                slot as *const c_void,
                CU2BYTES(((tablecount - i) * (*cb).name_entry_size as u32) as usize),
            );
            break;
        }

        /* Continue the loop for a later or duplicate name */

        slot = slot.add((*cb).name_entry_size as usize);
        i += 1;
    }

    tablecount += duplicate_count;

    loop {
        PUT2(slot, 0, (*ng).number);
        memcpy(
            slot.add(IMM2_SIZE) as *mut c_void,
            name as *const c_void,
            CU2BYTES(length as usize),
        );

        /* Add a terminating zero and fill the rest of the slot with zeroes. */

        memset(
            slot.add(IMM2_SIZE + length as usize) as *mut c_void,
            0,
            CU2BYTES(((*cb).name_entry_size as c_int - length - IMM2_SIZE as c_int) as usize),
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

/*************************************************
*    Find details of duplicate group names       *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_find_dupname_details8(
    name: PCRE2_SPTR,
    length: u32,
    indexptr: *mut c_int,
    countptr: *mut c_int,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> BOOL {
    let mut slot: *mut PCRE2_UCHAR = (*cb).name_table;

    /* Find the first entry in the table */

    let mut i: u32 = 0;
    while i < (*cb).names_found as u32 {
        if _pcre2_strncmp_8(name, slot.add(IMM2_SIZE), length as usize) == 0
            && *slot.add(IMM2_SIZE + length as usize) == 0
        {
            break;
        }
        slot = slot.add((*cb).name_entry_size as usize);
        i += 1;
    }

    /* This should not occur. Give an internal error. */

    if i >= (*cb).names_found as u32 {
        *errorcodeptr = ERR53;
        (*cb).erroroffset = name.offset_from((*cb).start_pattern) as PCRE2_SIZE;
        return FALSE;
    }

    /* Record the index and then see how many duplicates there are. */

    *indexptr = i as c_int;
    let mut count: c_int = 0;

    loop {
        count += 1;
        let groupnumber = GET2(slot, 0);
        (*cb).backref_map |= if groupnumber < 32 {
            1u32 << groupnumber
        } else {
            1
        };
        if groupnumber > (*cb).top_backref {
            (*cb).top_backref = groupnumber;
        }
        i += 1;
        if i >= (*cb).names_found as u32 {
            break;
        }
        slot = slot.add((*cb).name_entry_size as usize);
        if _pcre2_strncmp_8(name, slot.add(IMM2_SIZE), length as usize) != 0
            || *slot.add(IMM2_SIZE).add(length as usize) != 0
        {
            break;
        }
    }

    *countptr = count;
    TRUE
}

/* Process the capture list of scan substring and recurse operations. Since at
least one argument must be present, a 0 return value represents error. */

unsafe fn compile_process_capture_list(
    mut pptr: *mut u32,
    mut offset: PCRE2_SIZE,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> usize {
    let mut size: usize = 0;
    let end = (*cb).named_groups.add((*cb).names_found as usize);

    loop {
        pptr = pptr.add(1);

        match META_CODE(*pptr) {
            META_OFFSET => {
                // GETPLUSOFFSET(offset, pptr)
                pptr = pptr.add(1);
                offset = *pptr as PCRE2_SIZE;
                continue;
            }

            META_CAPTURE_NAME => {
                offset += META_DATA(*pptr) as PCRE2_SIZE;
                pptr = pptr.add(1);
                let length = *pptr;
                let name = (*cb).start_pattern.add(offset);

                let ng = _pcre2_compile_find_named_group8(name, length, cb);

                if ng.is_null() {
                    *errorcodeptr = ERR15;
                    (*cb).erroroffset = offset;
                    return 0;
                }

                if ((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME) == 0 {
                    *pptr.offset(-1) = META_CAPTURE_NUMBER;
                    *pptr.offset(0) = (*ng).number;
                    size += 1;
                    continue;
                }

                /* Remains only for duplicated names. */
                *pptr.offset(-1) = META_CAPTURE_NAME;
                *pptr.offset(0) = ng.offset_from((*cb).named_groups) as u32;
                size += 1;
                let name2 = (*ng).name;

                let mut ng_it = ng;
                loop {
                    ng_it = ng_it.add(1);
                    if !(ng_it < end) {
                        break;
                    }
                    if (*ng_it).name == name2 {
                        size += 1;
                    }
                }
                continue;
            }

            META_CAPTURE_NUMBER => {
                offset += META_DATA(*pptr) as PCRE2_SIZE;

                pptr = pptr.add(1);
                let i = *pptr;
                if i > (*cb).bracount {
                    *errorcodeptr = ERR15;
                    (*cb).erroroffset = offset;
                    return 0;
                }
                if i > (*cb).top_backref {
                    (*cb).top_backref = (i as u16) as u32;
                }
                size += 1;
                continue;
            }

            _ => {}
        }

        // PCRE2_ASSERT(size > 0);
        return size;
    }
}

/*******************************************************
*   Parse the arguments of scan substring operations   *
********************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_parse_scan_substr_args8(
    mut pptr: *mut u32,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
) -> *mut u32 {
    let end = (*cb).named_groups.add((*cb).names_found as usize);

    // PCRE2_ASSERT(*pptr == META_OFFSET);
    if compile_process_capture_list(pptr.offset(-1), 0, errorcodeptr, cb) == 0 {
        return ptr::null_mut();
    }

    /* Align to bytes. */
    let size: usize = (((*cb).bracount + 1 + 7) >> 3) as usize;
    let captures = ((*(*cb).cx).memctl.malloc.unwrap())(size, (*(*cb).cx).memctl.memory_data)
        as *mut u8;
    if captures.is_null() {
        *errorcodeptr = ERR21;
        // READPLUSOFFSET(cb->erroroffset, pptr)
        (*cb).erroroffset = *pptr.add(1) as PCRE2_SIZE;
        return ptr::null_mut();
    }

    memset(captures as *mut c_void, 0, size);

    loop {
        match META_CODE(*pptr) {
            META_OFFSET => {
                pptr = pptr.add(1);
                // SKIPOFFSET(pptr)
                pptr = pptr.add(1);
                continue;
            }

            META_CAPTURE_NAME => {
                let mut ng = (*cb).named_groups.add(*pptr.add(1) as usize);
                // PCRE2_ASSERT((ng->hash_dup & NAMED_GROUP_IS_DUPNAME) != 0);
                pptr = pptr.add(2);
                let name = (*ng).name;

                let mut all_found: BOOL = TRUE;
                loop {
                    if (*ng).name == name {
                        let capture_ptr = captures.add(((*ng).number >> 3) as usize);
                        // PCRE2_ASSERT(capture_ptr < captures + size);
                        let bit = (1u32 << ((*ng).number & 0x7)) as u8;

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

                if all_found == FALSE {
                    *lengthptr += 1 + 2 * IMM2_SIZE;
                    continue;
                }

                *pptr.offset(-2) = META_CAPTURE_NUMBER;
                *pptr.offset(-1) = 0;
                continue;
            }

            META_CAPTURE_NUMBER => {
                pptr = pptr.add(2);

                let capture_ptr = captures.add((*pptr.offset(-1) >> 3) as usize);
                // PCRE2_ASSERT(capture_ptr < captures + size);
                let bit = (1u32 << (*pptr.offset(-1) & 0x7)) as u8;

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

unsafe fn do_heapify_u16(captures: *mut u16, size: usize, mut i: usize) {
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

/*************************************************
*   Parse the arguments of recurse operations    *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_parse_recurse_args8(
    pptr_start: *mut u32,
    offset: PCRE2_SIZE,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> BOOL {
    let mut pptr = pptr_start;
    let end = (*cb).named_groups.add((*cb).names_found as usize);

    /* Process all arguments, compute the required size. */

    let size = compile_process_capture_list(pptr, offset, errorcodeptr, cb);
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
        (*(*cb).last_data).next = &mut (*args).header as *mut compile_data;
    } else {
        (*cb).first_data = &mut (*args).header as *mut compile_data;
    }

    (*cb).last_data = &mut (*args).header as *mut compile_data;

    /* Create the capture list. */

    let base: *mut u16 = args.add(1) as *mut u16;
    let mut captures: *mut u16 = base;

    loop {
        pptr = pptr.add(1);

        match META_CODE(*pptr) {
            META_OFFSET => {
                // SKIPOFFSET(pptr)
                pptr = pptr.add(1);
                continue;
            }

            META_CAPTURE_NAME => {
                pptr = pptr.add(1);
                let mut ng = (*cb).named_groups.add(*pptr as usize);
                // PCRE2_ASSERT((ng->hash_dup & NAMED_GROUP_IS_DUPNAME) != 0);
                *captures = (*ng).number as u16;
                captures = captures.add(1);

                let name = (*ng).name;

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

    // PCRE2_ASSERT(size == (size_t)(captures - base));
    (*args).skip_size = (pptr.offset_from(pptr_start) as usize) - 1;

    if size == 1 {
        return TRUE;
    }

    /* Sort captures. */

    let mut captures: *mut u16 = base;
    let mut i: usize = (size >> 1) - 1;
    loop {
        do_heapify_u16(captures, size, i);
        if i == 0 {
            break;
        }
        i -= 1;
    }

    let mut i: usize = size - 1;
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

    (*args).size = captures.offset_from(base) as usize;
    TRUE
}

/* End of pcre2_compile_cgroup.rs */
