/* Translated from c_src/src/pcre2_compile.c lines 2731-3111 */

/**************************************************
*        Parse capturing bracket argument list    *
**************************************************/

/* Reads a list of capture references. The references
can be numbers or names.

Arguments:
  ptrptr           points to the character pointer variable
  ptrend           points to the end of the input string
  utf              true if the input is UTF-encoded
  parsed_pattern   the parsed pattern pointer
  offset           last known offset
  errcodeptr       where to put an error code
  cb               pointer to the compile data block

Returns: updated parsed_pattern pointer on success
         NULL otherwise
*/

unsafe fn parse_capture_list(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    utf: BOOL,
    mut parsed_pattern: *mut u32,
    mut offset: PCRE2_SIZE,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> *mut u32 {
    let mut next_offset: PCRE2_SIZE = 0;
    let mut ptr: PCRE2_SPTR = *ptrptr;
    let mut name: PCRE2_SPTR = std::ptr::null();
    let mut terminator: PCRE2_UCHAR;
    let mut meta: u32 = 0;
    let mut namelen: u32 = 0;
    let mut i: c_int = 0;

    'failed: {
        'unclosed_parenthesis: {
            if ptr >= ptrend || *ptr as u32 != CHAR_LEFT_PARENTHESIS {
                *errorcodeptr = ERR118;
                break 'failed; /* goto FAILED */
            }

            loop {
                ptr = ptr.add(1);
                next_offset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;

                if ptr >= ptrend {
                    *errorcodeptr = ERR117;
                    break 'failed; /* goto FAILED */
                }

                /* Handle [+-]number cases */
                if read_number(
                    &mut ptr,
                    ptrend,
                    (*cb).bracount as i32,
                    MAX_GROUP_NUMBER,
                    ERR61 as u32,
                    &mut i,
                    errorcodeptr,
                ) != 0
                {
                    /* PCRE2_ASSERT(i >= 0); */
                    if i <= 0 {
                        *errorcodeptr = ERR15;
                        break 'failed; /* goto FAILED */
                    }
                    meta = META_CAPTURE_NUMBER;
                    namelen = i as u32;
                } else if *errorcodeptr != 0 {
                    break 'failed; /* Number too big */
                } else {
                    /* Handle 'name' or <name> cases. */
                    if *ptr as u32 == CHAR_LESS_THAN_SIGN {
                        terminator = CHAR_GREATER_THAN_SIGN as PCRE2_UCHAR;
                    } else if *ptr as u32 == CHAR_APOSTROPHE {
                        terminator = CHAR_APOSTROPHE as PCRE2_UCHAR;
                    } else {
                        *errorcodeptr = ERR117;
                        break 'failed; /* goto FAILED */
                    }

                    if read_name(
                        &mut ptr,
                        ptrend,
                        utf,
                        terminator as u32,
                        &mut next_offset,
                        &mut name,
                        &mut namelen,
                        errorcodeptr,
                        cb,
                    ) == 0
                    {
                        break 'failed; /* goto FAILED */
                    }

                    meta = META_CAPTURE_NAME;
                }

                /* PCRE2_ASSERT(next_offset > 0); */
                if offset == 0 || next_offset.wrapping_sub(offset) >= 0x10000 {
                    *parsed_pattern = META_OFFSET;
                    parsed_pattern = parsed_pattern.add(1);
                    PUTOFFSET!(next_offset, parsed_pattern);
                    offset = next_offset;
                }

                /* The offset is encoded as a relative offset, because for some
                inputs such as ",2" in (1,2,3), we only have space for two uint32_t
                values, and an opcode and absolute offset may require three uint32_t
                values. */
                *parsed_pattern = meta | next_offset.wrapping_sub(offset) as u32;
                parsed_pattern = parsed_pattern.add(1);
                *parsed_pattern = namelen;
                parsed_pattern = parsed_pattern.add(1);
                offset = next_offset;

                if ptr >= ptrend {
                    break 'unclosed_parenthesis; /* goto UNCLOSED_PARENTHESIS */
                }

                if *ptr as u32 == CHAR_RIGHT_PARENTHESIS {
                    break;
                }

                if *ptr as u32 != CHAR_COMMA {
                    *errorcodeptr = ERR24;
                    break 'failed; /* goto FAILED */
                }
            }

            *ptrptr = ptr.add(1);
            return parsed_pattern;
        }

        /* UNCLOSED_PARENTHESIS: */
        *errorcodeptr = ERR14;
        /* Falls through into FAILED */
    }

    /* FAILED: */
    *ptrptr = ptr;
    return std::ptr::null_mut();
}

/*************************************************
*          Manage callouts at start of cycle     *
*************************************************/

/* At the start of a new item in parse_regex() we are able to record the
details of the previous item in a prior callout, and also to set up an
automatic callout if enabled. Avoid having two adjacent automatic callouts,
which would otherwise happen for items such as \Q that contribute nothing to
the parsed pattern.

Arguments:
  ptr              current pattern pointer
  pcalloutptr      points to a pointer to previous callout, or NULL
  auto_callout     TRUE if auto_callouts are enabled
  parsed_pattern   the parsed pattern pointer
  cb               compile block

Returns: possibly updated parsed_pattern pointer.
*/

