/* Translated from c_src/src/pcre2_auto_possess.c lines 45-546 */

/* This macro represents the max size of list[] and that is used to keep
track of UCD info in several places, it should be kept on sync with the
value used by GenerateUcd.py */

const MAX_LIST: usize = 8;

/*************************************************
*        Tables for auto-possessification        *
*************************************************/

/* This table is used to check whether auto-possessification is possible
between adjacent character-type opcodes. The left-hand (repeated) opcode is
used to select the row, and the right-hand opcode is use to select the column.
A value of 1 means that auto-possessification is OK. For example, the second
value in the first row means that \D+\d can be turned into \D++\d.

The Unicode property types (\P and \p) have to be present to fill out the table
because of what their opcode values are, but the table values should always be
zero because property types are handled separately in the code. The last four
columns apply to items that cannot be repeated, so there is no need to have
rows for them. Note that OP_DIGIT etc. are generated only when PCRE2_UCP is
*not* set. When it is set, \d etc. are converted into OP_(NOT_)PROP codes. */

const APTROWS: usize = (LAST_AUTOTAB_LEFT_OP - FIRST_AUTOTAB_OP + 1) as usize;
const APTCOLS: usize = (LAST_AUTOTAB_RIGHT_OP - FIRST_AUTOTAB_OP + 1) as usize;

static autoposstab: [[u8; APTCOLS]; APTROWS] = [
/* \D \d \S \s \W \w  . .+ \C \P \p \R \H \h \V \v \X \Z \z  $ $M */
  [ 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0 ],  /* \D */
  [ 1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1, 1 ],  /* \d */
  [ 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1, 1 ],  /* \S */
  [ 0, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0 ],  /* \s */
  [ 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0 ],  /* \W */
  [ 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1, 1 ],  /* \w */
  [ 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0 ],  /* .  */
  [ 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0 ],  /* .+ */
  [ 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0 ],  /* \C */
  [ 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 ],  /* \P */
  [ 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 ],  /* \p */
  [ 0, 1, 0, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0 ],  /* \R */
  [ 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0 ],  /* \H */
  [ 0, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 0, 0, 1, 0, 0 ],  /* \h */
  [ 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0 ],  /* \V */
  [ 0, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 1, 0, 0 ],  /* \v */
  [ 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0 ]   /* \X */
];

/* This table is used to check whether auto-possessification is possible
between adjacent Unicode property opcodes (OP_PROP and OP_NOTPROP). The
left-hand (repeated) opcode is used to select the row, and the right-hand
opcode is used to select the column. The values are as follows:

  0   Always return FALSE (never auto-possessify)
  1   Character groups are distinct (possessify if both are OP_PROP)
  2   Check character categories in the same group (general or particular)
  3   TRUE if the two opcodes are not the same (PROP vs NOTPROP)

  4   Check left general category vs right particular category
  5   Check right general category vs left particular category

  6   Left alphanum vs right general category
  7   Left space vs right general category
  8   Left word vs right general category

  9   Right alphanum vs left general category
 10   Right space vs left general category
 11   Right word vs left general category

 12   Left alphanum vs right particular category
 13   Left space vs right particular category
 14   Left word vs right particular category

 15   Right alphanum vs left particular category
 16   Right space vs left particular category
 17   Right word vs left particular category
*/

static propposstab: [[u8; PT_TABSIZE]; PT_TABSIZE] = [
/* LAMP GC  PC  SC  SCX ALNUM SPACE PXSPACE WORD CLIST UCNC BIDICL BOOL */
  [ 3,  0,  0,  0,   0,    3,    1,      1,   0,    0,   0,    0,    0 ],  /* PT_LAMP */
  [ 0,  2,  4,  0,   0,    9,   10,     10,  11,    0,   0,    0,    0 ],  /* PT_GC */
  [ 0,  5,  2,  0,   0,   15,   16,     16,  17,    0,   0,    0,    0 ],  /* PT_PC */
  [ 0,  0,  0,  2,   2,    0,    0,      0,   0,    0,   0,    0,    0 ],  /* PT_SC */
  [ 0,  0,  0,  2,   2,    0,    0,      0,   0,    0,   0,    0,    0 ],  /* PT_SCX */
  [ 3,  6, 12,  0,   0,    3,    1,      1,   0,    0,   0,    0,    0 ],  /* PT_ALNUM */
  [ 1,  7, 13,  0,   0,    1,    3,      3,   1,    0,   0,    0,    0 ],  /* PT_SPACE */
  [ 1,  7, 13,  0,   0,    1,    3,      3,   1,    0,   0,    0,    0 ],  /* PT_PXSPACE */
  [ 0,  8, 14,  0,   0,    0,    1,      1,   3,    0,   0,    0,    0 ],  /* PT_WORD */
  [ 0,  0,  0,  0,   0,    0,    0,      0,   0,    0,   0,    0,    0 ],  /* PT_CLIST */
  [ 0,  0,  0,  0,   0,    0,    0,      0,   0,    0,   3,    0,    0 ],  /* PT_UCNC */
  [ 0,  0,  0,  0,   0,    0,    0,      0,   0,    0,   0,    0,    0 ],  /* PT_BIDICL */
  [ 0,  0,  0,  0,   0,    0,    0,      0,   0,    0,   0,    0,    0 ]   /* PT_BOOL */
  /* PT_ANY does not need a record. */
];

