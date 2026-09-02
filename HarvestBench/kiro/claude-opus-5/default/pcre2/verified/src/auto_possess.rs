//! Translation of `pcre2_auto_possess.c` (8-bit, `SUPPORT_UNICODE` on,
//! `SUPPORT_WIDE_CHARS` on, `SUPPORT_JIT` off).
//!
//! This module contains functions that scan a compiled pattern and change
//! repeats into possessive repeats where possible.

use crate::internal::*;
use crate::tables;
use core::ffi::c_int;
use core::ptr;

// ---------------------------------------------------------------------------
// Local character constants (ASCII / non-EBCDIC configuration).
// ---------------------------------------------------------------------------

const CHAR_HT: u32 = 0x09;
const CHAR_LF: u32 = 0x0a;
const CHAR_VT: u32 = 0x0b;
const CHAR_FF: u32 = 0x0c;
const CHAR_CR: u32 = 0x0d;
const CHAR_SPACE: u32 = 0x20;
const CHAR_UNDERSCORE: u32 = 0x5f;
const CHAR_NEL: u32 = 0x85;
const CHAR_NBSP: u32 = 0xa0;

/// `NOTACHAR` as a `u32` for convenient comparison against list contents.
const NOTACHAR_U32: u32 = NOTACHAR as u32;

// ---------------------------------------------------------------------------
// External helpers still living in other translation units.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    /// `PRIV(xclass)` — test a character against an extended class.
    #[link_name = "_pcre2_xclass_8"]
    fn priv_xclass(c: u32, data: PCRE2_SPTR, char_lists_end: *const u8, utf: BOOL) -> BOOL;

    /// `PRIV(eclass)` — test a character against a class using set operations.
    #[link_name = "_pcre2_eclass_8"]
    fn priv_eclass(
        c: u32,
        data_start: PCRE2_SPTR,
        data_end: PCRE2_SPTR,
        char_lists_end: *const u8,
        utf: BOOL,
    ) -> BOOL;
}

// ---------------------------------------------------------------------------
// This macro represents the max size of list[] and that is used to keep track
// of UCD info in several places, it should be kept in sync with the value used
// by GenerateUcd.py.
// ---------------------------------------------------------------------------

const MAX_LIST: usize = 8;

// ---------------------------------------------------------------------------
//        Tables for auto-possessification
// ---------------------------------------------------------------------------

// This table is used to check whether auto-possessification is possible between
// adjacent character-type opcodes. The left-hand (repeated) opcode is used to
// select the row, and the right-hand opcode is used to select the column. A
// value of 1 means that auto-possessification is OK.
//
// APTROWS = LAST_AUTOTAB_LEFT_OP - FIRST_AUTOTAB_OP + 1 = 17
// APTCOLS = LAST_AUTOTAB_RIGHT_OP - FIRST_AUTOTAB_OP + 1 = 21

const APTROWS: usize = (LAST_AUTOTAB_LEFT_OP - FIRST_AUTOTAB_OP + 1) as usize;
const APTCOLS: usize = (LAST_AUTOTAB_RIGHT_OP - FIRST_AUTOTAB_OP + 1) as usize;

static autoposstab: [[u8; APTCOLS]; APTROWS] = [
    /* \D \d \S \s \W \w  . .+ \C \P \p \R \H \h \V \v \X \Z \z  $ $M */
    [0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0], /* \D */
    [1, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1, 1], /* \d */
    [0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1, 1], /* \S */
    [0, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0], /* \s */
    [0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0], /* \W */
    [0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 1, 1, 1], /* \w */
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0], /* .  */
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0], /* .+ */
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0], /* \C */
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], /* \P */
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], /* \p */
    [0, 1, 0, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0], /* \R */
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0], /* \H */
    [0, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 0, 0, 1, 0, 0], /* \h */
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0], /* \V */
    [0, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 1, 0, 0], /* \v */
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0], /* \X */
];

// This table is used to check whether auto-possessification is possible between
// adjacent Unicode property opcodes (OP_PROP and OP_NOTPROP). The values are
// documented in the C source (0..17). It is PT_TABSIZE by PT_TABSIZE.

const PT_TABSIZE_U: usize = PT_TABSIZE as usize;

static propposstab: [[u8; PT_TABSIZE_U]; PT_TABSIZE_U] = [
    /* LAMP GC  PC  SC  SCX ALNUM SPACE PXSPACE WORD CLIST UCNC BIDICL BOOL */
    [3, 0, 0, 0, 0, 3, 1, 1, 0, 0, 0, 0, 0], /* PT_LAMP */
    [0, 2, 4, 0, 0, 9, 10, 10, 11, 0, 0, 0, 0], /* PT_GC */
    [0, 5, 2, 0, 0, 15, 16, 16, 17, 0, 0, 0, 0], /* PT_PC */
    [0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0], /* PT_SC */
    [0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0], /* PT_SCX */
    [3, 6, 12, 0, 0, 3, 1, 1, 0, 0, 0, 0, 0], /* PT_ALNUM */
    [1, 7, 13, 0, 0, 1, 3, 3, 1, 0, 0, 0, 0], /* PT_SPACE */
    [1, 7, 13, 0, 0, 1, 3, 3, 1, 0, 0, 0, 0], /* PT_PXSPACE */
    [0, 8, 14, 0, 0, 0, 1, 1, 3, 0, 0, 0, 0], /* PT_WORD */
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], /* PT_CLIST */
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0], /* PT_UCNC */
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], /* PT_BIDICL */
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], /* PT_BOOL */
                                             /* PT_ANY does not need a record. */
];

