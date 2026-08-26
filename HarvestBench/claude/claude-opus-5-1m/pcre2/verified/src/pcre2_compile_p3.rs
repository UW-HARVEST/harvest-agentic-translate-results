/* Translated from c_src/src/pcre2_compile.c lines 1489-2257 */

/*************************************************
*            Handle escapes                      *
*************************************************/

/* This function is called when a \ has been encountered. It either returns a
positive value for a simple escape such as \d, or 0 for a data character, which
is placed in chptr. A backreference to group n is returned as -(n+1). On
entry, ptr is pointing at the character after \. On exit, it points after the
final code unit of the escape sequence.

This function is also called from pcre2_substitute() to handle escape sequences
in replacement strings. In this case, the cb argument is NULL, and in the case
of escapes that have further processing, only sequences that define a data
character are recognised. The options argument is the final value of the
compiled pattern's options.

Arguments:
  ptrptr         points to the input position pointer
  ptrend         points to the end of the input
  chptr          points to a returned data character
  errorcodeptr   points to the errorcode variable (containing zero)
  options        the current options bits
  xoptions       the current extra options bits
  bracount       the number of capturing parentheses encountered so far
  isclass        TRUE if in a character class
  cb             compile data block or NULL when called from pcre2_substitute()

Returns:         zero => a data character
                 positive => a special escape sequence
                 negative => a numerical back reference
                 on error, errorcodeptr is set non-zero
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_check_escape_8(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    chptr: *mut u32,
    errorcodeptr: *mut c_int,
    options: u32,
    xoptions: u32,
    bracount: u32,
    isclass: BOOL,
    cb: *mut compile_block,
) -> c_int {
    let utf: BOOL = ((options & PCRE2_UTF) != 0) as BOOL;
    let mut alt_bsux: BOOL =
        (((options & PCRE2_ALT_BSUX) | (xoptions & PCRE2_EXTRA_ALT_BSUX)) != 0) as BOOL;
    let mut ptr: PCRE2_SPTR = *ptrptr;
    let mut c: u32 = 0;
    let mut cc: u32 = 0;
    let mut escape: c_int = 0;
    let mut i: c_int = 0;

    /* These three are declared in the inner block of the C function that handles
    the escapes needing further processing; they are hoisted here because the
    shared \x{ code is reached by a goto from outside that block. */

    let mut s: c_int = 0;
    let mut oldptr: PCRE2_SPTR = std::ptr::null();
    let mut overflow: BOOL = FALSE;

    /* If backslash is at the end of the string, it's an error. */

    if ptr >= ptrend {
        *errorcodeptr = ERR1;
        return 0;
    }

    GETCHARINCTEST!(c, ptr, utf); /* Get character value, increment pointer */
    *errorcodeptr = 0; /* Be optimistic */

    'exit: {
        'escape_failed_forward: {
            'come_from_nu: {
                /* Non-alphanumerics are literals, so we just leave the value in c. An
                initial value test saves a memory lookup for code points outside the
                alphanumeric range. */

                if c < ESCAPES_FIRST as u32 || c > ESCAPES_LAST as u32 {
                    /* Definitely literal */
                }
                /* Otherwise, do a table lookup. Non-zero values need little processing
                here. A positive value is a literal value for something like \n. A
                negative value is the negation of one of the ESC_ macros that is passed
                back for handling by the calling function. Some extra checking is needed
                for \N because only \N{U+dddd} is supported. If the value is zero,
                further processing is handled below. */
                else {
                    i = *escapes.as_ptr().add((c - ESCAPES_FIRST as u32) as usize) as c_int;

                    if i != 0 {
                        if i > 0 {
                            c = i as u32;
                            if c == CHAR_CR && (xoptions & PCRE2_EXTRA_ESCAPED_CR_IS_LF) != 0 {
                                c = CHAR_LF;
                            }
                        } else
                        /* Negative table entry */
                        {
                            escape = -i; /* Else return a special escape */
                            if !cb.is_null()
                                && (escape == ESC_P || escape == ESC_p || escape == ESC_X)
                            {
                                (*cb).external_flags |= PCRE2_HASBKPORX; /* Note \P, \p, or \X */
                            }

                            /* Perl supports \N{name} for character names and \N{U+dddd}
                            for numerical Unicode code points, as well as plain \N for
                            "not newline". PCRE does not support \N{name}. However, it
                            does support quantification such as \N{2,3}, so if \N{ is not
                            followed by U+dddd we check for a quantifier. */

                            if escape == ESC_N
                                && ptr < ptrend
                                && *ptr as u32 == CHAR_LEFT_CURLY_BRACKET
                            {
                                let mut p: PCRE2_SPTR = ptr.add(1);

                                /* Perl ignores spaces and tabs after { */

                                while p < ptrend
                                    && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT)
                                {
                                    p = p.add(1);
                                }

                                /* \N{U+ can be handled by the \x{ code. However, this
                                construction is not valid in EBCDIC environments because
                                it specifies a Unicode character, not a codepoint in the
                                local code. For example \N{U+0041} must be "A" in all
                                environments. Also, in Perl, \N{U+ forces Unicode casing
                                semantics for the entire pattern, so allow it only in UTF
                                (i.e. Unicode) mode. */

                                if ptrend.offset_from(p) > 1
                                    && *p as u32 == CHAR_U
                                    && *p.add(1) as u32 == CHAR_PLUS
                                {
                                    if utf != 0 {
                                        ptr = p.add(2);
                                        escape = 0; /* Not a fancy escape after all */
                                        break 'come_from_nu;
                                    }

                                    /* Improve error offset. */
                                    ptr = p.add(2);
                                    while ptr < ptrend
                                        && *xdigitab.as_ptr().add(*ptr as usize) as u32 != 0xff
                                    {
                                        ptr = ptr.add(1);
                                    }
                                    while ptr < ptrend
                                        && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT)
                                    {
                                        ptr = ptr.add(1);
                                    }
                                    if ptr < ptrend && *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
                                        ptr = ptr.add(1);
                                    }

                                    *errorcodeptr = ERR93;
                                }
                                /* Give an error in contexts where quantifiers are not
                                allowed (character classes; substitution strings). */
                                else if isclass != 0 || cb.is_null() {
                                    ptr = ptr.add(1); /* Skip over the opening brace */
                                    *errorcodeptr = ERR37;
                                }
                                /* Give an error if what follows is not a quantifier, but
                                don't override an error set by the quantifier reader (e.g.
                                number overflow). */
                                else {
                                    if read_repeat_counts(
                                        &mut p,
                                        ptrend,
                                        std::ptr::null_mut(),
                                        std::ptr::null_mut(),
                                        errorcodeptr,
                                    ) == FALSE
                                        && *errorcodeptr == 0
                                    {
                                        ptr = ptr.add(1); /* Skip over the opening brace */
                                        *errorcodeptr = ERR37;
                                    }
                                }
                            }
                        }
                    }
                    /* Escapes that need further processing, including those that are
                    unknown, have a zero entry in the lookup table. When called from
                    pcre2_substitute(), only \c, \o, and \x are recognized (\u and \U can
                    never appear as they are used for case forcing). */
                    else {
                        /* Filter calls from pcre2_substitute(). */

                        if cb.is_null() {
                            if !(c >= CHAR_0 && c <= CHAR_9)
                                && c != CHAR_c
                                && c != CHAR_o
                                && c != CHAR_x
                                && c != CHAR_g
                            {
                                *errorcodeptr = ERR3;
                                break 'exit;
                            }
                            alt_bsux = FALSE; /* Do not modify \x handling */
                        }

                        'sw: {
                            /* A number of Perl escapes are not handled by PCRE. We give
                            an explicit error. */

                            if c == CHAR_F || c == CHAR_l || c == CHAR_L {
                                *errorcodeptr = ERR37;
                            }
                            /* \u is unrecognized when neither PCRE2_ALT_BSUX nor
                            PCRE2_EXTRA_ALT_BSUX is set. Otherwise, \u must be followed by
                            exactly four hex digits or, if PCRE2_EXTRA_ALT_BSUX is set, by
                            any number of hex digits in braces. Otherwise it is a
                            lowercase u letter. This gives some compatibility with
                            ECMAScript (aka JavaScript). Unlike other braced items, white
                            space is NOT allowed. When \u{ is not followed by hex digits,
                            a special return is given because otherwise \u{ 12} (for
                            example) would be treated as u{12}. */
                            else if c == CHAR_u {
                                if alt_bsux == 0 {
                                    *errorcodeptr = ERR37;
                                } else {
                                    let mut xc: u32;

                                    if ptr >= ptrend {
                                        break 'sw;
                                    }
                                    if *ptr as u32 == CHAR_LEFT_CURLY_BRACKET
                                        && (xoptions & PCRE2_EXTRA_ALT_BSUX) != 0
                                    {
                                        let mut hptr: PCRE2_SPTR = ptr.add(1);

                                        cc = 0;
                                        loop {
                                            if !(hptr < ptrend) {
                                                break;
                                            }
                                            xc = *xdigitab.as_ptr().add(*hptr as usize) as u32;
                                            if xc == 0xff {
                                                break;
                                            }
                                            if (cc & 0xf0000000) != 0
                                            /* Test for 32-bit overflow */
                                            {
                                                *errorcodeptr = ERR77;
                                                ptr = hptr; /* Show where */
                                                break; /* *hptr != } will cause another break below */
                                            }
                                            cc = (cc << 4) | xc;
                                            hptr = hptr.add(1);
                                        }

                                        if hptr == ptr.add(1) ||   /* No hex digits */
                                           hptr >= ptrend ||       /* Hit end of input */
                                           *hptr as u32 != CHAR_RIGHT_CURLY_BRACKET
                                        /* No } terminator */
                                        {
                                            if isclass != 0 {
                                                break 'sw;
                                            } /* In a class, just treat as '\u' literal */
                                            escape = ESC_ub; /* Special return */
                                            ptr = ptr.add(1); /* Skip { */
                                            break 'sw; /* Hex escape not recognized */
                                        }

                                        c = cc; /* Accept the code point */
                                        ptr = hptr.add(1);
                                    } else
                                    /* Must be exactly 4 hex digits */
                                    {
                                        if ptrend.offset_from(ptr) < 4 {
                                            break 'sw;
                                        } /* Less than 4 chars */
                                        cc = *xdigitab.as_ptr().add(*ptr.add(0) as usize) as u32;
                                        if cc == 0xff {
                                            break 'sw;
                                        } /* Not a hex digit */
                                        xc = *xdigitab.as_ptr().add(*ptr.add(1) as usize) as u32;
                                        if xc == 0xff {
                                            break 'sw;
                                        } /* Not a hex digit */
                                        cc = (cc << 4) | xc;
                                        xc = *xdigitab.as_ptr().add(*ptr.add(2) as usize) as u32;
                                        if xc == 0xff {
                                            break 'sw;
                                        } /* Not a hex digit */
                                        cc = (cc << 4) | xc;
                                        xc = *xdigitab.as_ptr().add(*ptr.add(3) as usize) as u32;
                                        if xc == 0xff {
                                            break 'sw;
                                        } /* Not a hex digit */
                                        c = (cc << 4) | xc;
                                        ptr = ptr.add(4);
                                    }

                                    if utf != 0 {
                                        if c > 0x10ffffu32 {
                                            *errorcodeptr = ERR77;
                                        } else if c >= 0xd800
                                            && c <= 0xdfff
                                            && (xoptions & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES) == 0
                                        {
                                            *errorcodeptr = ERR73;
                                        }
                                    } else if c > MAX_NON_UTF_CHAR {
                                        *errorcodeptr = ERR77;
                                    }
                                }
                            }
                            /* \U is unrecognized unless PCRE2_ALT_BSUX or
                            PCRE2_EXTRA_ALT_BSUX is set, in which case it is an upper case
                            letter. */
                            else if c == CHAR_U {
                                if alt_bsux == 0 {
                                    *errorcodeptr = ERR37;
                                }
                            }
                            /* In a character class, \g is just a literal "g". Outside a
                            character class, \g must be followed by one of a number of
                            specific things:

                            (1) A number, either plain or braced. If positive, it is an
                            absolute backreference. If negative, it is a relative
                            backreference. This is a Perl 5.10 feature.

                            (2) Perl 5.10 also supports \g{name} as a reference to a named
                            group. This is part of Perl's movement towards a unified
                            syntax for back references. As this is synonymous with
                            \k{name}, we fudge it up by pretending it really was \k{name}.

                            (3) For Oniguruma compatibility we also support \g followed by
                            a name or a number either in angle brackets or in single
                            quotes. However, these are (possibly recursive) subroutine
                            calls, _not_ backreferences. We return the ESC_g code.

                            Summary: Return a negative number for a numerical back
                            reference (offset by 1), ESC_k for a named back reference, and
                            ESC_g for a named or numbered subroutine call.

                            The above describes the \g behaviour inside patterns. Inside
                            replacement strings (pcre2_substitute) we support only
                            \g<nameornum> for Python compatibility. Return ESG_g for the
                            named case, and -(num+1) for the numbered case. */
                            else if c == CHAR_g {
                                if isclass != 0 {
                                    break 'sw;
                                }

                                if ptr >= ptrend {
                                    *errorcodeptr = ERR57;
                                    break 'sw;
                                }

                                if cb.is_null() {
                                    let mut p: PCRE2_SPTR;
                                    /* Substitution strings */
                                    if *ptr as u32 != CHAR_LESS_THAN_SIGN {
                                        *errorcodeptr = ERR57;
                                        break 'sw;
                                    }

                                    p = ptr.add(1);

                                    if read_number(
                                        &mut p,
                                        ptrend,
                                        -1,
                                        MAX_GROUP_NUMBER,
                                        ERR61 as u32,
                                        &mut s,
                                        errorcodeptr,
                                    ) == FALSE
                                    {
                                        if *errorcodeptr == 0 {
                                            escape = ESC_g;
                                        } /* No number found */
                                        break 'sw;
                                    }

                                    if p >= ptrend || *p as u32 != CHAR_GREATER_THAN_SIGN {
                                        ptr = p;
                                        *errorcodeptr = ERR119; /* Missing terminator for number */
                                        break 'sw;
                                    }

                                    /* This is the reason that back references are returned
                                    as -(s+1) rather than just -s. In a pattern, \0 is not
                                    a back reference, but \g<0> is valid in a substitution
                                    string, so this must be representable. */
                                    ptr = p.add(1);
                                    escape = -(s + 1);
                                    break 'sw;
                                }

                                if *ptr as u32 == CHAR_LESS_THAN_SIGN
                                    || *ptr as u32 == CHAR_APOSTROPHE
                                {
                                    escape = ESC_g;
                                    break 'sw;
                                }

                                /* If there is a brace delimiter, try to read a numerical
                                reference. If there isn't one, assume we have a name and
                                treat it as \k. */

                                if *ptr as u32 == CHAR_LEFT_CURLY_BRACKET {
                                    let mut p: PCRE2_SPTR = ptr.add(1);

                                    while p < ptrend
                                        && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT)
                                    {
                                        p = p.add(1);
                                    }
                                    if read_number(
                                        &mut p,
                                        ptrend,
                                        bracount as i32,
                                        MAX_GROUP_NUMBER,
                                        ERR61 as u32,
                                        &mut s,
                                        errorcodeptr,
                                    ) == FALSE
                                    {
                                        if *errorcodeptr == 0 {
                                            escape = ESC_k;
                                        } /* No number found */
                                        break 'sw;
                                    }
                                    while p < ptrend
                                        && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT)
                                    {
                                        p = p.add(1);
                                    }

                                    if p >= ptrend || *p as u32 != CHAR_RIGHT_CURLY_BRACKET {
                                        ptr = p;
                                        *errorcodeptr = ERR119; /* Missing terminator for number */
                                        break 'sw;
                                    }
                                    ptr = p.add(1);
                                }
                                /* Read an undelimited number */
                                else {
                                    if read_number(
                                        &mut ptr,
                                        ptrend,
                                        bracount as i32,
                                        MAX_GROUP_NUMBER,
                                        ERR61 as u32,
                                        &mut s,
                                        errorcodeptr,
                                    ) == FALSE
                                    {
                                        if *errorcodeptr == 0 {
                                            *errorcodeptr = ERR57;
                                        } /* No number found */
                                        break 'sw;
                                    }
                                }

                                if s <= 0 {
                                    *errorcodeptr = ERR15;
                                    break 'sw;
                                }

                                escape = -(s + 1);
                            }
                            /* The handling of escape sequences consisting of a string of
                            digits starting with one that is not zero is not
                            straightforward. Perl has changed over the years. Nowadays
                            \g{} for backreferences and \o{} for octal are recommended to
                            avoid the ambiguities in the old syntax.

                            Outside a character class, the digits are read as a decimal
                            number. If the number is less than 10, or if there are that
                            many previous extracting left brackets, it is a back
                            reference. Otherwise, up to three octal digits are read to
                            form an escaped character code. Thus \123 is likely to be
                            octal 123 (cf \0123, which is octal 012 followed by the
                            literal 3). This is the "Perl style" of handling ambiguous
                            octal/backrefences such as \12.

                            There is an alternative disambiguation strategy, selected by
                            PCRE2_EXTRA_PYTHON_OCTAL, which follows Python's behaviour. An
                            octal must have either a leading zero, or exactly three octal
                            digits; otherwise it's a backreference. The disambiguation is
                            stable, and does not depend on how many capture groups are
                            defined (it's simply an invalid backreference if there is no
                            corresponding capture group). Additionally, octal values above
                            \377 (\xff) are rejected.

                            Inside a character class, \ followed by a digit is always
                            either a literal 8 or 9 or an octal number. */
                            else if (c >= CHAR_1 && c <= CHAR_9) || c == CHAR_0 {
                                if c != CHAR_0 {
                                    /* case CHAR_1 ... case CHAR_9 */

                                    if isclass != 0 {
                                        /* Fall through to octal handling; never a
                                        backreference inside a class. */
                                    } else if (xoptions & PCRE2_EXTRA_PYTHON_OCTAL) != 0 {
                                        /* Python-style disambiguation. */
                                        if *ptr.sub(1) as u32 <= CHAR_7
                                            && ptr.add(1) < ptrend
                                            && *ptr.add(0) as u32 >= CHAR_0
                                            && *ptr.add(0) as u32 <= CHAR_7
                                            && *ptr.add(1) as u32 >= CHAR_0
                                            && *ptr.add(1) as u32 <= CHAR_7
                                        {
                                            /* We peeked a three-digit octal, so fall through */
                                        } else {
                                            /* We are at a digit, so the only possible
                                            error from read_number() is a number that is
                                            too large. */
                                            ptr = ptr.sub(1); /* Back to the digit */

                                            if read_number(
                                                &mut ptr,
                                                ptrend,
                                                -1,
                                                MAX_GROUP_NUMBER,
                                                0,
                                                &mut s,
                                                errorcodeptr,
                                            ) == FALSE
                                            {
                                                *errorcodeptr = ERR61;
                                                break 'sw;
                                            }

                                            escape = -(s + 1);
                                            break 'sw;
                                        }
                                    } else {
                                        /* Perl-style disambiguation. */
                                        oldptr = ptr;
                                        ptr = ptr.sub(1); /* Back to the digit */

                                        /* As we know we are at a digit, the only possible
                                        error from read_number() is a number that is too
                                        large to be a group number. Because that number
                                        might be still valid if read as an octal,
                                        errorcodeptr is not set on failure and therefore a
                                        sentinel value of INT_MAX is used instead of the
                                        original value, and will be used later to properly
                                        set the error, if not falling through. */

                                        if read_number(
                                            &mut ptr,
                                            ptrend,
                                            -1,
                                            MAX_GROUP_NUMBER,
                                            0,
                                            &mut s,
                                            errorcodeptr,
                                        ) == FALSE
                                        {
                                            s = c_int::MAX;
                                        }

                                        /* \1 to \9 are always back references. \8x and
                                        \9x are too; \1x to \7x are octal escapes if there
                                        are not that many previous captures. */

                                        if s < 10 || c >= CHAR_8 || (s as u32) <= bracount {
                                            /* s > MAX_GROUP_NUMBER should not be possible
                                            because of read_number(), but we keep it just
                                            to be safe and because it will also catch the
                                            sentinel value that was set on failure by that
                                            function. */

                                            if (s as u32) > MAX_GROUP_NUMBER {
                                                /* PCRE2_ASSERT(s == INT_MAX); */
                                                *errorcodeptr = ERR61;
                                            } else {
                                                escape = -(s + 1);
                                            } /* Indicates a back reference */
                                            break 'sw;
                                        }

                                        ptr = oldptr; /* Put the pointer back and fall through */
                                    }

                                    /* Handle a digit following \ when the number is not a
                                    back reference, or we are within a character class. If
                                    the first digit is 8 or 9, Perl used to generate a
                                    binary zero and then treat the digit as a following
                                    literal. At least by Perl 5.18 this changed so as not
                                    to insert the binary zero. */

                                    if c >= CHAR_8 {
                                        break 'sw;
                                    }

                                    /* Fall through */
                                }

                                /* case CHAR_0: */

                                /* \0 always starts an octal number, but we may drop
                                through to here with a larger first octal digit. The
                                original code used just to take the least significant 8
                                bits of octal numbers (I think this is what early Perls
                                used to do). Nowadays we allow for larger numbers in UTF-8
                                mode and 16/32-bit mode, but no more than 3 octal digits. */

                                c -= CHAR_0;
                                loop {
                                    let t = i;
                                    i += 1;
                                    if !(t < 2
                                        && ptr < ptrend
                                        && *ptr as u32 >= CHAR_0
                                        && *ptr as u32 <= CHAR_7)
                                    {
                                        break;
                                    }
                                    c = c * 8 + {
                                        let t2 = *ptr;
                                        ptr = ptr.add(1);
                                        t2 as u32
                                    } - CHAR_0;
                                }
                                if c > 0xff {
                                    if (xoptions & PCRE2_EXTRA_PYTHON_OCTAL) != 0 {
                                        *errorcodeptr = ERR102;
                                    } else if utf == 0 {
                                        *errorcodeptr = ERR51;
                                    }
                                }

                                /* PCRE2_EXTRA_NO_BS0 disables the NUL escape '\0' but
                                doesn't affect two- or three-character octal escapes \00
                                and \000, nor \x00. */

                                if (xoptions & PCRE2_EXTRA_NO_BS0) != 0 && c == 0 && i == 1 {
                                    *errorcodeptr = ERR98;
                                }
                            }
                            /* \o is a relatively new Perl feature, supporting a more
                            general way of specifying character codes in octal. The only
                            supported form is \o{ddd}, with optional spaces or tabs after
                            { and before }. */
                            else if c == CHAR_o {
                                if ptr >= ptrend || *ptr as u32 != CHAR_LEFT_CURLY_BRACKET {
                                    *errorcodeptr = ERR55;
                                    break 'sw;
                                }
                                ptr = ptr.add(1);

                                while ptr < ptrend
                                    && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT)
                                {
                                    ptr = ptr.add(1);
                                }
                                if ptr >= ptrend || *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
                                    *errorcodeptr = ERR78;
                                    break 'sw;
                                }

                                c = 0;
                                overflow = FALSE;
                                while ptr < ptrend && *ptr as u32 >= CHAR_0 && *ptr as u32 <= CHAR_7
                                {
                                    cc = {
                                        let t = *ptr;
                                        ptr = ptr.add(1);
                                        t as u32
                                    };
                                    if c == 0 && cc == CHAR_0 {
                                        continue;
                                    } /* Leading zeroes */
                                    c = (c << 3) + (cc - CHAR_0);
                                    if c > (if utf != 0 { 0x10ffffu32 } else { 0xffu32 }) {
                                        overflow = TRUE;
                                        break;
                                    }
                                }

                                while ptr < ptrend
                                    && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT)
                                {
                                    ptr = ptr.add(1);
                                }

                                if overflow != 0 {
                                    while ptr < ptrend
                                        && *ptr as u32 >= CHAR_0
                                        && *ptr as u32 <= CHAR_7
                                    {
                                        ptr = ptr.add(1);
                                    }
                                    *errorcodeptr = ERR34;
                                } else if utf != 0
                                    && c >= 0xd800
                                    && c <= 0xdfff
                                    && (xoptions & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES) == 0
                                {
                                    *errorcodeptr = ERR73;
                                } else if ptr < ptrend && *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
                                    ptr = ptr.add(1);
                                } else {
                                    *errorcodeptr = ERR64;
                                    break 'escape_failed_forward;
                                }
                            }
                            /* When PCRE2_ALT_BSUX or PCRE2_EXTRA_ALT_BSUX is set, \x must
                            be followed by two hexadecimal digits. Otherwise it is a
                            lowercase x letter. */
                            else if c == CHAR_x {
                                if alt_bsux != 0 {
                                    let xc: u32;
                                    if ptrend.offset_from(ptr) < 2 {
                                        break 'sw;
                                    } /* Less than 2 characters */
                                    cc = *xdigitab.as_ptr().add(*ptr.add(0) as usize) as u32;
                                    if cc == 0xff {
                                        break 'sw;
                                    } /* Not a hex digit */
                                    xc = *xdigitab.as_ptr().add(*ptr.add(1) as usize) as u32;
                                    if xc == 0xff {
                                        break 'sw;
                                    } /* Not a hex digit */
                                    c = (cc << 4) | xc;
                                    ptr = ptr.add(2);
                                }
                                /* Handle \x in Perl's style. \x{ddd} is a character code
                                which can be greater than 0xff in UTF-8 or non-8bit mode,
                                but only if the ddd are hex digits. If not, { used to be
                                treated as a data character. However, Perl seems to read
                                hex digits up to the first non-such, and ignore the rest,
                                so that, for example \x{zz} matches a binary zero. This
                                seems crazy, so PCRE now gives an error. */
                                else {
                                    if ptr < ptrend && *ptr as u32 == CHAR_LEFT_CURLY_BRACKET {
                                        ptr = ptr.add(1);
                                        while ptr < ptrend
                                            && (*ptr as u32 == CHAR_SPACE
                                                || *ptr as u32 == CHAR_HT)
                                        {
                                            ptr = ptr.add(1);
                                        }

                                        /* The rest of the \x{...} processing is the code
                                        that follows the COME_FROM_NU label, which is
                                        shared with \N{U+...}. */
                                        break 'come_from_nu;
                                    }
                                    /* Read a up to two hex digits after \x */
                                    else {
                                        /* Perl has the surprising/broken behaviour that \x
                                        without following hex digits is treated as an
                                        escape for NUL. Their source code laments this but
                                        keeps it for backwards compatibility. A warning is
                                        printed when "use warnings" is enabled. Because we
                                        don't have warnings, we simply forbid it. */
                                        if ptr >= ptrend || {
                                            cc = *xdigitab.as_ptr().add(*ptr as usize) as u32;
                                            cc == 0xff
                                        } {
                                            /* Not a hex digit */
                                            *errorcodeptr = ERR78;
                                            break 'sw;
                                        }
                                        ptr = ptr.add(1);
                                        c = cc;

                                        /* With "use re 'strict'" Perl actually requires
                                        exactly two digits (error for \x, \xA and \xAAA).
                                        While \x was already rejected, this seems overly
                                        strict, and there seems little incentive to align
                                        with that, given the backwards-compatibility cost.

                                        For comparison, note that other engines disagree.
                                        For example:
                                          - Java allows 1 or 2 hex digits. Error if 0
                                            digits. No error if >2 digits
                                          - .NET requires 2 hex digits. Error if 0, 1
                                            digits. No error if >2 digits. */
                                        if ptr >= ptrend || {
                                            cc = *xdigitab.as_ptr().add(*ptr as usize) as u32;
                                            cc == 0xff
                                        } {
                                            break 'sw;
                                        } /* Not a hex digit */
                                        ptr = ptr.add(1);
                                        c = (c << 4) | cc;
                                    } /* End of \xdd handling */
                                } /* End of Perl-style \x handling */
                            }
                            /* The handling of \c is different in ASCII and EBCDIC
                            environments. In an ASCII (or Unicode) environment, an error
                            is given if the character following \c is not a printable
                            ASCII character. Otherwise, the following character is
                            upper-cased if it is a letter, and after that the 0x40 bit is
                            flipped. The result is the value of the escape. */
                            else if c == CHAR_c {
                                if ptr >= ptrend {
                                    *errorcodeptr = ERR2;
                                    break 'sw;
                                }
                                c = *ptr as u32;
                                if c >= CHAR_a && c <= CHAR_z {
                                    c = c - 32; /* UPPER_CASE(c) */
                                }

                                /* Handle \c in an ASCII/Unicode environment. */

                                if c < 32 || c > 126
                                /* Excludes all non-printable ASCII */
                                {
                                    *errorcodeptr = ERR68;
                                    break 'escape_failed_forward;
                                }
                                c ^= 0x40;

                                ptr = ptr.add(1);
                            }
                            /* Any other alphanumeric following \ is an error. Perl gives
                            an error only if in warning mode, but PCRE doesn't have a
                            warning mode. */
                            else {
                                *errorcodeptr = ERR3;
                            }
                        } /* End of switch */
                    }
                }

                /* Set the pointer to the next character before returning. */

                break 'exit; /* goto EXIT */
            }

            /* COME_FROM_NU: shared by \x{...} and \N{U+...} */

            if ptr >= ptrend || *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
                *errorcodeptr = ERR78;
                break 'exit;
            }
            c = 0;
            overflow = FALSE;

            while ptr < ptrend {
                cc = *xdigitab.as_ptr().add(*ptr as usize) as u32;
                if cc == 0xff {
                    break;
                }
                ptr = ptr.add(1);
                if c == 0 && cc == 0 {
                    continue;
                } /* Leading zeroes */
                c = (c << 4) | cc;
                if (utf != 0 && c > 0x10ffffu32) || (utf == 0 && c > MAX_NON_UTF_CHAR) {
                    overflow = TRUE;
                    break;
                }
            }

            /* Perl ignores spaces and tabs before } */

            while ptr < ptrend && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT) {
                ptr = ptr.add(1);
            }

            /* On overflow, skip remaining hex digits */

            if overflow != 0 {
                while ptr < ptrend && *xdigitab.as_ptr().add(*ptr as usize) as u32 != 0xff {
                    ptr = ptr.add(1);
                }
                *errorcodeptr = ERR34;
            } else if utf != 0
                && c >= 0xd800
                && c <= 0xdfff
                && (xoptions & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES) == 0
            {
                *errorcodeptr = ERR73;
            } else if ptr < ptrend && *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
                ptr = ptr.add(1);
            }
            /* If the sequence of hex digits (followed by optional space) does not end
            with '}', give an error. We used just to recognize this construct and fall
            through to the normal \x handling, but nowadays Perl gives an error, which
            seems much more sensible, so we do too. */
            else {
                *errorcodeptr = ERR67;
                break 'escape_failed_forward;
            }

            break 'exit; /* End of case CHAR_x -> EXIT */
        }

        /* ESCAPE_FAILED_FORWARD: some errors need to indicate the next character. */

        ptr = ptr.add(1);
        if utf != 0 {
            FORWARDCHARTEST!(ptr, ptrend);
        }
        /* goto EXIT */
    }

    /* EXIT: */

    *ptrptr = ptr;
    *chptr = c;
    escape
}
