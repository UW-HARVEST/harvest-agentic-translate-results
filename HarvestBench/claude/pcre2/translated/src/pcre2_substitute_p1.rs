/* Translated from c_src/src/pcre2_substitute.c lines 45-441 */

/* PTR_STACK_SIZE is used both as an array bound and compared against a
uint32_t, so it is declared here as a usize (cast where needed). */

const PTR_STACK_SIZE: usize = 20;

const SUBSTITUTE_OPTIONS: u32 = PCRE2_SUBSTITUTE_EXTENDED
    | PCRE2_SUBSTITUTE_GLOBAL
    | PCRE2_SUBSTITUTE_LITERAL
    | PCRE2_SUBSTITUTE_MATCHED
    | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH
    | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY
    | PCRE2_SUBSTITUTE_UNKNOWN_UNSET
    | PCRE2_SUBSTITUTE_UNSET_EMPTY;

/*************************************************
*           Find end of substitute text          *
*************************************************/

/* In extended mode, we recognize ${name:+set text:unset text} and similar
constructions. This requires the identification of unescaped : and }
characters. This function scans for such. It must deal with nested ${
constructions. The pointer to the text is updated, either to the required end
character, or to where an error was detected.

Arguments:
  code      points to the compiled expression (for options)
  ptrptr    points to the pointer to the start of the text (updated)
  ptrend    end of the whole string
  last      TRUE if the last expected string (only } recognized)

Returns:    0 on success
            negative error code on failure
*/

unsafe fn find_text_end(
    code: *const pcre2_real_code,
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    last: BOOL,
) -> c_int {
    let mut rc: c_int = 0;
    let mut nestlevel: u32 = 0;
    let mut literal: BOOL = FALSE;
    let mut ptr: PCRE2_SPTR = *ptrptr;

    'EXIT: {
        while ptr < ptrend {
            /* The body of the C for-loop; a C `continue' becomes `break 'CONTINUE'
            so that the loop increment below still happens. */
            'CONTINUE: {
                if literal != 0 {
                    if *ptr.add(0) as u32 == CHAR_BACKSLASH
                        && ptr < ptrend.sub(1)
                        && *ptr.add(1) as u32 == CHAR_E
                    {
                        literal = FALSE;
                        ptr = ptr.add(1);
                    }
                } else if *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
                    if nestlevel == 0 {
                        break 'EXIT;
                    }
                    nestlevel -= 1;
                } else if *ptr as u32 == CHAR_COLON && last == 0 && nestlevel == 0 {
                    break 'EXIT;
                } else if *ptr as u32 == CHAR_DOLLAR_SIGN {
                    if ptr < ptrend.sub(1) && *ptr.add(1) as u32 == CHAR_LEFT_CURLY_BRACKET {
                        nestlevel += 1;
                        ptr = ptr.add(1);
                    }
                } else if *ptr as u32 == CHAR_BACKSLASH {
                    let erc: c_int;
                    let mut errorcode: c_int = 0;
                    let mut ch: u32 = 0;
                    let esc_end_ptr: PCRE2_SPTR;

                    if ptr < ptrend.sub(1) {
                        let c = *ptr.add(1) as u32;
                        if c == CHAR_L || c == CHAR_l || c == CHAR_U || c == CHAR_u {
                            ptr = ptr.add(1);
                            break 'CONTINUE;
                        }
                    }

                    ptr = ptr.add(1); /* Must point after \ */
                    erc = _pcre2_check_escape_8(
                        &mut ptr,
                        ptrend,
                        &mut ch,
                        &mut errorcode,
                        (*code).overall_options,
                        (*code).extra_options,
                        (*code).top_bracket as u32,
                        FALSE,
                        std::ptr::null_mut(),
                    );
                    if errorcode != 0 {
                        /* errorcode from check_escape is positive, so must not be returned by
                        pcre2_substitute(). */
                        rc = PCRE2_ERROR_BADREPESCAPE;
                        break 'EXIT;
                    }

                    esc_end_ptr = ptr;
                    ptr = ptr.sub(1); /* Rewind by one, because the for-loop will increment it */

                    if erc == 0        /* Data character */
                        || erc == ESC_b /* Data character */
                        || erc == ESC_v /* Data character */
                        || erc == ESC_E
                    /* Isolated \E is ignored */
                    {
                        /* break */
                    } else if erc == ESC_Q {
                        literal = TRUE;
                    } else if erc == ESC_g {
                        /* The \g<name> form (\g<number> already handled by check_escape)

                        Don't worry about finding the matching ">". We are super, super lenient
                        about validating ${} replacements inside find_text_end(), so we certainly
                        don't need to worry about other syntax. Importantly, a \g<..> or $<...>
                        sequence can't contain a '}' character. */
                    } else {
                        if erc < 0 {
                            /* break; capture group reference */
                        } else {
                            ptr = esc_end_ptr;
                            rc = PCRE2_ERROR_BADREPESCAPE;
                            break 'EXIT;
                        }
                    }
                }
            }
            ptr = ptr.add(1);
        }

        rc = PCRE2_ERROR_REPMISSINGBRACE; /* Terminator not found */
    }

    /* EXIT: */
    *ptrptr = ptr;
    return rc;
}

