/* Translated from c_src/src/pcre2_study.c lines 1092-1916 */

/*************************************************
*      Create bitmap of starting code units      *
*************************************************/

/* This function scans a compiled unanchored expression recursively and
attempts to build a bitmap of the set of possible starting code units whose
values are less than 256. In 16-bit and 32-bit mode, values above 255 all cause
the 255 bit to be set. When calling set[_not]_type_bits() in UTF-8 (sic) mode
we pass a value of 16 rather than 32 as the final argument. (See comments in
those functions for the reason.)

The SSB_CONTINUE return is useful for parenthesized groups in patterns such as
(a*)b where the group provides some optional starting code units but scanning
must continue at the outer level to find at least one mandatory code unit. At
the outermost level, this function fails unless the result is SSB_DONE.

We restrict recursion (for nested groups) to 1000 to avoid stack overflow
issues.

Arguments:
  re           points to the compiled regex block
  code         points to an expression
  utf          TRUE if in UTF mode
  ucp          TRUE if in UCP mode
  depthptr     pointer to recurse depth

Returns:       SSB_FAIL     => Failed to find any starting code units
               SSB_DONE     => Found mandatory starting code units
               SSB_CONTINUE => Found optional starting code units
               SSB_UNKNOWN  => Hit an unrecognized opcode
               SSB_TOODEEP  => Recursion is too deep
*/

