/* Translated from c_src/src/pcre2_compile.c lines 2258-2730 */

/*************************************************
*               Handle \P and \p                 *
*************************************************/

/* This function is called after \P or \p has been encountered, provided that
PCRE2 is compiled with support for UTF and Unicode properties. On entry, the
contents of ptrptr are pointing after the P or p. On exit, it is left pointing
after the final code unit of the escape sequence.

Arguments:
  ptrptr         the pattern position pointer
  utf            true if the input is UTF-encoded
  negptr         a boolean that is set TRUE for negation else FALSE
  ptypeptr       an unsigned int that is set to the type value
  pdataptr       an unsigned int that is set to the detailed property value
  errorcodeptr   the error code variable
  cb             the compile data

Returns:         TRUE if the type value was found, or FALSE for an invalid type
*/

unsafe fn get_ucp(
    ptrptr: *mut PCRE2_SPTR,
    utf: BOOL,
    negptr: *mut BOOL,
    ptypeptr: *mut u16,
    pdataptr: *mut u16,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> BOOL {
    let mut c: u32 = 0;
    let mut i: isize = 0;
    let mut bot: PCRE2_SIZE;
    let mut top: PCRE2_SIZE;
    let mut ptr: PCRE2_SPTR = *ptrptr;
    let mut name: [PCRE2_UCHAR; 50] = [0; 50];
    let mut vptr: *mut PCRE2_UCHAR = std::ptr::null_mut();
    let mut ptscript: u16 = PT_NOTSCRIPT as u16;

    'error_return: {
        if ptr >= (*cb).end_pattern {
            break 'error_return;
        }
        GETCHARINCTEST!(c, ptr, utf);
        *negptr = FALSE;

        /* \P or \p can be followed by a name in {}, optionally preceded by ^ for
        negation. We must be handling Unicode encoding here, though we may be
        compiling for UTF-8 input in an EBCDIC environment. (PCRE2 does not support
        both EBCDIC input and Unicode input in the same build.) In accordance with
        Unicode's "loose matching" rules, ASCII white space, hyphens, and
        underscores are ignored. We don't use isspace() or tolower() because (a)
        code points may be greater than 255, and (b) they wouldn't work when
        compiling for Unicode in an EBCDIC environment. */

        if c == CHAR_LEFT_CURLY_BRACKET {
            if ptr >= (*cb).end_pattern {
                break 'error_return;
            }

            i = 0;
            /* sizeof(name)/sizeof(PCRE2_UCHAR) - 1 == 49 */
            'name_loop: while i < 49 {
                'redo: loop {
                    if ptr >= (*cb).end_pattern {
                        break 'error_return;
                    }
                    GETCHARINCTEST!(c, ptr, utf);

                    /* Skip ignorable Unicode characters. */

                    if c == CHAR_UNDERSCORE
                        || c == CHAR_MINUS
                        || c == CHAR_SPACE
                        || (c >= CHAR_HT && c <= CHAR_CR)
                    {
                        continue 'redo;
                    }

                    /* The first significant character being circumflex negates the
                    meaning of the item. */

                    if i == 0 && *negptr == 0 && c == CHAR_CIRCUMFLEX_ACCENT {
                        *negptr = TRUE;
                        continue 'redo;
                    }

                    if c == CHAR_RIGHT_CURLY_BRACKET {
                        break 'name_loop;
                    }

                    /* Names consist of ASCII letters and digits, but equals and colon
                    may also occur as a name/value separator. We must also allow for
                    \p{L&}. A simple check for a value between '&' and 'z' suffices
                    because anything else in a name or value will cause an "unknown
                    property" error anyway. */

                    if c < CHAR_AMPERSAND || c > CHAR_z {
                        break 'error_return;
                    }

                    /* Lower case a capital letter or remember where the name/value
                    separator is. */

                    if c >= CHAR_A && c <= CHAR_Z {
                        c |= 0x20;
                    } else if (c == CHAR_COLON || c == CHAR_EQUALS_SIGN) && vptr.is_null() {
                        vptr = name.as_mut_ptr().add(i as usize);
                    }

                    *name.as_mut_ptr().add(i as usize) = c as PCRE2_UCHAR;
                    break;
                }
                i += 1;
            }

            /* Error if the loop didn't end with '}' - either we hit the end of the
            pattern or the name was longer than any legal property name. */

            if c != CHAR_RIGHT_CURLY_BRACKET {
                break 'error_return;
            }
            *name.as_mut_ptr().add(i as usize) = 0;
        }
        /* If { doesn't follow \p or \P there is just one following character, which
        must be an ASCII letter. */
        else if c >= CHAR_A && c <= CHAR_Z {
            name[0] = (c | 0x20) as PCRE2_UCHAR; /* Lower case */
            name[1] = 0;
        } else if c >= CHAR_a && c <= CHAR_z {
            name[0] = c as PCRE2_UCHAR;
            name[1] = 0;
        } else {
            break 'error_return;
        }

        *ptrptr = ptr; /* Update pattern pointer */

        /* If the property contains ':' or '=' we have class name and value
        separately specified. The following are supported:

          . Bidi_Class (synonym bc), for which the property names are "bidi<name>".
          . Script (synonym sc) for which the property name is the script name
          . Script_Extensions (synonym scx), ditto

        As this is a small number, we currently just check the names directly. If
        this grows, a sorted table and a switch will be neater.

        For both the script properties, set a PT_xxx value so that (1) they can be
        distinguished and (2) invalid script names that happen to be the name of
        another property can be diagnosed. */

        if !vptr.is_null() {
            let mut offset: c_int = 0;
            let mut sname: [PCRE2_UCHAR; 8] = [0; 8];

            *vptr = 0; /* Terminate property name */
            if _pcre2_strcmp_c8_8(name.as_ptr(), b"bidiclass\0".as_ptr() as *const c_char) == 0
                || _pcre2_strcmp_c8_8(name.as_ptr(), b"bc\0".as_ptr() as *const c_char) == 0
            {
                offset = 4;
                sname[0] = CHAR_b as PCRE2_UCHAR;
                sname[1] = CHAR_i as PCRE2_UCHAR; /* There is no strcpy_c8 function */
                sname[2] = CHAR_d as PCRE2_UCHAR;
                sname[3] = CHAR_i as PCRE2_UCHAR;
            } else if _pcre2_strcmp_c8_8(name.as_ptr(), b"script\0".as_ptr() as *const c_char) == 0
                || _pcre2_strcmp_c8_8(name.as_ptr(), b"sc\0".as_ptr() as *const c_char) == 0
            {
                ptscript = PT_SC as u16;
            } else if _pcre2_strcmp_c8_8(
                name.as_ptr(),
                b"scriptextensions\0".as_ptr() as *const c_char,
            ) == 0
                || _pcre2_strcmp_c8_8(name.as_ptr(), b"scx\0".as_ptr() as *const c_char) == 0
            {
                ptscript = PT_SCX as u16;
            } else {
                *errorcodeptr = ERR47;
                return FALSE;
            }

            /* Adjust the string in name[] as needed */

            memmove(
                name.as_mut_ptr().add(offset as usize) as *mut c_void,
                vptr.add(1) as *const c_void,
                (name.as_mut_ptr().add(i as usize).offset_from(vptr) as usize)
                    * size_of::<PCRE2_UCHAR>(),
            );
            if offset != 0 {
                memmove(
                    name.as_mut_ptr() as *mut c_void,
                    sname.as_ptr() as *const c_void,
                    offset as usize * size_of::<PCRE2_UCHAR>(),
                );
            }
        }

        /* Search for a recognized property using binary chop. */

        bot = 0;
        top = _pcre2_utt_size_8;

        while bot < top {
            let r: c_int;
            i = ((bot + top) >> 1) as isize;
            let utt_i: *const ucp_type_table = _pcre2_utt_8.as_ptr().add(i as usize);
            r = _pcre2_strcmp_c8_8(
                name.as_ptr(),
                _pcre2_utt_names_8.as_ptr().add((*utt_i).name_offset as usize) as *const c_char,
            );

            /* When a matching property is found, some extra checking is needed when
            the \p{xx:yy} syntax is used and xx is either sc or scx. */

            if r == 0 {
                *pdataptr = (*utt_i).value;
                if vptr.is_null() || ptscript as u32 == PT_NOTSCRIPT {
                    *ptypeptr = (*utt_i).type_;
                    return TRUE;
                }

                if (*utt_i).type_ as u32 == PT_SC {
                    *ptypeptr = PT_SC as u16;
                    return TRUE;
                } else if (*utt_i).type_ as u32 == PT_SCX {
                    *ptypeptr = ptscript;
                    return TRUE;
                }

                break; /* Non-script found */
            }

            if r > 0 {
                bot = i as PCRE2_SIZE + 1;
            } else {
                top = i as PCRE2_SIZE;
            }
        }

        *errorcodeptr = ERR47; /* Unrecognized property */
        return FALSE;
    }

    /* ERROR_RETURN:            Malformed \P or \p */
    *errorcodeptr = ERR46;
    *ptrptr = ptr;
    FALSE
}

