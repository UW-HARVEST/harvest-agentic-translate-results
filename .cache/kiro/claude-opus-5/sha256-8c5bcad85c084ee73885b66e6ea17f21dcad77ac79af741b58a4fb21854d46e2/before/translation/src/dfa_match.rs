//! Translation of `c_src/src/pcre2_dfa_match.c`.
//!
//! The external function `pcre2_dfa_match()`, an alternative matching function
//! that uses a sort of DFA algorithm (not a true FSM). This is NOT
//! Perl-compatible, but it has advantages in certain applications.
//!
//! Build configuration: `PCRE2_CODE_UNIT_WIDTH == 8`, `SUPPORT_UNICODE`
//! (therefore `SUPPORT_WIDE_CHARS`), no JIT, no EBCDIC, no `PCRE2_DEBUG`,
//! `LINK_SIZE == 2`, `DFA_START_RWS_SIZE == 30720`.

#![allow(
    non_snake_case,
    non_upper_case_globals,
    non_camel_case_types,
    unused_parens,
    unused_assignments,
    unused_variables,
    dead_code
)]

use core::ffi::{c_int, c_void};

use crate::chars::*;
use crate::extuni::extuni;
use crate::internal::*;
use crate::newline::{is_newline, was_newline};
use crate::opcodes::*;
use crate::ucp::*;
use crate::valid_utf::valid_utf;
use crate::xclass::{eclass, xclass};

/* The public options that are permitted for pcre2_dfa_match(). */

const PUBLIC_DFA_MATCH_OPTIONS: u32 = PCRE2_ANCHORED
    | PCRE2_ENDANCHORED
    | PCRE2_NOTBOL
    | PCRE2_NOTEOL
    | PCRE2_NOTEMPTY
    | PCRE2_NOTEMPTY_ATSTART
    | PCRE2_NO_UTF_CHECK
    | PCRE2_PARTIAL_HARD
    | PCRE2_PARTIAL_SOFT
    | PCRE2_DFA_SHORTEST
    | PCRE2_DFA_RESTART
    | PCRE2_COPY_MATCHED_SUBJECT;

/* These are offsets that are used to turn the OP_TYPESTAR and friends opcodes
into others, under special conditions. A gap of 20 between the blocks should be
enough. The resulting opcodes don't have to be less than 256 because they are
never stored, so we push them well clear of the normal opcodes. Represented
here as u32 because they exceed the u8 opcode range. */

const OP_PROP_EXTRA: u32 = 300;
const OP_EXTUNI_EXTRA: u32 = 320;
const OP_ANYNL_EXTRA: u32 = 340;
const OP_HSPACE_EXTRA: u32 = 360;
const OP_VSPACE_EXTRA: u32 = 380;

const UINT32_MAX: u32 = u32::MAX;

/* This table identifies those opcodes that are followed immediately by a
character that is to be tested in some way. Non-zero values are the offsets
from the opcode where the character is to be found. */

#[rustfmt::skip]
static coptable: [u8; OP_TABLE_LENGTH as usize] = [
  0,                             /* End                                    */
  0, 0, 0, 0, 0,                 /* \A, \G, \K, \B, \b                     */
  0, 0, 0, 0, 0, 0,              /* \D, \d, \S, \s, \W, \w                 */
  0, 0, 0,                       /* Any, AllAny, Anybyte                   */
  0, 0,                          /* \P, \p                                 */
  0, 0, 0, 0, 0,                 /* \R, \H, \h, \V, \v                     */
  0,                             /* \X                                     */
  0, 0, 0, 0, 0, 0,              /* \Z, \z, $, $M, ^, ^M                   */
  1,                             /* Char                                   */
  1,                             /* Chari                                  */
  1,                             /* not                                    */
  1,                             /* noti                                   */
  /* Positive single-char repeats                                          */
  1, 1, 1, 1, 1, 1,              /* *, *?, +, +?, ?, ??                    */
  1+IMM2_SIZE as u8, 1+IMM2_SIZE as u8, /* upto, minupto                   */
  1+IMM2_SIZE as u8,             /* exact                                  */
  1, 1, 1, 1+IMM2_SIZE as u8,    /* *+, ++, ?+, upto+                      */
  1, 1, 1, 1, 1, 1,              /* *I, *?I, +I, +?I, ?I, ??I              */
  1+IMM2_SIZE as u8, 1+IMM2_SIZE as u8, /* upto I, minupto I               */
  1+IMM2_SIZE as u8,             /* exact I                                */
  1, 1, 1, 1+IMM2_SIZE as u8,    /* *+I, ++I, ?+I, upto+I                  */
  /* Negative single-char repeats - only for chars < 256                   */
  1, 1, 1, 1, 1, 1,              /* NOT *, *?, +, +?, ?, ??                */
  1+IMM2_SIZE as u8, 1+IMM2_SIZE as u8, /* NOT upto, minupto               */
  1+IMM2_SIZE as u8,             /* NOT exact                              */
  1, 1, 1, 1+IMM2_SIZE as u8,    /* NOT *+, ++, ?+, upto+                  */
  1, 1, 1, 1, 1, 1,              /* NOT *I, *?I, +I, +?I, ?I, ??I          */
  1+IMM2_SIZE as u8, 1+IMM2_SIZE as u8, /* NOT upto I, minupto I           */
  1+IMM2_SIZE as u8,             /* NOT exact I                            */
  1, 1, 1, 1+IMM2_SIZE as u8,    /* NOT *+I, ++I, ?+I, upto+I              */
  /* Positive type repeats                                                 */
  1, 1, 1, 1, 1, 1,              /* Type *, *?, +, +?, ?, ??               */
  1+IMM2_SIZE as u8, 1+IMM2_SIZE as u8, /* Type upto, minupto              */
  1+IMM2_SIZE as u8,             /* Type exact                             */
  1, 1, 1, 1+IMM2_SIZE as u8,    /* Type *+, ++, ?+, upto+                 */
  /* Character class & ref repeats                                         */
  0, 0, 0, 0, 0, 0,              /* *, *?, +, +?, ?, ??                    */
  0, 0,                          /* CRRANGE, CRMINRANGE                    */
  0, 0, 0, 0,                    /* Possessive *+, ++, ?+, CRPOSRANGE      */
  0,                             /* CLASS                                  */
  0,                             /* NCLASS                                 */
  0,                             /* XCLASS - variable length               */
  0,                             /* ECLASS - variable length               */
  0,                             /* REF                                    */
  0,                             /* REFI                                   */
  0,                             /* DNREF                                  */
  0,                             /* DNREFI                                 */
  0,                             /* RECURSE                                */
  0,                             /* CALLOUT                                */
  0,                             /* CALLOUT_STR                            */
  0,                             /* Alt                                    */
  0,                             /* Ket                                    */
  0,                             /* KetRmax                                */
  0,                             /* KetRmin                                */
  0,                             /* KetRpos                                */
  0, 0,                          /* Reverse, Vreverse                      */
  0,                             /* Assert                                 */
  0,                             /* Assert not                             */
  0,                             /* Assert behind                          */
  0,                             /* Assert behind not                      */
  0,                             /* NA assert                              */
  0,                             /* NA assert behind                       */
  0,                             /* Assert scan substring                  */
  0,                             /* ONCE                                   */
  0,                             /* SCRIPT_RUN                             */
  0, 0, 0, 0, 0,                 /* BRA, BRAPOS, CBRA, CBRAPOS, COND       */
  0, 0, 0, 0, 0,                 /* SBRA, SBRAPOS, SCBRA, SCBRAPOS, SCOND  */
  0, 0,                          /* CREF, DNCREF                           */
  0, 0,                          /* RREF, DNRREF                           */
  0, 0,                          /* FALSE, TRUE                            */
  0, 0, 0,                       /* BRAZERO, BRAMINZERO, BRAPOSZERO        */
  0, 0, 0,                       /* MARK, PRUNE, PRUNE_ARG                 */
  0, 0, 0, 0,                    /* SKIP, SKIP_ARG, THEN, THEN_ARG         */
  0, 0,                          /* COMMIT, COMMIT_ARG                     */
  0, 0, 0,                       /* FAIL, ACCEPT, ASSERT_ACCEPT            */
  0, 0, 0,                       /* CLOSE, SKIPZERO, DEFINE                */
  0, 0,                          /* \B and \b in UCP mode                  */
];

/* This table identifies those opcodes that inspect a character. */

#[rustfmt::skip]
static poptable: [u8; OP_TABLE_LENGTH as usize] = [
  0,                             /* End                                    */
  0, 0, 0, 1, 1,                 /* \A, \G, \K, \B, \b                     */
  1, 1, 1, 1, 1, 1,              /* \D, \d, \S, \s, \W, \w                 */
  1, 1, 1,                       /* Any, AllAny, Anybyte                   */
  1, 1,                          /* \P, \p                                 */
  1, 1, 1, 1, 1,                 /* \R, \H, \h, \V, \v                     */
  1,                             /* \X                                     */
  0, 0, 0, 0, 0, 0,              /* \Z, \z, $, $M, ^, ^M                   */
  1,                             /* Char                                   */
  1,                             /* Chari                                  */
  1,                             /* not                                    */
  1,                             /* noti                                   */
  /* Positive single-char repeats                                          */
  1, 1, 1, 1, 1, 1,              /* *, *?, +, +?, ?, ??                    */
  1, 1, 1,                       /* upto, minupto, exact                   */
  1, 1, 1, 1,                    /* *+, ++, ?+, upto+                      */
  1, 1, 1, 1, 1, 1,              /* *I, *?I, +I, +?I, ?I, ??I              */
  1, 1, 1,                       /* upto I, minupto I, exact I             */
  1, 1, 1, 1,                    /* *+I, ++I, ?+I, upto+I                  */
  /* Negative single-char repeats - only for chars < 256                   */
  1, 1, 1, 1, 1, 1,              /* NOT *, *?, +, +?, ?, ??                */
  1, 1, 1,                       /* NOT upto, minupto, exact               */
  1, 1, 1, 1,                    /* NOT *+, ++, ?+, upto+                  */
  1, 1, 1, 1, 1, 1,              /* NOT *I, *?I, +I, +?I, ?I, ??I          */
  1, 1, 1,                       /* NOT upto I, minupto I, exact I         */
  1, 1, 1, 1,                    /* NOT *+I, ++I, ?+I, upto+I              */
  /* Positive type repeats                                                 */
  1, 1, 1, 1, 1, 1,              /* Type *, *?, +, +?, ?, ??               */
  1, 1, 1,                       /* Type upto, minupto, exact              */
  1, 1, 1, 1,                    /* Type *+, ++, ?+, upto+                 */
  /* Character class & ref repeats                                         */
  1, 1, 1, 1, 1, 1,              /* *, *?, +, +?, ?, ??                    */
  1, 1,                          /* CRRANGE, CRMINRANGE                    */
  1, 1, 1, 1,                    /* Possessive *+, ++, ?+, CRPOSRANGE      */
  1,                             /* CLASS                                  */
  1,                             /* NCLASS                                 */
  1,                             /* XCLASS - variable length               */
  1,                             /* ECLASS - variable length               */
  0,                             /* REF                                    */
  0,                             /* REFI                                   */
  0,                             /* DNREF                                  */
  0,                             /* DNREFI                                 */
  0,                             /* RECURSE                                */
  0,                             /* CALLOUT                                */
  0,                             /* CALLOUT_STR                            */
  0,                             /* Alt                                    */
  0,                             /* Ket                                    */
  0,                             /* KetRmax                                */
  0,                             /* KetRmin                                */
  0,                             /* KetRpos                                */
  0, 0,                          /* Reverse, Vreverse                      */
  0,                             /* Assert                                 */
  0,                             /* Assert not                             */
  0,                             /* Assert behind                          */
  0,                             /* Assert behind not                      */
  0,                             /* NA assert                              */
  0,                             /* NA assert behind                       */
  0,                             /* Assert scan substring                  */
  0,                             /* ONCE                                   */
  0,                             /* SCRIPT_RUN                             */
  0, 0, 0, 0, 0,                 /* BRA, BRAPOS, CBRA, CBRAPOS, COND       */
  0, 0, 0, 0, 0,                 /* SBRA, SBRAPOS, SCBRA, SCBRAPOS, SCOND  */
  0, 0,                          /* CREF, DNCREF                           */
  0, 0,                          /* RREF, DNRREF                           */
  0, 0,                          /* FALSE, TRUE                            */
  0, 0, 0,                       /* BRAZERO, BRAMINZERO, BRAPOSZERO        */
  0, 0, 0,                       /* MARK, PRUNE, PRUNE_ARG                 */
  0, 0, 0, 0,                    /* SKIP, SKIP_ARG, THEN, THEN_ARG         */
  0, 0,                          /* COMMIT, COMMIT_ARG                     */
  0, 0, 0,                       /* FAIL, ACCEPT, ASSERT_ACCEPT            */
  0, 0, 0,                       /* CLOSE, SKIPZERO, DEFINE                */
  1, 1,                          /* \B and \b in UCP mode                  */
];

/* These 2 tables allow for compact code for testing for \D, \d, \S, \s, \W,
and \w */

#[rustfmt::skip]
static toptable1: [u8; 14] = [
  0, 0, 0, 0, 0, 0,
  ctype_digit, ctype_digit,
  ctype_space, ctype_space,
  ctype_word,  ctype_word,
  0, 0,                           /* OP_ANY, OP_ALLANY */
];

#[rustfmt::skip]
static toptable2: [u8; 14] = [
  0, 0, 0, 0, 0, 0,
  ctype_digit, 0,
  ctype_space, 0,
  ctype_word,  0,
  1, 1,                           /* OP_ANY, OP_ALLANY */
];

/* Structure for holding data about a particular state, which is in effect the
current data for an active path through the match tree. It must consist
entirely of ints because the working vector we are passed, and which we put
these structures in, is a vector of ints. */

#[repr(C)]
#[derive(Clone, Copy)]
struct stateblock {
    offset: c_int, /* Offset to opcode (-ve has meaning) */
    count: c_int,  /* Count for repeats */
    data: c_int,   /* Some use extra data */
}

const INTS_PER_STATEBLOCK: c_int =
    (core::mem::size_of::<stateblock>() / core::mem::size_of::<c_int>()) as c_int;

/* sizeof(PCRE2_SIZE)/sizeof(int) */
const OVEC_UNIT: usize = core::mem::size_of::<PCRE2_SIZE>() / core::mem::size_of::<c_int>();

const RWS_BASE_SIZE: usize = DFA_START_RWS_SIZE / core::mem::size_of::<c_int>();
const RWS_RSIZE: usize = 1000; /* Work size for recursion */
const RWS_OVEC_RSIZE: usize = 1000 * OVEC_UNIT; /* Ovector for recursion */
const RWS_OVEC_OSIZE: usize = 2 * OVEC_UNIT; /* Ovector in other cases */

/* This structure is at the start of each workspace block. */

#[repr(C)]
struct RWS_anchor {
    next: *mut RWS_anchor,
    size: u32, /* Number of ints */
    free: u32, /* Number of ints */
}

const RWS_ANCHOR_SIZE: usize =
    core::mem::size_of::<RWS_anchor>() / core::mem::size_of::<c_int>();

/* IS_NEWLINE / WAS_NEWLINE / ACROSSCHAR helpers. NLBLOCK is `mb`; PSSTART is
`start_subject`; PSEND is `end_subject`. These mirror the macros in
pcre2_internal.h / pcre2_intmodedep.h. */

#[inline]
unsafe fn is_newline_at(mb: *mut dfa_match_block, p: PCRE2_SPTR, utf: BOOL) -> bool {
    unsafe {
        if (*mb).nltype != NLTYPE_FIXED {
            p < (*mb).end_subject
                && is_newline(p, (*mb).nltype, (*mb).end_subject, &mut (*mb).nllen, utf) != 0
        } else {
            p <= (*mb).end_subject.sub((*mb).nllen as usize)
                && *p as u32 == (*mb).nl[0] as u32
                && ((*mb).nllen == 1 || *p.add(1) as u32 == (*mb).nl[1] as u32)
        }
    }
}

#[inline]
unsafe fn was_newline_at(mb: *mut dfa_match_block, p: PCRE2_SPTR, utf: BOOL) -> bool {
    unsafe {
        if (*mb).nltype != NLTYPE_FIXED {
            p > (*mb).start_subject
                && was_newline(p, (*mb).nltype, (*mb).start_subject, &mut (*mb).nllen, utf) != 0
        } else {
            p >= (*mb).start_subject.add((*mb).nllen as usize)
                && *p.sub((*mb).nllen as usize) as u32 == (*mb).nl[0] as u32
                && ((*mb).nllen == 1
                    || *p.sub((*mb).nllen as usize).add(1) as u32 == (*mb).nl[1] as u32)
        }
    }
}

