//! File-scope definitions of `pcre2_match.c`, shared between the Rust modules
//! into which that file has been split.

use crate::internal::*;

/// `PUBLIC_MATCH_OPTIONS`.
pub const PUBLIC_MATCH_OPTIONS: u32 = (PCRE2_ANCHORED
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
    | PCRE2_DISABLE_RECURSELOOP_CHECK) as u32;

// --- Returns from the match() function -------------------------------------

pub const MATCH_MATCH: c_int_ = 1;
pub const MATCH_NOMATCH: c_int_ = 0;

pub const MATCH_ACCEPT: c_int_ = -999;
pub const MATCH_KETRPOS: c_int_ = -998;

pub const MATCH_COMMIT: c_int_ = -997;
pub const MATCH_PRUNE: c_int_ = -996;
pub const MATCH_SKIP: c_int_ = -995;
pub const MATCH_SKIP_ARG: c_int_ = -994;
pub const MATCH_THEN: c_int_ = -993;
pub const MATCH_BACKTRACK_MAX: c_int_ = MATCH_THEN;
pub const MATCH_BACKTRACK_MIN: c_int_ = MATCH_COMMIT;

type c_int_ = core::ffi::c_int;

// --- Group frame type values -----------------------------------------------

pub const GF_CAPTURE: u32 = 0x00010000;
pub const GF_NOCAPTURE: u32 = 0x00020000;
pub const GF_CONDASSERT: u32 = 0x00030000;
pub const GF_RECURSE: u32 = 0x00040000;

/// `GF_IDMASK(a)`.
#[inline(always)]
pub const fn GF_IDMASK(a: u32) -> u32 {
    a & 0xffff0000
}

/// `GF_DATAMASK(a)`.
#[inline(always)]
pub const fn GF_DATAMASK(a: u32) -> u32 {
    a & 0x0000ffff
}

// --- Repetition types ------------------------------------------------------

pub const REPTYPE_MIN: u32 = 0;
pub const REPTYPE_MAX: u32 = 1;
pub const REPTYPE_POS: u32 = 2;

/// `rep_min` — minimum values for the common repeats.
pub static REP_MIN: [u32; 11] = [
    0, 0, // * and *?
    1, 1, // + and +?
    0, 0, // ? and ??
    0, 0, // dummy placefillers for OP_CR[MIN]RANGE
    0, 1, 0, // OP_CRPOS{STAR, PLUS, QUERY}
];

/// `rep_max` — maximum values for the common repeats; `UINT32_MAX` is infinity.
pub static REP_MAX: [u32; 11] = [
    u32::MAX,
    u32::MAX, // * and *?
    u32::MAX,
    u32::MAX, // + and +?
    1,
    1, // ? and ??
    0,
    0, // dummy placefillers for OP_CR[MIN]RANGE
    u32::MAX,
    u32::MAX,
    1, // OP_CRPOS{STAR, PLUS, QUERY}
];

/// `rep_typ` — repetition types, including `OP_CRPOSRANGE`.
pub static REP_TYP: [u32; 12] = [
    REPTYPE_MAX, REPTYPE_MIN, // * and *?
    REPTYPE_MAX, REPTYPE_MIN, // + and +?
    REPTYPE_MAX, REPTYPE_MIN, // ? and ??
    REPTYPE_MAX, REPTYPE_MIN, // OP_CRRANGE and OP_CRMINRANGE
    REPTYPE_POS, REPTYPE_POS, // OP_CRPOSSTAR, OP_CRPOSPLUS
    REPTYPE_POS, REPTYPE_POS, // OP_CRPOSQUERY, OP_CRPOSRANGE
];

// --- Numbers for RMATCH calls at backtracking points ------------------------

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

// SUPPORT_WIDE_CHARS
pub const RM100: u8 = 100;
pub const RM101: u8 = 101;
pub const RM102: u8 = 102;
pub const RM103: u8 = 103;

// SUPPORT_UNICODE
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