/*************************************************
*           Validate group name                  *
*************************************************/

/* This function scans for a capture group name, validating it
consists of legal characters, is not empty, and does not exceed
MAX_NAME_SIZE.

Arguments:
  ptrptr    points to the pointer to the start of the text (updated)
  ptrend    end of the whole string
  utf       true if the input is UTF-encoded
  ctypes    pointer to the character types table

Returns:    TRUE if a name was read
            FALSE otherwise
*/

unsafe fn read_name_subst(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    utf: BOOL,
    ctypes: *const u8,
) -> BOOL {
    let mut ptr: PCRE2_SPTR = *ptrptr;
    let nameptr: PCRE2_SPTR = ptr;

    'FAILED: {
        if ptr >= ptrend
        /* No characters in name */
        {
            break 'FAILED;
        }

        /* We do not need to check whether the name starts with a non-digit.
        We are simply referencing names here, not defining them. */

        /* See read_name in the pcre2_compile.c for the corresponding logic
        restricting group names inside the pattern itself. */

        if utf != 0 {
            let mut c: u32;
            let mut type_: u32;

            while ptr < ptrend {
                GETCHAR!(c, ptr);
                type_ = UCD_CHARTYPE(c);
                if type_ != ucp_Nd
                    && *_pcre2_ucp_gentype_8.as_ptr().add(type_ as usize) != ucp_L
                    && c != CHAR_UNDERSCORE
                {
                    break;
                }
                ptr = ptr.add(1);
                FORWARDCHARTEST!(ptr, ptrend);
            }
        }
        /* Handle group names in non-UTF modes. */
        else {
            while ptr < ptrend
                && MAX_255!(*ptr) != 0
                && (*ctypes.add(*ptr as usize) & ctype_word) != 0
            {
                ptr = ptr.add(1);
            }
        }

        /* Check name length */

        if ptr.offset_from(nameptr) > MAX_NAME_SIZE as isize {
            break 'FAILED;
        }

        /* Subpattern names must not be empty */
        if ptr == nameptr {
            break 'FAILED;
        }

        *ptrptr = ptr;
        return TRUE;
    }

    /* FAILED: */
    *ptrptr = ptr;
    return FALSE;
}

/*************************************************
*              Case transformations              *
*************************************************/

const PCRE2_SUBSTITUTE_CASE_NONE: c_int = 0;
// 1, 2, 3 are PCRE2_SUBSTITUTE_CASE_LOWER, UPPER, TITLE_FIRST.
const PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST: c_int = 4;

#[repr(C)]
#[derive(Clone, Copy)]
struct case_state {
    to_case: c_int, /* One of PCRE2_SUBSTITUTE_CASE_xyz */
    single_char: BOOL,
}

/* Helper to guess how much a string is likely to increase in size when
case-transformed. Usually, strings don't change size at all, but some rare
characters do grow. Estimate +10%, plus another few characters.

Performing this estimation is unfortunate, but inevitable, since we can't call
the callout if we ran out of buffer space to prepare its input.

Because this estimate is inexact (and in pathological cases, underestimates the
required buffer size) we must document that when you have a
substitute_case_callout, and you are using PCRE2_SUBSTITUTE_OVERFLOW_LENGTH, you
may need more than two calls to determine the final buffer size. */

unsafe fn pessimistic_case_inflation(len: PCRE2_SIZE) -> PCRE2_SIZE {
    return (len >> 3u32) + 10;
}

/* Case transformation behaviour if no callout is passed. */

