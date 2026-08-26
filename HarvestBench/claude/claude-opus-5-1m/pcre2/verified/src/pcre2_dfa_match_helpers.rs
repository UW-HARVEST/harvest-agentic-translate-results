/*************************************************
*               Process a callout                *
*************************************************/

/* This function is called to perform a callout.

Arguments:
  code              current code pointer
  offsets           points to current capture offsets
  current_subject   start of current subject match
  ptr               current position in subject
  mb                the match block
  extracode         extra code offset when called from condition
  lengthptr         where to return the callout length

Returns:            the return from the callout
*/

unsafe fn do_callout_dfa(
    code: PCRE2_SPTR,
    offsets: *mut PCRE2_SIZE,
    current_subject: PCRE2_SPTR,
    ptr: PCRE2_SPTR,
    mb: *mut dfa_match_block,
    extracode: PCRE2_SIZE,
    lengthptr: *mut PCRE2_SIZE,
) -> c_int {
    let cb: *mut pcre2_callout_block = (*mb).cb;

    *lengthptr = if *code.add(extracode) as u32 == OP_CALLOUT {
        *_pcre2_OP_lengths_8.as_ptr().add(OP_CALLOUT as usize) as PCRE2_SIZE
    } else {
        GET!(code, 1 + 2 * LINK_SIZE + extracode) as PCRE2_SIZE
    };

    if (*mb).callout.is_none() {
        return 0;
    } /* No callout provided */

    /* Fixed fields in the callout block are set once and for all at the start of
    matching. */

    (*cb).offset_vector = offsets;
    (*cb).start_match = current_subject.offset_from((*mb).start_subject) as PCRE2_SIZE;
    (*cb).current_position = ptr.offset_from((*mb).start_subject) as PCRE2_SIZE;
    (*cb).pattern_position = GET!(code, 1 + extracode) as PCRE2_SIZE;
    (*cb).next_item_length = GET!(code, 1 + LINK_SIZE + extracode) as PCRE2_SIZE;

    if *code.add(extracode) as u32 == OP_CALLOUT {
        (*cb).callout_number = *code.add(1 + 2 * LINK_SIZE + extracode) as u32;
        (*cb).callout_string_offset = 0;
        (*cb).callout_string = core::ptr::null();
        (*cb).callout_string_length = 0;
    } else {
        (*cb).callout_number = 0;
        (*cb).callout_string_offset = GET!(code, 1 + 3 * LINK_SIZE + extracode) as PCRE2_SIZE;
        (*cb).callout_string = code.add(1 + 4 * LINK_SIZE + extracode).add(1);
        (*cb).callout_string_length = (*lengthptr)
            .wrapping_sub(1 + 4 * LINK_SIZE)
            .wrapping_sub(2);
    }

    ((*mb).callout.unwrap())(cb, (*mb).callout_data)
}

/*************************************************
*         Expand local workspace memory          *
*************************************************/

/* This function is called when internal_dfa_match() is about to be called
recursively and there is insufficient working space left in the current
workspace block. If there's an existing next block, use it; otherwise get a new
block unless the heap limit is reached.

Arguments:
  rwsptr     pointer to block pointer (updated)
  ovecsize   space needed for an ovector
  mb         the match block

Returns:     0 rwsptr has been updated
            !0 an error code
*/

unsafe fn more_workspace(
    rwsptr: *mut *mut RWS_anchor,
    ovecsize: c_uint,
    mb: *mut dfa_match_block,
) -> c_int {
    let rws: *mut RWS_anchor = *rwsptr;
    let new: *mut RWS_anchor;

    if !(*rws).next.is_null() {
        new = (*rws).next;
    }
    /* Sizes in the RWS_anchor blocks are in units of sizeof(int), but
    mb->heap_limit and mb->heap_used are in kibibytes. Play carefully, to avoid
    overflow. */
    else {
        let mut newsize: u32 =
            if (*rws).size as usize >= (u32::MAX as usize) / (size_of::<c_int>() * 2) {
                ((u32::MAX as usize) / size_of::<c_int>()) as u32
            } else {
                (*rws).size.wrapping_mul(2)
            };
        let mut newsizeK: u32 = (newsize as usize / (1024 / size_of::<c_int>())) as u32;

        if (newsizeK as usize).wrapping_add((*mb).heap_used) > (*mb).heap_limit as usize {
            newsizeK = ((*mb).heap_limit as usize).wrapping_sub((*mb).heap_used) as u32;
        }
        newsize = (newsizeK as usize).wrapping_mul(1024 / size_of::<c_int>()) as u32;

        if (newsize as usize) < RWS_RSIZE + ovecsize as usize + RWS_ANCHOR_SIZE {
            return PCRE2_ERROR_HEAPLIMIT;
        }
        new = ((*mb).memctl.malloc.unwrap())(
            newsize as usize * size_of::<c_int>(),
            (*mb).memctl.memory_data,
        ) as *mut RWS_anchor;
        if new.is_null() {
            return PCRE2_ERROR_NOMEMORY;
        }
        (*mb).heap_used = (*mb).heap_used.wrapping_add(newsizeK as usize);
        (*new).next = core::ptr::null_mut();
        (*new).size = newsize;
        (*rws).next = new;
    }

    (*new).free = ((*new).size as usize).wrapping_sub(RWS_ANCHOR_SIZE) as u32;
    *rwsptr = new;
    0
}
