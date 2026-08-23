// Translated from c_src/src/pcre2_match.c
use crate::internal::*;

/* These defines identify the name of the block containing "static"
information, and fields within it: NLBLOCK = mb, PSSTART = start_subject,
PSEND = end_subject. */

pub const RECURSE_UNSET: u32 = 0xffffffff; /* Bigger than max group number */

/* Masks for identifying the public options that are permitted at match time. */

pub const PUBLIC_MATCH_OPTIONS: u32 = PCRE2_ANCHORED
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

pub const PUBLIC_JIT_MATCH_OPTIONS: u32 = PCRE2_NO_UTF_CHECK
    | PCRE2_NOTBOL
    | PCRE2_NOTEOL
    | PCRE2_NOTEMPTY
    | PCRE2_NOTEMPTY_ATSTART
    | PCRE2_PARTIAL_SOFT
    | PCRE2_PARTIAL_HARD
    | PCRE2_COPY_MATCHED_SUBJECT;

/* Non-error returns from and within the match() function. */

pub const MATCH_MATCH: c_int = 1;
pub const MATCH_NOMATCH: c_int = 0;

/* Special internal returns used in the match() function. */

pub const MATCH_ACCEPT: c_int = -999;
pub const MATCH_KETRPOS: c_int = -998;
pub const MATCH_COMMIT: c_int = -997;
pub const MATCH_PRUNE: c_int = -996;
pub const MATCH_SKIP: c_int = -995;
pub const MATCH_SKIP_ARG: c_int = -994;
pub const MATCH_THEN: c_int = -993;
pub const MATCH_BACKTRACK_MAX: c_int = MATCH_THEN;
pub const MATCH_BACKTRACK_MIN: c_int = MATCH_COMMIT;

/* Group frame type values. */

pub const GF_CAPTURE: u32 = 0x00010000;
pub const GF_NOCAPTURE: u32 = 0x00020000;
pub const GF_CONDASSERT: u32 = 0x00030000;
pub const GF_RECURSE: u32 = 0x00040000;

#[inline]
pub fn GF_IDMASK(a: u32) -> u32 {
    a & 0xffff0000
}
#[inline]
pub fn GF_DATAMASK(a: u32) -> u32 {
    a & 0x0000ffff
}

/* Repetition types */

pub const REPTYPE_MIN: u32 = 0;
pub const REPTYPE_MAX: u32 = 1;
pub const REPTYPE_POS: u32 = 2;

/* Min and max values for the common repeats; a maximum of UINT32_MAX =>
infinity. */

static rep_min: [u32; 11] = [
    0, 0, /* * and *? */
    1, 1, /* + and +? */
    0, 0, /* ? and ?? */
    0, 0, /* dummy placefillers for OP_CR[MIN]RANGE */
    0, 1, 0,
]; /* OP_CRPOS{STAR, PLUS, QUERY} */

static rep_max: [u32; 11] = [
    u32::MAX,
    u32::MAX, /* * and *? */
    u32::MAX,
    u32::MAX, /* + and +? */
    1,
    1, /* ? and ?? */
    0,
    0, /* dummy placefillers for OP_CR[MIN]RANGE */
    u32::MAX,
    u32::MAX,
    1,
]; /* OP_CRPOS{STAR, PLUS, QUERY} */

/* Repetition types - must include OP_CRPOSRANGE (not needed above) */

static rep_typ: [u32; 12] = [
    REPTYPE_MAX,
    REPTYPE_MIN, /* * and *? */
    REPTYPE_MAX,
    REPTYPE_MIN, /* + and +? */
    REPTYPE_MAX,
    REPTYPE_MIN, /* ? and ?? */
    REPTYPE_MAX,
    REPTYPE_MIN, /* OP_CRRANGE and OP_CRMINRANGE */
    REPTYPE_POS,
    REPTYPE_POS, /* OP_CRPOSSTAR, OP_CRPOSPLUS */
    REPTYPE_POS,
    REPTYPE_POS,
]; /* OP_CRPOSQUERY, OP_CRPOSRANGE */

