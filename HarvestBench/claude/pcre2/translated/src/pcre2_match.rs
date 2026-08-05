// Translation of pcre2_match.c (PCRE2 10.48, 8-bit, SUPPORT_UNICODE, no JIT,
// LINK_SIZE=2, IMM2_SIZE=2). Preserves match results byte-for-byte, reproducing
// the RMATCH/RRETURN heap-frame backtracking machine and every opcode case via a
// loop + return_id dispatch model.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]
#![allow(unused_parens)]

use core::ptr;
use core::ffi::{c_int, c_void};

use crate::pcre2_internal::*;

// Cross-module functions.
use crate::pcre2_context::_pcre2_default_match_context_8;
use crate::pcre2_extuni::_pcre2_extuni_8;
use crate::pcre2_newline::{_pcre2_is_newline_8, _pcre2_was_newline_8};
use crate::pcre2_ord2utf::_pcre2_ord2utf_8;
use crate::pcre2_script_run::_pcre2_script_run_8;
use crate::pcre2_string_utils::{_pcre2_strcmp_8, _pcre2_strlen_8};
use crate::pcre2_valid_utf::_pcre2_valid_utf_8;
use crate::pcre2_xclass::{_pcre2_eclass_8, _pcre2_xclass_8};

// ---------------------------------------------------------------------------
// Constants mirroring the #defines in pcre2_match.c
// ---------------------------------------------------------------------------

const RECURSE_UNSET: u32 = 0xffffffff;

// Isolated 0x80 byte error (see pcre2_internal.h UTF8 error numbering).
const PCRE2_ERROR_UTF8_ERR20: c_int = -22;

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

// Non-error returns from and within match().
const MATCH_MATCH: c_int = 1;
const MATCH_NOMATCH: c_int = 0;

// Special internal returns.
const MATCH_ACCEPT: c_int = -999;
const MATCH_KETRPOS: c_int = -998;
const MATCH_COMMIT: c_int = -997;
const MATCH_PRUNE: c_int = -996;
const MATCH_SKIP: c_int = -995;
const MATCH_SKIP_ARG: c_int = -994;
const MATCH_THEN: c_int = -993;
const MATCH_BACKTRACK_MAX: c_int = MATCH_THEN;
const MATCH_BACKTRACK_MIN: c_int = MATCH_COMMIT;

// Group frame type values.
const GF_CAPTURE: u32 = 0x00010000;
const GF_NOCAPTURE: u32 = 0x00020000;
const GF_CONDASSERT: u32 = 0x00030000;
const GF_RECURSE: u32 = 0x00040000;

#[inline]
fn GF_IDMASK(a: u32) -> u32 {
    a & 0xffff0000
}
#[inline]
fn GF_DATAMASK(a: u32) -> u32 {
    a & 0x0000ffff
}

// Repetition types.
const REPTYPE_MIN: u32 = 0;
const REPTYPE_MAX: u32 = 1;
const REPTYPE_POS: u32 = 2;

static REP_MIN: [u32; 11] = [0, 0, 1, 1, 0, 0, 0, 0, 0, 1, 0];
static REP_MAX: [u32; 11] = [
    u32::MAX,
    u32::MAX,
    u32::MAX,
    u32::MAX,
    1,
    1,
    0,
    0,
    u32::MAX,
    u32::MAX,
    1,
];
static REP_TYP: [u32; 12] = [
    REPTYPE_MAX,
    REPTYPE_MIN,
    REPTYPE_MAX,
    REPTYPE_MIN,
    REPTYPE_MAX,
    REPTYPE_MIN,
    REPTYPE_MAX,
    REPTYPE_MIN,
    REPTYPE_POS,
    REPTYPE_POS,
    REPTYPE_POS,
    REPTYPE_POS,
];

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

#[inline]
unsafe fn fbyte1(f: *mut heapframe) -> *mut u8 {
    (ptr::addr_of_mut!((*f).op)).add(1)
}
#[inline]
unsafe fn fbyte2(f: *mut heapframe) -> *mut u8 {
    (ptr::addr_of_mut!((*f).op)).add(2)
}

#[inline]
unsafe fn mem_eq(a: PCRE2_SPTR, b: PCRE2_SPTR, n: usize) -> bool {
    core::slice::from_raw_parts(a, n) == core::slice::from_raw_parts(b, n)
}

#[inline]
unsafe fn is_newline(p: PCRE2_SPTR, mb: *mut match_block, utf: BOOL) -> bool {
    if (*mb).nltype != NLTYPE_FIXED {
        p < (*mb).end_subject
            && _pcre2_is_newline_8(
                p,
                (*mb).nltype,
                (*mb).end_subject,
                ptr::addr_of_mut!((*mb).nllen),
                utf,
            ) != 0
    } else {
        let nllen = (*mb).nllen as usize;
        p <= (*mb).end_subject.wrapping_sub(nllen)
            && *p == (*mb).nl[0]
            && ((*mb).nllen == 1 || *p.add(1) == (*mb).nl[1])
    }
}

#[inline]
unsafe fn was_newline(p: PCRE2_SPTR, mb: *mut match_block, utf: BOOL) -> bool {
    if (*mb).nltype != NLTYPE_FIXED {
        p > (*mb).start_subject
            && _pcre2_was_newline_8(
                p,
                (*mb).nltype,
                (*mb).start_subject,
                ptr::addr_of_mut!((*mb).nllen),
                utf,
            ) != 0
    } else {
        let nllen = (*mb).nllen as usize;
        p >= (*mb).start_subject.add(nllen)
            && *p.sub(nllen) == (*mb).nl[0]
            && ((*mb).nllen == 1 || *p.sub(nllen).add(1) == (*mb).nl[1])
    }
}

// Whitespace classification (non-EBCDIC).
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
fn is_hspace_byte(c: u32) -> bool {
    matches!(c, 0x09 | 0x20 | 0xa0)
}
#[inline]
fn is_vspace(c: u32) -> bool {
    matches!(c, 0x0a | 0x0b | 0x0c | 0x0d | 0x85 | 0x2028 | 0x2029)
}
#[inline]
fn is_vspace_byte(c: u32) -> bool {
    matches!(c, 0x0a | 0x0b | 0x0c | 0x0d | 0x85)
}

#[inline]
fn ucd_any_i(ch: u32) -> bool {
    (ch | 0x20) == 0x69 || (ch | 1) == 0x0131
}
#[inline]
fn ucd_fold_i_turkish(ch: u32) -> u32 {
    if ch == 0x0130 {
        0x69
    } else if ch == 0x49 {
        0x0131
    } else {
        ch
    }
}

#[inline]
unsafe fn op_length(op: u8) -> usize {
    _pcre2_OP_lengths_8[op as usize] as usize
}

// ---------------------------------------------------------------------------
// do_callout
// ---------------------------------------------------------------------------

unsafe fn do_callout(
    F: *mut heapframe,
    mb: *mut match_block,
    lengthptr: *mut PCRE2_SIZE,
) -> c_int {
    let fecode = (*F).ecode;

    *lengthptr = if *fecode == OP_CALLOUT {
        op_length(OP_CALLOUT)
    } else {
        GET(fecode, 1 + 2 * LINK_SIZE) as usize
    };

    if (*mb).callout.is_none() {
        return 0; // No callout function provided
    }

    // callout_ovector = (PCRE2_SIZE *)(Fovector) - 2
    let fovector = ptr::addr_of_mut!((*F).ovector) as *mut PCRE2_SIZE;
    let callout_ovector = fovector.sub(2);

    let cb = (*mb).cb;
    (*cb).capture_top = ((*F).offset_top as u32) / 2 + 1;
    (*cb).capture_last = (*F).capture_last;
    (*cb).offset_vector = callout_ovector;
    (*cb).mark = (*mb).nomatch_mark;
    (*cb).current_position = ((*F).eptr as usize) - ((*mb).start_subject as usize);
    (*cb).pattern_position = GET(fecode, 1) as usize;
    (*cb).next_item_length = GET(fecode, 1 + LINK_SIZE) as usize;

    if *fecode == OP_CALLOUT {
        (*cb).callout_number = *fecode.add(1 + 2 * LINK_SIZE) as u32;
        (*cb).callout_string_offset = 0;
        (*cb).callout_string = ptr::null();
        (*cb).callout_string_length = 0;
    } else {
        (*cb).callout_number = 0;
        (*cb).callout_string_offset = GET(fecode, 1 + 3 * LINK_SIZE) as usize;
        (*cb).callout_string = fecode.add((1 + 4 * LINK_SIZE) + 1);
        (*cb).callout_string_length = *lengthptr - (1 + 4 * LINK_SIZE) - 2;
    }

    let save0 = *callout_ovector.add(0);
    let save1 = *callout_ovector.add(1);
    *callout_ovector.add(0) = PCRE2_UNSET;
    *callout_ovector.add(1) = PCRE2_UNSET;
    let rc = ((*mb).callout.unwrap())(cb, (*mb).callout_data);
    *callout_ovector.add(0) = save0;
    *callout_ovector.add(1) = save1;
    (*cb).callout_flags = 0;
    rc
}

// ---------------------------------------------------------------------------
// match_ref
// ---------------------------------------------------------------------------

unsafe fn match_ref(
    offset: PCRE2_SIZE,
    caseless: BOOL,
    caseopts: c_int,
    F: *mut heapframe,
    mb: *mut match_block,
    lengthptr: *mut PCRE2_SIZE,
) -> c_int {
    let fovector = ptr::addr_of_mut!((*F).ovector) as *mut PCRE2_SIZE;

    // Deal with an unset group.
    if offset >= (*F).offset_top || *fovector.add(offset) == PCRE2_UNSET {
        if ((*mb).poptions & PCRE2_MATCH_UNSET_BACKREF) != 0 {
            *lengthptr = 0;
            return 0; // Match
        } else {
            return -1; // No match
        }
    }

    let mut eptr = (*F).eptr;
    let eptr_start = eptr;
    let mut p = (*mb).start_subject.add(*fovector.add(offset));
    let mut length = *fovector.add(offset + 1) - *fovector.add(offset);

    if caseless != 0 {
        let utf = ((*mb).poptions & PCRE2_UTF) != 0;
        let caseless_restrict = (caseopts as u32 & REFI_FLAG_CASELESS_RESTRICT) != 0;
        let turkish_casing =
            !caseless_restrict && (caseopts as u32 & REFI_FLAG_TURKISH_CASING) != 0;

        if utf || ((*mb).poptions & PCRE2_UCP) != 0 {
            let endptr = p.add(length);

            while p < endptr {
                let c: u32;
                let d: u32;
                if eptr >= (*mb).end_subject {
                    return 1; // Partial match
                }
                if utf {
                    let (cc, cn) = GETCHARINC(eptr);
                    c = cc;
                    eptr = eptr.add(cn);
                    let (dd, dn) = GETCHARINC(p);
                    d = dd;
                    p = p.add(dn);
                } else {
                    c = *eptr as u32;
                    eptr = eptr.add(1);
                    d = *p as u32;
                    p = p.add(1);
                }

                if turkish_casing && ucd_any_i(d) {
                    let cf = ucd_fold_i_turkish(c);
                    let df = ucd_fold_i_turkish(d);
                    if cf != df {
                        return -1;
                    }
                } else if c != d && c != UCD_OTHERCASE(d) {
                    let ur = GET_UCD(d);
                    let base = ur.caseset as usize;
                    let mut idx = base;
                    // When PCRE2_EXTRA_CASELESS_RESTRICT is set, ignore any
                    // caseless sets that start with an ASCII character.
                    if caseless_restrict && _pcre2_ucd_caseless_sets_8[base] < 128 {
                        return -1;
                    }
                    loop {
                        let pp = _pcre2_ucd_caseless_sets_8[idx];
                        if c < pp {
                            return -1;
                        }
                        idx += 1;
                        if c == pp {
                            break;
                        }
                    }
                }
            }
        } else {
            // Not in UTF or UCP mode.
            while length > 0 {
                if eptr >= (*mb).end_subject {
                    return 1;
                }
                let cc = *eptr as u32;
                let cp = *p as u32;
                if *(*mb).lcc.add(cp as usize) != *(*mb).lcc.add(cc as usize) {
                    return -1;
                }
                p = p.add(1);
                eptr = eptr.add(1);
                length -= 1;
            }
        }
    } else {
        // Caseful.
        if (*mb).partial != 0 {
            while length > 0 {
                if eptr >= (*mb).end_subject {
                    return 1;
                }
                if *p != *eptr {
                    return -1;
                }
                p = p.add(1);
                eptr = eptr.add(1);
                length -= 1;
            }
        } else {
            if ((*mb).end_subject as usize - eptr as usize) < length || !mem_eq(p, eptr, length) {
                return -1;
            }
            eptr = eptr.add(length);
        }
    }

    *lengthptr = eptr as usize - eptr_start as usize;
    0
}

// ---------------------------------------------------------------------------
// recurse_update_offsets
// ---------------------------------------------------------------------------

unsafe fn recurse_update_offsets(F: *mut heapframe, P: *mut heapframe) {
    let mut dst = ptr::addr_of_mut!((*F).ovector) as *mut PCRE2_SIZE;
    let mut src = ptr::addr_of_mut!((*P).ovector) as *mut PCRE2_SIZE;
    let mut offset: PCRE2_SIZE = 2;
    let offset_top: PCRE2_SIZE = (*F).offset_top + 2;
    let mut diff: PCRE2_SIZE;
    let mut ecode = (*F).ecode;

    loop {
        diff = ((GET2(ecode, 1) as usize) << 1) - offset;
        ecode = ecode.add(1 + IMM2_SIZE);

        if offset + diff >= offset_top {
            while *ecode == OP_CREF {
                ecode = ecode.add(1 + IMM2_SIZE);
            }
            break;
        }

        if diff == 2 {
            *dst.add(0) = *src.add(0);
            *dst.add(1) = *src.add(1);
        } else if diff >= 4 {
            ptr::copy_nonoverlapping(src, dst, diff);
        }

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
        ptr::copy_nonoverlapping(src, dst, diff);
    }

    (*F).ecode = ecode;
    (*F).offset_top = if offset <= (*P).offset_top {
        (*P).offset_top
    } else {
        offset - 2
    };
}
// ---------------------------------------------------------------------------
// The match() engine.
//
// The C code uses a heap-frame backtracking machine driven by RMATCH/RRETURN
// macros implemented with gotos and a computed switch on Freturn_id. We model
// this with a single `'machine` loop over a `state` variable. Positive `state`
// values are the RMx resume ids (matching the C enum). Negative values are
// internal machine states.
// ---------------------------------------------------------------------------

const ST_MATCH_RECURSE: i32 = -1;
const ST_NEW_FRAME: i32 = -2;
const ST_MAINLOOP: i32 = -3;
const ST_RETURN_SWITCH: i32 = -4;
const ST_MAINLOOP2: i32 = -5;
const ST_MAINLOOP3: i32 = -6;
const ST_MAINLOOP4: i32 = -7;
const ST_MAINLOOP5: i32 = -8;
const ST_MAINLOOP6: i32 = -9;
const ST_MAINLOOP7: i32 = -10;
const ST_MAINLOOP8: i32 = -11;
const ST_MAINLOOP9: i32 = -12;
const ST_MAINLOOP10: i32 = -13;
const ST_MAINLOOP11: i32 = -14;
const ST_ASSERT_NL_OR_EOS: i32 = 1134;
const ST_WORD_BOUNDARY: i32 = 1135;

// Goto-label states (never stored in return_id, so any distinct value works).
const ST_REPEATCHAR: i32 = 1000;
const ST_REPEATNOTCHAR: i32 = 1001;
const ST_REPEATTYPE: i32 = 1002;
const ST_REF_REPEAT: i32 = 1003;
const ST_GROUPLOOP: i32 = 1004;
const ST_POSSESSIVE_GROUP: i32 = 1005;
// Maximize-backtrack loop-head states.
const ST_RC_RM203_LOOP: i32 = 1100;
const ST_RC_RM26_LOOP: i32 = 1101;
const ST_RC_RM28_LOOP: i32 = 1102;
const ST_RNC_RM205_LOOP: i32 = 1103;
const ST_RNC_RM30_LOOP: i32 = 1104;
const ST_RNC_RM207_LOOP: i32 = 1105;
const ST_RNC_RM32_LOOP: i32 = 1106;
const ST_CLASS_RM201_LOOP: i32 = 1107;
const ST_CLASS_RM24_LOOP: i32 = 1108;
const ST_XCLASS_RM101_LOOP: i32 = 1109;
const ST_ECLASS_RM103_LOOP: i32 = 1110;
const ST_TYPE_RM221_LOOP: i32 = 1111;
const ST_TYPE_RM219_LOOP: i32 = 1112;
const ST_TYPE_RM220_LOOP: i32 = 1113;
const ST_TYPE_RM34_LOOP: i32 = 1114;
const ST_REF_RM21_LOOP: i32 = 1115;
const ST_REF_RM22_LOOP: i32 = 1116;
const ST_TYPE_MIN_DISPATCH: i32 = 1117;
const ST_TYPE_MAX_DISPATCH: i32 = 1118;
const ST_TYPE_MAX_UTF: i32 = 1119;
const ST_TYPE_MAX_NONUTF: i32 = 1120;
const ST_POSSESSIVE_LOOP: i32 = 1121;
const ST_BRA_LOOP: i32 = 1122;
const ST_RECURSE_ENTRY: i32 = 1123;
const ST_RECURSE_LOOP: i32 = 1124;
const ST_ASSERT_LOOP: i32 = 1125;
const ST_ASSERT_NOT_LOOP: i32 = 1126;
const ST_SCS_LOOP: i32 = 1127;
const ST_COND_ASSERT_LOOP: i32 = 1128;
const ST_VREVERSE_LOOP: i32 = 1129;
const ST_SCS_ENTRY: i32 = 1130;
const ST_SCS_LOOP2: i32 = 1131;
const ST_COND_ENTRY: i32 = 1132;
const ST_KET_ENTRY: i32 = 1133;

// RM resume ids (match the C enum values; stored in return_id as u8).
const RM1: u8 = 1;
const RM2: u8 = 2;
const RM3: u8 = 3;
const RM4: u8 = 4;
const RM5: u8 = 5;
const RM6: u8 = 6;
const RM7: u8 = 7;
const RM8: u8 = 8;
const RM9: u8 = 9;
const RM10: u8 = 10;
const RM11: u8 = 11;
const RM12: u8 = 12;
const RM13: u8 = 13;
const RM14: u8 = 14;
const RM15: u8 = 15;
const RM16: u8 = 16;
const RM17: u8 = 17;
const RM18: u8 = 18;
const RM19: u8 = 19;
const RM20: u8 = 20;
const RM21: u8 = 21;
const RM22: u8 = 22;
const RM23: u8 = 23;
const RM24: u8 = 24;
const RM25: u8 = 25;
const RM26: u8 = 26;
const RM27: u8 = 27;
const RM28: u8 = 28;
const RM29: u8 = 29;
const RM30: u8 = 30;
const RM31: u8 = 31;
const RM32: u8 = 32;
const RM33: u8 = 33;
const RM34: u8 = 34;
const RM35: u8 = 35;
const RM36: u8 = 36;
const RM37: u8 = 37;
const RM38: u8 = 38;
const RM39: u8 = 39;
const RM100: u8 = 100;
const RM101: u8 = 101;
const RM102: u8 = 102;
const RM103: u8 = 103;
const RM200: u8 = 200;
const RM201: u8 = 201;
const RM202: u8 = 202;
const RM203: u8 = 203;
const RM204: u8 = 204;
const RM205: u8 = 205;
const RM206: u8 = 206;
const RM207: u8 = 207;
const RM208: u8 = 208;
const RM209: u8 = 209;
const RM210: u8 = 210;
const RM211: u8 = 211;
const RM212: u8 = 212;
const RM213: u8 = 213;
const RM214: u8 = 214;
const RM215: u8 = 215;
const RM216: u8 = 216;
const RM217: u8 = 217;
const RM218: u8 = 218;
const RM219: u8 = 219;
const RM220: u8 = 220;
const RM221: u8 = 221;
const RM222: u8 = 222;
const RM223: u8 = 223;
const RM224: u8 = 224;

#[inline]
unsafe fn frame_byte_add(f: *mut heapframe, off: usize) -> *mut heapframe {
    (f as *mut u8).add(off) as *mut heapframe
}
#[inline]
unsafe fn frame_byte_sub(f: *mut heapframe, off: usize) -> *mut heapframe {
    (f as *mut u8).sub(off) as *mut heapframe
}
#[inline]
unsafe fn fovec(f: *mut heapframe) -> *mut PCRE2_SIZE {
    ptr::addr_of_mut!((*f).ovector) as *mut PCRE2_SIZE
}

