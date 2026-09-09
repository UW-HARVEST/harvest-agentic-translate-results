//! Translated from pcre2_dfa_match.c, lines 530-3311 (internal_dfa_match).
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::macros::*;
use crate::types::*;
use core::ffi::{c_char, c_void};

/* Shared items that live in pcre2_dfa_match.c and are translated in
src/dfa_match.rs: coptable, poptable, toptable1, toptable2, stateblock,
RWS_anchor, INTS_PER_STATEBLOCK, OVEC_UNIT, RWS_RSIZE, RWS_OVEC_RSIZE,
RWS_OVEC_OSIZE, RWS_ANCHOR_SIZE, OP_PROP_EXTRA, OP_EXTUNI_EXTRA,
OP_ANYNL_EXTRA, OP_HSPACE_EXTRA, OP_VSPACE_EXTRA, do_callout_dfa,
more_workspace. */
use crate::dfa_match::*;

/* The C code writes case labels such as "OP_PROP_EXTRA + OP_TYPEPLUS".
Rust match patterns cannot contain arithmetic, so the sums are given names. */

const PROP_TYPEPLUS: u32 = OP_PROP_EXTRA + OP_TYPEPLUS;
const PROP_TYPEMINPLUS: u32 = OP_PROP_EXTRA + OP_TYPEMINPLUS;
const PROP_TYPEPOSPLUS: u32 = OP_PROP_EXTRA + OP_TYPEPOSPLUS;
const PROP_TYPEQUERY: u32 = OP_PROP_EXTRA + OP_TYPEQUERY;
const PROP_TYPEMINQUERY: u32 = OP_PROP_EXTRA + OP_TYPEMINQUERY;
const PROP_TYPEPOSQUERY: u32 = OP_PROP_EXTRA + OP_TYPEPOSQUERY;
const PROP_TYPESTAR: u32 = OP_PROP_EXTRA + OP_TYPESTAR;
const PROP_TYPEMINSTAR: u32 = OP_PROP_EXTRA + OP_TYPEMINSTAR;
const PROP_TYPEPOSSTAR: u32 = OP_PROP_EXTRA + OP_TYPEPOSSTAR;
const PROP_TYPEEXACT: u32 = OP_PROP_EXTRA + OP_TYPEEXACT;
const PROP_TYPEUPTO: u32 = OP_PROP_EXTRA + OP_TYPEUPTO;
const PROP_TYPEMINUPTO: u32 = OP_PROP_EXTRA + OP_TYPEMINUPTO;
const PROP_TYPEPOSUPTO: u32 = OP_PROP_EXTRA + OP_TYPEPOSUPTO;

const EXTUNI_TYPEPLUS: u32 = OP_EXTUNI_EXTRA + OP_TYPEPLUS;
const EXTUNI_TYPEMINPLUS: u32 = OP_EXTUNI_EXTRA + OP_TYPEMINPLUS;
const EXTUNI_TYPEPOSPLUS: u32 = OP_EXTUNI_EXTRA + OP_TYPEPOSPLUS;
const EXTUNI_TYPEQUERY: u32 = OP_EXTUNI_EXTRA + OP_TYPEQUERY;
const EXTUNI_TYPEMINQUERY: u32 = OP_EXTUNI_EXTRA + OP_TYPEMINQUERY;
const EXTUNI_TYPEPOSQUERY: u32 = OP_EXTUNI_EXTRA + OP_TYPEPOSQUERY;
const EXTUNI_TYPESTAR: u32 = OP_EXTUNI_EXTRA + OP_TYPESTAR;
const EXTUNI_TYPEMINSTAR: u32 = OP_EXTUNI_EXTRA + OP_TYPEMINSTAR;
const EXTUNI_TYPEPOSSTAR: u32 = OP_EXTUNI_EXTRA + OP_TYPEPOSSTAR;
const EXTUNI_TYPEEXACT: u32 = OP_EXTUNI_EXTRA + OP_TYPEEXACT;
const EXTUNI_TYPEUPTO: u32 = OP_EXTUNI_EXTRA + OP_TYPEUPTO;
const EXTUNI_TYPEMINUPTO: u32 = OP_EXTUNI_EXTRA + OP_TYPEMINUPTO;
const EXTUNI_TYPEPOSUPTO: u32 = OP_EXTUNI_EXTRA + OP_TYPEPOSUPTO;

const ANYNL_TYPEPLUS: u32 = OP_ANYNL_EXTRA + OP_TYPEPLUS;
const ANYNL_TYPEMINPLUS: u32 = OP_ANYNL_EXTRA + OP_TYPEMINPLUS;
const ANYNL_TYPEPOSPLUS: u32 = OP_ANYNL_EXTRA + OP_TYPEPOSPLUS;
const ANYNL_TYPEQUERY: u32 = OP_ANYNL_EXTRA + OP_TYPEQUERY;
const ANYNL_TYPEMINQUERY: u32 = OP_ANYNL_EXTRA + OP_TYPEMINQUERY;
const ANYNL_TYPEPOSQUERY: u32 = OP_ANYNL_EXTRA + OP_TYPEPOSQUERY;
const ANYNL_TYPESTAR: u32 = OP_ANYNL_EXTRA + OP_TYPESTAR;
const ANYNL_TYPEMINSTAR: u32 = OP_ANYNL_EXTRA + OP_TYPEMINSTAR;
const ANYNL_TYPEPOSSTAR: u32 = OP_ANYNL_EXTRA + OP_TYPEPOSSTAR;
const ANYNL_TYPEEXACT: u32 = OP_ANYNL_EXTRA + OP_TYPEEXACT;
const ANYNL_TYPEUPTO: u32 = OP_ANYNL_EXTRA + OP_TYPEUPTO;
const ANYNL_TYPEMINUPTO: u32 = OP_ANYNL_EXTRA + OP_TYPEMINUPTO;
const ANYNL_TYPEPOSUPTO: u32 = OP_ANYNL_EXTRA + OP_TYPEPOSUPTO;

const VSPACE_TYPEPLUS: u32 = OP_VSPACE_EXTRA + OP_TYPEPLUS;
const VSPACE_TYPEMINPLUS: u32 = OP_VSPACE_EXTRA + OP_TYPEMINPLUS;
const VSPACE_TYPEPOSPLUS: u32 = OP_VSPACE_EXTRA + OP_TYPEPOSPLUS;
const VSPACE_TYPEQUERY: u32 = OP_VSPACE_EXTRA + OP_TYPEQUERY;
const VSPACE_TYPEMINQUERY: u32 = OP_VSPACE_EXTRA + OP_TYPEMINQUERY;
const VSPACE_TYPEPOSQUERY: u32 = OP_VSPACE_EXTRA + OP_TYPEPOSQUERY;
const VSPACE_TYPESTAR: u32 = OP_VSPACE_EXTRA + OP_TYPESTAR;
const VSPACE_TYPEMINSTAR: u32 = OP_VSPACE_EXTRA + OP_TYPEMINSTAR;
const VSPACE_TYPEPOSSTAR: u32 = OP_VSPACE_EXTRA + OP_TYPEPOSSTAR;
const VSPACE_TYPEEXACT: u32 = OP_VSPACE_EXTRA + OP_TYPEEXACT;
const VSPACE_TYPEUPTO: u32 = OP_VSPACE_EXTRA + OP_TYPEUPTO;
const VSPACE_TYPEMINUPTO: u32 = OP_VSPACE_EXTRA + OP_TYPEMINUPTO;
const VSPACE_TYPEPOSUPTO: u32 = OP_VSPACE_EXTRA + OP_TYPEPOSUPTO;

const HSPACE_TYPEPLUS: u32 = OP_HSPACE_EXTRA + OP_TYPEPLUS;
const HSPACE_TYPEMINPLUS: u32 = OP_HSPACE_EXTRA + OP_TYPEMINPLUS;
const HSPACE_TYPEPOSPLUS: u32 = OP_HSPACE_EXTRA + OP_TYPEPOSPLUS;
const HSPACE_TYPEQUERY: u32 = OP_HSPACE_EXTRA + OP_TYPEQUERY;
const HSPACE_TYPEMINQUERY: u32 = OP_HSPACE_EXTRA + OP_TYPEMINQUERY;
const HSPACE_TYPEPOSQUERY: u32 = OP_HSPACE_EXTRA + OP_TYPEPOSQUERY;
const HSPACE_TYPESTAR: u32 = OP_HSPACE_EXTRA + OP_TYPESTAR;
const HSPACE_TYPEMINSTAR: u32 = OP_HSPACE_EXTRA + OP_TYPEMINSTAR;
const HSPACE_TYPEPOSSTAR: u32 = OP_HSPACE_EXTRA + OP_TYPEPOSSTAR;
const HSPACE_TYPEEXACT: u32 = OP_HSPACE_EXTRA + OP_TYPEEXACT;
const HSPACE_TYPEUPTO: u32 = OP_HSPACE_EXTRA + OP_TYPEUPTO;
const HSPACE_TYPEMINUPTO: u32 = OP_HSPACE_EXTRA + OP_TYPEMINUPTO;
const HSPACE_TYPEPOSUPTO: u32 = OP_HSPACE_EXTRA + OP_TYPEPOSUPTO;

/* CHAR_xxx values used below (ASCII/non-EBCDIC). */
const CHAR_HT: u32 = 0x09;
const CHAR_LF: u32 = 0x0a;
const CHAR_VT: u32 = 0x0b;
const CHAR_FF: u32 = 0x0c;
const CHAR_CR: u32 = 0x0d;
const CHAR_SPACE: u32 = 0x20;
const CHAR_DOLLAR_SIGN: u32 = 0x24;
const CHAR_COMMERCIAL_AT: u32 = 0x40;
const CHAR_GRAVE_ACCENT: u32 = 0x60;
const CHAR_NEL: u32 = 0x85;
const CHAR_NBSP: u32 = 0xa0;

/* HSPACE_CASES and VSPACE_CASES from pcre2_internal.h, as match patterns. */
macro_rules! HSPACE_CASES {
    () => {
        0x09 | 0x20
            | 0xa0
            | 0x1680
            | 0x180e
            | 0x2000..=0x200a
            | 0x202f
            | 0x205f
            | 0x3000
    };
}

macro_rules! VSPACE_CASES {
    () => {
        0x0a | 0x0b | 0x0c | 0x0d | 0x85 | 0x2028 | 0x2029
    };
}

/* The "switch(code[n])" block that tests a Unicode property occurs four times
in internal_dfa_match(), differing only in where the property type and the
property data are found. It is factored out here as a macro that yields the
value of the C variable "OK". */