// This table is used to check whether auto-possessification is possible between
// adjacent Unicode property opcodes (OP_PROP and OP_NOTPROP) when one specifies
// a general category and the other specifies a particular category. The row is
// selected by the general category and the column by the particular category.
// The value is 1 if the particular category is not part of the general
// category.

static catposstab: [[u8; 30]; 7] = [
    /* Cc Cf Cn Co Cs Ll Lm Lo Lt Lu Mc Me Mn Nd Nl No Pc Pd Pe Pf Pi Po Ps Sc Sk Sm So Zl Zp Zs */
    [0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], /* C */
    [1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], /* L */
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], /* M */
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], /* N */
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1], /* P */
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1], /* S */
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0], /* Z */
];

// This table is used when checking ALNUM, (PX)SPACE, SPACE, and WORD against a
// general or particular category. The properties in each row are those that
// apply to the character set in question.

static posspropstab: [[u8; 4]; 3] = [
    [ucp_L as u8, ucp_N as u8, ucp_N as u8, ucp_Nl as u8], /* ALNUM, 3rd and 4th values redundant */
    [ucp_Z as u8, ucp_Z as u8, ucp_C as u8, ucp_Cc as u8], /* SPACE and PXSPACE, 2nd value redundant */
    [ucp_L as u8, ucp_N as u8, ucp_P as u8, ucp_Po as u8], /* WORD */
];

// ---------------------------------------------------------------------------
//        Check a character and a property
// ---------------------------------------------------------------------------

