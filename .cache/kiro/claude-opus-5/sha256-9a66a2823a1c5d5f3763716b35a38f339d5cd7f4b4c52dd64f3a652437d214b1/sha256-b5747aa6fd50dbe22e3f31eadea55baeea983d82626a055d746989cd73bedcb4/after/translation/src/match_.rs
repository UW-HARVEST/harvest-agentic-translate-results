//! Translation of `c_src/src/pcre2_match.c`.
//!
//! The interpreter for compiled patterns. The `match()` function uses a
//! heap-based frame stack (rather than C recursion) via the RMATCH/RRETURN
//! machinery; this translation keeps that machinery literal, using raw
//! `heapframe` pointer arithmetic, `frame_size`, the `Fxxx` field accessors,
//! the `return_id` dispatch and the labelled resume points.
//!
//! Build configuration: `PCRE2_CODE_UNIT_WIDTH == 8`, `SUPPORT_UNICODE`
//! (therefore `SUPPORT_WIDE_CHARS`), no `SUPPORT_JIT`, no `EBCDIC`, no
//! `PCRE2_DEBUG`, no `SUPPORT_VALGRIND`, `LINK_SIZE == 2`.

#![allow(
    non_snake_case,
    non_upper_case_globals,
    unused_parens,
    unused_assignments,
    dead_code
)]

use core::ffi::{c_int, c_void};

use crate::chars::*;
use crate::context::_pcre2_default_match_context_8;
use crate::internal::*;
use crate::newline::{is_newline, was_newline};
use crate::opcodes::*;
use crate::ord2utf::ord2utf;
use crate::ucp::*;

/* Masks for identifying the public options that are permitted at match time. */

const PUBLIC_MATCH_OPTIONS: u32 = PCRE2_ANCHORED
    | PCRE2_ENDANCHORED
    | PCRE2_NOTBOL
    | PCRE2_NOTEOL
    | PCRE2_NOTEMPTY
    | PCRE2_NOTEMPTY_ATSTART
    | PCRE2_NO_UTF_CHECK
    | PCRE2_PARTIAL_HARD
    | PCRE2_PARTIAL_SOFT
    | PCRE2_NO_JIT
    | PCRE2_COPY_MATCHED_SUBJECT
    | PCRE2_DISABLE_RECURSELOOP_CHECK;

const RECURSE_UNSET: u32 = 0xffffffff; /* Bigger than max group number */

/* Non-error returns from and within the match() function. */

const MATCH_MATCH: c_int = 1;
const MATCH_NOMATCH: c_int = 0;

/* Special internal returns used in the match() function. */

const MATCH_ACCEPT: c_int = -999;
const MATCH_KETRPOS: c_int = -998;
const MATCH_COMMIT: c_int = -997;
const MATCH_PRUNE: c_int = -996;
const MATCH_SKIP: c_int = -995;
const MATCH_SKIP_ARG: c_int = -994;
const MATCH_THEN: c_int = -993;
const MATCH_BACKTRACK_MAX: c_int = MATCH_THEN;
const MATCH_BACKTRACK_MIN: c_int = MATCH_COMMIT;

/* Group frame type values. */

const GF_CAPTURE: u32 = 0x00010000;
const GF_NOCAPTURE: u32 = 0x00020000;
const GF_CONDASSERT: u32 = 0x00030000;
const GF_RECURSE: u32 = 0x00040000;

#[inline]
const fn GF_IDMASK(a: u32) -> u32 {
    a & 0xffff0000
}
#[inline]
const fn GF_DATAMASK(a: u32) -> u32 {
    a & 0x0000ffff
}

/* Repetition types */

const REPTYPE_MIN: u32 = 0;
const REPTYPE_MAX: u32 = 1;
const REPTYPE_POS: u32 = 2;

const UINT32_MAX: u32 = u32::MAX;

/* Min and max values for the common repeats; a maximum of UINT32_MAX =>
infinity. */

static rep_min: [u32; 11] = [0, 0, 1, 1, 0, 0, 0, 0, 0, 1, 0];
static rep_max: [u32; 11] = [
    UINT32_MAX, UINT32_MAX, UINT32_MAX, UINT32_MAX, 1, 1, 0, 0, UINT32_MAX, UINT32_MAX, 1,
];
static rep_typ: [u32; 12] = [
    REPTYPE_MAX, REPTYPE_MIN, REPTYPE_MAX, REPTYPE_MIN, REPTYPE_MAX, REPTYPE_MIN, REPTYPE_MAX,
    REPTYPE_MIN, REPTYPE_POS, REPTYPE_POS, REPTYPE_POS, REPTYPE_POS,
];

/* Define short names for general fields in the current backtrack frame, which
is always pointed to by the F variable. These are macros that expand to
`(*F).field`, matching the C `Fxxx` field #defines. */

macro_rules! Fback_frame { ($F:expr) => { (*$F).back_frame } }
macro_rules! Fcapture_last { ($F:expr) => { (*$F).capture_last } }
macro_rules! Fcurrent_recurse { ($F:expr) => { (*$F).current_recurse } }
macro_rules! Fecode { ($F:expr) => { (*$F).ecode } }
macro_rules! Feptr { ($F:expr) => { (*$F).eptr } }
macro_rules! Fgroup_frame_type { ($F:expr) => { (*$F).group_frame_type } }
macro_rules! Flast_group_offset { ($F:expr) => { (*$F).last_group_offset } }
macro_rules! Fmark { ($F:expr) => { (*$F).mark } }
macro_rules! Frdepth { ($F:expr) => { (*$F).rdepth } }
macro_rules! Fstart_match { ($F:expr) => { (*$F).start_match } }
macro_rules! Foffset_top { ($F:expr) => { (*$F).offset_top } }
macro_rules! Fop { ($F:expr) => { (*$F).op } }
/// `Fovector` -- pointer to the frame's flexible ovector array.
macro_rules! Fovector { ($F:expr) => { (&raw mut (*$F).ovector) as *mut PCRE2_SIZE } }
macro_rules! Freturn_id { ($F:expr) => { (*$F).return_id } }

/* PRIV(OP_lengths) access */
#[inline]
unsafe fn op_length(op: u8) -> usize {
    OP_LENGTHS[op as usize] as usize
}

/* HSPACE_CASES / VSPACE_CASES membership. */
#[inline]
fn is_hspace(c: u32) -> bool {
    matches!(
        c,
        0x09 | 0x20
            | 0xa0
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
    matches!(c, 0x0a | 0x0b | 0x0c | 0x0d | 0x85 | 0x2028 | 0x2029)
}

/*************************************************
*                Process a callout               *
*************************************************/

/* This function is called for all callouts, whether "standalone" or at the
start of a conditional group. Feptr will be pointing to either OP_CALLOUT or
OP_CALLOUT_STR. */

unsafe fn do_callout(
    F: *mut heapframe,
    mb: *mut match_block,
    lengthptr: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
        let rc: c_int;
        let save0: PCRE2_SIZE;
        let save1: PCRE2_SIZE;

        *lengthptr = if *Fecode!(F) == OP_CALLOUT {
            op_length(OP_CALLOUT) as PCRE2_SIZE
        } else {
            get(Fecode!(F), 1 + 2 * LINK_SIZE) as PCRE2_SIZE
        };

        if (*mb).callout.is_none() {
            return 0; /* No callout function provided */
        }

        /* Picky compilers complain about Fovector[-2] directly, so set up a
        separate pointer. */
        let callout_ovector: *mut PCRE2_SIZE = (Fovector!(F)).sub(2);

        let cb: *mut pcre2_callout_block = (*mb).cb;
        (*cb).capture_top = (Foffset_top!(F) as u32) / 2 + 1;
        (*cb).capture_last = Fcapture_last!(F);
        (*cb).offset_vector = callout_ovector;
        (*cb).mark = (*mb).nomatch_mark;
        (*cb).current_position = (Feptr!(F).offset_from((*mb).start_subject)) as PCRE2_SIZE;
        (*cb).pattern_position = get(Fecode!(F), 1) as PCRE2_SIZE;
        (*cb).next_item_length = get(Fecode!(F), 1 + LINK_SIZE) as PCRE2_SIZE;

        if *Fecode!(F) == OP_CALLOUT {
            /* Numerical callout */
            (*cb).callout_number = *Fecode!(F).add(1 + 2 * LINK_SIZE) as u32;
            (*cb).callout_string_offset = 0;
            (*cb).callout_string = core::ptr::null();
            (*cb).callout_string_length = 0;
        } else {
            /* String callout */
            (*cb).callout_number = 0;
            (*cb).callout_string_offset = get(Fecode!(F), 1 + 3 * LINK_SIZE) as PCRE2_SIZE;
            (*cb).callout_string = Fecode!(F).add(1 + 4 * LINK_SIZE).add(1);
            (*cb).callout_string_length = *lengthptr - (1 + 4 * LINK_SIZE) - 2;
        }

        save0 = *callout_ovector.add(0);
        save1 = *callout_ovector.add(1);
        *callout_ovector.add(0) = PCRE2_UNSET;
        *callout_ovector.add(1) = PCRE2_UNSET;
        rc = ((*mb).callout.unwrap())(cb, (*mb).callout_data);
        *callout_ovector.add(0) = save0;
        *callout_ovector.add(1) = save1;
        (*cb).callout_flags = 0;
        rc
    }
}

/*************************************************
*          Match a back-reference                *
*************************************************/

/* Returns:  = 0 successful match; number of code units matched is set
             < 0 no match
             > 0 partial match */

unsafe fn match_ref(
    offset: PCRE2_SIZE,
    caseless: BOOL,
    caseopts: c_int,
    F: *mut heapframe,
    mb: *mut match_block,
    lengthptr: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
        let mut p: PCRE2_SPTR;
        let mut length: PCRE2_SIZE;
        let mut eptr: PCRE2_SPTR;
        let eptr_start: PCRE2_SPTR;

        /* Deal with an unset group. */
        if offset >= Foffset_top!(F) || *Fovector!(F).add(offset) == PCRE2_UNSET {
            if ((*mb).poptions & PCRE2_MATCH_UNSET_BACKREF) != 0 {
                *lengthptr = 0;
                return 0; /* Match */
            } else {
                return -1; /* No match */
            }
        }

        eptr = Feptr!(F);
        eptr_start = eptr;
        p = (*mb).start_subject.add(*Fovector!(F).add(offset));
        length = *Fovector!(F).add(offset + 1) - *Fovector!(F).add(offset);

        if caseless != FALSE {
            let utf: BOOL = (((*mb).poptions & PCRE2_UTF) != 0) as BOOL;
            let caseless_restrict: BOOL =
                ((caseopts & REFI_FLAG_CASELESS_RESTRICT as c_int) != 0) as BOOL;
            let turkish_casing: BOOL = (caseless_restrict == FALSE
                && (caseopts & REFI_FLAG_TURKISH_CASING as c_int) != 0)
                as BOOL;

            if utf != FALSE || ((*mb).poptions & PCRE2_UCP) != 0 {
                let endptr: PCRE2_SPTR = p.add(length);

                while p < endptr {
                    let mut c: u32;
                    let d: u32;
                    if eptr >= (*mb).end_subject {
                        return 1; /* Partial match */
                    }

                    if utf != FALSE {
                        c = getcharinc(&mut eptr);
                        d = getcharinc(&mut p);
                    } else {
                        c = *eptr as u32;
                        eptr = eptr.add(1);
                        d = *p as u32;
                        p = p.add(1);
                    }

                    if turkish_casing != FALSE && ucd_any_i(d) {
                        c = ucd_fold_i_turkish(c);
                        let d2 = ucd_fold_i_turkish(d);
                        if c != d2 {
                            return -1; /* No match */
                        }
                    } else if c != d
                        && c != ((d as i32).wrapping_add(get_ucd(d).other_case) as u32)
                    {
                        let ur = get_ucd(d);
                        let mut pp = &UCD_CASELESS_SETS[ur.caseset as usize..];

                        if caseless_restrict != FALSE && pp[0] < 128 {
                            return -1; /* No match */
                        }

                        loop {
                            if c < pp[0] {
                                return -1; /* No match */
                            }
                            let v = pp[0];
                            pp = &pp[1..];
                            if c == v {
                                break;
                            }
                        }
                    }
                }
            } else {
                /* Not in UTF or UCP mode */
                while length > 0 {
                    let cc: u32;
                    let cp: u32;
                    if eptr >= (*mb).end_subject {
                        return 1; /* Partial match */
                    }
                    cc = *eptr as u32;
                    cp = *p as u32;
                    if table_get(cp, (*mb).lcc, cp) != table_get(cc, (*mb).lcc, cc) {
                        return -1; /* No match */
                    }
                    p = p.add(1);
                    eptr = eptr.add(1);
                    length -= 1;
                }
            }
        } else {
            /* Caseful case */
            if (*mb).partial != 0 {
                while length > 0 {
                    if eptr >= (*mb).end_subject {
                        return 1; /* Partial match */
                    }
                    let pc = *p;
                    p = p.add(1);
                    let ec = *eptr;
                    eptr = eptr.add(1);
                    if pc != ec {
                        return -1; /* No match */
                    }
                    length -= 1;
                }
            } else {
                if ((*mb).end_subject.offset_from(eptr) as PCRE2_SIZE) < length
                    || memcmp(p as *const c_void, eptr as *const c_void, cu2bytes(length)) != 0
                {
                    return -1; /* No match */
                }
                eptr = eptr.add(length);
            }
        }

        *lengthptr = eptr.offset_from(eptr_start) as PCRE2_SIZE;
        0 /* Match */
    }
}

/*************************************************
*     Restore offsets after a recurse            *
*************************************************/

unsafe fn recurse_update_offsets(F: *mut heapframe, P: *mut heapframe) {
    unsafe {
        let mut dst: *mut PCRE2_SIZE = Fovector!(F);
        let mut src: *mut PCRE2_SIZE = Fovector!(P);
        let mut offset: PCRE2_SIZE = 2;
        let offset_top: PCRE2_SIZE = Foffset_top!(F) + 2;
        let mut diff: PCRE2_SIZE;
        let mut ecode: PCRE2_SPTR = Fecode!(F);

        loop {
            diff = ((get2(ecode, 1) << 1) as PCRE2_SIZE).wrapping_sub(offset);
            ecode = ecode.add(1 + IMM2_SIZE);

            if offset + diff >= offset_top {
                /* Some OP_CREF opcodes are not processed, they must be skipped. */
                while *ecode == OP_CREF {
                    ecode = ecode.add(1 + IMM2_SIZE);
                }
                break;
            }

            if diff == 2 {
                *dst.add(0) = *src.add(0);
                *dst.add(1) = *src.add(1);
            } else if diff >= 4 {
                memcpy(dst, src, diff);
            }

            /* Skip the unmodified entry. */
            diff += 2;
            offset += diff;
            dst = dst.add(diff);
            src = src.add(diff);

            if *ecode != OP_CREF {
                break;
            }
        }

        diff = offset_top - offset;
        if diff == 2 {
            *dst.add(0) = *src.add(0);
            *dst.add(1) = *src.add(1);
        } else if diff >= 4 {
            memcpy(dst, src, diff);
        }

        Fecode!(F) = ecode;
        Foffset_top!(F) = if offset <= (*P).offset_top {
            (*P).offset_top
        } else {
            offset - 2
        };
    }
}

/*************************************************
*         Match from current position            *
*************************************************/

/* State labels for the interpreter goto machinery. ST_L<n> are the resume
points immediately after the corresponding RMATCH(..., RM<n>) call. */

const ST_MATCH_RECURSE: u32 = 0;
const ST_NEW_FRAME: u32 = 1;
const ST_MAIN_LOOP: u32 = 2;
const ST_RETURN_SWITCH: u32 = 3;
/* Resume labels are encoded as ST_LBASE + return_id. */
const ST_LBASE: u32 = 4;

const ST_L1: u32 = ST_LBASE + 1;
const ST_L2: u32 = ST_LBASE + 2;
const ST_L3: u32 = ST_LBASE + 3;
const ST_L4: u32 = ST_LBASE + 4;
const ST_L5: u32 = ST_LBASE + 5;
const ST_L6: u32 = ST_LBASE + 6;
const ST_L7: u32 = ST_LBASE + 7;
const ST_L8: u32 = ST_LBASE + 8;
const ST_L9: u32 = ST_LBASE + 9;
const ST_L10: u32 = ST_LBASE + 10;
const ST_L11: u32 = ST_LBASE + 11;
const ST_L12: u32 = ST_LBASE + 12;
const ST_L13: u32 = ST_LBASE + 13;
const ST_L14: u32 = ST_LBASE + 14;
const ST_L15: u32 = ST_LBASE + 15;
const ST_L16: u32 = ST_LBASE + 16;
const ST_L17: u32 = ST_LBASE + 17;
const ST_L18: u32 = ST_LBASE + 18;
const ST_L19: u32 = ST_LBASE + 19;
const ST_L20: u32 = ST_LBASE + 20;
const ST_L21: u32 = ST_LBASE + 21;
const ST_L22: u32 = ST_LBASE + 22;
const ST_L23: u32 = ST_LBASE + 23;
const ST_L24: u32 = ST_LBASE + 24;
const ST_L25: u32 = ST_LBASE + 25;
const ST_L26: u32 = ST_LBASE + 26;
const ST_L27: u32 = ST_LBASE + 27;
const ST_L28: u32 = ST_LBASE + 28;
const ST_L29: u32 = ST_LBASE + 29;
const ST_L30: u32 = ST_LBASE + 30;
const ST_L31: u32 = ST_LBASE + 31;
const ST_L32: u32 = ST_LBASE + 32;
const ST_L33: u32 = ST_LBASE + 33;
const ST_L34: u32 = ST_LBASE + 34;
const ST_L35: u32 = ST_LBASE + 35;
const ST_L36: u32 = ST_LBASE + 36;
const ST_L37: u32 = ST_LBASE + 37;
const ST_L38: u32 = ST_LBASE + 38;
const ST_L39: u32 = ST_LBASE + 39;
const ST_L100: u32 = ST_LBASE + 100;
const ST_L101: u32 = ST_LBASE + 101;
const ST_L102: u32 = ST_LBASE + 102;
const ST_L103: u32 = ST_LBASE + 103;
const ST_L200: u32 = ST_LBASE + 200;
const ST_L201: u32 = ST_LBASE + 201;
const ST_L202: u32 = ST_LBASE + 202;
const ST_L203: u32 = ST_LBASE + 203;
const ST_L204: u32 = ST_LBASE + 204;
const ST_L205: u32 = ST_LBASE + 205;
const ST_L206: u32 = ST_LBASE + 206;
const ST_L207: u32 = ST_LBASE + 207;
const ST_L208: u32 = ST_LBASE + 208;
const ST_L209: u32 = ST_LBASE + 209;
const ST_L210: u32 = ST_LBASE + 210;
const ST_L211: u32 = ST_LBASE + 211;
const ST_L212: u32 = ST_LBASE + 212;
const ST_L213: u32 = ST_LBASE + 213;
const ST_L214: u32 = ST_LBASE + 214;
const ST_L215: u32 = ST_LBASE + 215;
const ST_L216: u32 = ST_LBASE + 216;
const ST_L217: u32 = ST_LBASE + 217;
const ST_L218: u32 = ST_LBASE + 218;
const ST_L219: u32 = ST_LBASE + 219;
const ST_L220: u32 = ST_LBASE + 220;
const ST_L221: u32 = ST_LBASE + 221;
const ST_L222: u32 = ST_LBASE + 222;
const ST_L223: u32 = ST_LBASE + 223;
const ST_L224: u32 = ST_LBASE + 224;

/* Internal goto-label states (targets of C `goto` other than RMATCH resume). */
const ST_REPEATCHAR: u32 = 1000;
const ST_REPEATNOTCHAR: u32 = 1001;
const ST_REPEATTYPE: u32 = 1002;
const ST_REF_REPEAT: u32 = 1003;
const ST_GROUPLOOP: u32 = 1004;
const ST_POSSESSIVE_NON_CAPTURE: u32 = 1005;
const ST_POSSESSIVE_CAPTURE: u32 = 1006;
const ST_POSSESSIVE_GROUP: u32 = 1007;
const ST_ASSERT_NOT_FAILED: u32 = 1008;
const ST_ASSERT_NL_OR_EOS: u32 = 1009;
/* Extra internal states used by this translation to represent C `goto`
targets and multi-branch loops that have no dedicated C label. */
const ST_L_BRA_LOOP: u32 = 1010; /* OP_BRA THEN-free branch loop */
const ST_KET: u32 = 1011; /* OP_KET/KETRMIN/KETRMAX/KETRPOS body */
const ST_REPEATTYPE_MIN: u32 = 1012; /* REPEATTYPE minimizing dispatch */
const ST_REPEATTYPE_MAX: u32 = 1013; /* REPEATTYPE maximizing dispatch */

/* Returns:  MATCH_MATCH / MATCH_NOMATCH / negative MATCH_xxx / negative error. */

