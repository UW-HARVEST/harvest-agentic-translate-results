/* Translated from c_src/src/pcre2_compile.c lines 8574-8894 */

/*************************************************
*   Compile regex: a sequence of alternatives    *
*************************************************/

/* On entry, pptr is pointing past the bracket meta, but on return it points to
the closing bracket or META_END. The code variable is pointing at the code unit
into which the BRA operator has been stored. This function is used during the
pre-compile phase when we are trying to find out the amount of memory needed,
as well as during the real compile phase. The value of lengthptr distinguishes
the two phases.

Arguments:
  options           option bits, including any changes for this subpattern
  xoptions          extra option bits, ditto
  codeptr           -> the address of the current code pointer
  pptrptr           -> the address of the current parsed pattern pointer
  errorcodeptr      -> pointer to error code variable
  skipunits         skip this many code units at start (for brackets and OP_COND)
  firstcuptr        place to put the first required code unit
  firstcuflagsptr   place to put the first code unit flags
  reqcuptr          place to put the last required code unit
  reqcuflagsptr     place to put the last required code unit flags
  bcptr             pointer to the chain of currently open branches
  cb                points to the data block with tables pointers etc.
  lengthptr         NULL during the real compile phase
                    points to length accumulator during pre-compile phase

Returns:            0 There has been an error
                   +1 Success, this group must match at least one character
                   -1 Success, this group may match an empty string
*/