/* Numbers for RMATCH calls at backtracking points. */

pub const RM1: u8 = 1;
pub const RM2: u8 = 2;
pub const RM3: u8 = 3;
pub const RM4: u8 = 4;
pub const RM5: u8 = 5;
pub const RM6: u8 = 6;
pub const RM7: u8 = 7;
pub const RM8: u8 = 8;
pub const RM9: u8 = 9;
pub const RM10: u8 = 10;
pub const RM11: u8 = 11;
pub const RM12: u8 = 12;
pub const RM13: u8 = 13;
pub const RM14: u8 = 14;
pub const RM15: u8 = 15;
pub const RM16: u8 = 16;
pub const RM17: u8 = 17;
pub const RM18: u8 = 18;
pub const RM19: u8 = 19;
pub const RM20: u8 = 20;
pub const RM21: u8 = 21;
pub const RM22: u8 = 22;
pub const RM23: u8 = 23;
pub const RM24: u8 = 24;
pub const RM25: u8 = 25;
pub const RM26: u8 = 26;
pub const RM27: u8 = 27;
pub const RM28: u8 = 28;
pub const RM29: u8 = 29;
pub const RM30: u8 = 30;
pub const RM31: u8 = 31;
pub const RM32: u8 = 32;
pub const RM33: u8 = 33;
pub const RM34: u8 = 34;
pub const RM35: u8 = 35;
pub const RM36: u8 = 36;
pub const RM37: u8 = 37;
pub const RM38: u8 = 38;
pub const RM39: u8 = 39;
pub const RM100: u8 = 100;
pub const RM101: u8 = 101;
pub const RM102: u8 = 102;
pub const RM103: u8 = 103;
pub const RM200: u8 = 200;
pub const RM201: u8 = 201;
pub const RM202: u8 = 202;
pub const RM203: u8 = 203;
pub const RM204: u8 = 204;
pub const RM205: u8 = 205;
pub const RM206: u8 = 206;
pub const RM207: u8 = 207;
pub const RM208: u8 = 208;
pub const RM209: u8 = 209;
pub const RM210: u8 = 210;
pub const RM211: u8 = 211;
pub const RM212: u8 = 212;
pub const RM213: u8 = 213;
pub const RM214: u8 = 214;
pub const RM215: u8 = 215;
pub const RM216: u8 = 216;
pub const RM217: u8 = 217;
pub const RM218: u8 = 218;
pub const RM219: u8 = 219;
pub const RM220: u8 = 220;
pub const RM221: u8 = 221;
pub const RM222: u8 = 222;
pub const RM223: u8 = 223;
pub const RM224: u8 = 224;

/* ---------------------------------------------------------------------------
The C code uses gotos, which Rust does not have. The interpreter is therefore
translated into a state machine: a `state` variable holds the "program counter"
and 'sm: loop { ... } dispatches on it.

  * states 0..=172   : the opcode currently being processed (i.e. the C
                       `switch(Fop)` cases). The `match` arms use the OP_xxx
                       constants directly.
  * ST_TOP           : top of the C `for (;;)` loop: fetch Fop and dispatch.
  * ST_MATCH_RECURSE : the C MATCH_RECURSE label (create a new frame).
  * ST_NEW_FRAME     : the C NEW_FRAME label.
  * ST_RETURN_SWITCH : the C RETURN_SWITCH label (pop a frame and resume).
  * ST_L_RMnnn       : the C L_RMnnn resume labels (= 2000 + nnn).
  * ST_xxx (3000+)   : the other labels inside the switch arms.
--------------------------------------------------------------------------- */

const ST_TOP: u32 = 1000;
const ST_MATCH_RECURSE: u32 = 1001;
const ST_NEW_FRAME: u32 = 1002;
const ST_RETURN_SWITCH: u32 = 1003;

const ST_L_RM_BASE: u32 = 2000;