unsafe fn manage_callouts(
    ptr: PCRE2_SPTR,
    pcalloutptr: *mut *mut u32,
    auto_callout: BOOL,
    mut parsed_pattern: *mut u32,
    cb: *mut compile_block,
) -> *mut u32 {
    let mut previous_callout: *mut u32 = *pcalloutptr;

    if !previous_callout.is_null() {
        *previous_callout.add(2) = (ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE)
            .wrapping_sub(*previous_callout.add(1) as PCRE2_SIZE) as u32;
    }

    if auto_callout == 0 {
        previous_callout = std::ptr::null_mut();
    } else {
        if previous_callout.is_null()
            || previous_callout != parsed_pattern.sub(4)
            || *previous_callout.add(3) != 255
        {
            previous_callout = parsed_pattern; /* Set up new automatic callout */
            parsed_pattern = parsed_pattern.add(4);
            *previous_callout.add(0) = META_CALLOUT_NUMBER;
            *previous_callout.add(2) = 0;
            *previous_callout.add(3) = 255;
        }
        *previous_callout.add(1) = ptr.offset_from((*cb).start_pattern) as u32;
    }

    *pcalloutptr = previous_callout;
    return parsed_pattern;
}

/*************************************************
*          Handle \d, \D, \s, \S, \w, \W         *
*************************************************/

/* This function is called from parse_regex() below, both for freestanding
escapes, and those within classes, to handle those escapes that may change when
Unicode property support is requested. Note that PCRE2_UCP will never be set
without Unicode support because that is checked when pcre2_compile() is called.

Arguments:
  escape          the ESC_... value
  parsed_pattern  where to add the code
  options         options bits
  xoptions        extra options bits

Returns:          updated value of parsed_pattern
*/
unsafe fn handle_escdsw(
    escape: c_int,
    mut parsed_pattern: *mut u32,
    options: u32,
    xoptions: u32,
) -> *mut u32 {
    let mut ascii_option: u32 = 0;
    let mut prop: u32 = ESC_p as u32;

    match escape {
        ESC_D => {
            prop = ESC_P as u32;
            /* Fall through */
            ascii_option = PCRE2_EXTRA_ASCII_BSD;
        }
        ESC_d => {
            ascii_option = PCRE2_EXTRA_ASCII_BSD;
        }

        ESC_S => {
            prop = ESC_P as u32;
            /* Fall through */
            ascii_option = PCRE2_EXTRA_ASCII_BSS;
        }
        ESC_s => {
            ascii_option = PCRE2_EXTRA_ASCII_BSS;
        }

        ESC_W => {
            prop = ESC_P as u32;
            /* Fall through */
            ascii_option = PCRE2_EXTRA_ASCII_BSW;
        }
        ESC_w => {
            ascii_option = PCRE2_EXTRA_ASCII_BSW;
        }

        _ => {}
    }

    if (options & PCRE2_UCP) == 0 || (xoptions & ascii_option) != 0 {
        *parsed_pattern = META_ESCAPE + escape as u32;
        parsed_pattern = parsed_pattern.add(1);
    } else {
        *parsed_pattern = META_ESCAPE + prop;
        parsed_pattern = parsed_pattern.add(1);
        match escape {
            ESC_d | ESC_D => {
                *parsed_pattern = (PT_PC << 16) | ucp_Nd;
                parsed_pattern = parsed_pattern.add(1);
            }

            ESC_s | ESC_S => {
                *parsed_pattern = PT_SPACE << 16;
                parsed_pattern = parsed_pattern.add(1);
            }

            ESC_w | ESC_W => {
                *parsed_pattern = PT_WORD << 16;
                parsed_pattern = parsed_pattern.add(1);
            }

            _ => {}
        }
    }

    return parsed_pattern;
}

/*************************************************
* Maximum size of parsed_pattern for given input *
*************************************************/

/* This function is called from parse_regex() below, to determine the amount
of memory to allocate for parsed_pattern. It is also called to check whether
the amount of data written respects the amount of memory allocated.

Arguments:
  ptr             points to the start of the pattern
  ptrend          points to the end of the pattern
  utf             TRUE in UTF mode
  options         the options bits

Returns:          the number of uint32_t units for parsed_pattern
*/
unsafe fn max_parsed_pattern(
    ptr: PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    utf: BOOL,
    options: u32,
) -> isize {
    let big32count: PCRE2_SIZE = 0;
    let mut parsed_size_needed: isize;

    /* When PCRE2_AUTO_CALLOUT is not set, in all but one case the number of
    unsigned 32-bit ints written out to the parsed pattern is bounded by the length
    of the pattern. The exceptional case is when running in 32-bit, non-UTF mode,
    when literal characters greater than META_END (0x80000000) have to be coded as
    two units. In this case, therefore, we scan the pattern to check for such
    values. (Not applicable in the 8-bit library: (void)utf.) */

    parsed_size_needed = ptrend.offset_from(ptr) + big32count as isize;

    /* When PCRE2_AUTO_CALLOUT is set we have to assume a numerical callout (4
    elements) for each character. This is overkill, but memory is plentiful these
    days. */

    if (options & PCRE2_AUTO_CALLOUT) != 0 {
        parsed_size_needed += ptrend.offset_from(ptr) * 4;
    }

    return parsed_size_needed;
}
