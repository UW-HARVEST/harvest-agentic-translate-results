/* Translated from c_src/src/pcre2_compile.c lines 10279-11350 */

/*************************************************
*     External function to compile a pattern     *
*************************************************/

/* This function reads a regular expression in the form of a string and returns
a pointer to a block of store holding a compiled version of the expression.

Arguments:
  pattern       the regular expression
  patlen        the length of the pattern, or PCRE2_ZERO_TERMINATED
  options       option bits
  errorptr      pointer to errorcode
  erroroffset   pointer to error offset
  ccontext      points to a compile context or is NULL

Returns:        pointer to compiled data block, or NULL on error,
                with errorcode and erroroffset set
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_compile_8(
    pattern: PCRE2_SPTR,
    patlen: PCRE2_SIZE,
    options: u32,
    errorptr: *mut c_int,
    erroroffset: *mut PCRE2_SIZE,
    ccontext: *mut pcre2_real_compile_context,
) -> *mut pcre2_real_code {
    /* Scan-cache size used by the recursion/subroutine offset fixup below. */
    const RSCAN_CACHE_SIZE: c_int = 8;

    let mut pattern: PCRE2_SPTR = pattern;
    let mut patlen: PCRE2_SIZE = patlen;
    let mut options: u32 = options;
    let mut ccontext: *mut pcre2_real_compile_context = ccontext;

    let mut utf: BOOL = FALSE; /* Set TRUE for UTF mode */
    let mut ucp: BOOL = FALSE; /* Set TRUE for UCP mode */
    let mut has_lookbehind: BOOL = FALSE; /* Set TRUE if a lookbehind is found */
    let zero_terminated: BOOL; /* Set TRUE for zero-terminated pattern */
    let mut re: *mut pcre2_real_code = std::ptr::null_mut(); /* What we will return */
    let mut cb: compile_block = std::mem::zeroed(); /* "Static" compile-time data */
    let mut tables: *const u8 = std::ptr::null(); /* Char tables base pointer */

    let null_str: [PCRE2_UCHAR; 1] = [0xcd]; /* Dummy for handling null inputs */
    let mut code: *mut PCRE2_UCHAR; /* Current pointer in compiled code */
    let mut codestart: *mut PCRE2_UCHAR = std::ptr::null_mut(); /* Start of compiled code */
    let mut ptr: PCRE2_SPTR = std::ptr::null(); /* Current pointer in pattern */
    let mut pptr: *mut u32; /* Current pointer in parsed pattern */

    let mut length: PCRE2_SIZE = 1; /* Allow for final END opcode */
    let mut usedlength: PCRE2_SIZE = 0; /* Actual length used */
    let mut re_blocksize: PCRE2_SIZE; /* Size of memory block */
    let mut parsed_size_needed: PCRE2_SIZE; /* Needed for parsed pattern */

    let mut firstcuflags: u32 = 0;
    let mut reqcuflags: u32 = 0; /* Type of first/req code unit */
    let mut firstcu: u32 = 0;
    let mut reqcu: u32 = 0; /* Value of first/req code unit */
    let mut setflags: u32 = 0; /* NL and BSR set flags */
    let mut xoptions: u32; /* Flags from context, modified */

    let mut skipatstart: u32; /* When checking (*UTF) etc */
    let mut limit_heap: u32 = u32::MAX;
    let mut limit_match: u32 = u32::MAX; /* Unset match limits */
    let mut limit_depth: u32 = u32::MAX;

    let mut newline: c_int = 0; /* Unset; can be set by the pattern */
    let mut bsr: c_int = 0; /* Unset; can be set by the pattern */
    let mut errorcode: c_int = 0; /* Initialize to avoid compiler warn */
    let regexrc: c_int; /* Return from compile */

    let mut i: u32; /* Local loop counter */

    /* Enable all optimizations by default. */
    let mut optim_flags: u32 = if !ccontext.is_null() {
        (*ccontext).optimization_flags
    } else {
        PCRE2_OPTIMIZATION_ALL
    };

    /* Comments at the head of this file explain about these variables. */

    let mut stack_groupinfo: [u32; GROUPINFO_DEFAULT_SIZE as usize] =
        [0; GROUPINFO_DEFAULT_SIZE as usize];
    let mut stack_parsed_pattern: [u32; PARSED_PATTERN_DEFAULT_SIZE as usize] =
        [0; PARSED_PATTERN_DEFAULT_SIZE as usize];
    let mut named_groups: [named_group; NAMED_GROUP_LIST_SIZE as usize] = std::mem::zeroed();

    /* The workspace is used in different ways in the different compiling phases.
    It needs to be 16-bit aligned for the preliminary parsing scan. */

    let mut c16workspace: [u32; C16_WORK_SIZE as usize] = [0; C16_WORK_SIZE as usize];
    let cworkspace: *mut PCRE2_UCHAR = c16workspace.as_mut_ptr() as *mut PCRE2_UCHAR;

    'exit: {
        'had_error: {
            'had_early_error: {
                'had_cb_error: {
                    /* -------------- Check arguments and set up the pattern ----------------- */

                    /* There must be error code and offset pointers. */

                    if errorptr.is_null() {
                        if !erroroffset.is_null() {
                            *erroroffset = 0;
                        }
                        return std::ptr::null_mut();
                    }
                    if erroroffset.is_null() {
                        if !errorptr.is_null() {
                            *errorptr = ERR120;
                        }
                        return std::ptr::null_mut();
                    }
                    *errorptr = ERR0;
                    *erroroffset = 0;

                    /* There must be a pattern, but NULL is allowed with zero length. */

                    if pattern.is_null() {
                        if patlen == 0 {
                            pattern = null_str.as_ptr();
                        } else {
                            *errorptr = ERR16;
                            return std::ptr::null_mut();
                        }
                    }

                    /* A NULL compile context means "use a default context" */

                    if ccontext.is_null() {
                        ccontext = &raw mut _pcre2_default_compile_context_8;
                    }

                    /* PCRE2_MATCH_INVALID_UTF implies UTF */

                    if (options & PCRE2_MATCH_INVALID_UTF) != 0 {
                        options |= PCRE2_UTF;
                    }

                    /* Check that all undefined public option bits are zero. */

                    if (options & !PUBLIC_COMPILE_OPTIONS) != 0
                        || ((*ccontext).extra_options & !PUBLIC_COMPILE_EXTRA_OPTIONS) != 0
                    {
                        *errorptr = ERR17;
                        return std::ptr::null_mut();
                    }

                    if (options & PCRE2_LITERAL) != 0
                        && ((options & !PUBLIC_LITERAL_COMPILE_OPTIONS) != 0
                            || ((*ccontext).extra_options
                                & !PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS)
                                != 0)
                    {
                        *errorptr = ERR92;
                        return std::ptr::null_mut();
                    }

                    /* A zero-terminated pattern is indicated by the special length value
                    PCRE2_ZERO_TERMINATED. Check for an overlong pattern. */

                    zero_terminated = if patlen == PCRE2_ZERO_TERMINATED {
                        TRUE
                    } else {
                        FALSE
                    };
                    if zero_terminated != 0 {
                        patlen = _pcre2_strlen_8(pattern);
                    }
                    /* (void)zero_terminated; Silence compiler; only used if Valgrind enabled */

                    if patlen > (*ccontext).max_pattern_length {
                        *errorptr = ERR88;
                        return std::ptr::null_mut();
                    }

                    /* Optimization flags in 'options' can override those in the compile
                    context. This is because some options to disable optimizations were added
                    before the optimization flags word existed, and we need to continue
                    supporting them for backwards compatibility. */

                    if (options & PCRE2_NO_AUTO_POSSESS) != 0 {
                        optim_flags &= !PCRE2_OPTIM_AUTO_POSSESS;
                    }
                    if (options & PCRE2_NO_DOTSTAR_ANCHOR) != 0 {
                        optim_flags &= !PCRE2_OPTIM_DOTSTAR_ANCHOR;
                    }
                    if (options & PCRE2_NO_START_OPTIMIZE) != 0 {
                        optim_flags &= !PCRE2_OPTIM_START_OPTIMIZE;
                    }

                    /* From here on, all returns from this function should end up going via
                    the EXIT label. */

                    /* ------------ Initialize the "static" compile data -------------- */

                    tables = if !(*ccontext).tables.is_null() {
                        (*ccontext).tables
                    } else {
                        _pcre2_default_tables_8.as_ptr()
                    };

                    cb.lcc = tables.add(lcc_offset); /* Individual */
                    cb.fcc = tables.add(fcc_offset); /*   character */
                    cb.cbits = tables.add(cbits_offset); /*      tables */
                    cb.ctypes = tables.add(ctypes_offset);

                    cb.assert_depth = 0;
                    cb.bracount = 0;
                    cb.cx = ccontext;
                    cb.dupnames = FALSE;
                    cb.end_pattern = pattern.add(patlen);
                    cb.erroroffset = 0;
                    cb.external_flags = 0;
                    cb.external_options = options;
                    cb.groupinfo = stack_groupinfo.as_mut_ptr();
                    cb.had_recurse = FALSE;
                    cb.lastcapture = 0;
                    cb.max_lookbehind = 0; /* Max encountered */
                    cb.max_varlookbehind = (*ccontext).max_varlookbehind; /* Limit */
                    cb.name_entry_size = 0;
                    cb.name_table = std::ptr::null_mut();
                    cb.named_groups = named_groups.as_mut_ptr();
                    cb.named_group_list_size = NAMED_GROUP_LIST_SIZE as u32;
                    cb.names_found = 0;
                    cb.parens_depth = 0;
                    cb.parsed_pattern = stack_parsed_pattern.as_mut_ptr();
                    cb.req_varyopt = 0;
                    cb.start_code = cworkspace;
                    cb.start_pattern = pattern;
                    cb.start_workspace = cworkspace;
                    cb.workspace_size = COMPILE_WORK_SIZE as PCRE2_SIZE;
                    cb.first_data = std::ptr::null_mut();
                    cb.last_data = std::ptr::null_mut();
                    cb.char_lists_size = 0;

                    /* Maximum back reference and backref bitmap. The bitmap records up to 31
                    back references to help in deciding whether (.*) can be treated as
                    anchored or not. */

                    cb.top_backref = 0;
                    cb.backref_map = 0;

                    /* Escape sequences \1 to \9 are always back references, but as they are
                    only two characters long, only two elements can be used in the
                    parsed_pattern vector. The first contains the reference, and we'd like to
                    use the second to record the offset in the pattern, so that forward
                    references to non-existent groups can be diagnosed later with an offset.
                    However, on 64-bit systems, PCRE2_SIZE won't fit. Instead, we have a
                    vector of offsets for the first occurrence of \1 to \9, indexed by the
                    second parsed_pattern value. All other references have enough space for
                    the offset to be put into the parsed pattern. */

                    i = 0;
                    while i < 10 {
                        cb.small_ref_offset[i as usize] = PCRE2_UNSET;
                        i += 1;
                    }

                    /* --------------- Start looking at the pattern --------------- */

                    /* Unless PCRE2_LITERAL is set, check for global one-time option settings
                    at the start of the pattern, and remember the offset to the actual regex.
                    */

                    xoptions = (*ccontext).extra_options;
                    ptr = pattern;
                    skipatstart = 0;

                    if (options & PCRE2_LITERAL) == 0 {
                        while patlen.wrapping_sub(skipatstart as PCRE2_SIZE) >= 2
                            && *ptr.add(skipatstart as usize) as u32 == CHAR_LEFT_PARENTHESIS
                            && *ptr.add(skipatstart as usize + 1) as u32 == CHAR_ASTERISK
                        {
                            i = 0;
                            while i < pso_list.len() as u32 {
                                let p: *const pso = pso_list.as_ptr().add(i as usize);

                                if patlen
                                    .wrapping_sub(skipatstart as PCRE2_SIZE)
                                    .wrapping_sub(2)
                                    >= (*p).length as PCRE2_SIZE
                                    && _pcre2_strncmp_c8_8(
                                        ptr.add(skipatstart as usize + 2),
                                        (*p).name as *const c_char,
                                        (*p).length as usize,
                                    ) == 0
                                {
                                    let mut c: u32;
                                    let mut pp: u32;

                                    skipatstart += (*p).length as u32 + 2;
                                    let ptype: u32 = (*p).r#type as u32;

                                    if ptype == PSO_OPT as u32 {
                                        cb.external_options |= (*p).value;
                                    } else if ptype == PSO_XOPT as u32 {
                                        xoptions |= (*p).value;
                                    } else if ptype == PSO_FLG as u32 {
                                        setflags |= (*p).value;
                                    } else if ptype == PSO_NL as u32 {
                                        newline = (*p).value as c_int;
                                        setflags |= PCRE2_NL_SET;
                                    } else if ptype == PSO_BSR as u32 {
                                        bsr = (*p).value as c_int;
                                        setflags |= PCRE2_BSR_SET;
                                    } else if ptype == PSO_LIMM as u32
                                        || ptype == PSO_LIMD as u32
                                        || ptype == PSO_LIMH as u32
                                    {
                                        c = 0;
                                        pp = skipatstart;
                                        while (pp as PCRE2_SIZE) < patlen
                                            && (*ptr.add(pp as usize) as u32) >= CHAR_0
                                            && (*ptr.add(pp as usize) as u32) <= CHAR_9
                                        {
                                            if c > u32::MAX / 10 - 1 {
                                                break; /* Integer overflow */
                                            }
                                            c = c * 10
                                                + ({
                                                    let t = *ptr.add(pp as usize) as u32;
                                                    pp += 1;
                                                    t
                                                } - CHAR_0);
                                        }
                                        if (pp as PCRE2_SIZE) >= patlen
                                            || pp == skipatstart
                                            || *ptr.add(pp as usize) as u32
                                                != CHAR_RIGHT_PARENTHESIS
                                        {
                                            errorcode = ERR60;
                                            ptr = ptr.add(pp as usize);
                                            utf = FALSE; /* Used by HAD_EARLY_ERROR */
                                            break 'had_early_error;
                                        }
                                        if ptype == PSO_LIMH as u32 {
                                            limit_heap = c;
                                        } else if ptype == PSO_LIMM as u32 {
                                            limit_match = c;
                                        } else {
                                            limit_depth = c;
                                        }
                                        pp += 1;
                                        skipatstart = pp;
                                    } else if ptype == PSO_OPTMZ as u32 {
                                        optim_flags &= !((*p).value);

                                        /* For backward compatibility the three original
                                        VERBs to disable optimizations need to also update
                                        the corresponding bit in the external options. */

                                        if (*p).value == PCRE2_OPTIM_AUTO_POSSESS {
                                            cb.external_options |= PCRE2_NO_AUTO_POSSESS;
                                        } else if (*p).value == PCRE2_OPTIM_DOTSTAR_ANCHOR {
                                            cb.external_options |= PCRE2_NO_DOTSTAR_ANCHOR;
                                        } else if (*p).value == PCRE2_OPTIM_START_OPTIMIZE {
                                            cb.external_options |= PCRE2_NO_START_OPTIMIZE;
                                        }
                                    } else {
                                        /* LCOV_EXCL_START */
                                        /* All values in the enum need an explicit entry for
                                        this switch but until a better way to prevent coding
                                        mistakes is invented keep a catch all that triggers a
                                        debug build assert as a failsafe */
                                        /* PCRE2_DEBUG_UNREACHABLE(); */
                                        /* LCOV_EXCL_STOP */
                                    }

                                    break; /* Out of the table scan loop */
                                }
                                i += 1;
                            }
                            if i >= pso_list.len() as u32 {
                                break; /* Out of pso loop */
                            }
                        }
                        /* PCRE2_ASSERT(skipatstart <= patlen); */
                    }

                    /* End of pattern-start options; advance to start of real regex. */

                    ptr = ptr.add(skipatstart as usize);

                    /* Check UTF. We have the original options in 'options', with that value
                    as modified by (*UTF) etc in cb->external_options. */

                    utf = if (cb.external_options & PCRE2_UTF) != 0 {
                        TRUE
                    } else {
                        FALSE
                    };
                    if utf != 0 {
                        if (options & PCRE2_NEVER_UTF) != 0 {
                            errorcode = ERR74;
                            break 'had_early_error;
                        }
                        if (options & PCRE2_NO_UTF_CHECK) == 0 {
                            errorcode = _pcre2_valid_utf_8(pattern, patlen, erroroffset);
                            if errorcode != 0 {
                                break 'had_error; /* Offset was set by valid_utf() */
                            }
                        }
                    }

                    /* Check UCP lockout. */

                    ucp = if (cb.external_options & PCRE2_UCP) != 0 {
                        TRUE
                    } else {
                        FALSE
                    };
                    if ucp != 0 && (cb.external_options & PCRE2_NEVER_UCP) != 0 {
                        errorcode = ERR75;
                        break 'had_early_error;
                    }

                    /* PCRE2_EXTRA_TURKISH_CASING checks */

                    if (xoptions & PCRE2_EXTRA_TURKISH_CASING) != 0 {
                        if utf == 0 && ucp == 0 {
                            errorcode = ERR104;
                            break 'had_early_error;
                        }

                        if utf == 0 {
                            errorcode = ERR105;
                            break 'had_early_error;
                        }

                        if (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0 {
                            errorcode = ERR106;
                            break 'had_early_error;
                        }
                    }

                    /* Process the BSR setting. */

                    if bsr == 0 {
                        bsr = (*ccontext).bsr_convention as c_int;
                    }

                    /* Process the newline setting. */

                    if newline == 0 {
                        newline = (*ccontext).newline_convention as c_int;
                    }
                    cb.nltype = NLTYPE_FIXED;
                    if newline == PCRE2_NEWLINE_CR as c_int {
                        cb.nllen = 1;
                        cb.nl[0] = CHAR_CR as PCRE2_UCHAR;
                    } else if newline == PCRE2_NEWLINE_LF as c_int {
                        cb.nllen = 1;
                        cb.nl[0] = CHAR_NL as PCRE2_UCHAR;
                    } else if newline == PCRE2_NEWLINE_NUL as c_int {
                        cb.nllen = 1;
                        cb.nl[0] = CHAR_NUL as PCRE2_UCHAR;
                    } else if newline == PCRE2_NEWLINE_CRLF as c_int {
                        cb.nllen = 2;
                        cb.nl[0] = CHAR_CR as PCRE2_UCHAR;
                        cb.nl[1] = CHAR_NL as PCRE2_UCHAR;
                    } else if newline == PCRE2_NEWLINE_ANY as c_int {
                        cb.nltype = NLTYPE_ANY;
                    } else if newline == PCRE2_NEWLINE_ANYCRLF as c_int {
                        cb.nltype = NLTYPE_ANYCRLF;
                    } else {
                        /* LCOV_EXCL_START */
                        /* PCRE2_DEBUG_UNREACHABLE(); */
                        errorcode = ERR56;
                        break 'had_early_error;
                        /* LCOV_EXCL_STOP */
                    }

                    /* Pre-scan the pattern to do two things: (1) Discover the named groups
                    and their numerical equivalents, so that this information is always
                    available for the remaining processing. (2) At the same time, parse the
                    pattern and put a processed version into the parsed_pattern vector. This
                    has escapes interpreted and comments removed (amongst other things). */

                    /* Ensure that the parsed pattern buffer is big enough. For many smaller
                    patterns the vector on the stack (which was set up above) can be used. */

                    parsed_size_needed =
                        max_parsed_pattern(ptr, cb.end_pattern, utf, options) as PCRE2_SIZE;

                    /* Allow for 2x uint32_t at the start and 2 at the end, for
                    PCRE2_EXTRA_MATCH_WORD or PCRE2_EXTRA_MATCH_LINE (which are exclusive). */

                    if ((*ccontext).extra_options
                        & (PCRE2_EXTRA_MATCH_WORD | PCRE2_EXTRA_MATCH_LINE))
                        != 0
                    {
                        parsed_size_needed += 4;
                    }

                    /* When PCRE2_AUTO_CALLOUT is set we allow for one callout at the end. */

                    if (options & PCRE2_AUTO_CALLOUT) != 0 {
                        parsed_size_needed += 4;
                    }

                    parsed_size_needed += 1; /* For the final META_END */

                    if parsed_size_needed > PARSED_PATTERN_DEFAULT_SIZE as PCRE2_SIZE {
                        let heap_parsed_pattern: *mut u32 = ((*ccontext).memctl.malloc.unwrap())(
                            parsed_size_needed * size_of::<u32>(),
                            (*ccontext).memctl.memory_data,
                        ) as *mut u32;
                        if heap_parsed_pattern.is_null() {
                            *errorptr = ERR21;
                            break 'exit;
                        }
                        cb.parsed_pattern = heap_parsed_pattern;
                    }
                    cb.parsed_pattern_end = cb.parsed_pattern.add(parsed_size_needed);

                    /* Do the parsing scan. */

                    errorcode = parse_regex(
                        ptr,
                        cb.external_options,
                        xoptions,
                        &mut has_lookbehind,
                        &mut cb,
                    );
                    if errorcode != 0 {
                        break 'had_cb_error;
                    }

                    /* If there are any lookbehinds, scan the parsed pattern to figure out
                    their lengths. Workspace is needed to remember whether numbered groups
                    are or are not of limited length, and if limited, what the minimum and
                    maximum lengths are. This caching saves re-computing the length of any
                    group that is referenced more than once, which is particularly relevant
                    when recursion is involved. Unnumbered groups do not have this exposure
                    because they cannot be referenced. If there are sufficiently few groups,
                    the default index vector on the stack, as set up above, can be used.
                    Otherwise we have to get/free some heap memory. The vector must be
                    initialized to zero. */

                    if has_lookbehind != 0 {
                        let mut loopcount: c_int = 0;
                        if cb.bracount >= (GROUPINFO_DEFAULT_SIZE as u32) / 2 {
                            cb.groupinfo = ((*ccontext).memctl.malloc.unwrap())(
                                (2 * (cb.bracount as PCRE2_SIZE + 1)) * size_of::<u32>(),
                                (*ccontext).memctl.memory_data,
                            ) as *mut u32;
                            if cb.groupinfo.is_null() {
                                errorcode = ERR21;
                                cb.erroroffset = 0;
                                break 'had_cb_error;
                            }
                        }
                        memset(
                            cb.groupinfo as *mut c_void,
                            0,
                            (2 * cb.bracount as PCRE2_SIZE + 1) * size_of::<u32>(),
                        );
                        errorcode = check_lookbehinds(
                            cb.parsed_pattern,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            &mut cb,
                            &mut loopcount,
                        );
                        if errorcode != 0 {
                            break 'had_cb_error;
                        }
                    }

                    /* Pretend to compile the pattern while actually just accumulating the
                    amount of memory required in the 'length' variable. This behaviour is
                    triggered by passing a non-NULL final argument to compile_regex(). We pass
                    a block of workspace (cworkspace) for it to compile parts of the pattern
                    into; the compiled code is discarded when it is no longer needed, so
                    hopefully this workspace will never overflow, though there is a test for
                    its doing so.

                    On error, errorcode will be set non-zero, so we don't need to look at the
                    result of the function. The initial options have been put into the cb
                    block, but we still have to pass a separate options variable (the first
                    argument) because the options may change as the pattern is processed. */

                    cb.erroroffset = patlen; /* For any subsequent errors that do not set it */
                    pptr = cb.parsed_pattern;
                    code = cworkspace;
                    *code = OP_BRA as PCRE2_UCHAR;

                    compile_regex(
                        cb.external_options,
                        xoptions,
                        &mut code,
                        &mut pptr,
                        &mut errorcode,
                        0,
                        &mut firstcu,
                        &mut firstcuflags,
                        &mut reqcu,
                        &mut reqcuflags,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        &mut cb,
                        &mut length,
                    );

                    if errorcode != 0 {
                        break 'had_cb_error; /* Offset is in cb.erroroffset */
                    }

                    /* This should be caught in compile_regex(), but just in case... */

                    /* PCRE2_ASSERT((cb.char_lists_size & 0x3) == 0); */
                    if length > MAX_PATTERN_SIZE
                        || MAX_PATTERN_SIZE - length
                            < (cb.char_lists_size / size_of::<PCRE2_UCHAR>())
                    {
                        errorcode = ERR20;
                        cb.erroroffset = 0;
                        break 'had_cb_error;
                    }

                    /* Compute the size of, then, if not too large, get and initialize the
                    data block for storing the compiled pattern and names table. Integer
                    overflow should no longer be possible because nowadays we limit the
                    maximum value of cb.names_found and cb.name_entry_size. */

                    re_blocksize = CU2BYTES!(
                        (cb.names_found as PCRE2_SIZE) * (cb.name_entry_size as PCRE2_SIZE)
                    );

                    if cb.char_lists_size != 0 {
                        /* Align to 32 bit first. This ensures the
                        allocated area will also be 32 bit aligned. */
                        re_blocksize =
                            CLIST_ALIGN_TO!(re_blocksize, size_of::<u32>()) as PCRE2_SIZE;
                        re_blocksize += cb.char_lists_size;
                    }

                    re_blocksize += CU2BYTES!(length);

                    if re_blocksize > (*ccontext).max_pattern_compiled_length {
                        errorcode = ERR101;
                        cb.erroroffset = 0;
                        break 'had_cb_error;
                    }

                    re_blocksize += size_of::<pcre2_real_code>();
                    re = ((*ccontext).memctl.malloc.unwrap())(
                        re_blocksize,
                        (*ccontext).memctl.memory_data,
                    ) as *mut pcre2_real_code;
                    if re.is_null() {
                        errorcode = ERR21;
                        cb.erroroffset = 0;
                        break 'had_cb_error;
                    }

                    /* The compiler may put padding at the end of the pcre2_real_code
                    structure in order to round it up to a multiple of 4 or 8 bytes. This
                    means that when a compiled pattern is copied (for example, when
                    serialized) undefined bytes are read, and this annoys debuggers such as
                    valgrind. To avoid this, we explicitly write to the last 8 bytes of the
                    structure before setting the fields. */

                    memset(
                        (re as *mut u8).add(size_of::<pcre2_real_code>() - 8) as *mut c_void,
                        0,
                        8,
                    );
                    (*re).memctl = (*ccontext).memctl;
                    (*re).tables = tables;
                    (*re).executable_jit = std::ptr::null_mut();
                    memset((*re).start_bitmap.as_mut_ptr() as *mut c_void, 0, 32 * 1);
                    (*re).blocksize = re_blocksize;
                    (*re).code_start = re_blocksize - CU2BYTES!(length);
                    (*re).magic_number = MAGIC_NUMBER;
                    (*re).compile_options = options;
                    (*re).overall_options = cb.external_options;
                    (*re).extra_options = xoptions;
                    (*re).flags = 1u32 /* PCRE2_CODE_UNIT_WIDTH/8 */ | cb.external_flags | setflags;
                    (*re).limit_heap = limit_heap;
                    (*re).limit_match = limit_match;
                    (*re).limit_depth = limit_depth;
                    (*re).first_codeunit = 0;
                    (*re).last_codeunit = 0;
                    (*re).bsr_convention = bsr as u16;
                    (*re).newline_convention = newline as u16;
                    (*re).max_lookbehind = 0;
                    (*re).minlength = 0;
                    (*re).top_bracket = 0;
                    (*re).top_backref = 0;
                    (*re).name_entry_size = cb.name_entry_size;
                    (*re).name_count = cb.names_found;
                    (*re).optimization_flags = optim_flags;

                    /* The basic block is immediately followed by the name table, and the
                    compiled code follows after that. */

                    codestart = (re as *mut u8).add((*re).code_start) as *mut PCRE2_UCHAR;

                    /* Update the compile data block for the actual compile. The starting
                    points of the name/number translation table and of the code are passed
                    around in the compile data block. The start/end pattern and initial
                    options are already set from the pre-compile phase, as is the
                    name_entry_size field. */

                    cb.parens_depth = 0;
                    cb.assert_depth = 0;
                    cb.lastcapture = 0;
                    cb.name_table =
                        (re as *mut u8).add(size_of::<pcre2_real_code>()) as *mut PCRE2_UCHAR;
                    cb.start_code = codestart;
                    cb.req_varyopt = 0;
                    cb.had_accept = FALSE;
                    cb.had_pruneorskip = FALSE;
                    cb.char_lists_size = 0;

                    /* If any named groups were found, create the name/number table from the
                    list created in the pre-pass. */

                    if cb.names_found > 0 {
                        let mut ng: *mut named_group = cb.named_groups;
                        let mut tablecount: u32 = 0;

                        /* Length 0 represents duplicates, and they have already been
                        handled. */
                        i = 0;
                        while i < cb.names_found as u32 {
                            if (*ng).length > 0 {
                                tablecount =
                                    _pcre2_compile_add_name_to_table8(&mut cb, ng, tablecount);
                            }
                            i += 1;
                            ng = ng.add(1);
                        }

                        /* PCRE2_ASSERT(tablecount == cb.names_found); */
                    }

                    /* Set up a starting, non-extracting bracket, then compile the expression.
                    On error, errorcode will be set non-zero, so we don't need to look at the
                    result of the function here. */

                    pptr = cb.parsed_pattern;
                    code = codestart;
                    *code = OP_BRA as PCRE2_UCHAR;
                    regexrc = compile_regex(
                        (*re).overall_options,
                        (*re).extra_options,
                        &mut code,
                        &mut pptr,
                        &mut errorcode,
                        0,
                        &mut firstcu,
                        &mut firstcuflags,
                        &mut reqcu,
                        &mut reqcuflags,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        &mut cb,
                        std::ptr::null_mut(),
                    );
                    if regexrc < 0 {
                        (*re).flags |= PCRE2_MATCH_EMPTY;
                    }
                    (*re).top_bracket = cb.bracount as u16;
                    (*re).top_backref = cb.top_backref as u16;
                    (*re).max_lookbehind = cb.max_lookbehind as u16;

                    if cb.had_accept != 0 {
                        reqcu = 0; /* Must disable after (*ACCEPT) */
                        reqcuflags = REQ_NONE;
                        (*re).flags |= PCRE2_HASACCEPT; /* Disables minimum length */
                    }

                    /* Fill in the final opcode and check for disastrous overflow. If no
                    overflow, but the estimated length exceeds the really used length, adjust
                    the value of re->blocksize. */

                    *code = OP_END as PCRE2_UCHAR;
                    code = code.add(1);
                    usedlength = code.offset_from(codestart) as PCRE2_SIZE;
                    /* LCOV_EXCL_START */
                    if usedlength > length {
                        /* PCRE2_DEBUG_UNREACHABLE(); */
                        errorcode = ERR23; /* Overflow of code block - internal error */
                        cb.erroroffset = 0;
                        break 'had_cb_error;
                    }
                    /* LCOV_EXCL_STOP */

                    (*re).blocksize -= CU2BYTES!(length - usedlength);

                    /* Scan the pattern for recursion/subroutine calls and convert the group
                    numbers into offsets. Maintain a small cache so that repeated groups
                    containing recursions are efficiently handled. */

                    if errorcode == 0 && cb.had_recurse != 0 {
                        let mut rcode: *mut PCRE2_UCHAR;
                        let mut rgroup: PCRE2_SPTR;
                        let mut ccount: c_uint = 0;
                        let mut start: c_int = RSCAN_CACHE_SIZE;
                        let mut rc: [recurse_cache; RSCAN_CACHE_SIZE as usize] =
                            std::mem::zeroed();
                        let rcp: *mut recurse_cache = rc.as_mut_ptr();

                        rcode = find_recurse(codestart, utf);
                        while !rcode.is_null() {
                            let mut p: c_int;
                            let groupnumber: c_int;

                            groupnumber = GET!(rcode, 1) as c_int;
                            if groupnumber == 0 {
                                rgroup = codestart;
                            } else {
                                let mut search_from: PCRE2_SPTR = codestart;
                                rgroup = std::ptr::null();
                                i = 0;
                                p = start;
                                while i < ccount {
                                    if groupnumber == (*rcp.add(p as usize)).groupnumber {
                                        rgroup = (*rcp.add(p as usize)).group;
                                        break;
                                    }

                                    /* Group n+1 must always start to the right of group n,
                                    so we can save search time below when the new group
                                    number is greater than any of the previously found
                                    groups. */

                                    if groupnumber > (*rcp.add(p as usize)).groupnumber {
                                        search_from = (*rcp.add(p as usize)).group;
                                    }
                                    i += 1;
                                    p = (p + 1) & 7;
                                }

                                if rgroup.is_null() {
                                    rgroup =
                                        _pcre2_find_bracket_8(search_from, utf, groupnumber);
                                    /* LCOV_EXCL_START */
                                    if rgroup.is_null() {
                                        /* PCRE2_DEBUG_UNREACHABLE(); */
                                        errorcode = ERR53;
                                        break;
                                    }
                                    /* LCOV_EXCL_STOP */

                                    start -= 1;
                                    if start < 0 {
                                        start = RSCAN_CACHE_SIZE - 1;
                                    }
                                    (*rcp.add(start as usize)).groupnumber = groupnumber;
                                    (*rcp.add(start as usize)).group = rgroup;
                                    if ccount < RSCAN_CACHE_SIZE as c_uint {
                                        ccount += 1;
                                    }
                                }
                            }

                            PUT!(rcode, 1, rgroup.offset_from(codestart) as u32);

                            rcode = find_recurse(rcode.add(1 + LINK_SIZE), utf);
                        }
                    }

                    /* Unless disabled, check whether any single character iterators can be
                    auto-possessified. The function overwrites the appropriate opcode values,
                    so the type of the pointer must be cast. NOTE: the intermediate variable
                    "temp" is used in this code because at least one compiler gives a warning
                    about loss of "const" attribute if the cast (PCRE2_UCHAR *)codestart is
                    used directly in the function call. */

                    if errorcode == 0 && (optim_flags & PCRE2_OPTIM_AUTO_POSSESS) != 0 {
                        let temp: *mut PCRE2_UCHAR = codestart;
                        let possessify_rc: c_int = _pcre2_auto_possessify_8(temp, &cb);
                        /* LCOV_EXCL_START */
                        if possessify_rc != 0 {
                            /* PCRE2_DEBUG_UNREACHABLE(); */
                            errorcode = ERR80;
                            cb.erroroffset = 0;
                        }
                        /* LCOV_EXCL_STOP */
                    }

                    /* Failed to compile, or error while post-processing. */

                    if errorcode != 0 {
                        break 'had_cb_error;
                    }

                    /* Successful compile. If the anchored option was not passed, set it if
                    we can determine that the pattern is anchored by virtue of ^ characters
                    or \A or anything else, such as starting with non-atomic .* when DOTALL is
                    set and there are no occurrences of *PRUNE or *SKIP (though there is an
                    option to disable this case). */

                    if ((*re).overall_options & PCRE2_ANCHORED) == 0 {
                        let dotstar_anchor: BOOL =
                            if (optim_flags & PCRE2_OPTIM_DOTSTAR_ANCHOR) != 0 {
                                TRUE
                            } else {
                                FALSE
                            };
                        if is_anchored(codestart, 0, &mut cb, 0, FALSE, dotstar_anchor) != 0 {
                            (*re).overall_options |= PCRE2_ANCHORED;
                        }
                    }

                    /* Set up the first code unit or startline flag, the required code unit,
                    and then study the pattern. This code need not be obeyed if
                    PCRE2_OPTIM_START_OPTIMIZE is disabled, as the data it would create will
                    not be used. Note that a first code unit (but not the startline flag) is
                    useful for anchored patterns because it can still give a quick "no match"
                    and also avoid searching for a last code unit. */

                    if (optim_flags & PCRE2_OPTIM_START_OPTIMIZE) != 0 {
                        let mut minminlength: c_int = 0; /* For minimal minlength from first/required CU */
                        let study_rc: c_int;

                        /* If we do not have a first code unit, see if there is one that is
                        asserted (these are not saved during the compile because they can
                        cause conflicts with actual literals that follow). */

                        if firstcuflags >= REQ_NONE {
                            let mut assertedcuflags: u32 = 0;
                            let assertedcu: u32 =
                                find_firstassertedcu(codestart, &mut assertedcuflags, 0);
                            /* It would be wrong to use the asserted first code unit as
                             * `firstcu` for regexes which are able to match a 1-character
                             * string (e.g. /(?=a)b?a/) For that example, if we set both
                             * firstcu and reqcu to 'a', it would mean the subject string
                             * needs to be at least 2 characters long, which is wrong.
                             * With more analysis, we would be able to set firstcu in more
                             * cases. */
                            if assertedcuflags < REQ_NONE && assertedcu != reqcu {
                                firstcu = assertedcu;
                                firstcuflags = assertedcuflags;
                            }
                        }

                        /* Save the data for a first code unit. The existence of one means the
                        minimum length must be at least 1. */

                        if firstcuflags < REQ_NONE {
                            (*re).first_codeunit = firstcu;
                            (*re).flags |= PCRE2_FIRSTSET;
                            minminlength += 1;

                            /* Handle caseless first code units. */

                            if (firstcuflags & REQ_CASELESS) != 0 {
                                if firstcu < 128 || (utf == 0 && ucp == 0 && firstcu < 255) {
                                    if *cb.fcc.add(firstcu as usize) as u32 != firstcu {
                                        (*re).flags |= PCRE2_FIRSTCASELESS;
                                    }
                                }
                                /* The first code unit is > 128 in UTF or UCP mode, or > 255
                                otherwise. In 8-bit UTF mode, code units in the range 128-255
                                are introductory code units and cannot have another case, but
                                if UCP is set they may do. */
                                else if ucp != 0 && utf == 0 && UCD_OTHERCASE(firstcu) != firstcu
                                {
                                    (*re).flags |= PCRE2_FIRSTCASELESS;
                                }
                            }
                        }
                        /* When there is no first code unit, for non-anchored patterns, see if
                        we can set the PCRE2_STARTLINE flag. This is helpful for multiline
                        matches when all branches start with ^ and also when all branches
                        start with non-atomic .* for non-DOTALL matches when *PRUNE and SKIP
                        are not present. (There is an option that disables this case.) */
                        else if ((*re).overall_options & PCRE2_ANCHORED) == 0 {
                            let dotstar_anchor: BOOL =
                                if (optim_flags & PCRE2_OPTIM_DOTSTAR_ANCHOR) != 0 {
                                    TRUE
                                } else {
                                    FALSE
                                };
                            if is_startline(codestart, 0, &mut cb, 0, FALSE, dotstar_anchor) != 0
                            {
                                (*re).flags |= PCRE2_STARTLINE;
                            }
                        }

                        /* Handle the "required code unit", if one is set. In the UTF case we
                        can increment the minimum minimum length only if we are sure this
                        really is a different character and not a non-starting code unit of
                        the first character, because the minimum length count is in
                        characters, not code units. */

                        if reqcuflags < REQ_NONE {
                            if ((*re).overall_options & PCRE2_UTF) == 0 ||   /* Not UTF */
                               firstcuflags >= REQ_NONE ||                  /* First not set */
                               (firstcu & 0x80) == 0 ||                     /* First is ASCII */
                               (reqcu & 0x80) == 0
                            /* Req is ASCII */
                            {
                                minminlength += 1;
                            }

                            /* In the case of an anchored pattern, set up the value only if it
                            follows a variable length item in the pattern. */

                            if ((*re).overall_options & PCRE2_ANCHORED) == 0
                                || (reqcuflags & REQ_VARY) != 0
                            {
                                (*re).last_codeunit = reqcu;
                                (*re).flags |= PCRE2_LASTSET;

                                /* Handle caseless required code units as for first code units
                                (above). */

                                if (reqcuflags & REQ_CASELESS) != 0 {
                                    if reqcu < 128 || (utf == 0 && ucp == 0 && reqcu < 255) {
                                        if *cb.fcc.add(reqcu as usize) as u32 != reqcu {
                                            (*re).flags |= PCRE2_LASTCASELESS;
                                        }
                                    } else if ucp != 0
                                        && utf == 0
                                        && UCD_OTHERCASE(reqcu) != reqcu
                                    {
                                        (*re).flags |= PCRE2_LASTCASELESS;
                                    }
                                }
                            }
                        }

                        /* Study the compiled pattern to set up information such as a bitmap
                        of starting code units and a minimum matching length. */

                        study_rc = _pcre2_study_8(re);
                        /* LCOV_EXCL_START */
                        if study_rc != 0 {
                            /* PCRE2_DEBUG_UNREACHABLE(); */
                            errorcode = ERR31;
                            cb.erroroffset = 0;
                            break 'had_cb_error;
                        }
                        /* LCOV_EXCL_STOP */

                        /* If study() set a bitmap of starting code units, it implies a
                        minimum length of at least one. */

                        if ((*re).flags & PCRE2_FIRSTMAPSET) != 0 && minminlength == 0 {
                            minminlength = 1;
                        }

                        /* If the minimum length set (or not set) by study() is less than the
                        minimum implied by required code units, override it. */

                        if ((*re).minlength as c_int) < minminlength {
                            (*re).minlength = minminlength as u16;
                        }
                    } /* End of start-of-match optimizations. */

                    /* Control ends up here in all cases. */

                    /* All items must be freed. */
                    /* PCRE2_ASSERT(cb.first_data == NULL); */

                    break 'exit;
                }
                /* HAD_CB_ERROR: */

                /* Errors discovered in parse_regex() set the offset value in the compile
                block. Errors discovered before it is called must compute it from the ptr
                value. After parse_regex() is called, the offset in the compile block is set
                to the end of the pattern, but certain errors in compile_regex() may reset it
                if an offset is available in the parsed pattern. */

                ptr = pattern.wrapping_add(cb.erroroffset);
            }
            /* HAD_EARLY_ERROR: */
            /* Ensure we don't return out-of-range erroroffset. */
            /* PCRE2_ASSERT(ptr >= pattern); */
            /* PCRE2_ASSERT(ptr <= (pattern + patlen)); */
            /* Ensure that the erroroffset never slices a UTF-encoded character in half.
            If the input is invalid, then we return an offset just before the first invalid
            character, so the text to the left of the offset must always be valid. */
            *erroroffset = (ptr as PCRE2_SIZE).wrapping_sub(pattern as PCRE2_SIZE);
        }
        /* HAD_ERROR: */
        *errorptr = errorcode;
        pcre2_code_free_8(re);
        re = std::ptr::null_mut();

        if !cb.first_data.is_null() {
            let mut current_data: *mut compile_data = cb.first_data;
            loop {
                let next_data: *mut compile_data = (*current_data).next;
                ((*cb.cx).memctl.free.unwrap())(
                    current_data as *mut c_void,
                    (*cb.cx).memctl.memory_data,
                );
                current_data = next_data;
                if current_data.is_null() {
                    break;
                }
            }
        }

        /* goto EXIT; */
    }

    /* EXIT: */
    /* If memory was obtained for the parsed version of the pattern, free it before
    returning. Also free the list of named groups if a larger one had to be
    obtained, and likewise the group information vector. */

    if cb.parsed_pattern != stack_parsed_pattern.as_mut_ptr() {
        ((*ccontext).memctl.free.unwrap())(
            cb.parsed_pattern as *mut c_void,
            (*ccontext).memctl.memory_data,
        );
    }
    if cb.named_group_list_size > NAMED_GROUP_LIST_SIZE as u32 {
        ((*ccontext).memctl.free.unwrap())(
            cb.named_groups as *mut c_void,
            (*ccontext).memctl.memory_data,
        );
    }
    if cb.groupinfo != stack_groupinfo.as_mut_ptr() {
        ((*ccontext).memctl.free.unwrap())(
            cb.groupinfo as *mut c_void,
            (*ccontext).memctl.memory_data,
        );
    }

    return re; /* Will be NULL after an error */
}