const ST_L_RM1: u32 = 2001;
const ST_L_RM2: u32 = 2002;
const ST_L_RM3: u32 = 2003;
const ST_L_RM4: u32 = 2004;
const ST_L_RM5: u32 = 2005;
const ST_L_RM6: u32 = 2006;
const ST_L_RM7: u32 = 2007;
const ST_L_RM8: u32 = 2008;
const ST_L_RM9: u32 = 2009;
const ST_L_RM10: u32 = 2010;
const ST_L_RM11: u32 = 2011;
const ST_L_RM12: u32 = 2012;
const ST_L_RM13: u32 = 2013;
const ST_L_RM14: u32 = 2014;
const ST_L_RM15: u32 = 2015;
const ST_L_RM16: u32 = 2016;
const ST_L_RM17: u32 = 2017;
const ST_L_RM18: u32 = 2018;
const ST_L_RM19: u32 = 2019;
const ST_L_RM20: u32 = 2020;
const ST_L_RM21: u32 = 2021;
const ST_L_RM22: u32 = 2022;
const ST_L_RM23: u32 = 2023;
const ST_L_RM24: u32 = 2024;
const ST_L_RM25: u32 = 2025;
const ST_L_RM26: u32 = 2026;
const ST_L_RM27: u32 = 2027;
const ST_L_RM28: u32 = 2028;
const ST_L_RM29: u32 = 2029;
const ST_L_RM30: u32 = 2030;
const ST_L_RM31: u32 = 2031;
const ST_L_RM32: u32 = 2032;
const ST_L_RM33: u32 = 2033;
const ST_L_RM34: u32 = 2034;
const ST_L_RM35: u32 = 2035;
const ST_L_RM36: u32 = 2036;
const ST_L_RM37: u32 = 2037;
const ST_L_RM38: u32 = 2038;
const ST_L_RM39: u32 = 2039;
const ST_L_RM100: u32 = 2100;
const ST_L_RM101: u32 = 2101;
const ST_L_RM102: u32 = 2102;
const ST_L_RM103: u32 = 2103;
const ST_L_RM200: u32 = 2200;
const ST_L_RM201: u32 = 2201;
const ST_L_RM202: u32 = 2202;
const ST_L_RM203: u32 = 2203;
const ST_L_RM204: u32 = 2204;
const ST_L_RM205: u32 = 2205;
const ST_L_RM206: u32 = 2206;
const ST_L_RM207: u32 = 2207;
const ST_L_RM208: u32 = 2208;
const ST_L_RM209: u32 = 2209;
const ST_L_RM210: u32 = 2210;
const ST_L_RM211: u32 = 2211;
const ST_L_RM212: u32 = 2212;
const ST_L_RM213: u32 = 2213;
const ST_L_RM214: u32 = 2214;
const ST_L_RM215: u32 = 2215;
const ST_L_RM216: u32 = 2216;
const ST_L_RM217: u32 = 2217;
const ST_L_RM218: u32 = 2218;
const ST_L_RM219: u32 = 2219;
const ST_L_RM220: u32 = 2220;
const ST_L_RM221: u32 = 2221;
const ST_L_RM222: u32 = 2222;
const ST_L_RM223: u32 = 2223;
const ST_L_RM224: u32 = 2224;

/* The other labels in the C switch statement. Extra states that an individual
opcode group needs internally may be added in the 4000+ range by the code that
needs them. */

const ST_REPEATCHAR: u32 = 3001;
const ST_REPEATNOTCHAR: u32 = 3002;
const ST_REPEATTYPE: u32 = 3003;
const ST_REF_REPEAT: u32 = 3004;
const ST_POSSESSIVE_NON_CAPTURE: u32 = 3005;
const ST_POSSESSIVE_CAPTURE: u32 = 3006;
const ST_POSSESSIVE_GROUP: u32 = 3007;
const ST_GROUPLOOP: u32 = 3008;
const ST_ASSERT_NOT_FAILED: u32 = 3009;
const ST_ASSERT_NL_OR_EOS: u32 = 3010;
const ST_SCS_OFFSET_FOUND: u32 = 3011;
const ST_GOT_MAX: u32 = 3012;
const ST_ENDLOOP99: u32 = 3013;
const ST_ENDLOOP00: u32 = 3014;
const ST_ENDLOOP01: u32 = 3015;
const ST_ENDLOOP02: u32 = 3016;
const ST_ENDLOOP03: u32 = 3017;