macro_rules! PROP_TEST {
    ($c:expr, $codevalue:expr, $ptype:expr, $pdata:expr) => {{
        let OK: BOOL;
        let mut chartype: i32;
        let mut cp: *const u32;
        let prop: *const ucd_record = GET_UCD!($c);
        match $ptype as u32 {
            PT_LAMP => {
                chartype = (*prop).chartype as i32;
                OK = (chartype == ucp_Lu as i32
                    || chartype == ucp_Ll as i32
                    || chartype == ucp_Lt as i32) as BOOL;
            }

            PT_GC => {
                OK = (*crate::tables::_pcre2_ucp_gentype_8
                    .as_ptr()
                    .add((*prop).chartype as usize)
                    == $pdata as u32) as BOOL;
            }

            PT_PC => {
                OK = ((*prop).chartype as u32 == $pdata as u32) as BOOL;
            }

            PT_SC => {
                OK = ((*prop).script as u32 == $pdata as u32) as BOOL;
            }

            PT_SCX => {
                OK = ((*prop).script as u32 == $pdata as u32
                    || MAPBIT!(
                        crate::ucd::_pcre2_ucd_script_sets_8
                            .as_ptr()
                            .add(UCD_SCRIPTX_PROP!(prop) as usize),
                        $pdata
                    ) != 0) as BOOL;
            }

            /* These are specials for combination cases. */
            PT_ALNUM => {
                chartype = (*prop).chartype as i32;
                OK = (*crate::tables::_pcre2_ucp_gentype_8
                    .as_ptr()
                    .add(chartype as usize)
                    == ucp_L
                    || *crate::tables::_pcre2_ucp_gentype_8
                        .as_ptr()
                        .add(chartype as usize)
                        == ucp_N) as BOOL;
            }

            /* Perl space used to exclude VT, but from Perl 5.18 it is included,
            which means that Perl space and POSIX space are now identical. PCRE
            was changed at release 8.34. */
            PT_SPACE | PT_PXSPACE => {
                match $c {
                    HSPACE_CASES!() | VSPACE_CASES!() => {
                        OK = TRUE;
                    }

                    _ => {
                        OK = (*crate::tables::_pcre2_ucp_gentype_8
                            .as_ptr()
                            .add((*prop).chartype as usize)
                            == ucp_Z) as BOOL;
                    }
                }
            }

            PT_WORD => {
                chartype = (*prop).chartype as i32;
                OK = (*crate::tables::_pcre2_ucp_gentype_8
                    .as_ptr()
                    .add(chartype as usize)
                    == ucp_L
                    || *crate::tables::_pcre2_ucp_gentype_8
                        .as_ptr()
                        .add(chartype as usize)
                        == ucp_N
                    || chartype == ucp_Mn as i32
                    || chartype == ucp_Pc as i32) as BOOL;
            }

            PT_CLIST => {
                cp = crate::ucd::_pcre2_ucd_caseless_sets_8
                    .as_ptr()
                    .add($pdata as usize);
                let mut ok_tmp: BOOL;
                loop {
                    if $c < *cp {
                        ok_tmp = FALSE;
                        break;
                    }
                    let t = *cp;
                    cp = cp.add(1);
                    if $c == t {
                        ok_tmp = TRUE;
                        break;
                    }
                }
                OK = ok_tmp;
            }

            PT_UCNC => {
                OK = ($c == CHAR_DOLLAR_SIGN
                    || $c == CHAR_COMMERCIAL_AT
                    || $c == CHAR_GRAVE_ACCENT
                    || ($c >= 0xa0 && $c <= 0xd7ff)
                    || $c >= 0xe000) as BOOL;
            }

            PT_BIDICL => {
                OK = (UCD_BIDICLASS!($c) == $pdata as u32) as BOOL;
            }

            PT_BOOL => {
                OK = (MAPBIT!(
                    crate::ucd::_pcre2_ucd_boolprop_sets_8
                        .as_ptr()
                        .add(UCD_BPROPS_PROP!(prop) as usize),
                    $pdata
                ) != 0) as BOOL;
            }

            /* Should never occur, but keep compilers from grumbling. */
            _ => {
                OK = ($codevalue != OP_PROP) as BOOL;
            }
        }
        OK
    }};
}

/*************************************************
*     Match a Regular Expression - DFA engine    *
*************************************************/

/* This internal function applies a compiled pattern to a subject string,
starting at a given point, using a DFA engine. This function is called from the
external one, possibly multiple times if the pattern is not anchored. The
function calls itself recursively for some kinds of subpattern.

Arguments:
  mb                the match_data block with fixed information
  this_start_code   the opening bracket of this subexpression's code
  current_subject   where we currently are in the subject string
  start_offset      start offset in the subject string
  offsets           vector to contain the matching string offsets
  offsetcount       size of same
  workspace         vector of workspace
  wscount           size of same
  rlevel            function call recursion level

Returns:            > 0 => number of match offset pairs placed in offsets
                    = 0 => offsets overflowed; longest matches are present
                     -1 => failed to match
                   < -1 => some kind of unexpected problem
*/

