//! Translated from pcre2_dfa_match.c.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use core::ffi::{c_char, c_void};

/* #define NLBLOCK mb             Block containing newline information */
/* #define PSSTART start_subject  Field containing processed string start */
/* #define PSEND   end_subject    Field containing processed string end */

/// IS_NEWLINE(p) with NLBLOCK == mb.
macro_rules! IS_NEWLINE {
    ($p:expr, $mb:expr, $utf:expr) => {
        crate::macros::is_newline_block(
            $p,
            (*$mb).nltype,
            &mut (*$mb).nllen,
            (*$mb).nl.as_ptr(),
            (*$mb).end_subject,
            $utf,
        )
    };
}

/// WAS_NEWLINE(p) with NLBLOCK == mb.
macro_rules! WAS_NEWLINE {
    ($p:expr, $mb:expr, $utf:expr) => {
        crate::macros::was_newline_block(
            $p,
            (*$mb).nltype,
            &mut (*$mb).nllen,
            (*$mb).nl.as_ptr(),
            (*$mb).start_subject,
            $utf,
        )
    };
}

pub(crate) const PUBLIC_DFA_MATCH_OPTIONS: u32 = PCRE2_ANCHORED
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

/* These are offsets that are used to turn the OP_TYPESTAR and friends opcodes
into others, under special conditions. A gap of 20 between the blocks should be
enough. The resulting opcodes don't have to be less than 256 because they are
never stored, so we push them well clear of the normal opcodes. */

pub(crate) const OP_PROP_EXTRA: u32 = 300;
pub(crate) const OP_EXTUNI_EXTRA: u32 = 320;
pub(crate) const OP_ANYNL_EXTRA: u32 = 340;
pub(crate) const OP_HSPACE_EXTRA: u32 = 360;
pub(crate) const OP_VSPACE_EXTRA: u32 = 380;

/* This table identifies those opcodes that are followed immediately by a
character that is to be tested in some way. This makes it possible to
centralize the loading of these characters. In the case of Type * etc, the
"character" is the opcode for \D, \d, \S, \s, \W, or \w, which will always be a
small value. Non-zero values in the table are the offsets from the opcode where
the character is to be found. ***NOTE*** If the start of this table is
modified, the three tables that follow must also be modified. */

pub(crate) static coptable: [u8; OP_TABLE_LENGTH as usize] = [
    0,                                         /* End                                    */
    0, 0, 0, 0, 0,                             /* \A, \G, \K, \B, \b                     */
    0, 0, 0, 0, 0, 0,                          /* \D, \d, \S, \s, \W, \w                 */
    0, 0, 0,                                   /* Any, AllAny, Anybyte                   */
    0, 0,                                      /* \P, \p                                 */
    0, 0, 0, 0, 0,                             /* \R, \H, \h, \V, \v                     */
    0,                                         /* \X                                     */
    0, 0, 0, 0, 0, 0,                          /* \Z, \z, $, $M, ^, ^M                   */
    1,                                         /* Char                                   */
    1,                                         /* Chari                                  */
    1,                                         /* not                                    */
    1,                                         /* noti                                   */
    /* Positive single-char repeats                                                       */
    1, 1, 1, 1, 1, 1,                          /* *, *?, +, +?, ?, ??                    */
    1 + IMM2_SIZE as u8, 1 + IMM2_SIZE as u8,  /* upto, minupto                          */
    1 + IMM2_SIZE as u8,                       /* exact                                  */
    1, 1, 1, 1 + IMM2_SIZE as u8,              /* *+, ++, ?+, upto+                      */
    1, 1, 1, 1, 1, 1,                          /* *I, *?I, +I, +?I, ?I, ??I              */
    1 + IMM2_SIZE as u8, 1 + IMM2_SIZE as u8,  /* upto I, minupto I                      */
    1 + IMM2_SIZE as u8,                       /* exact I                                */
    1, 1, 1, 1 + IMM2_SIZE as u8,              /* *+I, ++I, ?+I, upto+I                  */
    /* Negative single-char repeats - only for chars < 256                                */
    1, 1, 1, 1, 1, 1,                          /* NOT *, *?, +, +?, ?, ??                */
    1 + IMM2_SIZE as u8, 1 + IMM2_SIZE as u8,  /* NOT upto, minupto                      */
    1 + IMM2_SIZE as u8,                       /* NOT exact                              */
    1, 1, 1, 1 + IMM2_SIZE as u8,              /* NOT *+, ++, ?+, upto+                  */
    1, 1, 1, 1, 1, 1,                          /* NOT *I, *?I, +I, +?I, ?I, ??I          */
    1 + IMM2_SIZE as u8, 1 + IMM2_SIZE as u8,  /* NOT upto I, minupto I                  */
    1 + IMM2_SIZE as u8,                       /* NOT exact I                            */
    1, 1, 1, 1 + IMM2_SIZE as u8,              /* NOT *+I, ++I, ?+I, upto+I              */
    /* Positive type repeats                                                              */
    1, 1, 1, 1, 1, 1,                          /* Type *, *?, +, +?, ?, ??               */
    1 + IMM2_SIZE as u8, 1 + IMM2_SIZE as u8,  /* Type upto, minupto                     */
    1 + IMM2_SIZE as u8,                       /* Type exact                             */
    1, 1, 1, 1 + IMM2_SIZE as u8,              /* Type *+, ++, ?+, upto+                 */
    /* Character class & ref repeats                                                      */
    0, 0, 0, 0, 0, 0,                          /* *, *?, +, +?, ?, ??                    */
    0, 0,                                      /* CRRANGE, CRMINRANGE                    */
    0, 0, 0, 0,                                /* Possessive *+, ++, ?+, CRPOSRANGE      */
    0,                                         /* CLASS                                  */
    0,                                         /* NCLASS                                 */
    0,                                         /* XCLASS - variable length               */
    0,                                         /* ECLASS - variable length               */
    0,                                         /* REF                                    */
    0,                                         /* REFI                                   */
    0,                                         /* DNREF                                  */
    0,                                         /* DNREFI                                 */
    0,                                         /* RECURSE                                */
    0,                                         /* CALLOUT                                */
    0,                                         /* CALLOUT_STR                            */
    0,                                         /* Alt                                    */
    0,                                         /* Ket                                    */
    0,                                         /* KetRmax                                */
    0,                                         /* KetRmin                                */
    0,                                         /* KetRpos                                */
    0, 0,                                      /* Reverse, Vreverse                      */
    0,                                         /* Assert                                 */
    0,                                         /* Assert not                             */
    0,                                         /* Assert behind                          */
    0,                                         /* Assert behind not                      */
    0,                                         /* NA assert                              */
    0,                                         /* NA assert behind                       */
    0,                                         /* Assert scan substring                  */
    0,                                         /* ONCE                                   */
    0,                                         /* SCRIPT_RUN                             */
    0, 0, 0, 0, 0,                             /* BRA, BRAPOS, CBRA, CBRAPOS, COND       */
    0, 0, 0, 0, 0,                             /* SBRA, SBRAPOS, SCBRA, SCBRAPOS, SCOND  */
    0, 0,                                      /* CREF, DNCREF                           */
    0, 0,                                      /* RREF, DNRREF                           */
    0, 0,                                      /* FALSE, TRUE                            */
    0, 0, 0,                                   /* BRAZERO, BRAMINZERO, BRAPOSZERO        */
    0, 0, 0,                                   /* MARK, PRUNE, PRUNE_ARG                 */
    0, 0, 0, 0,                                /* SKIP, SKIP_ARG, THEN, THEN_ARG         */
    0, 0,                                      /* COMMIT, COMMIT_ARG                     */
    0, 0, 0,                                   /* FAIL, ACCEPT, ASSERT_ACCEPT            */
    0, 0, 0,                                   /* CLOSE, SKIPZERO, DEFINE                */
    0, 0,                                      /* \B and \b in UCP mode                  */
];