unsafe fn set_start_bits(
    re: *mut pcre2_real_code,
    code: PCRE2_SPTR,
    utf: BOOL,
    ucp: BOOL,
    depthptr: *mut c_int,
) -> c_int {
    let mut c: u32;
    let mut yield_: c_int = SSB_DONE as c_int;

    let table_limit: c_int = if utf != 0 { 16 } else { 32 };

    let mut code: PCRE2_SPTR = code;

    *depthptr += 1;
    if *depthptr > 1000 {
        return SSB_TOODEEP as c_int;
    }

    loop {
        let mut try_next: BOOL = TRUE;
        let mut tcode: PCRE2_SPTR = code.add(1 + LINK_SIZE);

        if *code as u32 == OP_CBRA
            || *code as u32 == OP_SCBRA
            || *code as u32 == OP_CBRAPOS
            || *code as u32 == OP_SCBRAPOS
        {
            tcode = tcode.add(IMM2_SIZE);
        }

        'try_next_loop: while try_next != 0
        /* Loop for items in this branch */
        {
            let mut classmap: *const u8 = std::ptr::null();

            'end_case: {
                'handle_classmap: {
                    'class_case: {
                        'nclass_case: {
                            'typestar_case: {
                                'chari_case: {
                                    'char_case: {
                                        'bra_case: {
                                            match *tcode as u32 {
                                                /* Fail for a valid opcode that implies no starting bits. */
                                                OP_ACCEPT
                                                | OP_ASSERT_ACCEPT
                                                | OP_ALLANY
                                                | OP_ANY
                                                | OP_ANYBYTE
                                                | OP_CIRCM
                                                | OP_CLOSE
                                                | OP_COMMIT
                                                | OP_COMMIT_ARG
                                                | OP_COND
                                                | OP_CREF
                                                | OP_FALSE
                                                | OP_TRUE
                                                | OP_DNCREF
                                                | OP_DNREF
                                                | OP_DNREFI
                                                | OP_DNRREF
                                                | OP_DOLL
                                                | OP_DOLLM
                                                | OP_END
                                                | OP_EOD
                                                | OP_EODN
                                                | OP_EXTUNI
                                                | OP_FAIL
                                                | OP_MARK
                                                | OP_NOT
                                                | OP_NOTEXACT
                                                | OP_NOTEXACTI
                                                | OP_NOTI
                                                | OP_NOTMINPLUS
                                                | OP_NOTMINPLUSI
                                                | OP_NOTMINQUERY
                                                | OP_NOTMINQUERYI
                                                | OP_NOTMINSTAR
                                                | OP_NOTMINSTARI
                                                | OP_NOTMINUPTO
                                                | OP_NOTMINUPTOI
                                                | OP_NOTPLUS
                                                | OP_NOTPLUSI
                                                | OP_NOTPOSPLUS
                                                | OP_NOTPOSPLUSI
                                                | OP_NOTPOSQUERY
                                                | OP_NOTPOSQUERYI
                                                | OP_NOTPOSSTAR
                                                | OP_NOTPOSSTARI
                                                | OP_NOTPOSUPTO
                                                | OP_NOTPOSUPTOI
                                                | OP_NOTPROP
                                                | OP_NOTQUERY
                                                | OP_NOTQUERYI
                                                | OP_NOTSTAR
                                                | OP_NOTSTARI
                                                | OP_NOTUPTO
                                                | OP_NOTUPTOI
                                                | OP_NOT_HSPACE
                                                | OP_NOT_VSPACE
                                                | OP_PRUNE
                                                | OP_PRUNE_ARG
                                                | OP_RECURSE
                                                | OP_REF
                                                | OP_REFI
                                                | OP_REVERSE
                                                | OP_VREVERSE
                                                | OP_RREF
                                                | OP_SCOND
                                                | OP_SET_SOM
                                                | OP_SKIP
                                                | OP_SKIP_ARG
                                                | OP_SOD
                                                | OP_SOM
                                                | OP_THEN
                                                | OP_THEN_ARG => {
                                                    return SSB_FAIL as c_int;
                                                }

                                                /* OP_CIRC happens only at the start of an anchored branch
                                                (multiline ^ uses OP_CIRCM). Skip over it. */
                                                OP_CIRC => {
                                                    tcode = tcode.add(
                                                        *_pcre2_OP_lengths_8
                                                            .as_ptr()
                                                            .add(OP_CIRC as usize)
                                                            as usize,
                                                    );
                                                }

                                                /* A "real" property test implies no starting bits, but the
                                                fake property PT_CLIST identifies a list of characters. These
                                                lists are short, as they are used for characters with more
                                                than one "other case", so there is no point in recognizing
                                                them for OP_NOTPROP. */
                                                OP_PROP => {
                                                    if *tcode.add(1) as u32 != PT_CLIST {
                                                        return SSB_FAIL as c_int;
                                                    }
                                                    {
                                                        let mut p: *const u32 =
                                                            _pcre2_ucd_caseless_sets_8
                                                                .as_ptr()
                                                                .add(*tcode.add(2) as usize);
                                                        loop {
                                                            c = {
                                                                let t = *p;
                                                                p = p.add(1);
                                                                t
                                                            };
                                                            if !(c < NOTACHAR) {
                                                                break;
                                                            }
                                                            if utf != 0 {
                                                                let mut buff: [PCRE2_UCHAR; 6] =
                                                                    [0; 6];
                                                                _pcre2_ord2utf_8(
                                                                    c,
                                                                    buff.as_mut_ptr(),
                                                                );
                                                                c = buff[0] as u32;
                                                            }
                                                            if c > 0xff {
                                                                SET_BIT!(re, 0xff);
                                                            } else {
                                                                SET_BIT!(re, c);
                                                            }
                                                        }
                                                    }
                                                    try_next = FALSE;
                                                }

                                                /* We can ignore word boundary tests. */
                                                OP_WORD_BOUNDARY
                                                | OP_NOT_WORD_BOUNDARY
                                                | OP_UCP_WORD_BOUNDARY
                                                | OP_NOT_UCP_WORD_BOUNDARY => {
                                                    tcode = tcode.add(1);
                                                }

                                                /* For a positive lookahead assertion, inspect what
                                                immediately follows, ignoring intermediate assertions and
                                                callouts. If the next item is one that sets a mandatory
                                                character, skip this assertion. Otherwise, treat it the same
                                                as other bracket groups. */
                                                OP_ASSERT | OP_ASSERT_NA => {
                                                    let mut ncode: PCRE2_SPTR =
                                                        tcode.add(GET!(tcode, 1) as usize);
                                                    while *ncode as u32 == OP_ALT {
                                                        ncode = ncode.add(GET!(ncode, 1) as usize);
                                                    }
                                                    ncode = ncode.add(1 + LINK_SIZE);

                                                    /* Skip irrelevant items */

                                                    let mut done: BOOL = FALSE;
                                                    while done == 0 {
                                                        match *ncode as u32 {
                                                            OP_ASSERT
                                                            | OP_ASSERT_NOT
                                                            | OP_ASSERTBACK
                                                            | OP_ASSERTBACK_NOT
                                                            | OP_ASSERT_NA
                                                            | OP_ASSERTBACK_NA
                                                            | OP_ASSERT_SCS => {
                                                                ncode = ncode
                                                                    .add(GET!(ncode, 1) as usize);
                                                                while *ncode as u32 == OP_ALT {
                                                                    ncode = ncode.add(
                                                                        GET!(ncode, 1) as usize
                                                                    );
                                                                }
                                                                ncode = ncode.add(1 + LINK_SIZE);
                                                            }

                                                            OP_WORD_BOUNDARY
                                                            | OP_NOT_WORD_BOUNDARY
                                                            | OP_UCP_WORD_BOUNDARY
                                                            | OP_NOT_UCP_WORD_BOUNDARY => {
                                                                ncode = ncode.add(1);
                                                            }

                                                            OP_CALLOUT => {
                                                                ncode = ncode.add(
                                                                    *_pcre2_OP_lengths_8
                                                                        .as_ptr()
                                                                        .add(OP_CALLOUT as usize)
                                                                        as usize,
                                                                );
                                                            }

                                                            OP_CALLOUT_STR => {
                                                                ncode = ncode.add(GET!(
                                                                    ncode,
                                                                    1 + 2 * LINK_SIZE
                                                                ) as usize);
                                                            }

                                                            _ => {
                                                                done = TRUE;
                                                            }
                                                        }
                                                    }

                                                    /* Now check the next significant item. */

                                                    'assert_switch: {
                                                        match *ncode as u32 {
                                                            OP_PROP => {
                                                                if *ncode.add(1) as u32 != PT_CLIST
                                                                {
                                                                    break 'assert_switch;
                                                                }
                                                                /* Fall through */
                                                            }
                                                            OP_ANYNL
                                                            | OP_CHAR
                                                            | OP_CHARI
                                                            | OP_EXACT
                                                            | OP_EXACTI
                                                            | OP_HSPACE
                                                            | OP_MINPLUS
                                                            | OP_MINPLUSI
                                                            | OP_PLUS
                                                            | OP_PLUSI
                                                            | OP_POSPLUS
                                                            | OP_POSPLUSI
                                                            | OP_VSPACE
                                                            /* Note that these types will only be present in
                                                            non-UCP mode. */
                                                            | OP_DIGIT
                                                            | OP_NOT_DIGIT
                                                            | OP_WORDCHAR
                                                            | OP_NOT_WORDCHAR
                                                            | OP_WHITESPACE
                                                            | OP_NOT_WHITESPACE => {}

                                                            _ => {
                                                                break 'assert_switch;
                                                            }
                                                        }
                                                        tcode = ncode;
                                                        continue 'try_next_loop; /* With the following significant opcode */
                                                    }
                                                    /* Fall through */
                                                    break 'bra_case;
                                                }

                                                /* For a group bracket or a positive assertion without an
                                                immediately following mandatory setting, recurse to set bits
                                                from within the subpattern. If it can't find anything, we have
                                                to give up. If it finds some mandatory character(s), we are
                                                done for this branch. Otherwise, carry on scanning after the
                                                subpattern. */
                                                OP_BRA | OP_SBRA | OP_CBRA | OP_SCBRA | OP_BRAPOS
                                                | OP_SBRAPOS | OP_CBRAPOS | OP_SCBRAPOS | OP_ONCE
                                                | OP_SCRIPT_RUN => {
                                                    break 'bra_case;
                                                }

                                                /* If we hit ALT or KET, it means we haven't found anything
                                                mandatory in this branch, though we might have found something
                                                optional. For ALT, we continue with the next alternative, but
                                                we have to arrange that the final result from subpattern is
                                                SSB_CONTINUE rather than SSB_DONE. For KET, return
                                                SSB_CONTINUE: if this is the top level, that indicates
                                                failure, but after a nested subpattern, it causes scanning to
                                                continue. */
                                                OP_ALT => {
                                                    yield_ = SSB_CONTINUE as c_int;
                                                    try_next = FALSE;
                                                }

                                                OP_KET | OP_KETRMAX | OP_KETRMIN | OP_KETRPOS => {
                                                    return SSB_CONTINUE as c_int;
                                                }

                                                /* Skip over callout */
                                                OP_CALLOUT => {
                                                    tcode = tcode.add(
                                                        *_pcre2_OP_lengths_8
                                                            .as_ptr()
                                                            .add(OP_CALLOUT as usize)
                                                            as usize,
                                                    );
                                                }

                                                OP_CALLOUT_STR => {
                                                    tcode = tcode
                                                        .add(GET!(tcode, 1 + 2 * LINK_SIZE)
                                                            as usize);
                                                }

                                                /* Skip over lookbehind, negative lookahead, and scan
                                                substring assertions */
                                                OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT
                                                | OP_ASSERTBACK_NA | OP_ASSERT_SCS => {
                                                    loop {
                                                        tcode =
                                                            tcode.add(GET!(tcode, 1) as usize);
                                                        if !(*tcode as u32 == OP_ALT) {
                                                            break;
                                                        }
                                                    }
                                                    tcode = tcode.add(1 + LINK_SIZE);
                                                }

                                                /* BRAZERO does the bracket, but carries on. */
                                                OP_BRAZERO | OP_BRAMINZERO | OP_BRAPOSZERO => {
                                                    tcode = tcode.add(1);
                                                    let rc: c_int =
                                                        set_start_bits(re, tcode, utf, ucp, depthptr);
                                                    if rc == SSB_FAIL as c_int
                                                        || rc == SSB_UNKNOWN as c_int
                                                        || rc == SSB_TOODEEP as c_int
                                                    {
                                                        return rc;
                                                    }
                                                    loop {
                                                        tcode =
                                                            tcode.add(GET!(tcode, 1) as usize);
                                                        if !(*tcode as u32 == OP_ALT) {
                                                            break;
                                                        }
                                                    }
                                                    tcode = tcode.add(1 + LINK_SIZE);
                                                }

                                                /* SKIPZERO skips the bracket. */
                                                OP_SKIPZERO => {
                                                    tcode = tcode.add(1);
                                                    loop {
                                                        tcode =
                                                            tcode.add(GET!(tcode, 1) as usize);
                                                        if !(*tcode as u32 == OP_ALT) {
                                                            break;
                                                        }
                                                    }
                                                    tcode = tcode.add(1 + LINK_SIZE);
                                                }

                                                /* Single-char * or ? sets the bit and tries the next item */
                                                OP_STAR | OP_MINSTAR | OP_POSSTAR | OP_QUERY
                                                | OP_MINQUERY | OP_POSQUERY => {
                                                    tcode = set_table_bit(
                                                        re,
                                                        tcode.add(1),
                                                        FALSE,
                                                        utf,
                                                        ucp,
                                                    );
                                                }

                                                OP_STARI | OP_MINSTARI | OP_POSSTARI | OP_QUERYI
                                                | OP_MINQUERYI | OP_POSQUERYI => {
                                                    tcode = set_table_bit(
                                                        re,
                                                        tcode.add(1),
                                                        TRUE,
                                                        utf,
                                                        ucp,
                                                    );
                                                }

                                                /* Single-char upto sets the bit and tries the next */
                                                OP_UPTO | OP_MINUPTO | OP_POSUPTO => {
                                                    tcode = set_table_bit(
                                                        re,
                                                        tcode.add(1 + IMM2_SIZE),
                                                        FALSE,
                                                        utf,
                                                        ucp,
                                                    );
                                                }

                                                OP_UPTOI | OP_MINUPTOI | OP_POSUPTOI => {
                                                    tcode = set_table_bit(
                                                        re,
                                                        tcode.add(1 + IMM2_SIZE),
                                                        TRUE,
                                                        utf,
                                                        ucp,
                                                    );
                                                }

                                                /* At least one single char sets the bit and stops */
                                                OP_EXACT => {
                                                    tcode = tcode.add(IMM2_SIZE);
                                                    /* Fall through */
                                                    break 'char_case;
                                                }

                                                OP_CHAR | OP_PLUS | OP_MINPLUS | OP_POSPLUS => {
                                                    break 'char_case;
                                                }

                                                OP_EXACTI => {
                                                    tcode = tcode.add(IMM2_SIZE);
                                                    /* Fall through */
                                                    break 'chari_case;
                                                }

                                                OP_CHARI | OP_PLUSI | OP_MINPLUSI | OP_POSPLUSI => {
                                                    break 'chari_case;
                                                }

                                                /* Special spacing and line-terminating items. These
                                                recognize specific lists of characters. The difference
                                                between VSPACE and ANYNL is that the latter can match the
                                                two-character CRLF sequence, but that is not relevant for
                                                finding the first character, so their code here is
                                                identical. */
                                                OP_HSPACE => {
                                                    SET_BIT!(re, CHAR_HT);
                                                    SET_BIT!(re, CHAR_SPACE);

                                                    /* For the 8-bit library in UTF-8 mode, set the bits for
                                                    the first code units of horizontal space characters. */

                                                    if utf != 0 {
                                                        SET_BIT!(re, 0xC2); /* For U+00A0 */
                                                        SET_BIT!(re, 0xE1); /* For U+1680, U+180E */
                                                        SET_BIT!(re, 0xE2); /* For U+2000 - U+200A, U+202F, U+205F */
                                                        SET_BIT!(re, 0xE3); /* For U+3000 */
                                                    }
                                                    /* For the 8-bit library not in UTF-8 mode, set the bit
                                                    for NBSP. */
                                                    else {
                                                        SET_BIT!(re, CHAR_NBSP);
                                                    }

                                                    try_next = FALSE;
                                                }

                                                OP_ANYNL | OP_VSPACE => {
                                                    SET_BIT!(re, CHAR_LF);
                                                    SET_BIT!(re, CHAR_VT);
                                                    SET_BIT!(re, CHAR_FF);
                                                    SET_BIT!(re, CHAR_CR);

                                                    /* For the 8-bit library in UTF-8 mode, set the bits for
                                                    the first code units of vertical space characters. */

                                                    if utf != 0 {
                                                        SET_BIT!(re, 0xC2); /* For U+0085 (NEL) */
                                                        SET_BIT!(re, 0xE2); /* For U+2028, U+2029 */
                                                    }
                                                    /* For the 8-bit library not in UTF-8 mode, set the bit
                                                    for NEL. */
                                                    else {
                                                        SET_BIT!(re, CHAR_NEL);
                                                    }

                                                    try_next = FALSE;
                                                }

                                                /* Single character types set the bits and stop. Note that if
                                                PCRE2_UCP is set, we do not see these opcodes because \d etc
                                                are converted to properties. Therefore, these apply in the
                                                case when only characters less than 256 are recognized to
                                                match the types. */
                                                OP_NOT_DIGIT => {
                                                    set_nottype_bits(
                                                        re,
                                                        cbit_digit as c_int,
                                                        table_limit as c_uint,
                                                    );
                                                    try_next = FALSE;
                                                }

                                                OP_DIGIT => {
                                                    set_type_bits(
                                                        re,
                                                        cbit_digit as c_int,
                                                        table_limit as c_uint,
                                                    );
                                                    try_next = FALSE;
                                                }

                                                OP_NOT_WHITESPACE => {
                                                    set_nottype_bits(
                                                        re,
                                                        cbit_space as c_int,
                                                        table_limit as c_uint,
                                                    );
                                                    try_next = FALSE;
                                                }

                                                OP_WHITESPACE => {
                                                    set_type_bits(
                                                        re,
                                                        cbit_space as c_int,
                                                        table_limit as c_uint,
                                                    );
                                                    try_next = FALSE;
                                                }

                                                OP_NOT_WORDCHAR => {
                                                    set_nottype_bits(
                                                        re,
                                                        cbit_word as c_int,
                                                        table_limit as c_uint,
                                                    );
                                                    try_next = FALSE;
                                                }

                                                OP_WORDCHAR => {
                                                    set_type_bits(
                                                        re,
                                                        cbit_word as c_int,
                                                        table_limit as c_uint,
                                                    );
                                                    try_next = FALSE;
                                                }

                                                /* One or more character type fudges the pointer and
                                                restarts, knowing it will hit a single character type and
                                                stop there. */
                                                OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEPOSPLUS => {
                                                    tcode = tcode.add(1);
                                                }

                                                OP_TYPEEXACT => {
                                                    tcode = tcode.add(1 + IMM2_SIZE);
                                                }

                                                /* Zero or more repeats of character types set the bits and
                                                then try again. */
                                                OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEPOSUPTO => {
                                                    tcode = tcode.add(IMM2_SIZE);
                                                    /* Fall through */
                                                    break 'typestar_case;
                                                }

                                                OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPOSSTAR
                                                | OP_TYPEQUERY | OP_TYPEMINQUERY
                                                | OP_TYPEPOSQUERY => {
                                                    break 'typestar_case;
                                                }

                                                /* Set-based ECLASS: treat it the same as a "complex"
                                                XCLASS; give up. */
                                                OP_ECLASS => {
                                                    return SSB_FAIL as c_int;
                                                }

                                                /* Extended class: if there are any property checks, or if
                                                this is a negative XCLASS without a map, give up. If there
                                                are no property checks, there must be wide characters on the
                                                XCLASS list, because otherwise an XCLASS would not have been
                                                created. This means that code points >= 255 are potential
                                                starters. In the UTF-8 case we can scan them and set bits for
                                                the relevant leading bytes. */
                                                OP_XCLASS => {
                                                    let xclassflags: PCRE2_UCHAR =
                                                        *tcode.add(1 + LINK_SIZE);
                                                    if (xclassflags as u32 & XCL_HASPROP) != 0
                                                        || (xclassflags as u32
                                                            & (XCL_MAP | XCL_NOT))
                                                            == XCL_NOT
                                                    {
                                                        return SSB_FAIL as c_int;
                                                    }

                                                    /* We have a positive XCLASS or a negative one without a
                                                    map. Set up the map pointer if there is one, and fall
                                                    through. */

                                                    classmap = if (xclassflags as u32 & XCL_MAP) == 0
                                                    {
                                                        std::ptr::null()
                                                    } else {
                                                        tcode.add(1 + LINK_SIZE + 1) as *const u8
                                                    };

                                                    /* In UTF-8 mode, scan the character list and set bits
                                                    for leading bytes, then jump to handle the map. */

                                                    if utf != 0 && (xclassflags as u32 & XCL_NOT) == 0
                                                    {
                                                        let mut b: PCRE2_UCHAR;
                                                        let mut e: PCRE2_UCHAR;
                                                        let mut p: PCRE2_SPTR = tcode.add(
                                                            1 + LINK_SIZE
                                                                + 1
                                                                + (if classmap.is_null() {
                                                                    0
                                                                } else {
                                                                    32
                                                                }),
                                                        );
                                                        tcode = tcode.add(GET!(tcode, 1) as usize);

                                                        if *p as u32 >= XCL_LIST {
                                                            study_char_list(
                                                                p,
                                                                (*re).start_bitmap.as_mut_ptr(),
                                                                (re as *const u8)
                                                                    .add((*re).code_start),
                                                            );
                                                            break 'handle_classmap;
                                                        }

                                                        loop {
                                                            let item: PCRE2_UCHAR = {
                                                                let t = *p;
                                                                p = p.add(1);
                                                                t
                                                            };
                                                            match item as u32 {
                                                                XCL_SINGLE => {
                                                                    b = {
                                                                        let t = *p;
                                                                        p = p.add(1);
                                                                        t
                                                                    };
                                                                    while (*p & 0xc0) == 0x80 {
                                                                        p = p.add(1);
                                                                    }
                                                                    *(*re)
                                                                        .start_bitmap
                                                                        .as_mut_ptr()
                                                                        .add((b / 8) as usize) |=
                                                                        (1u32 << (b & 7)) as u8;
                                                                }

                                                                XCL_RANGE => {
                                                                    b = {
                                                                        let t = *p;
                                                                        p = p.add(1);
                                                                        t
                                                                    };
                                                                    while (*p & 0xc0) == 0x80 {
                                                                        p = p.add(1);
                                                                    }
                                                                    e = {
                                                                        let t = *p;
                                                                        p = p.add(1);
                                                                        t
                                                                    };
                                                                    while (*p & 0xc0) == 0x80 {
                                                                        p = p.add(1);
                                                                    }
                                                                    while b <= e {
                                                                        *(*re)
                                                                            .start_bitmap
                                                                            .as_mut_ptr()
                                                                            .add((b / 8) as usize) |=
                                                                            (1u32 << (b & 7)) as u8;
                                                                        b = b.wrapping_add(1);
                                                                    }
                                                                }

                                                                XCL_END => {
                                                                    break 'handle_classmap;
                                                                }

                                                                /* LCOV_EXCL_START */
                                                                _ => {
                                                                    /* PCRE2_DEBUG_UNREACHABLE(); */
                                                                    return SSB_UNKNOWN as c_int; /* Internal error, should not occur */
                                                                } /* LCOV_EXCL_STOP */
                                                            }
                                                        }
                                                    }

                                                    /* Fall through */
                                                    break 'nclass_case;
                                                }

                                                /* Enter here for a negative non-XCLASS. In the 8-bit
                                                library, if we are in UTF mode, any byte with a value >= 0xc4
                                                is a potentially valid starter because it starts a character
                                                with a value > 255. In 8-bit non-UTF mode, there is no
                                                difference between CLASS and NCLASS. */
                                                OP_NCLASS => {
                                                    break 'nclass_case;
                                                }

                                                OP_CLASS => {
                                                    break 'class_case;
                                                }

                                                /* If we reach something we don't understand, it means a new
                                                opcode has been created that hasn't been added to this
                                                function. Hopefully this problem will be discovered during
                                                testing. */
                                                _ => {
                                                    return SSB_UNKNOWN as c_int;
                                                }
                                            }
                                            break 'end_case;
                                        }

                                        /* Group bracket or positive assertion: recurse. */

                                        let rc: c_int =
                                            set_start_bits(re, tcode, utf, ucp, depthptr);
                                        if rc == SSB_DONE as c_int {
                                            try_next = FALSE;
                                        } else if rc == SSB_CONTINUE as c_int {
                                            loop {
                                                tcode = tcode.add(GET!(tcode, 1) as usize);
                                                if !(*tcode as u32 == OP_ALT) {
                                                    break;
                                                }
                                            }
                                            tcode = tcode.add(1 + LINK_SIZE);
                                        } else {
                                            return rc; /* FAIL, UNKNOWN, or TOODEEP */
                                        }
                                        break 'end_case;
                                    }

                                    /* OP_CHAR, OP_PLUS, OP_MINPLUS, OP_POSPLUS (and OP_EXACT) */

                                    set_table_bit(re, tcode.add(1), FALSE, utf, ucp);
                                    try_next = FALSE;
                                    break 'end_case;
                                }

                                /* OP_CHARI, OP_PLUSI, OP_MINPLUSI, OP_POSPLUSI (and OP_EXACTI) */

                                set_table_bit(re, tcode.add(1), TRUE, utf, ucp);
                                try_next = FALSE;
                                break 'end_case;
                            }

                            /* OP_TYPESTAR etc. (and OP_TYPEUPTO etc.) */

                            match *tcode.add(1) as u32 {
                                OP_ANY | OP_ALLANY => {
                                    return SSB_FAIL as c_int;
                                }

                                OP_HSPACE => {
                                    SET_BIT!(re, CHAR_HT);
                                    SET_BIT!(re, CHAR_SPACE);

                                    /* For the 8-bit library in UTF-8 mode, set the bits for the first
                                    code units of horizontal space characters. */

                                    if utf != 0 {
                                        SET_BIT!(re, 0xC2); /* For U+00A0 */
                                        SET_BIT!(re, 0xE1); /* For U+1680, U+180E */
                                        SET_BIT!(re, 0xE2); /* For U+2000 - U+200A, U+202F, U+205F */
                                        SET_BIT!(re, 0xE3); /* For U+3000 */
                                    }
                                    /* For the 8-bit library not in UTF-8 mode, set the bit for NBSP. */
                                    else {
                                        SET_BIT!(re, CHAR_NBSP);
                                    }
                                }

                                OP_ANYNL | OP_VSPACE => {
                                    SET_BIT!(re, CHAR_LF);
                                    SET_BIT!(re, CHAR_VT);
                                    SET_BIT!(re, CHAR_FF);
                                    SET_BIT!(re, CHAR_CR);

                                    /* For the 8-bit library in UTF-8 mode, set the bits for the first
                                    code units of vertical space characters. */

                                    if utf != 0 {
                                        SET_BIT!(re, 0xC2); /* For U+0085 (NEL) */
                                        SET_BIT!(re, 0xE2); /* For U+2028, U+2029 */
                                    }
                                    /* For the 8-bit library not in UTF-8 mode, set the bit for NEL. */
                                    else {
                                        SET_BIT!(re, CHAR_NEL);
                                    }
                                }

                                OP_NOT_DIGIT => {
                                    set_nottype_bits(re, cbit_digit as c_int, table_limit as c_uint);
                                }

                                OP_DIGIT => {
                                    set_type_bits(re, cbit_digit as c_int, table_limit as c_uint);
                                }

                                OP_NOT_WHITESPACE => {
                                    set_nottype_bits(re, cbit_space as c_int, table_limit as c_uint);
                                }

                                OP_WHITESPACE => {
                                    set_type_bits(re, cbit_space as c_int, table_limit as c_uint);
                                }

                                OP_NOT_WORDCHAR => {
                                    set_nottype_bits(re, cbit_word as c_int, table_limit as c_uint);
                                }

                                OP_WORDCHAR => {
                                    set_type_bits(re, cbit_word as c_int, table_limit as c_uint);
                                }

                                _ => {
                                    return SSB_FAIL as c_int;
                                }
                            }

                            tcode = tcode.add(2);
                            break 'end_case;
                        }

                        /* OP_NCLASS (also entered by falling through from OP_XCLASS) */

                        if utf != 0 {
                            *(*re).start_bitmap.as_mut_ptr().add(24) |= 0xf0; /* Bits for 0xc4 - 0xc8 */
                            memset(
                                (*re).start_bitmap.as_mut_ptr().add(25) as *mut c_void,
                                0xff,
                                7,
                            ); /* Bits for 0xc9 - 0xff */
                        }
                        /* Fall through */
                    }

                    /* Enter here for a positive non-XCLASS. If we have fallen through from
                    an XCLASS, classmap will already be set; just advance the code pointer.
                    Otherwise, set up classmap for a non-XCLASS and advance past it. */

                    if *tcode as u32 == OP_XCLASS {
                        tcode = tcode.add(GET!(tcode, 1) as usize);
                    } else {
                        tcode = tcode.add(1);
                        classmap = tcode as *const u8;
                        tcode = tcode.add(32);
                    }
                    /* Fall through to HANDLE_CLASSMAP */
                }

                /* HANDLE_CLASSMAP: */

                /* When wide characters are supported, classmap may be NULL. In UTF-8
                (sic) mode, the bits in a class bit map correspond to character values,
                not to byte values. However, the bit map we are constructing is for byte
                values. So we have to do a conversion for characters whose code point is
                greater than 127. In fact, there are only two possible starting bytes for
                characters in the range 128 - 255. */

                if !classmap.is_null() {
                    if utf != 0 {
                        c = 0;
                        while c < 16 {
                            *(*re).start_bitmap.as_mut_ptr().add(c as usize) |=
                                *classmap.add(c as usize);
                            c += 1;
                        }
                        c = 128;
                        while c < 256 {
                            if (*classmap.add((c / 8) as usize) as u32 & (1u32 << (c & 7))) != 0 {
                                let d: c_int = ((c >> 6) | 0xc0) as c_int; /* Set bit for this starter */
                                *(*re).start_bitmap.as_mut_ptr().add((d / 8) as usize) |=
                                    (1u32 << (d & 7)) as u8; /* and then skip on to the */
                                c = (c & 0xc0) + 0x40 - 1; /* next relevant character. */
                            }
                            c += 1;
                        }
                    }
                    /* In all modes except UTF-8, the two bit maps are compatible. */
                    else {
                        c = 0;
                        while c < 32 {
                            *(*re).start_bitmap.as_mut_ptr().add(c as usize) |=
                                *classmap.add(c as usize);
                            c += 1;
                        }
                    }
                }

                /* Act on what follows the class. For a zero minimum repeat, continue;
                otherwise stop processing. */

                match *tcode as u32 {
                    OP_CRSTAR | OP_CRMINSTAR | OP_CRQUERY | OP_CRMINQUERY | OP_CRPOSSTAR
                    | OP_CRPOSQUERY => {
                        tcode = tcode.add(1);
                    }

                    OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                        if GET2!(tcode, 1) == 0 {
                            tcode = tcode.add(1 + 2 * IMM2_SIZE);
                        } else {
                            try_next = FALSE;
                        }
                    }

                    _ => {
                        try_next = FALSE;
                    }
                }
                /* End of class handling case */
            } /* End of switch for opcodes */
        } /* End of try_next loop */

        code = code.add(GET!(code, 1) as usize); /* Advance to next branch */

        if !(*code as u32 == OP_ALT) {
            break;
        }
    }

    return yield_;
}
