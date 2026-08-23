/* Translated from c_src/src/pcre2_compile_class.c lines 1880-2770 */

/* ===================================================================*/
/* Here follows a block of ECLASS-compiling functions. You may well want to
read them from top to bottom; they are ordered from leafmost (at the top) to
outermost parser (at the bottom of the file). */

/* This function folds one operand using the negation operator.
The new, combined chunk of stack code is written out to *pop_info. */

unsafe fn fold_negation(
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
    preserve_classbits: BOOL,
) {
    /* If the chunk of stack code is already composed of multiple ops, we won't
    descend in and try and propagate the negation down the tree. (That would lead
    to O(n^2) compile-time, which could be exploitable with a malicious regex -
    although maybe that's not really too much of a worry in a library that offers
    an exponential-time matching function!) */

    if (*pop_info).op_single_type == 0 {
        if !lengthptr.is_null() {
            *lengthptr = (*lengthptr).wrapping_add(1);
        } else {
            *(*pop_info).code_start.add((*pop_info).length) = ECL_NOT as u8;
        }
        (*pop_info).length = (*pop_info).length.wrapping_add(1);
    }
    /* Otherwise, it's a nice single-op item, so we can easily fold in the negation
    without needing to produce an ECL_NOT. */
    else if (*pop_info).op_single_type as u32 == ECL_ANY
        || (*pop_info).op_single_type as u32 == ECL_NONE
    {
        (*pop_info).op_single_type = (if (*pop_info).op_single_type as u32 == ECL_NONE {
            ECL_ANY
        } else {
            ECL_NONE
        }) as u8;
        if lengthptr.is_null() {
            *((*pop_info).code_start) = (*pop_info).op_single_type;
        }
    } else {
        /* PCRE2_ASSERT(pop_info->op_single_type == ECL_XCLASS &&
                        pop_info->length >= 1 + LINK_SIZE + 1); */
        if lengthptr.is_null() {
            *(*pop_info).code_start.add(1 + LINK_SIZE) ^= XCL_NOT as u8;
        }
    }

    if preserve_classbits == 0 {
        let mut i: c_int = 0;
        while i < 8 {
            (*pop_info).bits.classwords[i as usize] = !(*pop_info).bits.classwords[i as usize];
            i += 1;
        }
    }
}

/* This function folds together two operands using a binary operator.
The new, combined chunk of stack code is written out to *lhs_op_info. */

