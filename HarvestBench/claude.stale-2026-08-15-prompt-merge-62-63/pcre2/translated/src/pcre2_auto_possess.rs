/*************************************************
*      Perl-Compatible Regular Expressions       *
*************************************************/

/* PCRE is a library of functions to support regular expressions whose syntax
and semantics are as close as possible to those of the Perl 5 language.

                       Written by Philip Hazel
     Original API code Copyright (c) 1997-2012 University of Cambridge
          New API code Copyright (c) 2016-2024 University of Cambridge

This module contains functions that scan a compiled pattern and change
repeats into possessive repeats where possible. This is a translation of
pcre2_auto_possess.c to Rust (PCRE2 10.48, 8-bit, SUPPORT_UNICODE, no JIT,
LINK_SIZE=2, IMM2_SIZE=2). */

#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(unused_parens)]

use core::ffi::c_int;

use crate::pcre2_internal::*;

/* This macro represents the max size of list[] and that is used to keep
track of UCD info in several places, it should be kept on sync with the
value used by GenerateUcd.py */
const MAX_LIST: usize = 8;

const NOTACHAR: u32 = 0xffffffff;

const CHAR_UNDERSCORE: u32 = 0x5f;

/* First/last opcodes used to index the auto-possessification table. These are
#defines in pcre2_internal.h; they are recreated here from the opcode values. */
const FIRST_AUTOTAB_OP: u32 = OP_NOT_DIGIT as u32;
const LAST_AUTOTAB_LEFT_OP: u32 = OP_EXTUNI as u32;
const LAST_AUTOTAB_RIGHT_OP: u32 = OP_DOLLM as u32;

/*************************************************
*        Tables for auto-possessification        *
*************************************************/

/* This table is used to check whether auto-possessification is possible
between adjacent character-type opcodes. The left-hand (repeated) opcode is
used to select the row, and the right-hand opcode is use to select the column. */

/*                     \D \d \S \s \W \w  . .+ \C \P \p \R \H \h \V \v \X \Z \z  $ $M */
static autoposstab: [[u8; 21]; 17] = [
    [
        0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
    ], /* \D */
    [
        1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1, 1,
    ], /* \d */
    [
        0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1, 1,
    ], /* \S */
    [
        0, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
    ], /* \s */
    [
        0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
    ], /* \W */
    [
        0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1, 1,
    ], /* \w */
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0,
    ], /* .  */
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
    ], /* .+ */
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
    ], /* \C */
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ], /* \P */
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ], /* \p */
    [
        0, 1, 0, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0,
    ], /* \R */
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0,
    ], /* \H */
    [
        0, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 0, 0, 1, 0, 0,
    ], /* \h */
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0,
    ], /* \V */
    [
        0, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 1, 0, 0,
    ], /* \v */
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
    ], /* \X */
];

/* This table is used to check whether auto-possessification is possible
between adjacent Unicode property opcodes (OP_PROP and OP_NOTPROP). */

/*                       LAMP GC  PC  SC  SCX ALNUM SPACE PXSPACE WORD CLIST UCNC BIDICL BOOL */
static propposstab: [[u8; PT_TABSIZE as usize]; PT_TABSIZE as usize] = [
    [3, 0, 0, 0, 0, 3, 1, 1, 0, 0, 0, 0, 0],     /* PT_LAMP */
    [0, 2, 4, 0, 0, 9, 10, 10, 11, 0, 0, 0, 0],  /* PT_GC */
    [0, 5, 2, 0, 0, 15, 16, 16, 17, 0, 0, 0, 0], /* PT_PC */
    [0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0],     /* PT_SC */
    [0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0],     /* PT_SCX */
    [3, 6, 12, 0, 0, 3, 1, 1, 0, 0, 0, 0, 0],    /* PT_ALNUM */
    [1, 7, 13, 0, 0, 1, 3, 3, 1, 0, 0, 0, 0],    /* PT_SPACE */
    [1, 7, 13, 0, 0, 1, 3, 3, 1, 0, 0, 0, 0],    /* PT_PXSPACE */
    [0, 8, 14, 0, 0, 0, 1, 1, 3, 0, 0, 0, 0],    /* PT_WORD */
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],     /* PT_CLIST */
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0],     /* PT_UCNC */
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],     /* PT_BIDICL */
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],     /* PT_BOOL */
                                                 /* PT_ANY does not need a record. */
];

/* This table is used to check whether auto-possessification is possible
between adjacent Unicode property opcodes when one specifies a general category
and the other specifies a particular category. */

/*                  Cc Cf Cn Co Cs Ll Lm Lo Lt Lu Mc Me Mn Nd Nl No Pc Pd Pe Pf Pi Po Ps Sc Sk Sm So Zl Zp Zs */
static catposstab: [[u8; 30]; 7] = [
    [
        0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    ], /* C */
    [
        1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    ], /* L */
    [
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    ], /* M */
    [
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    ], /* N */
    [
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1,
    ], /* P */
    [
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1,
    ], /* S */
    [
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0,
    ], /* Z */
];

/* This table is used when checking ALNUM, (PX)SPACE, SPACE, and WORD against
a general or particular category. */