/* This table identifies those opcodes that inspect a character. It is used to
remember the fact that a character could have been inspected when the end of
the subject is reached. ***NOTE*** If the start of this table is modified, the
two tables that follow must also be modified. */

pub(crate) static poptable: [u8; OP_TABLE_LENGTH as usize] = [
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

pub(crate) static toptable1: [u8; 14] = [
    0, 0, 0, 0, 0, 0,
    ctype_digit, ctype_digit,
    ctype_space, ctype_space,
    ctype_word, ctype_word,
    0, 0,                           /* OP_ANY, OP_ALLANY */
];

pub(crate) static toptable2: [u8; 14] = [
    0, 0, 0, 0, 0, 0,
    ctype_digit, 0,
    ctype_space, 0,
    ctype_word, 0,
    1, 1,                           /* OP_ANY, OP_ALLANY */
];

/* Structure for holding data about a particular state, which is in effect the
current data for an active path through the match tree. It must consist
entirely of ints because the working vector we are passed, and which we put
these structures in, is a vector of ints. */

#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct stateblock {
    pub offset: i32, /* Offset to opcode (-ve has meaning) */
    pub count: i32,  /* Count for repeats */
    pub data: i32,   /* Some use extra data */
}

pub(crate) const INTS_PER_STATEBLOCK: i32 =
    (core::mem::size_of::<stateblock>() / core::mem::size_of::<i32>()) as i32;

/* Before version 10.32 the recursive calls of internal_dfa_match() were passed
local working space and output vectors that were created on the stack. This has
caused issues for some patterns, especially in small-stack environments such as
Windows. A new scheme is now in use which sets up a vector on the stack, but if
this is too small, heap memory is used, up to the heap_limit. The main
parameters are all numbers of ints because the workspace is a vector of ints.

The size of the starting stack vector, DFA_START_RWS_SIZE, is in bytes, and is
defined in pcre2_internal.h so as to be available to pcre2test when it is
finding the minimum heap requirement for a match. */

pub(crate) const OVEC_UNIT: usize =
    core::mem::size_of::<PCRE2_SIZE>() / core::mem::size_of::<i32>();

pub(crate) const RWS_BASE_SIZE: usize = DFA_START_RWS_SIZE / core::mem::size_of::<i32>();
pub(crate) const RWS_RSIZE: usize = 1000; /* Work size for recursion */
pub(crate) const RWS_OVEC_RSIZE: usize = 1000 * OVEC_UNIT; /* Ovector for recursion */
pub(crate) const RWS_OVEC_OSIZE: usize = 2 * OVEC_UNIT; /* Ovector in other cases */

/* This structure is at the start of each workspace block. */

#[repr(C)]
pub(crate) struct RWS_anchor {
    pub next: *mut RWS_anchor,
    pub size: u32, /* Number of ints */
    pub free: u32, /* Number of ints */
}

pub(crate) const RWS_ANCHOR_SIZE: usize =
    core::mem::size_of::<RWS_anchor>() / core::mem::size_of::<i32>();

/* A local memchr() for 8-bit code units. */