unsafe fn r#match(
    start_eptr: PCRE2_SPTR,
    start_ecode_arg: PCRE2_SPTR,
    top_bracket: u16,
    frame_size: PCRE2_SIZE,
    match_data: *mut pcre2_match_data,
    mb: *mut match_block,
) -> c_int {
    let mut F: *mut heapframe;
    let mut N: *mut heapframe = ptr::null_mut();
    let mut P: *mut heapframe = ptr::null_mut();

    let mut frames_top: *mut heapframe;
    let mut assert_accept_frame: *mut heapframe = ptr::null_mut();
    let frame_copy_size: PCRE2_SIZE;

    let mut branch_end: PCRE2_SPTR = ptr::null();
    let mut branch_start: PCRE2_SPTR;
    let mut bracode: PCRE2_SPTR;
    let mut offset: PCRE2_SIZE = 0;
    let mut length: PCRE2_SIZE = 0;

    let mut rrc: c_int = 0;
    let mut proptype: i32 = -1;

    let mut i: u32;
    let mut fc: u32;
    let mut number: u32;
    let mut reptype: u32 = 0;
    let mut group_frame_type: u32;

    let mut condition: bool;
    let mut cur_is_word: bool;
    let mut prev_is_word: bool;

    let utf: BOOL = (((*mb).poptions & PCRE2_UTF) != 0) as BOOL;
    let ucp: BOOL = (((*mb).poptions & PCRE2_UCP) != 0) as BOOL;

    let mut start_ecode: PCRE2_SPTR = start_ecode_arg;

    frame_copy_size = frame_size - core::mem::offset_of!(heapframe, eptr);

    F = (*match_data).heapframes as *mut heapframe;
    frames_top =
        ((*match_data).heapframes as *mut u8).add((*match_data).heapframes_size) as *mut heapframe;

    (*F).rdepth = 0;
    (*F).capture_last = 0;
    (*F).current_recurse = RECURSE_UNSET;
    (*F).eptr = start_eptr;
    (*F).start_match = start_eptr;
    (*F).mark = ptr::null();
    (*F).offset_top = 0;
    (*F).last_group_offset = PCRE2_UNSET;
    group_frame_type = 0;

    let mut state: i32 = ST_NEW_FRAME;

    // Frame-field accessor macros.
    macro_rules! Fecode { () => { (*F).ecode }; }
    macro_rules! Feptr { () => { (*F).eptr }; }
    macro_rules! Fop { () => { (*F).op }; }
    macro_rules! Fcapture_last { () => { (*F).capture_last }; }
    macro_rules! Fcurrent_recurse { () => { (*F).current_recurse }; }
    macro_rules! Flast_group_offset { () => { (*F).last_group_offset }; }
    macro_rules! Fmark { () => { (*F).mark }; }
    macro_rules! Frdepth { () => { (*F).rdepth }; }
    macro_rules! Fstart_match { () => { (*F).start_match }; }
    macro_rules! Foffset_top { () => { (*F).offset_top }; }
    macro_rules! Fovector { () => { fovec(F) }; }
    macro_rules! Fback_frame { () => { (*F).back_frame }; }

    macro_rules! SCHECK_PARTIAL { () => {
        if (*mb).partial != 0 && (Feptr!() > (*mb).start_used_ptr || (*mb).allowemptypartial != 0) {
            (*mb).hitend = TRUE;
            if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL; }
        }
    }; }
    macro_rules! CHECK_PARTIAL { () => {
        if Feptr!() >= (*mb).end_subject { SCHECK_PARTIAL!(); }
    }; }
    macro_rules! RMATCH { ($lbl:lifetime, $ra:expr, $rb:expr) => {{
        start_ecode = $ra;
        (*F).return_id = $rb;
        state = ST_MATCH_RECURSE;
        continue $lbl;
    }}; }
    macro_rules! RRETURN { ($lbl:lifetime, $ra:expr) => {{
        rrc = $ra;
        state = ST_RETURN_SWITCH;
        continue $lbl;
    }}; }
    macro_rules! NEXT_OP { ($lbl:lifetime) => {{ state = ST_MAINLOOP; continue $lbl; }}; }
    macro_rules! Fbyte1 { () => { *fbyte1(F) }; }
    macro_rules! Fbyte2 { () => { *fbyte2(F) }; }
    macro_rules! ES { () => { (*mb).end_subject }; }

    'machine: loop {
        match state {
            ST_MATCH_RECURSE => {
                N = frame_byte_add(F, frame_size);
                if frame_byte_add(N, frame_size) >= frames_top {
                    let usedsize = (N as usize) - ((*match_data).heapframes as usize);
                    let mut newsize: PCRE2_SIZE;

                    if (*match_data).heapframes_size >= PCRE2_SIZE_MAX / 2 {
                        if (*match_data).heapframes_size == PCRE2_SIZE_MAX - 1 {
                            return PCRE2_ERROR_NOMEMORY;
                        }
                        newsize = PCRE2_SIZE_MAX - 1;
                    } else {
                        newsize = (*match_data).heapframes_size * 2;
                    }

                    if newsize / 1024 >= (*mb).heap_limit as usize {
                        let old_size = (*match_data).heapframes_size / 1024;
                        if (*mb).heap_limit as usize <= old_size {
                            return PCRE2_ERROR_HEAPLIMIT;
                        } else {
                            let mut max_delta = 1024 * ((*mb).heap_limit as usize - old_size);
                            let over_bytes = (*match_data).heapframes_size % 1024;
                            if over_bytes != 0 {
                                max_delta -= 1024 - over_bytes;
                            }
                            newsize = (*match_data).heapframes_size + max_delta;
                        }
                    }

                    if newsize - usedsize < frame_size {
                        return PCRE2_ERROR_HEAPLIMIT;
                    }
                    let newmem = ((*match_data).memctl.malloc.unwrap())(
                        newsize,
                        (*match_data).memctl.memory_data,
                    );
                    if newmem.is_null() {
                        return PCRE2_ERROR_NOMEMORY;
                    }
                    ptr::copy_nonoverlapping(
                        (*match_data).heapframes as *const u8,
                        newmem as *mut u8,
                        usedsize,
                    );

                    N = (newmem as *mut u8).add(usedsize) as *mut heapframe;
                    F = frame_byte_sub(N, frame_size);

                    ((*match_data).memctl.free.unwrap())(
                        (*match_data).heapframes,
                        (*match_data).memctl.memory_data,
                    );
                    (*match_data).heapframes = newmem;
                    (*match_data).heapframes_size = newsize;
                    frames_top = (newmem as *mut u8).add(newsize) as *mut heapframe;
                }

                ptr::copy_nonoverlapping(
                    (F as *const u8).add(core::mem::offset_of!(heapframe, eptr)),
                    (N as *mut u8).add(core::mem::offset_of!(heapframe, eptr)),
                    frame_copy_size,
                );

                (*N).rdepth = Frdepth!() + 1;
                F = N;
                state = ST_NEW_FRAME;
                continue 'machine;
            }

            ST_NEW_FRAME => {
                (*F).group_frame_type = group_frame_type;
                (*F).ecode = start_ecode;
                (*F).back_frame = frame_size;

                if group_frame_type != 0 {
                    Flast_group_offset!() = (F as usize) - ((*match_data).heapframes as usize);
                    if GF_IDMASK(group_frame_type) == GF_RECURSE {
                        Fcurrent_recurse!() = GF_DATAMASK(group_frame_type);
                    }
                    group_frame_type = 0;
                }

                let cc = (*mb).match_call_count;
                (*mb).match_call_count = cc.wrapping_add(1);
                if cc >= (*mb).match_limit {
                    return PCRE2_ERROR_MATCHLIMIT;
                }
                if Frdepth!() >= (*mb).match_limit_depth {
                    return PCRE2_ERROR_DEPTHLIMIT;
                }

                state = ST_MAINLOOP;
                continue 'machine;
            }

            ST_RETURN_SWITCH => {
                if Feptr!() > (*mb).last_used_ptr {
                    (*mb).last_used_ptr = Feptr!();
                }
                if Frdepth!() == 0 {
                    return rrc;
                }
                F = frame_byte_sub(F, Fback_frame!());
                (*(*mb).cb).callout_flags |= PCRE2_CALLOUT_BACKTRACK;
                state = (*F).return_id as i32;
                continue 'machine;
            }

            ST_MAINLOOP => {
                Fop!() = *Fecode!();
                match Fop!() {
                    OP_CLOSE => {
                        if Fcurrent_recurse!() == RECURSE_UNSET {
                            number = GET2(Fecode!(), 1);
                            offset = Flast_group_offset!();
                            loop {
                                if offset == PCRE2_UNSET {
                                    return PCRE2_ERROR_INTERNAL;
                                }
                                N = frame_byte_add(
                                    (*match_data).heapframes as *mut heapframe,
                                    offset,
                                );
                                P = frame_byte_sub(N, frame_size);
                                if (*N).group_frame_type == (GF_CAPTURE | number) {
                                    break;
                                }
                                offset = (*P).last_group_offset;
                            }
                            offset = ((number as usize) << 1) - 2;
                            Fcapture_last!() = number;
                            *Fovector!().add(offset) =
                                (*P).eptr as usize - (*mb).start_subject as usize;
                            *Fovector!().add(offset + 1) =
                                Feptr!() as usize - (*mb).start_subject as usize;
                            if offset >= Foffset_top!() {
                                Foffset_top!() = offset + 2;
                            }
                        }
                        Fecode!() = Fecode!().add(op_length(*Fecode!()));
                        NEXT_OP!('machine);
                    }

                    OP_ASSERT_ACCEPT => {
                        if Feptr!() > (*mb).last_used_ptr {
                            (*mb).last_used_ptr = Feptr!();
                        }
                        assert_accept_frame = F;
                        RRETURN!('machine, MATCH_ACCEPT);
                    }

                    OP_ACCEPT | OP_END => {
                        if Fop!() == OP_ACCEPT && Fcurrent_recurse!() != RECURSE_UNSET {
                            offset = Flast_group_offset!();
                            loop {
                                if offset == PCRE2_UNSET {
                                    return PCRE2_ERROR_INTERNAL;
                                }
                                N = frame_byte_add(
                                    (*match_data).heapframes as *mut heapframe,
                                    offset,
                                );
                                P = frame_byte_sub(N, frame_size);
                                if GF_IDMASK((*N).group_frame_type) == GF_RECURSE {
                                    break;
                                }
                                offset = (*P).last_group_offset;
                            }
                            (*P).eptr = Feptr!();
                            (*P).mark = Fmark!();
                            (*P).start_match = Fstart_match!();
                            F = P;
                            Fecode!() = Fecode!().add(1 + LINK_SIZE);
                            NEXT_OP!('machine);
                        }

                        // OP_END (or ACCEPT not in recursion) common code.
                        if Feptr!() == Fstart_match!()
                            && (((*mb).moptions & PCRE2_NOTEMPTY) != 0
                                || (((*mb).moptions & PCRE2_NOTEMPTY_ATSTART) != 0
                                    && Fstart_match!()
                                        == (*mb).start_subject.add((*mb).start_offset)))
                        {
                            RRETURN!('machine, MATCH_NOMATCH);
                        }

                        if Feptr!() < (*mb).end_subject
                            && (((*mb).moptions | (*mb).poptions) & PCRE2_ENDANCHORED) != 0
                        {
                            if Fop!() == OP_END {
                                RRETURN!('machine, MATCH_NOMATCH);
                            }
                            return MATCH_NOMATCH; // (*ACCEPT)
                        }

                        if Fstart_match!() < (*mb).start_subject.add((*mb).start_offset)
                            || Fstart_match!() > Feptr!()
                        {
                            if (*mb).allowlookaroundbsk == 0 {
                                return PCRE2_ERROR_BAD_BACKSLASH_K;
                            }
                        }

                        (*mb).end_match_ptr = Feptr!();
                        (*mb).end_offset_top = Foffset_top!();
                        (*mb).mark = Fmark!();
                        if Feptr!() > (*mb).last_used_ptr {
                            (*mb).last_used_ptr = Feptr!();
                        }

                        *(*match_data).ovector.as_mut_ptr().add(0) =
                            Fstart_match!() as usize - (*mb).start_subject as usize;
                        *(*match_data).ovector.as_mut_ptr().add(1) =
                            Feptr!() as usize - (*mb).start_subject as usize;

                        let mut ii: usize = 2
                            * (if (top_bracket as usize + 1) > (*match_data).oveccount as usize {
                                (*match_data).oveccount as usize
                            } else {
                                top_bracket as usize + 1
                            });
                        ptr::copy_nonoverlapping(
                            Fovector!(),
                            (*match_data).ovector.as_mut_ptr().add(2),
                            ii - 2,
                        );
                        while {
                            ii -= 1;
                            ii >= Foffset_top!() + 2
                        } {
                            *(*match_data).ovector.as_mut_ptr().add(ii) = PCRE2_UNSET;
                        }
                        return MATCH_MATCH;
                    }

                    OP_ANY => {
                        if is_newline(Feptr!(), mb, utf) {
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        if (*mb).partial != 0
                            && Feptr!() == ES!().sub(1)
                            && (*mb).nltype == NLTYPE_FIXED
                            && (*mb).nllen == 2
                            && *Feptr!() == (*mb).nl[0]
                        {
                            (*mb).hitend = TRUE;
                            if (*mb).partial > 1 {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        // fall through to ALLANY
                        if Feptr!() >= ES!() {
                            SCHECK_PARTIAL!();
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        Feptr!() = Feptr!().add(1);
                        if utf != 0 {
                            while Feptr!() < ES!() && (*Feptr!() & 0xc0) == 0x80 {
                                Feptr!() = Feptr!().add(1);
                            }
                        }
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }

                    OP_ALLANY => {
                        if Feptr!() >= ES!() {
                            SCHECK_PARTIAL!();
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        Feptr!() = Feptr!().add(1);
                        if utf != 0 {
                            while Feptr!() < ES!() && (*Feptr!() & 0xc0) == 0x80 {
                                Feptr!() = Feptr!().add(1);
                            }
                        }
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }

                    OP_ANYBYTE => {
                        if Feptr!() >= ES!() {
                            SCHECK_PARTIAL!();
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        Feptr!() = Feptr!().add(1);
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }

                    OP_CHAR => {
                        if utf != 0 {
                            length = 1;
                            Fecode!() = Fecode!().add(1);
                            let (_c, extra) = GETCHARLEN(Fecode!());
                            length = 1 + extra as usize;
                            if length > (ES!() as usize - Feptr!() as usize) {
                                CHECK_PARTIAL!();
                                RRETURN!('machine, MATCH_NOMATCH);
                            }
                            while length > 0 {
                                if *Fecode!() != *Feptr!() {
                                    RRETURN!('machine, MATCH_NOMATCH);
                                }
                                Fecode!() = Fecode!().add(1);
                                Feptr!() = Feptr!().add(1);
                                length -= 1;
                            }
                        } else {
                            if (ES!() as usize).wrapping_sub(Feptr!() as usize) < 1 {
                                SCHECK_PARTIAL!();
                                RRETURN!('machine, MATCH_NOMATCH);
                            }
                            if *Fecode!().add(1) != *Feptr!() {
                                RRETURN!('machine, MATCH_NOMATCH);
                            }
                            Feptr!() = Feptr!().add(1);
                            Fecode!() = Fecode!().add(2);
                        }
                        NEXT_OP!('machine);
                    }

                    OP_CHARI => {
                        if Feptr!() >= ES!() {
                            SCHECK_PARTIAL!();
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        if utf != 0 {
                            length = 1;
                            Fecode!() = Fecode!().add(1);
                            let (cc0, extra) = GETCHARLEN(Fecode!());
                            fc = cc0;
                            length = 1 + extra as usize;
                            if fc < 128 {
                                let cc = *Feptr!() as u32;
                                if *(*mb).lcc.add(fc as usize) != *(*mb).lcc.add(cc as usize) {
                                    RRETURN!('machine, MATCH_NOMATCH);
                                }
                                Fecode!() = Fecode!().add(1);
                                Feptr!() = Feptr!().add(1);
                            } else {
                                let (dc, dn) = GETCHARINC(Feptr!());
                                Feptr!() = Feptr!().add(dn);
                                Fecode!() = Fecode!().add(length);
                                if dc != fc && dc != UCD_OTHERCASE(fc) {
                                    RRETURN!('machine, MATCH_NOMATCH);
                                }
                            }
                        } else if ucp != 0 {
                            let cc = *Feptr!() as u32;
                            fc = *Fecode!().add(1) as u32;
                            if fc < 128 {
                                if *(*mb).lcc.add(fc as usize) != *(*mb).lcc.add(cc as usize) {
                                    RRETURN!('machine, MATCH_NOMATCH);
                                }
                            } else {
                                if cc != fc && cc != UCD_OTHERCASE(fc) {
                                    RRETURN!('machine, MATCH_NOMATCH);
                                }
                            }
                            Feptr!() = Feptr!().add(1);
                            Fecode!() = Fecode!().add(2);
                        } else {
                            if *(*mb).lcc.add(*Fecode!().add(1) as usize)
                                != *(*mb).lcc.add(*Feptr!() as usize)
                            {
                                RRETURN!('machine, MATCH_NOMATCH);
                            }
                            Feptr!() = Feptr!().add(1);
                            Fecode!() = Fecode!().add(2);
                        }
                        NEXT_OP!('machine);
                    }

                    OP_NOT | OP_NOTI => {
                        if Feptr!() >= ES!() {
                            SCHECK_PARTIAL!();
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        if utf != 0 {
                            let mut ch: u32;
                            Fecode!() = Fecode!().add(1);
                            let (c0, n0) = GETCHARINC(Fecode!());
                            ch = c0;
                            Fecode!() = Fecode!().add(n0);
                            let (fc0, fn0) = GETCHARINC(Feptr!());
                            fc = fc0;
                            Feptr!() = Feptr!().add(fn0);
                            if ch == fc {
                                RRETURN!('machine, MATCH_NOMATCH);
                            } else if Fop!() == OP_NOTI {
                                if ch > 127 {
                                    ch = UCD_OTHERCASE(ch);
                                } else {
                                    ch = *(*mb).fcc.add(ch as usize) as u32;
                                }
                                if ch == fc {
                                    RRETURN!('machine, MATCH_NOMATCH);
                                }
                            }
                        } else if ucp != 0 {
                            let mut ch: u32;
                            fc = *Feptr!() as u32;
                            Feptr!() = Feptr!().add(1);
                            ch = *Fecode!().add(1) as u32;
                            Fecode!() = Fecode!().add(2);
                            if ch == fc {
                                RRETURN!('machine, MATCH_NOMATCH);
                            } else if Fop!() == OP_NOTI {
                                if ch > 127 {
                                    ch = UCD_OTHERCASE(ch);
                                } else {
                                    ch = *(*mb).fcc.add(ch as usize) as u32;
                                }
                                if ch == fc {
                                    RRETURN!('machine, MATCH_NOMATCH);
                                }
                            }
                        } else {
                            let ch = *Fecode!().add(1) as u32;
                            fc = *Feptr!() as u32;
                            Feptr!() = Feptr!().add(1);
                            if ch == fc
                                || (Fop!() == OP_NOTI
                                    && *(*mb).fcc.add(ch as usize) as u32 == fc)
                            {
                                RRETURN!('machine, MATCH_NOMATCH);
                            }
                            Fecode!() = Fecode!().add(2);
                        }
                        NEXT_OP!('machine);
                    }

                    OP_EXACT | OP_EXACTI => {
                        (*F).fields.char_repeat.min = GET2(Fecode!(), 1);
                        (*F).fields.char_repeat.max = (*F).fields.char_repeat.min;
                        Fecode!() = Fecode!().add(1 + IMM2_SIZE);
                        state = ST_REPEATCHAR;
                        continue 'machine;
                    }
                    OP_POSUPTO | OP_POSUPTOI => {
                        reptype = REPTYPE_POS;
                        (*F).fields.char_repeat.min = 0;
                        (*F).fields.char_repeat.max = GET2(Fecode!(), 1);
                        Fecode!() = Fecode!().add(1 + IMM2_SIZE);
                        state = ST_REPEATCHAR;
                        continue 'machine;
                    }
                    OP_UPTO | OP_UPTOI => {
                        reptype = REPTYPE_MAX;
                        (*F).fields.char_repeat.min = 0;
                        (*F).fields.char_repeat.max = GET2(Fecode!(), 1);
                        Fecode!() = Fecode!().add(1 + IMM2_SIZE);
                        state = ST_REPEATCHAR;
                        continue 'machine;
                    }
                    OP_MINUPTO | OP_MINUPTOI => {
                        reptype = REPTYPE_MIN;
                        (*F).fields.char_repeat.min = 0;
                        (*F).fields.char_repeat.max = GET2(Fecode!(), 1);
                        Fecode!() = Fecode!().add(1 + IMM2_SIZE);
                        state = ST_REPEATCHAR;
                        continue 'machine;
                    }
                    OP_POSSTAR | OP_POSSTARI => {
                        reptype = REPTYPE_POS;
                        (*F).fields.char_repeat.min = 0;
                        (*F).fields.char_repeat.max = u32::MAX;
                        Fecode!() = Fecode!().add(1);
                        state = ST_REPEATCHAR;
                        continue 'machine;
                    }
                    OP_POSPLUS | OP_POSPLUSI => {
                        reptype = REPTYPE_POS;
                        (*F).fields.char_repeat.min = 1;
                        (*F).fields.char_repeat.max = u32::MAX;
                        Fecode!() = Fecode!().add(1);
                        state = ST_REPEATCHAR;
                        continue 'machine;
                    }
                    OP_POSQUERY | OP_POSQUERYI => {
                        reptype = REPTYPE_POS;
                        (*F).fields.char_repeat.min = 0;
                        (*F).fields.char_repeat.max = 1;
                        Fecode!() = Fecode!().add(1);
                        state = ST_REPEATCHAR;
                        continue 'machine;
                    }
                    OP_STAR | OP_STARI | OP_MINSTAR | OP_MINSTARI | OP_PLUS | OP_PLUSI
                    | OP_MINPLUS | OP_MINPLUSI | OP_QUERY | OP_QUERYI | OP_MINQUERY
                    | OP_MINQUERYI => {
                        fc = (*Fecode!() as u32)
                            - (if Fop!() < OP_STARI { OP_STAR as u32 } else { OP_STARI as u32 });
                        Fecode!() = Fecode!().add(1);
                        (*F).fields.char_repeat.min = REP_MIN[fc as usize];
                        (*F).fields.char_repeat.max = REP_MAX[fc as usize];
                        reptype = REP_TYP[fc as usize];
                        state = ST_REPEATCHAR;
                        continue 'machine;
                    }

                    _ => {
                        // Fall through to extended dispatch (handled after the
                        // primary match below via the ST_MAINLOOP2 mechanism).
                        state = ST_MAINLOOP2;
                        continue 'machine;
                    }
                }
            }

            ST_REPEATCHAR => {
                macro_rules! Llength { () => { *fbyte1(F) }; }
                macro_rules! Loclength { () => { *fbyte2(F) }; }
                macro_rules! Lstart_eptr { () => { (*F).fields.char_repeat.start_eptr }; }
                macro_rules! Lcharptr { () => { (*F).fields.char_repeat.charptr }; }
                macro_rules! Lmin { () => { (*F).fields.char_repeat.min }; }
                macro_rules! Lmax { () => { (*F).fields.char_repeat.max }; }
                macro_rules! Lc { () => { (*F).fields.char_repeat.c }; }
                macro_rules! Loc { () => { (*F).fields.char_repeat.oc }; }
                let loccu = ptr::addr_of_mut!((*F).fields.char_repeat.oc) as *mut u8;

                if utf != 0 {
                    length = 1;
                    Lcharptr!() = Fecode!();
                    let (c0, extra) = GETCHARLEN(Fecode!());
                    fc = c0;
                    length = 1 + extra as usize;
                    Fecode!() = Fecode!().add(length);
                    Llength!() = length as u8;

                    if length > 1 {
                        let othercase;
                        if Fop!() >= OP_STARI && {
                            othercase = UCD_OTHERCASE(fc);
                            othercase != fc
                        } {
                            Loclength!() = _pcre2_ord2utf_8(othercase, loccu) as u8;
                        } else {
                            Loclength!() = 0;
                        }

                        i = 1;
                        while i <= Lmin!() {
                            if Feptr!() <= ES!().sub(length)
                                && mem_eq(Feptr!(), Lcharptr!(), length)
                            {
                                Feptr!() = Feptr!().add(length);
                            } else if Loclength!() > 0
                                && Feptr!() <= ES!().sub(Loclength!() as usize)
                                && mem_eq(Feptr!(), loccu, Loclength!() as usize)
                            {
                                Feptr!() = Feptr!().add(Loclength!() as usize);
                            } else {
                                CHECK_PARTIAL!();
                                RRETURN!('machine, MATCH_NOMATCH);
                            }
                            i += 1;
                        }

                        if Lmin!() == Lmax!() {
                            NEXT_OP!('machine);
                        }

                        if reptype == REPTYPE_MIN {
                            RMATCH!('machine, Fecode!(), RM202);
                        } else {
                            Lstart_eptr!() = Feptr!();
                            i = Lmin!();
                            while i < Lmax!() {
                                if Feptr!() <= ES!().sub(Llength!() as usize)
                                    && mem_eq(Feptr!(), Lcharptr!(), Llength!() as usize)
                                {
                                    Feptr!() = Feptr!().add(Llength!() as usize);
                                } else if Loclength!() > 0
                                    && Feptr!() <= ES!().sub(Loclength!() as usize)
                                    && mem_eq(Feptr!(), loccu, Loclength!() as usize)
                                {
                                    Feptr!() = Feptr!().add(Loclength!() as usize);
                                } else {
                                    CHECK_PARTIAL!();
                                    break;
                                }
                                i += 1;
                            }
                            if reptype != REPTYPE_POS {
                                state = ST_RC_RM203_LOOP;
                                continue 'machine;
                            }
                        }
                        NEXT_OP!('machine);
                    }
                    Lc!() = fc;
                } else {
                    Lc!() = *Fecode!() as u32;
                    Fecode!() = Fecode!().add(1);
                }

                // Single-code-unit character.
                if Fop!() >= OP_STARI {
                    // Caseless.
                    if ucp != 0 && utf == 0 && Lc!() > 127 {
                        Loc!() = UCD_OTHERCASE(Lc!());
                    } else {
                        Loc!() = *(*mb).fcc.add(Lc!() as usize) as u32;
                    }

                    i = 1;
                    while i <= Lmin!() {
                        if Feptr!() >= ES!() {
                            SCHECK_PARTIAL!();
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        let cc = *Feptr!() as u32;
                        if Lc!() != cc && Loc!() != cc {
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        Feptr!() = Feptr!().add(1);
                        i += 1;
                    }
                    if Lmin!() == Lmax!() {
                        NEXT_OP!('machine);
                    }

                    if reptype == REPTYPE_MIN {
                        RMATCH!('machine, Fecode!(), RM25);
                    } else {
                        Lstart_eptr!() = Feptr!();
                        i = Lmin!();
                        while i < Lmax!() {
                            if Feptr!() >= ES!() {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            let cc = *Feptr!() as u32;
                            if Lc!() != cc && Loc!() != cc {
                                break;
                            }
                            Feptr!() = Feptr!().add(1);
                            i += 1;
                        }
                        if reptype != REPTYPE_POS {
                            state = ST_RC_RM26_LOOP;
                            continue 'machine;
                        }
                    }
                    NEXT_OP!('machine);
                } else {
                    // Caseful.
                    i = 1;
                    while i <= Lmin!() {
                        if Feptr!() >= ES!() {
                            SCHECK_PARTIAL!();
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        if Lc!() != *Feptr!() as u32 {
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        Feptr!() = Feptr!().add(1);
                        i += 1;
                    }
                    if Lmin!() == Lmax!() {
                        NEXT_OP!('machine);
                    }

                    if reptype == REPTYPE_MIN {
                        RMATCH!('machine, Fecode!(), RM27);
                    } else {
                        Lstart_eptr!() = Feptr!();
                        i = Lmin!();
                        while i < Lmax!() {
                            if Feptr!() >= ES!() {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            if Lc!() != *Feptr!() as u32 {
                                break;
                            }
                            Feptr!() = Feptr!().add(1);
                            i += 1;
                        }
                        if reptype != REPTYPE_POS {
                            state = ST_RC_RM28_LOOP;
                            continue 'machine;
                        }
                    }
                    NEXT_OP!('machine);
                }
            }

            // ---- REPEATCHAR minimize/maximize resume points ----
            x if x == RM202 as i32 => {
                // Minimize, wide UTF char.
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                let lmin = (*F).fields.char_repeat.min;
                (*F).fields.char_repeat.min = lmin + 1;
                if lmin >= (*F).fields.char_repeat.max { RRETURN!('machine, MATCH_NOMATCH); }
                let llen = *fbyte1(F) as usize;
                let loclen = *fbyte2(F) as usize;
                let charptr = (*F).fields.char_repeat.charptr;
                let loccu = ptr::addr_of!((*F).fields.char_repeat.oc) as *const u8;
                if Feptr!() <= ES!().sub(llen) && mem_eq(Feptr!(), charptr, llen) {
                    Feptr!() = Feptr!().add(llen);
                } else if loclen > 0 && Feptr!() <= ES!().sub(loclen) && mem_eq(Feptr!(), loccu, loclen) {
                    Feptr!() = Feptr!().add(loclen);
                } else {
                    CHECK_PARTIAL!();
                    RRETURN!('machine, MATCH_NOMATCH);
                }
                RMATCH!('machine, Fecode!(), RM202);
            }
            ST_RC_RM203_LOOP => {
                if Feptr!() <= (*F).fields.char_repeat.start_eptr { NEXT_OP!('machine); }
                RMATCH!('machine, Fecode!(), RM203);
            }
            x if x == RM203 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                Feptr!() = Feptr!().sub(1);
                Feptr!() = BACKCHAR(Feptr!());
                state = ST_RC_RM203_LOOP;
                continue 'machine;
            }
            x if x == RM25 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                let lmin = (*F).fields.char_repeat.min;
                (*F).fields.char_repeat.min = lmin + 1;
                if lmin >= (*F).fields.char_repeat.max { RRETURN!('machine, MATCH_NOMATCH); }
                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                let cc = *Feptr!() as u32;
                if (*F).fields.char_repeat.c != cc && (*F).fields.char_repeat.oc != cc {
                    RRETURN!('machine, MATCH_NOMATCH);
                }
                Feptr!() = Feptr!().add(1);
                RMATCH!('machine, Fecode!(), RM25);
            }
            ST_RC_RM26_LOOP => {
                if Feptr!() == (*F).fields.char_repeat.start_eptr { NEXT_OP!('machine); }
                RMATCH!('machine, Fecode!(), RM26);
            }
            x if x == RM26 as i32 => {
                Feptr!() = Feptr!().sub(1);
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                state = ST_RC_RM26_LOOP;
                continue 'machine;
            }
            x if x == RM27 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                let lmin = (*F).fields.char_repeat.min;
                (*F).fields.char_repeat.min = lmin + 1;
                if lmin >= (*F).fields.char_repeat.max { RRETURN!('machine, MATCH_NOMATCH); }
                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                if (*F).fields.char_repeat.c != *Feptr!() as u32 { RRETURN!('machine, MATCH_NOMATCH); }
                Feptr!() = Feptr!().add(1);
                RMATCH!('machine, Fecode!(), RM27);
            }
            ST_RC_RM28_LOOP => {
                if Feptr!() <= (*F).fields.char_repeat.start_eptr { NEXT_OP!('machine); }
                RMATCH!('machine, Fecode!(), RM28);
            }
            x if x == RM28 as i32 => {
                Feptr!() = Feptr!().sub(1);
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                state = ST_RC_RM28_LOOP;
                continue 'machine;
            }

            // ===== ST_MAINLOOP2: continuation of opcode dispatch =====
            ST_MAINLOOP2 => {
                match Fop!() {
                    OP_NOTEXACT | OP_NOTEXACTI => {
                        (*F).fields.charnot_repeat.min = GET2(Fecode!(), 1);
                        (*F).fields.charnot_repeat.max = (*F).fields.charnot_repeat.min;
                        Fecode!() = Fecode!().add(1 + IMM2_SIZE);
                        state = ST_REPEATNOTCHAR;
                        continue 'machine;
                    }
                    OP_NOTUPTO | OP_NOTUPTOI => {
                        (*F).fields.charnot_repeat.min = 0;
                        (*F).fields.charnot_repeat.max = GET2(Fecode!(), 1);
                        reptype = REPTYPE_MAX;
                        Fecode!() = Fecode!().add(1 + IMM2_SIZE);
                        state = ST_REPEATNOTCHAR;
                        continue 'machine;
                    }
                    OP_NOTMINUPTO | OP_NOTMINUPTOI => {
                        (*F).fields.charnot_repeat.min = 0;
                        (*F).fields.charnot_repeat.max = GET2(Fecode!(), 1);
                        reptype = REPTYPE_MIN;
                        Fecode!() = Fecode!().add(1 + IMM2_SIZE);
                        state = ST_REPEATNOTCHAR;
                        continue 'machine;
                    }
                    OP_NOTPOSSTAR | OP_NOTPOSSTARI => {
                        reptype = REPTYPE_POS;
                        (*F).fields.charnot_repeat.min = 0;
                        (*F).fields.charnot_repeat.max = u32::MAX;
                        Fecode!() = Fecode!().add(1);
                        state = ST_REPEATNOTCHAR;
                        continue 'machine;
                    }
                    OP_NOTPOSPLUS | OP_NOTPOSPLUSI => {
                        reptype = REPTYPE_POS;
                        (*F).fields.charnot_repeat.min = 1;
                        (*F).fields.charnot_repeat.max = u32::MAX;
                        Fecode!() = Fecode!().add(1);
                        state = ST_REPEATNOTCHAR;
                        continue 'machine;
                    }
                    OP_NOTPOSQUERY | OP_NOTPOSQUERYI => {
                        reptype = REPTYPE_POS;
                        (*F).fields.charnot_repeat.min = 0;
                        (*F).fields.charnot_repeat.max = 1;
                        Fecode!() = Fecode!().add(1);
                        state = ST_REPEATNOTCHAR;
                        continue 'machine;
                    }
                    OP_NOTPOSUPTO | OP_NOTPOSUPTOI => {
                        reptype = REPTYPE_POS;
                        (*F).fields.charnot_repeat.min = 0;
                        (*F).fields.charnot_repeat.max = GET2(Fecode!(), 1);
                        Fecode!() = Fecode!().add(1 + IMM2_SIZE);
                        state = ST_REPEATNOTCHAR;
                        continue 'machine;
                    }
                    OP_NOTSTAR | OP_NOTSTARI | OP_NOTMINSTAR | OP_NOTMINSTARI | OP_NOTPLUS
                    | OP_NOTPLUSI | OP_NOTMINPLUS | OP_NOTMINPLUSI | OP_NOTQUERY | OP_NOTQUERYI
                    | OP_NOTMINQUERY | OP_NOTMINQUERYI => {
                        fc = (*Fecode!() as u32)
                            - (if Fop!() >= OP_NOTSTARI { OP_NOTSTARI as u32 } else { OP_NOTSTAR as u32 });
                        Fecode!() = Fecode!().add(1);
                        (*F).fields.charnot_repeat.min = REP_MIN[fc as usize];
                        (*F).fields.charnot_repeat.max = REP_MAX[fc as usize];
                        reptype = REP_TYP[fc as usize];
                        state = ST_REPEATNOTCHAR;
                        continue 'machine;
                    }
                    _ => {
                        state = ST_MAINLOOP3;
                        continue 'machine;
                    }
                }
            }

            ST_REPEATNOTCHAR => {
                macro_rules! Lstart_eptr { () => { (*F).fields.charnot_repeat.start_eptr }; }
                macro_rules! Lmin { () => { (*F).fields.charnot_repeat.min }; }
                macro_rules! Lmax { () => { (*F).fields.charnot_repeat.max }; }
                macro_rules! Lc { () => { (*F).fields.charnot_repeat.c }; }
                macro_rules! Loc { () => { (*F).fields.charnot_repeat.oc }; }

                // GETCHARINCTEST(Lc, Fecode)
                {
                    let c0 = *Fecode!() as u32;
                    if utf != 0 && c0 >= 0xc0 {
                        let (v, n) = GETCHARINC(Fecode!());
                        Lc!() = v;
                        Fecode!() = Fecode!().add(n);
                    } else {
                        Lc!() = c0;
                        Fecode!() = Fecode!().add(1);
                    }
                }

                if Fop!() >= OP_NOTSTARI {
                    // Caseless.
                    if (utf != 0 || ucp != 0) && Lc!() > 127 {
                        Loc!() = UCD_OTHERCASE(Lc!());
                    } else {
                        Loc!() = *(*mb).fcc.add(Lc!() as usize) as u32;
                    }

                    if utf != 0 {
                        i = 1;
                        while i <= Lmin!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                            let (d, dn) = GETCHARINC(Feptr!());
                            Feptr!() = Feptr!().add(dn);
                            if Lc!() == d || Loc!() == d { RRETURN!('machine, MATCH_NOMATCH); }
                            i += 1;
                        }
                    } else {
                        i = 1;
                        while i <= Lmin!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                            if Lc!() == *Feptr!() as u32 || Loc!() == *Feptr!() as u32 {
                                RRETURN!('machine, MATCH_NOMATCH);
                            }
                            Feptr!() = Feptr!().add(1);
                            i += 1;
                        }
                    }

                    if Lmin!() == Lmax!() { NEXT_OP!('machine); }

                    if reptype == REPTYPE_MIN {
                        if utf != 0 { RMATCH!('machine, Fecode!(), RM204); }
                        else { RMATCH!('machine, Fecode!(), RM29); }
                    } else {
                        Lstart_eptr!() = Feptr!();
                        if utf != 0 {
                            i = Lmin!();
                            while i < Lmax!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                let (d, dl) = GETCHARLEN(Feptr!());
                                let len = 1 + dl as usize;
                                if Lc!() == d || Loc!() == d { break; }
                                Feptr!() = Feptr!().add(len);
                                i += 1;
                            }
                            if reptype != REPTYPE_POS {
                                state = ST_RNC_RM205_LOOP;
                                continue 'machine;
                            }
                        } else {
                            i = Lmin!();
                            while i < Lmax!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                if Lc!() == *Feptr!() as u32 || Loc!() == *Feptr!() as u32 { break; }
                                Feptr!() = Feptr!().add(1);
                                i += 1;
                            }
                            if reptype != REPTYPE_POS {
                                state = ST_RNC_RM30_LOOP;
                                continue 'machine;
                            }
                        }
                    }
                    NEXT_OP!('machine);
                } else {
                    // Caseful.
                    if utf != 0 {
                        i = 1;
                        while i <= Lmin!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                            let (d, dn) = GETCHARINC(Feptr!());
                            Feptr!() = Feptr!().add(dn);
                            if Lc!() == d { RRETURN!('machine, MATCH_NOMATCH); }
                            i += 1;
                        }
                    } else {
                        i = 1;
                        while i <= Lmin!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                            if Lc!() == *Feptr!() as u32 { RRETURN!('machine, MATCH_NOMATCH); }
                            Feptr!() = Feptr!().add(1);
                            i += 1;
                        }
                    }

                    if Lmin!() == Lmax!() { NEXT_OP!('machine); }

                    if reptype == REPTYPE_MIN {
                        if utf != 0 { RMATCH!('machine, Fecode!(), RM206); }
                        else { RMATCH!('machine, Fecode!(), RM31); }
                    } else {
                        Lstart_eptr!() = Feptr!();
                        if utf != 0 {
                            i = Lmin!();
                            while i < Lmax!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                let (d, dl) = GETCHARLEN(Feptr!());
                                let len = 1 + dl as usize;
                                if Lc!() == d { break; }
                                Feptr!() = Feptr!().add(len);
                                i += 1;
                            }
                            if reptype != REPTYPE_POS {
                                state = ST_RNC_RM207_LOOP;
                                continue 'machine;
                            }
                        } else {
                            i = Lmin!();
                            while i < Lmax!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                if Lc!() == *Feptr!() as u32 { break; }
                                Feptr!() = Feptr!().add(1);
                                i += 1;
                            }
                            if reptype != REPTYPE_POS {
                                state = ST_RNC_RM32_LOOP;
                                continue 'machine;
                            }
                        }
                    }
                    NEXT_OP!('machine);
                }
            }

            // ---- REPEATNOTCHAR resume points ----
            x if x == RM204 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                let lmin = (*F).fields.charnot_repeat.min;
                (*F).fields.charnot_repeat.min = lmin + 1;
                if lmin >= (*F).fields.charnot_repeat.max { RRETURN!('machine, MATCH_NOMATCH); }
                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                let (d, dn) = GETCHARINC(Feptr!());
                Feptr!() = Feptr!().add(dn);
                if (*F).fields.charnot_repeat.c == d || (*F).fields.charnot_repeat.oc == d {
                    RRETURN!('machine, MATCH_NOMATCH);
                }
                RMATCH!('machine, Fecode!(), RM204);
            }
            x if x == RM29 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                let lmin = (*F).fields.charnot_repeat.min;
                (*F).fields.charnot_repeat.min = lmin + 1;
                if lmin >= (*F).fields.charnot_repeat.max { RRETURN!('machine, MATCH_NOMATCH); }
                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                if (*F).fields.charnot_repeat.c == *Feptr!() as u32
                    || (*F).fields.charnot_repeat.oc == *Feptr!() as u32
                {
                    RRETURN!('machine, MATCH_NOMATCH);
                }
                Feptr!() = Feptr!().add(1);
                RMATCH!('machine, Fecode!(), RM29);
            }
            ST_RNC_RM205_LOOP => {
                if Feptr!() <= (*F).fields.charnot_repeat.start_eptr { NEXT_OP!('machine); }
                RMATCH!('machine, Fecode!(), RM205);
            }
            x if x == RM205 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                Feptr!() = Feptr!().sub(1);
                Feptr!() = BACKCHAR(Feptr!());
                state = ST_RNC_RM205_LOOP;
                continue 'machine;
            }
            ST_RNC_RM30_LOOP => {
                if Feptr!() == (*F).fields.charnot_repeat.start_eptr { NEXT_OP!('machine); }
                RMATCH!('machine, Fecode!(), RM30);
            }
            x if x == RM30 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                Feptr!() = Feptr!().sub(1);
                state = ST_RNC_RM30_LOOP;
                continue 'machine;
            }
            x if x == RM206 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                let lmin = (*F).fields.charnot_repeat.min;
                (*F).fields.charnot_repeat.min = lmin + 1;
                if lmin >= (*F).fields.charnot_repeat.max { RRETURN!('machine, MATCH_NOMATCH); }
                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                let (d, dn) = GETCHARINC(Feptr!());
                Feptr!() = Feptr!().add(dn);
                if (*F).fields.charnot_repeat.c == d { RRETURN!('machine, MATCH_NOMATCH); }
                RMATCH!('machine, Fecode!(), RM206);
            }
            x if x == RM31 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                let lmin = (*F).fields.charnot_repeat.min;
                (*F).fields.charnot_repeat.min = lmin + 1;
                if lmin >= (*F).fields.charnot_repeat.max { RRETURN!('machine, MATCH_NOMATCH); }
                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                if (*F).fields.charnot_repeat.c == *Feptr!() as u32 { RRETURN!('machine, MATCH_NOMATCH); }
                Feptr!() = Feptr!().add(1);
                RMATCH!('machine, Fecode!(), RM31);
            }
            ST_RNC_RM207_LOOP => {
                if Feptr!() <= (*F).fields.charnot_repeat.start_eptr { NEXT_OP!('machine); }
                RMATCH!('machine, Fecode!(), RM207);
            }
            x if x == RM207 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                Feptr!() = Feptr!().sub(1);
                Feptr!() = BACKCHAR(Feptr!());
                state = ST_RNC_RM207_LOOP;
                continue 'machine;
            }
            ST_RNC_RM32_LOOP => {
                if Feptr!() == (*F).fields.charnot_repeat.start_eptr { NEXT_OP!('machine); }
                RMATCH!('machine, Fecode!(), RM32);
            }
            x if x == RM32 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                Feptr!() = Feptr!().sub(1);
                state = ST_RNC_RM32_LOOP;
                continue 'machine;
            }

            // ===== ST_MAINLOOP3: class / xclass / eclass / type opcodes =====
            ST_MAINLOOP3 => {
                match Fop!() {
                    OP_NCLASS | OP_CLASS => {
                        macro_rules! Lbyte_map_address { () => { (*F).fields.class_repeat.byte_map_address }; }
                        macro_rules! Lstart_eptr { () => { (*F).fields.class_repeat.start_eptr }; }
                        macro_rules! Lmin { () => { (*F).fields.class_repeat.min }; }
                        macro_rules! Lmax { () => { (*F).fields.class_repeat.max }; }

                        Lbyte_map_address!() = Fecode!().add(1);
                        let lbyte_map = Lbyte_map_address!();
                        Fecode!() = Fecode!().add(1 + 32);

                        match *Fecode!() {
                            OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
                            | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
                                fc = *Fecode!() as u32 - OP_CRSTAR as u32;
                                Fecode!() = Fecode!().add(1);
                                Lmin!() = REP_MIN[fc as usize];
                                Lmax!() = REP_MAX[fc as usize];
                                reptype = REP_TYP[fc as usize];
                            }
                            OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                                Lmin!() = GET2(Fecode!(), 1);
                                Lmax!() = GET2(Fecode!(), 1 + IMM2_SIZE);
                                if Lmax!() == 0 { Lmax!() = u32::MAX; }
                                reptype = REP_TYP[(*Fecode!() - OP_CRSTAR) as usize];
                                Fecode!() = Fecode!().add(1 + 2 * IMM2_SIZE);
                            }
                            _ => {
                                Lmin!() = 1;
                                Lmax!() = 1;
                            }
                        }

                        // Ensure minimum.
                        if utf != 0 {
                            i = 1;
                            while i <= Lmin!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                                let (c0, cn) = GETCHARINC(Feptr!());
                                fc = c0;
                                Feptr!() = Feptr!().add(cn);
                                if fc > 255 {
                                    if Fop!() == OP_CLASS { RRETURN!('machine, MATCH_NOMATCH); }
                                } else if (*lbyte_map.add((fc / 8) as usize) & (1u8 << (fc & 7))) == 0 {
                                    RRETURN!('machine, MATCH_NOMATCH);
                                }
                                i += 1;
                            }
                        } else {
                            i = 1;
                            while i <= Lmin!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                                fc = *Feptr!() as u32;
                                Feptr!() = Feptr!().add(1);
                                if (*lbyte_map.add((fc / 8) as usize) & (1u8 << (fc & 7))) == 0 {
                                    RRETURN!('machine, MATCH_NOMATCH);
                                }
                                i += 1;
                            }
                        }

                        if Lmin!() == Lmax!() { NEXT_OP!('machine); }

                        if reptype == REPTYPE_MIN {
                            if utf != 0 { RMATCH!('machine, Fecode!(), RM200); }
                            else { RMATCH!('machine, Fecode!(), RM23); }
                        } else {
                            Lstart_eptr!() = Feptr!();
                            if utf != 0 {
                                i = Lmin!();
                                while i < Lmax!() {
                                    if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                    let (c0, cl) = GETCHARLEN(Feptr!());
                                    fc = c0;
                                    let len = 1 + cl as usize;
                                    if fc > 255 {
                                        if Fop!() == OP_CLASS { break; }
                                    } else if (*lbyte_map.add((fc / 8) as usize) & (1u8 << (fc & 7))) == 0 {
                                        break;
                                    }
                                    Feptr!() = Feptr!().add(len);
                                    i += 1;
                                }
                                if reptype == REPTYPE_POS { NEXT_OP!('machine); }
                                state = ST_CLASS_RM201_LOOP;
                                continue 'machine;
                            } else {
                                i = Lmin!();
                                while i < Lmax!() {
                                    if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                    fc = *Feptr!() as u32;
                                    if (*lbyte_map.add((fc / 8) as usize) & (1u8 << (fc & 7))) == 0 {
                                        break;
                                    }
                                    Feptr!() = Feptr!().add(1);
                                    i += 1;
                                }
                                if reptype == REPTYPE_POS { NEXT_OP!('machine); }
                                state = ST_CLASS_RM24_LOOP;
                                continue 'machine;
                            }
                        }
                    }

                    OP_XCLASS => {
                        macro_rules! Lstart_eptr { () => { (*F).fields.xclass_repeat.start_eptr }; }
                        macro_rules! Lxclass_data { () => { (*F).fields.xclass_repeat.xclass_data }; }
                        macro_rules! Lmin { () => { (*F).fields.xclass_repeat.min }; }
                        macro_rules! Lmax { () => { (*F).fields.xclass_repeat.max }; }

                        Lxclass_data!() = Fecode!().add(1 + LINK_SIZE);
                        Fecode!() = Fecode!().add(GET(Fecode!(), 1) as usize);

                        match *Fecode!() {
                            OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
                            | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
                                fc = *Fecode!() as u32 - OP_CRSTAR as u32;
                                Fecode!() = Fecode!().add(1);
                                Lmin!() = REP_MIN[fc as usize];
                                Lmax!() = REP_MAX[fc as usize];
                                reptype = REP_TYP[fc as usize];
                            }
                            OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                                Lmin!() = GET2(Fecode!(), 1);
                                Lmax!() = GET2(Fecode!(), 1 + IMM2_SIZE);
                                if Lmax!() == 0 { Lmax!() = u32::MAX; }
                                reptype = REP_TYP[(*Fecode!() - OP_CRSTAR) as usize];
                                Fecode!() = Fecode!().add(1 + 2 * IMM2_SIZE);
                            }
                            _ => {
                                Lmin!() = 1;
                                Lmax!() = 1;
                            }
                        }

                        i = 1;
                        while i <= Lmin!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                            let c0 = *Feptr!() as u32;
                            if utf != 0 && c0 >= 0xc0 {
                                let (v, n) = GETCHARINC(Feptr!());
                                fc = v;
                                Feptr!() = Feptr!().add(n);
                            } else {
                                fc = c0;
                                Feptr!() = Feptr!().add(1);
                            }
                            if _pcre2_xclass_8(fc, Lxclass_data!(), (*mb).start_code, utf) == 0 {
                                RRETURN!('machine, MATCH_NOMATCH);
                            }
                            i += 1;
                        }

                        if Lmin!() == Lmax!() { NEXT_OP!('machine); }

                        if reptype == REPTYPE_MIN {
                            RMATCH!('machine, Fecode!(), RM100);
                        } else {
                            Lstart_eptr!() = Feptr!();
                            i = Lmin!();
                            while i < Lmax!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                let (c0, cl) = GETCHARLEN(Feptr!());
                                let (fcv, len) = if utf != 0 {
                                    (c0, 1 + cl as usize)
                                } else {
                                    (*Feptr!() as u32, 1usize)
                                };
                                fc = fcv;
                                if _pcre2_xclass_8(fc, Lxclass_data!(), (*mb).start_code, utf) == 0 {
                                    break;
                                }
                                Feptr!() = Feptr!().add(len);
                                i += 1;
                            }
                            if reptype == REPTYPE_POS { NEXT_OP!('machine); }
                            state = ST_XCLASS_RM101_LOOP;
                            continue 'machine;
                        }
                    }

                    OP_ECLASS => {
                        macro_rules! Lstart_eptr { () => { (*F).fields.eclass_repeat.start_eptr }; }
                        macro_rules! Leclass_data { () => { (*F).fields.eclass_repeat.eclass_data }; }
                        macro_rules! Leclass_len { () => { (*F).fields.eclass_repeat.eclass_len }; }
                        macro_rules! Lmin { () => { (*F).fields.eclass_repeat.min }; }
                        macro_rules! Lmax { () => { (*F).fields.eclass_repeat.max }; }

                        Leclass_data!() = Fecode!().add(1 + LINK_SIZE);
                        Fecode!() = Fecode!().add(GET(Fecode!(), 1) as usize);
                        Leclass_len!() = Fecode!() as usize - Leclass_data!() as usize;

                        match *Fecode!() {
                            OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
                            | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
                                fc = *Fecode!() as u32 - OP_CRSTAR as u32;
                                Fecode!() = Fecode!().add(1);
                                Lmin!() = REP_MIN[fc as usize];
                                Lmax!() = REP_MAX[fc as usize];
                                reptype = REP_TYP[fc as usize];
                            }
                            OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                                Lmin!() = GET2(Fecode!(), 1);
                                Lmax!() = GET2(Fecode!(), 1 + IMM2_SIZE);
                                if Lmax!() == 0 { Lmax!() = u32::MAX; }
                                reptype = REP_TYP[(*Fecode!() - OP_CRSTAR) as usize];
                                Fecode!() = Fecode!().add(1 + 2 * IMM2_SIZE);
                            }
                            _ => {
                                Lmin!() = 1;
                                Lmax!() = 1;
                            }
                        }

                        i = 1;
                        while i <= Lmin!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                            let c0 = *Feptr!() as u32;
                            if utf != 0 && c0 >= 0xc0 {
                                let (v, n) = GETCHARINC(Feptr!());
                                fc = v;
                                Feptr!() = Feptr!().add(n);
                            } else {
                                fc = c0;
                                Feptr!() = Feptr!().add(1);
                            }
                            if _pcre2_eclass_8(
                                fc,
                                Leclass_data!(),
                                Leclass_data!().add(Leclass_len!()),
                                (*mb).start_code,
                                utf,
                            ) == 0 {
                                RRETURN!('machine, MATCH_NOMATCH);
                            }
                            i += 1;
                        }

                        if Lmin!() == Lmax!() { NEXT_OP!('machine); }

                        if reptype == REPTYPE_MIN {
                            RMATCH!('machine, Fecode!(), RM102);
                        } else {
                            Lstart_eptr!() = Feptr!();
                            i = Lmin!();
                            while i < Lmax!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                let (c0, cl) = GETCHARLEN(Feptr!());
                                let (fcv, len) = if utf != 0 {
                                    (c0, 1 + cl as usize)
                                } else {
                                    (*Feptr!() as u32, 1usize)
                                };
                                fc = fcv;
                                if _pcre2_eclass_8(
                                    fc,
                                    Leclass_data!(),
                                    Leclass_data!().add(Leclass_len!()),
                                    (*mb).start_code,
                                    utf,
                                ) == 0 {
                                    break;
                                }
                                Feptr!() = Feptr!().add(len);
                                i += 1;
                            }
                            if reptype == REPTYPE_POS { NEXT_OP!('machine); }
                            state = ST_ECLASS_RM103_LOOP;
                            continue 'machine;
                        }
                    }

                    _ => {
                        state = ST_MAINLOOP4;
                        continue 'machine;
                    }
                }
            }

            // ---- CLASS resume points ----
            x if x == RM200 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                let lmin = (*F).fields.class_repeat.min;
                (*F).fields.class_repeat.min = lmin + 1;
                if lmin >= (*F).fields.class_repeat.max { RRETURN!('machine, MATCH_NOMATCH); }
                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                let (c0, cn) = GETCHARINC(Feptr!());
                fc = c0;
                Feptr!() = Feptr!().add(cn);
                let lbyte_map = (*F).fields.class_repeat.byte_map_address;
                if fc > 255 {
                    if Fop!() == OP_CLASS { RRETURN!('machine, MATCH_NOMATCH); }
                } else if (*lbyte_map.add((fc / 8) as usize) & (1u8 << (fc & 7))) == 0 {
                    RRETURN!('machine, MATCH_NOMATCH);
                }
                RMATCH!('machine, Fecode!(), RM200);
            }
            x if x == RM23 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                let lmin = (*F).fields.class_repeat.min;
                (*F).fields.class_repeat.min = lmin + 1;
                if lmin >= (*F).fields.class_repeat.max { RRETURN!('machine, MATCH_NOMATCH); }
                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                fc = *Feptr!() as u32;
                Feptr!() = Feptr!().add(1);
                let lbyte_map = (*F).fields.class_repeat.byte_map_address;
                if (*lbyte_map.add((fc / 8) as usize) & (1u8 << (fc & 7))) == 0 {
                    RRETURN!('machine, MATCH_NOMATCH);
                }
                RMATCH!('machine, Fecode!(), RM23);
            }
            ST_CLASS_RM201_LOOP => {
                RMATCH!('machine, Fecode!(), RM201);
            }
            x if x == RM201 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                let sp = (*F).fields.class_repeat.start_eptr;
                let old = Feptr!();
                Feptr!() = Feptr!().sub(1);
                if old <= sp { RRETURN!('machine, MATCH_NOMATCH); }
                Feptr!() = BACKCHAR(Feptr!());
                state = ST_CLASS_RM201_LOOP;
                continue 'machine;
            }
            ST_CLASS_RM24_LOOP => {
                if Feptr!() >= (*F).fields.class_repeat.start_eptr {
                    RMATCH!('machine, Fecode!(), RM24);
                }
                RRETURN!('machine, MATCH_NOMATCH);
            }
            x if x == RM24 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                Feptr!() = Feptr!().sub(1);
                state = ST_CLASS_RM24_LOOP;
                continue 'machine;
            }
            ST_XCLASS_RM101_LOOP => {
                RMATCH!('machine, Fecode!(), RM101);
            }
            x if x == RM101 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                let sp = (*F).fields.xclass_repeat.start_eptr;
                let old = Feptr!();
                Feptr!() = Feptr!().sub(1);
                if old <= sp { RRETURN!('machine, MATCH_NOMATCH); }
                if utf != 0 { Feptr!() = BACKCHAR(Feptr!()); }
                state = ST_XCLASS_RM101_LOOP;
                continue 'machine;
            }
            x if x == RM100 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                let lmin = (*F).fields.xclass_repeat.min;
                (*F).fields.xclass_repeat.min = lmin + 1;
                if lmin >= (*F).fields.xclass_repeat.max { RRETURN!('machine, MATCH_NOMATCH); }
                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                let c0 = *Feptr!() as u32;
                if utf != 0 && c0 >= 0xc0 {
                    let (v, n) = GETCHARINC(Feptr!());
                    fc = v;
                    Feptr!() = Feptr!().add(n);
                } else {
                    fc = c0;
                    Feptr!() = Feptr!().add(1);
                }
                if _pcre2_xclass_8(fc, (*F).fields.xclass_repeat.xclass_data, (*mb).start_code, utf) == 0 {
                    RRETURN!('machine, MATCH_NOMATCH);
                }
                RMATCH!('machine, Fecode!(), RM100);
            }
            ST_ECLASS_RM103_LOOP => {
                RMATCH!('machine, Fecode!(), RM103);
            }
            x if x == RM103 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                let sp = (*F).fields.eclass_repeat.start_eptr;
                let old = Feptr!();
                Feptr!() = Feptr!().sub(1);
                if old <= sp { RRETURN!('machine, MATCH_NOMATCH); }
                if utf != 0 { Feptr!() = BACKCHAR(Feptr!()); }
                state = ST_ECLASS_RM103_LOOP;
                continue 'machine;
            }
            x if x == RM102 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                let lmin = (*F).fields.eclass_repeat.min;
                (*F).fields.eclass_repeat.min = lmin + 1;
                if lmin >= (*F).fields.eclass_repeat.max { RRETURN!('machine, MATCH_NOMATCH); }
                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                let c0 = *Feptr!() as u32;
                if utf != 0 && c0 >= 0xc0 {
                    let (v, n) = GETCHARINC(Feptr!());
                    fc = v;
                    Feptr!() = Feptr!().add(n);
                } else {
                    fc = c0;
                    Feptr!() = Feptr!().add(1);
                }
                let ed = (*F).fields.eclass_repeat.eclass_data;
                let el = (*F).fields.eclass_repeat.eclass_len;
                if _pcre2_eclass_8(fc, ed, ed.add(el), (*mb).start_code, utf) == 0 {
                    RRETURN!('machine, MATCH_NOMATCH);
                }
                RMATCH!('machine, Fecode!(), RM102);
            }

            // ===== ST_MAINLOOP4: character types (non-UCP), PROP, EXTUNI, type repeats =====
            ST_MAINLOOP4 => {
                macro_rules! GETCHARINCTEST_fc { () => {{
                    let c0 = *Feptr!() as u32;
                    if utf != 0 && c0 >= 0xc0 {
                        let (v, n) = GETCHARINC(Feptr!());
                        fc = v;
                        Feptr!() = Feptr!().add(n);
                    } else {
                        fc = c0;
                        Feptr!() = Feptr!().add(1);
                    }
                }}; }

                match Fop!() {
                    OP_NOT_DIGIT => {
                        if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                        GETCHARINCTEST_fc!();
                        if fc <= 255 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }
                    OP_DIGIT => {
                        if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                        GETCHARINCTEST_fc!();
                        if fc > 255 || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 {
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }
                    OP_NOT_WHITESPACE => {
                        if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                        GETCHARINCTEST_fc!();
                        if fc <= 255 && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 {
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }
                    OP_WHITESPACE => {
                        if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                        GETCHARINCTEST_fc!();
                        if fc > 255 || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 {
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }
                    OP_NOT_WORDCHAR => {
                        if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                        GETCHARINCTEST_fc!();
                        if fc <= 255 && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 {
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }
                    OP_WORDCHAR => {
                        if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                        GETCHARINCTEST_fc!();
                        if fc > 255 || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }
                    OP_ANYNL => {
                        if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                        GETCHARINCTEST_fc!();
                        match fc {
                            CHAR_CR => {
                                if Feptr!() >= ES!() {
                                    SCHECK_PARTIAL!();
                                } else if *Feptr!() as u32 == CHAR_LF {
                                    Feptr!() = Feptr!().add(1);
                                }
                            }
                            CHAR_LF => {}
                            CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                                    RRETURN!('machine, MATCH_NOMATCH);
                                }
                            }
                            _ => { RRETURN!('machine, MATCH_NOMATCH); }
                        }
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }
                    OP_NOT_HSPACE => {
                        if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                        GETCHARINCTEST_fc!();
                        if is_hspace(fc) { RRETURN!('machine, MATCH_NOMATCH); }
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }
                    OP_HSPACE => {
                        if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                        GETCHARINCTEST_fc!();
                        if !is_hspace(fc) { RRETURN!('machine, MATCH_NOMATCH); }
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }
                    OP_NOT_VSPACE => {
                        if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                        GETCHARINCTEST_fc!();
                        if is_vspace(fc) { RRETURN!('machine, MATCH_NOMATCH); }
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }
                    OP_VSPACE => {
                        if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                        GETCHARINCTEST_fc!();
                        if !is_vspace(fc) { RRETURN!('machine, MATCH_NOMATCH); }
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }

                    OP_PROP | OP_NOTPROP => {
                        if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                        GETCHARINCTEST_fc!();
                        {
                            let prop = GET_UCD(fc);
                            let notmatch = Fop!() == OP_NOTPROP;
                            match *Fecode!().add(1) as u32 {
                                PT_LAMP => {
                                    let chartype = prop.chartype as u32;
                                    if ((chartype == ucp_Lu || chartype == ucp_Ll || chartype == ucp_Lt)
                                        == notmatch)
                                    {
                                        RRETURN!('machine, MATCH_NOMATCH);
                                    }
                                }
                                PT_GC => {
                                    if ((*Fecode!().add(2) as u32
                                        == _pcre2_ucp_gentype_8[prop.chartype as usize])
                                        == notmatch)
                                    {
                                        RRETURN!('machine, MATCH_NOMATCH);
                                    }
                                }
                                PT_PC => {
                                    if ((*Fecode!().add(2) as u32 == prop.chartype as u32) == notmatch) {
                                        RRETURN!('machine, MATCH_NOMATCH);
                                    }
                                }
                                PT_SC => {
                                    if ((*Fecode!().add(2) as u32 == prop.script as u32) == notmatch) {
                                        RRETURN!('machine, MATCH_NOMATCH);
                                    }
                                }
                                PT_SCX => {
                                    let sx = UCD_SCRIPTX_PROP(prop) as usize;
                                    let ok = *Fecode!().add(2) as u32 == prop.script as u32
                                        || MAPBIT(&_pcre2_ucd_script_sets_8[sx..], *Fecode!().add(2) as u32) != 0;
                                    if ok == notmatch { RRETURN!('machine, MATCH_NOMATCH); }
                                }
                                PT_ALNUM => {
                                    let chartype = prop.chartype as usize;
                                    if ((_pcre2_ucp_gentype_8[chartype] == ucp_L
                                        || _pcre2_ucp_gentype_8[chartype] == ucp_N)
                                        == notmatch)
                                    {
                                        RRETURN!('machine, MATCH_NOMATCH);
                                    }
                                }
                                PT_SPACE | PT_PXSPACE => {
                                    if is_hspace(fc) || is_vspace(fc) {
                                        if notmatch { RRETURN!('machine, MATCH_NOMATCH); }
                                    } else if ((_pcre2_ucp_gentype_8[prop.chartype as usize] == ucp_Z)
                                        == notmatch)
                                    {
                                        RRETURN!('machine, MATCH_NOMATCH);
                                    }
                                }
                                PT_WORD => {
                                    let chartype = prop.chartype as u32;
                                    if ((_pcre2_ucp_gentype_8[chartype as usize] == ucp_L
                                        || _pcre2_ucp_gentype_8[chartype as usize] == ucp_N
                                        || chartype == ucp_Mn
                                        || chartype == ucp_Pc)
                                        == notmatch)
                                    {
                                        RRETURN!('machine, MATCH_NOMATCH);
                                    }
                                }
                                PT_CLIST => {
                                    let mut cp = *Fecode!().add(2) as usize;
                                    loop {
                                        let v = _pcre2_ucd_caseless_sets_8[cp];
                                        if fc < v {
                                            if notmatch { break; } else { RRETURN!('machine, MATCH_NOMATCH); }
                                        }
                                        cp += 1;
                                        if fc == v {
                                            if notmatch { RRETURN!('machine, MATCH_NOMATCH); } else { break; }
                                        }
                                    }
                                }
                                PT_UCNC => {
                                    if ((fc == CHAR_DOLLAR_SIGN
                                        || fc == CHAR_COMMERCIAL_AT
                                        || fc == CHAR_GRAVE_ACCENT
                                        || (fc >= 0xa0 && fc <= 0xd7ff)
                                        || fc >= 0xe000)
                                        == notmatch)
                                    {
                                        RRETURN!('machine, MATCH_NOMATCH);
                                    }
                                }
                                PT_BIDICL => {
                                    if ((UCD_BIDICLASS_PROP(prop) == *Fecode!().add(2) as u32)
                                        == notmatch)
                                    {
                                        RRETURN!('machine, MATCH_NOMATCH);
                                    }
                                }
                                PT_BOOL => {
                                    let bp = UCD_BPROPS_PROP(prop) as usize;
                                    let ok = MAPBIT(&_pcre2_ucd_boolprop_sets_8[bp..], *Fecode!().add(2) as u32) != 0;
                                    if ok == notmatch { RRETURN!('machine, MATCH_NOMATCH); }
                                }
                                _ => {
                                    return PCRE2_ERROR_INTERNAL;
                                }
                            }
                            Fecode!() = Fecode!().add(3);
                        }
                        NEXT_OP!('machine);
                    }

                    OP_EXTUNI => {
                        if Feptr!() >= ES!() {
                            SCHECK_PARTIAL!();
                            RRETURN!('machine, MATCH_NOMATCH);
                        } else {
                            GETCHARINCTEST_fc!();
                            Feptr!() = _pcre2_extuni_8(
                                fc,
                                Feptr!(),
                                (*mb).start_subject,
                                (*mb).end_subject,
                                utf,
                                ptr::null_mut(),
                            );
                        }
                        CHECK_PARTIAL!();
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }

                    _ => {
                        state = ST_MAINLOOP5;
                        continue 'machine;
                    }
                }
            }

            // ===== ST_MAINLOOP5: type repeats entry =====
            ST_MAINLOOP5 => {
                match Fop!() {
                    OP_TYPEEXACT => {
                        (*F).fields.type_repeat.min = GET2(Fecode!(), 1);
                        (*F).fields.type_repeat.max = (*F).fields.type_repeat.min;
                        Fecode!() = Fecode!().add(1 + IMM2_SIZE);
                        state = ST_REPEATTYPE;
                        continue 'machine;
                    }
                    OP_TYPEUPTO | OP_TYPEMINUPTO => {
                        (*F).fields.type_repeat.min = 0;
                        (*F).fields.type_repeat.max = GET2(Fecode!(), 1);
                        reptype = if *Fecode!() == OP_TYPEMINUPTO { REPTYPE_MIN } else { REPTYPE_MAX };
                        Fecode!() = Fecode!().add(1 + IMM2_SIZE);
                        state = ST_REPEATTYPE;
                        continue 'machine;
                    }
                    OP_TYPEPOSSTAR => {
                        reptype = REPTYPE_POS;
                        (*F).fields.type_repeat.min = 0;
                        (*F).fields.type_repeat.max = u32::MAX;
                        Fecode!() = Fecode!().add(1);
                        state = ST_REPEATTYPE;
                        continue 'machine;
                    }
                    OP_TYPEPOSPLUS => {
                        reptype = REPTYPE_POS;
                        (*F).fields.type_repeat.min = 1;
                        (*F).fields.type_repeat.max = u32::MAX;
                        Fecode!() = Fecode!().add(1);
                        state = ST_REPEATTYPE;
                        continue 'machine;
                    }
                    OP_TYPEPOSQUERY => {
                        reptype = REPTYPE_POS;
                        (*F).fields.type_repeat.min = 0;
                        (*F).fields.type_repeat.max = 1;
                        Fecode!() = Fecode!().add(1);
                        state = ST_REPEATTYPE;
                        continue 'machine;
                    }
                    OP_TYPEPOSUPTO => {
                        reptype = REPTYPE_POS;
                        (*F).fields.type_repeat.min = 0;
                        (*F).fields.type_repeat.max = GET2(Fecode!(), 1);
                        Fecode!() = Fecode!().add(1 + IMM2_SIZE);
                        state = ST_REPEATTYPE;
                        continue 'machine;
                    }
                    OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEQUERY
                    | OP_TYPEMINQUERY => {
                        fc = *Fecode!() as u32 - OP_TYPESTAR as u32;
                        Fecode!() = Fecode!().add(1);
                        (*F).fields.type_repeat.min = REP_MIN[fc as usize];
                        (*F).fields.type_repeat.max = REP_MAX[fc as usize];
                        reptype = REP_TYP[fc as usize];
                        state = ST_REPEATTYPE;
                        continue 'machine;
                    }
                    _ => {
                        state = ST_MAINLOOP6;
                        continue 'machine;
                    }
                }
            }

            ST_REPEATTYPE => {
                macro_rules! Lstart_eptr { () => { (*F).fields.type_repeat.start_eptr }; }
                macro_rules! Lmin { () => { (*F).fields.type_repeat.min }; }
                macro_rules! Lmax { () => { (*F).fields.type_repeat.max }; }
                macro_rules! Lctype { () => { (*F).fields.type_repeat.ctype }; }
                macro_rules! Lpropvalue { () => { (*F).fields.type_repeat.propvalue }; }
                macro_rules! CHK_END { () => {
                    if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                }; }
                macro_rules! GCI { () => {{
                    let c0 = *Feptr!() as u32;
                    if utf != 0 && c0 >= 0xc0 {
                        let (v, n) = GETCHARINC(Feptr!());
                        fc = v; Feptr!() = Feptr!().add(n);
                    } else { fc = c0; Feptr!() = Feptr!().add(1); }
                }}; }

                Lctype!() = *Fecode!() as u32;
                Fecode!() = Fecode!().add(1);

                if Lctype!() == OP_PROP as u32 || Lctype!() == OP_NOTPROP as u32 {
                    proptype = *Fecode!() as i32;
                    Fecode!() = Fecode!().add(1);
                    Lpropvalue!() = *Fecode!() as u32;
                    Fecode!() = Fecode!().add(1);
                } else {
                    proptype = -1;
                }

                // ---- Ensure minimum matches (Lmin > 0) ----
                if Lmin!() > 0 {
                    if proptype >= 0 {
                        let notmatch = Lctype!() == OP_NOTPROP as u32;
                        match proptype as u32 {
                            PT_LAMP => {
                                i = 1;
                                while i <= Lmin!() {
                                    CHK_END!(); GCI!();
                                    let ct = UCD_CHARTYPE(fc);
                                    if ((ct == ucp_Lu || ct == ucp_Ll || ct == ucp_Lt) == notmatch) {
                                        RRETURN!('machine, MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            PT_GC => {
                                i = 1;
                                while i <= Lmin!() {
                                    CHK_END!(); GCI!();
                                    if ((UCD_CATEGORY(fc) == Lpropvalue!()) == notmatch) {
                                        RRETURN!('machine, MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            PT_PC => {
                                i = 1;
                                while i <= Lmin!() {
                                    CHK_END!(); GCI!();
                                    if ((UCD_CHARTYPE(fc) == Lpropvalue!()) == notmatch) {
                                        RRETURN!('machine, MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            PT_SC => {
                                i = 1;
                                while i <= Lmin!() {
                                    CHK_END!(); GCI!();
                                    if ((UCD_SCRIPT(fc) == Lpropvalue!()) == notmatch) {
                                        RRETURN!('machine, MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            PT_SCX => {
                                i = 1;
                                while i <= Lmin!() {
                                    CHK_END!(); GCI!();
                                    let prop = GET_UCD(fc);
                                    let sx = UCD_SCRIPTX_PROP(prop) as usize;
                                    let ok = prop.script as u32 == Lpropvalue!()
                                        || MAPBIT(&_pcre2_ucd_script_sets_8[sx..], Lpropvalue!()) != 0;
                                    if ok == notmatch { RRETURN!('machine, MATCH_NOMATCH); }
                                    i += 1;
                                }
                            }
                            PT_ALNUM => {
                                i = 1;
                                while i <= Lmin!() {
                                    CHK_END!(); GCI!();
                                    let category = UCD_CATEGORY(fc);
                                    if ((category == ucp_L || category == ucp_N) == notmatch) {
                                        RRETURN!('machine, MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            PT_SPACE | PT_PXSPACE => {
                                i = 1;
                                while i <= Lmin!() {
                                    CHK_END!(); GCI!();
                                    if is_hspace(fc) || is_vspace(fc) {
                                        if notmatch { RRETURN!('machine, MATCH_NOMATCH); }
                                    } else if ((UCD_CATEGORY(fc) == ucp_Z) == notmatch) {
                                        RRETURN!('machine, MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            PT_WORD => {
                                i = 1;
                                while i <= Lmin!() {
                                    CHK_END!(); GCI!();
                                    let ct = UCD_CHARTYPE(fc);
                                    let category = _pcre2_ucp_gentype_8[ct as usize];
                                    if ((category == ucp_L || category == ucp_N
                                        || ct == ucp_Mn || ct == ucp_Pc) == notmatch) {
                                        RRETURN!('machine, MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            PT_CLIST => {
                                i = 1;
                                while i <= Lmin!() {
                                    CHK_END!(); GCI!();
                                    let mut cp = Lpropvalue!() as usize;
                                    loop {
                                        let v = _pcre2_ucd_caseless_sets_8[cp];
                                        if fc < v {
                                            if notmatch { break; } RRETURN!('machine, MATCH_NOMATCH);
                                        }
                                        cp += 1;
                                        if fc == v {
                                            if notmatch { RRETURN!('machine, MATCH_NOMATCH); } break;
                                        }
                                    }
                                    i += 1;
                                }
                            }
                            PT_UCNC => {
                                i = 1;
                                while i <= Lmin!() {
                                    CHK_END!(); GCI!();
                                    if ((fc == CHAR_DOLLAR_SIGN || fc == CHAR_COMMERCIAL_AT
                                        || fc == CHAR_GRAVE_ACCENT || (fc >= 0xa0 && fc <= 0xd7ff)
                                        || fc >= 0xe000) == notmatch) {
                                        RRETURN!('machine, MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            PT_BIDICL => {
                                i = 1;
                                while i <= Lmin!() {
                                    CHK_END!(); GCI!();
                                    if ((UCD_BIDICLASS(fc) == Lpropvalue!()) == notmatch) {
                                        RRETURN!('machine, MATCH_NOMATCH);
                                    }
                                    i += 1;
                                }
                            }
                            PT_BOOL => {
                                i = 1;
                                while i <= Lmin!() {
                                    CHK_END!(); GCI!();
                                    let prop = GET_UCD(fc);
                                    let bp = UCD_BPROPS_PROP(prop) as usize;
                                    let ok = MAPBIT(&_pcre2_ucd_boolprop_sets_8[bp..], Lpropvalue!()) != 0;
                                    if ok == notmatch { RRETURN!('machine, MATCH_NOMATCH); }
                                    i += 1;
                                }
                            }
                            _ => { return PCRE2_ERROR_INTERNAL; }
                        }
                    } else if Lctype!() == OP_EXTUNI as u32 {
                        i = 1;
                        while i <= Lmin!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                            else {
                                GCI!();
                                Feptr!() = _pcre2_extuni_8(fc, Feptr!(), (*mb).start_subject,
                                    (*mb).end_subject, utf, ptr::null_mut());
                            }
                            CHECK_PARTIAL!();
                            i += 1;
                        }
                    } else if utf != 0 {
                        // UTF, non-property.
                        i = 1;
                        while i <= Lmin!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                            match Lctype!() {
                                x if x == OP_ANY as u32 => {
                                    if is_newline(Feptr!(), mb, utf) { RRETURN!('machine, MATCH_NOMATCH); }
                                    if (*mb).partial != 0 && Feptr!().add(1) >= ES!()
                                        && (*mb).nltype == NLTYPE_FIXED && (*mb).nllen == 2
                                        && *Feptr!() == (*mb).nl[0] {
                                        (*mb).hitend = TRUE;
                                        if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL; }
                                    }
                                    Feptr!() = Feptr!().add(1);
                                    while Feptr!() < ES!() && (*Feptr!() & 0xc0) == 0x80 { Feptr!() = Feptr!().add(1); }
                                }
                                x if x == OP_ALLANY as u32 => {
                                    Feptr!() = Feptr!().add(1);
                                    while Feptr!() < ES!() && (*Feptr!() & 0xc0) == 0x80 { Feptr!() = Feptr!().add(1); }
                                }
                                x if x == OP_ANYBYTE as u32 => {
                                    // handled below as bulk; but faithful per-iter:
                                    if Feptr!() > ES!().sub(Lmin!() as usize) { RRETURN!('machine, MATCH_NOMATCH); }
                                    Feptr!() = Feptr!().add(Lmin!() as usize);
                                    break;
                                }
                                x if x == OP_ANYNL as u32 => {
                                    let (v, n) = GETCHARINC(Feptr!()); fc = v; Feptr!() = Feptr!().add(n);
                                    match fc {
                                        CHAR_CR => { if Feptr!() < ES!() && *Feptr!() as u32 == CHAR_LF { Feptr!() = Feptr!().add(1); } }
                                        CHAR_LF => {}
                                        CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                            if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF { RRETURN!('machine, MATCH_NOMATCH); }
                                        }
                                        _ => { RRETURN!('machine, MATCH_NOMATCH); }
                                    }
                                }
                                x if x == OP_NOT_HSPACE as u32 => {
                                    let (v, n) = GETCHARINC(Feptr!()); fc = v; Feptr!() = Feptr!().add(n);
                                    if is_hspace(fc) { RRETURN!('machine, MATCH_NOMATCH); }
                                }
                                x if x == OP_HSPACE as u32 => {
                                    let (v, n) = GETCHARINC(Feptr!()); fc = v; Feptr!() = Feptr!().add(n);
                                    if !is_hspace(fc) { RRETURN!('machine, MATCH_NOMATCH); }
                                }
                                x if x == OP_NOT_VSPACE as u32 => {
                                    let (v, n) = GETCHARINC(Feptr!()); fc = v; Feptr!() = Feptr!().add(n);
                                    if is_vspace(fc) { RRETURN!('machine, MATCH_NOMATCH); }
                                }
                                x if x == OP_VSPACE as u32 => {
                                    let (v, n) = GETCHARINC(Feptr!()); fc = v; Feptr!() = Feptr!().add(n);
                                    if !is_vspace(fc) { RRETURN!('machine, MATCH_NOMATCH); }
                                }
                                x if x == OP_NOT_DIGIT as u32 => {
                                    let (v, n) = GETCHARINC(Feptr!()); fc = v; Feptr!() = Feptr!().add(n);
                                    if fc < 128 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 { RRETURN!('machine, MATCH_NOMATCH); }
                                }
                                x if x == OP_DIGIT as u32 => {
                                    let cc = *Feptr!() as u32;
                                    if cc >= 128 || (*(*mb).ctypes.add(cc as usize) & ctype_digit) == 0 { RRETURN!('machine, MATCH_NOMATCH); }
                                    Feptr!() = Feptr!().add(1);
                                }
                                x if x == OP_NOT_WHITESPACE as u32 => {
                                    let cc = *Feptr!() as u32;
                                    if cc < 128 && (*(*mb).ctypes.add(cc as usize) & ctype_space) != 0 { RRETURN!('machine, MATCH_NOMATCH); }
                                    Feptr!() = Feptr!().add(1);
                                    while Feptr!() < ES!() && (*Feptr!() & 0xc0) == 0x80 { Feptr!() = Feptr!().add(1); }
                                }
                                x if x == OP_WHITESPACE as u32 => {
                                    let cc = *Feptr!() as u32;
                                    if cc >= 128 || (*(*mb).ctypes.add(cc as usize) & ctype_space) == 0 { RRETURN!('machine, MATCH_NOMATCH); }
                                    Feptr!() = Feptr!().add(1);
                                }
                                x if x == OP_NOT_WORDCHAR as u32 => {
                                    let cc = *Feptr!() as u32;
                                    if cc < 128 && (*(*mb).ctypes.add(cc as usize) & ctype_word) != 0 { RRETURN!('machine, MATCH_NOMATCH); }
                                    Feptr!() = Feptr!().add(1);
                                    while Feptr!() < ES!() && (*Feptr!() & 0xc0) == 0x80 { Feptr!() = Feptr!().add(1); }
                                }
                                x if x == OP_WORDCHAR as u32 => {
                                    let cc = *Feptr!() as u32;
                                    if cc >= 128 || (*(*mb).ctypes.add(cc as usize) & ctype_word) == 0 { RRETURN!('machine, MATCH_NOMATCH); }
                                    Feptr!() = Feptr!().add(1);
                                }
                                _ => { return PCRE2_ERROR_INTERNAL; }
                            }
                            i += 1;
                        }
                    } else {
                        // Non-UTF, non-property.
                        match Lctype!() {
                            x if x == OP_ANY as u32 => {
                                i = 1;
                                while i <= Lmin!() {
                                    if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                                    if is_newline(Feptr!(), mb, utf) { RRETURN!('machine, MATCH_NOMATCH); }
                                    if (*mb).partial != 0 && Feptr!().add(1) >= ES!()
                                        && (*mb).nltype == NLTYPE_FIXED && (*mb).nllen == 2
                                        && *Feptr!() == (*mb).nl[0] {
                                        (*mb).hitend = TRUE;
                                        if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL; }
                                    }
                                    Feptr!() = Feptr!().add(1);
                                    i += 1;
                                }
                            }
                            x if x == OP_ALLANY as u32 => {
                                if Feptr!() > ES!().sub(Lmin!() as usize) {
                                    SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH);
                                }
                                Feptr!() = Feptr!().add(Lmin!() as usize);
                            }
                            x if x == OP_ANYNL as u32 => {
                                i = 1;
                                while i <= Lmin!() {
                                    if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                                    let cc = *Feptr!() as u32; Feptr!() = Feptr!().add(1);
                                    match cc {
                                        CHAR_CR => { if Feptr!() < ES!() && *Feptr!() as u32 == CHAR_LF { Feptr!() = Feptr!().add(1); } }
                                        CHAR_LF => {}
                                        CHAR_VT | CHAR_FF | CHAR_NEL => {
                                            if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF { RRETURN!('machine, MATCH_NOMATCH); }
                                        }
                                        _ => { RRETURN!('machine, MATCH_NOMATCH); }
                                    }
                                    i += 1;
                                }
                            }
                            x if x == OP_NOT_HSPACE as u32 => {
                                i = 1;
                                while i <= Lmin!() {
                                    if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                                    let cc = *Feptr!() as u32; Feptr!() = Feptr!().add(1);
                                    if is_hspace_byte(cc) { RRETURN!('machine, MATCH_NOMATCH); }
                                    i += 1;
                                }
                            }
                            x if x == OP_HSPACE as u32 => {
                                i = 1;
                                while i <= Lmin!() {
                                    if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                                    let cc = *Feptr!() as u32; Feptr!() = Feptr!().add(1);
                                    if !is_hspace_byte(cc) { RRETURN!('machine, MATCH_NOMATCH); }
                                    i += 1;
                                }
                            }
                            x if x == OP_NOT_VSPACE as u32 => {
                                i = 1;
                                while i <= Lmin!() {
                                    if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                                    let cc = *Feptr!() as u32; Feptr!() = Feptr!().add(1);
                                    if is_vspace_byte(cc) { RRETURN!('machine, MATCH_NOMATCH); }
                                    i += 1;
                                }
                            }
                            x if x == OP_VSPACE as u32 => {
                                i = 1;
                                while i <= Lmin!() {
                                    if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                                    let cc = *Feptr!() as u32; Feptr!() = Feptr!().add(1);
                                    if !is_vspace_byte(cc) { RRETURN!('machine, MATCH_NOMATCH); }
                                    i += 1;
                                }
                            }
                            x if x == OP_NOT_DIGIT as u32 => {
                                i = 1;
                                while i <= Lmin!() {
                                    if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                                    if (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_digit) != 0 { RRETURN!('machine, MATCH_NOMATCH); }
                                    Feptr!() = Feptr!().add(1);
                                    i += 1;
                                }
                            }
                            x if x == OP_DIGIT as u32 => {
                                i = 1;
                                while i <= Lmin!() {
                                    if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                                    if (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_digit) == 0 { RRETURN!('machine, MATCH_NOMATCH); }
                                    Feptr!() = Feptr!().add(1);
                                    i += 1;
                                }
                            }
                            x if x == OP_NOT_WHITESPACE as u32 => {
                                i = 1;
                                while i <= Lmin!() {
                                    if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                                    if (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_space) != 0 { RRETURN!('machine, MATCH_NOMATCH); }
                                    Feptr!() = Feptr!().add(1);
                                    i += 1;
                                }
                            }
                            x if x == OP_WHITESPACE as u32 => {
                                i = 1;
                                while i <= Lmin!() {
                                    if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                                    if (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_space) == 0 { RRETURN!('machine, MATCH_NOMATCH); }
                                    Feptr!() = Feptr!().add(1);
                                    i += 1;
                                }
                            }
                            x if x == OP_NOT_WORDCHAR as u32 => {
                                i = 1;
                                while i <= Lmin!() {
                                    if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                                    if (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_word) != 0 { RRETURN!('machine, MATCH_NOMATCH); }
                                    Feptr!() = Feptr!().add(1);
                                    i += 1;
                                }
                            }
                            x if x == OP_WORDCHAR as u32 => {
                                i = 1;
                                while i <= Lmin!() {
                                    if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                                    if (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_word) == 0 { RRETURN!('machine, MATCH_NOMATCH); }
                                    Feptr!() = Feptr!().add(1);
                                    i += 1;
                                }
                            }
                            _ => { return PCRE2_ERROR_INTERNAL; }
                        }
                    }
                }

                if Lmin!() == Lmax!() { NEXT_OP!('machine); }

                if reptype == REPTYPE_MIN {
                    state = ST_TYPE_MIN_DISPATCH;
                    continue 'machine;
                } else {
                    Lstart_eptr!() = Feptr!();
                    state = ST_TYPE_MAX_DISPATCH;
                    continue 'machine;
                }
            }

            // ===== Type-repeat MINIMIZE dispatch =====
            ST_TYPE_MIN_DISPATCH => {
                if proptype >= 0 {
                    match proptype as u32 {
                        PT_LAMP => RMATCH!('machine, Fecode!(), RM208),
                        PT_GC => RMATCH!('machine, Fecode!(), RM209),
                        PT_PC => RMATCH!('machine, Fecode!(), RM210),
                        PT_SC => RMATCH!('machine, Fecode!(), RM211),
                        PT_SCX => RMATCH!('machine, Fecode!(), RM224),
                        PT_ALNUM => RMATCH!('machine, Fecode!(), RM212),
                        PT_SPACE | PT_PXSPACE => RMATCH!('machine, Fecode!(), RM213),
                        PT_WORD => RMATCH!('machine, Fecode!(), RM214),
                        PT_CLIST => RMATCH!('machine, Fecode!(), RM215),
                        PT_UCNC => RMATCH!('machine, Fecode!(), RM216),
                        PT_BIDICL => RMATCH!('machine, Fecode!(), RM223),
                        PT_BOOL => RMATCH!('machine, Fecode!(), RM222),
                        _ => { return PCRE2_ERROR_INTERNAL; }
                    }
                } else if (*F).fields.type_repeat.ctype == OP_EXTUNI as u32 {
                    RMATCH!('machine, Fecode!(), RM217);
                } else if utf != 0 {
                    RMATCH!('machine, Fecode!(), RM218);
                } else {
                    RMATCH!('machine, Fecode!(), RM33);
                }
            }

            // ===== Type-repeat MINIMIZE resume points =====
            x if x >= RM208 as i32 && x <= RM217 as i32
                || x == RM222 as i32 || x == RM223 as i32 || x == RM224 as i32
                || x == RM218 as i32 || x == RM33 as i32 => {
                macro_rules! Lmin { () => { (*F).fields.type_repeat.min }; }
                macro_rules! Lmax { () => { (*F).fields.type_repeat.max }; }
                macro_rules! Lctype { () => { (*F).fields.type_repeat.ctype }; }
                macro_rules! Lpropvalue { () => { (*F).fields.type_repeat.propvalue }; }
                macro_rules! GCI { () => {{
                    let c0 = *Feptr!() as u32;
                    if utf != 0 && c0 >= 0xc0 {
                        let (v, n) = GETCHARINC(Feptr!());
                        fc = v; Feptr!() = Feptr!().add(n);
                    } else { fc = c0; Feptr!() = Feptr!().add(1); }
                }}; }
                let notprop = Lctype!() == OP_NOTPROP as u32;

                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                let lmin = Lmin!();
                Lmin!() = lmin + 1;
                if lmin >= Lmax!() { RRETURN!('machine, MATCH_NOMATCH); }

                if x == RM217 as i32 {
                    // EXTUNI
                    if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                    else {
                        GCI!();
                        Feptr!() = _pcre2_extuni_8(fc, Feptr!(), (*mb).start_subject,
                            (*mb).end_subject, utf, ptr::null_mut());
                    }
                    CHECK_PARTIAL!();
                    RMATCH!('machine, Fecode!(), RM217);
                } else if x == RM218 as i32 {
                    // UTF non-property.
                    if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                    if Lctype!() == OP_ANY as u32 && is_newline(Feptr!(), mb, utf) { RRETURN!('machine, MATCH_NOMATCH); }
                    let (v, n) = GETCHARINC(Feptr!()); fc = v; Feptr!() = Feptr!().add(n);
                    match Lctype!() {
                        c if c == OP_ANY as u32 => {
                            if (*mb).partial != 0 && Feptr!() >= ES!()
                                && (*mb).nltype == NLTYPE_FIXED && (*mb).nllen == 2
                                && fc == (*mb).nl[0] as u32 {
                                (*mb).hitend = TRUE;
                                if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL; }
                            }
                        }
                        c if c == OP_ALLANY as u32 || c == OP_ANYBYTE as u32 => {}
                        c if c == OP_ANYNL as u32 => {
                            match fc {
                                CHAR_CR => { if Feptr!() < ES!() && *Feptr!() as u32 == CHAR_LF { Feptr!() = Feptr!().add(1); } }
                                CHAR_LF => {}
                                CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                    if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF { RRETURN!('machine, MATCH_NOMATCH); }
                                }
                                _ => { RRETURN!('machine, MATCH_NOMATCH); }
                            }
                        }
                        c if c == OP_NOT_HSPACE as u32 => { if is_hspace(fc) { RRETURN!('machine, MATCH_NOMATCH); } }
                        c if c == OP_HSPACE as u32 => { if !is_hspace(fc) { RRETURN!('machine, MATCH_NOMATCH); } }
                        c if c == OP_NOT_VSPACE as u32 => { if is_vspace(fc) { RRETURN!('machine, MATCH_NOMATCH); } }
                        c if c == OP_VSPACE as u32 => { if !is_vspace(fc) { RRETURN!('machine, MATCH_NOMATCH); } }
                        c if c == OP_NOT_DIGIT as u32 => { if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 { RRETURN!('machine, MATCH_NOMATCH); } }
                        c if c == OP_DIGIT as u32 => { if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 { RRETURN!('machine, MATCH_NOMATCH); } }
                        c if c == OP_NOT_WHITESPACE as u32 => { if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 { RRETURN!('machine, MATCH_NOMATCH); } }
                        c if c == OP_WHITESPACE as u32 => { if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 { RRETURN!('machine, MATCH_NOMATCH); } }
                        c if c == OP_NOT_WORDCHAR as u32 => { if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 { RRETURN!('machine, MATCH_NOMATCH); } }
                        c if c == OP_WORDCHAR as u32 => { if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 { RRETURN!('machine, MATCH_NOMATCH); } }
                        _ => { return PCRE2_ERROR_INTERNAL; }
                    }
                    RMATCH!('machine, Fecode!(), RM218);
                } else if x == RM33 as i32 {
                    // Non-UTF non-property.
                    if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                    if Lctype!() == OP_ANY as u32 && is_newline(Feptr!(), mb, utf) { RRETURN!('machine, MATCH_NOMATCH); }
                    fc = *Feptr!() as u32; Feptr!() = Feptr!().add(1);
                    match Lctype!() {
                        c if c == OP_ANY as u32 => {
                            if (*mb).partial != 0 && Feptr!() >= ES!()
                                && (*mb).nltype == NLTYPE_FIXED && (*mb).nllen == 2
                                && fc == (*mb).nl[0] as u32 {
                                (*mb).hitend = TRUE;
                                if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL; }
                            }
                        }
                        c if c == OP_ALLANY as u32 || c == OP_ANYBYTE as u32 => {}
                        c if c == OP_ANYNL as u32 => {
                            match fc {
                                CHAR_CR => { if Feptr!() < ES!() && *Feptr!() as u32 == CHAR_LF { Feptr!() = Feptr!().add(1); } }
                                CHAR_LF => {}
                                CHAR_VT | CHAR_FF | CHAR_NEL => {
                                    if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF { RRETURN!('machine, MATCH_NOMATCH); }
                                }
                                _ => { RRETURN!('machine, MATCH_NOMATCH); }
                            }
                        }
                        c if c == OP_NOT_HSPACE as u32 => { if is_hspace_byte(fc) { RRETURN!('machine, MATCH_NOMATCH); } }
                        c if c == OP_HSPACE as u32 => { if !is_hspace_byte(fc) { RRETURN!('machine, MATCH_NOMATCH); } }
                        c if c == OP_NOT_VSPACE as u32 => { if is_vspace_byte(fc) { RRETURN!('machine, MATCH_NOMATCH); } }
                        c if c == OP_VSPACE as u32 => { if !is_vspace_byte(fc) { RRETURN!('machine, MATCH_NOMATCH); } }
                        c if c == OP_NOT_DIGIT as u32 => { if (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 { RRETURN!('machine, MATCH_NOMATCH); } }
                        c if c == OP_DIGIT as u32 => { if (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 { RRETURN!('machine, MATCH_NOMATCH); } }
                        c if c == OP_NOT_WHITESPACE as u32 => { if (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 { RRETURN!('machine, MATCH_NOMATCH); } }
                        c if c == OP_WHITESPACE as u32 => { if (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 { RRETURN!('machine, MATCH_NOMATCH); } }
                        c if c == OP_NOT_WORDCHAR as u32 => { if (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 { RRETURN!('machine, MATCH_NOMATCH); } }
                        c if c == OP_WORDCHAR as u32 => { if (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 { RRETURN!('machine, MATCH_NOMATCH); } }
                        _ => { return PCRE2_ERROR_INTERNAL; }
                    }
                    RMATCH!('machine, Fecode!(), RM33);
                } else {
                    // Property min-loops.
                    if Feptr!() >= ES!() { SCHECK_PARTIAL!(); RRETURN!('machine, MATCH_NOMATCH); }
                    GCI!();
                    let resume = x as u8;
                    match resume {
                        RM208 => {
                            let ct = UCD_CHARTYPE(fc);
                            if ((ct == ucp_Lu || ct == ucp_Ll || ct == ucp_Lt) == notprop) { RRETURN!('machine, MATCH_NOMATCH); }
                        }
                        RM209 => { if ((UCD_CATEGORY(fc) == Lpropvalue!()) == notprop) { RRETURN!('machine, MATCH_NOMATCH); } }
                        RM210 => { if ((UCD_CHARTYPE(fc) == Lpropvalue!()) == notprop) { RRETURN!('machine, MATCH_NOMATCH); } }
                        RM211 => { if ((UCD_SCRIPT(fc) == Lpropvalue!()) == notprop) { RRETURN!('machine, MATCH_NOMATCH); } }
                        RM224 => {
                            let prop = GET_UCD(fc);
                            let sx = UCD_SCRIPTX_PROP(prop) as usize;
                            let ok = prop.script as u32 == Lpropvalue!()
                                || MAPBIT(&_pcre2_ucd_script_sets_8[sx..], Lpropvalue!()) != 0;
                            if ok == notprop { RRETURN!('machine, MATCH_NOMATCH); }
                        }
                        RM212 => {
                            let category = UCD_CATEGORY(fc);
                            if ((category == ucp_L || category == ucp_N) == notprop) { RRETURN!('machine, MATCH_NOMATCH); }
                        }
                        RM213 => {
                            if is_hspace(fc) || is_vspace(fc) {
                                if notprop { RRETURN!('machine, MATCH_NOMATCH); }
                            } else if ((UCD_CATEGORY(fc) == ucp_Z) == notprop) { RRETURN!('machine, MATCH_NOMATCH); }
                        }
                        RM214 => {
                            let ct = UCD_CHARTYPE(fc);
                            let category = _pcre2_ucp_gentype_8[ct as usize];
                            if ((category == ucp_L || category == ucp_N || ct == ucp_Mn || ct == ucp_Pc) == notprop) {
                                RRETURN!('machine, MATCH_NOMATCH);
                            }
                        }
                        RM215 => {
                            let mut cp = Lpropvalue!() as usize;
                            loop {
                                let v = _pcre2_ucd_caseless_sets_8[cp];
                                if fc < v { if notprop { break; } RRETURN!('machine, MATCH_NOMATCH); }
                                cp += 1;
                                if fc == v { if notprop { RRETURN!('machine, MATCH_NOMATCH); } break; }
                            }
                        }
                        RM216 => {
                            if ((fc == CHAR_DOLLAR_SIGN || fc == CHAR_COMMERCIAL_AT
                                || fc == CHAR_GRAVE_ACCENT || (fc >= 0xa0 && fc <= 0xd7ff)
                                || fc >= 0xe000) == notprop) { RRETURN!('machine, MATCH_NOMATCH); }
                        }
                        RM223 => { if ((UCD_BIDICLASS(fc) == Lpropvalue!()) == notprop) { RRETURN!('machine, MATCH_NOMATCH); } }
                        RM222 => {
                            let prop = GET_UCD(fc);
                            let bp = UCD_BPROPS_PROP(prop) as usize;
                            let ok = MAPBIT(&_pcre2_ucd_boolprop_sets_8[bp..], Lpropvalue!()) != 0;
                            if ok == notprop { RRETURN!('machine, MATCH_NOMATCH); }
                        }
                        _ => { return PCRE2_ERROR_INTERNAL; }
                    }
                    RMATCH!('machine, Fecode!(), resume);
                }
            }

            // ===== Type-repeat MAXIMIZE dispatch: forward scan then backtrack =====
            ST_TYPE_MAX_DISPATCH => {
                macro_rules! Lmin { () => { (*F).fields.type_repeat.min }; }
                macro_rules! Lmax { () => { (*F).fields.type_repeat.max }; }
                macro_rules! Lctype { () => { (*F).fields.type_repeat.ctype }; }
                macro_rules! Lpropvalue { () => { (*F).fields.type_repeat.propvalue }; }
                macro_rules! GCLEN { () => {{
                    let c0 = *Feptr!() as u32;
                    if utf != 0 && c0 >= 0xc0 {
                        let (v, e) = GETCHARLEN(Feptr!());
                        fc = v; (1usize + e as usize)
                    } else { fc = c0; 1usize }
                }}; }

                if proptype >= 0 {
                    let notmatch = Lctype!() == OP_NOTPROP as u32;
                    match proptype as u32 {
                        PT_LAMP => {
                            i = Lmin!();
                            while i < Lmax!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                let len = GCLEN!();
                                let ct = UCD_CHARTYPE(fc);
                                if ((ct == ucp_Lu || ct == ucp_Ll || ct == ucp_Lt) == notmatch) { break; }
                                Feptr!() = Feptr!().add(len);
                                i += 1;
                            }
                        }
                        PT_GC => {
                            i = Lmin!();
                            while i < Lmax!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                let len = GCLEN!();
                                if ((UCD_CATEGORY(fc) == Lpropvalue!()) == notmatch) { break; }
                                Feptr!() = Feptr!().add(len);
                                i += 1;
                            }
                        }
                        PT_PC => {
                            i = Lmin!();
                            while i < Lmax!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                let len = GCLEN!();
                                if ((UCD_CHARTYPE(fc) == Lpropvalue!()) == notmatch) { break; }
                                Feptr!() = Feptr!().add(len);
                                i += 1;
                            }
                        }
                        PT_SC => {
                            i = Lmin!();
                            while i < Lmax!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                let len = GCLEN!();
                                if ((UCD_SCRIPT(fc) == Lpropvalue!()) == notmatch) { break; }
                                Feptr!() = Feptr!().add(len);
                                i += 1;
                            }
                        }
                        PT_SCX => {
                            i = Lmin!();
                            while i < Lmax!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                let len = GCLEN!();
                                let prop = GET_UCD(fc);
                                let sx = UCD_SCRIPTX_PROP(prop) as usize;
                                let ok = prop.script as u32 == Lpropvalue!()
                                    || MAPBIT(&_pcre2_ucd_script_sets_8[sx..], Lpropvalue!()) != 0;
                                if ok == notmatch { break; }
                                Feptr!() = Feptr!().add(len);
                                i += 1;
                            }
                        }
                        PT_ALNUM => {
                            i = Lmin!();
                            while i < Lmax!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                let len = GCLEN!();
                                let category = UCD_CATEGORY(fc);
                                if ((category == ucp_L || category == ucp_N) == notmatch) { break; }
                                Feptr!() = Feptr!().add(len);
                                i += 1;
                            }
                        }
                        PT_SPACE | PT_PXSPACE => {
                            i = Lmin!();
                            'lp99: while i < Lmax!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                let len = GCLEN!();
                                if is_hspace(fc) || is_vspace(fc) {
                                    if notmatch { break 'lp99; }
                                } else if ((UCD_CATEGORY(fc) == ucp_Z) == notmatch) {
                                    break 'lp99;
                                }
                                Feptr!() = Feptr!().add(len);
                                i += 1;
                            }
                        }
                        PT_WORD => {
                            i = Lmin!();
                            while i < Lmax!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                let len = GCLEN!();
                                let ct = UCD_CHARTYPE(fc);
                                let category = _pcre2_ucp_gentype_8[ct as usize];
                                if ((category == ucp_L || category == ucp_N || ct == ucp_Mn || ct == ucp_Pc) == notmatch) { break; }
                                Feptr!() = Feptr!().add(len);
                                i += 1;
                            }
                        }
                        PT_CLIST => {
                            i = Lmin!();
                            'gotmax: while i < Lmax!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                let len = GCLEN!();
                                let mut cp = Lpropvalue!() as usize;
                                loop {
                                    let v = _pcre2_ucd_caseless_sets_8[cp];
                                    if fc < v { if notmatch { break; } else { break 'gotmax; } }
                                    cp += 1;
                                    if fc == v { if notmatch { break 'gotmax; } else { break; } }
                                }
                                Feptr!() = Feptr!().add(len);
                                i += 1;
                            }
                        }
                        PT_UCNC => {
                            i = Lmin!();
                            while i < Lmax!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                let len = GCLEN!();
                                if ((fc == CHAR_DOLLAR_SIGN || fc == CHAR_COMMERCIAL_AT
                                    || fc == CHAR_GRAVE_ACCENT || (fc >= 0xa0 && fc <= 0xd7ff)
                                    || fc >= 0xe000) == notmatch) { break; }
                                Feptr!() = Feptr!().add(len);
                                i += 1;
                            }
                        }
                        PT_BIDICL => {
                            i = Lmin!();
                            while i < Lmax!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                let len = GCLEN!();
                                if ((UCD_BIDICLASS(fc) == Lpropvalue!()) == notmatch) { break; }
                                Feptr!() = Feptr!().add(len);
                                i += 1;
                            }
                        }
                        PT_BOOL => {
                            i = Lmin!();
                            while i < Lmax!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                let len = GCLEN!();
                                let prop = GET_UCD(fc);
                                let bp = UCD_BPROPS_PROP(prop) as usize;
                                let ok = MAPBIT(&_pcre2_ucd_boolprop_sets_8[bp..], Lpropvalue!()) != 0;
                                if ok == notmatch { break; }
                                Feptr!() = Feptr!().add(len);
                                i += 1;
                            }
                        }
                        _ => { return PCRE2_ERROR_INTERNAL; }
                    }
                    if reptype == REPTYPE_POS { NEXT_OP!('machine); }
                    state = ST_TYPE_RM221_LOOP;
                    continue 'machine;
                } else if Lctype!() == OP_EXTUNI as u32 {
                    i = Lmin!();
                    while i < Lmax!() {
                        if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                        else {
                            let c0 = *Feptr!() as u32;
                            if utf != 0 && c0 >= 0xc0 {
                                let (v, n) = GETCHARINC(Feptr!()); fc = v; Feptr!() = Feptr!().add(n);
                            } else { fc = c0; Feptr!() = Feptr!().add(1); }
                            Feptr!() = _pcre2_extuni_8(fc, Feptr!(), (*mb).start_subject,
                                (*mb).end_subject, utf, ptr::null_mut());
                        }
                        CHECK_PARTIAL!();
                        i += 1;
                    }
                    if reptype == REPTYPE_POS { NEXT_OP!('machine); }
                    state = ST_TYPE_RM219_LOOP;
                    continue 'machine;
                } else if utf != 0 {
                    state = ST_TYPE_MAX_UTF;
                    continue 'machine;
                } else {
                    state = ST_TYPE_MAX_NONUTF;
                    continue 'machine;
                }
            }

            ST_TYPE_MAX_UTF => {
                macro_rules! Lmin { () => { (*F).fields.type_repeat.min }; }
                macro_rules! Lmax { () => { (*F).fields.type_repeat.max }; }
                macro_rules! Lctype { () => { (*F).fields.type_repeat.ctype }; }
                match Lctype!() {
                    c if c == OP_ANY as u32 => {
                        i = Lmin!();
                        while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            if is_newline(Feptr!(), mb, utf) { break; }
                            if (*mb).partial != 0 && Feptr!().add(1) >= ES!()
                                && (*mb).nltype == NLTYPE_FIXED && (*mb).nllen == 2
                                && *Feptr!() == (*mb).nl[0] {
                                (*mb).hitend = TRUE;
                                if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL; }
                            }
                            Feptr!() = Feptr!().add(1);
                            while Feptr!() < ES!() && (*Feptr!() & 0xc0) == 0x80 { Feptr!() = Feptr!().add(1); }
                            i += 1;
                        }
                    }
                    c if c == OP_ALLANY as u32 => {
                        if Lmax!() < u32::MAX {
                            i = Lmin!();
                            while i < Lmax!() {
                                if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                                Feptr!() = Feptr!().add(1);
                                while Feptr!() < ES!() && (*Feptr!() & 0xc0) == 0x80 { Feptr!() = Feptr!().add(1); }
                                i += 1;
                            }
                        } else {
                            Feptr!() = ES!();
                            SCHECK_PARTIAL!();
                        }
                    }
                    c if c == OP_ANYBYTE as u32 => {
                        fc = Lmax!() - Lmin!();
                        if fc as usize > (ES!() as usize - Feptr!() as usize) {
                            Feptr!() = ES!();
                            SCHECK_PARTIAL!();
                        } else { Feptr!() = Feptr!().add(fc as usize); }
                    }
                    c if c == OP_ANYNL as u32 => {
                        i = Lmin!();
                        while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            let (v, e) = GETCHARLEN(Feptr!()); fc = v; let len = 1 + e as usize;
                            if fc == CHAR_CR {
                                Feptr!() = Feptr!().add(1);
                                if Feptr!() >= ES!() { break; }
                                if *Feptr!() as u32 == CHAR_LF { Feptr!() = Feptr!().add(1); }
                            } else {
                                if fc != CHAR_LF && ((*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF
                                    || (fc != CHAR_VT && fc != CHAR_FF && fc != CHAR_NEL
                                        && fc != 0x2028 && fc != 0x2029)) {
                                    break;
                                }
                                Feptr!() = Feptr!().add(len);
                            }
                            i += 1;
                        }
                    }
                    c if c == OP_NOT_HSPACE as u32 || c == OP_HSPACE as u32 => {
                        i = Lmin!();
                        while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            let (v, e) = GETCHARLEN(Feptr!()); fc = v; let len = 1 + e as usize;
                            let gotspace = is_hspace(fc);
                            if gotspace == (Lctype!() == OP_NOT_HSPACE as u32) { break; }
                            Feptr!() = Feptr!().add(len);
                            i += 1;
                        }
                    }
                    c if c == OP_NOT_VSPACE as u32 || c == OP_VSPACE as u32 => {
                        i = Lmin!();
                        while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            let (v, e) = GETCHARLEN(Feptr!()); fc = v; let len = 1 + e as usize;
                            let gotspace = is_vspace(fc);
                            if gotspace == (Lctype!() == OP_NOT_VSPACE as u32) { break; }
                            Feptr!() = Feptr!().add(len);
                            i += 1;
                        }
                    }
                    c if c == OP_NOT_DIGIT as u32 => {
                        i = Lmin!();
                        while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            let (v, e) = GETCHARLEN(Feptr!()); fc = v; let len = 1 + e as usize;
                            if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 { break; }
                            Feptr!() = Feptr!().add(len);
                            i += 1;
                        }
                    }
                    c if c == OP_DIGIT as u32 => {
                        i = Lmin!();
                        while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            let (v, e) = GETCHARLEN(Feptr!()); fc = v; let len = 1 + e as usize;
                            if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 { break; }
                            Feptr!() = Feptr!().add(len);
                            i += 1;
                        }
                    }
                    c if c == OP_NOT_WHITESPACE as u32 => {
                        i = Lmin!();
                        while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            let (v, e) = GETCHARLEN(Feptr!()); fc = v; let len = 1 + e as usize;
                            if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 { break; }
                            Feptr!() = Feptr!().add(len);
                            i += 1;
                        }
                    }
                    c if c == OP_WHITESPACE as u32 => {
                        i = Lmin!();
                        while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            let (v, e) = GETCHARLEN(Feptr!()); fc = v; let len = 1 + e as usize;
                            if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 { break; }
                            Feptr!() = Feptr!().add(len);
                            i += 1;
                        }
                    }
                    c if c == OP_NOT_WORDCHAR as u32 => {
                        i = Lmin!();
                        while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            let (v, e) = GETCHARLEN(Feptr!()); fc = v; let len = 1 + e as usize;
                            if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 { break; }
                            Feptr!() = Feptr!().add(len);
                            i += 1;
                        }
                    }
                    c if c == OP_WORDCHAR as u32 => {
                        i = Lmin!();
                        while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            let (v, e) = GETCHARLEN(Feptr!()); fc = v; let len = 1 + e as usize;
                            if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 { break; }
                            Feptr!() = Feptr!().add(len);
                            i += 1;
                        }
                    }
                    _ => { return PCRE2_ERROR_INTERNAL; }
                }
                if reptype == REPTYPE_POS { NEXT_OP!('machine); }
                state = ST_TYPE_RM220_LOOP;
                continue 'machine;
            }

            ST_TYPE_MAX_NONUTF => {
                macro_rules! Lmin { () => { (*F).fields.type_repeat.min }; }
                macro_rules! Lmax { () => { (*F).fields.type_repeat.max }; }
                macro_rules! Lctype { () => { (*F).fields.type_repeat.ctype }; }
                match Lctype!() {
                    c if c == OP_ANY as u32 => {
                        i = Lmin!();
                        while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            if is_newline(Feptr!(), mb, utf) { break; }
                            if (*mb).partial != 0 && Feptr!().add(1) >= ES!()
                                && (*mb).nltype == NLTYPE_FIXED && (*mb).nllen == 2
                                && *Feptr!() == (*mb).nl[0] {
                                (*mb).hitend = TRUE;
                                if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL; }
                            }
                            Feptr!() = Feptr!().add(1);
                            i += 1;
                        }
                    }
                    c if c == OP_ALLANY as u32 || c == OP_ANYBYTE as u32 => {
                        fc = Lmax!() - Lmin!();
                        if fc as usize > (ES!() as usize - Feptr!() as usize) {
                            Feptr!() = ES!();
                            SCHECK_PARTIAL!();
                        } else { Feptr!() = Feptr!().add(fc as usize); }
                    }
                    c if c == OP_ANYNL as u32 => {
                        i = Lmin!();
                        while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            fc = *Feptr!() as u32;
                            if fc == CHAR_CR {
                                Feptr!() = Feptr!().add(1);
                                if Feptr!() >= ES!() { break; }
                                if *Feptr!() as u32 == CHAR_LF { Feptr!() = Feptr!().add(1); }
                            } else {
                                if fc != CHAR_LF && ((*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF
                                    || (fc != CHAR_VT && fc != CHAR_FF && fc != CHAR_NEL)) {
                                    break;
                                }
                                Feptr!() = Feptr!().add(1);
                            }
                            i += 1;
                        }
                    }
                    c if c == OP_NOT_HSPACE as u32 => {
                        i = Lmin!();
                        'l00: while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            if is_hspace_byte(*Feptr!() as u32) { break 'l00; }
                            Feptr!() = Feptr!().add(1);
                            i += 1;
                        }
                    }
                    c if c == OP_HSPACE as u32 => {
                        i = Lmin!();
                        'l01: while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            if !is_hspace_byte(*Feptr!() as u32) { break 'l01; }
                            Feptr!() = Feptr!().add(1);
                            i += 1;
                        }
                    }
                    c if c == OP_NOT_VSPACE as u32 => {
                        i = Lmin!();
                        'l02: while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            if is_vspace_byte(*Feptr!() as u32) { break 'l02; }
                            Feptr!() = Feptr!().add(1);
                            i += 1;
                        }
                    }
                    c if c == OP_VSPACE as u32 => {
                        i = Lmin!();
                        'l03: while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            if !is_vspace_byte(*Feptr!() as u32) { break 'l03; }
                            Feptr!() = Feptr!().add(1);
                            i += 1;
                        }
                    }
                    c if c == OP_NOT_DIGIT as u32 => {
                        i = Lmin!();
                        while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            if (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_digit) != 0 { break; }
                            Feptr!() = Feptr!().add(1);
                            i += 1;
                        }
                    }
                    c if c == OP_DIGIT as u32 => {
                        i = Lmin!();
                        while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            if (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_digit) == 0 { break; }
                            Feptr!() = Feptr!().add(1);
                            i += 1;
                        }
                    }
                    c if c == OP_NOT_WHITESPACE as u32 => {
                        i = Lmin!();
                        while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            if (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_space) != 0 { break; }
                            Feptr!() = Feptr!().add(1);
                            i += 1;
                        }
                    }
                    c if c == OP_WHITESPACE as u32 => {
                        i = Lmin!();
                        while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            if (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_space) == 0 { break; }
                            Feptr!() = Feptr!().add(1);
                            i += 1;
                        }
                    }
                    c if c == OP_NOT_WORDCHAR as u32 => {
                        i = Lmin!();
                        while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            if (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_word) != 0 { break; }
                            Feptr!() = Feptr!().add(1);
                            i += 1;
                        }
                    }
                    c if c == OP_WORDCHAR as u32 => {
                        i = Lmin!();
                        while i < Lmax!() {
                            if Feptr!() >= ES!() { SCHECK_PARTIAL!(); break; }
                            if (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_word) == 0 { break; }
                            Feptr!() = Feptr!().add(1);
                            i += 1;
                        }
                    }
                    _ => { return PCRE2_ERROR_INTERNAL; }
                }
                if reptype == REPTYPE_POS { NEXT_OP!('machine); }
                state = ST_TYPE_RM34_LOOP;
                continue 'machine;
            }

            // ---- Type-repeat maximize backtrack resume points ----
            ST_TYPE_RM221_LOOP => {
                if Feptr!() <= (*F).fields.type_repeat.start_eptr { NEXT_OP!('machine); }
                RMATCH!('machine, Fecode!(), RM221);
            }
            x if x == RM221 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                Feptr!() = Feptr!().sub(1);
                if utf != 0 { Feptr!() = BACKCHAR(Feptr!()); }
                state = ST_TYPE_RM221_LOOP;
                continue 'machine;
            }
            ST_TYPE_RM219_LOOP => {
                if Feptr!() <= (*F).fields.type_repeat.start_eptr { NEXT_OP!('machine); }
                RMATCH!('machine, Fecode!(), RM219);
            }
            x if x == RM219 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                // Backtrack over extended grapheme cluster.
                Feptr!() = Feptr!().sub(1);
                if utf == 0 { fc = *Feptr!() as u32; }
                else {
                    Feptr!() = BACKCHAR(Feptr!());
                    fc = GETCHAR(Feptr!());
                }
                let mut rgb = UCD_GRAPHBREAK(fc);
                loop {
                    if Feptr!() <= (*F).fields.type_repeat.start_eptr { break; }
                    let mut fptr = Feptr!().sub(1);
                    if utf == 0 { fc = *fptr as u32; }
                    else {
                        fptr = BACKCHAR(fptr);
                        fc = GETCHAR(fptr);
                    }
                    let lgb = UCD_GRAPHBREAK(fc);
                    if (_pcre2_ucp_gbtable_8[lgb as usize] & (1u32 << rgb)) == 0 { break; }
                    Feptr!() = fptr;
                    rgb = lgb;
                }
                state = ST_TYPE_RM219_LOOP;
                continue 'machine;
            }
            ST_TYPE_RM220_LOOP => {
                if Feptr!() <= (*F).fields.type_repeat.start_eptr { NEXT_OP!('machine); }
                RMATCH!('machine, Fecode!(), RM220);
            }
            x if x == RM220 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                Feptr!() = Feptr!().sub(1);
                Feptr!() = BACKCHAR(Feptr!());
                if (*F).fields.type_repeat.ctype == OP_ANYNL as u32
                    && Feptr!() > (*F).fields.type_repeat.start_eptr
                    && *Feptr!() as u32 == CHAR_NL
                    && *Feptr!().sub(1) as u32 == CHAR_CR {
                    Feptr!() = Feptr!().sub(1);
                }
                state = ST_TYPE_RM220_LOOP;
                continue 'machine;
            }
            ST_TYPE_RM34_LOOP => {
                if Feptr!() == (*F).fields.type_repeat.start_eptr { NEXT_OP!('machine); }
                RMATCH!('machine, Fecode!(), RM34);
            }
            x if x == RM34 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                Feptr!() = Feptr!().sub(1);
                if (*F).fields.type_repeat.ctype == OP_ANYNL as u32
                    && Feptr!() > (*F).fields.type_repeat.start_eptr
                    && *Feptr!() as u32 == CHAR_LF
                    && *Feptr!().sub(1) as u32 == CHAR_CR {
                    Feptr!() = Feptr!().sub(1);
                }
                state = ST_TYPE_RM34_LOOP;
                continue 'machine;
            }

            // ===== ST_MAINLOOP6: references, groups, recursion, assertions, etc. =====
            ST_MAINLOOP6 => {
                match Fop!() {
                    OP_DNREF | OP_DNREFI => {
                        Fbyte1!() = (Fop!() == OP_DNREFI) as u8;
                        Fbyte2!() = if Fop!() == OP_DNREFI { *Fecode!().add(1 + 2 * IMM2_SIZE) } else { 0 };
                        {
                            let mut count = GET2(Fecode!(), 1 + IMM2_SIZE) as i32;
                            let mut slot = (*mb).name_table
                                .add((GET2(Fecode!(), 1) as usize) * (*mb).name_entry_size as usize);
                            Fecode!() = Fecode!().add(1 + 2 * IMM2_SIZE + (if Fop!() == OP_DNREFI { 1 } else { 0 }));
                            while count > 0 {
                                count -= 1;
                                (*F).fields.ref_repeat.offset = ((GET2(slot, 0) as usize) << 1) - 2;
                                if (*F).fields.ref_repeat.offset < Foffset_top!()
                                    && *Fovector!().add((*F).fields.ref_repeat.offset) != PCRE2_UNSET {
                                    break;
                                }
                                slot = slot.add((*mb).name_entry_size as usize);
                            }
                        }
                        state = ST_REF_REPEAT;
                        continue 'machine;
                    }
                    OP_REF | OP_REFI => {
                        Fbyte1!() = (Fop!() == OP_REFI) as u8;
                        Fbyte2!() = if Fop!() == OP_REFI { *Fecode!().add(1 + IMM2_SIZE) } else { 0 };
                        (*F).fields.ref_repeat.offset = ((GET2(Fecode!(), 1) as usize) << 1) - 2;
                        Fecode!() = Fecode!().add(1 + IMM2_SIZE + (if Fop!() == OP_REFI { 1 } else { 0 }));
                        state = ST_REF_REPEAT;
                        continue 'machine;
                    }
                    _ => {
                        state = ST_MAINLOOP7;
                        continue 'machine;
                    }
                }
            }

            ST_REF_REPEAT => {
                macro_rules! Lstart { () => { (*F).fields.ref_repeat.start }; }
                macro_rules! Loffset { () => { (*F).fields.ref_repeat.offset }; }
                macro_rules! Llength { () => { (*F).fields.ref_repeat.length }; }
                macro_rules! Lmin { () => { (*F).fields.ref_repeat.min }; }
                macro_rules! Lmax { () => { (*F).fields.ref_repeat.max }; }
                macro_rules! Lcaseless { () => { *fbyte1(F) }; }
                macro_rules! Lcaseopts { () => { *fbyte2(F) }; }

                match *Fecode!() {
                    OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
                    | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
                        fc = *Fecode!() as u32 - OP_CRSTAR as u32;
                        Fecode!() = Fecode!().add(1);
                        Lmin!() = REP_MIN[fc as usize];
                        Lmax!() = REP_MAX[fc as usize];
                        reptype = REP_TYP[fc as usize];
                    }
                    OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                        Lmin!() = GET2(Fecode!(), 1);
                        Lmax!() = GET2(Fecode!(), 1 + IMM2_SIZE);
                        reptype = REP_TYP[(*Fecode!() - OP_CRSTAR) as usize];
                        if Lmax!() == 0 { Lmax!() = u32::MAX; }
                        Fecode!() = Fecode!().add(1 + 2 * IMM2_SIZE);
                    }
                    _ => {
                        rrc = match_ref(Loffset!(), Lcaseless!() as BOOL, Lcaseopts!() as c_int, F, mb, ptr::addr_of_mut!(length));
                        if rrc != 0 {
                            if rrc > 0 { Feptr!() = ES!(); }
                            CHECK_PARTIAL!();
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        Feptr!() = Feptr!().add(length);
                        NEXT_OP!('machine);
                    }
                }

                if Loffset!() < Foffset_top!() && *Fovector!().add(Loffset!()) != PCRE2_UNSET {
                    if *Fovector!().add(Loffset!()) == *Fovector!().add(Loffset!() + 1) {
                        NEXT_OP!('machine);
                    }
                } else {
                    if Lmin!() == 0 || ((*mb).poptions & PCRE2_MATCH_UNSET_BACKREF) != 0 {
                        NEXT_OP!('machine);
                    }
                }

                i = 1;
                while i <= Lmin!() {
                    let mut slength: PCRE2_SIZE = 0;
                    rrc = match_ref(Loffset!(), Lcaseless!() as BOOL, Lcaseopts!() as c_int, F, mb, ptr::addr_of_mut!(slength));
                    if rrc != 0 {
                        if rrc > 0 { Feptr!() = ES!(); }
                        CHECK_PARTIAL!();
                        RRETURN!('machine, MATCH_NOMATCH);
                    }
                    Feptr!() = Feptr!().add(slength);
                    i += 1;
                }

                if Lmin!() == Lmax!() { NEXT_OP!('machine); }

                if reptype == REPTYPE_MIN {
                    RMATCH!('machine, Fecode!(), RM20);
                } else {
                    let mut samelengths = true;
                    Lstart!() = Feptr!();
                    Llength!() = *Fovector!().add(Loffset!() + 1) - *Fovector!().add(Loffset!());

                    i = Lmin!();
                    while i < Lmax!() {
                        let mut slength: PCRE2_SIZE = 0;
                        rrc = match_ref(Loffset!(), Lcaseless!() as BOOL, Lcaseopts!() as c_int, F, mb, ptr::addr_of_mut!(slength));
                        if rrc != 0 {
                            if rrc > 0 && (*mb).partial != 0 && ES!() > (*mb).start_used_ptr {
                                (*mb).hitend = TRUE;
                                if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL; }
                            }
                            break;
                        }
                        if slength != Llength!() { samelengths = false; }
                        Feptr!() = Feptr!().add(slength);
                        i += 1;
                    }

                    if reptype == REPTYPE_POS { NEXT_OP!('machine); }

                    if samelengths {
                        state = ST_REF_RM21_LOOP;
                        continue 'machine;
                    } else {
                        Lmax!() = i;
                        state = ST_REF_RM22_LOOP;
                        continue 'machine;
                    }
                }
            }

            x if x == RM20 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                let lmin = (*F).fields.ref_repeat.min;
                (*F).fields.ref_repeat.min = lmin + 1;
                if lmin >= (*F).fields.ref_repeat.max { RRETURN!('machine, MATCH_NOMATCH); }
                let mut slength: PCRE2_SIZE = 0;
                rrc = match_ref((*F).fields.ref_repeat.offset, *fbyte1(F) as BOOL, *fbyte2(F) as c_int, F, mb, ptr::addr_of_mut!(slength));
                if rrc != 0 {
                    if rrc > 0 { Feptr!() = ES!(); }
                    CHECK_PARTIAL!();
                    RRETURN!('machine, MATCH_NOMATCH);
                }
                Feptr!() = Feptr!().add(slength);
                RMATCH!('machine, Fecode!(), RM20);
            }
            ST_REF_RM21_LOOP => {
                if Feptr!() >= (*F).fields.ref_repeat.start {
                    RMATCH!('machine, Fecode!(), RM21);
                }
                RRETURN!('machine, MATCH_NOMATCH);
            }
            x if x == RM21 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                Feptr!() = Feptr!().sub((*F).fields.ref_repeat.length);
                state = ST_REF_RM21_LOOP;
                continue 'machine;
            }
            ST_REF_RM22_LOOP => {
                RMATCH!('machine, Fecode!(), RM22);
            }
            x if x == RM22 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                if Feptr!() == (*F).fields.ref_repeat.start { RRETURN!('machine, MATCH_NOMATCH); }
                Feptr!() = (*F).fields.ref_repeat.start;
                (*F).fields.ref_repeat.max -= 1;
                i = (*F).fields.ref_repeat.min;
                while i < (*F).fields.ref_repeat.max {
                    let mut slength: PCRE2_SIZE = 0;
                    let _ = match_ref((*F).fields.ref_repeat.offset, *fbyte1(F) as BOOL, *fbyte2(F) as c_int, F, mb, ptr::addr_of_mut!(slength));
                    Feptr!() = Feptr!().add(slength);
                    i += 1;
                }
                state = ST_REF_RM22_LOOP;
                continue 'machine;
            }

            // ===== ST_MAINLOOP7: BRAZERO/BRAPOS/BRA/CBRA/groups =====
            ST_MAINLOOP7 => {
                match Fop!() {
                    OP_BRAZERO => {
                        Fecode!() = Fecode!().add(1);
                        RMATCH!('machine, Fecode!(), RM9);
                    }
                    OP_BRAMINZERO => {
                        Fecode!() = Fecode!().add(1);
                        let mut next_ecode = Fecode!();
                        loop {
                            next_ecode = next_ecode.add(GET(next_ecode, 1) as usize);
                            if *next_ecode != OP_ALT { break; }
                        }
                        RMATCH!('machine, next_ecode.add(1 + LINK_SIZE), RM10);
                    }
                    OP_SKIPZERO => {
                        let mut next_ecode = Fecode!().add(1);
                        loop {
                            next_ecode = next_ecode.add(GET(next_ecode, 1) as usize);
                            if *next_ecode != OP_ALT { break; }
                        }
                        Fecode!() = next_ecode.add(1 + LINK_SIZE);
                        NEXT_OP!('machine);
                    }
                    OP_BRAPOSZERO => {
                        Fbyte2!() = TRUE as u8; // Lzero_allowed
                        Fecode!() = Fecode!().add(1);
                        if *Fecode!() == OP_CBRAPOS || *Fecode!() == OP_SCBRAPOS {
                            number = GET2(Fecode!(), 1 + LINK_SIZE);
                            (*F).fields.op_brapos.frame_type = GF_CAPTURE | number;
                        } else {
                            (*F).fields.op_brapos.frame_type = GF_NOCAPTURE;
                        }
                        state = ST_POSSESSIVE_GROUP;
                        continue 'machine;
                    }
                    OP_BRAPOS | OP_SBRAPOS => {
                        Fbyte2!() = FALSE as u8;
                        (*F).fields.op_brapos.frame_type = GF_NOCAPTURE;
                        state = ST_POSSESSIVE_GROUP;
                        continue 'machine;
                    }
                    OP_CBRAPOS | OP_SCBRAPOS => {
                        Fbyte2!() = FALSE as u8;
                        number = GET2(Fecode!(), 1 + LINK_SIZE);
                        (*F).fields.op_brapos.frame_type = GF_CAPTURE | number;
                        state = ST_POSSESSIVE_GROUP;
                        continue 'machine;
                    }
                    OP_BRA => {
                        if (*mb).hasthen != 0 || Frdepth!() == 0 {
                            (*F).fields.op_bra.frame_type = 0;
                            state = ST_GROUPLOOP;
                            continue 'machine;
                        }
                        state = ST_BRA_LOOP;
                        continue 'machine;
                    }
                    OP_CBRA | OP_SCBRA => {
                        (*F).fields.op_bra.frame_type = GF_CAPTURE | GET2(Fecode!(), 1 + LINK_SIZE);
                        state = ST_GROUPLOOP;
                        continue 'machine;
                    }
                    OP_ONCE | OP_SCRIPT_RUN | OP_SBRA => {
                        (*F).fields.op_bra.frame_type = GF_NOCAPTURE;
                        state = ST_GROUPLOOP;
                        continue 'machine;
                    }
                    OP_RECURSE => {
                        state = ST_RECURSE_ENTRY;
                        continue 'machine;
                    }
                    _ => {
                        state = ST_MAINLOOP8;
                        continue 'machine;
                    }
                }
            }

            // ---- POSSESSIVE_GROUP ----
            ST_POSSESSIVE_GROUP => {
                // Lmatched_once = Fbyte1, Lzero_allowed = Fbyte2
                Fbyte1!() = FALSE as u8;
                (*F).fields.op_brapos.start_group = Fecode!();
                state = ST_POSSESSIVE_LOOP;
                continue 'machine;
            }
            ST_POSSESSIVE_LOOP => {
                (*F).fields.op_brapos.start_eptr = Feptr!();
                group_frame_type = (*F).fields.op_brapos.frame_type;
                RMATCH!('machine, Fecode!().add(op_length(*Fecode!())), RM8);
            }
            x if x == RM8 as i32 => {
                if rrc == MATCH_KETRPOS {
                    Fbyte1!() = TRUE as u8;
                    if Feptr!() == (*F).fields.op_brapos.start_eptr {
                        loop {
                            Fecode!() = Fecode!().add(GET(Fecode!(), 1) as usize);
                            if *Fecode!() != OP_ALT { break; }
                        }
                        // break out of loop -> success test
                        if Fbyte1!() != 0 || Fbyte2!() != 0 {
                            Fecode!() = Fecode!().add(1 + LINK_SIZE);
                            NEXT_OP!('machine);
                        }
                        RRETURN!('machine, MATCH_NOMATCH);
                    }
                    Fecode!() = (*F).fields.op_brapos.start_group;
                    state = ST_POSSESSIVE_LOOP;
                    continue 'machine;
                }
                if rrc == MATCH_THEN {
                    let next_ecode = Fecode!().add(GET(Fecode!(), 1) as usize);
                    if (*mb).verb_ecode_ptr < next_ecode
                        && (*Fecode!() == OP_ALT || *next_ecode == OP_ALT) {
                        rrc = MATCH_NOMATCH;
                    }
                }
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                Fecode!() = Fecode!().add(GET(Fecode!(), 1) as usize);
                if *Fecode!() == OP_ALT {
                    state = ST_POSSESSIVE_LOOP;
                    continue 'machine;
                }
                if Fbyte1!() != 0 || Fbyte2!() != 0 {
                    Fecode!() = Fecode!().add(1 + LINK_SIZE);
                    NEXT_OP!('machine);
                }
                RRETURN!('machine, MATCH_NOMATCH);
            }

            // ---- OP_BRA optimized branch loop ----
            ST_BRA_LOOP => {
                let current_branch = Fecode!();
                let next_branch = current_branch.add(GET(current_branch, 1) as usize);
                if *next_branch != OP_ALT {
                    Fecode!() = Fecode!().add(1 + LINK_SIZE);
                    NEXT_OP!('machine);
                }
                Fecode!() = next_branch;
                RMATCH!('machine, current_branch.add(1 + LINK_SIZE), RM1);
            }
            x if x == RM1 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                state = ST_BRA_LOOP;
                continue 'machine;
            }

            // ---- GROUPLOOP (RM2) ----
            ST_GROUPLOOP => {
                group_frame_type = (*F).fields.op_bra.frame_type;
                RMATCH!('machine, Fecode!().add(op_length(*Fecode!())), RM2);
            }
            x if x == RM2 as i32 => {
                if rrc == MATCH_THEN {
                    let next_ecode = Fecode!().add(GET(Fecode!(), 1) as usize);
                    if (*mb).verb_ecode_ptr < next_ecode
                        && (*Fecode!() == OP_ALT || *next_ecode == OP_ALT) {
                        rrc = MATCH_NOMATCH;
                    }
                }
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                Fecode!() = Fecode!().add(GET(Fecode!(), 1) as usize);
                if *Fecode!() != OP_ALT { RRETURN!('machine, MATCH_NOMATCH); }
                state = ST_GROUPLOOP;
                continue 'machine;
            }

            x if x == RM9 as i32 => {
                // BRAZERO after RMATCH
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                let mut next_ecode = Fecode!();
                loop {
                    next_ecode = next_ecode.add(GET(next_ecode, 1) as usize);
                    if *next_ecode != OP_ALT { break; }
                }
                Fecode!() = next_ecode.add(1 + LINK_SIZE);
                NEXT_OP!('machine);
            }
            x if x == RM10 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                NEXT_OP!('machine);
            }

            // ---- OP_RECURSE ----
            ST_RECURSE_ENTRY => {
                macro_rules! Lstart_branch { () => { (*F).fields.op_recurse.start_branch }; }
                macro_rules! Lframe_type { () => { (*F).fields.op_recurse.frame_type }; }

                bracode = (*mb).start_code.add(GET(Fecode!(), 1) as usize);
                number = if bracode == (*mb).start_code { 0 } else { GET2(bracode, 1 + LINK_SIZE) };

                if Fcurrent_recurse!() != RECURSE_UNSET {
                    offset = Flast_group_offset!();
                    while offset != PCRE2_UNSET {
                        N = frame_byte_add((*match_data).heapframes as *mut heapframe, offset);
                        P = frame_byte_sub(N, frame_size);
                        if (*N).group_frame_type == (GF_RECURSE | number) {
                            if Feptr!() == (*P).eptr
                                && (*mb).last_used_ptr == (*P).recurse_last_used
                                && ((*mb).moptions & PCRE2_DISABLE_RECURSELOOP_CHECK) == 0 {
                                return PCRE2_ERROR_RECURSELOOP;
                            }
                            break;
                        }
                        offset = (*P).last_group_offset;
                    }
                }

                (*F).recurse_last_used = (*mb).last_used_ptr;
                Lstart_branch!() = bracode;
                Lframe_type!() = GF_RECURSE | number;
                state = ST_RECURSE_LOOP;
                continue 'machine;
            }
            ST_RECURSE_LOOP => {
                group_frame_type = (*F).fields.op_recurse.frame_type;
                RMATCH!('machine, (*F).fields.op_recurse.start_branch.add(op_length(*(*F).fields.op_recurse.start_branch)), RM11);
            }
            x if x == RM11 as i32 => {
                let lframe_type = (*F).fields.op_recurse.frame_type;
                let start_branch = (*F).fields.op_recurse.start_branch;
                let next_ecode = start_branch.add(GET(start_branch, 1) as usize);

                if rrc >= MATCH_BACKTRACK_MIN && rrc <= MATCH_BACKTRACK_MAX
                    && (*mb).verb_current_recurse == (lframe_type ^ GF_RECURSE) {
                    if rrc == MATCH_THEN && (*mb).verb_ecode_ptr < next_ecode
                        && (*start_branch == OP_ALT || *next_ecode == OP_ALT) {
                        rrc = MATCH_NOMATCH;
                    } else {
                        RRETURN!('machine, MATCH_NOMATCH);
                    }
                }

                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                (*F).fields.op_recurse.start_branch = next_ecode;
                if *next_ecode != OP_ALT { RRETURN!('machine, MATCH_NOMATCH); }
                state = ST_RECURSE_LOOP;
                continue 'machine;
            }

            // ===== ST_MAINLOOP8: assertions, scs, callout, cond =====
            ST_MAINLOOP8 => {
                match Fop!() {
                    OP_ASSERT | OP_ASSERTBACK | OP_ASSERT_NA | OP_ASSERTBACK_NA => {
                        state = ST_ASSERT_LOOP;
                        continue 'machine;
                    }
                    OP_ASSERT_NOT | OP_ASSERTBACK_NOT => {
                        state = ST_ASSERT_NOT_LOOP;
                        continue 'machine;
                    }
                    OP_ASSERT_SCS => {
                        state = ST_SCS_ENTRY;
                        continue 'machine;
                    }
                    OP_CALLOUT | OP_CALLOUT_STR => {
                        rrc = do_callout(F, mb, ptr::addr_of_mut!(length));
                        if rrc > 0 { RRETURN!('machine, MATCH_NOMATCH); }
                        if rrc < 0 { RRETURN!('machine, rrc); }
                        Fecode!() = Fecode!().add(length);
                        NEXT_OP!('machine);
                    }
                    OP_COND | OP_SCOND => {
                        state = ST_COND_ENTRY;
                        continue 'machine;
                    }
                    _ => {
                        state = ST_MAINLOOP9;
                        continue 'machine;
                    }
                }
            }

            // ---- Positive assertions (RM3) ----
            ST_ASSERT_LOOP => {
                group_frame_type = GF_NOCAPTURE;
                RMATCH!('machine, Fecode!().add(op_length(*Fecode!())), RM3);
            }
            x if x == RM3 as i32 => {
                if rrc == MATCH_ACCEPT {
                    ptr::copy_nonoverlapping(
                        (assert_accept_frame as *const u8).add(core::mem::offset_of!(heapframe, ovector)) as *const PCRE2_SIZE,
                        Fovector!(),
                        (*assert_accept_frame).offset_top,
                    );
                    Foffset_top!() = (*assert_accept_frame).offset_top;
                    Fmark!() = (*assert_accept_frame).mark;
                    // break out -> advance to end of assertion
                    loop {
                        Fecode!() = Fecode!().add(GET(Fecode!(), 1) as usize);
                        if *Fecode!() != OP_ALT { break; }
                    }
                    Fecode!() = Fecode!().add(1 + LINK_SIZE);
                    NEXT_OP!('machine);
                }
                if rrc != MATCH_NOMATCH && rrc != MATCH_THEN { RRETURN!('machine, rrc); }
                Fecode!() = Fecode!().add(GET(Fecode!(), 1) as usize);
                if *Fecode!() != OP_ALT { RRETURN!('machine, MATCH_NOMATCH); }
                state = ST_ASSERT_LOOP;
                continue 'machine;
            }

            // ---- Negative assertions (RM4) ----
            ST_ASSERT_NOT_LOOP => {
                group_frame_type = GF_NOCAPTURE;
                RMATCH!('machine, Fecode!().add(op_length(*Fecode!())), RM4);
            }
            x if x == RM4 as i32 => {
                match rrc {
                    r if r == MATCH_ACCEPT || r == MATCH_MATCH => { RRETURN!('machine, MATCH_NOMATCH); }
                    r if r == MATCH_NOMATCH || r == MATCH_THEN => {
                        Fecode!() = Fecode!().add(GET(Fecode!(), 1) as usize);
                        if *Fecode!() != OP_ALT {
                            Fecode!() = Fecode!().add(1 + LINK_SIZE);
                            NEXT_OP!('machine);
                        }
                        state = ST_ASSERT_NOT_LOOP;
                        continue 'machine;
                    }
                    r if r == MATCH_COMMIT || r == MATCH_SKIP || r == MATCH_PRUNE => {
                        loop {
                            Fecode!() = Fecode!().add(GET(Fecode!(), 1) as usize);
                            if *Fecode!() != OP_ALT { break; }
                        }
                        Fecode!() = Fecode!().add(1 + LINK_SIZE);
                        NEXT_OP!('machine);
                    }
                    _ => { RRETURN!('machine, rrc); }
                }
            }

            // ---- OP_ASSERT_SCS ----
            ST_SCS_ENTRY => {
                macro_rules! Lsaved_end_subject { () => { (*F).fields.op_assert_scs.saved_end_subject }; }
                macro_rules! Lsaved_eptr { () => { (*F).fields.op_assert_scs.saved_eptr }; }
                macro_rules! Ltrue_end_extra { () => { (*F).fields.op_assert_scs.true_end_extra }; }
                macro_rules! Lsaved_moptions { () => { (*F).fields.op_assert_scs.saved_moptions }; }

                length = 0;
                let mut found = false;
                {
                    let mut ecode = Fecode!().add(1 + LINK_SIZE);
                    offset = 0;
                    'scan: loop {
                        if *ecode == OP_CREF {
                            length += 1 + IMM2_SIZE;
                            offset = ((GET2(ecode, 1) as usize) << 1) - 2;
                            ecode = ecode.add(1 + IMM2_SIZE);
                            if offset < Foffset_top!() && *Fovector!().add(offset) != PCRE2_UNSET {
                                found = true;
                                break 'scan;
                            }
                            continue;
                        }
                        if *ecode != OP_DNCREF { RRETURN!('machine, MATCH_NOMATCH); }

                        let mut count = GET2(ecode, 1 + IMM2_SIZE) as i32;
                        let mut slot = (*mb).name_table
                            .add((GET2(ecode, 1) as usize) * (*mb).name_entry_size as usize);
                        length += 1 + 2 * IMM2_SIZE;
                        ecode = ecode.add(1 + 2 * IMM2_SIZE);

                        while count > 0 {
                            offset = ((GET2(slot, 0) as usize) << 1) - 2;
                            if offset < Foffset_top!() && *Fovector!().add(offset) != PCRE2_UNSET {
                                found = true;
                                break 'scan;
                            }
                            slot = slot.add((*mb).name_entry_size as usize);
                            count -= 1;
                        }
                    }
                    let _ = found;

                    // Skip remaining options.
                    loop {
                        if *ecode == OP_CREF {
                            length += 1 + IMM2_SIZE;
                            ecode = ecode.add(1 + IMM2_SIZE);
                        } else if *ecode == OP_DNCREF {
                            length += 1 + 2 * IMM2_SIZE;
                            ecode = ecode.add(1 + 2 * IMM2_SIZE);
                        } else {
                            break;
                        }
                    }
                }

                Lsaved_end_subject!() = (*mb).end_subject;
                Ltrue_end_extra!() = (*mb).true_end_subject as usize - (*mb).end_subject as usize;
                Lsaved_eptr!() = Feptr!();
                Lsaved_moptions!() = (*mb).moptions;

                Feptr!() = (*mb).start_subject.add(*Fovector!().add(offset));
                let new_end = (*mb).start_subject.add(*Fovector!().add(offset + 1));
                (*mb).true_end_subject = new_end;
                (*mb).end_subject = new_end;
                (*mb).moptions &= !PCRE2_NOTEOL;

                state = ST_SCS_LOOP2;
                continue 'machine;
            }
            ST_SCS_LOOP2 => {
                group_frame_type = GF_NOCAPTURE;
                RMATCH!('machine, Fecode!().add(1 + LINK_SIZE + length), RM38);
            }
            x if x == RM38 as i32 => {
                macro_rules! Lsaved_end_subject { () => { (*F).fields.op_assert_scs.saved_end_subject }; }
                macro_rules! Lsaved_eptr { () => { (*F).fields.op_assert_scs.saved_eptr }; }
                macro_rules! Ltrue_end_extra { () => { (*F).fields.op_assert_scs.true_end_extra }; }
                macro_rules! Lsaved_moptions { () => { (*F).fields.op_assert_scs.saved_moptions }; }

                if rrc == MATCH_ACCEPT {
                    ptr::copy_nonoverlapping(
                        (assert_accept_frame as *const u8).add(core::mem::offset_of!(heapframe, ovector)) as *const PCRE2_SIZE,
                        Fovector!(),
                        (*assert_accept_frame).offset_top,
                    );
                    Foffset_top!() = (*assert_accept_frame).offset_top;
                    Fmark!() = (*assert_accept_frame).mark;
                    (*mb).end_subject = Lsaved_end_subject!();
                    (*mb).true_end_subject = (*mb).end_subject.add(Ltrue_end_extra!());
                    (*mb).moptions = Lsaved_moptions!();
                    // break -> advance past assertion
                    loop {
                        Fecode!() = Fecode!().add(GET(Fecode!(), 1) as usize);
                        if *Fecode!() != OP_ALT { break; }
                    }
                    Fecode!() = Fecode!().add(1 + LINK_SIZE);
                    Feptr!() = Lsaved_eptr!();
                    NEXT_OP!('machine);
                }

                if rrc != MATCH_NOMATCH && rrc != MATCH_THEN {
                    (*mb).end_subject = Lsaved_end_subject!();
                    (*mb).true_end_subject = (*mb).end_subject.add(Ltrue_end_extra!());
                    (*mb).moptions = Lsaved_moptions!();
                    RRETURN!('machine, rrc);
                }

                Fecode!() = Fecode!().add(GET(Fecode!(), 1) as usize);
                if *Fecode!() != OP_ALT {
                    (*mb).end_subject = Lsaved_end_subject!();
                    (*mb).true_end_subject = (*mb).end_subject.add(Ltrue_end_extra!());
                    (*mb).moptions = Lsaved_moptions!();
                    RRETURN!('machine, MATCH_NOMATCH);
                }
                length = 0;
                state = ST_SCS_LOOP2;
                continue 'machine;
            }

            // ---- OP_COND / OP_SCOND ----
            ST_COND_ENTRY => {
                macro_rules! Lstart_branch { () => { (*F).fields.op_cond.start_branch }; }
                macro_rules! Llength { () => { (*F).fields.op_cond.length }; }
                macro_rules! Lpositive { () => { *fbyte1(F) }; }

                Llength!() = GET(Fecode!(), 1) as usize;
                if *Fecode!().add(Llength!()) != OP_ALT {
                    Llength!() = Llength!() - (1 + LINK_SIZE);
                }
                Fecode!() = Fecode!().add(1 + LINK_SIZE);

                if *Fecode!() == OP_CALLOUT || *Fecode!() == OP_CALLOUT_STR {
                    rrc = do_callout(F, mb, ptr::addr_of_mut!(length));
                    if rrc > 0 { RRETURN!('machine, MATCH_NOMATCH); }
                    if rrc < 0 { RRETURN!('machine, rrc); }
                    Fecode!() = Fecode!().add(length);
                    Llength!() = Llength!() - length;
                }

                condition = false;
                match *Fecode!() {
                    OP_RREF => {
                        if Fcurrent_recurse!() != RECURSE_UNSET {
                            number = GET2(Fecode!(), 1);
                            condition = number == RREF_ANY || number == Fcurrent_recurse!();
                        }
                    }
                    OP_DNRREF => {
                        if Fcurrent_recurse!() != RECURSE_UNSET {
                            let mut count = GET2(Fecode!(), 1 + IMM2_SIZE) as i32;
                            let mut slot = (*mb).name_table
                                .add((GET2(Fecode!(), 1) as usize) * (*mb).name_entry_size as usize);
                            while count > 0 {
                                count -= 1;
                                number = GET2(slot, 0);
                                condition = number == Fcurrent_recurse!();
                                if condition { break; }
                                slot = slot.add((*mb).name_entry_size as usize);
                            }
                        }
                    }
                    OP_CREF => {
                        offset = ((GET2(Fecode!(), 1) as usize) << 1) - 2;
                        condition = offset < Foffset_top!() && *Fovector!().add(offset) != PCRE2_UNSET;
                    }
                    OP_DNCREF => {
                        let mut count = GET2(Fecode!(), 1 + IMM2_SIZE) as i32;
                        let mut slot = (*mb).name_table
                            .add((GET2(Fecode!(), 1) as usize) * (*mb).name_entry_size as usize);
                        while count > 0 {
                            count -= 1;
                            offset = ((GET2(slot, 0) as usize) << 1) - 2;
                            condition = offset < Foffset_top!() && *Fovector!().add(offset) != PCRE2_UNSET;
                            if condition { break; }
                            slot = slot.add((*mb).name_entry_size as usize);
                        }
                    }
                    OP_FALSE | OP_FAIL => {}
                    OP_TRUE => { condition = true; }
                    _ => {
                        Lpositive!() = (*Fecode!() == OP_ASSERT || *Fecode!() == OP_ASSERTBACK) as u8;
                        Lstart_branch!() = Fecode!();
                        state = ST_COND_ASSERT_LOOP;
                        continue 'machine;
                    }
                }

                // Choose branch according to condition.
                Fecode!() = Fecode!().add(if condition { op_length(*Fecode!()) } else { (*F).fields.op_cond.length });

                if Fop!() == OP_SCOND {
                    group_frame_type = GF_NOCAPTURE;
                    RMATCH!('machine, Fecode!(), RM35);
                }
                NEXT_OP!('machine);
            }
            ST_COND_ASSERT_LOOP => {
                group_frame_type = GF_CONDASSERT;
                RMATCH!('machine, (*F).fields.op_cond.start_branch.add(op_length(*(*F).fields.op_cond.start_branch)), RM5);
            }
            x if x == RM5 as i32 => {
                macro_rules! Lstart_branch { () => { (*F).fields.op_cond.start_branch }; }
                macro_rules! Lpositive { () => { *fbyte1(F) }; }
                let lpositive = Lpositive!() != 0;
                match rrc {
                    r if r == MATCH_ACCEPT => {
                        ptr::copy_nonoverlapping(
                            (assert_accept_frame as *const u8).add(core::mem::offset_of!(heapframe, ovector)) as *const PCRE2_SIZE,
                            Fovector!(),
                            (*assert_accept_frame).offset_top,
                        );
                        Foffset_top!() = (*assert_accept_frame).offset_top;
                        condition = lpositive;
                    }
                    r if r == MATCH_MATCH => {
                        condition = lpositive;
                    }
                    r if r == MATCH_NOMATCH || r == MATCH_THEN => {
                        Lstart_branch!() = Lstart_branch!().add(GET(Lstart_branch!(), 1) as usize);
                        if *Lstart_branch!() == OP_ALT {
                            state = ST_COND_ASSERT_LOOP;
                            continue 'machine;
                        }
                        condition = !lpositive;
                    }
                    r if r == MATCH_COMMIT || r == MATCH_SKIP || r == MATCH_PRUNE => {
                        condition = !lpositive;
                    }
                    _ => { RRETURN!('machine, rrc); }
                }

                if condition {
                    loop {
                        Fecode!() = Fecode!().add(GET(Fecode!(), 1) as usize);
                        if *Fecode!() != OP_ALT { break; }
                    }
                }

                Fecode!() = Fecode!().add(if condition { op_length(*Fecode!()) } else { (*F).fields.op_cond.length });

                if Fop!() == OP_SCOND {
                    group_frame_type = GF_NOCAPTURE;
                    RMATCH!('machine, Fecode!(), RM35);
                }
                NEXT_OP!('machine);
            }
            x if x == RM35 as i32 => {
                RRETURN!('machine, rrc);
            }

            // ===== ST_MAINLOOP9: REVERSE, VREVERSE, ALT, KET =====
            ST_MAINLOOP9 => {
                match Fop!() {
                    OP_REVERSE => {
                        number = GET2(Fecode!(), 1);
                        if utf != 0 {
                            while number > 0 {
                                number -= 1;
                                if Feptr!() <= (*mb).check_subject { RRETURN!('machine, MATCH_NOMATCH); }
                                Feptr!() = Feptr!().sub(1);
                                Feptr!() = BACKCHAR(Feptr!());
                            }
                        } else {
                            if (number as isize) > (Feptr!() as isize - (*mb).start_subject as isize) {
                                RRETURN!('machine, MATCH_NOMATCH);
                            }
                            Feptr!() = Feptr!().sub(number as usize);
                        }
                        if Feptr!() < (*mb).start_used_ptr { (*mb).start_used_ptr = Feptr!(); }
                        Fecode!() = Fecode!().add(1 + IMM2_SIZE);
                        NEXT_OP!('machine);
                    }
                    OP_VREVERSE => {
                        (*F).fields.op_vreverse.min = GET2(Fecode!(), 1);
                        (*F).fields.op_vreverse.max = GET2(Fecode!(), 1 + IMM2_SIZE);
                        if utf != 0 {
                            i = 0;
                            while i < (*F).fields.op_vreverse.max {
                                if Feptr!() == (*mb).start_subject {
                                    if i < (*F).fields.op_vreverse.min { RRETURN!('machine, MATCH_NOMATCH); }
                                    (*F).fields.op_vreverse.max = i;
                                    break;
                                }
                                Feptr!() = Feptr!().sub(1);
                                Feptr!() = BACKCHAR(Feptr!());
                                i += 1;
                            }
                        } else {
                            let diff = Feptr!() as isize - (*mb).start_subject as isize;
                            let available: u32 = if diff > 65535 { 65535 } else if diff > 0 { diff as u32 } else { 0 };
                            if (*F).fields.op_vreverse.min > available { RRETURN!('machine, MATCH_NOMATCH); }
                            if (*F).fields.op_vreverse.max > available { (*F).fields.op_vreverse.max = available; }
                            Feptr!() = Feptr!().sub((*F).fields.op_vreverse.max as usize);
                        }
                        state = ST_VREVERSE_LOOP;
                        continue 'machine;
                    }
                    OP_ALT => {
                        branch_end = Fecode!();
                        loop {
                            Fecode!() = Fecode!().add(GET(Fecode!(), 1) as usize);
                            if *Fecode!() != OP_ALT { break; }
                        }
                        NEXT_OP!('machine);
                    }
                    OP_KET | OP_KETRMIN | OP_KETRMAX | OP_KETRPOS => {
                        state = ST_KET_ENTRY;
                        continue 'machine;
                    }
                    _ => {
                        state = ST_MAINLOOP10;
                        continue 'machine;
                    }
                }
            }
            ST_VREVERSE_LOOP => {
                RMATCH!('machine, Fecode!().add(1 + 2 * IMM2_SIZE), RM37);
            }
            x if x == RM37 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                let lmax = (*F).fields.op_vreverse.max;
                (*F).fields.op_vreverse.max = lmax.wrapping_sub(1);
                if lmax <= (*F).fields.op_vreverse.min { RRETURN!('machine, MATCH_NOMATCH); }
                Feptr!() = Feptr!().add(1);
                if utf != 0 {
                    while Feptr!() < ES!() && (*Feptr!() & 0xc0) == 0x80 { Feptr!() = Feptr!().add(1); }
                }
                state = ST_VREVERSE_LOOP;
                continue 'machine;
            }

            // ---- OP_KET / KETRMIN / KETRMAX / KETRPOS ----
            ST_KET_ENTRY => {
                bracode = Fecode!().sub(GET(Fecode!(), 1) as usize);

                if branch_end.is_null() { branch_end = Fecode!(); }
                branch_start = bracode;
                while branch_start.add(GET(branch_start, 1) as usize) != branch_end {
                    branch_start = branch_start.add(GET(branch_start, 1) as usize);
                }
                branch_end = ptr::null();

                if *bracode != OP_BRA && *bracode != OP_COND {
                    N = frame_byte_add((*match_data).heapframes as *mut heapframe, Flast_group_offset!());
                    P = frame_byte_sub(N, frame_size);
                    Flast_group_offset!() = (*P).last_group_offset;

                    if (*N).group_frame_type == GF_CONDASSERT {
                        if (*bracode == OP_ASSERTBACK || *bracode == OP_ASSERTBACK_NOT)
                            && *branch_start.add(1 + LINK_SIZE) == OP_VREVERSE
                            && Feptr!() != (*P).eptr {
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        ptr::copy_nonoverlapping(
                            Fovector!() as *const PCRE2_SIZE,
                            (P as *mut u8).add(core::mem::offset_of!(heapframe, ovector)) as *mut PCRE2_SIZE,
                            Foffset_top!(),
                        );
                        (*P).offset_top = Foffset_top!();
                        (*P).mark = Fmark!();
                        Fback_frame!() = (F as usize) - (P as usize);
                        RRETURN!('machine, MATCH_MATCH);
                    }
                } else {
                    P = ptr::null_mut();
                }

                let mut handled_switch = true;
                match *bracode {
                    OP_BRA => {
                        if Fcurrent_recurse!() != 0 || *Fecode!().add(1 + LINK_SIZE) != OP_END {
                            handled_switch = false;
                        } else {
                            offset = Flast_group_offset!();
                            if offset == PCRE2_UNSET { return PCRE2_ERROR_INTERNAL; }
                            N = frame_byte_add((*match_data).heapframes as *mut heapframe, offset);
                            P = frame_byte_sub(N, frame_size);
                            Flast_group_offset!() = (*P).last_group_offset;
                            Fecode!() = (*P).ecode.add(1 + LINK_SIZE);
                            if *Fecode!() != OP_CREF {
                                ptr::copy_nonoverlapping(
                                    fovec(P) as *const PCRE2_SIZE,
                                    Fovector!(),
                                    Foffset_top!(),
                                );
                                Foffset_top!() = (*P).offset_top;
                            } else {
                                recurse_update_offsets(F, P);
                            }
                            Fcapture_last!() = (*P).capture_last;
                            Fcurrent_recurse!() = (*P).current_recurse;
                            state = ST_MAINLOOP;
                            continue 'machine;
                        }
                    }
                    OP_COND | OP_SCOND => {}
                    OP_ASSERTBACK_NA => {
                        if *branch_start.add(1 + LINK_SIZE) == OP_VREVERSE && Feptr!() != (*P).eptr {
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        if Feptr!() > (*mb).last_used_ptr { (*mb).last_used_ptr = Feptr!(); }
                        Feptr!() = (*P).eptr;
                    }
                    OP_ASSERT_NA => {
                        if Feptr!() > (*mb).last_used_ptr { (*mb).last_used_ptr = Feptr!(); }
                        Feptr!() = (*P).eptr;
                    }
                    OP_ASSERTBACK => {
                        if *branch_start.add(1 + LINK_SIZE) == OP_VREVERSE && Feptr!() != (*P).eptr {
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        if Feptr!() > (*mb).last_used_ptr { (*mb).last_used_ptr = Feptr!(); }
                        Feptr!() = (*P).eptr;
                        // fall to ONCE handling
                        Fback_frame!() = (F as usize) - (P as usize);
                        loop {
                            let y = GET((*P).ecode, 1) as usize;
                            if *(*P).ecode.add(y) != OP_ALT { break; }
                            (*P).ecode = (*P).ecode.add(y);
                        }
                    }
                    OP_ASSERT => {
                        if Feptr!() > (*mb).last_used_ptr { (*mb).last_used_ptr = Feptr!(); }
                        Feptr!() = (*P).eptr;
                        Fback_frame!() = (F as usize) - (P as usize);
                        loop {
                            let y = GET((*P).ecode, 1) as usize;
                            if *(*P).ecode.add(y) != OP_ALT { break; }
                            (*P).ecode = (*P).ecode.add(y);
                        }
                    }
                    OP_ONCE => {
                        Fback_frame!() = (F as usize) - (P as usize);
                        loop {
                            let y = GET((*P).ecode, 1) as usize;
                            if *(*P).ecode.add(y) != OP_ALT { break; }
                            (*P).ecode = (*P).ecode.add(y);
                        }
                    }
                    OP_ASSERTBACK_NOT => {
                        if *branch_start.add(1 + LINK_SIZE) == OP_VREVERSE && Feptr!() != (*P).eptr {
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        RRETURN!('machine, MATCH_MATCH);
                    }
                    OP_ASSERT_NOT => {
                        RRETURN!('machine, MATCH_MATCH);
                    }
                    OP_ASSERT_SCS => {
                        (*F).fields.op_assert_scs.saved_end_subject = (*mb).end_subject;
                        (*mb).end_subject = (*P).fields.op_assert_scs.saved_end_subject;
                        (*mb).true_end_subject = (*mb).end_subject.add((*P).fields.op_assert_scs.true_end_extra);
                        Feptr!() = (*P).fields.op_assert_scs.saved_eptr;
                        RMATCH!('machine, Fecode!().add(1 + LINK_SIZE), RM39);
                    }
                    OP_SCRIPT_RUN => {
                        if _pcre2_script_run_8((*P).eptr, Feptr!(), utf) == 0 { RRETURN!('machine, MATCH_NOMATCH); }
                    }
                    OP_CBRA | OP_CBRAPOS | OP_SCBRA | OP_SCBRAPOS => {
                        number = GET2(bracode, 1 + LINK_SIZE);
                        if Fcurrent_recurse!() == number {
                            P = frame_byte_sub(N, frame_size);
                            Fecode!() = (*P).ecode.add(1 + LINK_SIZE);
                            if *Fecode!() != OP_CREF {
                                ptr::copy_nonoverlapping(
                                    fovec(P) as *const PCRE2_SIZE,
                                    Fovector!(),
                                    Foffset_top!(),
                                );
                                Foffset_top!() = (*P).offset_top;
                            } else {
                                recurse_update_offsets(F, P);
                            }
                            Fcapture_last!() = (*P).capture_last;
                            Fcurrent_recurse!() = (*P).current_recurse;
                            state = ST_MAINLOOP;
                            continue 'machine;
                        }
                        offset = ((number as usize) << 1) - 2;
                        Fcapture_last!() = number;
                        *Fovector!().add(offset) = (*P).eptr as usize - (*mb).start_subject as usize;
                        *Fovector!().add(offset + 1) = Feptr!() as usize - (*mb).start_subject as usize;
                        if offset >= Foffset_top!() { Foffset_top!() = offset + 2; }
                    }
                    _ => {}
                }
                let _ = handled_switch;

                // KETRPOS
                if *Fecode!() == OP_KETRPOS {
                    ptr::copy_nonoverlapping(
                        (F as *const u8).add(core::mem::offset_of!(heapframe, eptr)),
                        (P as *mut u8).add(core::mem::offset_of!(heapframe, eptr)),
                        frame_copy_size,
                    );
                    RRETURN!('machine, MATCH_KETRPOS);
                }

                if Fop!() != OP_KET && (P.is_null() || Feptr!() != (*P).eptr) {
                    if Fop!() == OP_KETRMIN {
                        RMATCH!('machine, Fecode!().add(1 + LINK_SIZE), RM6);
                    }
                    RMATCH!('machine, bracode, RM7);
                }

                Fecode!() = Fecode!().add(1 + LINK_SIZE);
                NEXT_OP!('machine);
            }
            x if x == RM6 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                Fecode!() = Fecode!().sub(GET(Fecode!(), 1) as usize);
                // break: end of ket processing, continue main loop at bracode.
                NEXT_OP!('machine);
            }
            x if x == RM7 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                Fecode!() = Fecode!().add(1 + LINK_SIZE);
                NEXT_OP!('machine);
            }
            x if x == RM39 as i32 => {
                (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
                (*mb).true_end_subject = (*mb).end_subject;
                RRETURN!('machine, rrc);
            }

            // ===== ST_MAINLOOP10: anchors, word boundary, verbs =====
            ST_MAINLOOP10 => {
                match Fop!() {
                    OP_CIRC => {
                        if Feptr!() != (*mb).start_subject || ((*mb).moptions & PCRE2_NOTBOL) != 0 {
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }
                    OP_SOD => {
                        if Feptr!() != (*mb).start_subject { RRETURN!('machine, MATCH_NOMATCH); }
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }
                    OP_DOLL => {
                        if ((*mb).moptions & PCRE2_NOTEOL) != 0 { RRETURN!('machine, MATCH_NOMATCH); }
                        if ((*mb).poptions & PCRE2_DOLLAR_ENDONLY) == 0 {
                            state = ST_ASSERT_NL_OR_EOS;
                            continue 'machine;
                        }
                        // fall through to EOD
                        if Feptr!() < (*mb).true_end_subject { RRETURN!('machine, MATCH_NOMATCH); }
                        if (*mb).partial != 0 {
                            (*mb).hitend = TRUE;
                            if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL; }
                        }
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }
                    OP_EOD => {
                        if Feptr!() < (*mb).true_end_subject { RRETURN!('machine, MATCH_NOMATCH); }
                        if (*mb).partial != 0 {
                            (*mb).hitend = TRUE;
                            if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL; }
                        }
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }
                    OP_EODN => {
                        state = ST_ASSERT_NL_OR_EOS;
                        continue 'machine;
                    }
                    OP_CIRCM => {
                        if ((*mb).moptions & PCRE2_NOTBOL) != 0 && Feptr!() == (*mb).start_subject {
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        if Feptr!() != (*mb).start_subject
                            && ((Feptr!() == (*mb).end_subject
                                    && ((*mb).poptions & PCRE2_ALT_CIRCUMFLEX) == 0)
                                || !was_newline(Feptr!(), mb, utf)) {
                            RRETURN!('machine, MATCH_NOMATCH);
                        }
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }
                    OP_DOLLM => {
                        if Feptr!() < (*mb).end_subject {
                            if !is_newline(Feptr!(), mb, utf) {
                                if (*mb).partial != 0 && Feptr!().add(1) >= (*mb).end_subject
                                    && (*mb).nltype == NLTYPE_FIXED && (*mb).nllen == 2
                                    && *Feptr!() == (*mb).nl[0] {
                                    (*mb).hitend = TRUE;
                                    if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL; }
                                }
                                RRETURN!('machine, MATCH_NOMATCH);
                            }
                        } else {
                            if ((*mb).moptions & PCRE2_NOTEOL) != 0 { RRETURN!('machine, MATCH_NOMATCH); }
                            SCHECK_PARTIAL!();
                        }
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }
                    OP_SOM => {
                        if Feptr!() != (*mb).start_subject.add((*mb).start_offset) { RRETURN!('machine, MATCH_NOMATCH); }
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }
                    OP_SET_SOM => {
                        Fstart_match!() = Feptr!();
                        Fecode!() = Fecode!().add(1);
                        NEXT_OP!('machine);
                    }
                    OP_NOT_WORD_BOUNDARY | OP_WORD_BOUNDARY
                    | OP_NOT_UCP_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY => {
                        state = ST_WORD_BOUNDARY;
                        continue 'machine;
                    }
                    _ => {
                        state = ST_MAINLOOP11;
                        continue 'machine;
                    }
                }
            }

            ST_ASSERT_NL_OR_EOS => {
                if Feptr!() < (*mb).true_end_subject
                    && (!is_newline(Feptr!(), mb, utf)
                        || Feptr!() != (*mb).true_end_subject.sub((*mb).nllen as usize)) {
                    if (*mb).partial != 0 && Feptr!().add(1) >= (*mb).end_subject
                        && (*mb).nltype == NLTYPE_FIXED && (*mb).nllen == 2
                        && *Feptr!() == (*mb).nl[0] {
                        (*mb).hitend = TRUE;
                        if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL; }
                    }
                    RRETURN!('machine, MATCH_NOMATCH);
                }
                if (*mb).partial != 0 {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL; }
                }
                Fecode!() = Fecode!().add(1);
                NEXT_OP!('machine);
            }

            ST_WORD_BOUNDARY => {
                if Feptr!() == (*mb).check_subject {
                    prev_is_word = false;
                } else {
                    let mut lastptr = Feptr!().sub(1);
                    if utf != 0 {
                        lastptr = BACKCHAR(lastptr);
                        fc = GETCHAR(lastptr);
                    } else {
                        fc = *lastptr as u32;
                    }
                    if lastptr < (*mb).start_used_ptr { (*mb).start_used_ptr = lastptr; }
                    if Fop!() == OP_UCP_WORD_BOUNDARY || Fop!() == OP_NOT_UCP_WORD_BOUNDARY {
                        let chartype = UCD_CHARTYPE(fc);
                        let category = _pcre2_ucp_gentype_8[chartype as usize];
                        prev_is_word = category == ucp_L || category == ucp_N
                            || chartype == ucp_Mn || chartype == ucp_Pc;
                    } else {
                        prev_is_word = fc <= 255 && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0;
                    }
                }

                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    cur_is_word = false;
                } else {
                    let mut nextptr = Feptr!().add(1);
                    if utf != 0 {
                        while nextptr < (*mb).end_subject && (*nextptr & 0xc0) == 0x80 { nextptr = nextptr.add(1); }
                        fc = GETCHAR(Feptr!());
                    } else {
                        fc = *Feptr!() as u32;
                    }
                    if nextptr > (*mb).last_used_ptr { (*mb).last_used_ptr = nextptr; }
                    if Fop!() == OP_UCP_WORD_BOUNDARY || Fop!() == OP_NOT_UCP_WORD_BOUNDARY {
                        let chartype = UCD_CHARTYPE(fc);
                        let category = _pcre2_ucp_gentype_8[chartype as usize];
                        cur_is_word = category == ucp_L || category == ucp_N
                            || chartype == ucp_Mn || chartype == ucp_Pc;
                    } else {
                        cur_is_word = fc <= 255 && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0;
                    }
                }

                let op0 = *Fecode!();
                Fecode!() = Fecode!().add(1);
                let want_equal = op0 == OP_WORD_BOUNDARY || Fop!() == OP_UCP_WORD_BOUNDARY;
                let fail = if want_equal { cur_is_word == prev_is_word } else { cur_is_word != prev_is_word };
                if fail { RRETURN!('machine, MATCH_NOMATCH); }
                NEXT_OP!('machine);
            }

            // ===== ST_MAINLOOP11: backtracking verbs =====
            ST_MAINLOOP11 => {
                match Fop!() {
                    OP_MARK => {
                        Fmark!() = Fecode!().add(2);
                        (*mb).nomatch_mark = Fecode!().add(2);
                        RMATCH!('machine, Fecode!().add(op_length(*Fecode!()) + *Fecode!().add(1) as usize), RM12);
                    }
                    OP_FAIL => {
                        RRETURN!('machine, MATCH_NOMATCH);
                    }
                    OP_COMMIT => {
                        RMATCH!('machine, Fecode!().add(op_length(*Fecode!())), RM13);
                    }
                    OP_COMMIT_ARG => {
                        Fmark!() = Fecode!().add(2);
                        (*mb).nomatch_mark = Fecode!().add(2);
                        RMATCH!('machine, Fecode!().add(op_length(*Fecode!()) + *Fecode!().add(1) as usize), RM36);
                    }
                    OP_PRUNE => {
                        RMATCH!('machine, Fecode!().add(op_length(*Fecode!())), RM14);
                    }
                    OP_PRUNE_ARG => {
                        Fmark!() = Fecode!().add(2);
                        (*mb).nomatch_mark = Fecode!().add(2);
                        RMATCH!('machine, Fecode!().add(op_length(*Fecode!()) + *Fecode!().add(1) as usize), RM15);
                    }
                    OP_SKIP => {
                        RMATCH!('machine, Fecode!().add(op_length(*Fecode!())), RM16);
                    }
                    OP_SKIP_ARG => {
                        (*mb).skip_arg_count += 1;
                        if (*mb).skip_arg_count <= (*mb).ignore_skip_arg {
                            Fecode!() = Fecode!().add(op_length(*Fecode!()) + *Fecode!().add(1) as usize);
                            NEXT_OP!('machine);
                        }
                        RMATCH!('machine, Fecode!().add(op_length(*Fecode!()) + *Fecode!().add(1) as usize), RM17);
                    }
                    OP_THEN => {
                        RMATCH!('machine, Fecode!().add(op_length(*Fecode!())), RM18);
                    }
                    OP_THEN_ARG => {
                        Fmark!() = Fecode!().add(2);
                        (*mb).nomatch_mark = Fecode!().add(2);
                        RMATCH!('machine, Fecode!().add(op_length(*Fecode!()) + *Fecode!().add(1) as usize), RM19);
                    }
                    _ => {
                        return PCRE2_ERROR_INTERNAL;
                    }
                }
            }
            x if x == RM12 as i32 => {
                if rrc == MATCH_SKIP_ARG && _pcre2_strcmp_8(Fecode!().add(2), (*mb).verb_skip_ptr) == 0 {
                    (*mb).verb_skip_ptr = Feptr!();
                    RRETURN!('machine, MATCH_SKIP);
                }
                RRETURN!('machine, rrc);
            }
            x if x == RM13 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                (*mb).verb_current_recurse = Fcurrent_recurse!();
                RRETURN!('machine, MATCH_COMMIT);
            }
            x if x == RM36 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                (*mb).verb_current_recurse = Fcurrent_recurse!();
                RRETURN!('machine, MATCH_COMMIT);
            }
            x if x == RM14 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                (*mb).verb_current_recurse = Fcurrent_recurse!();
                RRETURN!('machine, MATCH_PRUNE);
            }
            x if x == RM15 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                (*mb).verb_current_recurse = Fcurrent_recurse!();
                RRETURN!('machine, MATCH_PRUNE);
            }
            x if x == RM16 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                (*mb).verb_skip_ptr = Feptr!();
                (*mb).verb_current_recurse = Fcurrent_recurse!();
                RRETURN!('machine, MATCH_SKIP);
            }
            x if x == RM17 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                (*mb).verb_skip_ptr = Fecode!().add(2);
                (*mb).verb_current_recurse = Fcurrent_recurse!();
                RRETURN!('machine, MATCH_SKIP_ARG);
            }
            x if x == RM18 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                (*mb).verb_ecode_ptr = Fecode!();
                (*mb).verb_current_recurse = Fcurrent_recurse!();
                RRETURN!('machine, MATCH_THEN);
            }
            x if x == RM19 as i32 => {
                if rrc != MATCH_NOMATCH { RRETURN!('machine, rrc); }
                (*mb).verb_ecode_ptr = Fecode!();
                (*mb).verb_current_recurse = Fcurrent_recurse!();
                RRETURN!('machine, MATCH_THEN);
            }

            _ => {
                return PCRE2_ERROR_INTERNAL;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// pcre2_match_8
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_match_8(
    code: *const pcre2_code,
    subject: PCRE2_SPTR,
    length_arg: PCRE2_SIZE,
    start_offset: PCRE2_SIZE,
    options: u32,
    match_data: *mut pcre2_match_data,
    mcontext_arg: *mut pcre2_match_context,
) -> c_int {
    let mut rc: c_int;
    let mut start_bits: *const u8 = ptr::null();
    let re = code as *const pcre2_real_code;
    let original_options = options;
    let mut length = length_arg;
    let mut subject = subject;
    let mut mcontext = mcontext_arg;

    let anchored: bool;
    let firstline: bool;
    let mut has_first_cu = false;
    let mut has_req_cu = false;
    let startline: bool;

    let mut memchr_found_first_cu: PCRE2_SPTR = ptr::null();
    let mut memchr_found_first_cu2: PCRE2_SPTR = ptr::null();

    let mut first_cu: u8 = 0;
    let mut first_cu2: u8 = 0;
    let mut req_cu: u8 = 0;
    let mut req_cu2: u8 = 0;

    let null_str: [u8; 1] = [0xcd];
    let original_subject = subject;
    let bumpalong_limit: PCRE2_SPTR;
    let mut end_subject: PCRE2_SPTR;
    let true_end_subject: PCRE2_SPTR;
    let mut start_match: PCRE2_SPTR;
    let mut req_cu_ptr: PCRE2_SPTR;
    let mut start_partial: PCRE2_SPTR;
    let mut match_partial: PCRE2_SPTR;

    let utf: bool;
    let ucp: bool;
    let allow_invalid: bool;
    let mut fragment_options: u32 = 0;

    let frame_size: PCRE2_SIZE;
    let mut heapframes_size: PCRE2_SIZE;

    let mut cb: pcre2_callout_block = core::mem::zeroed();
    let mut actual_match_block: match_block = core::mem::zeroed();
    let mb: *mut match_block = &mut actual_match_block;

    if subject.is_null() && length == 0 {
        subject = null_str.as_ptr();
    }

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
    req_cu_ptr = start_match.wrapping_sub(1);
    if length == PCRE2_ZERO_TERMINATED {
        length = _pcre2_strlen_8(subject);
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

    // Transfer (*NOTEMPTY)/(*NOTEMPTY_ATSTART) pattern flags to options.
    const FF: u32 = PCRE2_NOTEMPTY_SET | PCRE2_NE_ATST_SET;
    const OO: u32 = PCRE2_NOTEMPTY | PCRE2_NOTEMPTY_ATSTART;
    let mut options = options;
    options |= ((*re).flags & FF) / ((FF & FF.wrapping_neg()) / (OO & OO.wrapping_neg()));

    utf = ((*re).overall_options & PCRE2_UTF) != 0;
    allow_invalid = ((*re).overall_options & PCRE2_MATCH_INVALID_UTF) != 0;
    ucp = ((*re).overall_options & PCRE2_UCP) != 0;
    let _ = ucp;

    (*mb).partial = if (options & PCRE2_PARTIAL_HARD) != 0 {
        2
    } else if (options & PCRE2_PARTIAL_SOFT) != 0 {
        1
    } else {
        0
    };

    if (*mb).partial != 0 && (((*re).overall_options | options) & PCRE2_ENDANCHORED) != 0 {
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
    (*match_data).subject = ptr::null();
    (*match_data).startchar = 0;

    // No JIT. Proceed with interpreter matching.
    (*mb).check_subject = subject;

    // UTF validity check.
    if utf && (((options & PCRE2_NO_UTF_CHECK) == 0) || allow_invalid) {
        let mut skipped_bad_start = false;

        if allow_invalid {
            while start_match < end_subject && NOT_FIRSTCU(*start_match as u32) {
                start_match = start_match.add(1);
                skipped_bad_start = true;
            }
        } else if start_match < end_subject && NOT_FIRSTCU(*start_match as u32) {
            if start_offset > 0 {
                (*match_data).rc = PCRE2_ERROR_BADUTFOFFSET;
                return PCRE2_ERROR_BADUTFOFFSET;
            }
            (*match_data).rc = PCRE2_ERROR_UTF8_ERR20;
            return PCRE2_ERROR_UTF8_ERR20;
        }

        (*mb).check_subject = start_match;

        if !skipped_bad_start {
            let mut ii = (*re).max_lookbehind;
            while ii > 0 && (*mb).check_subject > subject {
                (*mb).check_subject = (*mb).check_subject.sub(1);
                while (*mb).check_subject > subject && (*(*mb).check_subject & 0xc0) == 0x80 {
                    (*mb).check_subject = (*mb).check_subject.sub(1);
                }
                ii -= 1;
            }
        }

        loop {
            rc = _pcre2_valid_utf_8(
                (*mb).check_subject,
                length - ((*mb).check_subject as usize - subject as usize),
                ptr::addr_of_mut!((*match_data).startchar),
            );
            if rc == 0 {
                break;
            }
            (*match_data).startchar += (*mb).check_subject as usize - subject as usize;
            if !allow_invalid || rc > 0 {
                (*match_data).rc = rc;
                return rc;
            }
            end_subject = subject.add((*match_data).startchar);

            if end_subject < start_match {
                (*mb).check_subject = end_subject.add(1);
                while (*mb).check_subject < start_match && NOT_FIRSTCU(*(*mb).check_subject as u32) {
                    (*mb).check_subject = (*mb).check_subject.add(1);
                }
                end_subject = true_end_subject;
            } else {
                fragment_options = PCRE2_NOTEOL;
                break;
            }
        }
    }

    if mcontext.is_null() {
        mcontext = ptr::addr_of_mut!(_pcre2_default_match_context_8);
        (*mb).memctl = (*re).memctl;
    } else {
        (*mb).memctl = (*mcontext).memctl;
    }

    anchored = (((*re).overall_options | options) & PCRE2_ANCHORED) != 0;
    firstline = !anchored && ((*re).overall_options & PCRE2_FIRSTLINE) != 0;
    startline = ((*re).flags & PCRE2_STARTLINE) != 0;
    bumpalong_limit = if (*mcontext).offset_limit == PCRE2_UNSET {
        true_end_subject
    } else {
        subject.add((*mcontext).offset_limit)
    };

    (*mb).cb = &mut cb;
    cb.version = 2;
    cb.subject = subject;
    cb.subject_length = end_subject as usize - subject as usize;
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
    (*mb).mark = ptr::null();
    (*mb).nomatch_mark = ptr::null();

    (*mb).name_table = (re as *const u8).add(core::mem::size_of::<pcre2_real_code>());
    (*mb).name_count = (*re).name_count;
    (*mb).name_entry_size = (*re).name_entry_size;
    (*mb).start_code = (re as *const u8).add((*re).code_start);

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

    let hf_align = core::mem::align_of::<heapframe>();
    frame_size = (core::mem::offset_of!(heapframe, ovector)
        + (*re).top_bracket as usize * 2 * core::mem::size_of::<PCRE2_SIZE>()
        + hf_align
        - 1)
        & !(hf_align - 1);

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
    if heapframes_size / 1024 > (*mb).heap_limit as usize {
        let max_size = 1024 * (*mb).heap_limit as usize;
        if max_size < frame_size {
            (*match_data).rc = PCRE2_ERROR_HEAPLIMIT;
            return PCRE2_ERROR_HEAPLIMIT;
        }
        heapframes_size = max_size;
    }

    if (*match_data).heapframes_size < heapframes_size {
        ((*match_data).memctl.free.unwrap())(
            (*match_data).heapframes,
            (*match_data).memctl.memory_data,
        );
        (*match_data).heapframes = ((*match_data).memctl.malloc.unwrap())(
            heapframes_size,
            (*match_data).memctl.memory_data,
        );
        if (*match_data).heapframes.is_null() {
            (*match_data).heapframes_size = 0;
            (*match_data).rc = PCRE2_ERROR_NOMEMORY;
            return PCRE2_ERROR_NOMEMORY;
        }
        (*match_data).heapframes_size = heapframes_size;
    }

    memset(
        ((*match_data).heapframes as *mut u8).add(core::mem::offset_of!(heapframe, ovector)) as *mut c_void,
        0xff,
        frame_size - core::mem::offset_of!(heapframe, ovector),
    );

    (*mb).lcc = (*re).tables.add(lcc_offset);
    (*mb).fcc = (*re).tables.add(fcc_offset);
    (*mb).ctypes = (*re).tables.add(ctypes_offset);

    if ((*re).flags & PCRE2_FIRSTSET) != 0 {
        has_first_cu = true;
        first_cu = (*re).first_codeunit as u8;
        first_cu2 = first_cu;
        if ((*re).flags & PCRE2_FIRSTCASELESS) != 0 {
            first_cu2 = *(*mb).fcc.add(first_cu as usize);
            if first_cu > 127 && ucp && !utf {
                first_cu2 = UCD_OTHERCASE(first_cu as u32) as u8;
            }
        }
    } else if !startline && ((*re).flags & PCRE2_FIRSTMAPSET) != 0 {
        start_bits = (*re).start_bitmap.as_ptr();
    }

    if ((*re).flags & PCRE2_LASTSET) != 0 {
        has_req_cu = true;
        req_cu = (*re).last_codeunit as u8;
        req_cu2 = req_cu;
        if ((*re).flags & PCRE2_LASTCASELESS) != 0 {
            req_cu2 = *(*mb).fcc.add(req_cu as usize);
            if req_cu > 127 && ucp && !utf {
                req_cu2 = UCD_OTHERCASE(req_cu as u32) as u8;
            }
        }
    }


    // ===== bumpalong / fragment-restart / endloop =====
    // `rc` holds the final match() result once we leave the bumpalong loop.
    'fragment_restart: loop {
        start_partial = ptr::null();
        match_partial = ptr::null();
        (*mb).hitend = FALSE;
        memchr_found_first_cu = ptr::null();
        memchr_found_first_cu2 = ptr::null();

        rc = MATCH_NOMATCH;
        let mut ended = false; // true => go directly to ENDLOOP handling

        'bumpalong: loop {
            let mut new_start_match: PCRE2_SPTR = ptr::null();

            if ((*re).optimization_flags & PCRE2_OPTIM_START_OPTIMIZE) != 0 {
                if firstline {
                    let mut t = start_match;
                    if utf {
                        while t < end_subject && !is_newline(t, mb, utf as BOOL) {
                            t = t.add(1);
                            while t < end_subject && (*t & 0xc0) == 0x80 {
                                t = t.add(1);
                            }
                        }
                    } else {
                        while t < end_subject && !is_newline(t, mb, utf as BOOL) {
                            t = t.add(1);
                        }
                    }
                    end_subject = t;
                }

                if anchored {
                    if has_first_cu || !start_bits.is_null() {
                        let mut ok = start_match < end_subject;
                        if ok {
                            let c = *start_match;
                            ok = has_first_cu && (c == first_cu || c == first_cu2);
                            if !ok && !start_bits.is_null() {
                                let cc = c;
                                ok = (*start_bits.add((cc / 8) as usize) & (1u8 << (cc & 7))) != 0;
                            }
                        }
                        if !ok {
                            rc = MATCH_NOMATCH;
                            break 'bumpalong;
                        }
                    }
                } else if has_first_cu {
                    if first_cu != first_cu2 {
                        let searchlength = end_subject as usize - start_match as usize;
                        let pp1: PCRE2_SPTR;
                        let pp2: PCRE2_SPTR;

                        if memchr_found_first_cu.is_null() || start_match > memchr_found_first_cu {
                            let found =
                                memchr(start_match as *const c_void, first_cu as c_int, searchlength);
                            pp1 = found as PCRE2_SPTR;
                            memchr_found_first_cu = if pp1.is_null() { end_subject } else { pp1 };
                        } else {
                            pp1 = if memchr_found_first_cu == end_subject {
                                ptr::null()
                            } else {
                                memchr_found_first_cu
                            };
                        }

                        if memchr_found_first_cu2.is_null() || start_match > memchr_found_first_cu2 {
                            let found =
                                memchr(start_match as *const c_void, first_cu2 as c_int, searchlength);
                            pp2 = found as PCRE2_SPTR;
                            memchr_found_first_cu2 = if pp2.is_null() { end_subject } else { pp2 };
                        } else {
                            pp2 = if memchr_found_first_cu2 == end_subject {
                                ptr::null()
                            } else {
                                memchr_found_first_cu2
                            };
                        }

                        if pp1.is_null() {
                            start_match = if pp2.is_null() { end_subject } else { pp2 };
                        } else {
                            start_match = if pp2.is_null() || pp1 < pp2 { pp1 } else { pp2 };
                        }
                    } else {
                        let found = memchr(
                            start_match as *const c_void,
                            first_cu as c_int,
                            end_subject as usize - start_match as usize,
                        );
                        start_match = found as PCRE2_SPTR;
                        if start_match.is_null() {
                            start_match = end_subject;
                        }
                    }

                    if (*mb).partial == 0 && start_match >= (*mb).end_subject {
                        rc = MATCH_NOMATCH;
                        break 'bumpalong;
                    }
                } else if startline {
                    if start_match > (*mb).start_subject.add(start_offset) {
                        if utf {
                            while start_match < end_subject && !was_newline(start_match, mb, utf as BOOL) {
                                start_match = start_match.add(1);
                                while start_match < end_subject && (*start_match & 0xc0) == 0x80 {
                                    start_match = start_match.add(1);
                                }
                            }
                        } else {
                            while start_match < end_subject && !was_newline(start_match, mb, utf as BOOL) {
                                start_match = start_match.add(1);
                            }
                        }

                        if *start_match.sub(1) as u32 == CHAR_CR
                            && ((*mb).nltype == NLTYPE_ANY || (*mb).nltype == NLTYPE_ANYCRLF)
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
                        rc = MATCH_NOMATCH;
                        break 'bumpalong;
                    }
                }

                end_subject = (*mb).end_subject;

                if (*mb).partial == 0 {
                    if (end_subject as usize - start_match as usize) < (*re).minlength as usize {
                        rc = MATCH_NOMATCH;
                        break 'bumpalong;
                    }

                    let mut p = start_match.add(if has_first_cu { 1 } else { 0 });
                    if has_req_cu && p > req_cu_ptr {
                        let check_length = end_subject as usize - start_match as usize;
                        if check_length < REQ_CU_MAX as usize
                            || (!anchored && check_length < REQ_CU_MAX as usize * 1000)
                        {
                            if req_cu != req_cu2 {
                                let pp = p;
                                let found = memchr(
                                    pp as *const c_void,
                                    req_cu as c_int,
                                    end_subject as usize - pp as usize,
                                );
                                p = found as PCRE2_SPTR;
                                if p.is_null() {
                                    let found2 = memchr(
                                        pp as *const c_void,
                                        req_cu2 as c_int,
                                        end_subject as usize - pp as usize,
                                    );
                                    p = found2 as PCRE2_SPTR;
                                    if p.is_null() {
                                        p = end_subject;
                                    }
                                }
                            } else {
                                let found = memchr(
                                    p as *const c_void,
                                    req_cu as c_int,
                                    end_subject as usize - p as usize,
                                );
                                p = found as PCRE2_SPTR;
                                if p.is_null() {
                                    p = end_subject;
                                }
                            }

                            if p >= end_subject {
                                rc = MATCH_NOMATCH;
                                break 'bumpalong;
                            }
                            req_cu_ptr = p;
                        }
                    }
                }
            }

            if start_match > bumpalong_limit {
                rc = MATCH_NOMATCH;
                break 'bumpalong;
            }

            cb.start_match = start_match as usize - subject as usize;
            cb.callout_flags |= PCRE2_CALLOUT_STARTMATCH;

            (*mb).start_used_ptr = start_match;
            (*mb).last_used_ptr = start_match;
            (*mb).moptions = options | fragment_options;
            (*mb).match_call_count = 0;
            (*mb).end_offset_top = 0;
            (*mb).skip_arg_count = 0;

            rc = r#match(
                start_match,
                (*mb).start_code,
                (*re).top_bracket,
                frame_size,
                match_data,
                mb,
            );

            if (*mb).hitend != 0 && start_partial.is_null() {
                start_partial = (*mb).start_used_ptr;
                match_partial = start_match;
            }

            if rc == MATCH_SKIP_ARG {
                new_start_match = start_match;
                (*mb).ignore_skip_arg = (*mb).skip_arg_count;
            } else if rc == MATCH_SKIP && (*mb).verb_skip_ptr > start_match {
                new_start_match = (*mb).verb_skip_ptr;
            } else if rc == MATCH_SKIP || rc == MATCH_NOMATCH || rc == MATCH_PRUNE || rc == MATCH_THEN {
                (*mb).ignore_skip_arg = 0;
                new_start_match = start_match.add(1);
                if utf {
                    while new_start_match < end_subject && (*new_start_match & 0xc0) == 0x80 {
                        new_start_match = new_start_match.add(1);
                    }
                }
            } else if rc == MATCH_COMMIT {
                rc = MATCH_NOMATCH;
                ended = true;
                break 'bumpalong;
            } else {
                ended = true;
                break 'bumpalong;
            }

            rc = MATCH_NOMATCH;

            if firstline && is_newline(start_match, mb, utf as BOOL) {
                break 'bumpalong;
            }

            start_match = new_start_match;

            if anchored || start_match > end_subject {
                break 'bumpalong;
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

            (*mb).mark = ptr::null();
        } // end 'bumpalong

        let _ = ended;

        // ===== ENDLOOP: handle invalid-UTF fragments. =====
        if utf && end_subject != true_end_subject && (rc == MATCH_NOMATCH || rc == PCRE2_ERROR_PARTIAL) {
            let mut restart = false;
            loop {
                start_match = end_subject.add(1);
                while start_match < true_end_subject && NOT_FIRSTCU(*start_match as u32) {
                    start_match = start_match.add(1);
                }

                if start_match >= true_end_subject {
                    rc = MATCH_NOMATCH;
                    match_partial = ptr::null();
                    break;
                }

                (*mb).check_subject = start_match;
                rc = _pcre2_valid_utf_8(
                    start_match,
                    length - (start_match as usize - subject as usize),
                    ptr::addr_of_mut!((*match_data).startchar),
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

    // ===== Fill in fields always returned. =====
    (*match_data).code = re;
    (*match_data).mark = (*mb).mark;
    (*match_data).matchedby = PCRE2_MATCHEDBY_INTERPRETER;
    (*match_data).options = original_options;

    if rc == MATCH_MATCH {
        (*match_data).rc = if (*mb).end_offset_top as usize >= 2 * (*match_data).oveccount as usize {
            0
        } else {
            (*mb).end_offset_top as c_int / 2 + 1
        };
        (*match_data).subject_length = length;
        (*match_data).start_offset = start_offset;
        (*match_data).startchar = start_match as usize - subject as usize;
        (*match_data).leftchar = (*mb).start_used_ptr as usize - subject as usize;
        (*match_data).rightchar = (if (*mb).last_used_ptr > (*mb).end_match_ptr {
            (*mb).last_used_ptr
        } else {
            (*mb).end_match_ptr
        }) as usize
            - subject as usize;
        if (options & PCRE2_COPY_MATCHED_SUBJECT) != 0 {
            if length != 0 {
                (*match_data).subject = ((*match_data).memctl.malloc.unwrap())(
                    length,
                    (*match_data).memctl.memory_data,
                ) as PCRE2_SPTR;
                if (*match_data).subject.is_null() {
                    (*match_data).rc = PCRE2_ERROR_NOMEMORY;
                    return PCRE2_ERROR_NOMEMORY;
                }
                memcpy(
                    (*match_data).subject as *mut c_void,
                    subject as *const c_void,
                    length,
                );
            } else {
                (*match_data).subject = ptr::null();
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
        *(*match_data).ovector.as_mut_ptr().add(0) = match_partial as usize - subject as usize;
        *(*match_data).ovector.as_mut_ptr().add(1) = end_subject as usize - subject as usize;
        (*match_data).startchar = match_partial as usize - subject as usize;
        (*match_data).leftchar = start_partial as usize - subject as usize;
        (*match_data).rightchar = end_subject as usize - subject as usize;
        (*match_data).rc = PCRE2_ERROR_PARTIAL;
    } else {
        (*match_data).subject = original_subject;
        (*match_data).subject_length = length;
        (*match_data).start_offset = start_offset;
        (*match_data).rc = PCRE2_ERROR_NOMATCH;
    }

    (*match_data).rc
}

// ---------------------------------------------------------------------------
// pcre2_next_match_8
// ---------------------------------------------------------------------------

unsafe fn do_bumpalong(match_data: *mut pcre2_match_data, offset: PCRE2_SIZE) -> PCRE2_SIZE {
    let subject = (*match_data).subject;
    let subject_length = (*match_data).subject_length;
    let utf = ((*(*match_data).code).overall_options & PCRE2_UTF) != 0;

    if *subject.add(offset) as u32 == CHAR_CR
        && offset + 1 < subject_length
        && *subject.add(offset + 1) as u32 == CHAR_LF
    {
        match (*(*match_data).code).newline_convention as u32 {
            PCRE2_NEWLINE_CRLF | PCRE2_NEWLINE_ANY | PCRE2_NEWLINE_ANYCRLF => {
                return offset + 2;
            }
            _ => {}
        }
    }

    if utf {
        let mut next = subject.add(offset + 1);
        let subject_end = subject.add(subject_length);
        while next < subject_end && (*next & 0xc0) == 0x80 {
            next = next.add(1);
        }
        return next as usize - subject as usize;
    }

    offset + 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_next_match_8(
    match_data: *mut pcre2_match_data,
    pstart_offset: *mut PCRE2_SIZE,
    poptions: *mut u32,
) -> c_int {
    let rc = (*match_data).rc;
    let start_offset = (*match_data).start_offset;
    let ovector = (*match_data).ovector.as_ptr();

    if rc < 0 {
        return FALSE;
    }

    let ov0 = *ovector.add(0);
    let ov1 = *ovector.add(1);

    if ov0 != start_offset && ov1 == start_offset {
        if start_offset >= (*match_data).subject_length {
            return FALSE;
        }
        *pstart_offset = do_bumpalong(match_data, ov1);
        *poptions = 0;
        return TRUE;
    }

    if ov0 == ov1 {
        if ov0 >= (*match_data).subject_length {
            return FALSE;
        }
        *pstart_offset = ov1;
        *poptions = PCRE2_NOTEMPTY_ATSTART;
        return TRUE;
    }

    *pstart_offset = ov1;
    *poptions = 0;
    TRUE
}