/* This table is used to check whether auto-possessification is possible
between adjacent Unicode property opcodes (OP_PROP and OP_NOTPROP) when one
specifies a general category and the other specifies a particular category. The
row is selected by the general category and the column by the particular
category. The value is 1 if the particular category is not part of the general
category. */

static catposstab: [[u8; 30]; 7] = [
/* Cc Cf Cn Co Cs Ll Lm Lo Lt Lu Mc Me Mn Nd Nl No Pc Pd Pe Pf Pi Po Ps Sc Sk Sm So Zl Zp Zs */
  [ 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1 ],  /* C */
  [ 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1 ],  /* L */
  [ 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1 ],  /* M */
  [ 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1 ],  /* N */
  [ 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1 ],  /* P */
  [ 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1 ],  /* S */
  [ 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0 ]   /* Z */
];

/* This table is used when checking ALNUM, (PX)SPACE, SPACE, and WORD against
a general or particular category. The properties in each row are those
that apply to the character set in question. Duplication means that a little
unnecessary work is done when checking, but this keeps things much simpler
because they can all use the same code. For more details see the comment where
this table is used.

Note: SPACE and PXSPACE used to be different because Perl excluded VT from
"space", but from Perl 5.18 it's included, so both categories are treated the
same here. */

static posspropstab: [[u8; 4]; 3] = [
  [ ucp_L as u8, ucp_N as u8, ucp_N as u8, ucp_Nl as u8 ],  /* ALNUM, 3rd and 4th values redundant */
  [ ucp_Z as u8, ucp_Z as u8, ucp_C as u8, ucp_Cc as u8 ],  /* SPACE and PXSPACE, 2nd value redundant */
  [ ucp_L as u8, ucp_N as u8, ucp_P as u8, ucp_Po as u8 ]   /* WORD */
];

/*************************************************
*        Check a character and a property        *
*************************************************/

/* This function is called by compare_opcodes() when a property item is
adjacent to a fixed character.

Arguments:
  c            the character
  ptype        the property type
  pdata        the data for the type
  negated      TRUE if it's a negated property (\P or \p{^)

Returns:       TRUE if auto-possessifying is OK
*/

