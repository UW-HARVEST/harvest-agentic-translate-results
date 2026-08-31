//! Translated from pcre2_match.c: the match() function (C lines 660-6945).
//!
//! The C function uses computed gotos (the RMATCH/RRETURN macros plus a handful
//! of ordinary labels) to implement backtracking without recursion. This is
//! reproduced here as an explicit state machine: `state` holds the label we are
//! "at", and every C `goto` becomes `state = <label>; continue 'sm;`.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::macros::*;
use crate::matcher::*;
use crate::types::*;
use core::ffi::{c_char, c_void};
use core::ptr::{copy, copy_nonoverlapping, null, null_mut, write_bytes};

/* ---- Values and tables from the head of pcre2_match.c ---- */

pub const RECURSE_UNSET: u32 = 0xffffffffu32;

pub const MATCH_MATCH: i32 = 1;
pub const MATCH_NOMATCH: i32 = 0;

pub const MATCH_ACCEPT: i32 = -999;
pub const MATCH_KETRPOS: i32 = -998;
pub const MATCH_COMMIT: i32 = -997;
pub const MATCH_PRUNE: i32 = -996;
pub const MATCH_SKIP: i32 = -995;
pub const MATCH_SKIP_ARG: i32 = -994;
pub const MATCH_THEN: i32 = -993;
pub const MATCH_BACKTRACK_MAX: i32 = MATCH_THEN;
pub const MATCH_BACKTRACK_MIN: i32 = MATCH_COMMIT;

pub const GF_CAPTURE: u32 = 0x00010000;
pub const GF_NOCAPTURE: u32 = 0x00020000;
pub const GF_CONDASSERT: u32 = 0x00030000;
pub const GF_RECURSE: u32 = 0x00040000;

#[inline]
pub fn GF_IDMASK(a: u32) -> u32 {
    a & 0xffff0000u32
}
#[inline]
pub fn GF_DATAMASK(a: u32) -> u32 {
    a & 0x0000ffffu32
}

pub const REPTYPE_MIN: u32 = 0;
pub const REPTYPE_MAX: u32 = 1;
pub const REPTYPE_POS: u32 = 2;

pub static rep_min: [u32; 11] = [0, 0, 1, 1, 0, 0, 0, 0, 0, 1, 0];

