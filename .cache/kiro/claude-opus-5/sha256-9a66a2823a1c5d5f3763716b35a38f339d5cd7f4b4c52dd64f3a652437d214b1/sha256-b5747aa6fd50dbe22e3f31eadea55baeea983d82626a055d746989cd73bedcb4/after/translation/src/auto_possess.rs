//! Translation of `c_src/src/pcre2_auto_possess.c`.
//!
//! Contains functions that scan a compiled pattern and change repeats into
//! possessive repeats where possible.

#![allow(non_snake_case, non_upper_case_globals, unused_parens)]

use core::ffi::c_int;

use crate::chars::*;
use crate::internal::*;
use crate::opcodes::*;
use crate::ucp::*;

/* This constant represents the max size of list[] and is used to keep track of
UCD info in several places; it should be kept in sync with the value used by
GenerateUcd.py */
const MAX_LIST: usize = 8;

/* `PRIV(xclass)` and `PRIV(eclass)` live in pcre2_xclass.c, which is compiled
separately. Reference them through their C ABI symbols. */
unsafe extern "C" {
    fn _pcre2_xclass_8(c: u32, data: PCRE2_SPTR, char_lists_end: *const u8, utf: BOOL) -> BOOL;
    fn _pcre2_eclass_8(
        c: u32,
        data_start: PCRE2_SPTR,
        data_end: PCRE2_SPTR,
        char_lists_end: *const u8,
        utf: BOOL,
    ) -> BOOL;
}

/*************************************************
*        Tables for auto-possessification        *
*************************************************/

/* This table is used to check whether auto-possessification is possible
between adjacent character-type opcodes. The left-hand (repeated) opcode is
used to select the row, and the right-hand opcode is use to select the column.
A value of 1 means that auto-possessification is OK. */

const APTROWS: usize = (LAST_AUTOTAB_LEFT_OP - FIRST_AUTOTAB_OP + 1) as usize;
const APTCOLS: usize = (LAST_AUTOTAB_RIGHT_OP - FIRST_AUTOTAB_OP + 1) as usize;

#[rustfmt::skip]
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
opcode is used to select the column. See the C source for the meaning of the
values. */

#[rustfmt::skip]
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
  [ 0,  0,  0,  0,   0,    0,    0,      0,   0,    0,   0,    0,    0 ],  /* PT_BOOL */
  /* PT_ANY does not need a record. */
];

/* This table is used to check whether auto-possessification is possible
between adjacent Unicode property opcodes (OP_PROP and OP_NOTPROP) when one
specifies a general category and the other specifies a particular category. The
row is selected by the general category and the column by the particular
category. The value is 1 if the particular category is not part of the general
category. */

#[rustfmt::skip]
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
a general or particular category. */

#[rustfmt::skip]
static posspropstab: [[u8; 4]; 3] = [
  [ ucp_L as u8, ucp_N as u8, ucp_N as u8, ucp_Nl as u8 ],  /* ALNUM, 3rd and 4th values redundant */
  [ ucp_Z as u8, ucp_Z as u8, ucp_C as u8, ucp_Cc as u8 ],  /* SPACE and PXSPACE, 2nd value redundant */
  [ ucp_L as u8, ucp_N as u8, ucp_P as u8, ucp_Po as u8 ],  /* WORD */
];

/* Helper: is `c` one of the HSPACE_CASES code points? */
#[inline]
fn is_hspace_case(c: u32) -> bool {
    match c {
        /* HSPACE_BYTE_CASES */
        CHAR_HT | CHAR_SPACE | CHAR_NBSP => true,
        /* HSPACE_MULTIBYTE_CASES */
        0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002 | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007
        | 0x2008 | 0x2009 | 0x200a | 0x202f | 0x205f | 0x3000 => true,
        _ => false,
    }
}