unsafe fn check_char_prop(c: u32, ptype: c_uint, pdata: c_uint, negated: BOOL) -> BOOL {
    let ok: BOOL;
    let rc: BOOL;
    let mut p: *const u32;
    let prop: *const ucd_record = GET_UCD(c);

    match ptype {
        PT_LAMP => {
            return (((*prop).chartype as u32 == ucp_Lu
                || (*prop).chartype as u32 == ucp_Ll
                || (*prop).chartype as u32 == ucp_Lt) as BOOL
                == negated) as BOOL;
        }

        PT_GC => {
            return ((pdata
                == *_pcre2_ucp_gentype_8
                    .as_ptr()
                    .add((*prop).chartype as usize)) as BOOL
                == negated) as BOOL;
        }

        PT_PC => {
            return ((pdata == (*prop).chartype as u32) as BOOL == negated) as BOOL;
        }

        PT_SC => {
            return ((pdata == (*prop).script as u32) as BOOL == negated) as BOOL;
        }

        PT_SCX => {
            ok = (pdata == (*prop).script as u32
                || MAPBIT!(
                    _pcre2_ucd_script_sets_8
                        .as_ptr()
                        .add(UCD_SCRIPTX_PROP(prop) as usize),
                    pdata
                ) != 0) as BOOL;
            return (ok == negated) as BOOL;
        }

        /* These are specials */

        PT_ALNUM => {
            return ((*_pcre2_ucp_gentype_8
                .as_ptr()
                .add((*prop).chartype as usize)
                == ucp_L
                || *_pcre2_ucp_gentype_8
                    .as_ptr()
                    .add((*prop).chartype as usize)
                    == ucp_N) as BOOL
                == negated) as BOOL;
        }

        /* Perl space used to exclude VT, but from Perl 5.18 it is included, which
        means that Perl space and POSIX space are now identical. PCRE was changed
        at release 8.34. */

        PT_SPACE     /* Perl space */
        | PT_PXSPACE /* POSIX space */ => {
            match c {
                /* HSPACE_CASES: */
                CHAR_HT | CHAR_SPACE | CHAR_NBSP | 0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002
                | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009 | 0x200a
                | 0x202f | 0x205f | 0x3000
                /* VSPACE_CASES: */
                | CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028 | 0x2029 => {
                    rc = negated;
                }

                _ => {
                    rc = ((*_pcre2_ucp_gentype_8
                        .as_ptr()
                        .add((*prop).chartype as usize)
                        == ucp_Z) as BOOL
                        == negated) as BOOL;
                }
            }
            return rc;
        }

        PT_WORD => {
            return ((*_pcre2_ucp_gentype_8
                .as_ptr()
                .add((*prop).chartype as usize)
                == ucp_L
                || *_pcre2_ucp_gentype_8
                    .as_ptr()
                    .add((*prop).chartype as usize)
                    == ucp_N
                || c == CHAR_UNDERSCORE) as BOOL
                == negated) as BOOL;
        }

        PT_CLIST => {
            p = _pcre2_ucd_caseless_sets_8
                .as_ptr()
                .add((*prop).caseset as usize);
            loop {
                if c < *p {
                    return (negated == 0) as BOOL;
                }
                let t = *p;
                p = p.add(1);
                if c == t {
                    return negated;
                }
            }
            /* Control should never reach here */
        }

        /* Haven't yet thought these through. */

        PT_BIDICL => {
            return FALSE;
        }

        PT_BOOL => {
            return FALSE;
        }

        _ => {}
    }

    return FALSE;
}

/*************************************************
*        Base opcode of repeated opcodes         *
*************************************************/

/* Returns the base opcode for repeated single character type opcodes. If the
opcode is not a repeated character type, it returns with the original value.

Arguments:  c opcode
Returns:    base opcode for the type
*/

unsafe fn get_repeat_base(c: PCRE2_UCHAR) -> PCRE2_UCHAR {
    return if c as u32 > OP_TYPEPOSUPTO {
        c
    } else if c as u32 >= OP_TYPESTAR {
        OP_TYPESTAR as PCRE2_UCHAR
    } else if c as u32 >= OP_NOTSTARI {
        OP_NOTSTARI as PCRE2_UCHAR
    } else if c as u32 >= OP_NOTSTAR {
        OP_NOTSTAR as PCRE2_UCHAR
    } else if c as u32 >= OP_STARI {
        OP_STARI as PCRE2_UCHAR
    } else {
        OP_STAR as PCRE2_UCHAR
    };
}

/*************************************************
*        Fill the character property list        *
*************************************************/

/* Checks whether the code points to an opcode that can take part in auto-
possessification, and if so, fills a list with its properties.

Arguments:
  code        points to start of expression
  utf         TRUE if in UTF mode
  ucp         TRUE if in UCP mode
  fcc         points to the case-flipping table
  list        points to output list
              list[0] will be filled with the opcode
              list[1] will be non-zero if this opcode
                can match an empty character string
              list[2..7] depends on the opcode

Returns:      points to the start of the next opcode if *code is accepted
              NULL if *code is not accepted
*/