/*************************************************
*           Check for POSIX class syntax         *
*************************************************/

/* This function is called when the sequence "[:" or "[." or "[=" is
encountered in a character class. It checks whether this is followed by a
sequence of characters terminated by a matching ":]" or ".]" or "=]". If we
reach an unescaped ']' without the special preceding character, return FALSE.

Arguments:
  ptr      pointer to the character after the initial [ (colon, dot, equals)
  ptrend   pointer to the end of the pattern
  endptr   where to return a pointer to the terminating ':', '.', or '='

Returns:   TRUE or FALSE
*/

unsafe fn check_posix_syntax(
    mut ptr: PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    endptr: *mut PCRE2_SPTR,
) -> BOOL {
    let terminator: PCRE2_UCHAR; /* Don't combine these lines; the Solaris cc */
    terminator = {
        let t = *ptr;
        ptr = ptr.add(1);
        t
    }; /* compiler warns about "non-constant" initializer. */

    while ptrend.offset_from(ptr) >= 2 {
        if *ptr as u32 == CHAR_BACKSLASH
            && (*ptr.add(1) as u32 == CHAR_RIGHT_SQUARE_BRACKET
                || *ptr.add(1) as u32 == CHAR_BACKSLASH)
        {
            ptr = ptr.add(1);
        } else if (*ptr as u32 == CHAR_LEFT_SQUARE_BRACKET && *ptr.add(1) == terminator)
            || *ptr as u32 == CHAR_RIGHT_SQUARE_BRACKET
        {
            return FALSE;
        } else if *ptr == terminator && *ptr.add(1) as u32 == CHAR_RIGHT_SQUARE_BRACKET {
            *endptr = ptr;
            return TRUE;
        }

        ptr = ptr.add(1);
    }

    FALSE
}