/* Spare states for the individual opcode-group fragments. Fragment k may use
ST_Ck_1 .. ST_Ck_8 for control flow that needs extra labels (e.g. the top of a
loop whose body contains an RMATCH). */

const ST_C1_1: u32 = 4011;
const ST_C1_2: u32 = 4012;
const ST_C1_3: u32 = 4013;
const ST_C1_4: u32 = 4014;
const ST_C1_5: u32 = 4015;
const ST_C1_6: u32 = 4016;
const ST_C1_7: u32 = 4017;
const ST_C1_8: u32 = 4018;

const ST_C2_1: u32 = 4021;
const ST_C2_2: u32 = 4022;
const ST_C2_3: u32 = 4023;
const ST_C2_4: u32 = 4024;
const ST_C2_5: u32 = 4025;
const ST_C2_6: u32 = 4026;
const ST_C2_7: u32 = 4027;
const ST_C2_8: u32 = 4028;

const ST_C3_1: u32 = 4031;
const ST_C3_2: u32 = 4032;
const ST_C3_3: u32 = 4033;
const ST_C3_4: u32 = 4034;
const ST_C3_5: u32 = 4035;
const ST_C3_6: u32 = 4036;
const ST_C3_7: u32 = 4037;
const ST_C3_8: u32 = 4038;

const ST_C4_1: u32 = 4041;
const ST_C4_2: u32 = 4042;
const ST_C4_3: u32 = 4043;
const ST_C4_4: u32 = 4044;
const ST_C4_5: u32 = 4045;
const ST_C4_6: u32 = 4046;
const ST_C4_7: u32 = 4047;
const ST_C4_8: u32 = 4048;

const ST_C5_1: u32 = 4051;
const ST_C5_2: u32 = 4052;
const ST_C5_3: u32 = 4053;
const ST_C5_4: u32 = 4054;
const ST_C5_5: u32 = 4055;
const ST_C5_6: u32 = 4056;
const ST_C5_7: u32 = 4057;
const ST_C5_8: u32 = 4058;

const ST_C6_1: u32 = 4061;
const ST_C6_2: u32 = 4062;
const ST_C6_3: u32 = 4063;
const ST_C6_4: u32 = 4064;
const ST_C6_5: u32 = 4065;
const ST_C6_6: u32 = 4066;
const ST_C6_7: u32 = 4067;
const ST_C6_8: u32 = 4068;

const ST_C7_1: u32 = 4071;
const ST_C7_2: u32 = 4072;
const ST_C7_3: u32 = 4073;
const ST_C7_4: u32 = 4074;
const ST_C7_5: u32 = 4075;
const ST_C7_6: u32 = 4076;
const ST_C7_7: u32 = 4077;
const ST_C7_8: u32 = 4078;

const ST_C8_1: u32 = 4081;
const ST_C8_2: u32 = 4082;
const ST_C8_3: u32 = 4083;
const ST_C8_4: u32 = 4084;
const ST_C8_5: u32 = 4085;
const ST_C8_6: u32 = 4086;
const ST_C8_7: u32 = 4087;
const ST_C8_8: u32 = 4088;

const ST_C9_1: u32 = 4091;
const ST_C9_2: u32 = 4092;
const ST_C9_3: u32 = 4093;
const ST_C9_4: u32 = 4094;
const ST_C9_5: u32 = 4095;
const ST_C9_6: u32 = 4096;
const ST_C9_7: u32 = 4097;
const ST_C9_8: u32 = 4098;

const ST_C10_1: u32 = 4101;
const ST_C10_2: u32 = 4102;
const ST_C10_3: u32 = 4103;
const ST_C10_4: u32 = 4104;
const ST_C10_5: u32 = 4105;
const ST_C10_6: u32 = 4106;
const ST_C10_7: u32 = 4107;
const ST_C10_8: u32 = 4108;

