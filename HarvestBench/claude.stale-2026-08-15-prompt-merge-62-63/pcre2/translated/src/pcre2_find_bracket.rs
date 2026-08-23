use crate::pcre2_internal::*;
use core::ptr;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_find_bracket_8(
    mut code: PCRE2_SPTR,
    utf: BOOL,
    number: i32,
) -> PCRE2_SPTR {
    loop {
        let c = *code;

        if c == OP_END {
            return ptr::null();
        }

        if c == OP_XCLASS || c == OP_ECLASS {
            code = code.add(GET(code, 1) as usize);
        } else if c == OP_CALLOUT_STR {
            code = code.add(GET(code, 1 + 2 * LINK_SIZE) as usize);
        } else if c == OP_REVERSE || c == OP_VREVERSE {
            if number < 0 {
                return code;
            }
            code = code.add(_pcre2_OP_lengths_8[c as usize] as usize);
        } else if c == OP_CBRA || c == OP_SCBRA || c == OP_CBRAPOS || c == OP_SCBRAPOS {
            let n = GET2(code, 1 + LINK_SIZE) as i32;
            if n == number {
                return code;
            }
            code = code.add(_pcre2_OP_lengths_8[c as usize] as usize);
        } else {
            match c {
                OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEQUERY
                | OP_TYPEMINQUERY | OP_TYPEPOSSTAR | OP_TYPEPOSPLUS | OP_TYPEPOSQUERY => {
                    if *code.add(1) == OP_PROP || *code.add(1) == OP_NOTPROP {
                        code = code.add(2);
                    }
                }
                OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEEXACT | OP_TYPEPOSUPTO => {
                    if *code.add(1 + IMM2_SIZE) == OP_PROP || *code.add(1 + IMM2_SIZE) == OP_NOTPROP
                    {
                        code = code.add(2);
                    }
                }
                OP_MARK | OP_COMMIT_ARG | OP_PRUNE_ARG | OP_SKIP_ARG | OP_THEN_ARG => {
                    code = code.add(*code.add(1) as usize);
                }
                _ => {}
            }

            code = code.add(_pcre2_OP_lengths_8[c as usize] as usize);

            if utf != 0 {
                match c {
                    OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI | OP_EXACT | OP_EXACTI | OP_NOTEXACT
                    | OP_NOTEXACTI | OP_UPTO | OP_UPTOI | OP_NOTUPTO | OP_NOTUPTOI | OP_MINUPTO
                    | OP_MINUPTOI | OP_NOTMINUPTO | OP_NOTMINUPTOI | OP_POSUPTO | OP_POSUPTOI
                    | OP_NOTPOSUPTO | OP_NOTPOSUPTOI | OP_STAR | OP_STARI | OP_NOTSTAR
                    | OP_NOTSTARI | OP_MINSTAR | OP_MINSTARI | OP_NOTMINSTAR | OP_NOTMINSTARI
                    | OP_POSSTAR | OP_POSSTARI | OP_NOTPOSSTAR | OP_NOTPOSSTARI | OP_PLUS
                    | OP_PLUSI | OP_NOTPLUS | OP_NOTPLUSI | OP_MINPLUS | OP_MINPLUSI
                    | OP_NOTMINPLUS | OP_NOTMINPLUSI | OP_POSPLUS | OP_POSPLUSI | OP_NOTPOSPLUS
                    | OP_NOTPOSPLUSI | OP_QUERY | OP_QUERYI | OP_NOTQUERY | OP_NOTQUERYI
                    | OP_MINQUERY | OP_MINQUERYI | OP_NOTMINQUERY | OP_NOTMINQUERYI | OP_POSQUERY
                    | OP_POSQUERYI | OP_NOTPOSQUERY | OP_NOTPOSQUERYI => {
                        let prev = *code.sub(1) as u32;
                        if HAS_EXTRALEN(prev) {
                            code = code.add(GET_EXTRALEN(prev) as usize);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