/*************************************************
*          Check POSIX class name                *
*************************************************/

/* This function is called to check the name given in a POSIX-style class entry
such as [:alnum:].

Arguments:
  ptr        points to the first letter
  len        the length of the name

Returns:     a value representing the name, or -1 if unknown
*/

unsafe fn check_posix_name(ptr: PCRE2_SPTR, len: c_int) -> c_int {
    let mut pn: *const c_char = posix_names.as_ptr() as *const c_char;
    let mut yield_: c_int = 0;
    while *posix_name_lengths.as_ptr().add(yield_ as usize) != 0 {
        if len == *posix_name_lengths.as_ptr().add(yield_ as usize) as c_int
            && _pcre2_strncmp_c8_8(ptr, pn, len as c_uint as usize) == 0
        {
            return yield_;
        }
        pn = pn.add(*posix_name_lengths.as_ptr().add(yield_ as usize) as usize + 1);
        yield_ += 1;
    }
    -1
}

/*************************************************
*       Read a subpattern or VERB name           *
*************************************************/

/* This function is called from parse_regex() below whenever it needs to read
the name of a subpattern or a (*VERB) or an (*alpha_assertion). The initial
pointer must be to the preceding character. If that character is '*' we are
reading a verb or alpha assertion name. The pointer is updated to point after
the name, for a VERB or alpha assertion name, or after the name's terminator
for a subpattern name. Returning both the offset and the name pointer is
redundant information, but some callers use one and some the other, so it is
simplest just to return both. When the name is in braces, spaces and tabs are
allowed (and ignored) at either end.

Arguments:
  ptrptr      points to the character pointer variable
  ptrend      points to the end of the input string
  utf         true if the input is UTF-encoded
  terminator  the terminator of a subpattern name must be this
  offsetptr   where to put the offset from the start of the pattern
  nameptr     where to put a pointer to the name in the input
  namelenptr  where to put the length of the name
  errcodeptr  where to put an error code
  cb          pointer to the compile data block

Returns:    TRUE if a name was read
            FALSE otherwise, with error code set
*/