/// This function is called by `compare_opcodes()` when a property item is
/// adjacent to a fixed character.
///
/// Arguments:
///   c        the character
///   ptype    the property type
///   pdata    the data for the type
///   negated  TRUE if it's a negated property (\P or \p{^)
///
/// Returns: TRUE if auto-possessifying is OK.
unsafe fn check_char_prop(c: u32, ptype: u32, pdata: u32, negated: BOOL) -> BOOL {
    unsafe {
        let negated_b = negated != 0;
        let prop = GET_UCD(c);

        match ptype as i64 {
            PT_LAMP => {
                let m = prop.chartype as u32 == ucp_Lu
                    || prop.chartype as u32 == ucp_Ll
                    || prop.chartype as u32 == ucp_Lt;
                return (m == negated_b) as BOOL;
            }

            PT_GC => {
                let m = pdata == tables::_pcre2_ucp_gentype[prop.chartype as usize];
                return (m == negated_b) as BOOL;
            }

            PT_PC => {
                let m = pdata == prop.chartype as u32;
                return (m == negated_b) as BOOL;
            }

            PT_SC => {
                let m = pdata == prop.script as u32;
                return (m == negated_b) as BOOL;
            }

            PT_SCX => {
                let ok = pdata == prop.script as u32
                    || MAPBIT(
                        tables::_pcre2_ucd_script_sets
                            .as_ptr()
                            .add(UCD_SCRIPTX_PROP(prop) as usize),
                        pdata,
                    ) != 0;
                return (ok == negated_b) as BOOL;
            }

            // These are specials.
            PT_ALNUM => {
                let m = tables::_pcre2_ucp_gentype[prop.chartype as usize] == ucp_L
                    || tables::_pcre2_ucp_gentype[prop.chartype as usize] == ucp_N;
                return (m == negated_b) as BOOL;
            }

            // Perl space used to exclude VT, but from Perl 5.18 it is included,
            // which means that Perl space and POSIX space are now identical.
            PT_SPACE | PT_PXSPACE => {
                let rc;
                if is_hspace(c) || is_vspace(c) {
                    rc = negated_b;
                } else {
                    rc = (tables::_pcre2_ucp_gentype[prop.chartype as usize] == ucp_Z) == negated_b;
                }
                return rc as BOOL;
            }

            PT_WORD => {
                let m = tables::_pcre2_ucp_gentype[prop.chartype as usize] == ucp_L
                    || tables::_pcre2_ucp_gentype[prop.chartype as usize] == ucp_N
                    || c == CHAR_UNDERSCORE;
                return (m == negated_b) as BOOL;
            }

            PT_CLIST => {
                let mut p = tables::_pcre2_ucd_caseless_sets
                    .as_ptr()
                    .add(prop.caseset as usize);
                loop {
                    if c < *p {
                        return (!negated_b) as BOOL;
                    }
                    let v = *p;
                    p = p.add(1);
                    if c == v {
                        return negated_b as BOOL;
                    }
                }
            }

            // Haven't yet thought these through.
            PT_BIDICL => FALSE,

            PT_BOOL => FALSE,

            _ => FALSE,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers replicating HSPACE_CASES / VSPACE_CASES.
// ---------------------------------------------------------------------------

#[inline]
fn is_hspace(c: u32) -> bool {
    matches!(
        c,
        CHAR_HT
            | CHAR_SPACE
            | CHAR_NBSP
            | 0x1680
            | 0x180e
            | 0x2000
            | 0x2001
            | 0x2002
            | 0x2003
            | 0x2004
            | 0x2005
            | 0x2006
            | 0x2007
            | 0x2008
            | 0x2009
            | 0x200a
            | 0x202f
            | 0x205f
            | 0x3000
    )
}

#[inline]
fn is_vspace(c: u32) -> bool {
    matches!(
        c,
        CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028 | 0x2029
    )
}

// ---------------------------------------------------------------------------
//        Base opcode of repeated opcodes
// ---------------------------------------------------------------------------

/// Returns the base opcode for repeated single character type opcodes. If the
/// opcode is not a repeated character type, it returns with the original value.
fn get_repeat_base(c: PCRE2_UCHAR) -> PCRE2_UCHAR {
    let cu = c as u32;
    if cu > OP_TYPEPOSUPTO {
        c
    } else if cu >= OP_TYPESTAR {
        OP_TYPESTAR as PCRE2_UCHAR
    } else if cu >= OP_NOTSTARI {
        OP_NOTSTARI as PCRE2_UCHAR
    } else if cu >= OP_NOTSTAR {
        OP_NOTSTAR as PCRE2_UCHAR
    } else if cu >= OP_STARI {
        OP_STARI as PCRE2_UCHAR
    } else {
        OP_STAR as PCRE2_UCHAR
    }
}

// ---------------------------------------------------------------------------
//        Fill the character property list
// ---------------------------------------------------------------------------

/// Checks whether the code points to an opcode that can take part in
/// auto-possessification, and if so, fills `list` with its properties.
///
/// Returns a pointer to the start of the next opcode if `*code` is accepted,
/// or NULL if `*code` is not accepted.
unsafe fn get_chr_property_list(
    code: PCRE2_SPTR,
    utf: BOOL,
    ucp: BOOL,
    fcc: *const u8,
    list: *mut u32,
) -> PCRE2_SPTR {
    unsafe {
        let mut code = code;
        let mut c = *code;
        let base: PCRE2_UCHAR;
        let mut end: PCRE2_SPTR;
        let class_end: PCRE2_SPTR;
        let chr: u32;

        *list.add(0) = c as u32;
        *list.add(1) = FALSE as u32;
        code = code.add(1);

        if c as u32 >= OP_STAR && c as u32 <= OP_TYPEPOSUPTO {
            base = get_repeat_base(c);
            c = (c as u32 - (base as u32 - OP_STAR)) as PCRE2_UCHAR;

            let cc = c as u32;
            if cc == OP_UPTO || cc == OP_MINUPTO || cc == OP_EXACT || cc == OP_POSUPTO {
                code = code.add(IMM2_SIZE_U);
            }

            *list.add(1) = (cc != OP_PLUS
                && cc != OP_MINPLUS
                && cc != OP_EXACT
                && cc != OP_POSPLUS) as u32;

            match base as u32 {
                OP_STAR => *list.add(0) = OP_CHAR,
                OP_STARI => *list.add(0) = OP_CHARI,
                OP_NOTSTAR => *list.add(0) = OP_NOT,
                OP_NOTSTARI => *list.add(0) = OP_NOTI,
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
                let mut p = code;
                let chr = GETCHARINCTEST(&mut p, utf != 0);
                code = p;
                *list.add(2) = chr;
                *list.add(3) = NOTACHAR_U32;
                return code;
            }

            OP_CHARI | OP_NOTI => {
                *list.add(0) = if c as u32 == OP_CHARI { OP_CHAR } else { OP_NOT };
                let mut p = code;
                let chr = GETCHARINCTEST(&mut p, utf != 0);
                code = p;
                *list.add(2) = chr;

                // SUPPORT_UNICODE branch.
                if chr < 128 || (chr < 256 && utf == 0 && ucp == 0) {
                    *list.add(3) = *fcc.add(chr as usize) as u32;
                } else {
                    *list.add(3) = UCD_OTHERCASE(chr);
                }

                // The othercase might be the same value.
                if chr == *list.add(3) {
                    *list.add(3) = NOTACHAR_U32;
                } else {
                    *list.add(4) = NOTACHAR_U32;
                }
                return code;
            }

            // SUPPORT_UNICODE
            OP_PROP | OP_NOTPROP => {
                if *code.add(0) as i64 != PT_CLIST {
                    *list.add(2) = *code.add(0) as u32;
                    *list.add(3) = *code.add(1) as u32;
                    return code.add(2);
                }

                // Convert only if we have enough space.
                let mut clist_src =
                    tables::_pcre2_ucd_caseless_sets.as_ptr().add(*code.add(1) as usize);
                let mut clist_dest = list.add(2);
                code = code.add(2);

                loop {
                    if clist_dest >= list.add(MAX_LIST) {
                        // Early return if there is not enough space.
                        *list.add(2) = *code.add(0) as u32;
                        *list.add(3) = *code.add(1) as u32;
                        return code;
                    }
                    *clist_dest = *clist_src;
                    clist_dest = clist_dest.add(1);
                    let v = *clist_src;
                    clist_src = clist_src.add(1);
                    if v == NOTACHAR_U32 {
                        break;
                    }
                }

                // All characters are stored. The terminating NOTACHAR is copied
                // from the clist itself.
                *list.add(0) = if c as u32 == OP_PROP { OP_CHAR } else { OP_NOT };
                return code;
            }

            OP_NCLASS | OP_CLASS | OP_XCLASS | OP_ECLASS => {
                // SUPPORT_WIDE_CHARS branch.
                if c as u32 == OP_XCLASS || c as u32 == OP_ECLASS {
                    end = code.add(GET(code, 0) as usize).offset(-1);
                } else {
                    end = code.add(32 / core::mem::size_of::<PCRE2_UCHAR>());
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
                        *list.add(1) = (GET2(end, 1) == 0) as u32;
                        end = end.add(1 + 2 * IMM2_SIZE_U);
                    }

                    _ => {}
                }
                *list.add(2) = end.offset_from(code) as u32;
                *list.add(3) = end.offset_from(class_end) as u32;
                return end;
            }

            _ => {}
        }

        ptr::null() // Opcode not accepted
    }
}

// ---------------------------------------------------------------------------
//    Scan further character sets for match
// ---------------------------------------------------------------------------

/// Checks whether the base and the current opcode have a common character, in
/// which case the base cannot be possessified.
///
/// Returns TRUE if the auto-possessification is possible.
unsafe fn compare_opcodes(
    code: PCRE2_SPTR,
    utf: BOOL,
    ucp: BOOL,
    cb: *const compile_block,
    base_list: *const u32,
    base_end: PCRE2_SPTR,
    rec_limit: *mut c_int,
) -> BOOL {
    unsafe {
        let mut code = code;
        let mut c: PCRE2_UCHAR;
        let mut list = [0u32; MAX_LIST];
        let mut chr_ptr: *const u32 = ptr::null();
        let mut ochr_ptr: *const u32;
        let mut list_ptr: *const u32 = ptr::null();
        let next_code: PCRE2_SPTR;
        let mut chr: u32;
        let mut accepted: BOOL;
        let mut invert_bits: BOOL;
        let mut entered_a_group: BOOL = FALSE;

        *rec_limit -= 1;
        if *rec_limit <= 0 {
            return FALSE; // Recursion has gone too deep
        }

        'outer: loop {
            let bracode: PCRE2_SPTR;

            // All operations move the code pointer forward. Therefore infinite
            // recursions are not possible.

            c = *code;

            // Skip over callouts.
            if c as u32 == OP_CALLOUT {
                code = code.add(tables::_pcre2_OP_lengths[c as usize] as usize);
                continue;
            }

            if c as u32 == OP_CALLOUT_STR {
                code = code.add(GET(code, 1 + 2 * LINK_SIZE_U) as usize);
                continue;
            }

            // At the end of a branch, skip to the end of the group and process
            // it.
            if c as u32 == OP_ALT {
                loop {
                    code = code.add(GET(code, 1) as usize);
                    if *code as u32 != OP_ALT {
                        break;
                    }
                }
                c = *code;
            }

            // Inspect the next opcode.
            match c as u32 {
                // We can always possessify a greedy iterator at the end of the
                // pattern, which is reached after skipping over the final
                // OP_KET. A non-greedy iterator must never be possessified.
                OP_END => return (*base_list.add(1) != 0) as BOOL,

                OP_KET | OP_KETRPOS => {
                    // The non-greedy case cannot be converted to a possessive
                    // form.
                    if *base_list.add(1) == 0 {
                        return FALSE;
                    }

                    // If the bracket is capturing it might be referenced by an
                    // OP_RECURSE so its last iterator can never be possessified
                    // if the pattern contains recursions.
                    bracode = code.offset(-(GET(code, 1) as isize));
                    match *bracode as u32 {
                        OP_CBRA | OP_SCBRA | OP_CBRAPOS | OP_SCBRAPOS => {
                            if (*cb).had_recurse != 0 {
                                return FALSE;
                            }
                        }

                        // A script run might have to backtrack if the iterated
                        // item can match characters from more than one script.
                        OP_SCRIPT_RUN => {
                            if *base_list.add(0) != OP_CHAR && *base_list.add(0) != OP_CHARI {
                                return FALSE;
                            }
                        }

                        // Atomic sub-patterns and forward assertions can always
                        // auto-possessify their last iterator, unless the group
                        // was entered as a result of checking a previous
                        // iterator.
                        OP_ASSERT | OP_ASSERT_NOT | OP_ONCE => {
                            return (entered_a_group == 0) as BOOL;
                        }

                        // Fixed-length lookbehinds can be treated the same way,
                        // but variable length lookbehinds must not
                        // auto-possessify their last iterator.
                        OP_ASSERTBACK | OP_ASSERTBACK_NOT => {
                            let mut bb = bracode;
                            loop {
                                if *bb.add(1 + LINK_SIZE_U) as u32 == OP_VREVERSE {
                                    return FALSE; // Variable
                                }
                                bb = bb.add(GET(bb, 1) as usize);
                                if *bb as u32 != OP_ALT {
                                    break;
                                }
                            }
                            return (entered_a_group == 0) as BOOL; // Not variable length
                        }

                        // Non-atomic assertions - don't possessify last
                        // iterator.
                        OP_ASSERT_NA | OP_ASSERTBACK_NA => return FALSE,

                        _ => {}
                    }

                    // Skip over the bracket and inspect what comes next.
                    code = code.add(tables::_pcre2_OP_lengths[c as usize] as usize);
                    continue;
                }

                // Handle cases where the next item is a group.
                OP_ONCE | OP_BRA | OP_CBRA => {
                    let mut next_c = code.add(GET(code, 1) as usize);
                    code = code.add(tables::_pcre2_OP_lengths[c as usize] as usize);

                    // Check each branch. We have to recurse a level for all but
                    // the last branch.
                    while *next_c as u32 == OP_ALT {
                        if compare_opcodes(code, utf, ucp, cb, base_list, base_end, rec_limit) == 0 {
                            return FALSE;
                        }
                        code = next_c.add(1 + LINK_SIZE_U);
                        next_c = next_c.add(GET(next_c, 1) as usize);
                    }

                    entered_a_group = TRUE;
                    continue;
                }

                OP_BRAZERO | OP_BRAMINZERO => {
                    let mut nc = code.add(1);
                    if *nc as u32 != OP_BRA && *nc as u32 != OP_CBRA && *nc as u32 != OP_ONCE {
                        return FALSE;
                    }

                    loop {
                        nc = nc.add(GET(nc, 1) as usize);
                        if *nc as u32 != OP_ALT {
                            break;
                        }
                    }

                    // The bracket content will be checked by the OP_BRA/OP_CBRA
                    // case above.
                    nc = nc.add(1 + LINK_SIZE_U);
                    if compare_opcodes(nc, utf, ucp, cb, base_list, base_end, rec_limit) == 0 {
                        return FALSE;
                    }

                    code = code.add(tables::_pcre2_OP_lengths[c as usize] as usize);
                    continue;
                }

                // The next opcode does not need special handling; fall through
                // and use it to see if the base can be possessified.
                _ => {}
            }

            // We now have the next appropriate opcode to compare with the base.
            // Check for a supported opcode, and load its properties.
            code = get_chr_property_list(code, utf, ucp, (*cb).fcc, list.as_mut_ptr());
            if code.is_null() {
                return FALSE; // Unsupported
            }

            // If either opcode is a small character list, set pointers for
            // comparing characters from that list with another list, or with a
            // property.
            if *base_list.add(0) == OP_CHAR {
                chr_ptr = base_list.add(2);
                list_ptr = list.as_ptr();
            } else if list[0] == OP_CHAR {
                chr_ptr = list.as_ptr().add(2);
                list_ptr = base_list;
            }
            // Character bitsets can also be compared to certain opcodes.
            // In 8 bit, non-UTF mode, OP_CLASS and OP_NCLASS are the same.
            else if *base_list.add(0) == OP_CLASS
                || list[0] == OP_CLASS
                || (utf == 0 && (*base_list.add(0) == OP_NCLASS || list[0] == OP_NCLASS))
            {
                let set1: *const u8;
                let set2: *const u8;
                let set_end: *const u8;

                if *base_list.add(0) == OP_CLASS || (utf == 0 && *base_list.add(0) == OP_NCLASS) {
                    set1 = base_end.offset(-(*base_list.add(2) as isize)) as *const u8;
                    list_ptr = list.as_ptr();
                } else {
                    set1 = code.offset(-(list[2] as isize)) as *const u8;
                    list_ptr = base_list;
                }

                invert_bits = FALSE;
                match *list_ptr.add(0) {
                    OP_CLASS | OP_NCLASS => {
                        let basep = if list_ptr == list.as_ptr() { code } else { base_end };
                        set2 = basep.offset(-(*list_ptr.add(2) as isize)) as *const u8;
                    }

                    // SUPPORT_WIDE_CHARS
                    OP_XCLASS => {
                        let basep = if list_ptr == list.as_ptr() { code } else { base_end };
                        let xclass_flags = basep
                            .offset(-(*list_ptr.add(2) as isize))
                            .add(LINK_SIZE_U);
                        if (*xclass_flags as i64 & XCL_HASPROP) != 0 {
                            return FALSE;
                        }
                        if (*xclass_flags as i64 & XCL_MAP) == 0 {
                            // No bits are set for characters < 256.
                            if list[1] == 0 {
                                return ((*xclass_flags as i64 & XCL_NOT) == 0) as BOOL;
                            }
                            // Might be an empty repeat.
                            continue;
                        }
                        set2 = xclass_flags.add(1);
                    }

                    OP_NOT_DIGIT => {
                        invert_bits = TRUE;
                        set2 = (*cb).cbits.add(cbit_digit as usize);
                    }
                    OP_DIGIT => {
                        set2 = (*cb).cbits.add(cbit_digit as usize);
                    }

                    OP_NOT_WHITESPACE => {
                        invert_bits = TRUE;
                        set2 = (*cb).cbits.add(cbit_space as usize);
                    }
                    OP_WHITESPACE => {
                        set2 = (*cb).cbits.add(cbit_space as usize);
                    }

                    OP_NOT_WORDCHAR => {
                        invert_bits = TRUE;
                        set2 = (*cb).cbits.add(cbit_word as usize);
                    }
                    OP_WORDCHAR => {
                        set2 = (*cb).cbits.add(cbit_word as usize);
                    }

                    _ => return FALSE,
                }

                // Because the bit sets are unaligned bytes, we need to perform
                // byte comparison here.
                let mut s1 = set1;
                let mut s2 = set2;
                set_end = set1.add(32);
                if invert_bits != 0 {
                    loop {
                        let a = *s1;
                        s1 = s1.add(1);
                        let b = *s2;
                        s2 = s2.add(1);
                        if (a & !b) != 0 {
                            return FALSE;
                        }
                        if s1 >= set_end {
                            break;
                        }
                    }
                } else {
                    loop {
                        let a = *s1;
                        s1 = s1.add(1);
                        let b = *s2;
                        s2 = s2.add(1);
                        if (a & b) != 0 {
                            return FALSE;
                        }
                        if s1 >= set_end {
                            break;
                        }
                    }
                }

                if list[1] == 0 {
                    return TRUE;
                }
                // Might be an empty repeat.
                continue;
            }
            // Some property combinations also acceptable. Unicode property
            // opcodes are processed specially; the rest can be handled with a
            // lookup table.
            else {
                let leftop: u32 = *base_list.add(0);
                let rightop: u32 = list[0];

                let mut accepted_v: BOOL = FALSE; // Always set in non-unicode case.

                if leftop == OP_PROP || leftop == OP_NOTPROP {
                    if rightop == OP_EOD {
                        accepted_v = TRUE;
                    } else if rightop == OP_PROP || rightop == OP_NOTPROP {
                        let n: u8;
                        let p: *const u8;
                        let same = leftop == rightop;
                        let lisprop = leftop == OP_PROP;
                        let risprop = rightop == OP_PROP;
                        let bothprop = lisprop && risprop;

                        n = propposstab[*base_list.add(2) as usize][list[2] as usize];
                        match n {
                            0 => {}
                            1 => accepted_v = bothprop as BOOL,
                            2 => {
                                accepted_v =
                                    ((*base_list.add(3) == list[3]) != same) as BOOL
                            }
                            3 => accepted_v = (!same) as BOOL,

                            4 => {
                                // Left general category, right particular category
                                accepted_v = (risprop
                                    && (catposstab[*base_list.add(3) as usize][list[3] as usize]
                                        != 0)
                                        == same) as BOOL;
                            }

                            5 => {
                                // Right general category, left particular category
                                accepted_v = (lisprop
                                    && (catposstab[list[3] as usize][*base_list.add(3) as usize]
                                        != 0)
                                        == same) as BOOL;
                            }

                            6 | 7 | 8 => {
                                // Left {alphanum,space,word} vs right general category
                                p = posspropstab[(n - 6) as usize].as_ptr();
                                accepted_v = (risprop
                                    && lisprop
                                        == (list[3] != *p.add(0) as u32
                                            && list[3] != *p.add(1) as u32
                                            && (list[3] != *p.add(2) as u32 || !lisprop)))
                                    as BOOL;
                            }

                            9 | 10 | 11 => {
                                // Right {alphanum,space,word} vs left general category
                                p = posspropstab[(n - 9) as usize].as_ptr();
                                accepted_v = (lisprop
                                    && risprop
                                        == (*base_list.add(3) != *p.add(0) as u32
                                            && *base_list.add(3) != *p.add(1) as u32
                                            && (*base_list.add(3) != *p.add(2) as u32 || !risprop)))
                                    as BOOL;
                            }

                            12 | 13 | 14 => {
                                // Left {alphanum,space,word} vs right particular category
                                p = posspropstab[(n - 12) as usize].as_ptr();
                                accepted_v = (risprop
                                    && lisprop
                                        == ((catposstab[*p.add(0) as usize][list[3] as usize] != 0)
                                            && (catposstab[*p.add(1) as usize][list[3] as usize]
                                                != 0)
                                            && (list[3] != *p.add(3) as u32 || !lisprop)))
                                    as BOOL;
                            }

                            15 | 16 | 17 => {
                                // Right {alphanum,space,word} vs left particular category
                                p = posspropstab[(n - 15) as usize].as_ptr();
                                accepted_v = (lisprop
                                    && risprop
                                        == ((catposstab[*p.add(0) as usize]
                                            [*base_list.add(3) as usize]
                                            != 0)
                                            && (catposstab[*p.add(1) as usize]
                                                [*base_list.add(3) as usize]
                                                != 0)
                                            && (*base_list.add(3) != *p.add(3) as u32 || !risprop)))
                                    as BOOL;
                            }

                            _ => {}
                        }
                    }

                    accepted = accepted_v;
                } else {
                    accepted = (leftop >= FIRST_AUTOTAB_OP as u32
                        && leftop <= LAST_AUTOTAB_LEFT_OP as u32
                        && rightop >= FIRST_AUTOTAB_OP as u32
                        && rightop <= LAST_AUTOTAB_RIGHT_OP as u32
                        && autoposstab[(leftop - FIRST_AUTOTAB_OP as u32) as usize]
                            [(rightop - FIRST_AUTOTAB_OP as u32) as usize]
                            != 0) as BOOL;
                }

                if accepted == 0 {
                    return FALSE;
                }

                if list[1] == 0 {
                    return TRUE;
                }
                // Might be an empty repeat.
                continue;
            }

            // Control reaches here only if one of the items is a small
            // character list. All characters are checked against the other
            // side.
            loop {
                chr = *chr_ptr;

                match *list_ptr.add(0) {
                    OP_CHAR => {
                        ochr_ptr = list_ptr.add(2);
                        loop {
                            if chr == *ochr_ptr {
                                return FALSE;
                            }
                            ochr_ptr = ochr_ptr.add(1);
                            if *ochr_ptr == NOTACHAR_U32 {
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
                            if *ochr_ptr == NOTACHAR_U32 {
                                break;
                            }
                        }
                        if *ochr_ptr == NOTACHAR_U32 {
                            return FALSE; // Not found
                        }
                    }

                    OP_DIGIT => {
                        if chr < 256 && (*(*cb).ctypes.add(chr as usize) as i64 & ctype_digit) != 0 {
                            return FALSE;
                        }
                    }

                    OP_NOT_DIGIT => {
                        if chr > 255 || (*(*cb).ctypes.add(chr as usize) as i64 & ctype_digit) == 0 {
                            return FALSE;
                        }
                    }

                    OP_WHITESPACE => {
                        if chr < 256 && (*(*cb).ctypes.add(chr as usize) as i64 & ctype_space) != 0 {
                            return FALSE;
                        }
                    }

                    OP_NOT_WHITESPACE => {
                        if chr > 255 || (*(*cb).ctypes.add(chr as usize) as i64 & ctype_space) == 0 {
                            return FALSE;
                        }
                    }

                    OP_WORDCHAR => {
                        if chr < 255 && (*(*cb).ctypes.add(chr as usize) as i64 & ctype_word) != 0 {
                            return FALSE;
                        }
                    }

                    OP_NOT_WORDCHAR => {
                        if chr > 255 || (*(*cb).ctypes.add(chr as usize) as i64 & ctype_word) == 0 {
                            return FALSE;
                        }
                    }

                    OP_HSPACE => {
                        if is_hspace(chr) {
                            return FALSE;
                        }
                    }

                    OP_NOT_HSPACE => {
                        if !is_hspace(chr) {
                            return FALSE;
                        }
                    }

                    OP_ANYNL | OP_VSPACE => {
                        if is_vspace(chr) {
                            return FALSE;
                        }
                    }

                    OP_NOT_VSPACE => {
                        if !is_vspace(chr) {
                            return FALSE;
                        }
                    }

                    OP_DOLL | OP_EODN => {
                        match chr {
                            CHAR_CR | CHAR_LF | CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                return FALSE;
                            }
                            _ => {}
                        }
                    }

                    OP_EOD => {
                        // Can always possessify before \z
                    }

                    // SUPPORT_UNICODE
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
                    }

                    OP_NCLASS => {
                        if chr > 255 {
                            return FALSE;
                        }
                        // Fall through to OP_CLASS behaviour.
                        let basep = if list_ptr == list.as_ptr() { code } else { base_end };
                        let class_bitset =
                            basep.offset(-(*list_ptr.add(2) as isize)) as *const u8;
                        if (*class_bitset.add((chr >> 3) as usize) & (1u8 << (chr & 7))) != 0 {
                            return FALSE;
                        }
                    }

                    OP_CLASS => {
                        if chr > 255 {
                            // break out of inner switch (no match here)
                        } else {
                            let basep = if list_ptr == list.as_ptr() { code } else { base_end };
                            let class_bitset =
                                basep.offset(-(*list_ptr.add(2) as isize)) as *const u8;
                            if (*class_bitset.add((chr >> 3) as usize) & (1u8 << (chr & 7))) != 0 {
                                return FALSE;
                            }
                        }
                    }

                    // SUPPORT_WIDE_CHARS
                    OP_XCLASS => {
                        let basep = if list_ptr == list.as_ptr() { code } else { base_end };
                        if priv_xclass(
                            chr,
                            basep.offset(-(*list_ptr.add(2) as isize)).add(LINK_SIZE_U),
                            (*cb).start_code as *const u8,
                            utf,
                        ) != 0
                        {
                            return FALSE;
                        }
                    }

                    OP_ECLASS => {
                        let basep = if list_ptr == list.as_ptr() { code } else { base_end };
                        if priv_eclass(
                            chr,
                            basep.offset(-(*list_ptr.add(2) as isize)).add(LINK_SIZE_U),
                            basep.offset(-(*list_ptr.add(3) as isize)),
                            (*cb).start_code as *const u8,
                            utf,
                        ) != 0
                        {
                            return FALSE;
                        }
                    }

                    _ => return FALSE,
                }

                chr_ptr = chr_ptr.add(1);
                if *chr_ptr == NOTACHAR_U32 {
                    break;
                }
            }

            // At least one character must be matched from this opcode.
            if list[1] == 0 {
                return TRUE;
            }

            // Loop back for another opcode (the `for(;;)` in C).
            let _ = &mut list; // keep list mutable across iterations
            continue 'outer;
        }
    }
}