const ST_C11_1: u32 = 4111;
const ST_C11_2: u32 = 4112;
const ST_C11_3: u32 = 4113;
const ST_C11_4: u32 = 4114;
const ST_C11_5: u32 = 4115;
const ST_C11_6: u32 = 4116;
const ST_C11_7: u32 = 4117;
const ST_C11_8: u32 = 4118;

const ST_C12_1: u32 = 4121;
const ST_C12_2: u32 = 4122;
const ST_C12_3: u32 = 4123;
const ST_C12_4: u32 = 4124;
const ST_C12_5: u32 = 4125;
const ST_C12_6: u32 = 4126;
const ST_C12_7: u32 = 4127;
const ST_C12_8: u32 = 4128;

const ST_C13_1: u32 = 4131;
const ST_C13_2: u32 = 4132;
const ST_C13_3: u32 = 4133;
const ST_C13_4: u32 = 4134;
const ST_C13_5: u32 = 4135;
const ST_C13_6: u32 = 4136;
const ST_C13_7: u32 = 4137;
const ST_C13_8: u32 = 4138;

/* Static helper functions of pcre2_match.c: do_callout(), match_ref() and
recurse_update_offsets(). */
include!("pcre2_match_helpers.rs");

/*************************************************
*         Match from current position            *
*************************************************/

/* This function is called to run one match attempt at a single starting point
in the subject.

Arguments:
   start_eptr   starting character in subject
   start_ecode  starting position in compiled code
   top_bracket  number of capturing parentheses in the pattern
   frame_size   size of each backtracking frame
   match_data   pointer to the match_data block
   mb           pointer to "static" variables block

Returns:        MATCH_MATCH if matched            )  these values are >= 0
                MATCH_NOMATCH if failed to match  )
                negative MATCH_xxx value for PRUNE, SKIP, etc
                negative PCRE2_ERROR_xxx value if aborted by an error condition
                (e.g. stopped by repeated call or depth limit)
*/