unsafe fn r#match(
    start_eptr: PCRE2_SPTR,
    start_ecode0: PCRE2_SPTR,
    top_bracket: u16,
    frame_size: PCRE2_SIZE,
    match_data: *mut pcre2_real_match_data,
    mb: *mut match_block,
) -> c_int {
    unsafe {
        /* Frame-handling variables */
        let mut F: *mut heapframe;
        let mut N: *mut heapframe = core::ptr::null_mut();
        let mut P: *mut heapframe = core::ptr::null_mut();
        let mut frames_top: *mut heapframe;
        let mut assert_accept_frame: *mut heapframe = core::ptr::null_mut();
        let frame_copy_size: PCRE2_SIZE;

        /* Local variables that do not need to be preserved over RMATCH(). */
        let mut branch_end: PCRE2_SPTR = core::ptr::null();
        let mut branch_start: PCRE2_SPTR;
        let mut bracode: PCRE2_SPTR;
        let mut offset: PCRE2_SIZE = 0;
        let mut length: PCRE2_SIZE = 0;

        let mut rrc: c_int = 0;
        let mut proptype: c_int = 0;

        let mut i: u32;
        let mut fc: u32;
        let mut number: u32;
        let mut reptype: u32 = 0;
        let mut group_frame_type: u32;

        let mut condition: BOOL;
        let mut cur_is_word: BOOL;
        let mut prev_is_word: BOOL;

        let utf: BOOL = (((*mb).poptions & PCRE2_UTF) != 0) as BOOL;
        let ucp: BOOL = (((*mb).poptions & PCRE2_UCP) != 0) as BOOL;

        /* Value that RMATCH passes to MATCH_RECURSE. */
        let mut start_ecode: PCRE2_SPTR;

        frame_copy_size = frame_size - core::mem::offset_of!(heapframe, eptr);

        /* Set up the first frame and the end of the frames vector. */
        F = (*match_data).heapframes;
        frames_top = (F as *mut u8).add((*match_data).heapframes_size) as *mut heapframe;

        Frdepth!(F) = 0;
        Fcapture_last!(F) = 0;
        Fcurrent_recurse!(F) = RECURSE_UNSET;
        Fstart_match!(F) = start_eptr;
        Feptr!(F) = start_eptr;
        Fmark!(F) = core::ptr::null();
        Foffset_top!(F) = 0;
        Flast_group_offset!(F) = PCRE2_UNSET;
        group_frame_type = 0;

        /* State machine for the interpreter's goto-based control flow. The
        loop label must be introduced before the RMATCH/RRETURN macros are
        defined, because Rust label hygiene resolves labels used inside a
        `macro_rules!` body at the definition site. */
        let mut state: u32 = ST_NEW_FRAME;
        start_ecode = start_ecode0;

        'dispatch: loop {

        /* --- Local macros implementing the goto machinery and partial-match /
        newline helpers. They refer to the local variables by name. --- */

        /* RMATCH(ra, rb): remember the resume label, set up a new frame, and
        jump to MATCH_RECURSE. Control resumes at state ST_L<rb> after the
        RRETURN that unwinds back to this frame. */
        macro_rules! RMATCH {
            ($ra:expr, $rb:expr) => {{
                start_ecode = $ra;
                Freturn_id!(F) = $rb as u8;
                state = ST_MATCH_RECURSE;
                continue 'dispatch;
            }};
        }

        /* RRETURN(ra): jump to RETURN_SWITCH with rrc set. */
        macro_rules! RRETURN {
            ($ra:expr) => {{
                rrc = $ra;
                state = ST_RETURN_SWITCH;
                continue 'dispatch;
            }};
        }

        /* SCHECK_PARTIAL / CHECK_PARTIAL. SCHECK_PARTIAL may return
        PCRE2_ERROR_PARTIAL directly from the function. */
        macro_rules! SCHECK_PARTIAL {
            () => {{
                if (*mb).partial != 0
                    && (Feptr!(F) > (*mb).start_used_ptr || (*mb).allowemptypartial != FALSE)
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
            }};
        }
        macro_rules! CHECK_PARTIAL {
            () => {{
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                }
            }};
        }

        /* IS_NEWLINE(p): NLBLOCK is mb, PSEND is end_subject. */
        macro_rules! IS_NEWLINE {
            ($p:expr) => {{
                if (*mb).nltype != NLTYPE_FIXED {
                    $p < (*mb).end_subject
                        && is_newline(
                            $p,
                            (*mb).nltype,
                            (*mb).end_subject,
                            &raw mut (*mb).nllen,
                            utf,
                        ) != FALSE
                } else {
                    $p <= (*mb).end_subject.sub((*mb).nllen as usize)
                        && *$p as u32 == (*mb).nl[0] as u32
                        && ((*mb).nllen == 1 || *$p.add(1) as u32 == (*mb).nl[1] as u32)
                }
            }};
        }

        /* WAS_NEWLINE(p): PSSTART is start_subject. */
        macro_rules! WAS_NEWLINE {
            ($p:expr) => {{
                if (*mb).nltype != NLTYPE_FIXED {
                    $p > (*mb).start_subject
                        && was_newline(
                            $p,
                            (*mb).nltype,
                            (*mb).start_subject,
                            &raw mut (*mb).nllen,
                            utf,
                        ) != FALSE
                } else {
                    $p >= (*mb).start_subject.add((*mb).nllen as usize)
                        && *$p.sub((*mb).nllen as usize) as u32 == (*mb).nl[0] as u32
                        && ((*mb).nllen == 1
                            || *$p.sub((*mb).nllen as usize).add(1) as u32 == (*mb).nl[1] as u32)
                }
            }};
        }

        /* ACROSSCHAR(condition, eptr, action): advance over UTF continuation
        bytes while condition holds. The C form passes `Feptr++` as the action;
        we take the pointer place and advance it. */
        macro_rules! ACROSSCHAR {
            ($cond:expr, $eptr:expr) => {{
                while ($cond) && (*$eptr & 0xc0u8) == 0x80u8 {
                    $eptr = $eptr.add(1);
                }
            }};
        }

        /* State machine for the interpreter's goto-based control flow. */
        match state {

            /* ---- MATCH_RECURSE: set up a new backtracking frame. ---- */
            ST_MATCH_RECURSE => {
                N = (F as *mut u8).add(frame_size) as *mut heapframe;
                if ((N as *mut u8).add(frame_size) as *mut heapframe) >= frames_top {
                    let new_frames: *mut heapframe;
                    let mut newsize: PCRE2_SIZE;
                    let usedsize: PCRE2_SIZE =
                        (N as *mut u8).offset_from((*match_data).heapframes as *mut u8) as PCRE2_SIZE;

                    if (*match_data).heapframes_size >= PCRE2_SIZE_MAX / 2 {
                        if (*match_data).heapframes_size == PCRE2_SIZE_MAX - 1 {
                            return PCRE2_ERROR_NOMEMORY;
                        }
                        newsize = PCRE2_SIZE_MAX - 1;
                    } else {
                        newsize = (*match_data).heapframes_size * 2;
                    }

                    if newsize / 1024 >= (*mb).heap_limit as PCRE2_SIZE {
                        let old_size: PCRE2_SIZE = (*match_data).heapframes_size / 1024;
                        if ((*mb).heap_limit as PCRE2_SIZE) <= old_size {
                            return PCRE2_ERROR_HEAPLIMIT;
                        } else {
                            let mut max_delta: PCRE2_SIZE =
                                1024 * ((*mb).heap_limit as PCRE2_SIZE - old_size);
                            let over_bytes: c_int =
                                ((*match_data).heapframes_size % 1024) as c_int;
                            if over_bytes != 0 {
                                max_delta -= 1024 - over_bytes as PCRE2_SIZE;
                            }
                            newsize = (*match_data).heapframes_size + max_delta;
                        }
                    }

                    if newsize - usedsize < frame_size {
                        return PCRE2_ERROR_HEAPLIMIT;
                    }
                    new_frames = ((*match_data).memctl.malloc.unwrap())(
                        newsize,
                        (*match_data).memctl.memory_data,
                    ) as *mut heapframe;
                    if new_frames.is_null() {
                        return PCRE2_ERROR_NOMEMORY;
                    }
                    memcpy(
                        new_frames as *mut u8,
                        (*match_data).heapframes as *const u8,
                        usedsize,
                    );

                    N = (new_frames as *mut u8).add(usedsize) as *mut heapframe;
                    F = (N as *mut u8).sub(frame_size) as *mut heapframe;

                    ((*match_data).memctl.free.unwrap())(
                        (*match_data).heapframes as *mut c_void,
                        (*match_data).memctl.memory_data,
                    );
                    (*match_data).heapframes = new_frames;
                    (*match_data).heapframes_size = newsize;
                    frames_top = (new_frames as *mut u8).add(newsize) as *mut heapframe;
                }

                /* Copy those fields that must be copied into the new frame, then
                increase the "recursion" depth and make the new frame current. */
                memcpy(
                    (N as *mut u8).add(core::mem::offset_of!(heapframe, eptr)),
                    (F as *const u8).add(core::mem::offset_of!(heapframe, eptr)),
                    frame_copy_size,
                );
                (*N).rdepth = Frdepth!(F) + 1;
                F = N;
                state = ST_NEW_FRAME;
                continue 'dispatch;
            }

            /* ---- NEW_FRAME: begin processing with the current frame. ---- */
            ST_NEW_FRAME => {
                Fgroup_frame_type!(F) = group_frame_type;
                Fecode!(F) = start_ecode;
                Fback_frame!(F) = frame_size;

                if group_frame_type != 0 {
                    Flast_group_offset!(F) =
                        (F as *mut u8).offset_from((*match_data).heapframes as *mut u8)
                            as PCRE2_SIZE;
                    if GF_IDMASK(group_frame_type) == GF_RECURSE {
                        Fcurrent_recurse!(F) = GF_DATAMASK(group_frame_type);
                    }
                    group_frame_type = 0;
                }

                /* Check limits before processing the opcodes. */
                if {
                    let v = (*mb).match_call_count;
                    (*mb).match_call_count = v + 1;
                    v
                } >= (*mb).match_limit
                {
                    return PCRE2_ERROR_MATCHLIMIT;
                }
                if Frdepth!(F) >= (*mb).match_limit_depth {
                    return PCRE2_ERROR_DEPTHLIMIT;
                }

                state = ST_MAIN_LOOP;
                continue 'dispatch;
            }

            /* ---- MAIN_LOOP: process the opcode at Fecode. ---- */
            ST_MAIN_LOOP => {
                Fop!(F) = *Fecode!(F);
                match Fop!(F) {
                    /* Before OP_ACCEPT there may be OP_CLOSE opcodes. */
                    OP_CLOSE => {
                        if Fcurrent_recurse!(F) == RECURSE_UNSET {
                            number = get2(Fecode!(F), 1);
                            offset = Flast_group_offset!(F);
                            loop {
                                if offset == PCRE2_UNSET {
                                    return PCRE2_ERROR_INTERNAL;
                                }
                                N = ((*match_data).heapframes as *mut u8).add(offset)
                                    as *mut heapframe;
                                P = (N as *mut u8).sub(frame_size) as *mut heapframe;
                                if (*N).group_frame_type == (GF_CAPTURE | number) {
                                    break;
                                }
                                offset = (*P).last_group_offset;
                            }
                            offset = ((number << 1) - 2) as PCRE2_SIZE;
                            Fcapture_last!(F) = number;
                            *Fovector!(F).add(offset) =
                                (*P).eptr.offset_from((*mb).start_subject) as PCRE2_SIZE;
                            *Fovector!(F).add(offset + 1) =
                                Feptr!(F).offset_from((*mb).start_subject) as PCRE2_SIZE;
                            if offset >= Foffset_top!(F) {
                                Foffset_top!(F) = offset + 2;
                            }
                        }
                        Fecode!(F) = Fecode!(F).add(op_length(*Fecode!(F)));
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    OP_ASSERT_ACCEPT => {
                        if Feptr!(F) > (*mb).last_used_ptr {
                            (*mb).last_used_ptr = Feptr!(F);
                        }
                        assert_accept_frame = F;
                        RRETURN!(MATCH_ACCEPT);
                    }

                    OP_ACCEPT | OP_END => {
                        if Fop!(F) == OP_ACCEPT && Fcurrent_recurse!(F) != RECURSE_UNSET {
                            offset = Flast_group_offset!(F);
                            loop {
                                if offset == PCRE2_UNSET {
                                    return PCRE2_ERROR_INTERNAL;
                                }
                                N = ((*match_data).heapframes as *mut u8).add(offset)
                                    as *mut heapframe;
                                P = (N as *mut u8).sub(frame_size) as *mut heapframe;
                                if GF_IDMASK((*N).group_frame_type) == GF_RECURSE {
                                    break;
                                }
                                offset = (*P).last_group_offset;
                            }
                            (*P).eptr = Feptr!(F);
                            (*P).mark = Fmark!(F);
                            (*P).start_match = Fstart_match!(F);
                            F = P;
                            Fecode!(F) = Fecode!(F).add(1 + LINK_SIZE);
                            state = ST_MAIN_LOOP;
                            continue 'dispatch;
                        }

                        /* Common OP_END / (*ACCEPT) not in recursion. */
                        if Feptr!(F) == Fstart_match!(F)
                            && (((*mb).moptions & PCRE2_NOTEMPTY) != 0
                                || (((*mb).moptions & PCRE2_NOTEMPTY_ATSTART) != 0
                                    && Fstart_match!(F)
                                        == (*mb).start_subject.add((*mb).start_offset)))
                        {
                            RRETURN!(MATCH_NOMATCH);
                        }

                        if Feptr!(F) < (*mb).end_subject
                            && (((*mb).moptions | (*mb).poptions) & PCRE2_ENDANCHORED) != 0
                        {
                            if Fop!(F) == OP_END {
                                RRETURN!(MATCH_NOMATCH);
                            }
                            return MATCH_NOMATCH; /* (*ACCEPT) */
                        }

                        if Fstart_match!(F) < (*mb).start_subject.add((*mb).start_offset)
                            || Fstart_match!(F) > Feptr!(F)
                        {
                            if (*mb).allowlookaroundbsk == FALSE {
                                return PCRE2_ERROR_BAD_BACKSLASH_K;
                            }
                        }

                        (*mb).end_match_ptr = Feptr!(F);
                        (*mb).end_offset_top = Foffset_top!(F);
                        (*mb).mark = Fmark!(F);
                        if Feptr!(F) > (*mb).last_used_ptr {
                            (*mb).last_used_ptr = Feptr!(F);
                        }

                        *(*match_data).ovector.as_mut_ptr().add(0) =
                            Fstart_match!(F).offset_from((*mb).start_subject) as PCRE2_SIZE;
                        *(*match_data).ovector.as_mut_ptr().add(1) =
                            Feptr!(F).offset_from((*mb).start_subject) as PCRE2_SIZE;

                        i = 2 * (if (top_bracket as u32 + 1) > (*match_data).oveccount as u32 {
                            (*match_data).oveccount as u32
                        } else {
                            top_bracket as u32 + 1
                        });
                        memcpy(
                            (*match_data).ovector.as_mut_ptr().add(2),
                            Fovector!(F),
                            (i as usize - 2),
                        );
                        loop {
                            i -= 1;
                            if !(i >= Foffset_top!(F) as u32 + 2) {
                                break;
                            }
                            *(*match_data).ovector.as_mut_ptr().add(i as usize) = PCRE2_UNSET;
                        }
                        return MATCH_MATCH; /* Note: NOT RRETURN */
                    }

                    /* Match any single character type except newline. */
                    OP_ANY | OP_ALLANY => {
                        if Fop!(F) == OP_ANY {
                            if IS_NEWLINE!(Feptr!(F)) {
                                RRETURN!(MATCH_NOMATCH);
                            }
                            if (*mb).partial != 0
                                && Feptr!(F) == (*mb).end_subject.sub(1)
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && *Feptr!(F) as u32 == (*mb).nl[0] as u32
                            {
                                (*mb).hitend = TRUE;
                                if (*mb).partial > 1 {
                                    return PCRE2_ERROR_PARTIAL;
                                }
                            }
                        }
                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        Feptr!(F) = Feptr!(F).add(1);
                        if utf != FALSE {
                            ACROSSCHAR!(Feptr!(F) < (*mb).end_subject, Feptr!(F));
                        }
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    OP_ANYBYTE => {
                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        Feptr!(F) = Feptr!(F).add(1);
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    OP_CHAR => {
                        if utf != FALSE {
                            length = 1;
                            Fecode!(F) = Fecode!(F).add(1);
                            let (ch, extra) = getcharlen(Fecode!(F));
                            fc = ch;
                            length += extra as PCRE2_SIZE;
                            if length > ((*mb).end_subject.offset_from(Feptr!(F)) as PCRE2_SIZE) {
                                CHECK_PARTIAL!();
                                RRETURN!(MATCH_NOMATCH);
                            }
                            while length > 0 {
                                let ec = *Fecode!(F);
                                Fecode!(F) = Fecode!(F).add(1);
                                let sc = *Feptr!(F);
                                Feptr!(F) = Feptr!(F).add(1);
                                if ec != sc {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                length -= 1;
                            }
                        } else {
                            if (*mb).end_subject.offset_from(Feptr!(F)) < 1 {
                                SCHECK_PARTIAL!();
                                RRETURN!(MATCH_NOMATCH);
                            }
                            let sc = *Feptr!(F);
                            Feptr!(F) = Feptr!(F).add(1);
                            if *Fecode!(F).add(1) != sc {
                                RRETURN!(MATCH_NOMATCH);
                            }
                            Fecode!(F) = Fecode!(F).add(2);
                        }
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    OP_CHARI => {
                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        if utf != FALSE {
                            length = 1;
                            Fecode!(F) = Fecode!(F).add(1);
                            let (ch, extra) = getcharlen(Fecode!(F));
                            fc = ch;
                            length += extra as PCRE2_SIZE;
                            if fc < 128 {
                                let cc = *Feptr!(F) as u32;
                                if *(*mb).lcc.add(fc as usize) as u32 != table_get(cc, (*mb).lcc, cc)
                                {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                Fecode!(F) = Fecode!(F).add(1);
                                Feptr!(F) = Feptr!(F).add(1);
                            } else {
                                let dc = getcharinc(&mut Feptr!(F));
                                Fecode!(F) = Fecode!(F).add(length);
                                if dc != fc && dc != ucd_othercase(fc) {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                            }
                        } else if ucp != FALSE {
                            let cc = *Feptr!(F) as u32;
                            fc = *Fecode!(F).add(1) as u32;
                            if fc < 128 {
                                if *(*mb).lcc.add(fc as usize) as u32
                                    != table_get(cc, (*mb).lcc, cc)
                                {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                            } else {
                                if cc != fc && cc != ucd_othercase(fc) {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                            }
                            Feptr!(F) = Feptr!(F).add(1);
                            Fecode!(F) = Fecode!(F).add(2);
                        } else {
                            let e1 = *Fecode!(F).add(1) as u32;
                            let s1 = *Feptr!(F) as u32;
                            if table_get(e1, (*mb).lcc, e1) != table_get(s1, (*mb).lcc, s1) {
                                RRETURN!(MATCH_NOMATCH);
                            }
                            Feptr!(F) = Feptr!(F).add(1);
                            Fecode!(F) = Fecode!(F).add(2);
                        }
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    OP_NOT | OP_NOTI => {
                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        if utf != FALSE {
                            let mut ch: u32;
                            Fecode!(F) = Fecode!(F).add(1);
                            ch = getcharinc(&mut Fecode!(F));
                            fc = getcharinc(&mut Feptr!(F));
                            if ch == fc {
                                RRETURN!(MATCH_NOMATCH);
                            } else if Fop!(F) == OP_NOTI {
                                if ch > 127 {
                                    ch = ucd_othercase(ch);
                                } else {
                                    ch = *(*mb).fcc.add(ch as usize) as u32;
                                }
                                if ch == fc {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                            }
                        } else if ucp != FALSE {
                            let mut ch: u32;
                            fc = *Feptr!(F) as u32;
                            Feptr!(F) = Feptr!(F).add(1);
                            ch = *Fecode!(F).add(1) as u32;
                            Fecode!(F) = Fecode!(F).add(2);
                            if ch == fc {
                                RRETURN!(MATCH_NOMATCH);
                            } else if Fop!(F) == OP_NOTI {
                                if ch > 127 {
                                    ch = ucd_othercase(ch);
                                } else {
                                    ch = *(*mb).fcc.add(ch as usize) as u32;
                                }
                                if ch == fc {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                            }
                        } else {
                            let ch = *Fecode!(F).add(1) as u32;
                            fc = *Feptr!(F) as u32;
                            Feptr!(F) = Feptr!(F).add(1);
                            if ch == fc
                                || (Fop!(F) == OP_NOTI && table_get(ch, (*mb).fcc, ch) == fc)
                            {
                                RRETURN!(MATCH_NOMATCH);
                            }
                            Fecode!(F) = Fecode!(F).add(2);
                        }
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    /* ---- Match a single character repeatedly. ---- */
                    OP_EXACT | OP_EXACTI => {
                        let m = get2(Fecode!(F), 1);
                        (*F).fields.char_repeat.min = m;
                        (*F).fields.char_repeat.max = m;
                        Fecode!(F) = Fecode!(F).add(1 + IMM2_SIZE);
                        state = ST_REPEATCHAR;
                        continue 'dispatch;
                    }
                    OP_POSUPTO | OP_POSUPTOI => {
                        reptype = REPTYPE_POS;
                        (*F).fields.char_repeat.min = 0;
                        (*F).fields.char_repeat.max = get2(Fecode!(F), 1);
                        Fecode!(F) = Fecode!(F).add(1 + IMM2_SIZE);
                        state = ST_REPEATCHAR;
                        continue 'dispatch;
                    }
                    OP_UPTO | OP_UPTOI => {
                        reptype = REPTYPE_MAX;
                        (*F).fields.char_repeat.min = 0;
                        (*F).fields.char_repeat.max = get2(Fecode!(F), 1);
                        Fecode!(F) = Fecode!(F).add(1 + IMM2_SIZE);
                        state = ST_REPEATCHAR;
                        continue 'dispatch;
                    }
                    OP_MINUPTO | OP_MINUPTOI => {
                        reptype = REPTYPE_MIN;
                        (*F).fields.char_repeat.min = 0;
                        (*F).fields.char_repeat.max = get2(Fecode!(F), 1);
                        Fecode!(F) = Fecode!(F).add(1 + IMM2_SIZE);
                        state = ST_REPEATCHAR;
                        continue 'dispatch;
                    }
                    OP_POSSTAR | OP_POSSTARI => {
                        reptype = REPTYPE_POS;
                        (*F).fields.char_repeat.min = 0;
                        (*F).fields.char_repeat.max = UINT32_MAX;
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_REPEATCHAR;
                        continue 'dispatch;
                    }
                    OP_POSPLUS | OP_POSPLUSI => {
                        reptype = REPTYPE_POS;
                        (*F).fields.char_repeat.min = 1;
                        (*F).fields.char_repeat.max = UINT32_MAX;
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_REPEATCHAR;
                        continue 'dispatch;
                    }
                    OP_POSQUERY | OP_POSQUERYI => {
                        reptype = REPTYPE_POS;
                        (*F).fields.char_repeat.min = 0;
                        (*F).fields.char_repeat.max = 1;
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_REPEATCHAR;
                        continue 'dispatch;
                    }
                    OP_STAR | OP_STARI | OP_MINSTAR | OP_MINSTARI | OP_PLUS | OP_PLUSI
                    | OP_MINPLUS | OP_MINPLUSI | OP_QUERY | OP_QUERYI | OP_MINQUERY
                    | OP_MINQUERYI => {
                        fc = (*Fecode!(F)
                            - (if Fop!(F) < OP_STARI { OP_STAR } else { OP_STARI }))
                            as u32;
                        Fecode!(F) = Fecode!(F).add(1);
                        (*F).fields.char_repeat.min = rep_min[fc as usize];
                        (*F).fields.char_repeat.max = rep_max[fc as usize];
                        reptype = rep_typ[fc as usize];
                        state = ST_REPEATCHAR;
                        continue 'dispatch;
                    }

                    /* ---- Match a negated single character repeatedly. ---- */
                    OP_NOTEXACT | OP_NOTEXACTI => {
                        let m = get2(Fecode!(F), 1);
                        (*F).fields.charnot_repeat.min = m;
                        (*F).fields.charnot_repeat.max = m;
                        Fecode!(F) = Fecode!(F).add(1 + IMM2_SIZE);
                        state = ST_REPEATNOTCHAR;
                        continue 'dispatch;
                    }
                    OP_NOTUPTO | OP_NOTUPTOI => {
                        (*F).fields.charnot_repeat.min = 0;
                        (*F).fields.charnot_repeat.max = get2(Fecode!(F), 1);
                        reptype = REPTYPE_MAX;
                        Fecode!(F) = Fecode!(F).add(1 + IMM2_SIZE);
                        state = ST_REPEATNOTCHAR;
                        continue 'dispatch;
                    }
                    OP_NOTMINUPTO | OP_NOTMINUPTOI => {
                        (*F).fields.charnot_repeat.min = 0;
                        (*F).fields.charnot_repeat.max = get2(Fecode!(F), 1);
                        reptype = REPTYPE_MIN;
                        Fecode!(F) = Fecode!(F).add(1 + IMM2_SIZE);
                        state = ST_REPEATNOTCHAR;
                        continue 'dispatch;
                    }
                    OP_NOTPOSSTAR | OP_NOTPOSSTARI => {
                        reptype = REPTYPE_POS;
                        (*F).fields.charnot_repeat.min = 0;
                        (*F).fields.charnot_repeat.max = UINT32_MAX;
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_REPEATNOTCHAR;
                        continue 'dispatch;
                    }
                    OP_NOTPOSPLUS | OP_NOTPOSPLUSI => {
                        reptype = REPTYPE_POS;
                        (*F).fields.charnot_repeat.min = 1;
                        (*F).fields.charnot_repeat.max = UINT32_MAX;
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_REPEATNOTCHAR;
                        continue 'dispatch;
                    }
                    OP_NOTPOSQUERY | OP_NOTPOSQUERYI => {
                        reptype = REPTYPE_POS;
                        (*F).fields.charnot_repeat.min = 0;
                        (*F).fields.charnot_repeat.max = 1;
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_REPEATNOTCHAR;
                        continue 'dispatch;
                    }
                    OP_NOTPOSUPTO | OP_NOTPOSUPTOI => {
                        reptype = REPTYPE_POS;
                        (*F).fields.charnot_repeat.min = 0;
                        (*F).fields.charnot_repeat.max = get2(Fecode!(F), 1);
                        Fecode!(F) = Fecode!(F).add(1 + IMM2_SIZE);
                        state = ST_REPEATNOTCHAR;
                        continue 'dispatch;
                    }
                    OP_NOTSTAR | OP_NOTSTARI | OP_NOTMINSTAR | OP_NOTMINSTARI | OP_NOTPLUS
                    | OP_NOTPLUSI | OP_NOTMINPLUS | OP_NOTMINPLUSI | OP_NOTQUERY | OP_NOTQUERYI
                    | OP_NOTMINQUERY | OP_NOTMINQUERYI => {
                        fc = (*Fecode!(F)
                            - (if Fop!(F) >= OP_NOTSTARI { OP_NOTSTARI } else { OP_NOTSTAR }))
                            as u32;
                        Fecode!(F) = Fecode!(F).add(1);
                        (*F).fields.charnot_repeat.min = rep_min[fc as usize];
                        (*F).fields.charnot_repeat.max = rep_max[fc as usize];
                        reptype = rep_typ[fc as usize];
                        state = ST_REPEATNOTCHAR;
                        continue 'dispatch;
                    }

                    /* ---- Match a bit-mapped character class, possibly repeated. ---- */
                    OP_NCLASS | OP_CLASS => {
                        (*F).fields.class_repeat.byte_map_address = Fecode!(F).add(1);
                        Fecode!(F) = Fecode!(F).add(1 + 32);

                        match *Fecode!(F) {
                            OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
                            | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
                                fc = (*Fecode!(F) - OP_CRSTAR) as u32;
                                Fecode!(F) = Fecode!(F).add(1);
                                (*F).fields.class_repeat.min = rep_min[fc as usize];
                                (*F).fields.class_repeat.max = rep_max[fc as usize];
                                reptype = rep_typ[fc as usize];
                            }
                            OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                                (*F).fields.class_repeat.min = get2(Fecode!(F), 1);
                                let mut mx = get2(Fecode!(F), 1 + IMM2_SIZE);
                                if mx == 0 {
                                    mx = UINT32_MAX;
                                }
                                (*F).fields.class_repeat.max = mx;
                                reptype = rep_typ[(*Fecode!(F) - OP_CRSTAR) as usize];
                                Fecode!(F) = Fecode!(F).add(1 + 2 * IMM2_SIZE);
                            }
                            _ => {
                                (*F).fields.class_repeat.min = 1;
                                (*F).fields.class_repeat.max = 1;
                            }
                        }

                        let lmin = (*F).fields.class_repeat.min;
                        /* Ensure the minimum number of matches are present. */
                        if utf != FALSE {
                            i = 1;
                            while i <= lmin {
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                fc = getcharinc(&mut Feptr!(F));
                                if fc > 255 {
                                    if Fop!(F) == OP_CLASS {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                } else {
                                    let bm = (*F).fields.class_repeat.byte_map_address;
                                    if (*bm.add((fc / 8) as usize) & (1u8 << (fc & 7))) == 0 {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                i += 1;
                            }
                        } else {
                            i = 1;
                            while i <= lmin {
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                fc = *Feptr!(F) as u32;
                                Feptr!(F) = Feptr!(F).add(1);
                                let bm = (*F).fields.class_repeat.byte_map_address;
                                if (*bm.add((fc / 8) as usize) & (1u8 << (fc & 7))) == 0 {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                i += 1;
                            }
                        }

                        if (*F).fields.class_repeat.min == (*F).fields.class_repeat.max {
                            state = ST_MAIN_LOOP;
                            continue 'dispatch;
                        }

                        if reptype == REPTYPE_MIN {
                            /* Minimizing: first RMATCH. */
                            if utf != FALSE {
                                RMATCH!(Fecode!(F), 200);
                            } else {
                                RMATCH!(Fecode!(F), 23);
                            }
                        } else {
                            /* Maximizing. */
                            (*F).fields.class_repeat.start_eptr = Feptr!(F);
                            if utf != FALSE {
                                let lmax = (*F).fields.class_repeat.max;
                                i = (*F).fields.class_repeat.min;
                                while i < lmax {
                                    let mut len: u32 = 1;
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        break;
                                    }
                                    let (ch, extra) = getcharlen(Feptr!(F));
                                    fc = ch;
                                    len += extra;
                                    if fc > 255 {
                                        if Fop!(F) == OP_CLASS {
                                            break;
                                        }
                                    } else {
                                        let bm = (*F).fields.class_repeat.byte_map_address;
                                        if (*bm.add((fc / 8) as usize) & (1u8 << (fc & 7))) == 0 {
                                            break;
                                        }
                                    }
                                    Feptr!(F) = Feptr!(F).add(len as usize);
                                    i += 1;
                                }
                                if reptype == REPTYPE_POS {
                                    state = ST_MAIN_LOOP;
                                    continue 'dispatch;
                                }
                                RMATCH!(Fecode!(F), 201);
                            } else {
                                let lmax = (*F).fields.class_repeat.max;
                                i = (*F).fields.class_repeat.min;
                                while i < lmax {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        break;
                                    }
                                    fc = *Feptr!(F) as u32;
                                    let bm = (*F).fields.class_repeat.byte_map_address;
                                    if (*bm.add((fc / 8) as usize) & (1u8 << (fc & 7))) == 0 {
                                        break;
                                    }
                                    Feptr!(F) = Feptr!(F).add(1);
                                    i += 1;
                                }
                                if reptype == REPTYPE_POS {
                                    state = ST_MAIN_LOOP;
                                    continue 'dispatch;
                                }
                                /* while (Feptr >= Lstart_eptr) RMATCH(RM24) */
                                if Feptr!(F) >= (*F).fields.class_repeat.start_eptr {
                                    RMATCH!(Fecode!(F), 24);
                                }
                                RRETURN!(MATCH_NOMATCH);
                            }
                        }
                    }

                    /* ---- Match an extended character class (XCLASS). ---- */
                    OP_XCLASS => {
                        (*F).fields.xclass_repeat.xclass_data = Fecode!(F).add(1 + LINK_SIZE);
                        Fecode!(F) = Fecode!(F).add(get(Fecode!(F), 1) as usize);

                        match *Fecode!(F) {
                            OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
                            | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
                                fc = (*Fecode!(F) - OP_CRSTAR) as u32;
                                Fecode!(F) = Fecode!(F).add(1);
                                (*F).fields.xclass_repeat.min = rep_min[fc as usize];
                                (*F).fields.xclass_repeat.max = rep_max[fc as usize];
                                reptype = rep_typ[fc as usize];
                            }
                            OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                                (*F).fields.xclass_repeat.min = get2(Fecode!(F), 1);
                                let mut mx = get2(Fecode!(F), 1 + IMM2_SIZE);
                                if mx == 0 {
                                    mx = UINT32_MAX;
                                }
                                (*F).fields.xclass_repeat.max = mx;
                                reptype = rep_typ[(*Fecode!(F) - OP_CRSTAR) as usize];
                                Fecode!(F) = Fecode!(F).add(1 + 2 * IMM2_SIZE);
                            }
                            _ => {
                                (*F).fields.xclass_repeat.min = 1;
                                (*F).fields.xclass_repeat.max = 1;
                            }
                        }

                        let lmin = (*F).fields.xclass_repeat.min;
                        i = 1;
                        while i <= lmin {
                            if Feptr!(F) >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                RRETURN!(MATCH_NOMATCH);
                            }
                            fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                            if crate::xclass::xclass(
                                fc,
                                (*F).fields.xclass_repeat.xclass_data,
                                (*mb).start_code as *const u8,
                                utf,
                            ) == FALSE
                            {
                                RRETURN!(MATCH_NOMATCH);
                            }
                            i += 1;
                        }

                        if (*F).fields.xclass_repeat.min == (*F).fields.xclass_repeat.max {
                            state = ST_MAIN_LOOP;
                            continue 'dispatch;
                        }

                        if reptype == REPTYPE_MIN {
                            RMATCH!(Fecode!(F), 100);
                        } else {
                            (*F).fields.xclass_repeat.start_eptr = Feptr!(F);
                            let lmax = (*F).fields.xclass_repeat.max;
                            i = (*F).fields.xclass_repeat.min;
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlentest(Feptr!(F), utf != FALSE);
                                fc = ch;
                                len += extra;
                                if crate::xclass::xclass(
                                    fc,
                                    (*F).fields.xclass_repeat.xclass_data,
                                    (*mb).start_code as *const u8,
                                    utf,
                                ) == FALSE
                                {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                            if reptype == REPTYPE_POS {
                                state = ST_MAIN_LOOP;
                                continue 'dispatch;
                            }
                            RMATCH!(Fecode!(F), 101);
                        }
                    }

                    /* ---- Match a complex, set-based character class (ECLASS). ---- */
                    OP_ECLASS => {
                        (*F).fields.eclass_repeat.eclass_data = Fecode!(F).add(1 + LINK_SIZE);
                        Fecode!(F) = Fecode!(F).add(get(Fecode!(F), 1) as usize);
                        (*F).fields.eclass_repeat.eclass_len = Fecode!(F)
                            .offset_from((*F).fields.eclass_repeat.eclass_data)
                            as PCRE2_SIZE;

                        match *Fecode!(F) {
                            OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
                            | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
                                fc = (*Fecode!(F) - OP_CRSTAR) as u32;
                                Fecode!(F) = Fecode!(F).add(1);
                                (*F).fields.eclass_repeat.min = rep_min[fc as usize];
                                (*F).fields.eclass_repeat.max = rep_max[fc as usize];
                                reptype = rep_typ[fc as usize];
                            }
                            OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                                (*F).fields.eclass_repeat.min = get2(Fecode!(F), 1);
                                let mut mx = get2(Fecode!(F), 1 + IMM2_SIZE);
                                if mx == 0 {
                                    mx = UINT32_MAX;
                                }
                                (*F).fields.eclass_repeat.max = mx;
                                reptype = rep_typ[(*Fecode!(F) - OP_CRSTAR) as usize];
                                Fecode!(F) = Fecode!(F).add(1 + 2 * IMM2_SIZE);
                            }
                            _ => {
                                (*F).fields.eclass_repeat.min = 1;
                                (*F).fields.eclass_repeat.max = 1;
                            }
                        }

                        let lmin = (*F).fields.eclass_repeat.min;
                        i = 1;
                        while i <= lmin {
                            if Feptr!(F) >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                RRETURN!(MATCH_NOMATCH);
                            }
                            fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                            if crate::xclass::eclass(
                                fc,
                                (*F).fields.eclass_repeat.eclass_data,
                                (*F).fields.eclass_repeat.eclass_data
                                    .add((*F).fields.eclass_repeat.eclass_len),
                                (*mb).start_code as *const u8,
                                utf,
                            ) == FALSE
                            {
                                RRETURN!(MATCH_NOMATCH);
                            }
                            i += 1;
                        }

                        if (*F).fields.eclass_repeat.min == (*F).fields.eclass_repeat.max {
                            state = ST_MAIN_LOOP;
                            continue 'dispatch;
                        }

                        if reptype == REPTYPE_MIN {
                            RMATCH!(Fecode!(F), 102);
                        } else {
                            (*F).fields.eclass_repeat.start_eptr = Feptr!(F);
                            let lmax = (*F).fields.eclass_repeat.max;
                            i = (*F).fields.eclass_repeat.min;
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlentest(Feptr!(F), utf != FALSE);
                                fc = ch;
                                len += extra;
                                if crate::xclass::eclass(
                                    fc,
                                    (*F).fields.eclass_repeat.eclass_data,
                                    (*F).fields.eclass_repeat.eclass_data
                                        .add((*F).fields.eclass_repeat.eclass_len),
                                    (*mb).start_code as *const u8,
                                    utf,
                                ) == FALSE
                                {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                            if reptype == REPTYPE_POS {
                                state = ST_MAIN_LOOP;
                                continue 'dispatch;
                            }
                            RMATCH!(Fecode!(F), 103);
                        }
                    }

                    /* ---- Character types when PCRE2_UCP is not set. ---- */
                    OP_NOT_DIGIT => {
                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                        if chmax_255(fc) && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                    OP_DIGIT => {
                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                        if !chmax_255(fc) || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                    OP_NOT_WHITESPACE => {
                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                        if chmax_255(fc) && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                    OP_WHITESPACE => {
                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                        if !chmax_255(fc) || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                    OP_NOT_WORDCHAR => {
                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                        if chmax_255(fc) && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                    OP_WORDCHAR => {
                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                        if !chmax_255(fc) || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                    OP_ANYNL => {
                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                        match fc {
                            CHAR_CR => {
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                } else if *Feptr!(F) as u32 == CHAR_LF {
                                    Feptr!(F) = Feptr!(F).add(1);
                                }
                            }
                            CHAR_LF => {}
                            CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                if (*mb).bsr_convention == PCRE2_BSR_ANYCRLF as u16 {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                            }
                            _ => {
                                RRETURN!(MATCH_NOMATCH);
                            }
                        }
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                    OP_NOT_HSPACE => {
                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                        if is_hspace(fc) {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                    OP_HSPACE => {
                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                        if !is_hspace(fc) {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                    OP_NOT_VSPACE => {
                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                        if is_vspace(fc) {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                    OP_VSPACE => {
                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                        if !is_vspace(fc) {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    /* ---- Check the next character by Unicode property. ---- */
                    OP_PROP | OP_NOTPROP => {
                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                        {
                            let prop: &UcdRecord = get_ucd(fc);
                            let notmatch: BOOL = (Fop!(F) == OP_NOTPROP) as BOOL;
                            let e1 = *Fecode!(F).add(1) as u32;
                            match e1 {
                                PT_LAMP => {
                                    let chartype = prop.chartype as u32;
                                    if ((chartype == ucp_Lu
                                        || chartype == ucp_Ll
                                        || chartype == ucp_Lt)
                                        as BOOL
                                        == notmatch)
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                PT_GC => {
                                    if ((*Fecode!(F).add(2) as u32
                                        == UCP_GENTYPE[prop.chartype as usize])
                                        as BOOL
                                        == notmatch)
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                PT_PC => {
                                    if ((*Fecode!(F).add(2) == prop.chartype) as BOOL == notmatch)
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                PT_SC => {
                                    if ((*Fecode!(F).add(2) == prop.script) as BOOL == notmatch) {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                PT_SCX => {
                                    let ok: BOOL = ((*Fecode!(F).add(2) == prop.script
                                        || mapbit(
                                            &UCD_SCRIPT_SETS[ucd_scriptx_prop(prop) as usize..],
                                            *Fecode!(F).add(2) as u32,
                                        ) != 0)
                                        as BOOL);
                                    if ok == notmatch {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                PT_ALNUM => {
                                    let chartype = prop.chartype as usize;
                                    if ((UCP_GENTYPE[chartype] == ucp_L
                                        || UCP_GENTYPE[chartype] == ucp_N)
                                        as BOOL
                                        == notmatch)
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                PT_SPACE | PT_PXSPACE => {
                                    if is_hspace(fc) || is_vspace(fc) {
                                        if notmatch != FALSE {
                                            RRETURN!(MATCH_NOMATCH);
                                        }
                                    } else {
                                        if ((UCP_GENTYPE[prop.chartype as usize] == ucp_Z)
                                            as BOOL
                                            == notmatch)
                                        {
                                            RRETURN!(MATCH_NOMATCH);
                                        }
                                    }
                                }
                                PT_WORD => {
                                    let chartype = prop.chartype as u32;
                                    if ((UCP_GENTYPE[chartype as usize] == ucp_L
                                        || UCP_GENTYPE[chartype as usize] == ucp_N
                                        || chartype == ucp_Mn
                                        || chartype == ucp_Pc)
                                        as BOOL
                                        == notmatch)
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                PT_CLIST => {
                                    let mut cp =
                                        &UCD_CASELESS_SETS[*Fecode!(F).add(2) as usize..];
                                    loop {
                                        if fc < cp[0] {
                                            if notmatch != FALSE {
                                                break;
                                            } else {
                                                RRETURN!(MATCH_NOMATCH);
                                            }
                                        }
                                        let v = cp[0];
                                        cp = &cp[1..];
                                        if fc == v {
                                            if notmatch != FALSE {
                                                RRETURN!(MATCH_NOMATCH);
                                            } else {
                                                break;
                                            }
                                        }
                                    }
                                }
                                PT_UCNC => {
                                    if ((fc == CHAR_DOLLAR_SIGN
                                        || fc == CHAR_COMMERCIAL_AT
                                        || fc == CHAR_GRAVE_ACCENT
                                        || (fc >= 0xa0 && fc <= 0xd7ff)
                                        || fc >= 0xe000)
                                        as BOOL
                                        == notmatch)
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                PT_BIDICL => {
                                    if ((ucd_bidiclass_prop(prop) == *Fecode!(F).add(2) as u32)
                                        as BOOL
                                        == notmatch)
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                PT_BOOL => {
                                    let ok: BOOL = (mapbit(
                                        &UCD_BOOLPROP_SETS[ucd_bprops_prop(prop) as usize..],
                                        *Fecode!(F).add(2) as u32,
                                    ) != 0)
                                        as BOOL;
                                    if ok == notmatch {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                _ => {
                                    return PCRE2_ERROR_INTERNAL;
                                }
                            }
                            Fecode!(F) = Fecode!(F).add(3);
                        }
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    /* ---- Match an extended Unicode sequence. ---- */
                    OP_EXTUNI => {
                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        } else {
                            fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                            Feptr!(F) = crate::extuni::extuni(
                                fc,
                                Feptr!(F),
                                (*mb).start_subject,
                                (*mb).end_subject,
                                utf,
                                core::ptr::null_mut(),
                            );
                        }
                        CHECK_PARTIAL!();
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    /* ---- Match a single character type repeatedly. ---- */
                    OP_TYPEEXACT => {
                        let m = get2(Fecode!(F), 1);
                        (*F).fields.type_repeat.min = m;
                        (*F).fields.type_repeat.max = m;
                        Fecode!(F) = Fecode!(F).add(1 + IMM2_SIZE);
                        state = ST_REPEATTYPE;
                        continue 'dispatch;
                    }
                    OP_TYPEUPTO | OP_TYPEMINUPTO => {
                        (*F).fields.type_repeat.min = 0;
                        (*F).fields.type_repeat.max = get2(Fecode!(F), 1);
                        reptype = if *Fecode!(F) == OP_TYPEMINUPTO {
                            REPTYPE_MIN
                        } else {
                            REPTYPE_MAX
                        };
                        Fecode!(F) = Fecode!(F).add(1 + IMM2_SIZE);
                        state = ST_REPEATTYPE;
                        continue 'dispatch;
                    }
                    OP_TYPEPOSSTAR => {
                        reptype = REPTYPE_POS;
                        (*F).fields.type_repeat.min = 0;
                        (*F).fields.type_repeat.max = UINT32_MAX;
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_REPEATTYPE;
                        continue 'dispatch;
                    }
                    OP_TYPEPOSPLUS => {
                        reptype = REPTYPE_POS;
                        (*F).fields.type_repeat.min = 1;
                        (*F).fields.type_repeat.max = UINT32_MAX;
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_REPEATTYPE;
                        continue 'dispatch;
                    }
                    OP_TYPEPOSQUERY => {
                        reptype = REPTYPE_POS;
                        (*F).fields.type_repeat.min = 0;
                        (*F).fields.type_repeat.max = 1;
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_REPEATTYPE;
                        continue 'dispatch;
                    }
                    OP_TYPEPOSUPTO => {
                        reptype = REPTYPE_POS;
                        (*F).fields.type_repeat.min = 0;
                        (*F).fields.type_repeat.max = get2(Fecode!(F), 1);
                        Fecode!(F) = Fecode!(F).add(1 + IMM2_SIZE);
                        state = ST_REPEATTYPE;
                        continue 'dispatch;
                    }
                    OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS
                    | OP_TYPEQUERY | OP_TYPEMINQUERY => {
                        fc = (*Fecode!(F) - OP_TYPESTAR) as u32;
                        Fecode!(F) = Fecode!(F).add(1);
                        (*F).fields.type_repeat.min = rep_min[fc as usize];
                        (*F).fields.type_repeat.max = rep_max[fc as usize];
                        reptype = rep_typ[fc as usize];
                        state = ST_REPEATTYPE;
                        continue 'dispatch;
                    }

                    /* ---- Match a back reference, possibly repeatedly. ---- */
                    OP_DNREF | OP_DNREFI => {
                        (*F).byte1 = (Fop!(F) == OP_DNREFI) as u8;
                        (*F).byte2 = if Fop!(F) == OP_DNREFI {
                            *Fecode!(F).add(1 + 2 * IMM2_SIZE)
                        } else {
                            0
                        };
                        {
                            let mut count = get2(Fecode!(F), 1 + IMM2_SIZE) as i32;
                            let mut slot: PCRE2_SPTR = (*mb)
                                .name_table
                                .add(get2(Fecode!(F), 1) as usize * (*mb).name_entry_size as usize);
                            Fecode!(F) = Fecode!(F)
                                .add(1 + 2 * IMM2_SIZE + (if Fop!(F) == OP_DNREFI { 1 } else { 0 }));

                            while count > 0 {
                                count -= 1;
                                let off = ((get2(slot, 0) << 1) as PCRE2_SIZE).wrapping_sub(2);
                                (*F).fields.ref_repeat.offset = off;
                                if off < Foffset_top!(F) && *Fovector!(F).add(off) != PCRE2_UNSET {
                                    break;
                                }
                                slot = slot.add((*mb).name_entry_size as usize);
                            }
                        }
                        state = ST_REF_REPEAT;
                        continue 'dispatch;
                    }
                    OP_REF | OP_REFI => {
                        (*F).byte1 = (Fop!(F) == OP_REFI) as u8;
                        (*F).byte2 = if Fop!(F) == OP_REFI {
                            *Fecode!(F).add(1 + IMM2_SIZE)
                        } else {
                            0
                        };
                        (*F).fields.ref_repeat.offset =
                            ((get2(Fecode!(F), 1) << 1) as PCRE2_SIZE).wrapping_sub(2);
                        Fecode!(F) = Fecode!(F)
                            .add(1 + IMM2_SIZE + (if Fop!(F) == OP_REFI { 1 } else { 0 }));
                        state = ST_REF_REPEAT;
                        continue 'dispatch;
                    }

                    /* ---- Parenthesized-group start opcodes. ---- */
                    OP_BRAZERO => {
                        Fecode!(F) = Fecode!(F).add(1);
                        RMATCH!(Fecode!(F), 9);
                    }
                    OP_BRAMINZERO => {
                        Fecode!(F) = Fecode!(F).add(1);
                        let mut next_ecode: PCRE2_SPTR = Fecode!(F);
                        loop {
                            next_ecode = next_ecode.add(get(next_ecode, 1) as usize);
                            if *next_ecode != OP_ALT {
                                break;
                            }
                        }
                        RMATCH!(next_ecode.add(1 + LINK_SIZE), 10);
                    }
                    OP_SKIPZERO => {
                        let mut next_ecode: PCRE2_SPTR = Fecode!(F).add(1);
                        loop {
                            next_ecode = next_ecode.add(get(next_ecode, 1) as usize);
                            if *next_ecode != OP_ALT {
                                break;
                            }
                        }
                        Fecode!(F) = next_ecode.add(1 + LINK_SIZE);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    OP_BRAPOSZERO => {
                        (*F).byte2 = TRUE as u8; /* Lzero_allowed = TRUE */
                        Fecode!(F) = Fecode!(F).add(1);
                        if *Fecode!(F) == OP_CBRAPOS || *Fecode!(F) == OP_SCBRAPOS {
                            state = ST_POSSESSIVE_CAPTURE;
                        } else {
                            state = ST_POSSESSIVE_NON_CAPTURE;
                        }
                        continue 'dispatch;
                    }
                    OP_BRAPOS | OP_SBRAPOS => {
                        (*F).byte2 = FALSE as u8; /* Lzero_allowed = FALSE */
                        state = ST_POSSESSIVE_NON_CAPTURE;
                        continue 'dispatch;
                    }
                    OP_CBRAPOS | OP_SCBRAPOS => {
                        (*F).byte2 = FALSE as u8; /* Lzero_allowed = FALSE */
                        state = ST_POSSESSIVE_CAPTURE;
                        continue 'dispatch;
                    }

                    OP_BRA => {
                        if (*mb).hasthen != FALSE || Frdepth!(F) == 0 {
                            (*F).fields.op_bra.frame_type = 0;
                            state = ST_GROUPLOOP;
                            continue 'dispatch;
                        }
                        state = ST_L_BRA_LOOP;
                        continue 'dispatch;
                    }

                    OP_CBRA | OP_SCBRA => {
                        (*F).fields.op_bra.frame_type =
                            GF_CAPTURE | get2(Fecode!(F), 1 + LINK_SIZE);
                        state = ST_GROUPLOOP;
                        continue 'dispatch;
                    }

                    OP_ONCE | OP_SCRIPT_RUN | OP_SBRA => {
                        (*F).fields.op_bra.frame_type = GF_NOCAPTURE;
                        state = ST_GROUPLOOP;
                        continue 'dispatch;
                    }

                    OP_RECURSE => {
                        bracode = (*mb).start_code.add(get(Fecode!(F), 1) as usize);
                        number = if bracode == (*mb).start_code {
                            0
                        } else {
                            get2(bracode, 1 + LINK_SIZE)
                        };

                        if Fcurrent_recurse!(F) != RECURSE_UNSET {
                            offset = Flast_group_offset!(F);
                            while offset != PCRE2_UNSET {
                                N = ((*match_data).heapframes as *mut u8).add(offset)
                                    as *mut heapframe;
                                P = (N as *mut u8).sub(frame_size) as *mut heapframe;
                                if (*N).group_frame_type == (GF_RECURSE | number) {
                                    if Feptr!(F) == (*P).eptr
                                        && (*mb).last_used_ptr == (*P).recurse_last_used
                                        && ((*mb).moptions & PCRE2_DISABLE_RECURSELOOP_CHECK) == 0
                                    {
                                        return PCRE2_ERROR_RECURSELOOP;
                                    }
                                    break;
                                }
                                offset = (*P).last_group_offset;
                            }
                        }

                        (*F).recurse_last_used = (*mb).last_used_ptr;
                        (*F).fields.op_recurse.start_branch = bracode;
                        (*F).fields.op_recurse.frame_type = GF_RECURSE | number;

                        group_frame_type = (*F).fields.op_recurse.frame_type;
                        RMATCH!(
                            (*F).fields.op_recurse.start_branch
                                .add(op_length(*(*F).fields.op_recurse.start_branch)),
                            11
                        );
                    }

                    OP_ASSERT | OP_ASSERTBACK | OP_ASSERT_NA | OP_ASSERTBACK_NA => {
                        group_frame_type = GF_NOCAPTURE;
                        RMATCH!(Fecode!(F).add(op_length(*Fecode!(F))), 3);
                    }

                    OP_ASSERT_NOT | OP_ASSERTBACK_NOT => {
                        group_frame_type = GF_NOCAPTURE;
                        RMATCH!(Fecode!(F).add(op_length(*Fecode!(F))), 4);
                    }

                    OP_ASSERT_SCS => {
                        length = 0;
                        {
                            let mut ecode: PCRE2_SPTR = Fecode!(F).add(1 + LINK_SIZE);
                            let mut count: i32;
                            let mut slot: PCRE2_SPTR;
                            offset = 0;

                            'scs_search: loop {
                                if *ecode == OP_CREF {
                                    length += (1 + IMM2_SIZE) as PCRE2_SIZE;
                                    offset = ((get2(ecode, 1) << 1) as PCRE2_SIZE).wrapping_sub(2);
                                    ecode = ecode.add(1 + IMM2_SIZE);
                                    if offset < Foffset_top!(F)
                                        && *Fovector!(F).add(offset) != PCRE2_UNSET
                                    {
                                        break 'scs_search;
                                    }
                                    continue;
                                }

                                if *ecode != OP_DNCREF {
                                    RRETURN!(MATCH_NOMATCH);
                                }

                                count = get2(ecode, 1 + IMM2_SIZE) as i32;
                                slot = (*mb).name_table.add(
                                    get2(ecode, 1) as usize * (*mb).name_entry_size as usize,
                                );
                                length += (1 + 2 * IMM2_SIZE) as PCRE2_SIZE;
                                ecode = ecode.add(1 + 2 * IMM2_SIZE);

                                while count > 0 {
                                    offset =
                                        ((get2(slot, 0) << 1) as PCRE2_SIZE).wrapping_sub(2);
                                    if offset < Foffset_top!(F)
                                        && *Fovector!(F).add(offset) != PCRE2_UNSET
                                    {
                                        break 'scs_search;
                                    }
                                    slot = slot.add((*mb).name_entry_size as usize);
                                    count -= 1;
                                }
                            }

                            /* Skip remaining options. */
                            loop {
                                if *ecode == OP_CREF {
                                    length += (1 + IMM2_SIZE) as PCRE2_SIZE;
                                    ecode = ecode.add(1 + IMM2_SIZE);
                                } else if *ecode == OP_DNCREF {
                                    length += (1 + 2 * IMM2_SIZE) as PCRE2_SIZE;
                                    ecode = ecode.add(1 + 2 * IMM2_SIZE);
                                } else {
                                    break;
                                }
                            }
                        }

                        (*F).fields.op_assert_scs.saved_end_subject = (*mb).end_subject;
                        (*F).fields.op_assert_scs.true_end_extra =
                            (*mb).true_end_subject.offset_from((*mb).end_subject) as PCRE2_SIZE;
                        (*F).fields.op_assert_scs.saved_eptr = Feptr!(F);
                        (*F).fields.op_assert_scs.saved_moptions = (*mb).moptions;

                        Feptr!(F) = (*mb).start_subject.add(*Fovector!(F).add(offset));
                        (*mb).end_subject = (*mb).start_subject.add(*Fovector!(F).add(offset + 1));
                        (*mb).true_end_subject = (*mb).end_subject;
                        (*mb).moptions &= !PCRE2_NOTEOL;

                        group_frame_type = GF_NOCAPTURE;
                        RMATCH!(Fecode!(F).add(1 + LINK_SIZE + length), 38);
                    }

                    OP_CALLOUT | OP_CALLOUT_STR => {
                        rrc = do_callout(F, mb, &mut length);
                        if rrc > 0 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        if rrc < 0 {
                            RRETURN!(rrc);
                        }
                        Fecode!(F) = Fecode!(F).add(length);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    OP_COND | OP_SCOND => {
                        (*F).fields.op_cond.length = get(Fecode!(F), 1) as PCRE2_SIZE;
                        if *Fecode!(F).add((*F).fields.op_cond.length) != OP_ALT {
                            (*F).fields.op_cond.length -= (1 + LINK_SIZE) as PCRE2_SIZE;
                        }
                        Fecode!(F) = Fecode!(F).add(1 + LINK_SIZE);

                        if *Fecode!(F) == OP_CALLOUT || *Fecode!(F) == OP_CALLOUT_STR {
                            rrc = do_callout(F, mb, &mut length);
                            if rrc > 0 {
                                RRETURN!(MATCH_NOMATCH);
                            }
                            if rrc < 0 {
                                RRETURN!(rrc);
                            }
                            Fecode!(F) = Fecode!(F).add(length);
                            (*F).fields.op_cond.length -= length;
                        }

                        condition = FALSE;
                        match *Fecode!(F) {
                            OP_RREF => {
                                if Fcurrent_recurse!(F) != RECURSE_UNSET {
                                    number = get2(Fecode!(F), 1);
                                    condition = (number == RREF_ANY
                                        || number == Fcurrent_recurse!(F))
                                        as BOOL;
                                }
                            }
                            OP_DNRREF => {
                                if Fcurrent_recurse!(F) != RECURSE_UNSET {
                                    let mut count = get2(Fecode!(F), 1 + IMM2_SIZE) as i32;
                                    let mut slot: PCRE2_SPTR = (*mb).name_table.add(
                                        get2(Fecode!(F), 1) as usize
                                            * (*mb).name_entry_size as usize,
                                    );
                                    while count > 0 {
                                        count -= 1;
                                        number = get2(slot, 0);
                                        condition = (number == Fcurrent_recurse!(F)) as BOOL;
                                        if condition != FALSE {
                                            break;
                                        }
                                        slot = slot.add((*mb).name_entry_size as usize);
                                    }
                                }
                            }
                            OP_CREF => {
                                offset = ((get2(Fecode!(F), 1) << 1) as PCRE2_SIZE).wrapping_sub(2);
                                condition = (offset < Foffset_top!(F)
                                    && *Fovector!(F).add(offset) != PCRE2_UNSET)
                                    as BOOL;
                            }
                            OP_DNCREF => {
                                let mut count = get2(Fecode!(F), 1 + IMM2_SIZE) as i32;
                                let mut slot: PCRE2_SPTR = (*mb).name_table.add(
                                    get2(Fecode!(F), 1) as usize * (*mb).name_entry_size as usize,
                                );
                                while count > 0 {
                                    count -= 1;
                                    offset =
                                        ((get2(slot, 0) << 1) as PCRE2_SIZE).wrapping_sub(2);
                                    condition = (offset < Foffset_top!(F)
                                        && *Fovector!(F).add(offset) != PCRE2_UNSET)
                                        as BOOL;
                                    if condition != FALSE {
                                        break;
                                    }
                                    slot = slot.add((*mb).name_entry_size as usize);
                                }
                            }
                            OP_FALSE | OP_FAIL => {}
                            OP_TRUE => {
                                condition = TRUE;
                            }
                            _ => {
                                (*F).byte1 = (*Fecode!(F) == OP_ASSERT
                                    || *Fecode!(F) == OP_ASSERTBACK)
                                    as u8;
                                (*F).fields.op_cond.start_branch = Fecode!(F);
                                group_frame_type = GF_CONDASSERT;
                                RMATCH!(
                                    (*F).fields.op_cond.start_branch
                                        .add(op_length(*(*F).fields.op_cond.start_branch)),
                                    5
                                );
                            }
                        }

                        /* Choose branch according to the condition (non-assertion path). */
                        Fecode!(F) = Fecode!(F).add(if condition != FALSE {
                            op_length(*Fecode!(F))
                        } else {
                            (*F).fields.op_cond.length
                        });

                        if Fop!(F) == OP_SCOND {
                            group_frame_type = GF_NOCAPTURE;
                            RMATCH!(Fecode!(F), 35);
                        }
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    OP_REVERSE => {
                        number = get2(Fecode!(F), 1);
                        if utf != FALSE {
                            while number > 0 {
                                number -= 1;
                                if Feptr!(F) <= (*mb).check_subject {
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                Feptr!(F) = Feptr!(F).sub(1);
                                backchar(&mut Feptr!(F));
                            }
                        } else {
                            if (number as isize)
                                > Feptr!(F).offset_from((*mb).start_subject)
                            {
                                RRETURN!(MATCH_NOMATCH);
                            }
                            Feptr!(F) = Feptr!(F).sub(number as usize);
                        }
                        if Feptr!(F) < (*mb).start_used_ptr {
                            (*mb).start_used_ptr = Feptr!(F);
                        }
                        Fecode!(F) = Fecode!(F).add(1 + IMM2_SIZE);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    OP_VREVERSE => {
                        (*F).fields.op_vreverse.min = get2(Fecode!(F), 1);
                        (*F).fields.op_vreverse.max = get2(Fecode!(F), 1 + IMM2_SIZE);

                        if utf != FALSE {
                            i = 0;
                            while i < (*F).fields.op_vreverse.max {
                                if Feptr!(F) == (*mb).start_subject {
                                    if i < (*F).fields.op_vreverse.min {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    (*F).fields.op_vreverse.max = i;
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).sub(1);
                                backchar(&mut Feptr!(F));
                                i += 1;
                            }
                        } else {
                            let diff = Feptr!(F).offset_from((*mb).start_subject);
                            let available: u32 = if diff > 65535 {
                                65535
                            } else if diff > 0 {
                                diff as u32
                            } else {
                                0
                            };
                            if (*F).fields.op_vreverse.min > available {
                                RRETURN!(MATCH_NOMATCH);
                            }
                            if (*F).fields.op_vreverse.max > available {
                                (*F).fields.op_vreverse.max = available;
                            }
                            Feptr!(F) = Feptr!(F).sub((*F).fields.op_vreverse.max as usize);
                        }

                        RMATCH!(Fecode!(F).add(1 + 2 * IMM2_SIZE), 37);
                    }

                    OP_ALT => {
                        branch_end = Fecode!(F);
                        loop {
                            Fecode!(F) = Fecode!(F).add(get(Fecode!(F), 1) as usize);
                            if *Fecode!(F) != OP_ALT {
                                break;
                            }
                        }
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    OP_KET | OP_KETRMIN | OP_KETRMAX | OP_KETRPOS => {
                        state = ST_KET;
                        continue 'dispatch;
                    }

                    OP_CIRC => {
                        if Feptr!(F) != (*mb).start_subject
                            || ((*mb).moptions & PCRE2_NOTBOL) != 0
                        {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    OP_SOD => {
                        if Feptr!(F) != (*mb).start_subject {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    OP_DOLL => {
                        if ((*mb).moptions & PCRE2_NOTEOL) != 0 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        if ((*mb).poptions & PCRE2_DOLLAR_ENDONLY) == 0 {
                            state = ST_ASSERT_NL_OR_EOS;
                            continue 'dispatch;
                        }
                        /* Fall through to OP_EOD behaviour. */
                        if Feptr!(F) < (*mb).true_end_subject {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        if (*mb).partial != 0 {
                            (*mb).hitend = TRUE;
                            if (*mb).partial > 1 {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    OP_EOD => {
                        if Feptr!(F) < (*mb).true_end_subject {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        if (*mb).partial != 0 {
                            (*mb).hitend = TRUE;
                            if (*mb).partial > 1 {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    OP_EODN => {
                        state = ST_ASSERT_NL_OR_EOS;
                        continue 'dispatch;
                    }

                    OP_CIRCM => {
                        if ((*mb).moptions & PCRE2_NOTBOL) != 0
                            && Feptr!(F) == (*mb).start_subject
                        {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        if Feptr!(F) != (*mb).start_subject
                            && ((Feptr!(F) == (*mb).end_subject
                                && ((*mb).poptions & PCRE2_ALT_CIRCUMFLEX) == 0)
                                || !WAS_NEWLINE!(Feptr!(F)))
                        {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    OP_DOLLM => {
                        if Feptr!(F) < (*mb).end_subject {
                            if !IS_NEWLINE!(Feptr!(F)) {
                                if (*mb).partial != 0
                                    && Feptr!(F).add(1) >= (*mb).end_subject
                                    && (*mb).nltype == NLTYPE_FIXED
                                    && (*mb).nllen == 2
                                    && *Feptr!(F) as u32 == (*mb).nl[0] as u32
                                {
                                    (*mb).hitend = TRUE;
                                    if (*mb).partial > 1 {
                                        return PCRE2_ERROR_PARTIAL;
                                    }
                                }
                                RRETURN!(MATCH_NOMATCH);
                            }
                        } else {
                            if ((*mb).moptions & PCRE2_NOTEOL) != 0 {
                                RRETURN!(MATCH_NOMATCH);
                            }
                            SCHECK_PARTIAL!();
                        }
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    OP_SOM => {
                        if Feptr!(F) != (*mb).start_subject.add((*mb).start_offset) {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    OP_SET_SOM => {
                        Fstart_match!(F) = Feptr!(F);
                        Fecode!(F) = Fecode!(F).add(1);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    OP_NOT_WORD_BOUNDARY | OP_WORD_BOUNDARY | OP_NOT_UCP_WORD_BOUNDARY
                    | OP_UCP_WORD_BOUNDARY => {
                        if Feptr!(F) == (*mb).check_subject {
                            prev_is_word = FALSE;
                        } else {
                            let mut lastptr: PCRE2_SPTR = Feptr!(F).sub(1);
                            if utf != FALSE {
                                backchar(&mut lastptr);
                                fc = getchar_(lastptr);
                            } else {
                                fc = *lastptr as u32;
                            }
                            if lastptr < (*mb).start_used_ptr {
                                (*mb).start_used_ptr = lastptr;
                            }
                            if Fop!(F) == OP_UCP_WORD_BOUNDARY
                                || Fop!(F) == OP_NOT_UCP_WORD_BOUNDARY
                            {
                                let chartype = ucd_chartype(fc);
                                let category = UCP_GENTYPE[chartype as usize];
                                prev_is_word = (category == ucp_L
                                    || category == ucp_N
                                    || chartype == ucp_Mn
                                    || chartype == ucp_Pc)
                                    as BOOL;
                            } else {
                                prev_is_word = (chmax_255(fc)
                                    && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0)
                                    as BOOL;
                            }
                        }

                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            cur_is_word = FALSE;
                        } else {
                            let mut nextptr: PCRE2_SPTR = Feptr!(F).add(1);
                            if utf != FALSE {
                                forwardchartest(&mut nextptr, (*mb).end_subject);
                                fc = getchar_(Feptr!(F));
                            } else {
                                fc = *Feptr!(F) as u32;
                            }
                            if nextptr > (*mb).last_used_ptr {
                                (*mb).last_used_ptr = nextptr;
                            }
                            if Fop!(F) == OP_UCP_WORD_BOUNDARY
                                || Fop!(F) == OP_NOT_UCP_WORD_BOUNDARY
                            {
                                let chartype = ucd_chartype(fc);
                                let category = UCP_GENTYPE[chartype as usize];
                                cur_is_word = (category == ucp_L
                                    || category == ucp_N
                                    || chartype == ucp_Mn
                                    || chartype == ucp_Pc)
                                    as BOOL;
                            } else {
                                cur_is_word = (chmax_255(fc)
                                    && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0)
                                    as BOOL;
                            }
                        }

                        {
                            let op0 = *Fecode!(F);
                            Fecode!(F) = Fecode!(F).add(1);
                            let want_equal =
                                op0 == OP_WORD_BOUNDARY || Fop!(F) == OP_UCP_WORD_BOUNDARY;
                            let fail = if want_equal {
                                cur_is_word == prev_is_word
                            } else {
                                cur_is_word != prev_is_word
                            };
                            if fail {
                                RRETURN!(MATCH_NOMATCH);
                            }
                        }
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    OP_MARK => {
                        Fmark!(F) = Fecode!(F).add(2);
                        (*mb).nomatch_mark = Fecode!(F).add(2);
                        RMATCH!(
                            Fecode!(F)
                                .add(op_length(*Fecode!(F)) + *Fecode!(F).add(1) as usize),
                            12
                        );
                    }

                    OP_FAIL => {
                        RRETURN!(MATCH_NOMATCH);
                    }

                    OP_COMMIT => {
                        RMATCH!(Fecode!(F).add(op_length(*Fecode!(F))), 13);
                    }

                    OP_COMMIT_ARG => {
                        Fmark!(F) = Fecode!(F).add(2);
                        (*mb).nomatch_mark = Fecode!(F).add(2);
                        RMATCH!(
                            Fecode!(F)
                                .add(op_length(*Fecode!(F)) + *Fecode!(F).add(1) as usize),
                            36
                        );
                    }

                    OP_PRUNE => {
                        RMATCH!(Fecode!(F).add(op_length(*Fecode!(F))), 14);
                    }

                    OP_PRUNE_ARG => {
                        Fmark!(F) = Fecode!(F).add(2);
                        (*mb).nomatch_mark = Fecode!(F).add(2);
                        RMATCH!(
                            Fecode!(F)
                                .add(op_length(*Fecode!(F)) + *Fecode!(F).add(1) as usize),
                            15
                        );
                    }

                    OP_SKIP => {
                        RMATCH!(Fecode!(F).add(op_length(*Fecode!(F))), 16);
                    }

                    OP_SKIP_ARG => {
                        (*mb).skip_arg_count += 1;
                        if (*mb).skip_arg_count <= (*mb).ignore_skip_arg {
                            Fecode!(F) = Fecode!(F)
                                .add(op_length(*Fecode!(F)) + *Fecode!(F).add(1) as usize);
                            state = ST_MAIN_LOOP;
                            continue 'dispatch;
                        }
                        RMATCH!(
                            Fecode!(F)
                                .add(op_length(*Fecode!(F)) + *Fecode!(F).add(1) as usize),
                            17
                        );
                    }

                    OP_THEN => {
                        RMATCH!(Fecode!(F).add(op_length(*Fecode!(F))), 18);
                    }

                    OP_THEN_ARG => {
                        Fmark!(F) = Fecode!(F).add(2);
                        (*mb).nomatch_mark = Fecode!(F).add(2);
                        RMATCH!(
                            Fecode!(F)
                                .add(op_length(*Fecode!(F)) + *Fecode!(F).add(1) as usize),
                            19
                        );
                    }

                    _ => {
                        return PCRE2_ERROR_INTERNAL;
                    }
                } /* End match Fop */
            } /* End ST_MAIN_LOOP arm */

            /* ---- REPEATCHAR: repeated single-character match. ---- */
            ST_REPEATCHAR => {
                if utf != FALSE {
                    length = 1;
                    (*F).fields.char_repeat.charptr = Fecode!(F);
                    let (ch, extra) = getcharlen(Fecode!(F));
                    fc = ch;
                    length += extra as PCRE2_SIZE;
                    Fecode!(F) = Fecode!(F).add(length);
                    (*F).byte1 = length as u8;

                    if length > 1 {
                        let othercase: u32;
                        if Fop!(F) >= OP_STARI && {
                            othercase = ucd_othercase(fc);
                            othercase != fc
                        } {
                            (*F).byte2 =
                                ord2utf(othercase, (&raw mut (*F).fields.char_repeat.oc.occu) as *mut u8)
                                    as u8;
                        } else {
                            (*F).byte2 = 0;
                        }

                        i = 1;
                        let lmin = (*F).fields.char_repeat.min;
                        while i <= lmin {
                            let ln = (*F).byte1 as usize;
                            let ocl = (*F).byte2 as usize;
                            if Feptr!(F) <= (*mb).end_subject.sub(length)
                                && memcmp(
                                    Feptr!(F) as *const c_void,
                                    (*F).fields.char_repeat.charptr as *const c_void,
                                    cu2bytes(length),
                                ) == 0
                            {
                                Feptr!(F) = Feptr!(F).add(length);
                            } else if ocl > 0
                                && Feptr!(F) <= (*mb).end_subject.sub(ocl)
                                && memcmp(
                                    Feptr!(F) as *const c_void,
                                    (&raw const (*F).fields.char_repeat.oc.occu) as *const c_void,
                                    cu2bytes(ocl),
                                ) == 0
                            {
                                Feptr!(F) = Feptr!(F).add(ocl);
                            } else {
                                CHECK_PARTIAL!();
                                RRETURN!(MATCH_NOMATCH);
                            }
                            let _ = ln;
                            i += 1;
                        }

                        if (*F).fields.char_repeat.min == (*F).fields.char_repeat.max {
                            state = ST_MAIN_LOOP;
                            continue 'dispatch;
                        }

                        if reptype == REPTYPE_MIN {
                            RMATCH!(Fecode!(F), 202);
                        } else {
                            (*F).fields.char_repeat.start_eptr = Feptr!(F);
                            i = (*F).fields.char_repeat.min;
                            let lmax = (*F).fields.char_repeat.max;
                            while i < lmax {
                                let ll = (*F).byte1 as usize;
                                let ocl = (*F).byte2 as usize;
                                if Feptr!(F) <= (*mb).end_subject.sub(ll)
                                    && memcmp(
                                        Feptr!(F) as *const c_void,
                                        (*F).fields.char_repeat.charptr as *const c_void,
                                        cu2bytes(ll),
                                    ) == 0
                                {
                                    Feptr!(F) = Feptr!(F).add(ll);
                                } else if ocl > 0
                                    && Feptr!(F) <= (*mb).end_subject.sub(ocl)
                                    && memcmp(
                                        Feptr!(F) as *const c_void,
                                        (&raw const (*F).fields.char_repeat.oc.occu) as *const c_void,
                                        cu2bytes(ocl),
                                    ) == 0
                                {
                                    Feptr!(F) = Feptr!(F).add(ocl);
                                } else {
                                    CHECK_PARTIAL!();
                                    break;
                                }
                                i += 1;
                            }
                            if reptype != REPTYPE_POS {
                                if Feptr!(F) > (*F).fields.char_repeat.start_eptr {
                                    RMATCH!(Fecode!(F), 203);
                                }
                            }
                            state = ST_MAIN_LOOP;
                            continue 'dispatch;
                        }
                    }

                    /* Length of UTF character is 1. */
                    (*F).fields.char_repeat.c = fc;
                } else {
                    (*F).fields.char_repeat.c = *Fecode!(F) as u32;
                    Fecode!(F) = Fecode!(F).add(1);
                }

                /* Caseless comparison. */
                if Fop!(F) >= OP_STARI {
                    let lc = (*F).fields.char_repeat.c;
                    if ucp != FALSE && utf == FALSE && lc > 127 {
                        (*F).fields.char_repeat.oc.oc = ucd_othercase(lc);
                    } else {
                        (*F).fields.char_repeat.oc.oc = *(*mb).fcc.add(lc as usize) as u32;
                    }

                    i = 1;
                    let lmin = (*F).fields.char_repeat.min;
                    while i <= lmin {
                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        let cc = *Feptr!(F) as u32;
                        if (*F).fields.char_repeat.c != cc && (*F).fields.char_repeat.oc.oc != cc {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        Feptr!(F) = Feptr!(F).add(1);
                        i += 1;
                    }
                    if (*F).fields.char_repeat.min == (*F).fields.char_repeat.max {
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    if reptype == REPTYPE_MIN {
                        RMATCH!(Fecode!(F), 25);
                    } else {
                        (*F).fields.char_repeat.start_eptr = Feptr!(F);
                        i = (*F).fields.char_repeat.min;
                        let lmax = (*F).fields.char_repeat.max;
                        while i < lmax {
                            if Feptr!(F) >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            let cc = *Feptr!(F) as u32;
                            if (*F).fields.char_repeat.c != cc
                                && (*F).fields.char_repeat.oc.oc != cc
                            {
                                break;
                            }
                            Feptr!(F) = Feptr!(F).add(1);
                            i += 1;
                        }
                        if reptype != REPTYPE_POS {
                            if Feptr!(F) != (*F).fields.char_repeat.start_eptr {
                                RMATCH!(Fecode!(F), 26);
                            }
                        }
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                } else {
                    /* Caseful comparisons. */
                    i = 1;
                    let lmin = (*F).fields.char_repeat.min;
                    while i <= lmin {
                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        let sc = *Feptr!(F);
                        Feptr!(F) = Feptr!(F).add(1);
                        if (*F).fields.char_repeat.c != sc as u32 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        i += 1;
                    }

                    if (*F).fields.char_repeat.min == (*F).fields.char_repeat.max {
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    if reptype == REPTYPE_MIN {
                        RMATCH!(Fecode!(F), 27);
                    } else {
                        (*F).fields.char_repeat.start_eptr = Feptr!(F);
                        i = (*F).fields.char_repeat.min;
                        let lmax = (*F).fields.char_repeat.max;
                        while i < lmax {
                            if Feptr!(F) >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            if (*F).fields.char_repeat.c != *Feptr!(F) as u32 {
                                break;
                            }
                            Feptr!(F) = Feptr!(F).add(1);
                            i += 1;
                        }
                        if reptype != REPTYPE_POS {
                            if Feptr!(F) > (*F).fields.char_repeat.start_eptr {
                                RMATCH!(Fecode!(F), 28);
                            }
                        }
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                }
            }

            /* Resume: minimize wide char (RM202). */
            ST_L202 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.char_repeat.min;
                    (*F).fields.char_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.char_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                let ll = (*F).byte1 as usize;
                let ocl = (*F).byte2 as usize;
                if Feptr!(F) <= (*mb).end_subject.sub(ll)
                    && memcmp(
                        Feptr!(F) as *const c_void,
                        (*F).fields.char_repeat.charptr as *const c_void,
                        cu2bytes(ll),
                    ) == 0
                {
                    Feptr!(F) = Feptr!(F).add(ll);
                } else if ocl > 0
                    && Feptr!(F) <= (*mb).end_subject.sub(ocl)
                    && memcmp(
                        Feptr!(F) as *const c_void,
                        (&raw const (*F).fields.char_repeat.oc.occu) as *const c_void,
                        cu2bytes(ocl),
                    ) == 0
                {
                    Feptr!(F) = Feptr!(F).add(ocl);
                } else {
                    CHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                RMATCH!(Fecode!(F), 202);
            }

            /* Resume: maximize wide char backtrack (RM203). */
            ST_L203 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                Feptr!(F) = Feptr!(F).sub(1);
                backchar(&mut Feptr!(F));
                if Feptr!(F) <= (*F).fields.char_repeat.start_eptr {
                    state = ST_MAIN_LOOP;
                    continue 'dispatch;
                }
                RMATCH!(Fecode!(F), 203);
            }

            /* Resume: minimize caseless single char (RM25). */
            ST_L25 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.char_repeat.min;
                    (*F).fields.char_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.char_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                let cc = *Feptr!(F) as u32;
                if (*F).fields.char_repeat.c != cc && (*F).fields.char_repeat.oc.oc != cc {
                    RRETURN!(MATCH_NOMATCH);
                }
                Feptr!(F) = Feptr!(F).add(1);
                RMATCH!(Fecode!(F), 25);
            }

            /* Resume: maximize caseless single char backtrack (RM26). */
            ST_L26 => {
                Feptr!(F) = Feptr!(F).sub(1);
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if Feptr!(F) == (*F).fields.char_repeat.start_eptr {
                    state = ST_MAIN_LOOP;
                    continue 'dispatch;
                }
                RMATCH!(Fecode!(F), 26);
            }

            /* Resume: minimize caseful single char (RM27). */
            ST_L27 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.char_repeat.min;
                    (*F).fields.char_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.char_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                let sc = *Feptr!(F);
                Feptr!(F) = Feptr!(F).add(1);
                if (*F).fields.char_repeat.c != sc as u32 {
                    RRETURN!(MATCH_NOMATCH);
                }
                RMATCH!(Fecode!(F), 27);
            }

            /* Resume: maximize caseful single char backtrack (RM28). */
            ST_L28 => {
                Feptr!(F) = Feptr!(F).sub(1);
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if Feptr!(F) <= (*F).fields.char_repeat.start_eptr {
                    state = ST_MAIN_LOOP;
                    continue 'dispatch;
                }
                RMATCH!(Fecode!(F), 28);
            }

            /* ---- REPEATNOTCHAR: repeated negated single-character match. ---- */
            ST_REPEATNOTCHAR => {
                (*F).fields.charnot_repeat.c =
                    getcharinctest(&mut Fecode!(F), utf != FALSE);

                if Fop!(F) >= OP_NOTSTARI {
                    /* Caseless */
                    let lc = (*F).fields.charnot_repeat.c;
                    if (utf != FALSE || ucp != FALSE) && lc > 127 {
                        (*F).fields.charnot_repeat.oc = ucd_othercase(lc);
                    } else {
                        (*F).fields.charnot_repeat.oc = *(*mb).fcc.add(lc as usize) as u32;
                    }

                    let lmin = (*F).fields.charnot_repeat.min;
                    if utf != FALSE {
                        i = 1;
                        while i <= lmin {
                            if Feptr!(F) >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                RRETURN!(MATCH_NOMATCH);
                            }
                            let d = getcharinc(&mut Feptr!(F));
                            if (*F).fields.charnot_repeat.c == d
                                || (*F).fields.charnot_repeat.oc == d
                            {
                                RRETURN!(MATCH_NOMATCH);
                            }
                            i += 1;
                        }
                    } else {
                        i = 1;
                        while i <= lmin {
                            if Feptr!(F) >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                RRETURN!(MATCH_NOMATCH);
                            }
                            let d = *Feptr!(F) as u32;
                            if (*F).fields.charnot_repeat.c == d
                                || (*F).fields.charnot_repeat.oc == d
                            {
                                RRETURN!(MATCH_NOMATCH);
                            }
                            Feptr!(F) = Feptr!(F).add(1);
                            i += 1;
                        }
                    }

                    if (*F).fields.charnot_repeat.min == (*F).fields.charnot_repeat.max {
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    if reptype == REPTYPE_MIN {
                        if utf != FALSE {
                            RMATCH!(Fecode!(F), 204);
                        } else {
                            RMATCH!(Fecode!(F), 29);
                        }
                    } else {
                        (*F).fields.charnot_repeat.start_eptr = Feptr!(F);
                        if utf != FALSE {
                            i = (*F).fields.charnot_repeat.min;
                            let lmax = (*F).fields.charnot_repeat.max;
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (d, extra) = getcharlen(Feptr!(F));
                                len += extra;
                                if (*F).fields.charnot_repeat.c == d
                                    || (*F).fields.charnot_repeat.oc == d
                                {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                            if reptype != REPTYPE_POS {
                                if Feptr!(F) > (*F).fields.charnot_repeat.start_eptr {
                                    RMATCH!(Fecode!(F), 205);
                                }
                            }
                            state = ST_MAIN_LOOP;
                            continue 'dispatch;
                        } else {
                            i = (*F).fields.charnot_repeat.min;
                            let lmax = (*F).fields.charnot_repeat.max;
                            while i < lmax {
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let d = *Feptr!(F) as u32;
                                if (*F).fields.charnot_repeat.c == d
                                    || (*F).fields.charnot_repeat.oc == d
                                {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(1);
                                i += 1;
                            }
                            if reptype != REPTYPE_POS {
                                if Feptr!(F) != (*F).fields.charnot_repeat.start_eptr {
                                    RMATCH!(Fecode!(F), 30);
                                }
                            }
                            state = ST_MAIN_LOOP;
                            continue 'dispatch;
                        }
                    }
                } else {
                    /* Caseful */
                    let lmin = (*F).fields.charnot_repeat.min;
                    if utf != FALSE {
                        i = 1;
                        while i <= lmin {
                            if Feptr!(F) >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                RRETURN!(MATCH_NOMATCH);
                            }
                            let d = getcharinc(&mut Feptr!(F));
                            if (*F).fields.charnot_repeat.c == d {
                                RRETURN!(MATCH_NOMATCH);
                            }
                            i += 1;
                        }
                    } else {
                        i = 1;
                        while i <= lmin {
                            if Feptr!(F) >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                RRETURN!(MATCH_NOMATCH);
                            }
                            let d = *Feptr!(F) as u32;
                            Feptr!(F) = Feptr!(F).add(1);
                            if (*F).fields.charnot_repeat.c == d {
                                RRETURN!(MATCH_NOMATCH);
                            }
                            i += 1;
                        }
                    }

                    if (*F).fields.charnot_repeat.min == (*F).fields.charnot_repeat.max {
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    if reptype == REPTYPE_MIN {
                        if utf != FALSE {
                            RMATCH!(Fecode!(F), 206);
                        } else {
                            RMATCH!(Fecode!(F), 31);
                        }
                    } else {
                        (*F).fields.charnot_repeat.start_eptr = Feptr!(F);
                        if utf != FALSE {
                            i = (*F).fields.charnot_repeat.min;
                            let lmax = (*F).fields.charnot_repeat.max;
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (d, extra) = getcharlen(Feptr!(F));
                                len += extra;
                                if (*F).fields.charnot_repeat.c == d {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                            if reptype != REPTYPE_POS {
                                if Feptr!(F) > (*F).fields.charnot_repeat.start_eptr {
                                    RMATCH!(Fecode!(F), 207);
                                }
                            }
                            state = ST_MAIN_LOOP;
                            continue 'dispatch;
                        } else {
                            i = (*F).fields.charnot_repeat.min;
                            let lmax = (*F).fields.charnot_repeat.max;
                            while i < lmax {
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                if (*F).fields.charnot_repeat.c == *Feptr!(F) as u32 {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(1);
                                i += 1;
                            }
                            if reptype != REPTYPE_POS {
                                if Feptr!(F) != (*F).fields.charnot_repeat.start_eptr {
                                    RMATCH!(Fecode!(F), 32);
                                }
                            }
                            state = ST_MAIN_LOOP;
                            continue 'dispatch;
                        }
                    }
                }
            }

            /* Resume: minimize caseless not-char, UTF (RM204). */
            ST_L204 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.charnot_repeat.min;
                    (*F).fields.charnot_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.charnot_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                let d = getcharinc(&mut Feptr!(F));
                if (*F).fields.charnot_repeat.c == d || (*F).fields.charnot_repeat.oc == d {
                    RRETURN!(MATCH_NOMATCH);
                }
                RMATCH!(Fecode!(F), 204);
            }

            /* Resume: minimize caseless not-char, non-UTF (RM29). */
            ST_L29 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.charnot_repeat.min;
                    (*F).fields.charnot_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.charnot_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                let d = *Feptr!(F) as u32;
                if (*F).fields.charnot_repeat.c == d || (*F).fields.charnot_repeat.oc == d {
                    RRETURN!(MATCH_NOMATCH);
                }
                Feptr!(F) = Feptr!(F).add(1);
                RMATCH!(Fecode!(F), 29);
            }

            /* Resume: maximize caseless not-char backtrack, UTF (RM205). */
            ST_L205 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                Feptr!(F) = Feptr!(F).sub(1);
                backchar(&mut Feptr!(F));
                if Feptr!(F) <= (*F).fields.charnot_repeat.start_eptr {
                    state = ST_MAIN_LOOP;
                    continue 'dispatch;
                }
                RMATCH!(Fecode!(F), 205);
            }

            /* Resume: maximize caseless not-char backtrack, non-UTF (RM30). */
            ST_L30 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                Feptr!(F) = Feptr!(F).sub(1);
                if Feptr!(F) == (*F).fields.charnot_repeat.start_eptr {
                    state = ST_MAIN_LOOP;
                    continue 'dispatch;
                }
                RMATCH!(Fecode!(F), 30);
            }

            /* Resume: minimize caseful not-char, UTF (RM206). */
            ST_L206 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.charnot_repeat.min;
                    (*F).fields.charnot_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.charnot_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                let d = getcharinc(&mut Feptr!(F));
                if (*F).fields.charnot_repeat.c == d {
                    RRETURN!(MATCH_NOMATCH);
                }
                RMATCH!(Fecode!(F), 206);
            }

            /* Resume: minimize caseful not-char, non-UTF (RM31). */
            ST_L31 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.charnot_repeat.min;
                    (*F).fields.charnot_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.charnot_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                let d = *Feptr!(F) as u32;
                Feptr!(F) = Feptr!(F).add(1);
                if (*F).fields.charnot_repeat.c == d {
                    RRETURN!(MATCH_NOMATCH);
                }
                RMATCH!(Fecode!(F), 31);
            }

            /* Resume: maximize caseful not-char backtrack, UTF (RM207). */
            ST_L207 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                Feptr!(F) = Feptr!(F).sub(1);
                backchar(&mut Feptr!(F));
                if Feptr!(F) <= (*F).fields.charnot_repeat.start_eptr {
                    state = ST_MAIN_LOOP;
                    continue 'dispatch;
                }
                RMATCH!(Fecode!(F), 207);
            }

            /* Resume: maximize caseful not-char backtrack, non-UTF (RM32). */
            ST_L32 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                Feptr!(F) = Feptr!(F).sub(1);
                if Feptr!(F) == (*F).fields.charnot_repeat.start_eptr {
                    state = ST_MAIN_LOOP;
                    continue 'dispatch;
                }
                RMATCH!(Fecode!(F), 32);
            }

            /* Resume: minimize bitmap class, UTF (RM200). */
            ST_L200 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.class_repeat.min;
                    (*F).fields.class_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.class_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                fc = getcharinc(&mut Feptr!(F));
                if fc > 255 {
                    if Fop!(F) == OP_CLASS {
                        RRETURN!(MATCH_NOMATCH);
                    }
                } else {
                    let bm = (*F).fields.class_repeat.byte_map_address;
                    if (*bm.add((fc / 8) as usize) & (1u8 << (fc & 7))) == 0 {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }
                RMATCH!(Fecode!(F), 200);
            }

            /* Resume: minimize bitmap class, non-UTF (RM23). */
            ST_L23 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.class_repeat.min;
                    (*F).fields.class_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.class_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                fc = *Feptr!(F) as u32;
                Feptr!(F) = Feptr!(F).add(1);
                let bm = (*F).fields.class_repeat.byte_map_address;
                if (*bm.add((fc / 8) as usize) & (1u8 << (fc & 7))) == 0 {
                    RRETURN!(MATCH_NOMATCH);
                }
                RMATCH!(Fecode!(F), 23);
            }

            /* Resume: maximize bitmap class backtrack, UTF (RM201). */
            ST_L201 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                let old = Feptr!(F);
                Feptr!(F) = Feptr!(F).sub(1);
                if old <= (*F).fields.class_repeat.start_eptr {
                    RRETURN!(MATCH_NOMATCH);
                }
                backchar(&mut Feptr!(F));
                RMATCH!(Fecode!(F), 201);
            }

            /* Resume: maximize bitmap class backtrack, non-UTF (RM24). */
            ST_L24 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                Feptr!(F) = Feptr!(F).sub(1);
                if Feptr!(F) >= (*F).fields.class_repeat.start_eptr {
                    RMATCH!(Fecode!(F), 24);
                }
                RRETURN!(MATCH_NOMATCH);
            }

            /* Resume: minimize XCLASS (RM100). */
            ST_L100 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.xclass_repeat.min;
                    (*F).fields.xclass_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.xclass_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                if crate::xclass::xclass(
                    fc,
                    (*F).fields.xclass_repeat.xclass_data,
                    (*mb).start_code as *const u8,
                    utf,
                ) == FALSE
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                RMATCH!(Fecode!(F), 100);
            }

            /* Resume: maximize XCLASS backtrack (RM101). */
            ST_L101 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                let old = Feptr!(F);
                Feptr!(F) = Feptr!(F).sub(1);
                if old <= (*F).fields.xclass_repeat.start_eptr {
                    RRETURN!(MATCH_NOMATCH);
                }
                if utf != FALSE {
                    backchar(&mut Feptr!(F));
                }
                RMATCH!(Fecode!(F), 101);
            }

            /* Resume: minimize ECLASS (RM102). */
            ST_L102 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.eclass_repeat.min;
                    (*F).fields.eclass_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.eclass_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                if crate::xclass::eclass(
                    fc,
                    (*F).fields.eclass_repeat.eclass_data,
                    (*F).fields.eclass_repeat.eclass_data
                        .add((*F).fields.eclass_repeat.eclass_len),
                    (*mb).start_code as *const u8,
                    utf,
                ) == FALSE
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                RMATCH!(Fecode!(F), 102);
            }

            /* Resume: maximize ECLASS backtrack (RM103). */
            ST_L103 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                let old = Feptr!(F);
                Feptr!(F) = Feptr!(F).sub(1);
                if old <= (*F).fields.eclass_repeat.start_eptr {
                    RRETURN!(MATCH_NOMATCH);
                }
                if utf != FALSE {
                    backchar(&mut Feptr!(F));
                }
                RMATCH!(Fecode!(F), 103);
            }

            /* ---- REPEATTYPE: repeated single character type. ---- */
            ST_REPEATTYPE => {
                (*F).fields.type_repeat.ctype = *Fecode!(F) as u32;
                Fecode!(F) = Fecode!(F).add(1);

                if (*F).fields.type_repeat.ctype == OP_PROP as u32
                    || (*F).fields.type_repeat.ctype == OP_NOTPROP as u32
                {
                    proptype = *Fecode!(F) as c_int;
                    (*F).fields.type_repeat.propvalue = *Fecode!(F).add(1) as u32;
                    Fecode!(F) = Fecode!(F).add(2);
                } else {
                    proptype = -1;
                }

                let lmin = (*F).fields.type_repeat.min;
                let lctype = (*F).fields.type_repeat.ctype as u8;
                let lpropvalue = (*F).fields.type_repeat.propvalue;

                /* First, ensure the minimum number of matches are present. */
                if lmin > 0 {
                    if proptype >= 0 {
                        let notmatch: BOOL = (lctype == OP_NOTPROP) as BOOL;
                        match proptype as u32 {
                            PT_LAMP => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                                    let chartype = ucd_chartype(fc);
                                    if ((chartype == ucp_Lu
                                        || chartype == ucp_Ll
                                        || chartype == ucp_Lt)
                                        as BOOL
                                        == notmatch)
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            PT_GC => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                                    if ((ucd_category(fc) == lpropvalue) as BOOL == notmatch) {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            PT_PC => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                                    if ((ucd_chartype(fc) == lpropvalue) as BOOL == notmatch) {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            PT_SC => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                                    if ((ucd_script(fc) == lpropvalue) as BOOL == notmatch) {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            PT_SCX => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                                    let prop = get_ucd(fc);
                                    let ok: BOOL = ((prop.script as u32 == lpropvalue
                                        || mapbit(
                                            &UCD_SCRIPT_SETS[ucd_scriptx_prop(prop) as usize..],
                                            lpropvalue,
                                        ) != 0)
                                        as BOOL);
                                    if ok == notmatch {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            PT_ALNUM => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                                    let category = ucd_category(fc);
                                    if ((category == ucp_L || category == ucp_N) as BOOL
                                        == notmatch)
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            PT_SPACE | PT_PXSPACE => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                                    if is_hspace(fc) || is_vspace(fc) {
                                        if notmatch != FALSE {
                                            RRETURN!(MATCH_NOMATCH);
                                        }
                                    } else if ((ucd_category(fc) == ucp_Z) as BOOL == notmatch) {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            PT_WORD => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                                    let chartype = ucd_chartype(fc);
                                    let category = UCP_GENTYPE[chartype as usize];
                                    if ((category == ucp_L
                                        || category == ucp_N
                                        || chartype == ucp_Mn
                                        || chartype == ucp_Pc)
                                        as BOOL
                                        == notmatch)
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            PT_CLIST => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                                    let mut cp = &UCD_CASELESS_SETS[lpropvalue as usize..];
                                    loop {
                                        if fc < cp[0] {
                                            if notmatch != FALSE {
                                                break;
                                            }
                                            RRETURN!(MATCH_NOMATCH);
                                        }
                                        let v = cp[0];
                                        cp = &cp[1..];
                                        if fc == v {
                                            if notmatch != FALSE {
                                                RRETURN!(MATCH_NOMATCH);
                                            }
                                            break;
                                        }
                                    }
                                    i += 1;
                                }
                            }
                            PT_UCNC => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                                    if ((fc == CHAR_DOLLAR_SIGN
                                        || fc == CHAR_COMMERCIAL_AT
                                        || fc == CHAR_GRAVE_ACCENT
                                        || (fc >= 0xa0 && fc <= 0xd7ff)
                                        || fc >= 0xe000)
                                        as BOOL
                                        == notmatch)
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            PT_BIDICL => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                                    if ((ucd_bidiclass(fc) == lpropvalue) as BOOL == notmatch) {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            PT_BOOL => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                                    let prop = get_ucd(fc);
                                    let ok: BOOL = (mapbit(
                                        &UCD_BOOLPROP_SETS[ucd_bprops_prop(prop) as usize..],
                                        lpropvalue,
                                    ) != 0)
                                        as BOOL;
                                    if ok == notmatch {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            _ => {
                                return PCRE2_ERROR_INTERNAL;
                            }
                        }
                    } else if lctype == OP_EXTUNI {
                        i = 1;
                        while i <= lmin {
                            if Feptr!(F) >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                RRETURN!(MATCH_NOMATCH);
                            } else {
                                fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                                Feptr!(F) = crate::extuni::extuni(
                                    fc,
                                    Feptr!(F),
                                    (*mb).start_subject,
                                    (*mb).end_subject,
                                    utf,
                                    core::ptr::null_mut(),
                                );
                            }
                            CHECK_PARTIAL!();
                            i += 1;
                        }
                    } else if utf != FALSE {
                        if lctype == OP_ANYBYTE {
                            if Feptr!(F) > (*mb).end_subject.sub(lmin as usize) {
                                RRETURN!(MATCH_NOMATCH);
                            }
                            Feptr!(F) = Feptr!(F).add(lmin as usize);
                        } else {
                        i = 1;
                        while i <= lmin {
                            if Feptr!(F) >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                RRETURN!(MATCH_NOMATCH);
                            }
                            match lctype {
                                OP_ANY => {
                                    if IS_NEWLINE!(Feptr!(F)) {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    if (*mb).partial != 0
                                        && Feptr!(F).add(1) >= (*mb).end_subject
                                        && (*mb).nltype == NLTYPE_FIXED
                                        && (*mb).nllen == 2
                                        && *Feptr!(F) as u32 == (*mb).nl[0] as u32
                                    {
                                        (*mb).hitend = TRUE;
                                        if (*mb).partial > 1 {
                                            return PCRE2_ERROR_PARTIAL;
                                        }
                                    }
                                    Feptr!(F) = Feptr!(F).add(1);
                                    ACROSSCHAR!(Feptr!(F) < (*mb).end_subject, Feptr!(F));
                                }
                                OP_ALLANY => {
                                    Feptr!(F) = Feptr!(F).add(1);
                                    ACROSSCHAR!(Feptr!(F) < (*mb).end_subject, Feptr!(F));
                                }
                                OP_ANYNL => {
                                    fc = getcharinc(&mut Feptr!(F));
                                    match fc {
                                        CHAR_CR => {
                                            if Feptr!(F) < (*mb).end_subject
                                                && *Feptr!(F) as u32 == CHAR_LF
                                            {
                                                Feptr!(F) = Feptr!(F).add(1);
                                            }
                                        }
                                        CHAR_LF => {}
                                        CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                            if (*mb).bsr_convention
                                                == PCRE2_BSR_ANYCRLF as u16
                                            {
                                                RRETURN!(MATCH_NOMATCH);
                                            }
                                        }
                                        _ => {
                                            RRETURN!(MATCH_NOMATCH);
                                        }
                                    }
                                }
                                OP_NOT_HSPACE => {
                                    fc = getcharinc(&mut Feptr!(F));
                                    if is_hspace(fc) {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                OP_HSPACE => {
                                    fc = getcharinc(&mut Feptr!(F));
                                    if !is_hspace(fc) {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                OP_NOT_VSPACE => {
                                    fc = getcharinc(&mut Feptr!(F));
                                    if is_vspace(fc) {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                OP_VSPACE => {
                                    fc = getcharinc(&mut Feptr!(F));
                                    if !is_vspace(fc) {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                OP_NOT_DIGIT => {
                                    fc = getcharinc(&mut Feptr!(F));
                                    if fc < 128
                                        && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                }
                                OP_DIGIT => {
                                    let cc = *Feptr!(F) as u32;
                                    if cc >= 128
                                        || (*(*mb).ctypes.add(cc as usize) & ctype_digit) == 0
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    Feptr!(F) = Feptr!(F).add(1);
                                }
                                OP_NOT_WHITESPACE => {
                                    let cc = *Feptr!(F) as u32;
                                    if cc < 128
                                        && (*(*mb).ctypes.add(cc as usize) & ctype_space) != 0
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    Feptr!(F) = Feptr!(F).add(1);
                                    ACROSSCHAR!(Feptr!(F) < (*mb).end_subject, Feptr!(F));
                                }
                                OP_WHITESPACE => {
                                    let cc = *Feptr!(F) as u32;
                                    if cc >= 128
                                        || (*(*mb).ctypes.add(cc as usize) & ctype_space) == 0
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    Feptr!(F) = Feptr!(F).add(1);
                                }
                                OP_NOT_WORDCHAR => {
                                    let cc = *Feptr!(F) as u32;
                                    if cc < 128
                                        && (*(*mb).ctypes.add(cc as usize) & ctype_word) != 0
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    Feptr!(F) = Feptr!(F).add(1);
                                    ACROSSCHAR!(Feptr!(F) < (*mb).end_subject, Feptr!(F));
                                }
                                OP_WORDCHAR => {
                                    let cc = *Feptr!(F) as u32;
                                    if cc >= 128
                                        || (*(*mb).ctypes.add(cc as usize) & ctype_word) == 0
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    Feptr!(F) = Feptr!(F).add(1);
                                }
                                _ => {
                                    return PCRE2_ERROR_INTERNAL;
                                }
                            }
                            i += 1;
                        }
                        }
                    } else {
                        /* Non-UTF minimum matching. */
                        match lctype {
                            OP_ANY => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    if IS_NEWLINE!(Feptr!(F)) {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    if (*mb).partial != 0
                                        && Feptr!(F).add(1) >= (*mb).end_subject
                                        && (*mb).nltype == NLTYPE_FIXED
                                        && (*mb).nllen == 2
                                        && *Feptr!(F) as u32 == (*mb).nl[0] as u32
                                    {
                                        (*mb).hitend = TRUE;
                                        if (*mb).partial > 1 {
                                            return PCRE2_ERROR_PARTIAL;
                                        }
                                    }
                                    Feptr!(F) = Feptr!(F).add(1);
                                    i += 1;
                                }
                            }
                            OP_ALLANY => {
                                if Feptr!(F) > (*mb).end_subject.sub(lmin as usize) {
                                    SCHECK_PARTIAL!();
                                    RRETURN!(MATCH_NOMATCH);
                                }
                                Feptr!(F) = Feptr!(F).add(lmin as usize);
                            }
                            OP_ANYNL => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    let ch = *Feptr!(F) as u32;
                                    Feptr!(F) = Feptr!(F).add(1);
                                    match ch {
                                        CHAR_CR => {
                                            if Feptr!(F) < (*mb).end_subject
                                                && *Feptr!(F) as u32 == CHAR_LF
                                            {
                                                Feptr!(F) = Feptr!(F).add(1);
                                            }
                                        }
                                        CHAR_LF => {}
                                        CHAR_VT | CHAR_FF | CHAR_NEL => {
                                            if (*mb).bsr_convention
                                                == PCRE2_BSR_ANYCRLF as u16
                                            {
                                                RRETURN!(MATCH_NOMATCH);
                                            }
                                        }
                                        _ => {
                                            RRETURN!(MATCH_NOMATCH);
                                        }
                                    }
                                    i += 1;
                                }
                            }
                            OP_NOT_HSPACE => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    let ch = *Feptr!(F) as u32;
                                    Feptr!(F) = Feptr!(F).add(1);
                                    if is_hspace(ch) {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            OP_HSPACE => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    let ch = *Feptr!(F) as u32;
                                    Feptr!(F) = Feptr!(F).add(1);
                                    if !is_hspace(ch) {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            OP_NOT_VSPACE => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    let ch = *Feptr!(F) as u32;
                                    Feptr!(F) = Feptr!(F).add(1);
                                    if is_vspace(ch) {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            OP_VSPACE => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    let ch = *Feptr!(F) as u32;
                                    Feptr!(F) = Feptr!(F).add(1);
                                    if !is_vspace(ch) {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            OP_NOT_DIGIT => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    if max_255(*Feptr!(F) as u32)
                                        && (*(*mb).ctypes.add(*Feptr!(F) as usize) & ctype_digit)
                                            != 0
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    Feptr!(F) = Feptr!(F).add(1);
                                    i += 1;
                                }
                            }
                            OP_DIGIT => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    if !max_255(*Feptr!(F) as u32)
                                        || (*(*mb).ctypes.add(*Feptr!(F) as usize) & ctype_digit)
                                            == 0
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    Feptr!(F) = Feptr!(F).add(1);
                                    i += 1;
                                }
                            }
                            OP_NOT_WHITESPACE => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    if max_255(*Feptr!(F) as u32)
                                        && (*(*mb).ctypes.add(*Feptr!(F) as usize) & ctype_space)
                                            != 0
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    Feptr!(F) = Feptr!(F).add(1);
                                    i += 1;
                                }
                            }
                            OP_WHITESPACE => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    if !max_255(*Feptr!(F) as u32)
                                        || (*(*mb).ctypes.add(*Feptr!(F) as usize) & ctype_space)
                                            == 0
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    Feptr!(F) = Feptr!(F).add(1);
                                    i += 1;
                                }
                            }
                            OP_NOT_WORDCHAR => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    if max_255(*Feptr!(F) as u32)
                                        && (*(*mb).ctypes.add(*Feptr!(F) as usize) & ctype_word)
                                            != 0
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    Feptr!(F) = Feptr!(F).add(1);
                                    i += 1;
                                }
                            }
                            OP_WORDCHAR => {
                                i = 1;
                                while i <= lmin {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    if !max_255(*Feptr!(F) as u32)
                                        || (*(*mb).ctypes.add(*Feptr!(F) as usize) & ctype_word)
                                            == 0
                                    {
                                        RRETURN!(MATCH_NOMATCH);
                                    }
                                    Feptr!(F) = Feptr!(F).add(1);
                                    i += 1;
                                }
                            }
                            _ => {
                                return PCRE2_ERROR_INTERNAL;
                            }
                        }
                    }
                }

                /* If Lmin == Lmax we are done. */
                if (*F).fields.type_repeat.min == (*F).fields.type_repeat.max {
                    state = ST_MAIN_LOOP;
                    continue 'dispatch;
                }

                if reptype == REPTYPE_MIN {
                    state = ST_REPEATTYPE_MIN;
                    continue 'dispatch;
                } else {
                    (*F).fields.type_repeat.start_eptr = Feptr!(F);
                    state = ST_REPEATTYPE_MAX;
                    continue 'dispatch;
                }
            }

            /* ---- REPEATTYPE minimizing: initial RMATCH dispatch. ---- */
            ST_REPEATTYPE_MIN => {
                if proptype >= 0 {
                    match proptype as u32 {
                        PT_LAMP => RMATCH!(Fecode!(F), 208),
                        PT_GC => RMATCH!(Fecode!(F), 209),
                        PT_PC => RMATCH!(Fecode!(F), 210),
                        PT_SC => RMATCH!(Fecode!(F), 211),
                        PT_SCX => RMATCH!(Fecode!(F), 224),
                        PT_ALNUM => RMATCH!(Fecode!(F), 212),
                        PT_SPACE | PT_PXSPACE => RMATCH!(Fecode!(F), 213),
                        PT_WORD => RMATCH!(Fecode!(F), 214),
                        PT_CLIST => RMATCH!(Fecode!(F), 215),
                        PT_UCNC => RMATCH!(Fecode!(F), 216),
                        PT_BIDICL => RMATCH!(Fecode!(F), 223),
                        PT_BOOL => RMATCH!(Fecode!(F), 222),
                        _ => {
                            return PCRE2_ERROR_INTERNAL;
                        }
                    }
                } else if (*F).fields.type_repeat.ctype as u8 == OP_EXTUNI {
                    RMATCH!(Fecode!(F), 217);
                } else if utf != FALSE {
                    RMATCH!(Fecode!(F), 218);
                } else {
                    RMATCH!(Fecode!(F), 33);
                }
            }

            /* Resume: REPEATTYPE minimize PT_LAMP (RM208). */
            ST_L208 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.type_repeat.min;
                    (*F).fields.type_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.type_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                let chartype = ucd_chartype(fc);
                let notmatch = ((*F).fields.type_repeat.ctype as u8 == OP_NOTPROP) as BOOL;
                if ((chartype == ucp_Lu || chartype == ucp_Ll || chartype == ucp_Lt) as BOOL
                    == notmatch)
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                RMATCH!(Fecode!(F), 208);
            }

            /* Resume: REPEATTYPE minimize PT_GC (RM209). */
            ST_L209 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.type_repeat.min;
                    (*F).fields.type_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.type_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                let notmatch = ((*F).fields.type_repeat.ctype as u8 == OP_NOTPROP) as BOOL;
                if ((ucd_category(fc) == (*F).fields.type_repeat.propvalue) as BOOL == notmatch) {
                    RRETURN!(MATCH_NOMATCH);
                }
                RMATCH!(Fecode!(F), 209);
            }

            /* Resume: REPEATTYPE minimize PT_PC (RM210). */
            ST_L210 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.type_repeat.min;
                    (*F).fields.type_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.type_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                let notmatch = ((*F).fields.type_repeat.ctype as u8 == OP_NOTPROP) as BOOL;
                if ((ucd_chartype(fc) == (*F).fields.type_repeat.propvalue) as BOOL == notmatch) {
                    RRETURN!(MATCH_NOMATCH);
                }
                RMATCH!(Fecode!(F), 210);
            }

            /* Resume: REPEATTYPE minimize PT_SC (RM211). */
            ST_L211 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.type_repeat.min;
                    (*F).fields.type_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.type_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                let notmatch = ((*F).fields.type_repeat.ctype as u8 == OP_NOTPROP) as BOOL;
                if ((ucd_script(fc) == (*F).fields.type_repeat.propvalue) as BOOL == notmatch) {
                    RRETURN!(MATCH_NOMATCH);
                }
                RMATCH!(Fecode!(F), 211);
            }

            /* Resume: REPEATTYPE minimize PT_SCX (RM224). */
            ST_L224 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.type_repeat.min;
                    (*F).fields.type_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.type_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                let prop = get_ucd(fc);
                let notmatch = ((*F).fields.type_repeat.ctype as u8 == OP_NOTPROP) as BOOL;
                let ok: BOOL = ((prop.script as u32 == (*F).fields.type_repeat.propvalue
                    || mapbit(
                        &UCD_SCRIPT_SETS[ucd_scriptx_prop(prop) as usize..],
                        (*F).fields.type_repeat.propvalue,
                    ) != 0) as BOOL);
                if ok == notmatch {
                    RRETURN!(MATCH_NOMATCH);
                }
                RMATCH!(Fecode!(F), 224);
            }

            /* Resume: REPEATTYPE minimize PT_ALNUM (RM212). */
            ST_L212 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.type_repeat.min;
                    (*F).fields.type_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.type_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                let category = ucd_category(fc);
                let notmatch = ((*F).fields.type_repeat.ctype as u8 == OP_NOTPROP) as BOOL;
                if ((category == ucp_L || category == ucp_N) as BOOL == notmatch) {
                    RRETURN!(MATCH_NOMATCH);
                }
                RMATCH!(Fecode!(F), 212);
            }

            /* Resume: REPEATTYPE minimize PT_SPACE/PXSPACE (RM213). */
            ST_L213 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.type_repeat.min;
                    (*F).fields.type_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.type_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                let notmatch = ((*F).fields.type_repeat.ctype as u8 == OP_NOTPROP) as BOOL;
                if is_hspace(fc) || is_vspace(fc) {
                    if notmatch != FALSE {
                        RRETURN!(MATCH_NOMATCH);
                    }
                } else if ((ucd_category(fc) == ucp_Z) as BOOL == notmatch) {
                    RRETURN!(MATCH_NOMATCH);
                }
                RMATCH!(Fecode!(F), 213);
            }

            /* Resume: REPEATTYPE minimize PT_WORD (RM214). */
            ST_L214 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.type_repeat.min;
                    (*F).fields.type_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.type_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                let chartype = ucd_chartype(fc);
                let category = UCP_GENTYPE[chartype as usize];
                let notmatch = ((*F).fields.type_repeat.ctype as u8 == OP_NOTPROP) as BOOL;
                if ((category == ucp_L
                    || category == ucp_N
                    || chartype == ucp_Mn
                    || chartype == ucp_Pc) as BOOL
                    == notmatch)
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                RMATCH!(Fecode!(F), 214);
            }

            /* Resume: REPEATTYPE minimize PT_CLIST (RM215). */
            ST_L215 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.type_repeat.min;
                    (*F).fields.type_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.type_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                let notmatch = ((*F).fields.type_repeat.ctype as u8 == OP_NOTPROP) as BOOL;
                let mut cp = &UCD_CASELESS_SETS[(*F).fields.type_repeat.propvalue as usize..];
                loop {
                    if fc < cp[0] {
                        if notmatch != FALSE {
                            break;
                        }
                        RRETURN!(MATCH_NOMATCH);
                    }
                    let v = cp[0];
                    cp = &cp[1..];
                    if fc == v {
                        if notmatch != FALSE {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        break;
                    }
                }
                RMATCH!(Fecode!(F), 215);
            }

            /* Resume: REPEATTYPE minimize PT_UCNC (RM216). */
            ST_L216 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.type_repeat.min;
                    (*F).fields.type_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.type_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                let notmatch = ((*F).fields.type_repeat.ctype as u8 == OP_NOTPROP) as BOOL;
                if ((fc == CHAR_DOLLAR_SIGN
                    || fc == CHAR_COMMERCIAL_AT
                    || fc == CHAR_GRAVE_ACCENT
                    || (fc >= 0xa0 && fc <= 0xd7ff)
                    || fc >= 0xe000) as BOOL
                    == notmatch)
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                RMATCH!(Fecode!(F), 216);
            }

            /* Resume: REPEATTYPE minimize PT_BIDICL (RM223). */
            ST_L223 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.type_repeat.min;
                    (*F).fields.type_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.type_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                let notmatch = ((*F).fields.type_repeat.ctype as u8 == OP_NOTPROP) as BOOL;
                if ((ucd_bidiclass(fc) == (*F).fields.type_repeat.propvalue) as BOOL == notmatch)
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                RMATCH!(Fecode!(F), 223);
            }

            /* Resume: REPEATTYPE minimize PT_BOOL (RM222). */
            ST_L222 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.type_repeat.min;
                    (*F).fields.type_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.type_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                let prop = get_ucd(fc);
                let notmatch = ((*F).fields.type_repeat.ctype as u8 == OP_NOTPROP) as BOOL;
                let ok: BOOL = (mapbit(
                    &UCD_BOOLPROP_SETS[ucd_bprops_prop(prop) as usize..],
                    (*F).fields.type_repeat.propvalue,
                ) != 0) as BOOL;
                if ok == notmatch {
                    RRETURN!(MATCH_NOMATCH);
                }
                RMATCH!(Fecode!(F), 222);
            }

            /* Resume: REPEATTYPE minimize EXTUNI (RM217). */
            ST_L217 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.type_repeat.min;
                    (*F).fields.type_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.type_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                } else {
                    fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                    Feptr!(F) = crate::extuni::extuni(
                        fc,
                        Feptr!(F),
                        (*mb).start_subject,
                        (*mb).end_subject,
                        utf,
                        core::ptr::null_mut(),
                    );
                }
                CHECK_PARTIAL!();
                RMATCH!(Fecode!(F), 217);
            }

            /* Resume: REPEATTYPE minimize non-property, UTF (RM218). */
            ST_L218 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.type_repeat.min;
                    (*F).fields.type_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.type_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                let lctype = (*F).fields.type_repeat.ctype as u8;
                if lctype == OP_ANY && IS_NEWLINE!(Feptr!(F)) {
                    RRETURN!(MATCH_NOMATCH);
                }
                fc = getcharinc(&mut Feptr!(F));
                match lctype {
                    OP_ANY => {
                        if (*mb).partial != 0
                            && Feptr!(F) >= (*mb).end_subject
                            && (*mb).nltype == NLTYPE_FIXED
                            && (*mb).nllen == 2
                            && fc == (*mb).nl[0] as u32
                        {
                            (*mb).hitend = TRUE;
                            if (*mb).partial > 1 {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                    }
                    OP_ALLANY | OP_ANYBYTE => {}
                    OP_ANYNL => match fc {
                        CHAR_CR => {
                            if Feptr!(F) < (*mb).end_subject && *Feptr!(F) as u32 == CHAR_LF {
                                Feptr!(F) = Feptr!(F).add(1);
                            }
                        }
                        CHAR_LF => {}
                        CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                            if (*mb).bsr_convention == PCRE2_BSR_ANYCRLF as u16 {
                                RRETURN!(MATCH_NOMATCH);
                            }
                        }
                        _ => {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    },
                    OP_NOT_HSPACE => {
                        if is_hspace(fc) {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    OP_HSPACE => {
                        if !is_hspace(fc) {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    OP_NOT_VSPACE => {
                        if is_vspace(fc) {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    OP_VSPACE => {
                        if !is_vspace(fc) {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    OP_NOT_DIGIT => {
                        if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    OP_DIGIT => {
                        if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    OP_NOT_WHITESPACE => {
                        if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    OP_WHITESPACE => {
                        if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    OP_NOT_WORDCHAR => {
                        if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    OP_WORDCHAR => {
                        if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    _ => {
                        return PCRE2_ERROR_INTERNAL;
                    }
                }
                RMATCH!(Fecode!(F), 218);
            }

            /* Resume: REPEATTYPE minimize non-property, non-UTF (RM33). */
            ST_L33 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.type_repeat.min;
                    (*F).fields.type_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.type_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                if Feptr!(F) >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                let lctype = (*F).fields.type_repeat.ctype as u8;
                if lctype == OP_ANY && IS_NEWLINE!(Feptr!(F)) {
                    RRETURN!(MATCH_NOMATCH);
                }
                fc = *Feptr!(F) as u32;
                Feptr!(F) = Feptr!(F).add(1);
                match lctype {
                    OP_ANY => {
                        if (*mb).partial != 0
                            && Feptr!(F) >= (*mb).end_subject
                            && (*mb).nltype == NLTYPE_FIXED
                            && (*mb).nllen == 2
                            && fc == (*mb).nl[0] as u32
                        {
                            (*mb).hitend = TRUE;
                            if (*mb).partial > 1 {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                    }
                    OP_ALLANY | OP_ANYBYTE => {}
                    OP_ANYNL => match fc {
                        CHAR_CR => {
                            if Feptr!(F) < (*mb).end_subject && *Feptr!(F) as u32 == CHAR_LF {
                                Feptr!(F) = Feptr!(F).add(1);
                            }
                        }
                        CHAR_LF => {}
                        CHAR_VT | CHAR_FF | CHAR_NEL => {
                            if (*mb).bsr_convention == PCRE2_BSR_ANYCRLF as u16 {
                                RRETURN!(MATCH_NOMATCH);
                            }
                        }
                        _ => {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    },
                    OP_NOT_HSPACE => {
                        if is_hspace(fc) {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    OP_HSPACE => {
                        if !is_hspace(fc) {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    OP_NOT_VSPACE => {
                        if is_vspace(fc) {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    OP_VSPACE => {
                        if !is_vspace(fc) {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    OP_NOT_DIGIT => {
                        if max_255(fc) && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    OP_DIGIT => {
                        if !max_255(fc) || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    OP_NOT_WHITESPACE => {
                        if max_255(fc) && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    OP_WHITESPACE => {
                        if !max_255(fc) || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    OP_NOT_WORDCHAR => {
                        if max_255(fc) && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    OP_WORDCHAR => {
                        if !max_255(fc) || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    _ => {
                        return PCRE2_ERROR_INTERNAL;
                    }
                }
                RMATCH!(Fecode!(F), 33);
            }

            /* ---- REPEATTYPE maximizing: inline scan then backtrack. ---- */
            ST_REPEATTYPE_MAX => {
                let lctype = (*F).fields.type_repeat.ctype as u8;
                let lmax = (*F).fields.type_repeat.max;
                let lpropvalue = (*F).fields.type_repeat.propvalue;

                if proptype >= 0 {
                    let notmatch: BOOL = (lctype == OP_NOTPROP) as BOOL;
                    i = (*F).fields.type_repeat.min;
                    match proptype as u32 {
                        PT_LAMP => {
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlentest(Feptr!(F), utf != FALSE);
                                fc = ch;
                                len += extra;
                                let chartype = ucd_chartype(fc);
                                if ((chartype == ucp_Lu || chartype == ucp_Ll || chartype == ucp_Lt)
                                    as BOOL
                                    == notmatch)
                                {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                        }
                        PT_GC => {
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlentest(Feptr!(F), utf != FALSE);
                                fc = ch;
                                len += extra;
                                if ((ucd_category(fc) == lpropvalue) as BOOL == notmatch) {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                        }
                        PT_PC => {
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlentest(Feptr!(F), utf != FALSE);
                                fc = ch;
                                len += extra;
                                if ((ucd_chartype(fc) == lpropvalue) as BOOL == notmatch) {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                        }
                        PT_SC => {
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlentest(Feptr!(F), utf != FALSE);
                                fc = ch;
                                len += extra;
                                if ((ucd_script(fc) == lpropvalue) as BOOL == notmatch) {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                        }
                        PT_SCX => {
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlentest(Feptr!(F), utf != FALSE);
                                fc = ch;
                                len += extra;
                                let prop = get_ucd(fc);
                                let ok: BOOL = ((prop.script as u32 == lpropvalue
                                    || mapbit(
                                        &UCD_SCRIPT_SETS[ucd_scriptx_prop(prop) as usize..],
                                        lpropvalue,
                                    ) != 0) as BOOL);
                                if ok == notmatch {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                        }
                        PT_ALNUM => {
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlentest(Feptr!(F), utf != FALSE);
                                fc = ch;
                                len += extra;
                                let category = ucd_category(fc);
                                if ((category == ucp_L || category == ucp_N) as BOOL == notmatch) {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                        }
                        PT_SPACE | PT_PXSPACE => {
                            'endloop99: while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlentest(Feptr!(F), utf != FALSE);
                                fc = ch;
                                len += extra;
                                if is_hspace(fc) || is_vspace(fc) {
                                    if notmatch != FALSE {
                                        break 'endloop99;
                                    }
                                } else if ((ucd_category(fc) == ucp_Z) as BOOL == notmatch) {
                                    break 'endloop99;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                        }
                        PT_WORD => {
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlentest(Feptr!(F), utf != FALSE);
                                fc = ch;
                                len += extra;
                                let chartype = ucd_chartype(fc);
                                let category = UCP_GENTYPE[chartype as usize];
                                if ((category == ucp_L
                                    || category == ucp_N
                                    || chartype == ucp_Mn
                                    || chartype == ucp_Pc) as BOOL
                                    == notmatch)
                                {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                        }
                        PT_CLIST => {
                            'got_max: while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlentest(Feptr!(F), utf != FALSE);
                                fc = ch;
                                len += extra;
                                let mut cp = &UCD_CASELESS_SETS[lpropvalue as usize..];
                                loop {
                                    if fc < cp[0] {
                                        if notmatch != FALSE {
                                            break;
                                        } else {
                                            break 'got_max;
                                        }
                                    }
                                    let v = cp[0];
                                    cp = &cp[1..];
                                    if fc == v {
                                        if notmatch != FALSE {
                                            break 'got_max;
                                        } else {
                                            break;
                                        }
                                    }
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                        }
                        PT_UCNC => {
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlentest(Feptr!(F), utf != FALSE);
                                fc = ch;
                                len += extra;
                                if ((fc == CHAR_DOLLAR_SIGN
                                    || fc == CHAR_COMMERCIAL_AT
                                    || fc == CHAR_GRAVE_ACCENT
                                    || (fc >= 0xa0 && fc <= 0xd7ff)
                                    || fc >= 0xe000) as BOOL
                                    == notmatch)
                                {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                        }
                        PT_BIDICL => {
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlentest(Feptr!(F), utf != FALSE);
                                fc = ch;
                                len += extra;
                                if ((ucd_bidiclass(fc) == lpropvalue) as BOOL == notmatch) {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                        }
                        PT_BOOL => {
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlentest(Feptr!(F), utf != FALSE);
                                fc = ch;
                                len += extra;
                                let prop = get_ucd(fc);
                                let ok: BOOL = (mapbit(
                                    &UCD_BOOLPROP_SETS[ucd_bprops_prop(prop) as usize..],
                                    lpropvalue,
                                ) != 0) as BOOL;
                                if ok == notmatch {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                        }
                        _ => {
                            return PCRE2_ERROR_INTERNAL;
                        }
                    }

                    if reptype == REPTYPE_POS {
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                    /* Backtrack (RM221). */
                    if Feptr!(F) <= (*F).fields.type_repeat.start_eptr {
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                    RMATCH!(Fecode!(F), 221);
                } else if lctype == OP_EXTUNI {
                    i = (*F).fields.type_repeat.min;
                    while i < lmax {
                        if Feptr!(F) >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        } else {
                            fc = getcharinctest(&mut Feptr!(F), utf != FALSE);
                            Feptr!(F) = crate::extuni::extuni(
                                fc,
                                Feptr!(F),
                                (*mb).start_subject,
                                (*mb).end_subject,
                                utf,
                                core::ptr::null_mut(),
                            );
                        }
                        CHECK_PARTIAL!();
                        i += 1;
                    }

                    if reptype == REPTYPE_POS {
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                    if Feptr!(F) <= (*F).fields.type_repeat.start_eptr {
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                    RMATCH!(Fecode!(F), 219);
                } else if utf != FALSE {
                    i = (*F).fields.type_repeat.min;
                    match lctype {
                        OP_ANY => {
                            while i < lmax {
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                if IS_NEWLINE!(Feptr!(F)) {
                                    break;
                                }
                                if (*mb).partial != 0
                                    && Feptr!(F).add(1) >= (*mb).end_subject
                                    && (*mb).nltype == NLTYPE_FIXED
                                    && (*mb).nllen == 2
                                    && *Feptr!(F) as u32 == (*mb).nl[0] as u32
                                {
                                    (*mb).hitend = TRUE;
                                    if (*mb).partial > 1 {
                                        return PCRE2_ERROR_PARTIAL;
                                    }
                                }
                                Feptr!(F) = Feptr!(F).add(1);
                                ACROSSCHAR!(Feptr!(F) < (*mb).end_subject, Feptr!(F));
                                i += 1;
                            }
                        }
                        OP_ALLANY => {
                            if lmax < UINT32_MAX {
                                while i < lmax {
                                    if Feptr!(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        break;
                                    }
                                    Feptr!(F) = Feptr!(F).add(1);
                                    ACROSSCHAR!(Feptr!(F) < (*mb).end_subject, Feptr!(F));
                                    i += 1;
                                }
                            } else {
                                Feptr!(F) = (*mb).end_subject;
                                SCHECK_PARTIAL!();
                            }
                        }
                        OP_ANYBYTE => {
                            fc = lmax - (*F).fields.type_repeat.min;
                            if fc as isize
                                > (*mb).end_subject.offset_from(Feptr!(F))
                            {
                                Feptr!(F) = (*mb).end_subject;
                                SCHECK_PARTIAL!();
                            } else {
                                Feptr!(F) = Feptr!(F).add(fc as usize);
                            }
                        }
                        OP_ANYNL => {
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlen(Feptr!(F));
                                fc = ch;
                                len += extra;
                                if fc == CHAR_CR {
                                    Feptr!(F) = Feptr!(F).add(1);
                                    if Feptr!(F) >= (*mb).end_subject {
                                        break;
                                    }
                                    if *Feptr!(F) as u32 == CHAR_LF {
                                        Feptr!(F) = Feptr!(F).add(1);
                                    }
                                } else {
                                    if fc != CHAR_LF
                                        && ((*mb).bsr_convention == PCRE2_BSR_ANYCRLF as u16
                                            || (fc != CHAR_VT
                                                && fc != CHAR_FF
                                                && fc != CHAR_NEL
                                                && fc != 0x2028
                                                && fc != 0x2029))
                                    {
                                        break;
                                    }
                                    Feptr!(F) = Feptr!(F).add(len as usize);
                                }
                                i += 1;
                            }
                        }
                        OP_NOT_HSPACE | OP_HSPACE => {
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlen(Feptr!(F));
                                fc = ch;
                                len += extra;
                                let gotspace = is_hspace(fc);
                                if gotspace == (lctype == OP_NOT_HSPACE) {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                        }
                        OP_NOT_VSPACE | OP_VSPACE => {
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlen(Feptr!(F));
                                fc = ch;
                                len += extra;
                                let gotspace = is_vspace(fc);
                                if gotspace == (lctype == OP_NOT_VSPACE) {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                        }
                        OP_NOT_DIGIT => {
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlen(Feptr!(F));
                                fc = ch;
                                len += extra;
                                if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                        }
                        OP_DIGIT => {
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlen(Feptr!(F));
                                fc = ch;
                                len += extra;
                                if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                        }
                        OP_NOT_WHITESPACE => {
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlen(Feptr!(F));
                                fc = ch;
                                len += extra;
                                if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                        }
                        OP_WHITESPACE => {
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlen(Feptr!(F));
                                fc = ch;
                                len += extra;
                                if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                        }
                        OP_NOT_WORDCHAR => {
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlen(Feptr!(F));
                                fc = ch;
                                len += extra;
                                if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                        }
                        OP_WORDCHAR => {
                            while i < lmax {
                                let mut len: u32 = 1;
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let (ch, extra) = getcharlen(Feptr!(F));
                                fc = ch;
                                len += extra;
                                if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(len as usize);
                                i += 1;
                            }
                        }
                        _ => {
                            return PCRE2_ERROR_INTERNAL;
                        }
                    }

                    if reptype == REPTYPE_POS {
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                    if Feptr!(F) <= (*F).fields.type_repeat.start_eptr {
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                    RMATCH!(Fecode!(F), 220);
                } else {
                    /* Not UTF mode. */
                    i = (*F).fields.type_repeat.min;
                    match lctype {
                        OP_ANY => {
                            while i < lmax {
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                if IS_NEWLINE!(Feptr!(F)) {
                                    break;
                                }
                                if (*mb).partial != 0
                                    && Feptr!(F).add(1) >= (*mb).end_subject
                                    && (*mb).nltype == NLTYPE_FIXED
                                    && (*mb).nllen == 2
                                    && *Feptr!(F) as u32 == (*mb).nl[0] as u32
                                {
                                    (*mb).hitend = TRUE;
                                    if (*mb).partial > 1 {
                                        return PCRE2_ERROR_PARTIAL;
                                    }
                                }
                                Feptr!(F) = Feptr!(F).add(1);
                                i += 1;
                            }
                        }
                        OP_ALLANY | OP_ANYBYTE => {
                            fc = lmax - (*F).fields.type_repeat.min;
                            if fc as isize > (*mb).end_subject.offset_from(Feptr!(F)) {
                                Feptr!(F) = (*mb).end_subject;
                                SCHECK_PARTIAL!();
                            } else {
                                Feptr!(F) = Feptr!(F).add(fc as usize);
                            }
                        }
                        OP_ANYNL => {
                            while i < lmax {
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                fc = *Feptr!(F) as u32;
                                if fc == CHAR_CR {
                                    Feptr!(F) = Feptr!(F).add(1);
                                    if Feptr!(F) >= (*mb).end_subject {
                                        break;
                                    }
                                    if *Feptr!(F) as u32 == CHAR_LF {
                                        Feptr!(F) = Feptr!(F).add(1);
                                    }
                                } else {
                                    if fc != CHAR_LF
                                        && ((*mb).bsr_convention == PCRE2_BSR_ANYCRLF as u16
                                            || (fc != CHAR_VT && fc != CHAR_FF && fc != CHAR_NEL))
                                    {
                                        break;
                                    }
                                    Feptr!(F) = Feptr!(F).add(1);
                                }
                                i += 1;
                            }
                        }
                        OP_NOT_HSPACE => {
                            'endloop00: while i < lmax {
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                if is_hspace(*Feptr!(F) as u32) {
                                    break 'endloop00;
                                }
                                Feptr!(F) = Feptr!(F).add(1);
                                i += 1;
                            }
                        }
                        OP_HSPACE => {
                            'endloop01: while i < lmax {
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                if !is_hspace(*Feptr!(F) as u32) {
                                    break 'endloop01;
                                }
                                Feptr!(F) = Feptr!(F).add(1);
                                i += 1;
                            }
                        }
                        OP_NOT_VSPACE => {
                            'endloop02: while i < lmax {
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                if is_vspace(*Feptr!(F) as u32) {
                                    break 'endloop02;
                                }
                                Feptr!(F) = Feptr!(F).add(1);
                                i += 1;
                            }
                        }
                        OP_VSPACE => {
                            'endloop03: while i < lmax {
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                if !is_vspace(*Feptr!(F) as u32) {
                                    break 'endloop03;
                                }
                                Feptr!(F) = Feptr!(F).add(1);
                                i += 1;
                            }
                        }
                        OP_NOT_DIGIT => {
                            while i < lmax {
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                if max_255(*Feptr!(F) as u32)
                                    && (*(*mb).ctypes.add(*Feptr!(F) as usize) & ctype_digit) != 0
                                {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(1);
                                i += 1;
                            }
                        }
                        OP_DIGIT => {
                            while i < lmax {
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                if !max_255(*Feptr!(F) as u32)
                                    || (*(*mb).ctypes.add(*Feptr!(F) as usize) & ctype_digit) == 0
                                {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(1);
                                i += 1;
                            }
                        }
                        OP_NOT_WHITESPACE => {
                            while i < lmax {
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                if max_255(*Feptr!(F) as u32)
                                    && (*(*mb).ctypes.add(*Feptr!(F) as usize) & ctype_space) != 0
                                {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(1);
                                i += 1;
                            }
                        }
                        OP_WHITESPACE => {
                            while i < lmax {
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                if !max_255(*Feptr!(F) as u32)
                                    || (*(*mb).ctypes.add(*Feptr!(F) as usize) & ctype_space) == 0
                                {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(1);
                                i += 1;
                            }
                        }
                        OP_NOT_WORDCHAR => {
                            while i < lmax {
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                if max_255(*Feptr!(F) as u32)
                                    && (*(*mb).ctypes.add(*Feptr!(F) as usize) & ctype_word) != 0
                                {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(1);
                                i += 1;
                            }
                        }
                        OP_WORDCHAR => {
                            while i < lmax {
                                if Feptr!(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                if !max_255(*Feptr!(F) as u32)
                                    || (*(*mb).ctypes.add(*Feptr!(F) as usize) & ctype_word) == 0
                                {
                                    break;
                                }
                                Feptr!(F) = Feptr!(F).add(1);
                                i += 1;
                            }
                        }
                        _ => {
                            return PCRE2_ERROR_INTERNAL;
                        }
                    }

                    if reptype == REPTYPE_POS {
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                    if Feptr!(F) == (*F).fields.type_repeat.start_eptr {
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                    RMATCH!(Fecode!(F), 34);
                }
            }

            /* Resume: REPEATTYPE maximize property backtrack (RM221). */
            ST_L221 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                Feptr!(F) = Feptr!(F).sub(1);
                if utf != FALSE {
                    backchar(&mut Feptr!(F));
                }
                if Feptr!(F) <= (*F).fields.type_repeat.start_eptr {
                    state = ST_MAIN_LOOP;
                    continue 'dispatch;
                }
                RMATCH!(Fecode!(F), 221);
            }

            /* Resume: REPEATTYPE maximize EXTUNI grapheme backtrack (RM219). */
            ST_L219 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                Feptr!(F) = Feptr!(F).sub(1);
                if utf == FALSE {
                    fc = *Feptr!(F) as u32;
                } else {
                    backchar(&mut Feptr!(F));
                    fc = getchar_(Feptr!(F));
                }
                let mut rgb = ucd_graphbreak(fc);

                loop {
                    if Feptr!(F) <= (*F).fields.type_repeat.start_eptr {
                        break;
                    }
                    let mut fptr: PCRE2_SPTR = Feptr!(F).sub(1);
                    if utf == FALSE {
                        fc = *fptr as u32;
                    } else {
                        backchar(&mut fptr);
                        fc = getchar_(fptr);
                    }
                    let lgb = ucd_graphbreak(fc);
                    if (UCP_GBTABLE[lgb as usize] & (1u32 << rgb)) == 0 {
                        break;
                    }
                    Feptr!(F) = fptr;
                    rgb = lgb;
                }

                if Feptr!(F) <= (*F).fields.type_repeat.start_eptr {
                    state = ST_MAIN_LOOP;
                    continue 'dispatch;
                }
                RMATCH!(Fecode!(F), 219);
            }

            /* Resume: REPEATTYPE maximize non-property backtrack, UTF (RM220). */
            ST_L220 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                Feptr!(F) = Feptr!(F).sub(1);
                backchar(&mut Feptr!(F));
                let lctype = (*F).fields.type_repeat.ctype as u8;
                if lctype == OP_ANYNL
                    && Feptr!(F) > (*F).fields.type_repeat.start_eptr
                    && *Feptr!(F) as u32 == CHAR_NL
                    && *Feptr!(F).sub(1) as u32 == CHAR_CR
                {
                    Feptr!(F) = Feptr!(F).sub(1);
                }
                if Feptr!(F) <= (*F).fields.type_repeat.start_eptr {
                    state = ST_MAIN_LOOP;
                    continue 'dispatch;
                }
                RMATCH!(Fecode!(F), 220);
            }

            /* Resume: REPEATTYPE maximize non-property backtrack, non-UTF (RM34). */
            ST_L34 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                Feptr!(F) = Feptr!(F).sub(1);
                let lctype = (*F).fields.type_repeat.ctype as u8;
                if lctype == OP_ANYNL
                    && Feptr!(F) > (*F).fields.type_repeat.start_eptr
                    && *Feptr!(F) as u32 == CHAR_LF
                    && *Feptr!(F).sub(1) as u32 == CHAR_CR
                {
                    Feptr!(F) = Feptr!(F).sub(1);
                }
                if Feptr!(F) == (*F).fields.type_repeat.start_eptr {
                    state = ST_MAIN_LOOP;
                    continue 'dispatch;
                }
                RMATCH!(Fecode!(F), 34);
            }

            /* ---- REF_REPEAT: back-reference, possibly repeated. ---- */
            ST_REF_REPEAT => {
                match *Fecode!(F) {
                    OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
                    | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
                        fc = (*Fecode!(F) - OP_CRSTAR) as u32;
                        Fecode!(F) = Fecode!(F).add(1);
                        (*F).fields.ref_repeat.min = rep_min[fc as usize];
                        (*F).fields.ref_repeat.max = rep_max[fc as usize];
                        reptype = rep_typ[fc as usize];
                    }
                    OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                        (*F).fields.ref_repeat.min = get2(Fecode!(F), 1);
                        (*F).fields.ref_repeat.max = get2(Fecode!(F), 1 + IMM2_SIZE);
                        reptype = rep_typ[(*Fecode!(F) - OP_CRSTAR) as usize];
                        if (*F).fields.ref_repeat.max == 0 {
                            (*F).fields.ref_repeat.max = UINT32_MAX;
                        }
                        Fecode!(F) = Fecode!(F).add(1 + 2 * IMM2_SIZE);
                    }
                    _ => {
                        /* No repeat follows. */
                        rrc = match_ref(
                            (*F).fields.ref_repeat.offset,
                            (*F).byte1 as BOOL,
                            (*F).byte2 as c_int,
                            F,
                            mb,
                            &mut length,
                        );
                        if rrc != 0 {
                            if rrc > 0 {
                                Feptr!(F) = (*mb).end_subject;
                            }
                            CHECK_PARTIAL!();
                            RRETURN!(MATCH_NOMATCH);
                        }
                        Feptr!(F) = Feptr!(F).add(length);
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                }

                let loffset = (*F).fields.ref_repeat.offset;
                if loffset < Foffset_top!(F) && *Fovector!(F).add(loffset) != PCRE2_UNSET {
                    if *Fovector!(F).add(loffset) == *Fovector!(F).add(loffset + 1) {
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                } else {
                    if (*F).fields.ref_repeat.min == 0
                        || ((*mb).poptions & PCRE2_MATCH_UNSET_BACKREF) != 0
                    {
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }
                }

                /* Ensure the minimum number of matches are present. */
                i = 1;
                let lmin = (*F).fields.ref_repeat.min;
                while i <= lmin {
                    let mut slength: PCRE2_SIZE = 0;
                    rrc = match_ref(
                        (*F).fields.ref_repeat.offset,
                        (*F).byte1 as BOOL,
                        (*F).byte2 as c_int,
                        F,
                        mb,
                        &mut slength,
                    );
                    if rrc != 0 {
                        if rrc > 0 {
                            Feptr!(F) = (*mb).end_subject;
                        }
                        CHECK_PARTIAL!();
                        RRETURN!(MATCH_NOMATCH);
                    }
                    Feptr!(F) = Feptr!(F).add(slength);
                    i += 1;
                }

                if (*F).fields.ref_repeat.min == (*F).fields.ref_repeat.max {
                    state = ST_MAIN_LOOP;
                    continue 'dispatch;
                }

                if reptype == REPTYPE_MIN {
                    RMATCH!(Fecode!(F), 20);
                } else {
                    /* Maximize. */
                    let mut samelengths: BOOL = TRUE;
                    (*F).fields.ref_repeat.start = Feptr!(F);
                    (*F).fields.ref_repeat.length = *Fovector!(F).add(loffset + 1)
                        - *Fovector!(F).add(loffset);

                    i = (*F).fields.ref_repeat.min;
                    let lmax = (*F).fields.ref_repeat.max;
                    while i < lmax {
                        let mut slength: PCRE2_SIZE = 0;
                        rrc = match_ref(
                            (*F).fields.ref_repeat.offset,
                            (*F).byte1 as BOOL,
                            (*F).byte2 as c_int,
                            F,
                            mb,
                            &mut slength,
                        );
                        if rrc != 0 {
                            if rrc > 0
                                && (*mb).partial != 0
                                && (*mb).end_subject > (*mb).start_used_ptr
                            {
                                (*mb).hitend = TRUE;
                                if (*mb).partial > 1 {
                                    return PCRE2_ERROR_PARTIAL;
                                }
                            }
                            break;
                        }
                        if slength != (*F).fields.ref_repeat.length {
                            samelengths = FALSE;
                        }
                        Feptr!(F) = Feptr!(F).add(slength);
                        i += 1;
                    }

                    if reptype == REPTYPE_POS {
                        state = ST_MAIN_LOOP;
                        continue 'dispatch;
                    }

                    if samelengths != FALSE {
                        if Feptr!(F) >= (*F).fields.ref_repeat.start {
                            RMATCH!(Fecode!(F), 21);
                        }
                        RRETURN!(MATCH_NOMATCH);
                    } else {
                        (*F).fields.ref_repeat.max = i;
                        RMATCH!(Fecode!(F), 22);
                    }
                }
            }

            /* Resume: REF_REPEAT minimize (RM20). */
            ST_L20 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.ref_repeat.min;
                    (*F).fields.ref_repeat.min = v.wrapping_add(1);
                    v
                } >= (*F).fields.ref_repeat.max
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                let mut slength: PCRE2_SIZE = 0;
                rrc = match_ref(
                    (*F).fields.ref_repeat.offset,
                    (*F).byte1 as BOOL,
                    (*F).byte2 as c_int,
                    F,
                    mb,
                    &mut slength,
                );
                if rrc != 0 {
                    if rrc > 0 {
                        Feptr!(F) = (*mb).end_subject;
                    }
                    CHECK_PARTIAL!();
                    RRETURN!(MATCH_NOMATCH);
                }
                Feptr!(F) = Feptr!(F).add(slength);
                RMATCH!(Fecode!(F), 20);
            }

            /* Resume: REF_REPEAT maximize, same lengths (RM21). */
            ST_L21 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                Feptr!(F) = Feptr!(F).sub((*F).fields.ref_repeat.length);
                if Feptr!(F) >= (*F).fields.ref_repeat.start {
                    RMATCH!(Fecode!(F), 21);
                }
                RRETURN!(MATCH_NOMATCH);
            }

            /* Resume: REF_REPEAT maximize, differing lengths (RM22). */
            ST_L22 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if Feptr!(F) == (*F).fields.ref_repeat.start {
                    RRETURN!(MATCH_NOMATCH);
                }
                Feptr!(F) = (*F).fields.ref_repeat.start;
                (*F).fields.ref_repeat.max -= 1;
                i = (*F).fields.ref_repeat.min;
                let lmax = (*F).fields.ref_repeat.max;
                while i < lmax {
                    let mut slength: PCRE2_SIZE = 0;
                    let _ = match_ref(
                        (*F).fields.ref_repeat.offset,
                        (*F).byte1 as BOOL,
                        (*F).byte2 as c_int,
                        F,
                        mb,
                        &mut slength,
                    );
                    Feptr!(F) = Feptr!(F).add(slength);
                    i += 1;
                }
                RMATCH!(Fecode!(F), 22);
            }

            /* ---- OP_BRA THEN-free branch loop. ---- */
            ST_L_BRA_LOOP => {
                let current_branch: PCRE2_SPTR = Fecode!(F);
                let next_branch: PCRE2_SPTR = current_branch.add(get(current_branch, 1) as usize);
                if *next_branch != OP_ALT {
                    Fecode!(F) = Fecode!(F).add(1 + LINK_SIZE);
                    state = ST_MAIN_LOOP;
                    continue 'dispatch;
                }
                Fecode!(F) = next_branch;
                RMATCH!(current_branch.add(1 + LINK_SIZE), 1);
            }

            /* Resume: OP_BRA branch loop (RM1). */
            ST_L1 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                state = ST_L_BRA_LOOP;
                continue 'dispatch;
            }

            /* Resume: OP_BRAZERO (RM9). */
            ST_L9 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                let mut next_ecode: PCRE2_SPTR = Fecode!(F);
                loop {
                    next_ecode = next_ecode.add(get(next_ecode, 1) as usize);
                    if *next_ecode != OP_ALT {
                        break;
                    }
                }
                Fecode!(F) = next_ecode.add(1 + LINK_SIZE);
                state = ST_MAIN_LOOP;
                continue 'dispatch;
            }

            /* Resume: OP_BRAMINZERO (RM10). */
            ST_L10 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                state = ST_MAIN_LOOP;
                continue 'dispatch;
            }

            /* ---- Possessive bracket group. ---- */
            ST_POSSESSIVE_NON_CAPTURE => {
                (*F).fields.op_brapos.frame_type = GF_NOCAPTURE;
                state = ST_POSSESSIVE_GROUP;
                continue 'dispatch;
            }
            ST_POSSESSIVE_CAPTURE => {
                number = get2(Fecode!(F), 1 + LINK_SIZE);
                (*F).fields.op_brapos.frame_type = GF_CAPTURE | number;
                state = ST_POSSESSIVE_GROUP;
                continue 'dispatch;
            }
            ST_POSSESSIVE_GROUP => {
                (*F).byte1 = FALSE as u8; /* Lmatched_once = FALSE */
                (*F).fields.op_brapos.start_group = Fecode!(F);

                (*F).fields.op_brapos.start_eptr = Feptr!(F);
                group_frame_type = (*F).fields.op_brapos.frame_type;
                RMATCH!(Fecode!(F).add(op_length(*Fecode!(F))), 8);
            }

            /* Resume: possessive group iteration (RM8). */
            ST_L8 => {
                if rrc == MATCH_KETRPOS {
                    (*F).byte1 = TRUE as u8; /* Lmatched_once */
                    if Feptr!(F) == (*F).fields.op_brapos.start_eptr {
                        loop {
                            Fecode!(F) = Fecode!(F).add(get(Fecode!(F), 1) as usize);
                            if *Fecode!(F) != OP_ALT {
                                break;
                            }
                        }
                        /* success if matched or zero allowed */
                        if (*F).byte1 != FALSE as u8 || (*F).byte2 != FALSE as u8 {
                            Fecode!(F) = Fecode!(F).add(1 + LINK_SIZE);
                            state = ST_MAIN_LOOP;
                            continue 'dispatch;
                        }
                        RRETURN!(MATCH_NOMATCH);
                    }
                    Fecode!(F) = (*F).fields.op_brapos.start_group;
                    (*F).fields.op_brapos.start_eptr = Feptr!(F);
                    group_frame_type = (*F).fields.op_brapos.frame_type;
                    RMATCH!(Fecode!(F).add(op_length(*Fecode!(F))), 8);
                }

                if rrc == MATCH_THEN {
                    let next_ecode: PCRE2_SPTR = Fecode!(F).add(get(Fecode!(F), 1) as usize);
                    if (*mb).verb_ecode_ptr < next_ecode
                        && (*Fecode!(F) == OP_ALT || *next_ecode == OP_ALT)
                    {
                        rrc = MATCH_NOMATCH;
                    }
                }

                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                Fecode!(F) = Fecode!(F).add(get(Fecode!(F), 1) as usize);
                if *Fecode!(F) == OP_ALT {
                    (*F).fields.op_brapos.start_eptr = Feptr!(F);
                    group_frame_type = (*F).fields.op_brapos.frame_type;
                    RMATCH!(Fecode!(F).add(op_length(*Fecode!(F))), 8);
                }

                if (*F).byte1 != FALSE as u8 || (*F).byte2 != FALSE as u8 {
                    Fecode!(F) = Fecode!(F).add(1 + LINK_SIZE);
                    state = ST_MAIN_LOOP;
                    continue 'dispatch;
                }
                RRETURN!(MATCH_NOMATCH);
            }

            /* ---- GROUPLOOP: atomic/capturing/non-empty-capable group. ---- */
            ST_GROUPLOOP => {
                group_frame_type = (*F).fields.op_bra.frame_type;
                RMATCH!(Fecode!(F).add(op_length(*Fecode!(F))), 2);
            }

            /* Resume: GROUPLOOP branch (RM2). */
            ST_L2 => {
                if rrc == MATCH_THEN {
                    let next_ecode: PCRE2_SPTR = Fecode!(F).add(get(Fecode!(F), 1) as usize);
                    if (*mb).verb_ecode_ptr < next_ecode
                        && (*Fecode!(F) == OP_ALT || *next_ecode == OP_ALT)
                    {
                        rrc = MATCH_NOMATCH;
                    }
                }
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                Fecode!(F) = Fecode!(F).add(get(Fecode!(F), 1) as usize);
                if *Fecode!(F) != OP_ALT {
                    RRETURN!(MATCH_NOMATCH);
                }
                group_frame_type = (*F).fields.op_bra.frame_type;
                RMATCH!(Fecode!(F).add(op_length(*Fecode!(F))), 2);
            }

            /* Resume: OP_RECURSE branch (RM11). */
            ST_L11 => {
                let next_ecode: PCRE2_SPTR = (*F)
                    .fields
                    .op_recurse
                    .start_branch
                    .add(get((*F).fields.op_recurse.start_branch, 1) as usize);
                let lframe_type = (*F).fields.op_recurse.frame_type;

                if rrc >= MATCH_BACKTRACK_MIN
                    && rrc <= MATCH_BACKTRACK_MAX
                    && (*mb).verb_current_recurse == (lframe_type ^ GF_RECURSE)
                {
                    if rrc == MATCH_THEN
                        && (*mb).verb_ecode_ptr < next_ecode
                        && (*(*F).fields.op_recurse.start_branch == OP_ALT
                            || *next_ecode == OP_ALT)
                    {
                        rrc = MATCH_NOMATCH;
                    } else {
                        RRETURN!(MATCH_NOMATCH);
                    }
                }

                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                (*F).fields.op_recurse.start_branch = next_ecode;
                if *(*F).fields.op_recurse.start_branch != OP_ALT {
                    RRETURN!(MATCH_NOMATCH);
                }
                group_frame_type = (*F).fields.op_recurse.frame_type;
                RMATCH!(
                    (*F).fields.op_recurse.start_branch
                        .add(op_length(*(*F).fields.op_recurse.start_branch)),
                    11
                );
            }

            /* Resume: positive assertion branch (RM3). */
            ST_L3 => {
                if rrc == MATCH_ACCEPT {
                    memcpy(
                        Fovector!(F),
                        (assert_accept_frame as *const u8)
                            .add(core::mem::offset_of!(heapframe, ovector))
                            as *const PCRE2_SIZE,
                        (*assert_accept_frame).offset_top,
                    );
                    Foffset_top!(F) = (*assert_accept_frame).offset_top;
                    Fmark!(F) = (*assert_accept_frame).mark;
                    loop {
                        Fecode!(F) = Fecode!(F).add(get(Fecode!(F), 1) as usize);
                        if *Fecode!(F) != OP_ALT {
                            break;
                        }
                    }
                    Fecode!(F) = Fecode!(F).add(1 + LINK_SIZE);
                    state = ST_MAIN_LOOP;
                    continue 'dispatch;
                }
                if rrc != MATCH_NOMATCH && rrc != MATCH_THEN {
                    RRETURN!(rrc);
                }
                Fecode!(F) = Fecode!(F).add(get(Fecode!(F), 1) as usize);
                if *Fecode!(F) != OP_ALT {
                    RRETURN!(MATCH_NOMATCH);
                }
                group_frame_type = GF_NOCAPTURE;
                RMATCH!(Fecode!(F).add(op_length(*Fecode!(F))), 3);
            }

            /* Resume: negative assertion branch (RM4). */
            ST_L4 => {
                match rrc {
                    MATCH_ACCEPT | MATCH_MATCH => {
                        RRETURN!(MATCH_NOMATCH);
                    }
                    MATCH_NOMATCH | MATCH_THEN => {
                        Fecode!(F) = Fecode!(F).add(get(Fecode!(F), 1) as usize);
                        if *Fecode!(F) != OP_ALT {
                            state = ST_ASSERT_NOT_FAILED;
                            continue 'dispatch;
                        }
                        group_frame_type = GF_NOCAPTURE;
                        RMATCH!(Fecode!(F).add(op_length(*Fecode!(F))), 4);
                    }
                    MATCH_COMMIT | MATCH_SKIP | MATCH_PRUNE => {
                        loop {
                            Fecode!(F) = Fecode!(F).add(get(Fecode!(F), 1) as usize);
                            if *Fecode!(F) != OP_ALT {
                                break;
                            }
                        }
                        state = ST_ASSERT_NOT_FAILED;
                        continue 'dispatch;
                    }
                    _ => {
                        RRETURN!(rrc);
                    }
                }
            }

            ST_ASSERT_NOT_FAILED => {
                Fecode!(F) = Fecode!(F).add(1 + LINK_SIZE);
                state = ST_MAIN_LOOP;
                continue 'dispatch;
            }

            /* Resume: scan-substring branch (RM38). */
            ST_L38 => {
                if rrc == MATCH_ACCEPT {
                    memcpy(
                        Fovector!(F),
                        (assert_accept_frame as *const u8)
                            .add(core::mem::offset_of!(heapframe, ovector))
                            as *const PCRE2_SIZE,
                        (*assert_accept_frame).offset_top,
                    );
                    Foffset_top!(F) = (*assert_accept_frame).offset_top;
                    Fmark!(F) = (*assert_accept_frame).mark;
                    (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
                    (*mb).true_end_subject =
                        (*mb).end_subject.add((*F).fields.op_assert_scs.true_end_extra);
                    (*mb).moptions = (*F).fields.op_assert_scs.saved_moptions;
                    /* fall through to success path below */
                    loop {
                        Fecode!(F) = Fecode!(F).add(get(Fecode!(F), 1) as usize);
                        if *Fecode!(F) != OP_ALT {
                            break;
                        }
                    }
                    Fecode!(F) = Fecode!(F).add(1 + LINK_SIZE);
                    Feptr!(F) = (*F).fields.op_assert_scs.saved_eptr;
                    state = ST_MAIN_LOOP;
                    continue 'dispatch;
                }

                if rrc != MATCH_NOMATCH && rrc != MATCH_THEN {
                    (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
                    (*mb).true_end_subject =
                        (*mb).end_subject.add((*F).fields.op_assert_scs.true_end_extra);
                    (*mb).moptions = (*F).fields.op_assert_scs.saved_moptions;
                    RRETURN!(rrc);
                }

                Fecode!(F) = Fecode!(F).add(get(Fecode!(F), 1) as usize);
                if *Fecode!(F) != OP_ALT {
                    (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
                    (*mb).true_end_subject =
                        (*mb).end_subject.add((*F).fields.op_assert_scs.true_end_extra);
                    (*mb).moptions = (*F).fields.op_assert_scs.saved_moptions;
                    RRETURN!(MATCH_NOMATCH);
                }
                length = 0;
                group_frame_type = GF_NOCAPTURE;
                RMATCH!(Fecode!(F).add(1 + LINK_SIZE + length), 38);
            }

            /* Resume: conditional assertion branch (RM5). */
            ST_L5 => {
                match rrc {
                    MATCH_ACCEPT => {
                        memcpy(
                            Fovector!(F),
                            (assert_accept_frame as *const u8)
                                .add(core::mem::offset_of!(heapframe, ovector))
                                as *const PCRE2_SIZE,
                            (*assert_accept_frame).offset_top,
                        );
                        Foffset_top!(F) = (*assert_accept_frame).offset_top;
                        condition = (*F).byte1 as BOOL;
                    }
                    MATCH_MATCH => {
                        condition = (*F).byte1 as BOOL;
                    }
                    MATCH_NOMATCH | MATCH_THEN => {
                        (*F).fields.op_cond.start_branch = (*F)
                            .fields
                            .op_cond
                            .start_branch
                            .add(get((*F).fields.op_cond.start_branch, 1) as usize);
                        if *(*F).fields.op_cond.start_branch == OP_ALT {
                            group_frame_type = GF_CONDASSERT;
                            RMATCH!(
                                (*F).fields.op_cond.start_branch
                                    .add(op_length(*(*F).fields.op_cond.start_branch)),
                                5
                            );
                        }
                        condition = ((*F).byte1 == 0) as BOOL;
                    }
                    MATCH_COMMIT | MATCH_SKIP | MATCH_PRUNE => {
                        condition = ((*F).byte1 == 0) as BOOL;
                    }
                    _ => {
                        RRETURN!(rrc);
                    }
                }

                /* After the assertion condition, Fecode points at the condition
                opcode. If condition true, skip to end of assertion. */
                if condition != FALSE {
                    loop {
                        Fecode!(F) = Fecode!(F).add(get(Fecode!(F), 1) as usize);
                        if *Fecode!(F) != OP_ALT {
                            break;
                        }
                    }
                }

                Fecode!(F) = Fecode!(F).add(if condition != FALSE {
                    op_length(*Fecode!(F))
                } else {
                    (*F).fields.op_cond.length
                });

                if Fop!(F) == OP_SCOND {
                    group_frame_type = GF_NOCAPTURE;
                    RMATCH!(Fecode!(F), 35);
                }
                state = ST_MAIN_LOOP;
                continue 'dispatch;
            }

            /* Resume: OP_SCOND descent (RM35). */
            ST_L35 => {
                RRETURN!(rrc);
            }

            /* Resume: OP_VREVERSE (RM37). */
            ST_L37 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                if {
                    let v = (*F).fields.op_vreverse.max;
                    (*F).fields.op_vreverse.max = v.wrapping_sub(1);
                    v
                } <= (*F).fields.op_vreverse.min
                {
                    RRETURN!(MATCH_NOMATCH);
                }
                Feptr!(F) = Feptr!(F).add(1);
                if utf != FALSE {
                    forwardchartest(&mut Feptr!(F), (*mb).end_subject);
                }
                RMATCH!(Fecode!(F).add(1 + 2 * IMM2_SIZE), 37);
            }

            ST_ASSERT_NL_OR_EOS => {
                if Feptr!(F) < (*mb).true_end_subject
                    && (!IS_NEWLINE!(Feptr!(F))
                        || Feptr!(F) != (*mb).true_end_subject.sub((*mb).nllen as usize))
                {
                    if (*mb).partial != 0
                        && Feptr!(F).add(1) >= (*mb).end_subject
                        && (*mb).nltype == NLTYPE_FIXED
                        && (*mb).nllen == 2
                        && *Feptr!(F) as u32 == (*mb).nl[0] as u32
                    {
                        (*mb).hitend = TRUE;
                        if (*mb).partial > 1 {
                            return PCRE2_ERROR_PARTIAL;
                        }
                    }
                    RRETURN!(MATCH_NOMATCH);
                }
                if (*mb).partial != 0 {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
                Fecode!(F) = Fecode!(F).add(1);
                state = ST_MAIN_LOOP;
                continue 'dispatch;
            }

            /* ---- KET: end of a parenthesized group. ---- */
            ST_KET => {
                bracode = Fecode!(F).sub(get(Fecode!(F), 1) as usize);

                if branch_end.is_null() {
                    branch_end = Fecode!(F);
                }
                branch_start = bracode;
                while branch_start.add(get(branch_start, 1) as usize) != branch_end {
                    branch_start = branch_start.add(get(branch_start, 1) as usize);
                }
                branch_end = core::ptr::null();

                if *bracode != OP_BRA && *bracode != OP_COND {
                    N = ((*match_data).heapframes as *mut u8).add(Flast_group_offset!(F))
                        as *mut heapframe;
                    P = (N as *mut u8).sub(frame_size) as *mut heapframe;
                    Flast_group_offset!(F) = (*P).last_group_offset;

                    if (*N).group_frame_type == GF_CONDASSERT {
                        if (*bracode == OP_ASSERTBACK || *bracode == OP_ASSERTBACK_NOT)
                            && *branch_start.add(1 + LINK_SIZE) == OP_VREVERSE
                            && Feptr!(F) != (*P).eptr
                        {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        memcpy(
                            (P as *mut u8).add(core::mem::offset_of!(heapframe, ovector))
                                as *mut PCRE2_SIZE,
                            Fovector!(F),
                            Foffset_top!(F),
                        );
                        (*P).offset_top = Foffset_top!(F);
                        (*P).mark = Fmark!(F);
                        Fback_frame!(F) = (F as *mut u8).offset_from(P as *mut u8) as PCRE2_SIZE;
                        RRETURN!(MATCH_MATCH);
                    }
                } else {
                    P = core::ptr::null_mut();
                }

                match *bracode {
                    OP_BRA => {
                        if Fcurrent_recurse!(F) != 0 || *Fecode!(F).add(1 + LINK_SIZE) != OP_END {
                            /* nothing to do */
                        } else {
                            offset = Flast_group_offset!(F);
                            if offset == PCRE2_UNSET {
                                return PCRE2_ERROR_INTERNAL;
                            }
                            N = ((*match_data).heapframes as *mut u8).add(offset)
                                as *mut heapframe;
                            P = (N as *mut u8).sub(frame_size) as *mut heapframe;
                            Flast_group_offset!(F) = (*P).last_group_offset;

                            Fecode!(F) = (*P).ecode.add(1 + LINK_SIZE);

                            if *Fecode!(F) != OP_CREF {
                                memcpy(
                                    Fovector!(F),
                                    Fovector!(P) as *const PCRE2_SIZE,
                                    Foffset_top!(F),
                                );
                                Foffset_top!(F) = (*P).offset_top;
                            } else {
                                recurse_update_offsets(F, P);
                            }

                            Fcapture_last!(F) = (*P).capture_last;
                            Fcurrent_recurse!(F) = (*P).current_recurse;
                            state = ST_MAIN_LOOP;
                            continue 'dispatch;
                        }
                    }
                    OP_COND | OP_SCOND => {}
                    OP_ASSERTBACK_NA => {
                        if *branch_start.add(1 + LINK_SIZE) == OP_VREVERSE
                            && Feptr!(F) != (*P).eptr
                        {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        if Feptr!(F) > (*mb).last_used_ptr {
                            (*mb).last_used_ptr = Feptr!(F);
                        }
                        Feptr!(F) = (*P).eptr;
                    }
                    OP_ASSERT_NA => {
                        if Feptr!(F) > (*mb).last_used_ptr {
                            (*mb).last_used_ptr = Feptr!(F);
                        }
                        Feptr!(F) = (*P).eptr;
                    }
                    OP_ASSERTBACK => {
                        if *branch_start.add(1 + LINK_SIZE) == OP_VREVERSE
                            && Feptr!(F) != (*P).eptr
                        {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        if Feptr!(F) > (*mb).last_used_ptr {
                            (*mb).last_used_ptr = Feptr!(F);
                        }
                        Feptr!(F) = (*P).eptr;
                        /* fall through to OP_ASSERT then OP_ONCE */
                        Fback_frame!(F) = (F as *mut u8).offset_from(P as *mut u8) as PCRE2_SIZE;
                        loop {
                            let y = get((*P).ecode, 1) as usize;
                            if *(*P).ecode.add(y) != OP_ALT {
                                break;
                            }
                            (*P).ecode = (*P).ecode.add(y);
                        }
                    }
                    OP_ASSERT => {
                        if Feptr!(F) > (*mb).last_used_ptr {
                            (*mb).last_used_ptr = Feptr!(F);
                        }
                        Feptr!(F) = (*P).eptr;
                        Fback_frame!(F) = (F as *mut u8).offset_from(P as *mut u8) as PCRE2_SIZE;
                        loop {
                            let y = get((*P).ecode, 1) as usize;
                            if *(*P).ecode.add(y) != OP_ALT {
                                break;
                            }
                            (*P).ecode = (*P).ecode.add(y);
                        }
                    }
                    OP_ONCE => {
                        Fback_frame!(F) = (F as *mut u8).offset_from(P as *mut u8) as PCRE2_SIZE;
                        loop {
                            let y = get((*P).ecode, 1) as usize;
                            if *(*P).ecode.add(y) != OP_ALT {
                                break;
                            }
                            (*P).ecode = (*P).ecode.add(y);
                        }
                    }
                    OP_ASSERTBACK_NOT => {
                        if *branch_start.add(1 + LINK_SIZE) == OP_VREVERSE
                            && Feptr!(F) != (*P).eptr
                        {
                            RRETURN!(MATCH_NOMATCH);
                        }
                        RRETURN!(MATCH_MATCH);
                    }
                    OP_ASSERT_NOT => {
                        RRETURN!(MATCH_MATCH);
                    }
                    OP_ASSERT_SCS => {
                        (*F).fields.op_assert_scs.saved_end_subject = (*mb).end_subject;
                        (*mb).end_subject = (*P).fields.op_assert_scs.saved_end_subject;
                        (*mb).true_end_subject =
                            (*mb).end_subject.add((*P).fields.op_assert_scs.true_end_extra);
                        Feptr!(F) = (*P).fields.op_assert_scs.saved_eptr;

                        RMATCH!(Fecode!(F).add(1 + LINK_SIZE), 39);
                    }
                    OP_SCRIPT_RUN => {
                        if crate::script_run::script_run((*P).eptr, Feptr!(F), utf) == FALSE {
                            RRETURN!(MATCH_NOMATCH);
                        }
                    }
                    OP_CBRA | OP_CBRAPOS | OP_SCBRA | OP_SCBRAPOS => {
                        number = get2(bracode, 1 + LINK_SIZE);

                        if Fcurrent_recurse!(F) == number {
                            P = (N as *mut u8).sub(frame_size) as *mut heapframe;
                            Fecode!(F) = (*P).ecode.add(1 + LINK_SIZE);

                            if *Fecode!(F) != OP_CREF {
                                memcpy(
                                    Fovector!(F),
                                    Fovector!(P) as *const PCRE2_SIZE,
                                    Foffset_top!(F),
                                );
                                Foffset_top!(F) = (*P).offset_top;
                            } else {
                                recurse_update_offsets(F, P);
                            }

                            Fcapture_last!(F) = (*P).capture_last;
                            Fcurrent_recurse!(F) = (*P).current_recurse;
                            state = ST_MAIN_LOOP;
                            continue 'dispatch;
                        }

                        offset = ((number << 1) - 2) as PCRE2_SIZE;
                        Fcapture_last!(F) = number;
                        *Fovector!(F).add(offset) =
                            (*P).eptr.offset_from((*mb).start_subject) as PCRE2_SIZE;
                        *Fovector!(F).add(offset + 1) =
                            Feptr!(F).offset_from((*mb).start_subject) as PCRE2_SIZE;
                        if offset >= Foffset_top!(F) {
                            Foffset_top!(F) = offset + 2;
                        }
                    }
                    _ => {}
                }

                /* OP_KETRPOS. */
                if *Fecode!(F) == OP_KETRPOS {
                    memcpy(
                        (P as *mut u8).add(core::mem::offset_of!(heapframe, eptr)),
                        (F as *const u8).add(core::mem::offset_of!(heapframe, eptr)),
                        frame_copy_size,
                    );
                    RRETURN!(MATCH_KETRPOS);
                }

                if Fop!(F) != OP_KET && (P.is_null() || Feptr!(F) != (*P).eptr) {
                    if Fop!(F) == OP_KETRMIN {
                        RMATCH!(Fecode!(F).add(1 + LINK_SIZE), 6);
                    }
                    RMATCH!(bracode, 7);
                }

                Fecode!(F) = Fecode!(F).add(1 + LINK_SIZE);
                state = ST_MAIN_LOOP;
                continue 'dispatch;
            }

            /* Resume: KETRMIN (RM6). */
            ST_L6 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                Fecode!(F) = Fecode!(F).sub(get(Fecode!(F), 1) as usize);
                state = ST_MAIN_LOOP;
                continue 'dispatch;
            }

            /* Resume: KETRMAX (RM7). */
            ST_L7 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                Fecode!(F) = Fecode!(F).add(1 + LINK_SIZE);
                state = ST_MAIN_LOOP;
                continue 'dispatch;
            }

            /* Resume: KET OP_ASSERT_SCS backtrack (RM39). */
            ST_L39 => {
                (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
                (*mb).true_end_subject = (*mb).end_subject;
                RRETURN!(rrc);
            }

            /* Resume: OP_MARK (RM12). */
            ST_L12 => {
                if rrc == MATCH_SKIP_ARG
                    && crate::string_utils::strcmp(Fecode!(F).add(2), (*mb).verb_skip_ptr) == 0
                {
                    (*mb).verb_skip_ptr = Feptr!(F);
                    RRETURN!(MATCH_SKIP);
                }
                RRETURN!(rrc);
            }

            /* Resume: OP_COMMIT (RM13). */
            ST_L13 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                (*mb).verb_current_recurse = Fcurrent_recurse!(F);
                RRETURN!(MATCH_COMMIT);
            }

            /* Resume: OP_COMMIT_ARG (RM36). */
            ST_L36 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                (*mb).verb_current_recurse = Fcurrent_recurse!(F);
                RRETURN!(MATCH_COMMIT);
            }

            /* Resume: OP_PRUNE (RM14). */
            ST_L14 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                (*mb).verb_current_recurse = Fcurrent_recurse!(F);
                RRETURN!(MATCH_PRUNE);
            }

            /* Resume: OP_PRUNE_ARG (RM15). */
            ST_L15 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                (*mb).verb_current_recurse = Fcurrent_recurse!(F);
                RRETURN!(MATCH_PRUNE);
            }

            /* Resume: OP_SKIP (RM16). */
            ST_L16 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                (*mb).verb_skip_ptr = Feptr!(F);
                (*mb).verb_current_recurse = Fcurrent_recurse!(F);
                RRETURN!(MATCH_SKIP);
            }

            /* Resume: OP_SKIP_ARG (RM17). */
            ST_L17 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                (*mb).verb_skip_ptr = Fecode!(F).add(2);
                (*mb).verb_current_recurse = Fcurrent_recurse!(F);
                RRETURN!(MATCH_SKIP_ARG);
            }

            /* Resume: OP_THEN (RM18). */
            ST_L18 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                (*mb).verb_ecode_ptr = Fecode!(F);
                (*mb).verb_current_recurse = Fcurrent_recurse!(F);
                RRETURN!(MATCH_THEN);
            }

            /* Resume: OP_THEN_ARG (RM19). */
            ST_L19 => {
                if rrc != MATCH_NOMATCH {
                    RRETURN!(rrc);
                }
                (*mb).verb_ecode_ptr = Fecode!(F);
                (*mb).verb_current_recurse = Fcurrent_recurse!(F);
                RRETURN!(MATCH_THEN);
            }

            /* ---- RETURN_SWITCH: unwind one frame and resume. ---- */
            ST_RETURN_SWITCH => {
                if Feptr!(F) > (*mb).last_used_ptr {
                    (*mb).last_used_ptr = Feptr!(F);
                }
                if Frdepth!(F) == 0 {
                    return rrc; /* Exit from the top level */
                }
                F = (F as *mut u8).sub(Fback_frame!(F)) as *mut heapframe;
                (*(*mb).cb).callout_flags |= PCRE2_CALLOUT_BACKTRACK;
                state = ST_LBASE + Freturn_id!(F) as u32;
                continue 'dispatch;
            }

            _ => {
                return PCRE2_ERROR_INTERNAL;
            }
            } /* End match state */
        } /* End 'dispatch loop */
    } /* End unsafe */
} /* End fn match */

/*************************************************
*           Match a Regular Expression           *
*************************************************/

const FF_FLAGS: u32 = PCRE2_NOTEMPTY_SET | PCRE2_NE_ATST_SET;
const OO_FLAGS: u32 = PCRE2_NOTEMPTY | PCRE2_NOTEMPTY_ATSTART;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_match_8(
    code: *const pcre2_real_code,
    mut subject: PCRE2_SPTR,
    mut length: PCRE2_SIZE,
    start_offset: PCRE2_SIZE,
    options: u32,
    match_data: *mut pcre2_real_match_data,
    mut mcontext: *mut pcre2_real_match_context,
) -> c_int {
    unsafe {
        let mut rc: c_int;
        let mut start_bits: *const u8 = core::ptr::null();
        let re: *const pcre2_real_code = code;
        let original_options: u32 = options;
        let mut options = options;

        let anchored: BOOL;
        let firstline: BOOL;
        let mut has_first_cu: BOOL = FALSE;
        let mut has_req_cu: BOOL = FALSE;
        let startline: BOOL;

        let mut memchr_found_first_cu: PCRE2_SPTR;
        let mut memchr_found_first_cu2: PCRE2_SPTR;

        let mut first_cu: PCRE2_UCHAR = 0;
        let mut first_cu2: PCRE2_UCHAR = 0;
        let mut req_cu: PCRE2_UCHAR = 0;
        let mut req_cu2: PCRE2_UCHAR = 0;

        let null_str: [PCRE2_UCHAR; 1] = [0xcd];
        let original_subject: PCRE2_SPTR = subject;
        let bumpalong_limit: PCRE2_SPTR;
        let mut end_subject: PCRE2_SPTR;
        let true_end_subject: PCRE2_SPTR;
        let mut start_match: PCRE2_SPTR;
        let mut req_cu_ptr: PCRE2_SPTR;
        let mut start_partial: PCRE2_SPTR;
        let mut match_partial: PCRE2_SPTR;

        let mut utf: BOOL = FALSE;
        let mut ucp: BOOL = FALSE;
        let allow_invalid: BOOL;
        let mut fragment_options: u32 = 0;

        let frame_size: PCRE2_SIZE;
        let mut heapframes_size: PCRE2_SIZE;

        let mut cb: pcre2_callout_block = core::mem::zeroed();
        let mut actual_match_block: match_block = core::mem::zeroed();
        let mb: *mut match_block = &mut actual_match_block;

        /* Recognize NULL, length 0 as an empty string. */
        if subject.is_null() && length == 0 {
            subject = null_str.as_ptr();
        }

        /* Plausibility checks. */
        if match_data.is_null() {
            return PCRE2_ERROR_NULL;
        }
        if code.is_null() || subject.is_null() {
            (*match_data).rc = PCRE2_ERROR_NULL;
            return PCRE2_ERROR_NULL;
        }
        if (options & !PUBLIC_MATCH_OPTIONS) != 0 {
            (*match_data).rc = PCRE2_ERROR_BADOPTION;
            return PCRE2_ERROR_BADOPTION;
        }

        start_match = subject.add(start_offset);
        req_cu_ptr = start_match.sub(1);
        if length == PCRE2_ZERO_TERMINATED {
            length = crate::string_utils::strlen(subject);
        }
        end_subject = subject.add(length);
        true_end_subject = end_subject;

        if start_offset > length {
            (*match_data).rc = PCRE2_ERROR_BADOFFSET;
            return PCRE2_ERROR_BADOFFSET;
        }

        if (*re).magic_number != MAGIC_NUMBER {
            (*match_data).rc = PCRE2_ERROR_BADMAGIC;
            return PCRE2_ERROR_BADMAGIC;
        }

        if ((*re).flags & PCRE2_MODE_MASK) != PCRE2_CODE_UNIT_WIDTH / 8 {
            (*match_data).rc = PCRE2_ERROR_BADMODE;
            return PCRE2_ERROR_BADMODE;
        }

        /* Transfer (*NOTEMPTY) flags from the pattern into the options. */
        options |= ((*re).flags & FF_FLAGS)
            / ((FF_FLAGS & (!FF_FLAGS + 1)) / (OO_FLAGS & (!OO_FLAGS + 1)));

        /* Initialize UTF/UCP parameters. */
        utf = (((*re).overall_options & PCRE2_UTF) != 0) as BOOL;
        allow_invalid = (((*re).overall_options & PCRE2_MATCH_INVALID_UTF) != 0) as BOOL;
        ucp = (((*re).overall_options & PCRE2_UCP) != 0) as BOOL;

        /* Partial matching flags into an integer. */
        (*mb).partial = if (options & PCRE2_PARTIAL_HARD) != 0 {
            2
        } else if (options & PCRE2_PARTIAL_SOFT) != 0 {
            1
        } else {
            0
        };

        if (*mb).partial != 0
            && (((*re).overall_options | options) & PCRE2_ENDANCHORED) != 0
        {
            (*match_data).rc = PCRE2_ERROR_BADOPTION;
            return PCRE2_ERROR_BADOPTION;
        }

        if !mcontext.is_null()
            && (*mcontext).offset_limit != PCRE2_UNSET
            && ((*re).overall_options & PCRE2_USE_OFFSET_LIMIT) == 0
        {
            (*match_data).rc = PCRE2_ERROR_BADOFFSETLIMIT;
            return PCRE2_ERROR_BADOFFSETLIMIT;
        }

        if ((*match_data).flags & PCRE2_MD_COPIED_SUBJECT) != 0 {
            ((*match_data).memctl.free.unwrap())(
                (*match_data).subject as *mut c_void,
                (*match_data).memctl.memory_data,
            );
            (*match_data).flags &= !PCRE2_MD_COPIED_SUBJECT;
        }
        (*match_data).subject = core::ptr::null();
        (*match_data).startchar = 0;

        /* No JIT in this build. Proceed with interpreter matching. */
        (*mb).check_subject = subject;

        /* UTF validity check and invalid-UTF fragment handling. */
        if utf != FALSE && ((options & PCRE2_NO_UTF_CHECK) == 0 || allow_invalid != FALSE) {
            let mut skipped_bad_start: BOOL = FALSE;

            if allow_invalid != FALSE {
                while start_match < end_subject && not_firstcu(*start_match as u32) {
                    start_match = start_match.add(1);
                    skipped_bad_start = TRUE;
                }
            } else if start_match < end_subject && not_firstcu(*start_match as u32) {
                if start_offset > 0 {
                    (*match_data).rc = PCRE2_ERROR_BADUTFOFFSET;
                    return PCRE2_ERROR_BADUTFOFFSET;
                }
                (*match_data).rc = PCRE2_ERROR_UTF8_ERR20;
                return PCRE2_ERROR_UTF8_ERR20;
            }

            (*mb).check_subject = start_match;

            if skipped_bad_start == FALSE {
                let mut i2 = (*re).max_lookbehind;
                while i2 > 0 && (*mb).check_subject > subject {
                    (*mb).check_subject = (*mb).check_subject.sub(1);
                    while (*mb).check_subject > subject
                        && (*(*mb).check_subject & 0xc0) == 0x80
                    {
                        (*mb).check_subject = (*mb).check_subject.sub(1);
                    }
                    i2 -= 1;
                }
            }

            loop {
                rc = crate::valid_utf::valid_utf(
                    (*mb).check_subject,
                    length - ((*mb).check_subject.offset_from(subject) as PCRE2_SIZE),
                    &raw mut (*match_data).startchar,
                );

                if rc == 0 {
                    break;
                }

                (*match_data).startchar +=
                    (*mb).check_subject.offset_from(subject) as PCRE2_SIZE;
                if allow_invalid == FALSE || rc > 0 {
                    (*match_data).rc = rc;
                    return rc;
                }
                end_subject = subject.add((*match_data).startchar);

                if end_subject < start_match {
                    (*mb).check_subject = end_subject.add(1);
                    while (*mb).check_subject < start_match
                        && not_firstcu(*(*mb).check_subject as u32)
                    {
                        (*mb).check_subject = (*mb).check_subject.add(1);
                    }
                    end_subject = true_end_subject;
                } else {
                    fragment_options = PCRE2_NOTEOL;
                    break;
                }
            }
        }

        /* A NULL match context means "use a default context". */
        if mcontext.is_null() {
            mcontext = &raw mut _pcre2_default_match_context_8;
            (*mb).memctl = (*re).memctl;
        } else {
            (*mb).memctl = (*mcontext).memctl;
        }

        anchored = ((((*re).overall_options | options) & PCRE2_ANCHORED) != 0) as BOOL;
        firstline = (anchored == FALSE && ((*re).overall_options & PCRE2_FIRSTLINE) != 0) as BOOL;
        startline = (((*re).flags & PCRE2_STARTLINE) != 0) as BOOL;
        bumpalong_limit = if (*mcontext).offset_limit == PCRE2_UNSET {
            true_end_subject
        } else {
            subject.add((*mcontext).offset_limit)
        };

        /* Callout block fixed fields. */
        (*mb).cb = &mut cb;
        cb.version = 2;
        cb.subject = subject;
        cb.subject_length = end_subject.offset_from(subject) as PCRE2_SIZE;
        cb.callout_flags = 0;

        (*mb).callout = (*mcontext).callout;
        (*mb).callout_data = (*mcontext).callout_data;

        (*mb).start_subject = subject;
        (*mb).start_offset = start_offset;
        (*mb).end_subject = end_subject;
        (*mb).true_end_subject = true_end_subject;
        (*mb).hasthen = (((*re).flags & PCRE2_HASTHEN) != 0) as BOOL;
        (*mb).hasbsk = (((*re).flags & PCRE2_HASBSK) != 0) as BOOL;
        (*mb).allowemptypartial =
            (((*re).max_lookbehind > 0) || ((*re).flags & PCRE2_MATCH_EMPTY) != 0) as BOOL;
        (*mb).allowlookaroundbsk =
            (((*re).extra_options & PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK) != 0) as BOOL;
        (*mb).poptions = (*re).overall_options;
        (*mb).ignore_skip_arg = 0;
        (*mb).mark = core::ptr::null();
        (*mb).nomatch_mark = core::ptr::null();

        (*mb).name_table =
            (re as *const u8).add(core::mem::size_of::<pcre2_real_code>()) as PCRE2_SPTR;
        (*mb).name_count = (*re).name_count;
        (*mb).name_entry_size = (*re).name_entry_size;
        (*mb).start_code = (re as *const u8).add((*re).code_start) as PCRE2_SPTR;

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
                (*match_data).rc = PCRE2_ERROR_INTERNAL;
                return PCRE2_ERROR_INTERNAL;
            }
        }

        /* Frame size (padded for alignment). */
        frame_size = (core::mem::offset_of!(heapframe, ovector)
            + (*re).top_bracket as usize * 2 * core::mem::size_of::<PCRE2_SIZE>()
            + HEAPFRAME_ALIGNMENT
            - 1)
            & !(HEAPFRAME_ALIGNMENT - 1);

        (*mb).heap_limit = if (*mcontext).heap_limit < (*re).limit_heap {
            (*mcontext).heap_limit
        } else {
            (*re).limit_heap
        };
        (*mb).match_limit = if (*mcontext).match_limit < (*re).limit_match {
            (*mcontext).match_limit
        } else {
            (*re).limit_match
        };
        (*mb).match_limit_depth = if (*mcontext).depth_limit < (*re).limit_depth {
            (*mcontext).depth_limit
        } else {
            (*re).limit_depth
        };

        heapframes_size = frame_size * 10;
        if heapframes_size < START_FRAMES_SIZE {
            heapframes_size = START_FRAMES_SIZE;
        }
        if heapframes_size / 1024 > (*mb).heap_limit as PCRE2_SIZE {
            let max_size: PCRE2_SIZE = 1024 * (*mb).heap_limit as PCRE2_SIZE;
            if max_size < frame_size {
                (*match_data).rc = PCRE2_ERROR_HEAPLIMIT;
                return PCRE2_ERROR_HEAPLIMIT;
            }
            heapframes_size = max_size;
        }

        if (*match_data).heapframes_size < heapframes_size {
            ((*match_data).memctl.free.unwrap())(
                (*match_data).heapframes as *mut c_void,
                (*match_data).memctl.memory_data,
            );
            (*match_data).heapframes = ((*match_data).memctl.malloc.unwrap())(
                heapframes_size,
                (*match_data).memctl.memory_data,
            ) as *mut heapframe;
            if (*match_data).heapframes.is_null() {
                (*match_data).heapframes_size = 0;
                (*match_data).rc = PCRE2_ERROR_NOMEMORY;
                return PCRE2_ERROR_NOMEMORY;
            }
            (*match_data).heapframes_size = heapframes_size;
        }

        memset(
            ((*match_data).heapframes as *mut u8)
                .add(core::mem::offset_of!(heapframe, ovector)),
            0xff,
            frame_size - core::mem::offset_of!(heapframe, ovector),
        );

        (*mb).lcc = (*re).tables.add(lcc_offset);
        (*mb).fcc = (*re).tables.add(fcc_offset);
        (*mb).ctypes = (*re).tables.add(ctypes_offset);

        /* First code unit(s). */
        if ((*re).flags & PCRE2_FIRSTSET) != 0 {
            has_first_cu = TRUE;
            first_cu = (*re).first_codeunit as u8;
            first_cu2 = first_cu;
            if ((*re).flags & PCRE2_FIRSTCASELESS) != 0 {
                first_cu2 = table_get(first_cu as u32, (*mb).fcc, first_cu as u32) as u8;
                if first_cu > 127 && ucp != FALSE && utf == FALSE {
                    first_cu2 = ucd_othercase(first_cu as u32) as u8;
                }
            }
        } else if startline == FALSE && ((*re).flags & PCRE2_FIRSTMAPSET) != 0 {
            start_bits = (*re).start_bitmap.as_ptr();
        }

        if ((*re).flags & PCRE2_LASTSET) != 0 {
            has_req_cu = TRUE;
            req_cu = (*re).last_codeunit as u8;
            req_cu2 = req_cu;
            if ((*re).flags & PCRE2_LASTCASELESS) != 0 {
                req_cu2 = table_get(req_cu as u32, (*mb).fcc, req_cu as u32) as u8;
                if req_cu > 127 && ucp != FALSE && utf == FALSE {
                    req_cu2 = ucd_othercase(req_cu as u32) as u8;
                }
            }
        }

        /* ==================== bumpalong / fragment loop ==================== */
        /* Local newline / char-advance helpers for the bumpalong loop. */
        macro_rules! IS_NEWLINE {
            ($p:expr) => {{
                if (*mb).nltype != NLTYPE_FIXED {
                    $p < (*mb).end_subject
                        && is_newline(
                            $p,
                            (*mb).nltype,
                            (*mb).end_subject,
                            &raw mut (*mb).nllen,
                            utf,
                        ) != FALSE
                } else {
                    $p <= (*mb).end_subject.sub((*mb).nllen as usize)
                        && *$p as u32 == (*mb).nl[0] as u32
                        && ((*mb).nllen == 1 || *$p.add(1) as u32 == (*mb).nl[1] as u32)
                }
            }};
        }
        macro_rules! WAS_NEWLINE {
            ($p:expr) => {{
                if (*mb).nltype != NLTYPE_FIXED {
                    $p > (*mb).start_subject
                        && was_newline(
                            $p,
                            (*mb).nltype,
                            (*mb).start_subject,
                            &raw mut (*mb).nllen,
                            utf,
                        ) != FALSE
                } else {
                    $p >= (*mb).start_subject.add((*mb).nllen as usize)
                        && *$p.sub((*mb).nllen as usize) as u32 == (*mb).nl[0] as u32
                        && ((*mb).nllen == 1
                            || *$p.sub((*mb).nllen as usize).add(1) as u32 == (*mb).nl[1] as u32)
                }
            }};
        }
        macro_rules! ACROSSCHAR {
            ($cond:expr, $eptr:expr) => {{
                while ($cond) && (*$eptr & 0xc0u8) == 0x80u8 {
                    $eptr = $eptr.add(1);
                }
            }};
        }

        'fragment_restart: loop {
            start_partial = core::ptr::null();
            match_partial = core::ptr::null();
            (*mb).hitend = FALSE;
            memchr_found_first_cu = core::ptr::null();
            memchr_found_first_cu2 = core::ptr::null();

            rc = 'bumpalong: loop {
                let mut new_start_match: PCRE2_SPTR;

                /* ---- Start of match optimizations. ---- */
                if ((*re).optimization_flags & PCRE2_OPTIM_START_OPTIMIZE) != 0 {
                    if firstline != FALSE {
                        let mut t: PCRE2_SPTR = start_match;
                        if utf != FALSE {
                            while t < end_subject && !IS_NEWLINE!(t) {
                                t = t.add(1);
                                ACROSSCHAR!(t < end_subject, t);
                            }
                        } else {
                            while t < end_subject && !IS_NEWLINE!(t) {
                                t = t.add(1);
                            }
                        }
                        end_subject = t;
                    }

                    if anchored != FALSE {
                        if has_first_cu != FALSE || !start_bits.is_null() {
                            let mut ok = start_match < end_subject;
                            if ok {
                                let mut c = *start_match as u32;
                                ok = has_first_cu != FALSE
                                    && (c == first_cu as u32 || c == first_cu2 as u32);
                                if !ok && !start_bits.is_null() {
                                    ok = (*start_bits.add((c / 8) as usize)
                                        & (1u8 << (c & 7)))
                                        != 0;
                                }
                                let _ = &mut c;
                            }
                            if !ok {
                                break 'bumpalong MATCH_NOMATCH;
                            }
                        }
                    } else {
                        if has_first_cu != FALSE {
                            if first_cu != first_cu2 {
                                /* Caseless: two memchr searches with caching. */
                                let mut pp1: PCRE2_SPTR;
                                let mut pp2: PCRE2_SPTR;
                                let searchlength =
                                    end_subject.offset_from(start_match) as usize;

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

                                if pp1.is_null() {
                                    start_match =
                                        if pp2.is_null() { end_subject } else { pp2 };
                                } else {
                                    start_match =
                                        if pp2.is_null() || pp1 < pp2 { pp1 } else { pp2 };
                                }
                            } else {
                                start_match = memchr(
                                    start_match as *const c_void,
                                    first_cu as c_int,
                                    end_subject.offset_from(start_match) as usize,
                                ) as PCRE2_SPTR;
                                if start_match.is_null() {
                                    start_match = end_subject;
                                }
                            }

                            if (*mb).partial == 0 && start_match >= (*mb).end_subject {
                                break 'bumpalong MATCH_NOMATCH;
                            }
                        } else if startline != FALSE {
                            if start_match > (*mb).start_subject.add(start_offset) {
                                if utf != FALSE {
                                    while start_match < end_subject
                                        && !WAS_NEWLINE!(start_match)
                                    {
                                        start_match = start_match.add(1);
                                        ACROSSCHAR!(
                                            start_match < end_subject,
                                            start_match
                                        );
                                    }
                                } else {
                                    while start_match < end_subject
                                        && !WAS_NEWLINE!(start_match)
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
                        } else if !start_bits.is_null() {
                            while start_match < end_subject {
                                let c = *start_match as u32;
                                if (*start_bits.add((c / 8) as usize) & (1u8 << (c & 7))) != 0 {
                                    break;
                                }
                                start_match = start_match.add(1);
                            }
                            if (*mb).partial == 0 && start_match >= (*mb).end_subject {
                                break 'bumpalong MATCH_NOMATCH;
                            }
                        }
                    }

                    /* Restore fudged end_subject. */
                    end_subject = (*mb).end_subject;

                    if (*mb).partial == 0 {
                        let mut p: PCRE2_SPTR;

                        if (end_subject.offset_from(start_match) as PCRE2_SIZE)
                            < (*re).minlength as PCRE2_SIZE
                        {
                            break 'bumpalong MATCH_NOMATCH;
                        }

                        p = start_match.add(if has_first_cu != FALSE { 1 } else { 0 });
                        if has_req_cu != FALSE && p > req_cu_ptr {
                            let check_length =
                                end_subject.offset_from(start_match) as PCRE2_SIZE;

                            if check_length < REQ_CU_MAX
                                || (anchored == FALSE && check_length < REQ_CU_MAX * 1000)
                            {
                                if req_cu != req_cu2 {
                                    let pp = p;
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
                                } else {
                                    p = memchr(
                                        p as *const c_void,
                                        req_cu as c_int,
                                        end_subject.offset_from(p) as usize,
                                    ) as PCRE2_SPTR;
                                    if p.is_null() {
                                        p = end_subject;
                                    }
                                }

                                if p >= end_subject {
                                    break 'bumpalong MATCH_NOMATCH;
                                }
                                req_cu_ptr = p;
                            }
                        }
                    }
                }
                /* ---- End of start of match optimizations. ---- */

                if start_match > bumpalong_limit {
                    break 'bumpalong MATCH_NOMATCH;
                }

                cb.start_match = start_match.offset_from(subject) as PCRE2_SIZE;
                cb.callout_flags |= PCRE2_CALLOUT_STARTMATCH;

                (*mb).start_used_ptr = start_match;
                (*mb).last_used_ptr = start_match;
                (*mb).moptions = options | fragment_options;
                (*mb).match_call_count = 0;
                (*mb).end_offset_top = 0;
                (*mb).skip_arg_count = 0;

                let mrc = r#match(
                    start_match,
                    (*mb).start_code,
                    (*re).top_bracket,
                    frame_size,
                    match_data,
                    mb,
                );

                if (*mb).hitend != FALSE && start_partial.is_null() {
                    start_partial = (*mb).start_used_ptr;
                    match_partial = start_match;
                }

                match mrc {
                    MATCH_SKIP_ARG => {
                        new_start_match = start_match;
                        (*mb).ignore_skip_arg = (*mb).skip_arg_count;
                    }
                    MATCH_SKIP => {
                        if (*mb).verb_skip_ptr > start_match {
                            new_start_match = (*mb).verb_skip_ptr;
                        } else {
                            /* Fall through to NOMATCH-style advance. */
                            (*mb).ignore_skip_arg = 0;
                            new_start_match = start_match.add(1);
                            if utf != FALSE {
                                ACROSSCHAR!(new_start_match < end_subject, new_start_match);
                            }
                        }
                    }
                    MATCH_NOMATCH | MATCH_PRUNE | MATCH_THEN => {
                        (*mb).ignore_skip_arg = 0;
                        new_start_match = start_match.add(1);
                        if utf != FALSE {
                            ACROSSCHAR!(new_start_match < end_subject, new_start_match);
                        }
                    }
                    MATCH_COMMIT => {
                        break 'bumpalong MATCH_NOMATCH;
                    }
                    _ => {
                        break 'bumpalong mrc;
                    }
                }

                /* No match at this point; reset and continue. */
                if firstline != FALSE && IS_NEWLINE!(start_match) {
                    break 'bumpalong MATCH_NOMATCH;
                }

                start_match = new_start_match;

                if anchored != FALSE || start_match > end_subject {
                    break 'bumpalong MATCH_NOMATCH;
                }

                if start_match > subject.add(start_offset)
                    && *start_match.sub(1) as u32 == CHAR_CR
                    && start_match < end_subject
                    && *start_match as u32 == CHAR_NL
                    && ((*re).flags & PCRE2_HASCRORLF) == 0
                    && ((*mb).nltype == NLTYPE_ANY
                        || (*mb).nltype == NLTYPE_ANYCRLF
                        || (*mb).nllen == 2)
                {
                    start_match = start_match.add(1);
                }

                (*mb).mark = core::ptr::null();
            };

            /* ---- ENDLOOP ---- */
            if utf != FALSE
                && end_subject != true_end_subject
                && (rc == MATCH_NOMATCH || rc == PCRE2_ERROR_PARTIAL)
            {
                let mut restart = false;
                loop {
                    start_match = end_subject.add(1);
                    while start_match < true_end_subject && not_firstcu(*start_match as u32) {
                        start_match = start_match.add(1);
                    }

                    if start_match >= true_end_subject {
                        rc = MATCH_NOMATCH;
                        match_partial = core::ptr::null();
                        break;
                    }

                    (*mb).check_subject = start_match;
                    rc = crate::valid_utf::valid_utf(
                        start_match,
                        length - (start_match.offset_from(subject) as PCRE2_SIZE),
                        &raw mut (*match_data).startchar,
                    );

                    if rc == 0 {
                        (*mb).end_subject = true_end_subject;
                        end_subject = true_end_subject;
                        fragment_options = PCRE2_NOTBOL;
                        restart = true;
                        break;
                    } else if rc < 0 {
                        (*mb).end_subject = start_match.add((*match_data).startchar);
                        end_subject = (*mb).end_subject;
                        if end_subject > start_match {
                            fragment_options = PCRE2_NOTBOL | PCRE2_NOTEOL;
                            restart = true;
                            break;
                        }
                    }
                }
                if restart {
                    continue 'fragment_restart;
                }
            }
            break 'fragment_restart;
        }

        /* Fill in fields always returned in the match data. */
        (*match_data).code = re;
        (*match_data).mark = (*mb).mark;
        (*match_data).matchedby = PCRE2_MATCHEDBY_INTERPRETER;
        (*match_data).options = original_options;

        if rc == MATCH_MATCH {
            (*match_data).rc = if (*mb).end_offset_top as c_int
                >= 2 * (*match_data).oveccount as c_int
            {
                0
            } else {
                (*mb).end_offset_top as c_int / 2 + 1
            };
            (*match_data).subject_length = length;
            (*match_data).start_offset = start_offset;
            (*match_data).startchar = start_match.offset_from(subject) as PCRE2_SIZE;
            (*match_data).leftchar =
                (*mb).start_used_ptr.offset_from(subject) as PCRE2_SIZE;
            (*match_data).rightchar = (if (*mb).last_used_ptr > (*mb).end_match_ptr {
                (*mb).last_used_ptr
            } else {
                (*mb).end_match_ptr
            })
            .offset_from(subject) as PCRE2_SIZE;
            if (options & PCRE2_COPY_MATCHED_SUBJECT) != 0 {
                if length != 0 {
                    (*match_data).subject = ((*match_data).memctl.malloc.unwrap())(
                        cu2bytes(length),
                        (*match_data).memctl.memory_data,
                    ) as PCRE2_SPTR;
                    if (*match_data).subject.is_null() {
                        (*match_data).rc = PCRE2_ERROR_NOMEMORY;
                        return PCRE2_ERROR_NOMEMORY;
                    }
                    memcpy(
                        (*match_data).subject as *mut u8,
                        subject as *const u8,
                        cu2bytes(length),
                    );
                } else {
                    (*match_data).subject = core::ptr::null();
                }
                (*match_data).flags |= PCRE2_MD_COPIED_SUBJECT;
            } else {
                (*match_data).subject = original_subject;
            }

            return (*match_data).rc;
        }

        (*match_data).mark = (*mb).nomatch_mark;

        if rc != MATCH_NOMATCH && rc != PCRE2_ERROR_PARTIAL {
            (*match_data).rc = rc;
        } else if !match_partial.is_null() {
            (*match_data).subject = original_subject;
            (*match_data).subject_length = length;
            (*match_data).start_offset = start_offset;
            *(*match_data).ovector.as_mut_ptr().add(0) =
                match_partial.offset_from(subject) as PCRE2_SIZE;
            *(*match_data).ovector.as_mut_ptr().add(1) =
                end_subject.offset_from(subject) as PCRE2_SIZE;
            (*match_data).startchar = match_partial.offset_from(subject) as PCRE2_SIZE;
            (*match_data).leftchar = start_partial.offset_from(subject) as PCRE2_SIZE;
            (*match_data).rightchar = end_subject.offset_from(subject) as PCRE2_SIZE;
            (*match_data).rc = PCRE2_ERROR_PARTIAL;
        } else {
            (*match_data).subject = original_subject;
            (*match_data).subject_length = length;
            (*match_data).start_offset = start_offset;
            (*match_data).rc = PCRE2_ERROR_NOMATCH;
        }

        (*match_data).rc
    }
}
