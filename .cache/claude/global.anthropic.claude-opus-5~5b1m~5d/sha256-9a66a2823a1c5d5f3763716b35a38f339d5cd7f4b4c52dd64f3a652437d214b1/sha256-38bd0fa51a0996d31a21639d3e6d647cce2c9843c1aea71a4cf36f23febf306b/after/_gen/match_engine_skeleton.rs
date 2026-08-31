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

// <<<EXTRA_STATE_CONSTS>>>

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

    // <<<EXTRA_LOCALS>>>

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
                    // <<<ARMS>>>

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

            // <<<STATES>>>

            /* Any other state value is an internal error. */
            _ => {
                return PCRE2_ERROR_INTERNAL;
            }
        }
    }
}
