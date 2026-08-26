// Translated from the match() function of pcre2_match.c (C lines 684-6950).
//
// The C function uses `goto` heavily (RMATCH/RRETURN plus a number of labels
// inside the big opcode switch). It is modelled here as a single flat dispatch
// loop `'sw` driven by the `lbl` variable. See MATCHER_CORE.md for the exact
// translation contract used by the opcode-arm chunk files in src/matcher_arms/.

use crate::internal::*;
use crate::matcher::*;
use crate::pcre2_pub::*;
use crate::tables::*;
use crate::ucd_data::*;
use crate::ucp::*;
use core::ffi::{c_char, c_int, c_uint, c_void};

/* ---- dispatch labels ---- */

pub(crate) const LBL_SWITCH: u32 = 0;
pub(crate) const LBL_TOP_OF_LOOP: u32 = 1;
pub(crate) const LBL_MATCH_RECURSE: u32 = 2;
pub(crate) const LBL_NEW_FRAME: u32 = 3;
pub(crate) const LBL_RETURN_SWITCH: u32 = 4;

pub(crate) const LBL_REPEATCHAR: u32 = 10;
pub(crate) const LBL_REPEATNOTCHAR: u32 = 11;
pub(crate) const LBL_REPEATTYPE: u32 = 12;
pub(crate) const LBL_REF_REPEAT: u32 = 13;
pub(crate) const LBL_POSSESSIVE_NON_CAPTURE: u32 = 14;
pub(crate) const LBL_POSSESSIVE_CAPTURE: u32 = 15;
pub(crate) const LBL_POSSESSIVE_GROUP: u32 = 16;
pub(crate) const LBL_GROUPLOOP: u32 = 17;
pub(crate) const LBL_ASSERT_NOT_FAILED: u32 = 18;
pub(crate) const LBL_ASSERT_NL_OR_EOS: u32 = 19;
pub(crate) const LBL_ENDLOOP99: u32 = 20;
pub(crate) const LBL_GOT_MAX: u32 = 21;
pub(crate) const LBL_ENDLOOP00: u32 = 22;
pub(crate) const LBL_ENDLOOP01: u32 = 23;
pub(crate) const LBL_ENDLOOP02: u32 = 24;
pub(crate) const LBL_ENDLOOP03: u32 = 25;
pub(crate) const LBL_SCS_OFFSET_FOUND: u32 = 26;
pub(crate) const LBL_REPEATTYPE_2: u32 = 27;
pub(crate) const LBL_REPEATTYPE_3: u32 = 28;

/* LBL_RM(n) == LBL_RM_BASE + n */
pub(crate) const LBL_RM_BASE: u32 = 1000;

/* ---- RMATCH return ids (enum in the C) ---- */

pub(crate) const RM1: u8 = 1;
pub(crate) const RM2: u8 = 2;
pub(crate) const RM3: u8 = 3;
pub(crate) const RM4: u8 = 4;
pub(crate) const RM5: u8 = 5;
pub(crate) const RM6: u8 = 6;
pub(crate) const RM7: u8 = 7;
pub(crate) const RM8: u8 = 8;
pub(crate) const RM9: u8 = 9;
pub(crate) const RM10: u8 = 10;
pub(crate) const RM11: u8 = 11;
pub(crate) const RM12: u8 = 12;
pub(crate) const RM13: u8 = 13;
pub(crate) const RM14: u8 = 14;
pub(crate) const RM15: u8 = 15;
pub(crate) const RM16: u8 = 16;
pub(crate) const RM17: u8 = 17;
pub(crate) const RM18: u8 = 18;
pub(crate) const RM19: u8 = 19;
pub(crate) const RM20: u8 = 20;
pub(crate) const RM21: u8 = 21;
pub(crate) const RM22: u8 = 22;
pub(crate) const RM23: u8 = 23;
pub(crate) const RM24: u8 = 24;
pub(crate) const RM25: u8 = 25;
pub(crate) const RM26: u8 = 26;
pub(crate) const RM27: u8 = 27;
pub(crate) const RM28: u8 = 28;
pub(crate) const RM29: u8 = 29;
pub(crate) const RM30: u8 = 30;
pub(crate) const RM31: u8 = 31;
pub(crate) const RM32: u8 = 32;
pub(crate) const RM33: u8 = 33;
pub(crate) const RM34: u8 = 34;
pub(crate) const RM35: u8 = 35;
pub(crate) const RM36: u8 = 36;
pub(crate) const RM37: u8 = 37;
pub(crate) const RM38: u8 = 38;
pub(crate) const RM39: u8 = 39;
pub(crate) const RM100: u8 = 100;
pub(crate) const RM101: u8 = 101;
pub(crate) const RM102: u8 = 102;
pub(crate) const RM103: u8 = 103;
pub(crate) const RM200: u8 = 200;
pub(crate) const RM201: u8 = 201;
pub(crate) const RM202: u8 = 202;
pub(crate) const RM203: u8 = 203;
pub(crate) const RM204: u8 = 204;
pub(crate) const RM205: u8 = 205;
pub(crate) const RM206: u8 = 206;
pub(crate) const RM207: u8 = 207;
pub(crate) const RM208: u8 = 208;
pub(crate) const RM209: u8 = 209;
pub(crate) const RM210: u8 = 210;
pub(crate) const RM211: u8 = 211;
pub(crate) const RM212: u8 = 212;
pub(crate) const RM213: u8 = 213;
pub(crate) const RM214: u8 = 214;
pub(crate) const RM215: u8 = 215;
pub(crate) const RM216: u8 = 216;
pub(crate) const RM217: u8 = 217;
pub(crate) const RM218: u8 = 218;
pub(crate) const RM219: u8 = 219;
pub(crate) const RM220: u8 = 220;
pub(crate) const RM221: u8 = 221;
pub(crate) const RM222: u8 = 222;
pub(crate) const RM223: u8 = 223;
pub(crate) const RM224: u8 = 224;

