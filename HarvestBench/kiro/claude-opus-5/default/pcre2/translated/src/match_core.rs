//! Translation of PART 2 of `pcre2_match.c` (C lines 684-6971): the single
//! `static int match(...)` function — the core PCRE2 backtracking interpreter.
//!
//! The C function is a computed-goto state machine built from the macros
//! `RMATCH(ra, rb)`, `RRETURN(ra)`, and a set of labels (`NEW_FRAME`,
//! `MATCH_RECURSE`, `RETURN_SWITCH`, plus per-opcode / shared labels). We model
//! it as one `loop` over a [`Lbl`] state enum, with every label (the main
//! opcode switch, every shared code block, and every backtracking return point
//! `L_RMn`) implemented as an arm of a single `match label` in one function
//! scope, so all locals stay directly in scope and `continue 'sm` / `return`
//! reproduce the C control flow exactly.
//!
//!   * `RMATCH(a, RMn)` splits its opcode case in two. The code *before* it
//!     does `start_ecode = a; *Freturn_id(F) = RMn; label = Lbl::MatchRecurse;
//!     continue 'sm;`. The code *after* it becomes `Lbl::L_RMn`.
//!   * A C `break;`/`continue;` at the end of a case does `label =
//!     Lbl::MainLoop; continue 'sm;`.
//!   * A C `goto LABEL;` does `label = Lbl::Label; continue 'sm;`.
//!   * `RRETURN(x)` does `rrc = x; label = Lbl::ReturnSwitch; continue 'sm;`.
//!
//! This is `static` in C, so it is a plain `pub(crate) unsafe fn` — NOT
//! `#[no_mangle]`, NOT `extern "C"`.

use crate::internal::*;
use crate::match_local::*;
use crate::match_util::*;
use core::ffi::c_int;
use core::mem::offset_of;
use core::ptr;

// `RECURSE_UNSET` — bigger than the max group number (C line 66).
const RECURSE_UNSET: u32 = 0xffffffffu32;

// EBCDIC not configured.
const CHAR_NL: u32 = 0x0a;
const CHAR_CR: u32 = 0x0d;

// Whitespace helpers, mirroring HSPACE_CASES / VSPACE_CASES (pcre2_internal.h,
// non-EBCDIC).
#[inline(always)]
fn is_hspace(fc: u32) -> bool {
    matches!(
        fc,
        0x09 | 0x20 | 0xa0 | 0x1680 | 0x180e | 0x2000 | 0x2001 | 0x2002 | 0x2003
            | 0x2004 | 0x2005 | 0x2006 | 0x2007 | 0x2008 | 0x2009 | 0x200a | 0x202f
            | 0x205f | 0x3000
    )
}
#[inline(always)]
fn is_vspace(fc: u32) -> bool {
    matches!(fc, 0x0a | 0x0b | 0x0c | 0x0d | 0x85 | 0x2028 | 0x2029)
}

// ---------------------------------------------------------------------------
// REPEATTYPE per-character predicates.
//
// `rt_prop_reject` mirrors the C body `if (predicate == notmatch) <reject>`,
// returning `true` when the C code would reject the character (RRETURN /
// break). `lctype` distinguishes OP_PROP (notmatch=false) from OP_NOTPROP
// (notmatch=true).
// ---------------------------------------------------------------------------

#[inline]
unsafe fn rt_prop_reject(fc: u32, proptype: c_int, propvalue: u32, lctype: u32) -> bool {
    unsafe {
        let notmatch = lctype == OP_NOTPROP;
        let prop = GET_UCD(fc);
        let matched = match proptype as i64 {
            PT_LAMP => {
                let ct = prop.chartype as u32;
                ct == ucp_Lu as u32 || ct == ucp_Ll as u32 || ct == ucp_Lt as u32
            }
            PT_GC => UCD_CATEGORY(fc) == propvalue,
            PT_PC => UCD_CHARTYPE(fc) == propvalue,
            PT_SC => UCD_SCRIPT(fc) == propvalue,
            PT_SCX => {
                prop.script as u32 == propvalue
                    || MAPBIT(
                        crate::tables::_pcre2_ucd_script_sets_8
                            .as_ptr()
                            .add(UCD_SCRIPTX_PROP(prop) as usize),
                        propvalue,
                    ) != 0
            }
            PT_ALNUM => {
                let cat = UCD_CATEGORY(fc);
                cat == ucp_L as u32 || cat == ucp_N as u32
            }
            PT_SPACE | PT_PXSPACE => {
                if is_hspace(fc) || is_vspace(fc) {
                    true
                } else {
                    UCD_CATEGORY(fc) == ucp_Z as u32
                }
            }
            PT_WORD => {
                let ct = UCD_CHARTYPE(fc);
                let cat = crate::tables::_pcre2_ucp_gentype[ct as usize];
                cat == ucp_L as u32 || cat == ucp_N as u32
                    || ct == ucp_Mn as u32 || ct == ucp_Pc as u32
            }
            PT_CLIST => {
                let mut cp: *const u32 = crate::tables::_pcre2_ucd_caseless_sets_8
                    .as_ptr()
                    .add(propvalue as usize);
                let mut found = false;
                loop {
                    if fc < *cp {
                        break;
                    }
                    let cur = *cp;
                    cp = cp.add(1);
                    if fc == cur {
                        found = true;
                        break;
                    }
                }
                found
            }
            PT_UCNC => {
                fc == 0x24 || fc == 0x40 || fc == 0x60
                    || (fc >= 0xa0 && fc <= 0xd7ff) || fc >= 0xe000
            }
            PT_BIDICL => UCD_BIDICLASS(fc) == propvalue,
            PT_BOOL => {
                MAPBIT(
                    crate::tables::_pcre2_ucd_boolprop_sets_8
                        .as_ptr()
                        .add(UCD_BPROPS_PROP(prop) as usize),
                    propvalue,
                ) != 0
            }
            _ => false,
        };
        matched == notmatch
    }
}

