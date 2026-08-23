/* Translated from pcre2_dfa_match.c
8-bit code units, SUPPORT_UNICODE, SUPPORT_WIDE_CHARS, no JIT, LINK_SIZE == 2,
IMM2_SIZE == 2. */

use crate::internal::*;
use crate::pcre2_pub::*;
use crate::tables::*;
use crate::ucd_data::*;
use crate::ucp::*;
use core::ffi::{c_int, c_uint, c_void};

/* #define NLBLOCK mb  / PSSTART start_subject / PSEND end_subject */

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

/*************************************************
*      Code parameters and static tables         *
*************************************************/

const OP_PROP_EXTRA: u32 = 300;
const OP_EXTUNI_EXTRA: u32 = 320;
const OP_ANYNL_EXTRA: u32 = 340;
const OP_HSPACE_EXTRA: u32 = 360;
const OP_VSPACE_EXTRA: u32 = 380;

/* Combined virtual opcodes, needed as constants for use in match patterns. */

const PROPX_TYPESTAR: u32 = OP_PROP_EXTRA + OP_TYPESTAR;
const PROPX_TYPEMINSTAR: u32 = OP_PROP_EXTRA + OP_TYPEMINSTAR;
const PROPX_TYPEPLUS: u32 = OP_PROP_EXTRA + OP_TYPEPLUS;
const PROPX_TYPEMINPLUS: u32 = OP_PROP_EXTRA + OP_TYPEMINPLUS;
const PROPX_TYPEQUERY: u32 = OP_PROP_EXTRA + OP_TYPEQUERY;
const PROPX_TYPEMINQUERY: u32 = OP_PROP_EXTRA + OP_TYPEMINQUERY;
const PROPX_TYPEUPTO: u32 = OP_PROP_EXTRA + OP_TYPEUPTO;
const PROPX_TYPEMINUPTO: u32 = OP_PROP_EXTRA + OP_TYPEMINUPTO;
const PROPX_TYPEEXACT: u32 = OP_PROP_EXTRA + OP_TYPEEXACT;
const PROPX_TYPEPOSSTAR: u32 = OP_PROP_EXTRA + OP_TYPEPOSSTAR;
const PROPX_TYPEPOSPLUS: u32 = OP_PROP_EXTRA + OP_TYPEPOSPLUS;
const PROPX_TYPEPOSQUERY: u32 = OP_PROP_EXTRA + OP_TYPEPOSQUERY;
const PROPX_TYPEPOSUPTO: u32 = OP_PROP_EXTRA + OP_TYPEPOSUPTO;

const EXTUNIX_TYPESTAR: u32 = OP_EXTUNI_EXTRA + OP_TYPESTAR;
const EXTUNIX_TYPEMINSTAR: u32 = OP_EXTUNI_EXTRA + OP_TYPEMINSTAR;
const EXTUNIX_TYPEPLUS: u32 = OP_EXTUNI_EXTRA + OP_TYPEPLUS;
const EXTUNIX_TYPEMINPLUS: u32 = OP_EXTUNI_EXTRA + OP_TYPEMINPLUS;
const EXTUNIX_TYPEQUERY: u32 = OP_EXTUNI_EXTRA + OP_TYPEQUERY;
const EXTUNIX_TYPEMINQUERY: u32 = OP_EXTUNI_EXTRA + OP_TYPEMINQUERY;
const EXTUNIX_TYPEUPTO: u32 = OP_EXTUNI_EXTRA + OP_TYPEUPTO;
const EXTUNIX_TYPEMINUPTO: u32 = OP_EXTUNI_EXTRA + OP_TYPEMINUPTO;
const EXTUNIX_TYPEEXACT: u32 = OP_EXTUNI_EXTRA + OP_TYPEEXACT;
const EXTUNIX_TYPEPOSSTAR: u32 = OP_EXTUNI_EXTRA + OP_TYPEPOSSTAR;
const EXTUNIX_TYPEPOSPLUS: u32 = OP_EXTUNI_EXTRA + OP_TYPEPOSPLUS;
const EXTUNIX_TYPEPOSQUERY: u32 = OP_EXTUNI_EXTRA + OP_TYPEPOSQUERY;
const EXTUNIX_TYPEPOSUPTO: u32 = OP_EXTUNI_EXTRA + OP_TYPEPOSUPTO;

const ANYNLX_TYPESTAR: u32 = OP_ANYNL_EXTRA + OP_TYPESTAR;
const ANYNLX_TYPEMINSTAR: u32 = OP_ANYNL_EXTRA + OP_TYPEMINSTAR;
const ANYNLX_TYPEPLUS: u32 = OP_ANYNL_EXTRA + OP_TYPEPLUS;
const ANYNLX_TYPEMINPLUS: u32 = OP_ANYNL_EXTRA + OP_TYPEMINPLUS;
const ANYNLX_TYPEQUERY: u32 = OP_ANYNL_EXTRA + OP_TYPEQUERY;
const ANYNLX_TYPEMINQUERY: u32 = OP_ANYNL_EXTRA + OP_TYPEMINQUERY;
const ANYNLX_TYPEUPTO: u32 = OP_ANYNL_EXTRA + OP_TYPEUPTO;
const ANYNLX_TYPEMINUPTO: u32 = OP_ANYNL_EXTRA + OP_TYPEMINUPTO;
const ANYNLX_TYPEEXACT: u32 = OP_ANYNL_EXTRA + OP_TYPEEXACT;
const ANYNLX_TYPEPOSSTAR: u32 = OP_ANYNL_EXTRA + OP_TYPEPOSSTAR;
const ANYNLX_TYPEPOSPLUS: u32 = OP_ANYNL_EXTRA + OP_TYPEPOSPLUS;
const ANYNLX_TYPEPOSQUERY: u32 = OP_ANYNL_EXTRA + OP_TYPEPOSQUERY;
const ANYNLX_TYPEPOSUPTO: u32 = OP_ANYNL_EXTRA + OP_TYPEPOSUPTO;

const HSPACEX_TYPESTAR: u32 = OP_HSPACE_EXTRA + OP_TYPESTAR;
const HSPACEX_TYPEMINSTAR: u32 = OP_HSPACE_EXTRA + OP_TYPEMINSTAR;
const HSPACEX_TYPEPLUS: u32 = OP_HSPACE_EXTRA + OP_TYPEPLUS;
const HSPACEX_TYPEMINPLUS: u32 = OP_HSPACE_EXTRA + OP_TYPEMINPLUS;
const HSPACEX_TYPEQUERY: u32 = OP_HSPACE_EXTRA + OP_TYPEQUERY;
const HSPACEX_TYPEMINQUERY: u32 = OP_HSPACE_EXTRA + OP_TYPEMINQUERY;
const HSPACEX_TYPEUPTO: u32 = OP_HSPACE_EXTRA + OP_TYPEUPTO;
const HSPACEX_TYPEMINUPTO: u32 = OP_HSPACE_EXTRA + OP_TYPEMINUPTO;
const HSPACEX_TYPEEXACT: u32 = OP_HSPACE_EXTRA + OP_TYPEEXACT;
const HSPACEX_TYPEPOSSTAR: u32 = OP_HSPACE_EXTRA + OP_TYPEPOSSTAR;
const HSPACEX_TYPEPOSPLUS: u32 = OP_HSPACE_EXTRA + OP_TYPEPOSPLUS;
const HSPACEX_TYPEPOSQUERY: u32 = OP_HSPACE_EXTRA + OP_TYPEPOSQUERY;
const HSPACEX_TYPEPOSUPTO: u32 = OP_HSPACE_EXTRA + OP_TYPEPOSUPTO;

const VSPACEX_TYPESTAR: u32 = OP_VSPACE_EXTRA + OP_TYPESTAR;
const VSPACEX_TYPEMINSTAR: u32 = OP_VSPACE_EXTRA + OP_TYPEMINSTAR;
const VSPACEX_TYPEPLUS: u32 = OP_VSPACE_EXTRA + OP_TYPEPLUS;
const VSPACEX_TYPEMINPLUS: u32 = OP_VSPACE_EXTRA + OP_TYPEMINPLUS;
const VSPACEX_TYPEQUERY: u32 = OP_VSPACE_EXTRA + OP_TYPEQUERY;
const VSPACEX_TYPEMINQUERY: u32 = OP_VSPACE_EXTRA + OP_TYPEMINQUERY;
const VSPACEX_TYPEUPTO: u32 = OP_VSPACE_EXTRA + OP_TYPEUPTO;
const VSPACEX_TYPEMINUPTO: u32 = OP_VSPACE_EXTRA + OP_TYPEMINUPTO;
const VSPACEX_TYPEEXACT: u32 = OP_VSPACE_EXTRA + OP_TYPEEXACT;
const VSPACEX_TYPEPOSSTAR: u32 = OP_VSPACE_EXTRA + OP_TYPEPOSSTAR;
const VSPACEX_TYPEPOSPLUS: u32 = OP_VSPACE_EXTRA + OP_TYPEPOSPLUS;
const VSPACEX_TYPEPOSQUERY: u32 = OP_VSPACE_EXTRA + OP_TYPEPOSQUERY;
const VSPACEX_TYPEPOSUPTO: u32 = OP_VSPACE_EXTRA + OP_TYPEPOSUPTO;

/* This table identifies those opcodes that are followed immediately by a
character that is to be tested in some way. */

static coptable: [u8; OP_TABLE_LENGTH] = [
    0, /* End                                    */
    0, 0, 0, 0, 0, /* \A, \G, \K, \B, \b                     */
    0, 0, 0, 0, 0, 0, /* \D, \d, \S, \s, \W, \w                 */
    0, 0, 0, /* Any, AllAny, Anybyte                   */
    0, 0, /* \P, \p                                 */
    0, 0, 0, 0, 0, /* \R, \H, \h, \V, \v                     */
    0, /* \X                                     */
    0, 0, 0, 0, 0, 0, /* \Z, \z, $, $M, ^, ^M                   */
    1, /* Char                                   */
    1, /* Chari                                  */
    1, /* not                                    */
    1, /* noti                                   */
    /* Positive single-char repeats                                          */
    1, 1, 1, 1, 1, 1, /* *, *?, +, +?, ?, ??                    */
    3, 3, /* upto, minupto                          */
    3, /* exact                                  */
    1, 1, 1, 3, /* *+, ++, ?+, upto+                      */
    1, 1, 1, 1, 1, 1, /* *I, *?I, +I, +?I, ?I, ??I              */
    3, 3, /* upto I, minupto I                      */
    3, /* exact I                                */
    1, 1, 1, 3, /* *+I, ++I, ?+I, upto+I                  */
    /* Negative single-char repeats - only for chars < 256                   */
    1, 1, 1, 1, 1, 1, /* NOT *, *?, +, +?, ?, ??                */
    3, 3, /* NOT upto, minupto                      */
    3, /* NOT exact                              */
    1, 1, 1, 3, /* NOT *+, ++, ?+, upto+                  */
    1, 1, 1, 1, 1, 1, /* NOT *I, *?I, +I, +?I, ?I, ??I          */
    3, 3, /* NOT upto I, minupto I                  */
    3, /* NOT exact I                            */
    1, 1, 1, 3, /* NOT *+I, ++I, ?+I, upto+I              */
    /* Positive type repeats                                                 */
    1, 1, 1, 1, 1, 1, /* Type *, *?, +, +?, ?, ??               */
    3, 3, /* Type upto, minupto                     */
    3, /* Type exact                             */
    1, 1, 1, 3, /* Type *+, ++, ?+, upto+                 */
    /* Character class & ref repeats                                         */
    0, 0, 0, 0, 0, 0, /* *, *?, +, +?, ?, ??                    */
    0, 0, /* CRRANGE, CRMINRANGE                    */
    0, 0, 0, 0, /* Possessive *+, ++, ?+, CRPOSRANGE      */
    0, /* CLASS                                  */
    0, /* NCLASS                                 */
    0, /* XCLASS - variable length               */
    0, /* ECLASS - variable length               */
    0, /* REF                                    */
    0, /* REFI                                   */
    0, /* DNREF                                  */
    0, /* DNREFI                                 */
    0, /* RECURSE                                */
    0, /* CALLOUT                                */
    0, /* CALLOUT_STR                            */
    0, /* Alt                                    */
    0, /* Ket                                    */
    0, /* KetRmax                                */
    0, /* KetRmin                                */
    0, /* KetRpos                                */
    0, 0, /* Reverse, Vreverse                      */
    0, /* Assert                                 */
    0, /* Assert not                             */
    0, /* Assert behind                          */
    0, /* Assert behind not                      */
    0, /* NA assert                              */
    0, /* NA assert behind                       */
    0, /* Assert scan substring                  */
    0, /* ONCE                                   */
    0, /* SCRIPT_RUN                             */
    0, 0, 0, 0, 0, /* BRA, BRAPOS, CBRA, CBRAPOS, COND       */
    0, 0, 0, 0, 0, /* SBRA, SBRAPOS, SCBRA, SCBRAPOS, SCOND  */
    0, 0, /* CREF, DNCREF                           */
    0, 0, /* RREF, DNRREF                           */
    0, 0, /* FALSE, TRUE                            */
    0, 0, 0, /* BRAZERO, BRAMINZERO, BRAPOSZERO        */
    0, 0, 0, /* MARK, PRUNE, PRUNE_ARG                 */
    0, 0, 0, 0, /* SKIP, SKIP_ARG, THEN, THEN_ARG         */
    0, 0, /* COMMIT, COMMIT_ARG                     */
    0, 0, 0, /* FAIL, ACCEPT, ASSERT_ACCEPT            */
    0, 0, 0, /* CLOSE, SKIPZERO, DEFINE                */
    0, 0, /* \B and \b in UCP mode                  */
];

/* This table identifies those opcodes that inspect a character. */

static poptable: [u8; OP_TABLE_LENGTH] = [
    0, /* End                                    */
    0, 0, 0, 1, 1, /* \A, \G, \K, \B, \b                     */
    1, 1, 1, 1, 1, 1, /* \D, \d, \S, \s, \W, \w                 */
    1, 1, 1, /* Any, AllAny, Anybyte                   */
    1, 1, /* \P, \p                                 */
    1, 1, 1, 1, 1, /* \R, \H, \h, \V, \v                     */
    1, /* \X                                     */
    0, 0, 0, 0, 0, 0, /* \Z, \z, $, $M, ^, ^M                   */
    1, /* Char                                   */
    1, /* Chari                                  */
    1, /* not                                    */
    1, /* noti                                   */
    /* Positive single-char repeats                                          */
    1, 1, 1, 1, 1, 1, /* *, *?, +, +?, ?, ??                    */
    1, 1, 1, /* upto, minupto, exact                   */
    1, 1, 1, 1, /* *+, ++, ?+, upto+                      */
    1, 1, 1, 1, 1, 1, /* *I, *?I, +I, +?I, ?I, ??I              */
    1, 1, 1, /* upto I, minupto I, exact I             */
    1, 1, 1, 1, /* *+I, ++I, ?+I, upto+I                  */
    /* Negative single-char repeats - only for chars < 256                   */
    1, 1, 1, 1, 1, 1, /* NOT *, *?, +, +?, ?, ??                */
    1, 1, 1, /* NOT upto, minupto, exact               */
    1, 1, 1, 1, /* NOT *+, ++, ?+, upto+                  */
    1, 1, 1, 1, 1, 1, /* NOT *I, *?I, +I, +?I, ?I, ??I          */
    1, 1, 1, /* NOT upto I, minupto I, exact I         */
    1, 1, 1, 1, /* NOT *+I, ++I, ?+I, upto+I              */
    /* Positive type repeats                                                 */
    1, 1, 1, 1, 1, 1, /* Type *, *?, +, +?, ?, ??               */
    1, 1, 1, /* Type upto, minupto, exact              */
    1, 1, 1, 1, /* Type *+, ++, ?+, upto+                 */
    /* Character class & ref repeats                                         */
    1, 1, 1, 1, 1, 1, /* *, *?, +, +?, ?, ??                    */
    1, 1, /* CRRANGE, CRMINRANGE                    */
    1, 1, 1, 1, /* Possessive *+, ++, ?+, CRPOSRANGE      */
    1, /* CLASS                                  */
    1, /* NCLASS                                 */
    1, /* XCLASS - variable length               */
    1, /* ECLASS - variable length               */
    0, /* REF                                    */
    0, /* REFI                                   */
    0, /* DNREF                                  */
    0, /* DNREFI                                 */
    0, /* RECURSE                                */
    0, /* CALLOUT                                */
    0, /* CALLOUT_STR                            */
    0, /* Alt                                    */
    0, /* Ket                                    */
    0, /* KetRmax                                */
    0, /* KetRmin                                */
    0, /* KetRpos                                */
    0, 0, /* Reverse, Vreverse                      */
    0, /* Assert                                 */
    0, /* Assert not                             */
    0, /* Assert behind                          */
    0, /* Assert behind not                      */
    0, /* NA assert                              */
    0, /* NA assert behind                       */
    0, /* Assert scan substring                  */
    0, /* ONCE                                   */
    0, /* SCRIPT_RUN                             */
    0, 0, 0, 0, 0, /* BRA, BRAPOS, CBRA, CBRAPOS, COND       */
    0, 0, 0, 0, 0, /* SBRA, SBRAPOS, SCBRA, SCBRAPOS, SCOND  */
    0, 0, /* CREF, DNCREF                           */
    0, 0, /* RREF, DNRREF                           */
    0, 0, /* FALSE, TRUE                            */
    0, 0, 0, /* BRAZERO, BRAMINZERO, BRAPOSZERO        */
    0, 0, 0, /* MARK, PRUNE, PRUNE_ARG                 */
    0, 0, 0, 0, /* SKIP, SKIP_ARG, THEN, THEN_ARG         */
    0, 0, /* COMMIT, COMMIT_ARG                     */
    0, 0, 0, /* FAIL, ACCEPT, ASSERT_ACCEPT            */
    0, 0, 0, /* CLOSE, SKIPZERO, DEFINE                */
    1, 1, /* \B and \b in UCP mode                  */
];