/* Helper: pointer to a frame's ovector (the declared array is a stand-in for a
variable-length trailing member, so never form a Rust reference to it). */
#[inline(always)]
pub(crate) unsafe fn ovec(f: *mut heapframe) -> *mut PCRE2_SIZE {
    core::ptr::addr_of_mut!((*f).ovector) as *mut PCRE2_SIZE
}

#[inline(always)]
pub(crate) unsafe fn frame_at(base: *mut heapframe, byte_offset: PCRE2_SIZE) -> *mut heapframe {
    (base as *mut u8).add(byte_offset) as *mut heapframe
}

#[inline(always)]
pub(crate) unsafe fn frame_add(f: *mut heapframe, bytes: PCRE2_SIZE) -> *mut heapframe {
    (f as *mut u8).add(bytes) as *mut heapframe
}

#[inline(always)]
pub(crate) unsafe fn frame_sub(f: *mut heapframe, bytes: PCRE2_SIZE) -> *mut heapframe {
    (f as *mut u8).sub(bytes) as *mut heapframe
}

/*************************************************
*         Match from current position            *
*************************************************/

pub(crate) unsafe fn match_(
    start_eptr: PCRE2_SPTR,
    start_ecode_arg: PCRE2_SPTR,
    top_bracket: u16,
    frame_size: PCRE2_SIZE,
    match_data: *mut pcre2_real_match_data,
    mb: *mut match_block,
) -> c_int {
    /* Frame-handling variables */

    let mut F: *mut heapframe;
    let mut N: *mut heapframe = core::ptr::null_mut();
    let mut P: *mut heapframe = core::ptr::null_mut();

    let mut frames_top: *mut heapframe;
    let mut assert_accept_frame: *mut heapframe = core::ptr::null_mut();
    let frame_copy_size: PCRE2_SIZE;

    /* Local variables that do not need to be preserved over calls to RMATCH(). */

    let mut start_ecode: PCRE2_SPTR = start_ecode_arg;
    let mut branch_end: PCRE2_SPTR = core::ptr::null();
    let mut branch_start: PCRE2_SPTR = core::ptr::null();
    let mut bracode: PCRE2_SPTR = core::ptr::null();
    let mut offset: PCRE2_SIZE = 0;
    let mut length: PCRE2_SIZE = 0;

    let mut rrc: c_int = 0;
    let mut proptype: c_int = 0;

    let mut i: u32 = 0;
    let mut fc: u32 = 0;
    let mut number: u32 = 0;
    let mut reptype: u32 = 0;
    let mut group_frame_type: u32;

    let mut condition: BOOL = FALSE;
    let mut cur_is_word: BOOL = FALSE;
    let mut prev_is_word: BOOL = FALSE;

    /* UTF and UCP flags */

    let utf: BOOL = (((*mb).poptions & PCRE2_UTF) != 0) as BOOL;
    let ucp: BOOL = (((*mb).poptions & PCRE2_UCP) != 0) as BOOL;

    /* This is the length of the last part of a backtracking frame that must be
    copied when a new frame is created. */

    frame_copy_size = frame_size - core::mem::offset_of!(heapframe, eptr);

    /* Set up the first frame and the end of the frames vector. */

    F = (*match_data).heapframes;
    frames_top = frame_at(F, (*match_data).heapframes_size);

    (*F).rdepth = 0; /* "Recursion" depth */
    (*F).capture_last = 0; /* Number of most recent capture */
    (*F).current_recurse = RECURSE_UNSET; /* Not pattern recursing. */
    (*F).eptr = start_eptr;
    (*F).start_match = start_eptr; /* Current data pointer and start match */
    (*F).mark = core::ptr::null(); /* Most recent mark */
    (*F).offset_top = 0; /* End of captures within the frame */
    (*F).last_group_offset = PCRE2_UNSET; /* Saved frame of most recent group */
    group_frame_type = 0; /* Not a start of group frame */

    let mut lbl: u32 = LBL_NEW_FRAME; /* Start processing with this frame */

    'sw: loop {
        if lbl == LBL_MATCH_RECURSE {
            /* Set up a new backtracking frame. If the vector is full, get a new one,
            doubling the size, but constrained by the heap limit (which is in KiB). */

            N = frame_add(F, frame_size);
            if frame_add(N, frame_size) >= frames_top {
                let new_: *mut heapframe;
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
                    if (*mb).heap_limit as PCRE2_SIZE <= old_size {
                        return PCRE2_ERROR_HEAPLIMIT;
                    } else {
                        let mut max_delta: PCRE2_SIZE =
                            1024 * ((*mb).heap_limit as PCRE2_SIZE - old_size);
                        let over_bytes: c_int =
                            ((*match_data).heapframes_size % 1024) as c_int;
                        if over_bytes != 0 {
                            max_delta -= (1024 - over_bytes) as PCRE2_SIZE;
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
                memcpy(
                    new_ as *mut c_void,
                    (*match_data).heapframes as *const c_void,
                    usedsize,
                );

                N = frame_at(new_, usedsize);
                F = frame_sub(N, frame_size);

                ((*match_data).memctl.free.unwrap())(
                    (*match_data).heapframes as *mut c_void,
                    (*match_data).memctl.memory_data,
                );
                (*match_data).heapframes = new_;
                (*match_data).heapframes_size = newsize;
                frames_top = frame_at(new_, newsize);
            }

            /* Copy those fields that must be copied into the new frame, increase the
            "recursion" depth (i.e. the new frame's index) and then make the new frame
            current. */

            memcpy(
                (N as *mut u8).add(core::mem::offset_of!(heapframe, eptr)) as *mut c_void,
                (F as *mut u8).add(core::mem::offset_of!(heapframe, eptr)) as *const c_void,
                frame_copy_size,
            );

            (*N).rdepth = (*F).rdepth + 1;
            F = N;

            lbl = LBL_NEW_FRAME;
        }

        if lbl == LBL_NEW_FRAME {
            (*F).group_frame_type = group_frame_type;
            (*F).ecode = start_ecode; /* Starting code pointer */
            (*F).back_frame = frame_size; /* Default is go back one frame */

            /* If this is a special type of group frame, remember its offset for quick
            access at the end of the group. If this is a recursion, set a new current
            recursion value. */

            if group_frame_type != 0 {
                (*F).last_group_offset =
                    (F as *mut u8).offset_from((*match_data).heapframes as *mut u8) as PCRE2_SIZE;
                if GF_IDMASK(group_frame_type) == GF_RECURSE {
                    (*F).current_recurse = GF_DATAMASK(group_frame_type);
                }
                group_frame_type = 0;
            }

            /* ================================================================= */
            /* This is the main processing loop. First check that we haven't recorded
            too many backtracks (search tree is too large), or that we haven't exceeded
            the recursive depth limit (used too many backtracking frames). If not,
            process the opcodes. */

            let mcc = (*mb).match_call_count;
            (*mb).match_call_count = mcc.wrapping_add(1);
            if mcc >= (*mb).match_limit {
                return PCRE2_ERROR_MATCHLIMIT;
            }
            if (*F).rdepth >= (*mb).match_limit_depth {
                return PCRE2_ERROR_DEPTHLIMIT;
            }

            lbl = LBL_TOP_OF_LOOP;
        }

        if lbl == LBL_TOP_OF_LOOP {
            (*F).op = *(*F).ecode;
            lbl = LBL_SWITCH;
        }

        if lbl == LBL_RETURN_SWITCH {
            if (*F).eptr > (*mb).last_used_ptr {
                (*mb).last_used_ptr = (*F).eptr;
            }
            if (*F).rdepth == 0 {
                return rrc; /* Exit from the top level */
            }
            F = frame_sub(F, (*F).back_frame); /* Backtrack */
            (*(*mb).cb).callout_flags |= PCRE2_CALLOUT_BACKTRACK; /* Note for callouts */

            let rid = (*F).return_id as u32;
            match rid {
                1..=39 | 100..=103 | 200..=224 => {
                    lbl = LBL_RM_BASE + rid;
                }
                _ => {
                    return PCRE2_ERROR_INTERNAL;
                }
            }
        }

        /* --------------------------------------------------------------------- */
        /* The opcode switch and the intra-switch labels, split across chunk files
        that are textually included here. Each chunk is a block expression that
        either handles the current `lbl` (and then `continue 'sw`) or falls through
        to the next chunk. */

        include!("matcher_arms/a.rs");
        include!("matcher_arms/b.rs");
        include!("matcher_arms/c.rs");
        include!("matcher_arms/d.rs");
        include!("matcher_arms/e.rs");
        include!("matcher_arms/e2.rs");
        include!("matcher_arms/e3.rs");
        include!("matcher_arms/f.rs");
        include!("matcher_arms/g.rs");
        include!("matcher_arms/h.rs");
        include!("matcher_arms/i.rs");

        /* Unrecognized opcode: the C code's `default:` in the opcode switch. */
        return PCRE2_ERROR_INTERNAL;
    }
}