unsafe fn default_substitute_case_callout(
    mut input: PCRE2_SPTR,
    input_len: PCRE2_SIZE,
    mut output: *mut PCRE2_UCHAR,
    mut output_cap: PCRE2_SIZE,
    state: *mut case_state,
    code: *const pcre2_real_code,
) -> PCRE2_SIZE {
    let input_end: PCRE2_SPTR = input.add(input_len);
    let utf: BOOL;
    let ucp: BOOL;
    let mut temp: [PCRE2_UCHAR; 6] = [0; 6];
    let mut next_to_upper: BOOL;
    let mut rest_to_upper: BOOL;
    let single_char: BOOL;
    let mut overflow: BOOL = FALSE;
    let mut written: PCRE2_SIZE = 0;

    /* Helpful simplifying invariant: input and output are disjoint buffers.
    (The C code has a PCRE2_ASSERT here; it is a no-op in release builds.) */

    utf = ((*code).overall_options & PCRE2_UTF != 0) as BOOL;
    ucp = ((*code).overall_options & PCRE2_UCP != 0) as BOOL;

    if input_len == 0 {
        return 0;
    }

    if (*state).to_case == PCRE2_SUBSTITUTE_CASE_LOWER as c_int
        || (*state).to_case == PCRE2_SUBSTITUTE_CASE_UPPER as c_int
    {
        /* LOWER can be single_char TRUE or FALSE; UPPER only single_char FALSE */
        rest_to_upper = ((*state).to_case == PCRE2_SUBSTITUTE_CASE_UPPER as c_int) as BOOL;
        next_to_upper = rest_to_upper;
    } else if (*state).to_case == PCRE2_SUBSTITUTE_CASE_TITLE_FIRST as c_int {
        /* Can be single_char TRUE or FALSE */
        next_to_upper = TRUE;
        rest_to_upper = FALSE;
        (*state).to_case = PCRE2_SUBSTITUTE_CASE_LOWER as c_int;
    } else if (*state).to_case == PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST {
        /* Can only be single_char FALSE */
        next_to_upper = FALSE;
        rest_to_upper = TRUE;
        (*state).to_case = PCRE2_SUBSTITUTE_CASE_UPPER as c_int;
    } else {
        /* LCOV_EXCL_START */
        return 0;
        /* LCOV_EXCL_STOP */
    }

    single_char = (*state).single_char;
    if single_char != 0 {
        (*state).to_case = PCRE2_SUBSTITUTE_CASE_NONE;
    }

    while input < input_end {
        let mut ch: u32;
        let chlen: c_uint;

        GETCHARINCTEST!(ch, input, utf);

        if (utf != 0 || ucp != 0) && ch >= 128 {
            let type_: u32 = UCD_CHARTYPE(ch);
            if *_pcre2_ucp_gentype_8.as_ptr().add(type_ as usize) == ucp_L
                && type_ != (if next_to_upper != 0 { ucp_Lu } else { ucp_Ll })
            {
                ch = UCD_OTHERCASE(ch);
            }

            /* TODO This is far from correct... it doesn't support the SpecialCasing.txt
            mappings, but worse, it's not even correct for all the ordinary case
            mappings. We should add support for those (at least), and then add the
            SpecialCasing.txt mappings for Esszet and ligatures, and finally use the
            Turkish casing flag on the match context. */
        } else if MAX_255!(ch) != 0 {
            if (*(*code)
                .tables
                .add(cbits_offset)
                .add(if next_to_upper != 0 {
                    cbit_upper
                } else {
                    cbit_lower
                })
                .add((ch / 8) as usize) as u32
                & (1u32 << (ch % 8)))
                == 0
            {
                ch = *(*code).tables.add(fcc_offset).add(ch as usize) as u32;
            }
        }

        if utf != 0 {
            chlen = _pcre2_ord2utf_8(ch, temp.as_mut_ptr());
        } else {
            temp[0] = ch as PCRE2_UCHAR;
            chlen = 1;
        }

        if overflow == 0 && chlen as PCRE2_SIZE <= output_cap {
            memcpy(
                output as *mut c_void,
                temp.as_ptr() as *const c_void,
                CU2BYTES!(chlen as PCRE2_SIZE),
            );
            output = output.add(chlen as usize);
            output_cap -= chlen as PCRE2_SIZE;
        } else {
            overflow = TRUE;
        }

        if chlen as PCRE2_SIZE > !(0 as PCRE2_SIZE) - written
        /* Integer overflow */
        {
            return !(0 as PCRE2_SIZE);
        }
        written += chlen as PCRE2_SIZE;

        next_to_upper = rest_to_upper;

        /* memcpy the remainder, if only transforming a single character. */

        if single_char != 0 {
            let rest_len: PCRE2_SIZE = input_end.offset_from(input) as PCRE2_SIZE;

            if overflow == 0 && rest_len <= output_cap {
                memcpy(
                    output as *mut c_void,
                    input as *const c_void,
                    CU2BYTES!(rest_len),
                );
            }

            if rest_len > !(0 as PCRE2_SIZE) - written
            /* Integer overflow */
            {
                return !(0 as PCRE2_SIZE);
            }
            written += rest_len;

            return written;
        }
    }

    return written;
}