/* These 2 tables allow for compact code for testing for \D, \d, \S, \s, \W,
and \w */

static toptable1: [u8; 14] = [
    0,
    0,
    0,
    0,
    0,
    0,
    ctype_digit,
    ctype_digit,
    ctype_space,
    ctype_space,
    ctype_word,
    ctype_word,
    0,
    0, /* OP_ANY, OP_ALLANY */
];

static toptable2: [u8; 14] = [
    0,
    0,
    0,
    0,
    0,
    0,
    ctype_digit,
    0,
    ctype_space,
    0,
    ctype_word,
    0,
    1,
    1, /* OP_ANY, OP_ALLANY */
];

/* Structure for holding data about a particular state. */

#[repr(C)]
#[derive(Copy, Clone)]
struct stateblock {
    offset: c_int, /* Offset to opcode (-ve has meaning) */
    count: c_int,  /* Count for repeats */
    data: c_int,   /* Some use extra data */
}

const INTS_PER_STATEBLOCK: c_int =
    (core::mem::size_of::<stateblock>() / core::mem::size_of::<c_int>()) as c_int;

const OVEC_UNIT: usize = core::mem::size_of::<PCRE2_SIZE>() / core::mem::size_of::<c_int>();

const RWS_BASE_SIZE: usize = DFA_START_RWS_SIZE / core::mem::size_of::<c_int>();
const RWS_RSIZE: usize = 1000;
const RWS_OVEC_RSIZE: usize = 1000 * OVEC_UNIT;
const RWS_OVEC_OSIZE: usize = 2 * OVEC_UNIT;

/* This structure is at the start of each workspace block. */

#[repr(C)]
#[derive(Copy, Clone)]
struct RWS_anchor {
    next: *mut RWS_anchor,
    size: u32, /* Number of ints */
    free: u32, /* Number of ints */
}

const RWS_ANCHOR_SIZE: usize =
    core::mem::size_of::<RWS_anchor>() / core::mem::size_of::<c_int>();

/*************************************************
*               Process a callout                *
*************************************************/

unsafe fn do_callout_dfa(
    code: PCRE2_SPTR,
    offsets: *mut PCRE2_SIZE,
    current_subject: PCRE2_SPTR,
    ptr: PCRE2_SPTR,
    mb: *mut dfa_match_block,
    extracode: PCRE2_SIZE,
    lengthptr: *mut PCRE2_SIZE,
) -> c_int {
    let cb: *mut pcre2_callout_block = (*mb).cb;

    *lengthptr = if *code.add(extracode) as u32 == OP_CALLOUT {
        _pcre2_OP_lengths_8[OP_CALLOUT as usize] as PCRE2_SIZE
    } else {
        GET(code, 1 + 2 * LINK_SIZE + extracode) as PCRE2_SIZE
    };

    if (*mb).callout.is_none() {
        return 0;
    } /* No callout provided */

    /* Fixed fields in the callout block are set once and for all at the start of
    matching. */

    (*cb).offset_vector = offsets;
    (*cb).start_match = current_subject.offset_from((*mb).start_subject) as PCRE2_SIZE;
    (*cb).current_position = ptr.offset_from((*mb).start_subject) as PCRE2_SIZE;
    (*cb).pattern_position = GET(code, 1 + extracode) as PCRE2_SIZE;
    (*cb).next_item_length = GET(code, 1 + LINK_SIZE + extracode) as PCRE2_SIZE;

    if *code.add(extracode) as u32 == OP_CALLOUT {
        (*cb).callout_number = *code.add(1 + 2 * LINK_SIZE + extracode) as u32;
        (*cb).callout_string_offset = 0;
        (*cb).callout_string = core::ptr::null();
        (*cb).callout_string_length = 0;
    } else {
        (*cb).callout_number = 0;
        (*cb).callout_string_offset = GET(code, 1 + 3 * LINK_SIZE + extracode) as PCRE2_SIZE;
        (*cb).callout_string = code.add(1 + 4 * LINK_SIZE + extracode).add(1);
        (*cb).callout_string_length = (*lengthptr)
            .wrapping_sub(1 + 4 * LINK_SIZE)
            .wrapping_sub(2);
    }

    ((*mb).callout.unwrap())(cb, (*mb).callout_data)
}

/*************************************************
*         Expand local workspace memory          *
*************************************************/

unsafe fn more_workspace(
    rwsptr: *mut *mut RWS_anchor,
    ovecsize: c_uint,
    mb: *mut dfa_match_block,
) -> c_int {
    let rws: *mut RWS_anchor = *rwsptr;
    let new_: *mut RWS_anchor;

    if !(*rws).next.is_null() {
        new_ = (*rws).next;
    }
    /* Sizes in the RWS_anchor blocks are in units of sizeof(int), but
    mb->heap_limit and mb->heap_used are in kibibytes. Play carefully, to avoid
    overflow. */
    else {
        let mut newsize: u32 = if (*rws).size as usize
            >= (u32::MAX as usize) / (core::mem::size_of::<c_int>() * 2)
        {
            ((u32::MAX as usize) / core::mem::size_of::<c_int>()) as u32
        } else {
            (*rws).size.wrapping_mul(2)
        };
        let mut newsizeK: u32 = newsize / ((1024 / core::mem::size_of::<c_int>()) as u32);

        if newsizeK as usize + (*mb).heap_used > (*mb).heap_limit as usize {
            newsizeK = ((*mb).heap_limit as usize).wrapping_sub((*mb).heap_used) as u32;
        }
        newsize = newsizeK.wrapping_mul((1024 / core::mem::size_of::<c_int>()) as u32);

        if (newsize as usize) < RWS_RSIZE + ovecsize as usize + RWS_ANCHOR_SIZE {
            return PCRE2_ERROR_HEAPLIMIT;
        }
        new_ = ((*mb).memctl.malloc.unwrap())(
            newsize as usize * core::mem::size_of::<c_int>(),
            (*mb).memctl.memory_data,
        ) as *mut RWS_anchor;
        if new_.is_null() {
            return PCRE2_ERROR_NOMEMORY;
        }
        (*mb).heap_used += newsizeK as PCRE2_SIZE;
        (*new_).next = core::ptr::null_mut();
        (*new_).size = newsize;
        (*rws).next = new_;
    }

    (*new_).free = ((*new_).size as usize).wrapping_sub(RWS_ANCHOR_SIZE) as u32;
    *rwsptr = new_;
    0
}

/* HSPACE_CASES / VSPACE_CASES from pcre2_internal.h (non-EBCDIC). */

macro_rules! HSPACE_CASES {
    () => {
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
    };
}

macro_rules! VSPACE_CASES {
    () => {
        CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028 | 0x2029
    };
}

/*************************************************
*     Match a Regular Expression - DFA engine    *
*************************************************/