// For the non-property, non-ANY character types: returns `true` when the C
// code would reject `fc` (RRETURN(MATCH_NOMATCH) / break). Applies to
// OP_NOT_DIGIT/OP_DIGIT/OP_NOT_WHITESPACE/OP_WHITESPACE/
// OP_NOT_WORDCHAR/OP_WORDCHAR/OP_NOT_HSPACE/OP_HSPACE/OP_NOT_VSPACE/OP_VSPACE.
#[inline]
unsafe fn rt_ctype_reject(fc: u32, lctype: u32, mb: *mut match_block) -> bool {
    unsafe {
        match lctype {
            OP_NOT_HSPACE => is_hspace(fc),
            OP_HSPACE => !is_hspace(fc),
            OP_NOT_VSPACE => is_vspace(fc),
            OP_VSPACE => !is_vspace(fc),
            OP_NOT_DIGIT => {
                CHMAX_255(fc) && (*(*mb).ctypes.add(fc as usize) as u32 & ctype_digit as u32) != 0
            }
            OP_DIGIT => {
                !CHMAX_255(fc) || (*(*mb).ctypes.add(fc as usize) as u32 & ctype_digit as u32) == 0
            }
            OP_NOT_WHITESPACE => {
                CHMAX_255(fc) && (*(*mb).ctypes.add(fc as usize) as u32 & ctype_space as u32) != 0
            }
            OP_WHITESPACE => {
                !CHMAX_255(fc) || (*(*mb).ctypes.add(fc as usize) as u32 & ctype_space as u32) == 0
            }
            OP_NOT_WORDCHAR => {
                CHMAX_255(fc) && (*(*mb).ctypes.add(fc as usize) as u32 & ctype_word as u32) != 0
            }
            OP_WORDCHAR => {
                !CHMAX_255(fc) || (*(*mb).ctypes.add(fc as usize) as u32 & ctype_word as u32) == 0
            }
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// State-machine labels
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lbl {
    NewFrame,
    MatchRecurse,
    MainLoop,
    ReturnSwitch,

    // Shared code-block labels reached by `goto` from more than one place.
    RepeatChar,           // REPEATCHAR   (C 1392)
    RepeatNotChar,        // REPEATNOTCHAR(C 1733)
    RepeatType,           // REPEATTYPE   (C 2290)
    RefRepeat,            // REF_REPEAT   (C 4595)
    PossessiveNonCapture, // POSSESSIVE_NON_CAPTURE (C 4862)
    PossessiveCapture,    // POSSESSIVE_CAPTURE (C 4870)
    PossessiveGroup,      // POSSESSIVE_GROUP   (C 4874)
    GroupLoop,            // GROUPLOOP    (C 4993)
    AssertNotFailed,      // ASSERT_NOT_FAILED (C 5170)
    ScsOffsetFound,       // SCS_OFFSET_FOUND  (C 5224)
    AssertNlOrEos,        // ASSERT_NL_OR_EOS  (C 5921)
    GotMax,               // GOT_MAX      (C 3884)

    // Backtracking return points (one per RMATCH call site).
    L_RM1, L_RM2, L_RM3, L_RM4, L_RM5, L_RM6, L_RM7, L_RM8, L_RM9, L_RM10,
    L_RM11, L_RM12, L_RM13, L_RM14, L_RM15, L_RM16, L_RM17, L_RM18, L_RM19, L_RM20,
    L_RM21, L_RM22, L_RM23, L_RM24, L_RM25, L_RM26, L_RM27, L_RM28, L_RM29, L_RM30,
    L_RM31, L_RM32, L_RM33, L_RM34, L_RM35, L_RM36, L_RM37, L_RM38, L_RM39,

    L_RM100, L_RM101, L_RM102, L_RM103,

    L_RM200, L_RM201, L_RM202, L_RM203, L_RM204, L_RM205, L_RM206, L_RM207,
    L_RM208, L_RM209, L_RM210, L_RM211, L_RM212, L_RM213, L_RM214, L_RM215,
    L_RM216, L_RM217, L_RM218, L_RM219, L_RM220, L_RM221, L_RM222, L_RM223,
    L_RM224,
}

// ---------------------------------------------------------------------------
// match()  (C line 684)
// ---------------------------------------------------------------------------

pub(crate) unsafe fn match_(
    start_eptr: PCRE2_SPTR,
    start_ecode: PCRE2_SPTR,
    top_bracket: u16,
    frame_size: PCRE2_SIZE,
    match_data: *mut pcre2_real_match_data,
    mb: *mut match_block,
) -> c_int {
    unsafe {
        let mut start_ecode = start_ecode;

        // Frame-handling variables.
        let mut F: *mut heapframe = ptr::null_mut();
        let mut N: *mut heapframe = ptr::null_mut();
        let mut P: *mut heapframe = ptr::null_mut();
        let mut frames_top: *mut heapframe = ptr::null_mut();
        let mut assert_accept_frame: *mut heapframe = ptr::null_mut();
        let frame_copy_size: PCRE2_SIZE;

        // Local variables not preserved over RMATCH() calls.
        let mut branch_end: PCRE2_SPTR = ptr::null();
        let mut branch_start: PCRE2_SPTR = ptr::null();
        let mut bracode: PCRE2_SPTR = ptr::null();
        let mut offset: PCRE2_SIZE = 0;
        let mut length: PCRE2_SIZE = 0;

        let mut rrc: c_int = 0;

        let mut proptype: c_int = 0; // type of character property

        let mut i: u32 = 0;
        let mut fc: u32 = 0;
        let mut number: u32 = 0;
        let mut reptype: u32 = 0;
        let mut group_frame_type: u32 = 0;

        let mut condition: BOOL = 0;
        let mut cur_is_word: BOOL = 0;
        let mut prev_is_word: BOOL = 0;

        // These locals hold values that must be re-read after a backtrack; we
        // keep them as plain locals but they are also mirrored in the frame
        // where the C stores them across RMATCH.

        // UTF and UCP flags.
        let utf: bool = ((*mb).poptions & PCRE2_UTF as u32) != 0;
        let ucp: bool = ((*mb).poptions & PCRE2_UCP as u32) != 0;

        frame_copy_size = frame_size - offset_of!(heapframe, eptr);

        // ---- Handy macros mirroring the C control-flow macros. -------------

        // `IS_NEWLINE(p)` (pcre2_internal.h) — NLBLOCK == mb, PSEND == end_subject.
        macro_rules! IS_NEWLINE {
            ($p:expr) => {{
                let p: PCRE2_SPTR = $p;
                if (*mb).nltype != NLTYPE_FIXED as u32 {
                    p < (*mb).end_subject
                        && crate::newline::_pcre2_is_newline_8(
                            p, (*mb).nltype, (*mb).end_subject,
                            &raw mut (*mb).nllen, utf as BOOL,
                        ) != FALSE
                } else {
                    p <= (*mb).end_subject.sub((*mb).nllen as usize)
                        && *p as u32 == (*mb).nl[0] as u32
                        && ((*mb).nllen == 1 || *p.add(1) as u32 == (*mb).nl[1] as u32)
                }
            }};
        }

        // `WAS_NEWLINE(p)` — PSSTART == start_subject.
        macro_rules! WAS_NEWLINE {
            ($p:expr) => {{
                let p: PCRE2_SPTR = $p;
                if (*mb).nltype != NLTYPE_FIXED as u32 {
                    p > (*mb).start_subject
                        && crate::newline::_pcre2_was_newline_8(
                            p, (*mb).nltype, (*mb).start_subject,
                            &raw mut (*mb).nllen, utf as BOOL,
                        ) != FALSE
                } else {
                    p >= (*mb).start_subject.add((*mb).nllen as usize)
                        && *p.sub((*mb).nllen as usize) as u32 == (*mb).nl[0] as u32
                        && ((*mb).nllen == 1
                            || *p.sub((*mb).nllen as usize - 1) as u32 == (*mb).nl[1] as u32)
                }
            }};
        }

        // `ACROSSCHAR(condition, eptr, eptr = <next>)` for 8-bit UTF.
        // Advances `$eptr` while the given `$cond` holds and it points at a
        // UTF-8 continuation byte.
        macro_rules! ACROSSCHAR {
            ($cond:expr, $eptr:expr) => {{
                while ($cond) && (*($eptr) & 0xc0u8) == 0x80u8 {
                    $eptr = ($eptr).add(1);
                }
            }};
        }

        // `RRETURN(ra)` == `{ rrc = ra; goto RETURN_SWITCH; }`

        // `RMATCH(ra, RMn)` == set new start_ecode + return id, go to
        // MATCH_RECURSE. The code after the C `RMATCH(...)` lives in `L_RMn`.

        // `SCHECK_PARTIAL()` — on hard-partial, causes a `return`.
        macro_rules! SCHECK_PARTIAL {
            () => {{
                if let Some(r) = crate::match_util::SCHECK_PARTIAL(F, mb) {
                    return r;
                }
            }};
        }

        // `CHECK_PARTIAL()`.
        macro_rules! CHECK_PARTIAL {
            () => {{
                if let Some(r) = crate::match_util::CHECK_PARTIAL(F, mb) {
                    return r;
                }
            }};
        }

        // A C `break;` / `continue;` at the end of an opcode case: repeat the
        // main loop.

        // First frame + end of frames vector.
        F = (*match_data).heapframes;
        frames_top = ((*match_data).heapframes as *mut u8)
            .add((*match_data).heapframes_size) as *mut heapframe;

        *Frdepth(F) = 0;
        *Fcapture_last(F) = 0;
        *Fcurrent_recurse(F) = RECURSE_UNSET;
        *Fstart_match(F) = start_eptr;
        *Feptr(F) = start_eptr;
        *Fmark(F) = ptr::null();
        *Foffset_top(F) = 0;
        *Flast_group_offset(F) = PCRE2_UNSET;
        group_frame_type = 0;

        let mut label = Lbl::NewFrame; // goto NEW_FRAME

        'sm: loop {
            match label {
                // ===========================================================
                // MATCH_RECURSE (C line 70)
                // ===========================================================
                Lbl::MatchRecurse => {
                    N = (F as *mut u8).add(frame_size) as *mut heapframe;
                    if (N as *mut u8).add(frame_size) as *mut heapframe >= frames_top {
                        let new_: *mut heapframe;
                        let mut newsize: PCRE2_SIZE;
                        let usedsize: PCRE2_SIZE =
                            (N as *mut u8).offset_from((*match_data).heapframes as *mut u8)
                                as PCRE2_SIZE;

                        if (*match_data).heapframes_size >= PCRE2_SIZE_MAX / 2 {
                            if (*match_data).heapframes_size == PCRE2_SIZE_MAX - 1 {
                                return PCRE2_ERROR_NOMEMORY as c_int;
                            }
                            newsize = PCRE2_SIZE_MAX - 1;
                        } else {
                            newsize = (*match_data).heapframes_size * 2;
                        }

                        if newsize / 1024 >= (*mb).heap_limit as PCRE2_SIZE {
                            let old_size: PCRE2_SIZE = (*match_data).heapframes_size / 1024;
                            if (*mb).heap_limit as PCRE2_SIZE <= old_size {
                                return PCRE2_ERROR_HEAPLIMIT as c_int;
                            } else {
                                let mut max_delta: PCRE2_SIZE =
                                    1024 * ((*mb).heap_limit as PCRE2_SIZE - old_size);
                                let over_bytes = (*match_data).heapframes_size % 1024;
                                if over_bytes != 0 {
                                    max_delta -= 1024 - over_bytes;
                                }
                                newsize = (*match_data).heapframes_size + max_delta;
                            }
                        }

                        if newsize - usedsize < frame_size {
                            return PCRE2_ERROR_HEAPLIMIT as c_int;
                        }
                        new_ = ((*match_data).memctl.malloc.unwrap())(
                            newsize,
                            (*match_data).memctl.memory_data,
                        ) as *mut heapframe;
                        if new_.is_null() {
                            return PCRE2_ERROR_NOMEMORY as c_int;
                        }
                        ptr::copy_nonoverlapping(
                            (*match_data).heapframes as *const u8,
                            new_ as *mut u8,
                            usedsize,
                        );

                        N = (new_ as *mut u8).add(usedsize) as *mut heapframe;
                        F = (N as *mut u8).sub(frame_size) as *mut heapframe;

                        ((*match_data).memctl.free.unwrap())(
                            (*match_data).heapframes as *mut core::ffi::c_void,
                            (*match_data).memctl.memory_data,
                        );
                        (*match_data).heapframes = new_;
                        (*match_data).heapframes_size = newsize;
                        frames_top = (new_ as *mut u8).add(newsize) as *mut heapframe;
                    }

                    ptr::copy_nonoverlapping(
                        (F as *const u8).add(offset_of!(heapframe, eptr)),
                        (N as *mut u8).add(offset_of!(heapframe, eptr)),
                        frame_copy_size,
                    );
                    *Frdepth(N) = *Frdepth(F) + 1;
                    F = N;

                    label = Lbl::NewFrame;
                    continue 'sm;
                }

                // ===========================================================
                // NEW_FRAME (C line 166)
                // ===========================================================
                Lbl::NewFrame => {
                    *Fgroup_frame_type(F) = group_frame_type;
                    *Fecode(F) = start_ecode;
                    *Fback_frame(F) = frame_size;

                    if group_frame_type != 0 {
                        *Flast_group_offset(F) = (F as *const u8)
                            .offset_from((*match_data).heapframes as *const u8)
                            as PCRE2_SIZE;
                        if GF_IDMASK(group_frame_type) == GF_RECURSE {
                            *Fcurrent_recurse(F) = GF_DATAMASK(group_frame_type);
                        }
                        group_frame_type = 0;
                    }

                    if {
                        let c = (*mb).match_call_count;
                        (*mb).match_call_count = c + 1;
                        c >= (*mb).match_limit
                    } {
                        return PCRE2_ERROR_MATCHLIMIT as c_int;
                    }
                    if *Frdepth(F) >= (*mb).match_limit_depth {
                        return PCRE2_ERROR_DEPTHLIMIT as c_int;
                    }

                    label = Lbl::MainLoop;
                    continue 'sm;
                }

                // ===========================================================
                // Main processing loop: `switch (*Fecode)` (C line 786+)
                // ===========================================================
                Lbl::MainLoop => {
                    *Fop(F) = *(*Fecode(F));
                    let Fop_val = *Fop(F) as u32;

                    match Fop_val {
                        // ---- OP_CLOSE (C 796) ----
                        OP_CLOSE => {
                            if *Fcurrent_recurse(F) == RECURSE_UNSET {
                                number = GET2(*Fecode(F), 1);
                                offset = *Flast_group_offset(F);
                                loop {
                                    debug_assert!(offset != PCRE2_UNSET);
                                    if offset == PCRE2_UNSET {
                                        return PCRE2_ERROR_INTERNAL as c_int;
                                    }
                                    N = ((*match_data).heapframes as *mut u8).add(offset)
                                        as *mut heapframe;
                                    P = (N as *mut u8).sub(frame_size) as *mut heapframe;
                                    if *Fgroup_frame_type(N) == (GF_CAPTURE | number) {
                                        break;
                                    }
                                    offset = *Flast_group_offset(P);
                                }
                                offset = ((number << 1) - 2) as PCRE2_SIZE;
                                *Fcapture_last(F) = number;
                                let ov = Fovector(F);
                                *ov.add(offset) = (*Feptr(P))
                                    .offset_from((*mb).start_subject)
                                    as PCRE2_SIZE;
                                *ov.add(offset + 1) = (*Feptr(F))
                                    .offset_from((*mb).start_subject)
                                    as PCRE2_SIZE;
                                if offset >= *Foffset_top(F) {
                                    *Foffset_top(F) = offset + 2;
                                }
                            }
                            *Fecode(F) = (*Fecode(F))
                                .add(crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize]
                                    as usize);
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_ASSERT_ACCEPT (C 819) ----
                        OP_ASSERT_ACCEPT => {
                            if *Feptr(F) > (*mb).last_used_ptr {
                                (*mb).last_used_ptr = *Feptr(F);
                            }
                            assert_accept_frame = F;
                            { rrc = MATCH_ACCEPT; label = Lbl::ReturnSwitch; continue 'sm; }
                        }

                        // ---- OP_ACCEPT / OP_END (C 828 / 851) ----
                        OP_ACCEPT | OP_END => {
                            // OP_ACCEPT prefix: handle recursion, else fall
                            // through to the OP_END-common code.
                            if Fop_val == OP_ACCEPT
                                && *Fcurrent_recurse(F) != RECURSE_UNSET
                            {
                                offset = *Flast_group_offset(F);
                                loop {
                                    debug_assert!(offset != PCRE2_UNSET);
                                    if offset == PCRE2_UNSET {
                                        return PCRE2_ERROR_INTERNAL as c_int;
                                    }
                                    N = ((*match_data).heapframes as *mut u8).add(offset)
                                        as *mut heapframe;
                                    P = (N as *mut u8).sub(frame_size) as *mut heapframe;
                                    if GF_IDMASK(*Fgroup_frame_type(N)) == GF_RECURSE {
                                        break;
                                    }
                                    offset = *Flast_group_offset(P);
                                }
                                *Feptr(P) = *Feptr(F);
                                *Fmark(P) = *Fmark(F);
                                *Fstart_match(P) = *Fstart_match(F);
                                F = P;
                                *Fecode(F) = (*Fecode(F)).add(1 + LINK_SIZE_U);
                                { label = Lbl::MainLoop; continue 'sm; }
                            }

                            // Common OP_END code (also entered by OP_ACCEPT
                            // fall-through when not in a recursion).

                            if *Feptr(F) == *Fstart_match(F)
                                && (((*mb).moptions & PCRE2_NOTEMPTY as u32) != 0
                                    || (((*mb).moptions & PCRE2_NOTEMPTY_ATSTART as u32) != 0
                                        && *Fstart_match(F)
                                            == (*mb).start_subject.add((*mb).start_offset)))
                            {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }

                            if *Feptr(F) < (*mb).end_subject
                                && (((*mb).moptions | (*mb).poptions) & PCRE2_ENDANCHORED as u32)
                                    != 0
                            {
                                if Fop_val == OP_END {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                return MATCH_NOMATCH; // (*ACCEPT)
                            }

                            if *Fstart_match(F) < (*mb).start_subject.add((*mb).start_offset)
                                || *Fstart_match(F) > *Feptr(F)
                            {
                                debug_assert!((*mb).hasbsk != FALSE);
                                if (*mb).allowlookaroundbsk == FALSE {
                                    return PCRE2_ERROR_BAD_BACKSLASH_K as c_int;
                                }
                            }

                            (*mb).end_match_ptr = *Feptr(F);
                            (*mb).end_offset_top = *Foffset_top(F);
                            (*mb).mark = *Fmark(F);
                            if *Feptr(F) > (*mb).last_used_ptr {
                                (*mb).last_used_ptr = *Feptr(F);
                            }

                            let ovec = (*match_data).ovec();
                            *ovec.add(0) = (*Fstart_match(F))
                                .offset_from((*mb).start_subject) as PCRE2_SIZE;
                            *ovec.add(1) = (*Feptr(F))
                                .offset_from((*mb).start_subject) as PCRE2_SIZE;

                            i = 2 * if (top_bracket as u32 + 1) > (*match_data).oveccount as u32 {
                                (*match_data).oveccount as u32
                            } else {
                                top_bracket as u32 + 1
                            };
                            ptr::copy_nonoverlapping(
                                Fovector(F) as *const PCRE2_SIZE,
                                ovec.add(2),
                                (i as usize - 2),
                            );
                            loop {
                                i -= 1;
                                if !(i as PCRE2_SIZE >= *Foffset_top(F) + 2) {
                                    // C: `while (--i >= Foffset_top + 2)` — i is
                                    // unsigned so the loop stops when i wraps.
                                    break;
                                }
                                *ovec.add(i as usize) = PCRE2_UNSET;
                            }
                            return MATCH_MATCH; // NOT RRETURN
                        }

                        // ---- OP_ANY / OP_ALLANY (C 953 / 971) ----
                        OP_ANY | OP_ALLANY => {
                            if Fop_val == OP_ANY {
                                if IS_NEWLINE!(*Feptr(F)) {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                if (*mb).partial != 0
                                    && *Feptr(F) == (*mb).end_subject.sub(1)
                                    && (*mb).nltype == NLTYPE_FIXED as u32
                                    && (*mb).nllen == 2
                                    && *(*Feptr(F)) as u32 == (*mb).nl[0] as u32
                                {
                                    (*mb).hitend = TRUE;
                                    if (*mb).partial > 1 {
                                        return PCRE2_ERROR_PARTIAL as c_int;
                                    }
                                }
                            }
                            // OP_ALLANY common code.
                            if *Feptr(F) >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            *Feptr(F) = (*Feptr(F)).add(1);
                            if utf {
                                let mut ep = *Feptr(F);
                                ACROSSCHAR!(ep < (*mb).end_subject, ep);
                                *Feptr(F) = ep;
                            }
                            *Fecode(F) = (*Fecode(F)).add(1);
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_ANYBYTE (C 987) ----
                        OP_ANYBYTE => {
                            if *Feptr(F) >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            *Feptr(F) = (*Feptr(F)).add(1);
                            *Fecode(F) = (*Fecode(F)).add(1);
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_CHAR (C 1000) ----
                        OP_CHAR => {
                            if utf {
                                length = 1;
                                *Fecode(F) = (*Fecode(F)).add(1);
                                let mut len_u = length as u32;
                                fc = GETCHARLEN(*Fecode(F), &mut len_u);
                                length = len_u as PCRE2_SIZE;
                                if length > (*mb).end_subject.offset_from(*Feptr(F)) as PCRE2_SIZE {
                                    CHECK_PARTIAL!();
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                while length > 0 {
                                    let a = *(*Fecode(F));
                                    *Fecode(F) = (*Fecode(F)).add(1);
                                    let b = *(*Feptr(F));
                                    *Feptr(F) = (*Feptr(F)).add(1);
                                    if a != b {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                    length -= 1;
                                }
                            } else {
                                if ((*mb).end_subject.offset_from(*Feptr(F))) < 1 {
                                    SCHECK_PARTIAL!();
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                let ec1 = *(*Fecode(F)).add(1);
                                let sc = *(*Feptr(F));
                                *Feptr(F) = (*Feptr(F)).add(1);
                                if ec1 != sc {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                *Fecode(F) = (*Fecode(F)).add(2);
                            }
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_CHARI (C 1036) ----
                        OP_CHARI => {
                            if *Feptr(F) >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            if utf {
                                length = 1;
                                *Fecode(F) = (*Fecode(F)).add(1);
                                let mut len_u = length as u32;
                                fc = GETCHARLEN(*Fecode(F), &mut len_u);
                                length = len_u as PCRE2_SIZE;
                                if fc < 128 {
                                    let cc = *(*Feptr(F)) as u32;
                                    if *(*mb).lcc.add(fc as usize) as u32
                                        != TABLE_GET(cc, (*mb).lcc, cc)
                                    {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                    *Fecode(F) = (*Fecode(F)).add(1);
                                    *Feptr(F) = (*Feptr(F)).add(1);
                                } else {
                                    let mut ep = *Feptr(F);
                                    let dc = GETCHARINC(&mut ep);
                                    *Feptr(F) = ep;
                                    *Fecode(F) = (*Fecode(F)).add(length);
                                    if dc != fc && dc != UCD_OTHERCASE(fc) {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                }
                            } else if ucp {
                                let cc = *(*Feptr(F)) as u32;
                                fc = *(*Fecode(F)).add(1) as u32;
                                if fc < 128 {
                                    if *(*mb).lcc.add(fc as usize) as u32
                                        != TABLE_GET(cc, (*mb).lcc, cc)
                                    {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                } else if cc != fc && cc != UCD_OTHERCASE(fc) {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                *Feptr(F) = (*Feptr(F)).add(1);
                                *Fecode(F) = (*Fecode(F)).add(2);
                            } else {
                                let a = *(*Fecode(F)).add(1) as u32;
                                let b = *(*Feptr(F)) as u32;
                                if TABLE_GET(a, (*mb).lcc, a) != TABLE_GET(b, (*mb).lcc, b) {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                *Feptr(F) = (*Feptr(F)).add(1);
                                *Fecode(F) = (*Fecode(F)).add(2);
                            }
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_NOT / OP_NOTI (C 1078) ----
                        OP_NOT | OP_NOTI => {
                            if *Feptr(F) >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            if utf {
                                let mut ch: u32;
                                *Fecode(F) = (*Fecode(F)).add(1);
                                let mut ecp = *Fecode(F);
                                ch = GETCHARINC(&mut ecp);
                                *Fecode(F) = ecp;
                                let mut ep = *Feptr(F);
                                fc = GETCHARINC(&mut ep);
                                *Feptr(F) = ep;
                                if ch == fc {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                } else if Fop_val == OP_NOTI {
                                    if ch > 127 {
                                        ch = UCD_OTHERCASE(ch);
                                    } else {
                                        ch = *(*mb).fcc.add(ch as usize) as u32;
                                    }
                                    if ch == fc {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                }
                            } else if ucp {
                                let mut ch: u32;
                                fc = *(*Feptr(F)) as u32;
                                *Feptr(F) = (*Feptr(F)).add(1);
                                ch = *(*Fecode(F)).add(1) as u32;
                                *Fecode(F) = (*Fecode(F)).add(2);
                                if ch == fc {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                } else if Fop_val == OP_NOTI {
                                    if ch > 127 {
                                        ch = UCD_OTHERCASE(ch);
                                    } else {
                                        ch = *(*mb).fcc.add(ch as usize) as u32;
                                    }
                                    if ch == fc {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                }
                            } else {
                                let ch = *(*Fecode(F)).add(1) as u32;
                                fc = *(*Feptr(F)) as u32;
                                *Feptr(F) = (*Feptr(F)).add(1);
                                if ch == fc
                                    || (Fop_val == OP_NOTI
                                        && TABLE_GET(ch, (*mb).fcc, ch) == fc)
                                {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                *Fecode(F) = (*Fecode(F)).add(2);
                            }
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- Repeated single-character non-matches (C 1660) ----
                        // These set up Lmin/Lmax/reptype and jump to
                        // REPEATNOTCHAR, which reads the character itself.
                        OP_NOTEXACT | OP_NOTEXACTI => {
                            *Lcharnot_min(F) = GET2(*Fecode(F), 1);
                            *Lcharnot_max(F) = *Lcharnot_min(F);
                            *Fecode(F) = (*Fecode(F)).add(1 + IMM2_SIZE_U);
                            label = Lbl::RepeatNotChar;
                            continue 'sm;
                        }

                        OP_NOTUPTO | OP_NOTUPTOI => {
                            *Lcharnot_min(F) = 0;
                            *Lcharnot_max(F) = GET2(*Fecode(F), 1);
                            reptype = REPTYPE_MAX;
                            *Fecode(F) = (*Fecode(F)).add(1 + IMM2_SIZE_U);
                            label = Lbl::RepeatNotChar;
                            continue 'sm;
                        }

                        OP_NOTMINUPTO | OP_NOTMINUPTOI => {
                            *Lcharnot_min(F) = 0;
                            *Lcharnot_max(F) = GET2(*Fecode(F), 1);
                            reptype = REPTYPE_MIN;
                            *Fecode(F) = (*Fecode(F)).add(1 + IMM2_SIZE_U);
                            label = Lbl::RepeatNotChar;
                            continue 'sm;
                        }

                        OP_NOTPOSSTAR | OP_NOTPOSSTARI => {
                            reptype = REPTYPE_POS;
                            *Lcharnot_min(F) = 0;
                            *Lcharnot_max(F) = u32::MAX;
                            *Fecode(F) = (*Fecode(F)).add(1);
                            label = Lbl::RepeatNotChar;
                            continue 'sm;
                        }

                        OP_NOTPOSPLUS | OP_NOTPOSPLUSI => {
                            reptype = REPTYPE_POS;
                            *Lcharnot_min(F) = 1;
                            *Lcharnot_max(F) = u32::MAX;
                            *Fecode(F) = (*Fecode(F)).add(1);
                            label = Lbl::RepeatNotChar;
                            continue 'sm;
                        }

                        OP_NOTPOSQUERY | OP_NOTPOSQUERYI => {
                            reptype = REPTYPE_POS;
                            *Lcharnot_min(F) = 0;
                            *Lcharnot_max(F) = 1;
                            *Fecode(F) = (*Fecode(F)).add(1);
                            label = Lbl::RepeatNotChar;
                            continue 'sm;
                        }

                        OP_NOTPOSUPTO | OP_NOTPOSUPTOI => {
                            reptype = REPTYPE_POS;
                            *Lcharnot_min(F) = 0;
                            *Lcharnot_max(F) = GET2(*Fecode(F), 1);
                            *Fecode(F) = (*Fecode(F)).add(1 + IMM2_SIZE_U);
                            label = Lbl::RepeatNotChar;
                            continue 'sm;
                        }

                        OP_NOTSTAR | OP_NOTSTARI | OP_NOTMINSTAR | OP_NOTMINSTARI
                        | OP_NOTPLUS | OP_NOTPLUSI | OP_NOTMINPLUS | OP_NOTMINPLUSI
                        | OP_NOTQUERY | OP_NOTQUERYI | OP_NOTMINQUERY
                        | OP_NOTMINQUERYI => {
                            fc = *(*Fecode(F)) as u32
                                - if Fop_val >= OP_NOTSTARI { OP_NOTSTARI } else { OP_NOTSTAR };
                            *Fecode(F) = (*Fecode(F)).add(1);
                            *Lcharnot_min(F) = REP_MIN[fc as usize];
                            *Lcharnot_max(F) = REP_MAX[fc as usize];
                            reptype = REP_TYP[fc as usize];
                            label = Lbl::RepeatNotChar;
                            continue 'sm;
                        }

                        // ---- Repeated single character (C 1303) ----
                        OP_EXACT | OP_EXACTI => {
                            *Lchar_min(F) = GET2(*Fecode(F), 1);
                            *Lchar_max(F) = *Lchar_min(F);
                            *Fecode(F) = (*Fecode(F)).add(1 + IMM2_SIZE_U);
                            label = Lbl::RepeatChar;
                            continue 'sm;
                        }
                        OP_POSUPTO | OP_POSUPTOI => {
                            reptype = REPTYPE_POS;
                            *Lchar_min(F) = 0;
                            *Lchar_max(F) = GET2(*Fecode(F), 1);
                            *Fecode(F) = (*Fecode(F)).add(1 + IMM2_SIZE_U);
                            label = Lbl::RepeatChar;
                            continue 'sm;
                        }
                        OP_UPTO | OP_UPTOI => {
                            reptype = REPTYPE_MAX;
                            *Lchar_min(F) = 0;
                            *Lchar_max(F) = GET2(*Fecode(F), 1);
                            *Fecode(F) = (*Fecode(F)).add(1 + IMM2_SIZE_U);
                            label = Lbl::RepeatChar;
                            continue 'sm;
                        }
                        OP_MINUPTO | OP_MINUPTOI => {
                            reptype = REPTYPE_MIN;
                            *Lchar_min(F) = 0;
                            *Lchar_max(F) = GET2(*Fecode(F), 1);
                            *Fecode(F) = (*Fecode(F)).add(1 + IMM2_SIZE_U);
                            label = Lbl::RepeatChar;
                            continue 'sm;
                        }
                        OP_POSSTAR | OP_POSSTARI => {
                            reptype = REPTYPE_POS;
                            *Lchar_min(F) = 0;
                            *Lchar_max(F) = u32::MAX;
                            *Fecode(F) = (*Fecode(F)).add(1);
                            label = Lbl::RepeatChar;
                            continue 'sm;
                        }
                        OP_POSPLUS | OP_POSPLUSI => {
                            reptype = REPTYPE_POS;
                            *Lchar_min(F) = 1;
                            *Lchar_max(F) = u32::MAX;
                            *Fecode(F) = (*Fecode(F)).add(1);
                            label = Lbl::RepeatChar;
                            continue 'sm;
                        }
                        OP_POSQUERY | OP_POSQUERYI => {
                            reptype = REPTYPE_POS;
                            *Lchar_min(F) = 0;
                            *Lchar_max(F) = 1;
                            *Fecode(F) = (*Fecode(F)).add(1);
                            label = Lbl::RepeatChar;
                            continue 'sm;
                        }
                        OP_STAR | OP_STARI | OP_MINSTAR | OP_MINSTARI | OP_PLUS | OP_PLUSI
                        | OP_MINPLUS | OP_MINPLUSI | OP_QUERY | OP_QUERYI | OP_MINQUERY
                        | OP_MINQUERYI => {
                            fc = *(*Fecode(F)) as u32
                                - if Fop_val < OP_STARI { OP_STAR } else { OP_STARI };
                            *Fecode(F) = (*Fecode(F)).add(1);
                            *Lchar_min(F) = REP_MIN[fc as usize];
                            *Lchar_max(F) = REP_MAX[fc as usize];
                            reptype = REP_TYP[fc as usize];
                            label = Lbl::RepeatChar;
                            continue 'sm;
                        }

                        // ---- OP_NCLASS / OP_CLASS (C 2065) ----
                        OP_NCLASS | OP_CLASS => {
                            *Lbyte_map_address(F) = (*Fecode(F)).add(1);
                            *Fecode(F) = (*Fecode(F)).add(1 + 32);
                            let byte_map = *Lbyte_map_address(F);

                            match *(*Fecode(F)) as u32 {
                                OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS
                                | OP_CRQUERY | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS
                                | OP_CRPOSQUERY => {
                                    fc = *(*Fecode(F)) as u32 - OP_CRSTAR;
                                    *Fecode(F) = (*Fecode(F)).add(1);
                                    *Lclass_min(F) = REP_MIN[fc as usize];
                                    *Lclass_max(F) = REP_MAX[fc as usize];
                                    reptype = REP_TYP[fc as usize];
                                }
                                OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                                    *Lclass_min(F) = GET2(*Fecode(F), 1);
                                    *Lclass_max(F) = GET2(*Fecode(F), 1 + IMM2_SIZE_U);
                                    if *Lclass_max(F) == 0 {
                                        *Lclass_max(F) = u32::MAX;
                                    }
                                    reptype = REP_TYP[(*(*Fecode(F)) as u32 - OP_CRSTAR) as usize];
                                    *Fecode(F) = (*Fecode(F)).add(1 + 2 * IMM2_SIZE_U);
                                }
                                _ => {
                                    *Lclass_min(F) = 1;
                                    *Lclass_max(F) = 1;
                                }
                            }

                            // Ensure the minimum number of matches.
                            if utf {
                                i = 1;
                                while i <= *Lclass_min(F) {
                                    if *Feptr(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                    let mut ep = *Feptr(F);
                                    fc = GETCHARINC(&mut ep);
                                    *Feptr(F) = ep;
                                    if fc > 255 {
                                        if Fop_val == OP_CLASS {
                                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                        }
                                    } else if (*byte_map.add((fc / 8) as usize)
                                        & (1u8 << (fc & 7)))
                                        == 0
                                    {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                    i += 1;
                                }
                            } else {
                                i = 1;
                                while i <= *Lclass_min(F) {
                                    if *Feptr(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                    fc = *(*Feptr(F)) as u32;
                                    *Feptr(F) = (*Feptr(F)).add(1);
                                    if (*byte_map.add((fc / 8) as usize) & (1u8 << (fc & 7)))
                                        == 0
                                    {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                    i += 1;
                                }
                            }

                            if *Lclass_min(F) == *Lclass_max(F) {
                                { label = Lbl::MainLoop; continue 'sm; }
                            }

                            if reptype == REPTYPE_MIN {
                                if utf {
                                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM200; label = Lbl::MatchRecurse; continue 'sm; }
                                } else {
                                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM23; label = Lbl::MatchRecurse; continue 'sm; }
                                }
                            } else {
                                // Maximize.
                                *Lclass_start_eptr(F) = *Feptr(F);
                                if utf {
                                    i = *Lclass_min(F);
                                    while i < *Lclass_max(F) {
                                        let mut len: u32 = 1;
                                        if *Feptr(F) >= (*mb).end_subject {
                                            SCHECK_PARTIAL!();
                                            break;
                                        }
                                        fc = GETCHARLEN(*Feptr(F), &mut len);
                                        if fc > 255 {
                                            if Fop_val == OP_CLASS {
                                                break;
                                            }
                                        } else if (*byte_map.add((fc / 8) as usize)
                                            & (1u8 << (fc & 7)))
                                            == 0
                                        {
                                            break;
                                        }
                                        *Feptr(F) = (*Feptr(F)).add(len as usize);
                                        i += 1;
                                    }
                                    if reptype == REPTYPE_POS {
                                        { label = Lbl::MainLoop; continue 'sm; }
                                    }
                                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM201; label = Lbl::MatchRecurse; continue 'sm; }
                                } else {
                                    i = *Lclass_min(F);
                                    while i < *Lclass_max(F) {
                                        if *Feptr(F) >= (*mb).end_subject {
                                            SCHECK_PARTIAL!();
                                            break;
                                        }
                                        fc = *(*Feptr(F)) as u32;
                                        if (*byte_map.add((fc / 8) as usize) & (1u8 << (fc & 7)))
                                            == 0
                                        {
                                            break;
                                        }
                                        *Feptr(F) = (*Feptr(F)).add(1);
                                        i += 1;
                                    }
                                    if reptype == REPTYPE_POS {
                                        { label = Lbl::MainLoop; continue 'sm; }
                                    }
                                    if *Feptr(F) >= *Lclass_start_eptr(F) {
                                        { start_ecode = *Fecode(F); *Freturn_id(F) = RM24; label = Lbl::MatchRecurse; continue 'sm; }
                                    }
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                            }
                        }

                        // ---- OP_XCLASS (C 2293) ----
                        OP_XCLASS => {
                            *Lxclass_data(F) = (*Fecode(F)).add(1 + LINK_SIZE_U);
                            *Fecode(F) = (*Fecode(F)).add(GET(*Fecode(F), 1) as usize);

                            match *(*Fecode(F)) as u32 {
                                OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS
                                | OP_CRQUERY | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS
                                | OP_CRPOSQUERY => {
                                    fc = *(*Fecode(F)) as u32 - OP_CRSTAR;
                                    *Fecode(F) = (*Fecode(F)).add(1);
                                    *Lxclass_min(F) = REP_MIN[fc as usize];
                                    *Lxclass_max(F) = REP_MAX[fc as usize];
                                    reptype = REP_TYP[fc as usize];
                                }
                                OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                                    *Lxclass_min(F) = GET2(*Fecode(F), 1);
                                    *Lxclass_max(F) = GET2(*Fecode(F), 1 + IMM2_SIZE_U);
                                    if *Lxclass_max(F) == 0 {
                                        *Lxclass_max(F) = u32::MAX;
                                    }
                                    reptype = REP_TYP[(*(*Fecode(F)) as u32 - OP_CRSTAR) as usize];
                                    *Fecode(F) = (*Fecode(F)).add(1 + 2 * IMM2_SIZE_U);
                                }
                                _ => {
                                    *Lxclass_min(F) = 1;
                                    *Lxclass_max(F) = 1;
                                }
                            }

                            i = 1;
                            while i <= *Lxclass_min(F) {
                                if *Feptr(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                let mut ep = *Feptr(F);
                                fc = GETCHARINCTEST(&mut ep, utf);
                                *Feptr(F) = ep;
                                if crate::xclass::_pcre2_xclass_8(
                                    fc, *Lxclass_data(F),
                                    (*mb).start_code as *const u8, utf as BOOL,
                                ) == FALSE
                                {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                i += 1;
                            }

                            if *Lxclass_min(F) == *Lxclass_max(F) {
                                { label = Lbl::MainLoop; continue 'sm; }
                            }

                            if reptype == REPTYPE_MIN {
                                { start_ecode = *Fecode(F); *Freturn_id(F) = RM100; label = Lbl::MatchRecurse; continue 'sm; }
                            } else {
                                *Lxclass_start_eptr(F) = *Feptr(F);
                                i = *Lxclass_min(F);
                                while i < *Lxclass_max(F) {
                                    let mut len: u32 = 1;
                                    if *Feptr(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        break;
                                    }
                                    fc = GETCHARLENTEST(*Feptr(F), &mut len, utf);
                                    if crate::xclass::_pcre2_xclass_8(
                                        fc, *Lxclass_data(F),
                                        (*mb).start_code as *const u8, utf as BOOL,
                                    ) == FALSE
                                    {
                                        break;
                                    }
                                    *Feptr(F) = (*Feptr(F)).add(len as usize);
                                    i += 1;
                                }
                                if reptype == REPTYPE_POS {
                                    { label = Lbl::MainLoop; continue 'sm; }
                                }
                                { start_ecode = *Fecode(F); *Freturn_id(F) = RM101; label = Lbl::MatchRecurse; continue 'sm; }
                            }
                        }

                        // ---- OP_ECLASS (C 2434) ----
                        OP_ECLASS => {
                            *Leclass_data(F) = (*Fecode(F)).add(1 + LINK_SIZE_U);
                            *Fecode(F) = (*Fecode(F)).add(GET(*Fecode(F), 1) as usize);
                            *Leclass_len(F) = (*Fecode(F))
                                .offset_from(*Leclass_data(F)) as PCRE2_SIZE;

                            match *(*Fecode(F)) as u32 {
                                OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS
                                | OP_CRQUERY | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS
                                | OP_CRPOSQUERY => {
                                    fc = *(*Fecode(F)) as u32 - OP_CRSTAR;
                                    *Fecode(F) = (*Fecode(F)).add(1);
                                    *Leclass_min(F) = REP_MIN[fc as usize];
                                    *Leclass_max(F) = REP_MAX[fc as usize];
                                    reptype = REP_TYP[fc as usize];
                                }
                                OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                                    *Leclass_min(F) = GET2(*Fecode(F), 1);
                                    *Leclass_max(F) = GET2(*Fecode(F), 1 + IMM2_SIZE_U);
                                    if *Leclass_max(F) == 0 {
                                        *Leclass_max(F) = u32::MAX;
                                    }
                                    reptype = REP_TYP[(*(*Fecode(F)) as u32 - OP_CRSTAR) as usize];
                                    *Fecode(F) = (*Fecode(F)).add(1 + 2 * IMM2_SIZE_U);
                                }
                                _ => {
                                    *Leclass_min(F) = 1;
                                    *Leclass_max(F) = 1;
                                }
                            }

                            i = 1;
                            while i <= *Leclass_min(F) {
                                if *Feptr(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                let mut ep = *Feptr(F);
                                fc = GETCHARINCTEST(&mut ep, utf);
                                *Feptr(F) = ep;
                                if crate::xclass::_pcre2_eclass_8(
                                    fc, *Leclass_data(F),
                                    (*Leclass_data(F)).add(*Leclass_len(F)),
                                    (*mb).start_code as *const u8, utf as BOOL,
                                ) == FALSE
                                {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                i += 1;
                            }

                            if *Leclass_min(F) == *Leclass_max(F) {
                                { label = Lbl::MainLoop; continue 'sm; }
                            }

                            if reptype == REPTYPE_MIN {
                                { start_ecode = *Fecode(F); *Freturn_id(F) = RM102; label = Lbl::MatchRecurse; continue 'sm; }
                            } else {
                                *Leclass_start_eptr(F) = *Feptr(F);
                                i = *Leclass_min(F);
                                while i < *Leclass_max(F) {
                                    let mut len: u32 = 1;
                                    if *Feptr(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        break;
                                    }
                                    fc = GETCHARLENTEST(*Feptr(F), &mut len, utf);
                                    if crate::xclass::_pcre2_eclass_8(
                                        fc, *Leclass_data(F),
                                        (*Leclass_data(F)).add(*Leclass_len(F)),
                                        (*mb).start_code as *const u8, utf as BOOL,
                                    ) == FALSE
                                    {
                                        break;
                                    }
                                    *Feptr(F) = (*Feptr(F)).add(len as usize);
                                    i += 1;
                                }
                                if reptype == REPTYPE_POS {
                                    { label = Lbl::MainLoop; continue 'sm; }
                                }
                                { start_ecode = *Fecode(F); *Freturn_id(F) = RM103; label = Lbl::MatchRecurse; continue 'sm; }
                            }
                        }

                        // ---- Char-type opcodes (PCRE2_UCP not set) (C 2580) ----
                        OP_NOT_DIGIT => {
                            if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                            let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                            if CHMAX_255(fc) && (*(*mb).ctypes.add(fc as usize) as u32 & ctype_digit as u32) != 0 {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            *Fecode(F) = (*Fecode(F)).add(1); { label = Lbl::MainLoop; continue 'sm; }
                        }
                        OP_DIGIT => {
                            if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                            let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                            if !CHMAX_255(fc) || (*(*mb).ctypes.add(fc as usize) as u32 & ctype_digit as u32) == 0 {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            *Fecode(F) = (*Fecode(F)).add(1); { label = Lbl::MainLoop; continue 'sm; }
                        }
                        OP_NOT_WHITESPACE => {
                            if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                            let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                            if CHMAX_255(fc) && (*(*mb).ctypes.add(fc as usize) as u32 & ctype_space as u32) != 0 {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            *Fecode(F) = (*Fecode(F)).add(1); { label = Lbl::MainLoop; continue 'sm; }
                        }
                        OP_WHITESPACE => {
                            if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                            let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                            if !CHMAX_255(fc) || (*(*mb).ctypes.add(fc as usize) as u32 & ctype_space as u32) == 0 {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            *Fecode(F) = (*Fecode(F)).add(1); { label = Lbl::MainLoop; continue 'sm; }
                        }
                        OP_NOT_WORDCHAR => {
                            if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                            let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                            if CHMAX_255(fc) && (*(*mb).ctypes.add(fc as usize) as u32 & ctype_word as u32) != 0 {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            *Fecode(F) = (*Fecode(F)).add(1); { label = Lbl::MainLoop; continue 'sm; }
                        }
                        OP_WORDCHAR => {
                            if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                            let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                            if !CHMAX_255(fc) || (*(*mb).ctypes.add(fc as usize) as u32 & ctype_word as u32) == 0 {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            *Fecode(F) = (*Fecode(F)).add(1); { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_ANYNL (C 2645) ----
                        OP_ANYNL => {
                            if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                            let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                            match fc {
                                CHAR_CR => {
                                    if *Feptr(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                    } else if *(*Feptr(F)) as u32 == CHAR_NL {
                                        *Feptr(F) = (*Feptr(F)).add(1);
                                    }
                                }
                                CHAR_NL => {}
                                0x0b | 0x0c | 0x85 | 0x2028 | 0x2029 => {
                                    if (*mb).bsr_convention as i64 == PCRE2_BSR_ANYCRLF {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                }
                                _ => { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                            }
                            *Fecode(F) = (*Fecode(F)).add(1); { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_NOT_HSPACE / OP_HSPACE (C 2680 / 2697) ----
                        OP_NOT_HSPACE => {
                            if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                            let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                            if is_hspace(fc) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                            *Fecode(F) = (*Fecode(F)).add(1); { label = Lbl::MainLoop; continue 'sm; }
                        }
                        OP_HSPACE => {
                            if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                            let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                            if !is_hspace(fc) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                            *Fecode(F) = (*Fecode(F)).add(1); { label = Lbl::MainLoop; continue 'sm; }
                        }
                        OP_NOT_VSPACE => {
                            if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                            let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                            if is_vspace(fc) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                            *Fecode(F) = (*Fecode(F)).add(1); { label = Lbl::MainLoop; continue 'sm; }
                        }
                        OP_VSPACE => {
                            if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                            let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                            if !is_vspace(fc) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                            *Fecode(F) = (*Fecode(F)).add(1); { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_PROP / OP_NOTPROP (C 2761) ----
                        OP_PROP | OP_NOTPROP => {
                            if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                            let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;

                            let prop = GET_UCD(fc);
                            let notmatch = Fop_val == OP_NOTPROP;
                            let ec1 = *(*Fecode(F)).add(1) as u32;
                            let ec2 = *(*Fecode(F)).add(2) as u32;
                            match ec1 as i64 {
                                PT_LAMP => {
                                    let chartype = prop.chartype as u32;
                                    if (chartype == ucp_Lu as u32 || chartype == ucp_Ll as u32
                                        || chartype == ucp_Lt as u32) == notmatch {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                }
                                PT_GC => {
                                    if (ec2 == crate::tables::_pcre2_ucp_gentype[prop.chartype as usize]) == notmatch {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                }
                                PT_PC => {
                                    if (ec2 == prop.chartype as u32) == notmatch { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                                }
                                PT_SC => {
                                    if (ec2 == prop.script as u32) == notmatch { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                                }
                                PT_SCX => {
                                    let ok = ec2 == prop.script as u32
                                        || MAPBIT(
                                            crate::tables::_pcre2_ucd_script_sets_8
                                                .as_ptr()
                                                .add(UCD_SCRIPTX_PROP(prop) as usize),
                                            ec2,
                                        ) != 0;
                                    if ok == notmatch { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                                }
                                PT_ALNUM => {
                                    let chartype = prop.chartype as usize;
                                    if (crate::tables::_pcre2_ucp_gentype[chartype] == ucp_L as u32
                                        || crate::tables::_pcre2_ucp_gentype[chartype] == ucp_N as u32)
                                        == notmatch { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                                }
                                PT_SPACE | PT_PXSPACE => {
                                    if is_hspace(fc) || is_vspace(fc) {
                                        if notmatch { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                                    } else if (crate::tables::_pcre2_ucp_gentype[prop.chartype as usize]
                                        == ucp_Z as u32) == notmatch {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                }
                                PT_WORD => {
                                    let chartype = prop.chartype as u32;
                                    if (crate::tables::_pcre2_ucp_gentype[chartype as usize] == ucp_L as u32
                                        || crate::tables::_pcre2_ucp_gentype[chartype as usize] == ucp_N as u32
                                        || chartype == ucp_Mn as u32
                                        || chartype == ucp_Pc as u32) == notmatch {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                }
                                PT_CLIST => {
                                    let mut cp: *const u32 = crate::tables::_pcre2_ucd_caseless_sets_8
                                        .as_ptr()
                                        .add(ec2 as usize);
                                    loop {
                                        if fc < *cp {
                                            if notmatch { break; } else { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                                        }
                                        let cur = *cp; cp = cp.add(1);
                                        if fc == cur {
                                            if notmatch { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } } else { break; }
                                        }
                                    }
                                }
                                PT_UCNC => {
                                    if ((fc == 0x24 || fc == 0x40 || fc == 0x60
                                        || (fc >= 0xa0 && fc <= 0xd7ff) || fc >= 0xe000)) == notmatch {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                }
                                PT_BIDICL => {
                                    if (UCD_BIDICLASS_PROP(prop) == ec2) == notmatch {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                }
                                PT_BOOL => {
                                    let ok = MAPBIT(
                                        crate::tables::_pcre2_ucd_boolprop_sets_8
                                            .as_ptr()
                                            .add(UCD_BPROPS_PROP(prop) as usize),
                                        ec2,
                                    ) != 0;
                                    if ok == notmatch { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                                }
                                _ => { return PCRE2_ERROR_INTERNAL as c_int; }
                            }
                            *Fecode(F) = (*Fecode(F)).add(3);
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_EXTUNI (C 2895) ----
                        OP_EXTUNI => {
                            if *Feptr(F) >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            } else {
                                let mut ep = *Feptr(F);
                                fc = GETCHARINCTEST(&mut ep, utf);
                                *Feptr(F) = ep;
                                *Feptr(F) = crate::extuni::_pcre2_extuni_8(
                                    fc, *Feptr(F), (*mb).start_subject, (*mb).end_subject,
                                    utf as BOOL, ptr::null_mut(),
                                );
                            }
                            CHECK_PARTIAL!();
                            *Fecode(F) = (*Fecode(F)).add(1);
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- Char-type repeat setup (C 2919) ----
                        OP_TYPEEXACT => {
                            *Ltype_min(F) = GET2(*Fecode(F), 1);
                            *Ltype_max(F) = *Ltype_min(F);
                            *Fecode(F) = (*Fecode(F)).add(1 + IMM2_SIZE_U);
                            label = Lbl::RepeatType; continue 'sm;
                        }
                        OP_TYPEUPTO | OP_TYPEMINUPTO => {
                            *Ltype_min(F) = 0;
                            *Ltype_max(F) = GET2(*Fecode(F), 1);
                            reptype = if *(*Fecode(F)) as u32 == OP_TYPEMINUPTO { REPTYPE_MIN } else { REPTYPE_MAX };
                            *Fecode(F) = (*Fecode(F)).add(1 + IMM2_SIZE_U);
                            label = Lbl::RepeatType; continue 'sm;
                        }
                        OP_TYPEPOSSTAR => {
                            reptype = REPTYPE_POS; *Ltype_min(F) = 0; *Ltype_max(F) = u32::MAX;
                            *Fecode(F) = (*Fecode(F)).add(1); label = Lbl::RepeatType; continue 'sm;
                        }
                        OP_TYPEPOSPLUS => {
                            reptype = REPTYPE_POS; *Ltype_min(F) = 1; *Ltype_max(F) = u32::MAX;
                            *Fecode(F) = (*Fecode(F)).add(1); label = Lbl::RepeatType; continue 'sm;
                        }
                        OP_TYPEPOSQUERY => {
                            reptype = REPTYPE_POS; *Ltype_min(F) = 0; *Ltype_max(F) = 1;
                            *Fecode(F) = (*Fecode(F)).add(1); label = Lbl::RepeatType; continue 'sm;
                        }
                        OP_TYPEPOSUPTO => {
                            reptype = REPTYPE_POS; *Ltype_min(F) = 0; *Ltype_max(F) = GET2(*Fecode(F), 1);
                            *Fecode(F) = (*Fecode(F)).add(1 + IMM2_SIZE_U);
                            label = Lbl::RepeatType; continue 'sm;
                        }
                        OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS
                        | OP_TYPEQUERY | OP_TYPEMINQUERY => {
                            fc = *(*Fecode(F)) as u32 - OP_TYPESTAR;
                            *Fecode(F) = (*Fecode(F)).add(1);
                            *Ltype_min(F) = REP_MIN[fc as usize];
                            *Ltype_max(F) = REP_MAX[fc as usize];
                            reptype = REP_TYP[fc as usize];
                            label = Lbl::RepeatType; continue 'sm;
                        }

                        // NOTE: OP_CRSTAR..OP_CRPOSRANGE never appear as a
                        // leading opcode here (they follow classes), so they
                        // fall to the default/unreachable arm if encountered.
                        // ---- OP_DNREF / OP_DNREFI (C 5249) ----
                        OP_DNREF | OP_DNREFI => {
                            *Fbyte1(F) = (Fop_val == OP_DNREFI) as u8; // Lcaseless
                            *Fbyte2(F) = if Fop_val == OP_DNREFI {
                                *(*Fecode(F)).add(1 + 2 * IMM2_SIZE_U) // Lcaseopts
                            } else {
                                0
                            };
                            {
                                let mut count = GET2(*Fecode(F), 1 + IMM2_SIZE_U) as i32;
                                let mut slot: PCRE2_SPTR = (*mb)
                                    .name_table
                                    .add(GET2(*Fecode(F), 1) as usize * (*mb).name_entry_size as usize);
                                *Fecode(F) = (*Fecode(F))
                                    .add(1 + 2 * IMM2_SIZE_U + if Fop_val == OP_DNREFI { 1 } else { 0 });

                                loop {
                                    let old = count;
                                    count -= 1;
                                    if old <= 0 {
                                        break;
                                    }
                                    *Loffset(F) = ((GET2(slot, 0) << 1) - 2) as PCRE2_SIZE;
                                    if *Loffset(F) < *Foffset_top(F)
                                        && *Fovector(F).add(*Loffset(F)) != PCRE2_UNSET
                                    {
                                        break;
                                    }
                                    slot = slot.add((*mb).name_entry_size as usize);
                                }
                            }
                            label = Lbl::RefRepeat;
                            continue 'sm;
                        }

                        // ---- OP_REF / OP_REFI (C 5267) ----
                        OP_REF | OP_REFI => {
                            *Fbyte1(F) = (Fop_val == OP_REFI) as u8; // Lcaseless
                            *Fbyte2(F) = if Fop_val == OP_REFI {
                                *(*Fecode(F)).add(1 + IMM2_SIZE_U) // Lcaseopts
                            } else {
                                0
                            };
                            *Loffset(F) = ((GET2(*Fecode(F), 1) << 1) - 2) as PCRE2_SIZE;
                            *Fecode(F) = (*Fecode(F))
                                .add(1 + IMM2_SIZE_U + if Fop_val == OP_REFI { 1 } else { 0 });
                            label = Lbl::RefRepeat;
                            continue 'sm;
                        }

                        // ---- OP_BRAZERO (C 5489) ----
                        OP_BRAZERO => {
                            *Fecode(F) = (*Fecode(F)).add(1);
                            { start_ecode = *Fecode(F); *Freturn_id(F) = RM9; label = Lbl::MatchRecurse; continue 'sm; }
                        }

                        // ---- OP_BRAMINZERO (C 5502) ----
                        OP_BRAMINZERO => {
                            *Fecode(F) = (*Fecode(F)).add(1);
                            let mut next_ecode: PCRE2_SPTR = *Fecode(F);
                            loop {
                                next_ecode = next_ecode.add(GET(next_ecode, 1) as usize);
                                if *next_ecode as u32 != OP_ALT {
                                    break;
                                }
                            }
                            { start_ecode = next_ecode.add(1 + LINK_SIZE_U); *Freturn_id(F) = RM10; label = Lbl::MatchRecurse; continue 'sm; }
                        }

                        // ---- OP_SKIPZERO (C 5514) ----
                        OP_SKIPZERO => {
                            let mut next_ecode: PCRE2_SPTR = (*Fecode(F)).add(1);
                            loop {
                                next_ecode = next_ecode.add(GET(next_ecode, 1) as usize);
                                if *next_ecode as u32 != OP_ALT {
                                    break;
                                }
                            }
                            *Fecode(F) = next_ecode.add(1 + LINK_SIZE_U);
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_BRAPOSZERO (C 5534) ----
                        OP_BRAPOSZERO => {
                            *Fbyte2(F) = TRUE as u8; // Lzero_allowed = TRUE
                            *Fecode(F) = (*Fecode(F)).add(1);
                            if *(*Fecode(F)) as u32 == OP_CBRAPOS
                                || *(*Fecode(F)) as u32 == OP_SCBRAPOS
                            {
                                label = Lbl::PossessiveCapture;
                                continue 'sm;
                            }
                            label = Lbl::PossessiveNonCapture;
                            continue 'sm;
                        }

                        // ---- OP_BRAPOS / OP_SBRAPOS (C 5541) ----
                        OP_BRAPOS | OP_SBRAPOS => {
                            *Fbyte2(F) = FALSE as u8; // Lzero_allowed = FALSE
                            label = Lbl::PossessiveNonCapture;
                            continue 'sm;
                        }

                        // ---- OP_CBRAPOS / OP_SCBRAPOS (C 5549) ----
                        OP_CBRAPOS | OP_SCBRAPOS => {
                            *Fbyte2(F) = FALSE as u8; // Lzero_allowed = FALSE
                            label = Lbl::PossessiveCapture;
                            continue 'sm;
                        }

                        // ---- OP_BRA (C 5622) ----
                        OP_BRA => {
                            if (*mb).hasthen != FALSE || *Frdepth(F) == 0 {
                                *Lbra_frame_type(F) = 0;
                                label = Lbl::GroupLoop;
                                continue 'sm;
                            }

                            loop {
                                let current_branch: PCRE2_SPTR = *Fecode(F);
                                let next_branch: PCRE2_SPTR =
                                    current_branch.add(GET(current_branch, 1) as usize);

                                if *next_branch as u32 != OP_ALT {
                                    break;
                                }

                                *Fecode(F) = next_branch;
                                { start_ecode = current_branch.add(1 + LINK_SIZE_U); *Freturn_id(F) = RM1; label = Lbl::MatchRecurse; continue 'sm; }
                            }

                            // Hit the start of the final branch; continue at
                            // this level.
                            *Fecode(F) = (*Fecode(F)).add(1 + LINK_SIZE_U);
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_CBRA / OP_SCBRA (C 5661) ----
                        OP_CBRA | OP_SCBRA => {
                            *Lbra_frame_type(F) = GF_CAPTURE | GET2(*Fecode(F), 1 + LINK_SIZE_U);
                            label = Lbl::GroupLoop;
                            continue 'sm;
                        }

                        // ---- OP_ONCE / OP_SCRIPT_RUN / OP_SBRA (C 5671) ----
                        OP_ONCE | OP_SCRIPT_RUN | OP_SBRA => {
                            *Lbra_frame_type(F) = GF_NOCAPTURE;
                            label = Lbl::GroupLoop;
                            continue 'sm;
                        }

                        // ---- OP_RECURSE (C 5707) ----
                        OP_RECURSE => {
                            bracode = (*mb).start_code.add(GET(*Fecode(F), 1) as usize);
                            number = if bracode == (*mb).start_code {
                                0
                            } else {
                                GET2(bracode, 1 + LINK_SIZE_U)
                            };

                            if *Fcurrent_recurse(F) != RECURSE_UNSET {
                                offset = *Flast_group_offset(F);
                                while offset != PCRE2_UNSET {
                                    N = ((*match_data).heapframes as *mut u8).add(offset)
                                        as *mut heapframe;
                                    P = (N as *mut u8).sub(frame_size) as *mut heapframe;
                                    if *Fgroup_frame_type(N) == (GF_RECURSE | number) {
                                        if *Feptr(F) == *Feptr(P)
                                            && (*mb).last_used_ptr == (*P).recurse_last_used
                                            && ((*mb).moptions
                                                & PCRE2_DISABLE_RECURSELOOP_CHECK as u32)
                                                == 0
                                        {
                                            return PCRE2_ERROR_RECURSELOOP as c_int;
                                        }
                                        break;
                                    }
                                    offset = *Flast_group_offset(P);
                                }
                            }

                            (*F).recurse_last_used = (*mb).last_used_ptr;
                            *Lrecurse_start_branch(F) = bracode;
                            *Lrecurse_frame_type(F) = GF_RECURSE | number;

                            group_frame_type = *Lrecurse_frame_type(F);
                            { start_ecode = (*Lrecurse_start_branch(F)).add(
                                    crate::tables::_pcre2_OP_lengths_8
                                        [*(*Lrecurse_start_branch(F)) as usize]
                                        as usize
                                ); *Freturn_id(F) = RM11; label = Lbl::MatchRecurse; continue 'sm; }
                        }

                        // ---- OP_ASSERT / OP_ASSERTBACK / OP_ASSERT_NA /
                        //      OP_ASSERTBACK_NA (C 5789) ----
                        OP_ASSERT | OP_ASSERTBACK | OP_ASSERT_NA | OP_ASSERTBACK_NA => {
                            group_frame_type = GF_NOCAPTURE;
                            { start_ecode = (*Fecode(F)).add(
                                    crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize]
                                        as usize
                                ); *Freturn_id(F) = RM3; label = Lbl::MatchRecurse; continue 'sm; }
                        }

                        // ---- OP_ASSERT_NOT / OP_ASSERTBACK_NOT (C 5820) ----
                        OP_ASSERT_NOT | OP_ASSERTBACK_NOT => {
                            group_frame_type = GF_NOCAPTURE;
                            { start_ecode = (*Fecode(F)).add(
                                    crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize]
                                        as usize
                                ); *Freturn_id(F) = RM4; label = Lbl::MatchRecurse; continue 'sm; }
                        }

                        // ---- OP_ASSERT_SCS (C 5867) ----
                        OP_ASSERT_SCS => {
                            length = 0;
                            {
                                let mut ecode: PCRE2_SPTR = (*Fecode(F)).add(1 + LINK_SIZE_U);

                                // Disable compiler warning (C sets offset = 0).
                                offset = 0;
                                let _ = offset;

                                // Find the first set offset among the CREF /
                                // DNCREF options. Breaking out of `'scan`
                                // models the C `goto SCS_OFFSET_FOUND`.
                                'scan: loop {
                                    if *ecode as u32 == OP_CREF {
                                        length += 1 + IMM2_SIZE_U;
                                        offset = ((GET2(ecode, 1) << 1) - 2) as PCRE2_SIZE;
                                        ecode = ecode.add(1 + IMM2_SIZE_U);
                                        if offset < *Foffset_top(F)
                                            && *Fovector(F).add(offset) != PCRE2_UNSET
                                        {
                                            break 'scan;
                                        }
                                        continue 'scan;
                                    }

                                    if *ecode as u32 != OP_DNCREF {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }

                                    let mut count = GET2(ecode, 1 + IMM2_SIZE_U) as i32;
                                    let mut slot: PCRE2_SPTR = (*mb).name_table.add(
                                        GET2(ecode, 1) as usize * (*mb).name_entry_size as usize,
                                    );
                                    length += 1 + 2 * IMM2_SIZE_U;
                                    ecode = ecode.add(1 + 2 * IMM2_SIZE_U);

                                    while count > 0 {
                                        offset = ((GET2(slot, 0) << 1) - 2) as PCRE2_SIZE;
                                        if offset < *Foffset_top(F)
                                            && *Fovector(F).add(offset) != PCRE2_UNSET
                                        {
                                            break 'scan;
                                        }
                                        slot = slot.add((*mb).name_entry_size as usize);
                                        count -= 1;
                                    }
                                }

                                // Stash `ecode` so `Lbl::ScsOffsetFound` can
                                // resume the "skip remaining options" scan; it
                                // overwrites this slot with `Feptr` (C's
                                // `Lsaved_eptr = Feptr`) once done.
                                *Lsaved_eptr(F) = ecode;
                            }
                            label = Lbl::ScsOffsetFound;
                            continue 'sm;
                        }

                        // ---- OP_CALLOUT / OP_CALLOUT_STR (C 5989) ----
                        OP_CALLOUT | OP_CALLOUT_STR => {
                            rrc = do_callout(F, mb, &raw mut length);
                            if rrc > 0 {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            if rrc < 0 {
                                { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            *Fecode(F) = (*Fecode(F)).add(length as usize);
                            { label = Lbl::MainLoop; continue 'sm; }
                        }
                        // ---- OP_COND / OP_SCOND (C 6008) ----
                        OP_COND | OP_SCOND => {
                            // Llength: offset to the second branch. If the
                            // second branch is non-existent, point to the KET.
                            *Lcond_length(F) = GET(*Fecode(F), 1) as PCRE2_SIZE;
                            if *(*Fecode(F)).add(*Lcond_length(F)) as u32 != OP_ALT {
                                *Lcond_length(F) -= 1 + LINK_SIZE_U;
                            }
                            *Fecode(F) = (*Fecode(F)).add(1 + LINK_SIZE_U);

                            // A callout may be inserted between OP_COND and an
                            // assertion condition.
                            if *(*Fecode(F)) as u32 == OP_CALLOUT
                                || *(*Fecode(F)) as u32 == OP_CALLOUT_STR
                            {
                                rrc = crate::match_util::do_callout(F, mb, &mut length);
                                if rrc > 0 {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                if rrc < 0 {
                                    { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                // Advance Fecode past the callout so it points
                                // to the condition, adjusting Llength so that
                                // Fecode+Llength is unchanged.
                                *Fecode(F) = (*Fecode(F)).add(length);
                                *Lcond_length(F) -= length;
                            }

                            // Test the various possible conditions.
                            condition = FALSE;
                            match *(*Fecode(F)) as u32 {
                                OP_RREF => {
                                    // Group recursion test.
                                    if *Fcurrent_recurse(F) != RECURSE_UNSET {
                                        number = GET2(*Fecode(F), 1);
                                        condition = (number == RREF_ANY as u32
                                            || number == *Fcurrent_recurse(F))
                                            as BOOL;
                                    }
                                }
                                OP_DNRREF => {
                                    // Duplicate named group recursion test.
                                    if *Fcurrent_recurse(F) != RECURSE_UNSET {
                                        let mut count = GET2(*Fecode(F), 1 + IMM2_SIZE_U) as i32;
                                        let mut slot: PCRE2_SPTR = (*mb).name_table.add(
                                            (GET2(*Fecode(F), 1) as usize)
                                                * (*mb).name_entry_size as usize,
                                        );
                                        while count > 0 {
                                            count -= 1;
                                            number = GET2(slot, 0);
                                            condition = (number == *Fcurrent_recurse(F)) as BOOL;
                                            if condition != FALSE {
                                                break;
                                            }
                                            slot = slot.add((*mb).name_entry_size as usize);
                                        }
                                    }
                                }
                                OP_CREF => {
                                    // Numbered group used test.
                                    offset = ((GET2(*Fecode(F), 1) << 1) - 2) as PCRE2_SIZE;
                                    condition = (offset < *Foffset_top(F)
                                        && *Fovector(F).add(offset) != PCRE2_UNSET)
                                        as BOOL;
                                }
                                OP_DNCREF => {
                                    // Duplicate named group used test.
                                    let mut count = GET2(*Fecode(F), 1 + IMM2_SIZE_U) as i32;
                                    let mut slot: PCRE2_SPTR = (*mb).name_table.add(
                                        (GET2(*Fecode(F), 1) as usize)
                                            * (*mb).name_entry_size as usize,
                                    );
                                    while count > 0 {
                                        count -= 1;
                                        offset = ((GET2(slot, 0) << 1) - 2) as PCRE2_SIZE;
                                        condition = (offset < *Foffset_top(F)
                                            && *Fovector(F).add(offset) != PCRE2_UNSET)
                                            as BOOL;
                                        if condition != FALSE {
                                            break;
                                        }
                                        slot = slot.add((*mb).name_entry_size as usize);
                                    }
                                }
                                OP_FALSE | OP_FAIL => {
                                    // The assertion (?!) becomes OP_FAIL.
                                }
                                OP_TRUE => {
                                    condition = TRUE;
                                }
                                _ => {
                                    // The condition is an assertion.
                                    *Fbyte1(F) = (*(*Fecode(F)) as u32 == OP_ASSERT
                                        || *(*Fecode(F)) as u32 == OP_ASSERTBACK)
                                        as u8;
                                    *Lcond_start_branch(F) = *Fecode(F);
                                    group_frame_type = GF_CONDASSERT;
                                    { start_ecode = (*Lcond_start_branch(F)).add(
                                            crate::tables::_pcre2_OP_lengths_8
                                                [*(*Lcond_start_branch(F)) as usize]
                                                as usize
                                        ); *Freturn_id(F) = RM5; label = Lbl::MatchRecurse; continue 'sm; }
                                }
                            }

                            // Choose branch according to the condition.
                            *Fecode(F) = (*Fecode(F)).add(if condition != FALSE {
                                crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize] as usize
                            } else {
                                *Lcond_length(F)
                            });

                            // For OP_SCOND (repeated conditional group that
                            // might match empty) descend a level so the start
                            // is remembered. For OP_COND, continue at this
                            // level.
                            if Fop_val == OP_SCOND {
                                group_frame_type = GF_NOCAPTURE;
                                { start_ecode = *Fecode(F); *Freturn_id(F) = RM35; label = Lbl::MatchRecurse; continue 'sm; }
                            }
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_REVERSE (C 6190) ----
                        OP_REVERSE => {
                            number = GET2(*Fecode(F), 1);
                            if utf {
                                // Move back `number` characters.
                                while number > 0 {
                                    number -= 1;
                                    if *Feptr(F) <= (*mb).check_subject {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                    *Feptr(F) = (*Feptr(F)).sub(1);
                                    let mut ep = *Feptr(F);
                                    BACKCHAR(&mut ep);
                                    *Feptr(F) = ep;
                                }
                            } else {
                                // Not UTF: count is code-unit count.
                                if (number as isize)
                                    > (*Feptr(F)).offset_from((*mb).start_subject)
                                {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                *Feptr(F) = (*Feptr(F)).sub(number as usize);
                            }
                            // Save earliest consulted char, then skip opcode.
                            if *Feptr(F) < (*mb).start_used_ptr {
                                (*mb).start_used_ptr = *Feptr(F);
                            }
                            *Fecode(F) = (*Fecode(F)).add(1 + IMM2_SIZE_U);
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_VREVERSE (C 6233) ----
                        OP_VREVERSE => {
                            *Lvreverse_min(F) = GET2(*Fecode(F), 1);
                            *Lvreverse_max(F) = GET2(*Fecode(F), 1 + IMM2_SIZE_U);

                            // Move back by the maximum branch length, then work
                            // forwards.
                            if utf {
                                i = 0;
                                while i < *Lvreverse_max(F) {
                                    if *Feptr(F) == (*mb).start_subject {
                                        if i < *Lvreverse_min(F) {
                                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                        }
                                        *Lvreverse_max(F) = i;
                                        break;
                                    }
                                    *Feptr(F) = (*Feptr(F)).sub(1);
                                    let mut ep = *Feptr(F);
                                    BACKCHAR(&mut ep);
                                    *Feptr(F) = ep;
                                    i += 1;
                                }
                            } else {
                                let diff = (*Feptr(F)).offset_from((*mb).start_subject);
                                let available: u32 = if diff > 65535 {
                                    65535
                                } else if diff > 0 {
                                    diff as u32
                                } else {
                                    0
                                };
                                if *Lvreverse_min(F) > available {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                if *Lvreverse_max(F) > available {
                                    *Lvreverse_max(F) = available;
                                }
                                *Feptr(F) = (*Feptr(F)).sub(*Lvreverse_max(F) as usize);
                            }

                            // Try matching, moving forward one char on failure,
                            // until we reach the minimum back length.
                            { start_ecode = (*Fecode(F)).add(1 + 2 * IMM2_SIZE_U); *Freturn_id(F) = RM37; label = Lbl::MatchRecurse; continue 'sm; }
                        }

                        // ---- OP_ALT (C 6292) ----
                        OP_ALT => {
                            branch_end = *Fecode(F);
                            loop {
                                *Fecode(F) = (*Fecode(F)).add(GET(*Fecode(F), 1) as usize);
                                if *(*Fecode(F)) as u32 != OP_ALT {
                                    break;
                                }
                            }
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_KET / OP_KETRMIN / OP_KETRMAX / OP_KETRPOS (C 6304) ----
                        OP_KET | OP_KETRMIN | OP_KETRMAX | OP_KETRPOS => {
                            bracode = (*Fecode(F)).sub(GET(*Fecode(F), 1) as usize);

                            if branch_end.is_null() {
                                branch_end = *Fecode(F);
                            }
                            branch_start = bracode;
                            while (branch_start).add(GET(branch_start, 1) as usize) != branch_end {
                                branch_start = branch_start.add(GET(branch_start, 1) as usize);
                            }
                            branch_end = ptr::null();

                            // Point N to the start-of-group frame, P to its
                            // predecessor.
                            if *bracode as u32 != OP_BRA && *bracode as u32 != OP_COND {
                                N = ((*match_data).heapframes as *mut u8)
                                    .add(*Flast_group_offset(F))
                                    as *mut heapframe;
                                P = (N as *mut u8).sub(frame_size) as *mut heapframe;
                                *Flast_group_offset(F) = *Flast_group_offset(P);

                                // End of an assertion that is a condition.
                                if *Fgroup_frame_type(N) == GF_CONDASSERT {
                                    if (*bracode as u32 == OP_ASSERTBACK
                                        || *bracode as u32 == OP_ASSERTBACK_NOT)
                                        && *branch_start.add(1 + LINK_SIZE_U) as u32 == OP_VREVERSE
                                        && *Feptr(F) != *Feptr(P)
                                    {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                    ptr::copy_nonoverlapping(
                                        Fovector(F) as *const u8,
                                        (P as *mut u8).add(offset_of!(heapframe, ovector)),
                                        *Foffset_top(F) * core::mem::size_of::<PCRE2_SIZE>(),
                                    );
                                    *Foffset_top(P) = *Foffset_top(F);
                                    *Fmark(P) = *Fmark(F);
                                    *Fback_frame(F) = (F as *mut u8).offset_from(P as *mut u8)
                                        as PCRE2_SIZE;
                                    { rrc = MATCH_MATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                            } else {
                                P = ptr::null_mut(); // starting frame not recorded
                            }

                            // The group was not a conditional assertion.
                            match *bracode as u32 {
                                OP_BRA => {
                                    // Whole-pattern recursion end detection.
                                    if *Fcurrent_recurse(F) != 0
                                        || *(*Fecode(F)).add(1 + LINK_SIZE_U) as u32 != OP_END
                                    {
                                        // Nothing to be done.
                                    } else {
                                        // End of whole-pattern recursion.
                                        offset = *Flast_group_offset(F);
                                        debug_assert!(offset != PCRE2_UNSET);
                                        if offset == PCRE2_UNSET {
                                            return PCRE2_ERROR_INTERNAL as c_int;
                                        }
                                        N = ((*match_data).heapframes as *mut u8).add(offset)
                                            as *mut heapframe;
                                        P = (N as *mut u8).sub(frame_size) as *mut heapframe;
                                        *Flast_group_offset(F) = *Flast_group_offset(P);

                                        // Reinstate previous captures, carry on.
                                        *Fecode(F) = (*Fecode(P)).add(1 + LINK_SIZE_U);

                                        if *(*Fecode(F)) as u32 != OP_CREF {
                                            ptr::copy_nonoverlapping(
                                                Fovector(P) as *const PCRE2_SIZE,
                                                Fovector(F),
                                                *Foffset_top(F),
                                            );
                                            *Foffset_top(F) = *Foffset_top(P);
                                        } else {
                                            // recurse_update_offsets(F, P) (C 513)
                                            let mut dst: *mut PCRE2_SIZE = Fovector(F);
                                            let mut src: *const PCRE2_SIZE = Fovector(P);
                                            let mut roff: PCRE2_SIZE = 2;
                                            let offset_top: PCRE2_SIZE = *Foffset_top(F) + 2;
                                            let mut rdiff: PCRE2_SIZE;
                                            let mut ecode: PCRE2_SPTR = *Fecode(F);
                                            loop {
                                                rdiff = ((GET2(ecode, 1) << 1) as PCRE2_SIZE) - roff;
                                                ecode = ecode.add(1 + IMM2_SIZE_U);
                                                if roff + rdiff >= offset_top {
                                                    while *ecode as u32 == OP_CREF {
                                                        ecode = ecode.add(1 + IMM2_SIZE_U);
                                                    }
                                                    break;
                                                }
                                                if rdiff == 2 {
                                                    *dst.add(0) = *src.add(0);
                                                    *dst.add(1) = *src.add(1);
                                                } else if rdiff >= 4 {
                                                    ptr::copy_nonoverlapping(src, dst, rdiff);
                                                }
                                                rdiff += 2;
                                                roff += rdiff;
                                                dst = dst.add(rdiff);
                                                src = src.add(rdiff);
                                                if *ecode as u32 != OP_CREF {
                                                    break;
                                                }
                                            }
                                            rdiff = offset_top - roff;
                                            if rdiff == 2 {
                                                *dst.add(0) = *src.add(0);
                                                *dst.add(1) = *src.add(1);
                                            } else if rdiff >= 4 {
                                                ptr::copy_nonoverlapping(src, dst, rdiff);
                                            }
                                        }

                                        *Fcapture_last(F) = *Fcapture_last(P);
                                        *Fcurrent_recurse(F) = *Fcurrent_recurse(P);
                                        { label = Lbl::MainLoop; continue 'sm; } // continue with next opcode
                                    }
                                }
                                OP_COND | OP_SCOND => {
                                    // No need to do anything for these.
                                }
                                OP_ASSERTBACK_NA => {
                                    if *branch_start.add(1 + LINK_SIZE_U) as u32 == OP_VREVERSE
                                        && *Feptr(F) != *Feptr(P)
                                    {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                    // Fall through to OP_ASSERT_NA.
                                    if *Feptr(F) > (*mb).last_used_ptr {
                                        (*mb).last_used_ptr = *Feptr(F);
                                    }
                                    *Feptr(F) = *Feptr(P);
                                }
                                OP_ASSERT_NA => {
                                    if *Feptr(F) > (*mb).last_used_ptr {
                                        (*mb).last_used_ptr = *Feptr(F);
                                    }
                                    *Feptr(F) = *Feptr(P);
                                }
                                OP_ASSERTBACK => {
                                    if *branch_start.add(1 + LINK_SIZE_U) as u32 == OP_VREVERSE
                                        && *Feptr(F) != *Feptr(P)
                                    {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                    // Fall through to OP_ASSERT.
                                    if *Feptr(F) > (*mb).last_used_ptr {
                                        (*mb).last_used_ptr = *Feptr(F);
                                    }
                                    *Feptr(F) = *Feptr(P);
                                    // Fall through to OP_ONCE.
                                    *Fback_frame(F) =
                                        (F as *mut u8).offset_from(P as *mut u8) as PCRE2_SIZE;
                                    loop {
                                        let y = GET(*Fecode(P), 1);
                                        if *(*Fecode(P)).add(y as usize) as u32 != OP_ALT {
                                            break;
                                        }
                                        *Fecode(P) = (*Fecode(P)).add(y as usize);
                                    }
                                }
                                OP_ASSERT => {
                                    if *Feptr(F) > (*mb).last_used_ptr {
                                        (*mb).last_used_ptr = *Feptr(F);
                                    }
                                    *Feptr(F) = *Feptr(P);
                                    // Fall through to OP_ONCE.
                                    *Fback_frame(F) =
                                        (F as *mut u8).offset_from(P as *mut u8) as PCRE2_SIZE;
                                    loop {
                                        let y = GET(*Fecode(P), 1);
                                        if *(*Fecode(P)).add(y as usize) as u32 != OP_ALT {
                                            break;
                                        }
                                        *Fecode(P) = (*Fecode(P)).add(y as usize);
                                    }
                                }
                                OP_ONCE => {
                                    *Fback_frame(F) =
                                        (F as *mut u8).offset_from(P as *mut u8) as PCRE2_SIZE;
                                    loop {
                                        let y = GET(*Fecode(P), 1);
                                        if *(*Fecode(P)).add(y as usize) as u32 != OP_ALT {
                                            break;
                                        }
                                        *Fecode(P) = (*Fecode(P)).add(y as usize);
                                    }
                                }
                                OP_ASSERTBACK_NOT => {
                                    if *branch_start.add(1 + LINK_SIZE_U) as u32 == OP_VREVERSE
                                        && *Feptr(F) != *Feptr(P)
                                    {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                    // Fall through to OP_ASSERT_NOT.
                                    { rrc = MATCH_MATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                OP_ASSERT_NOT => {
                                    { rrc = MATCH_MATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                OP_ASSERT_SCS => {
                                    // Preserve current end_subject, restore
                                    // before backtracking into the subpattern.
                                    *Lsaved_end_subject(F) = (*mb).end_subject;
                                    (*mb).end_subject =
                                        *Lsaved_end_subject(P);
                                    (*mb).true_end_subject =
                                        (*mb).end_subject.add(*Ltrue_end_extra(P));
                                    *Feptr(F) = *Lsaved_eptr(P);

                                    { start_ecode = (*Fecode(F)).add(1 + LINK_SIZE_U); *Freturn_id(F) = RM39; label = Lbl::MatchRecurse; continue 'sm; }
                                }
                                OP_SCRIPT_RUN => {
                                    if crate::script_run::_pcre2_script_run_8(
                                        *Feptr(P),
                                        *Feptr(F),
                                        utf as BOOL,
                                    ) == FALSE
                                    {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                }
                                OP_CBRA | OP_CBRAPOS | OP_SCBRA | OP_SCBRAPOS => {
                                    number = GET2(bracode, 1 + LINK_SIZE_U);

                                    // A recursively called group.
                                    if *Fcurrent_recurse(F) == number {
                                        P = (N as *mut u8).sub(frame_size) as *mut heapframe;
                                        *Fecode(F) = (*Fecode(P)).add(1 + LINK_SIZE_U);

                                        if *(*Fecode(F)) as u32 != OP_CREF {
                                            ptr::copy_nonoverlapping(
                                                Fovector(P) as *const PCRE2_SIZE,
                                                Fovector(F),
                                                *Foffset_top(F),
                                            );
                                            *Foffset_top(F) = *Foffset_top(P);
                                        } else {
                                            // recurse_update_offsets(F, P) (C 513)
                                            let mut dst: *mut PCRE2_SIZE = Fovector(F);
                                            let mut src: *const PCRE2_SIZE = Fovector(P);
                                            let mut roff: PCRE2_SIZE = 2;
                                            let offset_top: PCRE2_SIZE = *Foffset_top(F) + 2;
                                            let mut rdiff: PCRE2_SIZE;
                                            let mut ecode: PCRE2_SPTR = *Fecode(F);
                                            loop {
                                                rdiff = ((GET2(ecode, 1) << 1) as PCRE2_SIZE) - roff;
                                                ecode = ecode.add(1 + IMM2_SIZE_U);
                                                if roff + rdiff >= offset_top {
                                                    while *ecode as u32 == OP_CREF {
                                                        ecode = ecode.add(1 + IMM2_SIZE_U);
                                                    }
                                                    break;
                                                }
                                                if rdiff == 2 {
                                                    *dst.add(0) = *src.add(0);
                                                    *dst.add(1) = *src.add(1);
                                                } else if rdiff >= 4 {
                                                    ptr::copy_nonoverlapping(src, dst, rdiff);
                                                }
                                                rdiff += 2;
                                                roff += rdiff;
                                                dst = dst.add(rdiff);
                                                src = src.add(rdiff);
                                                if *ecode as u32 != OP_CREF {
                                                    break;
                                                }
                                            }
                                            rdiff = offset_top - roff;
                                            if rdiff == 2 {
                                                *dst.add(0) = *src.add(0);
                                                *dst.add(1) = *src.add(1);
                                            } else if rdiff >= 4 {
                                                ptr::copy_nonoverlapping(src, dst, rdiff);
                                            }
                                        }

                                        *Fcapture_last(F) = *Fcapture_last(P);
                                        *Fcurrent_recurse(F) = *Fcurrent_recurse(P);
                                        { label = Lbl::MainLoop; continue 'sm; } // continue with next opcode
                                    }

                                    // Deal with actual capturing.
                                    offset = ((number << 1) - 2) as PCRE2_SIZE;
                                    *Fcapture_last(F) = number;
                                    *Fovector(F).add(offset) =
                                        (*Feptr(P)).offset_from((*mb).start_subject) as PCRE2_SIZE;
                                    *Fovector(F).add(offset + 1) =
                                        (*Feptr(F)).offset_from((*mb).start_subject) as PCRE2_SIZE;
                                    if offset >= *Foffset_top(F) {
                                        *Foffset_top(F) = offset + 2;
                                    }
                                }
                                _ => {}
                            }

                            // OP_KETRPOS: possessive repeating ket. Remember
                            // current position and return MATCH_KETRPOS.
                            if *(*Fecode(F)) as u32 == OP_KETRPOS {
                                ptr::copy_nonoverlapping(
                                    (F as *const u8).add(offset_of!(heapframe, eptr)),
                                    (P as *mut u8).add(offset_of!(heapframe, eptr)),
                                    frame_copy_size,
                                );
                                { rrc = MATCH_KETRPOS; label = Lbl::ReturnSwitch; continue 'sm; }
                            }

                            // Different kinds of closing brackets.
                            if Fop_val != OP_KET
                                && (P.is_null() || *Feptr(F) != *Feptr(P))
                            {
                                if Fop_val == OP_KETRMIN {
                                    { start_ecode = (*Fecode(F)).add(1 + LINK_SIZE_U); *Freturn_id(F) = RM6; label = Lbl::MatchRecurse; continue 'sm; }
                                }
                                // Repeat the maximum number of times (KETRMAX).
                                { start_ecode = bracode; *Freturn_id(F) = RM7; label = Lbl::MatchRecurse; continue 'sm; }
                            }

                            // Carry on at this level.
                            *Fecode(F) = (*Fecode(F)).add(1 + LINK_SIZE_U);
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_CIRC (C 6570) ----
                        OP_CIRC => {
                            if *Feptr(F) != (*mb).start_subject
                                || ((*mb).moptions & PCRE2_NOTBOL as u32) != 0
                            {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            *Fecode(F) = (*Fecode(F)).add(1);
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_SOD (C 6576) ----
                        OP_SOD => {
                            if *Feptr(F) != (*mb).start_subject {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            *Fecode(F) = (*Fecode(F)).add(1);
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_DOLL (C 6584) ----
                        OP_DOLL => {
                            if ((*mb).moptions & PCRE2_NOTEOL as u32) != 0 {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            if ((*mb).poptions & PCRE2_DOLLAR_ENDONLY as u32) == 0 {
                                label = Lbl::AssertNlOrEos;
                                continue 'sm;
                            }
                            // Fall through to OP_EOD (\z).
                            if *Feptr(F) < (*mb).true_end_subject {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            if (*mb).partial != 0 {
                                (*mb).hitend = TRUE;
                                if (*mb).partial > 1 {
                                    return PCRE2_ERROR_PARTIAL as c_int;
                                }
                            }
                            *Fecode(F) = (*Fecode(F)).add(1);
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_EOD (C 6591) ----
                        OP_EOD => {
                            if *Feptr(F) < (*mb).true_end_subject {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            if (*mb).partial != 0 {
                                (*mb).hitend = TRUE;
                                if (*mb).partial > 1 {
                                    return PCRE2_ERROR_PARTIAL as c_int;
                                }
                            }
                            *Fecode(F) = (*Fecode(F)).add(1);
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_EODN (C 6603) ----
                        OP_EODN => {
                            label = Lbl::AssertNlOrEos;
                            continue 'sm;
                        }

                        // ---- OP_CIRCM (C 6637) ----
                        OP_CIRCM => {
                            if ((*mb).moptions & PCRE2_NOTBOL as u32) != 0
                                && *Feptr(F) == (*mb).start_subject
                            {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            if *Feptr(F) != (*mb).start_subject
                                && ((*Feptr(F) == (*mb).end_subject
                                    && ((*mb).poptions & PCRE2_ALT_CIRCUMFLEX as u32) == 0)
                                    || !WAS_NEWLINE!(*Feptr(F)))
                            {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            *Fecode(F) = (*Fecode(F)).add(1);
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_DOLLM (C 6651) ----
                        OP_DOLLM => {
                            if *Feptr(F) < (*mb).end_subject {
                                if !IS_NEWLINE!(*Feptr(F)) {
                                    if (*mb).partial != 0
                                        && (*Feptr(F)).add(1) >= (*mb).end_subject
                                        && (*mb).nltype == NLTYPE_FIXED as u32
                                        && (*mb).nllen == 2
                                        && *(*Feptr(F)) as u32 == (*mb).nl[0] as u32
                                    {
                                        (*mb).hitend = TRUE;
                                        if (*mb).partial > 1 {
                                            return PCRE2_ERROR_PARTIAL as c_int;
                                        }
                                    }
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                            } else {
                                if ((*mb).moptions & PCRE2_NOTEOL as u32) != 0 {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                SCHECK_PARTIAL!();
                            }
                            *Fecode(F) = (*Fecode(F)).add(1);
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_SOM (C 6680) ----
                        OP_SOM => {
                            if *Feptr(F)
                                != (*mb).start_subject.add((*mb).start_offset)
                            {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            *Fecode(F) = (*Fecode(F)).add(1);
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_SET_SOM (C 6689) ----
                        OP_SET_SOM => {
                            *Fstart_match(F) = *Feptr(F);
                            *Fecode(F) = (*Fecode(F)).add(1);
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- Word-boundary opcodes (C 6702-6705) ----
                        OP_NOT_WORD_BOUNDARY | OP_WORD_BOUNDARY
                        | OP_NOT_UCP_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY => {
                            // Previous character.
                            if *Feptr(F) == (*mb).check_subject {
                                prev_is_word = FALSE;
                            } else {
                                let mut lastptr: PCRE2_SPTR = (*Feptr(F)).sub(1);
                                if utf {
                                    BACKCHAR(&mut lastptr);
                                    fc = GETCHAR(lastptr);
                                } else {
                                    fc = *lastptr as u32;
                                }
                                if lastptr < (*mb).start_used_ptr {
                                    (*mb).start_used_ptr = lastptr;
                                }
                                if Fop_val == OP_UCP_WORD_BOUNDARY
                                    || Fop_val == OP_NOT_UCP_WORD_BOUNDARY
                                {
                                    let chartype = UCD_CHARTYPE(fc);
                                    let category =
                                        crate::tables::_pcre2_ucp_gentype[chartype as usize];
                                    prev_is_word = (category == ucp_L as u32
                                        || category == ucp_N as u32
                                        || chartype == ucp_Mn as u32
                                        || chartype == ucp_Pc as u32)
                                        as BOOL;
                                } else {
                                    prev_is_word = (CHMAX_255(fc)
                                        && (*(*mb).ctypes.add(fc as usize) as u32
                                            & ctype_word as u32)
                                            != 0) as BOOL;
                                }
                            }

                            // Next character.
                            if *Feptr(F) >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                cur_is_word = FALSE;
                            } else {
                                let mut nextptr: PCRE2_SPTR = (*Feptr(F)).add(1);
                                if utf {
                                    FORWARDCHARTEST(&mut nextptr, (*mb).end_subject);
                                    fc = GETCHAR(*Feptr(F));
                                } else {
                                    fc = *(*Feptr(F)) as u32;
                                }
                                if nextptr > (*mb).last_used_ptr {
                                    (*mb).last_used_ptr = nextptr;
                                }
                                if Fop_val == OP_UCP_WORD_BOUNDARY
                                    || Fop_val == OP_NOT_UCP_WORD_BOUNDARY
                                {
                                    let chartype = UCD_CHARTYPE(fc);
                                    let category =
                                        crate::tables::_pcre2_ucp_gentype[chartype as usize];
                                    cur_is_word = (category == ucp_L as u32
                                        || category == ucp_N as u32
                                        || chartype == ucp_Mn as u32
                                        || chartype == ucp_Pc as u32)
                                        as BOOL;
                                } else {
                                    cur_is_word = (CHMAX_255(fc)
                                        && (*(*mb).ctypes.add(fc as usize) as u32
                                            & ctype_word as u32)
                                            != 0) as BOOL;
                                }
                            }

                            // See if the situation is what we want.
                            let this_op = *(*Fecode(F)) as u32;
                            *Fecode(F) = (*Fecode(F)).add(1);
                            let want_equal =
                                this_op == OP_WORD_BOUNDARY || Fop_val == OP_UCP_WORD_BOUNDARY;
                            let reject = if want_equal {
                                cur_is_word == prev_is_word
                            } else {
                                cur_is_word != prev_is_word
                            };
                            if reject {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        // ---- OP_MARK (C 6777) ----
                        OP_MARK => {
                            *Fmark(F) = (*Fecode(F)).add(2);
                            (*mb).nomatch_mark = (*Fecode(F)).add(2);
                            { start_ecode = (*Fecode(F)).add(
                                    crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize]
                                        as usize
                                        + *(*Fecode(F)).add(1) as usize
                                ); *Freturn_id(F) = RM12; label = Lbl::MatchRecurse; continue 'sm; }
                        }

                        // ---- OP_FAIL (C 6796) ----
                        OP_FAIL => {
                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                        }

                        // ---- OP_COMMIT (C 6803) ----
                        OP_COMMIT => {
                            { start_ecode = (*Fecode(F)).add(
                                    crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize]
                                        as usize
                                ); *Freturn_id(F) = RM13; label = Lbl::MatchRecurse; continue 'sm; }
                        }

                        // ---- OP_COMMIT_ARG (C 6809) ----
                        OP_COMMIT_ARG => {
                            *Fmark(F) = (*Fecode(F)).add(2);
                            (*mb).nomatch_mark = (*Fecode(F)).add(2);
                            { start_ecode = (*Fecode(F)).add(
                                    crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize]
                                        as usize
                                        + *(*Fecode(F)).add(1) as usize
                                ); *Freturn_id(F) = RM36; label = Lbl::MatchRecurse; continue 'sm; }
                        }

                        // ---- OP_PRUNE (C 6816) ----
                        OP_PRUNE => {
                            { start_ecode = (*Fecode(F)).add(
                                    crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize]
                                        as usize
                                ); *Freturn_id(F) = RM14; label = Lbl::MatchRecurse; continue 'sm; }
                        }

                        // ---- OP_PRUNE_ARG (C 6822) ----
                        OP_PRUNE_ARG => {
                            *Fmark(F) = (*Fecode(F)).add(2);
                            (*mb).nomatch_mark = (*Fecode(F)).add(2);
                            { start_ecode = (*Fecode(F)).add(
                                    crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize]
                                        as usize
                                        + *(*Fecode(F)).add(1) as usize
                                ); *Freturn_id(F) = RM15; label = Lbl::MatchRecurse; continue 'sm; }
                        }

                        // ---- OP_SKIP (C 6829) ----
                        OP_SKIP => {
                            { start_ecode = (*Fecode(F)).add(
                                    crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize]
                                        as usize
                                ); *Freturn_id(F) = RM16; label = Lbl::MatchRecurse; continue 'sm; }
                        }

                        // ---- OP_SKIP_ARG (C 6844) ----
                        OP_SKIP_ARG => {
                            (*mb).skip_arg_count += 1;
                            if (*mb).skip_arg_count <= (*mb).ignore_skip_arg {
                                *Fecode(F) = (*Fecode(F)).add(
                                    crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize]
                                        as usize
                                        + *(*Fecode(F)).add(1) as usize,
                                );
                                { label = Lbl::MainLoop; continue 'sm; }
                            }
                            { start_ecode = (*Fecode(F)).add(
                                    crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize]
                                        as usize
                                        + *(*Fecode(F)).add(1) as usize
                                ); *Freturn_id(F) = RM17; label = Lbl::MatchRecurse; continue 'sm; }
                        }

                        // ---- OP_THEN (C 6866) ----
                        OP_THEN => {
                            { start_ecode = (*Fecode(F)).add(
                                    crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize]
                                        as usize
                                ); *Freturn_id(F) = RM18; label = Lbl::MatchRecurse; continue 'sm; }
                        }

                        // ---- OP_THEN_ARG (C 6873) ----
                        OP_THEN_ARG => {
                            *Fmark(F) = (*Fecode(F)).add(2);
                            (*mb).nomatch_mark = (*Fecode(F)).add(2);
                            { start_ecode = (*Fecode(F)).add(
                                    crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize]
                                        as usize
                                        + *(*Fecode(F)).add(1) as usize
                                ); *Freturn_id(F) = RM19; label = Lbl::MatchRecurse; continue 'sm; }
                        }

                        // ---- default (C 6893): internal error ----
                        _ => {
                            return PCRE2_ERROR_INTERNAL as c_int;
                        }
                    }
                }

                // ---- REPEATCHAR (C 1392) ----
                Lbl::RepeatChar => {
                    if utf {
                        let mut length: u32 = 1;
                        *Lcharptr(F) = *Fecode(F);
                        fc = GETCHARLEN(*Fecode(F), &mut length);
                        *Fecode(F) = (*Fecode(F)).add(length as usize);
                        *Fbyte1(F) = length as u8;

                        // Handle multi-code-unit character matching.
                        if length > 1 {
                            let othercase: u32;
                            if (*Fop(F) as u32) >= OP_STARI && {
                                othercase = UCD_OTHERCASE(fc);
                                othercase != fc
                            } {
                                *Fbyte2(F) =
                                    crate::ord2utf::_pcre2_ord2utf_8(othercase, Loccu(F)) as u8;
                            } else {
                                *Fbyte2(F) = 0;
                            }

                            i = 1;
                            while i <= *Lchar_min(F) {
                                let llength = *Fbyte1(F) as usize;
                                let loclength = *Fbyte2(F) as usize;
                                if *Feptr(F) <= (*mb).end_subject.sub(llength)
                                    && c_memcmp(*Feptr(F) as *const core::ffi::c_void, *Lcharptr(F) as *const core::ffi::c_void, llength) == 0
                                {
                                    *Feptr(F) = (*Feptr(F)).add(llength);
                                } else if loclength > 0
                                    && *Feptr(F) <= (*mb).end_subject.sub(loclength)
                                    && c_memcmp(*Feptr(F) as *const core::ffi::c_void, Loccu(F) as *const core::ffi::c_void, loclength) == 0
                                {
                                    *Feptr(F) = (*Feptr(F)).add(loclength);
                                } else {
                                    CHECK_PARTIAL!();
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                i += 1;
                            }

                            if *Lchar_min(F) == *Lchar_max(F) {
                                { label = Lbl::MainLoop; continue 'sm; }
                            }

                            if reptype == REPTYPE_MIN {
                                { start_ecode = *Fecode(F); *Freturn_id(F) = RM202; label = Lbl::MatchRecurse; continue 'sm; }
                            } else {
                                // Maximize.
                                *Lchar_start_eptr(F) = *Feptr(F);
                                i = *Lchar_min(F);
                                while i < *Lchar_max(F) {
                                    let llength = *Fbyte1(F) as usize;
                                    let loclength = *Fbyte2(F) as usize;
                                    if *Feptr(F) <= (*mb).end_subject.sub(llength)
                                        && c_memcmp(*Feptr(F) as *const core::ffi::c_void, *Lcharptr(F) as *const core::ffi::c_void, llength) == 0
                                    {
                                        *Feptr(F) = (*Feptr(F)).add(llength);
                                    } else if loclength > 0
                                        && *Feptr(F) <= (*mb).end_subject.sub(loclength)
                                        && c_memcmp(*Feptr(F) as *const core::ffi::c_void, Loccu(F) as *const core::ffi::c_void, loclength) == 0
                                    {
                                        *Feptr(F) = (*Feptr(F)).add(loclength);
                                    } else {
                                        CHECK_PARTIAL!();
                                        break;
                                    }
                                    i += 1;
                                }

                                if reptype != REPTYPE_POS {
                                    if *Feptr(F) <= *Lchar_start_eptr(F) {
                                        { label = Lbl::MainLoop; continue 'sm; }
                                    }
                                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM203; label = Lbl::MatchRecurse; continue 'sm; }
                                }
                                { label = Lbl::MainLoop; continue 'sm; }
                            }
                        }

                        // Length of UTF character is 1; preserve it and fall
                        // through to the non-UTF code.
                        *Lchar_c(F) = fc;
                    } else {
                        *Lchar_c(F) = *(*Fecode(F)) as u32;
                        *Fecode(F) = (*Fecode(F)).add(1);
                    }

                    // Caseless comparison.
                    if (*Fop(F) as u32) >= OP_STARI {
                        if ucp && !utf && *Lchar_c(F) > 127 {
                            *Lchar_oc(F) = UCD_OTHERCASE(*Lchar_c(F));
                        } else {
                            *Lchar_oc(F) = *(*mb).fcc.add(*Lchar_c(F) as usize) as u32;
                        }

                        i = 1;
                        while i <= *Lchar_min(F) {
                            if *Feptr(F) >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            let cc = *(*Feptr(F)) as u32;
                            if *Lchar_c(F) != cc && *Lchar_oc(F) != cc {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            *Feptr(F) = (*Feptr(F)).add(1);
                            i += 1;
                        }
                        if *Lchar_min(F) == *Lchar_max(F) {
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        if reptype == REPTYPE_MIN {
                            { start_ecode = *Fecode(F); *Freturn_id(F) = RM25; label = Lbl::MatchRecurse; continue 'sm; }
                        } else {
                            // Maximize.
                            *Lchar_start_eptr(F) = *Feptr(F);
                            i = *Lchar_min(F);
                            while i < *Lchar_max(F) {
                                if *Feptr(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                let cc = *(*Feptr(F)) as u32;
                                if *Lchar_c(F) != cc && *Lchar_oc(F) != cc {
                                    break;
                                }
                                *Feptr(F) = (*Feptr(F)).add(1);
                                i += 1;
                            }
                            if reptype != REPTYPE_POS {
                                if *Feptr(F) == *Lchar_start_eptr(F) {
                                    { label = Lbl::MainLoop; continue 'sm; }
                                }
                                { start_ecode = *Fecode(F); *Freturn_id(F) = RM26; label = Lbl::MatchRecurse; continue 'sm; }
                            }
                            { label = Lbl::MainLoop; continue 'sm; }
                        }
                    }
                    // Caseful comparisons (includes all multi-byte characters).
                    else {
                        i = 1;
                        while i <= *Lchar_min(F) {
                            if *Feptr(F) >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            let cc = *(*Feptr(F)) as u32;
                            *Feptr(F) = (*Feptr(F)).add(1);
                            if *Lchar_c(F) != cc {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            i += 1;
                        }

                        if *Lchar_min(F) == *Lchar_max(F) {
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        if reptype == REPTYPE_MIN {
                            { start_ecode = *Fecode(F); *Freturn_id(F) = RM27; label = Lbl::MatchRecurse; continue 'sm; }
                        } else {
                            // Maximize.
                            *Lchar_start_eptr(F) = *Feptr(F);
                            i = *Lchar_min(F);
                            while i < *Lchar_max(F) {
                                if *Feptr(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    break;
                                }
                                if *Lchar_c(F) != *(*Feptr(F)) as u32 {
                                    break;
                                }
                                *Feptr(F) = (*Feptr(F)).add(1);
                                i += 1;
                            }

                            if reptype != REPTYPE_POS {
                                if *Feptr(F) <= *Lchar_start_eptr(F) {
                                    { label = Lbl::MainLoop; continue 'sm; }
                                }
                                { start_ecode = *Fecode(F); *Freturn_id(F) = RM28; label = Lbl::MatchRecurse; continue 'sm; }
                            }
                            { label = Lbl::MainLoop; continue 'sm; }
                        }
                    }
                }

                // ---- after RMATCH(Fecode, RM202) at C 1434 (REPEATCHAR utf min) ----
                Lbl::L_RM202 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let old = *Lchar_min(F);
                    *Lchar_min(F) = old + 1;
                    if old >= *Lchar_max(F) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let llength = *Fbyte1(F) as usize;
                    let loclength = *Fbyte2(F) as usize;
                    if *Feptr(F) <= (*mb).end_subject.sub(llength)
                        && c_memcmp(*Feptr(F) as *const core::ffi::c_void, *Lcharptr(F) as *const core::ffi::c_void, llength) == 0
                    {
                        *Feptr(F) = (*Feptr(F)).add(llength);
                    } else if loclength > 0
                        && *Feptr(F) <= (*mb).end_subject.sub(loclength)
                        && c_memcmp(*Feptr(F) as *const core::ffi::c_void, Loccu(F) as *const core::ffi::c_void, loclength) == 0
                    {
                        *Feptr(F) = (*Feptr(F)).add(loclength);
                    } else {
                        CHECK_PARTIAL!();
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM202; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM203) at C 1477 (REPEATCHAR utf max) ----
                Lbl::L_RM203 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    *Feptr(F) = (*Feptr(F)).sub(1);
                    BACKCHAR(&mut *Feptr(F));
                    if *Feptr(F) <= *Lchar_start_eptr(F) {
                        { label = Lbl::MainLoop; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM203; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM25) at C 1537 (REPEATCHAR caseless min) ----
                Lbl::L_RM25 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let old = *Lchar_min(F);
                    *Lchar_min(F) = old + 1;
                    if old >= *Lchar_max(F) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    if *Feptr(F) >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let cc = *(*Feptr(F)) as u32;
                    if *Lchar_c(F) != cc && *Lchar_oc(F) != cc {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    *Feptr(F) = (*Feptr(F)).add(1);
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM25; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM26) at C 1570 (REPEATCHAR caseless max) ----
                Lbl::L_RM26 => {
                    *Feptr(F) = (*Feptr(F)).sub(1);
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    if *Feptr(F) == *Lchar_start_eptr(F) {
                        { label = Lbl::MainLoop; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM26; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM27) at C 1597 (REPEATCHAR caseful min) ----
                Lbl::L_RM27 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let old = *Lchar_min(F);
                    *Lchar_min(F) = old + 1;
                    if old >= *Lchar_max(F) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    if *Feptr(F) >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let cc = *(*Feptr(F)) as u32;
                    *Feptr(F) = (*Feptr(F)).add(1);
                    if *Lchar_c(F) != cc {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM27; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM28) at C 1627 (REPEATCHAR caseful max) ----
                Lbl::L_RM28 => {
                    *Feptr(F) = (*Feptr(F)).sub(1);
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    if *Feptr(F) <= *Lchar_start_eptr(F) {
                        { label = Lbl::MainLoop; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM28; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- REPEATNOTCHAR (C 1733) ----
                Lbl::RepeatNotChar => {
                    {
                        let mut ep = *Fecode(F);
                        *Lcharnot_c(F) = GETCHARINCTEST(&mut ep, utf);
                        *Fecode(F) = ep;
                    }

                    if (*Fop(F) as u32) >= OP_NOTSTARI {
                        // Caseless.
                        if (utf || ucp) && *Lcharnot_c(F) > 127 {
                            *Lcharnot_oc(F) = UCD_OTHERCASE(*Lcharnot_c(F));
                        } else {
                            *Lcharnot_oc(F) =
                                TABLE_GET(*Lcharnot_c(F), (*mb).fcc, *Lcharnot_c(F));
                        }

                        if utf {
                            i = 1;
                            while i <= *Lcharnot_min(F) {
                                if *Feptr(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                let mut ep = *Feptr(F);
                                let d = GETCHARINC(&mut ep);
                                *Feptr(F) = ep;
                                if *Lcharnot_c(F) == d || *Lcharnot_oc(F) == d {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                i += 1;
                            }
                        } else {
                            i = 1;
                            while i <= *Lcharnot_min(F) {
                                if *Feptr(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                let d = *(*Feptr(F)) as u32;
                                if *Lcharnot_c(F) == d || *Lcharnot_oc(F) == d {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                *Feptr(F) = (*Feptr(F)).add(1);
                                i += 1;
                            }
                        }

                        if *Lcharnot_min(F) == *Lcharnot_max(F) {
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        if reptype == REPTYPE_MIN {
                            if utf {
                                { start_ecode = *Fecode(F); *Freturn_id(F) = RM204; label = Lbl::MatchRecurse; continue 'sm; }
                            } else {
                                { start_ecode = *Fecode(F); *Freturn_id(F) = RM29; label = Lbl::MatchRecurse; continue 'sm; }
                            }
                        } else {
                            // Maximize.
                            *Lcharnot_start_eptr(F) = *Feptr(F);
                            if utf {
                                i = *Lcharnot_min(F);
                                while i < *Lcharnot_max(F) {
                                    let mut len: u32 = 1;
                                    if *Feptr(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        break;
                                    }
                                    let d = GETCHARLEN(*Feptr(F), &mut len);
                                    if *Lcharnot_c(F) == d || *Lcharnot_oc(F) == d {
                                        break;
                                    }
                                    *Feptr(F) = (*Feptr(F)).add(len as usize);
                                    i += 1;
                                }
                                if reptype != REPTYPE_POS {
                                    if *Feptr(F) <= *Lcharnot_start_eptr(F) {
                                        { label = Lbl::MainLoop; continue 'sm; }
                                    }
                                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM205; label = Lbl::MatchRecurse; continue 'sm; }
                                }
                                { label = Lbl::MainLoop; continue 'sm; }
                            } else {
                                i = *Lcharnot_min(F);
                                while i < *Lcharnot_max(F) {
                                    if *Feptr(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        break;
                                    }
                                    let d = *(*Feptr(F)) as u32;
                                    if *Lcharnot_c(F) == d || *Lcharnot_oc(F) == d {
                                        break;
                                    }
                                    *Feptr(F) = (*Feptr(F)).add(1);
                                    i += 1;
                                }
                                if reptype != REPTYPE_POS {
                                    if *Feptr(F) == *Lcharnot_start_eptr(F) {
                                        { label = Lbl::MainLoop; continue 'sm; }
                                    }
                                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM30; label = Lbl::MatchRecurse; continue 'sm; }
                                }
                                { label = Lbl::MainLoop; continue 'sm; }
                            }
                        }
                    }
                    // Caseful comparisons.
                    else {
                        if utf {
                            i = 1;
                            while i <= *Lcharnot_min(F) {
                                if *Feptr(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                let mut ep = *Feptr(F);
                                let d = GETCHARINC(&mut ep);
                                *Feptr(F) = ep;
                                if *Lcharnot_c(F) == d {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                i += 1;
                            }
                        } else {
                            i = 1;
                            while i <= *Lcharnot_min(F) {
                                if *Feptr(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                let d = *(*Feptr(F)) as u32;
                                *Feptr(F) = (*Feptr(F)).add(1);
                                if *Lcharnot_c(F) == d {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                i += 1;
                            }
                        }

                        if *Lcharnot_min(F) == *Lcharnot_max(F) {
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        if reptype == REPTYPE_MIN {
                            if utf {
                                { start_ecode = *Fecode(F); *Freturn_id(F) = RM206; label = Lbl::MatchRecurse; continue 'sm; }
                            } else {
                                { start_ecode = *Fecode(F); *Freturn_id(F) = RM31; label = Lbl::MatchRecurse; continue 'sm; }
                            }
                        } else {
                            // Maximize.
                            *Lcharnot_start_eptr(F) = *Feptr(F);
                            if utf {
                                i = *Lcharnot_min(F);
                                while i < *Lcharnot_max(F) {
                                    let mut len: u32 = 1;
                                    if *Feptr(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        break;
                                    }
                                    let d = GETCHARLEN(*Feptr(F), &mut len);
                                    if *Lcharnot_c(F) == d {
                                        break;
                                    }
                                    *Feptr(F) = (*Feptr(F)).add(len as usize);
                                    i += 1;
                                }
                                if reptype != REPTYPE_POS {
                                    if *Feptr(F) <= *Lcharnot_start_eptr(F) {
                                        { label = Lbl::MainLoop; continue 'sm; }
                                    }
                                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM207; label = Lbl::MatchRecurse; continue 'sm; }
                                }
                                { label = Lbl::MainLoop; continue 'sm; }
                            } else {
                                i = *Lcharnot_min(F);
                                while i < *Lcharnot_max(F) {
                                    if *Feptr(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        break;
                                    }
                                    if *Lcharnot_c(F) == *(*Feptr(F)) as u32 {
                                        break;
                                    }
                                    *Feptr(F) = (*Feptr(F)).add(1);
                                    i += 1;
                                }
                                if reptype != REPTYPE_POS {
                                    if *Feptr(F) == *Lcharnot_start_eptr(F) {
                                        { label = Lbl::MainLoop; continue 'sm; }
                                    }
                                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM32; label = Lbl::MatchRecurse; continue 'sm; }
                                }
                                { label = Lbl::MainLoop; continue 'sm; }
                            }
                        }
                    }
                }

                // ---- after RMATCH(Fecode, RM204) at C 1796 (REPEATNOTCHAR caseless utf min) ----
                Lbl::L_RM204 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let old = *Lcharnot_min(F);
                    *Lcharnot_min(F) = old + 1;
                    if old >= *Lcharnot_max(F) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    if *Feptr(F) >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let mut ep = *Feptr(F);
                    let d = GETCHARINC(&mut ep);
                    *Feptr(F) = ep;
                    if *Lcharnot_c(F) == d || *Lcharnot_oc(F) == d {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM204; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM29) at C 1815 (REPEATNOTCHAR caseless non-utf min) ----
                Lbl::L_RM29 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let old = *Lcharnot_min(F);
                    *Lcharnot_min(F) = old + 1;
                    if old >= *Lcharnot_max(F) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    if *Feptr(F) >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let d = *(*Feptr(F)) as u32;
                    if *Lcharnot_c(F) == d || *Lcharnot_oc(F) == d {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    *Feptr(F) = (*Feptr(F)).add(1);
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM29; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM205) at C 1860 (REPEATNOTCHAR caseless utf max) ----
                Lbl::L_RM205 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    *Feptr(F) = (*Feptr(F)).sub(1);
                    BACKCHAR(&mut *Feptr(F));
                    if *Feptr(F) <= *Lcharnot_start_eptr(F) {
                        { label = Lbl::MainLoop; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM205; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM30) at C 1884 (REPEATNOTCHAR caseless non-utf max) ----
                Lbl::L_RM30 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    *Feptr(F) = (*Feptr(F)).sub(1);
                    if *Feptr(F) == *Lcharnot_start_eptr(F) {
                        { label = Lbl::MainLoop; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM30; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM206) at C 1936 (REPEATNOTCHAR caseful utf min) ----
                Lbl::L_RM206 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let old = *Lcharnot_min(F);
                    *Lcharnot_min(F) = old + 1;
                    if old >= *Lcharnot_max(F) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    if *Feptr(F) >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let mut ep = *Feptr(F);
                    let d = GETCHARINC(&mut ep);
                    *Feptr(F) = ep;
                    if *Lcharnot_c(F) == d {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM206; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM31) at C 1954 (REPEATNOTCHAR caseful non-utf min) ----
                Lbl::L_RM31 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let old = *Lcharnot_min(F);
                    *Lcharnot_min(F) = old + 1;
                    if old >= *Lcharnot_max(F) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    if *Feptr(F) >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let d = *(*Feptr(F)) as u32;
                    *Feptr(F) = (*Feptr(F)).add(1);
                    if *Lcharnot_c(F) == d {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM31; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM207) at C 1998 (REPEATNOTCHAR caseful utf max) ----
                Lbl::L_RM207 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    *Feptr(F) = (*Feptr(F)).sub(1);
                    BACKCHAR(&mut *Feptr(F));
                    if *Feptr(F) <= *Lcharnot_start_eptr(F) {
                        { label = Lbl::MainLoop; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM207; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM32) at C 2021 (REPEATNOTCHAR caseful non-utf max) ----
                Lbl::L_RM32 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    *Feptr(F) = (*Feptr(F)).sub(1);
                    if *Feptr(F) == *Lcharnot_start_eptr(F) {
                        { label = Lbl::MainLoop; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM32; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM200) at C 2150 (OP_CLASS/OP_NCLASS utf min) ----
                Lbl::L_RM200 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let old = *Lclass_min(F);
                    *Lclass_min(F) = old + 1;
                    if old >= *Lclass_max(F) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    if *Feptr(F) >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let byte_map = *Lbyte_map_address(F);
                    let mut ep = *Feptr(F);
                    fc = GETCHARINC(&mut ep);
                    *Feptr(F) = ep;
                    if fc > 255 {
                        if (*Fop(F) as u32) == OP_CLASS {
                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                        }
                    } else if (*byte_map.add((fc / 8) as usize) & (1u8 << (fc & 7))) == 0 {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM200; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM23) at C 2173 (OP_CLASS/OP_NCLASS non-utf min) ----
                Lbl::L_RM23 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let old = *Lclass_min(F);
                    *Lclass_min(F) = old + 1;
                    if old >= *Lclass_max(F) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    if *Feptr(F) >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let byte_map = *Lbyte_map_address(F);
                    fc = *(*Feptr(F)) as u32;
                    *Feptr(F) = (*Feptr(F)).add(1);
                    if (*byte_map.add((fc / 8) as usize) & (1u8 << (fc & 7))) == 0 {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM23; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM201) at C 2230 (OP_CLASS/OP_NCLASS utf max) ----
                Lbl::L_RM201 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let at_start = *Feptr(F) <= *Lclass_start_eptr(F);
                    *Feptr(F) = (*Feptr(F)).sub(1);
                    if at_start {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    BACKCHAR(&mut *Feptr(F));
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM201; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM24) at C 2263 (OP_CLASS/OP_NCLASS non-utf max) ----
                Lbl::L_RM24 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    *Feptr(F) = (*Feptr(F)).sub(1);
                    if *Feptr(F) >= *Lclass_start_eptr(F) {
                        { start_ecode = *Fecode(F); *Freturn_id(F) = RM24; label = Lbl::MatchRecurse; continue 'sm; }
                    }
                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM100) at C 2357 (OP_XCLASS min) ----
                Lbl::L_RM100 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let old = *Lxclass_min(F);
                    *Lxclass_min(F) = old + 1;
                    if old >= *Lxclass_max(F) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    if *Feptr(F) >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let mut ep = *Feptr(F);
                    fc = GETCHARINCTEST(&mut ep, utf);
                    *Feptr(F) = ep;
                    if crate::xclass::_pcre2_xclass_8(
                        fc, *Lxclass_data(F),
                        (*mb).start_code as *const u8, utf as BOOL,
                    ) == FALSE
                    {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM100; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM101) at C 2404 (OP_XCLASS max) ----
                Lbl::L_RM101 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let at_start = *Feptr(F) <= *Lxclass_start_eptr(F);
                    *Feptr(F) = (*Feptr(F)).sub(1);
                    if at_start {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    if utf {
                        BACKCHAR(&mut *Feptr(F));
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM101; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM102) at C 2500 (OP_ECLASS min) ----
                Lbl::L_RM102 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let old = *Leclass_min(F);
                    *Leclass_min(F) = old + 1;
                    if old >= *Leclass_max(F) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    if *Feptr(F) >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let mut ep = *Feptr(F);
                    fc = GETCHARINCTEST(&mut ep, utf);
                    *Feptr(F) = ep;
                    if crate::xclass::_pcre2_eclass_8(
                        fc, *Leclass_data(F),
                        (*Leclass_data(F)).add(*Leclass_len(F)),
                        (*mb).start_code as *const u8, utf as BOOL,
                    ) == FALSE
                    {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM102; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM103) at C 2548 (OP_ECLASS max) ----
                Lbl::L_RM103 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let at_start = *Feptr(F) <= *Leclass_start_eptr(F);
                    *Feptr(F) = (*Feptr(F)).sub(1);
                    if at_start {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    if utf {
                        BACKCHAR(&mut *Feptr(F));
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM103; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- REPEATTYPE (C 2973) ----
                // Shared entry for all OP_TYPE* repeat opcodes. On entry
                // `Ltype_min`/`Ltype_max` are set, `reptype` is set (for the
                // *EXACT case reptype is left as-is but Lmin == Lmax so it is
                // never used), and `Fecode` points at the character-type byte.
                Lbl::RepeatType => {
                    // Lctype = *Fecode++;  (code for the character type)
                    *Lctype(F) = *(*Fecode(F)) as u32;
                    *Fecode(F) = (*Fecode(F)).add(1);

                    // Property tests carry an extra proptype + propvalue.
                    if *Lctype(F) == OP_PROP || *Lctype(F) == OP_NOTPROP {
                        proptype = *(*Fecode(F)) as c_int;
                        *Fecode(F) = (*Fecode(F)).add(1);
                        *Lpropvalue(F) = *(*Fecode(F)) as u32;
                        *Fecode(F) = (*Fecode(F)).add(1);
                    } else {
                        proptype = -1;
                    }

                    // -------------------------------------------------------
                    // First, ensure the minimum number of matches are present.
                    // No RMATCH in these loops, so "notmatch" is a local.
                    // -------------------------------------------------------
                    if *Ltype_min(F) > 0 {
                        let lctype = *Lctype(F);
                        let propvalue = *Lpropvalue(F);
                        if proptype >= 0 {
                            // Property tests in all modes.
                            i = 1;
                            while i <= *Ltype_min(F) {
                                if *Feptr(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                let mut ep = *Feptr(F);
                                fc = GETCHARINCTEST(&mut ep, utf);
                                *Feptr(F) = ep;
                                if rt_prop_reject(fc, proptype, propvalue, lctype) {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                                i += 1;
                            }
                        } else if lctype == OP_EXTUNI {
                            // Match extended Unicode sequences.
                            i = 1;
                            while i <= *Ltype_min(F) {
                                if *Feptr(F) >= (*mb).end_subject {
                                    SCHECK_PARTIAL!();
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                } else {
                                    let mut ep = *Feptr(F);
                                    fc = GETCHARINCTEST(&mut ep, utf);
                                    *Feptr(F) = ep;
                                    *Feptr(F) = crate::extuni::_pcre2_extuni_8(
                                        fc, *Feptr(F), (*mb).start_subject,
                                        (*mb).end_subject, utf as BOOL, ptr::null_mut(),
                                    );
                                }
                                CHECK_PARTIAL!();
                                i += 1;
                            }
                        } else if utf {
                            // Handle all other cases in UTF mode.
                            match lctype {
                                OP_ANY => {
                                    i = 1;
                                    while i <= *Ltype_min(F) {
                                        if *Feptr(F) >= (*mb).end_subject {
                                            SCHECK_PARTIAL!();
                                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                        }
                                        if IS_NEWLINE!(*Feptr(F)) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                                        if (*mb).partial != 0
                                            && (*Feptr(F)).add(1) >= (*mb).end_subject
                                            && (*mb).nltype == NLTYPE_FIXED as u32
                                            && (*mb).nllen == 2
                                            && *(*Feptr(F)) as u32 == (*mb).nl[0] as u32
                                        {
                                            (*mb).hitend = TRUE;
                                            if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL as c_int; }
                                        }
                                        *Feptr(F) = (*Feptr(F)).add(1);
                                        ACROSSCHAR!(*Feptr(F) < (*mb).end_subject, *Feptr(F));
                                        i += 1;
                                    }
                                }
                                OP_ALLANY => {
                                    i = 1;
                                    while i <= *Ltype_min(F) {
                                        if *Feptr(F) >= (*mb).end_subject {
                                            SCHECK_PARTIAL!();
                                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                        }
                                        *Feptr(F) = (*Feptr(F)).add(1);
                                        ACROSSCHAR!(*Feptr(F) < (*mb).end_subject, *Feptr(F));
                                        i += 1;
                                    }
                                }
                                OP_ANYBYTE => {
                                    if *Feptr(F) > (*mb).end_subject.sub(*Ltype_min(F) as usize) {
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                    *Feptr(F) = (*Feptr(F)).add(*Ltype_min(F) as usize);
                                }
                                OP_ANYNL => {
                                    i = 1;
                                    while i <= *Ltype_min(F) {
                                        if *Feptr(F) >= (*mb).end_subject {
                                            SCHECK_PARTIAL!();
                                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                        }
                                        let mut ep = *Feptr(F);
                                        fc = GETCHARINC(&mut ep);
                                        *Feptr(F) = ep;
                                        if fc == CHAR_CR {
                                            if *Feptr(F) < (*mb).end_subject
                                                && *(*Feptr(F)) as u32 == CHAR_NL {
                                                *Feptr(F) = (*Feptr(F)).add(1);
                                            }
                                        } else if fc == CHAR_NL {
                                            // ok
                                        } else if fc == 0x0b || fc == 0x0c || fc == 0x85
                                            || fc == 0x2028 || fc == 0x2029 {
                                            if (*mb).bsr_convention == PCRE2_BSR_ANYCRLF as u16 {
                                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                            }
                                        } else {
                                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                        }
                                        i += 1;
                                    }
                                }
                                // C 3336-3403: these four decode the whole
                                // character with GETCHARINC before testing.
                                OP_NOT_HSPACE | OP_HSPACE | OP_NOT_VSPACE | OP_VSPACE => {
                                    i = 1;
                                    while i <= *Ltype_min(F) {
                                        if *Feptr(F) >= (*mb).end_subject {
                                            SCHECK_PARTIAL!();
                                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                        }
                                        let mut ep = *Feptr(F);
                                        fc = GETCHARINC(&mut ep);
                                        *Feptr(F) = ep;
                                        let reject = match lctype {
                                            OP_NOT_HSPACE => is_hspace(fc),
                                            OP_HSPACE => !is_hspace(fc),
                                            OP_NOT_VSPACE => is_vspace(fc),
                                            _ => !is_vspace(fc), // OP_VSPACE
                                        };
                                        if reject {
                                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                        }
                                        i += 1;
                                    }
                                }

                                // C 3404: GETCHARINC, then reject ASCII digits.
                                OP_NOT_DIGIT => {
                                    i = 1;
                                    while i <= *Ltype_min(F) {
                                        if *Feptr(F) >= (*mb).end_subject {
                                            SCHECK_PARTIAL!();
                                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                        }
                                        let mut ep = *Feptr(F);
                                        fc = GETCHARINC(&mut ep);
                                        *Feptr(F) = ep;
                                        if fc < 128
                                            && (*(*mb).ctypes.add(fc as usize) as u32
                                                & ctype_digit as u32)
                                                != 0
                                        {
                                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                        }
                                        i += 1;
                                    }
                                }

                                // C 3418 / 3462 / 3502: a matching character is
                                // known to be a single code unit, so only the
                                // first byte is read and the pointer advances by
                                // exactly one.
                                OP_DIGIT | OP_WHITESPACE | OP_WORDCHAR => {
                                    let bit = match lctype {
                                        OP_DIGIT => ctype_digit as u32,
                                        OP_WHITESPACE => ctype_space as u32,
                                        _ => ctype_word as u32, // OP_WORDCHAR
                                    };
                                    i = 1;
                                    while i <= *Ltype_min(F) {
                                        if *Feptr(F) >= (*mb).end_subject {
                                            SCHECK_PARTIAL!();
                                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                        }
                                        let cc = *(*Feptr(F)) as u32;
                                        if cc >= 128
                                            || (*(*mb).ctypes.add(cc as usize) as u32 & bit) == 0
                                        {
                                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                        }
                                        *Feptr(F) = (*Feptr(F)).add(1);
                                        // No need to skip more code units - we
                                        // know it has only one.
                                        i += 1;
                                    }
                                }

                                // C 3432 / 3472: only the first byte is tested;
                                // the pointer then advances one code unit and
                                // skips any continuation bytes.
                                OP_NOT_WHITESPACE | OP_NOT_WORDCHAR => {
                                    let bit = if lctype == OP_NOT_WHITESPACE {
                                        ctype_space as u32
                                    } else {
                                        ctype_word as u32
                                    };
                                    i = 1;
                                    while i <= *Ltype_min(F) {
                                        if *Feptr(F) >= (*mb).end_subject {
                                            SCHECK_PARTIAL!();
                                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                        }
                                        let cc = *(*Feptr(F)) as u32;
                                        if cc < 128
                                            && (*(*mb).ctypes.add(cc as usize) as u32 & bit) != 0
                                        {
                                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                        }
                                        *Feptr(F) = (*Feptr(F)).add(1);
                                        {
                                            let mut ep = *Feptr(F);
                                            ACROSSCHAR!(ep < (*mb).end_subject, ep);
                                            *Feptr(F) = ep;
                                        }
                                        i += 1;
                                    }
                                }

                                _ => { return PCRE2_ERROR_INTERNAL as c_int; }
                            }
                        } else {
                            // Non-UTF case for min matching of operators other
                            // than OP_PROP/OP_NOTPROP.
                            match lctype {
                                OP_ANY => {
                                    i = 1;
                                    while i <= *Ltype_min(F) {
                                        if *Feptr(F) >= (*mb).end_subject {
                                            SCHECK_PARTIAL!();
                                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                        }
                                        if IS_NEWLINE!(*Feptr(F)) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                                        if (*mb).partial != 0
                                            && (*Feptr(F)).add(1) >= (*mb).end_subject
                                            && (*mb).nltype == NLTYPE_FIXED as u32
                                            && (*mb).nllen == 2
                                            && *(*Feptr(F)) as u32 == (*mb).nl[0] as u32
                                        {
                                            (*mb).hitend = TRUE;
                                            if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL as c_int; }
                                        }
                                        *Feptr(F) = (*Feptr(F)).add(1);
                                        i += 1;
                                    }
                                }
                                OP_ALLANY => {
                                    if *Feptr(F) > (*mb).end_subject.sub(*Ltype_min(F) as usize) {
                                        SCHECK_PARTIAL!();
                                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                    }
                                    *Feptr(F) = (*Feptr(F)).add(*Ltype_min(F) as usize);
                                }
                                OP_ANYNL => {
                                    i = 1;
                                    while i <= *Ltype_min(F) {
                                        if *Feptr(F) >= (*mb).end_subject {
                                            SCHECK_PARTIAL!();
                                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                        }
                                        fc = *(*Feptr(F)) as u32;
                                        *Feptr(F) = (*Feptr(F)).add(1);
                                        if fc == CHAR_CR {
                                            if *Feptr(F) < (*mb).end_subject
                                                && *(*Feptr(F)) as u32 == CHAR_NL {
                                                *Feptr(F) = (*Feptr(F)).add(1);
                                            }
                                        } else if fc == CHAR_NL {
                                            // ok
                                        } else if fc == 0x0b || fc == 0x0c || fc == 0x85 {
                                            if (*mb).bsr_convention == PCRE2_BSR_ANYCRLF as u16 {
                                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                            }
                                        } else {
                                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                        }
                                        i += 1;
                                    }
                                }
                                OP_NOT_HSPACE | OP_HSPACE | OP_NOT_VSPACE | OP_VSPACE
                                | OP_NOT_DIGIT | OP_DIGIT | OP_NOT_WHITESPACE | OP_WHITESPACE
                                | OP_NOT_WORDCHAR | OP_WORDCHAR => {
                                    i = 1;
                                    while i <= *Ltype_min(F) {
                                        if *Feptr(F) >= (*mb).end_subject {
                                            SCHECK_PARTIAL!();
                                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                        }
                                        fc = *(*Feptr(F)) as u32;
                                        if rt_ctype_reject(fc, lctype, mb) {
                                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                        }
                                        *Feptr(F) = (*Feptr(F)).add(1);
                                        i += 1;
                                    }
                                }
                                _ => { return PCRE2_ERROR_INTERNAL as c_int; }
                            }
                        }
                    }

                    // If Lmin == Lmax we are done. Continue with the main loop.
                    if *Ltype_min(F) == *Ltype_max(F) { { label = Lbl::MainLoop; continue 'sm; } }

                    // ---- Minimizing: RMATCH before each subsequent match. --
                    if reptype == REPTYPE_MIN {
                        let lctype = *Lctype(F);
                        if proptype >= 0 {
                            match proptype as i64 {
                                PT_LAMP => { { start_ecode = *Fecode(F); *Freturn_id(F) = 208; label = Lbl::MatchRecurse; continue 'sm; } }
                                PT_GC => { { start_ecode = *Fecode(F); *Freturn_id(F) = 209; label = Lbl::MatchRecurse; continue 'sm; } }
                                PT_PC => { { start_ecode = *Fecode(F); *Freturn_id(F) = 210; label = Lbl::MatchRecurse; continue 'sm; } }
                                PT_SC => { { start_ecode = *Fecode(F); *Freturn_id(F) = 211; label = Lbl::MatchRecurse; continue 'sm; } }
                                PT_SCX => { { start_ecode = *Fecode(F); *Freturn_id(F) = 224; label = Lbl::MatchRecurse; continue 'sm; } }
                                PT_ALNUM => { { start_ecode = *Fecode(F); *Freturn_id(F) = 212; label = Lbl::MatchRecurse; continue 'sm; } }
                                PT_SPACE | PT_PXSPACE => { { start_ecode = *Fecode(F); *Freturn_id(F) = 213; label = Lbl::MatchRecurse; continue 'sm; } }
                                PT_WORD => { { start_ecode = *Fecode(F); *Freturn_id(F) = 214; label = Lbl::MatchRecurse; continue 'sm; } }
                                PT_CLIST => { { start_ecode = *Fecode(F); *Freturn_id(F) = 215; label = Lbl::MatchRecurse; continue 'sm; } }
                                PT_UCNC => { { start_ecode = *Fecode(F); *Freturn_id(F) = 216; label = Lbl::MatchRecurse; continue 'sm; } }
                                PT_BIDICL => { { start_ecode = *Fecode(F); *Freturn_id(F) = 223; label = Lbl::MatchRecurse; continue 'sm; } }
                                PT_BOOL => { { start_ecode = *Fecode(F); *Freturn_id(F) = 222; label = Lbl::MatchRecurse; continue 'sm; } }
                                _ => { return PCRE2_ERROR_INTERNAL as c_int; }
                            }
                        } else if lctype == OP_EXTUNI {
                            { start_ecode = *Fecode(F); *Freturn_id(F) = 217; label = Lbl::MatchRecurse; continue 'sm; }
                        } else if utf {
                            { start_ecode = *Fecode(F); *Freturn_id(F) = 218; label = Lbl::MatchRecurse; continue 'sm; }
                        } else {
                            { start_ecode = *Fecode(F); *Freturn_id(F) = 33; label = Lbl::MatchRecurse; continue 'sm; }
                        }
                    }

                    // ---- Maximizing. ---------------------------------------
                    // "notmatch" is an ordinary local because the run loops do
                    // not call RMATCH. GOT_MAX / ENDLOOP* are local loop-exit
                    // labels modelled with labelled Rust `loop`/`break`.
                    *Ltype_start_eptr(F) = *Feptr(F); // Remember where we started
                    let lctype = *Lctype(F);
                    let propvalue = *Lpropvalue(F);

                    if proptype >= 0 {
                        let notmatch = lctype == OP_NOTPROP;
                        i = *Ltype_min(F);
                        match proptype as i64 {
                            PT_SPACE | PT_PXSPACE => {
                                // ENDLOOP99
                                'endloop99: loop {
                                    while i < *Ltype_max(F) {
                                        let mut len_u: u32 = 1;
                                        if *Feptr(F) >= (*mb).end_subject {
                                            SCHECK_PARTIAL!();
                                            break;
                                        }
                                        fc = GETCHARLENTEST(*Feptr(F), &mut len_u, utf);
                                        if is_hspace(fc) || is_vspace(fc) {
                                            if notmatch { break 'endloop99; }
                                        } else if (UCD_CATEGORY(fc) == ucp_Z as u32) == notmatch {
                                            break 'endloop99;
                                        }
                                        *Feptr(F) = (*Feptr(F)).add(len_u as usize);
                                        i += 1;
                                    }
                                    break;
                                }
                            }
                            PT_CLIST => {
                                // GOT_MAX (C 4567) — local to this loop.
                                'got_max: loop {
                                    while i < *Ltype_max(F) {
                                        let mut len_u: u32 = 1;
                                        if *Feptr(F) >= (*mb).end_subject {
                                            SCHECK_PARTIAL!();
                                            break;
                                        }
                                        fc = GETCHARLENTEST(*Feptr(F), &mut len_u, utf);
                                        let mut cp: *const u32 = crate::tables::_pcre2_ucd_caseless_sets_8
                                            .as_ptr()
                                            .add(propvalue as usize);
                                        loop {
                                            if fc < *cp {
                                                if notmatch { break; } else { break 'got_max; }
                                            }
                                            let cur = *cp;
                                            cp = cp.add(1);
                                            if fc == cur {
                                                if notmatch { break 'got_max; } else { break; }
                                            }
                                        }
                                        *Feptr(F) = (*Feptr(F)).add(len_u as usize);
                                        i += 1;
                                    }
                                    break;
                                }
                            }
                            _ => {
                                // All remaining property types share the same
                                // structure: reject via rt_prop_reject to break.
                                while i < *Ltype_max(F) {
                                    let mut len_u: u32 = 1;
                                    if *Feptr(F) >= (*mb).end_subject {
                                        SCHECK_PARTIAL!();
                                        break;
                                    }
                                    fc = GETCHARLENTEST(*Feptr(F), &mut len_u, utf);
                                    if rt_prop_reject(fc, proptype, propvalue, lctype) {
                                        break;
                                    }
                                    *Feptr(F) = (*Feptr(F)).add(len_u as usize);
                                    i += 1;
                                }
                            }
                        }

                        // Feptr is now past the end of the maximum run.
                        if reptype == REPTYPE_POS { { label = Lbl::MainLoop; continue 'sm; } }

                        // Backtrack. Use <= Lstart_eptr because \C in UTF mode
                        // can leave Lstart_eptr mid-character.
                        loop {
                            if *Feptr(F) <= *Ltype_start_eptr(F) { break; }
                            { start_ecode = *Fecode(F); *Freturn_id(F) = 221; label = Lbl::MatchRecurse; continue 'sm; }
                        }
                        // Backtracking exhausted: the minimum run already
                        // matched, so continue with the rest of the pattern.
                        { label = Lbl::MainLoop; continue 'sm; }
                    } else if lctype == OP_EXTUNI {
                        // Match extended Unicode grapheme clusters, maximally.
                        i = *Ltype_min(F);
                        while i < *Ltype_max(F) {
                            if *Feptr(F) >= (*mb).end_subject {
                                SCHECK_PARTIAL!();
                                break;
                            } else {
                                let mut ep = *Feptr(F);
                                fc = GETCHARINCTEST(&mut ep, utf);
                                *Feptr(F) = ep;
                                *Feptr(F) = crate::extuni::_pcre2_extuni_8(
                                    fc, *Feptr(F), (*mb).start_subject,
                                    (*mb).end_subject, utf as BOOL, ptr::null_mut(),
                                );
                            }
                            CHECK_PARTIAL!();
                            i += 1;
                        }

                        if reptype == REPTYPE_POS { { label = Lbl::MainLoop; continue 'sm; } }

                        loop {
                            if *Feptr(F) <= *Ltype_start_eptr(F) { break; }
                            { start_ecode = *Fecode(F); *Freturn_id(F) = 219; label = Lbl::MatchRecurse; continue 'sm; }
                        }
                        { label = Lbl::MainLoop; continue 'sm; }
                    } else if utf {
                        // UTF mode, non-property character types, maximally.
                        match lctype {
                            OP_ANY => {
                                i = *Ltype_min(F);
                                while i < *Ltype_max(F) {
                                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                    if IS_NEWLINE!(*Feptr(F)) { break; }
                                    if (*mb).partial != 0
                                        && (*Feptr(F)).add(1) >= (*mb).end_subject
                                        && (*mb).nltype == NLTYPE_FIXED as u32
                                        && (*mb).nllen == 2
                                        && *(*Feptr(F)) as u32 == (*mb).nl[0] as u32
                                    {
                                        (*mb).hitend = TRUE;
                                        if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL as c_int; }
                                    }
                                    *Feptr(F) = (*Feptr(F)).add(1);
                                    ACROSSCHAR!(*Feptr(F) < (*mb).end_subject, *Feptr(F));
                                    i += 1;
                                }
                            }
                            OP_ALLANY => {
                                if *Ltype_max(F) < u32::MAX {
                                    i = *Ltype_min(F);
                                    while i < *Ltype_max(F) {
                                        if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                        *Feptr(F) = (*Feptr(F)).add(1);
                                        ACROSSCHAR!(*Feptr(F) < (*mb).end_subject, *Feptr(F));
                                        i += 1;
                                    }
                                } else {
                                    *Feptr(F) = (*mb).end_subject; // Unlimited UTF-8 repeat
                                    SCHECK_PARTIAL!();
                                }
                            }
                            OP_ANYBYTE => {
                                fc = *Ltype_max(F) - *Ltype_min(F);
                                if fc as usize > ((*mb).end_subject).offset_from(*Feptr(F)) as usize {
                                    *Feptr(F) = (*mb).end_subject;
                                    SCHECK_PARTIAL!();
                                } else {
                                    *Feptr(F) = (*Feptr(F)).add(fc as usize);
                                }
                            }
                            OP_ANYNL => {
                                i = *Ltype_min(F);
                                while i < *Ltype_max(F) {
                                    let mut len_u: u32 = 1;
                                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                    fc = GETCHARLEN(*Feptr(F), &mut len_u);
                                    if fc == CHAR_CR {
                                        *Feptr(F) = (*Feptr(F)).add(1);
                                        if *Feptr(F) >= (*mb).end_subject { break; }
                                        if *(*Feptr(F)) as u32 == CHAR_NL { *Feptr(F) = (*Feptr(F)).add(1); }
                                    } else {
                                        if fc != CHAR_NL
                                            && ((*mb).bsr_convention == PCRE2_BSR_ANYCRLF as u16
                                                || (fc != 0x0b && fc != 0x0c && fc != 0x85
                                                    && fc != 0x2028 && fc != 0x2029))
                                        {
                                            break;
                                        }
                                        *Feptr(F) = (*Feptr(F)).add(len_u as usize);
                                    }
                                    i += 1;
                                }
                            }
                            OP_NOT_HSPACE | OP_HSPACE => {
                                i = *Ltype_min(F);
                                while i < *Ltype_max(F) {
                                    let mut len_u: u32 = 1;
                                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                    fc = GETCHARLEN(*Feptr(F), &mut len_u);
                                    let gotspace = is_hspace(fc);
                                    if gotspace == (lctype == OP_NOT_HSPACE) { break; }
                                    *Feptr(F) = (*Feptr(F)).add(len_u as usize);
                                    i += 1;
                                }
                            }
                            OP_NOT_VSPACE | OP_VSPACE => {
                                i = *Ltype_min(F);
                                while i < *Ltype_max(F) {
                                    let mut len_u: u32 = 1;
                                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                    fc = GETCHARLEN(*Feptr(F), &mut len_u);
                                    let gotspace = is_vspace(fc);
                                    if gotspace == (lctype == OP_NOT_VSPACE) { break; }
                                    *Feptr(F) = (*Feptr(F)).add(len_u as usize);
                                    i += 1;
                                }
                            }
                            OP_NOT_DIGIT => {
                                i = *Ltype_min(F);
                                while i < *Ltype_max(F) {
                                    let mut len_u: u32 = 1;
                                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                    fc = GETCHARLEN(*Feptr(F), &mut len_u);
                                    if fc < 256 && (*(*mb).ctypes.add(fc as usize) as u32 & ctype_digit as u32) != 0 { break; }
                                    *Feptr(F) = (*Feptr(F)).add(len_u as usize);
                                    i += 1;
                                }
                            }
                            OP_DIGIT => {
                                i = *Ltype_min(F);
                                while i < *Ltype_max(F) {
                                    let mut len_u: u32 = 1;
                                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                    fc = GETCHARLEN(*Feptr(F), &mut len_u);
                                    if fc >= 256 || (*(*mb).ctypes.add(fc as usize) as u32 & ctype_digit as u32) == 0 { break; }
                                    *Feptr(F) = (*Feptr(F)).add(len_u as usize);
                                    i += 1;
                                }
                            }
                            OP_NOT_WHITESPACE => {
                                i = *Ltype_min(F);
                                while i < *Ltype_max(F) {
                                    let mut len_u: u32 = 1;
                                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                    fc = GETCHARLEN(*Feptr(F), &mut len_u);
                                    if fc < 256 && (*(*mb).ctypes.add(fc as usize) as u32 & ctype_space as u32) != 0 { break; }
                                    *Feptr(F) = (*Feptr(F)).add(len_u as usize);
                                    i += 1;
                                }
                            }
                            OP_WHITESPACE => {
                                i = *Ltype_min(F);
                                while i < *Ltype_max(F) {
                                    let mut len_u: u32 = 1;
                                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                    fc = GETCHARLEN(*Feptr(F), &mut len_u);
                                    if fc >= 256 || (*(*mb).ctypes.add(fc as usize) as u32 & ctype_space as u32) == 0 { break; }
                                    *Feptr(F) = (*Feptr(F)).add(len_u as usize);
                                    i += 1;
                                }
                            }
                            OP_NOT_WORDCHAR => {
                                i = *Ltype_min(F);
                                while i < *Ltype_max(F) {
                                    let mut len_u: u32 = 1;
                                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                    fc = GETCHARLEN(*Feptr(F), &mut len_u);
                                    if fc < 256 && (*(*mb).ctypes.add(fc as usize) as u32 & ctype_word as u32) != 0 { break; }
                                    *Feptr(F) = (*Feptr(F)).add(len_u as usize);
                                    i += 1;
                                }
                            }
                            OP_WORDCHAR => {
                                i = *Ltype_min(F);
                                while i < *Ltype_max(F) {
                                    let mut len_u: u32 = 1;
                                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                    fc = GETCHARLEN(*Feptr(F), &mut len_u);
                                    if fc >= 256 || (*(*mb).ctypes.add(fc as usize) as u32 & ctype_word as u32) == 0 { break; }
                                    *Feptr(F) = (*Feptr(F)).add(len_u as usize);
                                    i += 1;
                                }
                            }
                            _ => { return PCRE2_ERROR_INTERNAL as c_int; }
                        }

                        if reptype == REPTYPE_POS { { label = Lbl::MainLoop; continue 'sm; } }

                        loop {
                            if *Feptr(F) <= *Ltype_start_eptr(F) { break; }
                            { start_ecode = *Fecode(F); *Freturn_id(F) = 220; label = Lbl::MatchRecurse; continue 'sm; }
                        }
                        { label = Lbl::MainLoop; continue 'sm; }
                    } else {
                        // Not UTF mode, maximally.
                        match lctype {
                            OP_ANY => {
                                i = *Ltype_min(F);
                                while i < *Ltype_max(F) {
                                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                    if IS_NEWLINE!(*Feptr(F)) { break; }
                                    if (*mb).partial != 0
                                        && (*Feptr(F)).add(1) >= (*mb).end_subject
                                        && (*mb).nltype == NLTYPE_FIXED as u32
                                        && (*mb).nllen == 2
                                        && *(*Feptr(F)) as u32 == (*mb).nl[0] as u32
                                    {
                                        (*mb).hitend = TRUE;
                                        if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL as c_int; }
                                    }
                                    *Feptr(F) = (*Feptr(F)).add(1);
                                    i += 1;
                                }
                            }
                            OP_ALLANY | OP_ANYBYTE => {
                                fc = *Ltype_max(F) - *Ltype_min(F);
                                if fc as usize > ((*mb).end_subject).offset_from(*Feptr(F)) as usize {
                                    *Feptr(F) = (*mb).end_subject;
                                    SCHECK_PARTIAL!();
                                } else {
                                    *Feptr(F) = (*Feptr(F)).add(fc as usize);
                                }
                            }
                            OP_ANYNL => {
                                i = *Ltype_min(F);
                                while i < *Ltype_max(F) {
                                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                    fc = *(*Feptr(F)) as u32;
                                    if fc == CHAR_CR {
                                        *Feptr(F) = (*Feptr(F)).add(1);
                                        if *Feptr(F) >= (*mb).end_subject { break; }
                                        if *(*Feptr(F)) as u32 == CHAR_NL { *Feptr(F) = (*Feptr(F)).add(1); }
                                    } else {
                                        if fc != CHAR_NL
                                            && ((*mb).bsr_convention == PCRE2_BSR_ANYCRLF as u16
                                                || (fc != 0x0b && fc != 0x0c && fc != 0x85))
                                        {
                                            break;
                                        }
                                        *Feptr(F) = (*Feptr(F)).add(1);
                                    }
                                    i += 1;
                                }
                            }
                            OP_NOT_HSPACE => {
                                // ENDLOOP00
                                'endloop00: loop {
                                    i = *Ltype_min(F);
                                    while i < *Ltype_max(F) {
                                        if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                        if is_hspace(*(*Feptr(F)) as u32) { break 'endloop00; }
                                        *Feptr(F) = (*Feptr(F)).add(1);
                                        i += 1;
                                    }
                                    break;
                                }
                            }
                            OP_HSPACE => {
                                // ENDLOOP01
                                'endloop01: loop {
                                    i = *Ltype_min(F);
                                    while i < *Ltype_max(F) {
                                        if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                        if !is_hspace(*(*Feptr(F)) as u32) { break 'endloop01; }
                                        *Feptr(F) = (*Feptr(F)).add(1);
                                        i += 1;
                                    }
                                    break;
                                }
                            }
                            OP_NOT_VSPACE => {
                                // ENDLOOP02
                                'endloop02: loop {
                                    i = *Ltype_min(F);
                                    while i < *Ltype_max(F) {
                                        if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                        if is_vspace(*(*Feptr(F)) as u32) { break 'endloop02; }
                                        *Feptr(F) = (*Feptr(F)).add(1);
                                        i += 1;
                                    }
                                    break;
                                }
                            }
                            OP_VSPACE => {
                                // ENDLOOP03
                                'endloop03: loop {
                                    i = *Ltype_min(F);
                                    while i < *Ltype_max(F) {
                                        if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                        if !is_vspace(*(*Feptr(F)) as u32) { break 'endloop03; }
                                        *Feptr(F) = (*Feptr(F)).add(1);
                                        i += 1;
                                    }
                                    break;
                                }
                            }
                            OP_NOT_DIGIT => {
                                i = *Ltype_min(F);
                                while i < *Ltype_max(F) {
                                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                    let cu = *(*Feptr(F)) as u32;
                                    if MAX_255(cu) && (*(*mb).ctypes.add(cu as usize) as u32 & ctype_digit as u32) != 0 { break; }
                                    *Feptr(F) = (*Feptr(F)).add(1);
                                    i += 1;
                                }
                            }
                            OP_DIGIT => {
                                i = *Ltype_min(F);
                                while i < *Ltype_max(F) {
                                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                    let cu = *(*Feptr(F)) as u32;
                                    if !MAX_255(cu) || (*(*mb).ctypes.add(cu as usize) as u32 & ctype_digit as u32) == 0 { break; }
                                    *Feptr(F) = (*Feptr(F)).add(1);
                                    i += 1;
                                }
                            }
                            OP_NOT_WHITESPACE => {
                                i = *Ltype_min(F);
                                while i < *Ltype_max(F) {
                                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                    let cu = *(*Feptr(F)) as u32;
                                    if MAX_255(cu) && (*(*mb).ctypes.add(cu as usize) as u32 & ctype_space as u32) != 0 { break; }
                                    *Feptr(F) = (*Feptr(F)).add(1);
                                    i += 1;
                                }
                            }
                            OP_WHITESPACE => {
                                i = *Ltype_min(F);
                                while i < *Ltype_max(F) {
                                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                    let cu = *(*Feptr(F)) as u32;
                                    if !MAX_255(cu) || (*(*mb).ctypes.add(cu as usize) as u32 & ctype_space as u32) == 0 { break; }
                                    *Feptr(F) = (*Feptr(F)).add(1);
                                    i += 1;
                                }
                            }
                            OP_NOT_WORDCHAR => {
                                i = *Ltype_min(F);
                                while i < *Ltype_max(F) {
                                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                    let cu = *(*Feptr(F)) as u32;
                                    if MAX_255(cu) && (*(*mb).ctypes.add(cu as usize) as u32 & ctype_word as u32) != 0 { break; }
                                    *Feptr(F) = (*Feptr(F)).add(1);
                                    i += 1;
                                }
                            }
                            OP_WORDCHAR => {
                                i = *Ltype_min(F);
                                while i < *Ltype_max(F) {
                                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); break; }
                                    let cu = *(*Feptr(F)) as u32;
                                    if !MAX_255(cu) || (*(*mb).ctypes.add(cu as usize) as u32 & ctype_word as u32) == 0 { break; }
                                    *Feptr(F) = (*Feptr(F)).add(1);
                                    i += 1;
                                }
                            }
                            _ => { return PCRE2_ERROR_INTERNAL as c_int; }
                        }

                        if reptype == REPTYPE_POS { { label = Lbl::MainLoop; continue 'sm; } }

                        loop {
                            if *Feptr(F) == *Ltype_start_eptr(F) { break; }
                            { start_ecode = *Fecode(F); *Freturn_id(F) = 34; label = Lbl::MatchRecurse; continue 'sm; }
                        }
                        { label = Lbl::MainLoop; continue 'sm; }
                    }
                }

                // ---- GOT_MAX (C 4567) ----
                // NOTE: In the C source `GOT_MAX` is a *local* label reached
                // only by `goto GOT_MAX` from inside the PT_CLIST maximizing
                // loop (C 4550/4559/4561); it does not cross any RMATCH and is
                // not reachable from any other case. It is therefore handled
                // inline as the `'got_max` labelled loop in `Lbl::RepeatType`
                // above. This arm exists only because the `Lbl` enum declares
                // `GotMax`; it is never entered as a state. If it is ever
                // reached it is a bug, so fail the match defensively.
                Lbl::GotMax => {
                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                }

                // ---- after RMATCH(..., RM208) at C 3787 (PT_LAMP, min) ----
                Lbl::L_RM208 => {
                    if rrc != MATCH_NOMATCH { { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let old = *Ltype_min(F); *Ltype_min(F) = old + 1;
                    if old >= *Ltype_max(F) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                    let chartype = UCD_CHARTYPE(fc);
                    if ((chartype == ucp_Lu as u32 || chartype == ucp_Ll as u32
                        || chartype == ucp_Lt as u32)) == (*Lctype(F) == OP_NOTPROP) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = 208; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(..., RM209) at C 3807 (PT_GC, min) ----
                Lbl::L_RM209 => {
                    if rrc != MATCH_NOMATCH { { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let old = *Ltype_min(F); *Ltype_min(F) = old + 1;
                    if old >= *Ltype_max(F) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                    if (UCD_CATEGORY(fc) == *Lpropvalue(F)) == (*Lctype(F) == OP_NOTPROP) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = 209; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(..., RM210) at C 3824 (PT_PC, min) ----
                Lbl::L_RM210 => {
                    if rrc != MATCH_NOMATCH { { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let old = *Ltype_min(F); *Ltype_min(F) = old + 1;
                    if old >= *Ltype_max(F) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                    if (UCD_CHARTYPE(fc) == *Lpropvalue(F)) == (*Lctype(F) == OP_NOTPROP) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = 210; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(..., RM211) at C 3841 (PT_SC, min) ----
                Lbl::L_RM211 => {
                    if rrc != MATCH_NOMATCH { { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let old = *Ltype_min(F); *Ltype_min(F) = old + 1;
                    if old >= *Ltype_max(F) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                    if (UCD_SCRIPT(fc) == *Lpropvalue(F)) == (*Lctype(F) == OP_NOTPROP) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = 211; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(..., RM224) at C 3860 (PT_SCX, min) ----
                Lbl::L_RM224 => {
                    if rrc != MATCH_NOMATCH { { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let old = *Ltype_min(F); *Ltype_min(F) = old + 1;
                    if old >= *Ltype_max(F) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                    let prop = GET_UCD(fc);
                    let ok = prop.script as u32 == *Lpropvalue(F)
                        || MAPBIT(
                            crate::tables::_pcre2_ucd_script_sets_8
                                .as_ptr()
                                .add(UCD_SCRIPTX_PROP(prop) as usize),
                            *Lpropvalue(F),
                        ) != 0;
                    if ok == (*Lctype(F) == OP_NOTPROP) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = 224; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(..., RM212) at C 3881 (PT_ALNUM, min) ----
                Lbl::L_RM212 => {
                    if rrc != MATCH_NOMATCH { { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let old = *Ltype_min(F); *Ltype_min(F) = old + 1;
                    if old >= *Ltype_max(F) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                    let category = UCD_CATEGORY(fc);
                    if ((category == ucp_L as u32 || category == ucp_N as u32))
                        == (*Lctype(F) == OP_NOTPROP) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = 212; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(..., RM213) at C 3904 (PT_SPACE/PT_PXSPACE, min) ----
                Lbl::L_RM213 => {
                    if rrc != MATCH_NOMATCH { { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let old = *Ltype_min(F); *Ltype_min(F) = old + 1;
                    if old >= *Ltype_max(F) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                    if is_hspace(fc) || is_vspace(fc) {
                        if *Lctype(F) == OP_NOTPROP { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    } else if (UCD_CATEGORY(fc) == ucp_Z as u32) == (*Lctype(F) == OP_NOTPROP) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = 213; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(..., RM214) at C 3932 (PT_WORD, min) ----
                Lbl::L_RM214 => {
                    if rrc != MATCH_NOMATCH { { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let old = *Ltype_min(F); *Ltype_min(F) = old + 1;
                    if old >= *Ltype_max(F) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                    let chartype = UCD_CHARTYPE(fc);
                    let category = crate::tables::_pcre2_ucp_gentype[chartype as usize];
                    if ((category == ucp_L as u32 || category == ucp_N as u32
                        || chartype == ucp_Mn as u32 || chartype == ucp_Pc as u32))
                        == (*Lctype(F) == OP_NOTPROP) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = 214; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(..., RM215) at C 3955 (PT_CLIST, min) ----
                Lbl::L_RM215 => {
                    if rrc != MATCH_NOMATCH { { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let old = *Ltype_min(F); *Ltype_min(F) = old + 1;
                    if old >= *Ltype_max(F) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                    let notmatch = *Lctype(F) == OP_NOTPROP;
                    let mut cp: *const u32 = crate::tables::_pcre2_ucd_caseless_sets_8
                        .as_ptr()
                        .add(*Lpropvalue(F) as usize);
                    let mut nomatch = false;
                    loop {
                        if fc < *cp {
                            if notmatch { break; } else { nomatch = true; break; }
                        }
                        let cur = *cp; cp = cp.add(1);
                        if fc == cur {
                            if notmatch { nomatch = true; }
                            break;
                        }
                    }
                    if nomatch { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = 215; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(..., RM216) at C 3991 (PT_UCNC, min) ----
                Lbl::L_RM216 => {
                    if rrc != MATCH_NOMATCH { { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let old = *Ltype_min(F); *Ltype_min(F) = old + 1;
                    if old >= *Ltype_max(F) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                    if ((fc == 0x24 || fc == 0x40 || fc == 0x60
                        || (fc >= 0xa0 && fc <= 0xd7ff) || fc >= 0xe000))
                        == (*Lctype(F) == OP_NOTPROP) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = 216; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(..., RM223) at C 4010 (PT_BIDICL, min) ----
                Lbl::L_RM223 => {
                    if rrc != MATCH_NOMATCH { { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let old = *Ltype_min(F); *Ltype_min(F) = old + 1;
                    if old >= *Ltype_max(F) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                    if (UCD_BIDICLASS(fc) == *Lpropvalue(F)) == (*Lctype(F) == OP_NOTPROP) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = 223; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(..., RM222) at C 4029 (PT_BOOL, min) ----
                Lbl::L_RM222 => {
                    if rrc != MATCH_NOMATCH { { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let old = *Ltype_min(F); *Ltype_min(F) = old + 1;
                    if old >= *Ltype_max(F) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let mut ep = *Feptr(F); fc = GETCHARINCTEST(&mut ep, utf); *Feptr(F) = ep;
                    let prop = GET_UCD(fc);
                    let ok = MAPBIT(
                        crate::tables::_pcre2_ucd_boolprop_sets_8
                            .as_ptr()
                            .add(UCD_BPROPS_PROP(prop) as usize),
                        *Lpropvalue(F),
                    ) != 0;
                    if ok == (*Lctype(F) == OP_NOTPROP) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = 222; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(..., RM217) at C 4063 (OP_EXTUNI, min) ----
                Lbl::L_RM217 => {
                    if rrc != MATCH_NOMATCH { { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let old = *Ltype_min(F); *Ltype_min(F) = old + 1;
                    if old >= *Ltype_max(F) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    if *Feptr(F) >= (*mb).end_subject {
                        SCHECK_PARTIAL!();
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    } else {
                        let mut ep = *Feptr(F);
                        fc = GETCHARINCTEST(&mut ep, utf);
                        *Feptr(F) = ep;
                        *Feptr(F) = crate::extuni::_pcre2_extuni_8(
                            fc, *Feptr(F), (*mb).start_subject, (*mb).end_subject,
                            utf as BOOL, ptr::null_mut(),
                        );
                    }
                    CHECK_PARTIAL!();
                    { start_ecode = *Fecode(F); *Freturn_id(F) = 217; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(..., RM218) at C 4090 (UTF non-prop, min) ----
                Lbl::L_RM218 => {
                    if rrc != MATCH_NOMATCH { { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let old = *Ltype_min(F); *Ltype_min(F) = old + 1;
                    if old >= *Ltype_max(F) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let lctype = *Lctype(F);
                    if lctype == OP_ANY && IS_NEWLINE!(*Feptr(F)) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let mut ep = *Feptr(F); fc = GETCHARINC(&mut ep); *Feptr(F) = ep;
                    match lctype {
                        OP_ANY => {
                            if (*mb).partial != 0
                                && *Feptr(F) >= (*mb).end_subject
                                && (*mb).nltype == NLTYPE_FIXED as u32
                                && (*mb).nllen == 2
                                && fc == (*mb).nl[0] as u32
                            {
                                (*mb).hitend = TRUE;
                                if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL as c_int; }
                            }
                        }
                        OP_ALLANY | OP_ANYBYTE => {}
                        OP_ANYNL => {
                            if fc == CHAR_CR {
                                if *Feptr(F) < (*mb).end_subject && *(*Feptr(F)) as u32 == CHAR_NL {
                                    *Feptr(F) = (*Feptr(F)).add(1);
                                }
                            } else if fc == CHAR_NL {
                                // ok
                            } else if fc == 0x0b || fc == 0x0c || fc == 0x85
                                || fc == 0x2028 || fc == 0x2029 {
                                if (*mb).bsr_convention == PCRE2_BSR_ANYCRLF as u16 {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                            } else {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                        }
                        OP_NOT_HSPACE => { if is_hspace(fc) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } } }
                        OP_HSPACE => { if !is_hspace(fc) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } } }
                        OP_NOT_VSPACE => { if is_vspace(fc) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } } }
                        OP_VSPACE => { if !is_vspace(fc) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } } }
                        OP_NOT_DIGIT => {
                            if fc < 256 && (*(*mb).ctypes.add(fc as usize) as u32 & ctype_digit as u32) != 0 { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                        }
                        OP_DIGIT => {
                            if fc >= 256 || (*(*mb).ctypes.add(fc as usize) as u32 & ctype_digit as u32) == 0 { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                        }
                        OP_NOT_WHITESPACE => {
                            if fc < 256 && (*(*mb).ctypes.add(fc as usize) as u32 & ctype_space as u32) != 0 { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                        }
                        OP_WHITESPACE => {
                            if fc >= 256 || (*(*mb).ctypes.add(fc as usize) as u32 & ctype_space as u32) == 0 { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                        }
                        OP_NOT_WORDCHAR => {
                            if fc < 256 && (*(*mb).ctypes.add(fc as usize) as u32 & ctype_word as u32) != 0 { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                        }
                        OP_WORDCHAR => {
                            if fc >= 256 || (*(*mb).ctypes.add(fc as usize) as u32 & ctype_word as u32) == 0 { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                        }
                        _ => { return PCRE2_ERROR_INTERNAL as c_int; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = 218; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(..., RM33) at C 4220 (non-UTF non-prop, min) ----
                Lbl::L_RM33 => {
                    if rrc != MATCH_NOMATCH { { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let old = *Ltype_min(F); *Ltype_min(F) = old + 1;
                    if old >= *Ltype_max(F) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    if *Feptr(F) >= (*mb).end_subject { SCHECK_PARTIAL!(); { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    let lctype = *Lctype(F);
                    if lctype == OP_ANY && IS_NEWLINE!(*Feptr(F)) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                    fc = *(*Feptr(F)) as u32; *Feptr(F) = (*Feptr(F)).add(1);
                    match lctype {
                        OP_ANY => {
                            if (*mb).partial != 0
                                && *Feptr(F) >= (*mb).end_subject
                                && (*mb).nltype == NLTYPE_FIXED as u32
                                && (*mb).nllen == 2
                                && fc == (*mb).nl[0] as u32
                            {
                                (*mb).hitend = TRUE;
                                if (*mb).partial > 1 { return PCRE2_ERROR_PARTIAL as c_int; }
                            }
                        }
                        OP_ALLANY | OP_ANYBYTE => {}
                        OP_ANYNL => {
                            if fc == CHAR_CR {
                                if *Feptr(F) < (*mb).end_subject && *(*Feptr(F)) as u32 == CHAR_NL {
                                    *Feptr(F) = (*Feptr(F)).add(1);
                                }
                            } else if fc == CHAR_NL {
                                // ok
                            } else if fc == 0x0b || fc == 0x0c || fc == 0x85 {
                                if (*mb).bsr_convention == PCRE2_BSR_ANYCRLF as u16 {
                                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                                }
                            } else {
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                        }
                        OP_NOT_HSPACE => { if is_hspace(fc) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } } }
                        OP_HSPACE => { if !is_hspace(fc) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } } }
                        OP_NOT_VSPACE => { if is_vspace(fc) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } } }
                        OP_VSPACE => { if !is_vspace(fc) { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } } }
                        OP_NOT_DIGIT => {
                            if MAX_255(fc) && (*(*mb).ctypes.add(fc as usize) as u32 & ctype_digit as u32) != 0 { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                        }
                        OP_DIGIT => {
                            if !MAX_255(fc) || (*(*mb).ctypes.add(fc as usize) as u32 & ctype_digit as u32) == 0 { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                        }
                        OP_NOT_WHITESPACE => {
                            if MAX_255(fc) && (*(*mb).ctypes.add(fc as usize) as u32 & ctype_space as u32) != 0 { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                        }
                        OP_WHITESPACE => {
                            if !MAX_255(fc) || (*(*mb).ctypes.add(fc as usize) as u32 & ctype_space as u32) == 0 { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                        }
                        OP_NOT_WORDCHAR => {
                            if MAX_255(fc) && (*(*mb).ctypes.add(fc as usize) as u32 & ctype_word as u32) != 0 { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                        }
                        OP_WORDCHAR => {
                            if !MAX_255(fc) || (*(*mb).ctypes.add(fc as usize) as u32 & ctype_word as u32) == 0 { { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; } }
                        }
                        _ => { return PCRE2_ERROR_INTERNAL as c_int; }
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = 33; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(..., RM221) at C 4641 (property, max backtrack) ----
                Lbl::L_RM221 => {
                    if rrc != MATCH_NOMATCH { { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; } }
                    *Feptr(F) = (*Feptr(F)).sub(1);
                    if utf { let mut ep = *Feptr(F); BACKCHAR(&mut ep); *Feptr(F) = ep; }
                    loop {
                        if *Feptr(F) <= *Ltype_start_eptr(F) { break; }
                        { start_ecode = *Fecode(F); *Freturn_id(F) = 221; label = Lbl::MatchRecurse; continue 'sm; }
                    }
                    { label = Lbl::MainLoop; continue 'sm; }
                }

                // ---- after RMATCH(..., RM219) at C 4684 (OP_EXTUNI, max backtrack) ----
                Lbl::L_RM219 => {
                    if rrc != MATCH_NOMATCH { { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; } }
                    // Backtracking over an extended grapheme cluster: inspect
                    // the previous two characters to decide if a break is
                    // permitted between them.
                    *Feptr(F) = (*Feptr(F)).sub(1);
                    if !utf {
                        fc = *(*Feptr(F)) as u32;
                    } else {
                        let mut ep = *Feptr(F); BACKCHAR(&mut ep); *Feptr(F) = ep;
                        fc = GETCHAR(*Feptr(F));
                    }
                    let mut rgb = UCD_GRAPHBREAK(fc);
                    loop {
                        if *Feptr(F) <= *Ltype_start_eptr(F) { break; } // At start of char run
                        let mut fptr = (*Feptr(F)).sub(1);
                        if !utf {
                            fc = *fptr as u32;
                        } else {
                            BACKCHAR(&mut fptr);
                            fc = GETCHAR(fptr);
                        }
                        let lgb = UCD_GRAPHBREAK(fc);
                        if (crate::tables::_pcre2_ucp_gbtable[lgb as usize] & (1u32 << rgb)) == 0 { break; }
                        *Feptr(F) = fptr;
                        rgb = lgb;
                    }
                    loop {
                        if *Feptr(F) <= *Ltype_start_eptr(F) { break; }
                        { start_ecode = *Fecode(F); *Freturn_id(F) = 219; label = Lbl::MatchRecurse; continue 'sm; }
                    }
                    { label = Lbl::MainLoop; continue 'sm; }
                }

                // ---- after RMATCH(..., RM220) at C 4960 (UTF non-prop, max backtrack) ----
                Lbl::L_RM220 => {
                    if rrc != MATCH_NOMATCH { { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; } }
                    *Feptr(F) = (*Feptr(F)).sub(1);
                    let mut ep = *Feptr(F); BACKCHAR(&mut ep); *Feptr(F) = ep;
                    if *Lctype(F) == OP_ANYNL && *Feptr(F) > *Ltype_start_eptr(F)
                        && *(*Feptr(F)) as u32 == CHAR_NL
                        && *(*Feptr(F)).sub(1) as u32 == CHAR_CR {
                        *Feptr(F) = (*Feptr(F)).sub(1);
                    }
                    loop {
                        if *Feptr(F) <= *Ltype_start_eptr(F) { break; }
                        { start_ecode = *Fecode(F); *Freturn_id(F) = 220; label = Lbl::MatchRecurse; continue 'sm; }
                    }
                    { label = Lbl::MainLoop; continue 'sm; }
                }

                // ---- after RMATCH(..., RM34) at C 5216 (non-UTF non-prop, max backtrack) ----
                Lbl::L_RM34 => {
                    if rrc != MATCH_NOMATCH { { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; } }
                    *Feptr(F) = (*Feptr(F)).sub(1);
                    if *Lctype(F) == OP_ANYNL && *Feptr(F) > *Ltype_start_eptr(F)
                        && *(*Feptr(F)) as u32 == CHAR_NL
                        && *(*Feptr(F)).sub(1) as u32 == CHAR_CR {
                        *Feptr(F) = (*Feptr(F)).sub(1);
                    }
                    loop {
                        if *Feptr(F) == *Ltype_start_eptr(F) { break; }
                        { start_ecode = *Fecode(F); *Freturn_id(F) = 34; label = Lbl::MatchRecurse; continue 'sm; }
                    }
                    { label = Lbl::MainLoop; continue 'sm; }
                }

                // ---- REF_REPEAT (C 5278) ----
                Lbl::RefRepeat => {
                    match *(*Fecode(F)) as u32 {
                        OP_CRSTAR | OP_CRMINSTAR | OP_CRPLUS | OP_CRMINPLUS | OP_CRQUERY
                        | OP_CRMINQUERY | OP_CRPOSSTAR | OP_CRPOSPLUS | OP_CRPOSQUERY => {
                            fc = *(*Fecode(F)) as u32 - OP_CRSTAR;
                            *Fecode(F) = (*Fecode(F)).add(1);
                            *Lref_min(F) = REP_MIN[fc as usize];
                            *Lref_max(F) = REP_MAX[fc as usize];
                            reptype = REP_TYP[fc as usize];
                        }
                        OP_CRRANGE | OP_CRMINRANGE | OP_CRPOSRANGE => {
                            *Lref_min(F) = GET2(*Fecode(F), 1);
                            *Lref_max(F) = GET2(*Fecode(F), 1 + IMM2_SIZE_U);
                            reptype = REP_TYP[(*(*Fecode(F)) as u32 - OP_CRSTAR) as usize];
                            if *Lref_max(F) == 0 {
                                *Lref_max(F) = u32::MAX;
                            }
                            *Fecode(F) = (*Fecode(F)).add(1 + 2 * IMM2_SIZE_U);
                        }
                        _ => {
                            // No repeat follows.
                            rrc = match_ref(
                                *Loffset(F), *Fbyte1(F) as BOOL, *Fbyte2(F) as c_int, F, mb,
                                &raw mut length,
                            );
                            if rrc != 0 {
                                if rrc > 0 {
                                    *Feptr(F) = (*mb).end_subject; // Partial match
                                }
                                CHECK_PARTIAL!();
                                { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                            }
                            *Feptr(F) = (*Feptr(F)).add(length as usize);
                            { label = Lbl::MainLoop; continue 'sm; }
                        }
                    }

                    // Handle repeated back references.
                    if *Loffset(F) < *Foffset_top(F)
                        && *Fovector(F).add(*Loffset(F)) != PCRE2_UNSET
                    {
                        if *Fovector(F).add(*Loffset(F)) == *Fovector(F).add(*Loffset(F) + 1) {
                            { label = Lbl::MainLoop; continue 'sm; }
                        }
                    } else {
                        // Group is not set.
                        if *Lref_min(F) == 0
                            || ((*mb).poptions & PCRE2_MATCH_UNSET_BACKREF as u32) != 0
                        {
                            { label = Lbl::MainLoop; continue 'sm; }
                        }
                    }

                    // First, ensure the minimum number of matches are present.
                    i = 1;
                    while i <= *Lref_min(F) {
                        let mut slength: PCRE2_SIZE = 0;
                        rrc = match_ref(
                            *Loffset(F), *Fbyte1(F) as BOOL, *Fbyte2(F) as c_int, F, mb,
                            &raw mut slength,
                        );
                        if rrc != 0 {
                            if rrc > 0 {
                                *Feptr(F) = (*mb).end_subject; // Partial match
                            }
                            CHECK_PARTIAL!();
                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                        }
                        *Feptr(F) = (*Feptr(F)).add(slength as usize);
                        i += 1;
                    }

                    // If min == max, we are done.
                    if *Lref_min(F) == *Lref_max(F) {
                        { label = Lbl::MainLoop; continue 'sm; }
                    }

                    // If minimizing, keep trying and advancing the pointer.
                    if reptype == REPTYPE_MIN {
                        { start_ecode = *Fecode(F); *Freturn_id(F) = RM20; label = Lbl::MatchRecurse; continue 'sm; }
                    } else {
                        // Maximizing: find the longest string and work
                        // backwards, as long as the matched lengths for each
                        // iteration are the same.
                        let mut samelengths = TRUE;
                        *Lstart(F) = *Feptr(F); // Starting position
                        *Lref_length(F) =
                            *Fovector(F).add(*Loffset(F) + 1) - *Fovector(F).add(*Loffset(F));

                        i = *Lref_min(F);
                        while i < *Lref_max(F) {
                            let mut slength: PCRE2_SIZE = 0;
                            rrc = match_ref(
                                *Loffset(F), *Fbyte1(F) as BOOL, *Fbyte2(F) as c_int, F, mb,
                                &raw mut slength,
                            );
                            if rrc != 0 {
                                // Can't use CHECK_PARTIAL because we don't want
                                // to update Feptr in the soft partial matching
                                // case.
                                if rrc > 0
                                    && (*mb).partial != 0
                                    && (*mb).end_subject > (*mb).start_used_ptr
                                {
                                    (*mb).hitend = TRUE;
                                    if (*mb).partial > 1 {
                                        return PCRE2_ERROR_PARTIAL as c_int;
                                    }
                                }
                                break;
                            }

                            if slength != *Lref_length(F) {
                                samelengths = FALSE;
                            }
                            *Feptr(F) = (*Feptr(F)).add(slength as usize);
                            i += 1;
                        }

                        // No recursion if the repeat type is possessive.
                        if reptype == REPTYPE_POS {
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        if samelengths != FALSE {
                            { start_ecode = *Fecode(F); *Freturn_id(F) = RM21; label = Lbl::MatchRecurse; continue 'sm; }
                        } else {
                            // The rare case of non-matching lengths.
                            *Lref_max(F) = i;
                            { start_ecode = *Fecode(F); *Freturn_id(F) = RM22; label = Lbl::MatchRecurse; continue 'sm; }
                        }
                    }
                }

                // ---- POSSESSIVE_NON_CAPTURE (C 5545) ----
                Lbl::PossessiveNonCapture => {
                    *Lbrapos_frame_type(F) = GF_NOCAPTURE; // Remembered frame type
                    label = Lbl::PossessiveGroup;
                    continue 'sm;
                }

                // ---- POSSESSIVE_CAPTURE (C 5553) ----
                Lbl::PossessiveCapture => {
                    number = GET2(*Fecode(F), 1 + LINK_SIZE_U);
                    *Lbrapos_frame_type(F) = GF_CAPTURE | number; // Remembered frame type
                    label = Lbl::PossessiveGroup;
                    continue 'sm;
                }

                // ---- POSSESSIVE_GROUP (C 5557) ----
                Lbl::PossessiveGroup => {
                    *Fbyte1(F) = FALSE as u8; // Lmatched_once = FALSE
                    *Lstart_group(F) = *Fecode(F); // Start of this group

                    // for (;;) { Lstart_eptr = Feptr; ...; RMATCH(RM8); ... }
                    *Lbrapos_start_eptr(F) = *Feptr(F); // Position at group start
                    group_frame_type = *Lbrapos_frame_type(F);
                    { start_ecode = (*Fecode(F)).add(
                            crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize] as usize
                        ); *Freturn_id(F) = RM8; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- GROUPLOOP (C 5676) ----
                Lbl::GroupLoop => {
                    group_frame_type = *Lbra_frame_type(F);
                    { start_ecode = (*Fecode(F)).add(
                            crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize] as usize
                        ); *Freturn_id(F) = RM2; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- ASSERT_NOT_FAILED (C 5853) ----
                Lbl::AssertNotFailed => {
                    *Fecode(F) = (*Fecode(F)).add(1 + LINK_SIZE_U);
                    { label = Lbl::MainLoop; continue 'sm; }
                }

                // ---- SCS_OFFSET_FOUND (C 5907) ----
                Lbl::ScsOffsetFound => {
                    // Resume the option scan from where OP_ASSERT_SCS stashed
                    // `ecode`; skip the remaining CREF / DNCREF options,
                    // accumulating into `length`.
                    let mut ecode: PCRE2_SPTR = *Lsaved_eptr(F);
                    loop {
                        if *ecode as u32 == OP_CREF {
                            length += 1 + IMM2_SIZE_U;
                            ecode = ecode.add(1 + IMM2_SIZE_U);
                        } else if *ecode as u32 == OP_DNCREF {
                            length += 1 + 2 * IMM2_SIZE_U;
                            ecode = ecode.add(1 + 2 * IMM2_SIZE_U);
                        } else {
                            break;
                        }
                    }

                    *Lsaved_end_subject(F) = (*mb).end_subject;
                    *Ltrue_end_extra(F) =
                        (*mb).true_end_subject.offset_from((*mb).end_subject) as PCRE2_SIZE;
                    *Lsaved_eptr(F) = *Feptr(F);
                    *Lsaved_moptions(F) = (*mb).moptions;

                    *Feptr(F) = (*mb).start_subject.add(*Fovector(F).add(offset));
                    (*mb).end_subject = (*mb).start_subject.add(*Fovector(F).add(offset + 1));
                    (*mb).true_end_subject = (*mb).end_subject;
                    (*mb).moptions &= !(PCRE2_NOTEOL as u32);

                    group_frame_type = GF_NOCAPTURE;
                    { start_ecode = (*Fecode(F)).add(1 + LINK_SIZE_U + length as usize); *Freturn_id(F) = RM38; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM20) at C 5363 (REF_REPEAT min) ----
                Lbl::L_RM20 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let old = *Lref_min(F);
                    *Lref_min(F) += 1;
                    if old >= *Lref_max(F) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let mut slength: PCRE2_SIZE = 0;
                    rrc = match_ref(
                        *Loffset(F), *Fbyte1(F) as BOOL, *Fbyte2(F) as c_int, F, mb,
                        &raw mut slength,
                    );
                    if rrc != 0 {
                        if rrc > 0 {
                            *Feptr(F) = (*mb).end_subject; // Partial match
                        }
                        CHECK_PARTIAL!();
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    *Feptr(F) = (*Feptr(F)).add(slength as usize);
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM20; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM21) at C 5423 (REF_REPEAT max) ----
                Lbl::L_RM21 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    *Feptr(F) = (*Feptr(F)).sub(*Lref_length(F));
                    if *Feptr(F) >= *Lstart(F) {
                        { start_ecode = *Fecode(F); *Freturn_id(F) = RM21; label = Lbl::MatchRecurse; continue 'sm; }
                    }
                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM22) at C 5437 (REF_REPEAT rescan) ----
                Lbl::L_RM22 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    if *Feptr(F) == *Lstart(F) {
                        // Failed after minimal repetition.
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    *Feptr(F) = *Lstart(F);
                    *Lref_max(F) -= 1;
                    i = *Lref_min(F);
                    while i < *Lref_max(F) {
                        let mut slength: PCRE2_SIZE = 0;
                        let _ = match_ref(
                            *Loffset(F), *Fbyte1(F) as BOOL, *Fbyte2(F) as c_int, F, mb,
                            &raw mut slength,
                        );
                        *Feptr(F) = (*Feptr(F)).add(slength as usize);
                        i += 1;
                    }
                    { start_ecode = *Fecode(F); *Freturn_id(F) = RM22; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(Fecode, RM9) at C 5494 (OP_BRAZERO) ----
                Lbl::L_RM9 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let mut next_ecode: PCRE2_SPTR = *Fecode(F);
                    loop {
                        next_ecode = next_ecode.add(GET(next_ecode, 1) as usize);
                        if *next_ecode as u32 != OP_ALT {
                            break;
                        }
                    }
                    *Fecode(F) = next_ecode.add(1 + LINK_SIZE_U);
                    { label = Lbl::MainLoop; continue 'sm; }
                }

                // ---- after RMATCH(..., RM10) at C 5509 (OP_BRAMINZERO) ----
                Lbl::L_RM10 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    { label = Lbl::MainLoop; continue 'sm; }
                }

                // ---- after RMATCH(..., RM8) at C 5565 (POSSESSIVE_GROUP) ----
                Lbl::L_RM8 => {
                    if rrc == MATCH_KETRPOS {
                        *Fbyte1(F) = TRUE as u8; // Lmatched_once = TRUE
                        if *Feptr(F) == *Lbrapos_start_eptr(F) {
                            // Empty match; skip to end.
                            loop {
                                *Fecode(F) = (*Fecode(F)).add(GET(*Fecode(F), 1) as usize);
                                if *(*Fecode(F)) as u32 != OP_ALT {
                                    break;
                                }
                            }
                            // break out of the for(;;) -> shared success tail.
                            *Fecode(F) = (*Fecode(F)).add(1 + LINK_SIZE_U);
                            { label = Lbl::MainLoop; continue 'sm; }
                        }

                        *Fecode(F) = *Lstart_group(F);
                        // `continue` the for(;;): re-run the group.
                        *Lbrapos_start_eptr(F) = *Feptr(F);
                        group_frame_type = *Lbrapos_frame_type(F);
                        { start_ecode = (*Fecode(F)).add(
                                crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize] as usize
                            ); *Freturn_id(F) = RM8; label = Lbl::MatchRecurse; continue 'sm; }
                    }

                    // See comment above about handling THEN.
                    if rrc == MATCH_THEN {
                        let next_ecode: PCRE2_SPTR = (*Fecode(F)).add(GET(*Fecode(F), 1) as usize);
                        if (*mb).verb_ecode_ptr < next_ecode
                            && (*(*Fecode(F)) as u32 == OP_ALT || *next_ecode as u32 == OP_ALT)
                        {
                            rrc = MATCH_NOMATCH;
                        }
                    }

                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    *Fecode(F) = (*Fecode(F)).add(GET(*Fecode(F), 1) as usize);
                    if *(*Fecode(F)) as u32 == OP_ALT {
                        // Next alternative of the for(;;).
                        *Lbrapos_start_eptr(F) = *Feptr(F);
                        group_frame_type = *Lbrapos_frame_type(F);
                        { start_ecode = (*Fecode(F)).add(
                                crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize] as usize
                            ); *Freturn_id(F) = RM8; label = Lbl::MatchRecurse; continue 'sm; }
                    }

                    // Loop exhausted: success if matched something or zero
                    // repeat allowed (Lmatched_once == byte1, Lzero_allowed ==
                    // byte2).
                    if *Fbyte1(F) != FALSE as u8 || *Fbyte2(F) != FALSE as u8 {
                        *Fecode(F) = (*Fecode(F)).add(1 + LINK_SIZE_U);
                        { label = Lbl::MainLoop; continue 'sm; }
                    }

                    { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                }

                // ---- after RMATCH(..., RM1) at C 5644 (OP_BRA non-final branch) ----
                Lbl::L_RM1 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    // Continue the for(;;) loop in OP_BRA.
                    loop {
                        let current_branch: PCRE2_SPTR = *Fecode(F);
                        let next_branch: PCRE2_SPTR =
                            current_branch.add(GET(current_branch, 1) as usize);

                        if *next_branch as u32 != OP_ALT {
                            break;
                        }

                        *Fecode(F) = next_branch;
                        { start_ecode = current_branch.add(1 + LINK_SIZE_U); *Freturn_id(F) = RM1; label = Lbl::MatchRecurse; continue 'sm; }
                    }

                    *Fecode(F) = (*Fecode(F)).add(1 + LINK_SIZE_U);
                    { label = Lbl::MainLoop; continue 'sm; }
                }

                // ---- after RMATCH(..., RM2) at C 5680 (GROUPLOOP) ----
                Lbl::L_RM2 => {
                    if rrc == MATCH_THEN {
                        let next_ecode: PCRE2_SPTR = (*Fecode(F)).add(GET(*Fecode(F), 1) as usize);
                        if (*mb).verb_ecode_ptr < next_ecode
                            && (*(*Fecode(F)) as u32 == OP_ALT || *next_ecode as u32 == OP_ALT)
                        {
                            rrc = MATCH_NOMATCH;
                        }
                    }
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    *Fecode(F) = (*Fecode(F)).add(GET(*Fecode(F), 1) as usize);
                    if *(*Fecode(F)) as u32 != OP_ALT {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    // Next iteration of the for(;;).
                    label = Lbl::GroupLoop;
                    continue 'sm;
                }

                // ---- after RMATCH(..., RM11) at C 5748 (OP_RECURSE) ----
                Lbl::L_RM11 => {
                    let next_ecode: PCRE2_SPTR =
                        (*Lrecurse_start_branch(F)).add(GET(*Lrecurse_start_branch(F), 1) as usize);

                    // Handle backtracking verbs.
                    if rrc >= MATCH_BACKTRACK_MIN
                        && rrc <= MATCH_BACKTRACK_MAX
                        && (*mb).verb_current_recurse == (*Lrecurse_frame_type(F) ^ GF_RECURSE)
                    {
                        if rrc == MATCH_THEN
                            && (*mb).verb_ecode_ptr < next_ecode
                            && (*(*Lrecurse_start_branch(F)) as u32 == OP_ALT
                                || *next_ecode as u32 == OP_ALT)
                        {
                            rrc = MATCH_NOMATCH;
                        } else {
                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                        }
                    }

                    // Carrying on after (*ACCEPT) in a recursion is handled in
                    // the OP_ACCEPT code; nothing to do here.
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    *Lrecurse_start_branch(F) = next_ecode;
                    if *(*Lrecurse_start_branch(F)) as u32 != OP_ALT {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    // Next iteration of the for(;;).
                    group_frame_type = *Lrecurse_frame_type(F);
                    { start_ecode = (*Lrecurse_start_branch(F)).add(
                            crate::tables::_pcre2_OP_lengths_8
                                [*(*Lrecurse_start_branch(F)) as usize]
                                as usize
                        ); *Freturn_id(F) = RM11; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(..., RM3) at C 5796 (positive assertions) ----
                Lbl::L_RM3 => {
                    if rrc == MATCH_ACCEPT {
                        ptr::copy_nonoverlapping(
                            (assert_accept_frame as *const u8).add(offset_of!(heapframe, ovector)),
                            Fovector(F) as *mut u8,
                            *Foffset_top(assert_accept_frame) as usize
                                * core::mem::size_of::<PCRE2_SIZE>(),
                        );
                        *Foffset_top(F) = *Foffset_top(assert_accept_frame);
                        *Fmark(F) = *Fmark(assert_accept_frame);
                        // break out of the for(;;) -> shared tail below.
                        loop {
                            *Fecode(F) = (*Fecode(F)).add(GET(*Fecode(F), 1) as usize);
                            if *(*Fecode(F)) as u32 != OP_ALT {
                                break;
                            }
                        }
                        *Fecode(F) = (*Fecode(F)).add(1 + LINK_SIZE_U);
                        { label = Lbl::MainLoop; continue 'sm; }
                    }
                    if rrc != MATCH_NOMATCH && rrc != MATCH_THEN {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    *Fecode(F) = (*Fecode(F)).add(GET(*Fecode(F), 1) as usize);
                    if *(*Fecode(F)) as u32 != OP_ALT {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    // Next branch: re-enter the for(;;).
                    group_frame_type = GF_NOCAPTURE;
                    { start_ecode = (*Fecode(F)).add(
                            crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize] as usize
                        ); *Freturn_id(F) = RM3; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(..., RM4) at C 5825 (negative assertions) ----
                Lbl::L_RM4 => {
                    match rrc {
                        // Assertion matched, therefore it fails.
                        MATCH_ACCEPT | MATCH_MATCH => {
                            { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                        }
                        // Branch failed, try next if present.
                        MATCH_NOMATCH | MATCH_THEN => {
                            *Fecode(F) = (*Fecode(F)).add(GET(*Fecode(F), 1) as usize);
                            if *(*Fecode(F)) as u32 != OP_ALT {
                                label = Lbl::AssertNotFailed;
                                continue 'sm;
                            }
                            // Next branch: re-enter the for(;;).
                            group_frame_type = GF_NOCAPTURE;
                            { start_ecode = (*Fecode(F)).add(
                                    crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize]
                                        as usize
                                ); *Freturn_id(F) = RM4; label = Lbl::MatchRecurse; continue 'sm; }
                        }
                        // Assertion forced to fail, therefore continue.
                        MATCH_COMMIT | MATCH_SKIP | MATCH_PRUNE => {
                            loop {
                                *Fecode(F) = (*Fecode(F)).add(GET(*Fecode(F), 1) as usize);
                                if *(*Fecode(F)) as u32 != OP_ALT {
                                    break;
                                }
                            }
                            label = Lbl::AssertNotFailed;
                            continue 'sm;
                        }
                        // Pass back any other return.
                        _ => {
                            { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                        }
                    }
                }

                // ---- after RMATCH(..., RM38) at C 5939 (OP_ASSERT_SCS) ----
                Lbl::L_RM38 => {
                    if rrc == MATCH_ACCEPT {
                        ptr::copy_nonoverlapping(
                            (assert_accept_frame as *const u8).add(offset_of!(heapframe, ovector)),
                            Fovector(F) as *mut u8,
                            *Foffset_top(assert_accept_frame) as usize
                                * core::mem::size_of::<PCRE2_SIZE>(),
                        );
                        *Foffset_top(F) = *Foffset_top(assert_accept_frame);
                        *Fmark(F) = *Fmark(assert_accept_frame);
                        (*mb).end_subject = *Lsaved_end_subject(F);
                        (*mb).true_end_subject = (*mb).end_subject.add(*Ltrue_end_extra(F));
                        (*mb).moptions = *Lsaved_moptions(F);
                        // break out of the for(;;) -> shared tail below.
                        loop {
                            *Fecode(F) = (*Fecode(F)).add(GET(*Fecode(F), 1) as usize);
                            if *(*Fecode(F)) as u32 != OP_ALT {
                                break;
                            }
                        }
                        *Fecode(F) = (*Fecode(F)).add(1 + LINK_SIZE_U);
                        *Feptr(F) = *Lsaved_eptr(F);
                        { label = Lbl::MainLoop; continue 'sm; }
                    }

                    if rrc != MATCH_NOMATCH && rrc != MATCH_THEN {
                        (*mb).end_subject = *Lsaved_end_subject(F);
                        (*mb).true_end_subject = (*mb).end_subject.add(*Ltrue_end_extra(F));
                        (*mb).moptions = *Lsaved_moptions(F);
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }

                    *Fecode(F) = (*Fecode(F)).add(GET(*Fecode(F), 1) as usize);
                    if *(*Fecode(F)) as u32 != OP_ALT {
                        (*mb).end_subject = *Lsaved_end_subject(F);
                        (*mb).true_end_subject = (*mb).end_subject.add(*Ltrue_end_extra(F));
                        (*mb).moptions = *Lsaved_moptions(F);
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    length = 0;
                    // Next iteration of the for(;;).
                    group_frame_type = GF_NOCAPTURE;
                    { start_ecode = (*Fecode(F)).add(1 + LINK_SIZE_U + length as usize); *Freturn_id(F) = RM38; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- ASSERT_NL_OR_EOS (C 6604) ----
                Lbl::AssertNlOrEos => {
                    if *Feptr(F) < (*mb).true_end_subject
                        && (!IS_NEWLINE!(*Feptr(F))
                            || *Feptr(F) != (*mb).true_end_subject.sub((*mb).nllen as usize))
                    {
                        if (*mb).partial != 0
                            && (*Feptr(F)).add(1) >= (*mb).end_subject
                            && (*mb).nltype == NLTYPE_FIXED as u32
                            && (*mb).nllen == 2
                            && *(*Feptr(F)) as u32 == (*mb).nl[0] as u32
                        {
                            (*mb).hitend = TRUE;
                            if (*mb).partial > 1 {
                                return PCRE2_ERROR_PARTIAL as c_int;
                            }
                        }
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }

                    // Either at end of string or \n before end.
                    if (*mb).partial != 0 {
                        (*mb).hitend = TRUE;
                        if (*mb).partial > 1 {
                            return PCRE2_ERROR_PARTIAL as c_int;
                        }
                    }
                    *Fecode(F) = (*Fecode(F)).add(1);
                    { label = Lbl::MainLoop; continue 'sm; }
                }

                // ---- RETURN_SWITCH (C 6909) ----
                Lbl::ReturnSwitch => {
                    if *Feptr(F) > (*mb).last_used_ptr {
                        (*mb).last_used_ptr = *Feptr(F);
                    }
                    if *Frdepth(F) == 0 {
                        return rrc; // Exit from the top level
                    }
                    F = (F as *mut u8).sub(*Fback_frame(F) as usize) as *mut heapframe; // Backtrack
                    (*(*mb).cb).callout_flags |= PCRE2_CALLOUT_BACKTRACK as u32; // Note for callouts

                    match *Freturn_id(F) {
                        RM1 => label = Lbl::L_RM1,
                        RM2 => label = Lbl::L_RM2,
                        RM3 => label = Lbl::L_RM3,
                        RM4 => label = Lbl::L_RM4,
                        RM5 => label = Lbl::L_RM5,
                        RM6 => label = Lbl::L_RM6,
                        RM7 => label = Lbl::L_RM7,
                        RM8 => label = Lbl::L_RM8,
                        RM9 => label = Lbl::L_RM9,
                        RM10 => label = Lbl::L_RM10,
                        RM11 => label = Lbl::L_RM11,
                        RM12 => label = Lbl::L_RM12,
                        RM13 => label = Lbl::L_RM13,
                        RM14 => label = Lbl::L_RM14,
                        RM15 => label = Lbl::L_RM15,
                        RM16 => label = Lbl::L_RM16,
                        RM17 => label = Lbl::L_RM17,
                        RM18 => label = Lbl::L_RM18,
                        RM19 => label = Lbl::L_RM19,
                        RM20 => label = Lbl::L_RM20,
                        RM21 => label = Lbl::L_RM21,
                        RM22 => label = Lbl::L_RM22,
                        RM23 => label = Lbl::L_RM23,
                        RM24 => label = Lbl::L_RM24,
                        RM25 => label = Lbl::L_RM25,
                        RM26 => label = Lbl::L_RM26,
                        RM27 => label = Lbl::L_RM27,
                        RM28 => label = Lbl::L_RM28,
                        RM29 => label = Lbl::L_RM29,
                        RM30 => label = Lbl::L_RM30,
                        RM31 => label = Lbl::L_RM31,
                        RM32 => label = Lbl::L_RM32,
                        RM33 => label = Lbl::L_RM33,
                        RM34 => label = Lbl::L_RM34,
                        RM35 => label = Lbl::L_RM35,
                        RM36 => label = Lbl::L_RM36,
                        RM37 => label = Lbl::L_RM37,
                        RM38 => label = Lbl::L_RM38,
                        RM39 => label = Lbl::L_RM39,
                        RM100 => label = Lbl::L_RM100,
                        RM101 => label = Lbl::L_RM101,
                        RM102 => label = Lbl::L_RM102,
                        RM103 => label = Lbl::L_RM103,
                        RM200 => label = Lbl::L_RM200,
                        RM201 => label = Lbl::L_RM201,
                        RM202 => label = Lbl::L_RM202,
                        RM203 => label = Lbl::L_RM203,
                        RM204 => label = Lbl::L_RM204,
                        RM205 => label = Lbl::L_RM205,
                        RM206 => label = Lbl::L_RM206,
                        RM207 => label = Lbl::L_RM207,
                        RM208 => label = Lbl::L_RM208,
                        RM209 => label = Lbl::L_RM209,
                        RM210 => label = Lbl::L_RM210,
                        RM211 => label = Lbl::L_RM211,
                        RM212 => label = Lbl::L_RM212,
                        RM213 => label = Lbl::L_RM213,
                        RM214 => label = Lbl::L_RM214,
                        RM215 => label = Lbl::L_RM215,
                        RM216 => label = Lbl::L_RM216,
                        RM217 => label = Lbl::L_RM217,
                        RM218 => label = Lbl::L_RM218,
                        RM219 => label = Lbl::L_RM219,
                        RM220 => label = Lbl::L_RM220,
                        RM221 => label = Lbl::L_RM221,
                        RM222 => label = Lbl::L_RM222,
                        RM223 => label = Lbl::L_RM223,
                        RM224 => label = Lbl::L_RM224,
                        _ => {
                            return PCRE2_ERROR_INTERNAL as c_int;
                        }
                    }
                    continue 'sm;
                }

                // ---- after RMATCH(..., RM5) in OP_COND assertion (C 6105) ----
                Lbl::L_RM5 => {
                    match rrc {
                        MATCH_ACCEPT => {
                            // Save captures.
                            ptr::copy_nonoverlapping(
                                (assert_accept_frame as *const u8)
                                    .add(offset_of!(heapframe, ovector))
                                    as *const PCRE2_SIZE,
                                Fovector(F),
                                *Foffset_top(assert_accept_frame),
                            );
                            *Foffset_top(F) = *Foffset_top(assert_accept_frame);
                            // Fall through: captures already in current frame.
                            condition = *Fbyte1(F) as BOOL; // TRUE for positive assertion
                        }
                        MATCH_MATCH => {
                            condition = *Fbyte1(F) as BOOL; // TRUE for positive assertion
                        }
                        MATCH_NOMATCH | MATCH_THEN => {
                            *Lcond_start_branch(F) =
                                (*Lcond_start_branch(F)).add(GET(*Lcond_start_branch(F), 1) as usize);
                            if *(*Lcond_start_branch(F)) as u32 == OP_ALT {
                                // Try next branch.
                                group_frame_type = GF_CONDASSERT;
                                { start_ecode = (*Lcond_start_branch(F)).add(
                                        crate::tables::_pcre2_OP_lengths_8
                                            [*(*Lcond_start_branch(F)) as usize]
                                            as usize
                                    ); *Freturn_id(F) = RM5; label = Lbl::MatchRecurse; continue 'sm; }
                            }
                            condition = (*Fbyte1(F) == 0) as BOOL; // TRUE for negative assertion
                        }
                        MATCH_COMMIT | MATCH_SKIP | MATCH_PRUNE => {
                            condition = (*Fbyte1(F) == 0) as BOOL;
                        }
                        _ => {
                            { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                        }
                    }

                    // If the condition is true, find the end of the assertion
                    // so advancing past it reaches the first branch.
                    if condition != FALSE {
                        loop {
                            *Fecode(F) = (*Fecode(F)).add(GET(*Fecode(F), 1) as usize);
                            if *(*Fecode(F)) as u32 != OP_ALT {
                                break;
                            }
                        }
                    }

                    // Choose branch according to the condition.
                    *Fecode(F) = (*Fecode(F)).add(if condition != FALSE {
                        crate::tables::_pcre2_OP_lengths_8[*(*Fecode(F)) as usize] as usize
                    } else {
                        *Lcond_length(F)
                    });

                    if (*Fop(F) as u32) == OP_SCOND {
                        group_frame_type = GF_NOCAPTURE;
                        { start_ecode = *Fecode(F); *Freturn_id(F) = RM35; label = Lbl::MatchRecurse; continue 'sm; }
                    }
                    { label = Lbl::MainLoop; continue 'sm; }
                }

                // ---- after RMATCH(..., RM35) in OP_SCOND (C 6169) ----
                Lbl::L_RM35 => {
                    { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                }

                // ---- after RMATCH(..., RM37) in OP_VREVERSE (C 6274) ----
                Lbl::L_RM37 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    let old_max = *Lvreverse_max(F);
                    *Lvreverse_max(F) = old_max.wrapping_sub(1);
                    if old_max <= *Lvreverse_min(F) {
                        { rrc = MATCH_NOMATCH; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    *Feptr(F) = (*Feptr(F)).add(1);
                    if utf {
                        let mut ep = *Feptr(F);
                        FORWARDCHARTEST(&mut ep, (*mb).end_subject);
                        *Feptr(F) = ep;
                    }
                    { start_ecode = (*Fecode(F)).add(1 + 2 * IMM2_SIZE_U); *Freturn_id(F) = RM37; label = Lbl::MatchRecurse; continue 'sm; }
                }

                // ---- after RMATCH(..., RM39) in OP_ASSERT_SCS KET (C 6469) ----
                Lbl::L_RM39 => {
                    (*mb).end_subject = *Lsaved_end_subject(F);
                    (*mb).true_end_subject = (*mb).end_subject;
                    { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                }

                // ---- after RMATCH(..., RM6) in OP_KETRMIN (C 6548) ----
                Lbl::L_RM6 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    *Fecode(F) = (*Fecode(F)).sub(GET(*Fecode(F), 1) as usize);
                    { label = Lbl::MainLoop; continue 'sm; } // end of ket processing
                }

                // ---- after RMATCH(..., RM7) in OP_KETRMAX (C 6556) ----
                Lbl::L_RM7 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    // Carry on at this level.
                    *Fecode(F) = (*Fecode(F)).add(1 + LINK_SIZE_U);
                    { label = Lbl::MainLoop; continue 'sm; }
                }

                // ---- after RMATCH(..., RM12) in OP_MARK (C 6779) ----
                Lbl::L_RM12 => {
                    // MATCH_SKIP_ARG: check whether the SKIP arg matches this
                    // MARK's argument.
                    if rrc == MATCH_SKIP_ARG
                        && crate::string_utils::_pcre2_strcmp_8(
                            (*Fecode(F)).add(2),
                            (*mb).verb_skip_ptr,
                        ) == 0
                    {
                        (*mb).verb_skip_ptr = *Feptr(F); // Pass back current position
                        { rrc = MATCH_SKIP; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                }

                // ---- after RMATCH(..., RM13) in OP_COMMIT (C 6804) ----
                Lbl::L_RM13 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    (*mb).verb_current_recurse = *Fcurrent_recurse(F);
                    { rrc = MATCH_COMMIT; label = Lbl::ReturnSwitch; continue 'sm; }
                }

                // ---- after RMATCH(..., RM36) in OP_COMMIT_ARG (C 6811) ----
                Lbl::L_RM36 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    (*mb).verb_current_recurse = *Fcurrent_recurse(F);
                    { rrc = MATCH_COMMIT; label = Lbl::ReturnSwitch; continue 'sm; }
                }

                // ---- after RMATCH(..., RM14) in OP_PRUNE (C 6817) ----
                Lbl::L_RM14 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    (*mb).verb_current_recurse = *Fcurrent_recurse(F);
                    { rrc = MATCH_PRUNE; label = Lbl::ReturnSwitch; continue 'sm; }
                }

                // ---- after RMATCH(..., RM15) in OP_PRUNE_ARG (C 6824) ----
                Lbl::L_RM15 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    (*mb).verb_current_recurse = *Fcurrent_recurse(F);
                    { rrc = MATCH_PRUNE; label = Lbl::ReturnSwitch; continue 'sm; }
                }

                // ---- after RMATCH(..., RM16) in OP_SKIP (C 6830) ----
                Lbl::L_RM16 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    (*mb).verb_skip_ptr = *Feptr(F); // Pass back current position
                    (*mb).verb_current_recurse = *Fcurrent_recurse(F);
                    { rrc = MATCH_SKIP; label = Lbl::ReturnSwitch; continue 'sm; }
                }

                // ---- after RMATCH(..., RM17) in OP_SKIP_ARG (C 6851) ----
                Lbl::L_RM17 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    // Pass back the current skip name; return MATCH_SKIP_ARG.
                    (*mb).verb_skip_ptr = (*Fecode(F)).add(2);
                    (*mb).verb_current_recurse = *Fcurrent_recurse(F);
                    { rrc = MATCH_SKIP_ARG; label = Lbl::ReturnSwitch; continue 'sm; }
                }

                // ---- after RMATCH(..., RM18) in OP_THEN (C 6867) ----
                Lbl::L_RM18 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    (*mb).verb_ecode_ptr = *Fecode(F);
                    (*mb).verb_current_recurse = *Fcurrent_recurse(F);
                    { rrc = MATCH_THEN; label = Lbl::ReturnSwitch; continue 'sm; }
                }

                // ---- after RMATCH(..., RM19) in OP_THEN_ARG (C 6875) ----
                Lbl::L_RM19 => {
                    if rrc != MATCH_NOMATCH {
                        { rrc = rrc; label = Lbl::ReturnSwitch; continue 'sm; }
                    }
                    (*mb).verb_ecode_ptr = *Fecode(F);
                    (*mb).verb_current_recurse = *Fcurrent_recurse(F);
                    { rrc = MATCH_THEN; label = Lbl::ReturnSwitch; continue 'sm; }
                }
            }
        }
    }
}