unsafe fn fold_binary(
    op: c_int,
    lhs_op_info: *mut eclass_op_info,
    rhs_op_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) {
    /* ECL_AND truth table:

       LHS  RHS  RESULT
       ----------------
       ANY  *    RHS
       *    ANY  LHS
       NONE *    NONE
       *    NONE NONE
       X    Y    X & Y
    */

    if op == ECL_AND as c_int {
        if (*rhs_op_info).op_single_type as u32 == ECL_ANY {
            /* no-op: drop the RHS */
        } else if (*lhs_op_info).op_single_type as u32 == ECL_ANY {
            /* no-op: drop the LHS, and memmove the RHS into its place */
            if lengthptr.is_null() {
                memmove(
                    (*lhs_op_info).code_start as *mut c_void,
                    (*rhs_op_info).code_start as *const c_void,
                    CU2BYTES!((*rhs_op_info).length),
                );
            }
            (*lhs_op_info).length = (*rhs_op_info).length;
            (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
        } else if (*rhs_op_info).op_single_type as u32 == ECL_NONE {
            /* the result is ECL_NONE: write into the LHS */
            if lengthptr.is_null() {
                *(*lhs_op_info).code_start.add(0) = ECL_NONE as u8;
            }
            (*lhs_op_info).length = 1;
            (*lhs_op_info).op_single_type = ECL_NONE as u8;
        } else if (*lhs_op_info).op_single_type as u32 == ECL_NONE {
            /* the result is ECL_NONE: drop the RHS */
        } else {
            /* Both of LHS & RHS are either ECL_XCLASS, or compound operations. */
            if !lengthptr.is_null() {
                *lengthptr = (*lengthptr).wrapping_add(1);
            } else {
                *(*rhs_op_info).code_start.add((*rhs_op_info).length) = ECL_AND as u8;
            }
            (*lhs_op_info).length = (*lhs_op_info)
                .length
                .wrapping_add((*rhs_op_info).length.wrapping_add(1));
            (*lhs_op_info).op_single_type = 0;
        }

        let mut i: c_int = 0;
        while i < 8 {
            (*lhs_op_info).bits.classwords[i as usize] &= (*rhs_op_info).bits.classwords[i as usize];
            i += 1;
        }
    }
    /* ECL_OR truth table:

       LHS  RHS  RESULT
       ----------------
       ANY  *    ANY
       *    ANY  ANY
       NONE *    RHS
       *    NONE LHS
       X    Y    X | Y
    */
    else if op == ECL_OR as c_int {
        if (*rhs_op_info).op_single_type as u32 == ECL_NONE {
            /* no-op: drop the RHS */
        } else if (*lhs_op_info).op_single_type as u32 == ECL_NONE {
            /* no-op: drop the LHS, and memmove the RHS into its place */
            if lengthptr.is_null() {
                memmove(
                    (*lhs_op_info).code_start as *mut c_void,
                    (*rhs_op_info).code_start as *const c_void,
                    CU2BYTES!((*rhs_op_info).length),
                );
            }
            (*lhs_op_info).length = (*rhs_op_info).length;
            (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
        } else if (*rhs_op_info).op_single_type as u32 == ECL_ANY {
            /* the result is ECL_ANY: write into the LHS */
            if lengthptr.is_null() {
                *(*lhs_op_info).code_start.add(0) = ECL_ANY as u8;
            }
            (*lhs_op_info).length = 1;
            (*lhs_op_info).op_single_type = ECL_ANY as u8;
        } else if (*lhs_op_info).op_single_type as u32 == ECL_ANY {
            /* the result is ECL_ANY: drop the RHS */
        } else {
            /* Both of LHS & RHS are either ECL_XCLASS, or compound operations. */
            if !lengthptr.is_null() {
                *lengthptr = (*lengthptr).wrapping_add(1);
            } else {
                *(*rhs_op_info).code_start.add((*rhs_op_info).length) = ECL_OR as u8;
            }
            (*lhs_op_info).length = (*lhs_op_info)
                .length
                .wrapping_add((*rhs_op_info).length.wrapping_add(1));
            (*lhs_op_info).op_single_type = 0;
        }

        let mut i: c_int = 0;
        while i < 8 {
            (*lhs_op_info).bits.classwords[i as usize] |= (*rhs_op_info).bits.classwords[i as usize];
            i += 1;
        }
    }
    /* ECL_XOR truth table:

       LHS  RHS  RESULT
       ----------------
       ANY  *    !RHS
       *    ANY  !LHS
       NONE *    RHS
       *    NONE LHS
       X    Y    X ^ Y
    */
    else if op == ECL_XOR as c_int {
        if (*rhs_op_info).op_single_type as u32 == ECL_NONE {
            /* no-op: drop the RHS */
        } else if (*lhs_op_info).op_single_type as u32 == ECL_NONE {
            /* no-op: drop the LHS, and memmove the RHS into its place */
            if lengthptr.is_null() {
                memmove(
                    (*lhs_op_info).code_start as *mut c_void,
                    (*rhs_op_info).code_start as *const c_void,
                    CU2BYTES!((*rhs_op_info).length),
                );
            }
            (*lhs_op_info).length = (*rhs_op_info).length;
            (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;
        } else if (*rhs_op_info).op_single_type as u32 == ECL_ANY {
            /* the result is !LHS: fold in the negation, and drop the RHS */
            /* Preserve the classbits, because we promise to deal with them later. */
            fold_negation(lhs_op_info, lengthptr, TRUE);
        } else if (*lhs_op_info).op_single_type as u32 == ECL_ANY {
            /* the result is !RHS: drop the LHS, memmove the RHS into its place, and
            fold in the negation */
            if lengthptr.is_null() {
                memmove(
                    (*lhs_op_info).code_start as *mut c_void,
                    (*rhs_op_info).code_start as *const c_void,
                    CU2BYTES!((*rhs_op_info).length),
                );
            }
            (*lhs_op_info).length = (*rhs_op_info).length;
            (*lhs_op_info).op_single_type = (*rhs_op_info).op_single_type;

            /* Preserve the classbits, because we promise to deal with them later. */
            fold_negation(lhs_op_info, lengthptr, TRUE);
        } else {
            /* Both of LHS & RHS are either ECL_XCLASS, or compound operations. */
            if !lengthptr.is_null() {
                *lengthptr = (*lengthptr).wrapping_add(1);
            } else {
                *(*rhs_op_info).code_start.add((*rhs_op_info).length) = ECL_XOR as u8;
            }
            (*lhs_op_info).length = (*lhs_op_info)
                .length
                .wrapping_add((*rhs_op_info).length.wrapping_add(1));
            (*lhs_op_info).op_single_type = 0;
        }

        let mut i: c_int = 0;
        while i < 8 {
            (*lhs_op_info).bits.classwords[i as usize] ^= (*rhs_op_info).bits.classwords[i as usize];
            i += 1;
        }
    }
    /* LCOV_EXCL_START */
    else {
        /* PCRE2_DEBUG_UNREACHABLE(); */
    }
    /* LCOV_EXCL_STOP */
}

/* This function consumes a group of implicitly-unioned class elements.
These can be characters, ranges, properties, or nested classes, as long
as they are all joined by being placed adjacently. */

unsafe fn compile_class_operand(
    context: *mut eclass_context,
    negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    let mut ptr: *mut u32 = *pptr;
    let prev_ptr: *mut u32;
    let mut code: *mut PCRE2_UCHAR = *pcode;
    let code_start: *mut PCRE2_UCHAR = code;
    let prev_length: PCRE2_SIZE = if !lengthptr.is_null() { *lengthptr } else { 0 };
    let extra_length: PCRE2_SIZE;
    let meta: u32 = META_CODE!(*ptr);

    'done: {
        if meta == META_CLASS_EMPTY_NOT || meta == META_CLASS_EMPTY {
            ptr = ptr.add(1);
            (*pop_info).length = 1;
            if ((meta == META_CLASS_EMPTY) as BOOL) == negated {
                (*pop_info).op_single_type = ECL_ANY as u8;
                *code = (*pop_info).op_single_type;
                code = code.add(1);
                memset(
                    (*pop_info).bits.classbits.as_mut_ptr() as *mut c_void,
                    0xff,
                    32,
                );
            } else {
                (*pop_info).op_single_type = ECL_NONE as u8;
                *code = (*pop_info).op_single_type;
                code = code.add(1);
                memset((*pop_info).bits.classbits.as_mut_ptr() as *mut c_void, 0, 32);
            }
        } else {
            if meta == META_CLASS || meta == META_CLASS_NOT {
                if (*ptr & CLASS_IS_ECLASS) != 0 {
                    if compile_eclass_nested(context, negated, &mut ptr, &mut code, pop_info,
                                             lengthptr) == FALSE
                    {
                        return FALSE;
                    }

                    /* PCRE2_ASSERT(*ptr == META_CLASS_END); */
                    ptr = ptr.add(1);
                    break 'done;
                }

                ptr = ptr.add(1);
                /* Fall through */
            }

            /* Scan forward characters, ranges, and properties.
            For example: inside [a-z_ -- m] we don't have brackets around "a-z_" but
            we still need to collect that fragment up into a "leaf" OP_CLASS. */

            prev_ptr = ptr;
            ptr = _pcre2_compile_class_not_nested_8(
                (*context).options,
                (*context).xoptions,
                ptr,
                &mut code,
                (((meta != META_CLASS_NOT) as BOOL) == negated) as BOOL,
                std::ptr::addr_of_mut!((*context).needs_bitmap),
                (*context).errorcodeptr,
                (*context).cb,
                lengthptr,
            );
            if ptr.is_null() {
                return FALSE;
            }

            /* We must have a 100% guarantee that ptr increases when
            compile_class_operand() returns, even on Release builds, so that we can
            statically prove our loops terminate. */
            /* LCOV_EXCL_START */
            if ptr <= prev_ptr {
                /* PCRE2_DEBUG_UNREACHABLE(); */
                return FALSE;
            }
            /* LCOV_EXCL_STOP */

            /* If we fell through above, consume the closing ']'. */
            if meta == META_CLASS || meta == META_CLASS_NOT {
                /* PCRE2_ASSERT(*ptr == META_CLASS_END); */
                ptr = ptr.add(1);
            }

            /* Regardless of whether (lengthptr == NULL), some data will still be written
            out to *pcode, which we need: we have to peek at it, to transform the opcode
            into the ECLASS version (since we need to hoist up the bitmaps). */
            extra_length = if !lengthptr.is_null() {
                (*lengthptr).wrapping_sub(prev_length)
            } else {
                0
            };

            /* Easiest case: convert OP_ALLANY to ECL_ANY */

            if *code_start as u32 == OP_ALLANY {
                (*pop_info).length = 1;
                (*pop_info).op_single_type = ECL_ANY as u8;
                *code_start = (*pop_info).op_single_type;
                memset(
                    (*pop_info).bits.classbits.as_mut_ptr() as *mut c_void,
                    0xff,
                    32,
                );
            }
            /* For OP_CLASS and OP_NCLASS, we hoist out the bitmap and convert to
            ECL_NONE / ECL_ANY respectively. */
            else if *code_start as u32 == OP_CLASS || *code_start as u32 == OP_NCLASS {
                (*pop_info).length = 1;
                (*pop_info).op_single_type = (if *code_start as u32 == OP_CLASS {
                    ECL_NONE
                } else {
                    ECL_ANY
                }) as u8;
                *code_start = (*pop_info).op_single_type;
                memcpy(
                    (*pop_info).bits.classbits.as_mut_ptr() as *mut c_void,
                    code_start.add(1) as *const c_void,
                    32,
                );
                /* Rewind the code pointer, but make sure we adjust *lengthptr, because we
                do need to reserve that space (even though we only use it temporarily). */
                if !lengthptr.is_null() {
                    *lengthptr = (*lengthptr)
                        .wrapping_add(code.offset_from(code_start.add(1)) as PCRE2_SIZE);
                }
                code = code_start.add(1);

                if (*context).needs_bitmap == 0 && *code_start as u32 == ECL_NONE {
                    let classwords: *mut u32 = (*pop_info).bits.classwords.as_mut_ptr();

                    let mut i: c_int = 0;
                    while i < 8 {
                        if *classwords.add(i as usize) != 0 {
                            (*context).needs_bitmap = TRUE;
                            break;
                        }
                        i += 1;
                    }
                } else {
                    (*context).needs_bitmap = TRUE;
                }
            }
            /* Finally, for OP_XCLASS we hoist out the bitmap (if any), and convert to
            ECL_XCLASS. */
            else {
                /* PCRE2_ASSERT(*code_start == OP_XCLASS); */
                (*pop_info).op_single_type = ECL_XCLASS as u8;
                *code_start = (*pop_info).op_single_type;

                memcpy(
                    (*pop_info).bits.classbits.as_mut_ptr() as *mut c_void,
                    (*(*context).cb).classbits.classbits.as_ptr() as *const c_void,
                    32,
                );
                (*pop_info).length =
                    (code.offset_from(code_start) as PCRE2_SIZE).wrapping_add(extra_length);
            }
        } /* End of switch(meta) */

        (*pop_info).code_start = if lengthptr.is_null() {
            code_start
        } else {
            std::ptr::null_mut()
        };

        if !lengthptr.is_null() {
            *lengthptr =
                (*lengthptr).wrapping_add(code.offset_from(code_start) as PCRE2_SIZE);
            code = code_start;
        }
    }

    /* DONE: */
    *pptr = ptr;
    *pcode = code;
    TRUE
}

/* This function consumes a group of implicitly-unioned class elements.
These can be characters, ranges, properties, or nested classes, as long
as they are all joined by being placed adjacently. */

unsafe fn compile_class_juxtaposition(
    context: *mut eclass_context,
    negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    let mut ptr: *mut u32 = *pptr;
    let mut code: *mut PCRE2_UCHAR = *pcode;

    /* See compile_class_binary_loose() for comments on compile-time folding of
    the "negated" flag. */

    /* Because it's a non-empty class, there must be an operand at the start. */
    if compile_class_operand(context, negated, &mut ptr, &mut code, pop_info, lengthptr) == FALSE {
        return FALSE;
    }

    while *ptr != META_CLASS_END && !(*ptr >= META_ECLASS_AND && *ptr <= META_ECLASS_NOT) {
        let op: u32;
        let rhs_negated: BOOL;
        let mut rhs_op_info: eclass_op_info = std::mem::zeroed();

        if negated != 0 {
            /* !(A juxtapose B)  ->  !A && !B */
            op = ECL_AND;
            rhs_negated = TRUE;
        } else {
            /* A juxtapose B  ->  A || B */
            op = ECL_OR;
            rhs_negated = FALSE;
        }

        /* An operand must follow the operator. */
        if compile_class_operand(
            context,
            rhs_negated,
            &mut ptr,
            &mut code,
            &mut rhs_op_info,
            lengthptr,
        ) == FALSE
        {
            return FALSE;
        }

        /* Convert infix to postfix (RPN). */
        fold_binary(op as c_int, pop_info, &mut rhs_op_info, lengthptr);
        if lengthptr.is_null() {
            code = (*pop_info).code_start.add((*pop_info).length);
        }
    }

    *pptr = ptr;
    *pcode = code;
    TRUE
}

/* This function consumes unary prefix operators. */

unsafe fn compile_class_unary(
    context: *mut eclass_context,
    mut negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    let mut ptr: *mut u32 = *pptr;

    while *ptr == META_ECLASS_NOT {
        ptr = ptr.add(1);
        negated = if negated == 0 { 1 } else { 0 };
    }

    *pptr = ptr;
    /* Because it's a non-empty class, there must be an operand. */
    if compile_class_juxtaposition(context, negated, pptr, pcode, pop_info, lengthptr) == FALSE {
        return FALSE;
    }

    TRUE
}

/* This function consumes tightly-binding binary operators. */

unsafe fn compile_class_binary_tight(
    context: *mut eclass_context,
    negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    let mut ptr: *mut u32 = *pptr;
    let mut code: *mut PCRE2_UCHAR = *pcode;

    /* See compile_class_binary_loose() for comments on compile-time folding of
    the "negated" flag. */

    /* Because it's a non-empty class, there must be an operand at the start. */
    if compile_class_unary(context, negated, &mut ptr, &mut code, pop_info, lengthptr) == FALSE {
        return FALSE;
    }

    while *ptr == META_ECLASS_AND {
        let op: u32;
        let rhs_negated: BOOL;
        let mut rhs_op_info: eclass_op_info = std::mem::zeroed();

        if negated != 0 {
            /* !(A && B)  ->  !A || !B */
            op = ECL_OR;
            rhs_negated = TRUE;
        } else {
            /* A && B  ->  A && B */
            op = ECL_AND;
            rhs_negated = FALSE;
        }

        ptr = ptr.add(1);

        /* An operand must follow the operator. */
        if compile_class_unary(
            context,
            rhs_negated,
            &mut ptr,
            &mut code,
            &mut rhs_op_info,
            lengthptr,
        ) == FALSE
        {
            return FALSE;
        }

        /* Convert infix to postfix (RPN). */
        fold_binary(op as c_int, pop_info, &mut rhs_op_info, lengthptr);
        if lengthptr.is_null() {
            code = (*pop_info).code_start.add((*pop_info).length);
        }
    }

    *pptr = ptr;
    *pcode = code;
    TRUE
}

/* This function consumes loosely-binding binary operators. */

unsafe fn compile_class_binary_loose(
    context: *mut eclass_context,
    negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    let mut ptr: *mut u32 = *pptr;
    let mut code: *mut PCRE2_UCHAR = *pcode;

    /* We really want to fold the negation operator, if at all possible, so that
    simple cases can be reduced down. In particular, in 8-bit no-UTF mode, we want
    to produce a fully-folded expression, so that we can guarantee not to emit any
    OP_ECLASS codes (in the same way that we never emit OP_XCLASS in this mode).

    This has the consequence that with a little ingenuity, we can in fact avoid
    emitting (nearly...) all cases of the "NOT" operator. Imagine that we have:
        !(A ...
    We have parsed the preceding "!", and we are about to parse the "A" operand. We
    don't know yet whether there will even be a following binary operand! Both of
    these are possibilities for what follows:
        !(A && B)
        !(A)
    However, we can still fold the "!" into the "A" operand, because no matter what
    the following binary operator will be, we can produce an expression which is
    equivalent. */

    /* Because it's a non-empty class, there must be an operand at the start. */
    if compile_class_binary_tight(context, negated, &mut ptr, &mut code, pop_info, lengthptr)
        == FALSE
    {
        return FALSE;
    }

    while *ptr >= META_ECLASS_OR && *ptr <= META_ECLASS_XOR {
        let op: u32;
        let op_neg: BOOL;
        let rhs_negated: BOOL;
        let mut rhs_op_info: eclass_op_info = std::mem::zeroed();

        if negated != 0 {
            /* The whole expression is being negated; we respond by unconditionally
            negating the LHS A, before seeing what follows. And hooray! We can recover,
            no matter what follows. */
            /* !(A || B)   ->  !A && !B                     */
            /* !(A -- B)   ->  !(A && !B)    ->  !A || B    */
            /* !(A XOR B)  ->  !(!A XOR !B)  ->  !A XNOR !B */
            op = if *ptr == META_ECLASS_OR {
                ECL_AND
            } else if *ptr == META_ECLASS_SUB {
                ECL_OR
            } else {
                /* *ptr == META_ECLASS_XOR */
                ECL_XOR
            };
            op_neg = (*ptr == META_ECLASS_XOR) as BOOL;
            rhs_negated = (*ptr != META_ECLASS_SUB) as BOOL;
        } else {
            /* A || B   ->  A || B  */
            /* A -- B   ->  A && !B */
            /* A XOR B  ->  A XOR B */
            op = if *ptr == META_ECLASS_OR {
                ECL_OR
            } else if *ptr == META_ECLASS_SUB {
                ECL_AND
            } else {
                /* *ptr == META_ECLASS_XOR */
                ECL_XOR
            };
            op_neg = FALSE;
            rhs_negated = (*ptr == META_ECLASS_SUB) as BOOL;
        }

        ptr = ptr.add(1);

        /* An operand must follow the operator. */
        if compile_class_binary_tight(
            context,
            rhs_negated,
            &mut ptr,
            &mut code,
            &mut rhs_op_info,
            lengthptr,
        ) == FALSE
        {
            return FALSE;
        }

        /* Convert infix to postfix (RPN). */
        fold_binary(op as c_int, pop_info, &mut rhs_op_info, lengthptr);
        if op_neg != 0 {
            fold_negation(pop_info, lengthptr, FALSE);
        }
        if lengthptr.is_null() {
            code = (*pop_info).code_start.add((*pop_info).length);
        }
    }

    *pptr = ptr;
    *pcode = code;
    TRUE
}

/* This function converts the META codes in pptr into opcodes written to
pcode. The pptr must start at a META_CLASS or META_CLASS_NOT.

The class is compiled as a left-associative sequence of operator
applications.

The pptr will be left pointing at the matching META_CLASS_END. */

unsafe fn compile_eclass_nested(
    context: *mut eclass_context,
    mut negated: BOOL,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    pop_info: *mut eclass_op_info,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    let mut ptr: *mut u32 = *pptr;

    /* The CLASS_IS_ECLASS bit must be set since it is a nested class. */
    /* PCRE2_ASSERT(*ptr == (META_CLASS | CLASS_IS_ECLASS) ||
                    *ptr == (META_CLASS_NOT | CLASS_IS_ECLASS)); */

    if {
        let t = *ptr;
        ptr = ptr.add(1);
        t
    } == (META_CLASS_NOT | CLASS_IS_ECLASS)
    {
        negated = if negated == 0 { 1 } else { 0 };
    }

    *pptr = (*pptr).add(1);

    /* Because it's a non-empty class, there must be an operand at the start. */
    if compile_class_binary_loose(context, negated, pptr, pcode, pop_info, lengthptr) == FALSE {
        return FALSE;
    }

    TRUE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_class_nested_8(
    options: u32,
    xoptions: u32,
    pptr: *mut *mut u32,
    pcode: *mut *mut PCRE2_UCHAR,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
) -> BOOL {
    let mut context: eclass_context = std::mem::zeroed();
    let mut op_info: eclass_op_info = std::mem::zeroed();
    let previous_length: PCRE2_SIZE = if !lengthptr.is_null() { *lengthptr } else { 0 };
    let mut code: *mut PCRE2_UCHAR = *pcode;
    let previous: *mut PCRE2_UCHAR;
    let mut allbitsone: BOOL = TRUE;

    context.needs_bitmap = FALSE;
    context.options = options;
    context.xoptions = xoptions;
    context.errorcodeptr = errorcodeptr;
    context.cb = cb;

    previous = code;
    *code = OP_ECLASS as u8;
    code = code.add(1);
    code = code.add(LINK_SIZE);
    *code = 0; /* Flags, currently zero. */
    code = code.add(1);
    if compile_eclass_nested(
        &mut context,
        FALSE,
        pptr,
        &mut code,
        &mut op_info,
        lengthptr,
    ) == FALSE
    {
        return FALSE;
    }

    if !lengthptr.is_null() {
        *lengthptr = (*lengthptr).wrapping_add(code.offset_from(previous) as PCRE2_SIZE);
        code = previous;
        /* (*lengthptr - previous_length) now holds the amount of buffer that
        we require to make the call to compile_class_nested() with
        lengthptr = NULL, and including the (1+LINK_SIZE+1) that we write out
        before that call. */
    }

    /* Do some useful counting of what's in the bitmap. */
    let mut i: c_int = 0;
    while i < 8 {
        if op_info.bits.classwords[i as usize] != 0xffffffff {
            allbitsone = FALSE;
            break;
        }
        i += 1;
    }

    /* After constant-folding the extended class syntax, it may turn out to be
    a simple class after all. In that case, we can unwrap it from the
    OP_ECLASS container - and in fact, we must do so, because in 8-bit
    no-Unicode mode the matcher is compiled without support for OP_ECLASS. */

    if op_info.op_single_type != 0 {
        /* Rewind back over the OP_ECLASS. */
        code = previous;

        /* If the bits are all ones, and the "high characters" are all matched
        too, we use a special-cased encoding of OP_ALLANY. */

        if op_info.op_single_type as u32 == ECL_ANY && allbitsone != 0 {
            /* Advancing code means rewinding lengthptr, at this point. */
            if !lengthptr.is_null() {
                *lengthptr = (*lengthptr).wrapping_sub(1);
            }
            *code = OP_ALLANY as u8;
            code = code.add(1);
        }
        /* If the high bits are all matched / all not-matched, then we emit an
        OP_NCLASS/OP_CLASS respectively. */
        else if op_info.op_single_type as u32 == ECL_ANY
            || op_info.op_single_type as u32 == ECL_NONE
        {
            let required_len: PCRE2_SIZE = 1 + 32;

            if !lengthptr.is_null() {
                if required_len > (*lengthptr).wrapping_sub(previous_length) {
                    *lengthptr = previous_length.wrapping_add(required_len);
                }
            }

            /* Advancing code means rewinding lengthptr, at this point. */
            if !lengthptr.is_null() {
                *lengthptr = (*lengthptr).wrapping_sub(required_len);
            }
            *code = (if op_info.op_single_type as u32 == ECL_ANY {
                OP_NCLASS
            } else {
                OP_CLASS
            }) as u8;
            code = code.add(1);
            memcpy(
                code as *mut c_void,
                op_info.bits.classbits.as_ptr() as *const c_void,
                32,
            );
            code = code.add(32);
        }
        /* Otherwise, we have an ECL_XCLASS, so we have the OP_XCLASS data
        there, but, we pulled out its bitmap into op_info, so now we have to
        put that back into the OP_XCLASS. */
        else {
            let need_map: BOOL = context.needs_bitmap;
            let required_len: PCRE2_SIZE;

            /* PCRE2_ASSERT(op_info.op_single_type == ECL_XCLASS); */
            required_len = op_info
                .length
                .wrapping_add(if need_map != 0 { 32 } else { 0 });

            if !lengthptr.is_null() {
                /* Don't unconditionally request all the space we need - we may
                already have asked for more during processing of the ECLASS. */
                if required_len > (*lengthptr).wrapping_sub(previous_length) {
                    *lengthptr = previous_length.wrapping_add(required_len);
                }

                /* The code we write out here won't be ignored, even during the
                (lengthptr != NULL) phase, because if there's a following quantifier
                it will peek backwards. So we do have to write out a (truncated)
                OP_XCLASS, even on this branch. */
                *lengthptr = (*lengthptr).wrapping_sub(1 + LINK_SIZE + 1);
                *code = OP_XCLASS as u8;
                code = code.add(1);
                PUT!(code, 0, (1 + LINK_SIZE + 1) as c_int);
                code = code.add(LINK_SIZE);
                *code = 0;
                code = code.add(1);
            } else {
                let rest: *mut PCRE2_UCHAR;
                let rest_len: PCRE2_SIZE;
                let flags: PCRE2_UCHAR;

                /* 1 unit: OP_XCLASS | LINK_SIZE units | 1 unit: flags | ...rest */
                /* PCRE2_ASSERT(op_info.length >= 1 + LINK_SIZE + 1); */
                rest = op_info.code_start.add(1 + LINK_SIZE + 1);
                rest_len = op_info
                    .code_start
                    .add(op_info.length)
                    .offset_from(rest) as PCRE2_SIZE;

                /* First read any data we use, before memmove splats it. */
                flags = *op_info.code_start.add(1 + LINK_SIZE);
                /* PCRE2_ASSERT((flags & XCL_MAP) == 0); */

                /* Next do the memmove before any writes. */
                memmove(
                    code.add(1 + LINK_SIZE + 1 + (if need_map != 0 { 32 } else { 0 }))
                        as *mut c_void,
                    rest as *const c_void,
                    CU2BYTES!(rest_len),
                );

                /* Finally write the header data. */
                *code = OP_XCLASS as u8;
                code = code.add(1);
                PUT!(code, 0, required_len as c_int);
                code = code.add(LINK_SIZE);
                *code = flags | (if need_map != 0 { XCL_MAP as u8 } else { 0 });
                code = code.add(1);
                if need_map != 0 {
                    memcpy(
                        code as *mut c_void,
                        op_info.bits.classbits.as_ptr() as *const c_void,
                        32,
                    );
                    code = code.add(32);
                }
                code = code.add(rest_len);
            }
        }
    }
    /* Otherwise, we're going to keep the OP_ECLASS. However, again we need
    to do some adjustment to insert the bitmap if we have one. */
    else {
        let need_map: BOOL = context.needs_bitmap;
        let required_len: PCRE2_SIZE = (1 + LINK_SIZE + 1 + (if need_map != 0 { 32 } else { 0 }))
            .wrapping_add(op_info.length);

        if !lengthptr.is_null() {
            if required_len > (*lengthptr).wrapping_sub(previous_length) {
                *lengthptr = previous_length.wrapping_add(required_len);
            }

            /* As for the XCLASS branch above, we do have to write out a dummy
            OP_ECLASS, because of the backwards peek by the quantifier code. Write
            out a (truncated) OP_ECLASS, even on this branch. */
            *lengthptr = (*lengthptr).wrapping_sub(1 + LINK_SIZE + 1);
            *code = OP_ECLASS as u8;
            code = code.add(1);
            PUT!(code, 0, (1 + LINK_SIZE + 1) as c_int);
            code = code.add(LINK_SIZE);
            *code = 0;
            code = code.add(1);
        } else {
            if need_map != 0 {
                let map_start: *mut PCRE2_UCHAR = previous.add(1 + LINK_SIZE + 1);
                *previous.add(1 + LINK_SIZE) |= ECL_MAP as u8;
                memmove(
                    map_start.add(32) as *mut c_void,
                    map_start as *const c_void,
                    CU2BYTES!(code.offset_from(map_start) as PCRE2_SIZE),
                );
                memcpy(
                    map_start as *mut c_void,
                    op_info.bits.classbits.as_ptr() as *const c_void,
                    32,
                );
                code = code.add(32);
            }
            PUT!(previous, 1, code.offset_from(previous) as c_int);
        }
    }

    *pcode = code;
    TRUE
}

/* End of pcre2_compile_class.c */