/* Helper: is `c` one of the VSPACE_CASES code points? */
#[inline]
fn is_vspace_case(c: u32) -> bool {
    match c {
        /* VSPACE_BYTE_CASES */
        CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL => true,
        /* VSPACE_MULTIBYTE_CASES */
        0x2028 | 0x2029 => true,
        _ => false,
    }
}

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
    unsafe {
        let prop = get_ucd(c);

        let btf = |b: bool| -> BOOL { if b { TRUE } else { FALSE } };
        let neg = negated != FALSE;

        match ptype {
            PT_LAMP => btf(
                (prop.chartype as u32 == ucp_Lu
                    || prop.chartype as u32 == ucp_Ll
                    || prop.chartype as u32 == ucp_Lt)
                    == neg,
            ),

            PT_GC => btf((pdata == UCP_GENTYPE[prop.chartype as usize]) == neg),

            PT_PC => btf((pdata == prop.chartype as u32) == neg),

            PT_SC => btf((pdata == prop.script as u32) == neg),

            PT_SCX => {
                let ok = pdata == prop.script as u32
                    || mapbit(
                        &UCD_SCRIPT_SETS[ucd_scriptx_prop(prop) as usize..],
                        pdata,
                    ) != 0;
                btf(ok == neg)
            }

            /* These are specials */
            PT_ALNUM => btf(
                (UCP_GENTYPE[prop.chartype as usize] == ucp_L
                    || UCP_GENTYPE[prop.chartype as usize] == ucp_N)
                    == neg,
            ),

            /* Perl space used to exclude VT, but from Perl 5.18 it is included,
            which means that Perl space and POSIX space are now identical. */
            PT_SPACE | PT_PXSPACE => {
                let rc = if is_hspace_case(c) || is_vspace_case(c) {
                    negated
                } else {
                    btf((UCP_GENTYPE[prop.chartype as usize] == ucp_Z) == neg)
                };
                rc
            }

            PT_WORD => btf(
                (UCP_GENTYPE[prop.chartype as usize] == ucp_L
                    || UCP_GENTYPE[prop.chartype as usize] == ucp_N
                    || c == CHAR_UNDERSCORE)
                    == neg,
            ),

            PT_CLIST => {
                let p = &UCD_CASELESS_SETS[prop.caseset as usize..];
                let mut idx = 0usize;
                loop {
                    if c < p[idx] {
                        return btf(!neg);
                    }
                    let v = p[idx];
                    idx += 1;
                    if c == v {
                        return negated;
                    }
                }
            }

            /* Haven't yet thought these through. */
            PT_BIDICL => FALSE,

            PT_BOOL => FALSE,

            _ => FALSE,
        }
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
    unsafe {
        let mut c: PCRE2_UCHAR = *code;
        let base: PCRE2_UCHAR;
        let mut end: PCRE2_SPTR;
        let class_end: PCRE2_SPTR;
        let chr: u32;

        *list.add(0) = c as u32;
        *list.add(1) = FALSE as u32;
        code = code.add(1);

        if c >= OP_STAR && c <= OP_TYPEPOSUPTO {
            base = get_repeat_base(c);
            c -= base - OP_STAR;

            if c == OP_UPTO || c == OP_MINUPTO || c == OP_EXACT || c == OP_POSUPTO {
                code = code.add(IMM2_SIZE);
            }

            *list.add(1) = (c != OP_PLUS && c != OP_MINPLUS && c != OP_EXACT && c != OP_POSPLUS)
                as u32;

            match base {
                OP_STAR => {
                    *list.add(0) = OP_CHAR as u32;
                }
                OP_STARI => {
                    *list.add(0) = OP_CHARI as u32;
                }
                OP_NOTSTAR => {
                    *list.add(0) = OP_NOT as u32;
                }
                OP_NOTSTARI => {
                    *list.add(0) = OP_NOTI as u32;
                }
                OP_TYPESTAR => {
                    *list.add(0) = *code as u32;
                    code = code.add(1);
                }
                _ => {}
            }
            c = *list.add(0) as PCRE2_UCHAR;
        }

        match c {
            OP_NOT_DIGIT | OP_DIGIT | OP_NOT_WHITESPACE | OP_WHITESPACE | OP_NOT_WORDCHAR
            | OP_WORDCHAR | OP_ANY | OP_ALLANY | OP_ANYNL | OP_NOT_HSPACE | OP_HSPACE
            | OP_NOT_VSPACE | OP_VSPACE | OP_EXTUNI | OP_EODN | OP_EOD | OP_DOLL | OP_DOLLM => {
                code
            }

            OP_CHAR | OP_NOT => {
                let mut p = code;
                chr = getcharinctest(&mut p, utf != FALSE);
                code = p;
                *list.add(2) = chr;
                *list.add(3) = NOTACHAR;
                code
            }

            OP_CHARI | OP_NOTI => {
                *list.add(0) = if c == OP_CHARI { OP_CHAR as u32 } else { OP_NOT as u32 };
                let mut p = code;
                chr = getcharinctest(&mut p, utf != FALSE);
                code = p;
                *list.add(2) = chr;

                /* SUPPORT_UNICODE branch */
                if chr < 128 || (chr < 256 && utf == FALSE && ucp == FALSE) {
                    *list.add(3) = *fcc.add(chr as usize) as u32;
                } else {
                    *list.add(3) = ucd_othercase(chr);
                }

                /* The othercase might be the same value. */
                if chr == *list.add(3) {
                    *list.add(3) = NOTACHAR;
                } else {
                    *list.add(4) = NOTACHAR;
                }
                code
            }

            OP_PROP | OP_NOTPROP => {
                if *code.add(0) != PT_CLIST as u8 {
                    *list.add(2) = *code.add(0) as u32;
                    *list.add(3) = *code.add(1) as u32;
                    return code.add(2);
                }

                /* Convert only if we have enough space. */
                let clist_src = &UCD_CASELESS_SETS[*code.add(1) as usize..];
                let mut src_idx = 0usize;
                let mut clist_dest = list.add(2);
                code = code.add(2);

                loop {
                    if clist_dest >= list.add(MAX_LIST) {
                        /* Early return if there is not enough space. */
                        *list.add(2) = *code.add(0) as u32;
                        *list.add(3) = *code.add(1) as u32;
                        return code;
                    }
                    *clist_dest = clist_src[src_idx];
                    clist_dest = clist_dest.add(1);
                    let v = clist_src[src_idx];
                    src_idx += 1;
                    if v == NOTACHAR {
                        break;
                    }
                }

                /* All characters are stored. The terminating NOTACHAR is copied
                from the clist itself. */
                *list.add(0) = if c == OP_PROP { OP_CHAR as u32 } else { OP_NOT as u32 };
                code
            }

            OP_NCLASS | OP_CLASS | OP_XCLASS | OP_ECLASS => {
                if c == OP_XCLASS || c == OP_ECLASS {
                    end = code.add(get(code, 0) as usize).sub(1);
                } else {
                    end = code.add(32 / core::mem::size_of::<PCRE2_UCHAR>());
                }
                class_end = end;

                match *end {
                    OP_CRSTAR | OP_CRMINSTAR | OP_CRQUERY | OP_CRMINQUERY | OP_CRPOSSTAR
                    | OP_CRPOSQUERY => {
                        *list.add(1) = TRUE as u32;
                        end = end.add(1);
                    }

                    OP_CRPLUS | OP_CRMINPLUS | OP_CRPOSPLUS => {
                        end = end.add(1);
                    }

                    OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                        *list.add(1) = (get2(end, 1) == 0) as u32;
                        end = end.add(1 + 2 * IMM2_SIZE);
                    }

                    _ => {}
                }
                *list.add(2) = end.offset_from(code) as u32;
                *list.add(3) = end.offset_from(class_end) as u32;
                end
            }

            _ => core::ptr::null(), /* Opcode not accepted */
        }
    }
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
    unsafe {
        let mut c: PCRE2_UCHAR;
        let mut list = [0u32; MAX_LIST];
        let mut chr_ptr: *const u32 = core::ptr::null();
        let mut ochr_ptr: *const u32;
        let mut list_ptr: *const u32 = core::ptr::null();
        let mut entered_a_group: BOOL = FALSE;

        *rec_limit -= 1;
        if *rec_limit <= 0 {
            return FALSE; /* Recursion has gone too deep */
        }

        /* Note: the base_list[1] contains whether the current opcode has a
        greedy (represented by a non-zero value) quantifier. */

        loop {
            /* All operations move the code pointer forward.
            Therefore infinite recursions are not possible. */
            c = *code;

            /* Skip over callouts */
            if c == OP_CALLOUT {
                code = code.add(OP_LENGTHS[c as usize] as usize);
                continue;
            }

            if c == OP_CALLOUT_STR {
                code = code.add(get(code, 1 + 2 * LINK_SIZE) as usize);
                continue;
            }

            /* At the end of a branch, skip to the end of the group and process
            it. */
            if c == OP_ALT {
                loop {
                    code = code.add(get(code, 1) as usize);
                    if *code != OP_ALT {
                        break;
                    }
                }
                c = *code;
            }

            /* Inspect the next opcode. */
            match c {
                /* We can always possessify a greedy iterator at the end of the
                pattern, which is reached after skipping over the final OP_KET.
                A non-greedy iterator must never be possessified. */
                OP_END => return (*base_list.add(1) != 0) as BOOL,

                /* When an iterator is at the end of certain kinds of group we
                can inspect what follows the group by skipping over the closing
                ket. */
                OP_KET | OP_KETRPOS => {
                    /* The non-greedy case cannot be converted to a possessive
                    form. */
                    if *base_list.add(1) == 0 {
                        return FALSE;
                    }

                    /* If the bracket is capturing it might be referenced by an
                    OP_RECURSE so its last iterator can never be possessified if
                    the pattern contains recursions. */
                    let mut bracode = code.sub(get(code, 1) as usize);
                    match *bracode {                        OP_CBRA | OP_SCBRA | OP_CBRAPOS | OP_SCBRAPOS => {
                            if (*cb).had_recurse != FALSE {
                                return FALSE;
                            }
                        }

                        /* A script run might have to backtrack if the iterated
                        item can match characters from more than one script. */
                        OP_SCRIPT_RUN => {
                            if *base_list.add(0) != OP_CHAR as u32
                                && *base_list.add(0) != OP_CHARI as u32
                            {
                                return FALSE;
                            }
                        }

                        /* Atomic sub-patterns and forward assertions can always
                        auto-possessify their last iterator. However, if the
                        group was entered as a result of checking a previous
                        iterator, this is not possible. */
                        OP_ASSERT | OP_ASSERT_NOT | OP_ONCE => {
                            return (entered_a_group == FALSE) as BOOL;
                        }

                        /* Fixed-length lookbehinds can be treated the same way,
                        but variable length lookbehinds must not auto-possessify
                        their last iterator. */
                        OP_ASSERTBACK | OP_ASSERTBACK_NOT => {
                            loop {
                                if *bracode.add(1 + LINK_SIZE) == OP_VREVERSE {
                                    return FALSE; /* Variable */
                                }
                                bracode = bracode.add(get(bracode, 1) as usize);
                                if *bracode != OP_ALT {
                                    break;
                                }
                            }
                            return (entered_a_group == FALSE) as BOOL; /* Not variable length */
                        }

                        /* Non-atomic assertions - don't possessify last
                        iterator. This needs more thought. */
                        OP_ASSERT_NA | OP_ASSERTBACK_NA => return FALSE,

                        _ => {}
                    }

                    /* Skip over the bracket and inspect what comes next. */
                    code = code.add(OP_LENGTHS[c as usize] as usize);
                    continue;
                }

                /* Handle cases where the next item is a group. */
                OP_ONCE | OP_BRA | OP_CBRA => {
                    let mut next_code = code.add(get(code, 1) as usize);
                    code = code.add(OP_LENGTHS[c as usize] as usize);

                    /* Check each branch. We have to recurse a level for all but
                    the last branch. */
                    while *next_code == OP_ALT {
                        if compare_opcodes(code, utf, ucp, cb, base_list, base_end, rec_limit)
                            == FALSE
                        {
                            return FALSE;
                        }
                        code = next_code.add(1 + LINK_SIZE);
                        next_code = next_code.add(get(next_code, 1) as usize);
                    }

                    entered_a_group = TRUE;
                    continue;
                }

                OP_BRAZERO | OP_BRAMINZERO => {
                    let mut nc = code.add(1);
                    if *nc != OP_BRA && *nc != OP_CBRA && *nc != OP_ONCE {
                        return FALSE;
                    }

                    loop {
                        nc = nc.add(get(nc, 1) as usize);
                        if *nc != OP_ALT {
                            break;
                        }
                    }

                    /* The bracket content will be checked by the OP_BRA/OP_CBRA
                    case above. */
                    nc = nc.add(1 + LINK_SIZE);
                    if compare_opcodes(nc, utf, ucp, cb, base_list, base_end, rec_limit) == FALSE {
                        return FALSE;
                    }

                    code = code.add(OP_LENGTHS[c as usize] as usize);
                    continue;
                }

                /* The next opcode does not need special handling; fall through
                and use it to see if the base can be possessified. */
                _ => {}
            }

            /* We now have the next appropriate opcode to compare with the base.
            Check for a supported opcode, and load its properties. */
            code = get_chr_property_list(code, utf, ucp, (*cb).fcc, list.as_mut_ptr());
            if code.is_null() {
                return FALSE; /* Unsupported */
            }

            /* If either opcode is a small character list, set pointers for
            comparing characters from that list with another list, or with a
            property. */
            if *base_list.add(0) == OP_CHAR as u32 {
                chr_ptr = base_list.add(2);
                list_ptr = list.as_ptr();
            } else if list[0] == OP_CHAR as u32 {
                chr_ptr = list.as_ptr().add(2);
                list_ptr = base_list;
            }
            /* Character bitsets can also be compared to certain opcodes. */
            else if *base_list.add(0) == OP_CLASS as u32
                || list[0] == OP_CLASS as u32
                /* In 8 bit, non-UTF mode, OP_CLASS and OP_NCLASS are the same. */
                || (utf == FALSE
                    && (*base_list.add(0) == OP_NCLASS as u32 || list[0] == OP_NCLASS as u32))
            {
                let mut set1: *const u8;
                let mut set2: *const u8;
                let mut invert_bits: BOOL;
                if *base_list.add(0) == OP_CLASS as u32
                    || (utf == FALSE && *base_list.add(0) == OP_NCLASS as u32)
                {
                    set1 = base_end.sub(*base_list.add(2) as usize);
                    list_ptr = list.as_ptr();
                } else {
                    set1 = code.sub(list[2] as usize);
                    list_ptr = base_list;
                }

                invert_bits = FALSE;
                match *list_ptr.add(0) as u8 {
                    OP_CLASS | OP_NCLASS => {
                        let base_ptr = if list_ptr == list.as_ptr() { code } else { base_end };
                        set2 = base_ptr.sub(*list_ptr.add(2) as usize);
                    }

                    OP_XCLASS => {
                        let base_ptr = if list_ptr == list.as_ptr() { code } else { base_end };
                        let xclass_flags =
                            base_ptr.sub(*list_ptr.add(2) as usize).add(LINK_SIZE);
                        if (*xclass_flags & XCL_HASPROP as u8) != 0 {
                            return FALSE;
                        }
                        if (*xclass_flags & XCL_MAP as u8) == 0 {
                            /* No bits are set for characters < 256. */
                            if list[1] == 0 {
                                return ((*xclass_flags & XCL_NOT as u8) == 0) as BOOL;
                            }
                            /* Might be an empty repeat. */
                            continue;
                        }
                        set2 = xclass_flags.add(1);
                    }

                    OP_NOT_DIGIT => {
                        invert_bits = TRUE;
                        set2 = (*cb).cbits.add(cbit_digit);
                    }
                    OP_DIGIT => {
                        set2 = (*cb).cbits.add(cbit_digit);
                    }

                    OP_NOT_WHITESPACE => {
                        invert_bits = TRUE;
                        set2 = (*cb).cbits.add(cbit_space);
                    }
                    OP_WHITESPACE => {
                        set2 = (*cb).cbits.add(cbit_space);
                    }

                    OP_NOT_WORDCHAR => {
                        invert_bits = TRUE;
                        set2 = (*cb).cbits.add(cbit_word);
                    }
                    OP_WORDCHAR => {
                        set2 = (*cb).cbits.add(cbit_word);
                    }

                    _ => return FALSE,
                }

                /* Because the bit sets are unaligned bytes, we need to perform
                byte comparison here. */
                let set_end = set1.add(32);
                if invert_bits != FALSE {
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
            /* Some property combinations also acceptable. */
            else {
                let leftop = *base_list.add(0);
                let rightop = list[0];

                let mut acc: BOOL = FALSE; /* Always set in non-unicode case. */
                if leftop == OP_PROP as u32 || leftop == OP_NOTPROP as u32 {
                    if rightop == OP_EOD as u32 {
                        acc = TRUE;
                    } else if rightop == OP_PROP as u32 || rightop == OP_NOTPROP as u32 {
                        let same: BOOL = (leftop == rightop) as BOOL;
                        let lisprop: BOOL = (leftop == OP_PROP as u32) as BOOL;
                        let risprop: BOOL = (rightop == OP_PROP as u32) as BOOL;
                        let bothprop: BOOL = (lisprop != FALSE && risprop != FALSE) as BOOL;

                        let n = propposstab[*base_list.add(2) as usize][list[2] as usize];
                        match n {
                            0 => {}
                            1 => acc = bothprop,
                            2 => {
                                acc =
                                    ((*base_list.add(3) == list[3]) as BOOL != same) as BOOL;
                            }
                            3 => acc = (same == FALSE) as BOOL,

                            4 => {
                                /* Left general category, right particular category */
                                acc = (risprop != FALSE
                                    && (catposstab[*base_list.add(3) as usize][list[3] as usize]
                                        as BOOL
                                        == same)) as BOOL;
                            }

                            5 => {
                                /* Right general category, left particular category */
                                acc = (lisprop != FALSE
                                    && (catposstab[list[3] as usize][*base_list.add(3) as usize]
                                        as BOOL
                                        == same)) as BOOL;
                            }

                            6 | 7 | 8 => {
                                /* Left {alnum,space,word} vs right general category */
                                let p = &posspropstab[(n - 6) as usize];
                                acc = (risprop != FALSE
                                    && (lisprop
                                        == ((list[3] != p[0] as u32
                                            && list[3] != p[1] as u32
                                            && (list[3] != p[2] as u32 || lisprop == FALSE))
                                            as BOOL)))
                                    as BOOL;
                            }

                            9 | 10 | 11 => {
                                /* Right {alnum,space,word} vs left general category */
                                let p = &posspropstab[(n - 9) as usize];
                                acc = (lisprop != FALSE
                                    && (risprop
                                        == ((*base_list.add(3) != p[0] as u32
                                            && *base_list.add(3) != p[1] as u32
                                            && (*base_list.add(3) != p[2] as u32
                                                || risprop == FALSE))
                                            as BOOL)))
                                    as BOOL;
                            }

                            12 | 13 | 14 => {
                                /* Left {alnum,space,word} vs right particular category */
                                let p = &posspropstab[(n - 12) as usize];
                                acc = (risprop != FALSE
                                    && (lisprop
                                        == ((catposstab[p[0] as usize][list[3] as usize] != 0
                                            && catposstab[p[1] as usize][list[3] as usize] != 0
                                            && (list[3] != p[3] as u32 || lisprop == FALSE))
                                            as BOOL)))
                                    as BOOL;
                            }

                            15 | 16 | 17 => {
                                /* Right {alnum,space,word} vs left particular category */
                                let p = &posspropstab[(n - 15) as usize];
                                acc = (lisprop != FALSE
                                    && (risprop
                                        == ((catposstab[p[0] as usize][*base_list.add(3) as usize]
                                            != 0
                                            && catposstab[p[1] as usize]
                                                [*base_list.add(3) as usize]
                                                != 0
                                            && (*base_list.add(3) != p[3] as u32
                                                || risprop == FALSE))
                                            as BOOL)))
                                    as BOOL;
                            }

                            _ => {}
                        }
                    }
                } else {
                    acc = (leftop >= FIRST_AUTOTAB_OP as u32
                        && leftop <= LAST_AUTOTAB_LEFT_OP as u32
                        && rightop >= FIRST_AUTOTAB_OP as u32
                        && rightop <= LAST_AUTOTAB_RIGHT_OP as u32
                        && autoposstab[(leftop - FIRST_AUTOTAB_OP as u32) as usize]
                            [(rightop - FIRST_AUTOTAB_OP as u32) as usize]
                            != 0) as BOOL;
                }

                let accepted = acc;

                if accepted == FALSE {
                    return FALSE;
                }

                if list[1] == 0 {
                    return TRUE;
                }
                /* Might be an empty repeat. */
                continue;
            }

            /* Control reaches here only if one of the items is a small
            character list. All characters are checked against the other side. */
            loop {
                let chr = *chr_ptr;

                match *list_ptr.add(0) as u8 {
                    OP_CHAR => {
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
                    }

                    OP_NOT => {
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

                    /* Note that OP_DIGIT etc. are generated only when PCRE2_UCP
                    is *not* set. */
                    OP_DIGIT => {
                        if chr < 256 && (*(*cb).ctypes.add(chr as usize) & ctype_digit) != 0 {
                            return FALSE;
                        }
                    }

                    OP_NOT_DIGIT => {
                        if chr > 255 || (*(*cb).ctypes.add(chr as usize) & ctype_digit) == 0 {
                            return FALSE;
                        }
                    }

                    OP_WHITESPACE => {
                        if chr < 256 && (*(*cb).ctypes.add(chr as usize) & ctype_space) != 0 {
                            return FALSE;
                        }
                    }

                    OP_NOT_WHITESPACE => {
                        if chr > 255 || (*(*cb).ctypes.add(chr as usize) & ctype_space) == 0 {
                            return FALSE;
                        }
                    }

                    OP_WORDCHAR => {
                        if chr < 255 && (*(*cb).ctypes.add(chr as usize) & ctype_word) != 0 {
                            return FALSE;
                        }
                    }

                    OP_NOT_WORDCHAR => {
                        if chr > 255 || (*(*cb).ctypes.add(chr as usize) & ctype_word) == 0 {
                            return FALSE;
                        }
                    }

                    OP_HSPACE => {
                        if is_hspace_case(chr) {
                            return FALSE;
                        }
                    }

                    OP_NOT_HSPACE => {
                        if !is_hspace_case(chr) {
                            return FALSE;
                        }
                    }

                    OP_ANYNL | OP_VSPACE => {
                        if is_vspace_case(chr) {
                            return FALSE;
                        }
                    }

                    OP_NOT_VSPACE => {
                        if !is_vspace_case(chr) {
                            return FALSE;
                        }
                    }

                    OP_DOLL | OP_EODN => match chr {
                        CHAR_CR | CHAR_LF | CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                            return FALSE;
                        }
                        _ => {}
                    },

                    OP_EOD => {
                        /* Can always possessify before \z */
                    }

                    OP_PROP | OP_NOTPROP => {
                        if check_char_prop(
                            chr,
                            *list_ptr.add(2),
                            *list_ptr.add(3),
                            (*list_ptr.add(0) == OP_NOTPROP as u32) as BOOL,
                        ) == FALSE
                        {
                            return FALSE;
                        }
                    }

                    OP_NCLASS => {
                        if chr > 255 {
                            return FALSE;
                        }
                        /* Fall through: chr <= 255 here, so the OP_CLASS
                        `chr > 255` check below would never break. */
                        let base_ptr = if list_ptr == list.as_ptr() { code } else { base_end };
                        let class_bitset = base_ptr.sub(*list_ptr.add(2) as usize);
                        if (*class_bitset.add((chr >> 3) as usize) & (1u8 << (chr & 7))) != 0 {
                            return FALSE;
                        }
                    }

                    OP_CLASS => {
                        if chr > 255 {
                            /* break out of the class check */
                        } else {
                            let base_ptr =
                                if list_ptr == list.as_ptr() { code } else { base_end };
                            let class_bitset = base_ptr.sub(*list_ptr.add(2) as usize);
                            if (*class_bitset.add((chr >> 3) as usize) & (1u8 << (chr & 7))) != 0 {
                                return FALSE;
                            }
                        }
                    }

                    OP_XCLASS => {
                        let base_ptr = if list_ptr == list.as_ptr() { code } else { base_end };
                        if _pcre2_xclass_8(
                            chr,
                            base_ptr.sub(*list_ptr.add(2) as usize).add(LINK_SIZE),
                            (*cb).start_code as *const u8,
                            utf,
                        ) != FALSE
                        {
                            return FALSE;
                        }
                    }

                    OP_ECLASS => {
                        let base_ptr = if list_ptr == list.as_ptr() { code } else { base_end };
                        if _pcre2_eclass_8(
                            chr,
                            base_ptr.sub(*list_ptr.add(2) as usize).add(LINK_SIZE),
                            base_ptr.sub(*list_ptr.add(3) as usize),
                            (*cb).start_code as *const u8,
                            utf,
                        ) != FALSE
                        {
                            return FALSE;
                        }
                    }

                    _ => return FALSE,
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
}

/*************************************************
*    Scan compiled regex for auto-possession     *
*************************************************/

/* Replaces single character iterations with their possessive alternatives if
appropriate. This function modifies the compiled opcode!

Arguments:
  code        points to start of the byte code
  cb          compile data block

Returns:      0 for success
              -1 if a non-existant opcode is encountered
*/

pub unsafe fn auto_possessify(mut code: *mut PCRE2_UCHAR, cb: *const compile_block) -> c_int {
    unsafe {
        let mut c: PCRE2_UCHAR;
        let mut end: PCRE2_SPTR;
        let mut repeat_opcode: *mut PCRE2_UCHAR;
        let mut list = [0u32; MAX_LIST];
        let mut rec_limit: c_int = 1000; /* Was 10,000 but clang+ASAN uses a lot of stack. */
        let utf: BOOL = (((*cb).external_options & PCRE2_UTF) != 0) as BOOL;
        let ucp: BOOL = (((*cb).external_options & PCRE2_UCP) != 0) as BOOL;

        loop {
            c = *code;

            if c >= OP_TABLE_LENGTH {
                return -1; /* Something gone wrong */
            }

            if c >= OP_STAR && c <= OP_TYPEPOSUPTO {
                c -= get_repeat_base(c) - OP_STAR;
                end = if c <= OP_MINUPTO {
                    get_chr_property_list(code, utf, ucp, (*cb).fcc, list.as_mut_ptr())
                } else {
                    core::ptr::null()
                };
                list[1] =
                    (c == OP_STAR || c == OP_PLUS || c == OP_QUERY || c == OP_UPTO) as u32;

                if !end.is_null()
                    && compare_opcodes(end, utf, ucp, cb, list.as_ptr(), end, &mut rec_limit)
                        != FALSE
                {
                    match c {
                        OP_STAR => *code += OP_POSSTAR - OP_STAR,
                        OP_MINSTAR => *code += OP_POSSTAR - OP_MINSTAR,
                        OP_PLUS => *code += OP_POSPLUS - OP_PLUS,
                        OP_MINPLUS => *code += OP_POSPLUS - OP_MINPLUS,
                        OP_QUERY => *code += OP_POSQUERY - OP_QUERY,
                        OP_MINQUERY => *code += OP_POSQUERY - OP_MINQUERY,
                        OP_UPTO => *code += OP_POSUPTO - OP_UPTO,
                        OP_MINUPTO => *code += OP_POSUPTO - OP_MINUPTO,
                        _ => {}
                    }
                }
                c = *code;
            } else if c == OP_CLASS || c == OP_NCLASS || c == OP_XCLASS || c == OP_ECLASS {
                if c == OP_XCLASS || c == OP_ECLASS {
                    repeat_opcode = code.add(get(code, 1) as usize);
                } else {
                    repeat_opcode = code.add(1 + (32 / core::mem::size_of::<PCRE2_UCHAR>()));
                }

                c = *repeat_opcode;
                if c >= OP_CRSTAR && c <= OP_CRMINRANGE {
                    /* The return from get_chr_property_list() will never be NULL
                    when *code (aka c) is one of the four class opcodes. */
                    end = get_chr_property_list(code, utf, ucp, (*cb).fcc, list.as_mut_ptr());
                    list[1] = ((c & 1) == 0) as u32;

                    if !end.is_null()
                        && compare_opcodes(end, utf, ucp, cb, list.as_ptr(), end, &mut rec_limit)
                            != FALSE
                    {
                        match c {
                            OP_CRSTAR | OP_CRMINSTAR => *repeat_opcode = OP_CRPOSSTAR,
                            OP_CRPLUS | OP_CRMINPLUS => *repeat_opcode = OP_CRPOSPLUS,
                            OP_CRQUERY | OP_CRMINQUERY => *repeat_opcode = OP_CRPOSQUERY,
                            OP_CRRANGE | OP_CRMINRANGE => *repeat_opcode = OP_CRPOSRANGE,
                            _ => {}
                        }
                    }
                }
                c = *code;
            }

            match c {
                OP_END => return 0,

                OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEQUERY
                | OP_TYPEMINQUERY | OP_TYPEPOSSTAR | OP_TYPEPOSPLUS | OP_TYPEPOSQUERY => {
                    if *code.add(1) == OP_PROP || *code.add(1) == OP_NOTPROP {
                        code = code.add(2);
                    }
                }

                OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEEXACT | OP_TYPEPOSUPTO => {
                    if *code.add(1 + IMM2_SIZE) == OP_PROP
                        || *code.add(1 + IMM2_SIZE) == OP_NOTPROP
                    {
                        code = code.add(2);
                    }
                }

                OP_CALLOUT_STR => {
                    code = code.add(get(code, 1 + 2 * LINK_SIZE) as usize);
                }

                OP_XCLASS | OP_ECLASS => {
                    code = code.add(get(code, 1) as usize);
                }

                OP_MARK | OP_COMMIT_ARG | OP_PRUNE_ARG | OP_SKIP_ARG | OP_THEN_ARG => {
                    code = code.add(*code.add(1) as usize);
                }

                _ => {}
            }

            /* Add in the fixed length from the table */
            code = code.add(OP_LENGTHS[c as usize] as usize);

            /* In UTF-8 mode, opcodes that are followed by a character may be
            followed by a multi-byte character. The length in the table is a
            minimum, so we have to arrange to skip the extra code units.
            MAYBE_UTF_MULTI is defined in 8-bit mode. */
            if utf != FALSE {
                match c {
                    OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI | OP_STAR | OP_MINSTAR | OP_PLUS
                    | OP_MINPLUS | OP_QUERY | OP_MINQUERY | OP_UPTO | OP_MINUPTO | OP_EXACT
                    | OP_POSSTAR | OP_POSPLUS | OP_POSQUERY | OP_POSUPTO | OP_STARI
                    | OP_MINSTARI | OP_PLUSI | OP_MINPLUSI | OP_QUERYI | OP_MINQUERYI | OP_UPTOI
                    | OP_MINUPTOI | OP_EXACTI | OP_POSSTARI | OP_POSPLUSI | OP_POSQUERYI
                    | OP_POSUPTOI | OP_NOTSTAR | OP_NOTMINSTAR | OP_NOTPLUS | OP_NOTMINPLUS
                    | OP_NOTQUERY | OP_NOTMINQUERY | OP_NOTUPTO | OP_NOTMINUPTO | OP_NOTEXACT
                    | OP_NOTPOSSTAR | OP_NOTPOSPLUS | OP_NOTPOSQUERY | OP_NOTPOSUPTO
                    | OP_NOTSTARI | OP_NOTMINSTARI | OP_NOTPLUSI | OP_NOTMINPLUSI | OP_NOTQUERYI
                    | OP_NOTMINQUERYI | OP_NOTUPTOI | OP_NOTMINUPTOI | OP_NOTEXACTI
                    | OP_NOTPOSSTARI | OP_NOTPOSPLUSI | OP_NOTPOSQUERYI | OP_NOTPOSUPTOI => {
                        if has_extralen(*code.sub(1) as u32) {
                            code = code.add(get_extralen(*code.sub(1) as u32) as usize);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Exported as `_pcre2_auto_possessify_8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_auto_possessify_8(
    code: *mut PCRE2_UCHAR,
    cb: *const compile_block,
) -> c_int {
    unsafe { auto_possessify(code, cb) }
}