unsafe fn r#match(
    start_eptr: PCRE2_SPTR,
    start_ecode_arg: PCRE2_SPTR,
    top_bracket: u16,
    frame_size: PCRE2_SIZE,
    match_data: *mut pcre2_real_match_data,
    mb: *mut match_block,
) -> c_int {
    /* Frame-handling variables */

    let mut F: *mut heapframe; /* Current frame pointer */
    let mut N: *mut heapframe = core::ptr::null_mut(); /* Temporary frame pointers */
    let mut P: *mut heapframe = core::ptr::null_mut();

    let mut frames_top: *mut heapframe; /* End of frames vector */
    let mut assert_accept_frame: *mut heapframe = core::ptr::null_mut(); /* For passing back a frame with captures */
    let frame_copy_size: PCRE2_SIZE; /* Amount to copy when creating a new frame */

    /* Local variables that do not need to be preserved over calls to RMATCH(). */

    let mut branch_end: PCRE2_SPTR = core::ptr::null();
    let mut branch_start: PCRE2_SPTR = core::ptr::null();
    let mut bracode: PCRE2_SPTR = core::ptr::null(); /* Temp pointer to start of group */
    let mut offset: PCRE2_SIZE = 0; /* Used for group offsets */
    let mut length: PCRE2_SIZE = 0; /* Used for various length calculations */

    let mut rrc: c_int = 0; /* Return from functions & backtracking "recursions" */
    let mut proptype: c_int = 0; /* Type of character property */

    let mut i: u32 = 0; /* Used for local loops */
    let mut fc: u32 = 0; /* Character values */
    let mut number: u32 = 0; /* Used for group and other numbers */
    let mut reptype: u32 = 0; /* Type of repetition (0 to avoid compiler warning) */
    let mut group_frame_type: u32; /* Specifies type for new group frames */

    let mut condition: BOOL = FALSE; /* Used in conditional groups */
    let mut cur_is_word: BOOL = FALSE; /* Used in "word" tests */
    let mut prev_is_word: BOOL = FALSE; /* Used in "word" tests */

    /* The code pointer for the next RMATCH "recursion" */
    let mut start_ecode: PCRE2_SPTR = start_ecode_arg;

    /* UTF and UCP flags */

    let utf: BOOL = (((*mb).poptions & PCRE2_UTF) != 0) as BOOL;
    let ucp: BOOL = (((*mb).poptions & PCRE2_UCP) != 0) as BOOL;

    /* This is the length of the last part of a backtracking frame that must be
    copied when a new frame is created. */

    frame_copy_size = frame_size - offset_of!(heapframe, eptr);

    /* Set up the first frame and the end of the frames vector. */

    F = (*match_data).heapframes;
    frames_top = ((*match_data).heapframes as *mut u8).add((*match_data).heapframes_size)
        as *mut heapframe;

    (*F).rdepth = 0; /* "Recursion" depth */
    (*F).capture_last = 0; /* Number of most recent capture */
    (*F).current_recurse = RECURSE_UNSET; /* Not pattern recursing. */
    (*F).eptr = start_eptr; /* Current data pointer */
    (*F).start_match = start_eptr; /* and start match */
    (*F).mark = core::ptr::null(); /* Most recent mark */
    (*F).offset_top = 0; /* End of captures within the frame */
    (*F).last_group_offset = PCRE2_UNSET; /* Saved frame of most recent group */
    group_frame_type = 0; /* Not a start of group frame */

    let mut state: u32 = ST_NEW_FRAME; /* Start processing with this frame */

    'sm: loop {
        /* ------------------------------------------------------------------
        Local macros standing in for the C macros of the same name. Note that
        RMATCH() and RRETURN() transfer control, so the code that follows an
        RMATCH() in the C source lives in the ST_L_RMnnn state.
        ------------------------------------------------------------------ */

        macro_rules! CHECK_PARTIAL {
            () => {
                if (*F).eptr >= (*mb).end_subject {
                    SCHECK_PARTIAL!();
                }
            };
        }

        macro_rules! SCHECK_PARTIAL {
            () => {
                if (*mb).partial != 0
                    && ((*F).eptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
                {
                    (*mb).hitend = TRUE;
                    if (*mb).partial > 1 {
                        return PCRE2_ERROR_PARTIAL;
                    }
                }
            };
        }

        macro_rules! RMATCH {
            ($ra:expr, $rb:expr) => {{
                start_ecode = $ra;
                (*F).return_id = $rb;
                state = ST_MATCH_RECURSE;
                continue 'sm;
            }};
        }

        macro_rules! RRETURN {
            ($ra:expr) => {{
                rrc = $ra;
                state = ST_RETURN_SWITCH;
                continue 'sm;
            }};
        }

        /* Fovector[n] */
        macro_rules! Fov {
            ($i:expr) => {
                *(*F).ovector.as_mut_ptr().add($i as usize)
            };
        }

        /* Frame machinery states. */

        match state {
            ST_MATCH_RECURSE => {
                /* Set up a new backtracking frame. If the vector is full, get a new one,
                doubling the size, but constrained by the heap limit (which is in KiB). */

                N = (F as *mut u8).add(frame_size) as *mut heapframe;
                if (N as *mut u8).add(frame_size) as *mut heapframe >= frames_top {
                    let new: *mut c_void;
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

                    /* With a heap limit set, the permitted additional size may not be enough
                    for another frame, so do a final check. */

                    if newsize - usedsize < frame_size {
                        return PCRE2_ERROR_HEAPLIMIT;
                    }
                    new = ((*match_data).memctl.malloc.unwrap())(
                        newsize,
                        (*match_data).memctl.memory_data,
                    );
                    if new.is_null() {
                        return PCRE2_ERROR_NOMEMORY;
                    }
                    memcpy(new, (*match_data).heapframes as *const c_void, usedsize);

                    N = (new as *mut u8).add(usedsize) as *mut heapframe;
                    F = (N as *mut u8).sub(frame_size) as *mut heapframe;

                    ((*match_data).memctl.free.unwrap())(
                        (*match_data).heapframes as *mut c_void,
                        (*match_data).memctl.memory_data,
                    );
                    (*match_data).heapframes = new as *mut heapframe;
                    (*match_data).heapframes_size = newsize;
                    frames_top = (new as *mut u8).add(newsize) as *mut heapframe;
                }

                /* Copy those fields that must be copied into the new frame, increase the
                "recursion" depth (i.e. the new frame's index) and then make the new frame
                current. */

                memcpy(
                    (N as *mut u8).add(offset_of!(heapframe, eptr)) as *mut c_void,
                    (F as *mut u8).add(offset_of!(heapframe, eptr)) as *const c_void,
                    frame_copy_size,
                );

                (*N).rdepth = (*F).rdepth + 1;
                F = N;

                state = ST_NEW_FRAME;
                continue 'sm;
            }

            ST_NEW_FRAME => {
                (*F).group_frame_type = group_frame_type;
                (*F).ecode = start_ecode; /* Starting code pointer */
                (*F).back_frame = frame_size; /* Default is go back one frame */

                /* If this is a special type of group frame, remember its offset for quick
                access at the end of the group. If this is a recursion, set a new current
                recursion value. */

                if group_frame_type != 0 {
                    (*F).last_group_offset = (F as *mut u8)
                        .offset_from((*match_data).heapframes as *mut u8)
                        as PCRE2_SIZE;
                    if GF_IDMASK(group_frame_type) == GF_RECURSE {
                        (*F).current_recurse = GF_DATAMASK(group_frame_type);
                    }
                    group_frame_type = 0;
                }

                /* This is the main processing loop. First check that we haven't recorded
                too many backtracks (search tree is too large), or that we haven't exceeded
                the recursive depth limit (used too many backtracking frames). If not,
                process the opcodes. */

                let mcc = (*mb).match_call_count;
                (*mb).match_call_count = mcc + 1;
                if mcc >= (*mb).match_limit {
                    return PCRE2_ERROR_MATCHLIMIT;
                }
                if (*F).rdepth >= (*mb).match_limit_depth {
                    return PCRE2_ERROR_DEPTHLIMIT;
                }

                state = ST_TOP;
                continue 'sm;
            }

            ST_TOP => {
                (*F).op = *(*F).ecode; /* Cast needed for 16-bit and 32-bit modes */
                state = (*F).op as u32;
                continue 'sm;
            }

            ST_RETURN_SWITCH => {
                /* The RRETURN() macro jumps here. The number that is saved in Freturn_id
                indicates which label we actually want to return to. The value in Frdepth
                is the index number of the frame in the vector. The return value has been
                placed in rrc. */

                if (*F).eptr > (*mb).last_used_ptr {
                    (*mb).last_used_ptr = (*F).eptr;
                }
                if (*F).rdepth == 0 {
                    return rrc; /* Exit from the top level */
                }
                F = (F as *mut u8).sub((*F).back_frame) as *mut heapframe; /* Backtrack */
                (*(*mb).cb).callout_flags |= PCRE2_CALLOUT_BACKTRACK; /* Note for callouts */

                state = ST_L_RM_BASE + (*F).return_id as u32;
                continue 'sm;
            }

            _ => {}
        }

        /* The opcode handlers and the L_RMnnn resume points, in the order in which
        they appear in the C source. */

        include!("pcre2_match_ops1.rs");
        include!("pcre2_match_ops2.rs");
        include!("pcre2_match_ops3.rs");
        include!("pcre2_match_ops4.rs");
        include!("pcre2_match_ops5.rs");
        include!("pcre2_match_ops6.rs");
        include!("pcre2_match_ops7.rs");
        include!("pcre2_match_ops8.rs");
        include!("pcre2_match_ops9.rs");
        include!("pcre2_match_ops10.rs");
        include!("pcre2_match_ops11.rs");
        include!("pcre2_match_ops12.rs");
        include!("pcre2_match_ops13.rs");

        /* ===================================================================== */
        /* There's been some horrible disaster. Arrival here can only mean there is
        something seriously wrong in the code above or the OP_xxx definitions. */

        return PCRE2_ERROR_INTERNAL;
    }
}

/*************************************************
*           Match a Regular Expression           *
*************************************************/

include!("pcre2_match_public.rs");
