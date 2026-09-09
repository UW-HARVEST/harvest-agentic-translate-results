//! Translated from pcre2_auto_possess.c.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use core::ffi::{c_char, c_void};

use crate::ord2utf::_pcre2_ord2utf_8;
use crate::tables::{_pcre2_OP_lengths_8, _pcre2_ucp_gentype_8, _pcre2_utf8_table4};
use crate::ucd::*;
use crate::xclass::{_pcre2_eclass_8, _pcre2_xclass_8};

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
  [ 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0 ],  /* \X */
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

static propposstab: [[u8; PT_TABSIZE as usize]; PT_TABSIZE as usize] = [
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
  [ 0,  0,  0,  0,   0,    0,    0,      0,   0,    0,   0,    0,    0 ],  /* PT_BOOL */
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
  [ 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0 ],  /* Z */
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
  [ ucp_L as u8, ucp_N as u8, ucp_P as u8, ucp_Po as u8 ],  /* WORD */
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

pub(crate) unsafe fn check_char_prop(c: u32, ptype: u32, pdata: u32, negated: BOOL) -> BOOL {
    let ok: BOOL;
    let rc: BOOL;
    let mut p: *const u32;
    let prop: *const ucd_record = GET_UCD!(c);

    match ptype {
        PT_LAMP => {
            return ((((*prop).chartype as u32 == ucp_Lu
                || (*prop).chartype as u32 == ucp_Ll
                || (*prop).chartype as u32 == ucp_Lt) as BOOL)
                == negated) as BOOL;
        }

        PT_GC => {
            return (((pdata
                == *_pcre2_ucp_gentype_8.as_ptr().add((*prop).chartype as usize))
                as BOOL)
                == negated) as BOOL;
        }

        PT_PC => {
            return (((pdata == (*prop).chartype as u32) as BOOL) == negated) as BOOL;
        }

        PT_SC => {
            return (((pdata == (*prop).script as u32) as BOOL) == negated) as BOOL;
        }

        PT_SCX => {
            ok = ((pdata == (*prop).script as u32)
                || MAPBIT!(
                    _pcre2_ucd_script_sets_8
                        .as_ptr()
                        .add(UCD_SCRIPTX_PROP!(prop) as usize),
                    pdata
                ) != 0) as BOOL;
            return (ok == negated) as BOOL;
        }

        /* These are specials */

        PT_ALNUM => {
            return ((((*_pcre2_ucp_gentype_8.as_ptr().add((*prop).chartype as usize)) == ucp_L
                || (*_pcre2_ucp_gentype_8.as_ptr().add((*prop).chartype as usize)) == ucp_N)
                as BOOL)
                == negated) as BOOL;
        }

        /* Perl space used to exclude VT, but from Perl 5.18 it is included, which
        means that Perl space and POSIX space are now identical. PCRE was changed
        at release 8.34. */

        /* PT_SPACE: Perl space; PT_PXSPACE: POSIX space */
        PT_SPACE | PT_PXSPACE => {
            match c {
                /* HSPACE_CASES */
                0x09 | 0x20 | 0xa0 | 0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002 | 0x2003
                | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009 | 0x200a | 0x202f
                | 0x205f | 0x3000
                /* VSPACE_CASES */
                | 0x0a | 0x0b | 0x0c | 0x0d | 0x85 | 0x2028 | 0x2029 => {
                    rc = negated;
                }

                _ => {
                    rc = (((*_pcre2_ucp_gentype_8.as_ptr().add((*prop).chartype as usize)
                        == ucp_Z) as BOOL)
                        == negated) as BOOL;
                }
            }
            return rc;
        }

        PT_WORD => {
            return ((((*_pcre2_ucp_gentype_8.as_ptr().add((*prop).chartype as usize)) == ucp_L
                || (*_pcre2_ucp_gentype_8.as_ptr().add((*prop).chartype as usize)) == ucp_N
                || c == 0x5f /* CHAR_UNDERSCORE */) as BOOL)
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
            /* PCRE2_DEBUG_UNREACHABLE(); Control should never reach here */
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

    FALSE
}

/*************************************************
*        Base opcode of repeated opcodes         *
*************************************************/

/* Returns the base opcode for repeated single character type opcodes. If the
opcode is not a repeated character type, it returns with the original value.

Arguments:  c opcode
Returns:    base opcode for the type
*/

pub(crate) unsafe fn get_repeat_base(c: PCRE2_UCHAR) -> PCRE2_UCHAR {
    if (c as u32) > OP_TYPEPOSUPTO {
        c
    } else if (c as u32) >= OP_TYPESTAR {
        OP_TYPESTAR as PCRE2_UCHAR
    } else if (c as u32) >= OP_NOTSTARI {
        OP_NOTSTARI as PCRE2_UCHAR
    } else if (c as u32) >= OP_NOTSTAR {
        OP_NOTSTAR as PCRE2_UCHAR
    } else if (c as u32) >= OP_STARI {
        OP_STARI as PCRE2_UCHAR
    } else {
        OP_STAR as PCRE2_UCHAR
    }
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

pub(crate) unsafe fn get_chr_property_list(
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

    if (c as u32) >= OP_STAR && (c as u32) <= OP_TYPEPOSUPTO {
        base = get_repeat_base(c);
        c = c.wrapping_sub(base.wrapping_sub(OP_STAR as PCRE2_UCHAR));

        if (c as u32) == OP_UPTO
            || (c as u32) == OP_MINUPTO
            || (c as u32) == OP_EXACT
            || (c as u32) == OP_POSUPTO
        {
            code = code.add(IMM2_SIZE);
        }

        *list.add(1) = (((c as u32) != OP_PLUS
            && (c as u32) != OP_MINPLUS
            && (c as u32) != OP_EXACT
            && (c as u32) != OP_POSPLUS) as BOOL) as u32;

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
            *list.add(0) = if (c as u32) == OP_CHARI { OP_CHAR } else { OP_NOT };
            GETCHARINCTEST!(chr, code, utf);
            *list.add(2) = chr;

            if chr < 128 || (chr < 256 && utf == 0 && ucp == 0) {
                *list.add(3) = *fcc.add(chr as usize) as u32;
            } else {
                *list.add(3) = UCD_OTHERCASE!(chr);
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
            if (*code.add(0) as u32) != PT_CLIST {
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
                    /* PCRE2_DEBUG_UNREACHABLE(); Remove if it ever triggers */
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

            *list.add(0) = if (c as u32) == OP_PROP { OP_CHAR } else { OP_NOT };
            return code;
        }

        OP_NCLASS | OP_CLASS | OP_XCLASS | OP_ECLASS => {
            if (c as u32) == OP_XCLASS || (c as u32) == OP_ECLASS {
                end = code.add(GET!(code, 0) as usize).wrapping_sub(1);
            } else {
                end = code.add(32 /* 32 / sizeof(PCRE2_UCHAR) */);
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
                    *list.add(1) = ((GET2!(end, 1) == 0) as BOOL) as u32;
                    end = end.add(1 + 2 * IMM2_SIZE);
                }

                _ => {}
            }
            *list.add(2) = (end as usize - code as usize) as u32;
            *list.add(3) = (end as usize - class_end as usize) as u32;
            return end;
        }

        _ => {}
    }

    core::ptr::null() /* Opcode not accepted */
}

/* Helpers for flat (row-major) access to the static tables above, matching the
C code's unchecked 2-D subscripting. */

#[inline]
unsafe fn propposstab_at(row: u32, col: u32) -> u8 {
    *(propposstab.as_ptr() as *const u8).add((row as usize) * (PT_TABSIZE as usize) + (col as usize))
}

#[inline]
unsafe fn catposstab_at(row: u32, col: u32) -> u8 {
    *(catposstab.as_ptr() as *const u8).add((row as usize) * 30 + (col as usize))
}

#[inline]
unsafe fn posspropstab_row(row: i32) -> *const u8 {
    (posspropstab.as_ptr() as *const u8).add((row as usize) * 4)
}

#[inline]
unsafe fn autoposstab_at(row: u32, col: u32) -> u8 {
    *(autoposstab.as_ptr() as *const u8).add((row as usize) * APTCOLS + (col as usize))
}

/*************************************************
*    Scan further character sets for match       *
*************************************************/

/* Checks whether the base and the current opcode have a common character, in
which case the base cannot be possessified.

Arguments:
  code        points to the byte code
  utf         TRUE in UTF mode
  ucp         TRUE in UCP mode
  cb          compile data block
  base_list   the data list of the base opcode
  base_end    the end of the base opcode
  rec_limit   points to recursion depth counter

Returns:      TRUE if the auto-possessification is possible
*/

pub(crate) unsafe fn compare_opcodes(
    mut code: PCRE2_SPTR,
    utf: BOOL,
    ucp: BOOL,
    cb: *const compile_block,
    base_list: *const u32,
    base_end: PCRE2_SPTR,
    rec_limit: *mut i32,
) -> BOOL {
    let mut c: PCRE2_UCHAR;
    let mut list_arr: [u32; MAX_LIST] = [0; MAX_LIST];
    let list: *mut u32 = list_arr.as_mut_ptr();
    let mut chr_ptr: *const u32 = core::ptr::null();
    let mut ochr_ptr: *const u32;
    let mut list_ptr: *const u32 = core::ptr::null();
    let mut next_code: PCRE2_SPTR;
    let mut xclass_flags: PCRE2_SPTR;
    let mut class_bitset: *const u8;
    let mut set1: *const u8 = core::ptr::null();
    let mut set2: *const u8 = core::ptr::null();
    let mut set_end: *const u8;
    let mut chr: u32;
    let mut accepted: BOOL;
    let mut invert_bits: BOOL;
    let mut entered_a_group: BOOL = FALSE;

    *rec_limit -= 1;
    if *rec_limit <= 0 {
        return FALSE; /* Recursion has gone too deep */
    }

    /* Note: the base_list[1] contains whether the current opcode has a greedy
    (represented by a non-zero value) quantifier. This is a different from
    other character type lists, which store here that the character iterator
    matches to an empty string (also represented by a non-zero value). */

    'outer: loop {
        let mut bracode: PCRE2_SPTR;

        /* All operations move the code pointer forward.
        Therefore infinite recursions are not possible. */

        c = *code;

        /* Skip over callouts */

        if (c as u32) == OP_CALLOUT {
            code = code.add(*_pcre2_OP_lengths_8.as_ptr().add(c as usize) as usize);
            continue 'outer;
        }

        if (c as u32) == OP_CALLOUT_STR {
            code = code.add(GET!(code, 1 + 2 * LINK_SIZE) as usize);
            continue 'outer;
        }

        /* At the end of a branch, skip to the end of the group and process it. */

        if (c as u32) == OP_ALT {
            loop {
                code = code.add(GET!(code, 1) as usize);
                if !((*code as u32) == OP_ALT) {
                    break;
                }
            }
            c = *code;
        }

        /* Inspect the next opcode. */

        match c as u32 {
            /* We can always possessify a greedy iterator at the end of the pattern,
            which is reached after skipping over the final OP_KET. A non-greedy
            iterator must never be possessified. */
            OP_END => {
                return (*base_list.add(1) != 0) as BOOL;
            }

            /* When an iterator is at the end of certain kinds of group we can inspect
            what follows the group by skipping over the closing ket. Note that this
            does not apply to OP_KETRMAX or OP_KETRMIN because what follows any given
            iteration is variable (could be another iteration or could be the next
            item). As these two opcodes are not listed in the next switch, they will
            end up as the next code to inspect, and return FALSE by virtue of being
            unsupported. */
            OP_KET | OP_KETRPOS => {
                /* The non-greedy case cannot be converted to a possessive form. */

                if *base_list.add(1) == 0 {
                    return FALSE;
                }

                /* If the bracket is capturing it might be referenced by an OP_RECURSE
                so its last iterator can never be possessified if the pattern contains
                recursions. (This could be improved by keeping a list of group numbers
                that are called by recursion.) */

                bracode = code.wrapping_sub(GET!(code, 1) as usize);
                'bsw: {
                    match *bracode as u32 {
                        OP_CBRA | OP_SCBRA | OP_CBRAPOS | OP_SCBRAPOS => {
                            if (*cb).had_recurse != 0 {
                                return FALSE;
                            }
                            break 'bsw;
                        }

                        /* A script run might have to backtrack if the iterated item can
                        match characters from more than one script. So give up unless
                        repeating an explicit character. */
                        OP_SCRIPT_RUN => {
                            if *base_list.add(0) != OP_CHAR && *base_list.add(0) != OP_CHARI {
                                return FALSE;
                            }
                            break 'bsw;
                        }

                        /* Atomic sub-patterns and forward assertions can always
                        auto-possessify their last iterator. However, if the group was
                        entered as a result of checking a previous iterator, this is not
                        possible. */
                        OP_ASSERT | OP_ASSERT_NOT | OP_ONCE => {
                            return (entered_a_group == 0) as BOOL;
                        }

                        /* Fixed-length lookbehinds can be treated the same way, but
                        variable length lookbehinds must not auto-possessify their last
                        iterator. Note that in order to identify a variable length
                        lookbehind we must check through all branches, because some may be
                        of fixed length. */
                        OP_ASSERTBACK | OP_ASSERTBACK_NOT => {
                            loop {
                                if (*bracode.add(1 + LINK_SIZE) as u32) == OP_VREVERSE {
                                    return FALSE; /* Variable */
                                }
                                bracode = bracode.add(GET!(bracode, 1) as usize);
                                if !((*bracode as u32) == OP_ALT) {
                                    break;
                                }
                            }
                            return (entered_a_group == 0) as BOOL; /* Not variable length */
                        }

                        /* Non-atomic assertions - don't possessify last iterator. This
                        needs more thought. */
                        OP_ASSERT_NA | OP_ASSERTBACK_NA => {
                            return FALSE;
                        }

                        _ => {}
                    }
                }

                /* Skip over the bracket and inspect what comes next. */

                code = code.add(*_pcre2_OP_lengths_8.as_ptr().add(c as usize) as usize);
                continue 'outer;
            }

            /* Handle cases where the next item is a group. */
            OP_ONCE | OP_BRA | OP_CBRA => {
                next_code = code.add(GET!(code, 1) as usize);
                code = code.add(*_pcre2_OP_lengths_8.as_ptr().add(c as usize) as usize);

                /* Check each branch. We have to recurse a level for all but the last
                branch. */

                while (*next_code as u32) == OP_ALT {
                    if compare_opcodes(code, utf, ucp, cb, base_list, base_end, rec_limit) == 0 {
                        return FALSE;
                    }
                    code = next_code.add(1 + LINK_SIZE);
                    next_code = next_code.add(GET!(next_code, 1) as usize);
                }

                entered_a_group = TRUE;
                continue 'outer;
            }

            OP_BRAZERO | OP_BRAMINZERO => {
                next_code = code.add(1);
                if (*next_code as u32) != OP_BRA
                    && (*next_code as u32) != OP_CBRA
                    && (*next_code as u32) != OP_ONCE
                {
                    return FALSE;
                }

                loop {
                    next_code = next_code.add(GET!(next_code, 1) as usize);
                    if !((*next_code as u32) == OP_ALT) {
                        break;
                    }
                }

                /* The bracket content will be checked by the OP_BRA/OP_CBRA case
                above. */

                next_code = next_code.add(1 + LINK_SIZE);
                if compare_opcodes(next_code, utf, ucp, cb, base_list, base_end, rec_limit) == 0 {
                    return FALSE;
                }

                code = code.add(*_pcre2_OP_lengths_8.as_ptr().add(c as usize) as usize);
                continue 'outer;
            }

            /* The next opcode does not need special handling; fall through and use it
            to see if the base can be possessified. */
            _ => {}
        }

        /* We now have the next appropriate opcode to compare with the base. Check
        for a supported opcode, and load its properties. */

        code = get_chr_property_list(code, utf, ucp, (*cb).fcc, list);
        if code.is_null() {
            return FALSE; /* Unsupported */
        }

        /* If either opcode is a small character list, set pointers for comparing
        characters from that list with another list, or with a property. */

        if *base_list.add(0) == OP_CHAR {
            chr_ptr = base_list.add(2);
            list_ptr = list;
        } else if *list.add(0) == OP_CHAR {
            chr_ptr = list.add(2);
            list_ptr = base_list;
        }
        /* Character bitsets can also be compared to certain opcodes. */
        else if *base_list.add(0) == OP_CLASS
            || *list.add(0) == OP_CLASS
            /* In 8 bit, non-UTF mode, OP_CLASS and OP_NCLASS are the same. */
            || (utf == 0 && (*base_list.add(0) == OP_NCLASS || *list.add(0) == OP_NCLASS))
        {
            if *base_list.add(0) == OP_CLASS || (utf == 0 && *base_list.add(0) == OP_NCLASS) {
                set1 = base_end.wrapping_sub(*base_list.add(2) as usize);
                list_ptr = list;
            } else {
                set1 = code.wrapping_sub(*list.add(2) as usize);
                list_ptr = base_list;
            }

            invert_bits = FALSE;
            'isw: {
                match *list_ptr.add(0) {
                    OP_CLASS | OP_NCLASS => {
                        set2 = (if list_ptr == list as *const u32 {
                            code
                        } else {
                            base_end
                        })
                        .wrapping_sub(*list_ptr.add(2) as usize);
                        break 'isw;
                    }

                    OP_XCLASS => {
                        xclass_flags = (if list_ptr == list as *const u32 {
                            code
                        } else {
                            base_end
                        })
                        .wrapping_sub(*list_ptr.add(2) as usize)
                        .add(LINK_SIZE);
                        if ((*xclass_flags as u32) & XCL_HASPROP) != 0 {
                            return FALSE;
                        }
                        if ((*xclass_flags as u32) & XCL_MAP) == 0 {
                            /* No bits are set for characters < 256. */
                            if *list.add(1) == 0 {
                                return (((*xclass_flags as u32) & XCL_NOT) == 0) as BOOL;
                            }
                            /* Might be an empty repeat. */
                            continue 'outer;
                        }
                        set2 = xclass_flags.add(1);
                        break 'isw;
                    }

                    OP_NOT_DIGIT | OP_DIGIT => {
                        if *list_ptr.add(0) == OP_NOT_DIGIT {
                            invert_bits = TRUE;
                            /* Fall through */
                        }
                        set2 = (*cb).cbits.add(cbit_digit);
                        break 'isw;
                    }

                    OP_NOT_WHITESPACE | OP_WHITESPACE => {
                        if *list_ptr.add(0) == OP_NOT_WHITESPACE {
                            invert_bits = TRUE;
                            /* Fall through */
                        }
                        set2 = (*cb).cbits.add(cbit_space);
                        break 'isw;
                    }

                    OP_NOT_WORDCHAR | OP_WORDCHAR => {
                        if *list_ptr.add(0) == OP_NOT_WORDCHAR {
                            invert_bits = TRUE;
                            /* Fall through */
                        }
                        set2 = (*cb).cbits.add(cbit_word);
                        break 'isw;
                    }

                    _ => {
                        return FALSE;
                    }
                }
            }

            /* Because the bit sets are unaligned bytes, we need to perform byte
            comparison here. */

            set_end = set1.add(32);
            if invert_bits != 0 {
                loop {
                    let a = *set1;
                    set1 = set1.add(1);
                    let b = *set2;
                    set2 = set2.add(1);
                    if (a & !b) != 0 {
                        return FALSE;
                    }
                    if !(set1 < set_end) {
                        break;
                    }
                }
            } else {
                loop {
                    let a = *set1;
                    set1 = set1.add(1);
                    let b = *set2;
                    set2 = set2.add(1);
                    if (a & b) != 0 {
                        return FALSE;
                    }
                    if !(set1 < set_end) {
                        break;
                    }
                }
            }

            if *list.add(1) == 0 {
                return TRUE;
            }
            /* Might be an empty repeat. */
            continue 'outer;
        }
        /* Some property combinations also acceptable. Unicode property opcodes are
        processed specially; the rest can be handled with a lookup table. */
        else {
            let leftop: u32;
            let rightop: u32;

            leftop = *base_list.add(0);
            rightop = *list.add(0);

            accepted = FALSE; /* Always set in non-unicode case. */
            if leftop == OP_PROP || leftop == OP_NOTPROP {
                if rightop == OP_EOD {
                    accepted = TRUE;
                } else if rightop == OP_PROP || rightop == OP_NOTPROP {
                    let n: i32;
                    let p: *const u8;
                    let same: BOOL = (leftop == rightop) as BOOL;
                    let lisprop: BOOL = (leftop == OP_PROP) as BOOL;
                    let risprop: BOOL = (rightop == OP_PROP) as BOOL;
                    let bothprop: BOOL = (lisprop != 0 && risprop != 0) as BOOL;

                    /* There's a table that specifies how each combination is to be
                    processed:
                      0   Always return FALSE (never auto-possessify)
                      1   Character groups are distinct (possessify if both are OP_PROP)
                      2   Check character categories in the same group (general or
                          particular)
                      3   Return TRUE if the two opcodes are not the same
                      ... see comments below
                    */

                    n = propposstab_at(*base_list.add(2), *list.add(2)) as i32;
                    match n {
                        0 => {}
                        1 => {
                            accepted = bothprop;
                        }
                        2 => {
                            accepted = (((*base_list.add(3) == *list.add(3)) as BOOL) != same)
                                as BOOL;
                        }
                        3 => {
                            accepted = (same == 0) as BOOL;
                        }

                        4 => {
                            /* Left general category, right particular category */
                            accepted = (risprop != 0
                                && catposstab_at(*base_list.add(3), *list.add(3)) as i32 == same)
                                as BOOL;
                        }

                        5 => {
                            /* Right general category, left particular category */
                            accepted = (lisprop != 0
                                && catposstab_at(*list.add(3), *base_list.add(3)) as i32 == same)
                                as BOOL;
                        }

                        /* This code is logically tricky. Think hard before fiddling with
                        it. The posspropstab table has four entries per row. Each row
                        relates to one of PCRE's special properties such as ALNUM or SPACE
                        or WORD. Only WORD actually needs all four entries, but using
                        repeats for the others means they can all use the same code below.

                        The first two entries in each row are Unicode general categories,
                        and apply always, because all the characters they include are part
                        of the PCRE character set. The third and fourth entries are a
                        general and a particular category, respectively, that include one
                        or more relevant characters. One or the other is used, depending on
                        whether the check is for a general or a particular category.
                        However, in both cases the category contains more characters than
                        the specials that are defined for the property being tested
                        against. Therefore, it cannot be used in a NOTPROP case.

                        Example: the row for WORD contains ucp_L, ucp_N, ucp_P, ucp_Po.
                        Underscore is covered by ucp_P or ucp_Po. */

                        /* 6: Left alphanum vs right general category
                           7: Left space vs right general category
                           8: Left word vs right general category */
                        6 | 7 | 8 => {
                            p = posspropstab_row(n - 6);
                            accepted = (risprop != 0
                                && lisprop
                                    == ((*list.add(3) != *p.add(0) as u32
                                        && *list.add(3) != *p.add(1) as u32
                                        && (*list.add(3) != *p.add(2) as u32 || lisprop == 0))
                                        as BOOL)) as BOOL;
                        }

                        /*  9: Right alphanum vs left general category
                           10: Right space vs left general category
                           11: Right word vs left general category */
                        9 | 10 | 11 => {
                            p = posspropstab_row(n - 9);
                            accepted = (lisprop != 0
                                && risprop
                                    == ((*base_list.add(3) != *p.add(0) as u32
                                        && *base_list.add(3) != *p.add(1) as u32
                                        && (*base_list.add(3) != *p.add(2) as u32
                                            || risprop == 0)) as BOOL)) as BOOL;
                        }

                        /* 12: Left alphanum vs right particular category
                           13: Left space vs right particular category
                           14: Left word vs right particular category */
                        12 | 13 | 14 => {
                            p = posspropstab_row(n - 12);
                            accepted = (risprop != 0
                                && lisprop
                                    == ((catposstab_at(*p.add(0) as u32, *list.add(3)) != 0
                                        && catposstab_at(*p.add(1) as u32, *list.add(3)) != 0
                                        && (*list.add(3) != *p.add(3) as u32 || lisprop == 0))
                                        as BOOL)) as BOOL;
                        }

                        /* 15: Right alphanum vs left particular category
                           16: Right space vs left particular category
                           17: Right word vs left particular category */
                        15 | 16 | 17 => {
                            p = posspropstab_row(n - 15);
                            accepted = (lisprop != 0
                                && risprop
                                    == ((catposstab_at(*p.add(0) as u32, *base_list.add(3)) != 0
                                        && catposstab_at(*p.add(1) as u32, *base_list.add(3))
                                            != 0
                                        && (*base_list.add(3) != *p.add(3) as u32
                                            || risprop == 0)) as BOOL)) as BOOL;
                        }

                        _ => {}
                    }
                }
            } else {
                accepted = (leftop >= FIRST_AUTOTAB_OP
                    && leftop <= LAST_AUTOTAB_LEFT_OP
                    && rightop >= FIRST_AUTOTAB_OP
                    && rightop <= LAST_AUTOTAB_RIGHT_OP
                    && autoposstab_at(leftop - FIRST_AUTOTAB_OP, rightop - FIRST_AUTOTAB_OP)
                        != 0) as BOOL;
            }

            if accepted == 0 {
                return FALSE;
            }

            if *list.add(1) == 0 {
                return TRUE;
            }
            /* Might be an empty repeat. */
            continue 'outer;
        }

        /* Control reaches here only if one of the items is a small character list.
        All characters are checked against the other side. */

        loop {
            chr = *chr_ptr;

            'csw: {
                match *list_ptr.add(0) {
                    OP_CHAR => {
                        ochr_ptr = list_ptr.add(2);
                        loop {
                            if chr == *ochr_ptr {
                                return FALSE;
                            }
                            ochr_ptr = ochr_ptr.add(1);
                            if !(*ochr_ptr != NOTACHAR) {
                                break;
                            }
                        }
                        break 'csw;
                    }

                    OP_NOT => {
                        ochr_ptr = list_ptr.add(2);
                        loop {
                            if chr == *ochr_ptr {
                                break;
                            }
                            ochr_ptr = ochr_ptr.add(1);
                            if !(*ochr_ptr != NOTACHAR) {
                                break;
                            }
                        }
                        if *ochr_ptr == NOTACHAR {
                            return FALSE; /* Not found */
                        }
                        break 'csw;
                    }

                    /* Note that OP_DIGIT etc. are generated only when PCRE2_UCP is *not*
                    set. When it is set, \d etc. are converted into OP_(NOT_)PROP codes. */
                    OP_DIGIT => {
                        if chr < 256 && (*(*cb).ctypes.add(chr as usize) & ctype_digit) != 0 {
                            return FALSE;
                        }
                        break 'csw;
                    }

                    OP_NOT_DIGIT => {
                        if chr > 255 || (*(*cb).ctypes.add(chr as usize) & ctype_digit) == 0 {
                            return FALSE;
                        }
                        break 'csw;
                    }

                    OP_WHITESPACE => {
                        if chr < 256 && (*(*cb).ctypes.add(chr as usize) & ctype_space) != 0 {
                            return FALSE;
                        }
                        break 'csw;
                    }

                    OP_NOT_WHITESPACE => {
                        if chr > 255 || (*(*cb).ctypes.add(chr as usize) & ctype_space) == 0 {
                            return FALSE;
                        }
                        break 'csw;
                    }

                    OP_WORDCHAR => {
                        if chr < 255 && (*(*cb).ctypes.add(chr as usize) & ctype_word) != 0 {
                            return FALSE;
                        }
                        break 'csw;
                    }

                    OP_NOT_WORDCHAR => {
                        if chr > 255 || (*(*cb).ctypes.add(chr as usize) & ctype_word) == 0 {
                            return FALSE;
                        }
                        break 'csw;
                    }

                    OP_HSPACE => {
                        match chr {
                            /* HSPACE_CASES */
                            0x09 | 0x20 | 0xa0 | 0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002
                            | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009
                            | 0x200a | 0x202f | 0x205f | 0x3000 => return FALSE,
                            _ => {}
                        }
                        break 'csw;
                    }

                    OP_NOT_HSPACE => {
                        match chr {
                            /* HSPACE_CASES */
                            0x09 | 0x20 | 0xa0 | 0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002
                            | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009
                            | 0x200a | 0x202f | 0x205f | 0x3000 => {}
                            _ => return FALSE,
                        }
                        break 'csw;
                    }

                    OP_ANYNL | OP_VSPACE => {
                        match chr {
                            /* VSPACE_CASES */
                            0x0a | 0x0b | 0x0c | 0x0d | 0x85 | 0x2028 | 0x2029 => return FALSE,
                            _ => {}
                        }
                        break 'csw;
                    }

                    OP_NOT_VSPACE => {
                        match chr {
                            /* VSPACE_CASES */
                            0x0a | 0x0b | 0x0c | 0x0d | 0x85 | 0x2028 | 0x2029 => {}
                            _ => return FALSE,
                        }
                        break 'csw;
                    }

                    OP_DOLL | OP_EODN => {
                        match chr {
                            0x0d /* CHAR_CR */
                            | 0x0a /* CHAR_LF */
                            | 0x0b /* CHAR_VT */
                            | 0x0c /* CHAR_FF */
                            | 0x85 /* CHAR_NEL */
                            | 0x2028
                            | 0x2029 => {
                                return FALSE;
                            }
                            _ => {}
                        }
                        break 'csw;
                    }

                    OP_EOD => {
                        /* Can always possessify before \z */
                        break 'csw;
                    }

                    OP_PROP | OP_NOTPROP => {
                        if check_char_prop(
                            chr,
                            *list_ptr.add(2),
                            *list_ptr.add(3),
                            (*list_ptr.add(0) == OP_NOTPROP) as BOOL,
                        ) == 0
                        {
                            return FALSE;
                        }
                        break 'csw;
                    }

                    OP_NCLASS => {
                        if chr > 255 {
                            return FALSE;
                        }
                        /* Fall through */
                        if chr > 255 {
                            break 'csw;
                        }
                        class_bitset = (if list_ptr == list as *const u32 {
                            code
                        } else {
                            base_end
                        })
                        .wrapping_sub(*list_ptr.add(2) as usize);
                        if (*class_bitset.add((chr >> 3) as usize) & (1u8 << (chr & 7))) != 0 {
                            return FALSE;
                        }
                        break 'csw;
                    }

                    OP_CLASS => {
                        if chr > 255 {
                            break 'csw;
                        }
                        class_bitset = (if list_ptr == list as *const u32 {
                            code
                        } else {
                            base_end
                        })
                        .wrapping_sub(*list_ptr.add(2) as usize);
                        if (*class_bitset.add((chr >> 3) as usize) & (1u8 << (chr & 7))) != 0 {
                            return FALSE;
                        }
                        break 'csw;
                    }

                    OP_XCLASS => {
                        if _pcre2_xclass_8(
                            chr,
                            (if list_ptr == list as *const u32 {
                                code
                            } else {
                                base_end
                            })
                            .wrapping_sub(*list_ptr.add(2) as usize)
                            .add(LINK_SIZE),
                            (*cb).start_code as *const u8,
                            utf,
                        ) != 0
                        {
                            return FALSE;
                        }
                        break 'csw;
                    }

                    OP_ECLASS => {
                        if _pcre2_eclass_8(
                            chr,
                            (if list_ptr == list as *const u32 {
                                code
                            } else {
                                base_end
                            })
                            .wrapping_sub(*list_ptr.add(2) as usize)
                            .add(LINK_SIZE),
                            (if list_ptr == list as *const u32 {
                                code
                            } else {
                                base_end
                            })
                            .wrapping_sub(*list_ptr.add(3) as usize),
                            (*cb).start_code as *const u8,
                            utf,
                        ) != 0
                        {
                            return FALSE;
                        }
                        break 'csw;
                    }

                    _ => {
                        return FALSE;
                    }
                }
            }

            chr_ptr = chr_ptr.add(1);
            if !(*chr_ptr != NOTACHAR) {
                break;
            }
        }

        /* At least one character must be matched from this opcode. */

        if *list.add(1) == 0 {
            return TRUE;
        }
    }

    /* PCRE2_DEBUG_UNREACHABLE(); Control should never reach here */
}

/*************************************************
*    Scan compiled regex for auto-possession     *
*************************************************/

/* Replaces single character iterations with their possessive alternatives
if appropriate. This function modifies the compiled opcode! Hitting a
non-existent opcode may indicate a bug in PCRE2, but it can also be caused if a
bad UTF string was compiled with PCRE2_NO_UTF_CHECK. The rec_limit catches
overly complicated or large patterns. In these cases, the check just stops,
leaving the remainder of the pattern unpossessified.

Arguments:
  code        points to start of the byte code
  cb          compile data block

Returns:      0 for success
              -1 if a non-existant opcode is encountered
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_auto_possessify_8(
    mut code: *mut PCRE2_UCHAR,
    cb: *const compile_block,
) -> i32 {
    let mut c: PCRE2_UCHAR;
    let mut end: PCRE2_SPTR;
    let mut repeat_opcode: *mut PCRE2_UCHAR;
    let mut list_arr: [u32; MAX_LIST] = [0; MAX_LIST];
    let list: *mut u32 = list_arr.as_mut_ptr();
    let mut rec_limit: i32 = 1000; /* Was 10,000 but clang+ASAN uses a lot of stack. */
    let utf: BOOL = (((*cb).external_options & PCRE2_UTF) != 0) as BOOL;
    let ucp: BOOL = (((*cb).external_options & PCRE2_UCP) != 0) as BOOL;

    loop {
        c = *code;

        if (c as u32) >= OP_TABLE_LENGTH {
            /* PCRE2_DEBUG_UNREACHABLE(); */
            return -1; /* Something gone wrong */
        }

        if (c as u32) >= OP_STAR && (c as u32) <= OP_TYPEPOSUPTO {
            c = c.wrapping_sub(get_repeat_base(c).wrapping_sub(OP_STAR as PCRE2_UCHAR));
            end = if (c as u32) <= OP_MINUPTO {
                get_chr_property_list(code as PCRE2_SPTR, utf, ucp, (*cb).fcc, list)
            } else {
                core::ptr::null()
            };
            *list.add(1) = (((c as u32) == OP_STAR
                || (c as u32) == OP_PLUS
                || (c as u32) == OP_QUERY
                || (c as u32) == OP_UPTO) as BOOL) as u32;

            if !end.is_null()
                && compare_opcodes(end, utf, ucp, cb, list, end, &mut rec_limit) != 0
            {
                match c as u32 {
                    OP_STAR => {
                        *code = (*code).wrapping_add((OP_POSSTAR - OP_STAR) as PCRE2_UCHAR);
                    }

                    OP_MINSTAR => {
                        *code = (*code).wrapping_add((OP_POSSTAR - OP_MINSTAR) as PCRE2_UCHAR);
                    }

                    OP_PLUS => {
                        *code = (*code).wrapping_add((OP_POSPLUS - OP_PLUS) as PCRE2_UCHAR);
                    }

                    OP_MINPLUS => {
                        *code = (*code).wrapping_add((OP_POSPLUS - OP_MINPLUS) as PCRE2_UCHAR);
                    }

                    OP_QUERY => {
                        *code = (*code).wrapping_add((OP_POSQUERY - OP_QUERY) as PCRE2_UCHAR);
                    }

                    OP_MINQUERY => {
                        *code = (*code).wrapping_add((OP_POSQUERY - OP_MINQUERY) as PCRE2_UCHAR);
                    }

                    OP_UPTO => {
                        *code = (*code).wrapping_add((OP_POSUPTO - OP_UPTO) as PCRE2_UCHAR);
                    }

                    OP_MINUPTO => {
                        *code = (*code).wrapping_add((OP_POSUPTO - OP_MINUPTO) as PCRE2_UCHAR);
                    }

                    _ => {}
                }
            }
            c = *code;
        } else if (c as u32) == OP_CLASS
            || (c as u32) == OP_NCLASS
            || (c as u32) == OP_XCLASS
            || (c as u32) == OP_ECLASS
        {
            if (c as u32) == OP_XCLASS || (c as u32) == OP_ECLASS {
                repeat_opcode = code.add(GET!(code, 1) as usize);
            } else {
                repeat_opcode = code.add(1 + 32 /* 32 / sizeof(PCRE2_UCHAR) */);
            }

            c = *repeat_opcode;
            if (c as u32) >= OP_CRSTAR && (c as u32) <= OP_CRMINRANGE {
                /* The return from get_chr_property_list() will never be NULL when
                *code (aka c) is one of the four class opcodes. However, gcc with
                -fanalyzer notes that a NULL return is possible, and grumbles. Hence we
                put in a check. */

                end = get_chr_property_list(code as PCRE2_SPTR, utf, ucp, (*cb).fcc, list);
                *list.add(1) = ((((c as u32) & 1) == 0) as BOOL) as u32;

                if !end.is_null()
                    && compare_opcodes(end, utf, ucp, cb, list, end, &mut rec_limit) != 0
                {
                    match c as u32 {
                        OP_CRSTAR | OP_CRMINSTAR => {
                            *repeat_opcode = OP_CRPOSSTAR as PCRE2_UCHAR;
                        }

                        OP_CRPLUS | OP_CRMINPLUS => {
                            *repeat_opcode = OP_CRPOSPLUS as PCRE2_UCHAR;
                        }

                        OP_CRQUERY | OP_CRMINQUERY => {
                            *repeat_opcode = OP_CRPOSQUERY as PCRE2_UCHAR;
                        }

                        OP_CRRANGE | OP_CRMINRANGE => {
                            *repeat_opcode = OP_CRPOSRANGE as PCRE2_UCHAR;
                        }

                        _ => {}
                    }
                }
            }
            c = *code;
        }

        match c as u32 {
            OP_END => {
                return 0;
            }

            OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEQUERY
            | OP_TYPEMINQUERY | OP_TYPEPOSSTAR | OP_TYPEPOSPLUS | OP_TYPEPOSQUERY => {
                if (*code.add(1) as u32) == OP_PROP || (*code.add(1) as u32) == OP_NOTPROP {
                    code = code.add(2);
                }
            }

            OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEEXACT | OP_TYPEPOSUPTO => {
                if (*code.add(1 + IMM2_SIZE) as u32) == OP_PROP
                    || (*code.add(1 + IMM2_SIZE) as u32) == OP_NOTPROP
                {
                    code = code.add(2);
                }
            }

            OP_CALLOUT_STR => {
                code = code.add(GET!(code, 1 + 2 * LINK_SIZE) as usize);
            }

            OP_XCLASS | OP_ECLASS => {
                code = code.add(GET!(code, 1) as usize);
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
        we have to arrange to skip the extra code units. */

        if utf != 0 {
            match c as u32 {
                OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI | OP_STAR | OP_MINSTAR | OP_PLUS
                | OP_MINPLUS | OP_QUERY | OP_MINQUERY | OP_UPTO | OP_MINUPTO | OP_EXACT
                | OP_POSSTAR | OP_POSPLUS | OP_POSQUERY | OP_POSUPTO | OP_STARI | OP_MINSTARI
                | OP_PLUSI | OP_MINPLUSI | OP_QUERYI | OP_MINQUERYI | OP_UPTOI | OP_MINUPTOI
                | OP_EXACTI | OP_POSSTARI | OP_POSPLUSI | OP_POSQUERYI | OP_POSUPTOI
                | OP_NOTSTAR | OP_NOTMINSTAR | OP_NOTPLUS | OP_NOTMINPLUS | OP_NOTQUERY
                | OP_NOTMINQUERY | OP_NOTUPTO | OP_NOTMINUPTO | OP_NOTEXACT | OP_NOTPOSSTAR
                | OP_NOTPOSPLUS | OP_NOTPOSQUERY | OP_NOTPOSUPTO | OP_NOTSTARI
                | OP_NOTMINSTARI | OP_NOTPLUSI | OP_NOTMINPLUSI | OP_NOTQUERYI
                | OP_NOTMINQUERYI | OP_NOTUPTOI | OP_NOTMINUPTOI | OP_NOTEXACTI
                | OP_NOTPOSSTARI | OP_NOTPOSPLUSI | OP_NOTPOSQUERYI | OP_NOTPOSUPTOI => {
                    if HAS_EXTRALEN!(*code.offset(-1)) {
                        code = code.add(GET_EXTRALEN!(*code.offset(-1)) as usize);
                    }
                }

                _ => {}
            }
        }
    }
}

/* End of pcre2_auto_possess.c */