pub(crate) unsafe fn dfa_memchr(s: PCRE2_SPTR, c: u8, n: PCRE2_SIZE) -> PCRE2_SPTR {
    let mut i: PCRE2_SIZE = 0;
    while i < n {
        if *s.add(i) == c {
            return s.add(i);
        }
        i += 1;
    }
    core::ptr::null()
}

/*************************************************
*               Process a callout                *
*************************************************/

/* This function is called to perform a callout.

Arguments:
  code              current code pointer
  offsets           points to current capture offsets
  current_subject   start of current subject match
  ptr               current position in subject
  mb                the match block
  extracode         extra code offset when called from condition
  lengthptr         where to return the callout length

Returns:            the return from the callout
*/

pub(crate) unsafe fn do_callout_dfa(
    code: PCRE2_SPTR,
    offsets: *mut PCRE2_SIZE,
    current_subject: PCRE2_SPTR,
    ptr: PCRE2_SPTR,
    mb: *mut dfa_match_block,
    extracode: PCRE2_SIZE,
    lengthptr: *mut PCRE2_SIZE,
) -> i32 {
    let cb: *mut pcre2_callout_block = (*mb).cb;

    *lengthptr = if *code.add(extracode) as u32 == OP_CALLOUT {
        *crate::tables::_pcre2_OP_lengths_8
            .as_ptr()
            .add(OP_CALLOUT as usize) as PCRE2_SIZE
    } else {
        GET!(code, 1 + 2 * LINK_SIZE + extracode) as PCRE2_SIZE
    };

    if (*mb).callout.is_none() {
        return 0; /* No callout provided */
    }

    /* Fixed fields in the callout block are set once and for all at the start of
    matching. */

    (*cb).offset_vector = offsets;
    (*cb).start_match = (current_subject as usize - (*mb).start_subject as usize) as PCRE2_SIZE;
    (*cb).current_position = (ptr as usize - (*mb).start_subject as usize) as PCRE2_SIZE;
    (*cb).pattern_position = GET!(code, 1 + extracode) as PCRE2_SIZE;
    (*cb).next_item_length = GET!(code, 1 + LINK_SIZE + extracode) as PCRE2_SIZE;

    if *code.add(extracode) as u32 == OP_CALLOUT {
        (*cb).callout_number = *code.add(1 + 2 * LINK_SIZE + extracode) as u32;
        (*cb).callout_string_offset = 0;
        (*cb).callout_string = core::ptr::null();
        (*cb).callout_string_length = 0;
    } else {
        (*cb).callout_number = 0;
        (*cb).callout_string_offset = GET!(code, 1 + 3 * LINK_SIZE + extracode) as PCRE2_SIZE;
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

/* This function is called when internal_dfa_match() is about to be called
recursively and there is insufficient working space left in the current
workspace block. If there's an existing next block, use it; otherwise get a new
block unless the heap limit is reached.

Arguments:
  rwsptr     pointer to block pointer (updated)
  ovecsize   space needed for an ovector
  mb         the match block

Returns:     0 rwsptr has been updated
            !0 an error code
*/

pub(crate) unsafe fn more_workspace(
    rwsptr: *mut *mut RWS_anchor,
    ovecsize: u32,
    mb: *mut dfa_match_block,
) -> i32 {
    let rws: *mut RWS_anchor = *rwsptr;
    let new: *mut RWS_anchor;

    if !(*rws).next.is_null() {
        new = (*rws).next;
    }
    /* Sizes in the RWS_anchor blocks are in units of sizeof(int), but
    mb->heap_limit and mb->heap_used are in kibibytes. Play carefully, to avoid
    overflow. */
    else {
        let mut newsize: u32 = if (*rws).size as usize
            >= (u32::MAX as usize) / (core::mem::size_of::<i32>() * 2)
        {
            ((u32::MAX as usize) / core::mem::size_of::<i32>()) as u32
        } else {
            (*rws).size * 2
        };
        let mut newsizeK: u32 =
            ((newsize as usize) / (1024 / core::mem::size_of::<i32>())) as u32;

        if (newsizeK as usize) + (*mb).heap_used > (*mb).heap_limit as usize {
            newsizeK = ((*mb).heap_limit as usize - (*mb).heap_used) as u32;
        }
        newsize = ((newsizeK as usize) * (1024 / core::mem::size_of::<i32>())) as u32;

        if (newsize as usize) < (1000u32.wrapping_add(ovecsize)) as usize + RWS_ANCHOR_SIZE {
            return PCRE2_ERROR_HEAPLIMIT;
        }
        new = ((*mb).memctl.malloc.unwrap())(
            (newsize as usize) * core::mem::size_of::<i32>(),
            (*mb).memctl.memory_data,
        ) as *mut RWS_anchor;
        if new.is_null() {
            return PCRE2_ERROR_NOMEMORY;
        }
        (*mb).heap_used += newsizeK as usize;
        (*new).next = core::ptr::null_mut();
        (*new).size = newsize;
        (*rws).next = new;
    }

    (*new).free = ((*new).size as usize - RWS_ANCHOR_SIZE) as u32;
    *rwsptr = new;
    0
}

/*************************************************
*     Match a pattern using the DFA algorithm    *
*************************************************/

/* This function matches a compiled pattern to a subject string, using the
alternate matching algorithm that finds all matches at once.

Arguments:
  code          points to the compiled pattern
  subject       subject string
  length        length of subject string
  startoffset   where to start matching in the subject
  options       option bits
  match_data    points to a match data structure
  gcontext      points to a match context
  workspace     pointer to workspace
  wscount       size of workspace

Returns:        > 0 => number of match offset pairs placed in offsets
                = 0 => offsets overflowed; longest matches are present
                 -1 => failed to match
               < -1 => some kind of unexpected problem
*/

#[repr(C, align(8))]
struct BaseRecursionWorkspace([core::mem::MaybeUninit<i32>; RWS_BASE_SIZE]);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_dfa_match_8(
    code: *const pcre2_real_code,
    mut subject: PCRE2_SPTR,
    mut length: PCRE2_SIZE,
    start_offset: PCRE2_SIZE,
    mut options: u32,
    match_data: *mut pcre2_real_match_data,
    mcontext: *mut pcre2_real_match_context,
    workspace: *mut i32,
    wscount: PCRE2_SIZE,
) -> i32 {
    let mut rc: i32 = 0;

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
    internal_dfa_match(). By putting this on the stack, it minimizes resource use
    in the case when it is not needed. If this is too small, more memory is
    obtained from the heap. At the start of each block is an anchor structure.*/

    let mut base_recursion_workspace: BaseRecursionWorkspace =
        BaseRecursionWorkspace([const { core::mem::MaybeUninit::uninit() }; RWS_BASE_SIZE]);
    let rws: *mut RWS_anchor = base_recursion_workspace.0.as_mut_ptr() as *mut RWS_anchor;
    (*rws).next = core::ptr::null_mut();
    (*rws).size = RWS_BASE_SIZE as u32;
    (*rws).free = (RWS_BASE_SIZE - RWS_ANCHOR_SIZE) as u32;

    'EXIT: {
        'NOMATCH_EXIT: {
            /* Recognize NULL, length 0 as an empty string. */

            if subject.is_null() && length == 0 {
                subject = null_str.as_ptr();
            }

            /* Plausibility checks */

            if match_data.is_null() {
                return PCRE2_ERROR_NULL;
            }
            if re.is_null() || subject.is_null() || workspace.is_null() {
                rc = PCRE2_ERROR_NULL;
                break 'EXIT; /* goto EXIT */
            }
            if (options & !PUBLIC_DFA_MATCH_OPTIONS) != 0 {
                rc = PCRE2_ERROR_BADOPTION;
                break 'EXIT; /* goto EXIT */
            }

            if length == PCRE2_ZERO_TERMINATED {
                length = crate::string_utils::_pcre2_strlen_8(subject);
            }

            if wscount < 20 {
                rc = PCRE2_ERROR_DFA_WSSIZE;
                break 'EXIT; /* goto EXIT */
            }
            if start_offset > length {
                rc = PCRE2_ERROR_BADOFFSET;
                break 'EXIT; /* goto EXIT */
            }

            /* Partial matching and PCRE2_ENDANCHORED are currently not allowed at the same
            time. */

            if (options & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT)) != 0
                && (((*re).overall_options | options) & PCRE2_ENDANCHORED) != 0
            {
                rc = PCRE2_ERROR_BADOPTION;
                break 'EXIT; /* goto EXIT */
            }

            /* Invalid UTF support is not available for DFA matching. */

            if ((*re).overall_options & PCRE2_MATCH_INVALID_UTF) != 0 {
                rc = PCRE2_ERROR_DFA_UINVALID_UTF;
                break 'EXIT; /* goto EXIT */
            }

            /* Check that the first field in the block is the magic number. If it is not,
            return with PCRE2_ERROR_BADMAGIC. */

            if (*re).magic_number != MAGIC_NUMBER {
                rc = PCRE2_ERROR_BADMAGIC;
                break 'EXIT; /* goto EXIT */
            }

            /* Check the code unit width. */

            if ((*re).flags & PCRE2_MODE_MASK) != (8u32 / 8)
            /* PCRE2_CODE_UNIT_WIDTH/8 */
            {
                rc = PCRE2_ERROR_BADMODE;
                break 'EXIT; /* goto EXIT */
            }

            /* PCRE2_NOTEMPTY and PCRE2_NOTEMPTY_ATSTART are match-time flags in the
            options variable for this function. Users of PCRE2 who are not calling the
            function directly would like to have a way of setting these flags, in the same
            way that they can set pcre2_compile() flags like PCRE2_NO_AUTO_POSSESS with
            constructions like (*NO_AUTOPOSSESS). To enable this, (*NOTEMPTY) and
            (*NOTEMPTY_ATSTART) set bits in the pattern's "flag" function which can now be
            transferred to the options for this function. The bits are guaranteed to be
            adjacent, but do not have the same values. This bit of Boolean trickery assumes
            that the match-time bits are not more significant than the flag bits. If by
            accident this is not the case, a compile-time division by zero error will
            occur. */

            {
                const FF: u32 = PCRE2_NOTEMPTY_SET | PCRE2_NE_ATST_SET;
                const OO: u32 = PCRE2_NOTEMPTY | PCRE2_NOTEMPTY_ATSTART;
                options |= ((*re).flags & FF)
                    / ((FF & (!FF).wrapping_add(1)) / (OO & (!OO).wrapping_add(1)));
            }

            /* If restarting after a partial match, do some sanity checks on the contents
            of the workspace. */

            if (options & PCRE2_DFA_RESTART) != 0 {
                if (*workspace & (-2i32)) != 0
                    || *workspace.add(1) < 1
                    || *workspace.add(1)
                        > ((wscount - 2) / (INTS_PER_STATEBLOCK as usize)) as i32
                {
                    rc = PCRE2_ERROR_DFA_BADRESTART;
                    break 'EXIT; /* goto EXIT */
                }
            }

            /* Set some local values */

            utf = (((*re).overall_options & PCRE2_UTF) != 0) as BOOL;
            start_match = subject.add(start_offset);
            end_subject = subject.add(length);
            req_cu_ptr = start_match.wrapping_sub(1);
            anchored = ((options & (PCRE2_ANCHORED | PCRE2_DFA_RESTART)) != 0
                || ((*re).overall_options & PCRE2_ANCHORED) != 0) as BOOL;

            /* The "must be at the start of a line" flags are used in a loop when finding
            where to start. */

            startline = (((*re).flags & PCRE2_STARTLINE) != 0) as BOOL;
            firstline =
                (anchored == 0 && ((*re).overall_options & PCRE2_FIRSTLINE) != 0) as BOOL;
            bumpalong_limit = end_subject;

            /* Initialize and set up the fixed fields in the callout block, with a pointer
            in the match block. */

            (*mb).cb = &mut cb;
            cb.version = 2;
            cb.subject = subject;
            cb.subject_length = (end_subject as usize - subject as usize) as PCRE2_SIZE;
            cb.callout_flags = 0;
            cb.capture_top = 1; /* No capture support */
            cb.capture_last = 0;
            cb.mark = core::ptr::null(); /* No (*MARK) support */

            /* Get data from the match context, if present, and fill in the remaining
            fields in the match block. It is an error to set an offset limit without
            setting the flag at compile time. */

            if mcontext.is_null() {
                (*mb).callout = None;
                (*mb).memctl = (*re).memctl;
                (*mb).match_limit =
                    (*core::ptr::addr_of!(crate::context::_pcre2_default_match_context_8))
                        .match_limit;
                (*mb).match_limit_depth =
                    (*core::ptr::addr_of!(crate::context::_pcre2_default_match_context_8))
                        .depth_limit;
                (*mb).heap_limit =
                    (*core::ptr::addr_of!(crate::context::_pcre2_default_match_context_8))
                        .heap_limit;
            } else {
                if (*mcontext).offset_limit != PCRE2_UNSET {
                    if ((*re).overall_options & PCRE2_USE_OFFSET_LIMIT) == 0 {
                        rc = PCRE2_ERROR_BADOFFSETLIMIT;
                        break 'EXIT; /* goto EXIT */
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

            (*mb).start_code = (re as *const u8).add((*re).code_start) as PCRE2_SPTR;
            (*mb).tables = (*re).tables;
            (*mb).start_subject = subject;
            (*mb).end_subject = end_subject;
            (*mb).start_offset = start_offset;
            (*mb).allowemptypartial = (((*re).max_lookbehind > 0)
                || ((*re).flags & PCRE2_MATCH_EMPTY) != 0) as BOOL;
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
                    (*mb).nl[0] = 0x0d; /* CHAR_CR */
                }

                PCRE2_NEWLINE_LF => {
                    (*mb).nllen = 1;
                    (*mb).nl[0] = 0x0a; /* CHAR_NL */
                }

                PCRE2_NEWLINE_NUL => {
                    (*mb).nllen = 1;
                    (*mb).nl[0] = 0x00; /* CHAR_NUL */
                }

                PCRE2_NEWLINE_CRLF => {
                    (*mb).nllen = 2;
                    (*mb).nl[0] = 0x0d; /* CHAR_CR */
                    (*mb).nl[1] = 0x0a; /* CHAR_NL */
                }

                PCRE2_NEWLINE_ANY => {
                    (*mb).nltype = NLTYPE_ANY;
                }

                PCRE2_NEWLINE_ANYCRLF => {
                    (*mb).nltype = NLTYPE_ANYCRLF;
                }

                /* LCOV_EXCL_START */
                _ => {
                    rc = PCRE2_ERROR_INTERNAL;
                    break 'EXIT; /* goto EXIT */
                } /* LCOV_EXCL_STOP */
            }

            /* Check a UTF string for validity if required. For 8-bit and 16-bit strings,
            we must also check that a starting offset does not point into the middle of a
            multiunit character. We check only the portion of the subject that is going to
            be inspected during matching - from the offset minus the maximum back reference
            to the given length. This saves time when a small part of a large subject is
            being matched by the use of a starting offset. Note that the maximum lookbehind
            is a number of characters, not code units. */

            if utf != 0 && (options & PCRE2_NO_UTF_CHECK) == 0 {
                let mut check_subject: PCRE2_SPTR = start_match; /* start_match includes offset */

                if start_offset > 0 {
                    let mut i: u32;
                    if start_match < end_subject && NOT_FIRSTCU!(*start_match) {
                        rc = PCRE2_ERROR_BADUTFOFFSET;
                        break 'EXIT; /* goto EXIT */
                    }
                    i = (*re).max_lookbehind as u32;
                    while i > 0 && check_subject > subject {
                        check_subject = check_subject.sub(1);
                        while check_subject > subject && (*check_subject & 0xc0) == 0x80 {
                            check_subject = check_subject.sub(1);
                        }
                        i -= 1;
                    }
                }

                /* Validate the relevant portion of the subject. After an error, adjust the
                offset to be an absolute offset in the whole string. */

                rc = crate::valid_utf::_pcre2_valid_utf_8(
                    check_subject,
                    length - (check_subject as usize - subject as usize) as PCRE2_SIZE,
                    &mut (*match_data).startchar,
                );
                if rc != 0 {
                    (*match_data).startchar +=
                        (check_subject as usize - subject as usize) as PCRE2_SIZE;
                    break 'EXIT; /* goto EXIT */
                }
            }

            /* Set up the first code unit to match, if available. If there's no first code
            unit there may be a bitmap of possible first characters. */

            if ((*re).flags & PCRE2_FIRSTSET) != 0 {
                has_first_cu = TRUE;
                first_cu = (*re).first_codeunit as PCRE2_UCHAR;
                first_cu2 = first_cu;
                if ((*re).flags & PCRE2_FIRSTCASELESS) != 0 {
                    first_cu2 = TABLE_GET!(first_cu, (*mb).tables.add(fcc_offset), first_cu);
                    if first_cu > 127 && utf == 0 && ((*re).overall_options & PCRE2_UCP) != 0 {
                        first_cu2 = UCD_OTHERCASE!(first_cu as u32) as PCRE2_UCHAR;
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
                    req_cu2 = TABLE_GET!(req_cu, (*mb).tables.add(fcc_offset), req_cu);
                    if req_cu > 127 && utf == 0 && ((*re).overall_options & PCRE2_UCP) != 0 {
                        req_cu2 = UCD_OTHERCASE!(req_cu as u32) as PCRE2_UCHAR;
                    }
                }
            }

            /* If the match data block was previously used with PCRE2_COPY_MATCHED_SUBJECT,
            free the memory that was obtained. */

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
            (*match_data).matchedby = PCRE2_MATCHEDBY_DFA_INTERPRETER;
            (*match_data).options = original_options;

            /* Call the main matching function, looping for a non-anchored regex after a
            failed match. If not restarting, perform certain optimizations at the start of
            a match. */

            loop {
                /* ----------------- Start of match optimizations ---------------- */

                /* There are some optimizations that avoid running the match if a known
                starting point is not found, or if a known later code unit is not present.
                However, there is an option (settable at compile time) that disables
                these, for testing and for ensuring that all callouts do actually occur.
                The optimizations must also be avoided when restarting a DFA match. */

                if ((*re).optimization_flags & PCRE2_OPTIM_START_OPTIMIZE) != 0
                    && (options & PCRE2_DFA_RESTART) == 0
                {
                    /* If firstline is TRUE, the start of the match is constrained to the first
                    line of a multiline string. That is, the match must be before or at the
                    first newline following the start of matching. Temporarily adjust
                    end_subject so that we stop the optimization scans for a first code unit
                    immediately after the first character of a newline (the first code unit can
                    legitimately be a newline). If the match fails at the newline, later code
                    breaks this loop. */

                    if firstline != 0 {
                        let mut t: PCRE2_SPTR = start_match;
                        if utf != 0 {
                            while t < end_subject && IS_NEWLINE!(t, mb, utf) == 0 {
                                t = t.add(1);
                                ACROSSCHAR!(t < end_subject, t, t = t.add(1));
                            }
                        } else {
                            while t < end_subject && IS_NEWLINE!(t, mb, utf) == 0 {
                                t = t.add(1);
                            }
                        }
                        end_subject = t;
                    }

                    /* Anchored: check the first code unit if one is recorded. This may seem
                    pointless but it can help in detecting a no match case without scanning for
                    the required code unit. */

                    if anchored != 0 {
                        if has_first_cu != 0 || !start_bits.is_null() {
                            let mut ok: BOOL = (start_match < end_subject) as BOOL;
                            if ok != 0 {
                                let c: PCRE2_UCHAR = *start_match;
                                ok = (has_first_cu != 0 && (c == first_cu || c == first_cu2))
                                    as BOOL;
                                if ok == 0 && !start_bits.is_null() {
                                    ok = ((*start_bits.add((c / 8) as usize)
                                        & (1u32 << (c & 7)) as u8)
                                        != 0) as BOOL;
                                }
                            }
                            if ok == 0 {
                                break;
                            }
                        }
                    }
                    /* Not anchored. Advance to a unique first code unit if there is one. */
                    else {
                        if has_first_cu != 0 {
                            if first_cu != first_cu2
                            /* Caseless */
                            {
                                /* In 8-bit mode, the use of memchr() gives a big speed up, even
                                though we have to call it twice in order to find the earliest
                                occurrence of the code unit in either of its cases. Caching is used
                                to remember the positions of previously found code units. This can
                                make a huge difference when the strings are very long and only one
                                case is actually present. */

                                let mut pp1: PCRE2_SPTR = core::ptr::null();
                                let mut pp2: PCRE2_SPTR = core::ptr::null();
                                let searchlength: PCRE2_SIZE =
                                    end_subject as usize - start_match as usize;

                                /* If we haven't got a previously found position for first_cu, or if
                                the current starting position is later, we need to do a search. If
                                the code unit is not found, set it to the end. */

                                if memchr_found_first_cu.is_null()
                                    || start_match > memchr_found_first_cu
                                {
                                    pp1 = dfa_memchr(start_match, first_cu, searchlength);
                                    memchr_found_first_cu =
                                        if pp1.is_null() { end_subject } else { pp1 };
                                }
                                /* If the start is before a previously found position, use the
                                previous position, or NULL if a previous search failed. */
                                else {
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
                                    pp2 = dfa_memchr(start_match, first_cu2, searchlength);
                                    memchr_found_first_cu2 =
                                        if pp2.is_null() { end_subject } else { pp2 };
                                } else {
                                    pp2 = if memchr_found_first_cu2 == end_subject {
                                        core::ptr::null()
                                    } else {
                                        memchr_found_first_cu2
                                    };
                                }

                                /* Set the start to the end of the subject if neither case was found.
                                Otherwise, use the earlier found point. */

                                if pp1.is_null() {
                                    start_match = if pp2.is_null() { end_subject } else { pp2 };
                                } else {
                                    start_match = if pp2.is_null() || pp1 < pp2 { pp1 } else { pp2 };
                                }
                            }
                            /* The caseful case is much simpler. */
                            else {
                                start_match = dfa_memchr(
                                    start_match,
                                    first_cu,
                                    end_subject as usize - start_match as usize,
                                );
                                if start_match.is_null() {
                                    start_match = end_subject;
                                }
                            }

                            /* If we can't find the required code unit, having reached the true end
                            of the subject, break the bumpalong loop, to force a match failure,
                            except when doing partial matching, when we let the next cycle run at
                            the end of the subject. To see why, consider the pattern /(?<=abc)def/,
                            which partially matches "abc", even though the string does not contain
                            the starting character "d". If we have not reached the true end of the
                            subject (PCRE2_FIRSTLINE caused end_subject to be temporarily modified)
                            we also let the cycle run, because the matching string is legitimately
                            allowed to start with the first code unit of a newline. */

                            if ((*mb).moptions & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT)) == 0
                                && start_match >= (*mb).end_subject
                            {
                                break;
                            }
                        }
                        /* If there's no first code unit, advance to just after a linebreak for a
                        multiline match if required. */
                        else if startline != 0 {
                            if start_match > (*mb).start_subject.add(start_offset) {
                                if utf != 0 {
                                    while start_match < end_subject
                                        && WAS_NEWLINE!(start_match, mb, utf) == 0
                                    {
                                        start_match = start_match.add(1);
                                        ACROSSCHAR!(
                                            start_match < end_subject,
                                            start_match,
                                            start_match = start_match.add(1)
                                        );
                                    }
                                } else {
                                    while start_match < end_subject
                                        && WAS_NEWLINE!(start_match, mb, utf) == 0
                                    {
                                        start_match = start_match.add(1);
                                    }
                                }

                                /* If we have just passed a CR and the newline option is ANY or
                                ANYCRLF, and we are now at a LF, advance the match position by one
                                more code unit. */

                                if *start_match.offset(-1) == 0x0d /* CHAR_CR */
                                    && ((*mb).nltype == NLTYPE_ANY
                                        || (*mb).nltype == NLTYPE_ANYCRLF)
                                    && start_match < end_subject
                                    && *start_match == 0x0a
                                /* CHAR_NL */
                                {
                                    start_match = start_match.add(1);
                                }
                            }
                        }
                        /* If there's no first code unit or a requirement for a multiline line
                        start, advance to a non-unique first code unit if any have been
                        identified. The bitmap contains only 256 bits. When code units are 16 or
                        32 bits wide, all code units greater than 254 set the 255 bit. */
                        else if !start_bits.is_null() {
                            while start_match < end_subject {
                                let c: u32 = *start_match as u32;
                                if (*start_bits.add((c / 8) as usize) & (1u32 << (c & 7)) as u8)
                                    != 0
                                {
                                    break;
                                }
                                start_match = start_match.add(1);
                            }

                            /* See comment above in first_cu checking about the next line. */

                            if ((*mb).moptions & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT)) == 0
                                && start_match >= (*mb).end_subject
                            {
                                break;
                            }
                        }
                    } /* End of first code unit handling */

                    /* Restore fudged end_subject */

                    end_subject = (*mb).end_subject;

                    /* The following two optimizations are disabled for partial matching. */

                    if ((*mb).moptions & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT)) == 0 {
                        let mut p: PCRE2_SPTR;

                        /* The minimum matching length is a lower bound; no actual string of that
                        length may actually match the pattern. Although the value is, strictly,
                        in characters, we treat it as code units to avoid spending too much time
                        in this optimization. */

                        if end_subject.offset_from(start_match) < (*re).minlength as isize {
                            break 'NOMATCH_EXIT; /* goto NOMATCH_EXIT */
                        }

                        /* If req_cu is set, we know that that code unit must appear in the
                        subject for the match to succeed. If the first code unit is set, req_cu
                        must be later in the subject; otherwise the test starts at the match
                        point. This optimization can save a huge amount of backtracking in
                        patterns with nested unlimited repeats that aren't going to match.
                        Writing separate code for cased/caseless versions makes it go faster, as
                        does using an autoincrement and backing off on a match. As in the case of
                        the first code unit, using memchr() in the 8-bit library gives a big
                        speed up. Unlike the first_cu check above, we do not need to call
                        memchr() twice in the caseless case because we only need to check for the
                        presence of the character in either case, not find the first occurrence.

                        The search can be skipped if the code unit was found later than the
                        current starting point in a previous iteration of the bumpalong loop.

                        HOWEVER: when the subject string is very, very long, searching to its end
                        can take a long time, and give bad performance on quite ordinary
                        patterns. This showed up when somebody was matching something like
                        /^\d+C/ on a 32-megabyte string... so we don't do this when the string is
                        sufficiently long, but it's worth searching a lot more for unanchored
                        patterns. */

                        p = start_match.add(if has_first_cu != 0 { 1 } else { 0 });
                        if has_req_cu != 0 && p > req_cu_ptr {
                            let check_length: PCRE2_SIZE =
                                end_subject as usize - start_match as usize;

                            if check_length < REQ_CU_MAX
                                || (anchored == 0 && check_length < REQ_CU_MAX * 1000)
                            {
                                if req_cu != req_cu2
                                /* Caseless */
                                {
                                    let pp: PCRE2_SPTR = p;
                                    p = dfa_memchr(
                                        pp,
                                        req_cu,
                                        end_subject as usize - pp as usize,
                                    );
                                    if p.is_null() {
                                        p = dfa_memchr(
                                            pp,
                                            req_cu2,
                                            end_subject as usize - pp as usize,
                                        );
                                        if p.is_null() {
                                            p = end_subject;
                                        }
                                    }
                                }
                                /* The caseful case */
                                else {
                                    p = dfa_memchr(
                                        p,
                                        req_cu,
                                        end_subject as usize - p as usize,
                                    );
                                    if p.is_null() {
                                        p = end_subject;
                                    }
                                }

                                /* If we can't find the required code unit, break the matching loop,
                                forcing a match failure. */

                                if p >= end_subject {
                                    break;
                                }

                                /* If we have found the required code unit, save the point where we
                                found it, so that we don't search again next time round the loop if
                                the start hasn't passed this code unit yet. */

                                req_cu_ptr = p;
                            }
                        }
                    }
                }

                /* ------------ End of start of match optimizations ------------ */

                /* Give no match if we have passed the bumpalong limit. */

                if start_match > bumpalong_limit {
                    break;
                }

                /* OK, now we can do the business */

                (*mb).start_used_ptr = start_match;
                (*mb).last_used_ptr = start_match;
                (*mb).recursive = core::ptr::null_mut();

                rc = crate::dfa_internal::internal_dfa_match(
                    mb,                                    /* fixed match data */
                    (*mb).start_code,                      /* this subexpression's code */
                    start_match,                           /* where we currently are */
                    start_offset,                          /* start offset in subject */
                    (*match_data).ovector.as_mut_ptr(),    /* offset vector */
                    (*match_data).oveccount as u32 * 2,    /* actual size of same */
                    workspace,                             /* workspace vector */
                    wscount as i32,                        /* size of same */
                    0,                                     /* function recurse level */
                    base_recursion_workspace.0.as_mut_ptr() as *mut i32,
                ); /* initial workspace for recursion */

                /* Anything other than "no match" means we are done, always; otherwise, carry
                on only if not anchored. */

                if rc != PCRE2_ERROR_NOMATCH || anchored != 0 {
                    if rc == PCRE2_ERROR_NOMATCH {
                        break 'NOMATCH_EXIT; /* goto NOMATCH_EXIT */
                    }

                    if rc == PCRE2_ERROR_PARTIAL && (*match_data).oveccount > 0 {
                        *(*match_data).ovector.as_mut_ptr().add(0) =
                            (start_match as usize - subject as usize) as PCRE2_SIZE;
                        *(*match_data).ovector.as_mut_ptr().add(1) =
                            (end_subject as usize - subject as usize) as PCRE2_SIZE;
                    }

                    if rc >= 0 || rc == PCRE2_ERROR_PARTIAL {
                        (*match_data).subject_length = length;
                        (*match_data).start_offset = start_offset;
                        (*match_data).leftchar =
                            ((*mb).start_used_ptr as usize - subject as usize) as PCRE2_SIZE;
                        (*match_data).rightchar =
                            ((*mb).last_used_ptr as usize - subject as usize) as PCRE2_SIZE;
                        (*match_data).startchar =
                            (start_match as usize - subject as usize) as PCRE2_SIZE;
                    }

                    if rc >= 0 && (options & PCRE2_COPY_MATCHED_SUBJECT) != 0 {
                        if length != 0 {
                            (*match_data).subject = ((*match_data).memctl.malloc.unwrap())(
                                CU2BYTES!(length),
                                (*match_data).memctl.memory_data,
                            ) as PCRE2_SPTR;
                            if (*match_data).subject.is_null() {
                                rc = PCRE2_ERROR_NOMEMORY;
                                break 'EXIT; /* goto EXIT */
                            }
                            core::ptr::copy_nonoverlapping(
                                subject,
                                (*match_data).subject as *mut u8,
                                CU2BYTES!(length),
                            );
                        } else {
                            (*match_data).subject = core::ptr::null();
                        }
                        (*match_data).flags |= PCRE2_MD_COPIED_SUBJECT;
                    } else if rc >= 0 || rc == PCRE2_ERROR_PARTIAL {
                        (*match_data).subject = original_subject;
                    }
                    break 'EXIT; /* goto EXIT */
                }

                /* Advance to the next subject character unless we are at the end of a line
                and firstline is set. */

                if firstline != 0 && IS_NEWLINE!(start_match, mb, utf) != 0 {
                    break;
                }
                start_match = start_match.add(1);
                if utf != 0 {
                    ACROSSCHAR!(
                        start_match < end_subject,
                        start_match,
                        start_match = start_match.add(1)
                    );
                }
                if start_match > end_subject {
                    break;
                }

                /* If we have just passed a CR and we are now at a LF, and the pattern does
                not contain any explicit matches for \r or \n, and the newline option is CRLF
                or ANY or ANYCRLF, advance the match position by one more character. */

                if *start_match.offset(-1) == 0x0d /* CHAR_CR */
                    && start_match < end_subject
                    && *start_match == 0x0a /* CHAR_NL */
                    && ((*re).flags & PCRE2_HASCRORLF) == 0
                    && ((*mb).nltype == NLTYPE_ANY
                        || (*mb).nltype == NLTYPE_ANYCRLF
                        || (*mb).nllen == 2)
                {
                    start_match = start_match.add(1);
                }
            } /* "Bumpalong" loop */

            /* fall through to NOMATCH_EXIT */
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
