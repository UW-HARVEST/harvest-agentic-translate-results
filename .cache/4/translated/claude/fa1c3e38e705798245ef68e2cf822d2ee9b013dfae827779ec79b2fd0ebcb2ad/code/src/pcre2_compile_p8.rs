/* Translated from c_src/src/pcre2_compile.c lines 8895-9392 */

/* (The comment block for is_anchored() precedes this region; the essentials:

We can also consider a regex to be anchored if OP_SOM starts all its branches.
This is the code for \G, which means "match at start of match position, taking
into account the match offset".

A branch is also implicitly anchored if it starts with .* and DOTALL is set,
because that will try the rest of the pattern at all possible matching points,
so there is no point trying again.... er ....

.... except when the .* appears inside capturing parentheses, and there is a
subsequent back reference to those parentheses. We haven't enough information
to catch that case precisely.

At first, the best we could do was to detect when .* was in capturing brackets
and the highest back reference was greater than or equal to that level.
However, by keeping a bitmap of the first 31 back references, we can catch some
of the more common cases more precisely.

... A second exception is when the .* appears inside an atomic group, because
this prevents the number of characters it matches from being adjusted.

Arguments:
  code           points to start of the compiled pattern
  bracket_map    a bitmap of which brackets we are inside while testing; this
                   handles up to substring 31; after that we just have to take
                   the less precise approach
  cb             points to the compile data block
  atomcount      atomic group level
  inassert       TRUE if in an assertion
  dotstar_anchor TRUE if automatic anchoring optimization is enabled

Returns:     TRUE or FALSE
*/