/* ACROSSCHAR(condition, eptr, eptr = ...): while condition and eptr is a
continuation byte, advance. In 8-bit UTF-8 mode. */
#[inline]
unsafe fn acrosschar_fwd(eptr: &mut PCRE2_SPTR, end: PCRE2_SPTR) {
    unsafe {
        while *eptr < end && (**eptr & 0xc0) == 0x80 {
            *eptr = eptr.add(1);
        }
    }
}

/*************************************************
*               Process a callout                *
*************************************************/

/* This function is called to perform a callout. Returns the return from the
callout. */

unsafe fn do_callout_dfa(
    code: PCRE2_SPTR,
    offsets: *mut PCRE2_SIZE,
    current_subject: PCRE2_SPTR,
    ptr: PCRE2_SPTR,
    mb: *mut dfa_match_block,
    extracode: PCRE2_SIZE,
    lengthptr: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
        let cb: *mut pcre2_callout_block = (*mb).cb;

        *lengthptr = if *code.add(extracode) == OP_CALLOUT {
            OP_LENGTHS[OP_CALLOUT as usize] as PCRE2_SIZE
        } else {
            get(code, 1 + 2 * LINK_SIZE + extracode) as PCRE2_SIZE
        };

        if (*mb).callout.is_none() {
            return 0; /* No callout provided */
        }

        /* Fixed fields in the callout block are set once and for all at the start
        of matching. */

        (*cb).offset_vector = offsets;
        (*cb).start_match = current_subject.offset_from((*mb).start_subject) as PCRE2_SIZE;
        (*cb).current_position = ptr.offset_from((*mb).start_subject) as PCRE2_SIZE;
        (*cb).pattern_position = get(code, 1 + extracode) as PCRE2_SIZE;
        (*cb).next_item_length = get(code, 1 + LINK_SIZE + extracode) as PCRE2_SIZE;

        if *code.add(extracode) == OP_CALLOUT {
            (*cb).callout_number = *code.add(1 + 2 * LINK_SIZE + extracode) as u32;
            (*cb).callout_string_offset = 0;
            (*cb).callout_string = core::ptr::null();
            (*cb).callout_string_length = 0;
        } else {
            (*cb).callout_number = 0;
            (*cb).callout_string_offset = get(code, 1 + 3 * LINK_SIZE + extracode) as PCRE2_SIZE;
            (*cb).callout_string = code.add((1 + 4 * LINK_SIZE + extracode) + 1);
            (*cb).callout_string_length = *lengthptr - (1 + 4 * LINK_SIZE) - 2;
        }

        ((*mb).callout.unwrap())(cb, (*mb).callout_data)
    }
}

/*************************************************
*         Expand local workspace memory          *
*************************************************/

/* This function is called when internal_dfa_match() is about to be called
recursively and there is insufficient working space left in the current
workspace block.

Returns:     0 rwsptr has been updated
            !0 an error code
*/

unsafe fn more_workspace(
    rwsptr: *mut *mut RWS_anchor,
    ovecsize: u32,
    mb: *mut dfa_match_block,
) -> c_int {
    unsafe {
        let rws: *mut RWS_anchor = *rwsptr;
        let new_: *mut RWS_anchor;

        if !(*rws).next.is_null() {
            new_ = (*rws).next;
        } else {
            /* Sizes in the RWS_anchor blocks are in units of sizeof(int), but
            mb->heap_limit and mb->heap_used are in kibibytes. Play carefully, to
            avoid overflow. */
            let mut newsize: u32 =
                if (*rws).size >= UINT32_MAX / (core::mem::size_of::<c_int>() as u32 * 2) {
                    UINT32_MAX / core::mem::size_of::<c_int>() as u32
                } else {
                    (*rws).size * 2
                };
            let mut newsizeK: u32 = newsize / (1024 / core::mem::size_of::<c_int>() as u32);

            if (newsizeK as PCRE2_SIZE) + (*mb).heap_used > (*mb).heap_limit as PCRE2_SIZE {
                newsizeK = ((*mb).heap_limit as PCRE2_SIZE - (*mb).heap_used) as u32;
            }
            newsize = newsizeK * (1024 / core::mem::size_of::<c_int>() as u32);

            if (newsize as usize) < RWS_RSIZE + ovecsize as usize + RWS_ANCHOR_SIZE {
                return PCRE2_ERROR_HEAPLIMIT;
            }
            let block = ((*mb).memctl.malloc.unwrap())(
                newsize as usize * core::mem::size_of::<c_int>(),
                (*mb).memctl.memory_data,
            ) as *mut RWS_anchor;
            if block.is_null() {
                return PCRE2_ERROR_NOMEMORY;
            }
            new_ = block;
            (*mb).heap_used += newsizeK as PCRE2_SIZE;
            (*new_).next = core::ptr::null_mut();
            (*new_).size = newsize;
            (*rws).next = new_;
        }

        (*new_).free = (*new_).size - RWS_ANCHOR_SIZE as u32;
        *rwsptr = new_;
        0
    }
}

/* Shared implementation of the Unicode property test used by OP_PROP/OP_NOTPROP
and the OP_*_EXTRA type-repeat variants. `proptype` and `propval` correspond to
the two code bytes read in the C switch. Returns the `OK` boolean. The final
`default` arm mirrors `OK = codevalue != OP_PROP`; the caller passes
`codevalue_is_prop = (codevalue == OP_PROP)`. */
#[inline]
unsafe fn dfa_prop_ok(c: u32, proptype: u8, propval: u8, codevalue_is_prop: bool) -> bool {
    unsafe {
        let prop: &UcdRecord = get_ucd(c);
        match proptype as u32 {
            PT_LAMP => {
                let chartype = prop.chartype as u32;
                chartype == ucp_Lu || chartype == ucp_Ll || chartype == ucp_Lt
            }
            PT_GC => UCP_GENTYPE[prop.chartype as usize] == propval as u32,
            PT_PC => prop.chartype == propval,
            PT_SC => prop.script == propval,
            PT_SCX => {
                prop.script == propval
                    || mapbit(
                        &UCD_SCRIPT_SETS[ucd_scriptx_prop(prop) as usize..],
                        propval as u32,
                    ) != 0
            }
            PT_ALNUM => {
                let chartype = prop.chartype as usize;
                UCP_GENTYPE[chartype] == ucp_L || UCP_GENTYPE[chartype] == ucp_N
            }
            /* Perl space and POSIX space are identical since Perl 5.18. */
            PT_SPACE | PT_PXSPACE => match c {
                CHAR_HT | CHAR_SPACE | CHAR_NBSP | 0x1680 | 0x180e | 0x2000 | 0x2001
                | 0x2002 | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009
                | 0x200a | 0x202f | 0x205f | 0x3000 | CHAR_LF | CHAR_VT | CHAR_FF
                | CHAR_CR | CHAR_NEL | 0x2028 | 0x2029 => true,
                _ => UCP_GENTYPE[prop.chartype as usize] == ucp_Z,
            },
            PT_WORD => {
                let chartype = prop.chartype as u32;
                UCP_GENTYPE[prop.chartype as usize] == ucp_L
                    || UCP_GENTYPE[prop.chartype as usize] == ucp_N
                    || chartype == ucp_Mn
                    || chartype == ucp_Pc
            }
            PT_CLIST => {
                let mut cp = UCD_CASELESS_SETS.as_ptr().add(propval as usize);
                loop {
                    if c < *cp {
                        break false;
                    }
                    let v = *cp;
                    cp = cp.add(1);
                    if c == v {
                        break true;
                    }
                }
            }
            PT_UCNC => {
                c == CHAR_DOLLAR_SIGN
                    || c == CHAR_COMMERCIAL_AT
                    || c == CHAR_GRAVE_ACCENT
                    || (c >= 0xa0 && c <= 0xd7ff)
                    || c >= 0xe000
            }
            PT_BIDICL => ucd_bidiclass(c) == propval as u32,
            PT_BOOL => {
                mapbit(
                    &UCD_BOOLPROP_SETS[ucd_bprops_prop(prop) as usize..],
                    propval as u32,
                ) != 0
            }
            /* Should never occur, but keep compilers from grumbling. */
            _ => !codevalue_is_prop,
        }
    }
}

/*************************************************
*     Match a Regular Expression - DFA engine    *
*************************************************/

/* This internal function applies a compiled pattern to a subject string,
starting at a given point, using a DFA engine. This function is called from the
external one, possibly multiple times if the pattern is not anchored. The
function calls itself recursively for some kinds of subpattern.

Returns:            > 0 => number of match offset pairs placed in offsets
                    = 0 => offsets overflowed; longest matches are present
                     -1 => failed to match
                   < -1 => some kind of unexpected problem
*/