pub static rep_max: [u32; 11] = [
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

pub static rep_typ: [u32; 12] = [
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

/* ---- State identifiers ---- */

/* The RMATCH return ids, with the same numeric values as the C enums. */
pub const RM1: u32 = 1;
pub const RM2: u32 = 2;
pub const RM3: u32 = 3;
pub const RM4: u32 = 4;
pub const RM5: u32 = 5;
pub const RM6: u32 = 6;
pub const RM7: u32 = 7;
pub const RM8: u32 = 8;
pub const RM9: u32 = 9;
pub const RM10: u32 = 10;
pub const RM11: u32 = 11;
pub const RM12: u32 = 12;
pub const RM13: u32 = 13;
pub const RM14: u32 = 14;
pub const RM15: u32 = 15;
pub const RM16: u32 = 16;
pub const RM17: u32 = 17;
pub const RM18: u32 = 18;
pub const RM19: u32 = 19;
pub const RM20: u32 = 20;
pub const RM21: u32 = 21;
pub const RM22: u32 = 22;
pub const RM23: u32 = 23;
pub const RM24: u32 = 24;
pub const RM25: u32 = 25;
pub const RM26: u32 = 26;
pub const RM27: u32 = 27;
pub const RM28: u32 = 28;
pub const RM29: u32 = 29;
pub const RM30: u32 = 30;
pub const RM31: u32 = 31;
pub const RM32: u32 = 32;
pub const RM33: u32 = 33;
pub const RM34: u32 = 34;
pub const RM35: u32 = 35;
pub const RM36: u32 = 36;
pub const RM37: u32 = 37;
pub const RM38: u32 = 38;
pub const RM39: u32 = 39;
pub const RM100: u32 = 100;
pub const RM101: u32 = 101;
pub const RM102: u32 = 102;
pub const RM103: u32 = 103;
pub const RM200: u32 = 200;
pub const RM201: u32 = 201;
pub const RM202: u32 = 202;
pub const RM203: u32 = 203;
pub const RM204: u32 = 204;
pub const RM205: u32 = 205;
pub const RM206: u32 = 206;
pub const RM207: u32 = 207;
pub const RM208: u32 = 208;
pub const RM209: u32 = 209;
pub const RM210: u32 = 210;
pub const RM211: u32 = 211;
pub const RM212: u32 = 212;
pub const RM213: u32 = 213;
pub const RM214: u32 = 214;
pub const RM215: u32 = 215;
pub const RM216: u32 = 216;
pub const RM217: u32 = 217;
pub const RM218: u32 = 218;
pub const RM219: u32 = 219;
pub const RM220: u32 = 220;
pub const RM221: u32 = 221;
pub const RM222: u32 = 222;
pub const RM223: u32 = 223;
pub const RM224: u32 = 224;

/* The structural labels of match(). */
pub const S_MATCH_RECURSE: u32 = 900;
pub const S_NEW_FRAME: u32 = 901;
pub const S_MAINLOOP: u32 = 902;
pub const S_RETURN_SWITCH: u32 = 903;

/* Ordinary labels inside the big switch, which are jumped to from more than one
place (C line numbers in comments). */
pub const L_REPEATCHAR: u32 = 910; /* 1392 */
pub const L_REPEATNOTCHAR: u32 = 911; /* 1733 */
pub const L_REPEATTYPE: u32 = 912; /* 2973 */
pub const L_REF_REPEAT: u32 = 913; /* 5278 */
pub const L_POSSESSIVE_NON_CAPTURE: u32 = 914; /* 5545 */
pub const L_POSSESSIVE_CAPTURE: u32 = 915; /* 5553 */
pub const L_POSSESSIVE_GROUP: u32 = 916; /* 5557 */
pub const L_GROUPLOOP: u32 = 917; /* 5676 */
pub const L_ASSERT_NOT_FAILED: u32 = 918; /* 5853 */
pub const L_ASSERT_NL_OR_EOS: u32 = 919; /* 6604 */
pub const L_SCS_OFFSET_FOUND: u32 = 920; /* 5907 */

/* ---- chunk A: EXTRA_STATE_CONSTS ---- */
/* Synthetic states for chunk A (C lines 900..2573). */

/* Shared fall-through target for OP_ACCEPT/OP_END (C line 976). */
pub const L_A_OP_END: u32 = 1000;
/* Shared fall-through target for OP_ANY/OP_ALLANY (C line 1076). */
pub const L_A_ALLANY: u32 = 1001;

/* REPEATCHAR: multi-code-unit UTF character (C lines 1430..1482). */
pub const L_A_UTF_MIN_LOOP: u32 = 1002; /* RM202 */
pub const L_A_UTF_MAX_LOOP: u32 = 1003; /* RM203 */
/* REPEATCHAR: single-code-unit, caseless (C lines 1532..1574). */
pub const L_A_CI_MIN_LOOP: u32 = 1004; /* RM25 */
pub const L_A_CI_MAX_LOOP: u32 = 1005; /* RM26 */
/* REPEATCHAR: single-code-unit, caseful (C lines 1593..1631). */
pub const L_A_CF_MIN_LOOP: u32 = 1006; /* RM27 */
pub const L_A_CF_MAX_LOOP: u32 = 1007; /* RM28 */

/* REPEATNOTCHAR: caseless (C lines 1788..1889). */
pub const L_A_N_CI_UMIN_LOOP: u32 = 1008; /* RM204 */
pub const L_A_N_CI_MIN_LOOP: u32 = 1009; /* RM29 */
pub const L_A_N_CI_UMAX_LOOP: u32 = 1010; /* RM205 */
pub const L_A_N_CI_MAX_LOOP: u32 = 1011; /* RM30 */
/* REPEATNOTCHAR: caseful (C lines 1928..2026). */
pub const L_A_N_CF_UMIN_LOOP: u32 = 1012; /* RM206 */
pub const L_A_N_CF_MIN_LOOP: u32 = 1013; /* RM31 */
pub const L_A_N_CF_UMAX_LOOP: u32 = 1014; /* RM207 */
pub const L_A_N_CF_MAX_LOOP: u32 = 1015; /* RM32 */

/* OP_CLASS/OP_NCLASS (C lines 2143..2270). */
pub const L_A_CLASS_UMIN_LOOP: u32 = 1016; /* RM200 */
pub const L_A_CLASS_MIN_LOOP: u32 = 1017; /* RM23 */
pub const L_A_CLASS_UMAX_LOOP: u32 = 1018; /* RM201 */
pub const L_A_CLASS_MAX_LOOP: u32 = 1019; /* RM24 */

/* OP_XCLASS (C lines 2353..2412). */
pub const L_A_XCLASS_MIN_LOOP: u32 = 1020; /* RM100 */
pub const L_A_XCLASS_MAX_LOOP: u32 = 1021; /* RM101 */

/* OP_ECLASS (C lines 2496..2556). */
pub const L_A_ECLASS_MIN_LOOP: u32 = 1022; /* RM102 */
pub const L_A_ECLASS_MAX_LOOP: u32 = 1023; /* RM103 */

/* memcmp(a, b, n) == 0 */
#[inline]
pub(crate) unsafe fn frag_a_memcmp_eq(a: PCRE2_SPTR, b: PCRE2_SPTR, n: PCRE2_SIZE) -> bool {
    let mut k: PCRE2_SIZE = 0;
    while k < n {
        if *a.add(k) != *b.add(k) {
            return false;
        }
        k += 1;
    }
    true
}
/* ---- chunk B: EXTRA_STATE_CONSTS ---- */
/* ------------------------------------------------------------------ *
 * Chunk B (C lines 2574..5488) synthetic states, 1100..1199.
 *
 * Every one of these is a "loop entry" state: the C code has an
 * RMATCH() inside a for(;;) loop, so instead of duplicating the loop
 * body we give the top of the loop its own state and the RMxx state
 * jumps back to it.
 * ------------------------------------------------------------------ */

/* REPEATTYPE, minimizing (REPTYPE_MIN) repeats - property tests */
pub const L_B_TR_MIN_LAMP: u32 = 1100; /* C 3784, RM208 */
pub const L_B_TR_MIN_GC: u32 = 1101; /* C 3805, RM209 */
pub const L_B_TR_MIN_PC: u32 = 1102; /* C 3822, RM210 */
pub const L_B_TR_MIN_SC: u32 = 1103; /* C 3839, RM211 */
pub const L_B_TR_MIN_SCX: u32 = 1104; /* C 3856, RM224 */
pub const L_B_TR_MIN_ALNUM: u32 = 1105; /* C 3878, RM212 */
pub const L_B_TR_MIN_SPACE: u32 = 1106; /* C 3902, RM213 */
pub const L_B_TR_MIN_WORD: u32 = 1107; /* C 3929, RM214 */
pub const L_B_TR_MIN_CLIST: u32 = 1108; /* C 3952, RM215 */
pub const L_B_TR_MIN_UCNC: u32 = 1109; /* C 3989, RM216 */
pub const L_B_TR_MIN_BIDICL: u32 = 1110; /* C 4008, RM223 */
pub const L_B_TR_MIN_BOOL: u32 = 1111; /* C 4025, RM222 */
/* REPEATTYPE, minimizing repeats - other character types */
pub const L_B_TR_MIN_EXTUNI: u32 = 1112; /* C 4061, RM217 */
pub const L_B_TR_MIN_UTF: u32 = 1113; /* C 4088, RM218 */
pub const L_B_TR_MIN_NOUTF: u32 = 1114; /* C 4218, RM33 */
/* REPEATTYPE, maximizing repeats - the backtracking loops */
pub const L_B_TR_MAX_PROP_BT: u32 = 1115; /* C 4638, RM221 */
pub const L_B_TR_MAX_EXTUNI_BT: u32 = 1116; /* C 4678, RM219 */
pub const L_B_TR_MAX_UTF_BT: u32 = 1117; /* C 4957, RM220 */
pub const L_B_TR_MAX_NOUTF_BT: u32 = 1118; /* C 5213, RM34 */
/* Repeated back references */
pub const L_B_REF_MINLOOP: u32 = 1119; /* C 5360, RM20 */
pub const L_B_REF_SAMELEN: u32 = 1120; /* C 5421, RM21 */
pub const L_B_REF_DIFFLEN: u32 = 1121; /* C 5435, RM22 */

/* The HSPACE_xxx_CASES / VSPACE_xxx_CASES lists of pcre2_internal.h,
8-bit mode, not EBCDIC. They are used in pattern position, so they must
be macros expanding to or-patterns. The scrutinee is always cast to u32. */

macro_rules! B_HSPACE_BYTE_CASES {
    () => {
        0x09u32 | 0x20u32 | 0xa0u32
    };
}
macro_rules! B_HSPACE_CASES {
    () => {
        0x09u32
            | 0x20u32
            | 0xa0u32
            | 0x1680u32
            | 0x180eu32
            | 0x2000u32
            | 0x2001u32
            | 0x2002u32
            | 0x2003u32
            | 0x2004u32
            | 0x2005u32
            | 0x2006u32
            | 0x2007u32
            | 0x2008u32
            | 0x2009u32
            | 0x200au32
            | 0x202fu32
            | 0x205fu32
            | 0x3000u32
    };
}
macro_rules! B_VSPACE_BYTE_CASES {
    () => {
        0x0au32 | 0x0bu32 | 0x0cu32 | 0x0du32 | 0x85u32
    };
}
macro_rules! B_VSPACE_CASES {
    () => {
        0x0au32 | 0x0bu32 | 0x0cu32 | 0x0du32 | 0x85u32 | 0x2028u32 | 0x2029u32
    };
}
macro_rules! B_HSPACE_VSPACE_CASES {
    () => {
        0x09u32
            | 0x20u32
            | 0xa0u32
            | 0x1680u32
            | 0x180eu32
            | 0x2000u32
            | 0x2001u32
            | 0x2002u32
            | 0x2003u32
            | 0x2004u32
            | 0x2005u32
            | 0x2006u32
            | 0x2007u32
            | 0x2008u32
            | 0x2009u32
            | 0x200au32
            | 0x202fu32
            | 0x205fu32
            | 0x3000u32
            | 0x0au32
            | 0x0bu32
            | 0x0cu32
            | 0x0du32
            | 0x85u32
            | 0x2028u32
            | 0x2029u32
    };
}
/* ---- chunk C: EXTRA_STATE_CONSTS ---- */
/* Synthetic loop-entry / continuation states for chunk C (C lines 5489..6900). */
pub const S_BRAPOS_LOOP: u32 = 1200; /* top of the for(;;) at C 5561 */
pub const S_BRAPOS_AFTER: u32 = 1201; /* after that loop, C 5596 */
pub const S_BRA_LOOP: u32 = 1202; /* top of the for(;;) at C 5629 */
pub const S_RECURSE_LOOP: u32 = 1203; /* top of the for(;;) at C 5743 */
pub const S_ASSERT_LOOP: u32 = 1204; /* top of the for(;;) at C 5793 */
pub const S_ASSERTNOT_LOOP: u32 = 1205; /* top of the for(;;) at C 5822 */
pub const S_SCS_CREF_LOOP: u32 = 1206; /* top of the for(;;) at C 5878 */
pub const S_SCS_MATCH_LOOP: u32 = 1207; /* top of the for(;;) at C 5936 */
pub const S_COND_ASSERT_LOOP: u32 = 1208; /* top of the for(;;) at C 6102 */
pub const S_COND_CHOOSE: u32 = 1209; /* C 6159, after the condition switch */
pub const S_VREVERSE_LOOP: u32 = 1210; /* top of the for(;;) at C 6272 */

/*************************************************
*         Match from current position            *
*************************************************/

/* This function is called to run one match attempt at a single starting point
in the subject. Returns MATCH_MATCH, MATCH_NOMATCH, a negative MATCH_xxx value
for PRUNE, SKIP etc, or a negative PCRE2_ERROR_xxx value. */

pub(crate) unsafe fn match_(
    start_eptr: PCRE2_SPTR,
    start_ecode_in: PCRE2_SPTR,
    top_bracket: u16,
    frame_size: PCRE2_SIZE,
    match_data: *mut pcre2_real_match_data,
    mb: *mut match_block,
) -> i32 {
    /* Frame-handling variables */
    let mut F: *mut heapframe; /* Current frame pointer */
    let mut N: *mut heapframe = null_mut(); /* Temporary frame pointers */
    let mut P: *mut heapframe = null_mut();

    let mut frames_top: *mut heapframe; /* End of frames vector */
    let mut assert_accept_frame: *mut heapframe = null_mut();
    let mut frame_copy_size: PCRE2_SIZE; /* Amount to copy when creating a new frame */

    /* Local variables that do not need to be preserved over calls to RMATCH(). */
    let mut branch_end: PCRE2_SPTR = null();
    let mut branch_start: PCRE2_SPTR = null();
    let mut bracode: PCRE2_SPTR = null(); /* Temp pointer to start of group */
    let mut offset: PCRE2_SIZE = 0; /* Used for group offsets */
    let mut length: PCRE2_SIZE = 0; /* Used for various length calculations */

    let mut rrc: i32 = 0; /* Return from functions & backtracking "recursions" */
    let mut proptype: i32 = 0; /* Type of character property */

    let mut i: u32 = 0; /* Used for local loops */
    let mut fc: u32 = 0; /* Character values */
    let mut number: u32 = 0; /* Used for group and other numbers */
    let mut reptype: u32 = 0; /* Type of repetition */
    let mut group_frame_type: u32 = 0; /* Specifies type for new group frames */

    let mut condition: BOOL = 0; /* Used in conditional groups */
    let mut cur_is_word: BOOL = 0; /* Used in "word" tests */
    let mut prev_is_word: BOOL = 0; /* Used in "word" tests */

    /* UTF and UCP flags */
    let utf: BOOL = (((*mb).poptions & PCRE2_UTF) != 0) as BOOL;
    let ucp: BOOL = (((*mb).poptions & PCRE2_UCP) != 0) as BOOL;

    /* The "argument" of the emulated recursive call. */
    let mut start_ecode: PCRE2_SPTR = start_ecode_in;

    /* ---- chunk A: EXTRA_LOCALS ---- */
let mut othercase: u32 = 0; /* REPEATCHAR, C line 1406 */
/* ---- chunk B: EXTRA_LOCALS ---- */
/* Chunk B needs no extra function-scope locals: every variable that C
declares inside one of its case blocks and that has to survive an
RMATCH() split point is already a field of the heap frame. */
/* ---- chunk C: EXTRA_LOCALS ---- */
/* chunk C: the `ecode` local of the OP_ASSERT_SCS case (C 5870); it must
survive the goto to SCS_OFFSET_FOUND, which is a separate state here. */
let mut scs_ecode: PCRE2_SPTR = null();

    /* Short names for the fields of the current frame, mirroring the #defines in
    pcre2_match.c. They are used as place expressions, e.g. `Fecode!() = code;`. */

    macro_rules! Fback_frame {
        () => {
            (*F).back_frame
        };
    }
    macro_rules! Fcapture_last {
        () => {
            (*F).capture_last
        };
    }
    macro_rules! Fcurrent_recurse {
        () => {
            (*F).current_recurse
        };
    }
    macro_rules! Fecode {
        () => {
            (*F).ecode
        };
    }
    macro_rules! Feptr {
        () => {
            (*F).eptr
        };
    }
    macro_rules! Fgroup_frame_type {
        () => {
            (*F).group_frame_type
        };
    }
    macro_rules! Flast_group_offset {
        () => {
            (*F).last_group_offset
        };
    }
    macro_rules! Fmark {
        () => {
            (*F).mark
        };
    }
    macro_rules! Frdepth {
        () => {
            (*F).rdepth
        };
    }
    macro_rules! Fstart_match {
        () => {
            (*F).start_match
        };
    }
    macro_rules! Foffset_top {
        () => {
            (*F).offset_top
        };
    }
    macro_rules! Fop {
        () => {
            (*F).op
        };
    }
    /* Fovector is a pointer to the frame's ovector: use `*Fovector!().add(n)`. */
    macro_rules! Fovector {
        () => {
            (*F).ovector.as_mut_ptr()
        };
    }
    macro_rules! Freturn_id {
        () => {
            (*F).return_id
        };
    }

    /* The partial-matching macros. They can `return` from the function. */
    macro_rules! SCHECK_PARTIAL {
        () => {
            if (*mb).partial != 0 && (Feptr!() > (*mb).start_used_ptr || (*mb).allowemptypartial != 0) {
                (*mb).hitend = TRUE;
                if (*mb).partial > 1 {
                    return PCRE2_ERROR_PARTIAL;
                }
            }
        };
    }
    macro_rules! CHECK_PARTIAL {
        () => {
            if Feptr!() >= (*mb).end_subject {
                SCHECK_PARTIAL!();
            }
        };
    }

    /* IS_NEWLINE / WAS_NEWLINE for NLBLOCK == mb (PSSTART = start_subject,
    PSEND = end_subject). */
    macro_rules! IS_NEWLINE {
        ($p:expr) => {
            crate::macros::is_newline_block(
                $p,
                (*mb).nltype,
                &mut (*mb).nllen,
                (*mb).nl.as_ptr(),
                (*mb).end_subject,
                utf,
            )
        };
    }
    macro_rules! WAS_NEWLINE {
        ($p:expr) => {
            crate::macros::was_newline_block(
                $p,
                (*mb).nltype,
                &mut (*mb).nllen,
                (*mb).nl.as_ptr(),
                (*mb).start_subject,
                utf,
            )
        };
    }


    /* This is the length of the last part of a backtracking frame that must be
    copied when a new frame is created. */

    frame_copy_size = frame_size - EPTR_OFFSET_IN_HEAPFRAME;

    /* Set up the first frame and the end of the frames vector. */

    F = (*match_data).heapframes;
    frames_top = ((F as *mut u8).add((*match_data).heapframes_size)) as *mut heapframe;

    Frdepth!() = 0; /* "Recursion" depth */
    Fcapture_last!() = 0; /* Number of most recent capture */
    Fcurrent_recurse!() = RECURSE_UNSET; /* Not pattern recursing. */
    Feptr!() = start_eptr; /* Current data pointer */
    Fstart_match!() = start_eptr; /* Start match */
    Fmark!() = null(); /* Most recent mark */
    Foffset_top!() = 0; /* End of captures within the frame */
    Flast_group_offset!() = PCRE2_UNSET; /* Saved frame of most recent group */
    group_frame_type = 0; /* Not a start of group frame */

    let mut state: u32 = S_NEW_FRAME; /* goto NEW_FRAME */

    'sm: loop {
        match state {
            /* ============================================================= */
            /* Come back here when we want to create a new frame for remembering a
            backtracking point. */
            S_MATCH_RECURSE => {
                /* Set up a new backtracking frame. If the vector is full, get a new
                one, doubling the size, but constrained by the heap limit (which is
                in KiB). */

                N = ((F as *mut u8).add(frame_size)) as *mut heapframe;
                if (((N as *mut u8).add(frame_size)) as *mut heapframe) >= frames_top {
                    let new_: *mut heapframe;
                    let mut newsize: PCRE2_SIZE;
                    let usedsize: PCRE2_SIZE =
                        (N as usize) - ((*match_data).heapframes as usize);

                    if (*match_data).heapframes_size >= PCRE2_SIZE_MAX / 2 {
                        if (*match_data).heapframes_size == PCRE2_SIZE_MAX - 1 {
                            return PCRE2_ERROR_NOMEMORY;
                        }
                        newsize = PCRE2_SIZE_MAX - 1;
                    } else {
                        newsize = (*match_data).heapframes_size * 2;
                    }

                    if newsize / 1024 >= (*mb).heap_limit as usize {
                        let old_size: PCRE2_SIZE = (*match_data).heapframes_size / 1024;
                        if ((*mb).heap_limit as usize) <= old_size {
                            return PCRE2_ERROR_HEAPLIMIT;
                        } else {
                            let mut max_delta: PCRE2_SIZE =
                                1024 * ((*mb).heap_limit as usize - old_size);
                            let over_bytes: i32 =
                                ((*match_data).heapframes_size % 1024) as i32;
                            if over_bytes != 0 {
                                max_delta -= (1024 - over_bytes) as usize;
                            }
                            newsize = (*match_data).heapframes_size + max_delta;
                        }
                    }

                    /* With a heap limit set, the permitted additional size may not be
                    enough for another frame, so do a final check. */

                    if newsize - usedsize < frame_size {
                        return PCRE2_ERROR_HEAPLIMIT;
                    }
                    new_ = ((*match_data).memctl.malloc.unwrap())(
                        newsize,
                        (*match_data).memctl.memory_data,
                    ) as *mut heapframe;
                    if new_.is_null() {
                        return PCRE2_ERROR_NOMEMORY;
                    }
                    copy_nonoverlapping(
                        (*match_data).heapframes as *const u8,
                        new_ as *mut u8,
                        usedsize,
                    );

                    N = ((new_ as *mut u8).add(usedsize)) as *mut heapframe;
                    F = ((N as *mut u8).sub(frame_size)) as *mut heapframe;

                    ((*match_data).memctl.free.unwrap())(
                        (*match_data).heapframes as *mut c_void,
                        (*match_data).memctl.memory_data,
                    );
                    (*match_data).heapframes = new_;
                    (*match_data).heapframes_size = newsize;
                    frames_top = ((new_ as *mut u8).add(newsize)) as *mut heapframe;
                }

                /* Copy those fields that must be copied into the new frame, increase
                the "recursion" depth (i.e. the new frame's index) and then make the
                new frame current. */

                copy_nonoverlapping(
                    (F as *const u8).add(EPTR_OFFSET_IN_HEAPFRAME),
                    (N as *mut u8).add(EPTR_OFFSET_IN_HEAPFRAME),
                    frame_copy_size,
                );

                (*N).rdepth = Frdepth!() + 1;
                F = N;

                /* Carry on processing with a new frame. */
                state = S_NEW_FRAME;
                continue 'sm;
            }

            S_NEW_FRAME => {
                Fgroup_frame_type!() = group_frame_type;
                Fecode!() = start_ecode; /* Starting code pointer */
                Fback_frame!() = frame_size; /* Default is go back one frame */

                /* If this is a special type of group frame, remember its offset for
                quick access at the end of the group. If this is a recursion, set a new
                current recursion value. */

                if group_frame_type != 0 {
                    Flast_group_offset!() =
                        (F as usize) - ((*match_data).heapframes as usize);
                    if GF_IDMASK(group_frame_type) == GF_RECURSE {
                        Fcurrent_recurse!() = GF_DATAMASK(group_frame_type);
                    }
                    group_frame_type = 0;
                }

                /* This is the main processing loop. First check that we haven't
                recorded too many backtracks (search tree is too large), or that we
                haven't exceeded the recursive depth limit (used too many backtracking
                frames). If not, process the opcodes. */

                let mcc = (*mb).match_call_count;
                (*mb).match_call_count = mcc + 1;
                if mcc >= (*mb).match_limit {
                    return PCRE2_ERROR_MATCHLIMIT;
                }
                if Frdepth!() >= (*mb).match_limit_depth {
                    return PCRE2_ERROR_DEPTHLIMIT;
                }

                state = S_MAINLOOP;
                continue 'sm;
            }

            S_MAINLOOP => {
                Fop!() = *Fecode!();
                match Fop!() as u32 {
                    /* ---- chunk A: ARMS ---- */
/* ===================================================================== */
/* Before OP_ACCEPT there may be any number of OP_CLOSE opcodes, to close
any currently open capturing brackets. Unlike reaching the end of a group,
where we know the starting frame is at the top of the chained frames, in
this case we have to search back for the relevant frame in case other types
of group that use chained frames have intervened. Multiple OP_CLOSEs always
come innermost first, which matches the chain order. We can ignore this in
a recursion, because captures are not passed out of recursions. */

OP_CLOSE => {
    if Fcurrent_recurse!() == RECURSE_UNSET {
        number = GET2!(Fecode!(), 1);
        offset = Flast_group_offset!();
        loop {
            /* Corrupted heapframes?. Trigger an assert and return an error */
            /* PCRE2_ASSERT(offset != PCRE2_UNSET); */
            if offset == PCRE2_UNSET {
                return PCRE2_ERROR_INTERNAL;
            }

            N = ((*match_data).heapframes as *mut u8).add(offset) as *mut heapframe;
            P = ((N as *mut u8).sub(frame_size)) as *mut heapframe;
            if (*N).group_frame_type == (GF_CAPTURE | number) {
                break;
            }
            offset = (*P).last_group_offset;
        }
        offset = ((number << 1) - 2) as PCRE2_SIZE;
        Fcapture_last!() = number;
        *Fovector!().add(offset) =
            ((*P).eptr as usize) - ((*mb).start_subject as usize);
        *Fovector!().add(offset + 1) =
            (Feptr!() as usize) - ((*mb).start_subject as usize);
        if offset >= Foffset_top!() {
            Foffset_top!() = offset + 2;
        }
    }
    Fecode!() = Fecode!()
        .add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Real or forced end of the pattern, assertion, or recursion. In an
assertion ACCEPT, update the last used pointer and remember the current
frame so that the captures and mark can be fished out of it. */

OP_ASSERT_ACCEPT => {
    if Feptr!() > (*mb).last_used_ptr {
        (*mb).last_used_ptr = Feptr!();
    }
    assert_accept_frame = F;
    rrc = MATCH_ACCEPT; /* RRETURN(MATCH_ACCEPT) */
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* For ACCEPT within a recursion, we have to find the most recent
recursion. If not in a recursion, fall through to code that is common with
OP_END. */

OP_ACCEPT => {
    if Fcurrent_recurse!() != RECURSE_UNSET {
        offset = Flast_group_offset!();
        loop {
            /* Corrupted heapframes?. Trigger an assert and return an error */
            /* PCRE2_ASSERT(offset != PCRE2_UNSET); */
            if offset == PCRE2_UNSET {
                return PCRE2_ERROR_INTERNAL;
            }

            N = ((*match_data).heapframes as *mut u8).add(offset) as *mut heapframe;
            P = ((N as *mut u8).sub(frame_size)) as *mut heapframe;
            if GF_IDMASK((*N).group_frame_type) == GF_RECURSE {
                break;
            }
            offset = (*P).last_group_offset;
        }

        /* N is now the frame of the recursion; the previous frame is at the
        OP_RECURSE position. Go back there, copying the current subject position
        and mark, and the start_match position (\K might have changed it), and
        then move on past the OP_RECURSE. */

        (*P).eptr = Feptr!();
        (*P).mark = Fmark!();
        (*P).start_match = Fstart_match!();
        F = P;
        Fecode!() = Fecode!().add(1 + LINK_SIZE);
        state = S_MAINLOOP; /* continue */
        continue 'sm;
    }
    /* Fall through */
    state = L_A_OP_END;
    continue 'sm;
}

/* OP_END itself can never be reached within a recursion because that is
picked up when the OP_KET that always precedes OP_END is reached. */

OP_END => {
    state = L_A_OP_END;
    continue 'sm;
}

/*===================================================================== */
/* Match any single character type except newline; have to take care with
CRLF newlines and partial matching. */

OP_ANY => {
    if IS_NEWLINE!(Feptr!()) != 0 {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if (*mb).partial != 0
        && Feptr!() == (*mb).end_subject.wrapping_sub(1)
        && (*mb).nltype == NLTYPE_FIXED
        && (*mb).nllen == 2
        && *Feptr!() == (*mb).nl[0]
    {
        (*mb).hitend = TRUE;
        if (*mb).partial > 1 {
            return PCRE2_ERROR_PARTIAL;
        }
    }
    /* Fall through */
    state = L_A_ALLANY;
    continue 'sm;
}

/* Match any single character whatsoever. */

OP_ALLANY => {
    state = L_A_ALLANY;
    continue 'sm;
}

/* ===================================================================== */
/* Match a single code unit, even in UTF mode. This opcode really does
match any code unit, even newline. */

OP_ANYBYTE => {
    if Feptr!() >= (*mb).end_subject
    /* DO NOT merge the Feptr++ here; it must */
    {
        /* not be updated before SCHECK_PARTIAL. */
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Feptr!() = Feptr!().add(1);
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Match a single character, casefully */

OP_CHAR => {
    if utf != 0 {
        length = 1;
        Fecode!() = Fecode!().add(1);
        GETCHARLEN!(fc, Fecode!(), length);
        if length > (((*mb).end_subject as usize) - (Feptr!() as usize)) as PCRE2_SIZE {
            CHECK_PARTIAL!(); /* Not SCHECK_PARTIAL() */
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        while length > 0 {
            let a_ = *Fecode!();
            Fecode!() = Fecode!().add(1);
            let b_ = *Feptr!();
            Feptr!() = Feptr!().add(1);
            if a_ != b_ {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            length -= 1;
        }
    }
    /* Not UTF mode */
    else {
        if (*mb).end_subject.offset_from(Feptr!()) < 1 {
            SCHECK_PARTIAL!(); /* This one can use SCHECK_PARTIAL() */
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        let b_ = *Feptr!();
        Feptr!() = Feptr!().add(1);
        if *Fecode!().add(1) != b_ {
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        Fecode!() = Fecode!().add(2);
    }
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Match a single character, caselessly. If we are at the end of the
subject, give up immediately. We get here only when the pattern character
has at most one other case. */

OP_CHARI => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }

    if utf != 0 {
        length = 1;
        Fecode!() = Fecode!().add(1);
        GETCHARLEN!(fc, Fecode!(), length);

        /* If the pattern character's value is < 128, we know that its other case
        (if any) is also < 128, so we can use the fast lookup table. */

        if fc < 128 {
            let cc: u32 = *Feptr!() as u32;
            if *(*mb).lcc.add(fc as usize) != TABLE_GET!(cc, (*mb).lcc, cc) {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            Fecode!() = Fecode!().add(1);
            Feptr!() = Feptr!().add(1);
        }
        /* Otherwise we must pick up the subject character and use Unicode
        property support to test its other case. */
        else {
            let mut dc: u32 = 0;
            GETCHARINC!(dc, Feptr!());
            Fecode!() = Fecode!().add(length);
            if dc != fc && dc != UCD_OTHERCASE!(fc) {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }
    }
    /* If UCP is set without UTF we must do the same as above, but with one
    character per code unit. */
    else if ucp != 0 {
        let cc: u32 = *Feptr!() as u32;
        fc = *Fecode!().add(1) as u32;
        if fc < 128 {
            if *(*mb).lcc.add(fc as usize) != TABLE_GET!(cc, (*mb).lcc, cc) {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        } else {
            if cc != fc && cc != UCD_OTHERCASE!(fc) {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }
        Feptr!() = Feptr!().add(1);
        Fecode!() = Fecode!().add(2);
    }
    /* Not UTF or UCP mode; use the table for characters < 256. */
    else {
        if TABLE_GET!(*Fecode!().add(1), (*mb).lcc, *Fecode!().add(1))
            != TABLE_GET!(*Feptr!(), (*mb).lcc, *Feptr!())
        {
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        Feptr!() = Feptr!().add(1);
        Fecode!() = Fecode!().add(2);
    }
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Match not a single character. */

OP_NOT | OP_NOTI => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }

    if utf != 0 {
        let mut ch: u32 = 0;
        Fecode!() = Fecode!().add(1);
        GETCHARINC!(ch, Fecode!());
        GETCHARINC!(fc, Feptr!());
        if ch == fc {
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) - caseful match */
            state = S_RETURN_SWITCH;
            continue 'sm;
        } else if Fop!() as u32 == OP_NOTI
        /* If caseless */
        {
            if ch > 127 {
                ch = UCD_OTHERCASE!(ch);
            } else {
                ch = *(*mb).fcc.add(ch as usize) as u32;
            }
            if ch == fc {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }
    }
    /* UCP without UTF is as above, but with one character per code unit. */
    else if ucp != 0 {
        let mut ch: u32;
        fc = *Feptr!() as u32;
        Feptr!() = Feptr!().add(1);
        ch = *Fecode!().add(1) as u32;
        Fecode!() = Fecode!().add(2);

        if ch == fc {
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) - caseful match */
            state = S_RETURN_SWITCH;
            continue 'sm;
        } else if Fop!() as u32 == OP_NOTI
        /* If caseless */
        {
            if ch > 127 {
                ch = UCD_OTHERCASE!(ch);
            } else {
                ch = *(*mb).fcc.add(ch as usize) as u32;
            }
            if ch == fc {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }
    }
    /* Neither UTF nor UCP is set */
    else {
        let ch: u32 = *Fecode!().add(1) as u32;
        fc = *Feptr!() as u32;
        Feptr!() = Feptr!().add(1);
        if ch == fc
            || (Fop!() as u32 == OP_NOTI
                && TABLE_GET!(ch, (*mb).fcc, ch) as u32 == fc)
        {
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        Fecode!() = Fecode!().add(2);
    }
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Match a single character repeatedly.
   Llength     -> (*F).byte1
   Loclength   -> (*F).byte2
   Lstart_eptr -> (*F).fields.char_repeat.start_eptr
   Lcharptr    -> (*F).fields.char_repeat.charptr
   Lmin        -> (*F).fields.char_repeat.min
   Lmax        -> (*F).fields.char_repeat.max
   Lc          -> (*F).fields.char_repeat.c
   Loc         -> (*F).fields.char_repeat.oc.oc
   Loccu       -> (*F).fields.char_repeat.oc.occu
*/

OP_EXACT | OP_EXACTI => {
    (*F).fields.char_repeat.max = GET2!(Fecode!(), 1);
    (*F).fields.char_repeat.min = (*F).fields.char_repeat.max;
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATCHAR; /* goto REPEATCHAR */
    continue 'sm;
}

OP_POSUPTO | OP_POSUPTOI => {
    reptype = REPTYPE_POS;
    (*F).fields.char_repeat.min = 0;
    (*F).fields.char_repeat.max = GET2!(Fecode!(), 1);
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATCHAR; /* goto REPEATCHAR */
    continue 'sm;
}

OP_UPTO | OP_UPTOI => {
    reptype = REPTYPE_MAX;
    (*F).fields.char_repeat.min = 0;
    (*F).fields.char_repeat.max = GET2!(Fecode!(), 1);
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATCHAR; /* goto REPEATCHAR */
    continue 'sm;
}

OP_MINUPTO | OP_MINUPTOI => {
    reptype = REPTYPE_MIN;
    (*F).fields.char_repeat.min = 0;
    (*F).fields.char_repeat.max = GET2!(Fecode!(), 1);
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATCHAR; /* goto REPEATCHAR */
    continue 'sm;
}

OP_POSSTAR | OP_POSSTARI => {
    reptype = REPTYPE_POS;
    (*F).fields.char_repeat.min = 0;
    (*F).fields.char_repeat.max = u32::MAX;
    Fecode!() = Fecode!().add(1);
    state = L_REPEATCHAR; /* goto REPEATCHAR */
    continue 'sm;
}

OP_POSPLUS | OP_POSPLUSI => {
    reptype = REPTYPE_POS;
    (*F).fields.char_repeat.min = 1;
    (*F).fields.char_repeat.max = u32::MAX;
    Fecode!() = Fecode!().add(1);
    state = L_REPEATCHAR; /* goto REPEATCHAR */
    continue 'sm;
}

OP_POSQUERY | OP_POSQUERYI => {
    reptype = REPTYPE_POS;
    (*F).fields.char_repeat.min = 0;
    (*F).fields.char_repeat.max = 1;
    Fecode!() = Fecode!().add(1);
    state = L_REPEATCHAR; /* goto REPEATCHAR */
    continue 'sm;
}

OP_STAR | OP_STARI | OP_MINSTAR | OP_MINSTARI | OP_PLUS | OP_PLUSI | OP_MINPLUS
| OP_MINPLUSI | OP_QUERY | OP_QUERYI | OP_MINQUERY | OP_MINQUERYI => {
    let t_ = *Fecode!();
    Fecode!() = Fecode!().add(1);
    fc = (t_ as u32)
        - (if (Fop!() as u32) < OP_STARI {
            OP_STAR
        } else {
            OP_STARI
        });
    (*F).fields.char_repeat.min = rep_min[fc as usize];
    (*F).fields.char_repeat.max = rep_max[fc as usize];
    reptype = rep_typ[fc as usize];
    /* Fall through to REPEATCHAR */
    state = L_REPEATCHAR;
    continue 'sm;
}

/* ===================================================================== */
/* Match a negated single one-byte character repeatedly.
   Lstart_eptr -> (*F).fields.charnot_repeat.start_eptr
   Lmin        -> (*F).fields.charnot_repeat.min
   Lmax        -> (*F).fields.charnot_repeat.max
   Lc          -> (*F).fields.charnot_repeat.c
   Loc         -> (*F).fields.charnot_repeat.oc
*/

OP_NOTEXACT | OP_NOTEXACTI => {
    (*F).fields.charnot_repeat.max = GET2!(Fecode!(), 1);
    (*F).fields.charnot_repeat.min = (*F).fields.charnot_repeat.max;
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATNOTCHAR; /* goto REPEATNOTCHAR */
    continue 'sm;
}

OP_NOTUPTO | OP_NOTUPTOI => {
    (*F).fields.charnot_repeat.min = 0;
    (*F).fields.charnot_repeat.max = GET2!(Fecode!(), 1);
    reptype = REPTYPE_MAX;
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATNOTCHAR; /* goto REPEATNOTCHAR */
    continue 'sm;
}

OP_NOTMINUPTO | OP_NOTMINUPTOI => {
    (*F).fields.charnot_repeat.min = 0;
    (*F).fields.charnot_repeat.max = GET2!(Fecode!(), 1);
    reptype = REPTYPE_MIN;
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATNOTCHAR; /* goto REPEATNOTCHAR */
    continue 'sm;
}

OP_NOTPOSSTAR | OP_NOTPOSSTARI => {
    reptype = REPTYPE_POS;
    (*F).fields.charnot_repeat.min = 0;
    (*F).fields.charnot_repeat.max = u32::MAX;
    Fecode!() = Fecode!().add(1);
    state = L_REPEATNOTCHAR; /* goto REPEATNOTCHAR */
    continue 'sm;
}

OP_NOTPOSPLUS | OP_NOTPOSPLUSI => {
    reptype = REPTYPE_POS;
    (*F).fields.charnot_repeat.min = 1;
    (*F).fields.charnot_repeat.max = u32::MAX;
    Fecode!() = Fecode!().add(1);
    state = L_REPEATNOTCHAR; /* goto REPEATNOTCHAR */
    continue 'sm;
}

OP_NOTPOSQUERY | OP_NOTPOSQUERYI => {
    reptype = REPTYPE_POS;
    (*F).fields.charnot_repeat.min = 0;
    (*F).fields.charnot_repeat.max = 1;
    Fecode!() = Fecode!().add(1);
    state = L_REPEATNOTCHAR; /* goto REPEATNOTCHAR */
    continue 'sm;
}

OP_NOTPOSUPTO | OP_NOTPOSUPTOI => {
    reptype = REPTYPE_POS;
    (*F).fields.charnot_repeat.min = 0;
    (*F).fields.charnot_repeat.max = GET2!(Fecode!(), 1);
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATNOTCHAR; /* goto REPEATNOTCHAR */
    continue 'sm;
}

OP_NOTSTAR | OP_NOTSTARI | OP_NOTMINSTAR | OP_NOTMINSTARI | OP_NOTPLUS
| OP_NOTPLUSI | OP_NOTMINPLUS | OP_NOTMINPLUSI | OP_NOTQUERY | OP_NOTQUERYI
| OP_NOTMINQUERY | OP_NOTMINQUERYI => {
    let t_ = *Fecode!();
    Fecode!() = Fecode!().add(1);
    fc = (t_ as u32)
        - (if (Fop!() as u32) >= OP_NOTSTARI {
            OP_NOTSTARI
        } else {
            OP_NOTSTAR
        });
    (*F).fields.charnot_repeat.min = rep_min[fc as usize];
    (*F).fields.charnot_repeat.max = rep_max[fc as usize];
    reptype = rep_typ[fc as usize];
    /* Fall through to REPEATNOTCHAR */
    state = L_REPEATNOTCHAR;
    continue 'sm;
}

/* ===================================================================== */
/* Match a bit-mapped character class, possibly repeatedly.
   Lbyte_map_address -> (*F).fields.class_repeat.byte_map_address
   Lstart_eptr       -> (*F).fields.class_repeat.start_eptr
   Lmin              -> (*F).fields.class_repeat.min
   Lmax              -> (*F).fields.class_repeat.max
*/

OP_NCLASS | OP_CLASS => {
    (*F).fields.class_repeat.byte_map_address = Fecode!().add(1); /* Save for matching */
    Fecode!() = Fecode!().add(1 + 32); /* Advance past the item */

    /* Look past the end of the item to see if there is repeat information
    following. Then obey similar code to character type repeats. */

    match *Fecode!() as u32 {
        OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
        | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
            let t_ = *Fecode!();
            Fecode!() = Fecode!().add(1);
            fc = (t_ as u32) - OP_CRSTAR;
            (*F).fields.class_repeat.min = rep_min[fc as usize];
            (*F).fields.class_repeat.max = rep_max[fc as usize];
            reptype = rep_typ[fc as usize];
        }

        OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
            (*F).fields.class_repeat.min = GET2!(Fecode!(), 1);
            (*F).fields.class_repeat.max = GET2!(Fecode!(), 1 + IMM2_SIZE);
            if (*F).fields.class_repeat.max == 0 {
                (*F).fields.class_repeat.max = u32::MAX; /* Max 0 => infinity */
            }
            reptype = rep_typ[((*Fecode!() as u32) - OP_CRSTAR) as usize];
            Fecode!() = Fecode!().add(1 + 2 * IMM2_SIZE);
        }

        _ => {
            /* No repeat follows */
            (*F).fields.class_repeat.max = 1;
            (*F).fields.class_repeat.min = 1;
        }
    }

    /* First, ensure the minimum number of matches are present. */

    if utf != 0 {
        i = 1;
        while i <= (*F).fields.class_repeat.min {
            if Feptr!() >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            GETCHARINC!(fc, Feptr!());
            if fc > 255 {
                if Fop!() as u32 == OP_CLASS {
                    rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            } else if (*(*F).fields.class_repeat.byte_map_address.add((fc / 8) as usize)
                as u32
                & (1u32 << (fc & 7)))
                == 0
            {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            i += 1;
        }
    }
    /* Not UTF mode */
    else {
        i = 1;
        while i <= (*F).fields.class_repeat.min {
            if Feptr!() >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            fc = *Feptr!() as u32;
            Feptr!() = Feptr!().add(1);
            if (*(*F).fields.class_repeat.byte_map_address.add((fc / 8) as usize) as u32
                & (1u32 << (fc & 7)))
                == 0
            {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            i += 1;
        }
    }

    /* If Lmax == Lmin we are done. Continue with main loop. */

    if (*F).fields.class_repeat.min == (*F).fields.class_repeat.max {
        state = S_MAINLOOP; /* continue */
        continue 'sm;
    }

    /* If minimizing, keep testing the rest of the expression and advancing
    the pointer while it matches the class. */

    if reptype == REPTYPE_MIN {
        if utf != 0 {
            state = L_A_CLASS_UMIN_LOOP;
            continue 'sm;
        }
        /* Not UTF mode */
        else {
            state = L_A_CLASS_MIN_LOOP;
            continue 'sm;
        }
    }
    /* If maximizing, find the longest possible run, then work backwards. */
    else {
        (*F).fields.class_repeat.start_eptr = Feptr!();

        if utf != 0 {
            i = (*F).fields.class_repeat.min;
            while i < (*F).fields.class_repeat.max {
                let mut len: i32 = 1;
                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    break;
                }
                GETCHARLEN!(fc, Feptr!(), len);
                if fc > 255 {
                    if Fop!() as u32 == OP_CLASS {
                        break;
                    }
                } else if (*(*F)
                    .fields
                    .class_repeat
                    .byte_map_address
                    .add((fc / 8) as usize) as u32
                    & (1u32 << (fc & 7)))
                    == 0
                {
                    break;
                }
                Feptr!() = Feptr!().add(len as usize);
                i += 1;
            }

            if reptype == REPTYPE_POS {
                state = S_MAINLOOP; /* continue - no backtracking */
                continue 'sm;
            }

            state = L_A_CLASS_UMAX_LOOP;
            continue 'sm;
        }
        /* Not UTF mode */
        else {
            i = (*F).fields.class_repeat.min;
            while i < (*F).fields.class_repeat.max {
                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    break;
                }
                fc = *Feptr!() as u32;
                if (*(*F).fields.class_repeat.byte_map_address.add((fc / 8) as usize)
                    as u32
                    & (1u32 << (fc & 7)))
                    == 0
                {
                    break;
                }
                Feptr!() = Feptr!().add(1);
                i += 1;
            }

            if reptype == REPTYPE_POS {
                state = S_MAINLOOP; /* continue - no backtracking */
                continue 'sm;
            }

            state = L_A_CLASS_MAX_LOOP;
            continue 'sm;
        }
    }
}

/* ===================================================================== */
/* Match an extended character class.
   Lstart_eptr  -> (*F).fields.xclass_repeat.start_eptr
   Lxclass_data -> (*F).fields.xclass_repeat.xclass_data
   Lmin         -> (*F).fields.xclass_repeat.min
   Lmax         -> (*F).fields.xclass_repeat.max
*/

OP_XCLASS => {
    (*F).fields.xclass_repeat.xclass_data = Fecode!().add(1 + LINK_SIZE); /* Save for matching */
    Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize); /* Advance past the item */

    match *Fecode!() as u32 {
        OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
        | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
            let t_ = *Fecode!();
            Fecode!() = Fecode!().add(1);
            fc = (t_ as u32) - OP_CRSTAR;
            (*F).fields.xclass_repeat.min = rep_min[fc as usize];
            (*F).fields.xclass_repeat.max = rep_max[fc as usize];
            reptype = rep_typ[fc as usize];
        }

        OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
            (*F).fields.xclass_repeat.min = GET2!(Fecode!(), 1);
            (*F).fields.xclass_repeat.max = GET2!(Fecode!(), 1 + IMM2_SIZE);
            if (*F).fields.xclass_repeat.max == 0 {
                (*F).fields.xclass_repeat.max = u32::MAX; /* Max 0 => infinity */
            }
            reptype = rep_typ[((*Fecode!() as u32) - OP_CRSTAR) as usize];
            Fecode!() = Fecode!().add(1 + 2 * IMM2_SIZE);
        }

        _ => {
            /* No repeat follows */
            (*F).fields.xclass_repeat.max = 1;
            (*F).fields.xclass_repeat.min = 1;
        }
    }

    /* First, ensure the minimum number of matches are present. */

    i = 1;
    while i <= (*F).fields.xclass_repeat.min {
        if Feptr!() >= (*mb).end_subject {
            SCHECK_PARTIAL!();
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        GETCHARINCTEST!(fc, Feptr!(), utf);
        if crate::xclass::_pcre2_xclass_8(
            fc,
            (*F).fields.xclass_repeat.xclass_data,
            (*mb).start_code,
            utf,
        ) == 0
        {
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        i += 1;
    }

    /* If Lmax == Lmin we can just continue with the main loop. */

    if (*F).fields.xclass_repeat.min == (*F).fields.xclass_repeat.max {
        state = S_MAINLOOP; /* continue */
        continue 'sm;
    }

    /* If minimizing, keep testing the rest of the expression and advancing
    the pointer while it matches the class. */

    if reptype == REPTYPE_MIN {
        state = L_A_XCLASS_MIN_LOOP;
        continue 'sm;
    }
    /* If maximizing, find the longest possible run, then work backwards. */
    else {
        (*F).fields.xclass_repeat.start_eptr = Feptr!();
        i = (*F).fields.xclass_repeat.min;
        while i < (*F).fields.xclass_repeat.max {
            let mut len: i32 = 1;
            if Feptr!() >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                break;
            }
            GETCHARLENTEST!(fc, Feptr!(), len, utf);
            if crate::xclass::_pcre2_xclass_8(
                fc,
                (*F).fields.xclass_repeat.xclass_data,
                (*mb).start_code,
                utf,
            ) == 0
            {
                break;
            }
            Feptr!() = Feptr!().add(len as usize);
            i += 1;
        }

        if reptype == REPTYPE_POS {
            state = S_MAINLOOP; /* continue - no backtracking */
            continue 'sm;
        }

        state = L_A_XCLASS_MAX_LOOP;
        continue 'sm;
    }
}

/* ===================================================================== */
/* Match a complex, set-based character class.
   Lstart_eptr  -> (*F).fields.eclass_repeat.start_eptr
   Leclass_data -> (*F).fields.eclass_repeat.eclass_data
   Leclass_len  -> (*F).fields.eclass_repeat.eclass_len
   Lmin         -> (*F).fields.eclass_repeat.min
   Lmax         -> (*F).fields.eclass_repeat.max
*/

OP_ECLASS => {
    (*F).fields.eclass_repeat.eclass_data = Fecode!().add(1 + LINK_SIZE); /* Save for matching */
    Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize); /* Advance past the item */
    (*F).fields.eclass_repeat.eclass_len =
        ((Fecode!() as usize) - ((*F).fields.eclass_repeat.eclass_data as usize)) as PCRE2_SIZE;

    match *Fecode!() as u32 {
        OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
        | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
            let t_ = *Fecode!();
            Fecode!() = Fecode!().add(1);
            fc = (t_ as u32) - OP_CRSTAR;
            (*F).fields.eclass_repeat.min = rep_min[fc as usize];
            (*F).fields.eclass_repeat.max = rep_max[fc as usize];
            reptype = rep_typ[fc as usize];
        }

        OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
            (*F).fields.eclass_repeat.min = GET2!(Fecode!(), 1);
            (*F).fields.eclass_repeat.max = GET2!(Fecode!(), 1 + IMM2_SIZE);
            if (*F).fields.eclass_repeat.max == 0 {
                (*F).fields.eclass_repeat.max = u32::MAX; /* Max 0 => infinity */
            }
            reptype = rep_typ[((*Fecode!() as u32) - OP_CRSTAR) as usize];
            Fecode!() = Fecode!().add(1 + 2 * IMM2_SIZE);
        }

        _ => {
            /* No repeat follows */
            (*F).fields.eclass_repeat.max = 1;
            (*F).fields.eclass_repeat.min = 1;
        }
    }

    /* First, ensure the minimum number of matches are present. */

    i = 1;
    while i <= (*F).fields.eclass_repeat.min {
        if Feptr!() >= (*mb).end_subject {
            SCHECK_PARTIAL!();
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        GETCHARINCTEST!(fc, Feptr!(), utf);
        if crate::xclass::_pcre2_eclass_8(
            fc,
            (*F).fields.eclass_repeat.eclass_data,
            (*F).fields
                .eclass_repeat
                .eclass_data
                .add((*F).fields.eclass_repeat.eclass_len),
            (*mb).start_code,
            utf,
        ) == 0
        {
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        i += 1;
    }

    /* If Lmax == Lmin we can just continue with the main loop. */

    if (*F).fields.eclass_repeat.min == (*F).fields.eclass_repeat.max {
        state = S_MAINLOOP; /* continue */
        continue 'sm;
    }

    /* If minimizing, keep testing the rest of the expression and advancing
    the pointer while it matches the class. */

    if reptype == REPTYPE_MIN {
        state = L_A_ECLASS_MIN_LOOP;
        continue 'sm;
    }
    /* If maximizing, find the longest possible run, then work backwards. */
    else {
        (*F).fields.eclass_repeat.start_eptr = Feptr!();
        i = (*F).fields.eclass_repeat.min;
        while i < (*F).fields.eclass_repeat.max {
            let mut len: i32 = 1;
            if Feptr!() >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                break;
            }
            GETCHARLENTEST!(fc, Feptr!(), len, utf);
            if crate::xclass::_pcre2_eclass_8(
                fc,
                (*F).fields.eclass_repeat.eclass_data,
                (*F).fields
                    .eclass_repeat
                    .eclass_data
                    .add((*F).fields.eclass_repeat.eclass_len),
                (*mb).start_code,
                utf,
            ) == 0
            {
                break;
            }
            Feptr!() = Feptr!().add(len as usize);
            i += 1;
        }

        if reptype == REPTYPE_POS {
            state = S_MAINLOOP; /* continue - no backtracking */
            continue 'sm;
        }

        state = L_A_ECLASS_MAX_LOOP;
        continue 'sm;
    }
}
/* ---- chunk B: ARMS ---- */
/* ===================================================================== */
/* Match various character types when PCRE2_UCP is not set. These opcodes
are not generated when PCRE2_UCP is set - instead appropriate property
tests are compiled. */

/* C 2574 */
OP_NOT_DIGIT => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if CHMAX_255!(fc) != 0 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 2586 */
OP_DIGIT => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if CHMAX_255!(fc) == 0 || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 2598 */
OP_NOT_WHITESPACE => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if CHMAX_255!(fc) != 0 && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 2610 */
OP_WHITESPACE => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if CHMAX_255!(fc) == 0 || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 2622 */
OP_NOT_WORDCHAR => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if CHMAX_255!(fc) != 0 && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 2634 */
OP_WORDCHAR => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if CHMAX_255!(fc) == 0 || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 2646 */
OP_ANYNL => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    match fc {
        0x0du32 /* CHAR_CR */ => {
            if Feptr!() >= (*mb).end_subject {
                SCHECK_PARTIAL!();
            } else if *Feptr!() == 0x0au8 /* CHAR_LF */ {
                Feptr!() = Feptr!().add(1);
            }
        }

        0x0au32 /* CHAR_LF */ => {}

        0x0bu32 | 0x0cu32 | 0x85u32 | 0x2028u32 | 0x2029u32 => {
            if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        _ => {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 2681 */
OP_NOT_HSPACE => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    match fc {
        B_HSPACE_CASES!() => {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        _ => {}
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 2696 */
OP_HSPACE => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    match fc {
        B_HSPACE_CASES!() => {}
        _ => {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 2711 */
OP_NOT_VSPACE => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    match fc {
        B_VSPACE_CASES!() => {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        _ => {}
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 2726 */
OP_VSPACE => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    match fc {
        B_VSPACE_CASES!() => {}
        _ => {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Check the next character by Unicode property. We will get here only
if the support is in the binary; otherwise a compile-time error occurs. */

/* C 2748 */
OP_PROP | OP_NOTPROP => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    {
        let chartype: u32;
        let prop: *const ucd_record = GET_UCD!(fc);
        let notmatch: BOOL = (Fop!() as u32 == OP_NOTPROP) as BOOL;

        match *Fecode!().add(1) as u32 {
            PT_LAMP => {
                chartype = (*prop).chartype as u32;
                if ((chartype == ucp_Lu || chartype == ucp_Ll || chartype == ucp_Lt) as BOOL)
                    == notmatch
                {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            PT_GC => {
                if ((*Fecode!().add(2) as u32
                    == crate::tables::_pcre2_ucp_gentype_8[(*prop).chartype as usize])
                    as BOOL)
                    == notmatch
                {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            PT_PC => {
                if ((*Fecode!().add(2) == (*prop).chartype) as BOOL) == notmatch {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            PT_SC => {
                if ((*Fecode!().add(2) == (*prop).script) as BOOL) == notmatch {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            PT_SCX => {
                let ok: BOOL = ((*Fecode!().add(2) == (*prop).script)
                    || MAPBIT!(
                        crate::ucd::_pcre2_ucd_script_sets_8
                            .as_ptr()
                            .add(UCD_SCRIPTX_PROP!(prop) as usize),
                        *Fecode!().add(2)
                    ) != 0) as BOOL;
                if ok == notmatch {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            /* These are specials */
            PT_ALNUM => {
                chartype = (*prop).chartype as u32;
                if ((crate::tables::_pcre2_ucp_gentype_8[chartype as usize] == ucp_L
                    || crate::tables::_pcre2_ucp_gentype_8[chartype as usize] == ucp_N)
                    as BOOL)
                    == notmatch
                {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            /* Perl space used to exclude VT, but from Perl 5.18 it is included,
            which means that Perl space and POSIX space are now identical. PCRE
            was changed at release 8.34. */
            PT_SPACE | PT_PXSPACE => match fc {
                B_HSPACE_VSPACE_CASES!() => {
                    if notmatch != 0 {
                        rrc = MATCH_NOMATCH;
                        state = S_RETURN_SWITCH;
                        continue 'sm;
                    }
                }

                _ => {
                    if ((crate::tables::_pcre2_ucp_gentype_8[(*prop).chartype as usize] == ucp_Z)
                        as BOOL)
                        == notmatch
                    {
                        rrc = MATCH_NOMATCH;
                        state = S_RETURN_SWITCH;
                        continue 'sm;
                    }
                }
            },

            PT_WORD => {
                chartype = (*prop).chartype as u32;
                if ((crate::tables::_pcre2_ucp_gentype_8[chartype as usize] == ucp_L
                    || crate::tables::_pcre2_ucp_gentype_8[chartype as usize] == ucp_N
                    || chartype == ucp_Mn
                    || chartype == ucp_Pc) as BOOL)
                    == notmatch
                {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            PT_CLIST => {
                let mut cp: *const u32 = crate::ucd::_pcre2_ucd_caseless_sets_8
                    .as_ptr()
                    .add(*Fecode!().add(2) as usize);
                loop {
                    if fc < *cp {
                        if notmatch != 0 {
                            break;
                        } else {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                    }
                    let t = *cp;
                    cp = cp.add(1);
                    if fc == t {
                        if notmatch != 0 {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        } else {
                            break;
                        }
                    }
                }
            }

            PT_UCNC => {
                if ((fc == 0x24u32 /* CHAR_DOLLAR_SIGN */
                    || fc == 0x40u32 /* CHAR_COMMERCIAL_AT */
                    || fc == 0x60u32 /* CHAR_GRAVE_ACCENT */
                    || (fc >= 0xa0 && fc <= 0xd7ff)
                    || fc >= 0xe000) as BOOL)
                    == notmatch
                {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            PT_BIDICL => {
                if ((UCD_BIDICLASS_PROP!(prop) == *Fecode!().add(2) as u32) as BOOL) == notmatch {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            PT_BOOL => {
                let ok: BOOL = (MAPBIT!(
                    crate::ucd::_pcre2_ucd_boolprop_sets_8
                        .as_ptr()
                        .add(UCD_BPROPS_PROP!(prop) as usize),
                    *Fecode!().add(2)
                ) != 0) as BOOL;
                if ok == notmatch {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            /* This should never occur */
            _ => {
                return PCRE2_ERROR_INTERNAL;
            }
        }

        Fecode!() = Fecode!().add(3);
    }
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Match an extended Unicode sequence. We will get here only if the support
is in the binary; otherwise a compile-time error occurs. */

/* C 2889 */
OP_EXTUNI => {
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    } else {
        GETCHARINCTEST!(fc, Feptr!(), utf);
        Feptr!() = crate::extuni::_pcre2_extuni_8(
            fc,
            Feptr!(),
            (*mb).start_subject,
            (*mb).end_subject,
            utf,
            null_mut(),
        );
    }
    CHECK_PARTIAL!();
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Match a single character type repeatedly. Note that the property type
does not need to be in a stack frame as it is not used within an RMATCH()
loop.

  Lstart_eptr -> (*F).fields.type_repeat.start_eptr
  Lmin        -> (*F).fields.type_repeat.min
  Lmax        -> (*F).fields.type_repeat.max
  Lctype      -> (*F).fields.type_repeat.ctype
  Lpropvalue  -> (*F).fields.type_repeat.propvalue                     */

/* C 2919 */
OP_TYPEEXACT => {
    (*F).fields.type_repeat.max = GET2!(Fecode!(), 1);
    (*F).fields.type_repeat.min = (*F).fields.type_repeat.max;
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATTYPE; /* goto REPEATTYPE */
    continue 'sm;
}

/* C 2924 */
OP_TYPEUPTO | OP_TYPEMINUPTO => {
    (*F).fields.type_repeat.min = 0;
    (*F).fields.type_repeat.max = GET2!(Fecode!(), 1);
    reptype = if *Fecode!() as u32 == OP_TYPEMINUPTO {
        REPTYPE_MIN
    } else {
        REPTYPE_MAX
    };
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATTYPE; /* goto REPEATTYPE */
    continue 'sm;
}

/* C 2932 */
OP_TYPEPOSSTAR => {
    reptype = REPTYPE_POS;
    (*F).fields.type_repeat.min = 0;
    (*F).fields.type_repeat.max = u32::MAX;
    Fecode!() = Fecode!().add(1);
    state = L_REPEATTYPE; /* goto REPEATTYPE */
    continue 'sm;
}

/* C 2939 */
OP_TYPEPOSPLUS => {
    reptype = REPTYPE_POS;
    (*F).fields.type_repeat.min = 1;
    (*F).fields.type_repeat.max = u32::MAX;
    Fecode!() = Fecode!().add(1);
    state = L_REPEATTYPE; /* goto REPEATTYPE */
    continue 'sm;
}

/* C 2946 */
OP_TYPEPOSQUERY => {
    reptype = REPTYPE_POS;
    (*F).fields.type_repeat.min = 0;
    (*F).fields.type_repeat.max = 1;
    Fecode!() = Fecode!().add(1);
    state = L_REPEATTYPE; /* goto REPEATTYPE */
    continue 'sm;
}

/* C 2953 */
OP_TYPEPOSUPTO => {
    reptype = REPTYPE_POS;
    (*F).fields.type_repeat.min = 0;
    (*F).fields.type_repeat.max = GET2!(Fecode!(), 1);
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = L_REPEATTYPE; /* goto REPEATTYPE */
    continue 'sm;
}

/* C 2960 */
OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEQUERY
| OP_TYPEMINQUERY => {
    /* fc = *Fecode++ - OP_TYPESTAR; */
    {
        let t = *Fecode!();
        Fecode!() = Fecode!().add(1);
        fc = (t as u32).wrapping_sub(OP_TYPESTAR);
    }
    (*F).fields.type_repeat.min = rep_min[fc as usize];
    (*F).fields.type_repeat.max = rep_max[fc as usize];
    reptype = rep_typ[fc as usize];

    /* Fall through into the common code at REPEATTYPE. */
    state = L_REPEATTYPE;
    continue 'sm;
}

/* ===================================================================== */
/* Match a back reference, possibly repeatedly. Look past the end of the
item to see if there is repeat information following. The OP_REF and
OP_REFI opcodes are used for a reference to a numbered group or to a
non-duplicated named group. For a duplicated named group, OP_DNREF and
OP_DNREFI are used. In this case we must scan the list of groups to which
the name refers, and use the first one that is set.

  Lstart    -> (*F).fields.ref_repeat.start
  Loffset   -> (*F).fields.ref_repeat.offset
  Llength   -> (*F).fields.ref_repeat.length
  Lmin      -> (*F).fields.ref_repeat.min
  Lmax      -> (*F).fields.ref_repeat.max
  Lcaseless -> (*F).byte1
  Lcaseopts -> (*F).byte2                                              */

/* C 5249 */
OP_DNREF | OP_DNREFI => {
    (*F).byte1 = (Fop!() as u32 == OP_DNREFI) as u8;
    (*F).byte2 = if Fop!() as u32 == OP_DNREFI {
        *Fecode!().add(1 + 2 * IMM2_SIZE)
    } else {
        0
    };
    {
        let mut count: i32 = GET2!(Fecode!(), 1 + IMM2_SIZE) as i32;
        let mut slot: PCRE2_SPTR = (*mb)
            .name_table
            .add((GET2!(Fecode!(), 1) * (*mb).name_entry_size as u32) as usize);
        Fecode!() = Fecode!()
            .add(1 + 2 * IMM2_SIZE + (if Fop!() as u32 == OP_DNREFI { 1 } else { 0 }));

        loop {
            let t = count;
            count = count - 1;
            if !(t > 0) {
                break;
            }
            (*F).fields.ref_repeat.offset =
                (GET2!(slot, 0) << 1).wrapping_sub(2) as PCRE2_SIZE;
            if (*F).fields.ref_repeat.offset < Foffset_top!()
                && *Fovector!().add((*F).fields.ref_repeat.offset) != PCRE2_UNSET
            {
                break;
            }
            slot = slot.add((*mb).name_entry_size as usize);
        }
    }
    state = L_REF_REPEAT; /* goto REF_REPEAT */
    continue 'sm;
}

/* C 5267 */
OP_REF | OP_REFI => {
    (*F).byte1 = (Fop!() as u32 == OP_REFI) as u8;
    (*F).byte2 = if Fop!() as u32 == OP_REFI {
        *Fecode!().add(1 + IMM2_SIZE)
    } else {
        0
    };
    (*F).fields.ref_repeat.offset = (GET2!(Fecode!(), 1) << 1).wrapping_sub(2) as PCRE2_SIZE;
    Fecode!() =
        Fecode!().add(1 + IMM2_SIZE + (if Fop!() as u32 == OP_REFI { 1 } else { 0 }));

    /* Fall through into the common code at REF_REPEAT. */
    state = L_REF_REPEAT;
    continue 'sm;
}
/* ---- chunk C: ARMS ---- */

/* ===================================================================== */
/* BRAZERO, BRAMINZERO and SKIPZERO occur just before a non-possessive
bracket group, indicating that it may occur zero times. It may repeat
infinitely, or not at all - i.e. it could be ()* or ()? or even (){0} in
the pattern. Brackets with fixed upper repeat limits are compiled as a
number of copies, with the optional ones preceded by BRAZERO or BRAMINZERO.
Possessive groups with possible zero repeats are preceded by BRAPOSZERO. */

OP_BRAZERO => {
    Fecode!() = Fecode!().add(1);
    /* RMATCH(Fecode, RM9) */
    start_ecode = Fecode!();
    Freturn_id!() = RM9 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

OP_BRAMINZERO => {
    let mut next_ecode: PCRE2_SPTR = Fecode!().add(1);
    Fecode!() = next_ecode;
    loop {
        next_ecode = next_ecode.add(GET!(next_ecode, 1) as usize);
        if *next_ecode as u32 != OP_ALT {
            break;
        }
    }
    /* RMATCH(next_ecode + 1 + LINK_SIZE, RM10) */
    start_ecode = next_ecode.add(1 + LINK_SIZE);
    Freturn_id!() = RM10 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

OP_SKIPZERO => {
    let mut next_ecode: PCRE2_SPTR = Fecode!().add(1);
    loop {
        next_ecode = next_ecode.add(GET!(next_ecode, 1) as usize);
        if *next_ecode as u32 != OP_ALT {
            break;
        }
    }
    Fecode!() = next_ecode.add(1 + LINK_SIZE);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Handle possessive brackets with an unlimited repeat. The end of these
brackets will always be OP_KETRPOS, which returns MATCH_KETRPOS without
going further in the pattern.

Lstart_eptr   = F->fields.op_brapos.start_eptr
Lstart_group  = F->fields.op_brapos.start_group
Lframe_type   = F->fields.op_brapos.frame_type
Lmatched_once = F->byte1
Lzero_allowed = F->byte2 */

OP_BRAPOSZERO => {
    (*F).byte2 = TRUE as u8; /* Zero repeat is allowed */
    Fecode!() = Fecode!().add(1);
    if *Fecode!() as u32 == OP_CBRAPOS || *Fecode!() as u32 == OP_SCBRAPOS {
        /* goto POSSESSIVE_CAPTURE */
        state = L_POSSESSIVE_CAPTURE;
        continue 'sm;
    }
    /* goto POSSESSIVE_NON_CAPTURE */
    state = L_POSSESSIVE_NON_CAPTURE;
    continue 'sm;
}

OP_BRAPOS | OP_SBRAPOS => {
    (*F).byte2 = FALSE as u8; /* Zero repeat not allowed */
    /* fall through to POSSESSIVE_NON_CAPTURE */
    state = L_POSSESSIVE_NON_CAPTURE;
    continue 'sm;
}

OP_CBRAPOS | OP_SCBRAPOS => {
    (*F).byte2 = FALSE as u8; /* Zero repeat not allowed */
    /* fall through to POSSESSIVE_CAPTURE */
    state = L_POSSESSIVE_CAPTURE;
    continue 'sm;
}

/* ===================================================================== */
/* Handle non-capturing brackets that cannot match an empty string. When we
get to the final alternative within the brackets, as long as there are no
THEN's in the pattern, we can optimize by not recording a new backtracking
point. (Ideally we should test for a THEN within this group, but we don't
have that information.) Don't do this if we are at the very top level,
however, because that would make handling assertions and once-only brackets
messier when there is nothing to go back to.

Lframe_type = F->fields.op_bra.frame_type */

OP_BRA => {
    if (*mb).hasthen != 0 || Frdepth!() == 0 {
        (*F).fields.op_bra.frame_type = 0;
        /* goto GROUPLOOP */
        state = L_GROUPLOOP;
        continue 'sm;
    }
    state = S_BRA_LOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Handle a capturing bracket, other than those that are possessive with an
unlimited repeat. */

OP_CBRA | OP_SCBRA => {
    (*F).fields.op_bra.frame_type = GF_CAPTURE | GET2!(Fecode!(), 1 + LINK_SIZE);
    /* goto GROUPLOOP */
    state = L_GROUPLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Atomic groups and non-capturing brackets that can match an empty string
must record a backtracking point and also set up a chained frame. */

OP_ONCE | OP_SCRIPT_RUN | OP_SBRA => {
    (*F).fields.op_bra.frame_type = GF_NOCAPTURE;
    /* fall through to GROUPLOOP */
    state = L_GROUPLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Pattern recursion either matches the current regex, or some
subexpression. The offset data is the offset to the starting bracket from
the start of the whole pattern. This is so that it works from duplicated
subpatterns. For a whole-pattern recursion, we have to infer the number
zero.

Lstart_branch = F->fields.op_recurse.start_branch
Lframe_type   = F->fields.op_recurse.frame_type */

OP_RECURSE => {
    bracode = (*mb).start_code.add(GET!(Fecode!(), 1) as usize);
    number = if bracode == (*mb).start_code {
        0
    } else {
        GET2!(bracode, 1 + LINK_SIZE)
    };

    /* If we are already in a pattern recursion, check for repeating the same
    one without changing the subject pointer or the last referenced character
    in the subject. This should catch convoluted mutual recursions; some
    simple cases are caught at compile time. However, there are rare cases when
    this check needs to be turned off. In this case, actual recursion loops
    will be caught by the match or heap limits. */

    if Fcurrent_recurse!() != RECURSE_UNSET {
        offset = Flast_group_offset!();
        while offset != PCRE2_UNSET {
            N = ((*match_data).heapframes as *mut u8).add(offset) as *mut heapframe;
            P = ((N as *mut u8).sub(frame_size)) as *mut heapframe;
            if (*N).group_frame_type == (GF_RECURSE | number) {
                if Feptr!() == (*P).eptr
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

    /* Remember the current last referenced character and then run the
    recursion branch by branch. */

    (*F).recurse_last_used = (*mb).last_used_ptr;
    (*F).fields.op_recurse.start_branch = bracode;
    (*F).fields.op_recurse.frame_type = GF_RECURSE | number;

    state = S_RECURSE_LOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Positive assertions are like other groups except that PCRE doesn't allow
the effect of (*THEN) to escape beyond an assertion; it is therefore
treated as NOMATCH. (*ACCEPT) is treated as successful assertion, with its
captures and mark retained. Any other return is an error. */

OP_ASSERT | OP_ASSERTBACK | OP_ASSERT_NA | OP_ASSERTBACK_NA => {
    state = S_ASSERT_LOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Handle negative assertions. Loop for each non-matching branch as for
positive assertions. */

OP_ASSERT_NOT | OP_ASSERTBACK_NOT => {
    state = S_ASSERTNOT_LOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Handle scan substring operation.

Lsaved_end_subject = F->fields.op_assert_scs.saved_end_subject
Lsaved_eptr        = F->fields.op_assert_scs.saved_eptr
Ltrue_end_extra    = F->fields.op_assert_scs.true_end_extra
Lsaved_moptions    = F->fields.op_assert_scs.saved_moptions */

OP_ASSERT_SCS => {
    length = 0;
    scs_ecode = Fecode!().add(1 + LINK_SIZE);

    /* Disable compiler warning. */
    offset = 0;

    state = S_SCS_CREF_LOOP;
    continue 'sm;
}

/* ===================================================================== */
/* The callout item calls an external function, if one is provided, passing
details of the match so far. This is mainly for debugging, though the
function is able to force a failure. */

OP_CALLOUT | OP_CALLOUT_STR => {
    rrc = do_callout(F, mb, &mut length);
    if rrc > 0 {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if rrc < 0 {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(length);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Conditional group: compilation checked that there are no more than two
branches. If the condition is false, skipping the first branch takes us
past the end of the item if there is only one branch, but that's exactly
what we want.

Lstart_branch = F->fields.op_cond.start_branch
Llength       = F->fields.op_cond.length
Lpositive     = F->byte1 */

OP_COND | OP_SCOND => {
    /* The variable Llength will be added to Fecode when the condition is
    false, to get to the second branch. Setting it to the offset to the ALT or
    KET, then incrementing Fecode achieves this effect. However, if the second
    branch is non-existent, we must point to the KET so that the end of the
    group is correctly processed. We now have Fecode pointing to the condition
    or callout. */

    (*F).fields.op_cond.length = GET!(Fecode!(), 1) as PCRE2_SIZE; /* Offset to the second branch */
    if *Fecode!().add((*F).fields.op_cond.length) as u32 != OP_ALT {
        (*F).fields.op_cond.length -= 1 + LINK_SIZE;
    }
    Fecode!() = Fecode!().add(1 + LINK_SIZE); /* From this opcode */

    /* Because of the way auto-callout works during compile, a callout item is
    inserted between OP_COND and an assertion condition. Such a callout can
    also be inserted manually. */

    if *Fecode!() as u32 == OP_CALLOUT || *Fecode!() as u32 == OP_CALLOUT_STR {
        rrc = do_callout(F, mb, &mut length);
        if rrc > 0 {
            /* RRETURN(MATCH_NOMATCH) */
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        if rrc < 0 {
            /* RRETURN(rrc) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }

        /* Advance Fecode past the callout, so it now points to the condition. We
        must adjust Llength so that the value of Fecode+Llength is unchanged. */

        Fecode!() = Fecode!().add(length);
        (*F).fields.op_cond.length -= length;
    }

    /* Test the various possible conditions */

    condition = FALSE;
    match *Fecode!() as u32 {
        OP_RREF => {
            /* Group recursion test */
            if Fcurrent_recurse!() != RECURSE_UNSET {
                number = GET2!(Fecode!(), 1);
                condition = (number == RREF_ANY || number == Fcurrent_recurse!()) as BOOL;
            }
        }

        OP_DNRREF => {
            /* Duplicate named group recursion test */
            if Fcurrent_recurse!() != RECURSE_UNSET {
                let mut count: i32 = GET2!(Fecode!(), 1 + IMM2_SIZE) as i32;
                let mut slot: PCRE2_SPTR = (*mb)
                    .name_table
                    .add((GET2!(Fecode!(), 1) as usize) * ((*mb).name_entry_size as usize));
                loop {
                    let c_ = count;
                    count -= 1;
                    if !(c_ > 0) {
                        break;
                    }
                    number = GET2!(slot, 0);
                    condition = (number == Fcurrent_recurse!()) as BOOL;
                    if condition != 0 {
                        break;
                    }
                    slot = slot.add((*mb).name_entry_size as usize);
                }
            }
        }

        OP_CREF => {
            /* Numbered group used test */
            offset = ((GET2!(Fecode!(), 1) << 1).wrapping_sub(2)) as PCRE2_SIZE; /* Doubled ref number */
            condition = (offset < Foffset_top!() && *Fovector!().add(offset) != PCRE2_UNSET) as BOOL;
        }

        OP_DNCREF => {
            /* Duplicate named group used test */
            let mut count: i32 = GET2!(Fecode!(), 1 + IMM2_SIZE) as i32;
            let mut slot: PCRE2_SPTR = (*mb)
                .name_table
                .add((GET2!(Fecode!(), 1) as usize) * ((*mb).name_entry_size as usize));
            loop {
                let c_ = count;
                count -= 1;
                if !(c_ > 0) {
                    break;
                }
                offset = ((GET2!(slot, 0) << 1).wrapping_sub(2)) as PCRE2_SIZE;
                condition =
                    (offset < Foffset_top!() && *Fovector!().add(offset) != PCRE2_UNSET) as BOOL;
                if condition != 0 {
                    break;
                }
                slot = slot.add((*mb).name_entry_size as usize);
            }
        }

        OP_FALSE | OP_FAIL => {
            /* The assertion (?!) becomes OP_FAIL */
        }

        OP_TRUE => {
            condition = TRUE;
        }

        /* The condition is an assertion. Run code similar to the assertion code
        above. */
        _ => {
            (*F).byte1 =
                (*Fecode!() as u32 == OP_ASSERT || *Fecode!() as u32 == OP_ASSERTBACK) as u8;
            (*F).fields.op_cond.start_branch = Fecode!();
            state = S_COND_ASSERT_LOOP;
            continue 'sm;
        }
    }

    state = S_COND_CHOOSE;
    continue 'sm;
}

/* ========================================================================= */
/*                  End of start of parenthesis opcodes                      */
/* ========================================================================= */

/* ===================================================================== */
/* Move the subject pointer back by one fixed amount. This occurs at the
start of each branch that has a fixed length in a lookbehind assertion. If
we are too close to the start to move back, fail. When working with UTF-8
we move back a number of characters, not bytes. */

OP_REVERSE => {
    number = GET2!(Fecode!(), 1);
    if utf != 0 {
        /* We used to do a simpler `while (number-- > 0)` but that triggers
        clang's unsigned integer overflow sanitizer. */
        while number > 0 {
            number -= 1;
            if Feptr!() <= (*mb).check_subject {
                /* RRETURN(MATCH_NOMATCH) */
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            Feptr!() = Feptr!().offset(-1);
            BACKCHAR!(Feptr!());
        }
    } else {
        /* No UTF support, or not in UTF mode: count is code unit count */
        if (number as isize) > Feptr!().offset_from((*mb).start_subject) {
            /* RRETURN(MATCH_NOMATCH) */
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        Feptr!() = Feptr!().sub(number as usize);
    }

    /* Save the earliest consulted character, then skip to next opcode */

    if Feptr!() < (*mb).start_used_ptr {
        (*mb).start_used_ptr = Feptr!();
    }
    Fecode!() = Fecode!().add(1 + IMM2_SIZE);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Move the subject pointer back by a variable amount. This occurs at the
start of each branch of a lookbehind assertion when the branch has a
variable, but limited, length. A loop is needed to try matching the branch
after moving back different numbers of characters. If we are too close to
the start to move back even the minimum amount, fail. When working with
UTF-8 we move back a number of characters, not bytes.

Lmin = F->fields.op_vreverse.min
Lmax = F->fields.op_vreverse.max */

OP_VREVERSE => {
    (*F).fields.op_vreverse.min = GET2!(Fecode!(), 1);
    (*F).fields.op_vreverse.max = GET2!(Fecode!(), 1 + IMM2_SIZE);

    /* Move back by the maximum branch length and then work forwards. This
    ensures that items such as \d{3,5} get the maximum length, which is
    relevant for captures, and makes for Perl compatibility. */

    if utf != 0 {
        i = 0;
        while i < (*F).fields.op_vreverse.max {
            if Feptr!() == (*mb).start_subject {
                if i < (*F).fields.op_vreverse.min {
                    /* RRETURN(MATCH_NOMATCH) */
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                (*F).fields.op_vreverse.max = i;
                break;
            }
            Feptr!() = Feptr!().offset(-1);
            BACKCHAR!(Feptr!());
            i += 1;
        }
    } else {
        /* No UTF support or not in UTF mode */
        let diff: isize = Feptr!().offset_from((*mb).start_subject);
        let available: u32 = if diff > 65535 {
            65535
        } else if diff > 0 {
            diff as i32 as u32
        } else {
            0
        };
        if (*F).fields.op_vreverse.min > available {
            /* RRETURN(MATCH_NOMATCH) */
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        if (*F).fields.op_vreverse.max > available {
            (*F).fields.op_vreverse.max = available;
        }
        Feptr!() = Feptr!().sub((*F).fields.op_vreverse.max as usize);
    }

    /* Now try matching, moving forward one character on failure, until we
    reach the minimum back length. */

    state = S_VREVERSE_LOOP;
    continue 'sm;
}

/* ===================================================================== */
/* An alternation is the end of a branch; scan along to find the end of the
bracketed group. */

OP_ALT => {
    branch_end = Fecode!();
    loop {
        Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
        if *Fecode!() as u32 != OP_ALT {
            break;
        }
    }
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* The end of a parenthesized group. For all but OP_BRA and OP_COND, the
starting frame was added to the chained frames in order to remember the
starting subject position for the group. (Not true for OP_BRA when it's a
whole pattern recursion, but that is handled separately below.)*/

OP_KET | OP_KETRMIN | OP_KETRMAX | OP_KETRPOS => {
    bracode = Fecode!().sub(GET!(Fecode!(), 1) as usize);

    if branch_end.is_null() {
        branch_end = Fecode!();
    }
    branch_start = bracode;
    while branch_start.add(GET!(branch_start, 1) as usize) != branch_end {
        branch_start = branch_start.add(GET!(branch_start, 1) as usize);
    }
    branch_end = null();

    /* Point N to the frame at the start of the most recent group, and P to its
    predecessor. Remember the subject pointer at the start of the group. */

    if *bracode as u32 != OP_BRA && *bracode as u32 != OP_COND {
        N = ((*match_data).heapframes as *mut u8).add(Flast_group_offset!()) as *mut heapframe;
        P = ((N as *mut u8).sub(frame_size)) as *mut heapframe;
        Flast_group_offset!() = (*P).last_group_offset;

        /* If we are at the end of an assertion that is a condition, first check
        to see if we are at the end of a variable-length branch in a lookbehind.
        If this is the case and we have not landed on the current character,
        return no match. Compare code below for non-condition lookbehinds. In
        other cases, return a match, discarding any intermediate backtracking
        points. Copy back the mark setting and the captures into the frame before
        N so that they are set on return. Doing this for all assertions, both
        positive and negative, seems to match what Perl does. */

        if (*N).group_frame_type == GF_CONDASSERT {
            if (*bracode as u32 == OP_ASSERTBACK || *bracode as u32 == OP_ASSERTBACK_NOT)
                && *branch_start.add(1 + LINK_SIZE) as u32 == OP_VREVERSE
                && Feptr!() != (*P).eptr
            {
                /* RRETURN(MATCH_NOMATCH) */
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            copy_nonoverlapping(
                Fovector!() as *const u8,
                (P as *mut u8).add(OVECTOR_OFFSET_IN_HEAPFRAME),
                Foffset_top!() * core::mem::size_of::<PCRE2_SIZE>(),
            );
            (*P).offset_top = Foffset_top!();
            (*P).mark = Fmark!();
            Fback_frame!() = (F as usize) - (P as usize);
            /* RRETURN(MATCH_MATCH) */
            rrc = MATCH_MATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    } else {
        P = null_mut(); /* Indicates starting frame not recorded */
    }

    /* The group was not a conditional assertion. */

    'ket_switch: {
        match *bracode as u32 {
            /* Whole pattern recursion is handled as a recursion into group 0, but
            the entire pattern is wrapped in OP_BRA/OP_KET rather than a capturing
            group - a design mistake: it should perhaps have been capture group 0.
            Anyway, that means the end of such recursion must be handled here. It is
            detected by checking for an immediately following OP_END when we are
            recursing in group 0. If this is not the end of a whole-pattern
            recursion, there is nothing to be done. */
            OP_BRA => {
                if Fcurrent_recurse!() != 0 || *Fecode!().add(1 + LINK_SIZE) as u32 != OP_END {
                    break 'ket_switch;
                }

                /* It is the end of whole-pattern recursion. */

                offset = Flast_group_offset!();

                /* Corrupted heapframes?. Trigger an assert and return an error */
                if offset == PCRE2_UNSET {
                    return PCRE2_ERROR_INTERNAL;
                }

                N = ((*match_data).heapframes as *mut u8).add(offset) as *mut heapframe;
                P = ((N as *mut u8).sub(frame_size)) as *mut heapframe;
                Flast_group_offset!() = (*P).last_group_offset;

                /* Reinstate the previous set of captures and then carry on after the
                recursion call. */

                Fecode!() = (*P).ecode.add(1 + LINK_SIZE);

                if *Fecode!() as u32 != OP_CREF {
                    copy_nonoverlapping(
                        (*P).ovector.as_ptr() as *const u8,
                        (*F).ovector.as_mut_ptr() as *mut u8,
                        Foffset_top!() * core::mem::size_of::<PCRE2_SIZE>(),
                    );
                    Foffset_top!() = (*P).offset_top;
                } else {
                    recurse_update_offsets(F, P);
                }

                Fcapture_last!() = (*P).capture_last;
                Fcurrent_recurse!() = (*P).current_recurse;
                /* continue: with next opcode */
                state = S_MAINLOOP;
                continue 'sm;
            }

            OP_COND | OP_SCOND => {
                /* No need to do anything for these */
            }

            /* Non-atomic positive assertions are like OP_BRA, except that the
            subject pointer must be put back to where it was at the start of the
            assertion. For a variable lookbehind, check its end point. */
            OP_ASSERTBACK_NA => {
                if *branch_start.add(1 + LINK_SIZE) as u32 == OP_VREVERSE
                    && Feptr!() != (*P).eptr
                {
                    /* RRETURN(MATCH_NOMATCH) */
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                /* Fall through to OP_ASSERT_NA */
                if Feptr!() > (*mb).last_used_ptr {
                    (*mb).last_used_ptr = Feptr!();
                }
                Feptr!() = (*P).eptr;
            }

            OP_ASSERT_NA => {
                if Feptr!() > (*mb).last_used_ptr {
                    (*mb).last_used_ptr = Feptr!();
                }
                Feptr!() = (*P).eptr;
            }

            /* Atomic positive assertions are like OP_ONCE, except that in addition
            the subject pointer must be put back to where it was at the start of the
            assertion. For a variable lookbehind, check its end point. */
            OP_ASSERTBACK => {
                if *branch_start.add(1 + LINK_SIZE) as u32 == OP_VREVERSE
                    && Feptr!() != (*P).eptr
                {
                    /* RRETURN(MATCH_NOMATCH) */
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                /* Fall through to OP_ASSERT */
                if Feptr!() > (*mb).last_used_ptr {
                    (*mb).last_used_ptr = Feptr!();
                }
                Feptr!() = (*P).eptr;
                /* Fall through to OP_ONCE */
                Fback_frame!() = (F as usize) - (P as usize);
                loop {
                    let y: u32 = GET!((*P).ecode, 1);
                    if *(*P).ecode.add(y as usize) as u32 != OP_ALT {
                        break;
                    }
                    (*P).ecode = (*P).ecode.add(y as usize);
                }
            }

            OP_ASSERT => {
                if Feptr!() > (*mb).last_used_ptr {
                    (*mb).last_used_ptr = Feptr!();
                }
                Feptr!() = (*P).eptr;
                /* Fall through to OP_ONCE */
                Fback_frame!() = (F as usize) - (P as usize);
                loop {
                    let y: u32 = GET!((*P).ecode, 1);
                    if *(*P).ecode.add(y as usize) as u32 != OP_ALT {
                        break;
                    }
                    (*P).ecode = (*P).ecode.add(y as usize);
                }
            }

            /* For an atomic group, discard internal backtracking points. We must
            also ensure that any remaining branches within the top-level of the group
            are not tried. Do this by adjusting the code pointer within the backtrack
            frame so that it points to the final branch. */
            OP_ONCE => {
                Fback_frame!() = (F as usize) - (P as usize);
                loop {
                    let y: u32 = GET!((*P).ecode, 1);
                    if *(*P).ecode.add(y as usize) as u32 != OP_ALT {
                        break;
                    }
                    (*P).ecode = (*P).ecode.add(y as usize);
                }
            }

            /* A matching negative assertion returns MATCH, which is turned into
            NOMATCH at the assertion level. For a variable lookbehind, check its end
            point. */
            OP_ASSERTBACK_NOT => {
                if *branch_start.add(1 + LINK_SIZE) as u32 == OP_VREVERSE
                    && Feptr!() != (*P).eptr
                {
                    /* RRETURN(MATCH_NOMATCH) */
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                /* Fall through to OP_ASSERT_NOT */
                /* RRETURN(MATCH_MATCH) */
                rrc = MATCH_MATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }

            OP_ASSERT_NOT => {
                /* RRETURN(MATCH_MATCH) */
                rrc = MATCH_MATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }

            /* A scan substring group must preserve the current end_subject,
            and restore it before the backtracking is performed into its sub
            pattern. */
            OP_ASSERT_SCS => {
                (*F).fields.op_assert_scs.saved_end_subject = (*mb).end_subject;
                (*mb).end_subject = (*P).fields.op_assert_scs.saved_end_subject;
                (*mb).true_end_subject = (*mb)
                    .end_subject
                    .add((*P).fields.op_assert_scs.true_end_extra);
                Feptr!() = (*P).fields.op_assert_scs.saved_eptr;

                /* RMATCH(Fecode + 1 + LINK_SIZE, RM39) */
                start_ecode = Fecode!().add(1 + LINK_SIZE);
                Freturn_id!() = RM39 as u8;
                state = S_MATCH_RECURSE;
                continue 'sm;
            }

            /* At the end of a script run, apply the script-checking rules. This code
            will never by exercised if Unicode support it not compiled, because in
            that environment script runs cause an error at compile time. */
            OP_SCRIPT_RUN => {
                if crate::script_run::_pcre2_script_run_8((*P).eptr, Feptr!(), utf) == 0 {
                    /* RRETURN(MATCH_NOMATCH) */
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }

            /* Whole-pattern recursion is coded as a recurse into group 0, and is
            handled with OP_BRA above. Other recursion is handled here. */
            OP_CBRA | OP_CBRAPOS | OP_SCBRA | OP_SCBRAPOS => {
                number = GET2!(bracode, 1 + LINK_SIZE);

                /* Handle a recursively called group. We reinstate the previous set of
                captures and then carry on after the recursion call. */

                if Fcurrent_recurse!() == number {
                    P = ((N as *mut u8).sub(frame_size)) as *mut heapframe;
                    Fecode!() = (*P).ecode.add(1 + LINK_SIZE);

                    if *Fecode!() as u32 != OP_CREF {
                        copy_nonoverlapping(
                            (*P).ovector.as_ptr() as *const u8,
                            (*F).ovector.as_mut_ptr() as *mut u8,
                            Foffset_top!() * core::mem::size_of::<PCRE2_SIZE>(),
                        );
                        Foffset_top!() = (*P).offset_top;
                    } else {
                        recurse_update_offsets(F, P);
                    }

                    Fcapture_last!() = (*P).capture_last;
                    Fcurrent_recurse!() = (*P).current_recurse;
                    /* continue: with next opcode */
                    state = S_MAINLOOP;
                    continue 'sm;
                }

                /* Deal with actual capturing. */

                offset = ((number << 1).wrapping_sub(2)) as PCRE2_SIZE;
                Fcapture_last!() = number;
                *Fovector!().add(offset) =
                    ((*P).eptr as usize) - ((*mb).start_subject as usize);
                *Fovector!().add(offset + 1) =
                    (Feptr!() as usize) - ((*mb).start_subject as usize);
                if offset >= Foffset_top!() {
                    Foffset_top!() = offset + 2;
                }
            }

            _ => {}
        } /* End actions relating to the starting opcode */
    }

    /* OP_KETRPOS is a possessive repeating ket. Remember the current position,
    and return the MATCH_KETRPOS. This makes it possible to do the repeats one
    at a time from the outer level. This must precede the empty string test -
    in this case that test is done at the outer level. */

    if *Fecode!() as u32 == OP_KETRPOS {
        copy_nonoverlapping(
            (F as *const u8).add(EPTR_OFFSET_IN_HEAPFRAME),
            (P as *mut u8).add(EPTR_OFFSET_IN_HEAPFRAME),
            frame_copy_size,
        );
        /* RRETURN(MATCH_KETRPOS) */
        rrc = MATCH_KETRPOS;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }

    /* Handle the different kinds of closing brackets. A non-repeating ket
    needs no special action, just continuing at this level. This also happens
    for the repeating kets if the group matched no characters, in order to
    forcibly break infinite loops. Otherwise, the repeating kets try the rest
    of the pattern or restart from the preceding bracket, in the appropriate
    order. */

    if Fop!() as u32 != OP_KET && (P.is_null() || Feptr!() != (*P).eptr) {
        if Fop!() as u32 == OP_KETRMIN {
            /* RMATCH(Fecode + 1 + LINK_SIZE, RM6) */
            start_ecode = Fecode!().add(1 + LINK_SIZE);
            Freturn_id!() = RM6 as u8;
            state = S_MATCH_RECURSE;
            continue 'sm;
        }

        /* Repeat the maximum number of times (KETRMAX) */

        /* RMATCH(bracode, RM7) */
        start_ecode = bracode;
        Freturn_id!() = RM7 as u8;
        state = S_MATCH_RECURSE;
        continue 'sm;
    }

    /* Carry on at this level for a non-repeating ket, or after matching an
    empty string, or after repeating for a maximum number of times. */

    Fecode!() = Fecode!().add(1 + LINK_SIZE);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Start and end of line assertions, not multiline mode. */

OP_CIRC => {
    /* Start of line, unless PCRE2_NOTBOL is set. */
    if Feptr!() != (*mb).start_subject || ((*mb).moptions & PCRE2_NOTBOL) != 0 {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

OP_SOD => {
    /* Unconditional start of subject */
    if Feptr!() != (*mb).start_subject {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* When PCRE2_NOTEOL is unset, assert before the subject end, or a
terminating newline unless PCRE2_DOLLAR_ENDONLY is set. */

OP_DOLL => {
    if ((*mb).moptions & PCRE2_NOTEOL) != 0 {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if ((*mb).poptions & PCRE2_DOLLAR_ENDONLY) == 0 {
        /* goto ASSERT_NL_OR_EOS */
        state = L_ASSERT_NL_OR_EOS;
        continue 'sm;
    }

    /* Fall through to OP_EOD */
    /* Unconditional end of subject assertion (\z). */
    if Feptr!() < (*mb).true_end_subject {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if (*mb).partial != 0 {
        (*mb).hitend = TRUE;
        if (*mb).partial > 1 {
            return PCRE2_ERROR_PARTIAL;
        }
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

OP_EOD => {
    if Feptr!() < (*mb).true_end_subject {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if (*mb).partial != 0 {
        (*mb).hitend = TRUE;
        if (*mb).partial > 1 {
            return PCRE2_ERROR_PARTIAL;
        }
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* End of subject or ending \n assertion (\Z) */

OP_EODN => {
    /* fall into ASSERT_NL_OR_EOS */
    state = L_ASSERT_NL_OR_EOS;
    continue 'sm;
}

/* ===================================================================== */
/* Start and end of line assertions, multiline mode. */

/* Start of subject unless notbol, or after any newline except for one at
the very end, unless PCRE2_ALT_CIRCUMFLEX is set. */

OP_CIRCM => {
    if ((*mb).moptions & PCRE2_NOTBOL) != 0 && Feptr!() == (*mb).start_subject {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() != (*mb).start_subject
        && ((Feptr!() == (*mb).end_subject && ((*mb).poptions & PCRE2_ALT_CIRCUMFLEX) == 0)
            || WAS_NEWLINE!(Feptr!()) == 0)
    {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* Assert before any newline, or before end of subject unless noteol is
set. */

OP_DOLLM => {
    if Feptr!() < (*mb).end_subject {
        if IS_NEWLINE!(Feptr!()) == 0 {
            if (*mb).partial != 0
                && Feptr!().add(1) >= (*mb).end_subject
                && (*mb).nltype == NLTYPE_FIXED
                && (*mb).nllen == 2
                && *Feptr!() == (*mb).nl[0]
            {
                (*mb).hitend = TRUE;
                if (*mb).partial > 1 {
                    return PCRE2_ERROR_PARTIAL;
                }
            }
            /* RRETURN(MATCH_NOMATCH) */
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    } else {
        if ((*mb).moptions & PCRE2_NOTEOL) != 0 {
            /* RRETURN(MATCH_NOMATCH) */
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        SCHECK_PARTIAL!();
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Start of match assertion */

OP_SOM => {
    if Feptr!() != (*mb).start_subject.add((*mb).start_offset) {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Reset the start of match point */

OP_SET_SOM => {
    Fstart_match!() = Feptr!();
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Word boundary assertions. Find out if the previous and current
characters are "word" characters. It takes a bit more work in UTF mode.
Characters > 255 are assumed to be "non-word" characters when PCRE2_UCP is
not set. When it is set, use Unicode properties if available, even when not
in UTF mode. Remember the earliest and latest consulted characters. */

OP_NOT_WORD_BOUNDARY | OP_WORD_BOUNDARY | OP_NOT_UCP_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY => {
    if Feptr!() == (*mb).check_subject {
        prev_is_word = FALSE;
    } else {
        let mut lastptr: PCRE2_SPTR = Feptr!().wrapping_sub(1);
        if utf != 0 {
            BACKCHAR!(lastptr);
            GETCHAR!(fc, lastptr);
        } else {
            fc = *lastptr as u32;
        }
        if lastptr < (*mb).start_used_ptr {
            (*mb).start_used_ptr = lastptr;
        }
        if Fop!() as u32 == OP_UCP_WORD_BOUNDARY || Fop!() as u32 == OP_NOT_UCP_WORD_BOUNDARY {
            let chartype: i32 = UCD_CHARTYPE!(fc) as i32;
            let category: i32 = crate::tables::_pcre2_ucp_gentype_8[chartype as usize] as i32;
            prev_is_word = (category == ucp_L as i32
                || category == ucp_N as i32
                || chartype == ucp_Mn as i32
                || chartype == ucp_Pc as i32) as BOOL;
        } else {
            prev_is_word = (CHMAX_255!(fc) != 0
                && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0) as BOOL;
        }
    }

    /* Get status of next character */

    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        cur_is_word = FALSE;
    } else {
        let mut nextptr: PCRE2_SPTR = Feptr!().add(1);
        if utf != 0 {
            FORWARDCHARTEST!(nextptr, (*mb).end_subject);
            GETCHAR!(fc, Feptr!());
        } else {
            fc = *Feptr!() as u32;
        }
        if nextptr > (*mb).last_used_ptr {
            (*mb).last_used_ptr = nextptr;
        }
        if Fop!() as u32 == OP_UCP_WORD_BOUNDARY || Fop!() as u32 == OP_NOT_UCP_WORD_BOUNDARY {
            let chartype: i32 = UCD_CHARTYPE!(fc) as i32;
            let category: i32 = crate::tables::_pcre2_ucp_gentype_8[chartype as usize] as i32;
            cur_is_word = (category == ucp_L as i32
                || category == ucp_N as i32
                || chartype == ucp_Mn as i32
                || chartype == ucp_Pc as i32) as BOOL;
        } else {
            cur_is_word = (CHMAX_255!(fc) != 0
                && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0) as BOOL;
        }
    }

    /* Now see if the situation is what we want */

    let this_op: u8 = *Fecode!();
    Fecode!() = Fecode!().add(1);
    let want: BOOL = if this_op as u32 == OP_WORD_BOUNDARY
        || Fop!() as u32 == OP_UCP_WORD_BOUNDARY
    {
        (cur_is_word == prev_is_word) as BOOL
    } else {
        (cur_is_word != prev_is_word) as BOOL
    };
    if want != 0 {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = S_MAINLOOP;
    continue 'sm;
}

/* ===================================================================== */
/* Backtracking (*VERB)s, with and without arguments. Note that if the
pattern is successfully matched, we do not come back from RMATCH. */

OP_MARK => {
    (*mb).nomatch_mark = Fecode!().add(2);
    Fmark!() = (*mb).nomatch_mark;
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode] + Fecode[1], RM12) */
    start_ecode = Fecode!()
        .add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize)
        .add(*Fecode!().add(1) as usize);
    Freturn_id!() = RM12 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

OP_FAIL => {
    /* RRETURN(MATCH_NOMATCH) */
    rrc = MATCH_NOMATCH;
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* Record the current recursing group number in mb->verb_current_recurse
when a backtracking return such as MATCH_COMMIT is given. This enables the
recurse processing to catch verbs from within the recursion. */

OP_COMMIT => {
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode], RM13) */
    start_ecode = Fecode!().add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize);
    Freturn_id!() = RM13 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

OP_COMMIT_ARG => {
    (*mb).nomatch_mark = Fecode!().add(2);
    Fmark!() = (*mb).nomatch_mark;
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode] + Fecode[1], RM36) */
    start_ecode = Fecode!()
        .add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize)
        .add(*Fecode!().add(1) as usize);
    Freturn_id!() = RM36 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

OP_PRUNE => {
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode], RM14) */
    start_ecode = Fecode!().add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize);
    Freturn_id!() = RM14 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

OP_PRUNE_ARG => {
    (*mb).nomatch_mark = Fecode!().add(2);
    Fmark!() = (*mb).nomatch_mark;
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode] + Fecode[1], RM15) */
    start_ecode = Fecode!()
        .add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize)
        .add(*Fecode!().add(1) as usize);
    Freturn_id!() = RM15 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

OP_SKIP => {
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode], RM16) */
    start_ecode = Fecode!().add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize);
    Freturn_id!() = RM16 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

/* Note that, for Perl compatibility, SKIP with an argument does NOT set
nomatch_mark. When a pattern match ends with a SKIP_ARG for which there was
not a matching mark, we have to re-run the match, ignoring the SKIP_ARG
that failed and any that precede it (either they also failed, or were not
triggered). To do this, we maintain a count of executed SKIP_ARGs. If a
SKIP_ARG gets to top level, the match is re-run with mb->ignore_skip_arg
set to the count of the one that failed. */

OP_SKIP_ARG => {
    (*mb).skip_arg_count += 1;
    if (*mb).skip_arg_count <= (*mb).ignore_skip_arg {
        Fecode!() = Fecode!()
            .add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize)
            .add(*Fecode!().add(1) as usize);
        state = S_MAINLOOP;
        continue 'sm;
    }
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode] + Fecode[1], RM17) */
    start_ecode = Fecode!()
        .add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize)
        .add(*Fecode!().add(1) as usize);
    Freturn_id!() = RM17 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

/* For THEN (and THEN_ARG) we pass back the address of the opcode, so that
the branch in which it occurs can be determined. */

OP_THEN => {
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode], RM18) */
    start_ecode = Fecode!().add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize);
    Freturn_id!() = RM18 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

OP_THEN_ARG => {
    (*mb).nomatch_mark = Fecode!().add(2);
    Fmark!() = (*mb).nomatch_mark;
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode] + Fecode[1], RM19) */
    start_ecode = Fecode!()
        .add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize)
        .add(*Fecode!().add(1) as usize);
    Freturn_id!() = RM19 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}


                    /* ================================================================= */
                    /* Unrecognized opcodes are internal errors. */
                    _ => {
                        return PCRE2_ERROR_INTERNAL;
                    }
                }
                /* Do not insert any code in here without much thought; it is assumed
                that "continue" in the code above comes out to here to repeat the main
                loop. */
                #[allow(unreachable_code)]
                {
                    state = S_MAINLOOP;
                    continue 'sm;
                }
            }

            /* ============================================================= */
            /* The RRETURN() macro jumps here. The number that is saved in
            Freturn_id indicates which label we actually want to return to. The
            value in Frdepth is the index number of the frame in the vector. The
            return value has been placed in rrc. */
            S_RETURN_SWITCH => {
                if Feptr!() > (*mb).last_used_ptr {
                    (*mb).last_used_ptr = Feptr!();
                }
                if Frdepth!() == 0 {
                    return rrc; /* Exit from the top level */
                }
                F = ((F as *mut u8).sub(Fback_frame!())) as *mut heapframe; /* Backtrack */
                (*(*mb).cb).callout_flags |= PCRE2_CALLOUT_BACKTRACK; /* Note for callouts */

                state = Freturn_id!() as u32;
                continue 'sm;
            }

            /* ---- chunk A: STATES ---- */
/* ------------------------------------------------------------------ */
/* Common code for OP_ACCEPT (not in a recursion) and OP_END. C line 976. */

L_A_OP_END => {
    /* Fail for an empty string match if either PCRE2_NOTEMPTY is set, or if
    PCRE2_NOTEMPTY_ATSTART is set and we have matched at the start of the
    subject. */

    if Feptr!() == Fstart_match!()
        && (((*mb).moptions & PCRE2_NOTEMPTY) != 0
            || (((*mb).moptions & PCRE2_NOTEMPTY_ATSTART) != 0
                && Fstart_match!() == (*mb).start_subject.add((*mb).start_offset)))
    {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }

    /* Fail if PCRE2_ENDANCHORED is set and the end of the match is not
    the end of the subject. */

    if Feptr!() < (*mb).end_subject
        && (((*mb).moptions | (*mb).poptions) & PCRE2_ENDANCHORED) != 0
    {
        if Fop!() as u32 == OP_END {
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }

        return MATCH_NOMATCH; /* (*ACCEPT) */
    }

    /* Fail if we detect that the start position was moved to be either after
    the end position (\K in lookahead) or before the start offset (\K in
    lookbehind). */

    if Fstart_match!() < (*mb).start_subject.add((*mb).start_offset)
        || Fstart_match!() > Feptr!()
    {
        /* PCRE2_ASSERT(mb->hasbsk); */

        if (*mb).allowlookaroundbsk == 0 {
            return PCRE2_ERROR_BAD_BACKSLASH_K;
        }
    }

    /* We have a successful match of the whole pattern. */

    (*mb).end_match_ptr = Feptr!(); /* Record where we ended */
    (*mb).end_offset_top = Foffset_top!(); /* and how many extracts were taken */
    (*mb).mark = Fmark!(); /* and the last success mark */
    if Feptr!() > (*mb).last_used_ptr {
        (*mb).last_used_ptr = Feptr!();
    }

    *(*match_data).ovector.as_mut_ptr().add(0) =
        (Fstart_match!() as usize) - ((*mb).start_subject as usize);
    *(*match_data).ovector.as_mut_ptr().add(1) =
        (Feptr!() as usize) - ((*mb).start_subject as usize);

    /* Set i to the smaller of the sizes of the external and frame ovectors. */

    i = (2 * (if (top_bracket as i32 + 1) > (*match_data).oveccount as i32 {
        (*match_data).oveccount as i32
    } else {
        top_bracket as i32 + 1
    })) as u32;
    copy_nonoverlapping(
        Fovector!() as *const u8,
        (*match_data).ovector.as_mut_ptr().add(2) as *mut u8,
        ((i as usize) - 2) * core::mem::size_of::<PCRE2_SIZE>(),
    );
    loop {
        i = i.wrapping_sub(1);
        if !((i as PCRE2_SIZE) >= Foffset_top!() + 2) {
            break;
        }
        *(*match_data).ovector.as_mut_ptr().add(i as usize) = PCRE2_UNSET;
    }
    return MATCH_MATCH; /* Note: NOT RRETURN */
}

/* ------------------------------------------------------------------ */
/* OP_ALLANY, also reached by falling through from OP_ANY. C line 1076. */

L_A_ALLANY => {
    if Feptr!() >= (*mb).end_subject
    /* DO NOT merge the Feptr++ here; it must */
    {
        /* not be updated before SCHECK_PARTIAL. */
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Feptr!() = Feptr!().add(1);
    if utf != 0 {
        /* ACROSSCHAR(Feptr < mb->end_subject, Feptr, Feptr++) */
        while Feptr!() < (*mb).end_subject && (*Feptr!() & 0xc0u8) == 0x80u8 {
            Feptr!() = Feptr!().add(1);
        }
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ------------------------------------------------------------------ */
/* REPEATCHAR: common code for all repeated single-character matches.
C line 1392. */

L_REPEATCHAR => {
    if utf != 0 {
        length = 1;
        (*F).fields.char_repeat.charptr = Fecode!();
        GETCHARLEN!(fc, Fecode!(), length);
        Fecode!() = Fecode!().add(length);
        (*F).byte1 = length as u8; /* Llength */

        /* Handle multi-code-unit character matching, caseful and caseless. */

        if length > 1 {
            /* if (Fop >= OP_STARI && (othercase = UCD_OTHERCASE(fc)) != fc) */
            let mut oc_cond_: bool = false;
            if (Fop!() as u32) >= OP_STARI
            /* Caseless */
            {
                othercase = UCD_OTHERCASE!(fc);
                oc_cond_ = othercase != fc;
            }
            if oc_cond_ {
                (*F).byte2 = crate::ord2utf::_pcre2_ord2utf_8(
                    othercase,
                    core::ptr::addr_of_mut!((*F).fields.char_repeat.oc.occu) as *mut u8,
                ) as u8;
            } else {
                (*F).byte2 = 0;
            }

            i = 1;
            while i <= (*F).fields.char_repeat.min {
                if Feptr!() <= (*mb).end_subject.wrapping_sub(length)
                    && frag_a_memcmp_eq(
                        Feptr!(),
                        (*F).fields.char_repeat.charptr,
                        length,
                    )
                {
                    Feptr!() = Feptr!().add(length);
                } else if (*F).byte2 > 0
                    && Feptr!() <= (*mb).end_subject.wrapping_sub((*F).byte2 as usize)
                    && frag_a_memcmp_eq(
                        Feptr!(),
                        core::ptr::addr_of!((*F).fields.char_repeat.oc.occu) as *const u8,
                        (*F).byte2 as usize,
                    )
                {
                    Feptr!() = Feptr!().add((*F).byte2 as usize);
                } else {
                    CHECK_PARTIAL!();
                    rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                i += 1;
            }

            if (*F).fields.char_repeat.min == (*F).fields.char_repeat.max {
                state = S_MAINLOOP; /* continue */
                continue 'sm;
            }

            if reptype == REPTYPE_MIN {
                state = L_A_UTF_MIN_LOOP;
                continue 'sm;
            } else
            /* Maximize */
            {
                (*F).fields.char_repeat.start_eptr = Feptr!();
                i = (*F).fields.char_repeat.min;
                while i < (*F).fields.char_repeat.max {
                    if Feptr!() <= (*mb).end_subject.wrapping_sub((*F).byte1 as usize)
                        && frag_a_memcmp_eq(
                            Feptr!(),
                            (*F).fields.char_repeat.charptr,
                            (*F).byte1 as usize,
                        )
                    {
                        Feptr!() = Feptr!().add((*F).byte1 as usize);
                    } else if (*F).byte2 > 0
                        && Feptr!()
                            <= (*mb).end_subject.wrapping_sub((*F).byte2 as usize)
                        && frag_a_memcmp_eq(
                            Feptr!(),
                            core::ptr::addr_of!((*F).fields.char_repeat.oc.occu)
                                as *const u8,
                            (*F).byte2 as usize,
                        )
                    {
                        Feptr!() = Feptr!().add((*F).byte2 as usize);
                    } else {
                        CHECK_PARTIAL!();
                        break;
                    }
                    i += 1;
                }

                /* After \C in UTF mode, Lstart_eptr might be in the middle of a
                Unicode character. Use <= Lstart_eptr to ensure backtracking
                doesn't go too far. */

                if reptype != REPTYPE_POS {
                    state = L_A_UTF_MAX_LOOP;
                    continue 'sm;
                }
            }
            state = S_MAINLOOP; /* break: end of repeated wide character handling */
            continue 'sm;
        }

        /* Length of UTF character is 1. Put it into the preserved variable and
        fall through to the non-UTF code. */

        (*F).fields.char_repeat.c = fc;
    }
    /* When not in UTF mode, load a single-code-unit character. */
    else {
        (*F).fields.char_repeat.c = *Fecode!() as u32;
        Fecode!() = Fecode!().add(1);
    }

    /* Caseless comparison */

    if (Fop!() as u32) >= OP_STARI {
        if ucp != 0 && utf == 0 && (*F).fields.char_repeat.c > 127 {
            (*F).fields.char_repeat.oc.oc = UCD_OTHERCASE!((*F).fields.char_repeat.c);
        } else {
            /* Lc will be < 128 in UTF-8 mode. */
            (*F).fields.char_repeat.oc.oc =
                *(*mb).fcc.add((*F).fields.char_repeat.c as usize) as u32;
        }

        i = 1;
        while i <= (*F).fields.char_repeat.min {
            if Feptr!() >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            let cc: u32 = *Feptr!() as u32;
            if (*F).fields.char_repeat.c != cc && (*F).fields.char_repeat.oc.oc != cc {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            Feptr!() = Feptr!().add(1);
            i += 1;
        }
        if (*F).fields.char_repeat.min == (*F).fields.char_repeat.max {
            state = S_MAINLOOP; /* continue */
            continue 'sm;
        }

        if reptype == REPTYPE_MIN {
            state = L_A_CI_MIN_LOOP;
            continue 'sm;
        } else
        /* Maximize */
        {
            (*F).fields.char_repeat.start_eptr = Feptr!();
            i = (*F).fields.char_repeat.min;
            while i < (*F).fields.char_repeat.max {
                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    break;
                }
                let cc: u32 = *Feptr!() as u32;
                if (*F).fields.char_repeat.c != cc
                    && (*F).fields.char_repeat.oc.oc != cc
                {
                    break;
                }
                Feptr!() = Feptr!().add(1);
                i += 1;
            }
            if reptype != REPTYPE_POS {
                state = L_A_CI_MAX_LOOP;
                continue 'sm;
            }
        }
    }
    /* Caseful comparisons (includes all multi-byte characters) */
    else {
        i = 1;
        while i <= (*F).fields.char_repeat.min {
            if Feptr!() >= (*mb).end_subject {
                SCHECK_PARTIAL!();
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            let cc: u32 = *Feptr!() as u32;
            Feptr!() = Feptr!().add(1);
            if (*F).fields.char_repeat.c != cc {
                rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            i += 1;
        }

        if (*F).fields.char_repeat.min == (*F).fields.char_repeat.max {
            state = S_MAINLOOP; /* continue */
            continue 'sm;
        }

        if reptype == REPTYPE_MIN {
            state = L_A_CF_MIN_LOOP;
            continue 'sm;
        } else
        /* Maximize */
        {
            (*F).fields.char_repeat.start_eptr = Feptr!();
            i = (*F).fields.char_repeat.min;
            while i < (*F).fields.char_repeat.max {
                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    break;
                }

                if (*F).fields.char_repeat.c != *Feptr!() as u32 {
                    break;
                }
                Feptr!() = Feptr!().add(1);
                i += 1;
            }

            if reptype != REPTYPE_POS {
                state = L_A_CF_MAX_LOOP;
                continue 'sm;
            }
        }
    }
    state = S_MAINLOOP; /* break */
    continue 'sm;
}

/* --- REPEATCHAR, UTF multi-unit, minimizing (C lines 1432..1448) --- */

L_A_UTF_MIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM202 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM202 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.char_repeat.min;
    (*F).fields.char_repeat.min = t_ + 1;
    if t_ >= (*F).fields.char_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() <= (*mb).end_subject.wrapping_sub((*F).byte1 as usize)
        && frag_a_memcmp_eq(
            Feptr!(),
            (*F).fields.char_repeat.charptr,
            (*F).byte1 as usize,
        )
    {
        Feptr!() = Feptr!().add((*F).byte1 as usize);
    } else if (*F).byte2 > 0
        && Feptr!() <= (*mb).end_subject.wrapping_sub((*F).byte2 as usize)
        && frag_a_memcmp_eq(
            Feptr!(),
            core::ptr::addr_of!((*F).fields.char_repeat.oc.occu) as *const u8,
            (*F).byte2 as usize,
        )
    {
        Feptr!() = Feptr!().add((*F).byte2 as usize);
    } else {
        CHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_A_UTF_MIN_LOOP;
    continue 'sm;
}

/* --- REPEATCHAR, UTF multi-unit, maximizing (C lines 1474..1481) --- */

L_A_UTF_MAX_LOOP => {
    if Feptr!() <= (*F).fields.char_repeat.start_eptr {
        state = S_MAINLOOP; /* break out of the for(;;), then break the switch */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM203 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM203 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    Feptr!() = Feptr!().wrapping_sub(1);
    BACKCHAR!(Feptr!());
    state = L_A_UTF_MAX_LOOP;
    continue 'sm;
}

/* --- REPEATCHAR, caseless, minimizing (C lines 1534..1548) --- */

L_A_CI_MIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM25 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM25 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.char_repeat.min;
    (*F).fields.char_repeat.min = t_ + 1;
    if t_ >= (*F).fields.char_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    let cc: u32 = *Feptr!() as u32;
    if (*F).fields.char_repeat.c != cc && (*F).fields.char_repeat.oc.oc != cc {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Feptr!() = Feptr!().add(1);
    state = L_A_CI_MIN_LOOP;
    continue 'sm;
}

/* --- REPEATCHAR, caseless, maximizing (C lines 1567..1573) --- */

L_A_CI_MAX_LOOP => {
    if Feptr!() == (*F).fields.char_repeat.start_eptr {
        state = S_MAINLOOP; /* break */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM26 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM26 => {
    Feptr!() = Feptr!().wrapping_sub(1);
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    state = L_A_CI_MAX_LOOP;
    continue 'sm;
}

/* --- REPEATCHAR, caseful, minimizing (C lines 1595..1606) --- */

L_A_CF_MIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM27 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM27 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.char_repeat.min;
    (*F).fields.char_repeat.min = t_ + 1;
    if t_ >= (*F).fields.char_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    let cc: u32 = *Feptr!() as u32;
    Feptr!() = Feptr!().add(1);
    if (*F).fields.char_repeat.c != cc {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_A_CF_MIN_LOOP;
    continue 'sm;
}

/* --- REPEATCHAR, caseful, maximizing (C lines 1624..1630) --- */

L_A_CF_MAX_LOOP => {
    if Feptr!() <= (*F).fields.char_repeat.start_eptr {
        state = S_MAINLOOP; /* break */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM28 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM28 => {
    Feptr!() = Feptr!().wrapping_sub(1);
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    state = L_A_CF_MAX_LOOP;
    continue 'sm;
}

/* ------------------------------------------------------------------ */
/* REPEATNOTCHAR: common code for all repeated single-character
non-matches. C line 1733. */

L_REPEATNOTCHAR => {
    GETCHARINCTEST!((*F).fields.charnot_repeat.c, Fecode!(), utf);

    /* The code is duplicated for the caseless and caseful cases, for speed. */

    if (Fop!() as u32) >= OP_NOTSTARI
    /* Caseless */
    {
        if (utf != 0 || ucp != 0) && (*F).fields.charnot_repeat.c > 127 {
            (*F).fields.charnot_repeat.oc =
                UCD_OTHERCASE!((*F).fields.charnot_repeat.c);
        } else {
            (*F).fields.charnot_repeat.oc = TABLE_GET!(
                (*F).fields.charnot_repeat.c,
                (*mb).fcc,
                (*F).fields.charnot_repeat.c
            ) as u32; /* Other case from table */
        }

        if utf != 0 {
            let mut d: u32 = 0;
            i = 1;
            while i <= (*F).fields.charnot_repeat.min {
                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                GETCHARINC!(d, Feptr!());
                if (*F).fields.charnot_repeat.c == d
                    || (*F).fields.charnot_repeat.oc == d
                {
                    rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                i += 1;
            }
        }
        /* Not UTF mode */
        else {
            i = 1;
            while i <= (*F).fields.charnot_repeat.min {
                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                if (*F).fields.charnot_repeat.c == *Feptr!() as u32
                    || (*F).fields.charnot_repeat.oc == *Feptr!() as u32
                {
                    rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                Feptr!() = Feptr!().add(1);
                i += 1;
            }
        }

        if (*F).fields.charnot_repeat.min == (*F).fields.charnot_repeat.max {
            state = S_MAINLOOP; /* continue - finished for exact count */
            continue 'sm;
        }

        if reptype == REPTYPE_MIN {
            if utf != 0 {
                state = L_A_N_CI_UMIN_LOOP;
                continue 'sm;
            }
            /* Not UTF mode */
            else {
                state = L_A_N_CI_MIN_LOOP;
                continue 'sm;
            }
        }
        /* Maximize case */
        else {
            (*F).fields.charnot_repeat.start_eptr = Feptr!();

            if utf != 0 {
                let mut d: u32 = 0;
                i = (*F).fields.charnot_repeat.min;
                while i < (*F).fields.charnot_repeat.max {
                    let mut len: i32 = 1;
                    if Feptr!() >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        break;
                    }
                    GETCHARLEN!(d, Feptr!(), len);
                    if (*F).fields.charnot_repeat.c == d
                        || (*F).fields.charnot_repeat.oc == d
                    {
                        break;
                    }
                    Feptr!() = Feptr!().add(len as usize);
                    i += 1;
                }

                /* After \C in UTF mode, Lstart_eptr might be in the middle of a
                Unicode character. Use <= Lstart_eptr to ensure backtracking
                doesn't go too far. */

                if reptype != REPTYPE_POS {
                    state = L_A_N_CI_UMAX_LOOP;
                    continue 'sm;
                }
            }
            /* Not UTF mode */
            else {
                i = (*F).fields.charnot_repeat.min;
                while i < (*F).fields.charnot_repeat.max {
                    if Feptr!() >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        break;
                    }
                    if (*F).fields.charnot_repeat.c == *Feptr!() as u32
                        || (*F).fields.charnot_repeat.oc == *Feptr!() as u32
                    {
                        break;
                    }
                    Feptr!() = Feptr!().add(1);
                    i += 1;
                }
                if reptype != REPTYPE_POS {
                    state = L_A_N_CI_MAX_LOOP;
                    continue 'sm;
                }
            }
        }
    }
    /* Caseful comparisons */
    else {
        if utf != 0 {
            let mut d: u32 = 0;
            i = 1;
            while i <= (*F).fields.charnot_repeat.min {
                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                GETCHARINC!(d, Feptr!());
                if (*F).fields.charnot_repeat.c == d {
                    rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                i += 1;
            }
        }
        /* Not UTF mode */
        else {
            i = 1;
            while i <= (*F).fields.charnot_repeat.min {
                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                let cc_ = *Feptr!() as u32;
                Feptr!() = Feptr!().add(1);
                if (*F).fields.charnot_repeat.c == cc_ {
                    rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                i += 1;
            }
        }

        if (*F).fields.charnot_repeat.min == (*F).fields.charnot_repeat.max {
            state = S_MAINLOOP; /* continue */
            continue 'sm;
        }

        if reptype == REPTYPE_MIN {
            if utf != 0 {
                state = L_A_N_CF_UMIN_LOOP;
                continue 'sm;
            }
            /* Not UTF mode */
            else {
                state = L_A_N_CF_MIN_LOOP;
                continue 'sm;
            }
        }
        /* Maximize case */
        else {
            (*F).fields.charnot_repeat.start_eptr = Feptr!();

            if utf != 0 {
                let mut d: u32 = 0;
                i = (*F).fields.charnot_repeat.min;
                while i < (*F).fields.charnot_repeat.max {
                    let mut len: i32 = 1;
                    if Feptr!() >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        break;
                    }
                    GETCHARLEN!(d, Feptr!(), len);
                    if (*F).fields.charnot_repeat.c == d {
                        break;
                    }
                    Feptr!() = Feptr!().add(len as usize);
                    i += 1;
                }

                /* After \C in UTF mode, Lstart_eptr might be in the middle of a
                Unicode character. */

                if reptype != REPTYPE_POS {
                    state = L_A_N_CF_UMAX_LOOP;
                    continue 'sm;
                }
            }
            /* Not UTF mode */
            else {
                i = (*F).fields.charnot_repeat.min;
                while i < (*F).fields.charnot_repeat.max {
                    if Feptr!() >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        break;
                    }
                    if (*F).fields.charnot_repeat.c == *Feptr!() as u32 {
                        break;
                    }
                    Feptr!() = Feptr!().add(1);
                    i += 1;
                }
                if reptype != REPTYPE_POS {
                    state = L_A_N_CF_MAX_LOOP;
                    continue 'sm;
                }
            }
        }
    }
    state = S_MAINLOOP; /* break */
    continue 'sm;
}

/* --- REPEATNOTCHAR, caseless, UTF, minimizing (C 1794..1806) --- */

L_A_N_CI_UMIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM204 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM204 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.charnot_repeat.min;
    (*F).fields.charnot_repeat.min = t_ + 1;
    if t_ >= (*F).fields.charnot_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    let mut d: u32 = 0;
    GETCHARINC!(d, Feptr!());
    if (*F).fields.charnot_repeat.c == d || (*F).fields.charnot_repeat.oc == d {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_A_N_CI_UMIN_LOOP;
    continue 'sm;
}

/* --- REPEATNOTCHAR, caseless, non-UTF, minimizing (C 1813..1825) --- */

L_A_N_CI_MIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM29 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM29 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.charnot_repeat.min;
    (*F).fields.charnot_repeat.min = t_ + 1;
    if t_ >= (*F).fields.charnot_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if (*F).fields.charnot_repeat.c == *Feptr!() as u32
        || (*F).fields.charnot_repeat.oc == *Feptr!() as u32
    {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Feptr!() = Feptr!().add(1);
    state = L_A_N_CI_MIN_LOOP;
    continue 'sm;
}

/* --- REPEATNOTCHAR, caseless, UTF, maximizing (C 1857..1864) --- */

L_A_N_CI_UMAX_LOOP => {
    if Feptr!() <= (*F).fields.charnot_repeat.start_eptr {
        state = S_MAINLOOP; /* break */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM205 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM205 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    Feptr!() = Feptr!().wrapping_sub(1);
    BACKCHAR!(Feptr!());
    state = L_A_N_CI_UMAX_LOOP;
    continue 'sm;
}

/* --- REPEATNOTCHAR, caseless, non-UTF, maximizing (C 1881..1887) --- */

L_A_N_CI_MAX_LOOP => {
    if Feptr!() == (*F).fields.charnot_repeat.start_eptr {
        state = S_MAINLOOP; /* break */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM30 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM30 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    Feptr!() = Feptr!().wrapping_sub(1);
    state = L_A_N_CI_MAX_LOOP;
    continue 'sm;
}

/* --- REPEATNOTCHAR, caseful, UTF, minimizing (C 1934..1946) --- */

L_A_N_CF_UMIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM206 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM206 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.charnot_repeat.min;
    (*F).fields.charnot_repeat.min = t_ + 1;
    if t_ >= (*F).fields.charnot_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    let mut d: u32 = 0;
    GETCHARINC!(d, Feptr!());
    if (*F).fields.charnot_repeat.c == d {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_A_N_CF_UMIN_LOOP;
    continue 'sm;
}

/* --- REPEATNOTCHAR, caseful, non-UTF, minimizing (C 1952..1963) --- */

L_A_N_CF_MIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM31 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM31 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.charnot_repeat.min;
    (*F).fields.charnot_repeat.min = t_ + 1;
    if t_ >= (*F).fields.charnot_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    let cc_ = *Feptr!() as u32;
    Feptr!() = Feptr!().add(1);
    if (*F).fields.charnot_repeat.c == cc_ {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_A_N_CF_MIN_LOOP;
    continue 'sm;
}

/* --- REPEATNOTCHAR, caseful, UTF, maximizing (C 1995..2002) --- */

L_A_N_CF_UMAX_LOOP => {
    if Feptr!() <= (*F).fields.charnot_repeat.start_eptr {
        state = S_MAINLOOP; /* break */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM207 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM207 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    Feptr!() = Feptr!().wrapping_sub(1);
    BACKCHAR!(Feptr!());
    state = L_A_N_CF_UMAX_LOOP;
    continue 'sm;
}

/* --- REPEATNOTCHAR, caseful, non-UTF, maximizing (C 2018..2024) --- */

L_A_N_CF_MAX_LOOP => {
    if Feptr!() == (*F).fields.charnot_repeat.start_eptr {
        state = S_MAINLOOP; /* break */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM32 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM32 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    Feptr!() = Feptr!().wrapping_sub(1);
    state = L_A_N_CF_MAX_LOOP;
    continue 'sm;
}

/* --- OP_CLASS/OP_NCLASS, UTF, minimizing (C 2148..2165) --- */

L_A_CLASS_UMIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM200 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM200 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.class_repeat.min;
    (*F).fields.class_repeat.min = t_ + 1;
    if t_ >= (*F).fields.class_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINC!(fc, Feptr!());
    if fc > 255 {
        if Fop!() as u32 == OP_CLASS {
            rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    } else if (*(*F).fields.class_repeat.byte_map_address.add((fc / 8) as usize) as u32
        & (1u32 << (fc & 7)))
        == 0
    {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_A_CLASS_UMIN_LOOP;
    continue 'sm;
}

/* --- OP_CLASS/OP_NCLASS, non-UTF, minimizing (C 2171..2190) --- */

L_A_CLASS_MIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM23 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM23 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.class_repeat.min;
    (*F).fields.class_repeat.min = t_ + 1;
    if t_ >= (*F).fields.class_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    fc = *Feptr!() as u32;
    Feptr!() = Feptr!().add(1);
    if (*(*F).fields.class_repeat.byte_map_address.add((fc / 8) as usize) as u32
        & (1u32 << (fc & 7)))
        == 0
    {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_A_CLASS_MIN_LOOP;
    continue 'sm;
}

/* --- OP_CLASS/OP_NCLASS, UTF, maximizing (C 2228..2234) --- */

L_A_CLASS_UMAX_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM201 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM201 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let old_ = Feptr!();
    Feptr!() = Feptr!().wrapping_sub(1);
    if old_ <= (*F).fields.class_repeat.start_eptr {
        /* break: tried at original position; then RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    BACKCHAR!(Feptr!());
    state = L_A_CLASS_UMAX_LOOP;
    continue 'sm;
}

/* --- OP_CLASS/OP_NCLASS, non-UTF, maximizing (C 2261..2266) --- */

L_A_CLASS_MAX_LOOP => {
    if !(Feptr!() >= (*F).fields.class_repeat.start_eptr) {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM24 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM24 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    Feptr!() = Feptr!().wrapping_sub(1);
    state = L_A_CLASS_MAX_LOOP;
    continue 'sm;
}

/* --- OP_XCLASS, minimizing (C 2355..2369) --- */

L_A_XCLASS_MIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM100 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM100 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.xclass_repeat.min;
    (*F).fields.xclass_repeat.min = t_ + 1;
    if t_ >= (*F).fields.xclass_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if crate::xclass::_pcre2_xclass_8(
        fc,
        (*F).fields.xclass_repeat.xclass_data,
        (*mb).start_code,
        utf,
    ) == 0
    {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_A_XCLASS_MIN_LOOP;
    continue 'sm;
}

/* --- OP_XCLASS, maximizing (C 2402..2411) --- */

L_A_XCLASS_MAX_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM101 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM101 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let old_ = Feptr!();
    Feptr!() = Feptr!().wrapping_sub(1);
    if old_ <= (*F).fields.xclass_repeat.start_eptr {
        /* break: tried at original position; then RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if utf != 0 {
        BACKCHAR!(Feptr!());
    }
    state = L_A_XCLASS_MAX_LOOP;
    continue 'sm;
}

/* --- OP_ECLASS, minimizing (C 2498..2512) --- */

L_A_ECLASS_MIN_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM102 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM102 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let t_ = (*F).fields.eclass_repeat.min;
    (*F).fields.eclass_repeat.min = t_ + 1;
    if t_ >= (*F).fields.eclass_repeat.max {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if crate::xclass::_pcre2_eclass_8(
        fc,
        (*F).fields.eclass_repeat.eclass_data,
        (*F).fields
            .eclass_repeat
            .eclass_data
            .add((*F).fields.eclass_repeat.eclass_len),
        (*mb).start_code,
        utf,
    ) == 0
    {
        rrc = MATCH_NOMATCH; /* RRETURN(MATCH_NOMATCH) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_A_ECLASS_MIN_LOOP;
    continue 'sm;
}

/* --- OP_ECLASS, maximizing (C 2546..2555) --- */

L_A_ECLASS_MAX_LOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM103 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM103 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH; /* RRETURN(rrc) */
        continue 'sm;
    }
    let old_ = Feptr!();
    Feptr!() = Feptr!().wrapping_sub(1);
    if old_ <= (*F).fields.eclass_repeat.start_eptr {
        /* break: tried at original position; then RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if utf != 0 {
        BACKCHAR!(Feptr!());
    }
    state = L_A_ECLASS_MAX_LOOP;
    continue 'sm;
}
/* ---- chunk B: STATES ---- */
/* ===================================================================== *
 * C 2973: REPEATTYPE - common code for all repeated character type
 * matches. The four inner labels ENDLOOP99 (C 4511), GOT_MAX (C 4567)
 * and ENDLOOP00..ENDLOOP03 (C 5054..5117) are each jumped to only from
 * inside the single loop that immediately precedes them, so they are
 * translated as Rust labeled blocks with `break 'label`, not as states.
 * ===================================================================== */
L_REPEATTYPE => {
    /* Lctype = *Fecode++;  Code for the character type */
    (*F).fields.type_repeat.ctype = *Fecode!() as u32;
    Fecode!() = Fecode!().add(1);

    if (*F).fields.type_repeat.ctype == OP_PROP || (*F).fields.type_repeat.ctype == OP_NOTPROP {
        proptype = *Fecode!() as i32;
        Fecode!() = Fecode!().add(1);
        (*F).fields.type_repeat.propvalue = *Fecode!() as u32;
        Fecode!() = Fecode!().add(1);
    } else {
        proptype = -1;
    }

    /* First, ensure the minimum number of matches are present. Use inline
    code for maximizing the speed, and do the type test once at the start
    (i.e. keep it out of the loops). As there are no calls to RMATCH in the
    loops, we can use an ordinary variable for "notmatch". The code for UTF
    mode is separated out for tidiness, except for Unicode property tests. */

    if (*F).fields.type_repeat.min > 0 {
        if proptype >= 0
        /* Property tests in all modes */
        {
            let notmatch: BOOL = ((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL;
            match proptype as u32 {
                PT_LAMP => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let chartype: i32;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        chartype = UCD_CHARTYPE!(fc) as i32;
                        if ((chartype == ucp_Lu as i32
                            || chartype == ucp_Ll as i32
                            || chartype == ucp_Lt as i32) as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                PT_GC => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        if ((UCD_CATEGORY!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                PT_PC => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        if ((UCD_CHARTYPE!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                PT_SC => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        if ((UCD_SCRIPT!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                PT_SCX => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let ok: BOOL;
                        let prop: *const ucd_record;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        prop = GET_UCD!(fc);
                        ok = (((*prop).script as u32 == (*F).fields.type_repeat.propvalue)
                            || MAPBIT!(
                                crate::ucd::_pcre2_ucd_script_sets_8
                                    .as_ptr()
                                    .add(UCD_SCRIPTX_PROP!(prop) as usize),
                                (*F).fields.type_repeat.propvalue
                            ) != 0) as BOOL;
                        if ok == notmatch {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                PT_ALNUM => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let category: i32;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        category = UCD_CATEGORY!(fc) as i32;
                        if ((category == ucp_L as i32 || category == ucp_N as i32) as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* Perl space used to exclude VT, but from Perl 5.18 it is included,
                which means that Perl space and POSIX space are now identical. PCRE
                was changed at release 8.34. */
                PT_SPACE | PT_PXSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        match fc {
                            B_HSPACE_VSPACE_CASES!() => {
                                if notmatch != 0 {
                                    rrc = MATCH_NOMATCH;
                                    state = S_RETURN_SWITCH;
                                    continue 'sm;
                                }
                            }
                            _ => {
                                if ((UCD_CATEGORY!(fc) == ucp_Z) as BOOL) == notmatch {
                                    rrc = MATCH_NOMATCH;
                                    state = S_RETURN_SWITCH;
                                    continue 'sm;
                                }
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                PT_WORD => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let chartype: i32;
                        let category: i32;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        chartype = UCD_CHARTYPE!(fc) as i32;
                        category = crate::tables::_pcre2_ucp_gentype_8[chartype as usize] as i32;
                        if ((category == ucp_L as i32
                            || category == ucp_N as i32
                            || chartype == ucp_Mn as i32
                            || chartype == ucp_Pc as i32) as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                PT_CLIST => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let mut cp: *const u32;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        cp = crate::ucd::_pcre2_ucd_caseless_sets_8
                            .as_ptr()
                            .add((*F).fields.type_repeat.propvalue as usize);
                        loop {
                            if fc < *cp {
                                if notmatch != 0 {
                                    break;
                                }
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                            let t = *cp;
                            cp = cp.add(1);
                            if fc == t {
                                if notmatch != 0 {
                                    rrc = MATCH_NOMATCH;
                                    state = S_RETURN_SWITCH;
                                    continue 'sm;
                                }
                                break;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                PT_UCNC => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        if ((fc == 0x24u32 /* CHAR_DOLLAR_SIGN */
                            || fc == 0x40u32 /* CHAR_COMMERCIAL_AT */
                            || fc == 0x60u32 /* CHAR_GRAVE_ACCENT */
                            || (fc >= 0xa0 && fc <= 0xd7ff)
                            || fc >= 0xe000) as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                PT_BIDICL => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        if ((UCD_BIDICLASS!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
                            == notmatch
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                PT_BOOL => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let ok: BOOL;
                        let prop: *const ucd_record;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINCTEST!(fc, Feptr!(), utf);
                        prop = GET_UCD!(fc);
                        ok = (MAPBIT!(
                            crate::ucd::_pcre2_ucd_boolprop_sets_8
                                .as_ptr()
                                .add(UCD_BPROPS_PROP!(prop) as usize),
                            (*F).fields.type_repeat.propvalue
                        ) != 0) as BOOL;
                        if ok == notmatch {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                /* This should not occur */
                _ => {
                    return PCRE2_ERROR_INTERNAL;
                }
            }
        }
        /* Match extended Unicode sequences. We will get here only if the
        support is in the binary; otherwise a compile-time error occurs. */
        else if (*F).fields.type_repeat.ctype == OP_EXTUNI {
            i = 1;
            while i <= (*F).fields.type_repeat.min {
                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                } else {
                    GETCHARINCTEST!(fc, Feptr!(), utf);
                    Feptr!() = crate::extuni::_pcre2_extuni_8(
                        fc,
                        Feptr!(),
                        (*mb).start_subject,
                        (*mb).end_subject,
                        utf,
                        null_mut(),
                    );
                }
                CHECK_PARTIAL!();
                i = i.wrapping_add(1);
            }
        }
        /* Handle all other cases in UTF mode */
        else if utf != 0 {
            match (*F).fields.type_repeat.ctype {
                OP_ANY => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        if IS_NEWLINE!(Feptr!()) != 0 {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        if (*mb).partial != 0
                            && Feptr!().add(1) >= (*mb).end_subject
                            && (*mb).nltype == NLTYPE_FIXED
                            && (*mb).nllen == 2
                            && *Feptr!() == (*mb).nl[0]
                        {
                            (*mb).hitend = TRUE;
                            if (*mb).partial > 1 {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        Feptr!() = Feptr!().add(1);
                        ACROSSCHAR!(
                            Feptr!() < (*mb).end_subject,
                            Feptr!(),
                            Feptr!() = Feptr!().add(1)
                        );
                        i = i.wrapping_add(1);
                    }
                }

                OP_ALLANY => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        ACROSSCHAR!(
                            Feptr!() < (*mb).end_subject,
                            Feptr!(),
                            Feptr!() = Feptr!().add(1)
                        );
                        i = i.wrapping_add(1);
                    }
                }

                OP_ANYBYTE => {
                    if Feptr!()
                        > (*mb)
                            .end_subject
                            .wrapping_sub((*F).fields.type_repeat.min as usize)
                    {
                        rrc = MATCH_NOMATCH;
                        state = S_RETURN_SWITCH;
                        continue 'sm;
                    }
                    Feptr!() = Feptr!().add((*F).fields.type_repeat.min as usize);
                }

                OP_ANYNL => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINC!(fc, Feptr!());
                        match fc {
                            0x0du32 /* CHAR_CR */ => {
                                if Feptr!() < (*mb).end_subject && *Feptr!() == 0x0au8 {
                                    Feptr!() = Feptr!().add(1);
                                }
                            }
                            0x0au32 /* CHAR_LF */ => {}
                            0x0bu32 | 0x0cu32 | 0x85u32 | 0x2028u32 | 0x2029u32 => {
                                if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                                    rrc = MATCH_NOMATCH;
                                    state = S_RETURN_SWITCH;
                                    continue 'sm;
                                }
                            }
                            _ => {
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_HSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINC!(fc, Feptr!());
                        match fc {
                            B_HSPACE_CASES!() => {
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                            _ => {}
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_HSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINC!(fc, Feptr!());
                        match fc {
                            B_HSPACE_CASES!() => {}
                            _ => {
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_VSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINC!(fc, Feptr!());
                        match fc {
                            B_VSPACE_CASES!() => {
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                            _ => {}
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_VSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINC!(fc, Feptr!());
                        match fc {
                            B_VSPACE_CASES!() => {}
                            _ => {
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_DIGIT => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        GETCHARINC!(fc, Feptr!());
                        if fc < 128 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_DIGIT => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let cc: u32;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        cc = *Feptr!() as u32;
                        if cc >= 128 || (*(*mb).ctypes.add(cc as usize) & ctype_digit) == 0 {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        /* No need to skip more code units - we know it has only one. */
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_WHITESPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let cc: u32;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        cc = *Feptr!() as u32;
                        if cc < 128 && (*(*mb).ctypes.add(cc as usize) & ctype_space) != 0 {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        ACROSSCHAR!(
                            Feptr!() < (*mb).end_subject,
                            Feptr!(),
                            Feptr!() = Feptr!().add(1)
                        );
                        i = i.wrapping_add(1);
                    }
                }

                OP_WHITESPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let cc: u32;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        cc = *Feptr!() as u32;
                        if cc >= 128 || (*(*mb).ctypes.add(cc as usize) & ctype_space) == 0 {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        /* No need to skip more code units - we know it has only one. */
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_WORDCHAR => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let cc: u32;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        cc = *Feptr!() as u32;
                        if cc < 128 && (*(*mb).ctypes.add(cc as usize) & ctype_word) != 0 {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        ACROSSCHAR!(
                            Feptr!() < (*mb).end_subject,
                            Feptr!(),
                            Feptr!() = Feptr!().add(1)
                        );
                        i = i.wrapping_add(1);
                    }
                }

                OP_WORDCHAR => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        let cc: u32;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        cc = *Feptr!() as u32;
                        if cc >= 128 || (*(*mb).ctypes.add(cc as usize) & ctype_word) == 0 {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        /* No need to skip more code units - we know it has only one. */
                        i = i.wrapping_add(1);
                    }
                }

                _ => {
                    return PCRE2_ERROR_INTERNAL;
                }
            } /* End switch(Lctype) */
        }
        /* Code for the non-UTF case for minimum matching of operators other
        than OP_PROP and OP_NOTPROP. */
        else {
            match (*F).fields.type_repeat.ctype {
                OP_ANY => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        if IS_NEWLINE!(Feptr!()) != 0 {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        if (*mb).partial != 0
                            && Feptr!().add(1) >= (*mb).end_subject
                            && (*mb).nltype == NLTYPE_FIXED
                            && (*mb).nllen == 2
                            && *Feptr!() == (*mb).nl[0]
                        {
                            (*mb).hitend = TRUE;
                            if (*mb).partial > 1 {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_ALLANY => {
                    if Feptr!()
                        > (*mb)
                            .end_subject
                            .wrapping_sub((*F).fields.type_repeat.min as usize)
                    {
                        SCHECK_PARTIAL!();
                        rrc = MATCH_NOMATCH;
                        state = S_RETURN_SWITCH;
                        continue 'sm;
                    }
                    Feptr!() = Feptr!().add((*F).fields.type_repeat.min as usize);
                }

                /* The OP_ANYBYTE case is cut out in C because \C gets turned into
                OP_ALLANY in non-UTF mode. */
                OP_ANYNL => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        let cc = *Feptr!();
                        Feptr!() = Feptr!().add(1);
                        match cc as u32 {
                            0x0du32 /* CHAR_CR */ => {
                                if Feptr!() < (*mb).end_subject && *Feptr!() == 0x0au8 {
                                    Feptr!() = Feptr!().add(1);
                                }
                            }
                            0x0au32 /* CHAR_LF */ => {}
                            0x0bu32 | 0x0cu32 | 0x85u32 => {
                                if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                                    rrc = MATCH_NOMATCH;
                                    state = S_RETURN_SWITCH;
                                    continue 'sm;
                                }
                            }
                            _ => {
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_HSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        let cc = *Feptr!();
                        Feptr!() = Feptr!().add(1);
                        match cc as u32 {
                            B_HSPACE_BYTE_CASES!() => {
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                            _ => {}
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_HSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        let cc = *Feptr!();
                        Feptr!() = Feptr!().add(1);
                        match cc as u32 {
                            B_HSPACE_BYTE_CASES!() => {}
                            _ => {
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_VSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        let cc = *Feptr!();
                        Feptr!() = Feptr!().add(1);
                        match cc as u32 {
                            B_VSPACE_BYTE_CASES!() => {
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                            _ => {}
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_VSPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        let cc = *Feptr!();
                        Feptr!() = Feptr!().add(1);
                        match cc as u32 {
                            B_VSPACE_BYTE_CASES!() => {}
                            _ => {
                                rrc = MATCH_NOMATCH;
                                state = S_RETURN_SWITCH;
                                continue 'sm;
                            }
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_DIGIT => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        if MAX_255!(*Feptr!()) != 0
                            && (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_digit) != 0
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_DIGIT => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        if MAX_255!(*Feptr!()) == 0
                            || (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_digit) == 0
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_WHITESPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        if MAX_255!(*Feptr!()) != 0
                            && (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_space) != 0
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_WHITESPACE => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        if MAX_255!(*Feptr!()) == 0
                            || (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_space) == 0
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_WORDCHAR => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        if MAX_255!(*Feptr!()) != 0
                            && (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_word) != 0
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_WORDCHAR => {
                    i = 1;
                    while i <= (*F).fields.type_repeat.min {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        if MAX_255!(*Feptr!()) == 0
                            || (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_word) == 0
                        {
                            rrc = MATCH_NOMATCH;
                            state = S_RETURN_SWITCH;
                            continue 'sm;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                _ => {
                    return PCRE2_ERROR_INTERNAL;
                }
            }
        }
    }

    /* If Lmin = Lmax we are done. Continue with the main loop. */

    if (*F).fields.type_repeat.min == (*F).fields.type_repeat.max {
        state = S_MAINLOOP; /* continue */
        continue 'sm;
    }

    /* If minimizing, we have to test the rest of the pattern before each
    subsequent match. This means we cannot use a local "notmatch" variable as
    in the other cases. As all 4 temporary 32-bit values in the frame are
    already in use, just test the type each time. */

    if reptype == REPTYPE_MIN {
        if proptype >= 0 {
            match proptype as u32 {
                PT_LAMP => {
                    state = L_B_TR_MIN_LAMP;
                    continue 'sm;
                }
                PT_GC => {
                    state = L_B_TR_MIN_GC;
                    continue 'sm;
                }
                PT_PC => {
                    state = L_B_TR_MIN_PC;
                    continue 'sm;
                }
                PT_SC => {
                    state = L_B_TR_MIN_SC;
                    continue 'sm;
                }
                PT_SCX => {
                    state = L_B_TR_MIN_SCX;
                    continue 'sm;
                }
                PT_ALNUM => {
                    state = L_B_TR_MIN_ALNUM;
                    continue 'sm;
                }
                PT_SPACE | PT_PXSPACE => {
                    state = L_B_TR_MIN_SPACE;
                    continue 'sm;
                }
                PT_WORD => {
                    state = L_B_TR_MIN_WORD;
                    continue 'sm;
                }
                PT_CLIST => {
                    state = L_B_TR_MIN_CLIST;
                    continue 'sm;
                }
                PT_UCNC => {
                    state = L_B_TR_MIN_UCNC;
                    continue 'sm;
                }
                PT_BIDICL => {
                    state = L_B_TR_MIN_BIDICL;
                    continue 'sm;
                }
                PT_BOOL => {
                    state = L_B_TR_MIN_BOOL;
                    continue 'sm;
                }
                /* This should never occur */
                _ => {
                    return PCRE2_ERROR_INTERNAL;
                }
            }
        }
        /* Match extended Unicode sequences. */
        else if (*F).fields.type_repeat.ctype == OP_EXTUNI {
            state = L_B_TR_MIN_EXTUNI;
            continue 'sm;
        }
        /* UTF mode for non-property testing character types. */
        else if utf != 0 {
            state = L_B_TR_MIN_UTF;
            continue 'sm;
        }
        /* Not UTF mode */
        else {
            state = L_B_TR_MIN_NOUTF;
            continue 'sm;
        }
    }
    /* If maximizing, it is worth using inline code for speed, doing the type
    test once at the start (i.e. keep it out of the loops). Once again,
    "notmatch" can be an ordinary local variable because the loops do not call
    RMATCH. */
    else {
        (*F).fields.type_repeat.start_eptr = Feptr!(); /* Remember where we started */

        if proptype >= 0 {
            let notmatch: BOOL = ((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL;
            match proptype as u32 {
                PT_LAMP => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let chartype: i32;
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLENTEST!(fc, Feptr!(), len, utf);
                        chartype = UCD_CHARTYPE!(fc) as i32;
                        if ((chartype == ucp_Lu as i32
                            || chartype == ucp_Ll as i32
                            || chartype == ucp_Lt as i32) as BOOL)
                            == notmatch
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                PT_GC => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLENTEST!(fc, Feptr!(), len, utf);
                        if ((UCD_CATEGORY!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
                            == notmatch
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                PT_PC => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLENTEST!(fc, Feptr!(), len, utf);
                        if ((UCD_CHARTYPE!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
                            == notmatch
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                PT_SC => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLENTEST!(fc, Feptr!(), len, utf);
                        if ((UCD_SCRIPT!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
                            == notmatch
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                PT_SCX => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let ok: BOOL;
                        let prop: *const ucd_record;
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLENTEST!(fc, Feptr!(), len, utf);
                        prop = GET_UCD!(fc);
                        ok = (((*prop).script as u32 == (*F).fields.type_repeat.propvalue)
                            || MAPBIT!(
                                crate::ucd::_pcre2_ucd_script_sets_8
                                    .as_ptr()
                                    .add(UCD_SCRIPTX_PROP!(prop) as usize),
                                (*F).fields.type_repeat.propvalue
                            ) != 0) as BOOL;
                        if ok == notmatch {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                PT_ALNUM => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let category: i32;
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLENTEST!(fc, Feptr!(), len, utf);
                        category = UCD_CATEGORY!(fc) as i32;
                        if ((category == ucp_L as i32 || category == ucp_N as i32) as BOOL)
                            == notmatch
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                /* Perl space used to exclude VT, but from Perl 5.18 it is included,
                which means that Perl space and POSIX space are now identical. PCRE
                was changed at release 8.34. */
                PT_SPACE | PT_PXSPACE => {
                    /* The C label ENDLOOP99 (C 4511) sits just after this loop; it
                    is reached only by the two gotos inside it. */
                    'endloop99: {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let mut len: i32 = 1;
                            if Feptr!() >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLENTEST!(fc, Feptr!(), len, utf);
                            match fc {
                                B_HSPACE_VSPACE_CASES!() => {
                                    if notmatch != 0 {
                                        break 'endloop99; /* goto ENDLOOP99 - break the loop */
                                    }
                                }
                                _ => {
                                    if ((UCD_CATEGORY!(fc) == ucp_Z) as BOOL) == notmatch {
                                        break 'endloop99; /* goto ENDLOOP99 - break the loop */
                                    }
                                }
                            }
                            Feptr!() = Feptr!().add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }
                }

                PT_WORD => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let chartype: i32;
                        let category: i32;
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLENTEST!(fc, Feptr!(), len, utf);
                        chartype = UCD_CHARTYPE!(fc) as i32;
                        category = crate::tables::_pcre2_ucp_gentype_8[chartype as usize] as i32;
                        if ((category == ucp_L as i32
                            || category == ucp_N as i32
                            || chartype == ucp_Mn as i32
                            || chartype == ucp_Pc as i32) as BOOL)
                            == notmatch
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                PT_CLIST => {
                    /* The C label GOT_MAX (C 4567) sits just after this loop; it is
                    reached only by the two gotos inside it. */
                    'got_max: {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            let mut cp: *const u32;
                            let mut len: i32 = 1;
                            if Feptr!() >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            GETCHARLENTEST!(fc, Feptr!(), len, utf);
                            {
                                cp = crate::ucd::_pcre2_ucd_caseless_sets_8
                                    .as_ptr()
                                    .add((*F).fields.type_repeat.propvalue as usize);
                                loop {
                                    if fc < *cp {
                                        if notmatch != 0 {
                                            break;
                                        } else {
                                            break 'got_max; /* goto GOT_MAX */
                                        }
                                    }
                                    let t = *cp;
                                    cp = cp.add(1);
                                    if fc == t {
                                        if notmatch != 0 {
                                            break 'got_max; /* goto GOT_MAX */
                                        } else {
                                            break;
                                        }
                                    }
                                }
                            }

                            Feptr!() = Feptr!().add(len as usize);
                            i = i.wrapping_add(1);
                        }
                    }
                }

                PT_UCNC => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLENTEST!(fc, Feptr!(), len, utf);
                        if ((fc == 0x24u32 /* CHAR_DOLLAR_SIGN */
                            || fc == 0x40u32 /* CHAR_COMMERCIAL_AT */
                            || fc == 0x60u32 /* CHAR_GRAVE_ACCENT */
                            || (fc >= 0xa0 && fc <= 0xd7ff)
                            || fc >= 0xe000) as BOOL)
                            == notmatch
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                PT_BIDICL => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLENTEST!(fc, Feptr!(), len, utf);
                        if ((UCD_BIDICLASS!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
                            == notmatch
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                PT_BOOL => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let ok: BOOL;
                        let prop: *const ucd_record;
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLENTEST!(fc, Feptr!(), len, utf);
                        prop = GET_UCD!(fc);
                        ok = (MAPBIT!(
                            crate::ucd::_pcre2_ucd_boolprop_sets_8
                                .as_ptr()
                                .add(UCD_BPROPS_PROP!(prop) as usize),
                            (*F).fields.type_repeat.propvalue
                        ) != 0) as BOOL;
                        if ok == notmatch {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                _ => {
                    return PCRE2_ERROR_INTERNAL;
                }
            }

            /* Feptr is now past the end of the maximum run */

            if reptype == REPTYPE_POS {
                state = S_MAINLOOP; /* continue - No backtracking */
                continue 'sm;
            }

            /* After \C in UTF mode, Lstart_eptr might be in the middle of a
            Unicode character. Use <= Lstart_eptr to ensure backtracking doesn't
            go too far. */

            state = L_B_TR_MAX_PROP_BT;
            continue 'sm;
        }
        /* Match extended Unicode grapheme clusters. We will get here only if the
        support is in the binary; otherwise a compile-time error occurs. */
        else if (*F).fields.type_repeat.ctype == OP_EXTUNI {
            i = (*F).fields.type_repeat.min;
            while i < (*F).fields.type_repeat.max {
                if Feptr!() >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                    break;
                } else {
                    GETCHARINCTEST!(fc, Feptr!(), utf);
                    Feptr!() = crate::extuni::_pcre2_extuni_8(
                        fc,
                        Feptr!(),
                        (*mb).start_subject,
                        (*mb).end_subject,
                        utf,
                        null_mut(),
                    );
                }
                CHECK_PARTIAL!();
                i = i.wrapping_add(1);
            }

            /* Feptr is now past the end of the maximum run */

            if reptype == REPTYPE_POS {
                state = S_MAINLOOP; /* continue - No backtracking */
                continue 'sm;
            }

            state = L_B_TR_MAX_EXTUNI_BT;
            continue 'sm;
        } else if utf != 0 {
            match (*F).fields.type_repeat.ctype {
                OP_ANY => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        if IS_NEWLINE!(Feptr!()) != 0 {
                            break;
                        }
                        if (*mb).partial != 0
                            && Feptr!().add(1) >= (*mb).end_subject
                            && (*mb).nltype == NLTYPE_FIXED
                            && (*mb).nllen == 2
                            && *Feptr!() == (*mb).nl[0]
                        {
                            (*mb).hitend = TRUE;
                            if (*mb).partial > 1 {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        Feptr!() = Feptr!().add(1);
                        ACROSSCHAR!(
                            Feptr!() < (*mb).end_subject,
                            Feptr!(),
                            Feptr!() = Feptr!().add(1)
                        );
                        i = i.wrapping_add(1);
                    }
                }

                OP_ALLANY => {
                    if (*F).fields.type_repeat.max < u32::MAX {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if Feptr!() >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            Feptr!() = Feptr!().add(1);
                            ACROSSCHAR!(
                                Feptr!() < (*mb).end_subject,
                                Feptr!(),
                                Feptr!() = Feptr!().add(1)
                            );
                            i = i.wrapping_add(1);
                        }
                    } else {
                        Feptr!() = (*mb).end_subject; /* Unlimited UTF-8 repeat */
                        SCHECK_PARTIAL!();
                    }
                }

                /* The "byte" (i.e. "code unit") case is the same as non-UTF */
                OP_ANYBYTE => {
                    fc = (*F)
                        .fields
                        .type_repeat
                        .max
                        .wrapping_sub((*F).fields.type_repeat.min);
                    if fc
                        > (((*mb).end_subject as usize).wrapping_sub(Feptr!() as usize)) as u32
                    {
                        Feptr!() = (*mb).end_subject;
                        SCHECK_PARTIAL!();
                    } else {
                        Feptr!() = Feptr!().add(fc as usize);
                    }
                }

                OP_ANYNL => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLEN!(fc, Feptr!(), len);
                        if fc == 0x0du32
                        /* CHAR_CR */
                        {
                            Feptr!() = Feptr!().add(1);
                            if Feptr!() >= (*mb).end_subject {
                                break;
                            }
                            if *Feptr!() == 0x0au8 {
                                Feptr!() = Feptr!().add(1);
                            }
                        } else {
                            if fc != 0x0au32
                                && ((*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF
                                    || (fc != 0x0bu32
                                        && fc != 0x0cu32
                                        && fc != 0x85u32
                                        && fc != 0x2028u32
                                        && fc != 0x2029u32))
                            {
                                break;
                            }
                            Feptr!() = Feptr!().add(len as usize);
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_HSPACE | OP_HSPACE => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let gotspace: BOOL;
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLEN!(fc, Feptr!(), len);
                        match fc {
                            B_HSPACE_CASES!() => {
                                gotspace = TRUE;
                            }
                            _ => {
                                gotspace = FALSE;
                            }
                        }
                        if gotspace
                            == (((*F).fields.type_repeat.ctype == OP_NOT_HSPACE) as BOOL)
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_VSPACE | OP_VSPACE => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let gotspace: BOOL;
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLEN!(fc, Feptr!(), len);
                        match fc {
                            B_VSPACE_CASES!() => {
                                gotspace = TRUE;
                            }
                            _ => {
                                gotspace = FALSE;
                            }
                        }
                        if gotspace
                            == (((*F).fields.type_repeat.ctype == OP_NOT_VSPACE) as BOOL)
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_DIGIT => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLEN!(fc, Feptr!(), len);
                        if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                OP_DIGIT => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLEN!(fc, Feptr!(), len);
                        if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_WHITESPACE => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLEN!(fc, Feptr!(), len);
                        if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                OP_WHITESPACE => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLEN!(fc, Feptr!(), len);
                        if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_WORDCHAR => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLEN!(fc, Feptr!(), len);
                        if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                OP_WORDCHAR => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        let mut len: i32 = 1;
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        GETCHARLEN!(fc, Feptr!(), len);
                        if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
                            break;
                        }
                        Feptr!() = Feptr!().add(len as usize);
                        i = i.wrapping_add(1);
                    }
                }

                _ => {
                    return PCRE2_ERROR_INTERNAL;
                }
            }

            if reptype == REPTYPE_POS {
                state = S_MAINLOOP; /* continue - No backtracking */
                continue 'sm;
            }

            /* After \C in UTF mode, Lstart_eptr might be in the middle of a
            Unicode character. Use <= Lstart_eptr to ensure backtracking doesn't go
            too far. */

            state = L_B_TR_MAX_UTF_BT;
            continue 'sm;
        }
        /* Not UTF mode */
        else {
            match (*F).fields.type_repeat.ctype {
                OP_ANY => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        if IS_NEWLINE!(Feptr!()) != 0 {
                            break;
                        }
                        if (*mb).partial != 0
                            && Feptr!().add(1) >= (*mb).end_subject
                            && (*mb).nltype == NLTYPE_FIXED
                            && (*mb).nllen == 2
                            && *Feptr!() == (*mb).nl[0]
                        {
                            (*mb).hitend = TRUE;
                            if (*mb).partial > 1 {
                                return PCRE2_ERROR_PARTIAL;
                            }
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_ALLANY | OP_ANYBYTE => {
                    fc = (*F)
                        .fields
                        .type_repeat
                        .max
                        .wrapping_sub((*F).fields.type_repeat.min);
                    if fc
                        > (((*mb).end_subject as usize).wrapping_sub(Feptr!() as usize)) as u32
                    {
                        Feptr!() = (*mb).end_subject;
                        SCHECK_PARTIAL!();
                    } else {
                        Feptr!() = Feptr!().add(fc as usize);
                    }
                }

                OP_ANYNL => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        fc = *Feptr!() as u32;
                        if fc == 0x0du32
                        /* CHAR_CR */
                        {
                            Feptr!() = Feptr!().add(1);
                            if Feptr!() >= (*mb).end_subject {
                                break;
                            }
                            if *Feptr!() == 0x0au8 {
                                Feptr!() = Feptr!().add(1);
                            }
                        } else {
                            if fc != 0x0au32
                                && ((*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF
                                    || (fc != 0x0bu32 && fc != 0x0cu32 && fc != 0x85u32))
                            {
                                break;
                            }
                            Feptr!() = Feptr!().add(1);
                        }
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_HSPACE => {
                    /* C label ENDLOOP00 (C 5054) */
                    'endloop00: {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if Feptr!() >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            match *Feptr!() as u32 {
                                B_HSPACE_BYTE_CASES!() => {
                                    break 'endloop00; /* goto ENDLOOP00 */
                                }
                                _ => {
                                    Feptr!() = Feptr!().add(1);
                                }
                            }
                            i = i.wrapping_add(1);
                        }
                    }
                }

                OP_HSPACE => {
                    /* C label ENDLOOP01 (C 5075) */
                    'endloop01: {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if Feptr!() >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            match *Feptr!() as u32 {
                                B_HSPACE_BYTE_CASES!() => {
                                    Feptr!() = Feptr!().add(1);
                                }
                                _ => {
                                    break 'endloop01; /* goto ENDLOOP01 */
                                }
                            }
                            i = i.wrapping_add(1);
                        }
                    }
                }

                OP_NOT_VSPACE => {
                    /* C label ENDLOOP02 (C 5096) */
                    'endloop02: {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if Feptr!() >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            match *Feptr!() as u32 {
                                B_VSPACE_BYTE_CASES!() => {
                                    break 'endloop02; /* goto ENDLOOP02 */
                                }
                                _ => {
                                    Feptr!() = Feptr!().add(1);
                                }
                            }
                            i = i.wrapping_add(1);
                        }
                    }
                }

                OP_VSPACE => {
                    /* C label ENDLOOP03 (C 5117) */
                    'endloop03: {
                        i = (*F).fields.type_repeat.min;
                        while i < (*F).fields.type_repeat.max {
                            if Feptr!() >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            }
                            match *Feptr!() as u32 {
                                B_VSPACE_BYTE_CASES!() => {
                                    Feptr!() = Feptr!().add(1);
                                }
                                _ => {
                                    break 'endloop03; /* goto ENDLOOP03 */
                                }
                            }
                            i = i.wrapping_add(1);
                        }
                    }
                }

                OP_NOT_DIGIT => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        if MAX_255!(*Feptr!()) != 0
                            && (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_digit) != 0
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_DIGIT => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        if MAX_255!(*Feptr!()) == 0
                            || (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_digit) == 0
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_WHITESPACE => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        if MAX_255!(*Feptr!()) != 0
                            && (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_space) != 0
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_WHITESPACE => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        if MAX_255!(*Feptr!()) == 0
                            || (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_space) == 0
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_NOT_WORDCHAR => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        if MAX_255!(*Feptr!()) != 0
                            && (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_word) != 0
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                OP_WORDCHAR => {
                    i = (*F).fields.type_repeat.min;
                    while i < (*F).fields.type_repeat.max {
                        if Feptr!() >= (*mb).end_subject {
                            SCHECK_PARTIAL!();
                            break;
                        }
                        if MAX_255!(*Feptr!()) == 0
                            || (*(*mb).ctypes.add(*Feptr!() as usize) & ctype_word) == 0
                        {
                            break;
                        }
                        Feptr!() = Feptr!().add(1);
                        i = i.wrapping_add(1);
                    }
                }

                _ => {
                    return PCRE2_ERROR_INTERNAL;
                }
            }

            if reptype == REPTYPE_POS {
                state = S_MAINLOOP; /* continue - No backtracking */
                continue 'sm;
            }

            state = L_B_TR_MAX_NOUTF_BT;
            continue 'sm;
        }
    }
}

/* ===================================================================== *
 * REPEATTYPE, minimizing repeats with property tests (C 3776..4053).
 * Each C `for(;;) { RMATCH(Fecode, RMnn); ... }` becomes a loop-entry
 * state holding the RMATCH plus the RMnn state holding the loop body.
 * ===================================================================== */

/* C 3783 PT_LAMP */
L_B_TR_MIN_LAMP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM208 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM208 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    {
        let chartype: i32 = UCD_CHARTYPE!(fc) as i32;
        if ((chartype == ucp_Lu as i32
            || chartype == ucp_Ll as i32
            || chartype == ucp_Lt as i32) as BOOL)
            == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL)
        {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    state = L_B_TR_MIN_LAMP;
    continue 'sm;
}

/* C 3804 PT_GC */
L_B_TR_MIN_GC => {
    start_ecode = Fecode!();
    Freturn_id!() = RM209 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM209 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if ((UCD_CATEGORY!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
        == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL)
    {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_B_TR_MIN_GC;
    continue 'sm;
}

/* C 3821 PT_PC */
L_B_TR_MIN_PC => {
    start_ecode = Fecode!();
    Freturn_id!() = RM210 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM210 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if ((UCD_CHARTYPE!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
        == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL)
    {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_B_TR_MIN_PC;
    continue 'sm;
}

/* C 3838 PT_SC */
L_B_TR_MIN_SC => {
    start_ecode = Fecode!();
    Freturn_id!() = RM211 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM211 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if ((UCD_SCRIPT!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
        == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL)
    {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_B_TR_MIN_SC;
    continue 'sm;
}

/* C 3855 PT_SCX */
L_B_TR_MIN_SCX => {
    start_ecode = Fecode!();
    Freturn_id!() = RM224 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM224 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    {
        let prop: *const ucd_record = GET_UCD!(fc);
        let ok: BOOL = (((*prop).script as u32 == (*F).fields.type_repeat.propvalue)
            || MAPBIT!(
                crate::ucd::_pcre2_ucd_script_sets_8
                    .as_ptr()
                    .add(UCD_SCRIPTX_PROP!(prop) as usize),
                (*F).fields.type_repeat.propvalue
            ) != 0) as BOOL;
        if ok == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL) {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    state = L_B_TR_MIN_SCX;
    continue 'sm;
}

/* C 3877 PT_ALNUM */
L_B_TR_MIN_ALNUM => {
    start_ecode = Fecode!();
    Freturn_id!() = RM212 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM212 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    {
        let category: i32 = UCD_CATEGORY!(fc) as i32;
        if ((category == ucp_L as i32 || category == ucp_N as i32) as BOOL)
            == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL)
        {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    state = L_B_TR_MIN_ALNUM;
    continue 'sm;
}

/* C 3900 PT_SPACE / PT_PXSPACE */
L_B_TR_MIN_SPACE => {
    start_ecode = Fecode!();
    Freturn_id!() = RM213 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM213 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    match fc {
        B_HSPACE_VSPACE_CASES!() => {
            if (*F).fields.type_repeat.ctype == OP_NOTPROP {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }
        _ => {
            if ((UCD_CATEGORY!(fc) == ucp_Z) as BOOL)
                == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL)
            {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }
    }
    state = L_B_TR_MIN_SPACE;
    continue 'sm;
}

/* C 3928 PT_WORD */
L_B_TR_MIN_WORD => {
    start_ecode = Fecode!();
    Freturn_id!() = RM214 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM214 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    {
        let chartype: i32 = UCD_CHARTYPE!(fc) as i32;
        let category: i32 = crate::tables::_pcre2_ucp_gentype_8[chartype as usize] as i32;
        if ((category == ucp_L as i32
            || category == ucp_N as i32
            || chartype == ucp_Mn as i32
            || chartype == ucp_Pc as i32) as BOOL)
            == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL)
        {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    state = L_B_TR_MIN_WORD;
    continue 'sm;
}

/* C 3951 PT_CLIST */
L_B_TR_MIN_CLIST => {
    start_ecode = Fecode!();
    Freturn_id!() = RM215 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM215 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    {
        let mut cp: *const u32 = crate::ucd::_pcre2_ucd_caseless_sets_8
            .as_ptr()
            .add((*F).fields.type_repeat.propvalue as usize);
        loop {
            if fc < *cp {
                if (*F).fields.type_repeat.ctype == OP_NOTPROP {
                    break;
                }
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            let t = *cp;
            cp = cp.add(1);
            if fc == t {
                if (*F).fields.type_repeat.ctype == OP_NOTPROP {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
                break;
            }
        }
    }
    state = L_B_TR_MIN_CLIST;
    continue 'sm;
}

/* C 3988 PT_UCNC */
L_B_TR_MIN_UCNC => {
    start_ecode = Fecode!();
    Freturn_id!() = RM216 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM216 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if ((fc == 0x24u32 /* CHAR_DOLLAR_SIGN */
        || fc == 0x40u32 /* CHAR_COMMERCIAL_AT */
        || fc == 0x60u32 /* CHAR_GRAVE_ACCENT */
        || (fc >= 0xa0 && fc <= 0xd7ff)
        || fc >= 0xe000) as BOOL)
        == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL)
    {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_B_TR_MIN_UCNC;
    continue 'sm;
}

/* C 4007 PT_BIDICL */
L_B_TR_MIN_BIDICL => {
    start_ecode = Fecode!();
    Freturn_id!() = RM223 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM223 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    if ((UCD_BIDICLASS!(fc) == (*F).fields.type_repeat.propvalue) as BOOL)
        == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL)
    {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_B_TR_MIN_BIDICL;
    continue 'sm;
}

/* C 4024 PT_BOOL */
L_B_TR_MIN_BOOL => {
    start_ecode = Fecode!();
    Freturn_id!() = RM222 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM222 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINCTEST!(fc, Feptr!(), utf);
    {
        let prop: *const ucd_record = GET_UCD!(fc);
        let ok: BOOL = (MAPBIT!(
            crate::ucd::_pcre2_ucd_boolprop_sets_8
                .as_ptr()
                .add(UCD_BPROPS_PROP!(prop) as usize),
            (*F).fields.type_repeat.propvalue
        ) != 0) as BOOL;
        if ok == (((*F).fields.type_repeat.ctype == OP_NOTPROP) as BOOL) {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    state = L_B_TR_MIN_BOOL;
    continue 'sm;
}

/* C 4059: minimizing repeat of extended Unicode sequences */
L_B_TR_MIN_EXTUNI => {
    start_ecode = Fecode!();
    Freturn_id!() = RM217 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM217 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    } else {
        GETCHARINCTEST!(fc, Feptr!(), utf);
        Feptr!() = crate::extuni::_pcre2_extuni_8(
            fc,
            Feptr!(),
            (*mb).start_subject,
            (*mb).end_subject,
            utf,
            null_mut(),
        );
    }
    CHECK_PARTIAL!();
    state = L_B_TR_MIN_EXTUNI;
    continue 'sm;
}

/* C 4086: UTF mode for non-property testing character types */
L_B_TR_MIN_UTF => {
    start_ecode = Fecode!();
    Freturn_id!() = RM218 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM218 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if (*F).fields.type_repeat.ctype == OP_ANY && IS_NEWLINE!(Feptr!()) != 0 {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    GETCHARINC!(fc, Feptr!());
    match (*F).fields.type_repeat.ctype {
        OP_ANY => {
            /* This is the non-NL case */
            if (*mb).partial != 0 /* Take care with CRLF partial */
                && Feptr!() >= (*mb).end_subject
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
            0x0du32 /* CHAR_CR */ => {
                if Feptr!() < (*mb).end_subject && *Feptr!() == 0x0au8 {
                    Feptr!() = Feptr!().add(1);
                }
            }
            0x0au32 /* CHAR_LF */ => {}
            0x0bu32 | 0x0cu32 | 0x85u32 | 0x2028u32 | 0x2029u32 => {
                if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }
            _ => {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        },

        OP_NOT_HSPACE => match fc {
            B_HSPACE_CASES!() => {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            _ => {}
        },

        OP_HSPACE => match fc {
            B_HSPACE_CASES!() => {}
            _ => {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        },

        OP_NOT_VSPACE => match fc {
            B_VSPACE_CASES!() => {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            _ => {}
        },

        OP_VSPACE => match fc {
            B_VSPACE_CASES!() => {}
            _ => {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        },

        OP_NOT_DIGIT => {
            if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        OP_DIGIT => {
            if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        OP_NOT_WHITESPACE => {
            if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        OP_WHITESPACE => {
            if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        OP_NOT_WORDCHAR => {
            if fc < 256 && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        OP_WORDCHAR => {
            if fc >= 256 || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        _ => {
            return PCRE2_ERROR_INTERNAL;
        }
    }
    state = L_B_TR_MIN_UTF;
    continue 'sm;
}

/* C 4218: not UTF mode */
L_B_TR_MIN_NOUTF => {
    start_ecode = Fecode!();
    Freturn_id!() = RM33 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM33 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.type_repeat.min;
        (*F).fields.type_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.type_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    if Feptr!() >= (*mb).end_subject {
        SCHECK_PARTIAL!();
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if (*F).fields.type_repeat.ctype == OP_ANY && IS_NEWLINE!(Feptr!()) != 0 {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    fc = *Feptr!() as u32;
    Feptr!() = Feptr!().add(1);
    match (*F).fields.type_repeat.ctype {
        OP_ANY => {
            /* This is the non-NL case */
            if (*mb).partial != 0 /* Take care with CRLF partial */
                && Feptr!() >= (*mb).end_subject
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
            0x0du32 /* CHAR_CR */ => {
                if Feptr!() < (*mb).end_subject && *Feptr!() == 0x0au8 {
                    Feptr!() = Feptr!().add(1);
                }
            }
            0x0au32 /* CHAR_LF */ => {}
            0x0bu32 | 0x0cu32 | 0x85u32 => {
                if (*mb).bsr_convention as u32 == PCRE2_BSR_ANYCRLF {
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }
            _ => {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        },

        OP_NOT_HSPACE => match fc {
            B_HSPACE_BYTE_CASES!() => {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            _ => {}
        },

        OP_HSPACE => match fc {
            B_HSPACE_BYTE_CASES!() => {}
            _ => {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        },

        OP_NOT_VSPACE => match fc {
            B_VSPACE_BYTE_CASES!() => {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
            _ => {}
        },

        OP_VSPACE => match fc {
            B_VSPACE_BYTE_CASES!() => {}
            _ => {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        },

        OP_NOT_DIGIT => {
            if MAX_255!(fc) != 0 && (*(*mb).ctypes.add(fc as usize) & ctype_digit) != 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        OP_DIGIT => {
            if MAX_255!(fc) == 0 || (*(*mb).ctypes.add(fc as usize) & ctype_digit) == 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        OP_NOT_WHITESPACE => {
            if MAX_255!(fc) != 0 && (*(*mb).ctypes.add(fc as usize) & ctype_space) != 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        OP_WHITESPACE => {
            if MAX_255!(fc) == 0 || (*(*mb).ctypes.add(fc as usize) & ctype_space) == 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        OP_NOT_WORDCHAR => {
            if MAX_255!(fc) != 0 && (*(*mb).ctypes.add(fc as usize) & ctype_word) != 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        OP_WORDCHAR => {
            if MAX_255!(fc) == 0 || (*(*mb).ctypes.add(fc as usize) & ctype_word) == 0 {
                rrc = MATCH_NOMATCH;
                state = S_RETURN_SWITCH;
                continue 'sm;
            }
        }

        _ => {
            return PCRE2_ERROR_INTERNAL;
        }
    }
    state = L_B_TR_MIN_NOUTF;
    continue 'sm;
}

/* ===================================================================== *
 * REPEATTYPE, maximizing repeats: the four backtracking loops. Leaving
 * any of these loops in C falls out of the enclosing block and reaches
 * the `break` at C 5224, i.e. the main loop.
 * ===================================================================== */

/* C 4638, property tests */
L_B_TR_MAX_PROP_BT => {
    if Feptr!() <= (*F).fields.type_repeat.start_eptr {
        state = S_MAINLOOP; /* break out of the for(;;) -> break (C 5224) */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM221 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM221 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Feptr!() = Feptr!().sub(1);
    if utf != 0 {
        BACKCHAR!(Feptr!());
    }
    state = L_B_TR_MAX_PROP_BT;
    continue 'sm;
}

/* C 4678, extended grapheme clusters */
L_B_TR_MAX_EXTUNI_BT => {
    if Feptr!() <= (*F).fields.type_repeat.start_eptr {
        state = S_MAINLOOP; /* At start of char run -> break (C 5224) */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM219 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM219 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }

    /* Backtracking over an extended grapheme cluster involves inspecting
    the previous two characters (if present) to see if a break is
    permitted between them. */

    {
        let mut lgb: i32;
        let mut rgb: i32;
        let mut fptr: PCRE2_SPTR;

        Feptr!() = Feptr!().sub(1);
        if utf == 0 {
            fc = *Feptr!() as u32;
        } else {
            BACKCHAR!(Feptr!());
            GETCHAR!(fc, Feptr!());
        }
        rgb = UCD_GRAPHBREAK!(fc) as i32;

        loop {
            if Feptr!() <= (*F).fields.type_repeat.start_eptr {
                break; /* At start of char run */
            }
            fptr = Feptr!().sub(1);
            if utf == 0 {
                fc = *fptr as u32;
            } else {
                BACKCHAR!(fptr);
                GETCHAR!(fc, fptr);
            }
            lgb = UCD_GRAPHBREAK!(fc) as i32;
            if (crate::tables::_pcre2_ucp_gbtable_8[lgb as usize] & (1u32 << rgb)) == 0 {
                break;
            }
            Feptr!() = fptr;
            rgb = lgb;
        }
    }
    state = L_B_TR_MAX_EXTUNI_BT;
    continue 'sm;
}

/* C 4957, UTF mode */
L_B_TR_MAX_UTF_BT => {
    if Feptr!() <= (*F).fields.type_repeat.start_eptr {
        state = S_MAINLOOP; /* break -> break (C 5224) */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM220 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM220 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Feptr!() = Feptr!().sub(1);
    BACKCHAR!(Feptr!());
    if (*F).fields.type_repeat.ctype == OP_ANYNL
        && Feptr!() > (*F).fields.type_repeat.start_eptr
        && *Feptr!() == 0x0au8 /* CHAR_NL */
        && *Feptr!().offset(-1) == 0x0du8
    /* CHAR_CR */
    {
        Feptr!() = Feptr!().sub(1);
    }
    state = L_B_TR_MAX_UTF_BT;
    continue 'sm;
}

/* C 5213, not UTF mode */
L_B_TR_MAX_NOUTF_BT => {
    if Feptr!() == (*F).fields.type_repeat.start_eptr {
        state = S_MAINLOOP; /* break -> break (C 5224) */
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM34 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM34 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Feptr!() = Feptr!().sub(1);
    if (*F).fields.type_repeat.ctype == OP_ANYNL
        && Feptr!() > (*F).fields.type_repeat.start_eptr
        && *Feptr!() == 0x0au8 /* CHAR_LF */
        && *Feptr!().offset(-1) == 0x0du8
    /* CHAR_CR */
    {
        Feptr!() = Feptr!().sub(1);
    }
    state = L_B_TR_MAX_NOUTF_BT;
    continue 'sm;
}

/* ===================================================================== *
 * C 5278: REF_REPEAT - set up for repetition of a back reference, or
 * handle the non-repeated case.
 * ===================================================================== */
L_REF_REPEAT => {
    match *Fecode!() as u32 {
        OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY | OP_CRMINQUERY
        | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
            /* fc = *Fecode++ - OP_CRSTAR; */
            {
                let t = *Fecode!();
                Fecode!() = Fecode!().add(1);
                fc = (t as u32).wrapping_sub(OP_CRSTAR);
            }
            (*F).fields.ref_repeat.min = rep_min[fc as usize];
            (*F).fields.ref_repeat.max = rep_max[fc as usize];
            reptype = rep_typ[fc as usize];
        }

        OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
            (*F).fields.ref_repeat.min = GET2!(Fecode!(), 1);
            (*F).fields.ref_repeat.max = GET2!(Fecode!(), 1 + IMM2_SIZE);
            reptype = rep_typ[(*Fecode!() as u32).wrapping_sub(OP_CRSTAR) as usize];
            if (*F).fields.ref_repeat.max == 0 {
                (*F).fields.ref_repeat.max = u32::MAX; /* Max 0 => infinity */
            }
            Fecode!() = Fecode!().add(1 + 2 * IMM2_SIZE);
        }

        /* No repeat follows */
        _ => {
            {
                rrc = match_ref(
                    (*F).fields.ref_repeat.offset,
                    (*F).byte1 as BOOL,
                    (*F).byte2 as i32,
                    F,
                    mb,
                    &mut length,
                );
                if rrc != 0 {
                    if rrc > 0 {
                        Feptr!() = (*mb).end_subject; /* Partial match */
                    }
                    CHECK_PARTIAL!();
                    rrc = MATCH_NOMATCH;
                    state = S_RETURN_SWITCH;
                    continue 'sm;
                }
            }
            Feptr!() = Feptr!().add(length);
            state = S_MAINLOOP; /* continue - With the main loop */
            continue 'sm;
        }
    }

    /* Handle repeated back references. If a set group has length zero, just
    continue with the main loop, because it matches however many times. For an
    unset reference, if the minimum is zero, we can also just continue. We can
    also continue if PCRE2_MATCH_UNSET_BACKREF is set, because this makes unset
    group behave as a zero-length group. For any other unset cases, carrying
    on will result in NOMATCH. */

    if (*F).fields.ref_repeat.offset < Foffset_top!()
        && *Fovector!().add((*F).fields.ref_repeat.offset) != PCRE2_UNSET
    {
        if *Fovector!().add((*F).fields.ref_repeat.offset)
            == *Fovector!().add((*F).fields.ref_repeat.offset + 1)
        {
            state = S_MAINLOOP; /* continue */
            continue 'sm;
        }
    } else
    /* Group is not set */
    {
        if (*F).fields.ref_repeat.min == 0
            || ((*mb).poptions & PCRE2_MATCH_UNSET_BACKREF) != 0
        {
            state = S_MAINLOOP; /* continue */
            continue 'sm;
        }
    }

    /* First, ensure the minimum number of matches are present. */

    i = 1;
    while i <= (*F).fields.ref_repeat.min {
        let mut slength: PCRE2_SIZE = 0;
        rrc = match_ref(
            (*F).fields.ref_repeat.offset,
            (*F).byte1 as BOOL,
            (*F).byte2 as i32,
            F,
            mb,
            &mut slength,
        );
        if rrc != 0 {
            if rrc > 0 {
                Feptr!() = (*mb).end_subject; /* Partial match */
            }
            CHECK_PARTIAL!();
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        Feptr!() = Feptr!().add(slength);
        i = i.wrapping_add(1);
    }

    /* If min = max, we are done. They are not both allowed to be zero. */

    if (*F).fields.ref_repeat.min == (*F).fields.ref_repeat.max {
        state = S_MAINLOOP; /* continue */
        continue 'sm;
    }

    /* If minimizing, keep trying and advancing the pointer. */

    if reptype == REPTYPE_MIN {
        state = L_B_REF_MINLOOP;
        continue 'sm;
    }
    /* If maximizing, find the longest string and work backwards, as long as
    the matched lengths for each iteration are the same. */
    else {
        let mut samelengths: BOOL = TRUE;
        (*F).fields.ref_repeat.start = Feptr!(); /* Starting position */
        (*F).fields.ref_repeat.length = (*Fovector!()
            .add((*F).fields.ref_repeat.offset + 1))
        .wrapping_sub(*Fovector!().add((*F).fields.ref_repeat.offset));

        i = (*F).fields.ref_repeat.min;
        while i < (*F).fields.ref_repeat.max {
            let mut slength: PCRE2_SIZE = 0;
            rrc = match_ref(
                (*F).fields.ref_repeat.offset,
                (*F).byte1 as BOOL,
                (*F).byte2 as i32,
                F,
                mb,
                &mut slength,
            );
            if rrc != 0 {
                /* Can't use CHECK_PARTIAL because we don't want to update Feptr in
                the soft partial matching case. */

                if rrc > 0 && (*mb).partial != 0 && (*mb).end_subject > (*mb).start_used_ptr {
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
            Feptr!() = Feptr!().add(slength);
            i = i.wrapping_add(1);
        }

        /* No recursion if the repeat type is possessive. */
        if reptype == REPTYPE_POS {
            state = S_MAINLOOP; /* break */
            continue 'sm;
        }

        /* If the length matched for each repetition is the same as the length of
        the captured group, we can easily work backwards. This is the normal
        case. However, in caseless UTF-8 mode there are pairs of case-equivalent
        characters whose lengths (in terms of code units) differ. However, this
        is very rare, so we handle it by re-matching fewer and fewer times. */

        if samelengths != 0 {
            state = L_B_REF_SAMELEN;
            continue 'sm;
        }
        /* The rare case of non-matching lengths. Re-scan the repetition for each
        iteration. We know that match_ref() will succeed every time. */
        else {
            (*F).fields.ref_repeat.max = i;
            state = L_B_REF_DIFFLEN;
            continue 'sm;
        }
    }
}

/* C 5360: minimizing repeat of a back reference */
L_B_REF_MINLOOP => {
    start_ecode = Fecode!();
    Freturn_id!() = RM20 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM20 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    {
        let t = (*F).fields.ref_repeat.min;
        (*F).fields.ref_repeat.min = t.wrapping_add(1);
        if t >= (*F).fields.ref_repeat.max {
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
    {
        let mut slength: PCRE2_SIZE = 0;
        rrc = match_ref(
            (*F).fields.ref_repeat.offset,
            (*F).byte1 as BOOL,
            (*F).byte2 as i32,
            F,
            mb,
            &mut slength,
        );
        if rrc != 0 {
            if rrc > 0 {
                Feptr!() = (*mb).end_subject; /* Partial match */
            }
            CHECK_PARTIAL!();
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
        Feptr!() = Feptr!().add(slength);
    }
    state = L_B_REF_MINLOOP;
    continue 'sm;
}

/* C 5421: maximizing, all repetitions the same length */
L_B_REF_SAMELEN => {
    if !(Feptr!() >= (*F).fields.ref_repeat.start) {
        /* End of the while loop: fall through to RRETURN(MATCH_NOMATCH) at
        C 5451. */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    start_ecode = Fecode!();
    Freturn_id!() = RM21 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM21 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Feptr!() = Feptr!().wrapping_sub((*F).fields.ref_repeat.length);
    state = L_B_REF_SAMELEN;
    continue 'sm;
}

/* C 5435: maximizing, the rare case of differing lengths */
L_B_REF_DIFFLEN => {
    start_ecode = Fecode!();
    Freturn_id!() = RM22 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}
RM22 => {
    if rrc != MATCH_NOMATCH {
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    if Feptr!() == (*F).fields.ref_repeat.start {
        /* Failed after minimal repetition: break out of the for(;;) and fall
        through to RRETURN(MATCH_NOMATCH) at C 5451. */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Feptr!() = (*F).fields.ref_repeat.start;
    (*F).fields.ref_repeat.max = (*F).fields.ref_repeat.max.wrapping_sub(1);
    i = (*F).fields.ref_repeat.min;
    while i < (*F).fields.ref_repeat.max {
        let mut slength: PCRE2_SIZE = 0;
        let _ = match_ref(
            (*F).fields.ref_repeat.offset,
            (*F).byte1 as BOOL,
            (*F).byte2 as i32,
            F,
            mb,
            &mut slength,
        );
        Feptr!() = Feptr!().add(slength);
        i = i.wrapping_add(1);
    }
    state = L_B_REF_DIFFLEN;
    continue 'sm;
}
/* ---- chunk C: STATES ---- */

/* ---- chunk C: labels and RMATCH resume points ---- */

/* POSSESSIVE_NON_CAPTURE: C 5545 */
L_POSSESSIVE_NON_CAPTURE => {
    (*F).fields.op_brapos.frame_type = GF_NOCAPTURE; /* Remembered frame type */
    /* goto POSSESSIVE_GROUP */
    state = L_POSSESSIVE_GROUP;
    continue 'sm;
}

/* POSSESSIVE_CAPTURE: C 5553 */
L_POSSESSIVE_CAPTURE => {
    number = GET2!(Fecode!(), 1 + LINK_SIZE);
    (*F).fields.op_brapos.frame_type = GF_CAPTURE | number; /* Remembered frame type */
    /* fall through to POSSESSIVE_GROUP */
    state = L_POSSESSIVE_GROUP;
    continue 'sm;
}

/* POSSESSIVE_GROUP: C 5557 */
L_POSSESSIVE_GROUP => {
    (*F).byte1 = FALSE as u8; /* Lmatched_once: never matched */
    (*F).fields.op_brapos.start_group = Fecode!(); /* Start of this group */
    state = S_BRAPOS_LOOP;
    continue 'sm;
}

/* Top of the for(;;) at C 5561 */
S_BRAPOS_LOOP => {
    (*F).fields.op_brapos.start_eptr = Feptr!(); /* Position at group start */
    group_frame_type = (*F).fields.op_brapos.frame_type;
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode], RM8) */
    start_ecode = Fecode!().add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize);
    Freturn_id!() = RM8 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM8 => {
    if rrc == MATCH_KETRPOS {
        (*F).byte1 = TRUE as u8; /* Matched at least once */
        if Feptr!() == (*F).fields.op_brapos.start_eptr {
            /* Empty match; skip to end */
            loop {
                Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
                if *Fecode!() as u32 != OP_ALT {
                    break;
                }
            }
            /* break out of the for(;;) */
            state = S_BRAPOS_AFTER;
            continue 'sm;
        }

        Fecode!() = (*F).fields.op_brapos.start_group;
        /* continue the for(;;) */
        state = S_BRAPOS_LOOP;
        continue 'sm;
    }

    /* See comment above about handling THEN. */

    if rrc == MATCH_THEN {
        let next_ecode: PCRE2_SPTR = Fecode!().add(GET!(Fecode!(), 1) as usize);
        if (*mb).verb_ecode_ptr < next_ecode
            && (*Fecode!() as u32 == OP_ALT || *next_ecode as u32 == OP_ALT)
        {
            rrc = MATCH_NOMATCH;
        }
    }

    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
    if *Fecode!() as u32 != OP_ALT {
        /* break out of the for(;;) */
        state = S_BRAPOS_AFTER;
        continue 'sm;
    }
    state = S_BRAPOS_LOOP;
    continue 'sm;
}

/* C 5594: success if matched something or zero repeat allowed */
S_BRAPOS_AFTER => {
    if (*F).byte1 != 0 || (*F).byte2 != 0 {
        Fecode!() = Fecode!().add(1 + LINK_SIZE);
        state = S_MAINLOOP;
        continue 'sm;
    }

    /* RRETURN(MATCH_NOMATCH) */
    rrc = MATCH_NOMATCH;
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* Top of the for(;;) at C 5629 (OP_BRA, no THEN in the pattern) */
S_BRA_LOOP => {
    let current_branch: PCRE2_SPTR = Fecode!();
    let next_branch: PCRE2_SPTR = current_branch.add(GET!(current_branch, 1) as usize);

    if *next_branch as u32 != OP_ALT {
        /* break: hit the start of the final branch. Continue at this level. */
        Fecode!() = Fecode!().add(1 + LINK_SIZE);
        state = S_MAINLOOP;
        continue 'sm;
    }

    /* This is never the final branch. We do not need to test for MATCH_THEN
    here because this code is not used when there is a THEN in the pattern. */

    Fecode!() = next_branch;

    /* RMATCH(current_branch + 1 + LINK_SIZE, RM1) */
    start_ecode = current_branch.add(1 + LINK_SIZE);
    Freturn_id!() = RM1 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM1 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = S_BRA_LOOP;
    continue 'sm;
}

/* GROUPLOOP: C 5676 */
L_GROUPLOOP => {
    group_frame_type = (*F).fields.op_bra.frame_type;
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode], RM2) */
    start_ecode = Fecode!().add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize);
    Freturn_id!() = RM2 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM2 => {
    if rrc == MATCH_THEN {
        let next_ecode: PCRE2_SPTR = Fecode!().add(GET!(Fecode!(), 1) as usize);
        if (*mb).verb_ecode_ptr < next_ecode
            && (*Fecode!() as u32 == OP_ALT || *next_ecode as u32 == OP_ALT)
        {
            rrc = MATCH_NOMATCH;
        }
    }
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
    if *Fecode!() as u32 != OP_ALT {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = L_GROUPLOOP;
    continue 'sm;
}

/* Top of the for(;;) at C 5743 (OP_RECURSE) */
S_RECURSE_LOOP => {
    group_frame_type = (*F).fields.op_recurse.frame_type;
    /* RMATCH(Lstart_branch + PRIV(OP_lengths)[*Lstart_branch], RM11) */
    start_ecode = (*F).fields.op_recurse.start_branch.add(
        crate::tables::_pcre2_OP_lengths_8[*(*F).fields.op_recurse.start_branch as usize] as usize,
    );
    Freturn_id!() = RM11 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM11 => {
    let next_ecode: PCRE2_SPTR = (*F)
        .fields
        .op_recurse
        .start_branch
        .add(GET!((*F).fields.op_recurse.start_branch, 1) as usize);

    /* Handle backtracking verbs, which are defined in a range that can
    easily be tested for. PCRE does not allow THEN, SKIP, PRUNE or COMMIT to
    escape beyond a recursion; they cause a NOMATCH for the entire recursion.

    When one of these verbs triggers, the current recursion group number is
    recorded. If it matches the recursion we are processing, the verb
    happened within the recursion and we must deal with it. Otherwise it must
    have happened after the recursion completed, and so has to be passed
    back. See comment above about handling THEN. */

    if rrc >= MATCH_BACKTRACK_MIN
        && rrc <= MATCH_BACKTRACK_MAX
        && (*mb).verb_current_recurse == ((*F).fields.op_recurse.frame_type ^ GF_RECURSE)
    {
        if rrc == MATCH_THEN
            && (*mb).verb_ecode_ptr < next_ecode
            && (*(*F).fields.op_recurse.start_branch as u32 == OP_ALT
                || *next_ecode as u32 == OP_ALT)
        {
            rrc = MATCH_NOMATCH;
        } else {
            /* RRETURN(MATCH_NOMATCH) */
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }

    /* Note that carrying on after (*ACCEPT) in a recursion is handled in the
    OP_ACCEPT code. Nothing needs to be done here. */

    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    (*F).fields.op_recurse.start_branch = next_ecode;
    if *(*F).fields.op_recurse.start_branch as u32 != OP_ALT {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = S_RECURSE_LOOP;
    continue 'sm;
}

/* Top of the for(;;) at C 5793 (positive assertions) */
S_ASSERT_LOOP => {
    group_frame_type = GF_NOCAPTURE;
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode], RM3) */
    start_ecode = Fecode!().add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize);
    Freturn_id!() = RM3 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM3 => {
    if rrc == MATCH_ACCEPT {
        copy_nonoverlapping(
            (assert_accept_frame as *const u8).add(OVECTOR_OFFSET_IN_HEAPFRAME),
            Fovector!() as *mut u8,
            (*assert_accept_frame).offset_top * core::mem::size_of::<PCRE2_SIZE>(),
        );
        Foffset_top!() = (*assert_accept_frame).offset_top;
        Fmark!() = (*assert_accept_frame).mark;
        /* break out of the for(;;) */
        loop {
            Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
            if *Fecode!() as u32 != OP_ALT {
                break;
            }
        }
        Fecode!() = Fecode!().add(1 + LINK_SIZE);
        state = S_MAINLOOP;
        continue 'sm;
    }
    if rrc != MATCH_NOMATCH && rrc != MATCH_THEN {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
    if *Fecode!() as u32 != OP_ALT {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = S_ASSERT_LOOP;
    continue 'sm;
}

/* Top of the for(;;) at C 5822 (negative assertions) */
S_ASSERTNOT_LOOP => {
    group_frame_type = GF_NOCAPTURE;
    /* RMATCH(Fecode + PRIV(OP_lengths)[*Fecode], RM4) */
    start_ecode = Fecode!().add(crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize);
    Freturn_id!() = RM4 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM4 => {
    match rrc {
        /* Assertion matched, therefore it fails. */
        MATCH_ACCEPT | MATCH_MATCH => {
            /* RRETURN(MATCH_NOMATCH) */
            rrc = MATCH_NOMATCH;
            state = S_RETURN_SWITCH;
            continue 'sm;
        }

        /* Branch failed, try next if present. */
        MATCH_NOMATCH | MATCH_THEN => {
            Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
            if *Fecode!() as u32 != OP_ALT {
                /* goto ASSERT_NOT_FAILED */
                state = L_ASSERT_NOT_FAILED;
                continue 'sm;
            }
            /* break out of the switch; round the for(;;) again */
            state = S_ASSERTNOT_LOOP;
            continue 'sm;
        }

        /* Assertion forced to fail, therefore continue. */
        MATCH_COMMIT | MATCH_SKIP | MATCH_PRUNE => {
            loop {
                Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
                if *Fecode!() as u32 != OP_ALT {
                    break;
                }
            }
            /* goto ASSERT_NOT_FAILED */
            state = L_ASSERT_NOT_FAILED;
            continue 'sm;
        }

        /* Pass back any other return */
        _ => {
            /* RRETURN(rrc) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }
}

/* ASSERT_NOT_FAILED: C 5853. None of the branches have matched or there was
a backtrack to (*COMMIT), (*SKIP), (*PRUNE), or (*THEN) in the last branch.
This is success for a negative assertion, so carry on. */
L_ASSERT_NOT_FAILED => {
    Fecode!() = Fecode!().add(1 + LINK_SIZE);
    state = S_MAINLOOP;
    continue 'sm;
}

/* Top of the for(;;) at C 5878 (OP_ASSERT_SCS condition scan) */
S_SCS_CREF_LOOP => {
    if *scs_ecode as u32 == OP_CREF {
        length += 1 + IMM2_SIZE;
        offset = ((GET2!(scs_ecode, 1) << 1).wrapping_sub(2)) as PCRE2_SIZE;
        scs_ecode = scs_ecode.add(1 + IMM2_SIZE);
        if offset < Foffset_top!() && *Fovector!().add(offset) != PCRE2_UNSET {
            /* goto SCS_OFFSET_FOUND */
            state = L_SCS_OFFSET_FOUND;
            continue 'sm;
        }
        state = S_SCS_CREF_LOOP;
        continue 'sm;
    }

    if *scs_ecode as u32 != OP_DNCREF {
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }

    let mut count: i32 = GET2!(scs_ecode, 1 + IMM2_SIZE) as i32;
    let mut slot: PCRE2_SPTR = (*mb)
        .name_table
        .add((GET2!(scs_ecode, 1) as usize) * ((*mb).name_entry_size as usize));
    length += 1 + 2 * IMM2_SIZE;
    scs_ecode = scs_ecode.add(1 + 2 * IMM2_SIZE);

    while count > 0 {
        offset = ((GET2!(slot, 0) << 1).wrapping_sub(2)) as PCRE2_SIZE;
        if offset < Foffset_top!() && *Fovector!().add(offset) != PCRE2_UNSET {
            /* goto SCS_OFFSET_FOUND */
            state = L_SCS_OFFSET_FOUND;
            continue 'sm;
        }
        slot = slot.add((*mb).name_entry_size as usize);
        count -= 1;
    }
    state = S_SCS_CREF_LOOP;
    continue 'sm;
}

/* SCS_OFFSET_FOUND: C 5907 */
L_SCS_OFFSET_FOUND => {
    /* Skip remaining options. */
    loop {
        if *scs_ecode as u32 == OP_CREF {
            length += 1 + IMM2_SIZE;
            scs_ecode = scs_ecode.add(1 + IMM2_SIZE);
        } else if *scs_ecode as u32 == OP_DNCREF {
            length += 1 + 2 * IMM2_SIZE;
            scs_ecode = scs_ecode.add(1 + 2 * IMM2_SIZE);
        } else {
            break;
        }
    }

    (*F).fields.op_assert_scs.saved_end_subject = (*mb).end_subject;
    (*F).fields.op_assert_scs.true_end_extra =
        ((*mb).true_end_subject as usize) - ((*mb).end_subject as usize);
    (*F).fields.op_assert_scs.saved_eptr = Feptr!();
    (*F).fields.op_assert_scs.saved_moptions = (*mb).moptions;

    Feptr!() = (*mb).start_subject.add(*Fovector!().add(offset));
    (*mb).end_subject = (*mb).start_subject.add(*Fovector!().add(offset + 1));
    (*mb).true_end_subject = (*mb).end_subject;
    (*mb).moptions &= !PCRE2_NOTEOL;

    state = S_SCS_MATCH_LOOP;
    continue 'sm;
}

/* Top of the for(;;) at C 5936 */
S_SCS_MATCH_LOOP => {
    group_frame_type = GF_NOCAPTURE;
    /* RMATCH(Fecode + 1 + LINK_SIZE + length, RM38) */
    start_ecode = Fecode!().add(1 + LINK_SIZE + length);
    Freturn_id!() = RM38 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM38 => {
    if rrc == MATCH_ACCEPT {
        copy_nonoverlapping(
            (assert_accept_frame as *const u8).add(OVECTOR_OFFSET_IN_HEAPFRAME),
            Fovector!() as *mut u8,
            (*assert_accept_frame).offset_top * core::mem::size_of::<PCRE2_SIZE>(),
        );
        Foffset_top!() = (*assert_accept_frame).offset_top;
        Fmark!() = (*assert_accept_frame).mark;
        (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
        (*mb).true_end_subject = (*mb)
            .end_subject
            .add((*F).fields.op_assert_scs.true_end_extra);
        (*mb).moptions = (*F).fields.op_assert_scs.saved_moptions;
        /* break out of the for(;;) */
        loop {
            Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
            if *Fecode!() as u32 != OP_ALT {
                break;
            }
        }
        Fecode!() = Fecode!().add(1 + LINK_SIZE);
        Feptr!() = (*F).fields.op_assert_scs.saved_eptr;
        state = S_MAINLOOP;
        continue 'sm;
    }

    if rrc != MATCH_NOMATCH && rrc != MATCH_THEN {
        (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
        (*mb).true_end_subject = (*mb)
            .end_subject
            .add((*F).fields.op_assert_scs.true_end_extra);
        (*mb).moptions = (*F).fields.op_assert_scs.saved_moptions;
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }

    Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
    if *Fecode!() as u32 != OP_ALT {
        (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
        (*mb).true_end_subject = (*mb)
            .end_subject
            .add((*F).fields.op_assert_scs.true_end_extra);
        (*mb).moptions = (*F).fields.op_assert_scs.saved_moptions;
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    length = 0;
    state = S_SCS_MATCH_LOOP;
    continue 'sm;
}

/* Top of the for(;;) at C 6102 (assertion condition of OP_COND) */
S_COND_ASSERT_LOOP => {
    group_frame_type = GF_CONDASSERT;
    /* RMATCH(Lstart_branch + PRIV(OP_lengths)[*Lstart_branch], RM5) */
    start_ecode = (*F).fields.op_cond.start_branch.add(
        crate::tables::_pcre2_OP_lengths_8[*(*F).fields.op_cond.start_branch as usize] as usize,
    );
    Freturn_id!() = RM5 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM5 => {
    match rrc {
        /* Save captures */
        MATCH_ACCEPT => {
            copy_nonoverlapping(
                (assert_accept_frame as *const u8).add(OVECTOR_OFFSET_IN_HEAPFRAME),
                Fovector!() as *mut u8,
                (*assert_accept_frame).offset_top * core::mem::size_of::<PCRE2_SIZE>(),
            );
            Foffset_top!() = (*assert_accept_frame).offset_top;

            /* Fall through into MATCH_MATCH */
            /* In the case of a match, the captures have already been put into
            the current frame. */
            condition = (*F).byte1 as BOOL; /* TRUE for positive assertion */
        }

        MATCH_MATCH => {
            condition = (*F).byte1 as BOOL; /* TRUE for positive assertion */
        }

        /* PCRE doesn't allow the effect of (*THEN) to escape beyond an
        assertion; it is therefore always treated as NOMATCH. */
        MATCH_NOMATCH | MATCH_THEN => {
            (*F).fields.op_cond.start_branch = (*F)
                .fields
                .op_cond
                .start_branch
                .add(GET!((*F).fields.op_cond.start_branch, 1) as usize);
            if *(*F).fields.op_cond.start_branch as u32 == OP_ALT {
                /* Try next branch */
                state = S_COND_ASSERT_LOOP;
                continue 'sm;
            }
            condition = ((*F).byte1 == 0) as BOOL; /* TRUE for negative assertion */
        }

        /* These force no match without checking other branches. */
        MATCH_COMMIT | MATCH_SKIP | MATCH_PRUNE => {
            condition = ((*F).byte1 == 0) as BOOL;
        }

        _ => {
            /* RRETURN(rrc) */
            state = S_RETURN_SWITCH;
            continue 'sm;
        }
    }

    /* break out of the branch loop */

    /* If the condition is true, find the end of the assertion so that
    advancing past it gets us to the start of the first branch. */

    if condition != 0 {
        loop {
            Fecode!() = Fecode!().add(GET!(Fecode!(), 1) as usize);
            if *Fecode!() as u32 != OP_ALT {
                break;
            }
        }
    }
    /* End of assertion condition */
    state = S_COND_CHOOSE;
    continue 'sm;
}

/* C 6157: choose branch according to the condition. */
S_COND_CHOOSE => {
    Fecode!() = Fecode!().add(if condition != 0 {
        crate::tables::_pcre2_OP_lengths_8[*Fecode!() as usize] as usize
    } else {
        (*F).fields.op_cond.length
    });

    /* If the opcode is OP_SCOND it means we are at a repeated conditional
    group that might match an empty string. We must therefore descend a level
    so that the start is remembered for checking. For OP_COND we can just
    continue at this level. */

    if Fop!() as u32 == OP_SCOND {
        group_frame_type = GF_NOCAPTURE;
        /* RMATCH(Fecode, RM35) */
        start_ecode = Fecode!();
        Freturn_id!() = RM35 as u8;
        state = S_MATCH_RECURSE;
        continue 'sm;
    }
    state = S_MAINLOOP;
    continue 'sm;
}

RM35 => {
    /* RRETURN(rrc) */
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* Top of the for(;;) at C 6272 (OP_VREVERSE) */
S_VREVERSE_LOOP => {
    /* RMATCH(Fecode + 1 + 2 * IMM2_SIZE, RM37) */
    start_ecode = Fecode!().add(1 + 2 * IMM2_SIZE);
    Freturn_id!() = RM37 as u8;
    state = S_MATCH_RECURSE;
    continue 'sm;
}

RM37 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    /* if (Lmax-- <= Lmin) RRETURN(MATCH_NOMATCH); */
    let old_max: u32 = (*F).fields.op_vreverse.max;
    (*F).fields.op_vreverse.max = old_max.wrapping_sub(1);
    if old_max <= (*F).fields.op_vreverse.min {
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Feptr!() = Feptr!().add(1);
    if utf != 0 {
        FORWARDCHARTEST!(Feptr!(), (*mb).end_subject);
    }
    state = S_VREVERSE_LOOP;
    continue 'sm;
}

/* C 6469: resume after the RMATCH in the OP_ASSERT_SCS case of the OP_KET
starting-opcode switch. */
RM39 => {
    (*mb).end_subject = (*F).fields.op_assert_scs.saved_end_subject;
    (*mb).true_end_subject = (*mb).end_subject;
    /* RRETURN(rrc) */
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* C 6548: resume after the OP_KETRMIN RMATCH. */
RM6 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().sub(GET!(Fecode!(), 1) as usize);
    /* End of ket processing */
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 6556: resume after the OP_KETRMAX RMATCH. */
RM7 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    Fecode!() = Fecode!().add(1 + LINK_SIZE);
    state = S_MAINLOOP;
    continue 'sm;
}

/* ASSERT_NL_OR_EOS: C 6604 */
L_ASSERT_NL_OR_EOS => {
    if Feptr!() < (*mb).true_end_subject
        && (IS_NEWLINE!(Feptr!()) == 0
            || Feptr!() != (*mb).true_end_subject.sub((*mb).nllen as usize))
    {
        if (*mb).partial != 0
            && Feptr!().add(1) >= (*mb).end_subject
            && (*mb).nltype == NLTYPE_FIXED
            && (*mb).nllen == 2
            && *Feptr!() == (*mb).nl[0]
        {
            (*mb).hitend = TRUE;
            if (*mb).partial > 1 {
                return PCRE2_ERROR_PARTIAL;
            }
        }
        /* RRETURN(MATCH_NOMATCH) */
        rrc = MATCH_NOMATCH;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }

    /* Either at end of string or \n before end. */

    if (*mb).partial != 0 {
        (*mb).hitend = TRUE;
        if (*mb).partial > 1 {
            return PCRE2_ERROR_PARTIAL;
        }
    }
    Fecode!() = Fecode!().add(1);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 6788: resume after the OP_MARK RMATCH. */
RM12 => {
    /* A return of MATCH_SKIP_ARG means that matching failed at SKIP with an
    argument, and we must check whether that argument matches this MARK's
    argument. It is passed back in mb->verb_skip_ptr. If it does match, we
    return MATCH_SKIP with mb->verb_skip_ptr now pointing to the subject
    position that corresponds to this mark. Otherwise, pass back the return
    code unaltered. */

    if rrc == MATCH_SKIP_ARG
        && crate::string_utils::_pcre2_strcmp_8(Fecode!().add(2), (*mb).verb_skip_ptr) == 0
    {
        (*mb).verb_skip_ptr = Feptr!(); /* Pass back current position */
        /* RRETURN(MATCH_SKIP) */
        rrc = MATCH_SKIP;
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    /* RRETURN(rrc) */
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* C 6805: resume after the OP_COMMIT RMATCH. */
RM13 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    (*mb).verb_current_recurse = Fcurrent_recurse!();
    /* RRETURN(MATCH_COMMIT) */
    rrc = MATCH_COMMIT;
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* C 6812: resume after the OP_COMMIT_ARG RMATCH. */
RM36 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    (*mb).verb_current_recurse = Fcurrent_recurse!();
    /* RRETURN(MATCH_COMMIT) */
    rrc = MATCH_COMMIT;
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* C 6818: resume after the OP_PRUNE RMATCH. */
RM14 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    (*mb).verb_current_recurse = Fcurrent_recurse!();
    /* RRETURN(MATCH_PRUNE) */
    rrc = MATCH_PRUNE;
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* C 6825: resume after the OP_PRUNE_ARG RMATCH. */
RM15 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    (*mb).verb_current_recurse = Fcurrent_recurse!();
    /* RRETURN(MATCH_PRUNE) */
    rrc = MATCH_PRUNE;
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* C 6831: resume after the OP_SKIP RMATCH. */
RM16 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    (*mb).verb_skip_ptr = Feptr!(); /* Pass back current position */
    (*mb).verb_current_recurse = Fcurrent_recurse!();
    /* RRETURN(MATCH_SKIP) */
    rrc = MATCH_SKIP;
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* C 6852: resume after the OP_SKIP_ARG RMATCH. */
RM17 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }

    /* Pass back the current skip name and return the special MATCH_SKIP_ARG
    return code. This will either be caught by a matching MARK, or get to the
    top, where it causes a rematch with mb->ignore_skip_arg set to the value of
    mb->skip_arg_count. */

    (*mb).verb_skip_ptr = Fecode!().add(2);
    (*mb).verb_current_recurse = Fcurrent_recurse!();
    /* RRETURN(MATCH_SKIP_ARG) */
    rrc = MATCH_SKIP_ARG;
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* C 6868: resume after the OP_THEN RMATCH. */
RM18 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    (*mb).verb_ecode_ptr = Fecode!();
    (*mb).verb_current_recurse = Fcurrent_recurse!();
    /* RRETURN(MATCH_THEN) */
    rrc = MATCH_THEN;
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* C 6876: resume after the OP_THEN_ARG RMATCH. */
RM19 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    (*mb).verb_ecode_ptr = Fecode!();
    (*mb).verb_current_recurse = Fcurrent_recurse!();
    /* RRETURN(MATCH_THEN) */
    rrc = MATCH_THEN;
    state = S_RETURN_SWITCH;
    continue 'sm;
}

/* C 5495: resume after the OP_BRAZERO RMATCH. */
RM9 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    let mut next_ecode: PCRE2_SPTR = Fecode!();
    loop {
        next_ecode = next_ecode.add(GET!(next_ecode, 1) as usize);
        if *next_ecode as u32 != OP_ALT {
            break;
        }
    }
    Fecode!() = next_ecode.add(1 + LINK_SIZE);
    state = S_MAINLOOP;
    continue 'sm;
}

/* C 5510: resume after the OP_BRAMINZERO RMATCH. */
RM10 => {
    if rrc != MATCH_NOMATCH {
        /* RRETURN(rrc) */
        state = S_RETURN_SWITCH;
        continue 'sm;
    }
    state = S_MAINLOOP;
    continue 'sm;
}

            /* Any other state value is an internal error. */
            _ => {
                return PCRE2_ERROR_INTERNAL;
            }
        }
    }
}