static posspropstab: [[u8; 4]; 3] = [
    [ucp_L as u8, ucp_N as u8, ucp_N as u8, ucp_Nl as u8], /* ALNUM, 3rd and 4th values redundant */
    [ucp_Z as u8, ucp_Z as u8, ucp_C as u8, ucp_Cc as u8], /* SPACE and PXSPACE, 2nd value redundant */
    [ucp_L as u8, ucp_N as u8, ucp_P as u8, ucp_Po as u8], /* WORD */
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

unsafe fn check_char_prop(c: u32, ptype: u32, pdata: u32, negated: BOOL) -> BOOL {
    let negated = negated != 0;
    let prop = GET_UCD(c);

    match ptype {
        PT_LAMP => {
            ((prop.chartype as u32 == ucp_Lu
                || prop.chartype as u32 == ucp_Ll
                || prop.chartype as u32 == ucp_Lt)
                == negated) as BOOL
        }

        PT_GC => ((pdata == _pcre2_ucp_gentype_8[prop.chartype as usize]) == negated) as BOOL,

        PT_PC => ((pdata == prop.chartype as u32) == negated) as BOOL,

        PT_SC => ((pdata == prop.script as u32) == negated) as BOOL,

        PT_SCX => {
            let ok = pdata == prop.script as u32
                || MAPBIT(
                    core::slice::from_raw_parts(
                        _pcre2_ucd_script_sets_8
                            .as_ptr()
                            .add(UCD_SCRIPTX_PROP(prop) as usize),
                        (_pcre2_ucd_script_sets_8.len())
                            .saturating_sub(UCD_SCRIPTX_PROP(prop) as usize),
                    ),
                    pdata,
                ) != 0;
            (ok == negated) as BOOL
        }

        /* These are specials */
        PT_ALNUM => {
            ((_pcre2_ucp_gentype_8[prop.chartype as usize] == ucp_L
                || _pcre2_ucp_gentype_8[prop.chartype as usize] == ucp_N)
                == negated) as BOOL
        }

        /* Perl space used to exclude VT, but from Perl 5.18 it is included, which
        means that Perl space and POSIX space are now identical. */
        PT_SPACE | PT_PXSPACE => {
            let rc;
            match c {
                /* HSPACE_CASES */
                CHAR_HT | CHAR_SPACE | CHAR_NBSP | 0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002
                | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009 | 0x200a
                | 0x202f | 0x205f | 0x3000
                /* VSPACE_CASES */
                | CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028 | 0x2029 => {
                    rc = negated;
                }
                _ => {
                    rc = (_pcre2_ucp_gentype_8[prop.chartype as usize] == ucp_Z) == negated;
                }
            }
            rc as BOOL
        }

        PT_WORD => {
            ((_pcre2_ucp_gentype_8[prop.chartype as usize] == ucp_L
                || _pcre2_ucp_gentype_8[prop.chartype as usize] == ucp_N
                || c == CHAR_UNDERSCORE)
                == negated) as BOOL
        }

        PT_CLIST => {
            let mut p = _pcre2_ucd_caseless_sets_8
                .as_ptr()
                .add(prop.caseset as usize);
            loop {
                if c < *p {
                    return (!negated) as BOOL;
                }
                let v = *p;
                p = p.add(1);
                if c == v {
                    return negated as BOOL;
                }
            }
        }

        /* Haven't yet thought these through. */
        PT_BIDICL => FALSE,

        PT_BOOL => FALSE,

        _ => FALSE,
    }
}

/*************************************************
*        Base opcode of repeated opcodes         *
*************************************************/

/* Returns the base opcode for repeated single character type opcodes. If the
opcode is not a repeated character type, it returns with the original value. */

fn get_repeat_base(c: PCRE2_UCHAR) -> PCRE2_UCHAR {
    if c > OP_TYPEPOSUPTO {
        c
    } else if c >= OP_TYPESTAR {
        OP_TYPESTAR
    } else if c >= OP_NOTSTARI {
        OP_NOTSTARI
    } else if c >= OP_NOTSTAR {
        OP_NOTSTAR
    } else if c >= OP_STARI {
        OP_STARI
    } else {
        OP_STAR
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
    let end: PCRE2_SPTR;
    let class_end: PCRE2_SPTR;
    let mut chr: u32;

    let clist_dest: *mut u32;
    let mut clist_src: *const u32;

    *list.add(0) = c as u32;
    *list.add(1) = FALSE as u32;
    code = code.add(1);

    if c >= OP_STAR && c <= OP_TYPEPOSUPTO {
        base = get_repeat_base(c);
        c -= base - OP_STAR;

        if c == OP_UPTO || c == OP_MINUPTO || c == OP_EXACT || c == OP_POSUPTO {
            code = code.add(IMM2_SIZE);
        }

        *list.add(1) = (c != OP_PLUS && c != OP_MINPLUS && c != OP_EXACT && c != OP_POSPLUS) as u32;

        match base {
            _ if base == OP_STAR => {
                *list.add(0) = OP_CHAR as u32;
            }
            _ if base == OP_STARI => {
                *list.add(0) = OP_CHARI as u32;
            }
            _ if base == OP_NOTSTAR => {
                *list.add(0) = OP_NOT as u32;
            }
            _ if base == OP_NOTSTARI => {
                *list.add(0) = OP_NOTI as u32;
            }
            _ if base == OP_TYPESTAR => {
                *list.add(0) = *code as u32;
                code = code.add(1);
            }
            _ => {}
        }
        c = *list.add(0) as PCRE2_UCHAR;
    }

    if c == OP_NOT_DIGIT
        || c == OP_DIGIT
        || c == OP_NOT_WHITESPACE
        || c == OP_WHITESPACE
        || c == OP_NOT_WORDCHAR
        || c == OP_WORDCHAR
        || c == OP_ANY
        || c == OP_ALLANY
        || c == OP_ANYNL
        || c == OP_NOT_HSPACE
        || c == OP_HSPACE
        || c == OP_NOT_VSPACE
        || c == OP_VSPACE
        || c == OP_EXTUNI
        || c == OP_EODN
        || c == OP_EOD
        || c == OP_DOLL
        || c == OP_DOLLM
    {
        return code;
    }

    if c == OP_CHAR || c == OP_NOT {
        /* GETCHARINCTEST(chr, code) */
        if utf == 0 {
            chr = *code as u32;
            code = code.add(1);
        } else {
            let (ch, len) = GETCHARINC(code);
            chr = ch;
            code = code.add(len);
        }
        *list.add(2) = chr;
        *list.add(3) = NOTACHAR;
        return code;
    }

    if c == OP_CHARI || c == OP_NOTI {
        *list.add(0) = if c == OP_CHARI {
            OP_CHAR as u32
        } else {
            OP_NOT as u32
        };
        /* GETCHARINCTEST(chr, code) */
        if utf == 0 {
            chr = *code as u32;
            code = code.add(1);
        } else {
            let (ch, len) = GETCHARINC(code);
            chr = ch;
            code = code.add(len);
        }
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

    if c == OP_PROP || c == OP_NOTPROP {
        if *code.add(0) != PT_CLIST as u8 {
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

        let mut dest = clist_dest;
        loop {
            if dest as usize >= list.add(MAX_LIST) as usize {
                /* Early return if there is not enough space. */
                *list.add(2) = *code.add(0) as u32;
                *list.add(3) = *code.add(1) as u32;
                return code;
            }
            *dest = *clist_src;
            dest = dest.add(1);
            let v = *clist_src;
            clist_src = clist_src.add(1);
            if v == NOTACHAR {
                break;
            }
        }

        /* All characters are stored. The terminating NOTACHAR is copied from the
        clist itself. */
        *list.add(0) = if c == OP_PROP {
            OP_CHAR as u32
        } else {
            OP_NOT as u32
        };
        return code;
    }

    if c == OP_NCLASS || c == OP_CLASS || c == OP_XCLASS || c == OP_ECLASS {
        if c == OP_XCLASS || c == OP_ECLASS {
            end = code.add(GET(code, 0) as usize - 1);
        } else {
            end = code.add(32); /* 32 / sizeof(PCRE2_UCHAR), sizeof==1 */
        }
        class_end = end;

        let mut end = end;
        match *end {
            _ if *end == OP_CRSTAR
                || *end == OP_CRMINSTAR
                || *end == OP_CRQUERY
                || *end == OP_CRMINQUERY
                || *end == OP_CRPOSSTAR
                || *end == OP_CRPOSQUERY =>
            {
                *list.add(1) = TRUE as u32;
                end = end.add(1);
            }
            _ if *end == OP_CRPLUS || *end == OP_CRMINPLUS || *end == OP_CRPOSPLUS => {
                end = end.add(1);
            }
            _ if *end == OP_CRRANGE || *end == OP_CRMINRANGE || *end == OP_CRPOSRANGE => {
                *list.add(1) = (GET2(end, 1) == 0) as u32;
                end = end.add(1 + 2 * IMM2_SIZE);
            }
            _ => {}
        }
        *list.add(2) = (end as usize - code as usize) as u32;
        *list.add(3) = (end as usize - class_end as usize) as u32;
        return end;
    }

    core::ptr::null() /* Opcode not accepted */
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

unsafe fn compare_opcodes(
    mut code: PCRE2_SPTR,
    utf: BOOL,
    ucp: BOOL,
    cb: *const compile_block,
    base_list: *const u32,
    base_end: PCRE2_SPTR,
    rec_limit: *mut c_int,
) -> BOOL {
    let mut c: PCRE2_UCHAR;
    let mut list = [0u32; MAX_LIST];
    let mut chr_ptr: *const u32 = core::ptr::null();
    let mut ochr_ptr: *const u32;
    let mut list_ptr: *const u32 = core::ptr::null();
    let mut next_code: PCRE2_SPTR;
    let mut xclass_flags: PCRE2_SPTR;
    let mut class_bitset: *const u8;
    let mut set1: *const u8;
    let mut set2: *const u8;
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
    (represented by a non-zero value) quantifier. */

    loop {
        let mut bracode: PCRE2_SPTR;

        /* All operations move the code pointer forward.
        Therefore infinite recursions are not possible. */
        c = *code;

        /* Skip over callouts */
        if c == OP_CALLOUT {
            code = code.add(_pcre2_OP_lengths_8[c as usize] as usize);
            continue;
        }

        if c == OP_CALLOUT_STR {
            code = code.add(GET(code, 1 + 2 * LINK_SIZE) as usize);
            continue;
        }

        /* At the end of a branch, skip to the end of the group and process it. */
        if c == OP_ALT {
            loop {
                code = code.add(GET(code, 1) as usize);
                if *code != OP_ALT {
                    break;
                }
            }
            c = *code;
        }

        /* Inspect the next opcode. */
        let mut fall_through = false;

        if c == OP_END {
            /* We can always possessify a greedy iterator at the end of the
            pattern. A non-greedy iterator must never be possessified. */
            return (*base_list.add(1) != 0) as BOOL;
        } else if c == OP_KET || c == OP_KETRPOS {
            /* The non-greedy case cannot be converted to a possessive form. */
            if *base_list.add(1) == 0 {
                return FALSE;
            }

            /* If the bracket is capturing it might be referenced by an OP_RECURSE
            so its last iterator can never be possessified if the pattern contains
            recursions. */
            bracode = code.sub(GET(code, 1) as usize);
            let bc = *bracode;
            if bc == OP_CBRA || bc == OP_SCBRA || bc == OP_CBRAPOS || bc == OP_SCBRAPOS {
                if (*cb).had_recurse != 0 {
                    return FALSE;
                }
            } else if bc == OP_SCRIPT_RUN {
                /* A script run might have to backtrack if the iterated item can match
                characters from more than one script. So give up unless repeating an
                explicit character. */
                if *base_list.add(0) != OP_CHAR as u32 && *base_list.add(0) != OP_CHARI as u32 {
                    return FALSE;
                }
            } else if bc == OP_ASSERT || bc == OP_ASSERT_NOT || bc == OP_ONCE {
                /* Atomic sub-patterns and forward assertions can always
                auto-possessify their last iterator. However, if the group was
                entered as a result of checking a previous iterator, this is not
                possible. */
                return (entered_a_group == 0) as BOOL;
            } else if bc == OP_ASSERTBACK || bc == OP_ASSERTBACK_NOT {
                /* Fixed-length lookbehinds can be treated the same way, but
                variable length lookbehinds must not auto-possessify their last
                iterator. */
                loop {
                    if *bracode.add(1 + LINK_SIZE) == OP_VREVERSE {
                        return FALSE; /* Variable */
                    }
                    bracode = bracode.add(GET(bracode, 1) as usize);
                    if *bracode != OP_ALT {
                        break;
                    }
                }
                return (entered_a_group == 0) as BOOL; /* Not variable length */
            } else if bc == OP_ASSERT_NA || bc == OP_ASSERTBACK_NA {
                /* Non-atomic assertions - don't possessify last iterator. */
                return FALSE;
            }

            /* Skip over the bracket and inspect what comes next. */
            code = code.add(_pcre2_OP_lengths_8[c as usize] as usize);
            continue;
        } else if c == OP_ONCE || c == OP_BRA || c == OP_CBRA {
            /* Handle cases where the next item is a group. */
            next_code = code.add(GET(code, 1) as usize);
            code = code.add(_pcre2_OP_lengths_8[c as usize] as usize);

            /* Check each branch. We have to recurse a level for all but the last
            branch. */
            while *next_code == OP_ALT {
                if compare_opcodes(code, utf, ucp, cb, base_list, base_end, rec_limit) == 0 {
                    return FALSE;
                }
                code = next_code.add(1 + LINK_SIZE);
                next_code = next_code.add(GET(next_code, 1) as usize);
            }

            entered_a_group = TRUE;
            continue;
        } else if c == OP_BRAZERO || c == OP_BRAMINZERO {
            next_code = code.add(1);
            if *next_code != OP_BRA && *next_code != OP_CBRA && *next_code != OP_ONCE {
                return FALSE;
            }

            loop {
                next_code = next_code.add(GET(next_code, 1) as usize);
                if *next_code != OP_ALT {
                    break;
                }
            }

            /* The bracket content will be checked by the OP_BRA/OP_CBRA case above. */
            next_code = next_code.add(1 + LINK_SIZE);
            if compare_opcodes(next_code, utf, ucp, cb, base_list, base_end, rec_limit) == 0 {
                return FALSE;
            }

            code = code.add(_pcre2_OP_lengths_8[c as usize] as usize);
            continue;
        } else {
            /* The next opcode does not need special handling; fall through and use
            it to see if the base can be possessified. */
            fall_through = true;
        }

        let _ = fall_through;

        /* We now have the next appropriate opcode to compare with the base. Check
        for a supported opcode, and load its properties. */
        code = get_chr_property_list(code, utf, ucp, (*cb).fcc, list.as_mut_ptr());
        if code.is_null() {
            return FALSE; /* Unsupported */
        }

        /* If either opcode is a small character list, set pointers for comparing
        characters from that list with another list, or with a property. */
        if *base_list.add(0) == OP_CHAR as u32 {
            chr_ptr = base_list.add(2);
            list_ptr = list.as_ptr();
        } else if list[0] == OP_CHAR as u32 {
            chr_ptr = list.as_ptr().add(2);
            list_ptr = base_list;
        }
        /* Character bitsets can also be compared to certain opcodes. In 8 bit,
        non-UTF mode, OP_CLASS and OP_NCLASS are the same. */
        else if *base_list.add(0) == OP_CLASS as u32
            || list[0] == OP_CLASS as u32
            || (utf == 0 && (*base_list.add(0) == OP_NCLASS as u32 || list[0] == OP_NCLASS as u32))
        {
            if *base_list.add(0) == OP_CLASS as u32
                || (utf == 0 && *base_list.add(0) == OP_NCLASS as u32)
            {
                set1 = base_end.sub(*base_list.add(2) as usize);
                list_ptr = list.as_ptr();
            } else {
                set1 = code.sub(list[2] as usize);
                list_ptr = base_list;
            }

            invert_bits = FALSE;
            let lp0 = *list_ptr.add(0);
            if lp0 == OP_CLASS as u32 || lp0 == OP_NCLASS as u32 {
                let this_end = if list_ptr == list.as_ptr() {
                    code
                } else {
                    base_end
                };
                set2 = this_end.sub(*list_ptr.add(2) as usize);
            } else if lp0 == OP_XCLASS as u32 {
                let this_end = if list_ptr == list.as_ptr() {
                    code
                } else {
                    base_end
                };
                xclass_flags = this_end.sub(*list_ptr.add(2) as usize).add(LINK_SIZE);
                if (*xclass_flags & XCL_HASPROP) != 0 {
                    return FALSE;
                }
                if (*xclass_flags & XCL_MAP) == 0 {
                    /* No bits are set for characters < 256. */
                    if list[1] == 0 {
                        return ((*xclass_flags & XCL_NOT) == 0) as BOOL;
                    }
                    /* Might be an empty repeat. */
                    continue;
                }
                set2 = xclass_flags.add(1);
            } else if lp0 == OP_NOT_DIGIT as u32 {
                invert_bits = TRUE;
                set2 = (*cb).cbits.add(cbit_digit);
            } else if lp0 == OP_DIGIT as u32 {
                set2 = (*cb).cbits.add(cbit_digit);
            } else if lp0 == OP_NOT_WHITESPACE as u32 {
                invert_bits = TRUE;
                set2 = (*cb).cbits.add(cbit_space);
            } else if lp0 == OP_WHITESPACE as u32 {
                set2 = (*cb).cbits.add(cbit_space);
            } else if lp0 == OP_NOT_WORDCHAR as u32 {
                invert_bits = TRUE;
                set2 = (*cb).cbits.add(cbit_word);
            } else if lp0 == OP_WORDCHAR as u32 {
                set2 = (*cb).cbits.add(cbit_word);
            } else {
                return FALSE;
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
                    if set1 >= set_end {
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
                    if set1 >= set_end {
                        break;
                    }
                }
            }

            if list[1] == 0 {
                return TRUE;
            }
            /* Might be an empty repeat. */
            continue;
        }
        /* Some property combinations also acceptable. Unicode property opcodes are
        processed specially; the rest can be handled with a lookup table. */
        else {
            let leftop: u32 = *base_list.add(0);
            let rightop: u32 = list[0];

            accepted = FALSE; /* Always set in non-unicode case. */
            if leftop == OP_PROP as u32 || leftop == OP_NOTPROP as u32 {
                if rightop == OP_EOD as u32 {
                    accepted = TRUE;
                } else if rightop == OP_PROP as u32 || rightop == OP_NOTPROP as u32 {
                    let n: i32;
                    let p: *const u8;
                    let same: bool = leftop == rightop;
                    let lisprop: bool = leftop == OP_PROP as u32;
                    let risprop: bool = rightop == OP_PROP as u32;
                    let bothprop: bool = lisprop && risprop;

                    n = propposstab[*base_list.add(2) as usize][list[2] as usize] as i32;
                    match n {
                        0 => {}
                        1 => accepted = bothprop as BOOL,
                        2 => accepted = ((*base_list.add(3) == list[3]) != same) as BOOL,
                        3 => accepted = (!same) as BOOL,

                        4 => {
                            /* Left general category, right particular category */
                            accepted = (risprop
                                && (catposstab[*base_list.add(3) as usize][list[3] as usize] != 0)
                                    == same) as BOOL;
                        }

                        5 => {
                            /* Right general category, left particular category */
                            accepted = (lisprop
                                && (catposstab[list[3] as usize][*base_list.add(3) as usize] != 0)
                                    == same) as BOOL;
                        }

                        6 | 7 | 8 => {
                            /* Left alphanum/space/word vs right general category */
                            p = posspropstab[(n - 6) as usize].as_ptr();
                            accepted = (risprop
                                && lisprop
                                    == (list[3] != *p.add(0) as u32
                                        && list[3] != *p.add(1) as u32
                                        && (list[3] != *p.add(2) as u32 || !lisprop)))
                                as BOOL;
                        }

                        9 | 10 | 11 => {
                            /* Right alphanum/space/word vs left general category */
                            p = posspropstab[(n - 9) as usize].as_ptr();
                            accepted = (lisprop
                                && risprop
                                    == (*base_list.add(3) != *p.add(0) as u32
                                        && *base_list.add(3) != *p.add(1) as u32
                                        && (*base_list.add(3) != *p.add(2) as u32 || !risprop)))
                                as BOOL;
                        }

                        12 | 13 | 14 => {
                            /* Left alphanum/space/word vs right particular category */
                            p = posspropstab[(n - 12) as usize].as_ptr();
                            accepted = (risprop
                                && lisprop
                                    == (catposstab[*p.add(0) as usize][list[3] as usize] != 0
                                        && catposstab[*p.add(1) as usize][list[3] as usize] != 0
                                        && (list[3] != *p.add(3) as u32 || !lisprop)))
                                as BOOL;
                        }

                        15 | 16 | 17 => {
                            /* Right alphanum/space/word vs left particular category */
                            p = posspropstab[(n - 15) as usize].as_ptr();
                            accepted = (lisprop
                                && risprop
                                    == (catposstab[*p.add(0) as usize][*base_list.add(3) as usize]
                                        != 0
                                        && catposstab[*p.add(1) as usize]
                                            [*base_list.add(3) as usize]
                                            != 0
                                        && (*base_list.add(3) != *p.add(3) as u32 || !risprop)))
                                as BOOL;
                        }

                        _ => {}
                    }
                }
            } else {
                accepted = (leftop >= FIRST_AUTOTAB_OP
                    && leftop <= LAST_AUTOTAB_LEFT_OP
                    && rightop >= FIRST_AUTOTAB_OP
                    && rightop <= LAST_AUTOTAB_RIGHT_OP
                    && autoposstab[(leftop - FIRST_AUTOTAB_OP) as usize]
                        [(rightop - FIRST_AUTOTAB_OP) as usize]
                        != 0) as BOOL;
            }

            if accepted == 0 {
                return FALSE;
            }

            if list[1] == 0 {
                return TRUE;
            }
            /* Might be an empty repeat. */
            continue;
        }

        /* Control reaches here only if one of the items is a small character list.
        All characters are checked against the other side. */
        loop {
            chr = *chr_ptr;

            let lp0 = *list_ptr.add(0);
            if lp0 == OP_CHAR as u32 {
                ochr_ptr = list_ptr.add(2);
                loop {
                    if chr == *ochr_ptr {
                        return FALSE;
                    }
                    ochr_ptr = ochr_ptr.add(1);
                    if *ochr_ptr == NOTACHAR {
                        break;
                    }
                }
            } else if lp0 == OP_NOT as u32 {
                ochr_ptr = list_ptr.add(2);
                loop {
                    if chr == *ochr_ptr {
                        break;
                    }
                    ochr_ptr = ochr_ptr.add(1);
                    if *ochr_ptr == NOTACHAR {
                        break;
                    }
                }
                if *ochr_ptr == NOTACHAR {
                    return FALSE; /* Not found */
                }
            }
            /* Note that OP_DIGIT etc. are generated only when PCRE2_UCP is *not*
            set. */
            else if lp0 == OP_DIGIT as u32 {
                if chr < 256 && (*(*cb).ctypes.add(chr as usize) & ctype_digit) != 0 {
                    return FALSE;
                }
            } else if lp0 == OP_NOT_DIGIT as u32 {
                if chr > 255 || (*(*cb).ctypes.add(chr as usize) & ctype_digit) == 0 {
                    return FALSE;
                }
            } else if lp0 == OP_WHITESPACE as u32 {
                if chr < 256 && (*(*cb).ctypes.add(chr as usize) & ctype_space) != 0 {
                    return FALSE;
                }
            } else if lp0 == OP_NOT_WHITESPACE as u32 {
                if chr > 255 || (*(*cb).ctypes.add(chr as usize) & ctype_space) == 0 {
                    return FALSE;
                }
            } else if lp0 == OP_WORDCHAR as u32 {
                if chr < 255 && (*(*cb).ctypes.add(chr as usize) & ctype_word) != 0 {
                    return FALSE;
                }
            } else if lp0 == OP_NOT_WORDCHAR as u32 {
                if chr > 255 || (*(*cb).ctypes.add(chr as usize) & ctype_word) == 0 {
                    return FALSE;
                }
            } else if lp0 == OP_HSPACE as u32 {
                match chr {
                    CHAR_HT | CHAR_SPACE | CHAR_NBSP | 0x1680 | 0x180e | 0x2000 | 0x2001
                    | 0x2002 | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009
                    | 0x200a | 0x202f | 0x205f | 0x3000 => return FALSE,
                    _ => {}
                }
            } else if lp0 == OP_NOT_HSPACE as u32 {
                match chr {
                    CHAR_HT | CHAR_SPACE | CHAR_NBSP | 0x1680 | 0x180e | 0x2000 | 0x2001
                    | 0x2002 | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009
                    | 0x200a | 0x202f | 0x205f | 0x3000 => {}
                    _ => return FALSE,
                }
            } else if lp0 == OP_ANYNL as u32 || lp0 == OP_VSPACE as u32 {
                match chr {
                    CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028 | 0x2029 => {
                        return FALSE
                    }
                    _ => {}
                }
            } else if lp0 == OP_NOT_VSPACE as u32 {
                match chr {
                    CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028 | 0x2029 => {}
                    _ => return FALSE,
                }
            } else if lp0 == OP_DOLL as u32 || lp0 == OP_EODN as u32 {
                match chr {
                    CHAR_CR | CHAR_LF | CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                        return FALSE
                    }
                    _ => {}
                }
            } else if lp0 == OP_EOD as u32 {
                /* Can always possessify before \z */
            } else if lp0 == OP_PROP as u32 || lp0 == OP_NOTPROP as u32 {
                if check_char_prop(
                    chr,
                    *list_ptr.add(2),
                    *list_ptr.add(3),
                    (lp0 == OP_NOTPROP as u32) as BOOL,
                ) == 0
                {
                    return FALSE;
                }
            } else if lp0 == OP_NCLASS as u32 {
                if chr > 255 {
                    return FALSE;
                }
                /* Fall through to OP_CLASS */
                if chr <= 255 {
                    let this_end = if list_ptr == list.as_ptr() {
                        code
                    } else {
                        base_end
                    };
                    class_bitset = this_end.sub(*list_ptr.add(2) as usize);
                    if (*class_bitset.add((chr >> 3) as usize) & (1u8 << (chr & 7))) != 0 {
                        return FALSE;
                    }
                }
            } else if lp0 == OP_CLASS as u32 {
                if chr <= 255 {
                    let this_end = if list_ptr == list.as_ptr() {
                        code
                    } else {
                        base_end
                    };
                    class_bitset = this_end.sub(*list_ptr.add(2) as usize);
                    if (*class_bitset.add((chr >> 3) as usize) & (1u8 << (chr & 7))) != 0 {
                        return FALSE;
                    }
                }
            } else if lp0 == OP_XCLASS as u32 {
                let this_end = if list_ptr == list.as_ptr() {
                    code
                } else {
                    base_end
                };
                if crate::pcre2_xclass::_pcre2_xclass_8(
                    chr,
                    this_end.sub(*list_ptr.add(2) as usize).add(LINK_SIZE),
                    (*cb).start_code as *const u8,
                    utf,
                ) != 0
                {
                    return FALSE;
                }
            } else if lp0 == OP_ECLASS as u32 {
                let this_end = if list_ptr == list.as_ptr() {
                    code
                } else {
                    base_end
                };
                if crate::pcre2_xclass::_pcre2_eclass_8(
                    chr,
                    this_end.sub(*list_ptr.add(2) as usize).add(LINK_SIZE),
                    this_end.sub(*list_ptr.add(3) as usize),
                    (*cb).start_code as *const u8,
                    utf,
                ) != 0
                {
                    return FALSE;
                }
            } else {
                return FALSE;
            }

            chr_ptr = chr_ptr.add(1);
            if *chr_ptr == NOTACHAR {
                break;
            }
        }

        /* At least one character must be matched from this opcode. */
        if list[1] == 0 {
            return TRUE;
        }
    }
}

/*************************************************
*    Scan compiled regex for auto-possession     *
*************************************************/

/* Replaces single character iterations with their possessive alternatives
if appropriate. This function modifies the compiled opcode!

Arguments:
  code        points to start of the byte code
  cb          compile data block

Returns:      0 for success
              -1 if a non-existant opcode is encountered
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_auto_possessify_8(
    mut code: *mut u8,
    cb: *const compile_block,
) -> c_int {
    let mut c: PCRE2_UCHAR;
    let mut end: PCRE2_SPTR;
    let mut repeat_opcode: *mut PCRE2_UCHAR;
    let mut list = [0u32; MAX_LIST];
    let mut rec_limit: c_int = 1000; /* Was 10,000 but clang+ASAN uses a lot of stack. */
    let utf: BOOL = ((*cb).external_options & PCRE2_UTF != 0) as BOOL;
    let ucp: BOOL = ((*cb).external_options & PCRE2_UCP != 0) as BOOL;

    loop {
        c = *code;

        if c as usize >= OP_TABLE_LENGTH {
            return -1; /* Something gone wrong */
        }

        if c >= OP_STAR && c <= OP_TYPEPOSUPTO {
            c -= get_repeat_base(c) - OP_STAR;
            end = if c <= OP_MINUPTO {
                get_chr_property_list(code, utf, ucp, (*cb).fcc, list.as_mut_ptr())
            } else {
                core::ptr::null()
            };
            list[1] = (c == OP_STAR || c == OP_PLUS || c == OP_QUERY || c == OP_UPTO) as u32;

            if !end.is_null()
                && compare_opcodes(end, utf, ucp, cb, list.as_ptr(), end, &mut rec_limit) != 0
            {
                if c == OP_STAR {
                    *code += OP_POSSTAR - OP_STAR;
                } else if c == OP_MINSTAR {
                    *code += OP_POSSTAR - OP_MINSTAR;
                } else if c == OP_PLUS {
                    *code += OP_POSPLUS - OP_PLUS;
                } else if c == OP_MINPLUS {
                    *code += OP_POSPLUS - OP_MINPLUS;
                } else if c == OP_QUERY {
                    *code += OP_POSQUERY - OP_QUERY;
                } else if c == OP_MINQUERY {
                    *code += OP_POSQUERY - OP_MINQUERY;
                } else if c == OP_UPTO {
                    *code += OP_POSUPTO - OP_UPTO;
                } else if c == OP_MINUPTO {
                    *code += OP_POSUPTO - OP_MINUPTO;
                }
            }
            c = *code;
        } else if c == OP_CLASS || c == OP_NCLASS || c == OP_XCLASS || c == OP_ECLASS {
            if c == OP_XCLASS || c == OP_ECLASS {
                repeat_opcode = code.add(GET(code, 1) as usize);
            } else {
                repeat_opcode = code.add(1 + 32); /* 1 + 32 / sizeof(PCRE2_UCHAR) */
            }

            c = *repeat_opcode;
            if c >= OP_CRSTAR && c <= OP_CRMINRANGE {
                /* The return from get_chr_property_list() will never be NULL when
                 *code (aka c) is one of the four class opcodes. */
                end = get_chr_property_list(code, utf, ucp, (*cb).fcc, list.as_mut_ptr());
                list[1] = ((c & 1) == 0) as u32;

                if !end.is_null()
                    && compare_opcodes(end, utf, ucp, cb, list.as_ptr(), end, &mut rec_limit) != 0
                {
                    if c == OP_CRSTAR || c == OP_CRMINSTAR {
                        *repeat_opcode = OP_CRPOSSTAR;
                    } else if c == OP_CRPLUS || c == OP_CRMINPLUS {
                        *repeat_opcode = OP_CRPOSPLUS;
                    } else if c == OP_CRQUERY || c == OP_CRMINQUERY {
                        *repeat_opcode = OP_CRPOSQUERY;
                    } else if c == OP_CRRANGE || c == OP_CRMINRANGE {
                        *repeat_opcode = OP_CRPOSRANGE;
                    }
                }
            }
            c = *code;
        }

        if c == OP_END {
            return 0;
        } else if c == OP_TYPESTAR
            || c == OP_TYPEMINSTAR
            || c == OP_TYPEPLUS
            || c == OP_TYPEMINPLUS
            || c == OP_TYPEQUERY
            || c == OP_TYPEMINQUERY
            || c == OP_TYPEPOSSTAR
            || c == OP_TYPEPOSPLUS
            || c == OP_TYPEPOSQUERY
        {
            if *code.add(1) == OP_PROP || *code.add(1) == OP_NOTPROP {
                code = code.add(2);
            }
        } else if c == OP_TYPEUPTO
            || c == OP_TYPEMINUPTO
            || c == OP_TYPEEXACT
            || c == OP_TYPEPOSUPTO
        {
            if *code.add(1 + IMM2_SIZE) == OP_PROP || *code.add(1 + IMM2_SIZE) == OP_NOTPROP {
                code = code.add(2);
            }
        } else if c == OP_CALLOUT_STR {
            code = code.add(GET(code, 1 + 2 * LINK_SIZE) as usize);
        } else if c == OP_XCLASS || c == OP_ECLASS {
            code = code.add(GET(code, 1) as usize);
        } else if c == OP_MARK
            || c == OP_COMMIT_ARG
            || c == OP_PRUNE_ARG
            || c == OP_SKIP_ARG
            || c == OP_THEN_ARG
        {
            code = code.add(*code.add(1) as usize);
        }

        /* Add in the fixed length from the table */
        code = code.add(_pcre2_OP_lengths_8[c as usize] as usize);

        /* In UTF-8 mode, opcodes that are followed by a character may be followed
        by a multi-byte character. The length in the table is a minimum, so we have
        to arrange to skip the extra code units. */
        if utf != 0 {
            match c {
                _ if c == OP_CHAR
                    || c == OP_CHARI
                    || c == OP_NOT
                    || c == OP_NOTI
                    || c == OP_STAR
                    || c == OP_MINSTAR
                    || c == OP_PLUS
                    || c == OP_MINPLUS
                    || c == OP_QUERY
                    || c == OP_MINQUERY
                    || c == OP_UPTO
                    || c == OP_MINUPTO
                    || c == OP_EXACT
                    || c == OP_POSSTAR
                    || c == OP_POSPLUS
                    || c == OP_POSQUERY
                    || c == OP_POSUPTO
                    || c == OP_STARI
                    || c == OP_MINSTARI
                    || c == OP_PLUSI
                    || c == OP_MINPLUSI
                    || c == OP_QUERYI
                    || c == OP_MINQUERYI
                    || c == OP_UPTOI
                    || c == OP_MINUPTOI
                    || c == OP_EXACTI
                    || c == OP_POSSTARI
                    || c == OP_POSPLUSI
                    || c == OP_POSQUERYI
                    || c == OP_POSUPTOI
                    || c == OP_NOTSTAR
                    || c == OP_NOTMINSTAR
                    || c == OP_NOTPLUS
                    || c == OP_NOTMINPLUS
                    || c == OP_NOTQUERY
                    || c == OP_NOTMINQUERY
                    || c == OP_NOTUPTO
                    || c == OP_NOTMINUPTO
                    || c == OP_NOTEXACT
                    || c == OP_NOTPOSSTAR
                    || c == OP_NOTPOSPLUS
                    || c == OP_NOTPOSQUERY
                    || c == OP_NOTPOSUPTO
                    || c == OP_NOTSTARI
                    || c == OP_NOTMINSTARI
                    || c == OP_NOTPLUSI
                    || c == OP_NOTMINPLUSI
                    || c == OP_NOTQUERYI
                    || c == OP_NOTMINQUERYI
                    || c == OP_NOTUPTOI
                    || c == OP_NOTMINUPTOI
                    || c == OP_NOTEXACTI
                    || c == OP_NOTPOSSTARI
                    || c == OP_NOTPOSPLUSI
                    || c == OP_NOTPOSQUERYI
                    || c == OP_NOTPOSUPTOI =>
                {
                    if HAS_EXTRALEN(*code.sub(1) as u32) {
                        code = code.add(GET_EXTRALEN(*code.sub(1) as u32) as usize);
                    }
                }
                _ => {}
            }
        }
    }
}

/* End of pcre2_auto_possess.rs */