pub(crate) unsafe fn internal_dfa_match(
    mb: *mut dfa_match_block,
    this_start_code: PCRE2_SPTR,
    current_subject: PCRE2_SPTR,
    start_offset: PCRE2_SIZE,
    offsets: *mut PCRE2_SIZE,
    offsetcount: u32,
    workspace: *mut i32,
    wscount: i32,
    rlevel: u32,
    RWS: *mut i32,
) -> i32 {
    let mut current_subject = current_subject;
    let mut offsetcount = offsetcount;
    let mut wscount = wscount;
    let mut rlevel = rlevel;
    let mut RWS = RWS;

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
    let mut active_count: i32 = 0;
    let mut new_count: i32;
    let mut match_count: i32;

    /* Some fields in the mb block are frequently referenced, so we load them into
    independent variables in the hope that this will perform better. */

    let start_subject: PCRE2_SPTR = (*mb).start_subject;
    let end_subject: PCRE2_SPTR = (*mb).end_subject;
    let start_code: PCRE2_SPTR = (*mb).start_code;

    let utf: BOOL = (((*mb).poptions & PCRE2_UTF) != 0) as BOOL;
    let utf_or_ucp: BOOL = (utf != 0 || ((*mb).poptions & PCRE2_UCP) != 0) as BOOL;

    let mut reset_could_continue: BOOL = FALSE;

    /* IS_NEWLINE / WAS_NEWLINE with NLBLOCK == mb, PSSTART == start_subject and
    PSEND == end_subject. */

    macro_rules! IS_NEWLINE {
        ($p:expr) => {
            crate::macros::is_newline_block(
                $p,
                (*mb).nltype,
                core::ptr::addr_of_mut!((*mb).nllen),
                core::ptr::addr_of!((*mb).nl) as *const u8,
                (*mb).end_subject,
                utf,
            ) != 0
        };
    }

    macro_rules! WAS_NEWLINE {
        ($p:expr) => {
            crate::macros::was_newline_block(
                $p,
                (*mb).nltype,
                core::ptr::addr_of_mut!((*mb).nllen),
                core::ptr::addr_of!((*mb).nl) as *const u8,
                (*mb).start_subject,
                utf,
            ) != 0
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
        rlevel = t.wrapping_add(1);
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

    /* The following macros are used for adding states to the two state vectors
    (one for the current character, one for the following character). They are
    local to internal_dfa_match() in C, and they refer to the local variables
    declared above, so they are defined here. */

    macro_rules! ADD_ACTIVE {
        ($x:expr, $y:expr) => {
            if {
                let t = active_count;
                active_count = t + 1;
                t
            } < wscount
            {
                (*next_active_state).offset = ($x);
                (*next_active_state).count = ($y);
                next_active_state = next_active_state.add(1);
            } else {
                return PCRE2_ERROR_DFA_WSSIZE;
            }
        };
    }

    macro_rules! ADD_ACTIVE_DATA {
        ($x:expr, $y:expr, $z:expr) => {
            if {
                let t = active_count;
                active_count = t + 1;
                t
            } < wscount
            {
                (*next_active_state).offset = ($x);
                (*next_active_state).count = ($y);
                (*next_active_state).data = ($z);
                next_active_state = next_active_state.add(1);
            } else {
                return PCRE2_ERROR_DFA_WSSIZE;
            }
        };
    }

    macro_rules! ADD_NEW {
        ($x:expr, $y:expr) => {
            if {
                let t = new_count;
                new_count = t + 1;
                t
            } < wscount
            {
                (*next_new_state).offset = ($x);
                (*next_new_state).count = ($y);
                next_new_state = next_new_state.add(1);
            } else {
                return PCRE2_ERROR_DFA_WSSIZE;
            }
        };
    }

    macro_rules! ADD_NEW_DATA {
        ($x:expr, $y:expr, $z:expr) => {
            if {
                let t = new_count;
                new_count = t + 1;
                t
            } < wscount
            {
                (*next_new_state).offset = ($x);
                (*next_new_state).count = ($y);
                (*next_new_state).data = ($z);
                next_new_state = next_new_state.add(1);
            } else {
                return PCRE2_ERROR_DFA_WSSIZE;
            }
        };
    }

    /* The first thing in any (sub) pattern is a bracket of some sort. Push all
    the alternative states onto the list, and find out where the end is. This
    makes is possible to use this function recursively, when we want to stop at a
    matching internal ket rather than at the end.

    If we are dealing with a backward assertion we have to find out the maximum
    amount to move back, and set up each alternative appropriately. */

    if *this_start_code as u32 == OP_ASSERTBACK || *this_start_code as u32 == OP_ASSERTBACK_NOT {
        let mut max_back: usize = 0;
        let gone_back: usize;

        end_code = this_start_code;
        loop {
            let back: usize = GET2!(end_code, 2 + LINK_SIZE) as usize;
            if back > max_back {
                max_back = back;
            }
            end_code = end_code.add(GET!(end_code, 1) as usize);
            if *end_code as u32 != OP_ALT {
                break;
            }
        }

        /* If we can't go back the amount required for the longest lookbehind
        pattern, go back as far as we can; some alternatives may still be viable. */

        /* In character mode we have to step back character by character */

        if utf != 0 {
            let mut gb: usize = 0;
            while gb < max_back {
                if current_subject <= start_subject {
                    break;
                }
                current_subject = current_subject.wrapping_sub(1);
                ACROSSCHAR!(
                    current_subject > start_subject,
                    current_subject,
                    current_subject = current_subject.wrapping_sub(1)
                );
                gb += 1;
            }
            gone_back = gb;
        }
        /* In byte-mode we can do this quickly. */
        else {
            let current_offset: usize = current_subject as usize - start_subject as usize;
            gone_back = if current_offset < max_back {
                current_offset
            } else {
                max_back
            };
            current_subject = current_subject.wrapping_sub(gone_back);
        }

        /* Save the earliest consulted character */

        if current_subject < (*mb).start_used_ptr {
            (*mb).start_used_ptr = current_subject;
        }

        /* Now we can process the individual branches. There will be an OP_REVERSE at
        the start of each branch, except when the length of the branch is zero. */

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
                GET2!(end_code, 2 + LINK_SIZE) as usize
            };
            if back <= gone_back {
                let bstate: i32 = (end_code as usize - start_code as usize) as i32
                    + 1
                    + LINK_SIZE as i32
                    + revlen as i32;
                ADD_NEW_DATA!(-bstate, 0, (gone_back - back) as i32);
            }
            end_code = end_code.add(GET!(end_code, 1) as usize);
            if *end_code as u32 != OP_ALT {
                break;
            }
        }
    }
    /* This is the code for a "normal" subpattern (not a backward assertion). The
    start of a whole pattern is always one of these. If we are at the top level,
    we may be asked to restart matching from the same point that we reached for a
    previous partial match. We still have to scan through the top-level branches to
    find the end state. */
    else {
        end_code = this_start_code;

        /* Restarting */

        if rlevel == 1 && ((*mb).moptions & PCRE2_DFA_RESTART) != 0 {
            loop {
                end_code = end_code.add(GET!(end_code, 1) as usize);
                if *end_code as u32 != OP_ALT {
                    break;
                }
            }
            new_count = *workspace.add(1);
            if *workspace.add(0) == 0 {
                core::ptr::copy_nonoverlapping(
                    active_states as *const u8,
                    new_states as *mut u8,
                    (new_count as usize) * core::mem::size_of::<stateblock>(),
                );
            }
        }
        /* Not restarting */
        else {
            let mut length: i32 = 1
                + LINK_SIZE as i32
                + (if *this_start_code as u32 == OP_CBRA
                    || *this_start_code as u32 == OP_SCBRA
                    || *this_start_code as u32 == OP_CBRAPOS
                    || *this_start_code as u32 == OP_SCBRAPOS
                {
                    IMM2_SIZE as i32
                } else {
                    0
                });
            loop {
                ADD_NEW!(
                    (end_code as usize - start_code as usize) as i32 + length,
                    0
                );
                end_code = end_code.add(GET!(end_code, 1) as usize);
                length = 1 + LINK_SIZE as i32;
                if *end_code as u32 != OP_ALT {
                    break;
                }
            }
        }
    }

    *workspace.add(0) = 0; /* Bit indicating which vector is current */

    /* Loop for scanning the subject */

    ptr = current_subject;
    'subject_loop: loop {
        let mut i: i32;
        let mut j: i32;
        let mut clen: i32;
        let mut dlen: i32;
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

        /* Load the current character from the subject outside the loop, as many
        different states may want to look at it, and we assume that at least one
        will. */

        if ptr < end_subject {
            clen = 1; /* Number of data items in the character */
            GETCHARLENTEST!(c, ptr, clen, utf);
        } else {
            clen = 0; /* This indicates the end of the subject */
            c = NOTACHAR; /* This value should never actually be used */
        }

        /* Scan up the active states and act on each one. The result of an action
        may be to add more states to the currently active list (e.g. on hitting a
        parenthesis) or it may be to put states on the new list, for considering
        when we move the character pointer on. */

        i = 0;
        while i < active_count {
            /* The body of the C for loop; "goto NEXT_ACTIVE_STATE" and "continue"
            are translated as "break 'NEXT_ACTIVE_STATE". */
            'NEXT_ACTIVE_STATE: {
                let current_state: *mut stateblock = active_states.offset(i as isize);
                let mut caseless: BOOL = FALSE;
                let mut code: PCRE2_SPTR;
                let mut codevalue: u32;
                let mut state_offset: i32 = (*current_state).offset;
                let mut rrc: i32;
                let mut count: i32;

                /* A negative offset is a special case meaning "hold off going to this
                (negated) state until the number of characters in the data field have
                been skipped". If the could_continue flag was passed over from a previous
                state, arrange for it to passed on. */

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
                        break 'NEXT_ACTIVE_STATE; /* continue */
                    } else {
                        state_offset = -state_offset;
                        (*current_state).offset = state_offset;
                    }
                }

                /* Check for a duplicate state with the same count, and skip if found.
                See the note at the head of this module about the possibility of improving
                performance here. */

                j = 0;
                while j < i {
                    if (*active_states.offset(j as isize)).offset == state_offset
                        && (*active_states.offset(j as isize)).count == (*current_state).count
                    {
                        break 'NEXT_ACTIVE_STATE; /* goto NEXT_ACTIVE_STATE */
                    }
                    j += 1;
                }

                /* The state offset is the offset to the opcode */

                code = start_code.offset(state_offset as isize);
                codevalue = *code as u32;

                /* If this opcode inspects a character, but we are at the end of the
                subject, remember the fact for use when testing for a partial match. */

                if clen == 0 && *poptable.as_ptr().add(codevalue as usize) != 0 {
                    could_continue = TRUE;
                }

                /* If this opcode is followed by an inline character, load it. It is
                tempting to test for the presence of a subject character here, but that
                is wrong, because sometimes zero repetitions of the subject are
                permitted.

                We also use this mechanism for opcodes such as OP_TYPEPLUS that take an
                argument that is not a data character - but is always one byte long because
                the values are small. We have to take special action to deal with  \P, \p,
                \H, \h, \V, \v and \X in this case. To keep the other cases fast, convert
                these ones to new opcodes. */

                if *coptable.as_ptr().add(codevalue as usize) > 0 {
                    dlen = 1;
                    if utf != 0 {
                        GETCHARLEN!(
                            d,
                            (code.add(*coptable.as_ptr().add(codevalue as usize) as usize)),
                            dlen
                        );
                    } else {
                        d = *code.add(*coptable.as_ptr().add(codevalue as usize) as usize) as u32;
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

                match codevalue {
                    /* ========================================================== */
                    /* Reached a closing bracket. If not at the end of the pattern, carry
                    on with the next opcode. For repeating opcodes, also add the repeat
                    state. Note that KETRPOS will always be encountered at the end of the
                    subpattern, because the possessive subpattern repeats are always handled
                    using recursive calls. Thus, it never adds any new states.

                    At the end of the (sub)pattern, unless we have an empty string and
                    PCRE2_NOTEMPTY is set, or PCRE2_NOTEMPTY_ATSTART is set and we are at the
                    start of the subject, save the match data, shifting up all previous
                    matches so we always have the longest first. */
                    OP_KET | OP_KETRMIN | OP_KETRMAX | OP_KETRPOS => {
                        if code != end_code {
                            ADD_ACTIVE!(state_offset + 1 + LINK_SIZE as i32, 0);
                            if codevalue != OP_KET {
                                ADD_ACTIVE!(state_offset - GET!(code, 1) as i32, 0);
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
                                } else if match_count > 0 && {
                                    match_count += 1;
                                    match_count * 2 > offsetcount as i32
                                } {
                                    match_count = 0;
                                }
                                count = (if match_count == 0 {
                                    offsetcount as i32
                                } else {
                                    match_count * 2
                                }) - 2;
                                if count > 0 {
                                    core::ptr::copy(
                                        offsets as *const u8,
                                        offsets.add(2) as *mut u8,
                                        (count as usize) * core::mem::size_of::<PCRE2_SIZE>(),
                                    );
                                }
                                if offsetcount >= 2 {
                                    *offsets.add(0) = (current_subject as usize
                                        - start_subject as usize)
                                        as PCRE2_SIZE;
                                    *offsets.add(1) =
                                        (ptr as usize - start_subject as usize) as PCRE2_SIZE;
                                }
                                if ((*mb).moptions & PCRE2_DFA_SHORTEST) != 0 {
                                    return match_count;
                                }
                            }
                        }
                    }

                    /* ========================================================== */
                    /* These opcodes add to the current list of states without looking
                    at the current character. */

                    /*-----------------------------------------------------------------*/
                    OP_ALT => {
                        loop {
                            code = code.add(GET!(code, 1) as usize);
                            if *code as u32 != OP_ALT {
                                break;
                            }
                        }
                        ADD_ACTIVE!((code as usize - start_code as usize) as i32, 0);
                    }

                    /*-----------------------------------------------------------------*/
                    OP_BRA | OP_SBRA => {
                        loop {
                            ADD_ACTIVE!(
                                (code as usize - start_code as usize) as i32
                                    + 1
                                    + LINK_SIZE as i32,
                                0
                            );
                            code = code.add(GET!(code, 1) as usize);
                            if *code as u32 != OP_ALT {
                                break;
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_CBRA | OP_SCBRA => {
                        ADD_ACTIVE!(
                            (code as usize - start_code as usize) as i32
                                + 1
                                + LINK_SIZE as i32
                                + IMM2_SIZE as i32,
                            0
                        );
                        code = code.add(GET!(code, 1) as usize);
                        while *code as u32 == OP_ALT {
                            ADD_ACTIVE!(
                                (code as usize - start_code as usize) as i32
                                    + 1
                                    + LINK_SIZE as i32,
                                0
                            );
                            code = code.add(GET!(code, 1) as usize);
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_BRAZERO | OP_BRAMINZERO => {
                        ADD_ACTIVE!(state_offset + 1, 0);
                        code = code.add(1 + GET!(code, 2) as usize);
                        while *code as u32 == OP_ALT {
                            code = code.add(GET!(code, 1) as usize);
                        }
                        ADD_ACTIVE!(
                            (code as usize - start_code as usize) as i32 + 1 + LINK_SIZE as i32,
                            0
                        );
                    }

                    /*-----------------------------------------------------------------*/
                    OP_SKIPZERO => {
                        code = code.add(1 + GET!(code, 2) as usize);
                        while *code as u32 == OP_ALT {
                            code = code.add(GET!(code, 1) as usize);
                        }
                        ADD_ACTIVE!(
                            (code as usize - start_code as usize) as i32 + 1 + LINK_SIZE as i32,
                            0
                        );
                    }

                    /*-----------------------------------------------------------------*/
                    OP_CIRC => {
                        if ptr == start_subject && ((*mb).moptions & PCRE2_NOTBOL) == 0 {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_CIRCM => {
                        if (ptr == start_subject && ((*mb).moptions & PCRE2_NOTBOL) == 0)
                            || ((ptr != end_subject
                                || ((*mb).poptions & PCRE2_ALT_CIRCUMFLEX) != 0)
                                && WAS_NEWLINE!(ptr))
                        {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_EOD => {
                        if ptr >= end_subject {
                            if ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0 {
                                return PCRE2_ERROR_PARTIAL;
                            } else {
                                ADD_ACTIVE!(state_offset + 1, 0);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_SOD => {
                        if ptr == start_subject {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_SOM => {
                        if ptr == start_subject.add(start_offset) {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    /* ========================================================== */
                    /* These opcodes inspect the next subject character, and sometimes
                    the previous one as well, but do not have an argument. The variable
                    clen contains the length of the current character and is zero if we are
                    at the end of the subject. */

                    /*-----------------------------------------------------------------*/
                    OP_ANY => {
                        if clen > 0 && !IS_NEWLINE!(ptr) {
                            if ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & (PCRE2_PARTIAL_HARD)) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                partial_newline = TRUE;
                                could_continue = TRUE;
                            } else {
                                ADD_NEW!(state_offset + 1, 0);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_ALLANY => {
                        if clen > 0 {
                            ADD_NEW!(state_offset + 1, 0);
                        }
                    }

                    /*-----------------------------------------------------------------*/
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

                    /*-----------------------------------------------------------------*/
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
                                    could_continue = TRUE;
                                }
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
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
                                    could_continue = TRUE;
                                }
                            }
                        } else if IS_NEWLINE!(ptr) {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_DIGIT | OP_WHITESPACE | OP_WORDCHAR => {
                        if clen > 0
                            && c < 256
                            && ((*ctypes.add(c as usize)
                                & *toptable1.as_ptr().add(codevalue as usize))
                                ^ *toptable2.as_ptr().add(codevalue as usize))
                                != 0
                        {
                            ADD_NEW!(state_offset + 1, 0);
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_NOT_DIGIT | OP_NOT_WHITESPACE | OP_NOT_WORDCHAR => {
                        if clen > 0
                            && (c >= 256
                                || ((*ctypes.add(c as usize)
                                    & *toptable1.as_ptr().add(codevalue as usize))
                                    ^ *toptable2.as_ptr().add(codevalue as usize))
                                    != 0)
                        {
                            ADD_NEW!(state_offset + 1, 0);
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_WORD_BOUNDARY
                    | OP_NOT_WORD_BOUNDARY
                    | OP_NOT_UCP_WORD_BOUNDARY
                    | OP_UCP_WORD_BOUNDARY => {
                        let left_word: BOOL;
                        let right_word: BOOL;

                        if ptr > start_subject {
                            let mut temp: PCRE2_SPTR = ptr.wrapping_sub(1);
                            if temp < (*mb).start_used_ptr {
                                (*mb).start_used_ptr = temp;
                            }
                            if utf != 0 {
                                BACKCHAR!(temp);
                            }
                            GETCHARTEST!(d, temp, utf);
                            if codevalue == OP_UCP_WORD_BOUNDARY
                                || codevalue == OP_NOT_UCP_WORD_BOUNDARY
                            {
                                let chartype: i32 = UCD_CHARTYPE!(d) as i32;
                                let category: u32 = *crate::tables::_pcre2_ucp_gentype_8
                                    .as_ptr()
                                    .add(chartype as usize);
                                left_word = (category == ucp_L
                                    || category == ucp_N
                                    || chartype == ucp_Mn as i32
                                    || chartype == ucp_Pc as i32)
                                    as BOOL;
                            } else {
                                left_word =
                                    (d < 256 && (*ctypes.add(d as usize) & ctype_word) != 0)
                                        as BOOL;
                            }
                        } else {
                            left_word = FALSE;
                        }

                        if clen > 0 {
                            if ptr >= (*mb).last_used_ptr {
                                let mut temp: PCRE2_SPTR = ptr.add(1);
                                if utf != 0 {
                                    FORWARDCHARTEST!(temp, (*mb).end_subject);
                                }
                                (*mb).last_used_ptr = temp;
                            }
                            if codevalue == OP_UCP_WORD_BOUNDARY
                                || codevalue == OP_NOT_UCP_WORD_BOUNDARY
                            {
                                let chartype: i32 = UCD_CHARTYPE!(c) as i32;
                                let category: u32 = *crate::tables::_pcre2_ucp_gentype_8
                                    .as_ptr()
                                    .add(chartype as usize);
                                right_word = (category == ucp_L
                                    || category == ucp_N
                                    || chartype == ucp_Mn as i32
                                    || chartype == ucp_Pc as i32)
                                    as BOOL;
                            } else {
                                right_word =
                                    (c < 256 && (*ctypes.add(c as usize) & ctype_word) != 0)
                                        as BOOL;
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

                    /*-----------------------------------------------------------------*/
                    /* Check the next character by Unicode property. We will get here only
                    if the support is in the binary; otherwise a compile-time error occurs.
                    */
                    OP_PROP | OP_NOTPROP => {
                        if clen > 0 {
                            let OK: BOOL =
                                PROP_TEST!(c, codevalue, *code.add(1), *code.add(2));

                            if OK == (codevalue == OP_PROP) as BOOL {
                                ADD_NEW!(state_offset + 3, 0);
                            }
                        }
                    }

                    /* ========================================================== */
                    /* These opcodes likewise inspect the subject character, but have an
                    argument that is not a data character. It is one of these opcodes:
                    OP_ANY, OP_ALLANY, OP_DIGIT, OP_NOT_DIGIT, OP_WHITESPACE, OP_NOT_SPACE,
                    OP_WORDCHAR, OP_NOT_WORDCHAR. The value is loaded into d. */
                    OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEPOSPLUS => {
                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 2, 0);
                        }
                        if clen > 0 {
                            if d == OP_ANY
                                && ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & (PCRE2_PARTIAL_HARD)) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                partial_newline = TRUE;
                                could_continue = TRUE;
                            } else if (c >= 256
                                && d != OP_DIGIT
                                && d != OP_WHITESPACE
                                && d != OP_WORDCHAR)
                                || (c < 256
                                    && (d != OP_ANY || !IS_NEWLINE!(ptr))
                                    && ((*ctypes.add(c as usize)
                                        & *toptable1.as_ptr().add(d as usize))
                                        ^ *toptable2.as_ptr().add(d as usize))
                                        != 0)
                            {
                                if count > 0 && codevalue == OP_TYPEPOSPLUS {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                ADD_NEW!(state_offset, count);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_TYPEQUERY | OP_TYPEMINQUERY | OP_TYPEPOSQUERY => {
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            if d == OP_ANY
                                && ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & (PCRE2_PARTIAL_HARD)) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                partial_newline = TRUE;
                                could_continue = TRUE;
                            } else if (c >= 256
                                && d != OP_DIGIT
                                && d != OP_WHITESPACE
                                && d != OP_WORDCHAR)
                                || (c < 256
                                    && (d != OP_ANY || !IS_NEWLINE!(ptr))
                                    && ((*ctypes.add(c as usize)
                                        & *toptable1.as_ptr().add(d as usize))
                                        ^ *toptable2.as_ptr().add(d as usize))
                                        != 0)
                            {
                                if codevalue == OP_TYPEPOSQUERY {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW!(state_offset + 2, 0);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPOSSTAR => {
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            if d == OP_ANY
                                && ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & (PCRE2_PARTIAL_HARD)) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                partial_newline = TRUE;
                                could_continue = TRUE;
                            } else if (c >= 256
                                && d != OP_DIGIT
                                && d != OP_WHITESPACE
                                && d != OP_WORDCHAR)
                                || (c < 256
                                    && (d != OP_ANY || !IS_NEWLINE!(ptr))
                                    && ((*ctypes.add(c as usize)
                                        & *toptable1.as_ptr().add(d as usize))
                                        ^ *toptable2.as_ptr().add(d as usize))
                                        != 0)
                            {
                                if codevalue == OP_TYPEPOSSTAR {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW!(state_offset, 0);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_TYPEEXACT => {
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            if d == OP_ANY
                                && ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & (PCRE2_PARTIAL_HARD)) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                partial_newline = TRUE;
                                could_continue = TRUE;
                            } else if (c >= 256
                                && d != OP_DIGIT
                                && d != OP_WHITESPACE
                                && d != OP_WORDCHAR)
                                || (c < 256
                                    && (d != OP_ANY || !IS_NEWLINE!(ptr))
                                    && ((*ctypes.add(c as usize)
                                        & *toptable1.as_ptr().add(d as usize))
                                        ^ *toptable2.as_ptr().add(d as usize))
                                        != 0)
                            {
                                count += 1;
                                if count >= GET2!(code, 1) as i32 {
                                    ADD_NEW!(state_offset + 1 + IMM2_SIZE as i32 + 1, 0);
                                } else {
                                    ADD_NEW!(state_offset, count);
                                }
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEPOSUPTO => {
                        ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as i32, 0);
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            if d == OP_ANY
                                && ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & (PCRE2_PARTIAL_HARD)) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                partial_newline = TRUE;
                                could_continue = TRUE;
                            } else if (c >= 256
                                && d != OP_DIGIT
                                && d != OP_WHITESPACE
                                && d != OP_WORDCHAR)
                                || (c < 256
                                    && (d != OP_ANY || !IS_NEWLINE!(ptr))
                                    && ((*ctypes.add(c as usize)
                                        & *toptable1.as_ptr().add(d as usize))
                                        ^ *toptable2.as_ptr().add(d as usize))
                                        != 0)
                            {
                                if codevalue == OP_TYPEPOSUPTO {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                if count >= GET2!(code, 1) as i32 {
                                    ADD_NEW!(state_offset + 2 + IMM2_SIZE as i32, 0);
                                } else {
                                    ADD_NEW!(state_offset, count);
                                }
                            }
                        }
                    }

                    /* ========================================================== */
                    /* These are virtual opcodes that are used when something like
                    OP_TYPEPLUS has OP_PROP, OP_NOTPROP, OP_ANYNL, or OP_EXTUNI as its
                    argument. It keeps the code above fast for the other cases. The argument
                    is in the d variable. */
                    PROP_TYPEPLUS | PROP_TYPEMINPLUS | PROP_TYPEPOSPLUS => {
                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 4, 0);
                        }
                        if clen > 0 {
                            let OK: BOOL =
                                PROP_TEST!(c, codevalue, *code.add(2), *code.add(3));

                            if OK == (d == OP_PROP) as BOOL {
                                if count > 0 && codevalue == PROP_TYPEPOSPLUS {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                ADD_NEW!(state_offset, count);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    EXTUNI_TYPEPLUS | EXTUNI_TYPEMINPLUS | EXTUNI_TYPEPOSPLUS => {
                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 2, 0);
                        }
                        if clen > 0 {
                            let mut ncount: i32 = 0;
                            if count > 0 && codevalue == EXTUNI_TYPEPOSPLUS {
                                active_count -= 1; /* Remove non-match possibility */
                                next_active_state = next_active_state.sub(1);
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

                    /*-----------------------------------------------------------------*/
                    ANYNL_TYPEPLUS | ANYNL_TYPEMINPLUS | ANYNL_TYPEPOSPLUS => {
                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 2, 0);
                        }
                        if clen > 0 {
                            let mut ncount: i32 = 0;
                            'ANYNL01: {
                                match c {
                                    CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                        if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                                            break 'ANYNL01;
                                        }
                                        /* goto ANYNL01: fall into the shared code below */
                                    }

                                    CHAR_CR => {
                                        if ptr.add(1) < end_subject
                                            && *ptr.add(1) as u32 == CHAR_LF
                                        {
                                            ncount = 1;
                                        }
                                        /* Fall through */
                                    }

                                    /* ANYNL01: */
                                    CHAR_LF => {}

                                    _ => break 'ANYNL01,
                                }
                                if count > 0 && codevalue == ANYNL_TYPEPOSPLUS {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                ADD_NEW_DATA!(-state_offset, count, ncount);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    VSPACE_TYPEPLUS | VSPACE_TYPEMINPLUS | VSPACE_TYPEPOSPLUS => {
                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 2, 0);
                        }
                        if clen > 0 {
                            let OK: BOOL;
                            match c {
                                VSPACE_CASES!() => {
                                    OK = TRUE;
                                }

                                _ => {
                                    OK = FALSE;
                                }
                            }

                            if OK == (d == OP_VSPACE) as BOOL {
                                if count > 0 && codevalue == VSPACE_TYPEPOSPLUS {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                ADD_NEW_DATA!(-state_offset, count, 0);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    HSPACE_TYPEPLUS | HSPACE_TYPEMINPLUS | HSPACE_TYPEPOSPLUS => {
                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 2, 0);
                        }
                        if clen > 0 {
                            let OK: BOOL;
                            match c {
                                HSPACE_CASES!() => {
                                    OK = TRUE;
                                }

                                _ => {
                                    OK = FALSE;
                                }
                            }

                            if OK == (d == OP_HSPACE) as BOOL {
                                if count > 0 && codevalue == HSPACE_TYPEPOSPLUS {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                ADD_NEW_DATA!(-state_offset, count, 0);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    PROP_TYPEQUERY | PROP_TYPEMINQUERY | PROP_TYPEPOSQUERY
                    | PROP_TYPESTAR | PROP_TYPEMINSTAR | PROP_TYPEPOSSTAR => {
                        /* The QUERY variants set count = 4 and "goto QS1"; the STAR
                        variants set count = 0 and fall through to QS1. */
                        count = match codevalue {
                            PROP_TYPEQUERY | PROP_TYPEMINQUERY | PROP_TYPEPOSQUERY => 4,
                            _ => 0,
                        };

                        /* QS1: */
                        ADD_ACTIVE!(state_offset + 4, 0);
                        if clen > 0 {
                            let OK: BOOL =
                                PROP_TEST!(c, codevalue, *code.add(2), *code.add(3));

                            if OK == (d == OP_PROP) as BOOL {
                                if codevalue == PROP_TYPEPOSSTAR
                                    || codevalue == PROP_TYPEPOSQUERY
                                {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW!(state_offset + count, 0);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    EXTUNI_TYPEQUERY | EXTUNI_TYPEMINQUERY | EXTUNI_TYPEPOSQUERY
                    | EXTUNI_TYPESTAR | EXTUNI_TYPEMINSTAR | EXTUNI_TYPEPOSSTAR => {
                        /* count = 2 then "goto QS2" for the QUERY variants; the STAR
                        variants set count = 0 and fall through to QS2. */
                        count = match codevalue {
                            EXTUNI_TYPEQUERY | EXTUNI_TYPEMINQUERY | EXTUNI_TYPEPOSQUERY => 2,
                            _ => 0,
                        };

                        /* QS2: */
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            let mut ncount: i32 = 0;
                            if codevalue == EXTUNI_TYPEPOSSTAR
                                || codevalue == EXTUNI_TYPEPOSQUERY
                            {
                                active_count -= 1; /* Remove non-match possibility */
                                next_active_state = next_active_state.sub(1);
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
                    ANYNL_TYPEQUERY | ANYNL_TYPEMINQUERY | ANYNL_TYPEPOSQUERY
                    | ANYNL_TYPESTAR | ANYNL_TYPEMINSTAR | ANYNL_TYPEPOSSTAR => {
                        /* count = 2 then "goto QS3" for the QUERY variants; the STAR
                        variants set count = 0 and fall through to QS3. */
                        count = match codevalue {
                            ANYNL_TYPEQUERY | ANYNL_TYPEMINQUERY | ANYNL_TYPEPOSQUERY => 2,
                            _ => 0,
                        };

                        /* QS3: */
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            let mut ncount: i32 = 0;
                            'ANYNL02: {
                                match c {
                                    CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                        if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                                            break 'ANYNL02;
                                        }
                                        /* goto ANYNL02: fall into the shared code below */
                                    }

                                    CHAR_CR => {
                                        if ptr.add(1) < end_subject
                                            && *ptr.add(1) as u32 == CHAR_LF
                                        {
                                            ncount = 1;
                                        }
                                        /* Fall through */
                                    }

                                    /* ANYNL02: */
                                    CHAR_LF => {}

                                    _ => break 'ANYNL02,
                                }
                                if codevalue == ANYNL_TYPEPOSSTAR
                                    || codevalue == ANYNL_TYPEPOSQUERY
                                {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW_DATA!(-(state_offset + count), 0, ncount);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    VSPACE_TYPEQUERY | VSPACE_TYPEMINQUERY | VSPACE_TYPEPOSQUERY
                    | VSPACE_TYPESTAR | VSPACE_TYPEMINSTAR | VSPACE_TYPEPOSSTAR => {
                        /* count = 2 then "goto QS4" for the QUERY variants; the STAR
                        variants set count = 0 and fall through to QS4. */
                        count = match codevalue {
                            VSPACE_TYPEQUERY | VSPACE_TYPEMINQUERY | VSPACE_TYPEPOSQUERY => 2,
                            _ => 0,
                        };

                        /* QS4: */
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            let OK: BOOL;
                            match c {
                                VSPACE_CASES!() => {
                                    OK = TRUE;
                                }

                                _ => {
                                    OK = FALSE;
                                }
                            }
                            if OK == (d == OP_VSPACE) as BOOL {
                                if codevalue == VSPACE_TYPEPOSSTAR
                                    || codevalue == VSPACE_TYPEPOSQUERY
                                {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW_DATA!(-(state_offset + count), 0, 0);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    HSPACE_TYPEQUERY | HSPACE_TYPEMINQUERY | HSPACE_TYPEPOSQUERY
                    | HSPACE_TYPESTAR | HSPACE_TYPEMINSTAR | HSPACE_TYPEPOSSTAR => {
                        /* count = 2 then "goto QS5" for the QUERY variants; the STAR
                        variants set count = 0 and fall through to QS5. */
                        count = match codevalue {
                            HSPACE_TYPEQUERY | HSPACE_TYPEMINQUERY | HSPACE_TYPEPOSQUERY => 2,
                            _ => 0,
                        };

                        /* QS5: */
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            let OK: BOOL;
                            match c {
                                HSPACE_CASES!() => {
                                    OK = TRUE;
                                }

                                _ => {
                                    OK = FALSE;
                                }
                            }

                            if OK == (d == OP_HSPACE) as BOOL {
                                if codevalue == HSPACE_TYPEPOSSTAR
                                    || codevalue == HSPACE_TYPEPOSQUERY
                                {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW_DATA!(-(state_offset + count), 0, 0);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    PROP_TYPEEXACT | PROP_TYPEUPTO | PROP_TYPEMINUPTO | PROP_TYPEPOSUPTO => {
                        if codevalue != PROP_TYPEEXACT {
                            ADD_ACTIVE!(state_offset + 1 + IMM2_SIZE as i32 + 3, 0);
                        }
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let OK: BOOL = PROP_TEST!(
                                c,
                                codevalue,
                                *code.add(1 + IMM2_SIZE + 1),
                                *code.add(1 + IMM2_SIZE + 2)
                            );

                            if OK == (d == OP_PROP) as BOOL {
                                if codevalue == PROP_TYPEPOSUPTO {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                if count >= GET2!(code, 1) as i32 {
                                    ADD_NEW!(state_offset + 1 + IMM2_SIZE as i32 + 3, 0);
                                } else {
                                    ADD_NEW!(state_offset, count);
                                }
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    EXTUNI_TYPEEXACT | EXTUNI_TYPEUPTO | EXTUNI_TYPEMINUPTO
                    | EXTUNI_TYPEPOSUPTO => {
                        if codevalue != EXTUNI_TYPEEXACT {
                            ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as i32, 0);
                        }
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let nptr: PCRE2_SPTR;
                            let mut ncount: i32 = 0;
                            if codevalue == EXTUNI_TYPEPOSUPTO {
                                active_count -= 1; /* Remove non-match possibility */
                                next_active_state = next_active_state.sub(1);
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
                            if count >= GET2!(code, 1) as i32 {
                                ADD_NEW_DATA!(
                                    -(state_offset + 2 + IMM2_SIZE as i32),
                                    0,
                                    ncount
                                );
                            } else {
                                ADD_NEW_DATA!(-state_offset, count, ncount);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    ANYNL_TYPEEXACT | ANYNL_TYPEUPTO | ANYNL_TYPEMINUPTO
                    | ANYNL_TYPEPOSUPTO => {
                        if codevalue != ANYNL_TYPEEXACT {
                            ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as i32, 0);
                        }
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let mut ncount: i32 = 0;
                            'ANYNL03: {
                                match c {
                                    CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                        if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                                            break 'ANYNL03;
                                        }
                                        /* goto ANYNL03: fall into the shared code below */
                                    }

                                    CHAR_CR => {
                                        if ptr.add(1) < end_subject
                                            && *ptr.add(1) as u32 == CHAR_LF
                                        {
                                            ncount = 1;
                                        }
                                        /* Fall through */
                                    }

                                    /* ANYNL03: */
                                    CHAR_LF => {}

                                    _ => break 'ANYNL03,
                                }
                                if codevalue == ANYNL_TYPEPOSUPTO {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                if count >= GET2!(code, 1) as i32 {
                                    ADD_NEW_DATA!(
                                        -(state_offset + 2 + IMM2_SIZE as i32),
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
                    VSPACE_TYPEEXACT | VSPACE_TYPEUPTO | VSPACE_TYPEMINUPTO
                    | VSPACE_TYPEPOSUPTO => {
                        if codevalue != VSPACE_TYPEEXACT {
                            ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as i32, 0);
                        }
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let OK: BOOL;
                            match c {
                                VSPACE_CASES!() => {
                                    OK = TRUE;
                                }

                                _ => {
                                    OK = FALSE;
                                }
                            }

                            if OK == (d == OP_VSPACE) as BOOL {
                                if codevalue == VSPACE_TYPEPOSUPTO {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                if count >= GET2!(code, 1) as i32 {
                                    ADD_NEW_DATA!(
                                        -(state_offset + 2 + IMM2_SIZE as i32),
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
                    HSPACE_TYPEEXACT | HSPACE_TYPEUPTO | HSPACE_TYPEMINUPTO
                    | HSPACE_TYPEPOSUPTO => {
                        if codevalue != HSPACE_TYPEEXACT {
                            ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as i32, 0);
                        }
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let OK: BOOL;
                            match c {
                                HSPACE_CASES!() => {
                                    OK = TRUE;
                                }

                                _ => {
                                    OK = FALSE;
                                }
                            }

                            if OK == (d == OP_HSPACE) as BOOL {
                                if codevalue == HSPACE_TYPEPOSUPTO {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                if count >= GET2!(code, 1) as i32 {
                                    ADD_NEW_DATA!(
                                        -(state_offset + 2 + IMM2_SIZE as i32),
                                        0,
                                        0
                                    );
                                } else {
                                    ADD_NEW_DATA!(-state_offset, count, 0);
                                }
                            }
                        }
                    }

                    /* ========================================================== */
                    /* These opcodes are followed by a character that is usually compared
                    to the current subject character; it is loaded into d. We still get
                    here even if there is no subject character, because in some cases zero
                    repetitions are permitted. */

                    /*-----------------------------------------------------------------*/
                    OP_CHAR => {
                        if clen > 0 && c == d {
                            ADD_NEW!(state_offset + dlen + 1, 0);
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_CHARI => {
                        'OP_CHARI: {
                            if clen == 0 {
                                break 'OP_CHARI;
                            }

                            if utf_or_ucp != 0 {
                                if c == d {
                                    ADD_NEW!(state_offset + dlen + 1, 0);
                                } else {
                                    let othercase: u32;
                                    if c < 128 {
                                        othercase = *fcc.add(c as usize) as u32;
                                    } else {
                                        othercase = UCD_OTHERCASE!(c);
                                    }
                                    if d == othercase {
                                        ADD_NEW!(state_offset + dlen + 1, 0);
                                    }
                                }
                            }
                            /* Not UTF or UCP mode */
                            else {
                                if TABLE_GET!(c, lcc, c) == TABLE_GET!(d, lcc, d) {
                                    ADD_NEW!(state_offset + 2, 0);
                                }
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    /* This is a tricky one because it can match more than one character.
                    Find out how many characters to skip, and then set up a negative state
                    to wait for them to pass before continuing. */
                    OP_EXTUNI => {
                        if clen > 0 {
                            let mut ncount: i32 = 0;
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

                    /*-----------------------------------------------------------------*/
                    /* This is a tricky like EXTUNI because it too can match more than one
                    character (when CR is followed by LF). In this case, set up a negative
                    state to wait for one character to pass before continuing. */
                    OP_ANYNL => {
                        if clen > 0 {
                            'OP_ANYNL: {
                                match c {
                                    CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                        if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                                            break 'OP_ANYNL;
                                        }
                                        /* Fall through to the CHAR_LF code */
                                        ADD_NEW!(state_offset + 1, 0);
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
                    }

                    /*-----------------------------------------------------------------*/
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

                    /*-----------------------------------------------------------------*/
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

                    /*-----------------------------------------------------------------*/
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

                    /*-----------------------------------------------------------------*/
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

                    /*-----------------------------------------------------------------*/
                    /* Match a negated single character casefully. */
                    OP_NOT => {
                        if clen > 0 && c != d {
                            ADD_NEW!(state_offset + dlen + 1, 0);
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    /* Match a negated single character caselessly. */
                    OP_NOTI => {
                        if clen > 0 {
                            let otherd: u32;
                            if utf_or_ucp != 0 && d >= 128 {
                                otherd = UCD_OTHERCASE!(d);
                            } else {
                                otherd = TABLE_GET!(d, fcc, d) as u32;
                            }
                            if c != d && c != otherd {
                                ADD_NEW!(state_offset + dlen + 1, 0);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_PLUSI | OP_MINPLUSI | OP_POSPLUSI | OP_NOTPLUSI | OP_NOTMINPLUSI
                    | OP_NOTPOSPLUSI | OP_PLUS | OP_MINPLUS | OP_POSPLUS | OP_NOTPLUS
                    | OP_NOTMINPLUS | OP_NOTPOSPLUS => {
                        match codevalue {
                            OP_PLUSI | OP_MINPLUSI | OP_POSPLUSI | OP_NOTPLUSI
                            | OP_NOTMINPLUSI | OP_NOTPOSPLUSI => {
                                caseless = TRUE;
                                codevalue -= OP_STARI - OP_STAR;
                                /* Fall through */
                            }
                            _ => {}
                        }
                        count = (*current_state).count; /* Already matched */
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + dlen + 1, 0);
                        }
                        if clen > 0 {
                            let mut otherd: u32 = NOTACHAR;
                            if caseless != 0 {
                                if utf_or_ucp != 0 && d >= 128 {
                                    otherd = UCD_OTHERCASE!(d);
                                } else {
                                    otherd = TABLE_GET!(d, fcc, d) as u32;
                                }
                            }
                            if (c == d || c == otherd) == (codevalue < OP_NOTSTAR) {
                                if count > 0
                                    && (codevalue == OP_POSPLUS || codevalue == OP_NOTPOSPLUS)
                                {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                ADD_NEW!(state_offset, count);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_QUERYI | OP_MINQUERYI | OP_POSQUERYI | OP_NOTQUERYI
                    | OP_NOTMINQUERYI | OP_NOTPOSQUERYI | OP_QUERY | OP_MINQUERY
                    | OP_POSQUERY | OP_NOTQUERY | OP_NOTMINQUERY | OP_NOTPOSQUERY => {
                        match codevalue {
                            OP_QUERYI | OP_MINQUERYI | OP_POSQUERYI | OP_NOTQUERYI
                            | OP_NOTMINQUERYI | OP_NOTPOSQUERYI => {
                                caseless = TRUE;
                                codevalue -= OP_STARI - OP_STAR;
                                /* Fall through */
                            }
                            _ => {}
                        }
                        ADD_ACTIVE!(state_offset + dlen + 1, 0);
                        if clen > 0 {
                            let mut otherd: u32 = NOTACHAR;
                            if caseless != 0 {
                                if utf_or_ucp != 0 && d >= 128 {
                                    otherd = UCD_OTHERCASE!(d);
                                } else {
                                    otherd = TABLE_GET!(d, fcc, d) as u32;
                                }
                            }
                            if (c == d || c == otherd) == (codevalue < OP_NOTSTAR) {
                                if codevalue == OP_POSQUERY || codevalue == OP_NOTPOSQUERY {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW!(state_offset + dlen + 1, 0);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_STARI | OP_MINSTARI | OP_POSSTARI | OP_NOTSTARI | OP_NOTMINSTARI
                    | OP_NOTPOSSTARI | OP_STAR | OP_MINSTAR | OP_POSSTAR | OP_NOTSTAR
                    | OP_NOTMINSTAR | OP_NOTPOSSTAR => {
                        match codevalue {
                            OP_STARI | OP_MINSTARI | OP_POSSTARI | OP_NOTSTARI
                            | OP_NOTMINSTARI | OP_NOTPOSSTARI => {
                                caseless = TRUE;
                                codevalue -= OP_STARI - OP_STAR;
                                /* Fall through */
                            }
                            _ => {}
                        }
                        ADD_ACTIVE!(state_offset + dlen + 1, 0);
                        if clen > 0 {
                            let mut otherd: u32 = NOTACHAR;
                            if caseless != 0 {
                                if utf_or_ucp != 0 && d >= 128 {
                                    otherd = UCD_OTHERCASE!(d);
                                } else {
                                    otherd = TABLE_GET!(d, fcc, d) as u32;
                                }
                            }
                            if (c == d || c == otherd) == (codevalue < OP_NOTSTAR) {
                                if codevalue == OP_POSSTAR || codevalue == OP_NOTPOSSTAR {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW!(state_offset, 0);
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_EXACTI | OP_NOTEXACTI | OP_EXACT | OP_NOTEXACT => {
                        match codevalue {
                            OP_EXACTI | OP_NOTEXACTI => {
                                caseless = TRUE;
                                codevalue -= OP_STARI - OP_STAR;
                                /* Fall through */
                            }
                            _ => {}
                        }
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let mut otherd: u32 = NOTACHAR;
                            if caseless != 0 {
                                if utf_or_ucp != 0 && d >= 128 {
                                    otherd = UCD_OTHERCASE!(d);
                                } else {
                                    otherd = TABLE_GET!(d, fcc, d) as u32;
                                }
                            }
                            if (c == d || c == otherd) == (codevalue < OP_NOTSTAR) {
                                count += 1;
                                if count >= GET2!(code, 1) as i32 {
                                    ADD_NEW!(
                                        state_offset + dlen + 1 + IMM2_SIZE as i32,
                                        0
                                    );
                                } else {
                                    ADD_NEW!(state_offset, count);
                                }
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_UPTOI | OP_MINUPTOI | OP_POSUPTOI | OP_NOTUPTOI | OP_NOTMINUPTOI
                    | OP_NOTPOSUPTOI | OP_UPTO | OP_MINUPTO | OP_POSUPTO | OP_NOTUPTO
                    | OP_NOTMINUPTO | OP_NOTPOSUPTO => {
                        match codevalue {
                            OP_UPTOI | OP_MINUPTOI | OP_POSUPTOI | OP_NOTUPTOI
                            | OP_NOTMINUPTOI | OP_NOTPOSUPTOI => {
                                caseless = TRUE;
                                codevalue -= OP_STARI - OP_STAR;
                                /* Fall through */
                            }
                            _ => {}
                        }
                        ADD_ACTIVE!(state_offset + dlen + 1 + IMM2_SIZE as i32, 0);
                        count = (*current_state).count; /* Number already matched */
                        if clen > 0 {
                            let mut otherd: u32 = NOTACHAR;
                            if caseless != 0 {
                                if utf_or_ucp != 0 && d >= 128 {
                                    otherd = UCD_OTHERCASE!(d);
                                } else {
                                    otherd = TABLE_GET!(d, fcc, d) as u32;
                                }
                            }
                            if (c == d || c == otherd) == (codevalue < OP_NOTSTAR) {
                                if codevalue == OP_POSUPTO || codevalue == OP_NOTPOSUPTO {
                                    active_count -= 1; /* Remove non-match possibility */
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                if count >= GET2!(code, 1) as i32 {
                                    ADD_NEW!(
                                        state_offset + dlen + 1 + IMM2_SIZE as i32,
                                        0
                                    );
                                } else {
                                    ADD_NEW!(state_offset, count);
                                }
                            }
                        }
                    }

                    /* ========================================================== */
                    /* These are the class-handling opcodes */
                    OP_CLASS | OP_NCLASS | OP_XCLASS | OP_ECLASS => {
                        let mut isinclass: BOOL = FALSE;
                        let next_state_offset: i32;
                        let ecode: PCRE2_SPTR;

                        /* An extended class may have a table or a list of single characters,
                        ranges, or both, and it may be positive or negative. There's a
                        function that sorts all this out. */

                        if codevalue == OP_XCLASS {
                            ecode = code.add(GET!(code, 1) as usize);
                            if clen > 0 {
                                isinclass = crate::xclass::_pcre2_xclass_8(
                                    c,
                                    code.add(1 + LINK_SIZE),
                                    (*mb).start_code as *const u8,
                                    utf,
                                );
                            }
                        }
                        /* A nested set-based class has internal opcodes for performing
                        set operations. */
                        else if codevalue == OP_ECLASS {
                            ecode = code.add(GET!(code, 1) as usize);
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
                        /* For a simple class, there is always just a 32-byte table, and we
                        can set isinclass from it. */
                        else {
                            ecode = code.add(1 + (32 / core::mem::size_of::<PCRE2_UCHAR>()));
                            if clen > 0 {
                                isinclass = if c > 255 {
                                    (codevalue == OP_NCLASS) as BOOL
                                } else {
                                    ((*(code.add(1) as *const u8).add((c / 8) as usize)
                                        & (1u32 << (c & 7)) as u8)
                                        != 0) as BOOL
                                };
                            }
                        }

                        /* At this point, isinclass is set for all kinds of class, and ecode
                        points to the byte after the end of the class. If there is a
                        quantifier, this is where it will be. */

                        next_state_offset = (ecode as usize - start_code as usize) as i32;

                        match *ecode as u32 {
                            OP_CRSTAR | OP_CRMINSTAR | OP_CRPOSSTAR => {
                                ADD_ACTIVE!(next_state_offset + 1, 0);
                                if isinclass != 0 {
                                    if *ecode as u32 == OP_CRPOSSTAR {
                                        active_count -= 1; /* Remove non-match possibility */
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
                                    if count > 0 && *ecode as u32 == OP_CRPOSPLUS {
                                        active_count -= 1; /* Remove non-match possibility */
                                        next_active_state = next_active_state.sub(1);
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
                                        next_active_state = next_active_state.sub(1);
                                    }
                                    ADD_NEW!(next_state_offset + 1, 0);
                                }
                            }

                            OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                                count = (*current_state).count; /* Already matched */
                                if count >= GET2!(ecode, 1) as i32 {
                                    ADD_ACTIVE!(
                                        next_state_offset + 1 + 2 * IMM2_SIZE as i32,
                                        0
                                    );
                                }
                                if isinclass != 0 {
                                    let max: i32 = GET2!(ecode, 1 + IMM2_SIZE) as i32;

                                    if *ecode as u32 == OP_CRPOSRANGE
                                        && count >= GET2!(ecode, 1) as i32
                                    {
                                        active_count -= 1; /* Remove non-match possibility */
                                        next_active_state = next_active_state.sub(1);
                                    }

                                    count += 1;
                                    if count >= max && max != 0 {
                                        /* Max 0 => no limit */
                                        ADD_NEW!(
                                            next_state_offset + 1 + 2 * IMM2_SIZE as i32,
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

                    /* ========================================================== */
                    /* These are the opcodes for fancy brackets of various kinds. We have
                    to use recursion in order to handle them. The "always failing" assertion
                    (?!) is optimised to OP_FAIL when compiling, so we have to support that,
                    though the other "backtracking verbs" are not supported. */
                    OP_FAIL => {}

                    OP_ASSERT | OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT => {
                        let mut rc: i32;
                        let local_workspace: *mut i32;
                        let local_offsets: *mut PCRE2_SIZE;
                        let mut endasscode: PCRE2_SPTR = code.add(GET!(code, 1) as usize);
                        let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;

                        if ((*rws).free as usize) < RWS_RSIZE + RWS_OVEC_OSIZE {
                            rc = more_workspace(&mut rws, RWS_OVEC_OSIZE as u32, mb);
                            if rc != 0 {
                                return rc;
                            }
                            RWS = rws as *mut i32;
                        }

                        local_offsets =
                            RWS.add(((*rws).size - (*rws).free) as usize) as *mut PCRE2_SIZE;
                        local_workspace = (local_offsets as *mut i32).add(RWS_OVEC_OSIZE);
                        (*rws).free -= (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        while *endasscode as u32 == OP_ALT {
                            endasscode = endasscode.add(GET!(endasscode, 1) as usize);
                        }

                        rc = internal_dfa_match(
                            mb,       /* static match data */
                            code,     /* this subexpression's code */
                            ptr,      /* where we currently are */
                            (ptr as usize - start_subject as usize) as PCRE2_SIZE, /* start offset */
                            local_offsets, /* offset vector */
                            (RWS_OVEC_OSIZE / OVEC_UNIT) as u32, /* size of same */
                            local_workspace, /* workspace vector */
                            RWS_RSIZE as i32, /* size of same */
                            rlevel,   /* function recursion level */
                            RWS,
                        ); /* recursion workspace */

                        (*rws).free += (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        if rc < 0 && rc != PCRE2_ERROR_NOMATCH {
                            return rc;
                        }
                        if (rc >= 0)
                            == (codevalue == OP_ASSERT || codevalue == OP_ASSERTBACK)
                        {
                            ADD_ACTIVE!(
                                (endasscode as usize - start_code as usize) as i32
                                    + LINK_SIZE as i32
                                    + 1,
                                0
                            );
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_COND | OP_SCOND => {
                        /* The C "break" that abandons this thread after a callout exits
                        the switch; here it exits this labelled block. */
                        'OP_COND: {
                            let codelink: i32 = GET!(code, 1) as i32;
                            let condcode: PCRE2_UCHAR;

                            /* Because of the way auto-callout works during compile, a callout item
                            is inserted between OP_COND and an assertion condition. This does not
                            happen for the other conditions. */

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
                                    1 + LINK_SIZE,
                                    &mut callout_length,
                                );
                                if rrc < 0 {
                                    return rrc; /* Abandon */
                                }
                                if rrc > 0 {
                                    break 'OP_COND; /* Fail this thread */
                                }
                                code = code.add(callout_length); /* Skip callout data */
                            }

                            condcode = *code.add(LINK_SIZE + 1);

                            /* Back reference conditions and duplicate named recursion conditions
                            are not supported */

                            if condcode as u32 == OP_CREF
                                || condcode as u32 == OP_DNCREF
                                || condcode as u32 == OP_DNRREF
                            {
                                return PCRE2_ERROR_DFA_UCOND;
                            }

                            /* The DEFINE condition is always false, and the assertion (?!) is
                            converted to OP_FAIL. */

                            if condcode as u32 == OP_FALSE || condcode as u32 == OP_FAIL {
                                ADD_ACTIVE!(
                                    state_offset + codelink + LINK_SIZE as i32 + 1,
                                    0
                                );
                            }
                            /* There is also an always-true condition */
                            else if condcode as u32 == OP_TRUE {
                                ADD_ACTIVE!(state_offset + LINK_SIZE as i32 + 2, 0);
                            }
                            /* The only supported version of OP_RREF is for the value RREF_ANY,
                            which means "test if in any recursion". We can't test for specifically
                            recursed groups. */
                            else if condcode as u32 == OP_RREF {
                                let value: u32 = GET2!(code, LINK_SIZE + 2);
                                if value != RREF_ANY {
                                    return PCRE2_ERROR_DFA_UCOND;
                                }
                                if !(*mb).recursive.is_null() {
                                    ADD_ACTIVE!(
                                        state_offset
                                            + LINK_SIZE as i32
                                            + 2
                                            + IMM2_SIZE as i32,
                                        0
                                    );
                                } else {
                                    ADD_ACTIVE!(
                                        state_offset + codelink + LINK_SIZE as i32 + 1,
                                        0
                                    );
                                }
                            }
                            /* Otherwise, the condition is an assertion */
                            else {
                                let mut rc: i32;
                                let local_workspace: *mut i32;
                                let local_offsets: *mut PCRE2_SIZE;
                                let asscode: PCRE2_SPTR = code.add(LINK_SIZE + 1);
                                let mut endasscode: PCRE2_SPTR =
                                    asscode.add(GET!(asscode, 1) as usize);
                                let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;

                                if ((*rws).free as usize) < RWS_RSIZE + RWS_OVEC_OSIZE {
                                    rc = more_workspace(&mut rws, RWS_OVEC_OSIZE as u32, mb);
                                    if rc != 0 {
                                        return rc;
                                    }
                                    RWS = rws as *mut i32;
                                }

                                local_offsets = RWS.add(((*rws).size - (*rws).free) as usize)
                                    as *mut PCRE2_SIZE;
                                local_workspace =
                                    (local_offsets as *mut i32).add(RWS_OVEC_OSIZE);
                                (*rws).free -= (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                                while *endasscode as u32 == OP_ALT {
                                    endasscode =
                                        endasscode.add(GET!(endasscode, 1) as usize);
                                }

                                rc = internal_dfa_match(
                                    mb,      /* fixed match data */
                                    asscode, /* this subexpression's code */
                                    ptr,     /* where we currently are */
                                    (ptr as usize - start_subject as usize) as PCRE2_SIZE,
                                    local_offsets, /* offset vector */
                                    (RWS_OVEC_OSIZE / OVEC_UNIT) as u32, /* size of same */
                                    local_workspace, /* workspace vector */
                                    RWS_RSIZE as i32, /* size of same */
                                    rlevel,  /* function recursion level */
                                    RWS,
                                ); /* recursion workspace */

                                (*rws).free += (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                                if rc < 0 && rc != PCRE2_ERROR_NOMATCH {
                                    return rc;
                                }
                                if (rc >= 0)
                                    == (condcode as u32 == OP_ASSERT
                                        || condcode as u32 == OP_ASSERTBACK)
                                {
                                    ADD_ACTIVE!(
                                        (endasscode as usize - start_code as usize) as i32
                                            + LINK_SIZE as i32
                                            + 1,
                                        0
                                    );
                                } else {
                                    ADD_ACTIVE!(
                                        state_offset + codelink + LINK_SIZE as i32 + 1,
                                        0
                                    );
                                }
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_RECURSE => {
                        let mut rc: i32;
                        let local_workspace: *mut i32;
                        let local_offsets: *mut PCRE2_SIZE;
                        let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;
                        let callpat: PCRE2_SPTR = start_code.add(GET!(code, 1) as usize);
                        let recno: u32 = if callpat == (*mb).start_code {
                            0
                        } else {
                            GET2!(callpat, 1 + LINK_SIZE)
                        };

                        /* Argument list has not been supported yet. */
                        if *code.add(1 + LINK_SIZE) as u32 == OP_CREF {
                            return PCRE2_ERROR_DFA_UITEM;
                        }

                        if ((*rws).free as usize) < RWS_RSIZE + RWS_OVEC_RSIZE {
                            rc = more_workspace(&mut rws, RWS_OVEC_RSIZE as u32, mb);
                            if rc != 0 {
                                return rc;
                            }
                            RWS = rws as *mut i32;
                        }

                        local_offsets =
                            RWS.add(((*rws).size - (*rws).free) as usize) as *mut PCRE2_SIZE;
                        local_workspace = (local_offsets as *mut i32).add(RWS_OVEC_RSIZE);
                        (*rws).free -= (RWS_RSIZE + RWS_OVEC_RSIZE) as u32;

                        /* Check for repeating a recursion without advancing the subject
                        pointer or last used character. This should catch convoluted mutual
                        recursions. (Some simple cases are caught at compile time.) */

                        {
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
                        }

                        /* Remember this recursion and where we started it so as to
                        catch infinite loops. */

                        new_recursive.group_num = recno;
                        new_recursive.subject_position = ptr;
                        new_recursive.last_used_ptr = (*mb).last_used_ptr;
                        new_recursive.prevrec = (*mb).recursive;
                        (*mb).recursive = &mut new_recursive;

                        rc = internal_dfa_match(
                            mb,      /* fixed match data */
                            callpat, /* this subexpression's code */
                            ptr,     /* where we currently are */
                            (ptr as usize - start_subject as usize) as PCRE2_SIZE,
                            local_offsets, /* offset vector */
                            (RWS_OVEC_RSIZE / OVEC_UNIT) as u32, /* size of same */
                            local_workspace, /* workspace vector */
                            RWS_RSIZE as i32, /* size of same */
                            rlevel,  /* function recursion level */
                            RWS,
                        ); /* recursion workspace */

                        (*rws).free += (RWS_RSIZE + RWS_OVEC_RSIZE) as u32;
                        (*mb).recursive = new_recursive.prevrec; /* Done this recursion */

                        /* Ran out of internal offsets */

                        if rc == 0 {
                            return PCRE2_ERROR_DFA_RECURSE;
                        }

                        /* For each successful matched substring, set up the next state with a
                        count of characters to skip before trying it. Note that the count is in
                        characters, not bytes. */

                        if rc > 0 {
                            rc = rc * 2 - 2;
                            while rc >= 0 {
                                let mut charcount: PCRE2_SIZE = (*local_offsets
                                    .add((rc + 1) as usize))
                                .wrapping_sub(*local_offsets.add(rc as usize));
                                if utf != 0 {
                                    let mut p: PCRE2_SPTR =
                                        start_subject.add(*local_offsets.add(rc as usize));
                                    let pp: PCRE2_SPTR =
                                        start_subject.add(*local_offsets.add((rc + 1) as usize));
                                    while p < pp {
                                        let t = *p;
                                        p = p.add(1);
                                        if NOT_FIRSTCU!(t) {
                                            charcount = charcount.wrapping_sub(1);
                                        }
                                    }
                                }
                                if charcount > 0 {
                                    ADD_NEW_DATA!(
                                        -(state_offset + LINK_SIZE as i32 + 1),
                                        0,
                                        charcount.wrapping_sub(1) as i32
                                    );
                                } else {
                                    ADD_ACTIVE!(state_offset + LINK_SIZE as i32 + 1, 0);
                                }
                                rc -= 2;
                            }
                        } else if rc != PCRE2_ERROR_NOMATCH {
                            return rc;
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_BRAPOS | OP_SBRAPOS | OP_CBRAPOS | OP_SCBRAPOS | OP_BRAPOSZERO => {
                        let mut rc: i32;
                        let local_workspace: *mut i32;
                        let local_offsets: *mut PCRE2_SIZE;
                        let mut charcount: PCRE2_SIZE;
                        let mut matched_count: PCRE2_SIZE;
                        let mut local_ptr: PCRE2_SPTR = ptr;
                        let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;
                        let allow_zero: BOOL;

                        if ((*rws).free as usize) < RWS_RSIZE + RWS_OVEC_OSIZE {
                            rc = more_workspace(&mut rws, RWS_OVEC_OSIZE as u32, mb);
                            if rc != 0 {
                                return rc;
                            }
                            RWS = rws as *mut i32;
                        }

                        local_offsets =
                            RWS.add(((*rws).size - (*rws).free) as usize) as *mut PCRE2_SIZE;
                        local_workspace = (local_offsets as *mut i32).add(RWS_OVEC_OSIZE);
                        (*rws).free -= (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        if codevalue == OP_BRAPOSZERO {
                            allow_zero = TRUE;
                            code = code.add(1); /* The following opcode will be one of the above BRAs */
                        } else {
                            allow_zero = FALSE;
                        }

                        /* Loop to match the subpattern as many times as possible as if it were
                        a complete pattern. */

                        matched_count = 0;
                        loop {
                            rc = internal_dfa_match(
                                mb,        /* fixed match data */
                                code,      /* this subexpression's code */
                                local_ptr, /* where we currently are */
                                (ptr as usize - start_subject as usize) as PCRE2_SIZE,
                                local_offsets, /* offset vector */
                                (RWS_OVEC_OSIZE / OVEC_UNIT) as u32, /* size of same */
                                local_workspace, /* workspace vector */
                                RWS_RSIZE as i32, /* size of same */
                                rlevel,    /* function recursion level */
                                RWS,
                            ); /* recursion workspace */

                            /* Failed to match */

                            if rc < 0 {
                                if rc != PCRE2_ERROR_NOMATCH {
                                    return rc;
                                }
                                break;
                            }

                            /* Matched: break the loop if zero characters matched. */

                            charcount =
                                (*local_offsets.add(1)).wrapping_sub(*local_offsets.add(0));
                            if charcount == 0 {
                                break;
                            }
                            local_ptr = local_ptr.add(charcount); /* Advance temporary position ptr */
                            matched_count += 1;
                        }

                        (*rws).free += (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        /* At this point we have matched the subpattern matched_count
                        times, and local_ptr is pointing to the character after the end of the
                        last match. */

                        if matched_count > 0 || allow_zero != 0 {
                            let mut end_subpattern: PCRE2_SPTR = code;
                            let next_state_offset: i32;

                            loop {
                                end_subpattern =
                                    end_subpattern.add(GET!(end_subpattern, 1) as usize);
                                if *end_subpattern as u32 != OP_ALT {
                                    break;
                                }
                            }
                            next_state_offset = (end_subpattern as usize
                                - start_code as usize) as i32
                                + LINK_SIZE as i32
                                + 1;

                            /* Optimization: if there are no more active states, and there
                            are no new states yet set up, then skip over the subject string
                            right here, to save looping. Otherwise, set up the new state to swing
                            into action when the end of the matched substring is reached. */

                            if i + 1 >= active_count && new_count == 0 {
                                ptr = local_ptr;
                                clen = 0;
                                ADD_NEW!(next_state_offset, 0);
                            } else {
                                let mut p: PCRE2_SPTR = ptr;
                                let pp: PCRE2_SPTR = local_ptr;
                                charcount = (pp as usize - p as usize) as PCRE2_SIZE;
                                if utf != 0 {
                                    while p < pp {
                                        let t = *p;
                                        p = p.add(1);
                                        if NOT_FIRSTCU!(t) {
                                            charcount = charcount.wrapping_sub(1);
                                        }
                                    }
                                }
                                ADD_NEW_DATA!(
                                    -next_state_offset,
                                    0,
                                    charcount.wrapping_sub(1) as i32
                                );
                            }
                        }
                    }

                    /*-----------------------------------------------------------------*/
                    OP_ONCE => {
                        let mut rc: i32;
                        let local_workspace: *mut i32;
                        let local_offsets: *mut PCRE2_SIZE;
                        let mut rws: *mut RWS_anchor = RWS as *mut RWS_anchor;

                        if ((*rws).free as usize) < RWS_RSIZE + RWS_OVEC_OSIZE {
                            rc = more_workspace(&mut rws, RWS_OVEC_OSIZE as u32, mb);
                            if rc != 0 {
                                return rc;
                            }
                            RWS = rws as *mut i32;
                        }

                        local_offsets =
                            RWS.add(((*rws).size - (*rws).free) as usize) as *mut PCRE2_SIZE;
                        local_workspace = (local_offsets as *mut i32).add(RWS_OVEC_OSIZE);
                        (*rws).free -= (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        rc = internal_dfa_match(
                            mb,   /* fixed match data */
                            code, /* this subexpression's code */
                            ptr,  /* where we currently are */
                            (ptr as usize - start_subject as usize) as PCRE2_SIZE,
                            local_offsets, /* offset vector */
                            (RWS_OVEC_OSIZE / OVEC_UNIT) as u32, /* size of same */
                            local_workspace, /* workspace vector */
                            RWS_RSIZE as i32, /* size of same */
                            rlevel, /* function recursion level */
                            RWS,
                        ); /* recursion workspace */

                        (*rws).free += (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        if rc >= 0 {
                            let mut end_subpattern: PCRE2_SPTR = code;
                            let mut charcount: PCRE2_SIZE =
                                (*local_offsets.add(1)).wrapping_sub(*local_offsets.add(0));
                            let next_state_offset: i32;
                            let repeat_state_offset: i32;

                            loop {
                                end_subpattern =
                                    end_subpattern.add(GET!(end_subpattern, 1) as usize);
                                if *end_subpattern as u32 != OP_ALT {
                                    break;
                                }
                            }
                            next_state_offset = (end_subpattern as usize
                                - start_code as usize) as i32
                                + LINK_SIZE as i32
                                + 1;

                            /* If the end of this subpattern is KETRMAX or KETRMIN, we must
                            arrange for the repeat state also to be added to the relevant list.
                            Calculate the offset, or set -1 for no repeat. */

                            repeat_state_offset = if *end_subpattern as u32 == OP_KETRMAX
                                || *end_subpattern as u32 == OP_KETRMIN
                            {
                                (end_subpattern as usize - start_code as usize) as i32
                                    - GET!(end_subpattern, 1) as i32
                            } else {
                                -1
                            };

                            /* If we have matched an empty string, add the next state at the
                            current character pointer. This is important so that the duplicate
                            checking kicks in, which is what breaks infinite loops that match an
                            empty string. */

                            if charcount == 0 {
                                ADD_ACTIVE!(next_state_offset, 0);
                            }
                            /* Optimization: if there are no more active states, and there
                            are no new states yet set up, then skip over the subject string
                            right here, to save looping. Otherwise, set up the new state to swing
                            into action when the end of the matched substring is reached. */
                            else if i + 1 >= active_count && new_count == 0 {
                                ptr = ptr.add(charcount);
                                clen = 0;
                                ADD_NEW!(next_state_offset, 0);

                                /* If we are adding a repeat state at the new character position,
                                we must fudge things so that it is the only current state.
                                Otherwise, it might be a duplicate of one we processed before, and
                                that would cause it to be skipped. */

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
                                        let t = *p;
                                        p = p.add(1);
                                        if NOT_FIRSTCU!(t) {
                                            charcount = charcount.wrapping_sub(1);
                                        }
                                    }
                                }
                                ADD_NEW_DATA!(
                                    -next_state_offset,
                                    0,
                                    charcount.wrapping_sub(1) as i32
                                );
                                if repeat_state_offset >= 0 {
                                    ADD_NEW_DATA!(
                                        -repeat_state_offset,
                                        0,
                                        charcount.wrapping_sub(1) as i32
                                    );
                                }
                            }
                        } else if rc != PCRE2_ERROR_NOMATCH {
                            return rc;
                        }
                    }

                    /* ========================================================== */
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
                            return rrc; /* Abandon */
                        }
                        if rrc == 0 {
                            ADD_ACTIVE!(state_offset + callout_length as i32, 0);
                        }
                    }

                    /* ========================================================== */
                    _ => {
                        /* Unsupported opcode */
                        return PCRE2_ERROR_DFA_UITEM;
                    }
                }
            }
            /* NEXT_ACTIVE_STATE: continue; */
            i += 1;
        } /* End of loop scanning active states */

        /* We have finished the processing at the current subject character. If no
        new states have been set for the next character, we have found all the
        matches that we are going to find. If partial matching has been requested,
        check for appropriate conditions.

        The "could_continue" variable is true if a state could have continued but
        for the fact that the end of the subject was reached. */

        if new_count <= 0 {
            if could_continue != 0 &&                            /* Some could go on, and */
                (                                            /* either... */
                ((*mb).moptions & PCRE2_PARTIAL_HARD) != 0      /* Hard partial */
                ||                                           /* or... */
                (((*mb).moptions & PCRE2_PARTIAL_SOFT) != 0 &&  /* Soft partial and */
                 match_count < 0)
                ) &&                                         /* And... */
                (
                partial_newline != 0 ||                   /* Either partial NL */
                  (                                  /* or ... */
                  ptr >= end_subject &&              /* End of subject and */
                    (                                  /* either */
                    ptr > (*mb).start_used_ptr ||        /* Inspected non-empty string */
                    (*mb).allowemptypartial != 0         /* or pattern has lookbehind */
                    )                                  /* or could match empty */
                  )
                )
            {
                match_count = PCRE2_ERROR_PARTIAL;
            }
            break 'subject_loop; /* Exit from loop along the subject string */
        }

        /* One or more states are active for the next character. */

        ptr = ptr.add(clen as usize); /* Advance to next subject character */
    } /* Loop to move along the subject string */

    /* Control gets here from "break" a few lines above. If we have a match and
    PCRE2_ENDANCHORED is set, the match fails. */

    if match_count >= 0
        && (((*mb).moptions | (*mb).poptions) & PCRE2_ENDANCHORED) != 0
        && ptr < end_subject
    {
        match_count = PCRE2_ERROR_NOMATCH;
    }

    match_count
}