unsafe fn compile_regex(
    options: u32,
    xoptions: u32,
    codeptr: *mut *mut PCRE2_UCHAR,
    pptrptr: *mut *mut u32,
    errorcodeptr: *mut c_int,
    skipunits: u32,
    firstcuptr: *mut u32,
    firstcuflagsptr: *mut u32,
    reqcuptr: *mut u32,
    reqcuflagsptr: *mut u32,
    bcptr: *mut branch_chain,
    open_caps: *mut open_capitem,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
) -> c_int {
    let mut options: u32 = options; /* May change dynamically (via compile_branch) */
    let mut xoptions: u32 = xoptions; /* Ditto */
    let mut open_caps: *mut open_capitem = open_caps;
    let mut code: *mut PCRE2_UCHAR = *codeptr;
    let mut last_branch: *mut PCRE2_UCHAR = code;
    let start_bracket: *mut PCRE2_UCHAR = code;
    let lookbehind: BOOL;
    let mut capitem: open_capitem = open_capitem {
        next: core::ptr::null_mut(),
        number: 0,
        assert_depth: 0,
    };
    let mut capnumber: c_int = 0;
    let mut okreturn: c_int = 1;
    let mut pptr: *mut u32 = *pptrptr;
    let mut firstcu: u32;
    let mut reqcu: u32;
    let mut lookbehindlength: u32;
    let lookbehindminlength: u32;
    let mut firstcuflags: u32;
    let mut reqcuflags: u32;
    let mut length: PCRE2_SIZE;
    let mut bc: branch_chain = branch_chain {
        outer: core::ptr::null_mut(),
        current_branch: core::ptr::null_mut(),
    };

    /* If set, call the external function that checks for stack availability. */

    if (*(*cb).cx).stack_guard.is_some()
        && ((*(*cb).cx).stack_guard.unwrap())(
            (*cb).parens_depth as u32,
            (*(*cb).cx).stack_guard_data,
        ) != 0
    {
        *errorcodeptr = ERR33;
        (*cb).erroroffset = 0;
        return 0;
    }

    /* Miscellaneous initialization */

    bc.outer = bcptr;
    bc.current_branch = code;

    reqcu = 0;
    firstcu = reqcu;
    reqcuflags = REQ_UNSET;
    firstcuflags = reqcuflags;

    /* Accumulate the length for use in the pre-compile phase. Start with the
    length of the BRA and KET and any extra code units that are required at the
    beginning. We accumulate in a local variable to save frequent testing of
    lengthptr for NULL. We cannot do this by looking at the value of 'code' at the
    start and end of each alternative, because compiled items are discarded during
    the pre-compile phase so that the workspace is not exceeded. */

    length = 2 + 2 * LINK_SIZE + skipunits as PCRE2_SIZE;

    /* Remember if this is a lookbehind assertion, and if it is, save its length
    and skip over the pattern offset. */

    lookbehind = (*code as u32 == OP_ASSERTBACK
        || *code as u32 == OP_ASSERTBACK_NOT
        || *code as u32 == OP_ASSERTBACK_NA) as BOOL;

    if lookbehind != 0 {
        lookbehindlength = META_DATA!(*pptr.offset(-1));
        lookbehindminlength = *pptr;
        pptr = pptr.add(SIZEOFFSET);
    } else {
        lookbehindlength = 0;
        lookbehindminlength = 0;
    }

    /* If this is a capturing subpattern, add to the chain of open capturing items
    so that we can detect them if (*ACCEPT) is encountered. Note that only OP_CBRA
    need be tested here; changing this opcode to one of its variants, e.g.
    OP_SCBRAPOS, happens later, after the group has been compiled. */

    if *code as u32 == OP_CBRA {
        capnumber = GET2!(code, 1 + LINK_SIZE) as c_int;
        capitem.number = capnumber as u16;
        capitem.next = open_caps;
        capitem.assert_depth = (*cb).assert_depth;
        open_caps = &mut capitem as *mut open_capitem;
    }

    /* Offset is set zero to mark that this bracket is still open */

    PUT!(code, 1, 0u32);
    code = code.add(1 + LINK_SIZE + skipunits as usize);

    /* Loop for each alternative branch */

    loop {
        let branch_return: c_int;
        let mut branchfirstcu: u32 = 0;
        let mut branchreqcu: u32 = 0;
        let mut branchfirstcuflags: u32 = REQ_UNSET;
        let mut branchreqcuflags: u32 = REQ_UNSET;

        /* Insert OP_REVERSE or OP_VREVERSE if this is a lookbehind assertion. There
        is only a single minimum length for the whole assertion. When the minimum
        length is LOOKBEHIND_MAX it means that all branches are of fixed length,
        though not necessarily the same length. In this case, the original OP_REVERSE
        can be used. It can also be used if a branch in a variable length lookbehind
        has the same maximum and minimum. Otherwise, use OP_VREVERSE, which has both
        maximum and minimum values. */

        if lookbehind != 0 && lookbehindlength > 0 {
            if lookbehindminlength == LOOKBEHIND_MAX as u32
                || lookbehindminlength == lookbehindlength
            {
                *code = OP_REVERSE as PCRE2_UCHAR;
                code = code.add(1);
                PUT2INC!(code, 0, lookbehindlength);
                length += 1 + IMM2_SIZE;
            } else {
                *code = OP_VREVERSE as PCRE2_UCHAR;
                code = code.add(1);
                PUT2INC!(code, 0, lookbehindminlength);
                PUT2INC!(code, 0, lookbehindlength);
                length += 1 + 2 * IMM2_SIZE;
            }
        }

        /* Now compile the branch; in the pre-compile phase its length gets added
        into the length. */

        branch_return = compile_branch(
            &mut options,
            &mut xoptions,
            &mut code,
            &mut pptr,
            errorcodeptr,
            &mut branchfirstcu,
            &mut branchfirstcuflags,
            &mut branchreqcu,
            &mut branchreqcuflags,
            &mut bc,
            open_caps,
            cb,
            if lengthptr.is_null() {
                core::ptr::null_mut()
            } else {
                &mut length as *mut PCRE2_SIZE
            },
        );
        if branch_return == 0 {
            return 0;
        }

        /* If a branch can match an empty string, so can the whole group. */

        if branch_return < 0 {
            okreturn = -1;
        }

        /* In the real compile phase, there is some post-processing to be done. */

        if lengthptr.is_null() {
            /* If this is the first branch, the firstcu and reqcu values for the
            branch become the values for the regex. */

            if *last_branch as u32 != OP_ALT {
                firstcu = branchfirstcu;
                firstcuflags = branchfirstcuflags;
                reqcu = branchreqcu;
                reqcuflags = branchreqcuflags;
            }
            /* If this is not the first branch, the first char and reqcu have to
            match the values from all the previous branches, except that if the
            previous value for reqcu didn't have REQ_VARY set, it can still match,
            and we set REQ_VARY for the group from this branch's value. */
            else {
                /* If we previously had a firstcu, but it doesn't match the new branch,
                we have to abandon the firstcu for the regex, but if there was
                previously no reqcu, it takes on the value of the old firstcu. */

                if firstcuflags != branchfirstcuflags || firstcu != branchfirstcu {
                    if firstcuflags < REQ_NONE {
                        if reqcuflags >= REQ_NONE {
                            reqcu = firstcu;
                            reqcuflags = firstcuflags;
                        }
                    }
                    firstcuflags = REQ_NONE;
                }

                /* If we (now or from before) have no firstcu, a firstcu from the
                branch becomes a reqcu if there isn't a branch reqcu. */

                if firstcuflags >= REQ_NONE
                    && branchfirstcuflags < REQ_NONE
                    && branchreqcuflags >= REQ_NONE
                {
                    branchreqcu = branchfirstcu;
                    branchreqcuflags = branchfirstcuflags;
                }

                /* Now ensure that the reqcus match */

                if ((reqcuflags & !REQ_VARY) != (branchreqcuflags & !REQ_VARY))
                    || reqcu != branchreqcu
                {
                    reqcuflags = REQ_NONE;
                } else {
                    reqcu = branchreqcu;
                    reqcuflags |= branchreqcuflags; /* To "or" REQ_VARY if present */
                }
            }
        }

        /* Handle reaching the end of the expression, either ')' or end of pattern.
        In the real compile phase, go back through the alternative branches and
        reverse the chain of offsets, with the field in the BRA item now becoming an
        offset to the first alternative. If there are no alternatives, it points to
        the end of the group. The length in the terminating ket is always the length
        of the whole bracketed item. Return leaving the pointer at the terminating
        char. */

        if META_CODE!(*pptr) != META_ALT {
            if lengthptr.is_null() {
                let mut branch_length: u32 = code.offset_from(last_branch) as u32;
                loop {
                    let prev_length: u32 = GET!(last_branch, 1);
                    PUT!(last_branch, 1, branch_length);
                    branch_length = prev_length;
                    last_branch = last_branch.sub(branch_length as usize);

                    if !(branch_length > 0) {
                        break;
                    }
                }
            }

            /* Fill in the ket */

            *code = OP_KET as PCRE2_UCHAR;
            PUT!(code, 1, code.offset_from(start_bracket) as u32);
            code = code.add(1 + LINK_SIZE);

            /* Set values to pass back */

            *codeptr = code;
            *pptrptr = pptr;
            *firstcuptr = firstcu;
            *firstcuflagsptr = firstcuflags;
            *reqcuptr = reqcu;
            *reqcuflagsptr = reqcuflags;
            if !lengthptr.is_null() {
                if (OFLOW_MAX as PCRE2_SIZE).wrapping_sub(*lengthptr) < length {
                    *errorcodeptr = ERR20;
                    return 0;
                }
                *lengthptr += length;
            }
            return okreturn;
        }

        /* Another branch follows. In the pre-compile phase, we can move the code
        pointer back to where it was for the start of the first branch. (That is,
        pretend that each branch is the only one.)

        In the real compile phase, insert an ALT node. Its length field points back
        to the previous branch while the bracket remains open. At the end the chain
        is reversed. It's done like this so that the start of the bracket has a
        zero offset until it is closed, making it possible to detect recursion. */

        if !lengthptr.is_null() {
            code = (*codeptr).add(1 + LINK_SIZE + skipunits as usize);
            length += 1 + LINK_SIZE;
        } else {
            *code = OP_ALT as PCRE2_UCHAR;
            PUT!(code, 1, code.offset_from(last_branch) as c_int);
            last_branch = code;
            bc.current_branch = last_branch;
            code = code.add(1 + LINK_SIZE);
        }

        /* Set the maximum lookbehind length for the next branch (if not in a
        lookbehind the value will be zero) and then advance past the vertical bar. */

        lookbehindlength = META_DATA!(*pptr);
        pptr = pptr.add(1);
    }

    /* LCOV_EXCL_START */
    /* PCRE2_DEBUG_UNREACHABLE(); Control should never reach here */
    /* return 0;                  Avoid compiler warnings */
    /* LCOV_EXCL_STOP */
}

/*************************************************
*          Check for anchored pattern            *
*************************************************/

/* Try to find out if this is an anchored regular expression. Consider each
alternative branch. If they all start with OP_SOD or OP_CIRC, or with a bracket
all of whose alternatives start with OP_SOD or OP_CIRC (recurse ad lib), then
it's anchored. However, if this is a multiline pattern, then only OP_SOD will
be found, because ^ generates OP_CIRCM in that mode.

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
