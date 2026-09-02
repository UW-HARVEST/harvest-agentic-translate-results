//! Translation of `pcre2_dfa_match.c` — the DFA (alternative) matching engine.
//!
//! 8-bit mode, `SUPPORT_UNICODE` on, `SUPPORT_JIT` off. This mirrors the C
//! source closely, using raw-pointer arithmetic to reproduce the state-machine
//! behaviour faithfully.

use crate::internal::*;

// ---------------------------------------------------------------------------
// Local numeric constants, narrowed to the natural Rust types. The values in
// `consts.rs` are typed `i64`; the state machine works in `u32`/`i32`, so we
// re-declare the handful we compare against here.
// ---------------------------------------------------------------------------

// CHAR constants (ASCII, non-EBCDIC build).
const CHAR_NUL: u32 = 0x00;
const CHAR_HT: u32 = 0x09;
const CHAR_LF: u32 = 0x0a;
const CHAR_VT: u32 = 0x0b;
const CHAR_FF: u32 = 0x0c;
const CHAR_CR: u32 = 0x0d;
const CHAR_NL: u32 = CHAR_LF;
const CHAR_NEL: u32 = 0x85;
const CHAR_SPACE: u32 = 0x20;
const CHAR_NBSP: u32 = 0xa0;
const CHAR_DOLLAR_SIGN: u32 = 0x24;
const CHAR_COMMERCIAL_AT: u32 = 0x40;
const CHAR_GRAVE_ACCENT: u32 = 0x60;

const PCRE2_MATCHEDBY_DFA_INTERPRETER: u8 = 1;

// Offsets used to turn OP_TYPESTAR and friends into other opcodes.
const OP_PROP_EXTRA: u32 = 300;
const OP_EXTUNI_EXTRA: u32 = 320;
const OP_ANYNL_EXTRA: u32 = 340;
const OP_HSPACE_EXTRA: u32 = 360;
const OP_VSPACE_EXTRA: u32 = 380;

// Widths.
const LINK_SIZE: usize = LINK_SIZE_U;
const IMM2_SIZE: usize = IMM2_SIZE_U;
const NOTACHAR: u32 = NOTACHAR_U;

// Publicly accepted option bits for pcre2_dfa_match().
const PUBLIC_DFA_MATCH_OPTIONS: u32 = (PCRE2_ANCHORED
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
    | PCRE2_COPY_MATCHED_SUBJECT) as u32;

// ---------------------------------------------------------------------------
// Convenience: local u32 copies of the opcode / property / error constants
// used as `match` arms, since `consts.rs` types them `i64`.
// ---------------------------------------------------------------------------