unsafe fn is_anchored(
    code: PCRE2_SPTR,
    bracket_map: u32,
    cb: *mut compile_block,
    atomcount: c_int,
    inassert: BOOL,
    dotstar_anchor: BOOL,
) -> BOOL {
    let mut code: PCRE2_SPTR = code;
    loop {
        let scode: PCRE2_SPTR = first_significant_code(
            code.add(*_pcre2_OP_lengths_8.as_ptr().add(*code as usize) as usize),
            FALSE,
        );
        let op: c_int = *scode as c_int;

        /* Non-capturing brackets */

        if op as u32 == OP_BRA
            || op as u32 == OP_BRAPOS
            || op as u32 == OP_SBRA
            || op as u32 == OP_SBRAPOS
        {
            if is_anchored(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }
        /* Capturing brackets */
        else if op as u32 == OP_CBRA
            || op as u32 == OP_CBRAPOS
            || op as u32 == OP_SCBRA
            || op as u32 == OP_SCBRAPOS
        {
            let n: c_int = GET2!(scode, 1 + LINK_SIZE) as c_int;
            let new_map: u32 = bracket_map | (if n < 32 { 1u32 << n } else { 1 });
            if is_anchored(scode, new_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }
        /* Positive forward assertion */
        else if op as u32 == OP_ASSERT || op as u32 == OP_ASSERT_NA {
            if is_anchored(scode, bracket_map, cb, atomcount, TRUE, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }
        /* Condition. If there is no second branch, it can't be anchored. */
        else if op as u32 == OP_COND || op as u32 == OP_SCOND {
            if *scode.add(GET!(scode, 1) as usize) as u32 != OP_ALT {
                return FALSE;
            }
            if is_anchored(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }
        /* Atomic groups */
        else if op as u32 == OP_ONCE {
            if is_anchored(scode, bracket_map, cb, atomcount + 1, inassert, dotstar_anchor) == FALSE
            {
                return FALSE;
            }
        }
        /* .* is not anchored unless DOTALL is set (which generates OP_ALLANY) and
        it isn't in brackets that are or may be referenced or inside an atomic
        group or an assertion. Also the pattern must not contain *PRUNE or *SKIP,
        because these break the feature. Consider, for example, /(?s).*?(*PRUNE)b/
        with the subject "aab", which matches "b", i.e. not at the start of a line.
        There is also an option that disables auto-anchoring. */
        else if op as u32 == OP_TYPESTAR
            || op as u32 == OP_TYPEMINSTAR
            || op as u32 == OP_TYPEPOSSTAR
        {
            if *scode.add(1) as u32 != OP_ALLANY
                || (bracket_map & (*cb).backref_map) != 0
                || atomcount > 0
                || (*cb).had_pruneorskip != 0
                || inassert != 0
                || dotstar_anchor == FALSE
            {
                return FALSE;
            }
        }
        /* Check for explicit anchoring */
        else if op as u32 != OP_SOD && op as u32 != OP_SOM && op as u32 != OP_CIRC {
            return FALSE;
        }

        code = code.add(GET!(code, 1) as usize);

        if !(*code as u32 == OP_ALT) {
            break;
        } /* Loop for each alternative */
    }
    TRUE
}

/*************************************************
*         Check for starting with ^ or .*        *
*************************************************/

/* This is called to find out if every branch starts with ^ or .* so that
"first char" processing can be done to speed things up in multiline
matching and for non-DOTALL patterns that start with .* (which must start at
the beginning or after \n). As in the case of is_anchored() (see above), we
have to take account of back references to capturing brackets that contain .*
because in that case we can't make the assumption. Also, the appearance of .*
inside atomic brackets or in an assertion, or in a pattern that contains *PRUNE
or *SKIP does not count, because once again the assumption no longer holds.

Arguments:
  code           points to start of the compiled pattern or a group
  bracket_map    a bitmap of which brackets we are inside while testing; this
                   handles up to substring 31; after that we just have to take
                   the less precise approach
  cb             points to the compile data
  atomcount      atomic group level
  inassert       TRUE if in an assertion
  dotstar_anchor TRUE if automatic anchoring optimization is enabled

Returns:         TRUE or FALSE
*/

unsafe fn is_startline(
    code: PCRE2_SPTR,
    bracket_map: c_uint,
    cb: *mut compile_block,
    atomcount: c_int,
    inassert: BOOL,
    dotstar_anchor: BOOL,
) -> BOOL {
    let mut code: PCRE2_SPTR = code;
    loop {
        let mut scode: PCRE2_SPTR = first_significant_code(
            code.add(*_pcre2_OP_lengths_8.as_ptr().add(*code as usize) as usize),
            FALSE,
        );
        let mut op: c_int = *scode as c_int;

        /* If we are at the start of a conditional assertion group, *both* the
        conditional assertion *and* what follows the condition must satisfy the test
        for start of line. Other kinds of condition fail. Note that there may be an
        auto-callout at the start of a condition. */

        if op as u32 == OP_COND {
            scode = scode.add(1 + LINK_SIZE);

            if *scode as u32 == OP_CALLOUT {
                scode = scode.add(*_pcre2_OP_lengths_8.as_ptr().add(OP_CALLOUT as usize) as usize);
            } else if *scode as u32 == OP_CALLOUT_STR {
                scode = scode.add(GET!(scode, 1 + 2 * LINK_SIZE) as usize);
            }

            match *scode as u32 {
                OP_CREF | OP_DNCREF | OP_RREF | OP_DNRREF | OP_FAIL | OP_FALSE | OP_TRUE => {
                    return FALSE;
                }

                _ => {
                    /* Assertion */
                    if is_startline(scode, bracket_map, cb, atomcount, TRUE, dotstar_anchor)
                        == FALSE
                    {
                        return FALSE;
                    }
                    loop {
                        scode = scode.add(GET!(scode, 1) as usize);
                        if !(*scode as u32 == OP_ALT) {
                            break;
                        }
                    }
                    scode = scode.add(1 + LINK_SIZE);
                }
            }
            scode = first_significant_code(scode, FALSE);
            op = *scode as c_int;
        }

        /* Non-capturing brackets */

        if op as u32 == OP_BRA
            || op as u32 == OP_BRAPOS
            || op as u32 == OP_SBRA
            || op as u32 == OP_SBRAPOS
        {
            if is_startline(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }
        /* Capturing brackets */
        else if op as u32 == OP_CBRA
            || op as u32 == OP_CBRAPOS
            || op as u32 == OP_SCBRA
            || op as u32 == OP_SCBRAPOS
        {
            let n: c_int = GET2!(scode, 1 + LINK_SIZE) as c_int;
            let new_map: c_uint = bracket_map | (if n < 32 { 1u32 << n } else { 1 });
            if is_startline(scode, new_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }
        /* Positive forward assertions */
        else if op as u32 == OP_ASSERT || op as u32 == OP_ASSERT_NA {
            if is_startline(scode, bracket_map, cb, atomcount, TRUE, dotstar_anchor) == FALSE {
                return FALSE;
            }
        }
        /* Atomic brackets */
        else if op as u32 == OP_ONCE {
            if is_startline(scode, bracket_map, cb, atomcount + 1, inassert, dotstar_anchor)
                == FALSE
            {
                return FALSE;
            }
        }
        /* .* means "start at start or after \n" if it isn't in atomic brackets or
        brackets that may be referenced or an assertion, and as long as the pattern
        does not contain *PRUNE or *SKIP, because these break the feature. Consider,
        for example, /.*?a(*PRUNE)b/ with the subject "aab", which matches "ab",
        i.e. not at the start of a line. There is also an option that disables this
        optimization. */
        else if op as u32 == OP_TYPESTAR
            || op as u32 == OP_TYPEMINSTAR
            || op as u32 == OP_TYPEPOSSTAR
        {
            if *scode.add(1) as u32 != OP_ANY
                || (bracket_map & (*cb).backref_map) != 0
                || atomcount > 0
                || (*cb).had_pruneorskip != 0
                || inassert != 0
                || dotstar_anchor == FALSE
            {
                return FALSE;
            }
        }
        /* Check for explicit circumflex; anything else gives a FALSE result. Note
        in particular that this includes atomic brackets OP_ONCE because the number
        of characters matched by .* cannot be adjusted inside them. */
        else if op as u32 != OP_CIRC && op as u32 != OP_CIRCM {
            return FALSE;
        }

        /* Move on to the next alternative */

        code = code.add(GET!(code, 1) as usize);

        if !(*code as u32 == OP_ALT) {
            break;
        } /* Loop for each alternative */
    }
    TRUE
}

/*************************************************
*   Scan compiled regex for recursion reference  *
*************************************************/

/* This function scans through a compiled pattern until it finds an instance of
OP_RECURSE.

Arguments:
  code        points to start of expression
  utf         TRUE in UTF mode

Returns:      pointer to the opcode for OP_RECURSE, or NULL if not found
*/

unsafe fn find_recurse(code: *mut PCRE2_UCHAR, utf: BOOL) -> *mut PCRE2_UCHAR {
    let mut code: *mut PCRE2_UCHAR = code;
    loop {
        let c: PCRE2_UCHAR = *code;
        if c as u32 == OP_END {
            return std::ptr::null_mut();
        }
        if c as u32 == OP_RECURSE {
            return code;
        }

        /* XCLASS is used for classes that cannot be represented just by a bit map.
        This includes negated single high-valued characters. ECLASS is used for
        classes that use set operations internally. CALLOUT_STR is used for
        callouts with string arguments. In each case the length in the table is
        zero; the actual length is stored in the compiled code. */

        if c as u32 == OP_XCLASS || c as u32 == OP_ECLASS {
            code = code.add(GET!(code, 1) as usize);
        } else if c as u32 == OP_CALLOUT_STR {
            code = code.add(GET!(code, 1 + 2 * LINK_SIZE) as usize);
        }
        /* Otherwise, we can get the item's length from the table, except that for
        repeated character types, we have to test for \p and \P, which have an extra
        two code units of parameters, and for MARK/PRUNE/SKIP/THEN with an argument,
        we must add in its length. */
        else {
            match c as u32 {
                OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEQUERY
                | OP_TYPEMINQUERY | OP_TYPEPOSSTAR | OP_TYPEPOSPLUS | OP_TYPEPOSQUERY => {
                    if *code.add(1) as u32 == OP_PROP || *code.add(1) as u32 == OP_NOTPROP {
                        code = code.add(2);
                    }
                }

                OP_TYPEPOSUPTO | OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEEXACT => {
                    if *code.add(1 + IMM2_SIZE) as u32 == OP_PROP
                        || *code.add(1 + IMM2_SIZE) as u32 == OP_NOTPROP
                    {
                        code = code.add(2);
                    }
                }

                OP_MARK | OP_COMMIT_ARG | OP_PRUNE_ARG | OP_SKIP_ARG | OP_THEN_ARG => {
                    code = code.add(*code.add(1) as usize);
                }

                _ => {}
            }

            /* Add in the fixed length from the table */

            code = code.add(*_pcre2_OP_lengths_8.as_ptr().add(c as usize) as usize);

            /* In UTF-8 and UTF-16 modes, opcodes that are followed by a character may
            be followed by a multi-unit character. The length in the table is a
            minimum, so we have to arrange to skip the extra units. */

            if utf != 0 {
                match c as u32 {
                    OP_CHAR
                    | OP_CHARI
                    | OP_NOT
                    | OP_NOTI
                    | OP_EXACT
                    | OP_EXACTI
                    | OP_NOTEXACT
                    | OP_NOTEXACTI
                    | OP_UPTO
                    | OP_UPTOI
                    | OP_NOTUPTO
                    | OP_NOTUPTOI
                    | OP_MINUPTO
                    | OP_MINUPTOI
                    | OP_NOTMINUPTO
                    | OP_NOTMINUPTOI
                    | OP_POSUPTO
                    | OP_POSUPTOI
                    | OP_NOTPOSUPTO
                    | OP_NOTPOSUPTOI
                    | OP_STAR
                    | OP_STARI
                    | OP_NOTSTAR
                    | OP_NOTSTARI
                    | OP_MINSTAR
                    | OP_MINSTARI
                    | OP_NOTMINSTAR
                    | OP_NOTMINSTARI
                    | OP_POSSTAR
                    | OP_POSSTARI
                    | OP_NOTPOSSTAR
                    | OP_NOTPOSSTARI
                    | OP_PLUS
                    | OP_PLUSI
                    | OP_NOTPLUS
                    | OP_NOTPLUSI
                    | OP_MINPLUS
                    | OP_MINPLUSI
                    | OP_NOTMINPLUS
                    | OP_NOTMINPLUSI
                    | OP_POSPLUS
                    | OP_POSPLUSI
                    | OP_NOTPOSPLUS
                    | OP_NOTPOSPLUSI
                    | OP_QUERY
                    | OP_QUERYI
                    | OP_NOTQUERY
                    | OP_NOTQUERYI
                    | OP_MINQUERY
                    | OP_MINQUERYI
                    | OP_NOTMINQUERY
                    | OP_NOTMINQUERYI
                    | OP_POSQUERY
                    | OP_POSQUERYI
                    | OP_NOTPOSQUERY
                    | OP_NOTPOSQUERYI => {
                        if HAS_EXTRALEN!(*code.offset(-1)) {
                            code = code.add(GET_EXTRALEN!(*code.offset(-1)) as usize);
                        }
                    }

                    _ => {}
                }
            }
        }
    }
}

/*************************************************
*    Check for asserted fixed first code unit    *
*************************************************/

/* During compilation, the "first code unit" settings from forward assertions
are discarded, because they can cause conflicts with actual literals that
follow. However, if we end up without a first code unit setting for an
unanchored pattern, it is worth scanning the regex to see if there is an
initial asserted first code unit. If all branches start with the same asserted
code unit, or with a non-conditional bracket all of whose alternatives start
with the same asserted code unit (recurse ad lib), then we return that code
unit, with the flags set to zero or REQ_CASELESS; otherwise return zero with
REQ_NONE in the flags.

Arguments:
  code       points to start of compiled pattern
  flags      points to the first code unit flags
  inassert   non-zero if in an assertion

Returns:     the fixed first code unit, or 0 with REQ_NONE in flags
*/

unsafe fn find_firstassertedcu(code: PCRE2_SPTR, flags: *mut u32, inassert: u32) -> u32 {
    let mut code: PCRE2_SPTR = code;
    let mut c: u32 = 0;
    let mut cflags: u32 = REQ_NONE;

    *flags = REQ_NONE;
    loop {
        let d: u32;
        let mut dflags: u32 = 0;
        let xl: c_int = if *code as u32 == OP_CBRA
            || *code as u32 == OP_SCBRA
            || *code as u32 == OP_CBRAPOS
            || *code as u32 == OP_SCBRAPOS
        {
            IMM2_SIZE as c_int
        } else {
            0
        };
        let mut scode: PCRE2_SPTR = first_significant_code(
            code.add(1 + LINK_SIZE).offset(xl as isize),
            TRUE,
        );
        let op: PCRE2_UCHAR = *scode;

        match op as u32 {
            OP_BRA | OP_BRAPOS | OP_CBRA | OP_SCBRA | OP_CBRAPOS | OP_SCBRAPOS | OP_ASSERT
            | OP_ASSERT_NA | OP_ONCE | OP_SCRIPT_RUN => {
                d = find_firstassertedcu(
                    scode,
                    &mut dflags,
                    inassert
                        + (if op as u32 == OP_ASSERT || op as u32 == OP_ASSERT_NA {
                            1
                        } else {
                            0
                        }),
                );
                if dflags >= REQ_NONE {
                    return 0;
                }
                if cflags >= REQ_NONE {
                    c = d;
                    cflags = dflags;
                } else if c != d || cflags != dflags {
                    return 0;
                }
            }

            /* OP_EXACT falls through into the OP_CHAR group */
            OP_EXACT | OP_CHAR | OP_PLUS | OP_MINPLUS | OP_POSPLUS => {
                if op as u32 == OP_EXACT {
                    scode = scode.add(IMM2_SIZE);
                }
                if inassert == 0 {
                    return 0;
                }
                if cflags >= REQ_NONE {
                    c = *scode.add(1) as u32;
                    cflags = 0;
                } else if c != *scode.add(1) as u32 {
                    return 0;
                }
            }

            /* OP_EXACTI falls through into the OP_CHARI group */
            OP_EXACTI | OP_CHARI | OP_PLUSI | OP_MINPLUSI | OP_POSPLUSI => {
                if op as u32 == OP_EXACTI {
                    scode = scode.add(IMM2_SIZE);
                }
                if inassert == 0 {
                    return 0;
                }

                /* If the character is more than one code unit long, we cannot set its
                first code unit when matching caselessly. Later scanning may pick up
                multiple code units. */

                if *scode.add(1) as u32 >= 0x80 {
                    return 0;
                }

                if cflags >= REQ_NONE {
                    c = *scode.add(1) as u32;
                    cflags = REQ_CASELESS;
                } else if c != *scode.add(1) as u32 {
                    return 0;
                }
            }

            _ => return 0,
        }

        code = code.add(GET!(code, 1) as usize);

        if !(*code as u32 == OP_ALT) {
            break;
        }
    }

    *flags = cflags;
    c
}