unsafe fn internal_dfa_match(
    mb: *mut dfa_match_block,
    this_start_code: PCRE2_SPTR,
    mut current_subject: PCRE2_SPTR,
    start_offset: PCRE2_SIZE,
    offsets: *mut PCRE2_SIZE,
    mut offsetcount: u32,
    workspace: *mut c_int,
    mut wscount: c_int,
    mut rlevel: u32,
    mut RWS: *mut c_int,
) -> c_int {
    unsafe {
        let mut active_states: *mut stateblock;
        let mut new_states: *mut stateblock;
        let mut temp_states: *mut stateblock;
        let mut next_active_state: *mut stateblock;
        let mut next_new_state: *mut stateblock;
        let ctypes: *const u8;
        let lcc: *const u8;
        let fcc: *const u8;
        let mut ptr: PCRE2_SPTR;
        let mut end_code: PCRE2_SPTR;
        let mut new_recursive: dfa_recursion_info = core::mem::zeroed();
        let mut active_count: c_int;
        let mut new_count: c_int;
        let mut match_count: c_int;

        /* Some fields in the mb block are frequently referenced, so we load them
        into independent variables in the hope that this will perform better. */

        let start_subject: PCRE2_SPTR = (*mb).start_subject;
        let end_subject: PCRE2_SPTR = (*mb).end_subject;
        let start_code: PCRE2_SPTR = (*mb).start_code;

        let utf: BOOL = (((*mb).poptions & PCRE2_UTF) != 0) as BOOL;
        let utf_or_ucp: BOOL =
            (utf != 0 || ((*mb).poptions & PCRE2_UCP) != 0) as BOOL;

        let mut reset_could_continue: BOOL = FALSE;

        (*mb).match_call_count += 1;
        if (*mb).match_call_count - 1 >= (*mb).match_limit {
            return PCRE2_ERROR_MATCHLIMIT;
        }
        let rl = rlevel;
        rlevel += 1;
        if rl > (*mb).match_limit_depth {
            return PCRE2_ERROR_DEPTHLIMIT;
        }
        offsetcount &= (-2i32) as u32; /* Round down */

        wscount -= 2;
        wscount = (wscount - (wscount % (INTS_PER_STATEBLOCK * 2)))
            / (2 * INTS_PER_STATEBLOCK);

        ctypes = (*mb).tables.add(ctypes_offset);
        lcc = (*mb).tables.add(lcc_offset);
        fcc = (*mb).tables.add(fcc_offset);

        match_count = PCRE2_ERROR_NOMATCH; /* A negative number */

        active_states = (workspace.add(2)) as *mut stateblock;
        new_states = active_states.add(wscount as usize);
        next_new_state = new_states;
        new_count = 0;

        /* The ADD_ACTIVE / ADD_NEW macros. In C these `return` on overflow; here
        they are implemented as local macros that early-return the enclosing
        function. */

        macro_rules! ADD_ACTIVE {
            ($x:expr, $y:expr) => {{
                if active_count < wscount {
                    active_count += 1;
                    (*next_active_state).offset = ($x);
                    (*next_active_state).count = ($y);
                    next_active_state = next_active_state.add(1);
                } else {
                    return PCRE2_ERROR_DFA_WSSIZE;
                }
            }};
        }
        macro_rules! ADD_ACTIVE_DATA {
            ($x:expr, $y:expr, $z:expr) => {{
                if active_count < wscount {
                    active_count += 1;
                    (*next_active_state).offset = ($x);
                    (*next_active_state).count = ($y);
                    (*next_active_state).data = ($z);
                    next_active_state = next_active_state.add(1);
                } else {
                    return PCRE2_ERROR_DFA_WSSIZE;
                }
            }};
        }
        macro_rules! ADD_NEW {
            ($x:expr, $y:expr) => {{
                if new_count < wscount {
                    new_count += 1;
                    (*next_new_state).offset = ($x);
                    (*next_new_state).count = ($y);
                    next_new_state = next_new_state.add(1);
                } else {
                    return PCRE2_ERROR_DFA_WSSIZE;
                }
            }};
        }
        macro_rules! ADD_NEW_DATA {
            ($x:expr, $y:expr, $z:expr) => {{
                if new_count < wscount {
                    new_count += 1;
                    (*next_new_state).offset = ($x);
                    (*next_new_state).count = ($y);
                    (*next_new_state).data = ($z);
                    next_new_state = next_new_state.add(1);
                } else {
                    return PCRE2_ERROR_DFA_WSSIZE;
                }
            }};
        }

        /* The first thing in any (sub) pattern is a bracket of some sort. Push all
        the alternative states onto the list, and find out where the end is. */

        if *this_start_code == OP_ASSERTBACK || *this_start_code == OP_ASSERTBACK_NOT {
            let mut max_back: usize = 0;
            let mut gone_back: usize;

            end_code = this_start_code;
            loop {
                let back = get2(end_code, 2 + LINK_SIZE) as usize;
                if back > max_back {
                    max_back = back;
                }
                end_code = end_code.add(get(end_code, 1) as usize);
                if *end_code != OP_ALT {
                    break;
                }
            }

            /* If we can't go back the amount required for the longest lookbehind
            pattern, go back as far as we can; some alternatives may still be
            viable. */

            /* In character mode we have to step back character by character */

            if utf != 0 {
                gone_back = 0;
                while gone_back < max_back {
                    if current_subject <= start_subject {
                        break;
                    }
                    current_subject = current_subject.sub(1);
                    while current_subject > start_subject
                        && (*current_subject & 0xc0) == 0x80
                    {
                        current_subject = current_subject.sub(1);
                    }
                    gone_back += 1;
                }
            } else {
                /* In byte-mode we can do this quickly. */
                let current_offset = current_subject.offset_from(start_subject) as usize;
                gone_back = if current_offset < max_back {
                    current_offset
                } else {
                    max_back
                };
                current_subject = current_subject.sub(gone_back);
            }

            /* Save the earliest consulted character */

            if current_subject < (*mb).start_used_ptr {
                (*mb).start_used_ptr = current_subject;
            }

            /* Now we can process the individual branches. There will be an
            OP_REVERSE at the start of each branch, except when the length of the
            branch is zero. */

            end_code = this_start_code;
            loop {
                let revlen: u32 = if *end_code.add(1 + LINK_SIZE) == OP_REVERSE {
                    (1 + IMM2_SIZE) as u32
                } else {
                    0
                };
                let back: usize = if revlen == 0 {
                    0
                } else {
                    get2(end_code, 2 + LINK_SIZE) as usize
                };
                if back <= gone_back {
                    let bstate = (end_code.offset_from(start_code) as c_int)
                        + 1
                        + LINK_SIZE as c_int
                        + revlen as c_int;
                    ADD_NEW_DATA!(-bstate, 0, (gone_back - back) as c_int);
                }
                end_code = end_code.add(get(end_code, 1) as usize);
                if *end_code != OP_ALT {
                    break;
                }
            }
        }
        /* This is the code for a "normal" subpattern (not a backward assertion). */
        else {
            end_code = this_start_code;

            /* Restarting */

            if rlevel == 1 && ((*mb).moptions & PCRE2_DFA_RESTART) != 0 {
                loop {
                    end_code = end_code.add(get(end_code, 1) as usize);
                    if *end_code != OP_ALT {
                        break;
                    }
                }
                new_count = *workspace.add(1);
                if *workspace == 0 {
                    memcpy(
                        new_states,
                        active_states,
                        new_count as usize,
                    );
                }
            }
            /* Not restarting */
            else {
                let mut length: c_int = 1
                    + LINK_SIZE as c_int
                    + if *this_start_code == OP_CBRA
                        || *this_start_code == OP_SCBRA
                        || *this_start_code == OP_CBRAPOS
                        || *this_start_code == OP_SCBRAPOS
                    {
                        IMM2_SIZE as c_int
                    } else {
                        0
                    };
                loop {
                    ADD_NEW!(
                        (end_code.offset_from(start_code) as c_int) + length,
                        0
                    );
                    end_code = end_code.add(get(end_code, 1) as usize);
                    length = 1 + LINK_SIZE as c_int;
                    if *end_code != OP_ALT {
                        break;
                    }
                }
            }
        }

        *workspace = 0; /* Bit indicating which vector is current */

        /* Loop for scanning the subject */

        ptr = current_subject;

        'subject: loop {
            let mut clen: c_int;
            let mut dlen: c_int;
            let mut c: u32;
            let mut d: u32;
            let mut partial_newline: BOOL = FALSE;
            let mut could_continue: BOOL = reset_could_continue;
            reset_could_continue = FALSE;

            if ptr > (*mb).last_used_ptr {
                (*mb).last_used_ptr = ptr;
            }

            /* Make the new state list into the active state list and empty the
            new state list. */

            temp_states = active_states;
            active_states = new_states;
            new_states = temp_states;
            active_count = new_count;
            new_count = 0;

            *workspace ^= 1; /* Remember for the restarting feature */
            *workspace.add(1) = active_count;

            /* Set the pointers for adding new states */

            next_active_state = active_states.add(active_count as usize);
            next_new_state = new_states;

            /* Load the current character from the subject outside the loop. */

            if ptr < end_subject {
                clen = 1; /* Number of data items in the character */
                let (cc, ln) = getcharlentest(ptr, utf != 0);
                c = cc;
                clen += ln as c_int;
            } else {
                clen = 0; /* This indicates the end of the subject */
                c = NOTACHAR; /* This value should never actually be used */
            }

            /* Scan up the active states and act on each one. */

            let mut i: c_int = 0;
            while i < active_count {
                let current_state: *mut stateblock = active_states.add(i as usize);
                let mut caseless: BOOL = FALSE;
                let mut code: PCRE2_SPTR;
                let mut codevalue: u32;
                let mut state_offset: c_int = (*current_state).offset;
                let mut rrc: c_int;
                let mut count: c_int;

                /* A negative offset is a special case meaning "hold off going to
                this (negated) state until the number of characters in the data
                field have been skipped". */

                if state_offset < 0 {
                    if (*current_state).data > 0 {
                        ADD_NEW_DATA!(
                            state_offset,
                            (*current_state).count,
                            (*current_state).data - 1
                        );
                        if could_continue != 0 {
                            reset_could_continue = TRUE;
                        }
                        i += 1;
                        continue;
                    } else {
                        state_offset = -state_offset;
                        (*current_state).offset = state_offset;
                    }
                }

                /* Check for a duplicate state with the same count, and skip if
                found. */

                let mut dup = false;
                let mut j: c_int = 0;
                while j < i {
                    if (*active_states.add(j as usize)).offset == state_offset
                        && (*active_states.add(j as usize)).count == (*current_state).count
                    {
                        dup = true;
                        break;
                    }
                    j += 1;
                }
                if dup {
                    i += 1;
                    continue;
                }

                /* The state offset is the offset to the opcode */

                code = start_code.add(state_offset as usize);
                codevalue = *code as u32;

                /* If this opcode inspects a character, but we are at the end of
                the subject, remember the fact for use when testing for a partial
                match. */

                if clen == 0 && poptable[codevalue as usize] != 0 {
                    could_continue = TRUE;
                }

                /* If this opcode is followed by an inline character, load it. */

                if coptable[codevalue as usize] > 0 {
                    dlen = 1;
                    if utf != 0 {
                        let (dd, dl) = getcharlen(code.add(coptable[codevalue as usize] as usize));
                        d = dd;
                        dlen += dl as c_int;
                    } else {
                        d = *code.add(coptable[codevalue as usize] as usize) as u32;
                    }
                    if codevalue >= OP_TYPESTAR as u32 {
                        match d as u8 {
                            OP_ANYBYTE => return PCRE2_ERROR_DFA_UITEM,
                            OP_NOTPROP | OP_PROP => codevalue += OP_PROP_EXTRA,
                            OP_ANYNL => codevalue += OP_ANYNL_EXTRA,
                            OP_EXTUNI => codevalue += OP_EXTUNI_EXTRA,
                            OP_NOT_HSPACE | OP_HSPACE => codevalue += OP_HSPACE_EXTRA,
                            OP_NOT_VSPACE | OP_VSPACE => codevalue += OP_VSPACE_EXTRA,
                            _ => {}
                        }
                    }
                } else {
                    dlen = 0; /* Not strictly necessary, but compilers moan */
                    d = NOTACHAR; /* if these variables are not set. */
                }

                /* Now process the individual opcodes. The `'active_state` labelled
                block replaces the C `NEXT_ACTIVE_STATE: continue;` goto: `break
                'active_state` skips to the end of processing for this state. */

                'active_state: {
                    match codevalue {

                    /* Reached a closing bracket. */
                    v if v == OP_KET as u32
                        || v == OP_KETRMIN as u32
                        || v == OP_KETRMAX as u32
                        || v == OP_KETRPOS as u32 =>
                    {
                        if code != end_code {
                            ADD_ACTIVE!(state_offset + 1 + LINK_SIZE as c_int, 0);
                            if codevalue != OP_KET as u32 {
                                ADD_ACTIVE!(state_offset - get(code, 1), 0);
                            }
                        } else {
                            if ptr > current_subject
                                || (((*mb).moptions & PCRE2_NOTEMPTY) == 0
                                    && (((*mb).moptions & PCRE2_NOTEMPTY_ATSTART) == 0
                                        || current_subject
                                            > start_subject.add((*mb).start_offset)))
                            {
                                if match_count < 0 {
                                    match_count = if offsetcount >= 2 { 1 } else { 0 };
                                } else if match_count > 0 {
                                    match_count += 1;
                                    if match_count * 2 > offsetcount as c_int {
                                        match_count = 0;
                                    }
                                }
                                count = (if match_count == 0 {
                                    offsetcount as c_int
                                } else {
                                    match_count * 2
                                }) - 2;
                                if count > 0 {
                                    memmove(
                                        offsets.add(2),
                                        offsets,
                                        count as usize,
                                    );
                                }
                                if offsetcount >= 2 {
                                    *offsets.add(0) =
                                        current_subject.offset_from(start_subject) as PCRE2_SIZE;
                                    *offsets.add(1) =
                                        ptr.offset_from(start_subject) as PCRE2_SIZE;
                                }
                                if ((*mb).moptions & PCRE2_DFA_SHORTEST) != 0 {
                                    return match_count;
                                }
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_ALT as u32 => {
                        loop {
                            code = code.add(get(code, 1) as usize);
                            if *code != OP_ALT {
                                break;
                            }
                        }
                        ADD_ACTIVE!(code.offset_from(start_code) as c_int, 0);
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_BRA as u32 || v == OP_SBRA as u32 => {
                        loop {
                            ADD_ACTIVE!(
                                code.offset_from(start_code) as c_int + 1 + LINK_SIZE as c_int,
                                0
                            );
                            code = code.add(get(code, 1) as usize);
                            if *code != OP_ALT {
                                break;
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_CBRA as u32 || v == OP_SCBRA as u32 => {
                        ADD_ACTIVE!(
                            code.offset_from(start_code) as c_int
                                + 1
                                + LINK_SIZE as c_int
                                + IMM2_SIZE as c_int,
                            0
                        );
                        code = code.add(get(code, 1) as usize);
                        while *code == OP_ALT {
                            ADD_ACTIVE!(
                                code.offset_from(start_code) as c_int + 1 + LINK_SIZE as c_int,
                                0
                            );
                            code = code.add(get(code, 1) as usize);
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_BRAZERO as u32 || v == OP_BRAMINZERO as u32 => {
                        ADD_ACTIVE!(state_offset + 1, 0);
                        code = code.add(1 + get(code, 2) as usize);
                        while *code == OP_ALT {
                            code = code.add(get(code, 1) as usize);
                        }
                        ADD_ACTIVE!(
                            code.offset_from(start_code) as c_int + 1 + LINK_SIZE as c_int,
                            0
                        );
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_SKIPZERO as u32 => {
                        code = code.add(1 + get(code, 2) as usize);
                        while *code == OP_ALT {
                            code = code.add(get(code, 1) as usize);
                        }
                        ADD_ACTIVE!(
                            code.offset_from(start_code) as c_int + 1 + LINK_SIZE as c_int,
                            0
                        );
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_CIRC as u32 => {
                        if ptr == start_subject && ((*mb).moptions & PCRE2_NOTBOL) == 0 {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_CIRCM as u32 => {
                        if (ptr == start_subject && ((*mb).moptions & PCRE2_NOTBOL) == 0)
                            || ((ptr != end_subject
                                || ((*mb).poptions & PCRE2_ALT_CIRCUMFLEX) != 0)
                                && was_newline_at(mb, ptr, utf))
                        {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_EOD as u32 => {
                        if ptr >= end_subject {
                            if ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                                return PCRE2_ERROR_PARTIAL;
                            } else {
                                ADD_ACTIVE!(state_offset + 1, 0);
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_SOD as u32 => {
                        if ptr == start_subject {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_SOM as u32 => {
                        if ptr == start_subject.add(start_offset) {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_ANY as u32 => {
                        if clen > 0 && !is_newline_at(mb, ptr, utf) {
                            if ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                could_continue = TRUE;
                                partial_newline = TRUE;
                            } else {
                                ADD_NEW!(state_offset + 1, 0);
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_ALLANY as u32 => {
                        if clen > 0 {
                            ADD_NEW!(state_offset + 1, 0);
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_EODN as u32 => {
                        if clen == 0
                            || (is_newline_at(mb, ptr, utf)
                                && ptr == end_subject.sub((*mb).nllen as usize))
                        {
                            if ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                                return PCRE2_ERROR_PARTIAL;
                            }
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_DOLL as u32 => {
                        if ((*mb).moptions & PCRE2_NOTEOL) == 0 {
                            if clen == 0 && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                                could_continue = TRUE;
                            } else if clen == 0
                                || (((*mb).poptions & PCRE2_DOLLAR_ENDONLY) == 0
                                    && is_newline_at(mb, ptr, utf)
                                    && (ptr == end_subject.sub((*mb).nllen as usize)))
                            {
                                ADD_ACTIVE!(state_offset + 1, 0);
                            } else if ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT))
                                    != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                if ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                                    reset_could_continue = TRUE;
                                    ADD_NEW_DATA!(-(state_offset + 1), 0, 1);
                                } else {
                                    could_continue = TRUE;
                                    partial_newline = TRUE;
                                }
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_DOLLM as u32 => {
                        if ((*mb).moptions & PCRE2_NOTEOL) == 0 {
                            if clen == 0 && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                                could_continue = TRUE;
                            } else if clen == 0
                                || (((*mb).poptions & PCRE2_DOLLAR_ENDONLY) == 0
                                    && is_newline_at(mb, ptr, utf))
                            {
                                ADD_ACTIVE!(state_offset + 1, 0);
                            } else if ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT))
                                    != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                if ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                                    reset_could_continue = TRUE;
                                    ADD_NEW_DATA!(-(state_offset + 1), 0, 1);
                                } else {
                                    could_continue = TRUE;
                                    partial_newline = TRUE;
                                }
                            }
                        } else if is_newline_at(mb, ptr, utf) {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_DIGIT as u32
                        || v == OP_WHITESPACE as u32
                        || v == OP_WORDCHAR as u32 =>
                    {
                        if clen > 0
                            && c < 256
                            && ((*ctypes.add(c as usize) & toptable1[codevalue as usize])
                                ^ toptable2[codevalue as usize])
                                != 0
                        {
                            ADD_NEW!(state_offset + 1, 0);
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_NOT_DIGIT as u32
                        || v == OP_NOT_WHITESPACE as u32
                        || v == OP_NOT_WORDCHAR as u32 =>
                    {
                        if clen > 0
                            && (c >= 256
                                || ((*ctypes.add(c as usize) & toptable1[codevalue as usize])
                                    ^ toptable2[codevalue as usize])
                                    != 0)
                        {
                            ADD_NEW!(state_offset + 1, 0);
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_WORD_BOUNDARY as u32
                        || v == OP_NOT_WORD_BOUNDARY as u32
                        || v == OP_NOT_UCP_WORD_BOUNDARY as u32
                        || v == OP_UCP_WORD_BOUNDARY as u32 =>
                    {
                        let left_word: c_int;
                        let right_word: c_int;

                        if ptr > start_subject {
                            let mut temp: PCRE2_SPTR = ptr.sub(1);
                            if temp < (*mb).start_used_ptr {
                                (*mb).start_used_ptr = temp;
                            }
                            if utf != 0 {
                                backchar(&mut temp);
                            }
                            d = getchartest(temp, utf != 0);
                            if codevalue == OP_UCP_WORD_BOUNDARY as u32
                                || codevalue == OP_NOT_UCP_WORD_BOUNDARY as u32
                            {
                                let chartype = ucd_chartype(d);
                                let category = UCP_GENTYPE[chartype as usize];
                                left_word = (category == ucp_L
                                    || category == ucp_N
                                    || chartype == ucp_Mn
                                    || chartype == ucp_Pc)
                                    as c_int;
                            } else {
                                left_word =
                                    (d < 256 && (*ctypes.add(d as usize) & ctype_word) != 0)
                                        as c_int;
                            }
                        } else {
                            left_word = FALSE;
                        }

                        if clen > 0 {
                            if ptr >= (*mb).last_used_ptr {
                                let mut temp: PCRE2_SPTR = ptr.add(1);
                                if utf != 0 {
                                    forwardchartest(&mut temp, (*mb).end_subject);
                                }
                                (*mb).last_used_ptr = temp;
                            }
                            if codevalue == OP_UCP_WORD_BOUNDARY as u32
                                || codevalue == OP_NOT_UCP_WORD_BOUNDARY as u32
                            {
                                let chartype = ucd_chartype(c);
                                let category = UCP_GENTYPE[chartype as usize];
                                right_word = (category == ucp_L
                                    || category == ucp_N
                                    || chartype == ucp_Mn
                                    || chartype == ucp_Pc)
                                    as c_int;
                            } else {
                                right_word =
                                    (c < 256 && (*ctypes.add(c as usize) & ctype_word) != 0)
                                        as c_int;
                            }
                        } else {
                            right_word = FALSE;
                        }

                        if (left_word == right_word)
                            == (codevalue == OP_NOT_WORD_BOUNDARY as u32
                                || codevalue == OP_NOT_UCP_WORD_BOUNDARY as u32)
                        {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* Check the next character by Unicode property. */
                    v if v == OP_PROP as u32 || v == OP_NOTPROP as u32 => {
                        if clen > 0 {
                            let ok = dfa_prop_ok(
                                c,
                                *code.add(1),
                                *code.add(2),
                                codevalue == OP_PROP as u32,
                            );
                            if ok == (codevalue == OP_PROP as u32) {
                                ADD_NEW!(state_offset + 3, 0);
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* These opcodes inspect the subject character with a type
                    argument loaded into d. */
                    v if v == OP_TYPEPLUS as u32
                        || v == OP_TYPEMINPLUS as u32
                        || v == OP_TYPEPOSPLUS as u32 =>
                    {
                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 2, 0);
                        }
                        if clen > 0 {
                            if d == OP_ANY as u32
                                && ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                could_continue = TRUE;
                                partial_newline = TRUE;
                            } else if (c >= 256
                                && d != OP_DIGIT as u32
                                && d != OP_WHITESPACE as u32
                                && d != OP_WORDCHAR as u32)
                                || (c < 256
                                    && (d != OP_ANY as u32 || !is_newline_at(mb, ptr, utf))
                                    && ((*ctypes.add(c as usize) & toptable1[d as usize])
                                        ^ toptable2[d as usize])
                                        != 0)
                            {
                                if count > 0 && codevalue == OP_TYPEPOSPLUS as u32 {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                ADD_NEW!(state_offset, count);
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_TYPEQUERY as u32
                        || v == OP_TYPEMINQUERY as u32
                        || v == OP_TYPEPOSQUERY as u32 =>
                    {
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            if d == OP_ANY as u32
                                && ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                could_continue = TRUE;
                                partial_newline = TRUE;
                            } else if (c >= 256
                                && d != OP_DIGIT as u32
                                && d != OP_WHITESPACE as u32
                                && d != OP_WORDCHAR as u32)
                                || (c < 256
                                    && (d != OP_ANY as u32 || !is_newline_at(mb, ptr, utf))
                                    && ((*ctypes.add(c as usize) & toptable1[d as usize])
                                        ^ toptable2[d as usize])
                                        != 0)
                            {
                                if codevalue == OP_TYPEPOSQUERY as u32 {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW!(state_offset + 2, 0);
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_TYPESTAR as u32
                        || v == OP_TYPEMINSTAR as u32
                        || v == OP_TYPEPOSSTAR as u32 =>
                    {
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            if d == OP_ANY as u32
                                && ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                could_continue = TRUE;
                                partial_newline = TRUE;
                            } else if (c >= 256
                                && d != OP_DIGIT as u32
                                && d != OP_WHITESPACE as u32
                                && d != OP_WORDCHAR as u32)
                                || (c < 256
                                    && (d != OP_ANY as u32 || !is_newline_at(mb, ptr, utf))
                                    && ((*ctypes.add(c as usize) & toptable1[d as usize])
                                        ^ toptable2[d as usize])
                                        != 0)
                            {
                                if codevalue == OP_TYPEPOSSTAR as u32 {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW!(state_offset, 0);
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_TYPEEXACT as u32 => {
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            if d == OP_ANY as u32
                                && ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                could_continue = TRUE;
                                partial_newline = TRUE;
                            } else if (c >= 256
                                && d != OP_DIGIT as u32
                                && d != OP_WHITESPACE as u32
                                && d != OP_WORDCHAR as u32)
                                || (c < 256
                                    && (d != OP_ANY as u32 || !is_newline_at(mb, ptr, utf))
                                    && ((*ctypes.add(c as usize) & toptable1[d as usize])
                                        ^ toptable2[d as usize])
                                        != 0)
                            {
                                count += 1;
                                if count >= get2(code, 1) as c_int {
                                    ADD_NEW!(state_offset + 1 + IMM2_SIZE as c_int + 1, 0);
                                } else {
                                    ADD_NEW!(state_offset, count);
                                }
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_TYPEUPTO as u32
                        || v == OP_TYPEMINUPTO as u32
                        || v == OP_TYPEPOSUPTO as u32 =>
                    {
                        ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as c_int, 0);
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            if d == OP_ANY as u32
                                && ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                could_continue = TRUE;
                                partial_newline = TRUE;
                            } else if (c >= 256
                                && d != OP_DIGIT as u32
                                && d != OP_WHITESPACE as u32
                                && d != OP_WORDCHAR as u32)
                                || (c < 256
                                    && (d != OP_ANY as u32 || !is_newline_at(mb, ptr, utf))
                                    && ((*ctypes.add(c as usize) & toptable1[d as usize])
                                        ^ toptable2[d as usize])
                                        != 0)
                            {
                                if codevalue == OP_TYPEPOSUPTO as u32 {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                if count >= get2(code, 1) as c_int {
                                    ADD_NEW!(state_offset + 2 + IMM2_SIZE as c_int, 0);
                                } else {
                                    ADD_NEW!(state_offset, count);
                                }
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* Virtual opcodes for TYPEPLUS with PROP/EXTUNI/ANYNL/
                    HSPACE/VSPACE arguments. */
                    v if v == OP_PROP_EXTRA + OP_TYPEPLUS as u32
                        || v == OP_PROP_EXTRA + OP_TYPEMINPLUS as u32
                        || v == OP_PROP_EXTRA + OP_TYPEPOSPLUS as u32 =>
                    {
                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 4, 0);
                        }
                        if clen > 0 {
                            let ok = dfa_prop_ok(c, *code.add(2), *code.add(3), false);
                            if ok == (d == OP_PROP as u32) {
                                if count > 0
                                    && codevalue == OP_PROP_EXTRA + OP_TYPEPOSPLUS as u32
                                {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                ADD_NEW!(state_offset, count);
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_EXTUNI_EXTRA + OP_TYPEPLUS as u32
                        || v == OP_EXTUNI_EXTRA + OP_TYPEMINPLUS as u32
                        || v == OP_EXTUNI_EXTRA + OP_TYPEPOSPLUS as u32 =>
                    {
                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 2, 0);
                        }
                        if clen > 0 {
                            let mut ncount: c_int = 0;
                            if count > 0
                                && codevalue == OP_EXTUNI_EXTRA + OP_TYPEPOSPLUS as u32
                            {
                                active_count -= 1;
                                next_active_state = next_active_state.sub(1);
                            }
                            extuni(
                                c,
                                ptr.add(clen as usize),
                                (*mb).start_subject,
                                end_subject,
                                utf,
                                &mut ncount,
                            );
                            count += 1;
                            ADD_NEW_DATA!(-state_offset, count, ncount);
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_ANYNL_EXTRA + OP_TYPEPLUS as u32
                        || v == OP_ANYNL_EXTRA + OP_TYPEMINPLUS as u32
                        || v == OP_ANYNL_EXTRA + OP_TYPEPOSPLUS as u32 =>
                    {
                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 2, 0);
                        }
                        if clen > 0 {
                            let mut ncount: c_int = 0;
                            let mut matched = false;
                            let mut skip = false;
                            match c {
                                CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                    if (*mb).bsr_convention == PCRE2_BSR_ANYCRLF as u16 {
                                        skip = true;
                                    } else {
                                        matched = true;
                                    }
                                }
                                CHAR_CR => {
                                    if ptr.add(1) < end_subject
                                        && *ptr.add(1) as u32 == CHAR_LF
                                    {
                                        ncount = 1;
                                    }
                                    matched = true;
                                }
                                CHAR_LF => {
                                    matched = true;
                                }
                                _ => {
                                    skip = true;
                                }
                            }
                            let _ = skip;
                            if matched {
                                if count > 0
                                    && codevalue == OP_ANYNL_EXTRA + OP_TYPEPOSPLUS as u32
                                {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                ADD_NEW_DATA!(-state_offset, count, ncount);
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_VSPACE_EXTRA + OP_TYPEPLUS as u32
                        || v == OP_VSPACE_EXTRA + OP_TYPEMINPLUS as u32
                        || v == OP_VSPACE_EXTRA + OP_TYPEPOSPLUS as u32 =>
                    {
                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 2, 0);
                        }
                        if clen > 0 {
                            let ok: BOOL = match c {
                                CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028
                                | 0x2029 => TRUE,
                                _ => FALSE,
                            };
                            if (ok != 0) == (d == OP_VSPACE as u32) {
                                if count > 0
                                    && codevalue == OP_VSPACE_EXTRA + OP_TYPEPOSPLUS as u32
                                {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                ADD_NEW_DATA!(-state_offset, count, 0);
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_HSPACE_EXTRA + OP_TYPEPLUS as u32
                        || v == OP_HSPACE_EXTRA + OP_TYPEMINPLUS as u32
                        || v == OP_HSPACE_EXTRA + OP_TYPEPOSPLUS as u32 =>
                    {
                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 2, 0);
                        }
                        if clen > 0 {
                            let ok: BOOL = match c {
                                CHAR_HT | CHAR_SPACE | CHAR_NBSP | 0x1680 | 0x180e | 0x2000
                                | 0x2001 | 0x2002 | 0x2003 | 0x2004 | 0x2005 | 0x2006
                                | 0x2007 | 0x2008 | 0x2009 | 0x200a | 0x202f | 0x205f
                                | 0x3000 => TRUE,
                                _ => FALSE,
                            };
                            if (ok != 0) == (d == OP_HSPACE as u32) {
                                if count > 0
                                    && codevalue == OP_HSPACE_EXTRA + OP_TYPEPOSPLUS as u32
                                {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                ADD_NEW_DATA!(-state_offset, count, 0);
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* PROP QUERY/STAR (QS1). QUERY sets count = 4, STAR count = 0. */
                    v if v == OP_PROP_EXTRA + OP_TYPEQUERY as u32
                        || v == OP_PROP_EXTRA + OP_TYPEMINQUERY as u32
                        || v == OP_PROP_EXTRA + OP_TYPEPOSQUERY as u32
                        || v == OP_PROP_EXTRA + OP_TYPESTAR as u32
                        || v == OP_PROP_EXTRA + OP_TYPEMINSTAR as u32
                        || v == OP_PROP_EXTRA + OP_TYPEPOSSTAR as u32 =>
                    {
                        count = if v == OP_PROP_EXTRA + OP_TYPEQUERY as u32
                            || v == OP_PROP_EXTRA + OP_TYPEMINQUERY as u32
                            || v == OP_PROP_EXTRA + OP_TYPEPOSQUERY as u32
                        {
                            4
                        } else {
                            0
                        };
                        ADD_ACTIVE!(state_offset + 4, 0);
                        if clen > 0 {
                            let ok = dfa_prop_ok(c, *code.add(2), *code.add(3), false);
                            if ok == (d == OP_PROP as u32) {
                                if codevalue == OP_PROP_EXTRA + OP_TYPEPOSSTAR as u32
                                    || codevalue == OP_PROP_EXTRA + OP_TYPEPOSQUERY as u32
                                {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW!(state_offset + count, 0);
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* EXTUNI QUERY/STAR (QS2). QUERY count = 2, STAR count = 0. */
                    v if v == OP_EXTUNI_EXTRA + OP_TYPEQUERY as u32
                        || v == OP_EXTUNI_EXTRA + OP_TYPEMINQUERY as u32
                        || v == OP_EXTUNI_EXTRA + OP_TYPEPOSQUERY as u32
                        || v == OP_EXTUNI_EXTRA + OP_TYPESTAR as u32
                        || v == OP_EXTUNI_EXTRA + OP_TYPEMINSTAR as u32
                        || v == OP_EXTUNI_EXTRA + OP_TYPEPOSSTAR as u32 =>
                    {
                        count = if v == OP_EXTUNI_EXTRA + OP_TYPEQUERY as u32
                            || v == OP_EXTUNI_EXTRA + OP_TYPEMINQUERY as u32
                            || v == OP_EXTUNI_EXTRA + OP_TYPEPOSQUERY as u32
                        {
                            2
                        } else {
                            0
                        };
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            let mut ncount: c_int = 0;
                            if codevalue == OP_EXTUNI_EXTRA + OP_TYPEPOSSTAR as u32
                                || codevalue == OP_EXTUNI_EXTRA + OP_TYPEPOSQUERY as u32
                            {
                                active_count -= 1;
                                next_active_state = next_active_state.sub(1);
                            }
                            extuni(
                                c,
                                ptr.add(clen as usize),
                                (*mb).start_subject,
                                end_subject,
                                utf,
                                &mut ncount,
                            );
                            ADD_NEW_DATA!(-(state_offset + count), 0, ncount);
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* ANYNL QUERY/STAR (QS3). QUERY count = 2, STAR count = 0. */
                    v if v == OP_ANYNL_EXTRA + OP_TYPEQUERY as u32
                        || v == OP_ANYNL_EXTRA + OP_TYPEMINQUERY as u32
                        || v == OP_ANYNL_EXTRA + OP_TYPEPOSQUERY as u32
                        || v == OP_ANYNL_EXTRA + OP_TYPESTAR as u32
                        || v == OP_ANYNL_EXTRA + OP_TYPEMINSTAR as u32
                        || v == OP_ANYNL_EXTRA + OP_TYPEPOSSTAR as u32 =>
                    {
                        count = if v == OP_ANYNL_EXTRA + OP_TYPEQUERY as u32
                            || v == OP_ANYNL_EXTRA + OP_TYPEMINQUERY as u32
                            || v == OP_ANYNL_EXTRA + OP_TYPEPOSQUERY as u32
                        {
                            2
                        } else {
                            0
                        };
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            let mut ncount: c_int = 0;
                            let mut matched = false;
                            match c {
                                CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                    if (*mb).bsr_convention != PCRE2_BSR_ANYCRLF as u16 {
                                        matched = true;
                                    }
                                }
                                CHAR_CR => {
                                    if ptr.add(1) < end_subject
                                        && *ptr.add(1) as u32 == CHAR_LF
                                    {
                                        ncount = 1;
                                    }
                                    matched = true;
                                }
                                CHAR_LF => {
                                    matched = true;
                                }
                                _ => {}
                            }
                            if matched {
                                if codevalue == OP_ANYNL_EXTRA + OP_TYPEPOSSTAR as u32
                                    || codevalue == OP_ANYNL_EXTRA + OP_TYPEPOSQUERY as u32
                                {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW_DATA!(-(state_offset + count), 0, ncount);
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* VSPACE QUERY/STAR (QS4). QUERY count = 2, STAR count = 0. */
                    v if v == OP_VSPACE_EXTRA + OP_TYPEQUERY as u32
                        || v == OP_VSPACE_EXTRA + OP_TYPEMINQUERY as u32
                        || v == OP_VSPACE_EXTRA + OP_TYPEPOSQUERY as u32
                        || v == OP_VSPACE_EXTRA + OP_TYPESTAR as u32
                        || v == OP_VSPACE_EXTRA + OP_TYPEMINSTAR as u32
                        || v == OP_VSPACE_EXTRA + OP_TYPEPOSSTAR as u32 =>
                    {
                        count = if v == OP_VSPACE_EXTRA + OP_TYPEQUERY as u32
                            || v == OP_VSPACE_EXTRA + OP_TYPEMINQUERY as u32
                            || v == OP_VSPACE_EXTRA + OP_TYPEPOSQUERY as u32
                        {
                            2
                        } else {
                            0
                        };
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            let ok: BOOL = match c {
                                CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028
                                | 0x2029 => TRUE,
                                _ => FALSE,
                            };
                            if (ok != 0) == (d == OP_VSPACE as u32) {
                                if codevalue == OP_VSPACE_EXTRA + OP_TYPEPOSSTAR as u32
                                    || codevalue == OP_VSPACE_EXTRA + OP_TYPEPOSQUERY as u32
                                {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW_DATA!(-(state_offset + count), 0, 0);
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* HSPACE QUERY/STAR (QS5). QUERY count = 2, STAR count = 0. */
                    v if v == OP_HSPACE_EXTRA + OP_TYPEQUERY as u32
                        || v == OP_HSPACE_EXTRA + OP_TYPEMINQUERY as u32
                        || v == OP_HSPACE_EXTRA + OP_TYPEPOSQUERY as u32
                        || v == OP_HSPACE_EXTRA + OP_TYPESTAR as u32
                        || v == OP_HSPACE_EXTRA + OP_TYPEMINSTAR as u32
                        || v == OP_HSPACE_EXTRA + OP_TYPEPOSSTAR as u32 =>
                    {
                        count = if v == OP_HSPACE_EXTRA + OP_TYPEQUERY as u32
                            || v == OP_HSPACE_EXTRA + OP_TYPEMINQUERY as u32
                            || v == OP_HSPACE_EXTRA + OP_TYPEPOSQUERY as u32
                        {
                            2
                        } else {
                            0
                        };
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            let ok: BOOL = match c {
                                CHAR_HT | CHAR_SPACE | CHAR_NBSP | 0x1680 | 0x180e | 0x2000
                                | 0x2001 | 0x2002 | 0x2003 | 0x2004 | 0x2005 | 0x2006
                                | 0x2007 | 0x2008 | 0x2009 | 0x200a | 0x202f | 0x205f
                                | 0x3000 => TRUE,
                                _ => FALSE,
                            };
                            if (ok != 0) == (d == OP_HSPACE as u32) {
                                if codevalue == OP_HSPACE_EXTRA + OP_TYPEPOSSTAR as u32
                                    || codevalue == OP_HSPACE_EXTRA + OP_TYPEPOSQUERY as u32
                                {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW_DATA!(-(state_offset + count), 0, 0);
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* PROP EXACT/UPTO. */
                    v if v == OP_PROP_EXTRA + OP_TYPEEXACT as u32
                        || v == OP_PROP_EXTRA + OP_TYPEUPTO as u32
                        || v == OP_PROP_EXTRA + OP_TYPEMINUPTO as u32
                        || v == OP_PROP_EXTRA + OP_TYPEPOSUPTO as u32 =>
                    {
                        if codevalue != OP_PROP_EXTRA + OP_TYPEEXACT as u32 {
                            ADD_ACTIVE!(state_offset + 1 + IMM2_SIZE as c_int + 3, 0);
                        }
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let ok = dfa_prop_ok(
                                c,
                                *code.add(1 + IMM2_SIZE + 1),
                                *code.add(1 + IMM2_SIZE + 2),
                                false,
                            );
                            if ok == (d == OP_PROP as u32) {
                                if codevalue == OP_PROP_EXTRA + OP_TYPEPOSUPTO as u32 {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                if count >= get2(code, 1) as c_int {
                                    ADD_NEW!(state_offset + 1 + IMM2_SIZE as c_int + 3, 0);
                                } else {
                                    ADD_NEW!(state_offset, count);
                                }
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* EXTUNI EXACT/UPTO. */
                    v if v == OP_EXTUNI_EXTRA + OP_TYPEEXACT as u32
                        || v == OP_EXTUNI_EXTRA + OP_TYPEUPTO as u32
                        || v == OP_EXTUNI_EXTRA + OP_TYPEMINUPTO as u32
                        || v == OP_EXTUNI_EXTRA + OP_TYPEPOSUPTO as u32 =>
                    {
                        if codevalue != OP_EXTUNI_EXTRA + OP_TYPEEXACT as u32 {
                            ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as c_int, 0);
                        }
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let mut ncount: c_int = 0;
                            if codevalue == OP_EXTUNI_EXTRA + OP_TYPEPOSUPTO as u32 {
                                active_count -= 1;
                                next_active_state = next_active_state.sub(1);
                            }
                            let nptr = extuni(
                                c,
                                ptr.add(clen as usize),
                                (*mb).start_subject,
                                end_subject,
                                utf,
                                &mut ncount,
                            );
                            if nptr >= end_subject
                                && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                            {
                                reset_could_continue = TRUE;
                            }
                            count += 1;
                            if count >= get2(code, 1) as c_int {
                                ADD_NEW_DATA!(
                                    -(state_offset + 2 + IMM2_SIZE as c_int),
                                    0,
                                    ncount
                                );
                            } else {
                                ADD_NEW_DATA!(-state_offset, count, ncount);
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* ANYNL EXACT/UPTO. */
                    v if v == OP_ANYNL_EXTRA + OP_TYPEEXACT as u32
                        || v == OP_ANYNL_EXTRA + OP_TYPEUPTO as u32
                        || v == OP_ANYNL_EXTRA + OP_TYPEMINUPTO as u32
                        || v == OP_ANYNL_EXTRA + OP_TYPEPOSUPTO as u32 =>
                    {
                        if codevalue != OP_ANYNL_EXTRA + OP_TYPEEXACT as u32 {
                            ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as c_int, 0);
                        }
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let mut ncount: c_int = 0;
                            let mut matched = false;
                            match c {
                                CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                    if (*mb).bsr_convention != PCRE2_BSR_ANYCRLF as u16 {
                                        matched = true;
                                    }
                                }
                                CHAR_CR => {
                                    if ptr.add(1) < end_subject
                                        && *ptr.add(1) as u32 == CHAR_LF
                                    {
                                        ncount = 1;
                                    }
                                    matched = true;
                                }
                                CHAR_LF => {
                                    matched = true;
                                }
                                _ => {}
                            }
                            if matched {
                                if codevalue == OP_ANYNL_EXTRA + OP_TYPEPOSUPTO as u32 {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                if count >= get2(code, 1) as c_int {
                                    ADD_NEW_DATA!(
                                        -(state_offset + 2 + IMM2_SIZE as c_int),
                                        0,
                                        ncount
                                    );
                                } else {
                                    ADD_NEW_DATA!(-state_offset, count, ncount);
                                }
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* VSPACE EXACT/UPTO. */
                    v if v == OP_VSPACE_EXTRA + OP_TYPEEXACT as u32
                        || v == OP_VSPACE_EXTRA + OP_TYPEUPTO as u32
                        || v == OP_VSPACE_EXTRA + OP_TYPEMINUPTO as u32
                        || v == OP_VSPACE_EXTRA + OP_TYPEPOSUPTO as u32 =>
                    {
                        if codevalue != OP_VSPACE_EXTRA + OP_TYPEEXACT as u32 {
                            ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as c_int, 0);
                        }
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let ok: BOOL = match c {
                                CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028
                                | 0x2029 => TRUE,
                                _ => FALSE,
                            };
                            if (ok != 0) == (d == OP_VSPACE as u32) {
                                if codevalue == OP_VSPACE_EXTRA + OP_TYPEPOSUPTO as u32 {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                if count >= get2(code, 1) as c_int {
                                    ADD_NEW_DATA!(
                                        -(state_offset + 2 + IMM2_SIZE as c_int),
                                        0,
                                        0
                                    );
                                } else {
                                    ADD_NEW_DATA!(-state_offset, count, 0);
                                }
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* HSPACE EXACT/UPTO. */
                    v if v == OP_HSPACE_EXTRA + OP_TYPEEXACT as u32
                        || v == OP_HSPACE_EXTRA + OP_TYPEUPTO as u32
                        || v == OP_HSPACE_EXTRA + OP_TYPEMINUPTO as u32
                        || v == OP_HSPACE_EXTRA + OP_TYPEPOSUPTO as u32 =>
                    {
                        if codevalue != OP_HSPACE_EXTRA + OP_TYPEEXACT as u32 {
                            ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as c_int, 0);
                        }
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let ok: BOOL = match c {
                                CHAR_HT | CHAR_SPACE | CHAR_NBSP | 0x1680 | 0x180e | 0x2000
                                | 0x2001 | 0x2002 | 0x2003 | 0x2004 | 0x2005 | 0x2006
                                | 0x2007 | 0x2008 | 0x2009 | 0x200a | 0x202f | 0x205f
                                | 0x3000 => TRUE,
                                _ => FALSE,
                            };
                            if (ok != 0) == (d == OP_HSPACE as u32) {
                                if codevalue == OP_HSPACE_EXTRA + OP_TYPEPOSUPTO as u32 {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                if count >= get2(code, 1) as c_int {
                                    ADD_NEW_DATA!(
                                        -(state_offset + 2 + IMM2_SIZE as c_int),
                                        0,
                                        0
                                    );
                                } else {
                                    ADD_NEW_DATA!(-state_offset, count, 0);
                                }
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* Opcodes followed by an inline character, loaded into d. */
                    v if v == OP_CHAR as u32 => {
                        if clen > 0 && c == d {
                            ADD_NEW!(state_offset + dlen + 1, 0);
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_CHARI as u32 => {
                        if clen == 0 {
                            break 'active_state;
                        }
                        if utf_or_ucp != 0 {
                            if c == d {
                                ADD_NEW!(state_offset + dlen + 1, 0);
                            } else {
                                let othercase: u32 = if c < 128 {
                                    *fcc.add(c as usize) as u32
                                } else {
                                    ucd_othercase(c)
                                };
                                if d == othercase {
                                    ADD_NEW!(state_offset + dlen + 1, 0);
                                }
                            }
                        } else {
                            /* Not UTF or UCP mode */
                            if table_get(c, lcc, c) == table_get(d, lcc, d) {
                                ADD_NEW!(state_offset + 2, 0);
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* OP_EXTUNI can match more than one character. */
                    v if v == OP_EXTUNI as u32 => {
                        if clen > 0 {
                            let mut ncount: c_int = 0;
                            let nptr = extuni(
                                c,
                                ptr.add(clen as usize),
                                (*mb).start_subject,
                                end_subject,
                                utf,
                                &mut ncount,
                            );
                            if nptr >= end_subject
                                && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                            {
                                reset_could_continue = TRUE;
                            }
                            ADD_NEW_DATA!(-(state_offset + 1), 0, ncount);
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* OP_ANYNL can match CR LF as one. */
                    v if v == OP_ANYNL as u32 => {
                        if clen > 0 {
                            match c {
                                CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                    if (*mb).bsr_convention == PCRE2_BSR_ANYCRLF as u16 {
                                        /* break out of switch(c): do nothing */
                                    } else {
                                        ADD_NEW!(state_offset + 1, 0);
                                    }
                                }
                                CHAR_LF => {
                                    ADD_NEW!(state_offset + 1, 0);
                                }
                                CHAR_CR => {
                                    if ptr.add(1) >= end_subject {
                                        ADD_NEW!(state_offset + 1, 0);
                                        if ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                                            reset_could_continue = TRUE;
                                        }
                                    } else if *ptr.add(1) as u32 == CHAR_LF {
                                        ADD_NEW_DATA!(-(state_offset + 1), 0, 1);
                                    } else {
                                        ADD_NEW!(state_offset + 1, 0);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_NOT_VSPACE as u32 => {
                        if clen > 0 {
                            match c {
                                CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028
                                | 0x2029 => {}
                                _ => {
                                    ADD_NEW!(state_offset + 1, 0);
                                }
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_VSPACE as u32 => {
                        if clen > 0 {
                            match c {
                                CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028
                                | 0x2029 => {
                                    ADD_NEW!(state_offset + 1, 0);
                                }
                                _ => {}
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_NOT_HSPACE as u32 => {
                        if clen > 0 {
                            match c {
                                CHAR_HT | CHAR_SPACE | CHAR_NBSP | 0x1680 | 0x180e | 0x2000
                                | 0x2001 | 0x2002 | 0x2003 | 0x2004 | 0x2005 | 0x2006
                                | 0x2007 | 0x2008 | 0x2009 | 0x200a | 0x202f | 0x205f
                                | 0x3000 => {}
                                _ => {
                                    ADD_NEW!(state_offset + 1, 0);
                                }
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_HSPACE as u32 => {
                        if clen > 0 {
                            match c {
                                CHAR_HT | CHAR_SPACE | CHAR_NBSP | 0x1680 | 0x180e | 0x2000
                                | 0x2001 | 0x2002 | 0x2003 | 0x2004 | 0x2005 | 0x2006
                                | 0x2007 | 0x2008 | 0x2009 | 0x200a | 0x202f | 0x205f
                                | 0x3000 => {
                                    ADD_NEW!(state_offset + 1, 0);
                                }
                                _ => {}
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* Match a negated single character casefully. */
                    v if v == OP_NOT as u32 => {
                        if clen > 0 && c != d {
                            ADD_NEW!(state_offset + dlen + 1, 0);
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* Match a negated single character caselessly. */
                    v if v == OP_NOTI as u32 => {
                        if clen > 0 {
                            let otherd: u32 = if utf_or_ucp != 0 && d >= 128 {
                                ucd_othercase(d)
                            } else {
                                table_get(d, fcc, d)
                            };
                            if c != d && c != otherd {
                                ADD_NEW!(state_offset + dlen + 1, 0);
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_PLUSI as u32
                        || v == OP_MINPLUSI as u32
                        || v == OP_POSPLUSI as u32
                        || v == OP_NOTPLUSI as u32
                        || v == OP_NOTMINPLUSI as u32
                        || v == OP_NOTPOSPLUSI as u32
                        || v == OP_PLUS as u32
                        || v == OP_MINPLUS as u32
                        || v == OP_POSPLUS as u32
                        || v == OP_NOTPLUS as u32
                        || v == OP_NOTMINPLUS as u32
                        || v == OP_NOTPOSPLUS as u32 =>
                    {
                        if codevalue >= OP_STARI as u32 {
                            caseless = TRUE;
                            codevalue -= (OP_STARI - OP_STAR) as u32;
                        }
                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + dlen + 1, 0);
                        }
                        if clen > 0 {
                            let mut otherd: u32 = NOTACHAR;
                            if caseless != 0 {
                                otherd = if utf_or_ucp != 0 && d >= 128 {
                                    ucd_othercase(d)
                                } else {
                                    table_get(d, fcc, d)
                                };
                            }
                            if (c == d || c == otherd) == (codevalue < OP_NOTSTAR as u32) {
                                if count > 0
                                    && (codevalue == OP_POSPLUS as u32
                                        || codevalue == OP_NOTPOSPLUS as u32)
                                {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                ADD_NEW!(state_offset, count);
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_QUERYI as u32
                        || v == OP_MINQUERYI as u32
                        || v == OP_POSQUERYI as u32
                        || v == OP_NOTQUERYI as u32
                        || v == OP_NOTMINQUERYI as u32
                        || v == OP_NOTPOSQUERYI as u32
                        || v == OP_QUERY as u32
                        || v == OP_MINQUERY as u32
                        || v == OP_POSQUERY as u32
                        || v == OP_NOTQUERY as u32
                        || v == OP_NOTMINQUERY as u32
                        || v == OP_NOTPOSQUERY as u32 =>
                    {
                        if codevalue >= OP_STARI as u32 {
                            caseless = TRUE;
                            codevalue -= (OP_STARI - OP_STAR) as u32;
                        }
                        ADD_ACTIVE!(state_offset + dlen + 1, 0);
                        if clen > 0 {
                            let mut otherd: u32 = NOTACHAR;
                            if caseless != 0 {
                                otherd = if utf_or_ucp != 0 && d >= 128 {
                                    ucd_othercase(d)
                                } else {
                                    table_get(d, fcc, d)
                                };
                            }
                            if (c == d || c == otherd) == (codevalue < OP_NOTSTAR as u32) {
                                if codevalue == OP_POSQUERY as u32
                                    || codevalue == OP_NOTPOSQUERY as u32
                                {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW!(state_offset + dlen + 1, 0);
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_STARI as u32
                        || v == OP_MINSTARI as u32
                        || v == OP_POSSTARI as u32
                        || v == OP_NOTSTARI as u32
                        || v == OP_NOTMINSTARI as u32
                        || v == OP_NOTPOSSTARI as u32
                        || v == OP_STAR as u32
                        || v == OP_MINSTAR as u32
                        || v == OP_POSSTAR as u32
                        || v == OP_NOTSTAR as u32
                        || v == OP_NOTMINSTAR as u32
                        || v == OP_NOTPOSSTAR as u32 =>
                    {
                        if codevalue >= OP_STARI as u32 {
                            caseless = TRUE;
                            codevalue -= (OP_STARI - OP_STAR) as u32;
                        }
                        ADD_ACTIVE!(state_offset + dlen + 1, 0);
                        if clen > 0 {
                            let mut otherd: u32 = NOTACHAR;
                            if caseless != 0 {
                                otherd = if utf_or_ucp != 0 && d >= 128 {
                                    ucd_othercase(d)
                                } else {
                                    table_get(d, fcc, d)
                                };
                            }
                            if (c == d || c == otherd) == (codevalue < OP_NOTSTAR as u32) {
                                if codevalue == OP_POSSTAR as u32
                                    || codevalue == OP_NOTPOSSTAR as u32
                                {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW!(state_offset, 0);
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_EXACTI as u32
                        || v == OP_NOTEXACTI as u32
                        || v == OP_EXACT as u32
                        || v == OP_NOTEXACT as u32 =>
                    {
                        if codevalue >= OP_STARI as u32 {
                            caseless = TRUE;
                            codevalue -= (OP_STARI - OP_STAR) as u32;
                        }
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let mut otherd: u32 = NOTACHAR;
                            if caseless != 0 {
                                otherd = if utf_or_ucp != 0 && d >= 128 {
                                    ucd_othercase(d)
                                } else {
                                    table_get(d, fcc, d)
                                };
                            }
                            if (c == d || c == otherd) == (codevalue < OP_NOTSTAR as u32) {
                                count += 1;
                                if count >= get2(code, 1) as c_int {
                                    ADD_NEW!(
                                        state_offset + dlen + 1 + IMM2_SIZE as c_int,
                                        0
                                    );
                                } else {
                                    ADD_NEW!(state_offset, count);
                                }
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_UPTOI as u32
                        || v == OP_MINUPTOI as u32
                        || v == OP_POSUPTOI as u32
                        || v == OP_NOTUPTOI as u32
                        || v == OP_NOTMINUPTOI as u32
                        || v == OP_NOTPOSUPTOI as u32
                        || v == OP_UPTO as u32
                        || v == OP_MINUPTO as u32
                        || v == OP_POSUPTO as u32
                        || v == OP_NOTUPTO as u32
                        || v == OP_NOTMINUPTO as u32
                        || v == OP_NOTPOSUPTO as u32 =>
                    {
                        if codevalue >= OP_STARI as u32 {
                            caseless = TRUE;
                            codevalue -= (OP_STARI - OP_STAR) as u32;
                        }
                        ADD_ACTIVE!(state_offset + dlen + 1 + IMM2_SIZE as c_int, 0);
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let mut otherd: u32 = NOTACHAR;
                            if caseless != 0 {
                                otherd = if utf_or_ucp != 0 && d >= 128 {
                                    ucd_othercase(d)
                                } else {
                                    table_get(d, fcc, d)
                                };
                            }
                            if (c == d || c == otherd) == (codevalue < OP_NOTSTAR as u32) {
                                if codevalue == OP_POSUPTO as u32
                                    || codevalue == OP_NOTPOSUPTO as u32
                                {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                if count >= get2(code, 1) as c_int {
                                    ADD_NEW!(
                                        state_offset + dlen + 1 + IMM2_SIZE as c_int,
                                        0
                                    );
                                } else {
                                    ADD_NEW!(state_offset, count);
                                }
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* The class-handling opcodes. */
                    v if v == OP_CLASS as u32
                        || v == OP_NCLASS as u32
                        || v == OP_XCLASS as u32
                        || v == OP_ECLASS as u32 =>
                    {
                        let mut isinclass: BOOL = FALSE;
                        let next_state_offset: c_int;
                        let ecode: PCRE2_SPTR;

                        /* An extended class may have a table or a list of single
                        characters, ranges, or both, and it may be positive or
                        negative. */

                        if codevalue == OP_XCLASS as u32 {
                            ecode = code.add(get(code, 1) as usize);
                            if clen > 0 {
                                isinclass = xclass(
                                    c,
                                    code.add(1 + LINK_SIZE),
                                    (*mb).start_code as *const u8,
                                    utf,
                                );
                            }
                        } else if codevalue == OP_ECLASS as u32 {
                            ecode = code.add(get(code, 1) as usize);
                            if clen > 0 {
                                isinclass = eclass(
                                    c,
                                    code.add(1 + LINK_SIZE),
                                    ecode,
                                    (*mb).start_code as *const u8,
                                    utf,
                                );
                            }
                        } else {
                            /* For a simple class, there is always just a 32-byte
                            table, and we can set isinclass from it. */
                            ecode = code.add(1 + 32);
                            if clen > 0 {
                                isinclass = if c > 255 {
                                    (codevalue == OP_NCLASS as u32) as BOOL
                                } else {
                                    ((*code.add(1).add((c / 8) as usize) & (1u8 << (c & 7)))
                                        != 0) as BOOL
                                };
                            }
                        }

                        /* At this point, isinclass is set for all kinds of class,
                        and ecode points to the byte after the end of the class. */

                        next_state_offset = ecode.offset_from(start_code) as c_int;

                        match *ecode {
                            OP_CRSTAR | OP_CRMINSTAR | OP_CRPOSSTAR => {
                                ADD_ACTIVE!(next_state_offset + 1, 0);
                                if isinclass != 0 {
                                    if *ecode == OP_CRPOSSTAR {
                                        active_count -= 1;
                                        next_active_state = next_active_state.sub(1);
                                    }
                                    ADD_NEW!(state_offset, 0);
                                }
                            }

                            OP_CRPLUS | OP_CRMINPLUS | OP_CRPOSPLUS => {
                                count = (*current_state).count; /* Already matched */
                                if count > 0 {
                                    ADD_ACTIVE!(next_state_offset + 1, 0);
                                }
                                if isinclass != 0 {
                                    if count > 0 && *ecode == OP_CRPOSPLUS {
                                        active_count -= 1;
                                        next_active_state = next_active_state.sub(1);
                                    }
                                    count += 1;
                                    ADD_NEW!(state_offset, count);
                                }
                            }

                            OP_CRQUERY | OP_CRMINQUERY | OP_CRPOSQUERY => {
                                ADD_ACTIVE!(next_state_offset + 1, 0);
                                if isinclass != 0 {
                                    if *ecode == OP_CRPOSQUERY {
                                        active_count -= 1;
                                        next_active_state = next_active_state.sub(1);
                                    }
                                    ADD_NEW!(next_state_offset + 1, 0);
                                }
                            }

                            OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                                count = (*current_state).count; /* Already matched */
                                if count >= get2(ecode, 1) as c_int {
                                    ADD_ACTIVE!(
                                        next_state_offset + 1 + 2 * IMM2_SIZE as c_int,
                                        0
                                    );
                                }
                                if isinclass != 0 {
                                    let max = get2(ecode, 1 + IMM2_SIZE) as c_int;

                                    if *ecode == OP_CRPOSRANGE
                                        && count >= get2(ecode, 1) as c_int
                                    {
                                        active_count -= 1;
                                        next_active_state = next_active_state.sub(1);
                                    }

                                    count += 1;
                                    if count >= max && max != 0 {
                                        ADD_NEW!(
                                            next_state_offset + 1 + 2 * IMM2_SIZE as c_int,
                                            0
                                        );
                                    } else {
                                        ADD_NEW!(state_offset, count);
                                    }
                                }
                            }

                            _ => {
                                if isinclass != 0 {
                                    ADD_NEW!(next_state_offset, 0);
                                }
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* Fancy brackets. (?!) is optimised to OP_FAIL. */
                    v if v == OP_FAIL as u32 => {}

                    v if v == OP_ASSERT as u32
                        || v == OP_ASSERT_NOT as u32
                        || v == OP_ASSERTBACK as u32
                        || v == OP_ASSERTBACK_NOT as u32 =>
                    {
                        let rc: c_int;
                        let local_workspace: *mut c_int;
                        let local_offsets: *mut PCRE2_SIZE;
                        let mut endasscode: PCRE2_SPTR = code.add(get(code, 1) as usize);
                        let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;

                        if ((*rws).free as usize) < RWS_RSIZE + RWS_OVEC_OSIZE {
                            let r = more_workspace(&mut rws, RWS_OVEC_OSIZE as u32, mb);
                            if r != 0 {
                                return r;
                            }
                            RWS = rws as *mut c_int;
                        }

                        local_offsets =
                            (RWS.add((*rws).size as usize - (*rws).free as usize))
                                as *mut PCRE2_SIZE;
                        local_workspace =
                            (local_offsets as *mut c_int).add(RWS_OVEC_OSIZE);
                        (*rws).free -= (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        while *endasscode == OP_ALT {
                            endasscode = endasscode.add(get(endasscode, 1) as usize);
                        }

                        rc = internal_dfa_match(
                            mb,
                            code,
                            ptr,
                            ptr.offset_from(start_subject) as PCRE2_SIZE,
                            local_offsets,
                            (RWS_OVEC_OSIZE / OVEC_UNIT) as u32,
                            local_workspace,
                            RWS_RSIZE as c_int,
                            rlevel,
                            RWS,
                        );

                        (*rws).free += (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        if rc < 0 && rc != PCRE2_ERROR_NOMATCH {
                            return rc;
                        }
                        if (rc >= 0)
                            == (codevalue == OP_ASSERT as u32
                                || codevalue == OP_ASSERTBACK as u32)
                        {
                            ADD_ACTIVE!(
                                endasscode.add(LINK_SIZE + 1).offset_from(start_code) as c_int,
                                0
                            );
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_COND as u32 || v == OP_SCOND as u32 => {
                        let codelink: c_int = get(code, 1);
                        let condcode: PCRE2_UCHAR;

                        /* Auto-callout inserts a callout item between OP_COND and
                        an assertion condition. */

                        if *code.add(LINK_SIZE + 1) == OP_CALLOUT
                            || *code.add(LINK_SIZE + 1) == OP_CALLOUT_STR
                        {
                            let mut callout_length: PCRE2_SIZE = 0;
                            rrc = do_callout_dfa(
                                code,
                                offsets,
                                current_subject,
                                ptr,
                                mb,
                                1 + LINK_SIZE,
                                &mut callout_length,
                            );
                            if rrc < 0 {
                                return rrc; /* Abandon */
                            }
                            if rrc > 0 {
                                break 'active_state; /* Fail this thread */
                            }
                            code = code.add(callout_length as usize); /* Skip callout data */
                        }

                        condcode = *code.add(LINK_SIZE + 1);

                        /* Back reference conditions and duplicate named recursion
                        conditions are not supported */

                        if condcode == OP_CREF
                            || condcode == OP_DNCREF
                            || condcode == OP_DNRREF
                        {
                            return PCRE2_ERROR_DFA_UCOND;
                        }

                        /* The DEFINE condition is always false, and the assertion
                        (?!) is converted to OP_FAIL. */

                        if condcode == OP_FALSE || condcode == OP_FAIL {
                            ADD_ACTIVE!(
                                state_offset + codelink + LINK_SIZE as c_int + 1,
                                0
                            );
                        } else if condcode == OP_TRUE {
                            ADD_ACTIVE!(state_offset + LINK_SIZE as c_int + 2, 0);
                        } else if condcode == OP_RREF {
                            let value = get2(code, LINK_SIZE + 2);
                            if value != RREF_ANY {
                                return PCRE2_ERROR_DFA_UCOND;
                            }
                            if !(*mb).recursive.is_null() {
                                ADD_ACTIVE!(
                                    state_offset
                                        + LINK_SIZE as c_int
                                        + 2
                                        + IMM2_SIZE as c_int,
                                    0
                                );
                            } else {
                                ADD_ACTIVE!(
                                    state_offset + codelink + LINK_SIZE as c_int + 1,
                                    0
                                );
                            }
                        } else {
                            /* Otherwise, the condition is an assertion */
                            let rc: c_int;
                            let local_workspace: *mut c_int;
                            let local_offsets: *mut PCRE2_SIZE;
                            let asscode: PCRE2_SPTR = code.add(LINK_SIZE + 1);
                            let mut endasscode: PCRE2_SPTR =
                                asscode.add(get(asscode, 1) as usize);
                            let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;

                            if ((*rws).free as usize) < RWS_RSIZE + RWS_OVEC_OSIZE {
                                let r = more_workspace(&mut rws, RWS_OVEC_OSIZE as u32, mb);
                                if r != 0 {
                                    return r;
                                }
                                RWS = rws as *mut c_int;
                            }

                            local_offsets =
                                (RWS.add((*rws).size as usize - (*rws).free as usize))
                                    as *mut PCRE2_SIZE;
                            local_workspace =
                                (local_offsets as *mut c_int).add(RWS_OVEC_OSIZE);
                            (*rws).free -= (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                            while *endasscode == OP_ALT {
                                endasscode = endasscode.add(get(endasscode, 1) as usize);
                            }

                            rc = internal_dfa_match(
                                mb,
                                asscode,
                                ptr,
                                ptr.offset_from(start_subject) as PCRE2_SIZE,
                                local_offsets,
                                (RWS_OVEC_OSIZE / OVEC_UNIT) as u32,
                                local_workspace,
                                RWS_RSIZE as c_int,
                                rlevel,
                                RWS,
                            );

                            (*rws).free += (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                            if rc < 0 && rc != PCRE2_ERROR_NOMATCH {
                                return rc;
                            }
                            if (rc >= 0)
                                == (condcode == OP_ASSERT || condcode == OP_ASSERTBACK)
                            {
                                ADD_ACTIVE!(
                                    endasscode.add(LINK_SIZE + 1).offset_from(start_code)
                                        as c_int,
                                    0
                                );
                            } else {
                                ADD_ACTIVE!(
                                    state_offset + codelink + LINK_SIZE as c_int + 1,
                                    0
                                );
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_RECURSE as u32 => {
                        let mut rc: c_int;
                        let local_workspace: *mut c_int;
                        let local_offsets: *mut PCRE2_SIZE;
                        let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;
                        let callpat: PCRE2_SPTR = start_code.add(get(code, 1) as usize);
                        let recno: u32 = if callpat == (*mb).start_code {
                            0
                        } else {
                            get2(callpat, 1 + LINK_SIZE)
                        };

                        /* Argument list has not been supported yet. */
                        if *code.add(1 + LINK_SIZE) == OP_CREF {
                            return PCRE2_ERROR_DFA_UITEM;
                        }

                        if ((*rws).free as usize) < RWS_RSIZE + RWS_OVEC_RSIZE {
                            let r = more_workspace(&mut rws, RWS_OVEC_RSIZE as u32, mb);
                            if r != 0 {
                                return r;
                            }
                            RWS = rws as *mut c_int;
                        }

                        local_offsets =
                            (RWS.add((*rws).size as usize - (*rws).free as usize))
                                as *mut PCRE2_SIZE;
                        local_workspace =
                            (local_offsets as *mut c_int).add(RWS_OVEC_RSIZE);
                        (*rws).free -= (RWS_RSIZE + RWS_OVEC_RSIZE) as u32;

                        /* Check for repeating a recursion without advancing. */
                        let mut ri: *mut dfa_recursion_info = (*mb).recursive;
                        while !ri.is_null() {
                            if recno == (*ri).group_num
                                && ptr == (*ri).subject_position
                                && (*mb).last_used_ptr == (*ri).last_used_ptr
                            {
                                return PCRE2_ERROR_RECURSELOOP;
                            }
                            ri = (*ri).prevrec as *mut dfa_recursion_info;
                        }

                        /* Remember this recursion. */
                        new_recursive.group_num = recno;
                        new_recursive.subject_position = ptr;
                        new_recursive.last_used_ptr = (*mb).last_used_ptr;
                        new_recursive.prevrec = (*mb).recursive;
                        (*mb).recursive = &mut new_recursive;

                        rc = internal_dfa_match(
                            mb,
                            callpat,
                            ptr,
                            ptr.offset_from(start_subject) as PCRE2_SIZE,
                            local_offsets,
                            (RWS_OVEC_RSIZE / OVEC_UNIT) as u32,
                            local_workspace,
                            RWS_RSIZE as c_int,
                            rlevel,
                            RWS,
                        );

                        (*rws).free += (RWS_RSIZE + RWS_OVEC_RSIZE) as u32;
                        (*mb).recursive = new_recursive.prevrec as *mut dfa_recursion_info;

                        /* Ran out of internal offsets */
                        if rc == 0 {
                            return PCRE2_ERROR_DFA_RECURSE;
                        }

                        if rc > 0 {
                            rc = rc * 2 - 2;
                            while rc >= 0 {
                                let mut charcount: PCRE2_SIZE = *local_offsets
                                    .add(rc as usize + 1)
                                    - *local_offsets.add(rc as usize);
                                if utf != 0 {
                                    let mut p: PCRE2_SPTR =
                                        start_subject.add(*local_offsets.add(rc as usize));
                                    let pp: PCRE2_SPTR = start_subject
                                        .add(*local_offsets.add(rc as usize + 1));
                                    while p < pp {
                                        let b = *p;
                                        p = p.add(1);
                                        if not_firstcu(b as u32) {
                                            charcount -= 1;
                                        }
                                    }
                                }
                                if charcount > 0 {
                                    ADD_NEW_DATA!(
                                        -(state_offset + LINK_SIZE as c_int + 1),
                                        0,
                                        (charcount - 1) as c_int
                                    );
                                } else {
                                    ADD_ACTIVE!(state_offset + LINK_SIZE as c_int + 1, 0);
                                }
                                rc -= 2;
                            }
                        } else if rc != PCRE2_ERROR_NOMATCH {
                            return rc;
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_BRAPOS as u32
                        || v == OP_SBRAPOS as u32
                        || v == OP_CBRAPOS as u32
                        || v == OP_SCBRAPOS as u32
                        || v == OP_BRAPOSZERO as u32 =>
                    {
                        let mut rc: c_int;
                        let local_workspace: *mut c_int;
                        let local_offsets: *mut PCRE2_SIZE;
                        let mut charcount: PCRE2_SIZE;
                        let mut matched_count: PCRE2_SIZE;
                        let mut local_ptr: PCRE2_SPTR = ptr;
                        let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;
                        let allow_zero: BOOL;

                        if ((*rws).free as usize) < RWS_RSIZE + RWS_OVEC_OSIZE {
                            let r = more_workspace(&mut rws, RWS_OVEC_OSIZE as u32, mb);
                            if r != 0 {
                                return r;
                            }
                            RWS = rws as *mut c_int;
                        }

                        local_offsets =
                            (RWS.add((*rws).size as usize - (*rws).free as usize))
                                as *mut PCRE2_SIZE;
                        local_workspace =
                            (local_offsets as *mut c_int).add(RWS_OVEC_OSIZE);
                        (*rws).free -= (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        if codevalue == OP_BRAPOSZERO as u32 {
                            allow_zero = TRUE;
                            code = code.add(1); /* The following opcode will be a BRA */
                        } else {
                            allow_zero = FALSE;
                        }

                        /* Loop to match the subpattern as many times as possible. */
                        matched_count = 0;
                        loop {
                            rc = internal_dfa_match(
                                mb,
                                code,
                                local_ptr,
                                ptr.offset_from(start_subject) as PCRE2_SIZE,
                                local_offsets,
                                (RWS_OVEC_OSIZE / OVEC_UNIT) as u32,
                                local_workspace,
                                RWS_RSIZE as c_int,
                                rlevel,
                                RWS,
                            );

                            /* Failed to match */
                            if rc < 0 {
                                if rc != PCRE2_ERROR_NOMATCH {
                                    return rc;
                                }
                                break;
                            }

                            /* Matched: break the loop if zero characters matched. */
                            charcount = *local_offsets.add(1) - *local_offsets.add(0);
                            if charcount == 0 {
                                break;
                            }
                            local_ptr = local_ptr.add(charcount); /* Advance temp position */
                            matched_count += 1;
                        }

                        (*rws).free += (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        if matched_count > 0 || allow_zero != 0 {
                            let mut end_subpattern: PCRE2_SPTR = code;
                            let next_state_offset: c_int;

                            loop {
                                end_subpattern =
                                    end_subpattern.add(get(end_subpattern, 1) as usize);
                                if *end_subpattern != OP_ALT {
                                    break;
                                }
                            }
                            next_state_offset = end_subpattern.offset_from(start_code) as c_int
                                + LINK_SIZE as c_int
                                + 1;

                            /* Optimization: skip over the subject string if no more
                            active states and no new states set up. */
                            if i + 1 >= active_count && new_count == 0 {
                                ptr = local_ptr;
                                clen = 0;
                                ADD_NEW!(next_state_offset, 0);
                            } else {
                                let mut p: PCRE2_SPTR = ptr;
                                let pp: PCRE2_SPTR = local_ptr;
                                charcount = pp.offset_from(p) as PCRE2_SIZE;
                                if utf != 0 {
                                    while p < pp {
                                        let b = *p;
                                        p = p.add(1);
                                        if not_firstcu(b as u32) {
                                            charcount -= 1;
                                        }
                                    }
                                }
                                ADD_NEW_DATA!(
                                    -next_state_offset,
                                    0,
                                    (charcount - 1) as c_int
                                );
                            }
                        }
                    }

                    /*-------------------------------------------------------*/
                    v if v == OP_ONCE as u32 => {
                        let rc: c_int;
                        let local_workspace: *mut c_int;
                        let local_offsets: *mut PCRE2_SIZE;
                        let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;

                        if ((*rws).free as usize) < RWS_RSIZE + RWS_OVEC_OSIZE {
                            let r = more_workspace(&mut rws, RWS_OVEC_OSIZE as u32, mb);
                            if r != 0 {
                                return r;
                            }
                            RWS = rws as *mut c_int;
                        }

                        local_offsets =
                            (RWS.add((*rws).size as usize - (*rws).free as usize))
                                as *mut PCRE2_SIZE;
                        local_workspace =
                            (local_offsets as *mut c_int).add(RWS_OVEC_OSIZE);
                        (*rws).free -= (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        rc = internal_dfa_match(
                            mb,
                            code,
                            ptr,
                            ptr.offset_from(start_subject) as PCRE2_SIZE,
                            local_offsets,
                            (RWS_OVEC_OSIZE / OVEC_UNIT) as u32,
                            local_workspace,
                            RWS_RSIZE as c_int,
                            rlevel,
                            RWS,
                        );

                        (*rws).free += (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        if rc >= 0 {
                            let mut end_subpattern: PCRE2_SPTR = code;
                            let mut charcount: PCRE2_SIZE =
                                *local_offsets.add(1) - *local_offsets.add(0);
                            let next_state_offset: c_int;
                            let repeat_state_offset: c_int;

                            loop {
                                end_subpattern =
                                    end_subpattern.add(get(end_subpattern, 1) as usize);
                                if *end_subpattern != OP_ALT {
                                    break;
                                }
                            }
                            next_state_offset = end_subpattern.offset_from(start_code) as c_int
                                + LINK_SIZE as c_int
                                + 1;

                            /* If KETRMAX or KETRMIN, arrange for the repeat state. */
                            repeat_state_offset = if *end_subpattern == OP_KETRMAX
                                || *end_subpattern == OP_KETRMIN
                            {
                                end_subpattern.offset_from(start_code) as c_int
                                    - get(end_subpattern, 1)
                            } else {
                                -1
                            };

                            if charcount == 0 {
                                ADD_ACTIVE!(next_state_offset, 0);
                            } else if i + 1 >= active_count && new_count == 0 {
                                ptr = ptr.add(charcount);
                                clen = 0;
                                ADD_NEW!(next_state_offset, 0);

                                /* If adding a repeat state, fudge things so it is
                                the only current state. */
                                if repeat_state_offset >= 0 {
                                    next_active_state = active_states;
                                    active_count = 0;
                                    i = -1;
                                    ADD_ACTIVE!(repeat_state_offset, 0);
                                }
                            } else {
                                if utf != 0 {
                                    let mut p: PCRE2_SPTR =
                                        start_subject.add(*local_offsets.add(0));
                                    let pp: PCRE2_SPTR =
                                        start_subject.add(*local_offsets.add(1));
                                    while p < pp {
                                        let b = *p;
                                        p = p.add(1);
                                        if not_firstcu(b as u32) {
                                            charcount -= 1;
                                        }
                                    }
                                }
                                ADD_NEW_DATA!(
                                    -next_state_offset,
                                    0,
                                    (charcount - 1) as c_int
                                );
                                if repeat_state_offset >= 0 {
                                    ADD_NEW_DATA!(
                                        -repeat_state_offset,
                                        0,
                                        (charcount - 1) as c_int
                                    );
                                }
                            }
                        } else if rc != PCRE2_ERROR_NOMATCH {
                            return rc;
                        }
                    }

                    /*-------------------------------------------------------*/
                    /* Handle callouts */
                    v if v == OP_CALLOUT as u32 || v == OP_CALLOUT_STR as u32 => {
                        let mut callout_length: PCRE2_SIZE = 0;
                        rrc = do_callout_dfa(
                            code,
                            offsets,
                            current_subject,
                            ptr,
                            mb,
                            0,
                            &mut callout_length,
                        );
                        if rrc < 0 {
                            return rrc; /* Abandon */
                        }
                        if rrc == 0 {
                            ADD_ACTIVE!(state_offset + callout_length as c_int, 0);
                        }
                    }

                    /*-------------------------------------------------------*/
                    _ => {
                        /* Unsupported opcode */
                        return PCRE2_ERROR_DFA_UITEM;
                    }
                    } /* End of match on codevalue */
                } /* End of 'active_state block */

                i += 1;
            } /* End of loop scanning active states */

            /* We have finished the processing at the current subject character. */

            if new_count <= 0 {
                if could_continue != 0
                    && (((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                        || (((*mb).moptions & PCRE2_PARTIAL_SOFT) != 0 && match_count < 0))
                    && (partial_newline != 0
                        || (ptr >= end_subject
                            && (ptr > (*mb).start_used_ptr
                                || (*mb).allowemptypartial != 0)))
                {
                    match_count = PCRE2_ERROR_PARTIAL;
                }
                break 'subject; /* Exit from loop along the subject string */
            }

            /* One or more states are active for the next character. */

            ptr = ptr.add(clen as usize); /* Advance to next subject character */
        } /* Loop to move along the subject string */

        /* Control gets here from "break" above. If we have a match and
        PCRE2_ENDANCHORED is set, the match fails. */

        if match_count >= 0
            && (((*mb).moptions | (*mb).poptions) & PCRE2_ENDANCHORED) != 0
            && ptr < end_subject
        {
            match_count = PCRE2_ERROR_NOMATCH;
        }

        match_count
    }
}

/*************************************************
*     Match a pattern using the DFA algorithm    *
*************************************************/

/// `PRIV`-less public entry point. This is the Rust-callable form; the exported
/// C ABI symbol `pcre2_dfa_match_8` wraps it below.
///
/// Returns: > 0 => number of match offset pairs placed in offsets
///          = 0 => offsets overflowed; longest matches are present
///           -1 => failed to match
///         < -1 => some kind of unexpected problem
pub unsafe fn pcre2_dfa_match(
    code: *const pcre2_real_code,
    mut subject: PCRE2_SPTR,
    mut length: PCRE2_SIZE,
    start_offset: PCRE2_SIZE,
    options: u32,
    match_data: *mut pcre2_real_match_data,
    mcontext: *mut pcre2_real_match_context,
    workspace: *mut c_int,
    wscount: PCRE2_SIZE,
) -> c_int {
    unsafe {
        let mut rc: c_int;

        let re: *const pcre2_real_code = code;
        let original_options = options;

        let null_str: [PCRE2_UCHAR; 1] = [0xcd];
        let original_subject: PCRE2_SPTR = subject;
        let mut start_match: PCRE2_SPTR;
        let mut end_subject: PCRE2_SPTR;
        let mut bumpalong_limit: PCRE2_SPTR;
        let mut req_cu_ptr: PCRE2_SPTR;

        let utf: BOOL;
        let anchored: BOOL;
        let startline: BOOL;
        let firstline: BOOL;
        let mut has_first_cu: BOOL = FALSE;
        let mut has_req_cu: BOOL = FALSE;

        let mut memchr_found_first_cu: PCRE2_SPTR = core::ptr::null();
        let mut memchr_found_first_cu2: PCRE2_SPTR = core::ptr::null();

        let mut first_cu: PCRE2_UCHAR = 0;
        let mut first_cu2: PCRE2_UCHAR = 0;
        let mut req_cu: PCRE2_UCHAR = 0;
        let mut req_cu2: PCRE2_UCHAR = 0;

        let mut start_bits: *const u8 = core::ptr::null();

        let mut cb: pcre2_callout_block = core::mem::zeroed();
        let mut actual_match_block: dfa_match_block = core::mem::zeroed();
        let mb: *mut dfa_match_block = &mut actual_match_block;

        /* Set up a starting block of memory for use during recursive calls. */
        let mut base_recursion_workspace: [c_int; RWS_BASE_SIZE] = [0; RWS_BASE_SIZE];
        let rws: *mut RWS_anchor = base_recursion_workspace.as_mut_ptr() as *mut RWS_anchor;
        (*rws).next = core::ptr::null_mut();
        (*rws).size = RWS_BASE_SIZE as u32;
        (*rws).free = (RWS_BASE_SIZE - RWS_ANCHOR_SIZE) as u32;

        let mut options = options;

        /* Recognize NULL, length 0 as an empty string. */
        if subject.is_null() && length == 0 {
            subject = null_str.as_ptr();
        }

        /* This closure body performs the whole match; `rc` is set and the code
        falls through to the EXIT cleanup. We use a labelled block to emulate the
        C `goto EXIT` / `goto NOMATCH_EXIT`. */

        'exit: {
            /* Plausibility checks */
            if match_data.is_null() {
                return PCRE2_ERROR_NULL;
            }
            if re.is_null() || subject.is_null() || workspace.is_null() {
                rc = PCRE2_ERROR_NULL;
                break 'exit;
            }
            if (options & !PUBLIC_DFA_MATCH_OPTIONS) != 0 {
                rc = PCRE2_ERROR_BADOPTION;
                break 'exit;
            }

            if length == PCRE2_ZERO_TERMINATED {
                length = crate::string_utils::strlen(subject);
            }

            if wscount < 20 {
                rc = PCRE2_ERROR_DFA_WSSIZE;
                break 'exit;
            }
            if start_offset > length {
                rc = PCRE2_ERROR_BADOFFSET;
                break 'exit;
            }

            /* Partial matching and PCRE2_ENDANCHORED are not allowed together. */
            if (options & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT)) != 0
                && (((*re).overall_options | options) & PCRE2_ENDANCHORED) != 0
            {
                rc = PCRE2_ERROR_BADOPTION;
                break 'exit;
            }

            /* Invalid UTF support is not available for DFA matching. */
            if ((*re).overall_options & PCRE2_MATCH_INVALID_UTF) != 0 {
                rc = PCRE2_ERROR_DFA_UINVALID_UTF;
                break 'exit;
            }

            /* Check the magic number. */
            if (*re).magic_number != MAGIC_NUMBER {
                rc = PCRE2_ERROR_BADMAGIC;
                break 'exit;
            }

            /* Check the code unit width. */
            if ((*re).flags & PCRE2_MODE_MASK) != PCRE2_CODE_UNIT_WIDTH / 8 {
                rc = PCRE2_ERROR_BADMODE;
                break 'exit;
            }

            /* Transfer the (*NOTEMPTY) flags into the options, mirroring the C
            Boolean trickery. FF = PCRE2_NOTEMPTY_SET|PCRE2_NE_ATST_SET,
            OO = PCRE2_NOTEMPTY|PCRE2_NOTEMPTY_ATSTART. */
            {
                const FF: u32 = PCRE2_NOTEMPTY_SET | PCRE2_NE_ATST_SET;
                const OO: u32 = PCRE2_NOTEMPTY | PCRE2_NOTEMPTY_ATSTART;
                options |= ((*re).flags & FF) / ((FF & (!FF + 1)) / (OO & (!OO + 1)));
            }

            /* If restarting after a partial match, sanity check the workspace. */
            if (options & PCRE2_DFA_RESTART) != 0 {
                if (*workspace.add(0) & (-2i32)) != 0
                    || *workspace.add(1) < 1
                    || *workspace.add(1)
                        > ((wscount - 2) / INTS_PER_STATEBLOCK as usize) as c_int
                {
                    rc = PCRE2_ERROR_DFA_BADRESTART;
                    break 'exit;
                }
            }

            /* Set some local values */
            utf = (((*re).overall_options & PCRE2_UTF) != 0) as BOOL;
            start_match = subject.add(start_offset);
            end_subject = subject.add(length);
            req_cu_ptr = start_match.sub(1);
            anchored = ((options & (PCRE2_ANCHORED | PCRE2_DFA_RESTART)) != 0
                || ((*re).overall_options & PCRE2_ANCHORED) != 0) as BOOL;

            startline = (((*re).flags & PCRE2_STARTLINE) != 0) as BOOL;
            firstline =
                (anchored == 0 && ((*re).overall_options & PCRE2_FIRSTLINE) != 0) as BOOL;
            bumpalong_limit = end_subject;

            /* Initialize the callout block. */
            (*mb).cb = &mut cb;
            cb.version = 2;
            cb.subject = subject;
            cb.subject_length = end_subject.offset_from(subject) as PCRE2_SIZE;
            cb.callout_flags = 0;
            cb.capture_top = 1; /* No capture support */
            cb.capture_last = 0;
            cb.mark = core::ptr::null(); /* No (*MARK) support */

            /* Get data from the match context. */
            if mcontext.is_null() {
                (*mb).callout = None;
                (*mb).memctl = (*re).memctl;
                let dmc = &raw const crate::context::_pcre2_default_match_context_8;
                (*mb).match_limit = (*dmc).match_limit;
                (*mb).match_limit_depth = (*dmc).depth_limit;
                (*mb).heap_limit = (*dmc).heap_limit;
            } else {
                if (*mcontext).offset_limit != PCRE2_UNSET {
                    if ((*re).overall_options & PCRE2_USE_OFFSET_LIMIT) == 0 {
                        rc = PCRE2_ERROR_BADOFFSETLIMIT;
                        break 'exit;
                    }
                    bumpalong_limit = subject.add((*mcontext).offset_limit);
                }
                (*mb).callout = (*mcontext).callout;
                (*mb).callout_data = (*mcontext).callout_data;
                (*mb).memctl = (*mcontext).memctl;
                (*mb).match_limit = (*mcontext).match_limit;
                (*mb).match_limit_depth = (*mcontext).depth_limit;
                (*mb).heap_limit = (*mcontext).heap_limit;
            }

            if (*mb).match_limit > (*re).limit_match {
                (*mb).match_limit = (*re).limit_match;
            }
            if (*mb).match_limit_depth > (*re).limit_depth {
                (*mb).match_limit_depth = (*re).limit_depth;
            }
            if (*mb).heap_limit > (*re).limit_heap {
                (*mb).heap_limit = (*re).limit_heap;
            }

            (*mb).start_code =
                (re as *const u8).add((*re).code_start) as PCRE2_SPTR;
            (*mb).tables = (*re).tables;
            (*mb).start_subject = subject;
            (*mb).end_subject = end_subject;
            (*mb).start_offset = start_offset;
            (*mb).allowemptypartial = (((*re).max_lookbehind > 0)
                || ((*re).flags & PCRE2_MATCH_EMPTY) != 0)
                as BOOL;
            (*mb).moptions = options;
            (*mb).poptions = (*re).overall_options;
            (*mb).match_call_count = 0;
            (*mb).heap_used = 0;

            /* Process the \R and newline settings. */
            (*mb).bsr_convention = (*re).bsr_convention;
            (*mb).nltype = NLTYPE_FIXED;
            match (*re).newline_convention as u32 {
                PCRE2_NEWLINE_CR => {
                    (*mb).nllen = 1;
                    (*mb).nl[0] = CHAR_CR as u8;
                }
                PCRE2_NEWLINE_LF => {
                    (*mb).nllen = 1;
                    (*mb).nl[0] = CHAR_NL as u8;
                }
                PCRE2_NEWLINE_NUL => {
                    (*mb).nllen = 1;
                    (*mb).nl[0] = CHAR_NUL as u8;
                }
                PCRE2_NEWLINE_CRLF => {
                    (*mb).nllen = 2;
                    (*mb).nl[0] = CHAR_CR as u8;
                    (*mb).nl[1] = CHAR_NL as u8;
                }
                PCRE2_NEWLINE_ANY => {
                    (*mb).nltype = NLTYPE_ANY;
                }
                PCRE2_NEWLINE_ANYCRLF => {
                    (*mb).nltype = NLTYPE_ANYCRLF;
                }
                _ => {
                    rc = PCRE2_ERROR_INTERNAL;
                    break 'exit;
                }
            }

            /* Check a UTF string for validity if required. */
            if utf != 0 && (options & PCRE2_NO_UTF_CHECK) == 0 {
                let mut check_subject: PCRE2_SPTR = start_match;

                if start_offset > 0 {
                    if start_match < end_subject && not_firstcu(*start_match as u32) {
                        rc = PCRE2_ERROR_BADUTFOFFSET;
                        break 'exit;
                    }
                    let mut i: u32 = (*re).max_lookbehind as u32;
                    while i > 0 && check_subject > subject {
                        check_subject = check_subject.sub(1);
                        while check_subject > subject
                            && (*check_subject & 0xc0) == 0x80
                        {
                            check_subject = check_subject.sub(1);
                        }
                        i -= 1;
                    }
                }

                rc = valid_utf(
                    check_subject,
                    length - (check_subject.offset_from(subject) as PCRE2_SIZE),
                    &mut (*match_data).startchar,
                );
                if rc != 0 {
                    (*match_data).startchar +=
                        check_subject.offset_from(subject) as PCRE2_SIZE;
                    break 'exit;
                }
            }

            /* Set up the first code unit to match, if available. */
            if ((*re).flags & PCRE2_FIRSTSET) != 0 {
                has_first_cu = TRUE;
                first_cu = (*re).first_codeunit as PCRE2_UCHAR;
                first_cu2 = first_cu;
                if ((*re).flags & PCRE2_FIRSTCASELESS) != 0 {
                    first_cu2 = table_get(
                        first_cu as u32,
                        (*mb).tables.add(fcc_offset),
                        first_cu as u32,
                    ) as PCRE2_UCHAR;
                    if first_cu > 127
                        && utf == 0
                        && ((*re).overall_options & PCRE2_UCP) != 0
                    {
                        first_cu2 = ucd_othercase(first_cu as u32) as PCRE2_UCHAR;
                    }
                }
            } else if startline == 0 && ((*re).flags & PCRE2_FIRSTMAPSET) != 0 {
                start_bits = (*re).start_bitmap.as_ptr();
            }

            /* There may be a "last known required code unit" set. */
            if ((*re).flags & PCRE2_LASTSET) != 0 {
                has_req_cu = TRUE;
                req_cu = (*re).last_codeunit as PCRE2_UCHAR;
                req_cu2 = req_cu;
                if ((*re).flags & PCRE2_LASTCASELESS) != 0 {
                    req_cu2 = table_get(
                        req_cu as u32,
                        (*mb).tables.add(fcc_offset),
                        req_cu as u32,
                    ) as PCRE2_UCHAR;
                    if req_cu > 127
                        && utf == 0
                        && ((*re).overall_options & PCRE2_UCP) != 0
                    {
                        req_cu2 = ucd_othercase(req_cu as u32) as PCRE2_UCHAR;
                    }
                }
            }

            /* Free a previously copied matched subject if present. */
            if ((*match_data).flags & PCRE2_MD_COPIED_SUBJECT) != 0 {
                ((*match_data).memctl.free.unwrap())(
                    (*match_data).subject as *mut c_void,
                    (*match_data).memctl.memory_data,
                );
                (*match_data).flags &= !PCRE2_MD_COPIED_SUBJECT;
            }

            /* Fill in fields that are always returned. */
            (*match_data).code = re;
            (*match_data).subject = core::ptr::null(); /* Default for match error */
            (*match_data).mark = core::ptr::null();
            (*match_data).matchedby = PCRE2_MATCHEDBY_DFA_INTERPRETER;
            (*match_data).options = original_options;

            /* Main matching loop with bumpalong. */
            let mut nomatch = false;
            'bumpalong: loop {
                /* ---------------- Start of match optimizations --------------- */
                if ((*re).optimization_flags & PCRE2_OPTIM_START_OPTIMIZE) != 0
                    && (options & PCRE2_DFA_RESTART) == 0
                {
                    /* firstline: constrain match to the first line. */
                    if firstline != 0 {
                        let mut t: PCRE2_SPTR = start_match;
                        if utf != 0 {
                            while t < end_subject && !is_newline_at(mb, t, utf) {
                                t = t.add(1);
                                acrosschar_fwd(&mut t, end_subject);
                            }
                        } else {
                            while t < end_subject && !is_newline_at(mb, t, utf) {
                                t = t.add(1);
                            }
                        }
                        end_subject = t;
                    }

                    /* Anchored: check the first code unit if one is recorded. */
                    if anchored != 0 {
                        if has_first_cu != 0 || !start_bits.is_null() {
                            let mut ok = start_match < end_subject;
                            if ok {
                                let cc: PCRE2_UCHAR = *start_match;
                                ok = has_first_cu != 0
                                    && (cc == first_cu || cc == first_cu2);
                                if !ok && !start_bits.is_null() {
                                    ok = (*start_bits.add((cc / 8) as usize)
                                        & (1u8 << (cc & 7)))
                                        != 0;
                                }
                            }
                            if !ok {
                                break 'bumpalong;
                            }
                        }
                    }
                    /* Not anchored. Advance to a unique first code unit. */
                    else {
                        if has_first_cu != 0 {
                            if first_cu != first_cu2 {
                                /* Caseless. 8-bit: use memchr twice with caching. */
                                let mut pp1: PCRE2_SPTR;
                                let mut pp2: PCRE2_SPTR;
                                let searchlength =
                                    end_subject.offset_from(start_match) as usize;

                                if memchr_found_first_cu.is_null()
                                    || start_match > memchr_found_first_cu
                                {
                                    let r = memchr(
                                        start_match as *const c_void,
                                        first_cu as c_int,
                                        searchlength,
                                    );
                                    pp1 = r as PCRE2_SPTR;
                                    memchr_found_first_cu =
                                        if pp1.is_null() { end_subject } else { pp1 };
                                } else {
                                    pp1 = if memchr_found_first_cu == end_subject {
                                        core::ptr::null()
                                    } else {
                                        memchr_found_first_cu
                                    };
                                }

                                if memchr_found_first_cu2.is_null()
                                    || start_match > memchr_found_first_cu2
                                {
                                    let r = memchr(
                                        start_match as *const c_void,
                                        first_cu2 as c_int,
                                        searchlength,
                                    );
                                    pp2 = r as PCRE2_SPTR;
                                    memchr_found_first_cu2 =
                                        if pp2.is_null() { end_subject } else { pp2 };
                                } else {
                                    pp2 = if memchr_found_first_cu2 == end_subject {
                                        core::ptr::null()
                                    } else {
                                        memchr_found_first_cu2
                                    };
                                }

                                if pp1.is_null() {
                                    start_match =
                                        if pp2.is_null() { end_subject } else { pp2 };
                                } else {
                                    start_match = if pp2.is_null() || pp1 < pp2 {
                                        pp1
                                    } else {
                                        pp2
                                    };
                                }
                            } else {
                                /* The caseful case is much simpler. */
                                let r = memchr(
                                    start_match as *const c_void,
                                    first_cu as c_int,
                                    end_subject.offset_from(start_match) as usize,
                                );
                                start_match = r as PCRE2_SPTR;
                                if start_match.is_null() {
                                    start_match = end_subject;
                                }
                            }

                            if ((*mb).moptions
                                & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT))
                                == 0
                                && start_match >= (*mb).end_subject
                            {
                                break 'bumpalong;
                            }
                        }
                        /* Advance to just after a linebreak for multiline. */
                        else if startline != 0 {
                            if start_match > (*mb).start_subject.add(start_offset) {
                                if utf != 0 {
                                    while start_match < end_subject
                                        && !was_newline_at(mb, start_match, utf)
                                    {
                                        start_match = start_match.add(1);
                                        acrosschar_fwd(&mut start_match, end_subject);
                                    }
                                } else {
                                    while start_match < end_subject
                                        && !was_newline_at(mb, start_match, utf)
                                    {
                                        start_match = start_match.add(1);
                                    }
                                }

                                if *start_match.sub(1) as u32 == CHAR_CR
                                    && ((*mb).nltype == NLTYPE_ANY
                                        || (*mb).nltype == NLTYPE_ANYCRLF)
                                    && start_match < end_subject
                                    && *start_match as u32 == CHAR_NL
                                {
                                    start_match = start_match.add(1);
                                }
                            }
                        }
                        /* Advance to a non-unique first code unit if any. */
                        else if !start_bits.is_null() {
                            while start_match < end_subject {
                                let cc: u32 = *start_match as u32;
                                if (*start_bits.add((cc / 8) as usize) & (1u8 << (cc & 7)))
                                    != 0
                                {
                                    break;
                                }
                                start_match = start_match.add(1);
                            }

                            if ((*mb).moptions
                                & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT))
                                == 0
                                && start_match >= (*mb).end_subject
                            {
                                break 'bumpalong;
                            }
                        }
                    } /* End of first code unit handling */

                    /* Restore fudged end_subject */
                    end_subject = (*mb).end_subject;

                    /* The following two optimizations are disabled for partial. */
                    if ((*mb).moptions & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT)) == 0 {
                        let mut p: PCRE2_SPTR;

                        if (end_subject.offset_from(start_match) as PCRE2_SIZE)
                            < (*re).minlength as PCRE2_SIZE
                        {
                            nomatch = true;
                            break 'bumpalong;
                        }

                        p = start_match.add(if has_first_cu != 0 { 1 } else { 0 });
                        if has_req_cu != 0 && p > req_cu_ptr {
                            let check_length =
                                end_subject.offset_from(start_match) as PCRE2_SIZE;

                            if check_length < REQ_CU_MAX
                                || (anchored == 0 && check_length < REQ_CU_MAX * 1000)
                            {
                                if req_cu != req_cu2 {
                                    /* Caseless */
                                    let pp = p;
                                    let r = memchr(
                                        pp as *const c_void,
                                        req_cu as c_int,
                                        end_subject.offset_from(pp) as usize,
                                    );
                                    p = r as PCRE2_SPTR;
                                    if p.is_null() {
                                        let r2 = memchr(
                                            pp as *const c_void,
                                            req_cu2 as c_int,
                                            end_subject.offset_from(pp) as usize,
                                        );
                                        p = r2 as PCRE2_SPTR;
                                        if p.is_null() {
                                            p = end_subject;
                                        }
                                    }
                                } else {
                                    /* The caseful case */
                                    let r = memchr(
                                        p as *const c_void,
                                        req_cu as c_int,
                                        end_subject.offset_from(p) as usize,
                                    );
                                    p = r as PCRE2_SPTR;
                                    if p.is_null() {
                                        p = end_subject;
                                    }
                                }

                                if p >= end_subject {
                                    break 'bumpalong;
                                }

                                req_cu_ptr = p;
                            }
                        }
                    }
                }
                /* ------------ End of start of match optimizations ------------ */

                /* Give no match if we have passed the bumpalong limit. */
                if start_match > bumpalong_limit {
                    break 'bumpalong;
                }

                /* OK, now we can do the business */
                (*mb).start_used_ptr = start_match;
                (*mb).last_used_ptr = start_match;
                (*mb).recursive = core::ptr::null_mut();

                rc = internal_dfa_match(
                    mb,
                    (*mb).start_code,
                    start_match,
                    start_offset,
                    (*match_data).ovector.as_mut_ptr(),
                    (*match_data).oveccount as u32 * 2,
                    workspace,
                    wscount as c_int,
                    0,
                    base_recursion_workspace.as_mut_ptr(),
                );

                /* Anything other than "no match" means we are done. */
                if rc != PCRE2_ERROR_NOMATCH || anchored != 0 {
                    if rc == PCRE2_ERROR_NOMATCH {
                        nomatch = true;
                        break 'bumpalong;
                    }

                    if rc == PCRE2_ERROR_PARTIAL && (*match_data).oveccount > 0 {
                        *(*match_data).ovector.as_mut_ptr().add(0) =
                            start_match.offset_from(subject) as PCRE2_SIZE;
                        *(*match_data).ovector.as_mut_ptr().add(1) =
                            end_subject.offset_from(subject) as PCRE2_SIZE;
                    }

                    if rc >= 0 || rc == PCRE2_ERROR_PARTIAL {
                        (*match_data).subject_length = length;
                        (*match_data).start_offset = start_offset;
                        (*match_data).leftchar =
                            (*mb).start_used_ptr.offset_from(subject) as PCRE2_SIZE;
                        (*match_data).rightchar =
                            (*mb).last_used_ptr.offset_from(subject) as PCRE2_SIZE;
                        (*match_data).startchar =
                            start_match.offset_from(subject) as PCRE2_SIZE;
                    }

                    if rc >= 0 && (options & PCRE2_COPY_MATCHED_SUBJECT) != 0 {
                        if length != 0 {
                            (*match_data).subject = ((*match_data).memctl.malloc.unwrap())(
                                length,
                                (*match_data).memctl.memory_data,
                            )
                                as PCRE2_SPTR;
                            if (*match_data).subject.is_null() {
                                rc = PCRE2_ERROR_NOMEMORY;
                                break 'exit;
                            }
                            memcpy(
                                (*match_data).subject as *mut u8,
                                subject,
                                length,
                            );
                        } else {
                            (*match_data).subject = core::ptr::null();
                        }
                        (*match_data).flags |= PCRE2_MD_COPIED_SUBJECT;
                    } else if rc >= 0 || rc == PCRE2_ERROR_PARTIAL {
                        (*match_data).subject = original_subject;
                    }
                    break 'exit;
                }

                /* Advance to the next subject character. */
                if firstline != 0 && is_newline_at(mb, start_match, utf) {
                    break 'bumpalong;
                }
                start_match = start_match.add(1);
                if utf != 0 {
                    acrosschar_fwd(&mut start_match, end_subject);
                }
                if start_match > end_subject {
                    break 'bumpalong;
                }

                /* Skip CR LF as one if appropriate. */
                if *start_match.sub(1) as u32 == CHAR_CR
                    && start_match < end_subject
                    && *start_match as u32 == CHAR_NL
                    && ((*re).flags & PCRE2_HASCRORLF) == 0
                    && ((*mb).nltype == NLTYPE_ANY
                        || (*mb).nltype == NLTYPE_ANYCRLF
                        || (*mb).nllen == 2)
                {
                    start_match = start_match.add(1);
                }
            } /* "Bumpalong" loop */

            let _ = nomatch;
            /* NOMATCH_EXIT */
            (*match_data).subject = original_subject;
            (*match_data).subject_length = length;
            (*match_data).start_offset = start_offset;
            rc = PCRE2_ERROR_NOMATCH;
        } /* 'exit block */

        /* EXIT: free the RWS chain (all but the base, which is on the stack). */
        {
            let mut rwsp: *mut RWS_anchor = rws;
            while !(*rwsp).next.is_null() {
                let next: *mut RWS_anchor = (*rwsp).next;
                (*rwsp).next = (*next).next;
                ((*mb).memctl.free.unwrap())(next as *mut c_void, (*mb).memctl.memory_data);
            }
        }

        (*match_data).rc = rc;
        rc
    }
}

/// Exported C ABI symbol.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_dfa_match_8(
    code: *const pcre2_real_code,
    subject: PCRE2_SPTR,
    length: PCRE2_SIZE,
    start_offset: PCRE2_SIZE,
    options: u32,
    match_data: *mut pcre2_real_match_data,
    mcontext: *mut pcre2_real_match_context,
    workspace: *mut c_int,
    wscount: PCRE2_SIZE,
) -> c_int {
    unsafe {
        pcre2_dfa_match(
            code,
            subject,
            length,
            start_offset,
            options,
            match_data,
            mcontext,
            workspace,
            wscount,
        )
    }
}