unsafe fn get_chr_property_list(
    mut code: PCRE2_SPTR,
    utf: BOOL,
    ucp: BOOL,
    fcc: *const u8,
    list: *mut u32,
) -> PCRE2_SPTR {
    let mut c: PCRE2_UCHAR = *code;
    let base: PCRE2_UCHAR;
    let mut end: PCRE2_SPTR;
    let class_end: PCRE2_SPTR;
    let mut chr: u32 = 0;

    let mut clist_dest: *mut u32;
    let mut clist_src: *const u32;

    *list.add(0) = c as u32;
    *list.add(1) = FALSE as u32;
    code = code.add(1);

    if c as u32 >= OP_STAR && c as u32 <= OP_TYPEPOSUPTO {
        base = get_repeat_base(c);
        c = c.wrapping_sub((base as u32).wrapping_sub(OP_STAR) as PCRE2_UCHAR);

        if c as u32 == OP_UPTO
            || c as u32 == OP_MINUPTO
            || c as u32 == OP_EXACT
            || c as u32 == OP_POSUPTO
        {
            code = code.add(IMM2_SIZE);
        }

        *list.add(1) = (c as u32 != OP_PLUS
            && c as u32 != OP_MINPLUS
            && c as u32 != OP_EXACT
            && c as u32 != OP_POSPLUS) as u32;

        match base as u32 {
            OP_STAR => {
                *list.add(0) = OP_CHAR;
            }

            OP_STARI => {
                *list.add(0) = OP_CHARI;
            }

            OP_NOTSTAR => {
                *list.add(0) = OP_NOT;
            }

            OP_NOTSTARI => {
                *list.add(0) = OP_NOTI;
            }

            OP_TYPESTAR => {
                *list.add(0) = *code as u32;
                code = code.add(1);
            }

            _ => {}
        }
        c = *list.add(0) as PCRE2_UCHAR;
    }

    match c as u32 {
        OP_NOT_DIGIT | OP_DIGIT | OP_NOT_WHITESPACE | OP_WHITESPACE | OP_NOT_WORDCHAR
        | OP_WORDCHAR | OP_ANY | OP_ALLANY | OP_ANYNL | OP_NOT_HSPACE | OP_HSPACE
        | OP_NOT_VSPACE | OP_VSPACE | OP_EXTUNI | OP_EODN | OP_EOD | OP_DOLL | OP_DOLLM => {
            return code;
        }

        OP_CHAR | OP_NOT => {
            GETCHARINCTEST!(chr, code, utf);
            *list.add(2) = chr;
            *list.add(3) = NOTACHAR;
            return code;
        }

        OP_CHARI | OP_NOTI => {
            *list.add(0) = if c as u32 == OP_CHARI { OP_CHAR } else { OP_NOT };
            GETCHARINCTEST!(chr, code, utf);
            *list.add(2) = chr;

            if chr < 128 || (chr < 256 && utf == 0 && ucp == 0) {
                *list.add(3) = *fcc.add(chr as usize) as u32;
            } else {
                *list.add(3) = UCD_OTHERCASE(chr);
            }

            /* The othercase might be the same value. */

            if chr == *list.add(3) {
                *list.add(3) = NOTACHAR;
            } else {
                *list.add(4) = NOTACHAR;
            }
            return code;
        }

        OP_PROP | OP_NOTPROP => {
            if *code.add(0) as u32 != PT_CLIST {
                *list.add(2) = *code.add(0) as u32;
                *list.add(3) = *code.add(1) as u32;
                return code.add(2);
            }

            /* Convert only if we have enough space. */

            clist_src = _pcre2_ucd_caseless_sets_8
                .as_ptr()
                .add(*code.add(1) as usize);
            clist_dest = list.add(2);
            code = code.add(2);

            loop {
                if clist_dest >= list.add(MAX_LIST) {
                    /* Early return if there is not enough space. GenerateUcd.py
                    generated a list with more than 5 characters and something
                    must be done about that going forward. */
                    *list.add(2) = *code.add(0) as u32;
                    *list.add(3) = *code.add(1) as u32;
                    return code;
                }
                *clist_dest = *clist_src;
                clist_dest = clist_dest.add(1);

                let t = *clist_src;
                clist_src = clist_src.add(1);
                if !(t != NOTACHAR) {
                    break;
                }
            }

            /* All characters are stored. The terminating NOTACHAR is copied from the
            clist itself. */

            *list.add(0) = if c as u32 == OP_PROP { OP_CHAR } else { OP_NOT };
            return code;
        }

        OP_NCLASS | OP_CLASS | OP_XCLASS | OP_ECLASS => {
            if c as u32 == OP_XCLASS || c as u32 == OP_ECLASS {
                end = code.add(GET!(code, 0) as usize).sub(1);
            } else {
                end = code.add(32 / size_of::<PCRE2_UCHAR>());
            }
            class_end = end;

            match *end as u32 {
                OP_CRSTAR | OP_CRMINSTAR | OP_CRQUERY | OP_CRMINQUERY | OP_CRPOSSTAR
                | OP_CRPOSQUERY => {
                    *list.add(1) = TRUE as u32;
                    end = end.add(1);
                }

                OP_CRPLUS | OP_CRMINPLUS | OP_CRPOSPLUS => {
                    end = end.add(1);
                }

                OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                    *list.add(1) = (GET2!(end, 1) == 0) as u32;
                    end = end.add(1 + 2 * IMM2_SIZE);
                }

                _ => {}
            }
            *list.add(2) = end.offset_from(code) as u32;
            *list.add(3) = end.offset_from(class_end) as u32;
            return end;
        }

        _ => {}
    }

    return std::ptr::null(); /* Opcode not accepted */
}
