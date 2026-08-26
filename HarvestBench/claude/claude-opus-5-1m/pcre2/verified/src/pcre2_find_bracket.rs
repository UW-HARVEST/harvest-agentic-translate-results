// Translated from c_src/src/pcre2_find_bracket.c
use crate::internal::*;

/* This module contains a single function that scans through a compiled pattern
until it finds a capturing bracket with the given number, or, if the number is
negative, an instance of OP_REVERSE or OP_VREVERSE for a lookbehind. The
function is called from pcre2_compile.c and also from pcre2_study.c when
finding the minimum matching length. */

/*************************************************
*    Scan compiled regex for specific bracket    *
*************************************************/

/*
Arguments:
  code        points to start of expression
  utf         TRUE in UTF mode
  number      the required bracket number or negative to find a lookbehind

Returns:      pointer to the opcode for the bracket, or NULL if not found
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_find_bracket_8(
    code: PCRE2_SPTR,
    utf: BOOL,
    number: c_int,
) -> PCRE2_SPTR {
    let mut code: PCRE2_SPTR = code;

    loop {
        let c: PCRE2_UCHAR = *code;

        if c as u32 == OP_END {
            return std::ptr::null();
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
        /* Handle lookbehind */
        else if c as u32 == OP_REVERSE || c as u32 == OP_VREVERSE {
            if number < 0 {
                return code;
            }
            code = code.add(*_pcre2_OP_lengths_8.as_ptr().add(c as usize) as usize);
        }
        /* Handle capturing bracket */
        else if c as u32 == OP_CBRA
            || c as u32 == OP_SCBRA
            || c as u32 == OP_CBRAPOS
            || c as u32 == OP_SCBRAPOS
        {
            let n: c_int = GET2!(code, 1 + LINK_SIZE) as c_int;
            if n == number {
                return code;
            }
            code = code.add(*_pcre2_OP_lengths_8.as_ptr().add(c as usize) as usize);
        }
        /* Otherwise, we can get the item's length from the table, except that for
        repeated character types, we have to test for \p and \P, which have an extra
        two bytes of parameters, and for MARK/PRUNE/SKIP/THEN with an argument, we
        must add in its length. */
        else {
            match c as u32 {
                OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEQUERY
                | OP_TYPEMINQUERY | OP_TYPEPOSSTAR | OP_TYPEPOSPLUS | OP_TYPEPOSQUERY => {
                    if *code.add(1) as u32 == OP_PROP || *code.add(1) as u32 == OP_NOTPROP {
                        code = code.add(2);
                    }
                }

                OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEEXACT | OP_TYPEPOSUPTO => {
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

            /* In UTF-8 and UTF-16 modes, opcodes that are followed by a character may be
            followed by a multi-byte character. The length in the table is a minimum, so
            we have to arrange to skip the extra bytes. */

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

/* End of pcre2_find_bracket.c */