unsafe fn read_name(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    utf: BOOL,
    terminator: u32,
    offsetptr: *mut PCRE2_SIZE,
    nameptr: *mut PCRE2_SPTR,
    namelenptr: *mut u32,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> BOOL {
    let mut ptr: PCRE2_SPTR = *ptrptr;
    let is_group: BOOL = ({
        let t = *ptr;
        ptr = ptr.add(1);
        t
    } as u32
        != CHAR_ASTERISK) as BOOL;
    let is_braced: BOOL = (terminator == CHAR_RIGHT_CURLY_BRACKET) as BOOL;

    'failed: {
        if is_braced != 0 {
            while ptr < ptrend && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT) {
                ptr = ptr.add(1);
            }
        }

        if ptr >= ptrend
        /* No characters in name */
        {
            *errorcodeptr = if is_group != 0 {
                ERR62 /* Subpattern name expected */
            } else {
                ERR60 /* Verb not recognized or malformed */
            };
            break 'failed;
        }

        *nameptr = ptr;
        *offsetptr = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;

        /* If this logic were ever to change, the matching function in
        pcre2_substitute.c ought to be updated to match. */

        /* In UTF mode, a group name may contain letters and decimal digits as
        defined by Unicode properties, and underscores, but must not start with a
        digit. */

        if utf != 0 && is_group != 0 {
            let mut c: u32 = 0;
            let mut type_: u32;
            let mut p: PCRE2_SPTR = ptr;

            GETCHARINC!(c, p); /* Peek at next character */
            type_ = UCD_CHARTYPE(c);

            if type_ == ucp_Nd {
                ptr = p;
                *errorcodeptr = ERR44;
                break 'failed;
            }

            loop {
                if type_ != ucp_Nd
                    && *_pcre2_ucp_gentype_8.as_ptr().add(type_ as usize) != ucp_L
                    && c != CHAR_UNDERSCORE
                {
                    break;
                }
                ptr = p; /* Accept character and peek again */
                if p >= ptrend {
                    break;
                }
                GETCHARINC!(c, p);
                type_ = UCD_CHARTYPE(c);
            }
        }
        /* Handle non-group names and group names in non-UTF modes. A group name must
        not start with a digit. If either of the others start with a digit it just
        won't be recognized. */
        else {
            if is_group != 0 && (*ptr as u32 >= CHAR_0 && *ptr as u32 <= CHAR_9) {
                ptr = ptr.add(1);
                *errorcodeptr = ERR44;
                break 'failed;
            }

            while ptr < ptrend
                && MAX_255!(*ptr) != 0
                && (*(*cb).ctypes.add(*ptr as usize) & ctype_word) != 0
            {
                ptr = ptr.add(1);
            }
        }

        /* Check name length */

        if ptr.offset_from(*nameptr) > MAX_NAME_SIZE as isize {
            *errorcodeptr = ERR48;
            break 'failed;
        }
        *namelenptr = ptr.offset_from(*nameptr) as u32;

        /* Subpattern names must not be empty, and their terminator is checked here.
        (What follows a verb or alpha assertion name is checked separately.) */

        if is_group != 0 {
            if ptr == *nameptr {
                *errorcodeptr = ERR62; /* Subpattern name expected */
                break 'failed;
            }
            if is_braced != 0 {
                while ptr < ptrend && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT) {
                    ptr = ptr.add(1);
                }
            }
            if terminator != 0 {
                if ptr >= ptrend || *ptr != terminator as PCRE2_UCHAR {
                    *errorcodeptr = ERR42;
                    break 'failed;
                }
                ptr = ptr.add(1);
            }
        }

        *ptrptr = ptr;
        return TRUE;
    }

    /* FAILED: */
    *ptrptr = ptr;
    FALSE
}