unsafe fn internal_dfa_match(
    mb: *mut dfa_match_block,
    this_start_code: PCRE2_SPTR,
    current_subject: PCRE2_SPTR,
    start_offset: PCRE2_SIZE,
    offsets: *mut PCRE2_SIZE,
    offsetcount: u32,
    workspace: *mut c_int,
    wscount: c_int,
    rlevel: u32,
    RWS: *mut c_int,
) -> c_int {
    let mut current_subject = current_subject;
    let mut offsetcount = offsetcount;
    let mut wscount = wscount;
    let mut rlevel = rlevel;
    let mut RWS = RWS;

    let mut active_states: *mut stateblock;
    let mut new_states: *mut stateblock;
    let mut temp_states: *mut stateblock;
    let mut next_active_state: *mut stateblock = core::ptr::null_mut();
    let mut next_new_state: *mut stateblock;
    let ctypes: *const u8;
    let lcc: *const u8;
    let fcc: *const u8;
    let mut ptr: PCRE2_SPTR;
    let mut end_code: PCRE2_SPTR;
    let mut new_recursive: dfa_recursion_info = core::mem::zeroed();
    let mut active_count: c_int = 0;
    let mut new_count: c_int;
    let mut match_count: c_int;

    /* Some fields in the mb block are frequently referenced, so we load them into
    independent variables in the hope that this will perform better. */

    let start_subject: PCRE2_SPTR = (*mb).start_subject;
    let end_subject: PCRE2_SPTR = (*mb).end_subject;
    let start_code: PCRE2_SPTR = (*mb).start_code;

    let utf: BOOL = if ((*mb).poptions & PCRE2_UTF) != 0 {
        TRUE
    } else {
        FALSE
    };
    let utf_or_ucp: BOOL = if utf != 0 || ((*mb).poptions & PCRE2_UCP) != 0 {
        TRUE
    } else {
        FALSE
    };

    let mut reset_could_continue: BOOL = FALSE;

    /* IS_NEWLINE / WAS_NEWLINE with NLBLOCK == mb, PSSTART == start_subject,
    PSEND == end_subject. */

    macro_rules! IS_NEWLINE {
        ($p:expr) => {
            (if (*mb).nltype != NLTYPE_FIXED {
                ($p) < (*mb).end_subject
                    && crate::newline::_pcre2_is_newline_8(
                        ($p),
                        (*mb).nltype,
                        (*mb).end_subject,
                        &mut (*mb).nllen,
                        utf,
                    ) != 0
            } else {
                ($p) <= (*mb).end_subject.wrapping_sub((*mb).nllen as usize)
                    && *($p) == (*mb).nl[0]
                    && ((*mb).nllen == 1 || *($p).add(1) == (*mb).nl[1])
            })
        };
    }

    macro_rules! WAS_NEWLINE {
        ($p:expr) => {
            (if (*mb).nltype != NLTYPE_FIXED {
                ($p) > (*mb).start_subject
                    && crate::newline::_pcre2_was_newline_8(
                        ($p),
                        (*mb).nltype,
                        (*mb).start_subject,
                        &mut (*mb).nllen,
                        utf,
                    ) != 0
            } else {
                ($p) >= (*mb).start_subject.wrapping_add((*mb).nllen as usize)
                    && *($p).wrapping_sub((*mb).nllen as usize) == (*mb).nl[0]
                    && ((*mb).nllen == 1
                        || *($p).wrapping_sub((*mb).nllen as usize).add(1) == (*mb).nl[1])
            })
        };
    }

    if {
        let t = (*mb).match_call_count;
        (*mb).match_call_count = t.wrapping_add(1);
        t
    } >= (*mb).match_limit
    {
        return PCRE2_ERROR_MATCHLIMIT;
    }
    if {
        let t = rlevel;
        rlevel = rlevel.wrapping_add(1);
        t
    } > (*mb).match_limit_depth
    {
        return PCRE2_ERROR_DEPTHLIMIT;
    }
    offsetcount &= (-2i32) as u32; /* Round down */

    wscount -= 2;
    wscount = (wscount - (wscount % (INTS_PER_STATEBLOCK * 2))) / (2 * INTS_PER_STATEBLOCK);

    ctypes = (*mb).tables.add(ctypes_offset);
    lcc = (*mb).tables.add(lcc_offset);
    fcc = (*mb).tables.add(fcc_offset);

    match_count = PCRE2_ERROR_NOMATCH; /* A negative number */

    active_states = workspace.add(2) as *mut stateblock;
    new_states = active_states.offset(wscount as isize);
    next_new_state = new_states;
    new_count = 0;

    /* The state-adding macros. */

    macro_rules! ADD_ACTIVE {
        ($x:expr, $y:expr) => {{
            let _oc = active_count;
            active_count = active_count.wrapping_add(1);
            if _oc < wscount {
                (*next_active_state).offset = ($x);
                (*next_active_state).count = ($y);
                next_active_state = next_active_state.offset(1);
            } else {
                return PCRE2_ERROR_DFA_WSSIZE;
            }
        }};
    }

    /* ADD_ACTIVE_DATA is defined in the C source but never used there. */
    #[allow(unused_macros)]
    macro_rules! ADD_ACTIVE_DATA {
        ($x:expr, $y:expr, $z:expr) => {{
            let _oc = active_count;
            active_count = active_count.wrapping_add(1);
            if _oc < wscount {
                (*next_active_state).offset = ($x);
                (*next_active_state).count = ($y);
                (*next_active_state).data = ($z);
                next_active_state = next_active_state.offset(1);
            } else {
                return PCRE2_ERROR_DFA_WSSIZE;
            }
        }};
    }

    macro_rules! ADD_NEW {
        ($x:expr, $y:expr) => {{
            let _nc = new_count;
            new_count = new_count.wrapping_add(1);
            if _nc < wscount {
                (*next_new_state).offset = ($x);
                (*next_new_state).count = ($y);
                next_new_state = next_new_state.offset(1);
            } else {
                return PCRE2_ERROR_DFA_WSSIZE;
            }
        }};
    }

    macro_rules! ADD_NEW_DATA {
        ($x:expr, $y:expr, $z:expr) => {{
            let _nc = new_count;
            new_count = new_count.wrapping_add(1);
            if _nc < wscount {
                (*next_new_state).offset = ($x);
                (*next_new_state).count = ($y);
                (*next_new_state).data = ($z);
                next_new_state = next_new_state.offset(1);
            } else {
                return PCRE2_ERROR_DFA_WSSIZE;
            }
        }};
    }

    /* The first thing in any (sub) pattern is a bracket of some sort. */

    if *this_start_code as u32 == OP_ASSERTBACK
        || *this_start_code as u32 == OP_ASSERTBACK_NOT
    {
        let mut max_back: usize = 0;
        let mut gone_back: usize = 0;

        end_code = this_start_code;
        loop {
            let back: usize = GET2(end_code, 2 + LINK_SIZE) as usize;
            if back > max_back {
                max_back = back;
            }
            end_code = end_code.add(GET(end_code, 1) as usize);
            if *end_code as u32 != OP_ALT {
                break;
            }
        }

        /* If we can't go back the amount required for the longest lookbehind
        pattern, go back as far as we can; some alternatives may still be viable. */

        /* In character mode we have to step back character by character */

        if utf != 0 {
            gone_back = 0;
            while gone_back < max_back {
                if current_subject <= start_subject {
                    break;
                }
                current_subject = current_subject.sub(1);
                while current_subject > start_subject && (*current_subject & 0xc0) == 0x80 {
                    current_subject = current_subject.sub(1);
                }
                gone_back += 1;
            }
        } else {
            /* In byte-mode we can do this quickly. */
            let current_offset: usize = current_subject.offset_from(start_subject) as usize;
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

        /* Now we can process the individual branches. */

        end_code = this_start_code;
        loop {
            let revlen: u32 = if *end_code.add(1 + LINK_SIZE) as u32 == OP_REVERSE {
                (1 + IMM2_SIZE) as u32
            } else {
                0
            };
            let back: usize = if revlen == 0 {
                0
            } else {
                GET2(end_code, 2 + LINK_SIZE) as usize
            };
            if back <= gone_back {
                let bstate: c_int = (end_code.offset_from(start_code) as usize
                    + 1
                    + LINK_SIZE
                    + revlen as usize) as c_int;
                ADD_NEW_DATA!(-bstate, 0, (gone_back - back) as c_int);
            }
            end_code = end_code.add(GET(end_code, 1) as usize);
            if *end_code as u32 != OP_ALT {
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
                end_code = end_code.add(GET(end_code, 1) as usize);
                if *end_code as u32 != OP_ALT {
                    break;
                }
            }
            new_count = *workspace.add(1);
            if *workspace.add(0) == 0 {
                memcpy(
                    new_states as *mut c_void,
                    active_states as *const c_void,
                    new_count as usize * core::mem::size_of::<stateblock>(),
                );
            }
        }
        /* Not restarting */
        else {
            let mut length: c_int = 1
                + LINK_SIZE as c_int
                + (if *this_start_code as u32 == OP_CBRA
                    || *this_start_code as u32 == OP_SCBRA
                    || *this_start_code as u32 == OP_CBRAPOS
                    || *this_start_code as u32 == OP_SCBRAPOS
                {
                    IMM2_SIZE as c_int
                } else {
                    0
                });
            loop {
                ADD_NEW!(end_code.offset_from(start_code) as c_int + length, 0);
                end_code = end_code.add(GET(end_code, 1) as usize);
                length = 1 + LINK_SIZE as c_int;
                if *end_code as u32 != OP_ALT {
                    break;
                }
            }
        }
    }

    *workspace.add(0) = 0; /* Bit indicating which vector is current */

    /* Loop for scanning the subject */

    ptr = current_subject;
    loop {
        let mut i: c_int;
        let mut j: c_int;
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

        *workspace.add(0) ^= 1; /* Remember for the restarting feature */
        *workspace.add(1) = active_count;

        /* Set the pointers for adding new states */

        next_active_state = active_states.offset(active_count as isize);
        next_new_state = new_states;

        /* Load the current character from the subject outside the loop. */

        if ptr < end_subject {
            clen = 1; /* Number of data items in the character */
            /* GETCHARLENTEST(c, ptr, clen) */
            c = *ptr as u32;
            if utf != 0 && c >= 0xc0 {
                clen += utf8_extra(c) as c_int;
                c = getutf8(c, ptr);
            }
        } else {
            clen = 0; /* This indicates the end of the subject */
            c = NOTACHAR; /* This value should never actually be used */
        }

        /* Scan up the active states and act on each one. */

        i = 0;
        while i < active_count {
            'NEXT_ACTIVE_STATE: {
                let current_state: *mut stateblock = active_states.offset(i as isize);
                let mut caseless: BOOL = FALSE;
                let mut code: PCRE2_SPTR;
                let mut codevalue: u32;
                let mut state_offset: c_int = (*current_state).offset;
                let mut rrc: c_int;
                let mut count: c_int = 0;

                /* A negative offset is a special case meaning "hold off going to this
                (negated) state until the number of characters in the data field have
                been skipped". */

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
                        break 'NEXT_ACTIVE_STATE;
                    } else {
                        state_offset = -state_offset;
                        (*current_state).offset = state_offset;
                    }
                }

                /* Check for a duplicate state with the same count, and skip if found. */

                j = 0;
                while j < i {
                    if (*active_states.offset(j as isize)).offset == state_offset
                        && (*active_states.offset(j as isize)).count == (*current_state).count
                    {
                        break 'NEXT_ACTIVE_STATE;
                    }
                    j += 1;
                }

                /* The state offset is the offset to the opcode */

                code = start_code.offset(state_offset as isize);
                codevalue = *code as u32;

                /* If this opcode inspects a character, but we are at the end of the
                subject, remember the fact for use when testing for a partial match. */

                if clen == 0 && poptable[codevalue as usize] != 0 {
                    could_continue = TRUE;
                }

                /* If this opcode is followed by an inline character, load it. */

                if coptable[codevalue as usize] > 0 {
                    dlen = 1;
                    if utf != 0 {
                        /* GETCHARLEN(d, code + coptable[codevalue], dlen) */
                        let dp: PCRE2_SPTR = code.add(coptable[codevalue as usize] as usize);
                        d = *dp as u32;
                        if d >= 0xc0 {
                            dlen += utf8_extra(d) as c_int;
                            d = getutf8(d, dp);
                        }
                    } else {
                        d = *code.add(coptable[codevalue as usize] as usize) as u32;
                    }
                    if codevalue >= OP_TYPESTAR {
                        match d {
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

                /* Now process the individual opcodes */

                let switch_value: u32 = codevalue;
                match switch_value {
                    /* ============================================================= */
                    /* Reached a closing bracket. */
                    OP_KET | OP_KETRMIN | OP_KETRMAX | OP_KETRPOS => {
                        if code != end_code {
                            ADD_ACTIVE!(state_offset + 1 + LINK_SIZE as c_int, 0);
                            if codevalue != OP_KET {
                                ADD_ACTIVE!(state_offset - GET(code, 1) as c_int, 0);
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
                                } else if match_count > 0
                                    && {
                                        match_count += 1;
                                        match_count * 2 > offsetcount as c_int
                                    }
                                {
                                    match_count = 0;
                                }
                                count = (if match_count == 0 {
                                    offsetcount as c_int
                                } else {
                                    match_count * 2
                                }) - 2;
                                if count > 0 {
                                    memmove(
                                        offsets.add(2) as *mut c_void,
                                        offsets as *const c_void,
                                        count as usize * core::mem::size_of::<PCRE2_SIZE>(),
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

                    /* ============================================================= */
                    /* These opcodes add to the current list of states without looking
                    at the current character. */
                    OP_ALT => {
                        loop {
                            code = code.add(GET(code, 1) as usize);
                            if *code as u32 != OP_ALT {
                                break;
                            }
                        }
                        ADD_ACTIVE!(code.offset_from(start_code) as c_int, 0);
                    }

                    OP_BRA | OP_SBRA => loop {
                        ADD_ACTIVE!(
                            code.offset_from(start_code) as c_int + 1 + LINK_SIZE as c_int,
                            0
                        );
                        code = code.add(GET(code, 1) as usize);
                        if *code as u32 != OP_ALT {
                            break;
                        }
                    },

                    OP_CBRA | OP_SCBRA => {
                        ADD_ACTIVE!(
                            code.offset_from(start_code) as c_int
                                + 1
                                + LINK_SIZE as c_int
                                + IMM2_SIZE as c_int,
                            0
                        );
                        code = code.add(GET(code, 1) as usize);
                        while *code as u32 == OP_ALT {
                            ADD_ACTIVE!(
                                code.offset_from(start_code) as c_int + 1 + LINK_SIZE as c_int,
                                0
                            );
                            code = code.add(GET(code, 1) as usize);
                        }
                    }

                    OP_BRAZERO | OP_BRAMINZERO => {
                        ADD_ACTIVE!(state_offset + 1, 0);
                        code = code.add(1 + GET(code, 2) as usize);
                        while *code as u32 == OP_ALT {
                            code = code.add(GET(code, 1) as usize);
                        }
                        ADD_ACTIVE!(
                            code.offset_from(start_code) as c_int + 1 + LINK_SIZE as c_int,
                            0
                        );
                    }

                    OP_SKIPZERO => {
                        code = code.add(1 + GET(code, 2) as usize);
                        while *code as u32 == OP_ALT {
                            code = code.add(GET(code, 1) as usize);
                        }
                        ADD_ACTIVE!(
                            code.offset_from(start_code) as c_int + 1 + LINK_SIZE as c_int,
                            0
                        );
                    }

                    OP_CIRC => {
                        if ptr == start_subject && ((*mb).moptions & PCRE2_NOTBOL) == 0 {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    OP_CIRCM => {
                        if (ptr == start_subject && ((*mb).moptions & PCRE2_NOTBOL) == 0)
                            || ((ptr != end_subject
                                || ((*mb).poptions & PCRE2_ALT_CIRCUMFLEX) != 0)
                                && WAS_NEWLINE!(ptr))
                        {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    OP_EOD => {
                        if ptr >= end_subject {
                            if ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                                return PCRE2_ERROR_PARTIAL;
                            } else {
                                ADD_ACTIVE!(state_offset + 1, 0);
                            }
                        }
                    }

                    OP_SOD => {
                        if ptr == start_subject {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    OP_SOM => {
                        if ptr == start_subject.add(start_offset) {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    /* ============================================================= */
                    /* These opcodes inspect the next subject character. */
                    OP_ANY => {
                        if clen > 0 && !IS_NEWLINE!(ptr) {
                            if ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                partial_newline = TRUE;
                                could_continue = partial_newline;
                            } else {
                                ADD_NEW!(state_offset + 1, 0);
                            }
                        }
                    }

                    OP_ALLANY => {
                        if clen > 0 {
                            ADD_NEW!(state_offset + 1, 0);
                        }
                    }

                    OP_EODN => {
                        if clen == 0
                            || (IS_NEWLINE!(ptr)
                                && ptr == end_subject.wrapping_sub((*mb).nllen as usize))
                        {
                            if ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                                return PCRE2_ERROR_PARTIAL;
                            }
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    OP_DOLL => {
                        if ((*mb).moptions & PCRE2_NOTEOL) == 0 {
                            if clen == 0 && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                                could_continue = TRUE;
                            } else if clen == 0
                                || (((*mb).poptions & PCRE2_DOLLAR_ENDONLY) == 0
                                    && IS_NEWLINE!(ptr)
                                    && (ptr == end_subject.wrapping_sub((*mb).nllen as usize)))
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
                                    partial_newline = TRUE;
                                    could_continue = partial_newline;
                                }
                            }
                        }
                    }

                    OP_DOLLM => {
                        if ((*mb).moptions & PCRE2_NOTEOL) == 0 {
                            if clen == 0 && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                                could_continue = TRUE;
                            } else if clen == 0
                                || (((*mb).poptions & PCRE2_DOLLAR_ENDONLY) == 0
                                    && IS_NEWLINE!(ptr))
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
                                    partial_newline = TRUE;
                                    could_continue = partial_newline;
                                }
                            }
                        } else if IS_NEWLINE!(ptr) {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    OP_DIGIT | OP_WHITESPACE | OP_WORDCHAR => {
                        if clen > 0
                            && c < 256
                            && ((*ctypes.add(c as usize) & toptable1[codevalue as usize])
                                ^ toptable2[codevalue as usize])
                                != 0
                        {
                            ADD_NEW!(state_offset + 1, 0);
                        }
                    }

                    OP_NOT_DIGIT | OP_NOT_WHITESPACE | OP_NOT_WORDCHAR => {
                        if clen > 0
                            && (c >= 256
                                || ((*ctypes.add(c as usize) & toptable1[codevalue as usize])
                                    ^ toptable2[codevalue as usize])
                                    != 0)
                        {
                            ADD_NEW!(state_offset + 1, 0);
                        }
                    }

                    OP_WORD_BOUNDARY
                    | OP_NOT_WORD_BOUNDARY
                    | OP_NOT_UCP_WORD_BOUNDARY
                    | OP_UCP_WORD_BOUNDARY => {
                        let left_word: c_int;
                        let right_word: c_int;

                        if ptr > start_subject {
                            let mut temp: PCRE2_SPTR = ptr.sub(1);
                            if temp < (*mb).start_used_ptr {
                                (*mb).start_used_ptr = temp;
                            }
                            if utf != 0 {
                                /* BACKCHAR(temp) */
                                while (*temp & 0xc0) == 0x80 {
                                    temp = temp.sub(1);
                                }
                            }
                            /* GETCHARTEST(d, temp) */
                            d = *temp as u32;
                            if utf != 0 && d >= 0xc0 {
                                d = getutf8(d, temp);
                            }
                            if codevalue == OP_UCP_WORD_BOUNDARY
                                || codevalue == OP_NOT_UCP_WORD_BOUNDARY
                            {
                                let chartype: c_int = UCD_CHARTYPE(d) as c_int;
                                let category: c_int =
                                    _pcre2_ucp_gentype_8[chartype as usize] as c_int;
                                left_word = (category == ucp_L as c_int
                                    || category == ucp_N as c_int
                                    || chartype == ucp_Mn as c_int
                                    || chartype == ucp_Pc as c_int) as c_int;
                            } else {
                                left_word = (d < 256
                                    && (*ctypes.add(d as usize) & ctype_word) != 0)
                                    as c_int;
                            }
                        } else {
                            left_word = FALSE;
                        }

                        if clen > 0 {
                            if ptr >= (*mb).last_used_ptr {
                                let mut temp: PCRE2_SPTR = ptr.add(1);
                                if utf != 0 {
                                    /* FORWARDCHARTEST(temp, mb->end_subject) */
                                    while temp < (*mb).end_subject && (*temp & 0xc0) == 0x80 {
                                        temp = temp.add(1);
                                    }
                                }
                                (*mb).last_used_ptr = temp;
                            }
                            if codevalue == OP_UCP_WORD_BOUNDARY
                                || codevalue == OP_NOT_UCP_WORD_BOUNDARY
                            {
                                let chartype: c_int = UCD_CHARTYPE(c) as c_int;
                                let category: c_int =
                                    _pcre2_ucp_gentype_8[chartype as usize] as c_int;
                                right_word = (category == ucp_L as c_int
                                    || category == ucp_N as c_int
                                    || chartype == ucp_Mn as c_int
                                    || chartype == ucp_Pc as c_int) as c_int;
                            } else {
                                right_word = (c < 256
                                    && (*ctypes.add(c as usize) & ctype_word) != 0)
                                    as c_int;
                            }
                        } else {
                            right_word = FALSE;
                        }

                        if (left_word == right_word)
                            == (codevalue == OP_NOT_WORD_BOUNDARY
                                || codevalue == OP_NOT_UCP_WORD_BOUNDARY)
                        {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    /* Check the next character by Unicode property. */
                    OP_PROP | OP_NOTPROP => {
                        if clen > 0 {
                            let mut OK: BOOL = FALSE;
                            let mut chartype: c_int;
                            let prop: &ucd_record = GET_UCD(c);
                            match *code.add(1) as u32 {
                                PT_LAMP => {
                                    chartype = prop.chartype as c_int;
                                    OK = (chartype == ucp_Lu as c_int
                                        || chartype == ucp_Ll as c_int
                                        || chartype == ucp_Lt as c_int)
                                        as BOOL;
                                }

                                PT_GC => {
                                    OK = (_pcre2_ucp_gentype_8[prop.chartype as usize]
                                        == *code.add(2) as u32) as BOOL;
                                }

                                PT_PC => {
                                    OK = (prop.chartype as u32 == *code.add(2) as u32) as BOOL;
                                }

                                PT_SC => {
                                    OK = (prop.script as u32 == *code.add(2) as u32) as BOOL;
                                }

                                PT_SCX => {
                                    OK = (prop.script as u32 == *code.add(2) as u32
                                        || script_set_bit(
                                            UCD_SCRIPTX_PROP(prop) as usize,
                                            *code.add(2) as u32,
                                        )) as BOOL;
                                }

                                PT_ALNUM => {
                                    chartype = prop.chartype as c_int;
                                    OK = (_pcre2_ucp_gentype_8[chartype as usize] == ucp_L
                                        || _pcre2_ucp_gentype_8[chartype as usize] == ucp_N)
                                        as BOOL;
                                }

                                PT_SPACE | PT_PXSPACE => {
                                    OK = match c {
                                        HSPACE_CASES!() | VSPACE_CASES!() => TRUE,
                                        _ => (_pcre2_ucp_gentype_8[prop.chartype as usize]
                                            == ucp_Z) as BOOL,
                                    };
                                }

                                PT_WORD => {
                                    chartype = prop.chartype as c_int;
                                    OK = (_pcre2_ucp_gentype_8[chartype as usize] == ucp_L
                                        || _pcre2_ucp_gentype_8[chartype as usize] == ucp_N
                                        || chartype == ucp_Mn as c_int
                                        || chartype == ucp_Pc as c_int) as BOOL;
                                }

                                PT_CLIST => {
                                    let mut cp: usize = *code.add(2) as usize;
                                    loop {
                                        if c < _pcre2_ucd_caseless_sets_8[cp] {
                                            OK = FALSE;
                                            break;
                                        }
                                        let cv = _pcre2_ucd_caseless_sets_8[cp];
                                        cp += 1;
                                        if c == cv {
                                            OK = TRUE;
                                            break;
                                        }
                                    }
                                }

                                PT_UCNC => {
                                    OK = (c == CHAR_DOLLAR_SIGN
                                        || c == CHAR_COMMERCIAL_AT
                                        || c == CHAR_GRAVE_ACCENT
                                        || (c >= 0xa0 && c <= 0xd7ff)
                                        || c >= 0xe000) as BOOL;
                                }

                                PT_BIDICL => {
                                    OK = (UCD_BIDICLASS(c) == *code.add(2) as u32) as BOOL;
                                }

                                PT_BOOL => {
                                    OK = boolprop_set_bit(
                                        UCD_BPROPS_PROP(prop) as usize,
                                        *code.add(2) as u32,
                                    ) as BOOL;
                                }

                                /* Should never occur, but keep compilers from grumbling. */
                                _ => {
                                    OK = (codevalue != OP_PROP) as BOOL;
                                }
                            }

                            if OK == (codevalue == OP_PROP) as BOOL {
                                ADD_NEW!(state_offset + 3, 0);
                            }
                        }
                    }

                    /* ============================================================= */
                    /* These opcodes likewise inspect the subject character, but have an
                    argument that is not a data character. The value is loaded into d. */
                    OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEPOSPLUS => {
                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 2, 0);
                        }
                        if clen > 0 {
                            if d == OP_ANY
                                && ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                partial_newline = TRUE;
                                could_continue = partial_newline;
                            } else if (c >= 256
                                && d != OP_DIGIT
                                && d != OP_WHITESPACE
                                && d != OP_WORDCHAR)
                                || (c < 256
                                    && (d != OP_ANY || !IS_NEWLINE!(ptr))
                                    && ((*ctypes.add(c as usize) & toptable1[d as usize])
                                        ^ toptable2[d as usize])
                                        != 0)
                            {
                                if count > 0 && codevalue == OP_TYPEPOSPLUS {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.offset(-1);
                                }
                                count += 1;
                                ADD_NEW!(state_offset, count);
                            }
                        }
                    }

                    OP_TYPEQUERY | OP_TYPEMINQUERY | OP_TYPEPOSQUERY => {
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            if d == OP_ANY
                                && ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                partial_newline = TRUE;
                                could_continue = partial_newline;
                            } else if (c >= 256
                                && d != OP_DIGIT
                                && d != OP_WHITESPACE
                                && d != OP_WORDCHAR)
                                || (c < 256
                                    && (d != OP_ANY || !IS_NEWLINE!(ptr))
                                    && ((*ctypes.add(c as usize) & toptable1[d as usize])
                                        ^ toptable2[d as usize])
                                        != 0)
                            {
                                if codevalue == OP_TYPEPOSQUERY {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.offset(-1);
                                }
                                ADD_NEW!(state_offset + 2, 0);
                            }
                        }
                    }

                    OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPOSSTAR => {
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            if d == OP_ANY
                                && ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                partial_newline = TRUE;
                                could_continue = partial_newline;
                            } else if (c >= 256
                                && d != OP_DIGIT
                                && d != OP_WHITESPACE
                                && d != OP_WORDCHAR)
                                || (c < 256
                                    && (d != OP_ANY || !IS_NEWLINE!(ptr))
                                    && ((*ctypes.add(c as usize) & toptable1[d as usize])
                                        ^ toptable2[d as usize])
                                        != 0)
                            {
                                if codevalue == OP_TYPEPOSSTAR {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.offset(-1);
                                }
                                ADD_NEW!(state_offset, 0);
                            }
                        }
                    }

                    OP_TYPEEXACT => {
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            if d == OP_ANY
                                && ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                partial_newline = TRUE;
                                could_continue = partial_newline;
                            } else if (c >= 256
                                && d != OP_DIGIT
                                && d != OP_WHITESPACE
                                && d != OP_WORDCHAR)
                                || (c < 256
                                    && (d != OP_ANY || !IS_NEWLINE!(ptr))
                                    && ((*ctypes.add(c as usize) & toptable1[d as usize])
                                        ^ toptable2[d as usize])
                                        != 0)
                            {
                                count += 1;
                                if count >= GET2(code, 1) as c_int {
                                    ADD_NEW!(state_offset + 1 + IMM2_SIZE as c_int + 1, 0);
                                } else {
                                    ADD_NEW!(state_offset, count);
                                }
                            }
                        }
                    }

                    OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEPOSUPTO => {
                        ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as c_int, 0);
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            if d == OP_ANY
                                && ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                partial_newline = TRUE;
                                could_continue = partial_newline;
                            } else if (c >= 256
                                && d != OP_DIGIT
                                && d != OP_WHITESPACE
                                && d != OP_WORDCHAR)
                                || (c < 256
                                    && (d != OP_ANY || !IS_NEWLINE!(ptr))
                                    && ((*ctypes.add(c as usize) & toptable1[d as usize])
                                        ^ toptable2[d as usize])
                                        != 0)
                            {
                                if codevalue == OP_TYPEPOSUPTO {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.offset(-1);
                                }
                                count += 1;
                                if count >= GET2(code, 1) as c_int {
                                    ADD_NEW!(state_offset + 2 + IMM2_SIZE as c_int, 0);
                                } else {
                                    ADD_NEW!(state_offset, count);
                                }
                            }
                        }
                    }

                    /* ============================================================= */
                    /* These are virtual opcodes that are used when something like
                    OP_TYPEPLUS has OP_PROP, OP_NOTPROP, OP_ANYNL, or OP_EXTUNI as its
                    argument. The argument is in the d variable. */
                    PROPX_TYPEPLUS | PROPX_TYPEMINPLUS | PROPX_TYPEPOSPLUS => {
                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 4, 0);
                        }
                        if clen > 0 {
                            let mut OK: BOOL = FALSE;
                            let mut chartype: c_int;
                            let prop: &ucd_record = GET_UCD(c);
                            match *code.add(2) as u32 {
                                PT_LAMP => {
                                    chartype = prop.chartype as c_int;
                                    OK = (chartype == ucp_Lu as c_int
                                        || chartype == ucp_Ll as c_int
                                        || chartype == ucp_Lt as c_int)
                                        as BOOL;
                                }
                                PT_GC => {
                                    OK = (_pcre2_ucp_gentype_8[prop.chartype as usize]
                                        == *code.add(3) as u32) as BOOL;
                                }
                                PT_PC => {
                                    OK = (prop.chartype as u32 == *code.add(3) as u32) as BOOL;
                                }
                                PT_SC => {
                                    OK = (prop.script as u32 == *code.add(3) as u32) as BOOL;
                                }
                                PT_SCX => {
                                    OK = (prop.script as u32 == *code.add(3) as u32
                                        || script_set_bit(
                                            UCD_SCRIPTX_PROP(prop) as usize,
                                            *code.add(3) as u32,
                                        )) as BOOL;
                                }
                                PT_ALNUM => {
                                    chartype = prop.chartype as c_int;
                                    OK = (_pcre2_ucp_gentype_8[chartype as usize] == ucp_L
                                        || _pcre2_ucp_gentype_8[chartype as usize] == ucp_N)
                                        as BOOL;
                                }
                                PT_SPACE | PT_PXSPACE => {
                                    OK = match c {
                                        HSPACE_CASES!() | VSPACE_CASES!() => TRUE,
                                        _ => (_pcre2_ucp_gentype_8[prop.chartype as usize]
                                            == ucp_Z) as BOOL,
                                    };
                                }
                                PT_WORD => {
                                    chartype = prop.chartype as c_int;
                                    OK = (_pcre2_ucp_gentype_8[chartype as usize] == ucp_L
                                        || _pcre2_ucp_gentype_8[chartype as usize] == ucp_N
                                        || chartype == ucp_Mn as c_int
                                        || chartype == ucp_Pc as c_int) as BOOL;
                                }
                                PT_CLIST => {
                                    let mut cp: usize = *code.add(3) as usize;
                                    loop {
                                        if c < _pcre2_ucd_caseless_sets_8[cp] {
                                            OK = FALSE;
                                            break;
                                        }
                                        let cv = _pcre2_ucd_caseless_sets_8[cp];
                                        cp += 1;
                                        if c == cv {
                                            OK = TRUE;
                                            break;
                                        }
                                    }
                                }
                                PT_UCNC => {
                                    OK = (c == CHAR_DOLLAR_SIGN
                                        || c == CHAR_COMMERCIAL_AT
                                        || c == CHAR_GRAVE_ACCENT
                                        || (c >= 0xa0 && c <= 0xd7ff)
                                        || c >= 0xe000) as BOOL;
                                }
                                PT_BIDICL => {
                                    OK = (UCD_BIDICLASS(c) == *code.add(3) as u32) as BOOL;
                                }
                                PT_BOOL => {
                                    OK = boolprop_set_bit(
                                        UCD_BPROPS_PROP(prop) as usize,
                                        *code.add(3) as u32,
                                    ) as BOOL;
                                }
                                _ => {
                                    OK = (codevalue != OP_PROP) as BOOL;
                                }
                            }

                            if OK == (d == OP_PROP) as BOOL {
                                if count > 0 && codevalue == PROPX_TYPEPOSPLUS {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.offset(-1);
                                }
                                count += 1;
                                ADD_NEW!(state_offset, count);
                            }
                        }
                    }

                    EXTUNIX_TYPEPLUS | EXTUNIX_TYPEMINPLUS | EXTUNIX_TYPEPOSPLUS => {
                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 2, 0);
                        }
                        if clen > 0 {
                            let mut ncount: c_int = 0;
                            if count > 0 && codevalue == EXTUNIX_TYPEPOSPLUS {
                                active_count -= 1; /* Remove non-match possibility */
                                next_active_state = next_active_state.offset(-1);
                            }
                            crate::extuni::_pcre2_extuni_8(
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

                    ANYNLX_TYPEPLUS | ANYNLX_TYPEMINPLUS | ANYNLX_TYPEPOSPLUS => {
                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 2, 0);
                        }
                        if clen > 0 {
                            let mut ncount: c_int = 0;
                            let mut anynl01: bool = false;
                            match c {
                                CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                    if (*mb).bsr_convention as u32 != PCRE2_BSR_ANYCRLF {
                                        anynl01 = true;
                                    }
                                }
                                CHAR_CR => {
                                    if ptr.add(1) < end_subject
                                        && *ptr.add(1) as u32 == CHAR_LF
                                    {
                                        ncount = 1;
                                    }
                                    anynl01 = true;
                                }
                                CHAR_LF => {
                                    anynl01 = true;
                                }
                                _ => {}
                            }
                            if anynl01 {
                                if count > 0 && codevalue == ANYNLX_TYPEPOSPLUS {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.offset(-1);
                                }
                                count += 1;
                                ADD_NEW_DATA!(-state_offset, count, ncount);
                            }
                        }
                    }

                    VSPACEX_TYPEPLUS | VSPACEX_TYPEMINPLUS | VSPACEX_TYPEPOSPLUS => {
                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 2, 0);
                        }
                        if clen > 0 {
                            let OK: BOOL = match c {
                                VSPACE_CASES!() => TRUE,
                                _ => FALSE,
                            };

                            if OK == (d == OP_VSPACE) as BOOL {
                                if count > 0 && codevalue == VSPACEX_TYPEPOSPLUS {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.offset(-1);
                                }
                                count += 1;
                                ADD_NEW_DATA!(-state_offset, count, 0);
                            }
                        }
                    }

                    HSPACEX_TYPEPLUS | HSPACEX_TYPEMINPLUS | HSPACEX_TYPEPOSPLUS => {
                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 2, 0);
                        }
                        if clen > 0 {
                            let OK: BOOL = match c {
                                HSPACE_CASES!() => TRUE,
                                _ => FALSE,
                            };

                            if OK == (d == OP_HSPACE) as BOOL {
                                if count > 0 && codevalue == HSPACEX_TYPEPOSPLUS {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.offset(-1);
                                }
                                count += 1;
                                ADD_NEW_DATA!(-state_offset, count, 0);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    PROPX_TYPEQUERY
                    | PROPX_TYPEMINQUERY
                    | PROPX_TYPEPOSQUERY
                    | PROPX_TYPESTAR
                    | PROPX_TYPEMINSTAR
                    | PROPX_TYPEPOSSTAR => {
                        count = if codevalue == PROPX_TYPEQUERY
                            || codevalue == PROPX_TYPEMINQUERY
                            || codevalue == PROPX_TYPEPOSQUERY
                        {
                            4
                        } else {
                            0
                        };

                        /* QS1: */
                        ADD_ACTIVE!(state_offset + 4, 0);
                        if clen > 0 {
                            let mut OK: BOOL = FALSE;
                            let mut chartype: c_int;
                            let prop: &ucd_record = GET_UCD(c);
                            match *code.add(2) as u32 {
                                PT_LAMP => {
                                    chartype = prop.chartype as c_int;
                                    OK = (chartype == ucp_Lu as c_int
                                        || chartype == ucp_Ll as c_int
                                        || chartype == ucp_Lt as c_int)
                                        as BOOL;
                                }
                                PT_GC => {
                                    OK = (_pcre2_ucp_gentype_8[prop.chartype as usize]
                                        == *code.add(3) as u32) as BOOL;
                                }
                                PT_PC => {
                                    OK = (prop.chartype as u32 == *code.add(3) as u32) as BOOL;
                                }
                                PT_SC => {
                                    OK = (prop.script as u32 == *code.add(3) as u32) as BOOL;
                                }
                                PT_SCX => {
                                    OK = (prop.script as u32 == *code.add(3) as u32
                                        || script_set_bit(
                                            UCD_SCRIPTX_PROP(prop) as usize,
                                            *code.add(3) as u32,
                                        )) as BOOL;
                                }
                                PT_ALNUM => {
                                    chartype = prop.chartype as c_int;
                                    OK = (_pcre2_ucp_gentype_8[chartype as usize] == ucp_L
                                        || _pcre2_ucp_gentype_8[chartype as usize] == ucp_N)
                                        as BOOL;
                                }
                                PT_SPACE | PT_PXSPACE => {
                                    OK = match c {
                                        HSPACE_CASES!() | VSPACE_CASES!() => TRUE,
                                        _ => (_pcre2_ucp_gentype_8[prop.chartype as usize]
                                            == ucp_Z) as BOOL,
                                    };
                                }
                                PT_WORD => {
                                    chartype = prop.chartype as c_int;
                                    OK = (_pcre2_ucp_gentype_8[chartype as usize] == ucp_L
                                        || _pcre2_ucp_gentype_8[chartype as usize] == ucp_N
                                        || chartype == ucp_Mn as c_int
                                        || chartype == ucp_Pc as c_int) as BOOL;
                                }
                                PT_CLIST => {
                                    let mut cp: usize = *code.add(3) as usize;
                                    loop {
                                        if c < _pcre2_ucd_caseless_sets_8[cp] {
                                            OK = FALSE;
                                            break;
                                        }
                                        let cv = _pcre2_ucd_caseless_sets_8[cp];
                                        cp += 1;
                                        if c == cv {
                                            OK = TRUE;
                                            break;
                                        }
                                    }
                                }
                                PT_UCNC => {
                                    OK = (c == CHAR_DOLLAR_SIGN
                                        || c == CHAR_COMMERCIAL_AT
                                        || c == CHAR_GRAVE_ACCENT
                                        || (c >= 0xa0 && c <= 0xd7ff)
                                        || c >= 0xe000) as BOOL;
                                }
                                PT_BIDICL => {
                                    OK = (UCD_BIDICLASS(c) == *code.add(3) as u32) as BOOL;
                                }
                                PT_BOOL => {
                                    OK = boolprop_set_bit(
                                        UCD_BPROPS_PROP(prop) as usize,
                                        *code.add(3) as u32,
                                    ) as BOOL;
                                }
                                _ => {
                                    OK = (codevalue != OP_PROP) as BOOL;
                                }
                            }

                            if OK == (d == OP_PROP) as BOOL {
                                if codevalue == PROPX_TYPEPOSSTAR
                                    || codevalue == PROPX_TYPEPOSQUERY
                                {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.offset(-1);
                                }
                                ADD_NEW!(state_offset + count, 0);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    EXTUNIX_TYPEQUERY
                    | EXTUNIX_TYPEMINQUERY
                    | EXTUNIX_TYPEPOSQUERY
                    | EXTUNIX_TYPESTAR
                    | EXTUNIX_TYPEMINSTAR
                    | EXTUNIX_TYPEPOSSTAR => {
                        count = if codevalue == EXTUNIX_TYPEQUERY
                            || codevalue == EXTUNIX_TYPEMINQUERY
                            || codevalue == EXTUNIX_TYPEPOSQUERY
                        {
                            2
                        } else {
                            0
                        };

                        /* QS2: */
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            let mut ncount: c_int = 0;
                            if codevalue == EXTUNIX_TYPEPOSSTAR
                                || codevalue == EXTUNIX_TYPEPOSQUERY
                            {
                                active_count -= 1; /* Remove non-match possibility */
                                next_active_state = next_active_state.offset(-1);
                            }
                            crate::extuni::_pcre2_extuni_8(
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

                    /*-----------------------------------------------------------------*/
                    ANYNLX_TYPEQUERY
                    | ANYNLX_TYPEMINQUERY
                    | ANYNLX_TYPEPOSQUERY
                    | ANYNLX_TYPESTAR
                    | ANYNLX_TYPEMINSTAR
                    | ANYNLX_TYPEPOSSTAR => {
                        count = if codevalue == ANYNLX_TYPEQUERY
                            || codevalue == ANYNLX_TYPEMINQUERY
                            || codevalue == ANYNLX_TYPEPOSQUERY
                        {
                            2
                        } else {
                            0
                        };

                        /* QS3: */
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            let mut ncount: c_int = 0;
                            let mut anynl02: bool = false;
                            match c {
                                CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                    if (*mb).bsr_convention as u32 != PCRE2_BSR_ANYCRLF {
                                        anynl02 = true;
                                    }
                                }
                                CHAR_CR => {
                                    if ptr.add(1) < end_subject
                                        && *ptr.add(1) as u32 == CHAR_LF
                                    {
                                        ncount = 1;
                                    }
                                    anynl02 = true;
                                }
                                CHAR_LF => {
                                    anynl02 = true;
                                }
                                _ => {}
                            }
                            if anynl02 {
                                if codevalue == ANYNLX_TYPEPOSSTAR
                                    || codevalue == ANYNLX_TYPEPOSQUERY
                                {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.offset(-1);
                                }
                                ADD_NEW_DATA!(-(state_offset + count), 0, ncount);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    VSPACEX_TYPEQUERY
                    | VSPACEX_TYPEMINQUERY
                    | VSPACEX_TYPEPOSQUERY
                    | VSPACEX_TYPESTAR
                    | VSPACEX_TYPEMINSTAR
                    | VSPACEX_TYPEPOSSTAR => {
                        count = if codevalue == VSPACEX_TYPEQUERY
                            || codevalue == VSPACEX_TYPEMINQUERY
                            || codevalue == VSPACEX_TYPEPOSQUERY
                        {
                            2
                        } else {
                            0
                        };

                        /* QS4: */
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            let OK: BOOL = match c {
                                VSPACE_CASES!() => TRUE,
                                _ => FALSE,
                            };
                            if OK == (d == OP_VSPACE) as BOOL {
                                if codevalue == VSPACEX_TYPEPOSSTAR
                                    || codevalue == VSPACEX_TYPEPOSQUERY
                                {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.offset(-1);
                                }
                                ADD_NEW_DATA!(-(state_offset + count), 0, 0);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    HSPACEX_TYPEQUERY
                    | HSPACEX_TYPEMINQUERY
                    | HSPACEX_TYPEPOSQUERY
                    | HSPACEX_TYPESTAR
                    | HSPACEX_TYPEMINSTAR
                    | HSPACEX_TYPEPOSSTAR => {
                        count = if codevalue == HSPACEX_TYPEQUERY
                            || codevalue == HSPACEX_TYPEMINQUERY
                            || codevalue == HSPACEX_TYPEPOSQUERY
                        {
                            2
                        } else {
                            0
                        };

                        /* QS5: */
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            let OK: BOOL = match c {
                                HSPACE_CASES!() => TRUE,
                                _ => FALSE,
                            };

                            if OK == (d == OP_HSPACE) as BOOL {
                                if codevalue == HSPACEX_TYPEPOSSTAR
                                    || codevalue == HSPACEX_TYPEPOSQUERY
                                {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.offset(-1);
                                }
                                ADD_NEW_DATA!(-(state_offset + count), 0, 0);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    PROPX_TYPEEXACT
                    | PROPX_TYPEUPTO
                    | PROPX_TYPEMINUPTO
                    | PROPX_TYPEPOSUPTO => {
                        if codevalue != PROPX_TYPEEXACT {
                            ADD_ACTIVE!(state_offset + 1 + IMM2_SIZE as c_int + 3, 0);
                        }
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let mut OK: BOOL = FALSE;
                            let mut chartype: c_int;
                            let prop: &ucd_record = GET_UCD(c);
                            match *code.add(1 + IMM2_SIZE + 1) as u32 {
                                PT_LAMP => {
                                    chartype = prop.chartype as c_int;
                                    OK = (chartype == ucp_Lu as c_int
                                        || chartype == ucp_Ll as c_int
                                        || chartype == ucp_Lt as c_int)
                                        as BOOL;
                                }
                                PT_GC => {
                                    OK = (_pcre2_ucp_gentype_8[prop.chartype as usize]
                                        == *code.add(1 + IMM2_SIZE + 2) as u32)
                                        as BOOL;
                                }
                                PT_PC => {
                                    OK = (prop.chartype as u32
                                        == *code.add(1 + IMM2_SIZE + 2) as u32)
                                        as BOOL;
                                }
                                PT_SC => {
                                    OK = (prop.script as u32
                                        == *code.add(1 + IMM2_SIZE + 2) as u32)
                                        as BOOL;
                                }
                                PT_SCX => {
                                    OK = (prop.script as u32
                                        == *code.add(1 + IMM2_SIZE + 2) as u32
                                        || script_set_bit(
                                            UCD_SCRIPTX_PROP(prop) as usize,
                                            *code.add(1 + IMM2_SIZE + 2) as u32,
                                        )) as BOOL;
                                }
                                PT_ALNUM => {
                                    chartype = prop.chartype as c_int;
                                    OK = (_pcre2_ucp_gentype_8[chartype as usize] == ucp_L
                                        || _pcre2_ucp_gentype_8[chartype as usize] == ucp_N)
                                        as BOOL;
                                }
                                PT_SPACE | PT_PXSPACE => {
                                    OK = match c {
                                        HSPACE_CASES!() | VSPACE_CASES!() => TRUE,
                                        _ => (_pcre2_ucp_gentype_8[prop.chartype as usize]
                                            == ucp_Z) as BOOL,
                                    };
                                }
                                PT_WORD => {
                                    chartype = prop.chartype as c_int;
                                    OK = (_pcre2_ucp_gentype_8[chartype as usize] == ucp_L
                                        || _pcre2_ucp_gentype_8[chartype as usize] == ucp_N
                                        || chartype == ucp_Mn as c_int
                                        || chartype == ucp_Pc as c_int) as BOOL;
                                }
                                PT_CLIST => {
                                    let mut cp: usize =
                                        *code.add(1 + IMM2_SIZE + 2) as usize;
                                    loop {
                                        if c < _pcre2_ucd_caseless_sets_8[cp] {
                                            OK = FALSE;
                                            break;
                                        }
                                        let cv = _pcre2_ucd_caseless_sets_8[cp];
                                        cp += 1;
                                        if c == cv {
                                            OK = TRUE;
                                            break;
                                        }
                                    }
                                }
                                PT_UCNC => {
                                    OK = (c == CHAR_DOLLAR_SIGN
                                        || c == CHAR_COMMERCIAL_AT
                                        || c == CHAR_GRAVE_ACCENT
                                        || (c >= 0xa0 && c <= 0xd7ff)
                                        || c >= 0xe000) as BOOL;
                                }
                                PT_BIDICL => {
                                    OK = (UCD_BIDICLASS(c)
                                        == *code.add(1 + IMM2_SIZE + 2) as u32)
                                        as BOOL;
                                }
                                PT_BOOL => {
                                    OK = boolprop_set_bit(
                                        UCD_BPROPS_PROP(prop) as usize,
                                        *code.add(1 + IMM2_SIZE + 2) as u32,
                                    ) as BOOL;
                                }
                                _ => {
                                    OK = (codevalue != OP_PROP) as BOOL;
                                }
                            }

                            if OK == (d == OP_PROP) as BOOL {
                                if codevalue == PROPX_TYPEPOSUPTO {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.offset(-1);
                                }
                                count += 1;
                                if count >= GET2(code, 1) as c_int {
                                    ADD_NEW!(state_offset + 1 + IMM2_SIZE as c_int + 3, 0);
                                } else {
                                    ADD_NEW!(state_offset, count);
                                }
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    EXTUNIX_TYPEEXACT
                    | EXTUNIX_TYPEUPTO
                    | EXTUNIX_TYPEMINUPTO
                    | EXTUNIX_TYPEPOSUPTO => {
                        if codevalue != EXTUNIX_TYPEEXACT {
                            ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as c_int, 0);
                        }
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let nptr: PCRE2_SPTR;
                            let mut ncount: c_int = 0;
                            if codevalue == EXTUNIX_TYPEPOSUPTO {
                                active_count -= 1; /* Remove non-match possibility */
                                next_active_state = next_active_state.offset(-1);
                            }
                            nptr = crate::extuni::_pcre2_extuni_8(
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
                            if count >= GET2(code, 1) as c_int {
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

                    /*-----------------------------------------------------------------*/
                    ANYNLX_TYPEEXACT
                    | ANYNLX_TYPEUPTO
                    | ANYNLX_TYPEMINUPTO
                    | ANYNLX_TYPEPOSUPTO => {
                        if codevalue != ANYNLX_TYPEEXACT {
                            ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as c_int, 0);
                        }
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let mut ncount: c_int = 0;
                            let mut anynl03: bool = false;
                            match c {
                                CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                    if (*mb).bsr_convention as u32 != PCRE2_BSR_ANYCRLF {
                                        anynl03 = true;
                                    }
                                }
                                CHAR_CR => {
                                    if ptr.add(1) < end_subject
                                        && *ptr.add(1) as u32 == CHAR_LF
                                    {
                                        ncount = 1;
                                    }
                                    anynl03 = true;
                                }
                                CHAR_LF => {
                                    anynl03 = true;
                                }
                                _ => {}
                            }
                            if anynl03 {
                                if codevalue == ANYNLX_TYPEPOSUPTO {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.offset(-1);
                                }
                                count += 1;
                                if count >= GET2(code, 1) as c_int {
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

                    /*-----------------------------------------------------------------*/
                    VSPACEX_TYPEEXACT
                    | VSPACEX_TYPEUPTO
                    | VSPACEX_TYPEMINUPTO
                    | VSPACEX_TYPEPOSUPTO => {
                        if codevalue != VSPACEX_TYPEEXACT {
                            ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as c_int, 0);
                        }
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let OK: BOOL = match c {
                                VSPACE_CASES!() => TRUE,
                                _ => FALSE,
                            };

                            if OK == (d == OP_VSPACE) as BOOL {
                                if codevalue == VSPACEX_TYPEPOSUPTO {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.offset(-1);
                                }
                                count += 1;
                                if count >= GET2(code, 1) as c_int {
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

                    /*-----------------------------------------------------------------*/
                    HSPACEX_TYPEEXACT
                    | HSPACEX_TYPEUPTO
                    | HSPACEX_TYPEMINUPTO
                    | HSPACEX_TYPEPOSUPTO => {
                        if codevalue != HSPACEX_TYPEEXACT {
                            ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as c_int, 0);
                        }
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let OK: BOOL = match c {
                                HSPACE_CASES!() => TRUE,
                                _ => FALSE,
                            };

                            if OK == (d == OP_HSPACE) as BOOL {
                                if codevalue == HSPACEX_TYPEPOSUPTO {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.offset(-1);
                                }
                                count += 1;
                                if count >= GET2(code, 1) as c_int {
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

                    /* ============================================================= */
                    /* These opcodes are followed by a character that is usually
                    compared to the current subject character; it is loaded into d. */
                    OP_CHAR => {
                        if clen > 0 && c == d {
                            ADD_NEW!(state_offset + dlen + 1, 0);
                        }
                    }

                    OP_CHARI => {
                        if clen == 0 {
                            break 'NEXT_ACTIVE_STATE;
                        }

                        if utf_or_ucp != 0 {
                            if c == d {
                                ADD_NEW!(state_offset + dlen + 1, 0);
                            } else {
                                let othercase: c_uint;
                                if c < 128 {
                                    othercase = *fcc.add(c as usize) as c_uint;
                                } else {
                                    othercase = UCD_OTHERCASE(c) as c_uint;
                                }
                                if d == othercase as u32 {
                                    ADD_NEW!(state_offset + dlen + 1, 0);
                                }
                            }
                        }
                        /* Not UTF or UCP mode */
                        else {
                            if TABLE_GET(c, lcc, c) == TABLE_GET(d, lcc, d) {
                                ADD_NEW!(state_offset + 2, 0);
                            }
                        }
                    }

                    /* This is a tricky one because it can match more than one
                    character. */
                    OP_EXTUNI => {
                        if clen > 0 {
                            let mut ncount: c_int = 0;
                            let nptr: PCRE2_SPTR = crate::extuni::_pcre2_extuni_8(
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

                    /* This is tricky like EXTUNI because it too can match more than
                    one character (when CR is followed by LF). */
                    OP_ANYNL => {
                        if clen > 0 {
                            let mut anynl_lf: bool = false;
                            match c {
                                CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                    if (*mb).bsr_convention as u32 != PCRE2_BSR_ANYCRLF {
                                        anynl_lf = true;
                                    }
                                }
                                CHAR_LF => {
                                    anynl_lf = true;
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
                            if anynl_lf {
                                ADD_NEW!(state_offset + 1, 0);
                            }
                        }
                    }

                    OP_NOT_VSPACE => {
                        if clen > 0 {
                            match c {
                                VSPACE_CASES!() => {}
                                _ => {
                                    ADD_NEW!(state_offset + 1, 0);
                                }
                            }
                        }
                    }

                    OP_VSPACE => {
                        if clen > 0 {
                            match c {
                                VSPACE_CASES!() => {
                                    ADD_NEW!(state_offset + 1, 0);
                                }
                                _ => {}
                            }
                        }
                    }

                    OP_NOT_HSPACE => {
                        if clen > 0 {
                            match c {
                                HSPACE_CASES!() => {}
                                _ => {
                                    ADD_NEW!(state_offset + 1, 0);
                                }
                            }
                        }
                    }

                    OP_HSPACE => {
                        if clen > 0 {
                            match c {
                                HSPACE_CASES!() => {
                                    ADD_NEW!(state_offset + 1, 0);
                                }
                                _ => {}
                            }
                        }
                    }

                    /* Match a negated single character casefully. */
                    OP_NOT => {
                        if clen > 0 && c != d {
                            ADD_NEW!(state_offset + dlen + 1, 0);
                        }
                    }

                    /* Match a negated single character caselessly. */
                    OP_NOTI => {
                        if clen > 0 {
                            let otherd: u32;
                            if utf_or_ucp != 0 && d >= 128 {
                                otherd = UCD_OTHERCASE(d);
                            } else {
                                otherd = TABLE_GET(d, fcc, d);
                            }
                            if c != d && c != otherd {
                                ADD_NEW!(state_offset + dlen + 1, 0);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_PLUSI
                    | OP_MINPLUSI
                    | OP_POSPLUSI
                    | OP_NOTPLUSI
                    | OP_NOTMINPLUSI
                    | OP_NOTPOSPLUSI
                    | OP_PLUS
                    | OP_MINPLUS
                    | OP_POSPLUS
                    | OP_NOTPLUS
                    | OP_NOTMINPLUS
                    | OP_NOTPOSPLUS => {
                        if switch_value == OP_PLUSI
                            || switch_value == OP_MINPLUSI
                            || switch_value == OP_POSPLUSI
                            || switch_value == OP_NOTPLUSI
                            || switch_value == OP_NOTMINPLUSI
                            || switch_value == OP_NOTPOSPLUSI
                        {
                            caseless = TRUE;
                            codevalue -= OP_STARI - OP_STAR;
                        }

                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + dlen + 1, 0);
                        }
                        if clen > 0 {
                            let mut otherd: u32 = NOTACHAR;
                            if caseless != 0 {
                                if utf_or_ucp != 0 && d >= 128 {
                                    otherd = UCD_OTHERCASE(d);
                                } else {
                                    otherd = TABLE_GET(d, fcc, d);
                                }
                            }
                            if (c == d || c == otherd) == (codevalue < OP_NOTSTAR) {
                                if count > 0
                                    && (codevalue == OP_POSPLUS || codevalue == OP_NOTPOSPLUS)
                                {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.offset(-1);
                                }
                                count += 1;
                                ADD_NEW!(state_offset, count);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_QUERYI
                    | OP_MINQUERYI
                    | OP_POSQUERYI
                    | OP_NOTQUERYI
                    | OP_NOTMINQUERYI
                    | OP_NOTPOSQUERYI
                    | OP_QUERY
                    | OP_MINQUERY
                    | OP_POSQUERY
                    | OP_NOTQUERY
                    | OP_NOTMINQUERY
                    | OP_NOTPOSQUERY => {
                        if switch_value == OP_QUERYI
                            || switch_value == OP_MINQUERYI
                            || switch_value == OP_POSQUERYI
                            || switch_value == OP_NOTQUERYI
                            || switch_value == OP_NOTMINQUERYI
                            || switch_value == OP_NOTPOSQUERYI
                        {
                            caseless = TRUE;
                            codevalue -= OP_STARI - OP_STAR;
                        }

                        ADD_ACTIVE!(state_offset + dlen + 1, 0);
                        if clen > 0 {
                            let mut otherd: u32 = NOTACHAR;
                            if caseless != 0 {
                                if utf_or_ucp != 0 && d >= 128 {
                                    otherd = UCD_OTHERCASE(d);
                                } else {
                                    otherd = TABLE_GET(d, fcc, d);
                                }
                            }
                            if (c == d || c == otherd) == (codevalue < OP_NOTSTAR) {
                                if codevalue == OP_POSQUERY || codevalue == OP_NOTPOSQUERY {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.offset(-1);
                                }
                                ADD_NEW!(state_offset + dlen + 1, 0);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_STARI
                    | OP_MINSTARI
                    | OP_POSSTARI
                    | OP_NOTSTARI
                    | OP_NOTMINSTARI
                    | OP_NOTPOSSTARI
                    | OP_STAR
                    | OP_MINSTAR
                    | OP_POSSTAR
                    | OP_NOTSTAR
                    | OP_NOTMINSTAR
                    | OP_NOTPOSSTAR => {
                        if switch_value == OP_STARI
                            || switch_value == OP_MINSTARI
                            || switch_value == OP_POSSTARI
                            || switch_value == OP_NOTSTARI
                            || switch_value == OP_NOTMINSTARI
                            || switch_value == OP_NOTPOSSTARI
                        {
                            caseless = TRUE;
                            codevalue -= OP_STARI - OP_STAR;
                        }

                        ADD_ACTIVE!(state_offset + dlen + 1, 0);
                        if clen > 0 {
                            let mut otherd: u32 = NOTACHAR;
                            if caseless != 0 {
                                if utf_or_ucp != 0 && d >= 128 {
                                    otherd = UCD_OTHERCASE(d);
                                } else {
                                    otherd = TABLE_GET(d, fcc, d);
                                }
                            }
                            if (c == d || c == otherd) == (codevalue < OP_NOTSTAR) {
                                if codevalue == OP_POSSTAR || codevalue == OP_NOTPOSSTAR {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.offset(-1);
                                }
                                ADD_NEW!(state_offset, 0);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_EXACTI | OP_NOTEXACTI | OP_EXACT | OP_NOTEXACT => {
                        if switch_value == OP_EXACTI || switch_value == OP_NOTEXACTI {
                            caseless = TRUE;
                            codevalue -= OP_STARI - OP_STAR;
                        }

                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let mut otherd: u32 = NOTACHAR;
                            if caseless != 0 {
                                if utf_or_ucp != 0 && d >= 128 {
                                    otherd = UCD_OTHERCASE(d);
                                } else {
                                    otherd = TABLE_GET(d, fcc, d);
                                }
                            }
                            if (c == d || c == otherd) == (codevalue < OP_NOTSTAR) {
                                count += 1;
                                if count >= GET2(code, 1) as c_int {
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

                    /*-----------------------------------------------------------------*/
                    OP_UPTOI
                    | OP_MINUPTOI
                    | OP_POSUPTOI
                    | OP_NOTUPTOI
                    | OP_NOTMINUPTOI
                    | OP_NOTPOSUPTOI
                    | OP_UPTO
                    | OP_MINUPTO
                    | OP_POSUPTO
                    | OP_NOTUPTO
                    | OP_NOTMINUPTO
                    | OP_NOTPOSUPTO => {
                        if switch_value == OP_UPTOI
                            || switch_value == OP_MINUPTOI
                            || switch_value == OP_POSUPTOI
                            || switch_value == OP_NOTUPTOI
                            || switch_value == OP_NOTMINUPTOI
                            || switch_value == OP_NOTPOSUPTOI
                        {
                            caseless = TRUE;
                            codevalue -= OP_STARI - OP_STAR;
                        }

                        ADD_ACTIVE!(state_offset + dlen + 1 + IMM2_SIZE as c_int, 0);
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let mut otherd: u32 = NOTACHAR;
                            if caseless != 0 {
                                if utf_or_ucp != 0 && d >= 128 {
                                    otherd = UCD_OTHERCASE(d);
                                } else {
                                    otherd = TABLE_GET(d, fcc, d);
                                }
                            }
                            if (c == d || c == otherd) == (codevalue < OP_NOTSTAR) {
                                if codevalue == OP_POSUPTO || codevalue == OP_NOTPOSUPTO {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.offset(-1);
                                }
                                count += 1;
                                if count >= GET2(code, 1) as c_int {
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

                    /* ============================================================= */
                    /* These are the class-handling opcodes */
                    OP_CLASS | OP_NCLASS | OP_XCLASS | OP_ECLASS => {
                        let mut isinclass: BOOL = FALSE;
                        let next_state_offset: c_int;
                        let ecode: PCRE2_SPTR;

                        /* An extended class may have a table or a list of single
                        characters, ranges, or both. */

                        if codevalue == OP_XCLASS {
                            ecode = code.add(GET(code, 1) as usize);
                            if clen > 0 {
                                isinclass = crate::xclass::_pcre2_xclass_8(
                                    c,
                                    code.add(1 + LINK_SIZE),
                                    (*mb).start_code as *const u8,
                                    utf,
                                );
                            }
                        }
                        /* A nested set-based class has internal opcodes for
                        performing set operations. */
                        else if codevalue == OP_ECLASS {
                            ecode = code.add(GET(code, 1) as usize);
                            if clen > 0 {
                                isinclass = crate::xclass::_pcre2_eclass_8(
                                    c,
                                    code.add(1 + LINK_SIZE),
                                    ecode,
                                    (*mb).start_code as *const u8,
                                    utf,
                                );
                            }
                        }
                        /* For a simple class, there is always just a 32-byte table. */
                        else {
                            ecode = code.add(1 + 32);
                            if clen > 0 {
                                isinclass = if c > 255 {
                                    (codevalue == OP_NCLASS) as BOOL
                                } else {
                                    ((*code.add(1).add((c / 8) as usize) as u32
                                        & (1u32 << (c & 7)))
                                        != 0) as BOOL
                                };
                            }
                        }

                        /* At this point, isinclass is set for all kinds of class, and
                        ecode points to the byte after the end of the class. */

                        next_state_offset = ecode.offset_from(start_code) as c_int;

                        match *ecode as u32 {
                            OP_CRSTAR | OP_CRMINSTAR | OP_CRPOSSTAR => {
                                ADD_ACTIVE!(next_state_offset + 1, 0);
                                if isinclass != 0 {
                                    if *ecode as u32 == OP_CRPOSSTAR {
                                        active_count -= 1; /* Remove non-match possibility */
                                        next_active_state = next_active_state.offset(-1);
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
                                    if count > 0 && *ecode as u32 == OP_CRPOSPLUS {
                                        active_count -= 1; /* Remove non-match possibility */
                                        next_active_state = next_active_state.offset(-1);
                                    }
                                    count += 1;
                                    ADD_NEW!(state_offset, count);
                                }
                            }

                            OP_CRQUERY | OP_CRMINQUERY | OP_CRPOSQUERY => {
                                ADD_ACTIVE!(next_state_offset + 1, 0);
                                if isinclass != 0 {
                                    if *ecode as u32 == OP_CRPOSQUERY {
                                        active_count -= 1; /* Remove non-match possibility */
                                        next_active_state = next_active_state.offset(-1);
                                    }
                                    ADD_NEW!(next_state_offset + 1, 0);
                                }
                            }

                            OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                                count = (*current_state).count; /* Already matched */
                                if count >= GET2(ecode, 1) as c_int {
                                    ADD_ACTIVE!(
                                        next_state_offset + 1 + 2 * IMM2_SIZE as c_int,
                                        0
                                    );
                                }
                                if isinclass != 0 {
                                    let max: c_int = GET2(ecode, 1 + IMM2_SIZE) as c_int;

                                    if *ecode as u32 == OP_CRPOSRANGE
                                        && count >= GET2(ecode, 1) as c_int
                                    {
                                        active_count -= 1; /* Remove non-match possibility */
                                        next_active_state = next_active_state.offset(-1);
                                    }

                                    count += 1;
                                    if count >= max && max != 0 {
                                        /* Max 0 => no limit */
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

                    /* ============================================================= */
                    /* These are the opcodes for fancy brackets of various kinds. */
                    OP_FAIL => {}

                    OP_ASSERT | OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT => {
                        let mut rc: c_int;
                        let local_workspace: *mut c_int;
                        let local_offsets: *mut PCRE2_SIZE;
                        let mut endasscode: PCRE2_SPTR = code.add(GET(code, 1) as usize);
                        let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;

                        if ((*rws).free as usize) < RWS_RSIZE + RWS_OVEC_OSIZE {
                            rc = more_workspace(&mut rws, RWS_OVEC_OSIZE as c_uint, mb);
                            if rc != 0 {
                                return rc;
                            }
                            RWS = rws as *mut c_int;
                        }

                        local_offsets = RWS
                            .offset((*rws).size as isize)
                            .offset(-((*rws).free as isize))
                            as *mut PCRE2_SIZE;
                        local_workspace = (local_offsets as *mut c_int).add(RWS_OVEC_OSIZE);
                        (*rws).free -= (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        while *endasscode as u32 == OP_ALT {
                            endasscode = endasscode.add(GET(endasscode, 1) as usize);
                        }

                        rc = internal_dfa_match(
                            mb,       /* static match data */
                            code,     /* this subexpression's code */
                            ptr,      /* where we currently are */
                            ptr.offset_from(start_subject) as PCRE2_SIZE, /* start offset */
                            local_offsets, /* offset vector */
                            (RWS_OVEC_OSIZE / OVEC_UNIT) as u32, /* size of same */
                            local_workspace, /* workspace vector */
                            RWS_RSIZE as c_int, /* size of same */
                            rlevel,   /* function recursion level */
                            RWS,      /* recursion workspace */
                        );

                        (*rws).free += (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        if rc < 0 && rc != PCRE2_ERROR_NOMATCH {
                            return rc;
                        }
                        if (rc >= 0)
                            == (codevalue == OP_ASSERT || codevalue == OP_ASSERTBACK)
                        {
                            ADD_ACTIVE!(
                                endasscode.add(LINK_SIZE + 1).offset_from(start_code) as c_int,
                                0
                            );
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_COND | OP_SCOND => {
                        let codelink: c_int = GET(code, 1) as c_int;
                        let condcode: PCRE2_UCHAR;

                        /* Because of the way auto-callout works during compile, a
                        callout item is inserted between OP_COND and an assertion
                        condition. */

                        if *code.add(LINK_SIZE + 1) as u32 == OP_CALLOUT
                            || *code.add(LINK_SIZE + 1) as u32 == OP_CALLOUT_STR
                        {
                            let mut callout_length: PCRE2_SIZE = 0;
                            rrc = do_callout_dfa(
                                code,
                                offsets,
                                current_subject,
                                ptr,
                                mb,
                                (1 + LINK_SIZE) as PCRE2_SIZE,
                                &mut callout_length,
                            );
                            if rrc < 0 {
                                return rrc;
                            } /* Abandon */
                            if rrc > 0 {
                                break 'NEXT_ACTIVE_STATE;
                            } /* Fail this thread */
                            code = code.add(callout_length); /* Skip callout data */
                        }

                        condcode = *code.add(LINK_SIZE + 1);

                        /* Back reference conditions and duplicate named recursion
                        conditions are not supported */

                        if condcode as u32 == OP_CREF
                            || condcode as u32 == OP_DNCREF
                            || condcode as u32 == OP_DNRREF
                        {
                            return PCRE2_ERROR_DFA_UCOND;
                        }

                        /* The DEFINE condition is always false, and the assertion (?!)
                        is converted to OP_FAIL. */

                        if condcode as u32 == OP_FALSE || condcode as u32 == OP_FAIL {
                            ADD_ACTIVE!(state_offset + codelink + LINK_SIZE as c_int + 1, 0);
                        }
                        /* There is also an always-true condition */
                        else if condcode as u32 == OP_TRUE {
                            ADD_ACTIVE!(state_offset + LINK_SIZE as c_int + 2, 0);
                        }
                        /* The only supported version of OP_RREF is for the value
                        RREF_ANY. */
                        else if condcode as u32 == OP_RREF {
                            let value: c_uint = GET2(code, LINK_SIZE + 2) as c_uint;
                            if value != RREF_ANY as c_uint {
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
                        }
                        /* Otherwise, the condition is an assertion */
                        else {
                            let mut rc: c_int;
                            let local_workspace: *mut c_int;
                            let local_offsets: *mut PCRE2_SIZE;
                            let asscode: PCRE2_SPTR = code.add(LINK_SIZE + 1);
                            let mut endasscode: PCRE2_SPTR =
                                asscode.add(GET(asscode, 1) as usize);
                            let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;

                            if ((*rws).free as usize) < RWS_RSIZE + RWS_OVEC_OSIZE {
                                rc = more_workspace(&mut rws, RWS_OVEC_OSIZE as c_uint, mb);
                                if rc != 0 {
                                    return rc;
                                }
                                RWS = rws as *mut c_int;
                            }

                            local_offsets = RWS
                                .offset((*rws).size as isize)
                                .offset(-((*rws).free as isize))
                                as *mut PCRE2_SIZE;
                            local_workspace =
                                (local_offsets as *mut c_int).add(RWS_OVEC_OSIZE);
                            (*rws).free -= (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                            while *endasscode as u32 == OP_ALT {
                                endasscode = endasscode.add(GET(endasscode, 1) as usize);
                            }

                            rc = internal_dfa_match(
                                mb,      /* fixed match data */
                                asscode, /* this subexpression's code */
                                ptr,     /* where we currently are */
                                ptr.offset_from(start_subject) as PCRE2_SIZE, /* start offset */
                                local_offsets, /* offset vector */
                                (RWS_OVEC_OSIZE / OVEC_UNIT) as u32, /* size of same */
                                local_workspace, /* workspace vector */
                                RWS_RSIZE as c_int, /* size of same */
                                rlevel,  /* function recursion level */
                                RWS,     /* recursion workspace */
                            );

                            (*rws).free += (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                            if rc < 0 && rc != PCRE2_ERROR_NOMATCH {
                                return rc;
                            }
                            if (rc >= 0)
                                == (condcode as u32 == OP_ASSERT
                                    || condcode as u32 == OP_ASSERTBACK)
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

                    /*-----------------------------------------------------------------*/
                    OP_RECURSE => {
                        let mut rc: c_int;
                        let local_workspace: *mut c_int;
                        let local_offsets: *mut PCRE2_SIZE;
                        let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;
                        let callpat: PCRE2_SPTR = start_code.add(GET(code, 1) as usize);
                        let recno: u32 = if callpat == (*mb).start_code {
                            0
                        } else {
                            GET2(callpat, 1 + LINK_SIZE)
                        };

                        /* Argument list has not been supported yet. */
                        if *code.add(1 + LINK_SIZE) as u32 == OP_CREF {
                            return PCRE2_ERROR_DFA_UITEM;
                        }

                        if ((*rws).free as usize) < RWS_RSIZE + RWS_OVEC_RSIZE {
                            rc = more_workspace(&mut rws, RWS_OVEC_RSIZE as c_uint, mb);
                            if rc != 0 {
                                return rc;
                            }
                            RWS = rws as *mut c_int;
                        }

                        local_offsets = RWS
                            .offset((*rws).size as isize)
                            .offset(-((*rws).free as isize))
                            as *mut PCRE2_SIZE;
                        local_workspace = (local_offsets as *mut c_int).add(RWS_OVEC_RSIZE);
                        (*rws).free -= (RWS_RSIZE + RWS_OVEC_RSIZE) as u32;

                        /* Check for repeating a recursion without advancing the subject
                        pointer or last used character. */

                        let mut ri: *mut dfa_recursion_info = (*mb).recursive;
                        while !ri.is_null() {
                            if recno == (*ri).group_num
                                && ptr == (*ri).subject_position
                                && (*mb).last_used_ptr == (*ri).last_used_ptr
                            {
                                return PCRE2_ERROR_RECURSELOOP;
                            }
                            ri = (*ri).prevrec;
                        }

                        /* Remember this recursion and where we started it so as to
                        catch infinite loops. */

                        new_recursive.group_num = recno;
                        new_recursive.subject_position = ptr;
                        new_recursive.last_used_ptr = (*mb).last_used_ptr;
                        new_recursive.prevrec = (*mb).recursive;
                        (*mb).recursive = &mut new_recursive as *mut dfa_recursion_info;

                        rc = internal_dfa_match(
                            mb,      /* fixed match data */
                            callpat, /* this subexpression's code */
                            ptr,     /* where we currently are */
                            ptr.offset_from(start_subject) as PCRE2_SIZE, /* start offset */
                            local_offsets, /* offset vector */
                            (RWS_OVEC_RSIZE / OVEC_UNIT) as u32, /* size of same */
                            local_workspace, /* workspace vector */
                            RWS_RSIZE as c_int, /* size of same */
                            rlevel,  /* function recursion level */
                            RWS,     /* recursion workspace */
                        );

                        (*rws).free += (RWS_RSIZE + RWS_OVEC_RSIZE) as u32;
                        (*mb).recursive = new_recursive.prevrec; /* Done this recursion */

                        /* Ran out of internal offsets */

                        if rc == 0 {
                            return PCRE2_ERROR_DFA_RECURSE;
                        }

                        /* For each successful matched substring, set up the next state
                        with a count of characters to skip before trying it. */

                        if rc > 0 {
                            rc = rc * 2 - 2;
                            while rc >= 0 {
                                let mut charcount: PCRE2_SIZE = (*local_offsets
                                    .add((rc + 1) as usize))
                                .wrapping_sub(*local_offsets.add(rc as usize));
                                if utf != 0 {
                                    let mut p: PCRE2_SPTR =
                                        start_subject.add(*local_offsets.add(rc as usize));
                                    let pp: PCRE2_SPTR = start_subject
                                        .add(*local_offsets.add((rc + 1) as usize));
                                    while p < pp {
                                        let v = *p;
                                        p = p.add(1);
                                        if NOT_FIRSTCU(v as u32) {
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

                    /*-----------------------------------------------------------------*/
                    OP_BRAPOS | OP_SBRAPOS | OP_CBRAPOS | OP_SCBRAPOS | OP_BRAPOSZERO => {
                        let mut rc: c_int;
                        let local_workspace: *mut c_int;
                        let local_offsets: *mut PCRE2_SIZE;
                        let mut charcount: PCRE2_SIZE;
                        let mut matched_count: PCRE2_SIZE;
                        let mut local_ptr: PCRE2_SPTR = ptr;
                        let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;
                        let allow_zero: BOOL;

                        if ((*rws).free as usize) < RWS_RSIZE + RWS_OVEC_OSIZE {
                            rc = more_workspace(&mut rws, RWS_OVEC_OSIZE as c_uint, mb);
                            if rc != 0 {
                                return rc;
                            }
                            RWS = rws as *mut c_int;
                        }

                        local_offsets = RWS
                            .offset((*rws).size as isize)
                            .offset(-((*rws).free as isize))
                            as *mut PCRE2_SIZE;
                        local_workspace = (local_offsets as *mut c_int).add(RWS_OVEC_OSIZE);
                        (*rws).free -= (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        if codevalue == OP_BRAPOSZERO {
                            allow_zero = TRUE;
                            code = code.add(1); /* The following opcode will be one of the above BRAs */
                        } else {
                            allow_zero = FALSE;
                        }

                        /* Loop to match the subpattern as many times as possible as if
                        it were a complete pattern. */

                        matched_count = 0;
                        loop {
                            rc = internal_dfa_match(
                                mb,        /* fixed match data */
                                code,      /* this subexpression's code */
                                local_ptr, /* where we currently are */
                                ptr.offset_from(start_subject) as PCRE2_SIZE, /* start offset */
                                local_offsets, /* offset vector */
                                (RWS_OVEC_OSIZE / OVEC_UNIT) as u32, /* size of same */
                                local_workspace, /* workspace vector */
                                RWS_RSIZE as c_int, /* size of same */
                                rlevel,    /* function recursion level */
                                RWS,       /* recursion workspace */
                            );

                            /* Failed to match */

                            if rc < 0 {
                                if rc != PCRE2_ERROR_NOMATCH {
                                    return rc;
                                }
                                break;
                            }

                            /* Matched: break the loop if zero characters matched. */

                            charcount = (*local_offsets.add(1))
                                .wrapping_sub(*local_offsets.add(0));
                            if charcount == 0 {
                                break;
                            }
                            local_ptr = local_ptr.add(charcount); /* Advance temporary position ptr */
                            matched_count += 1;
                        }

                        (*rws).free += (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        /* At this point we have matched the subpattern matched_count
                        times, and local_ptr is pointing to the character after the end
                        of the last match. */

                        if matched_count > 0 || allow_zero != 0 {
                            let mut end_subpattern: PCRE2_SPTR = code;
                            let next_state_offset: c_int;

                            loop {
                                end_subpattern =
                                    end_subpattern.add(GET(end_subpattern, 1) as usize);
                                if *end_subpattern as u32 != OP_ALT {
                                    break;
                                }
                            }
                            next_state_offset = (end_subpattern.offset_from(start_code)
                                as isize
                                + LINK_SIZE as isize
                                + 1) as c_int;

                            /* Optimization: if there are no more active states, and
                            there are no new states yet set up, then skip over the
                            subject string right here, to save looping. */

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
                                        let v = *p;
                                        p = p.add(1);
                                        if NOT_FIRSTCU(v as u32) {
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

                    /*-----------------------------------------------------------------*/
                    OP_ONCE => {
                        let mut rc: c_int;
                        let local_workspace: *mut c_int;
                        let local_offsets: *mut PCRE2_SIZE;
                        let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;

                        if ((*rws).free as usize) < RWS_RSIZE + RWS_OVEC_OSIZE {
                            rc = more_workspace(&mut rws, RWS_OVEC_OSIZE as c_uint, mb);
                            if rc != 0 {
                                return rc;
                            }
                            RWS = rws as *mut c_int;
                        }

                        local_offsets = RWS
                            .offset((*rws).size as isize)
                            .offset(-((*rws).free as isize))
                            as *mut PCRE2_SIZE;
                        local_workspace = (local_offsets as *mut c_int).add(RWS_OVEC_OSIZE);
                        (*rws).free -= (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        rc = internal_dfa_match(
                            mb,   /* fixed match data */
                            code, /* this subexpression's code */
                            ptr,  /* where we currently are */
                            ptr.offset_from(start_subject) as PCRE2_SIZE, /* start offset */
                            local_offsets, /* offset vector */
                            (RWS_OVEC_OSIZE / OVEC_UNIT) as u32, /* size of same */
                            local_workspace, /* workspace vector */
                            RWS_RSIZE as c_int, /* size of same */
                            rlevel, /* function recursion level */
                            RWS,  /* recursion workspace */
                        );

                        (*rws).free += (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        if rc >= 0 {
                            let mut end_subpattern: PCRE2_SPTR = code;
                            let mut charcount: PCRE2_SIZE = (*local_offsets.add(1))
                                .wrapping_sub(*local_offsets.add(0));
                            let next_state_offset: c_int;
                            let repeat_state_offset: c_int;

                            loop {
                                end_subpattern =
                                    end_subpattern.add(GET(end_subpattern, 1) as usize);
                                if *end_subpattern as u32 != OP_ALT {
                                    break;
                                }
                            }
                            next_state_offset = (end_subpattern.offset_from(start_code)
                                as isize
                                + LINK_SIZE as isize
                                + 1) as c_int;

                            /* If the end of this subpattern is KETRMAX or KETRMIN, we
                            must arrange for the repeat state also to be added to the
                            relevant list. */

                            repeat_state_offset = if *end_subpattern as u32 == OP_KETRMAX
                                || *end_subpattern as u32 == OP_KETRMIN
                            {
                                (end_subpattern.offset_from(start_code) as isize
                                    - GET(end_subpattern, 1) as isize) as c_int
                            } else {
                                -1
                            };

                            /* If we have matched an empty string, add the next state at
                            the current character pointer. */

                            if charcount == 0 {
                                ADD_ACTIVE!(next_state_offset, 0);
                            }
                            /* Optimization: if there are no more active states, and
                            there are no new states yet set up, then skip over the
                            subject string right here, to save looping. */
                            else if i + 1 >= active_count && new_count == 0 {
                                ptr = ptr.add(charcount);
                                clen = 0;
                                ADD_NEW!(next_state_offset, 0);

                                /* If we are adding a repeat state at the new character
                                position, we must fudge things so that it is the only
                                current state. */

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
                                        let v = *p;
                                        p = p.add(1);
                                        if NOT_FIRSTCU(v as u32) {
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

                    /* ============================================================= */
                    /* Handle callouts */
                    OP_CALLOUT | OP_CALLOUT_STR => {
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
                            return rrc;
                        } /* Abandon */
                        if rrc == 0 {
                            ADD_ACTIVE!(state_offset + callout_length as c_int, 0);
                        }
                    }

                    /* ============================================================= */
                    _ => {
                        /* Unsupported opcode */
                        return PCRE2_ERROR_DFA_UITEM;
                    }
                }
            }
            i += 1;
        } /* End of loop scanning active states */

        /* We have finished the processing at the current subject character. */

        if new_count <= 0 {
            if could_continue != 0
                && (((*mb).moptions & PCRE2_PARTIAL_HARD) != 0
                    || (((*mb).moptions & PCRE2_PARTIAL_SOFT) != 0 && match_count < 0))
                && (partial_newline != 0
                    || (ptr >= end_subject
                        && (ptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)))
            {
                match_count = PCRE2_ERROR_PARTIAL;
            }
            break; /* Exit from loop along the subject string */
        }

        /* One or more states are active for the next character. */

        ptr = ptr.add(clen as usize); /* Advance to next subject character */
    } /* Loop to move along the subject string */

    /* Control gets here from "break" a few lines above. */

    if match_count >= 0
        && (((*mb).moptions | (*mb).poptions) & PCRE2_ENDANCHORED) != 0
        && ptr < end_subject
    {
        match_count = PCRE2_ERROR_NOMATCH;
    }

    match_count
}

/*************************************************
*     Match a pattern using the DFA algorithm    *
*************************************************/

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
    let mut subject = subject;
    let mut length = length;
    let mut options = options;

    let mut rc: c_int = 0;

    let re: *const pcre2_real_code = code;
    let original_options: u32 = options;

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

    /* We need to have mb pointing to a match block, because the IS_NEWLINE macro
    is used below, and it expects NLBLOCK to be defined as a pointer. */

    let mut cb: pcre2_callout_block = core::mem::zeroed();
    let mut actual_match_block: dfa_match_block = core::mem::zeroed();
    let mb: *mut dfa_match_block = &mut actual_match_block;

    /* Set up a starting block of memory for use during recursive calls to
    internal_dfa_match(). */

    let mut base_rws_storage: core::mem::MaybeUninit<[c_int; RWS_BASE_SIZE]> =
        core::mem::MaybeUninit::uninit();
    let base_recursion_workspace: *mut c_int = base_rws_storage.as_mut_ptr() as *mut c_int;
    let rws: *mut RWS_anchor = base_recursion_workspace as *mut RWS_anchor;
    (*rws).next = core::ptr::null_mut();
    (*rws).size = RWS_BASE_SIZE as u32;
    (*rws).free = (RWS_BASE_SIZE - RWS_ANCHOR_SIZE) as u32;

    /* IS_NEWLINE / WAS_NEWLINE with NLBLOCK == mb. */

    macro_rules! IS_NEWLINE {
        ($p:expr) => {
            (if (*mb).nltype != NLTYPE_FIXED {
                ($p) < (*mb).end_subject
                    && crate::newline::_pcre2_is_newline_8(
                        ($p),
                        (*mb).nltype,
                        (*mb).end_subject,
                        &mut (*mb).nllen,
                        utf,
                    ) != 0
            } else {
                ($p) <= (*mb).end_subject.wrapping_sub((*mb).nllen as usize)
                    && *($p) == (*mb).nl[0]
                    && ((*mb).nllen == 1 || *($p).add(1) == (*mb).nl[1])
            })
        };
    }

    macro_rules! WAS_NEWLINE {
        ($p:expr) => {
            (if (*mb).nltype != NLTYPE_FIXED {
                ($p) > (*mb).start_subject
                    && crate::newline::_pcre2_was_newline_8(
                        ($p),
                        (*mb).nltype,
                        (*mb).start_subject,
                        &mut (*mb).nllen,
                        utf,
                    ) != 0
            } else {
                ($p) >= (*mb).start_subject.wrapping_add((*mb).nllen as usize)
                    && *($p).wrapping_sub((*mb).nllen as usize) == (*mb).nl[0]
                    && ((*mb).nllen == 1
                        || *($p).wrapping_sub((*mb).nllen as usize).add(1) == (*mb).nl[1])
            })
        };
    }

    /* Recognize NULL, length 0 as an empty string. */

    if subject.is_null() && length == 0 {
        subject = null_str.as_ptr();
    }

    /* Plausibility checks */

    if match_data.is_null() {
        return PCRE2_ERROR_NULL;
    }

    'EXIT: {
        'NOMATCH_EXIT: {
            if re.is_null() || subject.is_null() || workspace.is_null() {
                rc = PCRE2_ERROR_NULL;
                break 'EXIT;
            }
            if (options & !PUBLIC_DFA_MATCH_OPTIONS) != 0 {
                rc = PCRE2_ERROR_BADOPTION;
                break 'EXIT;
            }

            if length == PCRE2_ZERO_TERMINATED {
                length = crate::string_utils::_pcre2_strlen_8(subject);
            }

            if wscount < 20 {
                rc = PCRE2_ERROR_DFA_WSSIZE;
                break 'EXIT;
            }
            if start_offset > length {
                rc = PCRE2_ERROR_BADOFFSET;
                break 'EXIT;
            }

            /* Partial matching and PCRE2_ENDANCHORED are currently not allowed at
            the same time. */

            if (options & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT)) != 0
                && (((*re).overall_options | options) & PCRE2_ENDANCHORED) != 0
            {
                rc = PCRE2_ERROR_BADOPTION;
                break 'EXIT;
            }

            /* Invalid UTF support is not available for DFA matching. */

            if ((*re).overall_options & PCRE2_MATCH_INVALID_UTF) != 0 {
                rc = PCRE2_ERROR_DFA_UINVALID_UTF;
                break 'EXIT;
            }

            /* Check that the first field in the block is the magic number. */

            if (*re).magic_number != MAGIC_NUMBER {
                rc = PCRE2_ERROR_BADMAGIC;
                break 'EXIT;
            }

            /* Check the code unit width. */

            if ((*re).flags & PCRE2_MODE_MASK) != 1 {
                rc = PCRE2_ERROR_BADMODE;
                break 'EXIT;
            }

            /* Transfer the (*NOTEMPTY) / (*NOTEMPTY_ATSTART) pattern flags into the
            match options. */

            {
                const FF: u32 = PCRE2_NOTEMPTY_SET | PCRE2_NE_ATST_SET;
                const OO: u32 = PCRE2_NOTEMPTY | PCRE2_NOTEMPTY_ATSTART;
                options |= ((*re).flags & FF) / ((FF & FF.wrapping_neg()) / (OO & OO.wrapping_neg()));
            }

            /* If restarting after a partial match, do some sanity checks on the
            contents of the workspace. */

            if (options & PCRE2_DFA_RESTART) != 0 {
                if (*workspace.add(0) & (-2i32)) != 0
                    || *workspace.add(1) < 1
                    || *workspace.add(1)
                        > ((wscount - 2) / INTS_PER_STATEBLOCK as usize) as c_int
                {
                    rc = PCRE2_ERROR_DFA_BADRESTART;
                    break 'EXIT;
                }
            }

            /* Set some local values */

            utf = if ((*re).overall_options & PCRE2_UTF) != 0 {
                TRUE
            } else {
                FALSE
            };
            start_match = subject.add(start_offset);
            end_subject = subject.add(length);
            req_cu_ptr = start_match.wrapping_sub(1);
            anchored = if (options & (PCRE2_ANCHORED | PCRE2_DFA_RESTART)) != 0
                || ((*re).overall_options & PCRE2_ANCHORED) != 0
            {
                TRUE
            } else {
                FALSE
            };

            /* The "must be at the start of a line" flags are used in a loop when
            finding where to start. */

            startline = if ((*re).flags & PCRE2_STARTLINE) != 0 {
                TRUE
            } else {
                FALSE
            };
            firstline = if anchored == 0 && ((*re).overall_options & PCRE2_FIRSTLINE) != 0
            {
                TRUE
            } else {
                FALSE
            };
            bumpalong_limit = end_subject;

            /* Initialize and set up the fixed fields in the callout block. */

            (*mb).cb = &mut cb;
            cb.version = 2;
            cb.subject = subject;
            cb.subject_length = end_subject.offset_from(subject) as PCRE2_SIZE;
            cb.callout_flags = 0;
            cb.capture_top = 1; /* No capture support */
            cb.capture_last = 0;
            cb.mark = core::ptr::null(); /* No (*MARK) support */

            /* Get data from the match context, if present. */

            if mcontext.is_null() {
                (*mb).callout = None;
                (*mb).memctl = (*re).memctl;
                (*mb).match_limit = (*(&raw const crate::context::_pcre2_default_match_context_8))
                    .match_limit;
                (*mb).match_limit_depth =
                    (*(&raw const crate::context::_pcre2_default_match_context_8)).depth_limit;
                (*mb).heap_limit =
                    (*(&raw const crate::context::_pcre2_default_match_context_8)).heap_limit;
            } else {
                if (*mcontext).offset_limit != PCRE2_UNSET {
                    if ((*re).overall_options & PCRE2_USE_OFFSET_LIMIT) == 0 {
                        rc = PCRE2_ERROR_BADOFFSETLIMIT;
                        break 'EXIT;
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

            (*mb).start_code = (re as *const u8).add((*re).code_start);
            (*mb).tables = (*re).tables;
            (*mb).start_subject = subject;
            (*mb).end_subject = end_subject;
            (*mb).start_offset = start_offset;
            (*mb).allowemptypartial = if (*re).max_lookbehind > 0
                || ((*re).flags & PCRE2_MATCH_EMPTY) != 0
            {
                TRUE
            } else {
                FALSE
            };
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
                    (*mb).nl[0] = CHAR_CR as PCRE2_UCHAR;
                }

                PCRE2_NEWLINE_LF => {
                    (*mb).nllen = 1;
                    (*mb).nl[0] = CHAR_NL as PCRE2_UCHAR;
                }

                PCRE2_NEWLINE_NUL => {
                    (*mb).nllen = 1;
                    (*mb).nl[0] = CHAR_NUL as PCRE2_UCHAR;
                }

                PCRE2_NEWLINE_CRLF => {
                    (*mb).nllen = 2;
                    (*mb).nl[0] = CHAR_CR as PCRE2_UCHAR;
                    (*mb).nl[1] = CHAR_NL as PCRE2_UCHAR;
                }

                PCRE2_NEWLINE_ANY => {
                    (*mb).nltype = NLTYPE_ANY;
                }

                PCRE2_NEWLINE_ANYCRLF => {
                    (*mb).nltype = NLTYPE_ANYCRLF;
                }

                _ => {
                    rc = PCRE2_ERROR_INTERNAL;
                    break 'EXIT;
                }
            }

            /* Check a UTF string for validity if required. */

            if utf != 0 && (options & PCRE2_NO_UTF_CHECK) == 0 {
                let mut check_subject: PCRE2_SPTR = start_match; /* start_match includes offset */

                if start_offset > 0 {
                    let mut i: c_uint;
                    if start_match < end_subject && NOT_FIRSTCU(*start_match as u32) {
                        rc = PCRE2_ERROR_BADUTFOFFSET;
                        break 'EXIT;
                    }
                    i = (*re).max_lookbehind as c_uint;
                    while i > 0 && check_subject > subject {
                        check_subject = check_subject.sub(1);
                        while check_subject > subject && (*check_subject & 0xc0) == 0x80 {
                            check_subject = check_subject.sub(1);
                        }
                        i -= 1;
                    }
                }

                /* Validate the relevant portion of the subject. */

                rc = crate::valid_utf::_pcre2_valid_utf_8(
                    check_subject,
                    length - (check_subject.offset_from(subject) as PCRE2_SIZE),
                    &mut (*match_data).startchar,
                );
                if rc != 0 {
                    (*match_data).startchar +=
                        check_subject.offset_from(subject) as PCRE2_SIZE;
                    break 'EXIT;
                }
            }

            /* Set up the first code unit to match, if available. */

            if ((*re).flags & PCRE2_FIRSTSET) != 0 {
                has_first_cu = TRUE;
                first_cu = (*re).first_codeunit as PCRE2_UCHAR;
                first_cu2 = first_cu;
                if ((*re).flags & PCRE2_FIRSTCASELESS) != 0 {
                    first_cu2 = TABLE_GET(
                        first_cu as u32,
                        (*mb).tables.add(fcc_offset),
                        first_cu as u32,
                    ) as PCRE2_UCHAR;
                    if first_cu > 127 && utf == 0 && ((*re).overall_options & PCRE2_UCP) != 0
                    {
                        first_cu2 = UCD_OTHERCASE(first_cu as u32) as PCRE2_UCHAR;
                    }
                }
            } else {
                if startline == 0 && ((*re).flags & PCRE2_FIRSTMAPSET) != 0 {
                    start_bits = (*re).start_bitmap.as_ptr();
                }
            }

            /* There may be a "last known required code unit" set. */

            if ((*re).flags & PCRE2_LASTSET) != 0 {
                has_req_cu = TRUE;
                req_cu = (*re).last_codeunit as PCRE2_UCHAR;
                req_cu2 = req_cu;
                if ((*re).flags & PCRE2_LASTCASELESS) != 0 {
                    req_cu2 = TABLE_GET(
                        req_cu as u32,
                        (*mb).tables.add(fcc_offset),
                        req_cu as u32,
                    ) as PCRE2_UCHAR;
                    if req_cu > 127 && utf == 0 && ((*re).overall_options & PCRE2_UCP) != 0 {
                        req_cu2 = UCD_OTHERCASE(req_cu as u32) as PCRE2_UCHAR;
                    }
                }
            }

            /* If the match data block was previously used with
            PCRE2_COPY_MATCHED_SUBJECT, free the memory that was obtained. */

            if ((*match_data).flags & PCRE2_MD_COPIED_SUBJECT) != 0 {
                ((*match_data).memctl.free.unwrap())(
                    (*match_data).subject as *mut c_void,
                    (*match_data).memctl.memory_data,
                );
                (*match_data).flags &= !PCRE2_MD_COPIED_SUBJECT;
            }

            /* Fill in fields that are always returned in the match data. */

            (*match_data).code = re;
            (*match_data).subject = core::ptr::null(); /* Default for match error */
            (*match_data).mark = core::ptr::null();
            (*match_data).matchedby = PCRE2_MATCHEDBY_DFA_INTERPRETER as u8;
            (*match_data).options = original_options;

            /* Call the main matching function, looping for a non-anchored regex
            after a failed match. */

            'BUMPALONG: loop {
                /* ----------------- Start of match optimizations ---------------- */

                if ((*re).optimization_flags & PCRE2_OPTIM_START_OPTIMIZE) != 0
                    && (options & PCRE2_DFA_RESTART) == 0
                {
                    /* If firstline is TRUE, the start of the match is constrained to
                    the first line of a multiline string. */

                    if firstline != 0 {
                        let mut t: PCRE2_SPTR = start_match;
                        if utf != 0 {
                            while t < end_subject && !IS_NEWLINE!(t) {
                                t = t.add(1);
                                while t < end_subject && (*t & 0xc0) == 0x80 {
                                    t = t.add(1);
                                }
                            }
                        } else {
                            while t < end_subject && !IS_NEWLINE!(t) {
                                t = t.add(1);
                            }
                        }
                        end_subject = t;
                    }

                    /* Anchored: check the first code unit if one is recorded. */

                    if anchored != 0 {
                        if has_first_cu != 0 || !start_bits.is_null() {
                            let mut ok: BOOL =
                                if start_match < end_subject { TRUE } else { FALSE };
                            if ok != 0 {
                                let c: PCRE2_UCHAR = *start_match;
                                ok = if has_first_cu != 0
                                    && (c == first_cu || c == first_cu2)
                                {
                                    TRUE
                                } else {
                                    FALSE
                                };
                                if ok == 0 && !start_bits.is_null() {
                                    ok = if (*start_bits.add((c / 8) as usize)
                                        & (1u8 << (c & 7)))
                                        != 0
                                    {
                                        TRUE
                                    } else {
                                        FALSE
                                    };
                                }
                            }
                            if ok == 0 {
                                break 'BUMPALONG;
                            }
                        }
                    }
                    /* Not anchored. Advance to a unique first code unit if there is
                    one. */
                    else {
                        if has_first_cu != 0 {
                            if first_cu != first_cu2 {
                                /* Caseless */
                                /* In 8-bit mode, the use of memchr() gives a big speed
                                up, even though we have to call it twice in order to
                                find the earliest occurrence of the code unit in either
                                of its cases. */

                                let mut pp1: PCRE2_SPTR = core::ptr::null();
                                let mut pp2: PCRE2_SPTR = core::ptr::null();
                                let searchlength: PCRE2_SIZE =
                                    end_subject.offset_from(start_match) as PCRE2_SIZE;

                                if memchr_found_first_cu.is_null()
                                    || start_match > memchr_found_first_cu
                                {
                                    pp1 = memchr(
                                        start_match as *const c_void,
                                        first_cu as c_int,
                                        searchlength,
                                    ) as PCRE2_SPTR;
                                    memchr_found_first_cu =
                                        if pp1.is_null() { end_subject } else { pp1 };
                                } else {
                                    pp1 = if memchr_found_first_cu == end_subject {
                                        core::ptr::null()
                                    } else {
                                        memchr_found_first_cu
                                    };
                                }

                                /* Do the same thing for the other case. */

                                if memchr_found_first_cu2.is_null()
                                    || start_match > memchr_found_first_cu2
                                {
                                    pp2 = memchr(
                                        start_match as *const c_void,
                                        first_cu2 as c_int,
                                        searchlength,
                                    ) as PCRE2_SPTR;
                                    memchr_found_first_cu2 =
                                        if pp2.is_null() { end_subject } else { pp2 };
                                } else {
                                    pp2 = if memchr_found_first_cu2 == end_subject {
                                        core::ptr::null()
                                    } else {
                                        memchr_found_first_cu2
                                    };
                                }

                                /* Set the start to the end of the subject if neither
                                case was found. Otherwise, use the earlier found
                                point. */

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
                            }
                            /* The caseful case is much simpler. */
                            else {
                                start_match = memchr(
                                    start_match as *const c_void,
                                    first_cu as c_int,
                                    end_subject.offset_from(start_match) as usize,
                                ) as PCRE2_SPTR;
                                if start_match.is_null() {
                                    start_match = end_subject;
                                }
                            }

                            /* If we can't find the required code unit, having reached
                            the true end of the subject, break the bumpalong loop. */

                            if ((*mb).moptions & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT))
                                == 0
                                && start_match >= (*mb).end_subject
                            {
                                break 'BUMPALONG;
                            }
                        }
                        /* If there's no first code unit, advance to just after a
                        linebreak for a multiline match if required. */
                        else if startline != 0 {
                            if start_match > (*mb).start_subject.add(start_offset) {
                                if utf != 0 {
                                    while start_match < end_subject
                                        && !WAS_NEWLINE!(start_match)
                                    {
                                        start_match = start_match.add(1);
                                        while start_match < end_subject
                                            && (*start_match & 0xc0) == 0x80
                                        {
                                            start_match = start_match.add(1);
                                        }
                                    }
                                } else {
                                    while start_match < end_subject
                                        && !WAS_NEWLINE!(start_match)
                                    {
                                        start_match = start_match.add(1);
                                    }
                                }

                                /* If we have just passed a CR and the newline option is
                                ANY or ANYCRLF, and we are now at a LF, advance the
                                match position by one more code unit. */

                                if *start_match.offset(-1) as u32 == CHAR_CR
                                    && ((*mb).nltype == NLTYPE_ANY
                                        || (*mb).nltype == NLTYPE_ANYCRLF)
                                    && start_match < end_subject
                                    && *start_match as u32 == CHAR_NL
                                {
                                    start_match = start_match.add(1);
                                }
                            }
                        }
                        /* If there's no first code unit or a requirement for a
                        multiline line start, advance to a non-unique first code unit if
                        any have been identified. */
                        else if !start_bits.is_null() {
                            while start_match < end_subject {
                                let c: u32 = *start_match as u32;
                                if (*start_bits.add((c / 8) as usize) & (1u8 << (c & 7)))
                                    != 0
                                {
                                    break;
                                }
                                start_match = start_match.add(1);
                            }

                            /* See comment above in first_cu checking about the next
                            line. */

                            if ((*mb).moptions & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT))
                                == 0
                                && start_match >= (*mb).end_subject
                            {
                                break 'BUMPALONG;
                            }
                        }
                    } /* End of first code unit handling */

                    /* Restore fudged end_subject */

                    end_subject = (*mb).end_subject;

                    /* The following two optimizations are disabled for partial
                    matching. */

                    if ((*mb).moptions & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT)) == 0 {
                        let mut p: PCRE2_SPTR;

                        /* The minimum matching length is a lower bound. */

                        if (end_subject.offset_from(start_match) as isize)
                            < (*re).minlength as isize
                        {
                            break 'NOMATCH_EXIT;
                        }

                        /* If req_cu is set, we know that that code unit must appear in
                        the subject for the match to succeed. */

                        p = start_match.add(if has_first_cu != 0 { 1 } else { 0 });
                        if has_req_cu != 0 && p > req_cu_ptr {
                            let check_length: PCRE2_SIZE =
                                end_subject.offset_from(start_match) as PCRE2_SIZE;

                            if check_length < REQ_CU_MAX
                                || (anchored == 0 && check_length < REQ_CU_MAX * 1000)
                            {
                                if req_cu != req_cu2 {
                                    /* Caseless */
                                    let pp: PCRE2_SPTR = p;
                                    p = memchr(
                                        pp as *const c_void,
                                        req_cu as c_int,
                                        end_subject.offset_from(pp) as usize,
                                    ) as PCRE2_SPTR;
                                    if p.is_null() {
                                        p = memchr(
                                            pp as *const c_void,
                                            req_cu2 as c_int,
                                            end_subject.offset_from(pp) as usize,
                                        ) as PCRE2_SPTR;
                                        if p.is_null() {
                                            p = end_subject;
                                        }
                                    }
                                }
                                /* The caseful case */
                                else {
                                    p = memchr(
                                        p as *const c_void,
                                        req_cu as c_int,
                                        end_subject.offset_from(p) as usize,
                                    ) as PCRE2_SPTR;
                                    if p.is_null() {
                                        p = end_subject;
                                    }
                                }

                                /* If we can't find the required code unit, break the
                                matching loop, forcing a match failure. */

                                if p >= end_subject {
                                    break 'BUMPALONG;
                                }

                                /* If we have found the required code unit, save the
                                point where we found it. */

                                req_cu_ptr = p;
                            }
                        }
                    }
                }

                /* ------------ End of start of match optimizations ------------ */

                /* Give no match if we have passed the bumpalong limit. */

                if start_match > bumpalong_limit {
                    break 'BUMPALONG;
                }

                /* OK, now we can do the business */

                (*mb).start_used_ptr = start_match;
                (*mb).last_used_ptr = start_match;
                (*mb).recursive = core::ptr::null_mut();

                rc = internal_dfa_match(
                    mb,                                     /* fixed match data */
                    (*mb).start_code,                       /* this subexpression's code */
                    start_match,                            /* where we currently are */
                    start_offset,                           /* start offset in subject */
                    (*match_data).ovector.as_mut_ptr(),     /* offset vector */
                    (*match_data).oveccount as u32 * 2,     /* actual size of same */
                    workspace,                              /* workspace vector */
                    wscount as c_int,                       /* size of same */
                    0,                                      /* function recurse level */
                    base_recursion_workspace,               /* initial workspace */
                );

                /* Anything other than "no match" means we are done, always;
                otherwise, carry on only if not anchored. */

                if rc != PCRE2_ERROR_NOMATCH || anchored != 0 {
                    if rc == PCRE2_ERROR_NOMATCH {
                        break 'NOMATCH_EXIT;
                    }

                    if rc == PCRE2_ERROR_PARTIAL && (*match_data).oveccount > 0 {
                        (*match_data).ovector[0] =
                            start_match.offset_from(subject) as PCRE2_SIZE;
                        (*match_data).ovector[1] =
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
                                CU2BYTES(length),
                                (*match_data).memctl.memory_data,
                            ) as PCRE2_SPTR;
                            if (*match_data).subject.is_null() {
                                rc = PCRE2_ERROR_NOMEMORY;
                                break 'EXIT;
                            }
                            memcpy(
                                (*match_data).subject as *mut c_void,
                                subject as *const c_void,
                                CU2BYTES(length),
                            );
                        } else {
                            (*match_data).subject = core::ptr::null();
                        }
                        (*match_data).flags |= PCRE2_MD_COPIED_SUBJECT;
                    } else if rc >= 0 || rc == PCRE2_ERROR_PARTIAL {
                        (*match_data).subject = original_subject;
                    }
                    break 'EXIT;
                }

                /* Advance to the next subject character unless we are at the end of a
                line and firstline is set. */

                if firstline != 0 && IS_NEWLINE!(start_match) {
                    break 'BUMPALONG;
                }
                start_match = start_match.add(1);
                if utf != 0 {
                    while start_match < end_subject && (*start_match & 0xc0) == 0x80 {
                        start_match = start_match.add(1);
                    }
                }
                if start_match > end_subject {
                    break 'BUMPALONG;
                }

                /* If we have just passed a CR and we are now at a LF, and the pattern
                does not contain any explicit matches for \r or \n, and the newline
                option is CRLF or ANY or ANYCRLF, advance the match position by one
                more character. */

                if *start_match.offset(-1) as u32 == CHAR_CR
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
        }

        /* NOMATCH_EXIT: */
        (*match_data).subject = original_subject;
        (*match_data).subject_length = length;
        (*match_data).start_offset = start_offset;
        rc = PCRE2_ERROR_NOMATCH;
    }

    /* EXIT: */
    while !(*rws).next.is_null() {
        let next: *mut RWS_anchor = (*rws).next;
        (*rws).next = (*next).next;
        ((*mb).memctl.free.unwrap())(next as *mut c_void, (*mb).memctl.memory_data);
    }

    (*match_data).rc = rc;
    rc
}

/* End of pcre2_dfa_match.c */