// ---------------------------------------------------------------------------
//    Scan compiled regex for auto-possession
// ---------------------------------------------------------------------------

/// `PRIV(auto_possessify)` — replace single character iterations with their
/// possessive alternatives if appropriate. This function modifies the compiled
/// opcode!
///
/// Returns 0 for success, -1 if a non-existent opcode is encountered.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_auto_possessify_8(
    code: *mut PCRE2_UCHAR,
    cb: *const compile_block,
) -> c_int {
    unsafe {
        let mut code = code;
        let mut c: PCRE2_UCHAR;
        let mut end: PCRE2_SPTR;
        let mut repeat_opcode: *mut PCRE2_UCHAR;
        let mut list = [0u32; MAX_LIST];
        let mut rec_limit: c_int = 1000; // Was 10,000 but clang+ASAN uses a lot of stack.
        let utf: BOOL = ((*cb).external_options & PCRE2_UTF as u32 != 0) as BOOL;
        let ucp: BOOL = ((*cb).external_options & PCRE2_UCP as u32 != 0) as BOOL;

        loop {
            c = *code;

            if c as u32 >= OP_TABLE_LENGTH {
                return -1; // Something gone wrong
            }

            if c as u32 >= OP_STAR && c as u32 <= OP_TYPEPOSUPTO {
                c = (c as u32 - (get_repeat_base(c) as u32 - OP_STAR)) as PCRE2_UCHAR;
                let cc = c as u32;
                end = if cc <= OP_MINUPTO {
                    get_chr_property_list(code, utf, ucp, (*cb).fcc, list.as_mut_ptr())
                } else {
                    ptr::null()
                };
                list[1] =
                    (cc == OP_STAR || cc == OP_PLUS || cc == OP_QUERY || cc == OP_UPTO) as u32;

                if !end.is_null()
                    && compare_opcodes(
                        end,
                        utf,
                        ucp,
                        cb,
                        list.as_ptr(),
                        end,
                        &mut rec_limit,
                    ) != 0
                {
                    match cc {
                        OP_STAR => *code += (OP_POSSTAR - OP_STAR) as PCRE2_UCHAR,
                        OP_MINSTAR => *code += (OP_POSSTAR - OP_MINSTAR) as PCRE2_UCHAR,
                        OP_PLUS => *code += (OP_POSPLUS - OP_PLUS) as PCRE2_UCHAR,
                        OP_MINPLUS => *code += (OP_POSPLUS - OP_MINPLUS) as PCRE2_UCHAR,
                        OP_QUERY => *code += (OP_POSQUERY - OP_QUERY) as PCRE2_UCHAR,
                        OP_MINQUERY => *code += (OP_POSQUERY - OP_MINQUERY) as PCRE2_UCHAR,
                        OP_UPTO => *code += (OP_POSUPTO - OP_UPTO) as PCRE2_UCHAR,
                        OP_MINUPTO => *code += (OP_POSUPTO - OP_MINUPTO) as PCRE2_UCHAR,
                        _ => {}
                    }
                }
                c = *code;
            } else if c as u32 == OP_CLASS
                || c as u32 == OP_NCLASS
                || c as u32 == OP_XCLASS
                || c as u32 == OP_ECLASS
            {
                if c as u32 == OP_XCLASS || c as u32 == OP_ECLASS {
                    repeat_opcode = code.add(GET(code, 1) as usize);
                } else {
                    repeat_opcode = code.add(1 + (32 / core::mem::size_of::<PCRE2_UCHAR>()));
                }

                c = *repeat_opcode;
                if c as u32 >= OP_CRSTAR && c as u32 <= OP_CRMINRANGE {
                    end = get_chr_property_list(code, utf, ucp, (*cb).fcc, list.as_mut_ptr());
                    list[1] = ((c as u32 & 1) == 0) as u32;

                    if !end.is_null()
                        && compare_opcodes(
                            end,
                            utf,
                            ucp,
                            cb,
                            list.as_ptr(),
                            end,
                            &mut rec_limit,
                        ) != 0
                    {
                        match c as u32 {
                            OP_CRSTAR | OP_CRMINSTAR => *repeat_opcode = OP_CRPOSSTAR as PCRE2_UCHAR,
                            OP_CRPLUS | OP_CRMINPLUS => *repeat_opcode = OP_CRPOSPLUS as PCRE2_UCHAR,
                            OP_CRQUERY | OP_CRMINQUERY => {
                                *repeat_opcode = OP_CRPOSQUERY as PCRE2_UCHAR
                            }
                            OP_CRRANGE | OP_CRMINRANGE => {
                                *repeat_opcode = OP_CRPOSRANGE as PCRE2_UCHAR
                            }
                            _ => {}
                        }
                    }
                }
                c = *code;
            }

            match c as u32 {
                OP_END => return 0,

                OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEQUERY
                | OP_TYPEMINQUERY | OP_TYPEPOSSTAR | OP_TYPEPOSPLUS | OP_TYPEPOSQUERY => {
                    if *code.add(1) as u32 == OP_PROP || *code.add(1) as u32 == OP_NOTPROP {
                        code = code.add(2);
                    }
                }

                OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEEXACT | OP_TYPEPOSUPTO => {
                    if *code.add(1 + IMM2_SIZE_U) as u32 == OP_PROP
                        || *code.add(1 + IMM2_SIZE_U) as u32 == OP_NOTPROP
                    {
                        code = code.add(2);
                    }
                }

                OP_CALLOUT_STR => {
                    code = code.add(GET(code, 1 + 2 * LINK_SIZE_U) as usize);
                }

                // SUPPORT_WIDE_CHARS
                OP_XCLASS | OP_ECLASS => {
                    code = code.add(GET(code, 1) as usize);
                }

                OP_MARK | OP_COMMIT_ARG | OP_PRUNE_ARG | OP_SKIP_ARG | OP_THEN_ARG => {
                    code = code.add(*code.add(1) as usize);
                }

                _ => {}
            }

            // Add in the fixed length from the table.
            code = code.add(tables::_pcre2_OP_lengths[c as usize] as usize);

            // In UTF-8 mode, opcodes that are followed by a character may be
            // followed by a multi-byte character. The length in the table is a
            // minimum, so we have to arrange to skip the extra code units.
            // (MAYBE_UTF_MULTI is defined in 8-bit UTF mode.)
            if utf != 0 {
                match c as u32 {
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
                        if HAS_EXTRALEN(*code.offset(-1) as u32) {
                            code = code.add(GET_EXTRALEN(*code.offset(-1) as u32) as usize);
                        }
                    }

                    _ => {}
                }
            }
        }
    }
}