macro_rules! ops {
    ($($name:ident),* $(,)?) => {
        $( #[allow(non_upper_case_globals)] const $name: u32 = crate::consts::$name as u32; )*
    };
}

ops!(
    OP_END, OP_ALT, OP_KET, OP_KETRMIN, OP_KETRMAX, OP_KETRPOS, OP_BRA, OP_SBRA, OP_CBRA, OP_SCBRA,
    OP_BRAZERO, OP_BRAMINZERO, OP_SKIPZERO, OP_CIRC, OP_CIRCM, OP_EOD, OP_SOD, OP_SOM, OP_EODN,
    OP_DOLL, OP_DOLLM, OP_ANY, OP_ALLANY, OP_ANYBYTE, OP_DIGIT, OP_WHITESPACE, OP_WORDCHAR,
    OP_NOT_DIGIT, OP_NOT_WHITESPACE, OP_NOT_WORDCHAR, OP_WORD_BOUNDARY, OP_NOT_WORD_BOUNDARY,
    OP_NOT_UCP_WORD_BOUNDARY, OP_UCP_WORD_BOUNDARY, OP_PROP, OP_NOTPROP, OP_ANYNL, OP_EXTUNI,
    OP_HSPACE, OP_NOT_HSPACE, OP_VSPACE, OP_NOT_VSPACE, OP_TYPESTAR, OP_TYPEMINSTAR, OP_TYPEPOSSTAR,
    OP_TYPEPLUS, OP_TYPEMINPLUS, OP_TYPEPOSPLUS, OP_TYPEQUERY, OP_TYPEMINQUERY, OP_TYPEPOSQUERY,
    OP_TYPEEXACT, OP_TYPEUPTO, OP_TYPEMINUPTO, OP_TYPEPOSUPTO, OP_CHAR, OP_CHARI, OP_NOT, OP_NOTI,
    OP_STAR, OP_STARI, OP_MINSTAR, OP_MINSTARI, OP_POSSTAR, OP_POSSTARI, OP_PLUS, OP_PLUSI,
    OP_MINPLUS, OP_MINPLUSI, OP_POSPLUS, OP_POSPLUSI, OP_QUERY, OP_QUERYI, OP_MINQUERY, OP_MINQUERYI,
    OP_POSQUERY, OP_POSQUERYI, OP_UPTO, OP_UPTOI, OP_MINUPTO, OP_MINUPTOI, OP_POSUPTO, OP_POSUPTOI,
    OP_EXACT, OP_EXACTI, OP_NOTSTAR, OP_NOTSTARI, OP_NOTMINSTAR, OP_NOTMINSTARI, OP_NOTPOSSTAR,
    OP_NOTPOSSTARI, OP_NOTPLUS, OP_NOTPLUSI, OP_NOTMINPLUS, OP_NOTMINPLUSI, OP_NOTPOSPLUS,
    OP_NOTPOSPLUSI, OP_NOTQUERY, OP_NOTQUERYI, OP_NOTMINQUERY, OP_NOTMINQUERYI, OP_NOTPOSQUERY,
    OP_NOTPOSQUERYI, OP_NOTUPTO, OP_NOTUPTOI, OP_NOTMINUPTO, OP_NOTMINUPTOI, OP_NOTPOSUPTO,
    OP_NOTPOSUPTOI, OP_NOTEXACT, OP_NOTEXACTI, OP_CLASS, OP_NCLASS, OP_XCLASS, OP_ECLASS,
    OP_CRSTAR, OP_CRMINSTAR, OP_CRPOSSTAR, OP_CRPLUS, OP_CRMINPLUS, OP_CRPOSPLUS, OP_CRQUERY,
    OP_CRMINQUERY, OP_CRPOSQUERY, OP_CRRANGE, OP_CRMINRANGE, OP_CRPOSRANGE, OP_FAIL, OP_ASSERT,
    OP_ASSERT_NOT, OP_ASSERTBACK, OP_ASSERTBACK_NOT, OP_COND, OP_SCOND, OP_RECURSE, OP_ONCE,
    OP_BRAPOS, OP_SBRAPOS, OP_CBRAPOS, OP_SCBRAPOS, OP_BRAPOSZERO, OP_CALLOUT, OP_CALLOUT_STR,
    OP_REVERSE, OP_CREF, OP_DNCREF, OP_DNRREF, OP_RREF, OP_FALSE, OP_TRUE,
);

// Property-type constants (PT_*).
#[allow(non_upper_case_globals)]
mod pt {
    pub const PT_LAMP: u32 = crate::consts::PT_LAMP as u32;
    pub const PT_GC: u32 = crate::consts::PT_GC as u32;
    pub const PT_PC: u32 = crate::consts::PT_PC as u32;
    pub const PT_SC: u32 = crate::consts::PT_SC as u32;
    pub const PT_SCX: u32 = crate::consts::PT_SCX as u32;
    pub const PT_ALNUM: u32 = crate::consts::PT_ALNUM as u32;
    pub const PT_SPACE: u32 = crate::consts::PT_SPACE as u32;
    pub const PT_PXSPACE: u32 = crate::consts::PT_PXSPACE as u32;
    pub const PT_WORD: u32 = crate::consts::PT_WORD as u32;
    pub const PT_CLIST: u32 = crate::consts::PT_CLIST as u32;
    pub const PT_UCNC: u32 = crate::consts::PT_UCNC as u32;
    pub const PT_BIDICL: u32 = crate::consts::PT_BIDICL as u32;
    pub const PT_BOOL: u32 = crate::consts::PT_BOOL as u32;
}

#[allow(non_upper_case_globals)]
mod ucp {
    pub const ucp_L: u32 = crate::consts::ucp_L;
    pub const ucp_N: u32 = crate::consts::ucp_N;
    pub const ucp_Z: u32 = crate::consts::ucp_Z;
    pub const ucp_Lu: u32 = crate::consts::ucp_Lu;
    pub const ucp_Ll: u32 = crate::consts::ucp_Ll;
    pub const ucp_Lt: u32 = crate::consts::ucp_Lt;
    pub const ucp_Mn: u32 = crate::consts::ucp_Mn;
    pub const ucp_Pc: u32 = crate::consts::ucp_Pc;
}

// Error codes (as i32 for the `int` return type).
#[allow(non_upper_case_globals)]
mod err {
    pub const NOMATCH: i32 = crate::consts::PCRE2_ERROR_NOMATCH as i32;
    pub const PARTIAL: i32 = crate::consts::PCRE2_ERROR_PARTIAL as i32;
    pub const NULL: i32 = crate::consts::PCRE2_ERROR_NULL as i32;
    pub const BADOPTION: i32 = crate::consts::PCRE2_ERROR_BADOPTION as i32;
    pub const BADOFFSET: i32 = crate::consts::PCRE2_ERROR_BADOFFSET as i32;
    pub const DFA_UITEM: i32 = crate::consts::PCRE2_ERROR_DFA_UITEM as i32;
    pub const DFA_UCOND: i32 = crate::consts::PCRE2_ERROR_DFA_UCOND as i32;
    pub const DFA_UINVALID_UTF: i32 = crate::consts::PCRE2_ERROR_DFA_UINVALID_UTF as i32;
    pub const BADMAGIC: i32 = crate::consts::PCRE2_ERROR_BADMAGIC as i32;
    pub const BADMODE: i32 = crate::consts::PCRE2_ERROR_BADMODE as i32;
    pub const BADUTFOFFSET: i32 = crate::consts::PCRE2_ERROR_BADUTFOFFSET as i32;
    pub const DFA_BADRESTART: i32 = crate::consts::PCRE2_ERROR_DFA_BADRESTART as i32;
    pub const HEAPLIMIT: i32 = crate::consts::PCRE2_ERROR_HEAPLIMIT as i32;
    pub const NOMEMORY: i32 = crate::consts::PCRE2_ERROR_NOMEMORY as i32;
    pub const MATCHLIMIT: i32 = crate::consts::PCRE2_ERROR_MATCHLIMIT as i32;
    pub const DEPTHLIMIT: i32 = crate::consts::PCRE2_ERROR_DEPTHLIMIT as i32;
    pub const DFA_WSSIZE: i32 = crate::consts::PCRE2_ERROR_DFA_WSSIZE as i32;
    pub const DFA_RECURSE: i32 = crate::consts::PCRE2_ERROR_DFA_RECURSE as i32;
    pub const RECURSELOOP: i32 = crate::consts::PCRE2_ERROR_RECURSELOOP as i32;
    pub const INTERNAL: i32 = crate::consts::PCRE2_ERROR_INTERNAL as i32;
    pub const BADOFFSETLIMIT: i32 = crate::consts::PCRE2_ERROR_BADOFFSETLIMIT as i32;
}

// ctype bits & table offsets.
const CTYPE_DIGIT: u8 = crate::consts::ctype_digit as u8;
const CTYPE_SPACE: u8 = crate::consts::ctype_space as u8;
const CTYPE_WORD: u8 = crate::consts::ctype_word as u8;
const CTYPES_OFFSET: usize = crate::consts::ctypes_offset as usize;
const LCC_OFFSET: usize = crate::consts::lcc_offset as usize;
const FCC_OFFSET: usize = crate::consts::fcc_offset as usize;

const RREF_ANY: u32 = crate::consts::RREF_ANY as u32;
const REQ_CU_MAX: usize = crate::consts::REQ_CU_MAX as usize;
const MAX_UTF_CODE_POINT: u32 = crate::consts::MAX_UTF_CODE_POINT as u32;

// Newline conventions (from re->newline_convention, a u16).
const PCRE2_NEWLINE_CR: u16 = crate::consts::PCRE2_NEWLINE_CR as u16;
const PCRE2_NEWLINE_LF: u16 = crate::consts::PCRE2_NEWLINE_LF as u16;
const PCRE2_NEWLINE_NUL: u16 = crate::consts::PCRE2_NEWLINE_NUL as u16;
const PCRE2_NEWLINE_CRLF: u16 = crate::consts::PCRE2_NEWLINE_CRLF as u16;
const PCRE2_NEWLINE_ANY: u16 = crate::consts::PCRE2_NEWLINE_ANY as u16;
const PCRE2_NEWLINE_ANYCRLF: u16 = crate::consts::PCRE2_NEWLINE_ANYCRLF as u16;

const NLTYPE_FIXED: u32 = crate::consts::NLTYPE_FIXED as u32;
const NLTYPE_ANY: u32 = crate::consts::NLTYPE_ANY as u32;
const NLTYPE_ANYCRLF: u32 = crate::consts::NLTYPE_ANYCRLF as u32;

const PCRE2_BSR_ANYCRLF: u16 = crate::consts::PCRE2_BSR_ANYCRLF as u16;

// Match-time / pattern option bit masks (u32).
const O_PARTIAL_HARD: u32 = PCRE2_PARTIAL_HARD as u32;
const O_PARTIAL_SOFT: u32 = PCRE2_PARTIAL_SOFT as u32;
const O_NOTBOL: u32 = PCRE2_NOTBOL as u32;
const O_NOTEOL: u32 = PCRE2_NOTEOL as u32;
const O_NOTEMPTY: u32 = PCRE2_NOTEMPTY as u32;
const O_NOTEMPTY_ATSTART: u32 = PCRE2_NOTEMPTY_ATSTART as u32;
const O_DFA_SHORTEST: u32 = PCRE2_DFA_SHORTEST as u32;
const O_DFA_RESTART: u32 = PCRE2_DFA_RESTART as u32;
const O_ANCHORED: u32 = PCRE2_ANCHORED as u32;
const O_ENDANCHORED: u32 = PCRE2_ENDANCHORED as u32;
const O_UTF: u32 = PCRE2_UTF as u32;
const O_UCP: u32 = PCRE2_UCP as u32;
const O_ALT_CIRCUMFLEX: u32 = PCRE2_ALT_CIRCUMFLEX as u32;
const O_DOLLAR_ENDONLY: u32 = PCRE2_DOLLAR_ENDONLY as u32;
const O_COPY_MATCHED_SUBJECT: u32 = PCRE2_COPY_MATCHED_SUBJECT as u32;
const O_NO_UTF_CHECK: u32 = PCRE2_NO_UTF_CHECK as u32;
const O_USE_OFFSET_LIMIT: u32 = PCRE2_USE_OFFSET_LIMIT as u32;
const O_MATCH_INVALID_UTF: u32 = PCRE2_MATCH_INVALID_UTF as u32;
const O_OPTIM_START_OPTIMIZE: u32 = PCRE2_OPTIM_START_OPTIMIZE as u32;

// Pattern flag bits (u32).
const F_STARTLINE: u32 = PCRE2_STARTLINE as u32;
const F_FIRSTLINE: u32 = PCRE2_FIRSTLINE as u32;
const F_FIRSTSET: u32 = PCRE2_FIRSTSET as u32;
const F_FIRSTCASELESS: u32 = PCRE2_FIRSTCASELESS as u32;
const F_FIRSTMAPSET: u32 = PCRE2_FIRSTMAPSET as u32;
const F_LASTSET: u32 = PCRE2_LASTSET as u32;
const F_LASTCASELESS: u32 = PCRE2_LASTCASELESS as u32;
const F_HASCRORLF: u32 = PCRE2_HASCRORLF as u32;
const F_MATCH_EMPTY: u32 = PCRE2_MATCH_EMPTY as u32;
const F_NOTEMPTY_SET: u32 = PCRE2_NOTEMPTY_SET as u32;
const F_NE_ATST_SET: u32 = PCRE2_NE_ATST_SET as u32;

const MD_COPIED_SUBJECT: u8 = PCRE2_MD_COPIED_SUBJECT as u8;
const PCRE2_MODE_MASK: u32 = crate::consts::PCRE2_MODE_MASK as u32;
const MAGIC_NUMBER: u32 = crate::consts::MAGIC_NUMBER as u32;

// ---------------------------------------------------------------------------
// Static tables
// ---------------------------------------------------------------------------

// coptable: offset from the opcode where an inline character/argument is found.
static COPTABLE: [u8; OP_TABLE_LENGTH as usize] = {
    const A: u8 = 1 + IMM2_SIZE as u8;
    [
        0, // End
        0, 0, 0, 0, 0, // \A, \G, \K, \B, \b
        0, 0, 0, 0, 0, 0, // \D, \d, \S, \s, \W, \w
        0, 0, 0, // Any, AllAny, Anybyte
        0, 0, // \P, \p
        0, 0, 0, 0, 0, // \R, \H, \h, \V, \v
        0, // \X
        0, 0, 0, 0, 0, 0, // \Z, \z, $, $M, ^, ^M
        1, // Char
        1, // Chari
        1, // not
        1, // noti
        // Positive single-char repeats
        1, 1, 1, 1, 1, 1, // *, *?, +, +?, ?, ??
        A, A, // upto, minupto
        A, // exact
        1, 1, 1, A, // *+, ++, ?+, upto+
        1, 1, 1, 1, 1, 1, // *I, *?I, +I, +?I, ?I, ??I
        A, A, // upto I, minupto I
        A, // exact I
        1, 1, 1, A, // *+I, ++I, ?+I, upto+I
        // Negative single-char repeats - only for chars < 256
        1, 1, 1, 1, 1, 1, // NOT *, *?, +, +?, ?, ??
        A, A, // NOT upto, minupto
        A, // NOT exact
        1, 1, 1, A, // NOT *+, ++, ?+, upto+
        1, 1, 1, 1, 1, 1, // NOT *I, *?I, +I, +?I, ?I, ??I
        A, A, // NOT upto I, minupto I
        A, // NOT exact I
        1, 1, 1, A, // NOT *+I, ++I, ?+I, upto+I
        // Positive type repeats
        1, 1, 1, 1, 1, 1, // Type *, *?, +, +?, ?, ??
        A, A, // Type upto, minupto
        A, // Type exact
        1, 1, 1, A, // Type *+, ++, ?+, upto+
        // Character class & ref repeats
        0, 0, 0, 0, 0, 0, // *, *?, +, +?, ?, ??
        0, 0, // CRRANGE, CRMINRANGE
        0, 0, 0, 0, // Possessive *+, ++, ?+, CRPOSRANGE
        0, // CLASS
        0, // NCLASS
        0, // XCLASS - variable length
        0, // ECLASS - variable length
        0, // REF
        0, // REFI
        0, // DNREF
        0, // DNREFI
        0, // RECURSE
        0, // CALLOUT
        0, // CALLOUT_STR
        0, // Alt
        0, // Ket
        0, // KetRmax
        0, // KetRmin
        0, // KetRpos
        0, 0, // Reverse, Vreverse
        0, // Assert
        0, // Assert not
        0, // Assert behind
        0, // Assert behind not
        0, // NA assert
        0, // NA assert behind
        0, // Assert scan substring
        0, // ONCE
        0, // SCRIPT_RUN
        0, 0, 0, 0, 0, // BRA, BRAPOS, CBRA, CBRAPOS, COND
        0, 0, 0, 0, 0, // SBRA, SBRAPOS, SCBRA, SCBRAPOS, SCOND
        0, 0, // CREF, DNCREF
        0, 0, // RREF, DNRREF
        0, 0, // FALSE, TRUE
        0, 0, 0, // BRAZERO, BRAMINZERO, BRAPOSZERO
        0, 0, 0, // MARK, PRUNE, PRUNE_ARG
        0, 0, 0, 0, // SKIP, SKIP_ARG, THEN, THEN_ARG
        0, 0, // COMMIT, COMMIT_ARG
        0, 0, 0, // FAIL, ACCEPT, ASSERT_ACCEPT
        0, 0, 0, // CLOSE, SKIPZERO, DEFINE
        0, 0, // \B and \b in UCP mode
    ]
};

// poptable: opcodes that inspect a character.
static POPTABLE: [u8; OP_TABLE_LENGTH as usize] = [
    0, // End
    0, 0, 0, 1, 1, // \A, \G, \K, \B, \b
    1, 1, 1, 1, 1, 1, // \D, \d, \S, \s, \W, \w
    1, 1, 1, // Any, AllAny, Anybyte
    1, 1, // \P, \p
    1, 1, 1, 1, 1, // \R, \H, \h, \V, \v
    1, // \X
    0, 0, 0, 0, 0, 0, // \Z, \z, $, $M, ^, ^M
    1, // Char
    1, // Chari
    1, // not
    1, // noti
    // Positive single-char repeats
    1, 1, 1, 1, 1, 1, // *, *?, +, +?, ?, ??
    1, 1, 1, // upto, minupto, exact
    1, 1, 1, 1, // *+, ++, ?+, upto+
    1, 1, 1, 1, 1, 1, // *I, *?I, +I, +?I, ?I, ??I
    1, 1, 1, // upto I, minupto I, exact I
    1, 1, 1, 1, // *+I, ++I, ?+I, upto+I
    // Negative single-char repeats - only for chars < 256
    1, 1, 1, 1, 1, 1, // NOT *, *?, +, +?, ?, ??
    1, 1, 1, // NOT upto, minupto, exact
    1, 1, 1, 1, // NOT *+, ++, ?+, upto+
    1, 1, 1, 1, 1, 1, // NOT *I, *?I, +I, +?I, ?I, ??I
    1, 1, 1, // NOT upto I, minupto I, exact I
    1, 1, 1, 1, // NOT *+I, ++I, ?+I, upto+I
    // Positive type repeats
    1, 1, 1, 1, 1, 1, // Type *, *?, +, +?, ?, ??
    1, 1, 1, // Type upto, minupto, exact
    1, 1, 1, 1, // Type *+, ++, ?+, upto+
    // Character class & ref repeats
    1, 1, 1, 1, 1, 1, // *, *?, +, +?, ?, ??
    1, 1, // CRRANGE, CRMINRANGE
    1, 1, 1, 1, // Possessive *+, ++, ?+, CRPOSRANGE
    1, // CLASS
    1, // NCLASS
    1, // XCLASS - variable length
    1, // ECLASS - variable length
    0, // REF
    0, // REFI
    0, // DNREF
    0, // DNREFI
    0, // RECURSE
    0, // CALLOUT
    0, // CALLOUT_STR
    0, // Alt
    0, // Ket
    0, // KetRmax
    0, // KetRmin
    0, // KetRpos
    0, 0, // Reverse, Vreverse
    0, // Assert
    0, // Assert not
    0, // Assert behind
    0, // Assert behind not
    0, // NA assert
    0, // NA assert behind
    0, // Assert scan substring
    0, // ONCE
    0, // SCRIPT_RUN
    0, 0, 0, 0, 0, // BRA, BRAPOS, CBRA, CBRAPOS, COND
    0, 0, 0, 0, 0, // SBRA, SBRAPOS, SCBRA, SCBRAPOS, SCOND
    0, 0, // CREF, DNCREF
    0, 0, // RREF, DNRREF
    0, 0, // FALSE, TRUE
    0, 0, 0, // BRAZERO, BRAMINZERO, BRAPOSZERO
    0, 0, 0, // MARK, PRUNE, PRUNE_ARG
    0, 0, 0, 0, // SKIP, SKIP_ARG, THEN, THEN_ARG
    0, 0, // COMMIT, COMMIT_ARG
    0, 0, 0, // FAIL, ACCEPT, ASSERT_ACCEPT
    0, 0, 0, // CLOSE, SKIPZERO, DEFINE
    1, 1, // \B and \b in UCP mode
];

// toptable1 / toptable2 for testing \D, \d, \S, \s, \W, \w.
static TOPTABLE1: [u8; 14] = [
    0, 0, 0, 0, 0, 0, CTYPE_DIGIT, CTYPE_DIGIT, CTYPE_SPACE, CTYPE_SPACE, CTYPE_WORD, CTYPE_WORD, 0,
    0, // OP_ANY, OP_ALLANY
];

static TOPTABLE2: [u8; 14] = [
    0, 0, 0, 0, 0, 0, CTYPE_DIGIT, 0, CTYPE_SPACE, 0, CTYPE_WORD, 0, 1, 1, // OP_ANY, OP_ALLANY
];

// ---------------------------------------------------------------------------
// State block and recursion workspace structures.
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
struct stateblock {
    offset: c_int_local, // Offset to opcode (-ve has meaning)
    count: c_int_local,  // Count for repeats
    data: c_int_local,   // Some use extra data
}

// The workspace is a vector of C `int`s.
#[allow(non_camel_case_types)]
type c_int_local = core::ffi::c_int;

const INTS_PER_STATEBLOCK: usize = core::mem::size_of::<stateblock>() / core::mem::size_of::<c_int_local>();

const OVEC_UNIT: usize = core::mem::size_of::<PCRE2_SIZE>() / core::mem::size_of::<c_int_local>();

const RWS_BASE_SIZE: usize = (crate::consts::DFA_START_RWS_SIZE as usize) / core::mem::size_of::<c_int_local>();
const RWS_RSIZE: usize = 1000;
const RWS_OVEC_RSIZE: usize = 1000 * OVEC_UNIT;
const RWS_OVEC_OSIZE: usize = 2 * OVEC_UNIT;

#[repr(C)]
struct RWS_anchor {
    next: *mut RWS_anchor,
    size: u32, // number of ints
    free: u32, // number of ints
}

const RWS_ANCHOR_SIZE: usize = core::mem::size_of::<RWS_anchor>() / core::mem::size_of::<c_int_local>();

// ---------------------------------------------------------------------------
// do_callout_dfa()
// ---------------------------------------------------------------------------

unsafe fn do_callout_dfa(
    code: PCRE2_SPTR,
    offsets: *mut PCRE2_SIZE,
    current_subject: PCRE2_SPTR,
    ptr: PCRE2_SPTR,
    mb: *mut dfa_match_block,
    extracode: PCRE2_SIZE,
    lengthptr: *mut PCRE2_SIZE,
) -> c_int_local {
    unsafe {
        let cb = (*mb).cb;

        *lengthptr = if *code.add(extracode) as u32 == OP_CALLOUT {
            crate::tables::_pcre2_OP_lengths[OP_CALLOUT as usize] as PCRE2_SIZE
        } else {
            GET(code, 1 + 2 * LINK_SIZE + extracode) as PCRE2_SIZE
        };

        if (*mb).callout.is_none() {
            return 0; // No callout provided
        }

        (*cb).offset_vector = offsets;
        (*cb).start_match = (current_subject as usize - (*mb).start_subject as usize) as PCRE2_SIZE;
        (*cb).current_position = (ptr as usize - (*mb).start_subject as usize) as PCRE2_SIZE;
        (*cb).pattern_position = GET(code, 1 + extracode) as PCRE2_SIZE;
        (*cb).next_item_length = GET(code, 1 + LINK_SIZE + extracode) as PCRE2_SIZE;

        if *code.add(extracode) as u32 == OP_CALLOUT {
            (*cb).callout_number = *code.add(1 + 2 * LINK_SIZE + extracode) as u32;
            (*cb).callout_string_offset = 0;
            (*cb).callout_string = core::ptr::null();
            (*cb).callout_string_length = 0;
        } else {
            (*cb).callout_number = 0;
            (*cb).callout_string_offset = GET(code, 1 + 3 * LINK_SIZE + extracode) as PCRE2_SIZE;
            (*cb).callout_string = code.add(1 + 4 * LINK_SIZE + extracode + 1);
            (*cb).callout_string_length = *lengthptr - (1 + 4 * LINK_SIZE) - 2;
        }

        ((*mb).callout.unwrap())(cb, (*mb).callout_data)
    }
}

// ---------------------------------------------------------------------------
// more_workspace()
// ---------------------------------------------------------------------------

unsafe fn more_workspace(
    rwsptr: *mut *mut RWS_anchor,
    ovecsize: u32,
    mb: *mut dfa_match_block,
) -> c_int_local {
    unsafe {
        let rws = *rwsptr;
        let new: *mut RWS_anchor;

        if !(*rws).next.is_null() {
            new = (*rws).next;
        } else {
            let mut newsize: u32 = if (*rws).size >= u32::MAX / (core::mem::size_of::<c_int_local>() as u32 * 2) {
                u32::MAX / core::mem::size_of::<c_int_local>() as u32
            } else {
                (*rws).size * 2
            };
            let mut newsize_k: u32 = newsize / (1024 / core::mem::size_of::<c_int_local>() as u32);

            if (newsize_k as u64 + (*mb).heap_used as u64) > (*mb).heap_limit as u64 {
                newsize_k = ((*mb).heap_limit as PCRE2_SIZE - (*mb).heap_used) as u32;
            }
            newsize = newsize_k * (1024 / core::mem::size_of::<c_int_local>() as u32);

            if (newsize as usize) < RWS_RSIZE + ovecsize as usize + RWS_ANCHOR_SIZE {
                return err::HEAPLIMIT;
            }
            let malloc = match (*mb).memctl.malloc {
                Some(f) => f,
                None => return err::NOMEMORY,
            };
            let p = malloc(
                newsize as usize * core::mem::size_of::<c_int_local>(),
                (*mb).memctl.memory_data,
            ) as *mut RWS_anchor;
            if p.is_null() {
                return err::NOMEMORY;
            }
            (*mb).heap_used += newsize_k as PCRE2_SIZE;
            (*p).next = core::ptr::null_mut();
            (*p).size = newsize;
            (*rws).next = p;
            new = p;
        }

        (*new).free = (*new).size - RWS_ANCHOR_SIZE as u32;
        *rwsptr = new;
        0
    }
}

// ---------------------------------------------------------------------------
// internal_dfa_match()
// ---------------------------------------------------------------------------

unsafe fn internal_dfa_match(
    mb: *mut dfa_match_block,
    this_start_code: PCRE2_SPTR,
    current_subject: PCRE2_SPTR,
    start_offset: PCRE2_SIZE,
    offsets: *mut PCRE2_SIZE,
    offsetcount: u32,
    workspace: *mut c_int_local,
    wscount: c_int_local,
    rlevel: u32,
    RWS: *mut c_int_local,
) -> c_int_local {
    unsafe {
        // Helpers to read a state block from an `*mut stateblock` base.
        #[inline(always)]
        unsafe fn sb_at(base: *mut stateblock, i: usize) -> *mut stateblock {
            unsafe { base.add(i) }
        }

        let mut current_subject = current_subject;
        let mut RWS = RWS;
        let mut rlevel = rlevel;
        let mut offsetcount = offsetcount;
        let mut wscount = wscount;

        let start_subject: PCRE2_SPTR = (*mb).start_subject;
        let end_subject: PCRE2_SPTR = (*mb).end_subject;
        let start_code: PCRE2_SPTR = (*mb).start_code;

        let utf: bool = ((*mb).poptions & O_UTF) != 0;
        let utf_or_ucp: bool = utf || ((*mb).poptions & O_UCP) != 0;

        let mut reset_could_continue = false;

        (*mb).match_call_count += 1;
        if (*mb).match_call_count - 1 >= (*mb).match_limit {
            return err::MATCHLIMIT;
        }
        let old_rlevel = rlevel;
        rlevel += 1;
        if old_rlevel > (*mb).match_limit_depth {
            return err::DEPTHLIMIT;
        }
        offsetcount &= (-2i32) as u32; // Round down

        wscount -= 2;
        wscount = (wscount - (wscount % (INTS_PER_STATEBLOCK as c_int_local * 2)))
            / (2 * INTS_PER_STATEBLOCK as c_int_local);
        let wscount_us = wscount as usize;

        let ctypes: *const u8 = (*mb).tables.add(CTYPES_OFFSET);
        let lcc: *const u8 = (*mb).tables.add(LCC_OFFSET);
        let fcc: *const u8 = (*mb).tables.add(FCC_OFFSET);

        let mut match_count: c_int_local = err::NOMATCH;

        // active_states / new_states are views into `workspace + 2`.
        let mut active_states: *mut stateblock = (workspace.add(2)) as *mut stateblock;
        let mut new_states: *mut stateblock = active_states.add(wscount_us);
        let mut next_new_state: *mut stateblock = new_states;
        let mut new_count: usize = 0;

        let mut active_count: usize = 0;
        let mut next_active_state: *mut stateblock = active_states; // placeholder

        // --- ADD_* helper macros ---------------------------------------
        // These mirror the C macros; on overflow they `return DFA_WSSIZE`.
        macro_rules! ADD_ACTIVE {
            ($x:expr, $y:expr) => {{
                if active_count < wscount_us {
                    active_count += 1;
                    (*next_active_state).offset = $x;
                    (*next_active_state).count = $y;
                    next_active_state = next_active_state.add(1);
                } else {
                    return err::DFA_WSSIZE;
                }
            }};
        }
        // Defined but never used, exactly as in pcre2_dfa_match.c.
        #[allow(unused_macros)]
        macro_rules! ADD_ACTIVE_DATA {
            ($x:expr, $y:expr, $z:expr) => {{
                if active_count < wscount_us {
                    active_count += 1;
                    (*next_active_state).offset = $x;
                    (*next_active_state).count = $y;
                    (*next_active_state).data = $z;
                    next_active_state = next_active_state.add(1);
                } else {
                    return err::DFA_WSSIZE;
                }
            }};
        }
        macro_rules! ADD_NEW {
            ($x:expr, $y:expr) => {{
                if new_count < wscount_us {
                    new_count += 1;
                    (*next_new_state).offset = $x;
                    (*next_new_state).count = $y;
                    next_new_state = next_new_state.add(1);
                } else {
                    return err::DFA_WSSIZE;
                }
            }};
        }
        macro_rules! ADD_NEW_DATA {
            ($x:expr, $y:expr, $z:expr) => {{
                if new_count < wscount_us {
                    new_count += 1;
                    (*next_new_state).offset = $x;
                    (*next_new_state).count = $y;
                    (*next_new_state).data = $z;
                    next_new_state = next_new_state.add(1);
                } else {
                    return err::DFA_WSSIZE;
                }
            }};
        }

        // IS_NEWLINE / WAS_NEWLINE, with NLBLOCK = mb, PSSTART = start_subject,
        // PSEND = end_subject.
        macro_rules! IS_NEWLINE {
            ($p:expr) => {{
                let p_: PCRE2_SPTR = $p;
                if (*mb).nltype != NLTYPE_FIXED {
                    p_ < end_subject
                        && crate::newline::_pcre2_is_newline_8(
                            p_,
                            (*mb).nltype,
                            end_subject,
                            &mut (*mb).nllen,
                            utf as BOOL,
                        ) != 0
                } else {
                    let nllen = (*mb).nllen as usize;
                    p_ as usize <= end_subject as usize - nllen
                        && *p_ == (*mb).nl[0]
                        && ((*mb).nllen == 1 || *p_.add(1) == (*mb).nl[1])
                }
            }};
        }
        macro_rules! WAS_NEWLINE {
            ($p:expr) => {{
                let p_: PCRE2_SPTR = $p;
                if (*mb).nltype != NLTYPE_FIXED {
                    p_ > start_subject
                        && crate::newline::_pcre2_was_newline_8(
                            p_,
                            (*mb).nltype,
                            start_subject,
                            &mut (*mb).nllen,
                            utf as BOOL,
                        ) != 0
                } else {
                    let nllen = (*mb).nllen as usize;
                    p_ as usize >= start_subject as usize + nllen
                        && *p_.sub(nllen) == (*mb).nl[0]
                        && ((*mb).nllen == 1 || *p_.sub(nllen).add(1) == (*mb).nl[1])
                }
            }};
        }

        // We need a place to hold end_code; it is set in the branch below.
        let end_code: PCRE2_SPTR;

        if *this_start_code as u32 == OP_ASSERTBACK
            || *this_start_code as u32 == OP_ASSERTBACK_NOT
        {
            let mut max_back: usize = 0;
            let gone_back: usize;

            let mut ec = this_start_code;
            loop {
                let back = GET2(ec, 2 + LINK_SIZE) as usize;
                if back > max_back {
                    max_back = back;
                }
                ec = ec.add(GET(ec, 1) as usize);
                if *ec as u32 != OP_ALT {
                    break;
                }
            }

            if utf {
                let mut gb = 0usize;
                while gb < max_back {
                    if current_subject <= start_subject {
                        break;
                    }
                    current_subject = current_subject.sub(1);
                    while current_subject > start_subject && (*current_subject & 0xc0) == 0x80 {
                        current_subject = current_subject.sub(1);
                    }
                    gb += 1;
                }
                gone_back = gb;
            } else {
                let current_offset = current_subject as usize - start_subject as usize;
                gone_back = if current_offset < max_back {
                    current_offset
                } else {
                    max_back
                };
                current_subject = current_subject.sub(gone_back);
            }

            if current_subject < (*mb).start_used_ptr {
                (*mb).start_used_ptr = current_subject;
            }

            let mut ec = this_start_code;
            loop {
                let revlen = if *ec.add(1 + LINK_SIZE) as u32 == OP_REVERSE {
                    1 + IMM2_SIZE
                } else {
                    0
                };
                let back = if revlen == 0 {
                    0usize
                } else {
                    GET2(ec, 2 + LINK_SIZE) as usize
                };
                if back <= gone_back {
                    let bstate =
                        (ec as usize - start_code as usize + 1 + LINK_SIZE + revlen) as c_int_local;
                    ADD_NEW_DATA!(-bstate, 0, (gone_back - back) as c_int_local);
                }
                ec = ec.add(GET(ec, 1) as usize);
                if *ec as u32 != OP_ALT {
                    break;
                }
            }
            end_code = ec;
        } else {
            let mut ec = this_start_code;

            if rlevel == 1 && ((*mb).moptions & O_DFA_RESTART) != 0 {
                loop {
                    ec = ec.add(GET(ec, 1) as usize);
                    if *ec as u32 != OP_ALT {
                        break;
                    }
                }
                new_count = *workspace.add(1) as usize;
                if *workspace.add(0) == 0 {
                    core::ptr::copy_nonoverlapping(
                        active_states,
                        new_states,
                        new_count,
                    );
                }
            } else {
                let mut length = 1
                    + LINK_SIZE
                    + if *this_start_code as u32 == OP_CBRA
                        || *this_start_code as u32 == OP_SCBRA
                        || *this_start_code as u32 == OP_CBRAPOS
                        || *this_start_code as u32 == OP_SCBRAPOS
                    {
                        IMM2_SIZE
                    } else {
                        0
                    };
                loop {
                    ADD_NEW!((ec as usize - start_code as usize + length) as c_int_local, 0);
                    ec = ec.add(GET(ec, 1) as usize);
                    length = 1 + LINK_SIZE;
                    if *ec as u32 != OP_ALT {
                        break;
                    }
                }
            }
            end_code = ec;
        }

        *workspace.add(0) = 0; // Bit indicating which vector is current

        // ---------------- Loop for scanning the subject ----------------
        let mut ptr: PCRE2_SPTR = current_subject;
        'subject: loop {
            let mut clen: c_int_local;
            let mut c: u32;
            let mut partial_newline = false;
            let mut could_continue = reset_could_continue;
            reset_could_continue = false;

            if ptr > (*mb).last_used_ptr {
                (*mb).last_used_ptr = ptr;
            }

            // Swap active/new lists.
            let temp_states = active_states;
            active_states = new_states;
            new_states = temp_states;
            active_count = new_count;
            new_count = 0;

            *workspace.add(0) ^= 1;
            *workspace.add(1) = active_count as c_int_local;

            next_active_state = active_states.add(active_count);
            next_new_state = new_states;

            if ptr < end_subject {
                clen = 1;
                let mut l: u32 = 1;
                c = GETCHARLENTEST(ptr, &mut l, utf);
                clen = l as c_int_local;
            } else {
                clen = 0;
                c = NOTACHAR;
            }

            // ---------- Scan up the active states ----------
            let mut i: isize = 0;
            while (i as usize) < active_count {
                let current_state: *mut stateblock = sb_at(active_states, i as usize);
                let mut caseless = false;
                let mut code: PCRE2_SPTR;
                let mut codevalue: u32;
                let mut state_offset: c_int_local = (*current_state).offset;
                let mut rrc: c_int_local;
                let mut count: c_int_local;

                // Negative offset: delayed (negated) state.
                if state_offset < 0 {
                    if (*current_state).data > 0 {
                        ADD_NEW_DATA!(state_offset, (*current_state).count, (*current_state).data - 1);
                        if could_continue {
                            reset_could_continue = true;
                        }
                        i += 1;
                        continue;
                    } else {
                        state_offset = -state_offset;
                        (*current_state).offset = state_offset;
                    }
                }

                // Duplicate-state check.
                let mut dup = false;
                {
                    let mut j: isize = 0;
                    while j < i {
                        let sj = sb_at(active_states, j as usize);
                        if (*sj).offset == state_offset && (*sj).count == (*current_state).count {
                            dup = true;
                            break;
                        }
                        j += 1;
                    }
                }
                if dup {
                    i += 1;
                    continue;
                }

                code = start_code.add(state_offset as usize);
                codevalue = *code as u32;

                if clen == 0 && POPTABLE[codevalue as usize] != 0 {
                    could_continue = true;
                }

                // Load inline argument if this opcode is followed by one.
                let mut dlen: c_int_local;
                let mut d: u32;
                if COPTABLE[codevalue as usize] > 0 {
                    dlen = 1;
                    if utf {
                        let mut l: u32 = 1;
                        d = GETCHARLEN(code.add(COPTABLE[codevalue as usize] as usize), &mut l);
                        dlen = l as c_int_local;
                    } else {
                        d = *code.add(COPTABLE[codevalue as usize] as usize) as u32;
                    }
                    if codevalue >= OP_TYPESTAR {
                        match d {
                            x if x == OP_ANYBYTE => return err::DFA_UITEM,
                            x if x == OP_NOTPROP || x == OP_PROP => codevalue += OP_PROP_EXTRA,
                            x if x == OP_ANYNL => codevalue += OP_ANYNL_EXTRA,
                            x if x == OP_EXTUNI => codevalue += OP_EXTUNI_EXTRA,
                            x if x == OP_NOT_HSPACE || x == OP_HSPACE => codevalue += OP_HSPACE_EXTRA,
                            x if x == OP_NOT_VSPACE || x == OP_VSPACE => codevalue += OP_VSPACE_EXTRA,
                            _ => {}
                        }
                    }
                } else {
                    dlen = 0;
                    d = NOTACHAR;
                }

                // Helper reproducing the repeated "switch(pt_type)" property
                // test used by OP_PROP/NOTPROP and the *_EXTRA type opcodes.
                // `pt_ptr` points to the PT_ selector byte, `pt_ptr[1]` is the
                // property value. Returns the boolean `OK`.
                macro_rules! prop_ok {
                    ($pt_ptr:expr, $cv_is_prop:expr) => {{
                        use pt::*;
                        use ucp::*;
                        let ptp: PCRE2_SPTR = $pt_ptr;
                        let prop = GET_UCD(c);
                        let sel = *ptp as u32;
                        let val = *ptp.add(1) as u32;
                        let ok: bool;
                        if sel == PT_LAMP {
                            let ct = prop.chartype as u32;
                            ok = ct == ucp_Lu || ct == ucp_Ll || ct == ucp_Lt;
                        } else if sel == PT_GC {
                            ok = crate::tables::_pcre2_ucp_gentype[prop.chartype as usize] == val;
                        } else if sel == PT_PC {
                            ok = prop.chartype as u32 == val;
                        } else if sel == PT_SC {
                            ok = prop.script as u32 == val;
                        } else if sel == PT_SCX {
                            ok = prop.script as u32 == val
                                || MAPBIT(
                                    crate::tables::_pcre2_ucd_script_sets
                                        .as_ptr()
                                        .add(UCD_SCRIPTX_PROP(prop) as usize),
                                    val,
                                ) != 0;
                        } else if sel == PT_ALNUM {
                            let ct = prop.chartype as u32;
                            ok = crate::tables::_pcre2_ucp_gentype[ct as usize] == ucp_L
                                || crate::tables::_pcre2_ucp_gentype[ct as usize] == ucp_N;
                        } else if sel == PT_SPACE || sel == PT_PXSPACE {
                            ok = match c {
                                CHAR_HT | CHAR_SPACE | CHAR_NBSP | 0x1680 | 0x180e | 0x2000
                                | 0x2001 | 0x2002 | 0x2003 | 0x2004 | 0x2005 | 0x2006 | 0x2007
                                | 0x2008 | 0x2009 | 0x200a | 0x202f | 0x205f | 0x3000 => true,
                                CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028
                                | 0x2029 => true,
                                _ => {
                                    crate::tables::_pcre2_ucp_gentype[prop.chartype as usize]
                                        == ucp_Z
                                }
                            };
                        } else if sel == PT_WORD {
                            let ct = prop.chartype as u32;
                            ok = crate::tables::_pcre2_ucp_gentype[ct as usize] == ucp_L
                                || crate::tables::_pcre2_ucp_gentype[ct as usize] == ucp_N
                                || ct == ucp_Mn
                                || ct == ucp_Pc;
                        } else if sel == PT_CLIST {
                            let mut cp = crate::tables::_pcre2_ucd_caseless_sets
                                .as_ptr()
                                .add(val as usize);
                            loop {
                                if c < *cp {
                                    ok = false;
                                    break;
                                }
                                let v = *cp;
                                cp = cp.add(1);
                                if c == v {
                                    ok = true;
                                    break;
                                }
                            }
                        } else if sel == PT_UCNC {
                            ok = c == CHAR_DOLLAR_SIGN
                                || c == CHAR_COMMERCIAL_AT
                                || c == CHAR_GRAVE_ACCENT
                                || (c >= 0xa0 && c <= 0xd7ff)
                                || c >= 0xe000;
                        } else if sel == PT_BIDICL {
                            ok = UCD_BIDICLASS(c) == val;
                        } else if sel == PT_BOOL {
                            ok = MAPBIT(
                                crate::tables::_pcre2_ucd_boolprop_sets
                                    .as_ptr()
                                    .add(UCD_BPROPS_PROP(prop) as usize),
                                val,
                            ) != 0;
                        } else {
                            ok = !$cv_is_prop;
                        }
                        ok
                    }};
                }

                // The C code uses `switch (codevalue)` with `break` to leave the
                // switch (== our match arm end) and `goto NEXT_ACTIVE_STATE`
                // (== fall through to the shared `i += 1; continue`). Where C
                // uses `goto QSn`/`goto ANYNLnn` for shared tails, we pre-set
                // `count` and share the block via a helper macro or duplicate.
                match codevalue {
                    // ---- Closing brackets ------------------------------
                    x if x == OP_KET || x == OP_KETRMIN || x == OP_KETRMAX || x == OP_KETRPOS => {
                        if code != end_code {
                            ADD_ACTIVE!(state_offset + 1 + LINK_SIZE as c_int_local, 0);
                            if codevalue != OP_KET {
                                ADD_ACTIVE!(state_offset - GET(code, 1) as c_int_local, 0);
                            }
                        } else if ptr > current_subject
                            || (((*mb).moptions & O_NOTEMPTY) == 0
                                && (((*mb).moptions & O_NOTEMPTY_ATSTART) == 0
                                    || current_subject
                                        > start_subject.add(
                                            (*mb).start_offset,
                                        )))
                        {
                            if match_count < 0 {
                                match_count = if offsetcount >= 2 { 1 } else { 0 };
                            } else if match_count > 0 && {
                                match_count += 1;
                                match_count * 2 > offsetcount as c_int_local
                            } {
                                match_count = 0;
                            }
                            count = (if match_count == 0 {
                                offsetcount as c_int_local
                            } else {
                                match_count * 2
                            }) - 2;
                            if count > 0 {
                                core::ptr::copy(
                                    offsets,
                                    offsets.add(2),
                                    count as usize,
                                );
                            }
                            if offsetcount >= 2 {
                                *offsets.add(0) =
                                    (current_subject as usize - start_subject as usize) as PCRE2_SIZE;
                                *offsets.add(1) =
                                    (ptr as usize - start_subject as usize) as PCRE2_SIZE;
                            }
                            if ((*mb).moptions & O_DFA_SHORTEST) != 0 {
                                return match_count;
                            }
                        }
                    }

                    // ---- States added without inspecting the char -----
                    x if x == OP_ALT => {
                        let mut cc = code;
                        loop {
                            cc = cc.add(GET(cc, 1) as usize);
                            if *cc as u32 != OP_ALT {
                                break;
                            }
                        }
                        ADD_ACTIVE!((cc as usize - start_code as usize) as c_int_local, 0);
                    }

                    x if x == OP_BRA || x == OP_SBRA => {
                        let mut cc = code;
                        loop {
                            ADD_ACTIVE!(
                                (cc as usize - start_code as usize + 1 + LINK_SIZE) as c_int_local,
                                0
                            );
                            cc = cc.add(GET(cc, 1) as usize);
                            if *cc as u32 != OP_ALT {
                                break;
                            }
                        }
                    }

                    x if x == OP_CBRA || x == OP_SCBRA => {
                        let mut cc = code;
                        ADD_ACTIVE!(
                            (cc as usize - start_code as usize + 1 + LINK_SIZE + IMM2_SIZE)
                                as c_int_local,
                            0
                        );
                        cc = cc.add(GET(cc, 1) as usize);
                        while *cc as u32 == OP_ALT {
                            ADD_ACTIVE!(
                                (cc as usize - start_code as usize + 1 + LINK_SIZE) as c_int_local,
                                0
                            );
                            cc = cc.add(GET(cc, 1) as usize);
                        }
                    }

                    x if x == OP_BRAZERO || x == OP_BRAMINZERO => {
                        ADD_ACTIVE!(state_offset + 1, 0);
                        let mut cc = code.add(1 + GET(code, 2) as usize);
                        while *cc as u32 == OP_ALT {
                            cc = cc.add(GET(cc, 1) as usize);
                        }
                        ADD_ACTIVE!(
                            (cc as usize - start_code as usize + 1 + LINK_SIZE) as c_int_local,
                            0
                        );
                    }

                    x if x == OP_SKIPZERO => {
                        let mut cc = code.add(1 + GET(code, 2) as usize);
                        while *cc as u32 == OP_ALT {
                            cc = cc.add(GET(cc, 1) as usize);
                        }
                        ADD_ACTIVE!(
                            (cc as usize - start_code as usize + 1 + LINK_SIZE) as c_int_local,
                            0
                        );
                    }

                    x if x == OP_CIRC => {
                        if ptr == start_subject && ((*mb).moptions & O_NOTBOL) == 0 {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    x if x == OP_CIRCM => {
                        if (ptr == start_subject && ((*mb).moptions & O_NOTBOL) == 0)
                            || ((ptr != end_subject
                                || ((*mb).poptions & O_ALT_CIRCUMFLEX) != 0)
                                && WAS_NEWLINE!(ptr))
                        {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    x if x == OP_EOD => {
                        if ptr >= end_subject {
                            if ((*mb).moptions & O_PARTIAL_HARD) != 0 {
                                return err::PARTIAL;
                            } else {
                                ADD_ACTIVE!(state_offset + 1, 0);
                            }
                        }
                    }

                    x if x == OP_SOD => {
                        if ptr == start_subject {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    x if x == OP_SOM => {
                        if ptr == start_subject.add(start_offset) {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    // ---- Inspect next char, no argument ----------------
                    x if x == OP_ANY => {
                        if clen > 0 && !IS_NEWLINE!(ptr) {
                            if ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & O_PARTIAL_HARD) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                could_continue = true;
                                partial_newline = true;
                            } else {
                                ADD_NEW!(state_offset + 1, 0);
                            }
                        }
                    }

                    x if x == OP_ALLANY => {
                        if clen > 0 {
                            ADD_NEW!(state_offset + 1, 0);
                        }
                    }

                    x if x == OP_EODN => {
                        if clen == 0 || (IS_NEWLINE!(ptr) && ptr == end_subject.sub((*mb).nllen as usize)) {
                            if ((*mb).moptions & O_PARTIAL_HARD) != 0 {
                                return err::PARTIAL;
                            }
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    x if x == OP_DOLL => {
                        if ((*mb).moptions & O_NOTEOL) == 0 {
                            if clen == 0 && ((*mb).moptions & O_PARTIAL_HARD) != 0 {
                                could_continue = true;
                            } else if clen == 0
                                || (((*mb).poptions & O_DOLLAR_ENDONLY) == 0
                                    && IS_NEWLINE!(ptr)
                                    && ptr == end_subject.sub((*mb).nllen as usize))
                            {
                                ADD_ACTIVE!(state_offset + 1, 0);
                            } else if ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & (O_PARTIAL_HARD | O_PARTIAL_SOFT)) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                if ((*mb).moptions & O_PARTIAL_HARD) != 0 {
                                    reset_could_continue = true;
                                    ADD_NEW_DATA!(-(state_offset + 1), 0, 1);
                                } else {
                                    could_continue = true;
                                    partial_newline = true;
                                }
                            }
                        }
                    }

                    x if x == OP_DOLLM => {
                        if ((*mb).moptions & O_NOTEOL) == 0 {
                            if clen == 0 && ((*mb).moptions & O_PARTIAL_HARD) != 0 {
                                could_continue = true;
                            } else if clen == 0
                                || (((*mb).poptions & O_DOLLAR_ENDONLY) == 0 && IS_NEWLINE!(ptr))
                            {
                                ADD_ACTIVE!(state_offset + 1, 0);
                            } else if ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & (O_PARTIAL_HARD | O_PARTIAL_SOFT)) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                if ((*mb).moptions & O_PARTIAL_HARD) != 0 {
                                    reset_could_continue = true;
                                    ADD_NEW_DATA!(-(state_offset + 1), 0, 1);
                                } else {
                                    could_continue = true;
                                    partial_newline = true;
                                }
                            }
                        } else if IS_NEWLINE!(ptr) {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    x if x == OP_DIGIT || x == OP_WHITESPACE || x == OP_WORDCHAR => {
                        if clen > 0
                            && c < 256
                            && ((*ctypes.add(c as usize) & TOPTABLE1[codevalue as usize])
                                ^ TOPTABLE2[codevalue as usize])
                                != 0
                        {
                            ADD_NEW!(state_offset + 1, 0);
                        }
                    }

                    x if x == OP_NOT_DIGIT || x == OP_NOT_WHITESPACE || x == OP_NOT_WORDCHAR => {
                        if clen > 0
                            && (c >= 256
                                || ((*ctypes.add(c as usize) & TOPTABLE1[codevalue as usize])
                                    ^ TOPTABLE2[codevalue as usize])
                                    != 0)
                        {
                            ADD_NEW!(state_offset + 1, 0);
                        }
                    }

                    x if x == OP_WORD_BOUNDARY
                        || x == OP_NOT_WORD_BOUNDARY
                        || x == OP_NOT_UCP_WORD_BOUNDARY
                        || x == OP_UCP_WORD_BOUNDARY =>
                    {
                        let left_word: bool;
                        let right_word: bool;

                        if ptr > start_subject {
                            let mut temp = ptr.sub(1);
                            if temp < (*mb).start_used_ptr {
                                (*mb).start_used_ptr = temp;
                            }
                            if utf {
                                BACKCHAR(&mut temp);
                            }
                            let dd = GETCHARTEST(temp, utf);
                            if codevalue == OP_UCP_WORD_BOUNDARY
                                || codevalue == OP_NOT_UCP_WORD_BOUNDARY
                            {
                                let chartype = UCD_CHARTYPE(dd);
                                let category =
                                    crate::tables::_pcre2_ucp_gentype[chartype as usize];
                                left_word = category == ucp::ucp_L
                                    || category == ucp::ucp_N
                                    || chartype == ucp::ucp_Mn
                                    || chartype == ucp::ucp_Pc;
                            } else {
                                left_word =
                                    dd < 256 && (*ctypes.add(dd as usize) & CTYPE_WORD) != 0;
                            }
                        } else {
                            left_word = false;
                        }

                        if clen > 0 {
                            if ptr >= (*mb).last_used_ptr {
                                let mut temp = ptr.add(1);
                                if utf {
                                    FORWARDCHARTEST(&mut temp, (*mb).end_subject);
                                }
                                (*mb).last_used_ptr = temp;
                            }
                            if codevalue == OP_UCP_WORD_BOUNDARY
                                || codevalue == OP_NOT_UCP_WORD_BOUNDARY
                            {
                                let chartype = UCD_CHARTYPE(c);
                                let category =
                                    crate::tables::_pcre2_ucp_gentype[chartype as usize];
                                right_word = category == ucp::ucp_L
                                    || category == ucp::ucp_N
                                    || chartype == ucp::ucp_Mn
                                    || chartype == ucp::ucp_Pc;
                            } else {
                                right_word = c < 256 && (*ctypes.add(c as usize) & CTYPE_WORD) != 0;
                            }
                        } else {
                            right_word = false;
                        }

                        if (left_word == right_word)
                            == (codevalue == OP_NOT_WORD_BOUNDARY
                                || codevalue == OP_NOT_UCP_WORD_BOUNDARY)
                        {
                            ADD_ACTIVE!(state_offset + 1, 0);
                        }
                    }

                    // ---- \p and \P ------------------------------------
                    x if x == OP_PROP || x == OP_NOTPROP => {
                        if clen > 0 {
                            let ok = prop_ok!(code.add(1), codevalue == OP_PROP);
                            if ok == (codevalue == OP_PROP) {
                                ADD_NEW!(state_offset + 3, 0);
                            }
                        }
                    }

                    // ---- Type repeats (arg in d) -----------------------
                    x if x == OP_TYPEPLUS || x == OP_TYPEMINPLUS || x == OP_TYPEPOSPLUS => {
                        count = (*current_state).count;
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 2, 0);
                        }
                        if clen > 0 {
                            if d == OP_ANY
                                && ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & O_PARTIAL_HARD) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                could_continue = true;
                                partial_newline = true;
                            } else if (c >= 256
                                && d != OP_DIGIT
                                && d != OP_WHITESPACE
                                && d != OP_WORDCHAR)
                                || (c < 256
                                    && (d != OP_ANY || !IS_NEWLINE!(ptr))
                                    && ((*ctypes.add(c as usize) & TOPTABLE1[d as usize])
                                        ^ TOPTABLE2[d as usize])
                                        != 0)
                            {
                                if count > 0 && codevalue == OP_TYPEPOSPLUS {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                ADD_NEW!(state_offset, count);
                            }
                        }
                    }

                    x if x == OP_TYPEQUERY || x == OP_TYPEMINQUERY || x == OP_TYPEPOSQUERY => {
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            if d == OP_ANY
                                && ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & O_PARTIAL_HARD) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                could_continue = true;
                                partial_newline = true;
                            } else if (c >= 256
                                && d != OP_DIGIT
                                && d != OP_WHITESPACE
                                && d != OP_WORDCHAR)
                                || (c < 256
                                    && (d != OP_ANY || !IS_NEWLINE!(ptr))
                                    && ((*ctypes.add(c as usize) & TOPTABLE1[d as usize])
                                        ^ TOPTABLE2[d as usize])
                                        != 0)
                            {
                                if codevalue == OP_TYPEPOSQUERY {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW!(state_offset + 2, 0);
                            }
                        }
                    }

                    x if x == OP_TYPESTAR || x == OP_TYPEMINSTAR || x == OP_TYPEPOSSTAR => {
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            if d == OP_ANY
                                && ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & O_PARTIAL_HARD) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                could_continue = true;
                                partial_newline = true;
                            } else if (c >= 256
                                && d != OP_DIGIT
                                && d != OP_WHITESPACE
                                && d != OP_WORDCHAR)
                                || (c < 256
                                    && (d != OP_ANY || !IS_NEWLINE!(ptr))
                                    && ((*ctypes.add(c as usize) & TOPTABLE1[d as usize])
                                        ^ TOPTABLE2[d as usize])
                                        != 0)
                            {
                                if codevalue == OP_TYPEPOSSTAR {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW!(state_offset, 0);
                            }
                        }
                    }

                    x if x == OP_TYPEEXACT => {
                        count = (*current_state).count;
                        if clen > 0 {
                            if d == OP_ANY
                                && ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & O_PARTIAL_HARD) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                could_continue = true;
                                partial_newline = true;
                            } else if (c >= 256
                                && d != OP_DIGIT
                                && d != OP_WHITESPACE
                                && d != OP_WORDCHAR)
                                || (c < 256
                                    && (d != OP_ANY || !IS_NEWLINE!(ptr))
                                    && ((*ctypes.add(c as usize) & TOPTABLE1[d as usize])
                                        ^ TOPTABLE2[d as usize])
                                        != 0)
                            {
                                count += 1;
                                if count >= GET2(code, 1) as c_int_local {
                                    ADD_NEW!(state_offset + 1 + IMM2_SIZE as c_int_local + 1, 0);
                                } else {
                                    ADD_NEW!(state_offset, count);
                                }
                            }
                        }
                    }

                    x if x == OP_TYPEUPTO || x == OP_TYPEMINUPTO || x == OP_TYPEPOSUPTO => {
                        ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as c_int_local, 0);
                        count = (*current_state).count;
                        if clen > 0 {
                            if d == OP_ANY
                                && ptr.add(1) >= (*mb).end_subject
                                && ((*mb).moptions & O_PARTIAL_HARD) != 0
                                && (*mb).nltype == NLTYPE_FIXED
                                && (*mb).nllen == 2
                                && c == (*mb).nl[0] as u32
                            {
                                could_continue = true;
                                partial_newline = true;
                            } else if (c >= 256
                                && d != OP_DIGIT
                                && d != OP_WHITESPACE
                                && d != OP_WORDCHAR)
                                || (c < 256
                                    && (d != OP_ANY || !IS_NEWLINE!(ptr))
                                    && ((*ctypes.add(c as usize) & TOPTABLE1[d as usize])
                                        ^ TOPTABLE2[d as usize])
                                        != 0)
                            {
                                if codevalue == OP_TYPEPOSUPTO {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                if count >= GET2(code, 1) as c_int_local {
                                    ADD_NEW!(state_offset + 2 + IMM2_SIZE as c_int_local, 0);
                                } else {
                                    ADD_NEW!(state_offset, count);
                                }
                            }
                        }
                    }

                    // ---- PROP_EXTRA + TYPEPLUS/MINPLUS/POSPLUS ---------
                    x if x == OP_PROP_EXTRA + OP_TYPEPLUS
                        || x == OP_PROP_EXTRA + OP_TYPEMINPLUS
                        || x == OP_PROP_EXTRA + OP_TYPEPOSPLUS =>
                    {
                        count = (*current_state).count;
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 4, 0);
                        }
                        if clen > 0 {
                            let ok = prop_ok!(code.add(2), false);
                            if ok == (d == OP_PROP) {
                                if count > 0 && codevalue == OP_PROP_EXTRA + OP_TYPEPOSPLUS {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                ADD_NEW!(state_offset, count);
                            }
                        }
                    }

                    // ---- EXTUNI_EXTRA + TYPEPLUS ----------------------
                    x if x == OP_EXTUNI_EXTRA + OP_TYPEPLUS
                        || x == OP_EXTUNI_EXTRA + OP_TYPEMINPLUS
                        || x == OP_EXTUNI_EXTRA + OP_TYPEPOSPLUS =>
                    {
                        count = (*current_state).count;
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 2, 0);
                        }
                        if clen > 0 {
                            let mut ncount: c_int_local = 0;
                            if count > 0 && codevalue == OP_EXTUNI_EXTRA + OP_TYPEPOSPLUS {
                                active_count -= 1;
                                next_active_state = next_active_state.sub(1);
                            }
                            crate::extuni::_pcre2_extuni_8(
                                c,
                                ptr.add(clen as usize),
                                (*mb).start_subject,
                                end_subject,
                                utf as BOOL,
                                &mut ncount,
                            );
                            count += 1;
                            ADD_NEW_DATA!(-state_offset, count, ncount);
                        }
                    }

                    // ---- ANYNL_EXTRA + TYPEPLUS -----------------------
                    x if x == OP_ANYNL_EXTRA + OP_TYPEPLUS
                        || x == OP_ANYNL_EXTRA + OP_TYPEMINPLUS
                        || x == OP_ANYNL_EXTRA + OP_TYPEPOSPLUS =>
                    {
                        count = (*current_state).count;
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 2, 0);
                        }
                        if clen > 0 {
                            let mut ncount: c_int_local = 0;
                            let mut matched = true;
                            match c {
                                CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                    if (*mb).bsr_convention == PCRE2_BSR_ANYCRLF {
                                        matched = false;
                                    }
                                    // else fall to ANYNL01 (LF handling below)
                                }
                                CHAR_CR => {
                                    if ptr.add(1) < end_subject && *ptr.add(1) as u32 == CHAR_LF {
                                        ncount = 1;
                                    }
                                }
                                CHAR_LF => {}
                                _ => {
                                    matched = false;
                                }
                            }
                            if matched {
                                if count > 0 && codevalue == OP_ANYNL_EXTRA + OP_TYPEPOSPLUS {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                ADD_NEW_DATA!(-state_offset, count, ncount);
                            }
                        }
                    }

                    // ---- VSPACE_EXTRA + TYPEPLUS ----------------------
                    x if x == OP_VSPACE_EXTRA + OP_TYPEPLUS
                        || x == OP_VSPACE_EXTRA + OP_TYPEMINPLUS
                        || x == OP_VSPACE_EXTRA + OP_TYPEPOSPLUS =>
                    {
                        count = (*current_state).count;
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 2, 0);
                        }
                        if clen > 0 {
                            let ok = is_vspace(c);
                            if ok == (d == OP_VSPACE) {
                                if count > 0 && codevalue == OP_VSPACE_EXTRA + OP_TYPEPOSPLUS {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                ADD_NEW_DATA!(-state_offset, count, 0);
                            }
                        }
                    }

                    // ---- HSPACE_EXTRA + TYPEPLUS ----------------------
                    x if x == OP_HSPACE_EXTRA + OP_TYPEPLUS
                        || x == OP_HSPACE_EXTRA + OP_TYPEMINPLUS
                        || x == OP_HSPACE_EXTRA + OP_TYPEPOSPLUS =>
                    {
                        count = (*current_state).count;
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + 2, 0);
                        }
                        if clen > 0 {
                            let ok = is_hspace(c);
                            if ok == (d == OP_HSPACE) {
                                if count > 0 && codevalue == OP_HSPACE_EXTRA + OP_TYPEPOSPLUS {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                ADD_NEW_DATA!(-state_offset, count, 0);
                            }
                        }
                    }

                    // ---- PROP_EXTRA + QUERY/STAR (QS1) ----------------
                    x if x == OP_PROP_EXTRA + OP_TYPEQUERY
                        || x == OP_PROP_EXTRA + OP_TYPEMINQUERY
                        || x == OP_PROP_EXTRA + OP_TYPEPOSQUERY
                        || x == OP_PROP_EXTRA + OP_TYPESTAR
                        || x == OP_PROP_EXTRA + OP_TYPEMINSTAR
                        || x == OP_PROP_EXTRA + OP_TYPEPOSSTAR =>
                    {
                        count = if x == OP_PROP_EXTRA + OP_TYPEQUERY
                            || x == OP_PROP_EXTRA + OP_TYPEMINQUERY
                            || x == OP_PROP_EXTRA + OP_TYPEPOSQUERY
                        {
                            4
                        } else {
                            0
                        };
                        ADD_ACTIVE!(state_offset + 4, 0);
                        if clen > 0 {
                            let ok = prop_ok!(code.add(2), false);
                            if ok == (d == OP_PROP) {
                                if codevalue == OP_PROP_EXTRA + OP_TYPEPOSSTAR
                                    || codevalue == OP_PROP_EXTRA + OP_TYPEPOSQUERY
                                {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW!(state_offset + count, 0);
                            }
                        }
                    }

                    // ---- EXTUNI_EXTRA + QUERY/STAR (QS2) --------------
                    x if x == OP_EXTUNI_EXTRA + OP_TYPEQUERY
                        || x == OP_EXTUNI_EXTRA + OP_TYPEMINQUERY
                        || x == OP_EXTUNI_EXTRA + OP_TYPEPOSQUERY
                        || x == OP_EXTUNI_EXTRA + OP_TYPESTAR
                        || x == OP_EXTUNI_EXTRA + OP_TYPEMINSTAR
                        || x == OP_EXTUNI_EXTRA + OP_TYPEPOSSTAR =>
                    {
                        count = if x == OP_EXTUNI_EXTRA + OP_TYPEQUERY
                            || x == OP_EXTUNI_EXTRA + OP_TYPEMINQUERY
                            || x == OP_EXTUNI_EXTRA + OP_TYPEPOSQUERY
                        {
                            2
                        } else {
                            0
                        };
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            let mut ncount: c_int_local = 0;
                            if codevalue == OP_EXTUNI_EXTRA + OP_TYPEPOSSTAR
                                || codevalue == OP_EXTUNI_EXTRA + OP_TYPEPOSQUERY
                            {
                                active_count -= 1;
                                next_active_state = next_active_state.sub(1);
                            }
                            crate::extuni::_pcre2_extuni_8(
                                c,
                                ptr.add(clen as usize),
                                (*mb).start_subject,
                                end_subject,
                                utf as BOOL,
                                &mut ncount,
                            );
                            ADD_NEW_DATA!(-(state_offset + count), 0, ncount);
                        }
                    }

                    // ---- ANYNL_EXTRA + QUERY/STAR (QS3) --------------
                    x if x == OP_ANYNL_EXTRA + OP_TYPEQUERY
                        || x == OP_ANYNL_EXTRA + OP_TYPEMINQUERY
                        || x == OP_ANYNL_EXTRA + OP_TYPEPOSQUERY
                        || x == OP_ANYNL_EXTRA + OP_TYPESTAR
                        || x == OP_ANYNL_EXTRA + OP_TYPEMINSTAR
                        || x == OP_ANYNL_EXTRA + OP_TYPEPOSSTAR =>
                    {
                        count = if x == OP_ANYNL_EXTRA + OP_TYPEQUERY
                            || x == OP_ANYNL_EXTRA + OP_TYPEMINQUERY
                            || x == OP_ANYNL_EXTRA + OP_TYPEPOSQUERY
                        {
                            2
                        } else {
                            0
                        };
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            let mut ncount: c_int_local = 0;
                            let mut matched = true;
                            match c {
                                CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                    if (*mb).bsr_convention == PCRE2_BSR_ANYCRLF {
                                        matched = false;
                                    }
                                }
                                CHAR_CR => {
                                    if ptr.add(1) < end_subject && *ptr.add(1) as u32 == CHAR_LF {
                                        ncount = 1;
                                    }
                                }
                                CHAR_LF => {}
                                _ => matched = false,
                            }
                            if matched {
                                if codevalue == OP_ANYNL_EXTRA + OP_TYPEPOSSTAR
                                    || codevalue == OP_ANYNL_EXTRA + OP_TYPEPOSQUERY
                                {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW_DATA!(-(state_offset + count), 0, ncount);
                            }
                        }
                    }

                    // ---- VSPACE_EXTRA + QUERY/STAR (QS4) --------------
                    x if x == OP_VSPACE_EXTRA + OP_TYPEQUERY
                        || x == OP_VSPACE_EXTRA + OP_TYPEMINQUERY
                        || x == OP_VSPACE_EXTRA + OP_TYPEPOSQUERY
                        || x == OP_VSPACE_EXTRA + OP_TYPESTAR
                        || x == OP_VSPACE_EXTRA + OP_TYPEMINSTAR
                        || x == OP_VSPACE_EXTRA + OP_TYPEPOSSTAR =>
                    {
                        count = if x == OP_VSPACE_EXTRA + OP_TYPEQUERY
                            || x == OP_VSPACE_EXTRA + OP_TYPEMINQUERY
                            || x == OP_VSPACE_EXTRA + OP_TYPEPOSQUERY
                        {
                            2
                        } else {
                            0
                        };
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            let ok = is_vspace(c);
                            if ok == (d == OP_VSPACE) {
                                if codevalue == OP_VSPACE_EXTRA + OP_TYPEPOSSTAR
                                    || codevalue == OP_VSPACE_EXTRA + OP_TYPEPOSQUERY
                                {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW_DATA!(-(state_offset + count), 0, 0);
                            }
                        }
                    }

                    // ---- HSPACE_EXTRA + QUERY/STAR (QS5) --------------
                    x if x == OP_HSPACE_EXTRA + OP_TYPEQUERY
                        || x == OP_HSPACE_EXTRA + OP_TYPEMINQUERY
                        || x == OP_HSPACE_EXTRA + OP_TYPEPOSQUERY
                        || x == OP_HSPACE_EXTRA + OP_TYPESTAR
                        || x == OP_HSPACE_EXTRA + OP_TYPEMINSTAR
                        || x == OP_HSPACE_EXTRA + OP_TYPEPOSSTAR =>
                    {
                        count = if x == OP_HSPACE_EXTRA + OP_TYPEQUERY
                            || x == OP_HSPACE_EXTRA + OP_TYPEMINQUERY
                            || x == OP_HSPACE_EXTRA + OP_TYPEPOSQUERY
                        {
                            2
                        } else {
                            0
                        };
                        ADD_ACTIVE!(state_offset + 2, 0);
                        if clen > 0 {
                            let ok = is_hspace(c);
                            if ok == (d == OP_HSPACE) {
                                if codevalue == OP_HSPACE_EXTRA + OP_TYPEPOSSTAR
                                    || codevalue == OP_HSPACE_EXTRA + OP_TYPEPOSQUERY
                                {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW_DATA!(-(state_offset + count), 0, 0);
                            }
                        }
                    }

                    // ---- PROP_EXTRA + EXACT/UPTO ----------------------
                    x if x == OP_PROP_EXTRA + OP_TYPEEXACT
                        || x == OP_PROP_EXTRA + OP_TYPEUPTO
                        || x == OP_PROP_EXTRA + OP_TYPEMINUPTO
                        || x == OP_PROP_EXTRA + OP_TYPEPOSUPTO =>
                    {
                        if codevalue != OP_PROP_EXTRA + OP_TYPEEXACT {
                            ADD_ACTIVE!(state_offset + 1 + IMM2_SIZE as c_int_local + 3, 0);
                        }
                        count = (*current_state).count;
                        if clen > 0 {
                            let ok = prop_ok!(code.add(1 + IMM2_SIZE + 1), false);
                            if ok == (d == OP_PROP) {
                                if codevalue == OP_PROP_EXTRA + OP_TYPEPOSUPTO {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                if count >= GET2(code, 1) as c_int_local {
                                    ADD_NEW!(state_offset + 1 + IMM2_SIZE as c_int_local + 3, 0);
                                } else {
                                    ADD_NEW!(state_offset, count);
                                }
                            }
                        }
                    }

                    // ---- EXTUNI_EXTRA + EXACT/UPTO --------------------
                    x if x == OP_EXTUNI_EXTRA + OP_TYPEEXACT
                        || x == OP_EXTUNI_EXTRA + OP_TYPEUPTO
                        || x == OP_EXTUNI_EXTRA + OP_TYPEMINUPTO
                        || x == OP_EXTUNI_EXTRA + OP_TYPEPOSUPTO =>
                    {
                        if codevalue != OP_EXTUNI_EXTRA + OP_TYPEEXACT {
                            ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as c_int_local, 0);
                        }
                        count = (*current_state).count;
                        if clen > 0 {
                            let mut ncount: c_int_local = 0;
                            if codevalue == OP_EXTUNI_EXTRA + OP_TYPEPOSUPTO {
                                active_count -= 1;
                                next_active_state = next_active_state.sub(1);
                            }
                            let nptr = crate::extuni::_pcre2_extuni_8(
                                c,
                                ptr.add(clen as usize),
                                (*mb).start_subject,
                                end_subject,
                                utf as BOOL,
                                &mut ncount,
                            );
                            if nptr >= end_subject && ((*mb).moptions & O_PARTIAL_HARD) != 0 {
                                reset_could_continue = true;
                            }
                            count += 1;
                            if count >= GET2(code, 1) as c_int_local {
                                ADD_NEW_DATA!(-(state_offset + 2 + IMM2_SIZE as c_int_local), 0, ncount);
                            } else {
                                ADD_NEW_DATA!(-state_offset, count, ncount);
                            }
                        }
                    }

                    // ---- ANYNL_EXTRA + EXACT/UPTO --------------------
                    x if x == OP_ANYNL_EXTRA + OP_TYPEEXACT
                        || x == OP_ANYNL_EXTRA + OP_TYPEUPTO
                        || x == OP_ANYNL_EXTRA + OP_TYPEMINUPTO
                        || x == OP_ANYNL_EXTRA + OP_TYPEPOSUPTO =>
                    {
                        if codevalue != OP_ANYNL_EXTRA + OP_TYPEEXACT {
                            ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as c_int_local, 0);
                        }
                        count = (*current_state).count;
                        if clen > 0 {
                            let mut ncount: c_int_local = 0;
                            let mut matched = true;
                            match c {
                                CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                    if (*mb).bsr_convention == PCRE2_BSR_ANYCRLF {
                                        matched = false;
                                    }
                                }
                                CHAR_CR => {
                                    if ptr.add(1) < end_subject && *ptr.add(1) as u32 == CHAR_LF {
                                        ncount = 1;
                                    }
                                }
                                CHAR_LF => {}
                                _ => matched = false,
                            }
                            if matched {
                                if codevalue == OP_ANYNL_EXTRA + OP_TYPEPOSUPTO {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                if count >= GET2(code, 1) as c_int_local {
                                    ADD_NEW_DATA!(
                                        -(state_offset + 2 + IMM2_SIZE as c_int_local),
                                        0,
                                        ncount
                                    );
                                } else {
                                    ADD_NEW_DATA!(-state_offset, count, ncount);
                                }
                            }
                        }
                    }

                    // ---- VSPACE_EXTRA + EXACT/UPTO -------------------
                    x if x == OP_VSPACE_EXTRA + OP_TYPEEXACT
                        || x == OP_VSPACE_EXTRA + OP_TYPEUPTO
                        || x == OP_VSPACE_EXTRA + OP_TYPEMINUPTO
                        || x == OP_VSPACE_EXTRA + OP_TYPEPOSUPTO =>
                    {
                        if codevalue != OP_VSPACE_EXTRA + OP_TYPEEXACT {
                            ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as c_int_local, 0);
                        }
                        count = (*current_state).count;
                        if clen > 0 {
                            let ok = is_vspace(c);
                            if ok == (d == OP_VSPACE) {
                                if codevalue == OP_VSPACE_EXTRA + OP_TYPEPOSUPTO {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                if count >= GET2(code, 1) as c_int_local {
                                    ADD_NEW_DATA!(
                                        -(state_offset + 2 + IMM2_SIZE as c_int_local),
                                        0,
                                        0
                                    );
                                } else {
                                    ADD_NEW_DATA!(-state_offset, count, 0);
                                }
                            }
                        }
                    }

                    // ---- HSPACE_EXTRA + EXACT/UPTO -------------------
                    x if x == OP_HSPACE_EXTRA + OP_TYPEEXACT
                        || x == OP_HSPACE_EXTRA + OP_TYPEUPTO
                        || x == OP_HSPACE_EXTRA + OP_TYPEMINUPTO
                        || x == OP_HSPACE_EXTRA + OP_TYPEPOSUPTO =>
                    {
                        if codevalue != OP_HSPACE_EXTRA + OP_TYPEEXACT {
                            ADD_ACTIVE!(state_offset + 2 + IMM2_SIZE as c_int_local, 0);
                        }
                        count = (*current_state).count;
                        if clen > 0 {
                            let ok = is_hspace(c);
                            if ok == (d == OP_HSPACE) {
                                if codevalue == OP_HSPACE_EXTRA + OP_TYPEPOSUPTO {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                if count >= GET2(code, 1) as c_int_local {
                                    ADD_NEW_DATA!(
                                        -(state_offset + 2 + IMM2_SIZE as c_int_local),
                                        0,
                                        0
                                    );
                                } else {
                                    ADD_NEW_DATA!(-state_offset, count, 0);
                                }
                            }
                        }
                    }

                    // ---- Opcodes followed by a data char (d) ----------
                    x if x == OP_CHAR => {
                        if clen > 0 && c == d {
                            ADD_NEW!(state_offset + dlen + 1, 0);
                        }
                    }

                    x if x == OP_CHARI => {
                        if clen != 0 {
                            if utf_or_ucp {
                                if c == d {
                                    ADD_NEW!(state_offset + dlen + 1, 0);
                                } else {
                                    let othercase = if c < 128 {
                                        *fcc.add(c as usize) as u32
                                    } else {
                                        UCD_OTHERCASE(c)
                                    };
                                    if d == othercase {
                                        ADD_NEW!(state_offset + dlen + 1, 0);
                                    }
                                }
                            } else if TABLE_GET(c, lcc, c) == TABLE_GET(d, lcc, d) {
                                ADD_NEW!(state_offset + 2, 0);
                            }
                        }
                    }

                    x if x == OP_EXTUNI => {
                        if clen > 0 {
                            let mut ncount: c_int_local = 0;
                            let nptr = crate::extuni::_pcre2_extuni_8(
                                c,
                                ptr.add(clen as usize),
                                (*mb).start_subject,
                                end_subject,
                                utf as BOOL,
                                &mut ncount,
                            );
                            if nptr >= end_subject && ((*mb).moptions & O_PARTIAL_HARD) != 0 {
                                reset_could_continue = true;
                            }
                            ADD_NEW_DATA!(-(state_offset + 1), 0, ncount);
                        }
                    }

                    x if x == OP_ANYNL => {
                        if clen > 0 {
                            match c {
                                CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029
                                    if (*mb).bsr_convention != PCRE2_BSR_ANYCRLF =>
                                {
                                    ADD_NEW!(state_offset + 1, 0);
                                }
                                CHAR_VT | CHAR_FF | CHAR_NEL | 0x2028 | 0x2029 => {
                                    // bsr == ANYCRLF: no match
                                }
                                CHAR_LF => {
                                    ADD_NEW!(state_offset + 1, 0);
                                }
                                CHAR_CR => {
                                    if ptr.add(1) >= end_subject {
                                        ADD_NEW!(state_offset + 1, 0);
                                        if ((*mb).moptions & O_PARTIAL_HARD) != 0 {
                                            reset_could_continue = true;
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

                    x if x == OP_NOT_VSPACE => {
                        if clen > 0 && !is_vspace(c) {
                            ADD_NEW!(state_offset + 1, 0);
                        }
                    }

                    x if x == OP_VSPACE => {
                        if clen > 0 && is_vspace(c) {
                            ADD_NEW!(state_offset + 1, 0);
                        }
                    }

                    x if x == OP_NOT_HSPACE => {
                        if clen > 0 && !is_hspace(c) {
                            ADD_NEW!(state_offset + 1, 0);
                        }
                    }

                    x if x == OP_HSPACE => {
                        if clen > 0 && is_hspace(c) {
                            ADD_NEW!(state_offset + 1, 0);
                        }
                    }

                    x if x == OP_NOT => {
                        if clen > 0 && c != d {
                            ADD_NEW!(state_offset + dlen + 1, 0);
                        }
                    }

                    x if x == OP_NOTI => {
                        if clen > 0 {
                            let otherd = if utf_or_ucp && d >= 128 {
                                UCD_OTHERCASE(d)
                            } else {
                                TABLE_GET(d, fcc, d)
                            };
                            if c != d && c != otherd {
                                ADD_NEW!(state_offset + dlen + 1, 0);
                            }
                        }
                    }

                    // ---- PLUS family (caseless via OP_*I) -------------
                    x if x == OP_PLUSI
                        || x == OP_MINPLUSI
                        || x == OP_POSPLUSI
                        || x == OP_NOTPLUSI
                        || x == OP_NOTMINPLUSI
                        || x == OP_NOTPOSPLUSI
                        || x == OP_PLUS
                        || x == OP_MINPLUS
                        || x == OP_POSPLUS
                        || x == OP_NOTPLUS
                        || x == OP_NOTMINPLUS
                        || x == OP_NOTPOSPLUS =>
                    {
                        if x == OP_PLUSI
                            || x == OP_MINPLUSI
                            || x == OP_POSPLUSI
                            || x == OP_NOTPLUSI
                            || x == OP_NOTMINPLUSI
                            || x == OP_NOTPOSPLUSI
                        {
                            caseless = true;
                            codevalue -= OP_STARI - OP_STAR;
                        }
                        count = (*current_state).count;
                        if count > 0 {
                            ADD_ACTIVE!(state_offset + dlen + 1, 0);
                        }
                        if clen > 0 {
                            let mut otherd = NOTACHAR;
                            if caseless {
                                otherd = if utf_or_ucp && d >= 128 {
                                    UCD_OTHERCASE(d)
                                } else {
                                    TABLE_GET(d, fcc, d)
                                };
                            }
                            if (c == d || c == otherd) == (codevalue < OP_NOTSTAR) {
                                if count > 0
                                    && (codevalue == OP_POSPLUS || codevalue == OP_NOTPOSPLUS)
                                {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                ADD_NEW!(state_offset, count);
                            }
                        }
                    }

                    // ---- QUERY family --------------------------------
                    x if x == OP_QUERYI
                        || x == OP_MINQUERYI
                        || x == OP_POSQUERYI
                        || x == OP_NOTQUERYI
                        || x == OP_NOTMINQUERYI
                        || x == OP_NOTPOSQUERYI
                        || x == OP_QUERY
                        || x == OP_MINQUERY
                        || x == OP_POSQUERY
                        || x == OP_NOTQUERY
                        || x == OP_NOTMINQUERY
                        || x == OP_NOTPOSQUERY =>
                    {
                        if x == OP_QUERYI
                            || x == OP_MINQUERYI
                            || x == OP_POSQUERYI
                            || x == OP_NOTQUERYI
                            || x == OP_NOTMINQUERYI
                            || x == OP_NOTPOSQUERYI
                        {
                            caseless = true;
                            codevalue -= OP_STARI - OP_STAR;
                        }
                        ADD_ACTIVE!(state_offset + dlen + 1, 0);
                        if clen > 0 {
                            let mut otherd = NOTACHAR;
                            if caseless {
                                otherd = if utf_or_ucp && d >= 128 {
                                    UCD_OTHERCASE(d)
                                } else {
                                    TABLE_GET(d, fcc, d)
                                };
                            }
                            if (c == d || c == otherd) == (codevalue < OP_NOTSTAR) {
                                if codevalue == OP_POSQUERY || codevalue == OP_NOTPOSQUERY {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW!(state_offset + dlen + 1, 0);
                            }
                        }
                    }

                    // ---- STAR family ---------------------------------
                    x if x == OP_STARI
                        || x == OP_MINSTARI
                        || x == OP_POSSTARI
                        || x == OP_NOTSTARI
                        || x == OP_NOTMINSTARI
                        || x == OP_NOTPOSSTARI
                        || x == OP_STAR
                        || x == OP_MINSTAR
                        || x == OP_POSSTAR
                        || x == OP_NOTSTAR
                        || x == OP_NOTMINSTAR
                        || x == OP_NOTPOSSTAR =>
                    {
                        if x == OP_STARI
                            || x == OP_MINSTARI
                            || x == OP_POSSTARI
                            || x == OP_NOTSTARI
                            || x == OP_NOTMINSTARI
                            || x == OP_NOTPOSSTARI
                        {
                            caseless = true;
                            codevalue -= OP_STARI - OP_STAR;
                        }
                        ADD_ACTIVE!(state_offset + dlen + 1, 0);
                        if clen > 0 {
                            let mut otherd = NOTACHAR;
                            if caseless {
                                otherd = if utf_or_ucp && d >= 128 {
                                    UCD_OTHERCASE(d)
                                } else {
                                    TABLE_GET(d, fcc, d)
                                };
                            }
                            if (c == d || c == otherd) == (codevalue < OP_NOTSTAR) {
                                if codevalue == OP_POSSTAR || codevalue == OP_NOTPOSSTAR {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                ADD_NEW!(state_offset, 0);
                            }
                        }
                    }

                    // ---- EXACT family --------------------------------
                    x if x == OP_EXACTI
                        || x == OP_NOTEXACTI
                        || x == OP_EXACT
                        || x == OP_NOTEXACT =>
                    {
                        if x == OP_EXACTI || x == OP_NOTEXACTI {
                            caseless = true;
                            codevalue -= OP_STARI - OP_STAR;
                        }
                        count = (*current_state).count;
                        if clen > 0 {
                            let mut otherd = NOTACHAR;
                            if caseless {
                                otherd = if utf_or_ucp && d >= 128 {
                                    UCD_OTHERCASE(d)
                                } else {
                                    TABLE_GET(d, fcc, d)
                                };
                            }
                            if (c == d || c == otherd) == (codevalue < OP_NOTSTAR) {
                                count += 1;
                                if count >= GET2(code, 1) as c_int_local {
                                    ADD_NEW!(state_offset + dlen + 1 + IMM2_SIZE as c_int_local, 0);
                                } else {
                                    ADD_NEW!(state_offset, count);
                                }
                            }
                        }
                    }

                    // ---- UPTO family ---------------------------------
                    x if x == OP_UPTOI
                        || x == OP_MINUPTOI
                        || x == OP_POSUPTOI
                        || x == OP_NOTUPTOI
                        || x == OP_NOTMINUPTOI
                        || x == OP_NOTPOSUPTOI
                        || x == OP_UPTO
                        || x == OP_MINUPTO
                        || x == OP_POSUPTO
                        || x == OP_NOTUPTO
                        || x == OP_NOTMINUPTO
                        || x == OP_NOTPOSUPTO =>
                    {
                        if x == OP_UPTOI
                            || x == OP_MINUPTOI
                            || x == OP_POSUPTOI
                            || x == OP_NOTUPTOI
                            || x == OP_NOTMINUPTOI
                            || x == OP_NOTPOSUPTOI
                        {
                            caseless = true;
                            codevalue -= OP_STARI - OP_STAR;
                        }
                        ADD_ACTIVE!(state_offset + dlen + 1 + IMM2_SIZE as c_int_local, 0);
                        count = (*current_state).count;
                        if clen > 0 {
                            let mut otherd = NOTACHAR;
                            if caseless {
                                otherd = if utf_or_ucp && d >= 128 {
                                    UCD_OTHERCASE(d)
                                } else {
                                    TABLE_GET(d, fcc, d)
                                };
                            }
                            if (c == d || c == otherd) == (codevalue < OP_NOTSTAR) {
                                if codevalue == OP_POSUPTO || codevalue == OP_NOTPOSUPTO {
                                    active_count -= 1;
                                    next_active_state = next_active_state.sub(1);
                                }
                                count += 1;
                                if count >= GET2(code, 1) as c_int_local {
                                    ADD_NEW!(state_offset + dlen + 1 + IMM2_SIZE as c_int_local, 0);
                                } else {
                                    ADD_NEW!(state_offset, count);
                                }
                            }
                        }
                    }

                    // ---- Class-handling opcodes ----------------------
                    x if x == OP_CLASS || x == OP_NCLASS || x == OP_XCLASS || x == OP_ECLASS => {
                        let mut isinclass = false;
                        let ecode: PCRE2_SPTR;

                        if codevalue == OP_XCLASS {
                            ecode = code.add(GET(code, 1) as usize);
                            if clen > 0 {
                                isinclass = crate::xclass::_pcre2_xclass_8(
                                    c,
                                    code.add(1 + LINK_SIZE),
                                    (*mb).start_code as *const u8,
                                    utf as BOOL,
                                ) != 0;
                            }
                        } else if codevalue == OP_ECLASS {
                            ecode = code.add(GET(code, 1) as usize);
                            if clen > 0 {
                                isinclass = crate::xclass::_pcre2_eclass_8(
                                    c,
                                    code.add(1 + LINK_SIZE),
                                    ecode,
                                    (*mb).start_code as *const u8,
                                    utf as BOOL,
                                ) != 0;
                            }
                        } else {
                            ecode = code.add(1 + 32);
                            if clen > 0 {
                                isinclass = if c > 255 {
                                    codevalue == OP_NCLASS
                                } else {
                                    (*code.add(1 + (c as usize) / 8) & (1u8 << (c & 7))) != 0
                                };
                            }
                        }

                        let next_state_offset = (ecode as usize - start_code as usize) as c_int_local;

                        match *ecode as u32 {
                            v if v == OP_CRSTAR || v == OP_CRMINSTAR || v == OP_CRPOSSTAR => {
                                ADD_ACTIVE!(next_state_offset + 1, 0);
                                if isinclass {
                                    if *ecode as u32 == OP_CRPOSSTAR {
                                        active_count -= 1;
                                        next_active_state = next_active_state.sub(1);
                                    }
                                    ADD_NEW!(state_offset, 0);
                                }
                            }
                            v if v == OP_CRPLUS || v == OP_CRMINPLUS || v == OP_CRPOSPLUS => {
                                count = (*current_state).count;
                                if count > 0 {
                                    ADD_ACTIVE!(next_state_offset + 1, 0);
                                }
                                if isinclass {
                                    if count > 0 && *ecode as u32 == OP_CRPOSPLUS {
                                        active_count -= 1;
                                        next_active_state = next_active_state.sub(1);
                                    }
                                    count += 1;
                                    ADD_NEW!(state_offset, count);
                                }
                            }
                            v if v == OP_CRQUERY || v == OP_CRMINQUERY || v == OP_CRPOSQUERY => {
                                ADD_ACTIVE!(next_state_offset + 1, 0);
                                if isinclass {
                                    if *ecode as u32 == OP_CRPOSQUERY {
                                        active_count -= 1;
                                        next_active_state = next_active_state.sub(1);
                                    }
                                    ADD_NEW!(next_state_offset + 1, 0);
                                }
                            }
                            v if v == OP_CRRANGE || v == OP_CRMINRANGE || v == OP_CRPOSRANGE => {
                                count = (*current_state).count;
                                if count >= GET2(ecode, 1) as c_int_local {
                                    ADD_ACTIVE!(
                                        next_state_offset + 1 + 2 * IMM2_SIZE as c_int_local,
                                        0
                                    );
                                }
                                if isinclass {
                                    let max = GET2(ecode, 1 + IMM2_SIZE) as c_int_local;
                                    if *ecode as u32 == OP_CRPOSRANGE
                                        && count >= GET2(ecode, 1) as c_int_local
                                    {
                                        active_count -= 1;
                                        next_active_state = next_active_state.sub(1);
                                    }
                                    count += 1;
                                    if count >= max && max != 0 {
                                        ADD_NEW!(
                                            next_state_offset + 1 + 2 * IMM2_SIZE as c_int_local,
                                            0
                                        );
                                    } else {
                                        ADD_NEW!(state_offset, count);
                                    }
                                }
                            }
                            _ => {
                                if isinclass {
                                    ADD_NEW!(next_state_offset, 0);
                                }
                            }
                        }
                    }

                    // ---- Fancy brackets ------------------------------
                    x if x == OP_FAIL => {
                        // Always fails; nothing to do.
                    }

                    x if x == OP_ASSERT
                        || x == OP_ASSERT_NOT
                        || x == OP_ASSERTBACK
                        || x == OP_ASSERTBACK_NOT =>
                    {
                        let rc: c_int_local;
                        let local_workspace: *mut c_int_local;
                        let local_offsets: *mut PCRE2_SIZE;
                        let mut endasscode = code.add(GET(code, 1) as usize);
                        let mut rws = RWS as *mut RWS_anchor;

                        if (*rws).free < (RWS_RSIZE + RWS_OVEC_OSIZE) as u32 {
                            let r = more_workspace(&mut rws, RWS_OVEC_OSIZE as u32, mb);
                            if r != 0 {
                                return r;
                            }
                            RWS = rws as *mut c_int_local;
                        }

                        local_offsets = RWS.add((*rws).size as usize - (*rws).free as usize)
                            as *mut PCRE2_SIZE;
                        local_workspace = (local_offsets as *mut c_int_local).add(RWS_OVEC_OSIZE);
                        (*rws).free -= (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        while *endasscode as u32 == OP_ALT {
                            endasscode = endasscode.add(GET(endasscode, 1) as usize);
                        }

                        rc = internal_dfa_match(
                            mb,
                            code,
                            ptr,
                            (ptr as usize - start_subject as usize) as PCRE2_SIZE,
                            local_offsets,
                            (RWS_OVEC_OSIZE / OVEC_UNIT) as u32,
                            local_workspace,
                            RWS_RSIZE as c_int_local,
                            rlevel,
                            RWS,
                        );

                        (*rws).free += (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        if rc < 0 && rc != err::NOMATCH {
                            return rc;
                        }
                        if (rc >= 0)
                            == (codevalue == OP_ASSERT || codevalue == OP_ASSERTBACK)
                        {
                            ADD_ACTIVE!(
                                (endasscode.add(LINK_SIZE + 1) as usize - start_code as usize)
                                    as c_int_local,
                                0
                            );
                        }
                    }

                    x if x == OP_COND || x == OP_SCOND => {
                        let codelink = GET(code, 1) as c_int_local;
                        let mut code = code;

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
                                return rrc;
                            }
                            if rrc > 0 {
                                i += 1;
                                continue;
                            }
                            code = code.add(callout_length as usize);
                        }

                        let condcode = *code.add(LINK_SIZE + 1) as u32;

                        if condcode == OP_CREF || condcode == OP_DNCREF || condcode == OP_DNRREF {
                            return err::DFA_UCOND;
                        }

                        if condcode == OP_FALSE || condcode == OP_FAIL {
                            ADD_ACTIVE!(state_offset + codelink + LINK_SIZE as c_int_local + 1, 0);
                        } else if condcode == OP_TRUE {
                            ADD_ACTIVE!(state_offset + LINK_SIZE as c_int_local + 2, 0);
                        } else if condcode == OP_RREF {
                            let value = GET2(code, LINK_SIZE + 2);
                            if value != RREF_ANY {
                                return err::DFA_UCOND;
                            }
                            if !(*mb).recursive.is_null() {
                                ADD_ACTIVE!(
                                    state_offset + LINK_SIZE as c_int_local + 2
                                        + IMM2_SIZE as c_int_local,
                                    0
                                );
                            } else {
                                ADD_ACTIVE!(
                                    state_offset + codelink + LINK_SIZE as c_int_local + 1,
                                    0
                                );
                            }
                        } else {
                            let rc: c_int_local;
                            let local_workspace: *mut c_int_local;
                            let local_offsets: *mut PCRE2_SIZE;
                            let asscode = code.add(LINK_SIZE + 1);
                            let mut endasscode = asscode.add(GET(asscode, 1) as usize);
                            let mut rws = RWS as *mut RWS_anchor;

                            if (*rws).free < (RWS_RSIZE + RWS_OVEC_OSIZE) as u32 {
                                let r = more_workspace(&mut rws, RWS_OVEC_OSIZE as u32, mb);
                                if r != 0 {
                                    return r;
                                }
                                RWS = rws as *mut c_int_local;
                            }

                            local_offsets = RWS
                                .add((*rws).size as usize - (*rws).free as usize)
                                as *mut PCRE2_SIZE;
                            local_workspace =
                                (local_offsets as *mut c_int_local).add(RWS_OVEC_OSIZE);
                            (*rws).free -= (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                            while *endasscode as u32 == OP_ALT {
                                endasscode = endasscode.add(GET(endasscode, 1) as usize);
                            }

                            rc = internal_dfa_match(
                                mb,
                                asscode,
                                ptr,
                                (ptr as usize - start_subject as usize) as PCRE2_SIZE,
                                local_offsets,
                                (RWS_OVEC_OSIZE / OVEC_UNIT) as u32,
                                local_workspace,
                                RWS_RSIZE as c_int_local,
                                rlevel,
                                RWS,
                            );

                            (*rws).free += (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                            if rc < 0 && rc != err::NOMATCH {
                                return rc;
                            }
                            if (rc >= 0)
                                == (condcode == OP_ASSERT || condcode == OP_ASSERTBACK)
                            {
                                ADD_ACTIVE!(
                                    (endasscode.add(LINK_SIZE + 1) as usize
                                        - start_code as usize)
                                        as c_int_local,
                                    0
                                );
                            } else {
                                ADD_ACTIVE!(
                                    state_offset + codelink + LINK_SIZE as c_int_local + 1,
                                    0
                                );
                            }
                        }
                    }

                    x if x == OP_RECURSE => {
                        let mut rc: c_int_local;
                        let local_workspace: *mut c_int_local;
                        let local_offsets: *mut PCRE2_SIZE;
                        let mut rws = RWS as *mut RWS_anchor;
                        let callpat = start_code.add(GET(code, 1) as usize);
                        let recno = if callpat == (*mb).start_code {
                            0u32
                        } else {
                            GET2(callpat, 1 + LINK_SIZE)
                        };

                        if *code.add(1 + LINK_SIZE) as u32 == OP_CREF {
                            return err::DFA_UITEM;
                        }

                        if (*rws).free < (RWS_RSIZE + RWS_OVEC_RSIZE) as u32 {
                            let r = more_workspace(&mut rws, RWS_OVEC_RSIZE as u32, mb);
                            if r != 0 {
                                return r;
                            }
                            RWS = rws as *mut c_int_local;
                        }

                        local_offsets = RWS.add((*rws).size as usize - (*rws).free as usize)
                            as *mut PCRE2_SIZE;
                        local_workspace = (local_offsets as *mut c_int_local).add(RWS_OVEC_RSIZE);
                        (*rws).free -= (RWS_RSIZE + RWS_OVEC_RSIZE) as u32;

                        let mut ri = (*mb).recursive;
                        while !ri.is_null() {
                            if recno == (*ri).group_num
                                && ptr == (*ri).subject_position
                                && (*mb).last_used_ptr == (*ri).last_used_ptr
                            {
                                return err::RECURSELOOP;
                            }
                            ri = (*ri).prevrec;
                        }

                        let mut new_recursive: dfa_recursion_info = core::mem::zeroed();
                        new_recursive.group_num = recno;
                        new_recursive.subject_position = ptr;
                        new_recursive.last_used_ptr = (*mb).last_used_ptr;
                        new_recursive.prevrec = (*mb).recursive;
                        (*mb).recursive = &mut new_recursive;

                        rc = internal_dfa_match(
                            mb,
                            callpat,
                            ptr,
                            (ptr as usize - start_subject as usize) as PCRE2_SIZE,
                            local_offsets,
                            (RWS_OVEC_RSIZE / OVEC_UNIT) as u32,
                            local_workspace,
                            RWS_RSIZE as c_int_local,
                            rlevel,
                            RWS,
                        );

                        (*rws).free += (RWS_RSIZE + RWS_OVEC_RSIZE) as u32;
                        (*mb).recursive = new_recursive.prevrec;

                        if rc == 0 {
                            return err::DFA_RECURSE;
                        }

                        if rc > 0 {
                            let mut k = rc * 2 - 2;
                            while k >= 0 {
                                let mut charcount = *local_offsets.add((k + 1) as usize)
                                    - *local_offsets.add(k as usize);
                                if utf {
                                    let mut p = start_subject.add(*local_offsets.add(k as usize));
                                    let pp =
                                        start_subject.add(*local_offsets.add((k + 1) as usize));
                                    while p < pp {
                                        let b = *p;
                                        p = p.add(1);
                                        if NOT_FIRSTCU(b as u32) {
                                            charcount -= 1;
                                        }
                                    }
                                }
                                if charcount > 0 {
                                    ADD_NEW_DATA!(
                                        -(state_offset + LINK_SIZE as c_int_local + 1),
                                        0,
                                        (charcount - 1) as c_int_local
                                    );
                                } else {
                                    ADD_ACTIVE!(state_offset + LINK_SIZE as c_int_local + 1, 0);
                                }
                                k -= 2;
                            }
                        } else if rc != err::NOMATCH {
                            return rc;
                        }
                    }

                    x if x == OP_BRAPOS
                        || x == OP_SBRAPOS
                        || x == OP_CBRAPOS
                        || x == OP_SCBRAPOS
                        || x == OP_BRAPOSZERO =>
                    {
                        let mut rc: c_int_local;
                        let local_workspace: *mut c_int_local;
                        let local_offsets: *mut PCRE2_SIZE;
                        let mut charcount: PCRE2_SIZE;
                        let mut matched_count: PCRE2_SIZE;
                        let mut local_ptr = ptr;
                        let mut rws = RWS as *mut RWS_anchor;
                        let allow_zero: bool;
                        let mut code = code;

                        if (*rws).free < (RWS_RSIZE + RWS_OVEC_OSIZE) as u32 {
                            let r = more_workspace(&mut rws, RWS_OVEC_OSIZE as u32, mb);
                            if r != 0 {
                                return r;
                            }
                            RWS = rws as *mut c_int_local;
                        }

                        local_offsets = RWS.add((*rws).size as usize - (*rws).free as usize)
                            as *mut PCRE2_SIZE;
                        local_workspace = (local_offsets as *mut c_int_local).add(RWS_OVEC_OSIZE);
                        (*rws).free -= (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        if codevalue == OP_BRAPOSZERO {
                            allow_zero = true;
                            code = code.add(1);
                        } else {
                            allow_zero = false;
                        }

                        matched_count = 0;
                        loop {
                            rc = internal_dfa_match(
                                mb,
                                code,
                                local_ptr,
                                (ptr as usize - start_subject as usize) as PCRE2_SIZE,
                                local_offsets,
                                (RWS_OVEC_OSIZE / OVEC_UNIT) as u32,
                                local_workspace,
                                RWS_RSIZE as c_int_local,
                                rlevel,
                                RWS,
                            );

                            if rc < 0 {
                                if rc != err::NOMATCH {
                                    return rc;
                                }
                                break;
                            }

                            charcount = *local_offsets.add(1) - *local_offsets.add(0);
                            if charcount == 0 {
                                break;
                            }
                            local_ptr = local_ptr.add(charcount);
                            matched_count += 1;
                        }

                        (*rws).free += (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        if matched_count > 0 || allow_zero {
                            let mut end_subpattern = code;
                            let next_state_offset: c_int_local;

                            loop {
                                end_subpattern = end_subpattern.add(GET(end_subpattern, 1) as usize);
                                if *end_subpattern as u32 != OP_ALT {
                                    break;
                                }
                            }
                            next_state_offset = (end_subpattern as usize - start_code as usize
                                + LINK_SIZE
                                + 1) as c_int_local;

                            if i as usize + 1 >= active_count && new_count == 0 {
                                ptr = local_ptr;
                                clen = 0;
                                ADD_NEW!(next_state_offset, 0);
                            } else {
                                let mut p = ptr;
                                let pp = local_ptr;
                                charcount = (pp as usize - p as usize) as PCRE2_SIZE;
                                if utf {
                                    while p < pp {
                                        let b = *p;
                                        p = p.add(1);
                                        if NOT_FIRSTCU(b as u32) {
                                            charcount -= 1;
                                        }
                                    }
                                }
                                ADD_NEW_DATA!(-next_state_offset, 0, (charcount - 1) as c_int_local);
                            }
                        }
                    }

                    x if x == OP_ONCE => {
                        let rc: c_int_local;
                        let local_workspace: *mut c_int_local;
                        let local_offsets: *mut PCRE2_SIZE;
                        let mut rws = RWS as *mut RWS_anchor;

                        if (*rws).free < (RWS_RSIZE + RWS_OVEC_OSIZE) as u32 {
                            let r = more_workspace(&mut rws, RWS_OVEC_OSIZE as u32, mb);
                            if r != 0 {
                                return r;
                            }
                            RWS = rws as *mut c_int_local;
                        }

                        local_offsets = RWS.add((*rws).size as usize - (*rws).free as usize)
                            as *mut PCRE2_SIZE;
                        local_workspace = (local_offsets as *mut c_int_local).add(RWS_OVEC_OSIZE);
                        (*rws).free -= (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        rc = internal_dfa_match(
                            mb,
                            code,
                            ptr,
                            (ptr as usize - start_subject as usize) as PCRE2_SIZE,
                            local_offsets,
                            (RWS_OVEC_OSIZE / OVEC_UNIT) as u32,
                            local_workspace,
                            RWS_RSIZE as c_int_local,
                            rlevel,
                            RWS,
                        );

                        (*rws).free += (RWS_RSIZE + RWS_OVEC_OSIZE) as u32;

                        if rc >= 0 {
                            let mut end_subpattern = code;
                            let mut charcount = *local_offsets.add(1) - *local_offsets.add(0);
                            let next_state_offset: c_int_local;
                            let repeat_state_offset: c_int_local;

                            loop {
                                end_subpattern = end_subpattern.add(GET(end_subpattern, 1) as usize);
                                if *end_subpattern as u32 != OP_ALT {
                                    break;
                                }
                            }
                            next_state_offset = (end_subpattern as usize - start_code as usize
                                + LINK_SIZE
                                + 1) as c_int_local;

                            repeat_state_offset = if *end_subpattern as u32 == OP_KETRMAX
                                || *end_subpattern as u32 == OP_KETRMIN
                            {
                                (end_subpattern as usize - start_code as usize
                                    - GET(end_subpattern, 1) as usize)
                                    as c_int_local
                            } else {
                                -1
                            };

                            if charcount == 0 {
                                ADD_ACTIVE!(next_state_offset, 0);
                            } else if i as usize + 1 >= active_count && new_count == 0 {
                                ptr = ptr.add(charcount);
                                clen = 0;
                                ADD_NEW!(next_state_offset, 0);

                                if repeat_state_offset >= 0 {
                                    next_active_state = active_states;
                                    active_count = 0;
                                    i = -1;
                                    ADD_ACTIVE!(repeat_state_offset, 0);
                                }
                            } else {
                                if utf {
                                    let mut p = start_subject.add(*local_offsets.add(0));
                                    let pp = start_subject.add(*local_offsets.add(1));
                                    while p < pp {
                                        let b = *p;
                                        p = p.add(1);
                                        if NOT_FIRSTCU(b as u32) {
                                            charcount -= 1;
                                        }
                                    }
                                }
                                ADD_NEW_DATA!(-next_state_offset, 0, (charcount - 1) as c_int_local);
                                if repeat_state_offset >= 0 {
                                    ADD_NEW_DATA!(
                                        -repeat_state_offset,
                                        0,
                                        (charcount - 1) as c_int_local
                                    );
                                }
                            }
                        } else if rc != err::NOMATCH {
                            return rc;
                        }
                    }

                    // ---- Callouts ------------------------------------
                    x if x == OP_CALLOUT || x == OP_CALLOUT_STR => {
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
                            return rrc;
                        }
                        if rrc == 0 {
                            ADD_ACTIVE!(state_offset + callout_length as c_int_local, 0);
                        }
                    }

                    // ---- Unsupported ---------------------------------
                    _ => {
                        return err::DFA_UITEM;
                    }
                }

                // NEXT_ACTIVE_STATE: continue
                i += 1;
            } // End of loop scanning active states

            // Finished processing at the current subject character.
            if new_count == 0 {
                if could_continue
                    && (((*mb).moptions & O_PARTIAL_HARD) != 0
                        || (((*mb).moptions & O_PARTIAL_SOFT) != 0 && match_count < 0))
                    && (partial_newline
                        || (ptr >= end_subject
                            && (ptr > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)))
                {
                    match_count = err::PARTIAL;
                }
                break 'subject;
            }

            ptr = ptr.add(clen as usize);
        } // Loop to move along the subject string

        if match_count >= 0
            && (((*mb).moptions | (*mb).poptions) & O_ENDANCHORED) != 0
            && ptr < end_subject
        {
            match_count = err::NOMATCH;
        }

        match_count
    }
}

// ---------------------------------------------------------------------------
// Whitespace helpers (HSPACE_CASES / VSPACE_CASES).
// ---------------------------------------------------------------------------

#[inline(always)]
fn is_hspace(c: u32) -> bool {
    matches!(
        c,
        CHAR_HT
            | CHAR_SPACE
            | CHAR_NBSP
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

#[inline(always)]
fn is_vspace(c: u32) -> bool {
    matches!(c, CHAR_LF | CHAR_VT | CHAR_FF | CHAR_CR | CHAR_NEL | 0x2028 | 0x2029)
}

// ---------------------------------------------------------------------------
// pcre2_dfa_match() — the exported entry point.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_dfa_match_8(
    code: *const pcre2_code,
    subject: PCRE2_SPTR,
    length: PCRE2_SIZE,
    start_offset: PCRE2_SIZE,
    options: u32,
    match_data: *mut pcre2_match_data,
    mcontext: *mut pcre2_match_context,
    workspace: *mut c_int_local,
    wscount: PCRE2_SIZE,
) -> c_int_local {
    unsafe {
        let mut rc: c_int_local;

        let re = code as *const pcre2_real_code;
        let original_options = options;
        let mut options = options;
        let mut length = length;

        let null_str: [PCRE2_UCHAR; 1] = [0xcd];
        let original_subject = subject;
        let mut subject = subject;
        let mut start_match: PCRE2_SPTR;
        let mut end_subject: PCRE2_SPTR;
        let mut bumpalong_limit: PCRE2_SPTR;
        let mut req_cu_ptr: PCRE2_SPTR;

        let utf: bool;
        let anchored: bool;
        let startline: bool;
        let firstline: bool;
        let mut has_first_cu = false;
        let mut has_req_cu = false;

        let mut memchr_found_first_cu: PCRE2_SPTR = core::ptr::null();
        let mut memchr_found_first_cu2: PCRE2_SPTR = core::ptr::null();

        let mut first_cu: PCRE2_UCHAR = 0;
        let mut first_cu2: PCRE2_UCHAR = 0;
        let mut req_cu: PCRE2_UCHAR = 0;
        let mut req_cu2: PCRE2_UCHAR = 0;

        let mut start_bits: *const u8 = core::ptr::null();

        let mut cb: pcre2_callout_block = core::mem::zeroed();
        let mut actual_match_block: dfa_match_block = core::mem::zeroed();
        let mb: *mut dfa_match_block = &mut actual_match_block;

        // Base recursion workspace on the stack.
        let mut base_recursion_workspace: [c_int_local; RWS_BASE_SIZE] = [0; RWS_BASE_SIZE];
        let rws0 = base_recursion_workspace.as_mut_ptr() as *mut RWS_anchor;
        (*rws0).next = core::ptr::null_mut();
        (*rws0).size = RWS_BASE_SIZE as u32;
        (*rws0).free = (RWS_BASE_SIZE - RWS_ANCHOR_SIZE) as u32;

        // Recognize NULL, length 0 as an empty string.
        if subject.is_null() && length == 0 {
            subject = null_str.as_ptr();
        }

        if match_data.is_null() {
            return err::NULL;
        }

        // The label EXIT cleanup is implemented by an inner closure `exit_cleanup`.
        // We store the final rc via a small state and jump using labelled blocks.

        'exit: {
            if re.is_null() || subject.is_null() || workspace.is_null() {
                rc = err::NULL;
                break 'exit;
            }
            if (options & !PUBLIC_DFA_MATCH_OPTIONS) != 0 {
                rc = err::BADOPTION;
                break 'exit;
            }

            if length == PCRE2_ZERO_TERMINATED {
                length = crate::string_utils::_pcre2_strlen_8(subject);
            }

            if wscount < 20 {
                rc = err::DFA_WSSIZE;
                break 'exit;
            }
            if start_offset > length {
                rc = err::BADOFFSET;
                break 'exit;
            }

            if (options & (O_PARTIAL_HARD | O_PARTIAL_SOFT)) != 0
                && (((*re).overall_options | options) & O_ENDANCHORED) != 0
            {
                rc = err::BADOPTION;
                break 'exit;
            }

            if ((*re).overall_options & O_MATCH_INVALID_UTF) != 0 {
                rc = err::DFA_UINVALID_UTF;
                break 'exit;
            }

            if (*re).magic_number != MAGIC_NUMBER {
                rc = err::BADMAGIC;
                break 'exit;
            }

            if ((*re).flags & PCRE2_MODE_MASK) != (8 / 8) {
                rc = err::BADMODE;
                break 'exit;
            }

            // Transfer (*NOTEMPTY) / (*NOTEMPTY_ATSTART) flag bits to options.
            // FF = NOTEMPTY_SET|NE_ATST_SET, OO = NOTEMPTY|NOTEMPTY_ATSTART.
            const FF: u32 = F_NOTEMPTY_SET | F_NE_ATST_SET;
            const OO: u32 = O_NOTEMPTY | O_NOTEMPTY_ATSTART;
            options |= ((*re).flags & FF) / ((FF & (FF.wrapping_neg())) / (OO & (OO.wrapping_neg())));

            if (options & O_DFA_RESTART) != 0 {
                if (*workspace.add(0) & (-2i32)) != 0
                    || *workspace.add(1) < 1
                    || *workspace.add(1)
                        > ((wscount - 2) / INTS_PER_STATEBLOCK) as c_int_local
                {
                    rc = err::DFA_BADRESTART;
                    break 'exit;
                }
            }

            utf = ((*re).overall_options & O_UTF) != 0;
            start_match = subject.add(start_offset);
            end_subject = subject.add(length);
            req_cu_ptr = start_match.sub(1);
            anchored = (options & (O_ANCHORED | O_DFA_RESTART)) != 0
                || ((*re).overall_options & O_ANCHORED) != 0;

            startline = ((*re).flags & F_STARTLINE) != 0;
            firstline = !anchored && ((*re).overall_options & F_FIRSTLINE) != 0;
            bumpalong_limit = end_subject;

            (*mb).cb = &mut cb;
            cb.version = 2;
            cb.subject = subject;
            cb.subject_length = (end_subject as usize - subject as usize) as PCRE2_SIZE;
            cb.callout_flags = 0;
            cb.capture_top = 1;
            cb.capture_last = 0;
            cb.mark = core::ptr::null();

            if mcontext.is_null() {
                (*mb).callout = None;
                (*mb).memctl = (*re).memctl;
                (*mb).match_limit = crate::context::_pcre2_default_match_context_8.match_limit;
                (*mb).match_limit_depth = crate::context::_pcre2_default_match_context_8.depth_limit;
                (*mb).heap_limit = crate::context::_pcre2_default_match_context_8.heap_limit;
            } else {
                if (*mcontext).offset_limit != PCRE2_UNSET {
                    if ((*re).overall_options & O_USE_OFFSET_LIMIT) == 0 {
                        rc = err::BADOFFSETLIMIT;
                        break 'exit;
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

            (*mb).start_code = (re as *const u8).add((*re).code_start);
            (*mb).tables = (*re).tables;
            (*mb).start_subject = subject;
            (*mb).end_subject = end_subject;
            (*mb).start_offset = start_offset;
            (*mb).allowemptypartial = if (*re).max_lookbehind > 0
                || ((*re).flags & F_MATCH_EMPTY) != 0
            {
                TRUE
            } else {
                FALSE
            };
            (*mb).moptions = options;
            (*mb).poptions = (*re).overall_options;
            (*mb).match_call_count = 0;
            (*mb).heap_used = 0;

            (*mb).bsr_convention = (*re).bsr_convention;
            (*mb).nltype = NLTYPE_FIXED;
            match (*re).newline_convention {
                v if v == PCRE2_NEWLINE_CR => {
                    (*mb).nllen = 1;
                    (*mb).nl[0] = CHAR_CR as u8;
                }
                v if v == PCRE2_NEWLINE_LF => {
                    (*mb).nllen = 1;
                    (*mb).nl[0] = CHAR_NL as u8;
                }
                v if v == PCRE2_NEWLINE_NUL => {
                    (*mb).nllen = 1;
                    (*mb).nl[0] = CHAR_NUL as u8;
                }
                v if v == PCRE2_NEWLINE_CRLF => {
                    (*mb).nllen = 2;
                    (*mb).nl[0] = CHAR_CR as u8;
                    (*mb).nl[1] = CHAR_NL as u8;
                }
                v if v == PCRE2_NEWLINE_ANY => {
                    (*mb).nltype = NLTYPE_ANY;
                }
                v if v == PCRE2_NEWLINE_ANYCRLF => {
                    (*mb).nltype = NLTYPE_ANYCRLF;
                }
                _ => {
                    rc = err::INTERNAL;
                    break 'exit;
                }
            }

            // UTF validity check.
            if utf && (options & O_NO_UTF_CHECK) == 0 {
                let mut check_subject = start_match;

                if start_offset > 0 {
                    if start_match < end_subject && NOT_FIRSTCU(*start_match as u32) {
                        rc = err::BADUTFOFFSET;
                        break 'exit;
                    }
                    let mut ii = (*re).max_lookbehind as u32;
                    while ii > 0 && check_subject > subject {
                        check_subject = check_subject.sub(1);
                        while check_subject > subject && (*check_subject & 0xc0) == 0x80 {
                            check_subject = check_subject.sub(1);
                        }
                        ii -= 1;
                    }
                }

                rc = crate::valid_utf::_pcre2_valid_utf_8(
                    check_subject,
                    length - (check_subject as usize - subject as usize) as PCRE2_SIZE,
                    &mut (*match_data).startchar,
                );
                if rc != 0 {
                    (*match_data).startchar +=
                        (check_subject as usize - subject as usize) as PCRE2_SIZE;
                    break 'exit;
                }
            }

            // First code unit / bitmap.
            if ((*re).flags & F_FIRSTSET) != 0 {
                has_first_cu = true;
                first_cu = (*re).first_codeunit as PCRE2_UCHAR;
                first_cu2 = first_cu;
                if ((*re).flags & F_FIRSTCASELESS) != 0 {
                    first_cu2 = TABLE_GET(first_cu as u32, (*mb).tables.add(FCC_OFFSET), first_cu as u32)
                        as PCRE2_UCHAR;
                    if first_cu > 127 && !utf && ((*re).overall_options & O_UCP) != 0 {
                        first_cu2 = UCD_OTHERCASE(first_cu as u32) as PCRE2_UCHAR;
                    }
                }
            } else if !startline && ((*re).flags & F_FIRSTMAPSET) != 0 {
                start_bits = (*re).start_bitmap.as_ptr();
            }

            if ((*re).flags & F_LASTSET) != 0 {
                has_req_cu = true;
                req_cu = (*re).last_codeunit as PCRE2_UCHAR;
                req_cu2 = req_cu;
                if ((*re).flags & F_LASTCASELESS) != 0 {
                    req_cu2 = TABLE_GET(req_cu as u32, (*mb).tables.add(FCC_OFFSET), req_cu as u32)
                        as PCRE2_UCHAR;
                    if req_cu > 127 && !utf && ((*re).overall_options & O_UCP) != 0 {
                        req_cu2 = UCD_OTHERCASE(req_cu as u32) as PCRE2_UCHAR;
                    }
                }
            }

            // Free previously copied subject if needed.
            if ((*match_data).flags & MD_COPIED_SUBJECT) != 0 {
                if let Some(freefn) = (*match_data).memctl.free {
                    freefn(
                        (*match_data).subject as *mut core::ffi::c_void,
                        (*match_data).memctl.memory_data,
                    );
                }
                (*match_data).flags &= !MD_COPIED_SUBJECT;
            }

            (*match_data).code = re;
            (*match_data).subject = core::ptr::null();
            (*match_data).mark = core::ptr::null();
            (*match_data).matchedby = PCRE2_MATCHEDBY_DFA_INTERPRETER;
            (*match_data).options = original_options;

            // ---------------- Bumpalong loop ----------------
            let mut nomatch = false;
            loop {
                // ----- Start-of-match optimizations -----
                if ((*re).optimization_flags & O_OPTIM_START_OPTIMIZE) != 0
                    && (options & O_DFA_RESTART) == 0
                {
                    if firstline {
                        let mut t = start_match;
                        if utf {
                            while t < end_subject && !is_newline_mb(mb, t, utf) {
                                t = t.add(1);
                                while t < end_subject && (*t & 0xc0) == 0x80 {
                                    t = t.add(1);
                                }
                            }
                        } else {
                            while t < end_subject && !is_newline_mb(mb, t, utf) {
                                t = t.add(1);
                            }
                        }
                        end_subject = t;
                    }

                    if anchored {
                        if has_first_cu || !start_bits.is_null() {
                            let mut ok = start_match < end_subject;
                            if ok {
                                let mut cc = *start_match as u32;
                                ok = has_first_cu
                                    && (cc == first_cu as u32 || cc == first_cu2 as u32);
                                if !ok && !start_bits.is_null() {
                                    let _ = &mut cc;
                                    ok = (*start_bits.add((cc / 8) as usize)
                                        & (1u8 << (cc & 7)))
                                        != 0;
                                }
                            }
                            if !ok {
                                break;
                            }
                        }
                    } else {
                        if has_first_cu {
                            if first_cu != first_cu2 {
                                // Caseless, 8-bit memchr twice with caching.
                                let mut pp1: PCRE2_SPTR;
                                let mut pp2: PCRE2_SPTR;
                                let searchlength = end_subject as usize - start_match as usize;

                                if memchr_found_first_cu.is_null()
                                    || start_match > memchr_found_first_cu
                                {
                                    pp1 = memchr_ptr(start_match, first_cu, searchlength);
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
                                    pp2 = memchr_ptr(start_match, first_cu2, searchlength);
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
                                    start_match = if pp2.is_null() { end_subject } else { pp2 };
                                } else {
                                    start_match = if pp2.is_null() || pp1 < pp2 { pp1 } else { pp2 };
                                }
                            } else {
                                let p = memchr_ptr(
                                    start_match,
                                    first_cu,
                                    end_subject as usize - start_match as usize,
                                );
                                start_match = if p.is_null() { end_subject } else { p };
                            }

                            if ((*mb).moptions & (O_PARTIAL_HARD | O_PARTIAL_SOFT)) == 0
                                && start_match >= (*mb).end_subject
                            {
                                break;
                            }
                        } else if startline {
                            if start_match > (*mb).start_subject.add(start_offset) {
                                if utf {
                                    while start_match < end_subject
                                        && !was_newline_mb(mb, start_match, utf)
                                    {
                                        start_match = start_match.add(1);
                                        while start_match < end_subject
                                            && (*start_match & 0xc0) == 0x80
                                        {
                                            start_match = start_match.add(1);
                                        }
                                    }
                                } else {
                                    while start_match < end_subject
                                        && !was_newline_mb(mb, start_match, utf)
                                    {
                                        start_match = start_match.add(1);
                                    }
                                }

                                if *start_match.offset(-1) as u32 == CHAR_CR
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
                                let cc = *start_match as u32;
                                if (*start_bits.add((cc / 8) as usize) & (1u8 << (cc & 7))) != 0 {
                                    break;
                                }
                                start_match = start_match.add(1);
                            }

                            if ((*mb).moptions & (O_PARTIAL_HARD | O_PARTIAL_SOFT)) == 0
                                && start_match >= (*mb).end_subject
                            {
                                break;
                            }
                        }
                    }

                    end_subject = (*mb).end_subject;

                    if ((*mb).moptions & (O_PARTIAL_HARD | O_PARTIAL_SOFT)) == 0 {
                        let mut p: PCRE2_SPTR;

                        if ((end_subject as usize - start_match as usize) as PCRE2_SIZE)
                            < (*re).minlength as PCRE2_SIZE
                        {
                            nomatch = true;
                            break;
                        }

                        p = start_match.add(if has_first_cu { 1 } else { 0 });
                        if has_req_cu && p > req_cu_ptr {
                            let check_length = end_subject as usize - start_match as usize;

                            if check_length < REQ_CU_MAX
                                || (!anchored && check_length < REQ_CU_MAX * 1000)
                            {
                                if req_cu != req_cu2 {
                                    let pp = p;
                                    p = memchr_ptr(pp, req_cu, end_subject as usize - pp as usize);
                                    if p.is_null() {
                                        p = memchr_ptr(
                                            pp,
                                            req_cu2,
                                            end_subject as usize - pp as usize,
                                        );
                                        if p.is_null() {
                                            p = end_subject;
                                        }
                                    }
                                } else {
                                    p = memchr_ptr(p, req_cu, end_subject as usize - p as usize);
                                    if p.is_null() {
                                        p = end_subject;
                                    }
                                }

                                if p >= end_subject {
                                    break;
                                }
                                req_cu_ptr = p;
                            }
                        }
                    }
                }
                // ----- End of start-of-match optimizations -----

                if start_match > bumpalong_limit {
                    break;
                }

                (*mb).start_used_ptr = start_match;
                (*mb).last_used_ptr = start_match;
                (*mb).recursive = core::ptr::null_mut();

                rc = internal_dfa_match(
                    mb,
                    (*mb).start_code,
                    start_match,
                    start_offset,
                    (*match_data).ovec(),
                    (*match_data).oveccount as u32 * 2,
                    workspace,
                    wscount as c_int_local,
                    0,
                    base_recursion_workspace.as_mut_ptr(),
                );

                if rc != err::NOMATCH || anchored {
                    if rc == err::NOMATCH {
                        nomatch = true;
                        break;
                    }

                    if rc == err::PARTIAL && (*match_data).oveccount > 0 {
                        *(*match_data).ovec().add(0) =
                            (start_match as usize - subject as usize) as PCRE2_SIZE;
                        *(*match_data).ovec().add(1) =
                            (end_subject as usize - subject as usize) as PCRE2_SIZE;
                    }

                    if rc >= 0 || rc == err::PARTIAL {
                        (*match_data).subject_length = length;
                        (*match_data).start_offset = start_offset;
                        (*match_data).leftchar =
                            ((*mb).start_used_ptr as usize - subject as usize) as PCRE2_SIZE;
                        (*match_data).rightchar =
                            ((*mb).last_used_ptr as usize - subject as usize) as PCRE2_SIZE;
                        (*match_data).startchar =
                            (start_match as usize - subject as usize) as PCRE2_SIZE;
                    }

                    if rc >= 0 && (options & O_COPY_MATCHED_SUBJECT) != 0 {
                        if length != 0 {
                            let m = (*match_data).memctl.malloc.unwrap()(
                                CU2BYTES(length),
                                (*match_data).memctl.memory_data,
                            );
                            if m.is_null() {
                                rc = err::NOMEMORY;
                                break 'exit;
                            }
                            (*match_data).subject = m as PCRE2_SPTR;
                            core::ptr::copy_nonoverlapping(
                                subject,
                                m as *mut u8,
                                CU2BYTES(length),
                            );
                        } else {
                            (*match_data).subject = core::ptr::null();
                        }
                        (*match_data).flags |= MD_COPIED_SUBJECT;
                    } else if rc >= 0 || rc == err::PARTIAL {
                        (*match_data).subject = original_subject;
                    }
                    break 'exit;
                }

                // Advance for the next bumpalong iteration.
                if firstline && is_newline_mb(mb, start_match, utf) {
                    break;
                }
                start_match = start_match.add(1);
                if utf {
                    while start_match < end_subject && (*start_match & 0xc0) == 0x80 {
                        start_match = start_match.add(1);
                    }
                }
                if start_match > end_subject {
                    break;
                }

                if *start_match.offset(-1) as u32 == CHAR_CR
                    && start_match < end_subject
                    && *start_match as u32 == CHAR_NL
                    && ((*re).flags & F_HASCRORLF) == 0
                    && ((*mb).nltype == NLTYPE_ANY
                        || (*mb).nltype == NLTYPE_ANYCRLF
                        || (*mb).nllen == 2)
                {
                    start_match = start_match.add(1);
                }
            } // Bumpalong loop

            let _ = nomatch;
            // NOMATCH_EXIT:
            (*match_data).subject = original_subject;
            (*match_data).subject_length = length;
            (*match_data).start_offset = start_offset;
            rc = err::NOMATCH;
        } // 'exit

        // EXIT: free the extra workspace blocks.
        let mut rwsp = rws0;
        while !(*rwsp).next.is_null() {
            let next = (*rwsp).next;
            (*rwsp).next = (*next).next;
            if let Some(freefn) = (*mb).memctl.free {
                freefn(next as *mut core::ffi::c_void, (*mb).memctl.memory_data);
            }
        }

        (*match_data).rc = rc;
        rc
    }
}

// ---------------------------------------------------------------------------
// Small helpers used by the exported function.
// ---------------------------------------------------------------------------

/// IS_NEWLINE evaluated against a dfa_match_block (used outside
/// internal_dfa_match, where `mb` is a pointer and `utf` is a bool).
#[inline]
unsafe fn is_newline_mb(mb: *mut dfa_match_block, p: PCRE2_SPTR, utf: bool) -> bool {
    unsafe {
        if (*mb).nltype != NLTYPE_FIXED {
            p < (*mb).end_subject
                && crate::newline::_pcre2_is_newline_8(
                    p,
                    (*mb).nltype,
                    (*mb).end_subject,
                    &mut (*mb).nllen,
                    utf as BOOL,
                ) != 0
        } else {
            let nllen = (*mb).nllen as usize;
            p as usize <= (*mb).end_subject as usize - nllen
                && *p == (*mb).nl[0]
                && ((*mb).nllen == 1 || *p.add(1) == (*mb).nl[1])
        }
    }
}

/// WAS_NEWLINE evaluated against a dfa_match_block.
#[inline]
unsafe fn was_newline_mb(mb: *mut dfa_match_block, p: PCRE2_SPTR, utf: bool) -> bool {
    unsafe {
        if (*mb).nltype != NLTYPE_FIXED {
            p > (*mb).start_subject
                && crate::newline::_pcre2_was_newline_8(
                    p,
                    (*mb).nltype,
                    (*mb).start_subject,
                    &mut (*mb).nllen,
                    utf as BOOL,
                ) != 0
        } else {
            let nllen = (*mb).nllen as usize;
            p as usize >= (*mb).start_subject as usize + nllen
                && *p.sub(nllen) == (*mb).nl[0]
                && ((*mb).nllen == 1 || *p.sub(nllen).add(1) == (*mb).nl[1])
        }
    }
}

/// `memchr` returning a pointer within `[hay, hay+n)` or NULL.
#[inline]
unsafe fn memchr_ptr(hay: PCRE2_SPTR, needle: PCRE2_UCHAR, n: usize) -> PCRE2_SPTR {
    unsafe {
        let mut i = 0usize;
        while i < n {
            if *hay.add(i) == needle {
                return hay.add(i);
            }
            i += 1;
        }
        core::ptr::null()
    }
}
