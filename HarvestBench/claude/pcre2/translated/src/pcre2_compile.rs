// Translated from pcre2_compile.c (PCRE2 10.48, 8-bit, SUPPORT_UNICODE, no JIT,
// LINK_SIZE=2, IMM2_SIZE=2, SUPPORT_WIDE_CHARS).
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_parens)]
#![allow(unused_assignments)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_mut)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use crate::pcre2_internal::*;

use crate::pcre2_string_utils::_pcre2_strcmp_c8_8 as _pcre2_strcmp_c8;
use crate::pcre2_string_utils::_pcre2_strlen_8 as _pcre2_strlen;
use crate::pcre2_string_utils::_pcre2_strncmp_8 as _pcre2_strncmp;
use crate::pcre2_string_utils::_pcre2_strncmp_c8_8 as _pcre2_strncmp_c8;

// ---------------------------------------------------------------------------
// Local constants
// ---------------------------------------------------------------------------

const MAX_GROUP_NUMBER: u32 = 65535;
const MAX_REPEAT_COUNT: u32 = 65535;
const REPEAT_UNLIMITED: u32 = MAX_REPEAT_COUNT + 1;

const COMPILE_WORK_SIZE: usize = 3000 * LINK_SIZE; // Size in code units
const C16_WORK_SIZE: usize = (COMPILE_WORK_SIZE * 1) / 2; // *sizeof(u8)/sizeof(u16)
const GROUPINFO_DEFAULT_SIZE: usize = 256;
const WORK_SIZE_SAFETY_MARGIN: usize = 100;
const NAMED_GROUP_LIST_SIZE: u32 = 20;
const PARSED_PATTERN_DEFAULT_SIZE: usize = 1024;

const INT_MAX: c_int = 2147483647;
const UINT32_MAX: u32 = 0xffffffff;
const OFLOW_MAX: PCRE2_SIZE = (INT_MAX as PCRE2_SIZE) - 20;

const PCRE2_MAJOR: u32 = 10;
const PCRE2_MINOR: u32 = 48;

// Values and flags for xxcuflags
const REQ_UNSET: u32 = 0xffffffff;
const REQ_NONE: u32 = 0xfffffffe;
const REQ_CASELESS: u32 = 0x00000001;
const REQ_VARY: u32 = 0x00000002;

// groupinfo flags
const GI_SET_FIXED_LENGTH: u32 = 0x80000000;
const GI_NOT_FIXED_LENGTH: u32 = 0x40000000;
const GI_FIXED_LENGTH_MASK: u32 = 0x0000ffff;

// PSKIP types
const PSKIP_ALT: u32 = 0;
const PSKIP_CLASS: u32 = 1;
const PSKIP_KET: u32 = 2;

// Range analysis states
const RANGE_NO: u32 = 0;
const RANGE_STARTED: u32 = 1;
const RANGE_FORBID_NO: u32 = 2;
const RANGE_FORBID_STARTED: u32 = 3;
const RANGE_OK_ESCAPED: u32 = 4;
const RANGE_OK_LITERAL: u32 = 5;

// Class operator states
const CLASS_OP_EMPTY: u32 = 0;
const CLASS_OP_OPERAND: u32 = 1;
const CLASS_OP_OPERATOR: u32 = 2;

// Class parse mode states
const CLASS_MODE_NORMAL: u32 = 0;
const CLASS_MODE_ALT_EXT: u32 = 1;
const CLASS_MODE_PERL_EXT: u32 = 2;
const CLASS_MODE_PERL_EXT_LEAF: u32 = 3;

// nest_save flags
const NSF_RESET: u16 = 0x0001;
const NSF_CONDASSERT: u16 = 0x0002;
const NSF_ATOMICSR: u16 = 0x0004;

const PARSE_TRACKED_OPTIONS: u32 = PCRE2_CASELESS
    | PCRE2_DOTALL
    | PCRE2_DUPNAMES
    | PCRE2_EXTENDED
    | PCRE2_EXTENDED_MORE
    | PCRE2_MULTILINE
    | PCRE2_NO_AUTO_CAPTURE
    | PCRE2_UNGREEDY;

const PARSE_TRACKED_EXTRA_OPTIONS: u32 = PCRE2_EXTRA_CASELESS_RESTRICT
    | PCRE2_EXTRA_ASCII_BSD
    | PCRE2_EXTRA_ASCII_BSS
    | PCRE2_EXTRA_ASCII_BSW
    | PCRE2_EXTRA_ASCII_DIGIT
    | PCRE2_EXTRA_ASCII_POSIX;

// Public option masks
const PUBLIC_LITERAL_COMPILE_OPTIONS: u32 = PCRE2_ANCHORED
    | PCRE2_AUTO_CALLOUT
    | PCRE2_CASELESS
    | PCRE2_ENDANCHORED
    | PCRE2_FIRSTLINE
    | PCRE2_LITERAL
    | PCRE2_MATCH_INVALID_UTF
    | PCRE2_NO_START_OPTIMIZE
    | PCRE2_NO_UTF_CHECK
    | PCRE2_USE_OFFSET_LIMIT
    | PCRE2_UTF;

const PUBLIC_COMPILE_OPTIONS: u32 = PUBLIC_LITERAL_COMPILE_OPTIONS
    | PCRE2_ALLOW_EMPTY_CLASS
    | PCRE2_ALT_BSUX
    | PCRE2_ALT_CIRCUMFLEX
    | PCRE2_ALT_VERBNAMES
    | PCRE2_DOLLAR_ENDONLY
    | PCRE2_DOTALL
    | PCRE2_DUPNAMES
    | PCRE2_EXTENDED
    | PCRE2_EXTENDED_MORE
    | PCRE2_MATCH_UNSET_BACKREF
    | PCRE2_MULTILINE
    | PCRE2_NEVER_BACKSLASH_C
    | PCRE2_NEVER_UCP
    | PCRE2_NEVER_UTF
    | PCRE2_NO_AUTO_CAPTURE
    | PCRE2_NO_AUTO_POSSESS
    | PCRE2_NO_DOTSTAR_ANCHOR
    | PCRE2_UCP
    | PCRE2_UNGREEDY
    | PCRE2_ALT_EXTENDED_CLASS;

const PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS: u32 = PCRE2_EXTRA_MATCH_LINE
    | PCRE2_EXTRA_MATCH_WORD
    | PCRE2_EXTRA_CASELESS_RESTRICT
    | PCRE2_EXTRA_TURKISH_CASING;

const PUBLIC_COMPILE_EXTRA_OPTIONS: u32 = PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS
    | PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES
    | PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL
    | PCRE2_EXTRA_ESCAPED_CR_IS_LF
    | PCRE2_EXTRA_ALT_BSUX
    | PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK
    | PCRE2_EXTRA_ASCII_BSD
    | PCRE2_EXTRA_ASCII_BSS
    | PCRE2_EXTRA_ASCII_BSW
    | PCRE2_EXTRA_ASCII_POSIX
    | PCRE2_EXTRA_ASCII_DIGIT
    | PCRE2_EXTRA_PYTHON_OCTAL
    | PCRE2_EXTRA_NO_BS0
    | PCRE2_EXTRA_NEVER_CALLOUT;

// pso types
const PSO_OPT: u16 = 0;
const PSO_XOPT: u16 = 1;
const PSO_FLG: u16 = 2;
const PSO_NL: u16 = 3;
const PSO_BSR: u16 = 4;
const PSO_LIMH: u16 = 5;
const PSO_LIMM: u16 = 6;
const PSO_LIMD: u16 = 7;
const PSO_OPTMZ: u16 = 8;

const ESCAPES_FIRST: u32 = CHAR_0;
const ESCAPES_LAST: u32 = CHAR_z;

const RREF_ANY: u32 = 0xffff;

const CDATA_RECURSE_ARGS: u16 = 0;

// ---------------------------------------------------------------------------
// Helper macros
// ---------------------------------------------------------------------------

// *p++ = v
macro_rules! wr {
    ($p:expr, $v:expr) => {{
        *$p = $v;
        $p = $p.add(1);
    }};
}

// PUTOFFSET (SIZEOFFSET == 1): *p++ = s
macro_rules! PUTOFFSET {
    ($s:expr, $p:expr) => {{
        *$p = $s as u32;
        $p = $p.add(1);
    }};
}
// GETOFFSET: s = *p++
macro_rules! GETOFFSET {
    ($s:expr, $p:expr) => {{
        $s = *$p as PCRE2_SIZE;
        $p = $p.add(1);
    }};
}
// GETPLUSOFFSET: s = *(++p)
macro_rules! GETPLUSOFFSET {
    ($s:expr, $p:expr) => {{
        $p = $p.add(1);
        $s = *$p as PCRE2_SIZE;
    }};
}
// READPLUSOFFSET: s = p[1]
macro_rules! READPLUSOFFSET {
    ($s:expr, $p:expr) => {{
        $s = *$p.add(1) as PCRE2_SIZE;
    }};
}
// SKIPOFFSET: p++
macro_rules! SKIPOFFSET {
    ($p:expr) => {{
        $p = $p.add(1);
    }};
}

// PUTINC(a,n,d): PUT(a,n,d); a += LINK_SIZE
macro_rules! PUTINC {
    ($a:expr, $n:expr, $d:expr) => {{
        PUT($a, $n, $d);
        $a = $a.add(LINK_SIZE);
    }};
}
// PUT2INC(a,n,d): PUT2(a,n,d); a += IMM2_SIZE
macro_rules! PUT2INC {
    ($a:expr, $n:expr, $d:expr) => {{
        PUT2($a, $n, $d);
        $a = $a.add(IMM2_SIZE);
    }};
}

#[inline]
unsafe fn oplen(op: u8) -> usize {
    _pcre2_OP_lengths_8[op as usize] as usize
}

#[inline]
unsafe fn NAMED_GROUP_GET_HASH(ng: *mut named_group) -> u16 {
    (*ng).hash_dup & NAMED_GROUP_HASH_MASK
}

#[inline]
fn UCD_ANY_I(ch: u32) -> bool {
    (ch | 0x20) == 0x69 || (ch | 1) == 0x0131
}

#[inline]
fn UCD_DOTTED_I(ch: u32) -> bool {
    ch == 0x69 || ch == 0x0130
}

#[inline]
fn IS_DIGIT(x: u32) -> bool {
    x >= CHAR_0 && x <= CHAR_9
}

#[inline]
unsafe fn xdigit(c: u32) -> u32 {
    XDIGITAB[c as usize] as u32
}

// GETCHARINCTEST: c = *eptr++, decode UTF if utf.
#[inline]
unsafe fn getcharinctest(ptr: &mut PCRE2_SPTR, utf: bool) -> u32 {
    let c = **ptr as u32;
    if utf && c >= 0xc0 {
        let (v, n) = GETCHARINC(*ptr);
        *ptr = (*ptr).add(n);
        v
    } else {
        *ptr = (*ptr).add(1);
        c
    }
}

// GETCHARINC (known utf mode true): c = *eptr++ then decode.
#[inline]
unsafe fn getcharinc_utf(ptr: &mut PCRE2_SPTR) -> u32 {
    let (v, n) = GETCHARINC(*ptr);
    *ptr = (*ptr).add(n);
    v
}

// FORWARDCHARTEST(eptr, end)
#[inline]
unsafe fn forwardchartest(ptr: &mut PCRE2_SPTR, end: PCRE2_SPTR) {
    while *ptr < end && (**ptr & 0xc0) == 0x80 {
        *ptr = (*ptr).add(1);
    }
}

// BACKCHAR(eptr)
#[inline]
unsafe fn backchar(ptr: &mut PCRE2_SPTR) {
    while (**ptr & 0xc0) == 0x80 {
        *ptr = (*ptr).sub(1);
    }
}

// PUTCHAR(c, p): write char, return length in code units.
#[inline]
unsafe fn putchar(c: u32, p: *mut PCRE2_UCHAR, utf: bool) -> u32 {
    if utf && c > MAX_UTF_SINGLE_CU {
        crate::pcre2_ord2utf::_pcre2_ord2utf_8(c, p)
    } else {
        *p = c as u8;
        1
    }
}

// ---------------------------------------------------------------------------
// Static tables
// ---------------------------------------------------------------------------

static XDIGITAB: [u8; 256] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
];

// escapes table, indexed from '0' (0x30) to 'z' (0x7a).
static ESCAPES: [i16; 75] = [
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0, // '0'..'9'
    (ESCAPES_FIRST + 0x0a) as i16, // ':'
    (ESCAPES_FIRST + 0x0b) as i16, // ';'
    (ESCAPES_FIRST + 0x0c) as i16, // '<'
    (ESCAPES_FIRST + 0x0d) as i16, // '='
    (ESCAPES_FIRST + 0x0e) as i16, // '>'
    (ESCAPES_FIRST + 0x0f) as i16, // '?'
    (ESCAPES_FIRST + 0x10) as i16, // '@'
    -(ESC_A as i16),               // 'A'
    -(ESC_B as i16),               // 'B'
    -(ESC_C as i16),               // 'C'
    -(ESC_D as i16),               // 'D'
    -(ESC_E as i16),               // 'E'
    0,                             // 'F'
    -(ESC_G as i16),               // 'G'
    -(ESC_H as i16),               // 'H'
    0,                             // 'I'
    0,                             // 'J'
    -(ESC_K as i16),               // 'K'
    0,                             // 'L'
    0,                             // 'M'
    -(ESC_N as i16),               // 'N'
    0,                             // 'O'
    -(ESC_P as i16),               // 'P'
    -(ESC_Q as i16),               // 'Q'
    -(ESC_R as i16),               // 'R'
    -(ESC_S as i16),               // 'S'
    0,                             // 'T'
    0,                             // 'U'
    -(ESC_V as i16),               // 'V'
    -(ESC_W as i16),               // 'W'
    -(ESC_X as i16),               // 'X'
    0,                             // 'Y'
    -(ESC_Z as i16),               // 'Z'
    (ESCAPES_FIRST + 0x2b) as i16, // '['
    (ESCAPES_FIRST + 0x2c) as i16, // '\'
    (ESCAPES_FIRST + 0x2d) as i16, // ']'
    (ESCAPES_FIRST + 0x2e) as i16, // '^'
    (ESCAPES_FIRST + 0x2f) as i16, // '_'
    (ESCAPES_FIRST + 0x30) as i16, // '`'
    CHAR_BEL as i16,               // 'a'
    -(ESC_b as i16),               // 'b'
    0,                             // 'c'
    -(ESC_d as i16),               // 'd'
    CHAR_ESC as i16,               // 'e'
    CHAR_FF as i16,                // 'f'
    0,                             // 'g'
    -(ESC_h as i16),               // 'h'
    0,                             // 'i'
    0,                             // 'j'
    -(ESC_k as i16),               // 'k'
    0,                             // 'l'
    0,                             // 'm'
    CHAR_LF as i16,                // 'n'
    0,                             // 'o'
    -(ESC_p as i16),               // 'p'
    0,                             // 'q'
    CHAR_CR as i16,                // 'r'
    -(ESC_s as i16),               // 's'
    CHAR_HT as i16,                // 't'
    0,                             // 'u'
    -(ESC_v as i16),               // 'v'
    -(ESC_w as i16),               // 'w'
    0,                             // 'x'
    0,                             // 'y'
    -(ESC_z as i16),               // 'z'
];

// Table of extra lengths for each meta code (SIZEOFFSET == 1).
static META_EXTRA_LENGTHS: [u8; 73] = [
    0, // META_END
    0, // META_ALT
    0, // META_ATOMIC
    0, // META_BACKREF
    2, // META_BACKREF_BYNAME (1+SIZEOFFSET)
    1, // META_BIGVALUE
    3, // META_CALLOUT_NUMBER
    4, // META_CALLOUT_STRING (3+SIZEOFFSET)
    0, // META_CAPTURE
    0, // META_CIRCUMFLEX
    0, // META_CLASS
    0, // META_CLASS_EMPTY
    0, // META_CLASS_EMPTY_NOT
    0, // META_CLASS_END
    0, // META_CLASS_NOT
    0, // META_COND_ASSERT
    1, // META_COND_DEFINE (SIZEOFFSET)
    2, // META_COND_NAME (1+SIZEOFFSET)
    2, // META_COND_NUMBER
    2, // META_COND_RNAME
    2, // META_COND_RNUMBER
    3, // META_COND_VERSION
    1, // META_OFFSET (SIZEOFFSET)
    0, // META_SCS
    1, // META_CAPTURE_NAME
    1, // META_CAPTURE_NUMBER
    0, // META_DOLLAR
    0, // META_DOT
    0, // META_ESCAPE
    0, // META_KET
    0, // META_NOCAPTURE
    2, // META_OPTIONS
    1, // META_POSIX
    1, // META_POSIX_NEG
    0, // META_RANGE_ESCAPED
    0, // META_RANGE_LITERAL
    1, // META_RECURSE (SIZEOFFSET)
    2, // META_RECURSE_BYNAME (1+SIZEOFFSET)
    0, // META_SCRIPT_RUN
    0, // META_LOOKAHEAD
    0, // META_LOOKAHEADNOT
    1, // META_LOOKBEHIND (SIZEOFFSET)
    1, // META_LOOKBEHINDNOT (SIZEOFFSET)
    0, // META_LOOKAHEAD_NA
    1, // META_LOOKBEHIND_NA (SIZEOFFSET)
    1, // META_MARK
    0, // META_ACCEPT
    0, // META_FAIL
    0, // META_COMMIT
    1, // META_COMMIT_ARG
    0, // META_PRUNE
    1, // META_PRUNE_ARG
    0, // META_SKIP
    1, // META_SKIP_ARG
    0, // META_THEN
    1, // META_THEN_ARG
    0, // META_ASTERISK
    0, // META_ASTERISK_PLUS
    0, // META_ASTERISK_QUERY
    0, // META_PLUS
    0, // META_PLUS_PLUS
    0, // META_PLUS_QUERY
    0, // META_QUERY
    0, // META_QUERY_PLUS
    0, // META_QUERY_QUERY
    2, // META_MINMAX
    2, // META_MINMAX_PLUS
    2, // META_MINMAX_QUERY
    0, // META_ECLASS_AND
    0, // META_ECLASS_OR
    0, // META_ECLASS_SUB
    0, // META_ECLASS_XOR
    0, // META_ECLASS_NOT
];

// opcode_possessify, indexed by opcode, up to OP_CALLOUT (119) inclusive.
static OPCODE_POSSESSIFY: [u8; 120] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0-15
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 16-31
    0,           // NOTI
    OP_POSSTAR, 0, // STAR, MINSTAR
    OP_POSPLUS, 0,
    OP_POSQUERY, 0,
    OP_POSUPTO, 0,
    0, // EXACT
    0, 0, 0, 0,
    OP_POSSTARI, 0,
    OP_POSPLUSI, 0,
    OP_POSQUERYI, 0,
    OP_POSUPTOI, 0,
    0, // EXACTI
    0, 0, 0, 0,
    OP_NOTPOSSTAR, 0,
    OP_NOTPOSPLUS, 0,
    OP_NOTPOSQUERY, 0,
    OP_NOTPOSUPTO, 0,
    0, // NOTEXACT
    0, 0, 0, 0,
    OP_NOTPOSSTARI, 0,
    OP_NOTPOSPLUSI, 0,
    OP_NOTPOSQUERYI, 0,
    OP_NOTPOSUPTOI, 0,
    0, // NOTEXACTI
    0, 0, 0, 0,
    OP_TYPEPOSSTAR, 0,
    OP_TYPEPOSPLUS, 0,
    OP_TYPEPOSQUERY, 0,
    OP_TYPEPOSUPTO, 0,
    0, // TYPEEXACT
    0, 0, 0, 0,
    OP_CRPOSSTAR, 0,
    OP_CRPOSPLUS, 0,
    OP_CRPOSQUERY, 0,
    OP_CRPOSRANGE, 0,
    0, 0, 0, 0, // CRPOS...
    0, 0, 0, 0, // CLASS, NCLASS, XCLASS, ECLASS
    0, 0,       // REF, REFI
    0, 0,       // DNREF, DNREFI
    0, 0,       // RECURSE, CALLOUT
];

// Verb names, concatenated with embedded NULs.
static VERBNAMES: &[u8] = b"\0MARK\0ACCEPT\0F\0FAIL\0COMMIT\0PRUNE\0SKIP\0THEN\0";

#[derive(Clone, Copy)]
struct VerbItem {
    len: u32,
    meta: u32,
    has_arg: i32,
}

static VERBS: [VerbItem; 9] = [
    VerbItem { len: 0, meta: META_MARK, has_arg: 1 },
    VerbItem { len: 4, meta: META_MARK, has_arg: 1 },
    VerbItem { len: 6, meta: META_ACCEPT, has_arg: -1 },
    VerbItem { len: 1, meta: META_FAIL, has_arg: -1 },
    VerbItem { len: 4, meta: META_FAIL, has_arg: -1 },
    VerbItem { len: 6, meta: META_COMMIT, has_arg: 0 },
    VerbItem { len: 5, meta: META_PRUNE, has_arg: 0 },
    VerbItem { len: 4, meta: META_SKIP, has_arg: 0 },
    VerbItem { len: 4, meta: META_THEN, has_arg: 0 },
];

const VERBCOUNT: usize = 9;

static VERBOPS: [u32; 11] = [
    OP_MARK as u32,
    OP_ACCEPT as u32,
    OP_FAIL as u32,
    OP_COMMIT as u32,
    OP_COMMIT_ARG as u32,
    OP_PRUNE as u32,
    OP_PRUNE_ARG as u32,
    OP_SKIP as u32,
    OP_SKIP_ARG as u32,
    OP_THEN as u32,
    OP_THEN_ARG as u32,
];

// Alpha assertion names.
static ALASNAMES: &[u8] = b"pla\0plb\0napla\0naplb\0nla\0nlb\0positive_lookahead\0positive_lookbehind\0non_atomic_positive_lookahead\0non_atomic_positive_lookbehind\0negative_lookahead\0negative_lookbehind\0scs\0scan_substring\0atomic\0sr\0asr\0script_run\0atomic_script_run\0";

#[derive(Clone, Copy)]
struct AlasItem {
    len: u32,
    meta: u32,
}

static ALASMETA: [AlasItem; 19] = [
    AlasItem { len: 3, meta: META_LOOKAHEAD },
    AlasItem { len: 3, meta: META_LOOKBEHIND },
    AlasItem { len: 5, meta: META_LOOKAHEAD_NA },
    AlasItem { len: 5, meta: META_LOOKBEHIND_NA },
    AlasItem { len: 3, meta: META_LOOKAHEADNOT },
    AlasItem { len: 3, meta: META_LOOKBEHINDNOT },
    AlasItem { len: 18, meta: META_LOOKAHEAD },
    AlasItem { len: 19, meta: META_LOOKBEHIND },
    AlasItem { len: 29, meta: META_LOOKAHEAD_NA },
    AlasItem { len: 30, meta: META_LOOKBEHIND_NA },
    AlasItem { len: 18, meta: META_LOOKAHEADNOT },
    AlasItem { len: 19, meta: META_LOOKBEHINDNOT },
    AlasItem { len: 3, meta: META_SCS },
    AlasItem { len: 14, meta: META_SCS },
    AlasItem { len: 6, meta: META_ATOMIC },
    AlasItem { len: 2, meta: META_SCRIPT_RUN },
    AlasItem { len: 3, meta: META_ATOMIC_SCRIPT_RUN },
    AlasItem { len: 10, meta: META_SCRIPT_RUN },
    AlasItem { len: 17, meta: META_ATOMIC_SCRIPT_RUN },
];

const ALASCOUNT: usize = 19;

static CHARTYPEOFFSET: [u32; 4] = [
    (OP_STAR - OP_STAR) as u32,
    (OP_STARI - OP_STAR) as u32,
    (OP_NOTSTAR - OP_STAR) as u32,
    (OP_NOTSTARI - OP_STAR) as u32,
];

static POSIX_NAMES: &[u8] = b"alpha\0lower\0upper\0alnum\0ascii\0blank\0cntrl\0digit\0graph\0print\0punct\0space\0word\0xdigit\0";

static POSIX_NAME_LENGTHS: [u8; 15] = [5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 4, 6, 0];

// POSIX class Unicode substitutes.
static POSIX_SUBSTITUTES: [i32; 28] = [
    PT_GC as i32, ucp_L as i32,  // alpha
    PT_PC as i32, ucp_Ll as i32, // lower
    PT_PC as i32, ucp_Lu as i32, // upper
    PT_ALNUM as i32, 0,          // alnum
    -1, 0,                       // ascii
    -1, 1,                       // blank
    PT_PC as i32, ucp_Cc as i32, // cntrl
    PT_PC as i32, ucp_Nd as i32, // digit
    PT_PXGRAPH as i32, 0,        // graph
    PT_PXPRINT as i32, 0,        // print
    PT_PXPUNCT as i32, 0,        // punct
    PT_PXSPACE as i32, 0,        // space
    PT_WORD as i32, 0,           // word
    PT_PXXDIGIT as i32, 0,       // xdigit
];

#[derive(Clone, Copy)]
struct Pso {
    name: &'static [u8],
    length: u16,
    typ: u16,
    value: u32,
}

static PSO_LIST: [Pso; 23] = [
    Pso { name: b"UTF8)", length: 5, typ: PSO_OPT, value: PCRE2_UTF },
    Pso { name: b"UTF)", length: 4, typ: PSO_OPT, value: PCRE2_UTF },
    Pso { name: b"UCP)", length: 4, typ: PSO_OPT, value: PCRE2_UCP },
    Pso { name: b"NOTEMPTY)", length: 9, typ: PSO_FLG, value: PCRE2_NOTEMPTY_SET },
    Pso { name: b"NOTEMPTY_ATSTART)", length: 17, typ: PSO_FLG, value: PCRE2_NE_ATST_SET },
    Pso { name: b"NO_AUTO_POSSESS)", length: 16, typ: PSO_OPTMZ, value: PCRE2_OPTIM_AUTO_POSSESS },
    Pso { name: b"NO_DOTSTAR_ANCHOR)", length: 18, typ: PSO_OPTMZ, value: PCRE2_OPTIM_DOTSTAR_ANCHOR },
    Pso { name: b"NO_JIT)", length: 7, typ: PSO_FLG, value: PCRE2_NOJIT },
    Pso { name: b"NO_START_OPT)", length: 13, typ: PSO_OPTMZ, value: PCRE2_OPTIM_START_OPTIMIZE },
    Pso { name: b"CASELESS_RESTRICT)", length: 18, typ: PSO_XOPT, value: PCRE2_EXTRA_CASELESS_RESTRICT },
    Pso { name: b"TURKISH_CASING)", length: 15, typ: PSO_XOPT, value: PCRE2_EXTRA_TURKISH_CASING },
    Pso { name: b"LIMIT_HEAP=", length: 11, typ: PSO_LIMH, value: 0 },
    Pso { name: b"LIMIT_MATCH=", length: 12, typ: PSO_LIMM, value: 0 },
    Pso { name: b"LIMIT_DEPTH=", length: 12, typ: PSO_LIMD, value: 0 },
    Pso { name: b"LIMIT_RECURSION=", length: 16, typ: PSO_LIMD, value: 0 },
    Pso { name: b"CR)", length: 3, typ: PSO_NL, value: PCRE2_NEWLINE_CR },
    Pso { name: b"LF)", length: 3, typ: PSO_NL, value: PCRE2_NEWLINE_LF },
    Pso { name: b"CRLF)", length: 5, typ: PSO_NL, value: PCRE2_NEWLINE_CRLF },
    Pso { name: b"ANY)", length: 4, typ: PSO_NL, value: PCRE2_NEWLINE_ANY },
    Pso { name: b"NUL)", length: 4, typ: PSO_NL, value: PCRE2_NEWLINE_NUL },
    Pso { name: b"ANYCRLF)", length: 8, typ: PSO_NL, value: PCRE2_NEWLINE_ANYCRLF },
    Pso { name: b"BSR_ANYCRLF)", length: 12, typ: PSO_BSR, value: PCRE2_BSR_ANYCRLF },
    Pso { name: b"BSR_UNICODE)", length: 12, typ: PSO_BSR, value: PCRE2_BSR_UNICODE },
];

// nest_save structure
#[repr(C)]
#[derive(Clone, Copy)]
struct nest_save {
    nest_depth: u16,
    reset_group: u16,
    max_group: u16,
    flags: u16,
    options: u32,
    xoptions: u32,
}

// ---------------------------------------------------------------------------
// Copy compiled code
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_code_copy_8(code: *const pcre2_code) -> *mut pcre2_code {
    if code.is_null() {
        return ptr::null_mut();
    }
    let newcode = ((*code).memctl.malloc.unwrap())((*code).blocksize, (*code).memctl.memory_data)
        as *mut pcre2_code;
    if newcode.is_null() {
        return ptr::null_mut();
    }
    memcpy(newcode as *mut c_void, code as *const c_void, (*code).blocksize);
    (*newcode).executable_jit = ptr::null_mut();

    if ((*code).flags & PCRE2_DEREF_TABLES) != 0 {
        let ref_count = (*code).tables.add(TABLES_LENGTH) as *mut PCRE2_SIZE;
        *ref_count += 1;
    }

    newcode
}

// ---------------------------------------------------------------------------
// Copy compiled code and character tables
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_code_copy_with_tables_8(
    code: *const pcre2_code,
) -> *mut pcre2_code {
    if code.is_null() {
        return ptr::null_mut();
    }
    let newcode = ((*code).memctl.malloc.unwrap())((*code).blocksize, (*code).memctl.memory_data)
        as *mut pcre2_code;
    if newcode.is_null() {
        return ptr::null_mut();
    }
    memcpy(newcode as *mut c_void, code as *const c_void, (*code).blocksize);
    (*newcode).executable_jit = ptr::null_mut();

    let newtables = ((*code).memctl.malloc.unwrap())(
        TABLES_LENGTH + core::mem::size_of::<PCRE2_SIZE>(),
        (*code).memctl.memory_data,
    ) as *mut u8;
    if newtables.is_null() {
        ((*code).memctl.free.unwrap())(newcode as *mut c_void, (*code).memctl.memory_data);
        return ptr::null_mut();
    }
    memcpy(newtables as *mut c_void, (*code).tables as *const c_void, TABLES_LENGTH);
    let ref_count = newtables.add(TABLES_LENGTH) as *mut PCRE2_SIZE;
    *ref_count = 1;

    (*newcode).tables = newtables;
    (*newcode).flags |= PCRE2_DEREF_TABLES;
    newcode
}

// ---------------------------------------------------------------------------
// Free compiled code
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_code_free_8(code: *mut pcre2_code) {
    if !code.is_null() {
        if !(*code).executable_jit.is_null() {
            crate::pcre2_jit_compile::_pcre2_jit_free_8(
                (*code).executable_jit,
                &mut (*code).memctl as *mut pcre2_memctl,
            );
        }

        if ((*code).flags & PCRE2_DEREF_TABLES) != 0 {
            let ref_count = (*code).tables.add(TABLES_LENGTH) as *mut PCRE2_SIZE;
            if *ref_count > 0 {
                *ref_count -= 1;
                if *ref_count == 0 {
                    ((*code).memctl.free.unwrap())(
                        (*code).tables as *mut c_void,
                        (*code).memctl.memory_data,
                    );
                }
            }
        }

        ((*code).memctl.free.unwrap())(code as *mut c_void, (*code).memctl.memory_data);
    }
}

// ---------------------------------------------------------------------------
// Read a number, possibly signed
// ---------------------------------------------------------------------------

unsafe fn read_number(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    allow_sign: i32,
    mut max_value: u32,
    max_error: u32,
    intptr: *mut c_int,
    errorcodeptr: *mut c_int,
) -> BOOL {
    let mut sign: i32 = 0;
    let mut n: u32 = 0;
    let mut ptr = *ptrptr;
    let mut yield_: BOOL = FALSE;

    *errorcodeptr = 0;

    if allow_sign >= 0 && ptr < ptrend {
        if *ptr as u32 == CHAR_PLUS {
            sign = 1;
            max_value -= allow_sign as u32;
            ptr = ptr.add(1);
        } else if *ptr as u32 == CHAR_MINUS {
            sign = -1;
            ptr = ptr.add(1);
        }
    }

    if ptr >= ptrend || !IS_DIGIT(*ptr as u32) {
        *ptrptr = ptr;
        return FALSE;
    }
    while ptr < ptrend && IS_DIGIT(*ptr as u32) {
        n = n * 10 + (*ptr as u32 - CHAR_0);
        ptr = ptr.add(1);
        if n > max_value {
            *errorcodeptr = max_error as c_int;
            while ptr < ptrend && IS_DIGIT(*ptr as u32) {
                ptr = ptr.add(1);
            }
            *intptr = n as c_int;
            *ptrptr = ptr;
            return yield_;
        }
    }

    if allow_sign >= 0 && sign != 0 {
        if n == 0 {
            *errorcodeptr = ERR26; // +0 and -0 are not allowed
            *intptr = n as c_int;
            *ptrptr = ptr;
            return yield_;
        }

        if sign > 0 {
            n += allow_sign as u32;
        } else if n > allow_sign as u32 {
            *errorcodeptr = ERR15; // Non-existent subpattern
            *intptr = n as c_int;
            *ptrptr = ptr;
            return yield_;
        } else {
            n = allow_sign as u32 + 1 - n;
        }
    }

    yield_ = TRUE;

    *intptr = n as c_int;
    *ptrptr = ptr;
    yield_
}

// ---------------------------------------------------------------------------
// Read repeat counts
// ---------------------------------------------------------------------------

unsafe fn read_repeat_counts(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    minp: *mut u32,
    maxp: *mut u32,
    errorcodeptr: *mut c_int,
) -> BOOL {
    let mut p = *ptrptr;
    let mut pp: PCRE2_SPTR;
    let mut yield_: BOOL = FALSE;
    let mut had_minimum: BOOL = FALSE;
    let mut min: i32 = 0;
    let mut max: i32 = REPEAT_UNLIMITED as i32;

    *errorcodeptr = 0;
    while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
        p = p.add(1);
    }

    pp = p;
    if pp < ptrend && IS_DIGIT(*pp as u32) {
        had_minimum = TRUE;
        pp = pp.add(1);
        while pp < ptrend && IS_DIGIT(*pp as u32) {
            pp = pp.add(1);
        }
    }

    while pp < ptrend && (*pp as u32 == CHAR_SPACE || *pp as u32 == CHAR_HT) {
        pp = pp.add(1);
    }
    if pp >= ptrend {
        *ptrptr = p;
        return FALSE;
    }

    if *pp as u32 == CHAR_RIGHT_CURLY_BRACKET {
        if had_minimum == FALSE {
            *ptrptr = p;
            return FALSE;
        }
    } else {
        if *pp as u32 != CHAR_COMMA {
            *ptrptr = p;
            return FALSE;
        }
        pp = pp.add(1);
        while pp < ptrend && (*pp as u32 == CHAR_SPACE || *pp as u32 == CHAR_HT) {
            pp = pp.add(1);
        }
        if pp >= ptrend {
            *ptrptr = p;
            return FALSE;
        }
        if IS_DIGIT(*pp as u32) {
            pp = pp.add(1);
            while pp < ptrend && IS_DIGIT(*pp as u32) {
                pp = pp.add(1);
            }
        } else if had_minimum == FALSE {
            *ptrptr = p;
            return FALSE;
        }
        while pp < ptrend && (*pp as u32 == CHAR_SPACE || *pp as u32 == CHAR_HT) {
            pp = pp.add(1);
        }
        if pp >= ptrend || *pp as u32 != CHAR_RIGHT_CURLY_BRACKET {
            *ptrptr = p;
            return FALSE;
        }
    }

    // Now process the quantifier for real.
    if read_number(&mut p, ptrend, -1, MAX_REPEAT_COUNT, ERR5 as u32, &mut min, errorcodeptr)
        == FALSE
    {
        if *errorcodeptr != 0 {
            *ptrptr = p;
            return yield_;
        }
        p = p.add(1); // Skip comma and subsequent spaces
        while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
            p = p.add(1);
        }
        if read_number(&mut p, ptrend, -1, MAX_REPEAT_COUNT, ERR5 as u32, &mut max, errorcodeptr)
            == FALSE
        {
            if *errorcodeptr != 0 {
                *ptrptr = p;
                return yield_;
            }
        }
    } else {
        while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
            p = p.add(1);
        }
        if *p as u32 == CHAR_RIGHT_CURLY_BRACKET {
            max = min;
        } else {
            p = p.add(1); // Skip comma and subsequent spaces
            while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
                p = p.add(1);
            }
            if read_number(&mut p, ptrend, -1, MAX_REPEAT_COUNT, ERR5 as u32, &mut max, errorcodeptr)
                == FALSE
            {
                if *errorcodeptr != 0 {
                    *ptrptr = p;
                    return yield_;
                }
            }

            if max < min {
                *errorcodeptr = ERR4;
                *ptrptr = p;
                return yield_;
            }
        }
    }

    // Valid quantifier exists
    while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
        p = p.add(1);
    }
    p = p.add(1);
    yield_ = TRUE;
    if !minp.is_null() {
        *minp = min as u32;
    }
    if !maxp.is_null() {
        *maxp = max as u32;
    }

    *ptrptr = p;
    yield_
}

// ---------------------------------------------------------------------------
// Handle escapes: PRIV(check_escape)
// ---------------------------------------------------------------------------

enum HexOut {
    Done,          // C "break" out of switch -> EXIT
    FailedForward, // goto ESCAPE_FAILED_FORWARD
}

// Shared code entered at the COME_FROM_NU label. On entry ptr points just after
// "\x{" (spaces skipped) or after "U+" for \N{U+...}.
#[inline]
unsafe fn scan_hex_braces(
    ptr: &mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    utf: bool,
    xoptions: u32,
    c: &mut u32,
    errorcodeptr: *mut c_int,
) -> HexOut {
    let mut cc: u32;
    let mut overflow = false;

    if *ptr >= ptrend || **ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
        *errorcodeptr = ERR78;
        return HexOut::Done;
    }
    *c = 0;

    while *ptr < ptrend && {
        cc = xdigit(**ptr as u32);
        cc != 0xff
    } {
        *ptr = (*ptr).add(1);
        if *c == 0 && cc == 0 {
            continue; // Leading zeroes
        }
        *c = (*c << 4) | cc;
        if (utf && *c > 0x10ffff) || (!utf && *c > MAX_NON_UTF_CHAR) {
            overflow = true;
            break;
        }
    }

    while *ptr < ptrend && (**ptr as u32 == CHAR_SPACE || **ptr as u32 == CHAR_HT) {
        *ptr = (*ptr).add(1);
    }

    if overflow {
        while *ptr < ptrend && xdigit(**ptr as u32) != 0xff {
            *ptr = (*ptr).add(1);
        }
        *errorcodeptr = ERR34;
    } else if utf && *c >= 0xd800 && *c <= 0xdfff
        && (xoptions & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES) == 0
    {
        *errorcodeptr = ERR73;
    } else if *ptr < ptrend && **ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
        *ptr = (*ptr).add(1);
    } else {
        *errorcodeptr = ERR67;
        return HexOut::FailedForward;
    }
    HexOut::Done
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_check_escape_8(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    chptr: *mut u32,
    errorcodeptr: *mut c_int,
    options: u32,
    xoptions: u32,
    bracount: u32,
    isclass: BOOL,
    cb: *mut compile_block,
) -> c_int {
    let utf = (options & PCRE2_UTF) != 0;
    let mut alt_bsux =
        ((options & PCRE2_ALT_BSUX) | (xoptions & PCRE2_EXTRA_ALT_BSUX)) != 0;
    let mut ptr = *ptrptr;
    let mut c: u32;
    let mut cc: u32;
    let mut escape: c_int = 0;
    let mut i: c_int;

    // If backslash is at the end of the string, it's an error.
    if ptr >= ptrend {
        *errorcodeptr = ERR1;
        // In C, this early-return doesn't update *ptrptr or *chptr.
        return 0;
    }

    c = getcharinctest(&mut ptr, utf);
    *errorcodeptr = 0;

    let mut failed_forward = false;

    'exit: {
        if c < ESCAPES_FIRST || c > ESCAPES_LAST {
            // Definitely literal
        } else if {
            i = ESCAPES[(c - ESCAPES_FIRST) as usize] as c_int;
            i != 0
        } {
            if i > 0 {
                c = i as u32;
                if c == CHAR_CR && (xoptions & PCRE2_EXTRA_ESCAPED_CR_IS_LF) != 0 {
                    c = CHAR_LF;
                }
            } else {
                escape = -i;
                if !cb.is_null() && (escape == ESC_P || escape == ESC_p || escape == ESC_X) {
                    (*cb).external_flags |= PCRE2_HASBKPORX;
                }

                if escape == ESC_N && ptr < ptrend && *ptr as u32 == CHAR_LEFT_CURLY_BRACKET {
                    let mut p = ptr.add(1);
                    while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
                        p = p.add(1);
                    }

                    if (ptrend as usize - p as usize) > 1
                        && *p as u32 == CHAR_U
                        && *p.add(1) as u32 == CHAR_PLUS
                    {
                        if utf {
                            ptr = p.add(2);
                            escape = 0;
                            // goto COME_FROM_NU
                            match scan_hex_braces(&mut ptr, ptrend, utf, xoptions, &mut c, errorcodeptr) {
                                HexOut::Done => break 'exit,
                                HexOut::FailedForward => {
                                    failed_forward = true;
                                    break 'exit;
                                }
                            }
                        }
                        // Non-utf path falls to error (dead in UTF build always? utf may be false)
                        ptr = p.add(2);
                        while ptr < ptrend && xdigit(*ptr as u32) != 0xff {
                            ptr = ptr.add(1);
                        }
                        while ptr < ptrend && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT) {
                            ptr = ptr.add(1);
                        }
                        if ptr < ptrend && *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
                            ptr = ptr.add(1);
                        }
                        *errorcodeptr = ERR93;
                    } else if isclass != 0 || cb.is_null() {
                        ptr = ptr.add(1);
                        *errorcodeptr = ERR37;
                    } else {
                        if read_repeat_counts(&mut p, ptrend, ptr::null_mut(), ptr::null_mut(), errorcodeptr) == FALSE
                            && *errorcodeptr == 0
                        {
                            ptr = ptr.add(1);
                            *errorcodeptr = ERR37;
                        }
                    }
                }
            }
        } else {
            // Zero entry: further processing. In C, i == escapes[...] == 0 here.
            i = 0;
            let mut s: c_int;
            let mut oldptr: PCRE2_SPTR;
            let mut overflow: bool;

            if cb.is_null() {
                if !(c >= CHAR_0 && c <= CHAR_9)
                    && c != CHAR_c
                    && c != CHAR_o
                    && c != CHAR_x
                    && c != CHAR_g
                {
                    *errorcodeptr = ERR3;
                    break 'exit;
                }
                alt_bsux = false;
            }

            'sw: {
                match c {
                    x if x == CHAR_F || x == CHAR_l || x == CHAR_L => {
                        *errorcodeptr = ERR37;
                    }

                    x if x == CHAR_u => {
                        if !alt_bsux {
                            *errorcodeptr = ERR37;
                        } else {
                            let mut xc: u32;
                            if ptr >= ptrend {
                                break 'sw;
                            }
                            if *ptr as u32 == CHAR_LEFT_CURLY_BRACKET
                                && (xoptions & PCRE2_EXTRA_ALT_BSUX) != 0
                            {
                                let mut hptr = ptr.add(1);
                                cc = 0;
                                while hptr < ptrend && {
                                    xc = xdigit(*hptr as u32);
                                    xc != 0xff
                                } {
                                    if (cc & 0xf0000000) != 0 {
                                        *errorcodeptr = ERR77;
                                        ptr = hptr;
                                        break;
                                    }
                                    cc = (cc << 4) | xc;
                                    hptr = hptr.add(1);
                                }

                                if hptr == ptr.add(1)
                                    || hptr >= ptrend
                                    || *hptr as u32 != CHAR_RIGHT_CURLY_BRACKET
                                {
                                    if isclass != 0 {
                                        break 'sw;
                                    }
                                    escape = ESC_ub;
                                    ptr = ptr.add(1);
                                    break 'sw;
                                }

                                c = cc;
                                ptr = hptr.add(1);
                            } else {
                                if (ptrend as usize - ptr as usize) < 4 {
                                    break 'sw;
                                }
                                cc = xdigit(*ptr as u32);
                                if cc == 0xff {
                                    break 'sw;
                                }
                                xc = xdigit(*ptr.add(1) as u32);
                                if xc == 0xff {
                                    break 'sw;
                                }
                                cc = (cc << 4) | xc;
                                xc = xdigit(*ptr.add(2) as u32);
                                if xc == 0xff {
                                    break 'sw;
                                }
                                cc = (cc << 4) | xc;
                                xc = xdigit(*ptr.add(3) as u32);
                                if xc == 0xff {
                                    break 'sw;
                                }
                                c = (cc << 4) | xc;
                                ptr = ptr.add(4);
                            }

                            if utf {
                                if c > 0x10ffff {
                                    *errorcodeptr = ERR77;
                                } else if c >= 0xd800
                                    && c <= 0xdfff
                                    && (xoptions & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES) == 0
                                {
                                    *errorcodeptr = ERR73;
                                }
                            } else if c > MAX_NON_UTF_CHAR {
                                *errorcodeptr = ERR77;
                            }
                        }
                    }

                    x if x == CHAR_U => {
                        if !alt_bsux {
                            *errorcodeptr = ERR37;
                        }
                    }

                    x if x == CHAR_g => {
                        if isclass != 0 {
                            break 'sw;
                        }

                        if ptr >= ptrend {
                            *errorcodeptr = ERR57;
                            break 'sw;
                        }

                        if cb.is_null() {
                            if *ptr as u32 != CHAR_LESS_THAN_SIGN {
                                *errorcodeptr = ERR57;
                                break 'sw;
                            }
                            let mut p = ptr.add(1);
                            s = 0;
                            if read_number(&mut p, ptrend, -1, MAX_GROUP_NUMBER, ERR61 as u32, &mut s, errorcodeptr) == FALSE {
                                if *errorcodeptr == 0 {
                                    escape = ESC_g;
                                }
                                break 'sw;
                            }
                            if p >= ptrend || *p as u32 != CHAR_GREATER_THAN_SIGN {
                                ptr = p;
                                *errorcodeptr = ERR119;
                                break 'sw;
                            }
                            ptr = p.add(1);
                            escape = -(s + 1);
                            break 'sw;
                        }

                        if *ptr as u32 == CHAR_LESS_THAN_SIGN || *ptr as u32 == CHAR_APOSTROPHE {
                            escape = ESC_g;
                            break 'sw;
                        }

                        if *ptr as u32 == CHAR_LEFT_CURLY_BRACKET {
                            let mut p = ptr.add(1);
                            while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
                                p = p.add(1);
                            }
                            s = 0;
                            if read_number(&mut p, ptrend, bracount as i32, MAX_GROUP_NUMBER, ERR61 as u32, &mut s, errorcodeptr) == FALSE {
                                if *errorcodeptr == 0 {
                                    escape = ESC_k;
                                }
                                break 'sw;
                            }
                            while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
                                p = p.add(1);
                            }
                            if p >= ptrend || *p as u32 != CHAR_RIGHT_CURLY_BRACKET {
                                ptr = p;
                                *errorcodeptr = ERR119;
                                break 'sw;
                            }
                            ptr = p.add(1);
                        } else {
                            s = 0;
                            if read_number(&mut ptr, ptrend, bracount as i32, MAX_GROUP_NUMBER, ERR61 as u32, &mut s, errorcodeptr) == FALSE {
                                if *errorcodeptr == 0 {
                                    *errorcodeptr = ERR57;
                                }
                                break 'sw;
                            }
                        }

                        if s <= 0 {
                            *errorcodeptr = ERR15;
                            break 'sw;
                        }
                        escape = -(s + 1);
                    }

                    x if (x >= CHAR_1 && x <= CHAR_9) => {
                        // Digits 1-9
                        let mut do_octal = false;
                        if isclass != 0 {
                            do_octal = true;
                        } else if (xoptions & PCRE2_EXTRA_PYTHON_OCTAL) != 0 {
                            if *ptr.sub(1) as u32 <= CHAR_7
                                && ptr.add(1) < ptrend
                                && *ptr as u32 >= CHAR_0
                                && *ptr as u32 <= CHAR_7
                                && *ptr.add(1) as u32 >= CHAR_0
                                && *ptr.add(1) as u32 <= CHAR_7
                            {
                                do_octal = true;
                            } else {
                                ptr = ptr.sub(1);
                                s = 0;
                                if read_number(&mut ptr, ptrend, -1, MAX_GROUP_NUMBER, 0, &mut s, errorcodeptr) == FALSE {
                                    *errorcodeptr = ERR61;
                                    break 'sw;
                                }
                                escape = -(s + 1);
                                break 'sw;
                            }
                        } else {
                            oldptr = ptr;
                            ptr = ptr.sub(1);
                            s = 0;
                            if read_number(&mut ptr, ptrend, -1, MAX_GROUP_NUMBER, 0, &mut s, errorcodeptr) == FALSE {
                                s = INT_MAX;
                            }
                            if s < 10 || c >= CHAR_8 || (s as u32) <= bracount {
                                if (s as u32) > MAX_GROUP_NUMBER {
                                    *errorcodeptr = ERR61;
                                } else {
                                    escape = -(s + 1);
                                }
                                break 'sw;
                            }
                            ptr = oldptr;
                            do_octal = true;
                        }

                        // Handle a digit following \ when not a back reference.
                        if !do_octal {
                            break 'sw;
                        }
                        if c >= CHAR_8 {
                            break 'sw;
                        }

                        // Fall through to octal (CHAR_0 code) with c as first digit.
                        c -= CHAR_0;
                        while {
                            let cont = i < 2 && ptr < ptrend && *ptr as u32 >= CHAR_0 && *ptr as u32 <= CHAR_7;
                            i += 1;
                            cont
                        } {
                            c = c * 8 + *ptr as u32 - CHAR_0;
                            ptr = ptr.add(1);
                        }
                        if c > 0xff {
                            if (xoptions & PCRE2_EXTRA_PYTHON_OCTAL) != 0 {
                                *errorcodeptr = ERR102;
                            } else if !utf {
                                *errorcodeptr = ERR51;
                            }
                        }
                        if (xoptions & PCRE2_EXTRA_NO_BS0) != 0 && c == 0 && i == 1 {
                            *errorcodeptr = ERR98;
                        }
                    }

                    x if x == CHAR_0 => {
                        c -= CHAR_0;
                        while {
                            let cont = i < 2 && ptr < ptrend && *ptr as u32 >= CHAR_0 && *ptr as u32 <= CHAR_7;
                            i += 1;
                            cont
                        } {
                            c = c * 8 + *ptr as u32 - CHAR_0;
                            ptr = ptr.add(1);
                        }
                        if c > 0xff {
                            if (xoptions & PCRE2_EXTRA_PYTHON_OCTAL) != 0 {
                                *errorcodeptr = ERR102;
                            } else if !utf {
                                *errorcodeptr = ERR51;
                            }
                        }
                        if (xoptions & PCRE2_EXTRA_NO_BS0) != 0 && c == 0 && i == 1 {
                            *errorcodeptr = ERR98;
                        }
                    }

                    x if x == CHAR_o => {
                        if ptr >= ptrend || *ptr as u32 != CHAR_LEFT_CURLY_BRACKET {
                            *errorcodeptr = ERR55;
                            break 'sw;
                        }
                        ptr = ptr.add(1);
                        while ptr < ptrend && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT) {
                            ptr = ptr.add(1);
                        }
                        if ptr >= ptrend || *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
                            *errorcodeptr = ERR78;
                            break 'sw;
                        }
                        c = 0;
                        overflow = false;
                        while ptr < ptrend && *ptr as u32 >= CHAR_0 && *ptr as u32 <= CHAR_7 {
                            cc = *ptr as u32;
                            ptr = ptr.add(1);
                            if c == 0 && cc == CHAR_0 {
                                continue;
                            }
                            c = (c << 3) + (cc - CHAR_0);
                            if c > (if utf { 0x10ffff } else { 0xff }) {
                                overflow = true;
                                break;
                            }
                        }
                        while ptr < ptrend && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT) {
                            ptr = ptr.add(1);
                        }
                        if overflow {
                            while ptr < ptrend && *ptr as u32 >= CHAR_0 && *ptr as u32 <= CHAR_7 {
                                ptr = ptr.add(1);
                            }
                            *errorcodeptr = ERR34;
                        } else if utf && c >= 0xd800 && c <= 0xdfff
                            && (xoptions & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES) == 0
                        {
                            *errorcodeptr = ERR73;
                        } else if ptr < ptrend && *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
                            ptr = ptr.add(1);
                        } else {
                            *errorcodeptr = ERR64;
                            failed_forward = true;
                            break 'exit;
                        }
                    }

                    x if x == CHAR_x => {
                        if alt_bsux {
                            let mut xc: u32;
                            if (ptrend as usize - ptr as usize) < 2 {
                                break 'sw;
                            }
                            cc = xdigit(*ptr as u32);
                            if cc == 0xff {
                                break 'sw;
                            }
                            xc = xdigit(*ptr.add(1) as u32);
                            if xc == 0xff {
                                break 'sw;
                            }
                            c = (cc << 4) | xc;
                            ptr = ptr.add(2);
                        } else {
                            if ptr < ptrend && *ptr as u32 == CHAR_LEFT_CURLY_BRACKET {
                                ptr = ptr.add(1);
                                while ptr < ptrend
                                    && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT)
                                {
                                    ptr = ptr.add(1);
                                }
                                // COME_FROM_NU
                                match scan_hex_braces(&mut ptr, ptrend, utf, xoptions, &mut c, errorcodeptr) {
                                    HexOut::Done => {}
                                    HexOut::FailedForward => {
                                        failed_forward = true;
                                        break 'exit;
                                    }
                                }
                            } else {
                                if ptr >= ptrend || {
                                    cc = xdigit(*ptr as u32);
                                    cc == 0xff
                                } {
                                    *errorcodeptr = ERR78;
                                    break 'sw;
                                }
                                ptr = ptr.add(1);
                                c = cc;
                                if ptr >= ptrend || {
                                    cc = xdigit(*ptr as u32);
                                    cc == 0xff
                                } {
                                    break 'sw;
                                }
                                ptr = ptr.add(1);
                                c = (c << 4) | cc;
                            }
                        }
                    }

                    x if x == CHAR_c => {
                        if ptr >= ptrend {
                            *errorcodeptr = ERR2;
                            break 'sw;
                        }
                        c = *ptr as u32;
                        if c >= CHAR_a && c <= CHAR_z {
                            c -= 32;
                        }
                        if c < 32 || c > 126 {
                            *errorcodeptr = ERR68;
                            failed_forward = true;
                            break 'exit;
                        }
                        c ^= 0x40;
                        ptr = ptr.add(1);
                    }

                    _ => {
                        *errorcodeptr = ERR3;
                    }
                }
            } // 'sw
        }
    } // 'exit

    if failed_forward {
        ptr = ptr.add(1);
        if utf {
            forwardchartest(&mut ptr, ptrend);
        }
    }

    *ptrptr = ptr;
    *chptr = c;
    escape
}

// ---------------------------------------------------------------------------
// Handle \P and \p : get_ucp
// ---------------------------------------------------------------------------

unsafe fn get_ucp(
    ptrptr: *mut PCRE2_SPTR,
    utf: bool,
    negptr: *mut BOOL,
    ptypeptr: *mut u16,
    pdataptr: *mut u16,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> BOOL {
    let mut c: u32;
    let mut i: isize;
    let mut bot: PCRE2_SIZE;
    let mut top: PCRE2_SIZE;
    let mut ptr = *ptrptr;
    let mut name: [PCRE2_UCHAR; 50] = [0; 50];
    let mut vptr: *mut PCRE2_UCHAR = ptr::null_mut();
    let mut ptscript: u16 = PT_NOTSCRIPT as u16;

    if ptr >= (*cb).end_pattern {
        *errorcodeptr = ERR46;
        *ptrptr = ptr;
        return FALSE;
    }
    c = getcharinctest(&mut ptr, utf);
    *negptr = FALSE;

    if c == CHAR_LEFT_CURLY_BRACKET {
        if ptr >= (*cb).end_pattern {
            *errorcodeptr = ERR46;
            *ptrptr = ptr;
            return FALSE;
        }

        i = 0;
        let mut ended_ok = false;
        while i < (50 - 1) as isize {
            // REDO loop
            loop {
                if ptr >= (*cb).end_pattern {
                    *errorcodeptr = ERR46;
                    *ptrptr = ptr;
                    return FALSE;
                }
                c = getcharinctest(&mut ptr, utf);

                if c == CHAR_UNDERSCORE
                    || c == CHAR_MINUS
                    || c == CHAR_SPACE
                    || (c >= CHAR_HT && c <= CHAR_CR)
                {
                    continue; // REDO
                }
                if i == 0 && *negptr == FALSE && c == CHAR_CIRCUMFLEX_ACCENT {
                    *negptr = TRUE;
                    continue; // REDO
                }
                break;
            }

            if c == CHAR_RIGHT_CURLY_BRACKET {
                ended_ok = true;
                break;
            }

            if c < CHAR_AMPERSAND || c > CHAR_z {
                *errorcodeptr = ERR46;
                *ptrptr = ptr;
                return FALSE;
            }

            if c >= CHAR_A && c <= CHAR_Z {
                c |= 0x20;
            } else if (c == CHAR_COLON || c == CHAR_EQUALS_SIGN) && vptr.is_null() {
                vptr = name.as_mut_ptr().add(i as usize);
            }

            name[i as usize] = c as u8;
            i += 1;
        }

        if !ended_ok && c != CHAR_RIGHT_CURLY_BRACKET {
            *errorcodeptr = ERR46;
            *ptrptr = ptr;
            return FALSE;
        }
        name[i as usize] = 0;
    } else if c >= CHAR_A && c <= CHAR_Z {
        name[0] = (c | 0x20) as u8;
        name[1] = 0;
        i = 1;
    } else if c >= CHAR_a && c <= CHAR_z {
        name[0] = c as u8;
        name[1] = 0;
        i = 1;
    } else {
        *errorcodeptr = ERR46;
        *ptrptr = ptr;
        return FALSE;
    }

    *ptrptr = ptr;

    if !vptr.is_null() {
        let mut offset = 0usize;
        let mut sname: [PCRE2_UCHAR; 8] = [0; 8];

        *vptr = 0;
        if _pcre2_strcmp_c8(name.as_ptr(), b"bidiclass\0".as_ptr() as *const c_char) == 0
            || _pcre2_strcmp_c8(name.as_ptr(), b"bc\0".as_ptr() as *const c_char) == 0
        {
            offset = 4;
            sname[0] = CHAR_b as u8;
            sname[1] = CHAR_i as u8;
            sname[2] = CHAR_d as u8;
            sname[3] = CHAR_i as u8;
        } else if _pcre2_strcmp_c8(name.as_ptr(), b"script\0".as_ptr() as *const c_char) == 0
            || _pcre2_strcmp_c8(name.as_ptr(), b"sc\0".as_ptr() as *const c_char) == 0
        {
            ptscript = PT_SC as u16;
        } else if _pcre2_strcmp_c8(name.as_ptr(), b"scriptextensions\0".as_ptr() as *const c_char)
            == 0
            || _pcre2_strcmp_c8(name.as_ptr(), b"scx\0".as_ptr() as *const c_char) == 0
        {
            ptscript = PT_SCX as u16;
        } else {
            *errorcodeptr = ERR47;
            return FALSE;
        }

        // memmove(name + offset, vptr + 1, (name + i - vptr))
        let vptr_idx = (vptr as usize - name.as_ptr() as usize);
        let count = (i as usize) - vptr_idx; // name + i - vptr
        memmove(
            name.as_mut_ptr().add(offset) as *mut c_void,
            vptr.add(1) as *const c_void,
            count,
        );
        if offset != 0 {
            memmove(
                name.as_mut_ptr() as *mut c_void,
                sname.as_ptr() as *const c_void,
                offset,
            );
        }
    }

    bot = 0;
    top = _pcre2_utt_size_8;

    while bot < top {
        i = ((bot + top) >> 1) as isize;
        let r = _pcre2_strcmp_c8(
            name.as_ptr(),
            _pcre2_utt_names_8.as_ptr().add(_pcre2_utt_8[i as usize].name_offset as usize)
                as *const c_char,
        );

        if r == 0 {
            *pdataptr = _pcre2_utt_8[i as usize].value;
            if vptr.is_null() || ptscript == PT_NOTSCRIPT as u16 {
                *ptypeptr = _pcre2_utt_8[i as usize].type_;
                return TRUE;
            }

            match _pcre2_utt_8[i as usize].type_ as u32 {
                PT_SC => {
                    *ptypeptr = PT_SC as u16;
                    return TRUE;
                }
                PT_SCX => {
                    *ptypeptr = ptscript;
                    return TRUE;
                }
                _ => {}
            }
            break; // Non-script found
        }

        if r > 0 {
            bot = i as usize + 1;
        } else {
            top = i as usize;
        }
    }

    *errorcodeptr = ERR47;
    FALSE
}

// ---------------------------------------------------------------------------
// Check for POSIX class syntax
// ---------------------------------------------------------------------------

unsafe fn check_posix_syntax(
    mut ptr: PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    endptr: *mut PCRE2_SPTR,
) -> BOOL {
    let terminator = *ptr as u32;
    ptr = ptr.add(1);

    while (ptrend as usize - ptr as usize) >= 2 {
        if *ptr as u32 == CHAR_BACKSLASH
            && (*ptr.add(1) as u32 == CHAR_RIGHT_SQUARE_BRACKET
                || *ptr.add(1) as u32 == CHAR_BACKSLASH)
        {
            ptr = ptr.add(1);
        } else if (*ptr as u32 == CHAR_LEFT_SQUARE_BRACKET && *ptr.add(1) as u32 == terminator)
            || *ptr as u32 == CHAR_RIGHT_SQUARE_BRACKET
        {
            return FALSE;
        } else if *ptr as u32 == terminator && *ptr.add(1) as u32 == CHAR_RIGHT_SQUARE_BRACKET {
            *endptr = ptr;
            return TRUE;
        }
        ptr = ptr.add(1);
    }

    FALSE
}

// ---------------------------------------------------------------------------
// Check POSIX class name
// ---------------------------------------------------------------------------

unsafe fn check_posix_name(ptr: PCRE2_SPTR, len: c_int) -> c_int {
    let mut pn = POSIX_NAMES.as_ptr();
    let mut yield_: c_int = 0;
    while POSIX_NAME_LENGTHS[yield_ as usize] != 0 {
        if len == POSIX_NAME_LENGTHS[yield_ as usize] as c_int
            && _pcre2_strncmp_c8(ptr, pn as *const c_char, len as usize) == 0
        {
            return yield_;
        }
        pn = pn.add(POSIX_NAME_LENGTHS[yield_ as usize] as usize + 1);
        yield_ += 1;
    }
    -1
}

// ---------------------------------------------------------------------------
// Read a subpattern or VERB name
// ---------------------------------------------------------------------------

unsafe fn read_name(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    utf: bool,
    terminator: u32,
    offsetptr: *mut PCRE2_SIZE,
    nameptr: *mut PCRE2_SPTR,
    namelenptr: *mut u32,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> BOOL {
    let mut ptr = *ptrptr;
    let is_group = *ptr as u32 != CHAR_ASTERISK;
    ptr = ptr.add(1);
    let is_braced = terminator == CHAR_RIGHT_CURLY_BRACKET;

    if is_braced {
        while ptr < ptrend && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT) {
            ptr = ptr.add(1);
        }
    }

    if ptr >= ptrend {
        *errorcodeptr = if is_group { ERR62 } else { ERR60 };
        *ptrptr = ptr;
        return FALSE;
    }

    *nameptr = ptr;
    *offsetptr = (ptr as usize - (*cb).start_pattern as usize) as PCRE2_SIZE;

    if utf && is_group {
        let mut c: u32;
        let mut typ: u32;
        let mut p = ptr;

        c = getcharinc_utf(&mut p); // Peek at next char
        typ = UCD_CHARTYPE(c);

        if typ == ucp_Nd {
            ptr = p;
            *errorcodeptr = ERR44;
            *ptrptr = ptr;
            return FALSE;
        }

        loop {
            if typ != ucp_Nd && _pcre2_ucp_gentype_8[typ as usize] != ucp_L && c != CHAR_UNDERSCORE
            {
                break;
            }
            ptr = p;
            if p >= ptrend {
                break;
            }
            c = getcharinc_utf(&mut p);
            typ = UCD_CHARTYPE(c);
        }
    } else {
        if is_group && IS_DIGIT(*ptr as u32) {
            ptr = ptr.add(1);
            *errorcodeptr = ERR44;
            *ptrptr = ptr;
            return FALSE;
        }

        while ptr < ptrend && ((*(*cb).ctypes.add(*ptr as usize) & ctype_word) != 0) {
            ptr = ptr.add(1);
        }
    }

    if (ptr as usize - *nameptr as usize) as u32 > MAX_NAME_SIZE {
        *errorcodeptr = ERR48;
        *ptrptr = ptr;
        return FALSE;
    }
    *namelenptr = (ptr as usize - *nameptr as usize) as u32;

    if is_group {
        if ptr == *nameptr {
            *errorcodeptr = ERR62;
            *ptrptr = ptr;
            return FALSE;
        }
        if is_braced {
            while ptr < ptrend && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT) {
                ptr = ptr.add(1);
            }
        }
        if terminator != 0 {
            if ptr >= ptrend || *ptr as u32 != terminator {
                *errorcodeptr = ERR42;
                *ptrptr = ptr;
                return FALSE;
            }
            ptr = ptr.add(1);
        }
    }

    *ptrptr = ptr;
    TRUE
}

// ---------------------------------------------------------------------------
// Parse capturing bracket argument list
// ---------------------------------------------------------------------------

unsafe fn parse_capture_list(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    utf: bool,
    mut parsed_pattern: *mut u32,
    mut offset: PCRE2_SIZE,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> *mut u32 {
    let mut next_offset: PCRE2_SIZE;
    let mut ptr = *ptrptr;
    let mut name: PCRE2_SPTR = ptr::null();
    let mut terminator: u32;
    let mut meta: u32;
    let mut namelen: u32 = 0;
    let mut i: c_int = 0;

    if ptr >= ptrend || *ptr as u32 != CHAR_LEFT_PARENTHESIS {
        *errorcodeptr = ERR118;
        *ptrptr = ptr;
        return ptr::null_mut();
    }

    loop {
        ptr = ptr.add(1);
        next_offset = (ptr as usize - (*cb).start_pattern as usize) as PCRE2_SIZE;

        if ptr >= ptrend {
            *errorcodeptr = ERR117;
            *ptrptr = ptr;
            return ptr::null_mut();
        }

        if read_number(&mut ptr, ptrend, (*cb).bracount as i32, MAX_GROUP_NUMBER, ERR61 as u32, &mut i, errorcodeptr) != FALSE
        {
            if i <= 0 {
                *errorcodeptr = ERR15;
                *ptrptr = ptr;
                return ptr::null_mut();
            }
            meta = META_CAPTURE_NUMBER;
            namelen = i as u32;
        } else if *errorcodeptr != 0 {
            *ptrptr = ptr;
            return ptr::null_mut();
        } else {
            if *ptr as u32 == CHAR_LESS_THAN_SIGN {
                terminator = CHAR_GREATER_THAN_SIGN;
            } else if *ptr as u32 == CHAR_APOSTROPHE {
                terminator = CHAR_APOSTROPHE;
            } else {
                *errorcodeptr = ERR117;
                *ptrptr = ptr;
                return ptr::null_mut();
            }

            if read_name(&mut ptr, ptrend, utf, terminator, &mut next_offset, &mut name, &mut namelen, errorcodeptr, cb) == FALSE {
                *ptrptr = ptr;
                return ptr::null_mut();
            }

            meta = META_CAPTURE_NAME;
        }

        if offset == 0 || (next_offset - offset) >= 0x10000 {
            wr!(parsed_pattern, META_OFFSET);
            PUTOFFSET!(next_offset, parsed_pattern);
            offset = next_offset;
        }

        wr!(parsed_pattern, meta | (next_offset - offset) as u32);
        wr!(parsed_pattern, namelen);
        offset = next_offset;

        if ptr >= ptrend {
            *errorcodeptr = ERR14;
            *ptrptr = ptr;
            return ptr::null_mut();
        }

        if *ptr as u32 == CHAR_RIGHT_PARENTHESIS {
            break;
        }

        if *ptr as u32 != CHAR_COMMA {
            *errorcodeptr = ERR24;
            *ptrptr = ptr;
            return ptr::null_mut();
        }
    }

    *ptrptr = ptr.add(1);
    parsed_pattern
}

// ---------------------------------------------------------------------------
// Manage callouts at start of cycle
// ---------------------------------------------------------------------------

unsafe fn manage_callouts(
    ptr: PCRE2_SPTR,
    pcalloutptr: *mut *mut u32,
    auto_callout: bool,
    mut parsed_pattern: *mut u32,
    cb: *mut compile_block,
) -> *mut u32 {
    let mut previous_callout = *pcalloutptr;

    if !previous_callout.is_null() {
        *previous_callout.add(2) = (ptr as usize
            - (*cb).start_pattern as usize
            - *previous_callout.add(1) as usize) as u32;
    }

    if !auto_callout {
        previous_callout = ptr::null_mut();
    } else {
        if previous_callout.is_null()
            || previous_callout != parsed_pattern.sub(4)
            || *previous_callout.add(3) != 255
        {
            previous_callout = parsed_pattern;
            parsed_pattern = parsed_pattern.add(4);
            *previous_callout.add(0) = META_CALLOUT_NUMBER;
            *previous_callout.add(2) = 0;
            *previous_callout.add(3) = 255;
        }
        *previous_callout.add(1) = (ptr as usize - (*cb).start_pattern as usize) as u32;
    }

    *pcalloutptr = previous_callout;
    parsed_pattern
}

// ---------------------------------------------------------------------------
// Handle \d, \D, \s, \S, \w, \W : handle_escdsw
// ---------------------------------------------------------------------------

unsafe fn handle_escdsw(
    escape: c_int,
    mut parsed_pattern: *mut u32,
    options: u32,
    xoptions: u32,
) -> *mut u32 {
    let mut ascii_option: u32 = 0;
    let mut prop: c_int = ESC_p;

    match escape {
        x if x == ESC_D => {
            prop = ESC_P;
            ascii_option = PCRE2_EXTRA_ASCII_BSD;
        }
        x if x == ESC_d => {
            ascii_option = PCRE2_EXTRA_ASCII_BSD;
        }
        x if x == ESC_S => {
            prop = ESC_P;
            ascii_option = PCRE2_EXTRA_ASCII_BSS;
        }
        x if x == ESC_s => {
            ascii_option = PCRE2_EXTRA_ASCII_BSS;
        }
        x if x == ESC_W => {
            prop = ESC_P;
            ascii_option = PCRE2_EXTRA_ASCII_BSW;
        }
        x if x == ESC_w => {
            ascii_option = PCRE2_EXTRA_ASCII_BSW;
        }
        _ => {}
    }

    if (options & PCRE2_UCP) == 0 || (xoptions & ascii_option) != 0 {
        wr!(parsed_pattern, META_ESCAPE + escape as u32);
    } else {
        wr!(parsed_pattern, META_ESCAPE + prop as u32);
        match escape {
            x if x == ESC_d || x == ESC_D => {
                wr!(parsed_pattern, (PT_PC << 16) | ucp_Nd);
            }
            x if x == ESC_s || x == ESC_S => {
                wr!(parsed_pattern, PT_SPACE << 16);
            }
            x if x == ESC_w || x == ESC_W => {
                wr!(parsed_pattern, PT_WORD << 16);
            }
            _ => {}
        }
    }

    parsed_pattern
}

// ---------------------------------------------------------------------------
// Maximum size of parsed_pattern for given input
// ---------------------------------------------------------------------------

unsafe fn max_parsed_pattern(
    ptr: PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    _utf: bool,
    options: u32,
) -> isize {
    let big32count: PCRE2_SIZE = 0; // 8-bit: never
    let mut parsed_size_needed: isize;

    parsed_size_needed = (ptrend as usize - ptr as usize) as isize + big32count as isize;

    if (options & PCRE2_AUTO_CALLOUT) != 0 {
        parsed_size_needed += (ptrend as usize - ptr as usize) as isize * 4;
    }

    parsed_size_needed
}

// ---------------------------------------------------------------------------
// Find first significant opcode
// ---------------------------------------------------------------------------

unsafe fn first_significant_code(mut code: PCRE2_SPTR, skipassert: BOOL) -> PCRE2_SPTR {
    loop {
        match *code {
            OP_ASSERT_NOT | OP_ASSERTBACK | OP_ASSERTBACK_NOT | OP_ASSERTBACK_NA => {
                if skipassert == FALSE {
                    return code;
                }
                loop {
                    code = code.add(GET(code, 1) as usize);
                    if *code != OP_ALT {
                        break;
                    }
                }
                code = code.add(oplen(*code));
            }

            OP_WORD_BOUNDARY | OP_NOT_WORD_BOUNDARY | OP_UCP_WORD_BOUNDARY
            | OP_NOT_UCP_WORD_BOUNDARY => {
                if skipassert == FALSE {
                    return code;
                }
                code = code.add(oplen(*code));
            }

            OP_CALLOUT | OP_CREF | OP_DNCREF | OP_RREF | OP_DNRREF | OP_FALSE | OP_TRUE => {
                code = code.add(oplen(*code));
            }

            OP_CALLOUT_STR => {
                code = code.add(GET(code, 1 + 2 * LINK_SIZE) as usize);
            }

            OP_SKIPZERO => {
                code = code.add(2 + GET(code, 2) as usize + LINK_SIZE);
            }

            OP_COND | OP_SCOND => {
                if *code.add(1 + LINK_SIZE) != OP_FALSE
                    || *code.add(GET(code, 1) as usize) != OP_KET
                {
                    return code;
                }
                code = code.add(GET(code, 1) as usize + 1 + LINK_SIZE);
            }

            OP_MARK | OP_COMMIT_ARG | OP_PRUNE_ARG | OP_SKIP_ARG | OP_THEN_ARG => {
                code = code.add(*code.add(1) as usize + oplen(*code));
            }

            _ => return code,
        }
    }
}

// ---------------------------------------------------------------------------
// Scan compiled regex for recursion reference : find_recurse
// ---------------------------------------------------------------------------

unsafe fn find_recurse(mut code: *mut PCRE2_UCHAR, utf: bool) -> *mut PCRE2_UCHAR {
    loop {
        let c = *code;
        if c == OP_END {
            return ptr::null_mut();
        }
        if c == OP_RECURSE {
            return code;
        }

        if c == OP_XCLASS || c == OP_ECLASS {
            code = code.add(GET(code, 1) as usize);
        } else if c == OP_CALLOUT_STR {
            code = code.add(GET(code, 1 + 2 * LINK_SIZE) as usize);
        } else {
            match c {
                OP_TYPESTAR | OP_TYPEMINSTAR | OP_TYPEPLUS | OP_TYPEMINPLUS | OP_TYPEQUERY
                | OP_TYPEMINQUERY | OP_TYPEPOSSTAR | OP_TYPEPOSPLUS | OP_TYPEPOSQUERY => {
                    if *code.add(1) == OP_PROP || *code.add(1) == OP_NOTPROP {
                        code = code.add(2);
                    }
                }
                OP_TYPEPOSUPTO | OP_TYPEUPTO | OP_TYPEMINUPTO | OP_TYPEEXACT => {
                    if *code.add(1 + IMM2_SIZE) == OP_PROP || *code.add(1 + IMM2_SIZE) == OP_NOTPROP
                    {
                        code = code.add(2);
                    }
                }
                OP_MARK | OP_COMMIT_ARG | OP_PRUNE_ARG | OP_SKIP_ARG | OP_THEN_ARG => {
                    code = code.add(*code.add(1) as usize);
                }
                _ => {}
            }

            code = code.add(oplen(c));

            if utf {
                match c {
                    OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI | OP_EXACT | OP_EXACTI | OP_NOTEXACT
                    | OP_NOTEXACTI | OP_UPTO | OP_UPTOI | OP_NOTUPTO | OP_NOTUPTOI | OP_MINUPTO
                    | OP_MINUPTOI | OP_NOTMINUPTO | OP_NOTMINUPTOI | OP_POSUPTO | OP_POSUPTOI
                    | OP_NOTPOSUPTO | OP_NOTPOSUPTOI | OP_STAR | OP_STARI | OP_NOTSTAR
                    | OP_NOTSTARI | OP_MINSTAR | OP_MINSTARI | OP_NOTMINSTAR | OP_NOTMINSTARI
                    | OP_POSSTAR | OP_POSSTARI | OP_NOTPOSSTAR | OP_NOTPOSSTARI | OP_PLUS
                    | OP_PLUSI | OP_NOTPLUS | OP_NOTPLUSI | OP_MINPLUS | OP_MINPLUSI
                    | OP_NOTMINPLUS | OP_NOTMINPLUSI | OP_POSPLUS | OP_POSPLUSI | OP_NOTPOSPLUS
                    | OP_NOTPOSPLUSI | OP_QUERY | OP_QUERYI | OP_NOTQUERY | OP_NOTQUERYI
                    | OP_MINQUERY | OP_MINQUERYI | OP_NOTMINQUERY | OP_NOTMINQUERYI | OP_POSQUERY
                    | OP_POSQUERYI | OP_NOTPOSQUERY | OP_NOTPOSQUERYI => {
                        if HAS_EXTRALEN(*code.sub(1) as u32) {
                            code = code.add(GET_EXTRALEN(*code.sub(1) as u32) as usize);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Check for anchored pattern : is_anchored
// ---------------------------------------------------------------------------

unsafe fn is_anchored(
    mut code: PCRE2_SPTR,
    bracket_map: u32,
    cb: *mut compile_block,
    atomcount: c_int,
    inassert: BOOL,
    dotstar_anchor: BOOL,
) -> BOOL {
    loop {
        let scode = first_significant_code(code.add(oplen(*code)), FALSE);
        let op = *scode;

        if op == OP_BRA || op == OP_BRAPOS || op == OP_SBRA || op == OP_SBRAPOS {
            if is_anchored(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        } else if op == OP_CBRA || op == OP_CBRAPOS || op == OP_SCBRA || op == OP_SCBRAPOS {
            let n = GET2(scode, 1 + LINK_SIZE);
            let new_map = bracket_map | (if n < 32 { 1u32 << n } else { 1 });
            if is_anchored(scode, new_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        } else if op == OP_ASSERT || op == OP_ASSERT_NA {
            if is_anchored(scode, bracket_map, cb, atomcount, TRUE, dotstar_anchor) == FALSE {
                return FALSE;
            }
        } else if op == OP_COND || op == OP_SCOND {
            if *scode.add(GET(scode, 1) as usize) != OP_ALT {
                return FALSE;
            }
            if is_anchored(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        } else if op == OP_ONCE {
            if is_anchored(scode, bracket_map, cb, atomcount + 1, inassert, dotstar_anchor) == FALSE
            {
                return FALSE;
            }
        } else if op == OP_TYPESTAR || op == OP_TYPEMINSTAR || op == OP_TYPEPOSSTAR {
            if *scode.add(1) != OP_ALLANY
                || (bracket_map & (*cb).backref_map) != 0
                || atomcount > 0
                || (*cb).had_pruneorskip != 0
                || inassert != FALSE
                || dotstar_anchor == FALSE
            {
                return FALSE;
            }
        } else if op != OP_SOD && op != OP_SOM && op != OP_CIRC {
            return FALSE;
        }

        code = code.add(GET(code, 1) as usize);
        if *code != OP_ALT {
            break;
        }
    }
    TRUE
}

// ---------------------------------------------------------------------------
// Check for starting with ^ or .* : is_startline
// ---------------------------------------------------------------------------

unsafe fn is_startline(
    mut code: PCRE2_SPTR,
    bracket_map: u32,
    cb: *mut compile_block,
    atomcount: c_int,
    inassert: BOOL,
    dotstar_anchor: BOOL,
) -> BOOL {
    loop {
        let mut scode = first_significant_code(code.add(oplen(*code)), FALSE);
        let mut op = *scode;

        if op == OP_COND {
            scode = scode.add(1 + LINK_SIZE);

            if *scode == OP_CALLOUT {
                scode = scode.add(oplen(OP_CALLOUT));
            } else if *scode == OP_CALLOUT_STR {
                scode = scode.add(GET(scode, 1 + 2 * LINK_SIZE) as usize);
            }

            match *scode {
                OP_CREF | OP_DNCREF | OP_RREF | OP_DNRREF | OP_FAIL | OP_FALSE | OP_TRUE => {
                    return FALSE;
                }
                _ => {
                    if is_startline(scode, bracket_map, cb, atomcount, TRUE, dotstar_anchor)
                        == FALSE
                    {
                        return FALSE;
                    }
                    loop {
                        scode = scode.add(GET(scode, 1) as usize);
                        if *scode != OP_ALT {
                            break;
                        }
                    }
                    scode = scode.add(1 + LINK_SIZE);
                }
            }
            scode = first_significant_code(scode, FALSE);
            op = *scode;
        }

        if op == OP_BRA || op == OP_BRAPOS || op == OP_SBRA || op == OP_SBRAPOS {
            if is_startline(scode, bracket_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        } else if op == OP_CBRA || op == OP_CBRAPOS || op == OP_SCBRA || op == OP_SCBRAPOS {
            let n = GET2(scode, 1 + LINK_SIZE);
            let new_map = bracket_map | (if n < 32 { 1u32 << n } else { 1 });
            if is_startline(scode, new_map, cb, atomcount, inassert, dotstar_anchor) == FALSE {
                return FALSE;
            }
        } else if op == OP_ASSERT || op == OP_ASSERT_NA {
            if is_startline(scode, bracket_map, cb, atomcount, TRUE, dotstar_anchor) == FALSE {
                return FALSE;
            }
        } else if op == OP_ONCE {
            if is_startline(scode, bracket_map, cb, atomcount + 1, inassert, dotstar_anchor)
                == FALSE
            {
                return FALSE;
            }
        } else if op == OP_TYPESTAR || op == OP_TYPEMINSTAR || op == OP_TYPEPOSSTAR {
            if *scode.add(1) != OP_ANY
                || (bracket_map & (*cb).backref_map) != 0
                || atomcount > 0
                || (*cb).had_pruneorskip != 0
                || inassert != FALSE
                || dotstar_anchor == FALSE
            {
                return FALSE;
            }
        } else if op != OP_CIRC && op != OP_CIRCM {
            return FALSE;
        }

        code = code.add(GET(code, 1) as usize);
        if *code != OP_ALT {
            break;
        }
    }
    TRUE
}

// ---------------------------------------------------------------------------
// Check for asserted fixed first code unit : find_firstassertedcu
// ---------------------------------------------------------------------------

unsafe fn find_firstassertedcu(
    mut code: PCRE2_SPTR,
    flags: *mut u32,
    inassert: u32,
) -> u32 {
    let mut c: u32 = 0;
    let mut cflags: u32 = REQ_NONE;

    *flags = REQ_NONE;
    loop {
        let mut d: u32;
        let mut dflags: u32 = 0;
        let xl = if *code == OP_CBRA || *code == OP_SCBRA || *code == OP_CBRAPOS
            || *code == OP_SCBRAPOS
        {
            IMM2_SIZE
        } else {
            0
        };
        let mut scode = first_significant_code(code.add(1 + LINK_SIZE + xl), TRUE);
        let op = *scode;

        match op {
            OP_BRA | OP_BRAPOS | OP_CBRA | OP_SCBRA | OP_CBRAPOS | OP_SCBRAPOS | OP_ASSERT
            | OP_ASSERT_NA | OP_ONCE | OP_SCRIPT_RUN => {
                d = find_firstassertedcu(
                    scode,
                    &mut dflags,
                    inassert + if op == OP_ASSERT || op == OP_ASSERT_NA { 1 } else { 0 },
                );
                if dflags >= REQ_NONE {
                    return 0;
                }
                if cflags >= REQ_NONE {
                    c = d;
                    cflags = dflags;
                } else if c != d || cflags != dflags {
                    return 0;
                }
            }

            OP_EXACT => {
                scode = scode.add(IMM2_SIZE);
                // fall through
                if inassert == 0 {
                    return 0;
                }
                if cflags >= REQ_NONE {
                    c = *scode.add(1) as u32;
                    cflags = 0;
                } else if c != *scode.add(1) as u32 {
                    return 0;
                }
            }

            OP_CHAR | OP_PLUS | OP_MINPLUS | OP_POSPLUS => {
                if inassert == 0 {
                    return 0;
                }
                if cflags >= REQ_NONE {
                    c = *scode.add(1) as u32;
                    cflags = 0;
                } else if c != *scode.add(1) as u32 {
                    return 0;
                }
            }

            OP_EXACTI => {
                scode = scode.add(IMM2_SIZE);
                if inassert == 0 {
                    return 0;
                }
                if *scode.add(1) as u32 >= 0x80 {
                    return 0;
                }
                if cflags >= REQ_NONE {
                    c = *scode.add(1) as u32;
                    cflags = REQ_CASELESS;
                } else if c != *scode.add(1) as u32 {
                    return 0;
                }
            }

            OP_CHARI | OP_PLUSI | OP_MINPLUSI | OP_POSPLUSI => {
                if inassert == 0 {
                    return 0;
                }
                if *scode.add(1) as u32 >= 0x80 {
                    return 0;
                }
                if cflags >= REQ_NONE {
                    c = *scode.add(1) as u32;
                    cflags = REQ_CASELESS;
                } else if c != *scode.add(1) as u32 {
                    return 0;
                }
            }

            _ => return 0,
        }

        code = code.add(GET(code, 1) as usize);
        if *code != OP_ALT {
            break;
        }
    }

    *flags = cflags;
    c
}

// ---------------------------------------------------------------------------
// Skip in parsed pattern : parsed_skip
// ---------------------------------------------------------------------------

unsafe fn parsed_skip(mut pptr: *mut u32, skiptype: u32) -> *mut u32 {
    let mut nestlevel: u32 = 0;

    loop {
        let mut meta = META_CODE(*pptr);

        let mut do_extra = true;
        match meta {
            META_END => return ptr::null_mut(),

            META_BACKREF => {
                if META_DATA(*pptr) >= 10 {
                    pptr = pptr.add(SIZEOFFSET);
                }
            }

            META_ESCAPE => {
                if *pptr - META_ESCAPE == ESC_P as u32 || *pptr - META_ESCAPE == ESC_p as u32 {
                    pptr = pptr.add(1);
                }
            }

            META_MARK | META_COMMIT_ARG | META_PRUNE_ARG | META_SKIP_ARG | META_THEN_ARG => {
                pptr = pptr.add(*pptr.add(1) as usize);
            }

            META_CLASS_END => {
                if skiptype == PSKIP_CLASS {
                    return pptr;
                }
            }

            META_ATOMIC | META_CAPTURE | META_COND_ASSERT | META_COND_DEFINE | META_COND_NAME
            | META_COND_NUMBER | META_COND_RNAME | META_COND_RNUMBER | META_COND_VERSION
            | META_SCS | META_LOOKAHEAD | META_LOOKAHEADNOT | META_LOOKAHEAD_NA
            | META_LOOKBEHIND | META_LOOKBEHINDNOT | META_LOOKBEHIND_NA | META_NOCAPTURE
            | META_SCRIPT_RUN => {
                nestlevel += 1;
            }

            META_ALT => {
                if nestlevel == 0 && skiptype == PSKIP_ALT {
                    return pptr;
                }
            }

            META_KET => {
                if nestlevel == 0 {
                    return pptr;
                }
                nestlevel -= 1;
            }

            _ => {
                if meta < META_END {
                    // Literal
                    pptr = pptr.add(1);
                    do_extra = false;
                }
            }
        }

        if do_extra {
            meta = (meta >> 16) & 0x7fff;
            if meta as usize >= META_EXTRA_LENGTHS.len() {
                return ptr::null_mut();
            }
            pptr = pptr.add(META_EXTRA_LENGTHS[meta as usize] as usize);
            pptr = pptr.add(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Find length of a parsed group : get_grouplength
// ---------------------------------------------------------------------------

unsafe fn get_grouplength(
    pptrptr: *mut *mut u32,
    minptr: *mut c_int,
    isinline: BOOL,
    errcodeptr: *mut c_int,
    lcptr: *mut c_int,
    group: c_int,
    recurses: *mut parsed_recurse_check,
    cb: *mut compile_block,
) -> c_int {
    let gi = (*cb).groupinfo.add(2 * group as usize);
    let mut branchlength: c_int;
    let mut branchminlength: c_int = 0;
    let mut grouplength: c_int = -1;
    let mut groupminlength: c_int = INT_MAX;

    if group > 0 && ((*cb).external_flags & PCRE2_DUPCAPUSED) == 0 {
        let groupinfo = *gi.add(0);
        if (groupinfo & GI_NOT_FIXED_LENGTH) != 0 {
            return -1;
        }
        if (groupinfo & GI_SET_FIXED_LENGTH) != 0 {
            if isinline != FALSE {
                *pptrptr = parsed_skip(*pptrptr, PSKIP_KET);
            }
            *minptr = *gi.add(1) as c_int;
            return (groupinfo & GI_FIXED_LENGTH_MASK) as c_int;
        }
    }

    loop {
        branchlength = get_branchlength(pptrptr, &mut branchminlength, errcodeptr, lcptr, recurses, cb);
        if branchlength < 0 {
            // ISNOTFIXED
            if group > 0 {
                *gi.add(0) |= GI_NOT_FIXED_LENGTH;
            }
            return -1;
        }
        if branchlength > grouplength {
            grouplength = branchlength;
        }
        if branchminlength < groupminlength {
            groupminlength = branchminlength;
        }
        if **pptrptr == META_KET {
            break;
        }
        *pptrptr = (*pptrptr).add(1); // Skip META_ALT
    }

    if group > 0 {
        *gi.add(0) |= (GI_SET_FIXED_LENGTH | grouplength as u32);
        *gi.add(1) = groupminlength as u32;
    }

    *minptr = groupminlength;
    grouplength
}

// ---------------------------------------------------------------------------
// Find length of a parsed branch : get_branchlength
// ---------------------------------------------------------------------------

unsafe fn get_branchlength(
    pptrptr: *mut *mut u32,
    minptr: *mut c_int,
    errcodeptr: *mut c_int,
    lcptr: *mut c_int,
    recurses: *mut parsed_recurse_check,
    cb: *mut compile_block,
) -> c_int {
    let mut branchlength: c_int = 0;
    let mut branchminlength: c_int = 0;
    let mut grouplength: c_int;
    let mut groupminlength: c_int = 0;
    let mut lastitemlength: u32 = 0;
    let mut lastitemminlength: u32 = 0;
    let mut pptr = *pptrptr;
    let mut offset: PCRE2_SIZE;
    let mut this_recurse = parsed_recurse_check {
        prev: ptr::null_mut(),
        groupptr: ptr::null_mut(),
    };

    *lcptr += 1;
    if *lcptr > 2000 {
        *errcodeptr = ERR35;
        return -1;
    }

    loop {
        let mut escape: u32;
        let mut min: u32;
        let mut max: u32;
        let mut group: u32 = 0;
        let mut itemlength: u32 = 0;
        let mut itemminlength: u32 = 0;

        // Flag used for goto ISNOTFIXED / REPETITION handling.
        let mut goto_isnotfixed = false;
        let mut goto_check_group = false;
        let mut do_repetition = false;
        let mut rep_holder_min: u32 = 0;
        let mut rep_holder_max: u32 = 0;

        if *pptr < META_END {
            itemlength = 1;
            itemminlength = 1;
        } else {
            match META_CODE(*pptr) {
                META_KET | META_ALT => {
                    *pptrptr = pptr;
                    *minptr = branchminlength;
                    return branchlength;
                }

                META_ACCEPT | META_FAIL => {
                    pptr = parsed_skip(pptr, PSKIP_ALT);
                    if pptr.is_null() {
                        *errcodeptr = ERR90;
                        return -1;
                    }
                    *pptrptr = pptr;
                    *minptr = branchminlength;
                    return branchlength;
                }

                META_MARK | META_COMMIT_ARG | META_PRUNE_ARG | META_SKIP_ARG | META_THEN_ARG => {
                    pptr = pptr.add(*pptr.add(1) as usize + 1);
                }

                META_CIRCUMFLEX | META_COMMIT | META_DOLLAR | META_PRUNE | META_SKIP
                | META_THEN => {}

                META_OPTIONS => {
                    pptr = pptr.add(2);
                }

                META_BIGVALUE => {
                    itemlength = 1;
                    itemminlength = 1;
                    pptr = pptr.add(1);
                }

                META_CLASS | META_CLASS_NOT => {
                    itemlength = 1;
                    itemminlength = 1;
                    pptr = parsed_skip(pptr, PSKIP_CLASS);
                    if pptr.is_null() {
                        *errcodeptr = ERR90;
                        return -1;
                    }
                }

                META_CLASS_EMPTY_NOT | META_DOT => {
                    itemlength = 1;
                    itemminlength = 1;
                }

                META_CALLOUT_NUMBER => {
                    pptr = pptr.add(3);
                }

                META_CALLOUT_STRING => {
                    pptr = pptr.add(3 + SIZEOFFSET);
                }

                META_ESCAPE => {
                    escape = META_DATA(*pptr);
                    if escape == ESC_X as u32 {
                        return -1;
                    }
                    if escape == ESC_R as u32 {
                        itemminlength = 1;
                        itemlength = 2;
                    } else if escape > ESC_b as u32 && escape < ESC_Z as u32 {
                        if ((*cb).external_options & PCRE2_UTF) != 0 && escape == ESC_C as u32 {
                            *errcodeptr = ERR36;
                            return -1;
                        }
                        itemlength = 1;
                        itemminlength = 1;
                        if escape == ESC_p as u32 || escape == ESC_P as u32 {
                            pptr = pptr.add(1);
                        }
                    }
                }

                META_LOOKAHEAD | META_LOOKAHEADNOT | META_LOOKAHEAD_NA | META_SCS => {
                    *errcodeptr = check_lookbehinds(pptr.add(1), &mut pptr, recurses, cb, lcptr);
                    if *errcodeptr != 0 {
                        return -1;
                    }

                    match *pptr.add(1) {
                        META_ASTERISK | META_ASTERISK_PLUS | META_ASTERISK_QUERY | META_PLUS
                        | META_PLUS_PLUS | META_PLUS_QUERY | META_QUERY | META_QUERY_PLUS
                        | META_QUERY_QUERY => {
                            pptr = pptr.add(1);
                        }
                        META_MINMAX | META_MINMAX_PLUS | META_MINMAX_QUERY => {
                            pptr = pptr.add(3);
                        }
                        _ => {}
                    }
                }

                META_LOOKBEHIND | META_LOOKBEHINDNOT | META_LOOKBEHIND_NA => {
                    if set_lookbehind_lengths(&mut pptr, errcodeptr, lcptr, recurses, cb) == FALSE {
                        return -1;
                    }
                }

                META_BACKREF_BYNAME => {
                    if ((*cb).external_options & PCRE2_MATCH_UNSET_BACKREF) != 0 {
                        goto_isnotfixed = true;
                    } else {
                        // fall through to RECURSE_BYNAME
                        let name: PCRE2_SPTR;
                        let mut is_dupname: BOOL = FALSE;
                        let ng: *mut named_group;
                        let meta_code = META_CODE(*pptr);
                        pptr = pptr.add(1);
                        let length = *pptr;

                        GETPLUSOFFSET!(offset, pptr);
                        name = (*cb).start_pattern.add(offset);
                        ng = crate::pcre2_compile_cgroup::_pcre2_compile_find_named_group8(
                            name, length, cb,
                        );

                        if ng.is_null() {
                            *errcodeptr = ERR15;
                            (*cb).erroroffset = offset;
                            return -1;
                        }

                        group = (*ng).number;
                        is_dupname = if ((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME) != 0 { TRUE } else { FALSE };

                        if meta_code == META_RECURSE_BYNAME
                            || (is_dupname == FALSE && ((*cb).external_flags & PCRE2_DUPCAPUSED) == 0)
                        {
                            // RECURSE_OR_BACKREF_LENGTH
                            match recurse_or_backref_length(
                                group, offset, &mut pptr, errcodeptr, lcptr, recurses, cb,
                                &mut itemlength, &mut itemminlength,
                            ) {
                                RbrlOutcome::Ok => {}
                                RbrlOutcome::NotFixed => goto_isnotfixed = true,
                                RbrlOutcome::Error => return -1,
                            }
                        } else {
                            goto_isnotfixed = true;
                        }
                    }
                }

                META_RECURSE_BYNAME => {
                    let name: PCRE2_SPTR;
                    let ng: *mut named_group;
                    let meta_code = META_CODE(*pptr);
                    pptr = pptr.add(1);
                    let length = *pptr;

                    GETPLUSOFFSET!(offset, pptr);
                    name = (*cb).start_pattern.add(offset);
                    ng = crate::pcre2_compile_cgroup::_pcre2_compile_find_named_group8(
                        name, length, cb,
                    );

                    if ng.is_null() {
                        *errcodeptr = ERR15;
                        (*cb).erroroffset = offset;
                        return -1;
                    }

                    group = (*ng).number;
                    let is_dupname = ((*ng).hash_dup & NAMED_GROUP_IS_DUPNAME) != 0;

                    if meta_code == META_RECURSE_BYNAME
                        || (!is_dupname && ((*cb).external_flags & PCRE2_DUPCAPUSED) == 0)
                    {
                        match recurse_or_backref_length(
                            group, offset, &mut pptr, errcodeptr, lcptr, recurses, cb,
                            &mut itemlength, &mut itemminlength,
                        ) {
                            RbrlOutcome::Ok => {}
                            RbrlOutcome::NotFixed => goto_isnotfixed = true,
                            RbrlOutcome::Error => return -1,
                        }
                    } else {
                        goto_isnotfixed = true;
                    }
                }

                META_BACKREF => {
                    if ((*cb).external_options & PCRE2_MATCH_UNSET_BACKREF) != 0
                        || ((*cb).external_flags & PCRE2_DUPCAPUSED) != 0
                    {
                        goto_isnotfixed = true;
                    } else {
                        group = META_DATA(*pptr);
                        if group < 10 {
                            offset = (*cb).small_ref_offset[group as usize];
                            match recurse_or_backref_length(
                                group, offset, &mut pptr, errcodeptr, lcptr, recurses, cb,
                                &mut itemlength, &mut itemminlength,
                            ) {
                                RbrlOutcome::Ok => {}
                                RbrlOutcome::NotFixed => goto_isnotfixed = true,
                                RbrlOutcome::Error => return -1,
                            }
                        } else {
                            // Fall through to META_RECURSE
                            group = META_DATA(*pptr);
                            GETPLUSOFFSET!(offset, pptr);
                            match recurse_or_backref_length(
                                group, offset, &mut pptr, errcodeptr, lcptr, recurses, cb,
                                &mut itemlength, &mut itemminlength,
                            ) {
                                RbrlOutcome::Ok => {}
                                RbrlOutcome::NotFixed => goto_isnotfixed = true,
                                RbrlOutcome::Error => return -1,
                            }
                        }
                    }
                }

                META_RECURSE => {
                    group = META_DATA(*pptr);
                    GETPLUSOFFSET!(offset, pptr);
                    match recurse_or_backref_length(
                        group, offset, &mut pptr, errcodeptr, lcptr, recurses, cb,
                        &mut itemlength, &mut itemminlength,
                    ) {
                        RbrlOutcome::Ok => {}
                        RbrlOutcome::NotFixed => goto_isnotfixed = true,
                        RbrlOutcome::Error => return -1,
                    }
                }

                META_COND_DEFINE => {
                    pptr = parsed_skip(pptr.add(1), PSKIP_KET);
                }

                META_COND_NAME | META_COND_NUMBER | META_COND_RNAME | META_COND_RNUMBER => {
                    pptr = pptr.add(2 + SIZEOFFSET);
                    goto_check_group = true;
                }

                META_COND_ASSERT => {
                    pptr = pptr.add(1);
                    goto_check_group = true;
                }

                META_COND_VERSION => {
                    pptr = pptr.add(4);
                    goto_check_group = true;
                }

                META_CAPTURE => {
                    group = META_DATA(*pptr);
                    pptr = pptr.add(1);
                    goto_check_group = true;
                }

                META_ATOMIC | META_NOCAPTURE | META_SCRIPT_RUN => {
                    pptr = pptr.add(1);
                    goto_check_group = true;
                }

                META_QUERY | META_QUERY_PLUS | META_QUERY_QUERY => {
                    min = 0;
                    max = 1;
                    do_repetition = true;
                    // handled below
                    // Save min/max
                    rep_holder_min = min;
                    rep_holder_max = max;
                }

                META_MINMAX | META_MINMAX_PLUS | META_MINMAX_QUERY => {
                    min = *pptr.add(1);
                    max = *pptr.add(2);
                    pptr = pptr.add(2);
                    do_repetition = true;
                    rep_holder_min = min;
                    rep_holder_max = max;
                }

                _ => {
                    goto_isnotfixed = true;
                }
            }
        }

        // Handle CHECK_GROUP jump
        if goto_check_group {
            grouplength = get_grouplength(
                &mut pptr, &mut groupminlength, TRUE, errcodeptr, lcptr, group as c_int, recurses, cb,
            );
            if grouplength < 0 {
                return -1;
            }
            itemlength = grouplength as u32;
            itemminlength = groupminlength as u32;
        }

        // Handle REPETITION
        if do_repetition {
            min = rep_holder_min;
            max = rep_holder_max;
            if max != REPEAT_UNLIMITED {
                if lastitemlength != 0
                    && max != 0
                    && (INT_MAX - branchlength) / (lastitemlength as c_int) < (max - 1) as c_int
                {
                    *errcodeptr = ERR87;
                    return -1;
                }
                if min == 0 {
                    branchminlength -= lastitemminlength as c_int;
                } else {
                    itemminlength = (min - 1) * lastitemminlength;
                }
                if max == 0 {
                    branchlength -= lastitemlength as c_int;
                } else {
                    itemlength = (max - 1) * lastitemlength;
                }
            } else {
                // fall through to ISNOTFIXED
                goto_isnotfixed = true;
            }
        }

        if goto_isnotfixed {
            *errcodeptr = ERR25;
            return -1;
        }

        if INT_MAX - branchlength < itemlength as c_int
            || {
                branchlength += itemlength as c_int;
                branchlength > LOOKBEHIND_MAX
            }
        {
            *errcodeptr = ERR87;
            return -1;
        }

        branchminlength += itemminlength as c_int;

        lastitemlength = itemlength;
        lastitemminlength = itemminlength;

        pptr = pptr.add(1);
    }
}

// Outcome of RECURSE_OR_BACKREF_LENGTH shared block.
enum RbrlOutcome {
    Ok,
    NotFixed,
    Error,
}

unsafe fn recurse_or_backref_length(
    group: u32,
    mut offset: PCRE2_SIZE,
    pptr_io: *mut *mut u32,
    errcodeptr: *mut c_int,
    lcptr: *mut c_int,
    recurses: *mut parsed_recurse_check,
    cb: *mut compile_block,
    itemlength: *mut u32,
    itemminlength: *mut u32,
) -> RbrlOutcome {
    let mut pptr = *pptr_io;
    let mut r: *mut parsed_recurse_check;
    let mut gptr: *mut u32;
    let mut gptrend: *mut u32;
    let mut groupminlength: c_int = 0;

    if group > (*cb).bracount {
        (*cb).erroroffset = offset;
        *errcodeptr = ERR15;
        return RbrlOutcome::Error;
    }
    if group == 0 {
        *pptr_io = pptr;
        return RbrlOutcome::NotFixed; // Local recursion
    }

    gptr = (*cb).parsed_pattern;
    while *gptr != META_END {
        if META_CODE(*gptr) == META_BIGVALUE {
            gptr = gptr.add(1);
        } else if *gptr == (META_CAPTURE | group) {
            break;
        }
        gptr = gptr.add(1);
    }

    gptrend = parsed_skip(gptr.add(1), PSKIP_KET);
    if gptrend.is_null() {
        *errcodeptr = ERR90;
        return RbrlOutcome::Error;
    }
    if pptr > gptr && pptr < gptrend {
        *pptr_io = pptr;
        return RbrlOutcome::NotFixed; // Local recursion
    }
    r = recurses;
    while !r.is_null() {
        if (*r).groupptr == gptr {
            break;
        }
        r = (*r).prev;
    }
    if !r.is_null() {
        *pptr_io = pptr;
        return RbrlOutcome::NotFixed; // Mutual recursion
    }
    let mut this_recurse = parsed_recurse_check {
        prev: recurses,
        groupptr: gptr,
    };

    gptr = gptr.add(1);
    let grouplength = get_grouplength(
        &mut gptr,
        &mut groupminlength,
        FALSE,
        errcodeptr,
        lcptr,
        group as c_int,
        &mut this_recurse,
        cb,
    );
    if grouplength < 0 {
        *pptr_io = pptr;
        if *errcodeptr == 0 {
            return RbrlOutcome::NotFixed;
        }
        return RbrlOutcome::Error;
    }
    *itemlength = grouplength as u32;
    *itemminlength = groupminlength as u32;
    *pptr_io = pptr;
    RbrlOutcome::Ok
}

// ---------------------------------------------------------------------------
// Set lengths in a lookbehind : set_lookbehind_lengths
// ---------------------------------------------------------------------------

unsafe fn set_lookbehind_lengths(
    pptrptr: *mut *mut u32,
    errcodeptr: *mut c_int,
    lcptr: *mut c_int,
    recurses: *mut parsed_recurse_check,
    cb: *mut compile_block,
) -> BOOL {
    let offset: PCRE2_SIZE;
    let mut bptr = *pptrptr;
    let gbptr = bptr;
    let mut maxlength: c_int = 0;
    let mut minlength: c_int = INT_MAX;
    let mut variable: BOOL = FALSE;

    READPLUSOFFSET!(offset, bptr);
    *pptrptr = (*pptrptr).add(SIZEOFFSET);

    loop {
        let mut branchminlength: c_int = 0;

        *pptrptr = (*pptrptr).add(1);
        let branchlength =
            get_branchlength(pptrptr, &mut branchminlength, errcodeptr, lcptr, recurses, cb);

        if branchlength < 0 {
            if *errcodeptr == 0 {
                *errcodeptr = ERR25;
            }
            if (*cb).erroroffset == PCRE2_UNSET {
                (*cb).erroroffset = offset;
            }
            return FALSE;
        }

        if branchlength != branchminlength {
            variable = TRUE;
        }
        if branchminlength < minlength {
            minlength = branchminlength;
        }
        if branchlength > maxlength {
            maxlength = branchlength;
        }
        if branchlength > (*cb).max_lookbehind {
            (*cb).max_lookbehind = branchlength;
        }
        *bptr |= branchlength as u32;
        bptr = *pptrptr;

        if META_CODE(*bptr) != META_ALT {
            break;
        }
    }

    if variable != FALSE {
        *gbptr.add(1) = minlength as u32;
        if (maxlength as PCRE2_SIZE) > (*cb).max_varlookbehind as PCRE2_SIZE {
            *errcodeptr = ERR100;
            (*cb).erroroffset = offset;
            return FALSE;
        }
    } else {
        *gbptr.add(1) = LOOKBEHIND_MAX as u32;
    }

    TRUE
}

// ---------------------------------------------------------------------------
// Check parsed pattern lookbehinds : check_lookbehinds
// ---------------------------------------------------------------------------

unsafe fn check_lookbehinds(
    mut pptr: *mut u32,
    retptr: *mut *mut u32,
    recurses: *mut parsed_recurse_check,
    cb: *mut compile_block,
    lcptr: *mut c_int,
) -> c_int {
    let errorcode: c_int = 0;
    let mut nestlevel: c_int = 0;

    (*cb).erroroffset = PCRE2_UNSET;

    while *pptr != META_END {
        if *pptr < META_END {
            pptr = pptr.add(1);
            continue;
        }

        match META_CODE(*pptr) {
            META_ESCAPE => {
                if *pptr - META_ESCAPE == ESC_P as u32 || *pptr - META_ESCAPE == ESC_p as u32 {
                    pptr = pptr.add(1);
                }
            }

            META_KET => {
                nestlevel -= 1;
                if nestlevel < 0 {
                    if !retptr.is_null() {
                        *retptr = pptr;
                    }
                    return 0;
                }
            }

            META_ATOMIC | META_CAPTURE | META_COND_ASSERT | META_SCS | META_LOOKAHEAD
            | META_LOOKAHEADNOT | META_LOOKAHEAD_NA | META_NOCAPTURE | META_SCRIPT_RUN => {
                nestlevel += 1;
            }

            META_ACCEPT | META_ALT | META_ASTERISK | META_ASTERISK_PLUS | META_ASTERISK_QUERY
            | META_BACKREF | META_CIRCUMFLEX | META_CLASS | META_CLASS_EMPTY
            | META_CLASS_EMPTY_NOT | META_CLASS_END | META_CLASS_NOT | META_COMMIT
            | META_DOLLAR | META_DOT | META_FAIL | META_PLUS | META_PLUS_PLUS | META_PLUS_QUERY
            | META_PRUNE | META_QUERY | META_QUERY_PLUS | META_QUERY_QUERY | META_RANGE_ESCAPED
            | META_RANGE_LITERAL | META_SKIP | META_THEN => {}

            META_OFFSET | META_RECURSE => {
                pptr = pptr.add(SIZEOFFSET);
            }

            META_BACKREF_BYNAME | META_RECURSE_BYNAME => {
                pptr = pptr.add(1 + SIZEOFFSET);
            }

            META_COND_DEFINE => {
                pptr = pptr.add(SIZEOFFSET);
                nestlevel += 1;
            }

            META_COND_NAME | META_COND_NUMBER | META_COND_RNAME | META_COND_RNUMBER => {
                pptr = pptr.add(1 + SIZEOFFSET);
                nestlevel += 1;
            }

            META_COND_VERSION => {
                pptr = pptr.add(3);
                nestlevel += 1;
            }

            META_CALLOUT_STRING => {
                pptr = pptr.add(3 + SIZEOFFSET);
            }

            META_BIGVALUE | META_POSIX | META_POSIX_NEG | META_CAPTURE_NAME
            | META_CAPTURE_NUMBER => {
                pptr = pptr.add(1);
            }

            META_MINMAX | META_MINMAX_QUERY | META_MINMAX_PLUS | META_OPTIONS => {
                pptr = pptr.add(2);
            }

            META_CALLOUT_NUMBER => {
                pptr = pptr.add(3);
            }

            META_MARK | META_COMMIT_ARG | META_PRUNE_ARG | META_SKIP_ARG | META_THEN_ARG => {
                pptr = pptr.add(1 + *pptr.add(1) as usize);
            }

            META_LOOKBEHIND | META_LOOKBEHINDNOT | META_LOOKBEHIND_NA => {
                let mut ec = 0;
                if set_lookbehind_lengths(&mut pptr, &mut ec, lcptr, recurses, cb) == FALSE {
                    return ec;
                }
            }

            _ => {
                (*cb).erroroffset = 0;
                return ERR70;
            }
        }

        pptr = pptr.add(1);
    }

    errorcode
}

// Control disposition used to emulate C gotos in parse_regex helpers.
enum Ctl {
    Fall,        // C: break out of switch, continue main loop naturally
    Cont,        // C: continue (next iteration)
    Fail,        // C: goto FAILED (errorcode already set)
    FailBack,    // C: goto FAILED_BACK
    FailForward, // C: goto FAILED_FORWARD
    Unclosed,    // C: goto UNCLOSED_PARENTHESIS
}

// Character class processing (main `[` case and `(?[` case).
// On entry class_mode_state is set, c holds '[' (or first char for perl-ext),
// and *ptr points just after that character.
unsafe fn process_class(
    pp: &mut *mut u32,
    ptr: &mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    options: u32,
    xoptions: u32,
    utf: bool,
    inescq: &mut bool,
    okquantifier: &mut bool,
    has_lookbehind: *mut BOOL,
    errorcode: &mut c_int,
    cb: *mut compile_block,
    mut class_mode_state: u32,
    mut c: u32,
) -> Ctl {
    let mut parsed_pattern = *pp;
    let mut negate_class: bool;
    let mut tempptr: PCRE2_SPTR;
    let mut escape: c_int;
    let mut class_range_state: u32;
    let mut class_op_state: u32;
    let mut class_start: *mut u32;
    let mut class_depth_m1: i16;
    let mut class_maxdepth_m1: i16;
    let mut class_range_forbid_ptr: PCRE2_SPTR = ptr::null();

    macro_rules! ret_fail {
        () => {{
            *pp = parsed_pattern;
            return Ctl::Fail;
        }};
    }

    *okquantifier = true;

    class_depth_m1 = -1;
    class_maxdepth_m1 = -1;
    class_range_state = RANGE_NO;
    class_op_state = CLASS_OP_EMPTY;
    class_start = ptr::null_mut();

    // Outer class content loop. c is set to '[' initially so the loop handles the
    // start of the class.
    'classloop: loop {
        let mut char_is_literal = true;
        // 'body handles CLASS_LITERAL fall-through and CLASS_CONTINUE tail.
        let mut goto_class_literal = false;
        let mut goto_class_continue = false;

        'classbody: {
            if *inescq {
                if c == CHAR_BACKSLASH && *ptr < ptrend && **ptr as u32 == CHAR_E {
                    *inescq = false;
                    *ptr = (*ptr).add(1);
                    goto_class_continue = true;
                    break 'classbody;
                }
                if class_mode_state == CLASS_MODE_PERL_EXT {
                    *errorcode = ERR116;
                    ret_fail!();
                }
                goto_class_literal = true;
                break 'classbody;
            }

            if (c == CHAR_SPACE || c == CHAR_HT)
                && ((options & PCRE2_EXTENDED_MORE) != 0
                    || class_mode_state >= CLASS_MODE_PERL_EXT)
            {
                goto_class_continue = true;
                break 'classbody;
            }

            // POSIX class names
            if class_depth_m1 >= 0
                && c == CHAR_LEFT_SQUARE_BRACKET
                && (ptrend as usize - *ptr as usize) >= 3
                && (**ptr as u32 == CHAR_COLON
                    || **ptr as u32 == CHAR_DOT
                    || **ptr as u32 == CHAR_EQUALS_SIGN)
                && {
                    tempptr = ptr::null();
                    check_posix_syntax(*ptr, ptrend, &mut tempptr) != FALSE
                }
            {
                let mut posix_negate = false;
                let mut posix_class: c_int;

                if class_range_state == RANGE_STARTED {
                    *ptr = tempptr.add(2);
                    *errorcode = ERR50;
                    ret_fail!();
                }

                if class_range_state == RANGE_FORBID_STARTED {
                    *ptr = class_range_forbid_ptr;
                    *errorcode = ERR50;
                    ret_fail!();
                }

                if class_op_state == CLASS_OP_OPERAND && class_mode_state == CLASS_MODE_PERL_EXT {
                    *ptr = tempptr.add(2);
                    *errorcode = ERR113;
                    ret_fail!();
                }

                if **ptr as u32 != CHAR_COLON {
                    *ptr = tempptr.add(2);
                    *errorcode = ERR13;
                    ret_fail!();
                }

                *ptr = (*ptr).add(1);
                if **ptr as u32 == CHAR_CIRCUMFLEX_ACCENT {
                    posix_negate = true;
                    *ptr = (*ptr).add(1);
                }

                posix_class = check_posix_name(*ptr, (tempptr as usize - *ptr as usize) as c_int);
                *ptr = tempptr.add(2);
                if posix_class < 0 {
                    *errorcode = ERR30;
                    ret_fail!();
                }

                class_range_state = RANGE_FORBID_NO;
                class_op_state = CLASS_OP_OPERAND;

                if (options & PCRE2_UCP) != 0
                    && (xoptions & PCRE2_EXTRA_ASCII_POSIX) == 0
                    && !((xoptions & PCRE2_EXTRA_ASCII_DIGIT) != 0
                        && (posix_class == PC_DIGIT as c_int
                            || posix_class == PC_XDIGIT as c_int))
                {
                    let ptype = POSIX_SUBSTITUTES[(2 * posix_class) as usize];
                    let pvalue = POSIX_SUBSTITUTES[(2 * posix_class + 1) as usize];

                    if ptype >= 0 {
                        wr!(parsed_pattern, META_ESCAPE + if posix_negate { ESC_P } else { ESC_p } as u32);
                        wr!(parsed_pattern, ((ptype << 16) | pvalue) as u32);
                        goto_class_continue = true;
                        break 'classbody;
                    }

                    if pvalue != 0 {
                        wr!(parsed_pattern, META_ESCAPE + if posix_negate { ESC_H } else { ESC_h } as u32);
                        goto_class_continue = true;
                        break 'classbody;
                    }
                    // Fall through
                }

                wr!(parsed_pattern, if posix_negate { META_POSIX_NEG } else { META_POSIX });
                wr!(parsed_pattern, posix_class as u32);
            }
            // Start of outermost/nested class
            else if (c == CHAR_LEFT_SQUARE_BRACKET
                && (class_depth_m1 < 0
                    || class_mode_state == CLASS_MODE_ALT_EXT
                    || class_mode_state == CLASS_MODE_PERL_EXT))
                || (c == CHAR_LEFT_PARENTHESIS && class_mode_state == CLASS_MODE_PERL_EXT)
            {
                let start_c = c;
                let new_class_mode_state: u32;

                if start_c == CHAR_LEFT_SQUARE_BRACKET
                    && class_mode_state == CLASS_MODE_PERL_EXT
                    && class_depth_m1 >= 0
                {
                    new_class_mode_state = CLASS_MODE_PERL_EXT_LEAF;
                } else {
                    new_class_mode_state = class_mode_state;
                }

                if class_range_state == RANGE_STARTED {
                    *parsed_pattern.sub(1) = CHAR_MINUS;
                }

                if class_op_state == CLASS_OP_OPERAND && class_mode_state == CLASS_MODE_PERL_EXT {
                    *errorcode = ERR113;
                    ret_fail!();
                }

                if class_depth_m1 as isize >= (ECLASS_NEST_LIMIT as isize - 1) {
                    *ptr = (*ptr).sub(1);
                    *errorcode = ERR107;
                    ret_fail!();
                }

                negate_class = false;
                loop {
                    if *ptr >= ptrend {
                        if start_c == CHAR_LEFT_PARENTHESIS {
                            *errorcode = ERR14;
                        } else {
                            *errorcode = ERR6;
                        }
                        ret_fail!();
                    }

                    c = getcharinctest(ptr, utf);
                    if new_class_mode_state == CLASS_MODE_PERL_EXT {
                        break;
                    } else if c == CHAR_BACKSLASH {
                        if *ptr < ptrend && **ptr as u32 == CHAR_E {
                            *ptr = (*ptr).add(1);
                        } else if (ptrend as usize - *ptr as usize) >= 3
                            && _pcre2_strncmp_c8(*ptr, b"Q\\E".as_ptr() as *const c_char, 3) == 0
                        {
                            *ptr = (*ptr).add(3);
                        } else {
                            break;
                        }
                    } else if (c == CHAR_SPACE || c == CHAR_HT)
                        && ((options & PCRE2_EXTENDED_MORE) != 0
                            || new_class_mode_state >= CLASS_MODE_PERL_EXT)
                    {
                        continue;
                    } else if !negate_class && c == CHAR_CIRCUMFLEX_ACCENT {
                        negate_class = true;
                    } else {
                        break;
                    }
                }

                // Empty class
                if c == CHAR_RIGHT_SQUARE_BRACKET
                    && ((*cb).external_options & PCRE2_ALLOW_EMPTY_CLASS) != 0
                    && new_class_mode_state < CLASS_MODE_PERL_EXT
                {
                    if !class_start.is_null() {
                        *class_start |= CLASS_IS_ECLASS;
                        class_start = ptr::null_mut();
                    }

                    wr!(parsed_pattern, if negate_class { META_CLASS_EMPTY_NOT } else { META_CLASS_EMPTY });

                    if class_depth_m1 < 0 {
                        break 'classloop;
                    }

                    class_range_state = RANGE_NO;
                    class_op_state = CLASS_OP_OPERAND;
                    goto_class_continue = true;
                    break 'classbody;
                }

                // Enter a non-empty class.
                if !class_start.is_null() {
                    *class_start |= CLASS_IS_ECLASS;
                    class_start = ptr::null_mut();
                }

                class_start = parsed_pattern;
                wr!(parsed_pattern, if negate_class { META_CLASS_NOT } else { META_CLASS });
                class_range_state = RANGE_NO;
                class_op_state = CLASS_OP_EMPTY;
                class_mode_state = new_class_mode_state;
                class_depth_m1 += 1;
                if class_maxdepth_m1 < class_depth_m1 {
                    class_maxdepth_m1 = class_depth_m1;
                }
                (*cb).class_op_used[class_depth_m1 as usize] = 0;

                if c == CHAR_RIGHT_SQUARE_BRACKET && new_class_mode_state != CLASS_MODE_PERL_EXT {
                    class_range_state = RANGE_OK_LITERAL;
                    class_op_state = CLASS_OP_OPERAND;
                    wr!(parsed_pattern, c);
                    *okquantifier = true;
                    goto_class_continue = true;
                    break 'classbody;
                }

                continue 'classloop; // c already loaded
            }
            // End of the class
            else if c == CHAR_RIGHT_SQUARE_BRACKET
                || (c == CHAR_RIGHT_PARENTHESIS && class_mode_state == CLASS_MODE_PERL_EXT)
            {
                if class_mode_state == CLASS_MODE_PERL_EXT {
                    if c == CHAR_RIGHT_SQUARE_BRACKET && class_depth_m1 != 0 {
                        *errorcode = ERR14;
                        *ptr = (*ptr).sub(1);
                        ret_fail!();
                    }
                    if c == CHAR_RIGHT_PARENTHESIS && class_depth_m1 < 1 {
                        *errorcode = ERR22;
                        ret_fail!();
                    }
                }

                if class_op_state == CLASS_OP_OPERATOR {
                    *errorcode = ERR110;
                    ret_fail!();
                }

                if class_mode_state == CLASS_MODE_PERL_EXT && class_op_state == CLASS_OP_EMPTY {
                    *errorcode = ERR114;
                    ret_fail!();
                }

                if class_range_state == RANGE_STARTED {
                    *parsed_pattern.sub(1) = CHAR_MINUS;
                }

                wr!(parsed_pattern, META_CLASS_END);

                class_depth_m1 -= 1;
                if class_depth_m1 < 0 {
                    if class_mode_state == CLASS_MODE_PERL_EXT {
                        if *ptr >= ptrend || **ptr as u32 != CHAR_RIGHT_PARENTHESIS {
                            *errorcode = ERR115;
                            ret_fail!();
                        }
                        *ptr = (*ptr).add(1);
                    }
                    break 'classloop;
                }

                class_range_state = RANGE_NO;
                class_op_state = CLASS_OP_OPERAND;
                if class_mode_state == CLASS_MODE_PERL_EXT_LEAF {
                    class_mode_state = CLASS_MODE_PERL_EXT;
                }
                class_start = ptr::null_mut();
            }
            // Perl set binary operator
            else if class_mode_state == CLASS_MODE_PERL_EXT
                && (c == CHAR_PLUS
                    || c == CHAR_VERTICAL_LINE
                    || c == CHAR_MINUS
                    || c == CHAR_AMPERSAND
                    || c == CHAR_CIRCUMFLEX_ACCENT)
            {
                if class_op_state != CLASS_OP_OPERAND {
                    *errorcode = ERR109;
                    ret_fail!();
                }

                if !class_start.is_null() {
                    *class_start |= CLASS_IS_ECLASS;
                    class_start = ptr::null_mut();
                }

                wr!(
                    parsed_pattern,
                    if c == CHAR_PLUS {
                        META_ECLASS_OR
                    } else if c == CHAR_VERTICAL_LINE {
                        META_ECLASS_OR
                    } else if c == CHAR_MINUS {
                        META_ECLASS_SUB
                    } else if c == CHAR_AMPERSAND {
                        META_ECLASS_AND
                    } else {
                        META_ECLASS_XOR
                    }
                );
                class_range_state = RANGE_NO;
                class_op_state = CLASS_OP_OPERATOR;
            }
            // Perl set unary operator
            else if class_mode_state == CLASS_MODE_PERL_EXT && c == CHAR_EXCLAMATION_MARK {
                if class_op_state == CLASS_OP_OPERAND {
                    *errorcode = ERR113;
                    ret_fail!();
                }

                if !class_start.is_null() {
                    *class_start |= CLASS_IS_ECLASS;
                    class_start = ptr::null_mut();
                }

                wr!(parsed_pattern, META_ECLASS_NOT);
                class_range_state = RANGE_NO;
                class_op_state = CLASS_OP_OPERATOR;
            }
            // UTS#18 set operator
            else if class_mode_state == CLASS_MODE_ALT_EXT
                && (c == CHAR_VERTICAL_LINE
                    || c == CHAR_MINUS
                    || c == CHAR_AMPERSAND
                    || c == CHAR_TILDE)
                && *ptr < ptrend
                && **ptr as u32 == c
            {
                *ptr = (*ptr).add(1);

                if *ptr < ptrend && **ptr as u32 == c {
                    while *ptr < ptrend && **ptr as u32 == c {
                        *ptr = (*ptr).add(1);
                    }
                    *errorcode = ERR108;
                    ret_fail!();
                }

                if class_op_state != CLASS_OP_OPERAND {
                    *errorcode = ERR109;
                    ret_fail!();
                }

                if (*cb).class_op_used[class_depth_m1 as usize] != 0
                    && (*cb).class_op_used[class_depth_m1 as usize] != c as u8
                {
                    *errorcode = ERR111;
                    ret_fail!();
                }

                if !class_start.is_null() {
                    *class_start |= CLASS_IS_ECLASS;
                    class_start = ptr::null_mut();
                }

                if class_range_state == RANGE_STARTED {
                    *parsed_pattern.sub(1) = CHAR_MINUS;
                }

                wr!(
                    parsed_pattern,
                    if c == CHAR_VERTICAL_LINE {
                        META_ECLASS_OR
                    } else if c == CHAR_MINUS {
                        META_ECLASS_SUB
                    } else if c == CHAR_AMPERSAND {
                        META_ECLASS_AND
                    } else {
                        META_ECLASS_XOR
                    }
                );
                class_range_state = RANGE_NO;
                class_op_state = CLASS_OP_OPERATOR;
                (*cb).class_op_used[class_depth_m1 as usize] = c as u8;
            }
            // Escapes in a class
            else if c == CHAR_BACKSLASH {
                tempptr = *ptr;
                escape = _pcre2_check_escape_8(
                    ptr,
                    ptrend,
                    &mut c,
                    errorcode,
                    options,
                    xoptions,
                    (*cb).bracount,
                    TRUE,
                    cb,
                );

                if *errorcode != 0 {
                    if (xoptions & PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL) == 0
                        || class_mode_state >= CLASS_MODE_PERL_EXT
                    {
                        ret_fail!();
                    }
                    *ptr = tempptr;
                    if *ptr >= ptrend {
                        c = CHAR_BACKSLASH;
                    } else {
                        c = getcharinctest(ptr, utf);
                    }
                    escape = 0;
                }

                let mut did_break_switch = true;
                match escape {
                    0 => {
                        char_is_literal = false;
                        goto_class_literal = true;
                        break 'classbody;
                    }
                    x if x == ESC_b => {
                        c = CHAR_BS;
                        char_is_literal = false;
                        goto_class_literal = true;
                        break 'classbody;
                    }
                    x if x == ESC_k => {
                        c = CHAR_k;
                        char_is_literal = false;
                        goto_class_literal = true;
                        break 'classbody;
                    }
                    x if x == ESC_Q => {
                        *inescq = true;
                        goto_class_continue = true;
                        break 'classbody;
                    }
                    x if x == ESC_E => {
                        goto_class_continue = true;
                        break 'classbody;
                    }
                    x if x == ESC_B || x == ESC_R || x == ESC_X => {
                        *errorcode = ERR7;
                        ret_fail!();
                    }
                    x if x == ESC_N => {
                        *errorcode = ERR71;
                        ret_fail!();
                    }
                    x if x == ESC_H || x == ESC_h || x == ESC_V || x == ESC_v => {
                        wr!(parsed_pattern, META_ESCAPE + escape as u32);
                    }
                    x if x == ESC_d
                        || x == ESC_D
                        || x == ESC_s
                        || x == ESC_S
                        || x == ESC_w
                        || x == ESC_W =>
                    {
                        parsed_pattern = handle_escdsw(escape, parsed_pattern, options, xoptions);
                    }
                    x if x == ESC_P || x == ESC_p => {
                        let mut negated: BOOL = FALSE;
                        let mut ptype: u16 = 0;
                        let mut pdata: u16 = 0;
                        if get_ucp(ptr, utf, &mut negated, &mut ptype, &mut pdata, errorcode, cb)
                            == FALSE
                        {
                            ret_fail!();
                        }

                        if (options & PCRE2_CASELESS) != 0
                            && ptype == PT_PC as u16
                            && (pdata == ucp_Lu as u16
                                || pdata == ucp_Ll as u16
                                || pdata == ucp_Lt as u16)
                        {
                            ptype = PT_LAMP as u16;
                            pdata = 0;
                        }

                        if negated != FALSE {
                            escape = if escape == ESC_P { ESC_p } else { ESC_P };
                        }
                        wr!(parsed_pattern, META_ESCAPE + escape as u32);
                        wr!(parsed_pattern, ((ptype as u32) << 16) | pdata as u32);
                    }
                    _ => {
                        // default and ESC_A/ESC_Z/ESC_z/ESC_G/ESC_K/ESC_C
                        *errorcode = ERR7;
                        ret_fail!();
                    }
                }
                let _ = did_break_switch;

                // After switch-cases that "break": these describe a set of chars.
                if class_range_state == RANGE_STARTED {
                    *errorcode = ERR50;
                    ret_fail!();
                }

                if class_range_state == RANGE_FORBID_STARTED {
                    *ptr = class_range_forbid_ptr;
                    *errorcode = ERR50;
                    ret_fail!();
                }

                if class_op_state == CLASS_OP_OPERAND && class_mode_state == CLASS_MODE_PERL_EXT {
                    *errorcode = ERR113;
                    ret_fail!();
                }

                class_range_state = RANGE_FORBID_NO;
                class_op_state = CLASS_OP_OPERAND;
            }
            // Forbid unescaped literals in Perl extended class
            else if class_mode_state == CLASS_MODE_PERL_EXT {
                *errorcode = ERR116;
                ret_fail!();
            }
            // Potential start of range
            else if c == CHAR_MINUS && class_range_state >= RANGE_OK_ESCAPED {
                wr!(
                    parsed_pattern,
                    if class_range_state == RANGE_OK_LITERAL {
                        META_RANGE_LITERAL
                    } else {
                        META_RANGE_ESCAPED
                    }
                );
                class_range_state = RANGE_STARTED;
            }
            // Forbidden start of range
            else if c == CHAR_MINUS && class_range_state == RANGE_FORBID_NO {
                wr!(parsed_pattern, CHAR_MINUS);
                class_range_state = RANGE_FORBID_STARTED;
                class_range_forbid_ptr = *ptr;
            }
            // Literal character
            else {
                goto_class_literal = true;
            }
        } // 'classbody

        // CLASS_LITERAL:
        if goto_class_literal {
            if class_op_state == CLASS_OP_OPERAND && class_mode_state == CLASS_MODE_PERL_EXT {
                *errorcode = ERR113;
                ret_fail!();
            }

            if class_range_state == RANGE_STARTED {
                if c == *parsed_pattern.sub(2) {
                    parsed_pattern = parsed_pattern.sub(1);
                } else if *parsed_pattern.sub(2) > c {
                    *errorcode = ERR8;
                    ret_fail!();
                } else {
                    if !char_is_literal && *parsed_pattern.sub(1) == META_RANGE_LITERAL {
                        *parsed_pattern.sub(1) = META_RANGE_ESCAPED;
                    }
                    wr!(parsed_pattern, c);
                }
                class_range_state = RANGE_NO;
                class_op_state = CLASS_OP_OPERAND;
            } else if class_range_state == RANGE_FORBID_STARTED {
                *ptr = class_range_forbid_ptr;
                *errorcode = ERR50;
                ret_fail!();
            } else {
                class_range_state = if char_is_literal {
                    RANGE_OK_LITERAL
                } else {
                    RANGE_OK_ESCAPED
                };
                class_op_state = CLASS_OP_OPERAND;
                wr!(parsed_pattern, c);
            }
        }

        // CLASS_CONTINUE:
        let _ = goto_class_continue;
        if *ptr >= ptrend {
            if class_mode_state == CLASS_MODE_PERL_EXT && class_depth_m1 > 0 {
                *errorcode = ERR14;
            }
            if class_mode_state == CLASS_MODE_ALT_EXT
                && class_depth_m1 == 0
                && class_maxdepth_m1 == 1
            {
                *errorcode = ERR112;
            } else {
                *errorcode = ERR6;
            }
            ret_fail!();
        }
        c = getcharinctest(ptr, utf);
    } // 'classloop

    *pp = parsed_pattern;
    Ctl::Fall
}

// ---------------------------------------------------------------------------
// Parse regex and identify named groups : parse_regex
// ---------------------------------------------------------------------------

// Targets for goto-dispatch inside the '(' handling.
#[derive(PartialEq, Clone, Copy)]
enum PT {
    Done,        // finished '(' handling -> continue main loop
    SetRecursion,
    RecursionByNumber,
    RecurseByName,
    ReadRecursionArguments,
    DefineName,
    AtomicGroup,
    PositiveLookAhead,
    PositiveNonatomicLookAhead,
    NegativeLookAhead,
    PostLookbehind,
    PostAssertion,
}

unsafe fn parse_regex(
    mut ptr: PCRE2_SPTR,
    mut options: u32,
    mut xoptions: u32,
    has_lookbehind: *mut BOOL,
    cb: *mut compile_block,
) -> c_int {
    let mut c: u32;
    let mut delimiter: u32;
    let mut namelen: u32 = 0;
    let mut class_mode_state: u32;
    let mut verblengthptr: *mut u32 = ptr::null_mut();
    let mut verbstartptr: *mut u32 = ptr::null_mut();
    let mut previous_callout: *mut u32 = ptr::null_mut();
    let mut parsed_pattern = (*cb).parsed_pattern;
    let parsed_pattern_end = (*cb).parsed_pattern_end;
    let mut this_parsed_item: *mut u32 = ptr::null_mut();
    let mut prev_parsed_item: *mut u32 = ptr::null_mut();
    let mut meta_quantifier: u32 = 0;
    let mut add_after_mark: u32 = 0;
    let mut nest_depth: u16 = 0;
    let mut hash: u16;
    let mut after_manual_callout: c_int = 0;
    let mut expect_cond_assert: c_int = 0;
    let mut errorcode: c_int = 0;
    let mut escape: c_int;
    let mut i: c_int = 0;
    let mut inescq = false;
    let mut inverbname = false;
    let utf = (options & PCRE2_UTF) != 0;
    let auto_callout = (options & PCRE2_AUTO_CALLOUT) != 0;
    let mut is_dupname: bool;
    let mut okquantifier = false;
    let mut thisptr: PCRE2_SPTR;
    let mut name: PCRE2_SPTR = ptr::null();
    let ptrend = (*cb).end_pattern;
    let mut verbnamestart: PCRE2_SPTR = ptr::null();
    let mut ng: *mut named_group;
    let start_pattern = (*cb).start_pattern;

    // nest stack (nest_save vector in workspace)
    let mut top_nest: *mut nest_save = ptr::null_mut();
    let end_nests_base = (*cb).start_workspace.add((*cb).workspace_size) as *mut u8;
    let end_nests: *mut nest_save = (end_nests_base.sub(
        ((*cb).workspace_size * 1) % core::mem::size_of::<nest_save>(),
    )) as *mut nest_save;
    let workspace_base = (*cb).start_workspace as *mut nest_save;

    macro_rules! do_fail {
        () => {{
            (*cb).erroroffset = (ptr as usize - start_pattern as usize) as PCRE2_SIZE;
            return errorcode;
        }};
    }
    macro_rules! do_fail_back {
        () => {{
            ptr = ptr.sub(1);
            if utf {
                backchar(&mut ptr);
            }
            do_fail!();
        }};
    }
    macro_rules! do_fail_forward {
        () => {{
            ptr = ptr.add(1);
            if utf {
                forwardchartest(&mut ptr, ptrend);
            }
            do_fail!();
        }};
    }
    macro_rules! do_unclosed {
        () => {{
            errorcode = ERR14;
            do_fail!();
        }};
    }

    // Leading items for word/line matching.
    if (xoptions & PCRE2_EXTRA_MATCH_LINE) != 0 {
        wr!(parsed_pattern, META_CIRCUMFLEX);
        wr!(parsed_pattern, META_NOCAPTURE);
    } else if (xoptions & PCRE2_EXTRA_MATCH_WORD) != 0 {
        wr!(parsed_pattern, META_ESCAPE + ESC_b as u32);
        wr!(parsed_pattern, META_NOCAPTURE);
    }

    // Literal pattern fast path.
    if (options & PCRE2_LITERAL) != 0 {
        while ptr < ptrend {
            thisptr = ptr;
            c = getcharinctest(&mut ptr, utf);
            if auto_callout {
                parsed_pattern =
                    manage_callouts(thisptr, &mut previous_callout, auto_callout, parsed_pattern, cb);
            }
            wr!(parsed_pattern, c);
            okquantifier = true;
        }
        // goto PARSED_END
        return parsed_end(
            ptr, options, xoptions, utf, auto_callout, nest_depth, inverbname, ptrend,
            &mut previous_callout, parsed_pattern, parsed_pattern_end, cb, start_pattern,
        );
    }

    if (options & PCRE2_EXTENDED_MORE) != 0 {
        options |= PCRE2_EXTENDED;
    }

    'mainloop: while ptr < ptrend {
        let mut min_repeat: u32 = 0;
        let mut max_repeat: u32 = 0;
        let mut set: u32;
        let mut unset: u32;
        let mut xset: u32;
        let mut xunset: u32;
        let mut terminator: u32;
        let prev_meta_quantifier: u32;
        let prev_okquantifier: bool;
        let mut tempptr: PCRE2_SPTR;
        let mut offset: PCRE2_SIZE = 0;

        if nest_depth as u32 > (*(*cb).cx).parens_nest_limit {
            errorcode = ERR19;
            do_fail!();
        }

        if this_parsed_item != parsed_pattern {
            prev_parsed_item = this_parsed_item;
            this_parsed_item = parsed_pattern;
        }

        thisptr = ptr;
        c = getcharinctest(&mut ptr, utf);

        // Inside \Q..\E
        if inescq {
            if c == CHAR_BACKSLASH && ptr < ptrend && *ptr as u32 == CHAR_E {
                inescq = false;
                ptr = ptr.add(1);
            } else {
                if inverbname {
                    wr!(parsed_pattern, c);
                } else {
                    after_manual_callout -= 1;
                    if after_manual_callout < 0 {
                        parsed_pattern = manage_callouts(
                            thisptr, &mut previous_callout, auto_callout, parsed_pattern, cb,
                        );
                    }
                    wr!(parsed_pattern, c);
                    okquantifier = true;
                }
                meta_quantifier = 0;
            }
            continue 'mainloop;
        }

        // (*VERB:NAME) name characters
        if inverbname
            && (((options & (PCRE2_EXTENDED | PCRE2_ALT_VERBNAMES))
                != (PCRE2_EXTENDED | PCRE2_ALT_VERBNAMES))
                || (c > 255 && (c | 1) != 0x200f && (c | 1) != 0x2029)
                || (c < 256
                    && c != CHAR_NUMBER_SIGN
                    && (*(*cb).ctypes.add(c as usize) & ctype_space) == 0
                    && c != CHAR_NEL))
        {
            let verbnamelength: PCRE2_SIZE;
            match c {
                CHAR_RIGHT_PARENTHESIS => {
                    inverbname = false;
                    verbnamelength =
                        (parsed_pattern as usize - verblengthptr as usize) / 4 - 1;
                    if (ptr as usize - verbnamestart as usize) as isize - 1 > MAX_MARK() as isize {
                        ptr = ptr.sub(1);
                        errorcode = ERR76;
                        do_fail!();
                    }
                    *verblengthptr = verbnamelength as u32;

                    if add_after_mark != 0 {
                        wr!(parsed_pattern, add_after_mark);
                        add_after_mark = 0;
                    }
                }

                CHAR_BACKSLASH => {
                    if (options & PCRE2_ALT_VERBNAMES) != 0 {
                        escape = _pcre2_check_escape_8(
                            &mut ptr, ptrend, &mut c, &mut errorcode, options, xoptions,
                            (*cb).bracount, FALSE, cb,
                        );
                        if errorcode != 0 {
                            do_fail!();
                        }
                    } else {
                        escape = 0;
                    }

                    match escape {
                        0 => {
                            wr!(parsed_pattern, c);
                        }
                        x if x == ESC_ub => {
                            wr!(parsed_pattern, CHAR_u);
                            wr!(parsed_pattern, CHAR_LEFT_CURLY_BRACKET);
                            okquantifier = true;
                        }
                        x if x == ESC_Q => {
                            inescq = true;
                        }
                        x if x == ESC_E => {}
                        _ => {
                            errorcode = ERR40;
                            do_fail!();
                        }
                    }
                }

                _ => {
                    wr!(parsed_pattern, c);
                }
            }
            continue 'mainloop;
        }

        // \Q and \E handling (not changing quantifier state)
        if c == CHAR_BACKSLASH && ptr < ptrend {
            if *ptr as u32 == CHAR_Q || *ptr as u32 == CHAR_E {
                if expect_cond_assert > 0
                    && *ptr as u32 == CHAR_Q
                    && !((ptrend as usize - ptr as usize) >= 3
                        && *ptr.add(1) as u32 == CHAR_BACKSLASH
                        && *ptr.add(2) as u32 == CHAR_E)
                {
                    ptr = ptr.sub(1);
                    errorcode = ERR28;
                    do_fail!();
                }
                inescq = *ptr as u32 == CHAR_Q;
                ptr = ptr.add(1);
                continue 'mainloop;
            }
        }

        // Skip whitespace and # comments in extended mode.
        if (options & PCRE2_EXTENDED) != 0 {
            if c < 256 && (*(*cb).ctypes.add(c as usize) & ctype_space) != 0 {
                continue 'mainloop;
            }
            if c == CHAR_NEL || (c | 1) == 0x200f || (c | 1) == 0x2029 {
                continue 'mainloop;
            }
            if c == CHAR_NUMBER_SIGN {
                while ptr < ptrend {
                    if is_newline_at(ptr, cb, utf) {
                        ptr = ptr.add((*cb).nllen as usize);
                        break;
                    }
                    ptr = ptr.add(1);
                    if utf {
                        forwardchartest(&mut ptr, ptrend);
                    }
                }
                continue 'mainloop;
            }
        }

        // Skip bracketed comments (?# ... )
        if c == CHAR_LEFT_PARENTHESIS
            && (ptrend as usize - ptr as usize) >= 2
            && *ptr.add(0) as u32 == CHAR_QUESTION_MARK
            && *ptr.add(1) as u32 == CHAR_NUMBER_SIGN
        {
            ptr = ptr.add(1);
            while ptr < ptrend && *ptr as u32 != CHAR_RIGHT_PARENTHESIS {
                ptr = ptr.add(1);
            }
            if ptr >= ptrend {
                errorcode = ERR18;
                do_fail!();
            }
            ptr = ptr.add(1);
            continue 'mainloop;
        }

        // Fill in previous callout / auto callout if next item isn't a quantifier.
        if c != CHAR_ASTERISK
            && c != CHAR_PLUS
            && c != CHAR_QUESTION_MARK
            && (c != CHAR_LEFT_CURLY_BRACKET || {
                tempptr = ptr;
                read_repeat_counts(&mut tempptr, ptrend, ptr::null_mut(), ptr::null_mut(), &mut errorcode)
                    == FALSE
            })
        {
            after_manual_callout -= 1;
            if after_manual_callout < 0 {
                parsed_pattern = manage_callouts(
                    thisptr, &mut previous_callout, auto_callout, parsed_pattern, cb,
                );
                this_parsed_item = parsed_pattern;
            }
        }

        // Conditional assertion expectation.
        if expect_cond_assert > 0 {
            let mut ok = c == CHAR_LEFT_PARENTHESIS
                && (ptrend as usize - ptr as usize) >= 3
                && (*ptr.add(0) as u32 == CHAR_QUESTION_MARK
                    || *ptr.add(0) as u32 == CHAR_ASTERISK);
            if ok {
                if *ptr.add(0) as u32 == CHAR_ASTERISK {
                    ok = (*(*cb).ctypes.add(*ptr.add(1) as usize) & ctype_lcletter) != 0;
                } else {
                    match *ptr.add(1) as u32 {
                        CHAR_C => {
                            ok = expect_cond_assert == 2;
                        }
                        CHAR_EQUALS_SIGN | CHAR_EXCLAMATION_MARK => {}
                        CHAR_LESS_THAN_SIGN => {
                            ok = *ptr.add(2) as u32 == CHAR_EQUALS_SIGN
                                || *ptr.add(2) as u32 == CHAR_EXCLAMATION_MARK;
                        }
                        _ => {
                            ok = false;
                        }
                    }
                }
            }

            if !ok {
                errorcode = ERR28;
                if expect_cond_assert == 2 {
                    do_fail!();
                }
                do_fail_back!();
            }
        }

        let prev_expect_cond_assert = expect_cond_assert;
        expect_cond_assert = 0;

        prev_okquantifier = okquantifier;
        prev_meta_quantifier = meta_quantifier;
        okquantifier = false;
        meta_quantifier = 0;

        if prev_meta_quantifier != 0 && (c == CHAR_QUESTION_MARK || c == CHAR_PLUS) {
            let idx: isize = if prev_meta_quantifier == META_MINMAX { -3 } else { -1 };
            *parsed_pattern.offset(idx) = prev_meta_quantifier
                + (if c == CHAR_QUESTION_MARK { 0x00020000 } else { 0x00010000 });
            continue 'mainloop;
        }

        // ----- main switch on c -----
        // meta_quantifier used by CHECK_QUANTIFIER block.
        let mut go_check_quantifier = false;

        match c {
            CHAR_BACKSLASH => {
                tempptr = ptr;
                escape = _pcre2_check_escape_8(
                    &mut ptr, ptrend, &mut c, &mut errorcode, options, xoptions, (*cb).bracount,
                    FALSE, cb,
                );
                if errorcode != 0 {
                    // ESCAPE_FAILED
                    if (xoptions & PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL) == 0 {
                        do_fail!();
                    }
                    ptr = tempptr;
                    if ptr >= ptrend {
                        c = CHAR_BACKSLASH;
                    } else {
                        c = getcharinctest(&mut ptr, utf);
                    }
                    escape = 0;
                }

                if escape == 0 {
                    wr!(parsed_pattern, c);
                    okquantifier = true;
                } else if escape < 0 {
                    offset = (ptr as usize - start_pattern as usize) as PCRE2_SIZE;
                    escape = -escape - 1;
                    wr!(parsed_pattern, META_BACKREF | escape as u32);
                    if escape < 10 {
                        if (*cb).small_ref_offset[escape as usize] == PCRE2_UNSET {
                            (*cb).small_ref_offset[escape as usize] = offset;
                        }
                    } else {
                        PUTOFFSET!(offset, parsed_pattern);
                    }
                    okquantifier = true;
                } else {
                    // escape > 0
                    match escape {
                        x if x == ESC_C => {
                            if (options & PCRE2_NEVER_BACKSLASH_C) != 0 {
                                errorcode = ERR83;
                                do_fail!(); // ESCAPE_FAILED (errorcode set, no forward)
                            }
                            okquantifier = true;
                            wr!(parsed_pattern, META_ESCAPE + escape as u32);
                        }
                        x if x == ESC_ub => {
                            wr!(parsed_pattern, CHAR_u);
                            wr!(parsed_pattern, CHAR_LEFT_CURLY_BRACKET);
                            okquantifier = true;
                        }
                        x if x == ESC_X
                            || x == ESC_H
                            || x == ESC_h
                            || x == ESC_N
                            || x == ESC_R
                            || x == ESC_V
                            || x == ESC_v =>
                        {
                            okquantifier = true;
                            wr!(parsed_pattern, META_ESCAPE + escape as u32);
                        }
                        x if x == ESC_d
                            || x == ESC_D
                            || x == ESC_s
                            || x == ESC_S
                            || x == ESC_w
                            || x == ESC_W =>
                        {
                            okquantifier = true;
                            parsed_pattern =
                                handle_escdsw(escape, parsed_pattern, options, xoptions);
                        }
                        x if x == ESC_P || x == ESC_p => {
                            let mut negated: BOOL = FALSE;
                            let mut ptype: u16 = 0;
                            let mut pdata: u16 = 0;
                            if get_ucp(&mut ptr, utf, &mut negated, &mut ptype, &mut pdata, &mut errorcode, cb)
                                == FALSE
                            {
                                do_fail!(); // ESCAPE_FAILED
                            }
                            if negated != FALSE {
                                escape = if escape == ESC_P { ESC_p } else { ESC_P };
                            }
                            wr!(parsed_pattern, META_ESCAPE + escape as u32);
                            wr!(parsed_pattern, ((ptype as u32) << 16) | pdata as u32);
                            okquantifier = true;
                        }
                        x if x == ESC_g || x == ESC_k => {
                            if ptr >= ptrend
                                || (*ptr as u32 != CHAR_LEFT_CURLY_BRACKET
                                    && *ptr as u32 != CHAR_LESS_THAN_SIGN
                                    && *ptr as u32 != CHAR_APOSTROPHE)
                            {
                                errorcode = if escape == ESC_g { ERR57 } else { ERR69 };
                                do_fail!(); // ESCAPE_FAILED
                            }
                            terminator = if *ptr as u32 == CHAR_LESS_THAN_SIGN {
                                CHAR_GREATER_THAN_SIGN
                            } else if *ptr as u32 == CHAR_APOSTROPHE {
                                CHAR_APOSTROPHE
                            } else {
                                CHAR_RIGHT_CURLY_BRACKET
                            };

                            let mut did_recursion = false;
                            if escape == ESC_g && terminator != CHAR_RIGHT_CURLY_BRACKET {
                                let mut p = ptr.add(1);
                                if read_number(&mut p, ptrend, (*cb).bracount as i32, MAX_GROUP_NUMBER, ERR61 as u32, &mut i, &mut errorcode) != FALSE {
                                    if p >= ptrend || *p as u32 != terminator {
                                        ptr = p;
                                        errorcode = ERR119;
                                        do_fail!(); // ESCAPE_FAILED
                                    }
                                    ptr = p.add(1);
                                    // goto SET_RECURSION
                                    offset = 0;
                                    match paren_dispatch(
                                        PT::SetRecursion, i, terminator, offset, &mut ptr, ptrend,
                                        utf, &mut parsed_pattern, &mut errorcode, cb, start_pattern,
                                        &mut okquantifier, &mut nest_depth, options, has_lookbehind,
                                    ) {
                                        ParenRes::Done => { did_recursion = true; }
                                        ParenRes::Fail => do_fail!(),
                                        ParenRes::FailForward => do_fail_forward!(),
                                        ParenRes::Unclosed => do_unclosed!(),
                                    }
                                } else if errorcode != 0 {
                                    do_fail!(); // ESCAPE_FAILED
                                }
                            }

                            if !did_recursion {
                                if read_name(&mut ptr, ptrend, utf, terminator, &mut offset, &mut name, &mut namelen, &mut errorcode, cb) == FALSE {
                                    do_fail!(); // ESCAPE_FAILED
                                }

                                wr!(
                                    parsed_pattern,
                                    if escape == ESC_k || terminator == CHAR_RIGHT_CURLY_BRACKET {
                                        META_BACKREF_BYNAME
                                    } else {
                                        META_RECURSE_BYNAME
                                    }
                                );
                                wr!(parsed_pattern, namelen);
                                PUTOFFSET!(offset, parsed_pattern);
                                okquantifier = true;
                            }
                        }
                        _ => {
                            // \A, \B, \b, \G, \K, \Z, \z
                            wr!(parsed_pattern, META_ESCAPE + escape as u32);
                        }
                    }
                }
            }

            CHAR_CIRCUMFLEX_ACCENT => {
                wr!(parsed_pattern, META_CIRCUMFLEX);
            }

            CHAR_DOLLAR_SIGN => {
                wr!(parsed_pattern, META_DOLLAR);
            }

            CHAR_DOT => {
                wr!(parsed_pattern, META_DOT);
                okquantifier = true;
            }

            CHAR_ASTERISK => {
                meta_quantifier = META_ASTERISK;
                go_check_quantifier = true;
            }
            CHAR_PLUS => {
                meta_quantifier = META_PLUS;
                go_check_quantifier = true;
            }
            CHAR_QUESTION_MARK => {
                meta_quantifier = META_QUERY;
                go_check_quantifier = true;
            }

            CHAR_LEFT_CURLY_BRACKET => {
                if read_repeat_counts(&mut ptr, ptrend, &mut min_repeat, &mut max_repeat, &mut errorcode) == FALSE {
                    if errorcode != 0 {
                        do_fail!();
                    }
                    wr!(parsed_pattern, c);
                    okquantifier = true;
                    continue 'mainloop;
                }
                meta_quantifier = META_MINMAX;
                go_check_quantifier = true;
            }

            CHAR_LEFT_SQUARE_BRACKET => {
                // Handle [[:<:]] / [[:>:]]
                if (ptrend as usize - ptr as usize) >= 6
                    && (_pcre2_strncmp_c8(ptr, b"[:<:]]".as_ptr() as *const c_char, 6) == 0
                        || _pcre2_strncmp_c8(ptr, b"[:>:]]".as_ptr() as *const c_char, 6) == 0)
                {
                    wr!(parsed_pattern, META_ESCAPE + ESC_b as u32);
                    if *ptr.add(2) as u32 == CHAR_LESS_THAN_SIGN {
                        wr!(parsed_pattern, META_LOOKAHEAD);
                    } else {
                        wr!(parsed_pattern, META_LOOKBEHIND);
                        *has_lookbehind = TRUE;
                        PUTOFFSET!(0usize, parsed_pattern);
                    }
                    if (options & PCRE2_UCP) == 0 {
                        wr!(parsed_pattern, META_ESCAPE + ESC_w as u32);
                    } else {
                        wr!(parsed_pattern, META_ESCAPE + ESC_p as u32);
                        wr!(parsed_pattern, PT_WORD << 16);
                    }
                    wr!(parsed_pattern, META_KET);
                    ptr = ptr.add(6);
                    okquantifier = true;
                    continue 'mainloop;
                }

                if ptr < ptrend
                    && (*ptr as u32 == CHAR_COLON
                        || *ptr as u32 == CHAR_DOT
                        || *ptr as u32 == CHAR_EQUALS_SIGN)
                    && {
                        tempptr = ptr::null();
                        check_posix_syntax(ptr, ptrend, &mut tempptr) != FALSE
                    }
                {
                    errorcode = if *ptr as u32 == CHAR_COLON { ERR12 } else { ERR13 };
                    ptr = ptr.sub(1);
                    ptr = tempptr.add(2);
                    do_fail!();
                }

                class_mode_state = if (options & PCRE2_ALT_EXTENDED_CLASS) != 0 {
                    CLASS_MODE_ALT_EXT
                } else {
                    CLASS_MODE_NORMAL
                };

                match process_class(
                    &mut parsed_pattern, &mut ptr, ptrend, options, xoptions, utf, &mut inescq,
                    &mut okquantifier, has_lookbehind, &mut errorcode, cb, class_mode_state, c,
                ) {
                    Ctl::Fall => {}
                    Ctl::Fail => do_fail!(),
                    _ => do_fail!(),
                }
            }

            CHAR_LEFT_PARENTHESIS => {
                if ptr >= ptrend {
                    do_unclosed!();
                }

                // Non-(? branch: captures, verbs, alpha assertions.
                let mut go_paren = PT::Done;
                let mut paren_i: c_int = 0;
                let mut paren_terminator: u32 = 0;
                let mut paren_offset: PCRE2_SIZE = 0;
                let mut handled_here = false;

                if *ptr as u32 != CHAR_QUESTION_MARK {
                    if *ptr as u32 != CHAR_ASTERISK {
                        nest_depth += 1;
                        if (options & PCRE2_NO_AUTO_CAPTURE) == 0 {
                            if (*cb).bracount >= MAX_GROUP_NUMBER {
                                errorcode = ERR97;
                                do_fail!();
                            }
                            (*cb).bracount += 1;
                            wr!(parsed_pattern, META_CAPTURE | (*cb).bracount);
                        } else {
                            wr!(parsed_pattern, META_NOCAPTURE);
                        }
                        handled_here = true; // break; done with paren
                    } else if (ptrend as usize - ptr as usize) <= 1 || {
                        c = *ptr.add(1) as u32;
                        c == CHAR_RIGHT_PARENTHESIS
                    } {
                        // (* at end or (*) -> break, gives error later
                        handled_here = true;
                    } else if (c <= 255) && (*(*cb).ctypes.add(c as usize) & ctype_lcletter) != 0 {
                        // alpha assertion
                        let mut vn = ALASNAMES.as_ptr();
                        if read_name(&mut ptr, ptrend, utf, 0, &mut offset, &mut name, &mut namelen, &mut errorcode, cb) == FALSE {
                            do_fail!();
                        }
                        if ptr >= ptrend {
                            do_unclosed!();
                        }
                        if *ptr as u32 != CHAR_COLON {
                            errorcode = ERR95;
                            do_fail_forward!();
                        }

                        i = 0;
                        while (i as usize) < ALASCOUNT {
                            if namelen == ALASMETA[i as usize].len
                                && _pcre2_strncmp_c8(name, vn as *const c_char, namelen as usize) == 0
                            {
                                break;
                            }
                            vn = vn.add(ALASMETA[i as usize].len as usize + 1);
                            i += 1;
                        }

                        if i as usize >= ALASCOUNT {
                            errorcode = ERR95;
                            do_fail!();
                        }

                        let meta = ALASMETA[i as usize].meta;
                        if prev_expect_cond_assert > 0
                            && (meta < META_LOOKAHEAD || meta > META_LOOKBEHINDNOT)
                        {
                            errorcode = ERR28;
                            do_fail!();
                        }

                        match meta {
                            META_ATOMIC => {
                                go_paren = PT::AtomicGroup;
                            }
                            META_LOOKAHEAD => {
                                go_paren = PT::PositiveLookAhead;
                            }
                            META_LOOKAHEAD_NA => {
                                go_paren = PT::PositiveNonatomicLookAhead;
                            }
                            META_LOOKAHEADNOT => {
                                go_paren = PT::NegativeLookAhead;
                            }
                            META_SCS => {
                                ptr = ptr.add(1);
                                wr!(parsed_pattern, META_SCS);
                                parsed_pattern = parse_capture_list(
                                    &mut ptr, ptrend, utf, parsed_pattern, 0, &mut errorcode, cb,
                                );
                                if parsed_pattern.is_null() {
                                    do_fail!();
                                }
                                go_paren = PT::PostAssertion;
                            }
                            META_LOOKBEHIND | META_LOOKBEHINDNOT | META_LOOKBEHIND_NA => {
                                wr!(parsed_pattern, meta);
                                ptr = ptr.sub(1);
                                go_paren = PT::PostLookbehind;
                            }
                            META_SCRIPT_RUN | META_ATOMIC_SCRIPT_RUN => {
                                wr!(parsed_pattern, META_SCRIPT_RUN);
                                nest_depth += 1;
                                ptr = ptr.add(1);
                                if meta == META_ATOMIC_SCRIPT_RUN {
                                    wr!(parsed_pattern, META_ATOMIC);
                                    if top_nest.is_null() {
                                        top_nest = workspace_base;
                                    } else {
                                        top_nest = top_nest.add(1);
                                        if top_nest >= end_nests {
                                            errorcode = ERR84;
                                            do_fail!();
                                        }
                                    }
                                    (*top_nest).nest_depth = nest_depth;
                                    (*top_nest).flags = NSF_ATOMICSR;
                                    (*top_nest).options = options & PARSE_TRACKED_OPTIONS;
                                    (*top_nest).xoptions = xoptions & PARSE_TRACKED_EXTRA_OPTIONS;
                                }
                                handled_here = true;
                            }
                            _ => {
                                errorcode = ERR89;
                                do_fail!();
                            }
                        }
                    } else {
                        // (*VERB) handling
                        let mut vn = VERBNAMES.as_ptr();
                        if read_name(&mut ptr, ptrend, utf, 0, &mut offset, &mut name, &mut namelen, &mut errorcode, cb) == FALSE {
                            do_fail!();
                        }
                        if ptr >= ptrend
                            || (*ptr as u32 != CHAR_COLON && *ptr as u32 != CHAR_RIGHT_PARENTHESIS)
                        {
                            errorcode = ERR60;
                            do_fail!();
                        }

                        i = 0;
                        while (i as usize) < VERBCOUNT {
                            if namelen == VERBS[i as usize].len
                                && _pcre2_strncmp_c8(name, vn as *const c_char, namelen as usize) == 0
                            {
                                break;
                            }
                            vn = vn.add(VERBS[i as usize].len as usize + 1);
                            i += 1;
                        }

                        if i as usize >= VERBCOUNT {
                            errorcode = ERR60;
                            do_fail!();
                        }

                        if *ptr as u32 == CHAR_COLON
                            && ptr.add(1) < ptrend
                            && *ptr.add(1) as u32 == CHAR_RIGHT_PARENTHESIS
                        {
                            ptr = ptr.add(1);
                        }

                        if VERBS[i as usize].has_arg > 0 && *ptr as u32 != CHAR_COLON {
                            errorcode = ERR66;
                            do_fail!();
                        }

                        verbstartptr = parsed_pattern;
                        okquantifier = VERBS[i as usize].meta == META_ACCEPT;

                        let was_colon = *ptr as u32 == CHAR_COLON;
                        ptr = ptr.add(1);
                        if was_colon {
                            if VERBS[i as usize].has_arg < 0 {
                                add_after_mark = VERBS[i as usize].meta;
                                wr!(parsed_pattern, META_MARK);
                            } else {
                                wr!(
                                    parsed_pattern,
                                    VERBS[i as usize].meta
                                        + if VERBS[i as usize].meta != META_MARK {
                                            0x00010000
                                        } else {
                                            0
                                        }
                                );
                            }
                            verblengthptr = parsed_pattern;
                            parsed_pattern = parsed_pattern.add(1);
                            verbnamestart = ptr;
                            inverbname = true;
                        } else {
                            wr!(parsed_pattern, VERBS[i as usize].meta);
                        }
                        handled_here = true;
                    }
                }

                if !handled_here && go_paren == PT::Done && *ptr as u32 == CHAR_QUESTION_MARK {
                    // (? branch
                    ptr = ptr.add(1);
                    if ptr >= ptrend {
                        do_unclosed!();
                    }

                    // The big (? switch. Handle option-setting & others that
                    // don't jump to shared labels inline; jumps set go_paren.
                    match parse_question_paren(
                        &mut ptr, ptrend, utf, &mut options, &mut xoptions, &mut parsed_pattern,
                        &mut errorcode, cb, start_pattern, &mut nest_depth, &mut top_nest,
                        end_nests, workspace_base, &mut expect_cond_assert, &mut okquantifier,
                        has_lookbehind, prev_expect_cond_assert, &mut go_paren, &mut paren_i,
                        &mut paren_terminator, &mut paren_offset, &mut previous_callout,
                        &mut after_manual_callout,
                    ) {
                        ParenRes::Done => {
                            if go_paren == PT::Done && paren_terminator != 0xFFFFFFFF {
                                handled_here = true;
                            }
                        }
                        ParenRes::Fail => do_fail!(),
                        ParenRes::FailForward => do_fail_forward!(),
                        ParenRes::Unclosed => do_unclosed!(),
                    }
                }

                // Run shared paren label dispatch if requested.
                if !handled_here && go_paren != PT::Done {
                    match paren_dispatch(
                        go_paren, paren_i, paren_terminator, paren_offset, &mut ptr, ptrend, utf,
                        &mut parsed_pattern, &mut errorcode, cb, start_pattern, &mut okquantifier,
                        &mut nest_depth, options, has_lookbehind,
                    ) {
                        ParenRes::Done => {}
                        ParenRes::Fail => do_fail!(),
                        ParenRes::FailForward => do_fail_forward!(),
                        ParenRes::Unclosed => do_unclosed!(),
                    }

                    // PostAssertion / PostLookbehind need cond-assert nest tracking.
                    if go_paren == PT::PositiveLookAhead
                        || go_paren == PT::PositiveNonatomicLookAhead
                        || go_paren == PT::NegativeLookAhead
                        || go_paren == PT::PostAssertion
                        || go_paren == PT::PostLookbehind
                    {
                        nest_depth += 1;
                        if prev_expect_cond_assert > 0 {
                            if top_nest.is_null() {
                                top_nest = workspace_base;
                            } else {
                                top_nest = top_nest.add(1);
                                if top_nest >= end_nests {
                                    errorcode = ERR84;
                                    do_fail!();
                                }
                            }
                            (*top_nest).nest_depth = nest_depth;
                            (*top_nest).flags = NSF_CONDASSERT;
                            (*top_nest).options = options & PARSE_TRACKED_OPTIONS;
                            (*top_nest).xoptions = xoptions & PARSE_TRACKED_EXTRA_OPTIONS;
                        }
                    } else if go_paren == PT::DefineName {
                        // DefineName increments nest_depth internally (in dispatch).
                    }
                }

                // Handle FROM_PERL_EXTENDED_CLASS: (?[ ... ])
                if !handled_here && go_paren == PT::Done && paren_terminator == 0xFFFFFFFF {
                    // sentinel: (?[ case sets paren_terminator = 0xFFFFFFFF and paren_i holds c
                    class_mode_state = CLASS_MODE_PERL_EXT;
                    c = paren_i as u32;
                    match process_class(
                        &mut parsed_pattern, &mut ptr, ptrend, options, xoptions, utf,
                        &mut inescq, &mut okquantifier, has_lookbehind, &mut errorcode, cb,
                        class_mode_state, c,
                    ) {
                        Ctl::Fall => {}
                        _ => do_fail!(),
                    }
                }
            }

            CHAR_VERTICAL_LINE => {
                if !top_nest.is_null()
                    && (*top_nest).nest_depth == nest_depth
                    && ((*top_nest).flags & NSF_RESET) != 0
                {
                    if (*cb).bracount > (*top_nest).max_group as u32 {
                        (*top_nest).max_group = (*cb).bracount as u16;
                    }
                    (*cb).bracount = (*top_nest).reset_group as u32;
                }
                wr!(parsed_pattern, META_ALT);
            }

            CHAR_RIGHT_PARENTHESIS => {
                okquantifier = true;
                if !top_nest.is_null() && (*top_nest).nest_depth == nest_depth {
                    options = (options & !PARSE_TRACKED_OPTIONS) | (*top_nest).options;
                    xoptions = (xoptions & !PARSE_TRACKED_EXTRA_OPTIONS) | (*top_nest).xoptions;
                    if ((*top_nest).flags & NSF_RESET) != 0
                        && (*top_nest).max_group as u32 > (*cb).bracount
                    {
                        (*cb).bracount = (*top_nest).max_group as u32;
                    }
                    if ((*top_nest).flags & NSF_CONDASSERT) != 0 {
                        okquantifier = false;
                    }

                    if ((*top_nest).flags & NSF_ATOMICSR) != 0 {
                        wr!(parsed_pattern, META_KET);
                    }

                    if top_nest == workspace_base {
                        top_nest = ptr::null_mut();
                    } else {
                        top_nest = top_nest.sub(1);
                    }
                }
                if nest_depth == 0 {
                    errorcode = ERR22;
                    do_fail!();
                }
                nest_depth -= 1;
                wr!(parsed_pattern, META_KET);
            }

            _ => {
                // default: literal
                wr!(parsed_pattern, c);
                okquantifier = true;
            }
        }

        // CHECK_QUANTIFIER block
        if go_check_quantifier {
            if !prev_okquantifier {
                errorcode = ERR9;
                do_fail!();
            }

            if *prev_parsed_item == META_ACCEPT {
                let mut p = parsed_pattern.sub(1);
                while p >= verbstartptr {
                    *p.add(1) = *p.add(0);
                    if p == verbstartptr {
                        break;
                    }
                    p = p.sub(1);
                }
                *verbstartptr = META_NOCAPTURE;
                *parsed_pattern.add(1) = META_KET;
                parsed_pattern = parsed_pattern.add(2);
            }

            wr!(parsed_pattern, meta_quantifier);
            if c == CHAR_LEFT_CURLY_BRACKET {
                wr!(parsed_pattern, min_repeat);
                wr!(parsed_pattern, max_repeat);
            }
        }
    } // 'mainloop

    if inverbname && ptr >= ptrend {
        errorcode = ERR60;
        do_fail!();
    }

    parsed_end(
        ptr, options, xoptions, utf, auto_callout, nest_depth, inverbname, ptrend,
        &mut previous_callout, parsed_pattern, parsed_pattern_end, cb, start_pattern,
    )
}

// PARSED_END finalization.
unsafe fn parsed_end(
    mut ptr: PCRE2_SPTR,
    _options: u32,
    xoptions: u32,
    _utf: bool,
    auto_callout: bool,
    nest_depth: u16,
    _inverbname: bool,
    _ptrend: PCRE2_SPTR,
    previous_callout: *mut *mut u32,
    mut parsed_pattern: *mut u32,
    parsed_pattern_end: *mut u32,
    cb: *mut compile_block,
    start_pattern: PCRE2_SPTR,
) -> c_int {
    let mut errorcode: c_int;

    parsed_pattern =
        manage_callouts(ptr, previous_callout, auto_callout, parsed_pattern, cb);

    if (xoptions & PCRE2_EXTRA_MATCH_LINE) != 0 {
        wr!(parsed_pattern, META_KET);
        wr!(parsed_pattern, META_DOLLAR);
    } else if (xoptions & PCRE2_EXTRA_MATCH_WORD) != 0 {
        wr!(parsed_pattern, META_KET);
        wr!(parsed_pattern, META_ESCAPE + ESC_b as u32);
    }

    let _ = parsed_pattern_end;

    *parsed_pattern = META_END;
    if nest_depth == 0 {
        return 0;
    }

    // UNCLOSED_PARENTHESIS
    errorcode = ERR14;
    (*cb).erroroffset = (ptr as usize - start_pattern as usize) as PCRE2_SIZE;
    let _ = &mut ptr;
    errorcode
}

#[inline]
fn MAX_MARK() -> u32 {
    (1u32 << 8) - 1
}

// IS_NEWLINE(ptr) with NLBLOCK = cb.
unsafe fn is_newline_at(p: PCRE2_SPTR, cb: *mut compile_block, utf: bool) -> bool {
    if (*cb).nltype != NLTYPE_FIXED {
        p < (*cb).end_pattern
            && crate::pcre2_newline::_pcre2_is_newline_8(
                p,
                (*cb).nltype,
                (*cb).end_pattern,
                &mut (*cb).nllen,
                utf as BOOL,
            ) != FALSE
    } else {
        (p as usize) <= ((*cb).end_pattern as usize - (*cb).nllen as usize)
            && *p == (*cb).nl[0]
            && ((*cb).nllen == 1 || *p.add(1) == (*cb).nl[1])
    }
}

enum ParenRes {
    Done,
    Fail,
    FailForward,
    Unclosed,
}

// Shared "label" dispatch (SET_RECURSION, RECURSE_BY_NAME, READ_RECURSION_ARGUMENTS,
// DEFINE_NAME, ATOMIC_GROUP, look-ahead labels, POST_LOOKBEHIND, POST_ASSERTION).
// The caller handles the POST_ASSERTION nest-depth/cond-assert tracking.
unsafe fn paren_dispatch(
    start: PT,
    mut i: c_int,
    mut terminator: u32,
    mut offset: PCRE2_SIZE,
    ptr: &mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    utf: bool,
    pp: &mut *mut u32,
    errorcode: &mut c_int,
    cb: *mut compile_block,
    start_pattern: PCRE2_SPTR,
    okquantifier: &mut bool,
    nest_depth: &mut u16,
    options: u32,
    has_lookbehind: *mut BOOL,
) -> ParenRes {
    let mut parsed_pattern = *pp;
    let mut label = start;

    macro_rules! save {
        () => {{
            *pp = parsed_pattern;
        }};
    }

    // Chain of fall-through labels.
    loop {
        match label {
            PT::RecursionByNumber => {
                // read_number then SET_RECURSION
                if read_number(
                    ptr,
                    ptrend,
                    if IS_DIGIT(**ptr as u32) { -1 } else { (*cb).bracount as i32 },
                    MAX_GROUP_NUMBER,
                    ERR61 as u32,
                    &mut i,
                    errorcode,
                ) == FALSE
                {
                    save!();
                    return ParenRes::Fail;
                }
                terminator = CHAR_NUL;
                label = PT::SetRecursion;
            }

            PT::SetRecursion => {
                wr!(parsed_pattern, META_RECURSE | i as u32);
                offset = (*ptr as usize - start_pattern as usize) as PCRE2_SIZE;
                label = PT::ReadRecursionArguments;
            }

            PT::RecurseByName => {
                let mut name: PCRE2_SPTR = ptr::null();
                let mut namelen: u32 = 0;
                if read_name(ptr, ptrend, utf, 0, &mut offset, &mut name, &mut namelen, errorcode, cb)
                    == FALSE
                {
                    save!();
                    return ParenRes::Fail;
                }
                wr!(parsed_pattern, META_RECURSE_BYNAME);
                wr!(parsed_pattern, namelen);
                terminator = CHAR_NUL;
                label = PT::ReadRecursionArguments;
            }

            PT::ReadRecursionArguments => {
                PUTOFFSET!(offset, parsed_pattern);
                *okquantifier = true;

                if terminator != CHAR_NUL {
                    save!();
                    return ParenRes::Done;
                }

                if *ptr < ptrend && **ptr as u32 == CHAR_LEFT_PARENTHESIS {
                    parsed_pattern = parse_capture_list(
                        ptr, ptrend, utf, parsed_pattern, offset, errorcode, cb,
                    );
                    if parsed_pattern.is_null() {
                        // parsed_pattern is NULL; nothing more to save.
                        return ParenRes::Fail;
                    }
                }

                if *ptr >= ptrend || **ptr as u32 != CHAR_RIGHT_PARENTHESIS {
                    save!();
                    return ParenRes::Unclosed;
                }

                *ptr = (*ptr).add(1);
                save!();
                return ParenRes::Done;
            }

            PT::DefineName => {
                let mut name: PCRE2_SPTR = ptr::null();
                let mut namelen: u32 = 0;
                if read_name(ptr, ptrend, utf, terminator, &mut offset, &mut name, &mut namelen, errorcode, cb)
                    == FALSE
                {
                    save!();
                    return ParenRes::Fail;
                }

                if (*cb).bracount >= MAX_GROUP_NUMBER {
                    *errorcode = ERR97;
                    save!();
                    return ParenRes::Fail;
                }
                (*cb).bracount += 1;
                wr!(parsed_pattern, META_CAPTURE | (*cb).bracount);
                *nest_depth += 1;

                if (*cb).names_found as u32 >= MAX_NAME_COUNT {
                    *errorcode = ERR49;
                    save!();
                    return ParenRes::Fail;
                }

                if namelen + IMM2_SIZE as u32 + 1 > (*cb).name_entry_size as u32 {
                    (*cb).name_entry_size = (namelen + IMM2_SIZE as u32 + 1) as u16;
                }

                let mut is_dupname = false;
                let mut hash =
                    crate::pcre2_compile_cgroup::_pcre2_compile_get_hash_from_name8(name, namelen);
                let mut ng = (*cb).named_groups;
                let mut idx: c_int = 0;
                let mut broke = false;
                while idx < (*cb).names_found as c_int {
                    if namelen == (*ng).length as u32
                        && hash == NAMED_GROUP_GET_HASH(ng)
                        && _pcre2_strncmp(name, (*ng).name, namelen as PCRE2_SIZE) == 0
                    {
                        if (*ng).number == (*cb).bracount {
                            broke = true;
                            break;
                        }
                        if (options & PCRE2_DUPNAMES) == 0 {
                            *errorcode = ERR43;
                            save!();
                            return ParenRes::Fail;
                        }

                        (*ng).hash_dup |= NAMED_GROUP_IS_DUPNAME;
                        is_dupname = true;
                        (*cb).dupnames = TRUE;

                        name = (*ng).name;
                        namelen = 0;

                        while idx < (*cb).names_found as c_int {
                            if (*ng).name == name && (*ng).number == (*cb).bracount {
                                break;
                            }
                            idx += 1;
                            ng = ng.add(1);
                        }
                        broke = true;
                        break;
                    } else if (*ng).number == (*cb).bracount {
                        *errorcode = ERR65;
                        save!();
                        return ParenRes::Fail;
                    }
                    idx += 1;
                    ng = ng.add(1);
                }

                if broke && idx < (*cb).names_found as c_int {
                    save!();
                    return ParenRes::Done;
                }

                if (*cb).names_found as u32 >= (*cb).named_group_list_size {
                    let newsize = (*cb).named_group_list_size * 2;
                    let newspace = ((*(*cb).cx).memctl.malloc.unwrap())(
                        newsize as usize * core::mem::size_of::<named_group>(),
                        (*(*cb).cx).memctl.memory_data,
                    ) as *mut named_group;
                    if newspace.is_null() {
                        *errorcode = ERR21;
                        save!();
                        return ParenRes::Fail;
                    }

                    memcpy(
                        newspace as *mut c_void,
                        (*cb).named_groups as *const c_void,
                        (*cb).named_group_list_size as usize * core::mem::size_of::<named_group>(),
                    );
                    if (*cb).named_group_list_size > NAMED_GROUP_LIST_SIZE {
                        ((*(*cb).cx).memctl.free.unwrap())(
                            (*cb).named_groups as *mut c_void,
                            (*(*cb).cx).memctl.memory_data,
                        );
                    }
                    (*cb).named_groups = newspace;
                    (*cb).named_group_list_size = newsize;
                }

                if is_dupname {
                    hash |= NAMED_GROUP_IS_DUPNAME;
                }

                let nf = (*cb).names_found as usize;
                (*(*cb).named_groups.add(nf)).name = name;
                (*(*cb).named_groups.add(nf)).length = namelen as u16;
                (*(*cb).named_groups.add(nf)).number = (*cb).bracount;
                (*(*cb).named_groups.add(nf)).hash_dup = hash;
                (*cb).names_found += 1;
                save!();
                return ParenRes::Done;
            }

            PT::AtomicGroup => {
                wr!(parsed_pattern, META_ATOMIC);
                *nest_depth += 1;
                *ptr = (*ptr).add(1);
                save!();
                return ParenRes::Done;
            }

            PT::PositiveLookAhead => {
                wr!(parsed_pattern, META_LOOKAHEAD);
                *ptr = (*ptr).add(1);
                save!();
                return ParenRes::Done; // POST_ASSERTION handled by caller
            }

            PT::PositiveNonatomicLookAhead => {
                wr!(parsed_pattern, META_LOOKAHEAD_NA);
                *ptr = (*ptr).add(1);
                save!();
                return ParenRes::Done;
            }

            PT::NegativeLookAhead => {
                wr!(parsed_pattern, META_LOOKAHEADNOT);
                *ptr = (*ptr).add(1);
                save!();
                return ParenRes::Done;
            }

            PT::PostLookbehind => {
                *has_lookbehind = TRUE;
                offset = (*ptr as usize - start_pattern as usize - 2) as PCRE2_SIZE;
                PUTOFFSET!(offset, parsed_pattern);
                *ptr = (*ptr).add(2);
                save!();
                return ParenRes::Done; // fall to POST_ASSERTION in caller
            }

            PT::PostAssertion => {
                // Nothing here; caller does the nest tracking.
                save!();
                return ParenRes::Done;
            }

            PT::Done => {
                save!();
                return ParenRes::Done;
            }
        }
    }
}

// The big (? switch. On entry *ptr points at the char after "(?".
// Returns the disposition; sets *go_paren for shared-label jumps and the
// paren_* out-params used by paren_dispatch.
unsafe fn parse_question_paren(
    ptr: &mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    utf: bool,
    options: &mut u32,
    xoptions: &mut u32,
    pp: &mut *mut u32,
    errorcode: &mut c_int,
    cb: *mut compile_block,
    start_pattern: PCRE2_SPTR,
    nest_depth: &mut u16,
    top_nest: &mut *mut nest_save,
    end_nests: *mut nest_save,
    workspace_base: *mut nest_save,
    expect_cond_assert: &mut c_int,
    okquantifier: &mut bool,
    has_lookbehind: *mut BOOL,
    prev_expect_cond_assert: c_int,
    go_paren: &mut PT,
    paren_i: &mut c_int,
    paren_terminator: &mut u32,
    paren_offset: &mut PCRE2_SIZE,
    previous_callout: &mut *mut u32,
    after_manual_callout: &mut c_int,
) -> ParenRes {
    let mut parsed_pattern = *pp;
    let mut i: c_int = 0;
    let mut offset: PCRE2_SIZE = 0;
    let mut terminator: u32;
    let mut name: PCRE2_SPTR = ptr::null();
    let mut namelen: u32 = 0;

    macro_rules! save {
        () => {{
            *pp = parsed_pattern;
        }};
    }

    match **ptr as u32 {
        CHAR_P => {
            *ptr = (*ptr).add(1);
            if *ptr >= ptrend {
                save!();
                return ParenRes::Unclosed;
            }

            if **ptr as u32 == CHAR_LESS_THAN_SIGN {
                *paren_terminator = CHAR_GREATER_THAN_SIGN;
                *go_paren = PT::DefineName;
                save!();
                return ParenRes::Done;
            }

            if **ptr as u32 == CHAR_GREATER_THAN_SIGN {
                *go_paren = PT::RecurseByName;
                save!();
                return ParenRes::Done;
            }

            if **ptr as u32 != CHAR_EQUALS_SIGN {
                *errorcode = ERR41;
                save!();
                return ParenRes::FailForward;
            }
            if read_name(ptr, ptrend, utf, CHAR_RIGHT_PARENTHESIS, &mut offset, &mut name, &mut namelen, errorcode, cb) == FALSE {
                save!();
                return ParenRes::Fail;
            }
            wr!(parsed_pattern, META_BACKREF_BYNAME);
            wr!(parsed_pattern, namelen);
            PUTOFFSET!(offset, parsed_pattern);
            *okquantifier = true;
            save!();
            return ParenRes::Done;
        }

        CHAR_R => {
            *paren_i = 0;
            *ptr = (*ptr).add(1);
            if *ptr >= ptrend
                || (**ptr as u32 != CHAR_RIGHT_PARENTHESIS
                    && **ptr as u32 != CHAR_LEFT_PARENTHESIS)
            {
                *errorcode = ERR58;
                save!();
                return ParenRes::Fail;
            }
            *paren_terminator = CHAR_NUL;
            *go_paren = PT::SetRecursion;
            save!();
            return ParenRes::Done;
        }

        CHAR_PLUS => {
            if ptr.add(1) >= ptrend {
                *ptr = (*ptr).add(1);
                save!();
                return ParenRes::Unclosed;
            }
            if !IS_DIGIT(*ptr.add(1) as u32) {
                *errorcode = ERR29;
                *ptr = (*ptr).add(1);
                save!();
                return ParenRes::FailForward;
            }
            // fall through to RECURSION_BYNUMBER
            *go_paren = PT::RecursionByNumber;
            save!();
            return ParenRes::Done;
        }

        CHAR_0 | CHAR_1 | CHAR_2 | CHAR_3 | CHAR_4 | CHAR_5 | CHAR_6 | CHAR_7 | CHAR_8
        | CHAR_9 => {
            *go_paren = PT::RecursionByNumber;
            save!();
            return ParenRes::Done;
        }

        CHAR_AMPERSAND => {
            *go_paren = PT::RecurseByName;
            save!();
            return ParenRes::Done;
        }

        CHAR_C => {
            if (*xoptions & PCRE2_EXTRA_NEVER_CALLOUT) != 0 {
                *ptr = (*ptr).add(1);
                *errorcode = ERR103;
                save!();
                return ParenRes::Fail;
            }

            *ptr = (*ptr).add(1);
            if *ptr >= ptrend {
                save!();
                return ParenRes::Unclosed;
            }

            *expect_cond_assert = prev_expect_cond_assert - 1;

            if !(*previous_callout).is_null()
                && (*options & PCRE2_AUTO_CALLOUT) != 0
                && *previous_callout == parsed_pattern.sub(4)
                && *parsed_pattern.sub(1) == 255
            {
                parsed_pattern = *previous_callout;
            }

            *previous_callout = parsed_pattern;
            *after_manual_callout = 1;

            if **ptr as u32 != CHAR_RIGHT_PARENTHESIS && !IS_DIGIT(**ptr as u32) {
                let calloutlength: PCRE2_SIZE;
                let startptr = *ptr;

                let mut delimiter: u32 = 0;
                i = 0;
                while _pcre2_callout_start_delims_8[i as usize] != 0 {
                    if **ptr as u32 == _pcre2_callout_start_delims_8[i as usize] {
                        delimiter = _pcre2_callout_end_delims_8[i as usize];
                        break;
                    }
                    i += 1;
                }
                if delimiter == 0 {
                    *errorcode = ERR82;
                    save!();
                    return ParenRes::FailForward;
                }

                *parsed_pattern = META_CALLOUT_STRING;
                parsed_pattern = parsed_pattern.add(3);

                loop {
                    *ptr = (*ptr).add(1);
                    if *ptr >= ptrend {
                        *errorcode = ERR81;
                        *ptr = startptr;
                        save!();
                        return ParenRes::Fail;
                    }
                    if **ptr as u32 == delimiter && {
                        *ptr = (*ptr).add(1);
                        *ptr >= ptrend || **ptr as u32 != delimiter
                    } {
                        break;
                    }
                }

                calloutlength = (*ptr as usize - startptr as usize) as PCRE2_SIZE;
                if calloutlength > UINT32_MAX as PCRE2_SIZE {
                    *errorcode = ERR72;
                    save!();
                    return ParenRes::Fail;
                }
                wr!(parsed_pattern, calloutlength as u32);
                offset = (startptr as usize - start_pattern as usize) as PCRE2_SIZE;
                PUTOFFSET!(offset, parsed_pattern);
            } else {
                let mut n: c_int = 0;
                *parsed_pattern = META_CALLOUT_NUMBER;
                parsed_pattern = parsed_pattern.add(3);
                while *ptr < ptrend && IS_DIGIT(**ptr as u32) {
                    n = n * 10 + (**ptr as i32 - CHAR_0 as i32);
                    *ptr = (*ptr).add(1);
                    if n > 255 {
                        *errorcode = ERR38;
                        save!();
                        return ParenRes::Fail;
                    }
                }
                wr!(parsed_pattern, n as u32);
            }

            if *ptr >= ptrend || **ptr as u32 != CHAR_RIGHT_PARENTHESIS {
                *errorcode = ERR39;
                save!();
                return ParenRes::Fail;
            }
            *ptr = (*ptr).add(1);

            *(*previous_callout).add(1) = (*ptr as usize - start_pattern as usize) as u32;
            *(*previous_callout).add(2) = 0;
            save!();
            return ParenRes::Done;
        }

        CHAR_LEFT_PARENTHESIS => {
            // Conditional group
            *ptr = (*ptr).add(1);
            if *ptr >= ptrend {
                save!();
                return ParenRes::Unclosed;
            }
            *nest_depth += 1;

            if **ptr as u32 == CHAR_QUESTION_MARK || **ptr as u32 == CHAR_ASTERISK {
                wr!(parsed_pattern, META_COND_ASSERT);
                *ptr = (*ptr).sub(1);
                *expect_cond_assert = 2;
                save!();
                return ParenRes::Done;
            }

            if read_number(ptr, ptrend, (*cb).bracount as i32, MAX_GROUP_NUMBER, ERR61 as u32, &mut i, errorcode) != FALSE {
                if i <= 0 {
                    *errorcode = ERR15;
                    save!();
                    return ParenRes::Fail;
                }
                wr!(parsed_pattern, META_COND_NUMBER);
                offset = (*ptr as usize - start_pattern as usize - 2) as PCRE2_SIZE;
                PUTOFFSET!(offset, parsed_pattern);
                wr!(parsed_pattern, i as u32);
            } else if *errorcode != 0 {
                save!();
                return ParenRes::Fail;
            } else if (ptrend as usize - *ptr as usize) >= 10
                && _pcre2_strncmp_c8(*ptr, b"VERSION".as_ptr() as *const c_char, 7) == 0
                && *ptr.add(7) as u32 != CHAR_RIGHT_PARENTHESIS
            {
                let mut ge: u32 = 0;
                let mut major: c_int = 0;
                let mut minor: c_int = 0;

                *ptr = (*ptr).add(7);
                if **ptr as u32 == CHAR_GREATER_THAN_SIGN {
                    ge = 1;
                    *ptr = (*ptr).add(1);
                }

                if **ptr as u32 != CHAR_EQUALS_SIGN || {
                    *ptr = (*ptr).add(1);
                    !IS_DIGIT(**ptr as u32)
                } {
                    *errorcode = ERR79;
                    if ge == 0 {
                        save!();
                        return ParenRes::FailForward;
                    }
                    save!();
                    return ParenRes::Fail;
                }

                if read_number(ptr, ptrend, -1, 1000, ERR79 as u32, &mut major, errorcode) == FALSE {
                    save!();
                    return ParenRes::Fail;
                }

                if *ptr < ptrend && **ptr as u32 == CHAR_DOT {
                    *ptr = (*ptr).add(1);
                    if *ptr >= ptrend || !IS_DIGIT(**ptr as u32) {
                        *errorcode = ERR79;
                        if *ptr < ptrend {
                            save!();
                            return ParenRes::FailForward;
                        }
                        save!();
                        return ParenRes::Fail;
                    }
                    if read_number(ptr, ptrend, -1, 1000, ERR79 as u32, &mut minor, errorcode) == FALSE {
                        save!();
                        return ParenRes::Fail;
                    }
                }
                if *ptr >= ptrend || **ptr as u32 != CHAR_RIGHT_PARENTHESIS {
                    *errorcode = ERR79;
                    if *ptr < ptrend {
                        save!();
                        return ParenRes::FailForward;
                    }
                    save!();
                    return ParenRes::Fail;
                }

                wr!(parsed_pattern, META_COND_VERSION);
                wr!(parsed_pattern, ge);
                wr!(parsed_pattern, major as u32);
                wr!(parsed_pattern, minor as u32);
            } else {
                let mut was_r_ampersand = false;

                if **ptr as u32 == CHAR_R
                    && (ptrend as usize - *ptr as usize) > 1
                    && *ptr.add(1) as u32 == CHAR_AMPERSAND
                {
                    terminator = CHAR_RIGHT_PARENTHESIS;
                    was_r_ampersand = true;
                    *ptr = (*ptr).add(1);
                } else if **ptr as u32 == CHAR_LESS_THAN_SIGN {
                    terminator = CHAR_GREATER_THAN_SIGN;
                } else if **ptr as u32 == CHAR_APOSTROPHE {
                    terminator = CHAR_APOSTROPHE;
                } else {
                    terminator = CHAR_RIGHT_PARENTHESIS;
                    *ptr = (*ptr).sub(1);
                }

                if read_name(ptr, ptrend, utf, terminator, &mut offset, &mut name, &mut namelen, errorcode, cb) == FALSE {
                    save!();
                    return ParenRes::Fail;
                }

                if was_r_ampersand {
                    *parsed_pattern = META_COND_RNAME;
                    *ptr = (*ptr).sub(1);
                } else if terminator == CHAR_RIGHT_PARENTHESIS {
                    if namelen == 6
                        && _pcre2_strncmp_c8(name, b"DEFINE".as_ptr() as *const c_char, 6) == 0
                    {
                        *parsed_pattern = META_COND_DEFINE;
                    } else {
                        i = 1;
                        while i < namelen as c_int {
                            if !IS_DIGIT(*name.add(i as usize) as u32) {
                                break;
                            }
                            i += 1;
                        }
                        *parsed_pattern = if *name as u32 == CHAR_R && i >= namelen as c_int {
                            META_COND_RNUMBER
                        } else {
                            META_COND_NAME
                        };
                    }
                    *ptr = (*ptr).sub(1);
                } else {
                    *parsed_pattern = META_COND_NAME;
                }

                let was_define = *parsed_pattern == META_COND_DEFINE;
                parsed_pattern = parsed_pattern.add(1);
                if !was_define {
                    wr!(parsed_pattern, namelen);
                }
                PUTOFFSET!(offset, parsed_pattern);
            }

            if *ptr >= ptrend || **ptr as u32 != CHAR_RIGHT_PARENTHESIS {
                *errorcode = ERR24;
                save!();
                return ParenRes::Fail;
            }
            *ptr = (*ptr).add(1);
            save!();
            return ParenRes::Done;
        }

        CHAR_GREATER_THAN_SIGN => {
            *go_paren = PT::AtomicGroup;
            save!();
            return ParenRes::Done;
        }

        CHAR_EQUALS_SIGN => {
            *go_paren = PT::PositiveLookAhead;
            save!();
            return ParenRes::Done;
        }

        CHAR_ASTERISK => {
            *go_paren = PT::PositiveNonatomicLookAhead;
            save!();
            return ParenRes::Done;
        }

        CHAR_EXCLAMATION_MARK => {
            *go_paren = PT::NegativeLookAhead;
            save!();
            return ParenRes::Done;
        }

        CHAR_LESS_THAN_SIGN => {
            if (ptrend as usize - *ptr as usize) <= 1
                || (*ptr.add(1) as u32 != CHAR_EQUALS_SIGN
                    && *ptr.add(1) as u32 != CHAR_EXCLAMATION_MARK
                    && *ptr.add(1) as u32 != CHAR_ASTERISK)
            {
                *paren_terminator = CHAR_GREATER_THAN_SIGN;
                *go_paren = PT::DefineName;
                save!();
                return ParenRes::Done;
            }
            wr!(
                parsed_pattern,
                if *ptr.add(1) as u32 == CHAR_EQUALS_SIGN {
                    META_LOOKBEHIND
                } else if *ptr.add(1) as u32 == CHAR_EXCLAMATION_MARK {
                    META_LOOKBEHINDNOT
                } else {
                    META_LOOKBEHIND_NA
                }
            );
            *has_lookbehind = TRUE;
            offset = (*ptr as usize - start_pattern as usize - 2) as PCRE2_SIZE;
            PUTOFFSET!(offset, parsed_pattern);
            *ptr = (*ptr).add(2);
            // fall through to POST_ASSERTION
            *go_paren = PT::PostAssertion;
            save!();
            return ParenRes::Done;
        }

        CHAR_APOSTROPHE => {
            *paren_terminator = CHAR_APOSTROPHE;
            *go_paren = PT::DefineName;
            save!();
            return ParenRes::Done;
        }

        CHAR_LEFT_SQUARE_BRACKET => {
            // (?[...]) : signal FROM_PERL_EXTENDED_CLASS to caller.
            let cbyte = **ptr as i32; // c = *ptr++ (the '[')
            *ptr = (*ptr).add(1);
            *paren_i = cbyte;
            *paren_terminator = 0xFFFFFFFF; // sentinel
            *go_paren = PT::Done;
            save!();
            return ParenRes::Done;
        }

        _ => {
            // default: (?- + digit => recursion, else option setting.
            if **ptr as u32 == CHAR_MINUS
                && (ptrend as usize - *ptr as usize) > 1
                && IS_DIGIT(*ptr.add(1) as u32)
            {
                *go_paren = PT::RecursionByNumber;
                save!();
                return ParenRes::Done;
            }

            *nest_depth += 1;
            if top_nest.is_null() {
                *top_nest = workspace_base;
            } else {
                *top_nest = top_nest.add(1);
                if *top_nest >= end_nests {
                    *errorcode = ERR84;
                    save!();
                    return ParenRes::Fail;
                }
            }
            (**top_nest).nest_depth = *nest_depth;
            (**top_nest).flags = 0;
            (**top_nest).options = *options & PARSE_TRACKED_OPTIONS;
            (**top_nest).xoptions = *xoptions & PARSE_TRACKED_EXTRA_OPTIONS;

            if **ptr as u32 == CHAR_VERTICAL_LINE {
                (**top_nest).reset_group = (*cb).bracount as u16;
                (**top_nest).max_group = (*cb).bracount as u16;
                (**top_nest).flags |= NSF_RESET;
                (*cb).external_flags |= PCRE2_DUPCAPUSED;
                wr!(parsed_pattern, META_NOCAPTURE);
                *ptr = (*ptr).add(1);
            } else {
                let mut hyphenok = true;
                let oldoptions = *options;
                let oldxoptions = *xoptions;
                let mut set: u32 = 0;
                let mut unset: u32 = 0;
                let mut xset: u32 = 0;
                let mut xunset: u32 = 0;
                let mut optset_is_unset = false; // false => &set, true => &unset
                let mut xoptset_is_unset = false;

                (**top_nest).reset_group = 0;
                (**top_nest).max_group = 0;

                if *ptr < ptrend && **ptr as u32 == CHAR_CIRCUMFLEX_ACCENT {
                    *options &= !(PCRE2_CASELESS
                        | PCRE2_MULTILINE
                        | PCRE2_NO_AUTO_CAPTURE
                        | PCRE2_DOTALL
                        | PCRE2_EXTENDED
                        | PCRE2_EXTENDED_MORE);
                    *xoptions &= !PCRE2_EXTRA_CASELESS_RESTRICT;
                    hyphenok = false;
                    *ptr = (*ptr).add(1);
                }

                while *ptr < ptrend
                    && **ptr as u32 != CHAR_RIGHT_PARENTHESIS
                    && **ptr as u32 != CHAR_COLON
                {
                    let ch = **ptr as u32;
                    *ptr = (*ptr).add(1);
                    macro_rules! optset {
                        ($v:expr) => {{
                            if optset_is_unset { unset |= $v; } else { set |= $v; }
                        }};
                    }
                    macro_rules! xoptset {
                        ($v:expr) => {{
                            if xoptset_is_unset { xunset |= $v; } else { xset |= $v; }
                        }};
                    }
                    match ch {
                        CHAR_MINUS => {
                            if !hyphenok {
                                *errorcode = ERR94;
                                save!();
                                return ParenRes::Fail;
                            }
                            optset_is_unset = true;
                            xoptset_is_unset = true;
                            hyphenok = false;
                        }
                        CHAR_a => {
                            if *ptr < ptrend {
                                if **ptr as u32 == CHAR_D {
                                    xoptset!(PCRE2_EXTRA_ASCII_BSD);
                                    *ptr = (*ptr).add(1);
                                    continue;
                                }
                                if **ptr as u32 == CHAR_P {
                                    xoptset!(PCRE2_EXTRA_ASCII_POSIX | PCRE2_EXTRA_ASCII_DIGIT);
                                    *ptr = (*ptr).add(1);
                                    continue;
                                }
                                if **ptr as u32 == CHAR_S {
                                    xoptset!(PCRE2_EXTRA_ASCII_BSS);
                                    *ptr = (*ptr).add(1);
                                    continue;
                                }
                                if **ptr as u32 == CHAR_T {
                                    xoptset!(PCRE2_EXTRA_ASCII_DIGIT);
                                    *ptr = (*ptr).add(1);
                                    continue;
                                }
                                if **ptr as u32 == CHAR_W {
                                    xoptset!(PCRE2_EXTRA_ASCII_BSW);
                                    *ptr = (*ptr).add(1);
                                    continue;
                                }
                            }
                            xoptset!(
                                PCRE2_EXTRA_ASCII_BSD
                                    | PCRE2_EXTRA_ASCII_BSS
                                    | PCRE2_EXTRA_ASCII_BSW
                                    | PCRE2_EXTRA_ASCII_DIGIT
                                    | PCRE2_EXTRA_ASCII_POSIX
                            );
                        }
                        CHAR_J => {
                            optset!(PCRE2_DUPNAMES);
                            (*cb).external_flags |= PCRE2_JCHANGED;
                        }
                        CHAR_i => optset!(PCRE2_CASELESS),
                        CHAR_m => optset!(PCRE2_MULTILINE),
                        CHAR_n => optset!(PCRE2_NO_AUTO_CAPTURE),
                        CHAR_r => xoptset!(PCRE2_EXTRA_CASELESS_RESTRICT),
                        CHAR_s => optset!(PCRE2_DOTALL),
                        CHAR_U => optset!(PCRE2_UNGREEDY),
                        CHAR_x => {
                            optset!(PCRE2_EXTENDED);
                            if *ptr < ptrend && **ptr as u32 == CHAR_x {
                                optset!(PCRE2_EXTENDED_MORE);
                                *ptr = (*ptr).add(1);
                            }
                        }
                        _ => {
                            *errorcode = ERR11;
                            save!();
                            return ParenRes::Fail;
                        }
                    }
                }

                if (set & (PCRE2_EXTENDED | PCRE2_EXTENDED_MORE)) == PCRE2_EXTENDED
                    || (unset & PCRE2_EXTENDED) != 0
                {
                    unset |= PCRE2_EXTENDED_MORE;
                }

                *options = (*options | set) & (!unset);
                *xoptions = (*xoptions | xset) & (!xunset);

                if *ptr >= ptrend {
                    save!();
                    return ParenRes::Unclosed;
                }
                let ended = **ptr as u32 == CHAR_RIGHT_PARENTHESIS;
                *ptr = (*ptr).add(1);
                if ended {
                    *nest_depth -= 1;
                    if *top_nest > workspace_base
                        && (*top_nest.offset(-1)).nest_depth == *nest_depth
                    {
                        *top_nest = top_nest.offset(-1);
                    } else {
                        (**top_nest).nest_depth = *nest_depth;
                    }
                } else {
                    wr!(parsed_pattern, META_NOCAPTURE);
                }

                if *options != oldoptions || *xoptions != oldxoptions {
                    wr!(parsed_pattern, META_OPTIONS);
                    wr!(parsed_pattern, *options);
                    wr!(parsed_pattern, *xoptions);
                }
            }
            save!();
            return ParenRes::Done;
        }
    }
}

// ---------------------------------------------------------------------------
// Compile one branch : compile_branch
// ---------------------------------------------------------------------------

unsafe fn compile_branch(
    optionsptr: *mut u32,
    xoptionsptr: *mut u32,
    codeptr: *mut *mut PCRE2_UCHAR,
    pptrptr: *mut *mut u32,
    errorcodeptr: *mut c_int,
    firstcuptr: *mut u32,
    firstcuflagsptr: *mut u32,
    reqcuptr: *mut u32,
    reqcuflagsptr: *mut u32,
    bcptr: *mut branch_chain,
    open_caps: *mut open_capitem,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
) -> c_int {
    let mut bravalue: c_int = 0;
    let mut okreturn: c_int = -1;
    let mut group_return: c_int = 0;
    let mut repeat_min: u32 = 0;
    let mut repeat_max: u32 = 0;
    let mut greedy_default: u32;
    let mut greedy_non_default: u32;
    let mut repeat_type: u32;
    let mut op_type: u32;
    let mut options = *optionsptr;
    let mut xoptions = *xoptionsptr;
    let mut firstcu: u32;
    let mut reqcu: u32;
    let mut zeroreqcu: u32;
    let mut zerofirstcu: u32;
    let mut pptr = *pptrptr;
    let mut meta: u32;
    let mut meta_arg: u32;
    let mut firstcuflags: u32;
    let mut reqcuflags: u32;
    let mut zeroreqcuflags: u32;
    let mut zerofirstcuflags: u32;
    let mut req_caseopt: u32;
    let mut reqvary: u32 = 0;
    let mut tempreqvary: u32 = 0;
    let mut offset: PCRE2_SIZE = 0;
    let mut length_prevgroup: PCRE2_SIZE = 0;
    let mut code = *codeptr;
    let mut last_code = code;
    let mut orig_code = code;
    let mut tempcode: *mut PCRE2_UCHAR;
    let mut previous: *mut PCRE2_UCHAR = ptr::null_mut();
    let mut op_previous: u8;
    let mut groupsetfirstcu = false;
    let mut had_accept = false;
    let mut matched_char = false;
    let mut previous_matched_char = false;
    let mut reset_caseful = false;

    let utf = (options & PCRE2_UTF) != 0;
    let ucp = (options & PCRE2_UCP) != 0;

    greedy_default = ((options & PCRE2_UNGREEDY) != 0) as u32;
    greedy_non_default = greedy_default ^ 1;

    firstcu = 0;
    reqcu = 0;
    zerofirstcu = 0;
    zeroreqcu = 0;
    firstcuflags = REQ_UNSET;
    reqcuflags = REQ_UNSET;
    zerofirstcuflags = REQ_UNSET;
    zeroreqcuflags = REQ_UNSET;

    req_caseopt = if (options & PCRE2_CASELESS) != 0 { REQ_CASELESS } else { 0 };

    loop {
        let mut note_group_empty: bool;
        let mut mclength: u32;
        let mut skipunits: u32;
        let mut subreqcu: u32 = 0;
        let mut subfirstcu: u32 = 0;
        let mut groupnumber: u32;
        let mut verbarglen: u32;
        let mut verbculen: u32;
        let mut subreqcuflags: u32 = 0;
        let mut subfirstcuflags: u32 = 0;
        let mut oc: *mut open_capitem;
        let mut mcbuffer: [PCRE2_UCHAR; 8] = [0; 8];

        meta = META_CODE(*pptr);
        meta_arg = META_DATA(*pptr);

        if !lengthptr.is_null() {
            if code >= (*cb).start_workspace.add((*cb).workspace_size) {
                *errorcodeptr = ERR52;
                (*cb).erroroffset = 0;
                return 0;
            }

            if code > (*cb).start_workspace.add((*cb).workspace_size - WORK_SIZE_SAFETY_MARGIN) {
                *errorcodeptr = ERR86;
                (*cb).erroroffset = 0;
                return 0;
            }

            if code < last_code {
                code = last_code;
            }

            if meta < META_ASTERISK || meta > META_MINMAX_QUERY {
                if OFLOW_MAX - *lengthptr < (code as usize - orig_code as usize) {
                    *errorcodeptr = ERR20;
                    (*cb).erroroffset = 0;
                    return 0;
                }
                *lengthptr += (code as usize - orig_code as usize) as PCRE2_SIZE;
                if *lengthptr > MAX_PATTERN_SIZE {
                    *errorcodeptr = ERR20;
                    (*cb).erroroffset = 0;
                    return 0;
                }
                code = orig_code;
            }

            last_code = code;
        }

        if meta < META_ASTERISK || meta > META_MINMAX_QUERY {
            previous = code;
            if matched_char && !had_accept {
                okreturn = 1;
            }
        }

        previous_matched_char = matched_char;
        matched_char = false;
        note_group_empty = false;
        skipunits = 0;

        // Control flags for shared labels within the switch.
        let mut goto_group_process = false;
        let mut goto_group_process_note_empty = false;
        let mut goto_normal_char = false;
        let mut goto_normal_char_set = false;
        let mut goto_class_caseless_char = false;
        let mut goto_handle_single_reference = false;
        let mut goto_handle_numerical_recursion = false;
        let mut goto_do_repeat = false;
        let mut goto_meta_escape = false;
        let mut normal_char_meta: u32 = 0;

        match meta {
            META_END | META_ALT | META_KET => {
                *firstcuptr = firstcu;
                *firstcuflagsptr = firstcuflags;
                *reqcuptr = reqcu;
                *reqcuflagsptr = reqcuflags;
                *codeptr = code;
                *pptrptr = pptr;
                return okreturn;
            }

            META_CIRCUMFLEX => {
                if (options & PCRE2_MULTILINE) != 0 {
                    if firstcuflags == REQ_UNSET {
                        zerofirstcuflags = REQ_NONE;
                        firstcuflags = REQ_NONE;
                    }
                    wr!(code, OP_CIRCM);
                } else {
                    wr!(code, OP_CIRC);
                }
            }

            META_DOLLAR => {
                wr!(code, if (options & PCRE2_MULTILINE) != 0 { OP_DOLLM } else { OP_DOLL });
            }

            META_DOT => {
                matched_char = true;
                if firstcuflags == REQ_UNSET {
                    firstcuflags = REQ_NONE;
                }
                zerofirstcu = firstcu;
                zerofirstcuflags = firstcuflags;
                zeroreqcu = reqcu;
                zeroreqcuflags = reqcuflags;
                wr!(code, if (options & PCRE2_DOTALL) != 0 { OP_ALLANY } else { OP_ANY });
            }

            META_CLASS_EMPTY | META_CLASS_EMPTY_NOT => {
                matched_char = true;
                if meta == META_CLASS_EMPTY_NOT {
                    wr!(code, OP_ALLANY);
                } else {
                    wr!(code, OP_CLASS);
                    memset(code as *mut c_void, 0, 32);
                    code = code.add(32);
                }
                if firstcuflags == REQ_UNSET {
                    firstcuflags = REQ_NONE;
                }
                zerofirstcu = firstcu;
                zerofirstcuflags = firstcuflags;
            }

            META_CLASS_NOT | META_CLASS => {
                matched_char = true;

                let mut do_class_end_processing = false;

                if (*pptr & CLASS_IS_ECLASS) != 0 {
                    if crate::pcre2_compile_class::_pcre2_compile_class_nested_8(
                        options, xoptions, &mut pptr, &mut code, errorcodeptr, cb, lengthptr,
                    ) == FALSE
                    {
                        return 0;
                    }
                    do_class_end_processing = true;
                }

                if !do_class_end_processing {
                    if *pptr.add(1) < META_END && *pptr.add(2) == META_CLASS_END {
                        let cc = *pptr.add(1);
                        pptr = pptr.add(2);
                        if meta == META_CLASS {
                            meta = cc;
                            // goto NORMAL_CHAR_SET
                            goto_normal_char_set = true;
                            normal_char_meta = meta;
                        } else {
                            // negative one-char class
                            zeroreqcu = reqcu;
                            zeroreqcuflags = reqcuflags;
                            if firstcuflags == REQ_UNSET {
                                firstcuflags = REQ_NONE;
                            }
                            zerofirstcu = firstcu;
                            zerofirstcuflags = firstcuflags;

                            let mut handled = false;
                            if (utf || ucp) && (options & PCRE2_CASELESS) != 0 {
                                let mut caseset: u32;
                                if (xoptions
                                    & (PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT))
                                    == PCRE2_EXTRA_TURKISH_CASING
                                    && UCD_ANY_I(cc)
                                {
                                    caseset = _pcre2_ucd_turkish_dotted_i_caseset_8
                                        + (if UCD_DOTTED_I(cc) { 0 } else { 3 });
                                } else {
                                    caseset = UCD_CASESET(cc);
                                    if caseset != 0
                                        && (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0
                                        && _pcre2_ucd_caseless_sets_8[caseset as usize] < 128
                                    {
                                        caseset = 0;
                                    }
                                }

                                if caseset != 0 {
                                    wr!(code, OP_NOTPROP);
                                    wr!(code, PT_CLIST as u8);
                                    wr!(code, caseset as u8);
                                    handled = true;
                                }
                            }

                            if !handled {
                                wr!(code, if (options & PCRE2_CASELESS) != 0 { OP_NOTI } else { OP_NOT });
                                code = code.add(putchar(cc, code, utf) as usize);
                            }
                        }
                    } else if meta == META_CLASS
                        && *pptr.add(1) < META_END
                        && *pptr.add(2) < META_END
                        && *pptr.add(3) == META_CLASS_END
                    {
                        let cc = *pptr.add(1);
                        let mut two_char_ok = false;
                        if (UCD_CASESET(cc) == 0
                            || ((xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0
                                && cc < 128
                                && *pptr.add(2) < 128))
                            && !((xoptions
                                & (PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT))
                                == PCRE2_EXTRA_TURKISH_CASING
                                && UCD_ANY_I(cc))
                        {
                            let d: u32;
                            if (utf || ucp) && cc > 127 {
                                d = UCD_OTHERCASE(cc);
                            } else {
                                d = *(*cb).fcc.add(cc as usize) as u32;
                            }

                            if cc != d && *pptr.add(2) == d {
                                pptr = pptr.add(3);
                                meta = cc;
                                if (options & PCRE2_CASELESS) == 0 {
                                    reset_caseful = true;
                                    options |= PCRE2_CASELESS;
                                    req_caseopt = REQ_CASELESS;
                                }
                                goto_class_caseless_char = true;
                                normal_char_meta = meta;
                                two_char_ok = true;
                            }
                        }
                        if !two_char_ok {
                            pptr = crate::pcre2_compile_class::_pcre2_compile_class_not_nested_8(
                                options,
                                xoptions,
                                pptr.add(1),
                                &mut code,
                                (meta == META_CLASS_NOT) as BOOL,
                                ptr::null_mut(),
                                errorcodeptr,
                                cb,
                                lengthptr,
                            );
                            if pptr.is_null() {
                                return 0;
                            }
                            do_class_end_processing = true;
                        }
                    } else {
                        pptr = crate::pcre2_compile_class::_pcre2_compile_class_not_nested_8(
                            options,
                            xoptions,
                            pptr.add(1),
                            &mut code,
                            (meta == META_CLASS_NOT) as BOOL,
                            ptr::null_mut(),
                            errorcodeptr,
                            cb,
                            lengthptr,
                        );
                        if pptr.is_null() {
                            return 0;
                        }
                        do_class_end_processing = true;
                    }
                }

                if do_class_end_processing {
                    // CLASS_END_PROCESSING
                    if firstcuflags == REQ_UNSET {
                        firstcuflags = REQ_NONE;
                    }
                    zerofirstcu = firstcu;
                    zerofirstcuflags = firstcuflags;
                    zeroreqcu = reqcu;
                    zeroreqcuflags = reqcuflags;
                }
                // else fall through to normal char handling via flags
            }

            META_ACCEPT => {
                (*cb).had_accept = TRUE;
                had_accept = true;
                oc = open_caps;
                while !oc.is_null() && (*oc).assert_depth >= (*cb).assert_depth {
                    if !lengthptr.is_null() {
                        *lengthptr += (1 + IMM2_SIZE) as PCRE2_SIZE;
                    } else {
                        wr!(code, OP_CLOSE);
                        PUT2INC!(code, 0, (*oc).number as u32);
                    }
                    oc = (*oc).next;
                }
                wr!(code, if (*cb).assert_depth > 0 { OP_ASSERT_ACCEPT } else { OP_ACCEPT });
                if firstcuflags == REQ_UNSET {
                    firstcuflags = REQ_NONE;
                }
            }

            META_PRUNE | META_SKIP => {
                (*cb).had_pruneorskip = TRUE;
                wr!(code, VERBOPS[((meta - META_MARK) >> 16) as usize] as u8);
            }
            META_COMMIT | META_FAIL => {
                wr!(code, VERBOPS[((meta - META_MARK) >> 16) as usize] as u8);
            }

            META_THEN => {
                (*cb).external_flags |= PCRE2_HASTHEN;
                wr!(code, OP_THEN);
            }

            META_THEN_ARG | META_PRUNE_ARG | META_SKIP_ARG | META_MARK | META_COMMIT_ARG => {
                if meta == META_THEN_ARG {
                    (*cb).external_flags |= PCRE2_HASTHEN;
                }
                if meta == META_PRUNE_ARG || meta == META_SKIP_ARG {
                    (*cb).had_pruneorskip = TRUE;
                }
                // VERB_ARG
                wr!(code, VERBOPS[((meta - META_MARK) >> 16) as usize] as u8);
                pptr = pptr.add(1);
                verbarglen = *pptr;
                verbculen = 0;
                tempcode = code;
                code = code.add(1);
                let mut vi: c_int = 0;
                while vi < verbarglen as c_int {
                    pptr = pptr.add(1);
                    meta = *pptr;
                    if utf {
                        mclength = crate::pcre2_ord2utf::_pcre2_ord2utf_8(meta, mcbuffer.as_mut_ptr());
                    } else {
                        mclength = 1;
                        mcbuffer[0] = meta as u8;
                    }
                    if !lengthptr.is_null() {
                        *lengthptr += mclength as PCRE2_SIZE;
                    } else {
                        memcpy(code as *mut c_void, mcbuffer.as_ptr() as *const c_void, mclength as usize);
                        code = code.add(mclength as usize);
                        verbculen += mclength;
                    }
                    vi += 1;
                }
                *tempcode = verbculen as u8;
                wr!(code, 0);
            }

            META_OPTIONS => {
                pptr = pptr.add(1);
                options = *pptr;
                *optionsptr = options;
                pptr = pptr.add(1);
                xoptions = *pptr;
                *xoptionsptr = xoptions;
                greedy_default = ((options & PCRE2_UNGREEDY) != 0) as u32;
                greedy_non_default = greedy_default ^ 1;
                req_caseopt = if (options & PCRE2_CASELESS) != 0 { REQ_CASELESS } else { 0 };
            }

            META_OFFSET => {
                if !lengthptr.is_null() {
                    pptr = crate::pcre2_compile_cgroup::_pcre2_compile_parse_scan_substr_args8(
                        pptr, errorcodeptr, cb, lengthptr,
                    );
                    if pptr.is_null() {
                        return 0;
                    }
                } else {
                    loop {
                        match META_CODE(*pptr) {
                            META_OFFSET => {
                                pptr = pptr.add(1);
                                SKIPOFFSET!(pptr);
                                continue;
                            }
                            META_CAPTURE_NAME => {
                                let ng2 = (*cb).named_groups.add(*pptr.add(1) as usize);
                                pptr = pptr.add(2);
                                let mut count: c_int = 0;
                                let mut index: c_int = 0;
                                if crate::pcre2_compile_cgroup::_pcre2_compile_find_dupname_details8(
                                    (*ng2).name, (*ng2).length as u32, &mut index, &mut count,
                                    errorcodeptr, cb,
                                ) == FALSE
                                {
                                    return 0;
                                }
                                *code.add(0) = OP_DNCREF;
                                PUT2(code, 1, index as u32);
                                PUT2(code, 1 + IMM2_SIZE, count as u32);
                                code = code.add(1 + 2 * IMM2_SIZE);
                                continue;
                            }
                            META_CAPTURE_NUMBER => {
                                pptr = pptr.add(2);
                                if *pptr.sub(1) == 0 {
                                    continue;
                                }
                                *code.add(0) = OP_CREF;
                                PUT2(code, 1, *pptr.sub(1));
                                code = code.add(1 + IMM2_SIZE);
                                continue;
                            }
                            _ => break,
                        }
                    }
                    pptr = pptr.sub(1);
                }
            }

            META_SCS => {
                bravalue = OP_ASSERT_SCS as c_int;
                (*cb).assert_depth += 1;
                goto_group_process = true;
            }

            META_COND_RNUMBER | META_COND_NAME | META_COND_RNAME => {
                bravalue = OP_COND as c_int;

                if !lengthptr.is_null() {
                    let start_pptr = pptr;
                    pptr = pptr.add(1);
                    let length = *pptr;

                    GETPLUSOFFSET!(offset, pptr);
                    let nm = (*cb).start_pattern.add(offset);

                    let ng2 = crate::pcre2_compile_cgroup::_pcre2_compile_find_named_group8(
                        nm, length, cb,
                    );

                    if ng2.is_null() {
                        groupnumber = 0;
                        if meta == META_COND_RNUMBER {
                            let mut ii: u32 = 1;
                            while ii < length {
                                groupnumber = groupnumber * 10 + (*nm.add(ii as usize) as u32 - CHAR_0);
                                if groupnumber > MAX_GROUP_NUMBER {
                                    *errorcodeptr = ERR61;
                                    (*cb).erroroffset = offset + ii as usize;
                                    return 0;
                                }
                                ii += 1;
                            }
                        }

                        if meta != META_COND_RNUMBER || groupnumber > (*cb).bracount {
                            *errorcodeptr = ERR15;
                            (*cb).erroroffset = offset;
                            return 0;
                        }

                        if groupnumber == 0 {
                            groupnumber = RREF_ANY;
                        }
                        *start_pptr.add(1) = groupnumber;
                        skipunits = 1 + IMM2_SIZE as u32;
                        goto_group_process_note_empty = true;
                    } else {
                        if meta == META_COND_RNUMBER {
                            meta = META_COND_NAME;
                        }

                        if ((*ng2).hash_dup & NAMED_GROUP_IS_DUPNAME) == 0 {
                            if (*ng2).number > (*cb).top_backref {
                                (*cb).top_backref = (*ng2).number;
                            }
                            *start_pptr.add(0) = meta;
                            *start_pptr.add(1) = (*ng2).number;
                            skipunits = 1 + IMM2_SIZE as u32;
                            goto_group_process_note_empty = true;
                        } else {
                            *start_pptr.add(0) = meta | 1;
                            *start_pptr.add(1) =
                                (ng2 as usize - (*cb).named_groups as usize) as u32
                                    / core::mem::size_of::<named_group>() as u32;
                            skipunits = 1 + 2 * IMM2_SIZE as u32;
                        }
                    }
                } else {
                    if meta == META_COND_RNUMBER {
                        *code.add(1 + LINK_SIZE) = OP_RREF;
                        PUT2(code, 2 + LINK_SIZE, *pptr.add(1));
                        skipunits = 1 + IMM2_SIZE as u32;
                        pptr = pptr.add(1 + SIZEOFFSET);
                        goto_group_process_note_empty = true;
                    } else if meta_arg == 0 {
                        *code.add(1 + LINK_SIZE) =
                            if meta == META_COND_RNAME { OP_RREF } else { OP_CREF };
                        PUT2(code, 2 + LINK_SIZE, *pptr.add(1));
                        skipunits = 1 + IMM2_SIZE as u32;
                        pptr = pptr.add(1 + SIZEOFFSET);
                        goto_group_process_note_empty = true;
                    } else {
                        let ng2 = (*cb).named_groups.add(*pptr.add(1) as usize);
                        let mut count: c_int = 0;
                        let mut index: c_int = 0;
                        if crate::pcre2_compile_cgroup::_pcre2_compile_find_dupname_details8(
                            (*ng2).name, (*ng2).length as u32, &mut index, &mut count,
                            errorcodeptr, cb,
                        ) == FALSE
                        {
                            return 0;
                        }
                        *code.add(1 + LINK_SIZE) =
                            if meta == META_COND_RNAME { OP_DNRREF } else { OP_DNCREF };
                        PUT2(code, 2 + LINK_SIZE, index as u32);
                        PUT2(code, 2 + LINK_SIZE + IMM2_SIZE, count as u32);
                        skipunits = 1 + 2 * IMM2_SIZE as u32;
                        pptr = pptr.add(1 + SIZEOFFSET);
                        goto_group_process_note_empty = true;
                    }
                }

                if !goto_group_process && !goto_group_process_note_empty {
                    goto_group_process_note_empty = true;
                }
            }

            META_COND_DEFINE => {
                bravalue = OP_COND as c_int;
                GETPLUSOFFSET!(offset, pptr);
                *code.add(1 + LINK_SIZE) = OP_DEFINE;
                skipunits = 1;
                goto_group_process = true;
            }

            META_COND_NUMBER => {
                bravalue = OP_COND as c_int;
                GETPLUSOFFSET!(offset, pptr);
                pptr = pptr.add(1);
                groupnumber = *pptr;
                if groupnumber > (*cb).bracount {
                    *errorcodeptr = ERR15;
                    (*cb).erroroffset = offset;
                    return 0;
                }
                if groupnumber > (*cb).top_backref {
                    (*cb).top_backref = groupnumber;
                }
                offset -= 2;
                *code.add(1 + LINK_SIZE) = OP_CREF;
                skipunits = 1 + IMM2_SIZE as u32;
                PUT2(code, 2 + LINK_SIZE, groupnumber);
                goto_group_process_note_empty = true;
            }

            META_COND_VERSION => {
                bravalue = OP_COND as c_int;
                if *pptr.add(1) > 0 {
                    *code.add(1 + LINK_SIZE) = if (PCRE2_MAJOR > *pptr.add(2))
                        || (PCRE2_MAJOR == *pptr.add(2) && PCRE2_MINOR >= *pptr.add(3))
                    {
                        OP_TRUE
                    } else {
                        OP_FALSE
                    };
                } else {
                    *code.add(1 + LINK_SIZE) =
                        if PCRE2_MAJOR == *pptr.add(2) && PCRE2_MINOR == *pptr.add(3) {
                            OP_TRUE
                        } else {
                            OP_FALSE
                        };
                }
                skipunits = 1;
                pptr = pptr.add(3);
                goto_group_process_note_empty = true;
            }

            META_COND_ASSERT => {
                bravalue = OP_COND as c_int;
                goto_group_process_note_empty = true;
            }

            META_LOOKAHEAD => {
                bravalue = OP_ASSERT as c_int;
                (*cb).assert_depth += 1;
                goto_group_process = true;
            }
            META_LOOKAHEAD_NA => {
                bravalue = OP_ASSERT_NA as c_int;
                (*cb).assert_depth += 1;
                goto_group_process = true;
            }
            META_LOOKAHEADNOT => {
                if *pptr.add(1) == META_KET
                    && (*pptr.add(2) < META_ASTERISK || *pptr.add(2) > META_MINMAX_QUERY)
                {
                    wr!(code, OP_FAIL);
                    pptr = pptr.add(1);
                } else {
                    bravalue = OP_ASSERT_NOT as c_int;
                    (*cb).assert_depth += 1;
                    goto_group_process = true;
                }
            }
            META_LOOKBEHIND => {
                bravalue = OP_ASSERTBACK as c_int;
                (*cb).assert_depth += 1;
                goto_group_process = true;
            }
            META_LOOKBEHINDNOT => {
                bravalue = OP_ASSERTBACK_NOT as c_int;
                (*cb).assert_depth += 1;
                goto_group_process = true;
            }
            META_LOOKBEHIND_NA => {
                bravalue = OP_ASSERTBACK_NA as c_int;
                (*cb).assert_depth += 1;
                goto_group_process = true;
            }

            META_ATOMIC => {
                bravalue = OP_ONCE as c_int;
                goto_group_process_note_empty = true;
            }
            META_SCRIPT_RUN => {
                bravalue = OP_SCRIPT_RUN as c_int;
                goto_group_process_note_empty = true;
            }
            META_NOCAPTURE => {
                bravalue = OP_BRA as c_int;
                goto_group_process_note_empty = true;
            }

            META_BACKREF_BYNAME | META_RECURSE_BYNAME => {
                let mut count: c_int = 0;
                let mut index: c_int = 0;
                let nm: PCRE2_SPTR;
                let ng2: *mut named_group;
                pptr = pptr.add(1);
                let length = *pptr;

                GETPLUSOFFSET!(offset, pptr);
                nm = (*cb).start_pattern.add(offset);

                ng2 = crate::pcre2_compile_cgroup::_pcre2_compile_find_named_group8(nm, length, cb);

                if ng2.is_null() {
                    *errorcodeptr = ERR15;
                    (*cb).erroroffset = offset;
                    return 0;
                }

                groupnumber = (*ng2).number;

                if meta == META_RECURSE_BYNAME {
                    meta_arg = groupnumber;
                    goto_handle_numerical_recursion = true;
                } else {
                    (*cb).backref_map |= if groupnumber < 32 { 1u32 << groupnumber } else { 1 };
                    if groupnumber > (*cb).top_backref {
                        (*cb).top_backref = groupnumber;
                    }

                    if ((*ng2).hash_dup & NAMED_GROUP_IS_DUPNAME) == 0 {
                        meta_arg = groupnumber;
                        goto_handle_single_reference = true;
                    } else {
                        if lengthptr.is_null()
                            && crate::pcre2_compile_cgroup::_pcre2_compile_find_dupname_details8(
                                nm, length, &mut index, &mut count, errorcodeptr, cb,
                            ) == FALSE
                        {
                            return 0;
                        }

                        if firstcuflags == REQ_UNSET {
                            firstcuflags = REQ_NONE;
                        }
                        wr!(code, if (options & PCRE2_CASELESS) != 0 { OP_DNREFI } else { OP_DNREF });
                        PUT2INC!(code, 0, index as u32);
                        PUT2INC!(code, 0, count as u32);
                        if (options & PCRE2_CASELESS) != 0 {
                            wr!(
                                code,
                                (if (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0 {
                                    REFI_FLAG_CASELESS_RESTRICT
                                } else {
                                    0
                                } | if (xoptions & PCRE2_EXTRA_TURKISH_CASING) != 0 {
                                    REFI_FLAG_TURKISH_CASING
                                } else {
                                    0
                                }) as u8
                            );
                        }
                    }
                }
            }

            META_CALLOUT_NUMBER => {
                *code.add(0) = OP_CALLOUT;
                PUT(code, 1, *pptr.add(1));
                PUT(code, 1 + LINK_SIZE, *pptr.add(2));
                *code.add(1 + 2 * LINK_SIZE) = *pptr.add(3) as u8;
                pptr = pptr.add(3);
                code = code.add(oplen(OP_CALLOUT));
            }

            META_CALLOUT_STRING => {
                if !lengthptr.is_null() {
                    *lengthptr += (*pptr.add(3) as PCRE2_SIZE) + (1 + 4 * LINK_SIZE) as PCRE2_SIZE;
                    pptr = pptr.add(3);
                    SKIPOFFSET!(pptr);
                } else {
                    let pp0: PCRE2_SPTR;
                    let mut delimiter: u32;
                    let mut length = *pptr.add(3);
                    let mut callout_string = code.add(1 + 4 * LINK_SIZE);

                    *code.add(0) = OP_CALLOUT_STR;
                    PUT(code, 1, *pptr.add(1));
                    PUT(code, 1 + LINK_SIZE, *pptr.add(2));

                    pptr = pptr.add(3);
                    GETPLUSOFFSET!(offset, pptr);
                    let mut pp = (*cb).start_pattern.add(offset);
                    delimiter = *pp as u32;
                    *callout_string = *pp;
                    callout_string = callout_string.add(1);
                    pp = pp.add(1);
                    if delimiter == CHAR_LEFT_CURLY_BRACKET {
                        delimiter = CHAR_RIGHT_CURLY_BRACKET;
                    }
                    PUT(code, 1 + 3 * LINK_SIZE, (offset + 1) as u32);

                    loop {
                        length -= 1;
                        if length <= 1 {
                            break;
                        }
                        if *pp as u32 == delimiter && *pp.add(1) as u32 == delimiter {
                            *callout_string = delimiter as u8;
                            callout_string = callout_string.add(1);
                            pp = pp.add(2);
                            length -= 1;
                        } else {
                            *callout_string = *pp;
                            callout_string = callout_string.add(1);
                            pp = pp.add(1);
                        }
                    }
                    *callout_string = CHAR_NUL as u8;
                    callout_string = callout_string.add(1);

                    PUT(code, 1 + 2 * LINK_SIZE, (callout_string as usize - code as usize) as u32);
                    code = callout_string;
                }
            }

            META_MINMAX_PLUS | META_MINMAX_QUERY | META_MINMAX => {
                pptr = pptr.add(1);
                repeat_min = *pptr;
                pptr = pptr.add(1);
                repeat_max = *pptr;
                goto_do_repeat = true;
            }
            META_ASTERISK | META_ASTERISK_PLUS | META_ASTERISK_QUERY => {
                repeat_min = 0;
                repeat_max = REPEAT_UNLIMITED;
                goto_do_repeat = true;
            }
            META_PLUS | META_PLUS_PLUS | META_PLUS_QUERY => {
                repeat_min = 1;
                repeat_max = REPEAT_UNLIMITED;
                goto_do_repeat = true;
            }
            META_QUERY | META_QUERY_PLUS | META_QUERY_QUERY => {
                repeat_min = 0;
                repeat_max = 1;
                goto_do_repeat = true;
            }

            META_BIGVALUE => {
                pptr = pptr.add(1);
                goto_normal_char = true;
            }

            META_BACKREF => {
                if meta_arg < 10 {
                    offset = (*cb).small_ref_offset[meta_arg as usize];
                } else {
                    GETPLUSOFFSET!(offset, pptr);
                }

                if meta_arg > (*cb).bracount {
                    (*cb).erroroffset = offset;
                    *errorcodeptr = ERR15;
                    return 0;
                }

                goto_handle_single_reference = true;
            }

            META_RECURSE => {
                GETPLUSOFFSET!(offset, pptr);
                if meta_arg > (*cb).bracount {
                    (*cb).erroroffset = offset;
                    *errorcodeptr = ERR15;
                    return 0;
                }
                goto_handle_numerical_recursion = true;
            }

            META_CAPTURE => {
                bravalue = OP_CBRA as c_int;
                skipunits = IMM2_SIZE as u32;
                PUT2(code, 1 + LINK_SIZE, meta_arg);
                (*cb).lastcapture = meta_arg;
                goto_group_process_note_empty = true;
            }

            META_ESCAPE => {
                goto_meta_escape = true;
            }

            _ => {
                if meta >= META_END {
                    *errorcodeptr = ERR89;
                    return 0;
                }
                goto_normal_char = true;
            }
        }

        // ---- Shared label dispatch (emulating C gotos) ----

        // GROUP_PROCESS_NOTE_EMPTY / GROUP_PROCESS
        if goto_group_process_note_empty {
            note_group_empty = true;
            goto_group_process = true;
        }

        if goto_group_process {
            (*cb).parens_depth += 1;
            *code = bravalue as u8;
            pptr = pptr.add(1);
            tempcode = code;
            tempreqvary = (*cb).req_varyopt;
            length_prevgroup = 0;

            group_return = compile_regex(
                options,
                xoptions,
                &mut tempcode,
                &mut pptr,
                errorcodeptr,
                skipunits,
                &mut subfirstcu,
                &mut subfirstcuflags,
                &mut subreqcu,
                &mut subreqcuflags,
                bcptr,
                open_caps,
                cb,
                if lengthptr.is_null() { ptr::null_mut() } else { &mut length_prevgroup },
            );
            if group_return == 0 {
                return 0;
            }

            (*cb).parens_depth -= 1;

            if note_group_empty && bravalue != OP_COND as c_int && group_return > 0 {
                matched_char = true;
            }

            if bravalue >= OP_ASSERT as c_int && bravalue <= OP_ASSERT_SCS as c_int {
                (*cb).assert_depth -= 1;
            }

            if bravalue == OP_COND as c_int && lengthptr.is_null() {
                let mut tc = code;
                let mut condcount = 0;
                loop {
                    condcount += 1;
                    tc = tc.add(GET(tc, 1) as usize);
                    if *tc == OP_KET {
                        break;
                    }
                }

                if *code.add(LINK_SIZE + 1) == OP_DEFINE {
                    if condcount > 1 {
                        (*cb).erroroffset = offset;
                        *errorcodeptr = ERR54;
                        return 0;
                    }
                    *code.add(LINK_SIZE + 1) = OP_FALSE;
                    bravalue = OP_DEFINE as c_int;
                } else {
                    if condcount > 2 {
                        (*cb).erroroffset = offset;
                        *errorcodeptr = ERR27;
                        return 0;
                    }
                    if condcount == 1 {
                        subfirstcuflags = REQ_NONE;
                        subreqcuflags = REQ_NONE;
                    } else if group_return > 0 {
                        matched_char = true;
                    }
                }
            }

            if !lengthptr.is_null() {
                if OFLOW_MAX - *lengthptr < length_prevgroup - 2 - 2 * LINK_SIZE {
                    *errorcodeptr = ERR20;
                    return 0;
                }
                *lengthptr += length_prevgroup - 2 - 2 * LINK_SIZE;
                code = code.add(1);
                PUTINC!(code, 0, (1 + LINK_SIZE) as u32);
                wr!(code, OP_KET);
                PUTINC!(code, 0, (1 + LINK_SIZE) as u32);
                pptr = pptr.add(1);
                continue;
            }

            code = tempcode;

            if bravalue == OP_DEFINE as c_int {
                pptr = pptr.add(1);
                continue;
            }

            zeroreqcu = reqcu;
            zeroreqcuflags = reqcuflags;
            zerofirstcu = firstcu;
            zerofirstcuflags = firstcuflags;
            groupsetfirstcu = false;

            if bravalue >= OP_ONCE as c_int {
                if firstcuflags == REQ_UNSET && subfirstcuflags != REQ_UNSET {
                    if subfirstcuflags < REQ_NONE {
                        firstcu = subfirstcu;
                        firstcuflags = subfirstcuflags;
                        groupsetfirstcu = true;
                    } else {
                        firstcuflags = REQ_NONE;
                    }
                    zerofirstcuflags = REQ_NONE;
                } else if subfirstcuflags < REQ_NONE && subreqcuflags >= REQ_NONE {
                    subreqcu = subfirstcu;
                    subreqcuflags = subfirstcuflags | tempreqvary;
                }

                if subreqcuflags < REQ_NONE {
                    reqcu = subreqcu;
                    reqcuflags = subreqcuflags;
                }
            } else if (bravalue == OP_ASSERT as c_int || bravalue == OP_ASSERT_NA as c_int)
                && subreqcuflags < REQ_NONE
                && subfirstcuflags < REQ_NONE
            {
                reqcu = subreqcu;
                reqcuflags = subreqcuflags;
            }

            pptr = pptr.add(1);
            continue;
        }

        // HANDLE_NUMERICAL_RECURSION
        if goto_handle_numerical_recursion {
            *code = OP_RECURSE;
            PUT(code, 1, meta_arg);
            code = code.add(1 + LINK_SIZE);
            length_prevgroup = (1 + LINK_SIZE) as PCRE2_SIZE;

            if META_CODE(*pptr.add(1)) == META_OFFSET
                || META_CODE(*pptr.add(1)) == META_CAPTURE_NAME
                || META_CODE(*pptr.add(1)) == META_CAPTURE_NUMBER
            {
                if !lengthptr.is_null() {
                    if crate::pcre2_compile_cgroup::_pcre2_compile_parse_recurse_args8(
                        pptr, offset, errorcodeptr, cb,
                    ) == FALSE
                    {
                        return 0;
                    }
                    let args = (*cb).last_data as *mut recurse_arguments;
                    length_prevgroup += (*args).size * (1 + IMM2_SIZE);
                    *lengthptr += ((*args).size * (1 + IMM2_SIZE)) as PCRE2_SIZE;
                    pptr = pptr.add((*args).skip_size);
                } else {
                    let args = (*cb).first_data as *mut recurse_arguments;
                    let mut current = (args.add(1)) as *mut u16;
                    let end = current.add((*args).size);

                    loop {
                        *code.add(0) = OP_CREF;
                        PUT2(code, 1, *current as u32);
                        code = code.add(1 + IMM2_SIZE);
                        current = current.add(1);
                        if current >= end {
                            break;
                        }
                    }

                    length_prevgroup += (*args).size * (1 + IMM2_SIZE);
                    pptr = pptr.add((*args).skip_size);
                    (*cb).first_data = (*args).header.next;
                    ((*(*cb).cx).memctl.free.unwrap())(
                        args as *mut c_void,
                        (*(*cb).cx).memctl.memory_data,
                    );
                }
            }

            groupsetfirstcu = false;
            (*cb).had_recurse = TRUE;
            if firstcuflags == REQ_UNSET {
                firstcuflags = REQ_NONE;
            }
            zerofirstcu = firstcu;
            zerofirstcuflags = firstcuflags;
            pptr = pptr.add(1);
            continue;
        }

        // META_ESCAPE
        if goto_meta_escape {
            if meta_arg > ESC_b as u32 && meta_arg < ESC_Z as u32 {
                matched_char = true;
                if firstcuflags == REQ_UNSET {
                    firstcuflags = REQ_NONE;
                }
            }

            zerofirstcu = firstcu;
            zerofirstcuflags = firstcuflags;
            zeroreqcu = reqcu;
            zeroreqcuflags = reqcuflags;

            if meta_arg == ESC_P as u32 || meta_arg == ESC_p as u32 {
                pptr = pptr.add(1);
                let mut ptype = *pptr >> 16;
                let mut pdata = *pptr & 0xffff;

                if (options & PCRE2_CASELESS) != 0
                    && ptype == PT_PC
                    && (pdata == ucp_Lu || pdata == ucp_Ll || pdata == ucp_Lt)
                {
                    ptype = PT_LAMP;
                    pdata = 0;
                }

                if ptype == PT_ANY {
                    if meta_arg == ESC_P as u32 {
                        wr!(code, OP_CLASS);
                        memset(code as *mut c_void, 0, 32);
                        code = code.add(32);
                    } else {
                        wr!(code, OP_ALLANY);
                    }
                } else {
                    wr!(code, if meta_arg == ESC_p as u32 { OP_PROP } else { OP_NOTPROP });
                    wr!(code, ptype as u8);
                    wr!(code, pdata as u8);
                }
                pptr = pptr.add(1);
                continue;
            }

            if (*cb).assert_depth > 0
                && meta_arg == ESC_K as u32
                && (xoptions & PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK) == 0
            {
                *errorcodeptr = ERR99;
                return 0;
            }

            match meta_arg {
                x if x == ESC_C as u32 => {
                    (*cb).external_flags |= PCRE2_HASBKC;
                    if !utf {
                        meta_arg = OP_ALLANY as u32;
                    }
                }
                x if x == ESC_B as u32 || x == ESC_b as u32 => {
                    if (options & PCRE2_UCP) != 0 && (xoptions & PCRE2_EXTRA_ASCII_BSW) == 0 {
                        meta_arg = if meta_arg == ESC_B as u32 {
                            OP_NOT_UCP_WORD_BOUNDARY as u32
                        } else {
                            OP_UCP_WORD_BOUNDARY as u32
                        };
                    }
                    if (*cb).max_lookbehind == 0 {
                        (*cb).max_lookbehind = 1;
                    }
                }
                x if x == ESC_A as u32 => {
                    if (*cb).max_lookbehind == 0 {
                        (*cb).max_lookbehind = 1;
                    }
                }
                x if x == ESC_K as u32 => {
                    (*cb).external_flags |= PCRE2_HASBSK;
                }
                _ => {}
            }

            wr!(code, meta_arg as u8);
            pptr = pptr.add(1);
            continue;
        }

        // NORMAL_CHAR / NORMAL_CHAR_SET / CLASS_CASELESS_CHAR
        if goto_normal_char || goto_normal_char_set || goto_class_caseless_char {
            if goto_normal_char {
                meta = *pptr;
                normal_char_meta = meta;
            }
            if goto_normal_char || goto_normal_char_set {
                meta = normal_char_meta;
                matched_char = true;

                let mut jump_caseless = false;
                if (utf || ucp) && (options & PCRE2_CASELESS) != 0 {
                    let mut caseset: u32;
                    if (xoptions & (PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT))
                        == PCRE2_EXTRA_TURKISH_CASING
                        && UCD_ANY_I(meta)
                    {
                        caseset = _pcre2_ucd_turkish_dotted_i_caseset_8
                            + (if UCD_DOTTED_I(meta) { 0 } else { 3 });
                    } else {
                        caseset = UCD_CASESET(meta);
                        if caseset != 0
                            && (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0
                            && _pcre2_ucd_caseless_sets_8[caseset as usize] < 128
                        {
                            caseset = 0;
                        }
                    }

                    if caseset != 0 {
                        wr!(code, OP_PROP);
                        wr!(code, PT_CLIST as u8);
                        wr!(code, caseset as u8);
                        if firstcuflags == REQ_UNSET {
                            firstcuflags = REQ_NONE;
                            zerofirstcuflags = REQ_NONE;
                        }
                        pptr = pptr.add(1);
                        continue;
                    }
                }
            } else {
                // goto_class_caseless_char: meta already set
                meta = normal_char_meta;
            }

            // CLASS_CASELESS_CHAR:
            if utf {
                mclength = crate::pcre2_ord2utf::_pcre2_ord2utf_8(meta, mcbuffer.as_mut_ptr());
            } else {
                mclength = 1;
                mcbuffer[0] = meta as u8;
            }

            wr!(code, if (options & PCRE2_CASELESS) != 0 { OP_CHARI } else { OP_CHAR });
            memcpy(code as *mut c_void, mcbuffer.as_ptr() as *const c_void, mclength as usize);
            code = code.add(mclength as usize);

            if mcbuffer[0] as u32 == CHAR_CR || mcbuffer[0] as u32 == CHAR_NL {
                (*cb).external_flags |= PCRE2_HASCRORLF;
            }

            if firstcuflags == REQ_UNSET {
                zerofirstcuflags = REQ_NONE;
                zeroreqcu = reqcu;
                zeroreqcuflags = reqcuflags;

                if mclength == 1 || req_caseopt == 0 {
                    firstcu = mcbuffer[0] as u32;
                    firstcuflags = req_caseopt;
                    if mclength != 1 {
                        reqcu = *code.sub(1) as u32;
                        reqcuflags = (*cb).req_varyopt;
                    }
                } else {
                    firstcuflags = REQ_NONE;
                    reqcuflags = REQ_NONE;
                }
            } else {
                zerofirstcu = firstcu;
                zerofirstcuflags = firstcuflags;
                zeroreqcu = reqcu;
                zeroreqcuflags = reqcuflags;
                if mclength == 1 || req_caseopt == 0 {
                    reqcu = *code.sub(1) as u32;
                    reqcuflags = req_caseopt | (*cb).req_varyopt;
                }
            }

            if reset_caseful {
                options &= !PCRE2_CASELESS;
                req_caseopt = 0;
                reset_caseful = false;
            }

            pptr = pptr.add(1);
            continue;
        }

        // HANDLE_SINGLE_REFERENCE
        if goto_handle_single_reference {
            if firstcuflags == REQ_UNSET {
                zerofirstcuflags = REQ_NONE;
                firstcuflags = REQ_NONE;
            }
            wr!(code, if (options & PCRE2_CASELESS) != 0 { OP_REFI } else { OP_REF });
            PUT2INC!(code, 0, meta_arg);
            if (options & PCRE2_CASELESS) != 0 {
                wr!(
                    code,
                    (if (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0 {
                        REFI_FLAG_CASELESS_RESTRICT
                    } else {
                        0
                    } | if (xoptions & PCRE2_EXTRA_TURKISH_CASING) != 0 {
                        REFI_FLAG_TURKISH_CASING
                    } else {
                        0
                    }) as u8
                );
            }

            (*cb).backref_map |= if meta_arg < 32 { 1u32 << meta_arg } else { 1 };
            if meta_arg > (*cb).top_backref {
                (*cb).top_backref = meta_arg;
            }
            pptr = pptr.add(1);
            continue;
        }

        // REPEAT handling
        if goto_do_repeat {
            match compile_repeat(
                meta, meta_arg, &mut repeat_min, &mut repeat_max, previous_matched_char,
                &mut matched_char, &mut code, &mut previous, &mut firstcu, &mut firstcuflags,
                &mut reqcu, &mut reqcuflags, &mut zerofirstcu, &mut zerofirstcuflags,
                &mut zeroreqcu, &mut zeroreqcuflags, &mut reqvary, greedy_default,
                greedy_non_default, &mut length_prevgroup, &mut group_return, groupsetfirstcu,
                utf, cb, lengthptr, errorcodeptr,
            ) {
                RepRes::Ok | RepRes::EndRepeat => {
                    (*cb).req_varyopt |= reqvary;
                }
                RepRes::Error => return 0,
            }
            pptr = pptr.add(1);
            continue;
        }

        pptr = pptr.add(1);
    }
}

enum RepRes {
    Ok,
    Error,
    EndRepeat,
}

// Emit OP_STAR/PLUS/etc for single char or char-type repeats (OUTPUT_SINGLE_REPEAT).
unsafe fn output_single_repeat(
    code_io: &mut *mut PCRE2_UCHAR,
    previous: *mut PCRE2_UCHAR,
    op_previous: u8,
    op_type: u32,
    prop_type: c_int,
    prop_value: c_int,
    mcbuffer: &mut [PCRE2_UCHAR; 8],
    mclength: u32,
    repeat_min_io: &mut u32,
    repeat_max_io: &mut u32,
    repeat_type_in: u32,
) -> RepRes {
    let mut code;
    let oldcode = *code_io;
    let mut repeat_min = *repeat_min_io;
    let mut repeat_max = *repeat_max_io;
    let mut repeat_type = repeat_type_in;

    code = previous;

    if repeat_max == 0 {
        *code_io = code;
        *repeat_min_io = repeat_min;
        *repeat_max_io = repeat_max;
        return RepRes::EndRepeat;
    }

    repeat_type += op_type;

    if repeat_min == 0 {
        if repeat_max == REPEAT_UNLIMITED {
            wr!(code, OP_STAR + repeat_type as u8);
        } else if repeat_max == 1 {
            wr!(code, OP_QUERY + repeat_type as u8);
        } else {
            wr!(code, OP_UPTO + repeat_type as u8);
            PUT2INC!(code, 0, repeat_max);
        }
    } else if repeat_min == 1 {
        if repeat_max == REPEAT_UNLIMITED {
            wr!(code, OP_PLUS + repeat_type as u8);
        } else {
            code = oldcode;
            if repeat_max == 1 {
                *code_io = code;
                *repeat_min_io = repeat_min;
                *repeat_max_io = repeat_max;
                return RepRes::EndRepeat;
            }
            wr!(code, OP_UPTO + repeat_type as u8);
            PUT2INC!(code, 0, repeat_max - 1);
        }
    } else {
        wr!(code, OP_EXACT + op_type as u8);
        PUT2INC!(code, 0, repeat_min);

        if repeat_max != repeat_min {
            if mclength > 0 {
                memcpy(code as *mut c_void, mcbuffer.as_ptr() as *const c_void, mclength as usize);
                code = code.add(mclength as usize);
            } else {
                wr!(code, op_previous);
                if prop_type >= 0 {
                    wr!(code, prop_type as u8);
                    wr!(code, prop_value as u8);
                }
            }

            if repeat_max == REPEAT_UNLIMITED {
                wr!(code, OP_STAR + repeat_type as u8);
            } else {
                repeat_max -= repeat_min;
                if repeat_max == 1 {
                    wr!(code, OP_QUERY + repeat_type as u8);
                } else {
                    wr!(code, OP_UPTO + repeat_type as u8);
                    PUT2INC!(code, 0, repeat_max);
                }
            }
        }
    }

    if mclength > 0 {
        memcpy(code as *mut c_void, mcbuffer.as_ptr() as *const c_void, mclength as usize);
        code = code.add(mclength as usize);
    } else {
        wr!(code, op_previous);
        if prop_type >= 0 {
            wr!(code, prop_type as u8);
            wr!(code, prop_value as u8);
        }
    }

    *code_io = code;
    *repeat_min_io = repeat_min;
    *repeat_max_io = repeat_max;
    RepRes::Ok
}

// Handle repetition of a bracketed group (the big OP_BRA... case). Returns
// EndRepeat if the C code did "goto END_REPEAT".
unsafe fn compile_bracket_repeat(
    code_io: &mut *mut PCRE2_UCHAR,
    previous_io: &mut *mut PCRE2_UCHAR,
    op_previous: u8,
    repeat_min_io: &mut u32,
    repeat_max_io: &mut u32,
    repeat_type: u32,
    possessive_quantifier: &mut bool,
    length_prevgroup_io: &mut PCRE2_SIZE,
    group_return: c_int,
    firstcu: &mut u32,
    firstcuflags: &mut u32,
    reqcu: &mut u32,
    reqcuflags: &mut u32,
    groupsetfirstcu: bool,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
    errorcodeptr: *mut c_int,
) -> RepRes {
    let mut code = *code_io;
    let mut previous = *previous_io;
    let mut repeat_min = *repeat_min_io;
    let mut repeat_max = *repeat_max_io;
    let length_prevgroup = *length_prevgroup_io;

    let len = (code as usize - previous as usize) as c_int;
    let mut bralink: *mut PCRE2_UCHAR = ptr::null_mut();
    let mut brazeroptr: *mut PCRE2_UCHAR = ptr::null_mut();

    macro_rules! save_ret {
        ($v:expr) => {{
            *code_io = code;
            *previous_io = previous;
            *repeat_min_io = repeat_min;
            *repeat_max_io = repeat_max;
            *length_prevgroup_io = length_prevgroup;
            return $v;
        }};
    }

    if repeat_max == 1 && repeat_min == 1 && !*possessive_quantifier {
        save_ret!(RepRes::EndRepeat);
    }

    if op_previous == OP_COND
        && *previous.add(LINK_SIZE + 1) == OP_FALSE
        && *previous.add(GET(previous, 1) as usize) != OP_ALT
    {
        save_ret!(RepRes::EndRepeat);
    }

    if op_previous < OP_ONCE {
        if repeat_max == REPEAT_UNLIMITED {
            repeat_max = repeat_min + 1;
        }
    }

    if repeat_min == 0 {
        if repeat_max <= 1 || repeat_max == REPEAT_UNLIMITED {
            memmove(previous.add(1) as *mut c_void, previous as *const c_void, len as usize);
            code = code.add(1);
            if repeat_max == 0 {
                *previous = OP_SKIPZERO;
                previous = previous.add(1);
                save_ret!(RepRes::EndRepeat);
            }
            brazeroptr = previous;
            *previous = OP_BRAZERO + repeat_type as u8;
            previous = previous.add(1);
        } else {
            memmove(previous.add(2 + LINK_SIZE) as *mut c_void, previous as *const c_void, len as usize);
            code = code.add(2 + LINK_SIZE);
            *previous = OP_BRAZERO + repeat_type as u8;
            previous = previous.add(1);
            *previous = OP_BRA;
            previous = previous.add(1);

            let linkoffset = if bralink.is_null() {
                0
            } else {
                (previous as usize - bralink as usize) as u32
            };
            bralink = previous;
            PUTINC!(previous, 0, linkoffset);
        }

        if repeat_max != REPEAT_UNLIMITED {
            repeat_max -= 1;
        }
    } else {
        if repeat_min > 1 {
            if !lengthptr.is_null() {
                let mut delta: PCRE2_SIZE = 0;
                if crate::pcre2_chkdint::_pcre2_ckd_smul_8(
                    &mut delta,
                    (repeat_min - 1) as i32,
                    length_prevgroup as i32,
                ) != FALSE
                    || OFLOW_MAX - *lengthptr < delta
                {
                    *errorcodeptr = ERR20;
                    return RepRes::Error;
                }
                *lengthptr += delta;
            } else {
                if groupsetfirstcu && *reqcuflags >= REQ_NONE {
                    *reqcu = *firstcu;
                    *reqcuflags = *firstcuflags;
                }
                let mut i = 1u32;
                while i < repeat_min {
                    memcpy(code as *mut c_void, previous as *const c_void, len as usize);
                    code = code.add(len as usize);
                    i += 1;
                }
            }
        }

        if repeat_max != REPEAT_UNLIMITED {
            repeat_max -= repeat_min;
        }
    }

    if repeat_max != REPEAT_UNLIMITED {
        if !lengthptr.is_null() && repeat_max > 0 {
            let mut delta: PCRE2_SIZE = 0;
            if crate::pcre2_chkdint::_pcre2_ckd_smul_8(
                &mut delta,
                repeat_max as i32,
                (length_prevgroup as usize + 1 + 2 + 2 * LINK_SIZE) as i32,
            ) != FALSE
                || OFLOW_MAX + (2 + 2 * LINK_SIZE) - *lengthptr < delta
            {
                *errorcodeptr = ERR20;
                return RepRes::Error;
            }
            delta -= (2 + 2 * LINK_SIZE) as PCRE2_SIZE;
            *lengthptr += delta;
        } else {
            let mut i = repeat_max;
            while i >= 1 {
                *code = OP_BRAZERO + repeat_type as u8;
                code = code.add(1);

                if i != 1 {
                    *code = OP_BRA;
                    code = code.add(1);
                    let linkoffset = if bralink.is_null() {
                        0
                    } else {
                        (code as usize - bralink as usize) as u32
                    };
                    bralink = code;
                    PUTINC!(code, 0, linkoffset);
                }

                memcpy(code as *mut c_void, previous as *const c_void, len as usize);
                code = code.add(len as usize);
                i -= 1;
            }

            while !bralink.is_null() {
                let linkoffset = (code as usize - bralink as usize + 1) as u32;
                let bra = code.sub(linkoffset as usize);
                let oldlinkoffset = GET(bra, 1);
                bralink = if oldlinkoffset == 0 {
                    ptr::null_mut()
                } else {
                    bralink.sub(oldlinkoffset as usize)
                };
                *code = OP_KET;
                code = code.add(1);
                PUTINC!(code, 0, linkoffset);
                PUT(bra, 1, linkoffset);
            }
        }
    } else {
        let ketcode = code.sub(1 + LINK_SIZE);
        let bracode = ketcode.sub(GET(ketcode, 1) as usize);

        if *bracode == OP_ONCE && *possessive_quantifier {
            *bracode = OP_BRA;
        }

        if *bracode == OP_ONCE || *bracode == OP_SCRIPT_RUN {
            *ketcode = OP_KETRMAX + repeat_type as u8;
        } else {
            if lengthptr.is_null() {
                if group_return < 0 {
                    *bracode += OP_SBRA - OP_BRA;
                }
                if *bracode == OP_COND && *bracode.add(GET(bracode, 1) as usize) != OP_ALT {
                    *bracode = OP_SCOND;
                }
            }

            if *possessive_quantifier {
                if *bracode == OP_COND || *bracode == OP_SCOND {
                    let mut nlen = (code as usize - bracode as usize) as c_int;
                    memmove(
                        bracode.add(1 + LINK_SIZE) as *mut c_void,
                        bracode as *const c_void,
                        nlen as usize,
                    );
                    code = code.add(1 + LINK_SIZE);
                    nlen += (1 + LINK_SIZE) as c_int;
                    *bracode = if *bracode == OP_COND { OP_BRAPOS } else { OP_SBRAPOS };
                    *code = OP_KETRPOS;
                    code = code.add(1);
                    PUTINC!(code, 0, nlen as u32);
                    PUT(bracode, 1, nlen as u32);
                } else {
                    *bracode += 1;
                    *ketcode = OP_KETRPOS;
                }

                if !brazeroptr.is_null() {
                    *brazeroptr = OP_BRAPOSZERO;
                }
                if repeat_min < 2 {
                    *possessive_quantifier = false;
                }
            } else {
                *ketcode = OP_KETRMAX + repeat_type as u8;
            }
        }
    }

    save_ret!(RepRes::Ok);
}

// possessive_quantifier tail handling (wrap in ONCE / switch to POS opcode).
unsafe fn possessive_tail(
    code_io: &mut *mut PCRE2_UCHAR,
    mut tempcode: *mut PCRE2_UCHAR,
    possessive_quantifier: bool,
    utf: bool,
) -> RepRes {
    let mut code = *code_io;
    if possessive_quantifier {
        match *tempcode {
            OP_TYPEEXACT => {
                tempcode = tempcode.add(
                    oplen(*tempcode)
                        + if *tempcode.add(1 + IMM2_SIZE) == OP_PROP
                            || *tempcode.add(1 + IMM2_SIZE) == OP_NOTPROP
                        {
                            2
                        } else {
                            0
                        },
                );
            }
            OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI | OP_EXACT | OP_EXACTI | OP_NOTEXACT
            | OP_NOTEXACTI => {
                tempcode = tempcode.add(oplen(*tempcode));
                if utf && HAS_EXTRALEN(*tempcode.sub(1) as u32) {
                    tempcode = tempcode.add(GET_EXTRALEN(*tempcode.sub(1) as u32) as usize);
                }
            }
            OP_CLASS | OP_NCLASS => {
                tempcode = tempcode.add(1 + 32);
            }
            OP_XCLASS | OP_ECLASS => {
                tempcode = tempcode.add(GET(tempcode, 1) as usize);
            }
            OP_REF | OP_REFI | OP_DNREF | OP_DNREFI => {
                tempcode = tempcode.add(oplen(*tempcode));
            }
            _ => {}
        }

        let mut len = (code as usize - tempcode as usize) as c_int;
        if len > 0 {
            let repcode = *tempcode as usize;
            if repcode < OP_CALLOUT as usize && OPCODE_POSSESSIFY[repcode] > 0 {
                *tempcode = OPCODE_POSSESSIFY[repcode];
            } else {
                memmove(
                    tempcode.add(1 + LINK_SIZE) as *mut c_void,
                    tempcode as *const c_void,
                    len as usize,
                );
                code = code.add(1 + LINK_SIZE);
                len += (1 + LINK_SIZE) as c_int;
                *tempcode.add(0) = OP_ONCE;
                *code = OP_KET;
                code = code.add(1);
                PUTINC!(code, 0, len as u32);
                PUT(tempcode, 1, len as u32);
            }
        }
    }
    *code_io = code;
    RepRes::Ok
}

unsafe fn compile_repeat(
    meta: u32,
    _meta_arg: u32,
    repeat_min_io: &mut u32,
    repeat_max_io: &mut u32,
    previous_matched_char: bool,
    matched_char: &mut bool,
    code_io: &mut *mut PCRE2_UCHAR,
    previous_io: &mut *mut PCRE2_UCHAR,
    firstcu: &mut u32,
    firstcuflags: &mut u32,
    reqcu: &mut u32,
    reqcuflags: &mut u32,
    zerofirstcu: &mut u32,
    zerofirstcuflags: &mut u32,
    zeroreqcu: &mut u32,
    zeroreqcuflags: &mut u32,
    reqvary_out: &mut u32,
    greedy_default: u32,
    greedy_non_default: u32,
    length_prevgroup_io: &mut PCRE2_SIZE,
    group_return_io: &mut c_int,
    groupsetfirstcu: bool,
    utf: bool,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
    errorcodeptr: *mut c_int,
) -> RepRes {
    let mut repeat_min = *repeat_min_io;
    let mut repeat_max = *repeat_max_io;
    let mut code = *code_io;
    let mut previous = *previous_io;
    let mut length_prevgroup = *length_prevgroup_io;
    let mut group_return = *group_return_io;
    let mut op_type: u32 = 0;
    let mut repeat_type: u32;
    let mut possessive_quantifier: bool;
    let mut mclength: u32 = 0;
    let mut mcbuffer: [PCRE2_UCHAR; 8] = [0; 8];
    let mut tempcode: *mut PCRE2_UCHAR;
    let mut op_previous: u8;

    let mut goto_output_single_repeat = false;
    let mut goto_end_repeat = false;

    if previous_matched_char && repeat_min > 0 {
        *matched_char = true;
    }

    let reqvary = if repeat_min == repeat_max { 0 } else { REQ_VARY };

    if repeat_min == 0 {
        *firstcu = *zerofirstcu;
        *firstcuflags = *zerofirstcuflags;
        *reqcu = *zeroreqcu;
        *reqcuflags = *zeroreqcuflags;
    }

    match meta {
        META_MINMAX_PLUS | META_ASTERISK_PLUS | META_PLUS_PLUS | META_QUERY_PLUS => {
            repeat_type = 0;
            possessive_quantifier = true;
        }
        META_MINMAX_QUERY | META_ASTERISK_QUERY | META_PLUS_QUERY | META_QUERY_QUERY => {
            repeat_type = greedy_non_default;
            possessive_quantifier = false;
        }
        _ => {
            repeat_type = greedy_default;
            possessive_quantifier = false;
        }
    }

    tempcode = previous;
    op_previous = *previous;

    macro_rules! finish {
        () => {{
            *reqvary_out = reqvary;
            *code_io = code;
            *previous_io = previous;
            *length_prevgroup_io = length_prevgroup;
            *group_return_io = group_return;
            *repeat_min_io = repeat_min;
            *repeat_max_io = repeat_max;
            return RepRes::Ok;
        }};
    }

    match op_previous {
        OP_CHAR | OP_CHARI | OP_NOT | OP_NOTI => {
            if repeat_max == 1 && repeat_min == 1 {
                finish!();
            }
            op_type = CHARTYPEOFFSET[(op_previous - OP_CHAR) as usize];

            if utf && NOT_FIRSTCU(*code.sub(1) as u32) {
                let mut lastchar = code.sub(1);
                while (*lastchar & 0xc0) == 0x80 {
                    lastchar = lastchar.sub(1);
                }
                mclength = (code as usize - lastchar as usize) as u32;
                memcpy(mcbuffer.as_mut_ptr() as *mut c_void, lastchar as *const c_void, mclength as usize);
            } else {
                mcbuffer[0] = *code.sub(1);
                mclength = 1;
                if op_previous <= OP_CHARI && repeat_min > 1 {
                    *reqcu = mcbuffer[0] as u32;
                    *reqcuflags = (*cb).req_varyopt;
                    if op_previous == OP_CHARI {
                        *reqcuflags |= REQ_CASELESS;
                    }
                }
            }
            goto_output_single_repeat = true;
        }

        OP_XCLASS | OP_ECLASS | OP_CLASS | OP_NCLASS | OP_REF | OP_REFI | OP_DNREF | OP_DNREFI => {
            if repeat_max == 0 {
                code = previous;
                goto_end_repeat = true;
            } else if repeat_max == 1 && repeat_min == 1 {
                goto_end_repeat = true;
            } else {
                if repeat_min == 0 && repeat_max == REPEAT_UNLIMITED {
                    wr!(code, OP_CRSTAR + repeat_type as u8);
                } else if repeat_min == 1 && repeat_max == REPEAT_UNLIMITED {
                    wr!(code, OP_CRPLUS + repeat_type as u8);
                } else if repeat_min == 0 && repeat_max == 1 {
                    wr!(code, OP_CRQUERY + repeat_type as u8);
                } else {
                    wr!(code, OP_CRRANGE + repeat_type as u8);
                    PUT2INC!(code, 0, repeat_min);
                    if repeat_max == REPEAT_UNLIMITED {
                        repeat_max = 0;
                    }
                    PUT2INC!(code, 0, repeat_max);
                }
            }
        }

        OP_RECURSE => {
            if repeat_max == 1 && repeat_min == 1 && !possessive_quantifier {
                goto_end_repeat = true;
            } else {
                let mut fall_to_bra = true;
                if repeat_min > 0 && (repeat_min != 1 || repeat_max != REPEAT_UNLIMITED) {
                    let mut replicate = repeat_min as c_int;
                    if repeat_min == repeat_max {
                        replicate -= 1;
                    }

                    if !lengthptr.is_null() {
                        let mut delta: PCRE2_SIZE = 0;
                        if crate::pcre2_chkdint::_pcre2_ckd_smul_8(
                            &mut delta, replicate, length_prevgroup as i32,
                        ) != FALSE
                            || OFLOW_MAX - *lengthptr < delta
                        {
                            *errorcodeptr = ERR20;
                            return RepRes::Error;
                        }
                        *lengthptr += delta;
                    } else {
                        for _ in 0..replicate {
                            memcpy(code as *mut c_void, previous as *const c_void, length_prevgroup as usize);
                            previous = code;
                            code = code.add(length_prevgroup as usize);
                        }
                    }

                    if repeat_min == repeat_max {
                        finish!();
                    }
                    if repeat_max != REPEAT_UNLIMITED {
                        repeat_max -= repeat_min;
                    }
                    repeat_min = 0;
                }
                let _ = fall_to_bra;

                let length = if !lengthptr.is_null() {
                    (1 + LINK_SIZE) as PCRE2_SIZE
                } else {
                    length_prevgroup
                };

                memmove(
                    previous.add(1 + LINK_SIZE) as *mut c_void,
                    previous as *const c_void,
                    length as usize,
                );
                op_previous = OP_BRA;
                *previous = OP_BRA;
                PUT(previous, 1, (1 + LINK_SIZE + length as usize) as u32);
                *previous.add(1 + LINK_SIZE + length as usize) = OP_KET;
                PUT(previous, 2 + LINK_SIZE + length as usize, (1 + LINK_SIZE + length as usize) as u32);
                code = code.add(2 + 2 * LINK_SIZE);
                length_prevgroup += (2 + 2 * LINK_SIZE) as PCRE2_SIZE;
                group_return = -1;

                match compile_bracket_repeat(
                    &mut code, &mut previous, op_previous, &mut repeat_min, &mut repeat_max,
                    repeat_type, &mut possessive_quantifier, &mut length_prevgroup, group_return,
                    firstcu, firstcuflags, reqcu, reqcuflags, groupsetfirstcu, cb, lengthptr,
                    errorcodeptr,
                ) {
                    RepRes::Ok => {
                        match possessive_tail(&mut code, tempcode, possessive_quantifier, utf) {
                            RepRes::Ok => {}
                            _ => return RepRes::Error,
                        }
                    }
                    RepRes::EndRepeat => {}
                    RepRes::Error => return RepRes::Error,
                }
                finish!();
            }
        }

        OP_ASSERT | OP_ASSERT_NOT | OP_ASSERT_NA | OP_ASSERTBACK | OP_ASSERTBACK_NOT
        | OP_ASSERTBACK_NA | OP_ASSERT_SCS | OP_ONCE | OP_SCRIPT_RUN | OP_BRA | OP_CBRA
        | OP_COND => {
            match compile_bracket_repeat(
                &mut code, &mut previous, op_previous, &mut repeat_min, &mut repeat_max,
                repeat_type, &mut possessive_quantifier, &mut length_prevgroup, group_return,
                firstcu, firstcuflags, reqcu, reqcuflags, groupsetfirstcu, cb, lengthptr,
                errorcodeptr,
            ) {
                RepRes::Ok => {}
                RepRes::EndRepeat => goto_end_repeat = true,
                RepRes::Error => return RepRes::Error,
            }
        }

        _ => {
            if op_previous >= OP_EODN || op_previous <= OP_WORD_BOUNDARY {
                *errorcodeptr = ERR10;
                return RepRes::Error;
            }

            if repeat_max == 1 && repeat_min == 1 {
                goto_end_repeat = true;
            } else {
                op_type = (OP_TYPESTAR - OP_STAR) as u32;
                mclength = 0;

                let prop_type: c_int;
                let prop_value: c_int;
                if op_previous == OP_PROP || op_previous == OP_NOTPROP {
                    prop_type = *previous.add(1) as c_int;
                    prop_value = *previous.add(2) as c_int;
                } else {
                    prop_type = -1;
                    prop_value = -1;
                }

                match output_single_repeat(
                    &mut code, previous, op_previous, op_type, prop_type, prop_value, &mut mcbuffer,
                    mclength, &mut repeat_min, &mut repeat_max, repeat_type,
                ) {
                    RepRes::Ok => {}
                    RepRes::EndRepeat => goto_end_repeat = true,
                    RepRes::Error => return RepRes::Error,
                }
            }
        }
    }

    if goto_output_single_repeat {
        let prop_type: c_int = -1;
        let prop_value: c_int = -1;
        match output_single_repeat(
            &mut code, previous, op_previous, op_type, prop_type, prop_value, &mut mcbuffer,
            mclength, &mut repeat_min, &mut repeat_max, repeat_type,
        ) {
            RepRes::Ok => {}
            RepRes::EndRepeat => goto_end_repeat = true,
            RepRes::Error => return RepRes::Error,
        }
    }

    if !goto_end_repeat {
        match possessive_tail(&mut code, tempcode, possessive_quantifier, utf) {
            RepRes::Ok => {}
            _ => return RepRes::Error,
        }
    }

    *reqvary_out = reqvary;
    *code_io = code;
    *previous_io = previous;
    *length_prevgroup_io = length_prevgroup;
    *group_return_io = group_return;
    *repeat_min_io = repeat_min;
    *repeat_max_io = repeat_max;
    RepRes::Ok
}

#[inline]
fn NOT_FIRSTCU(c: u32) -> bool {
    (c & 0xc0) == 0x80
}

// ---------------------------------------------------------------------------
// Compile regex: a sequence of alternatives : compile_regex
// ---------------------------------------------------------------------------

unsafe fn compile_regex(
    mut options: u32,
    mut xoptions: u32,
    codeptr: *mut *mut PCRE2_UCHAR,
    pptrptr: *mut *mut u32,
    errorcodeptr: *mut c_int,
    skipunits: u32,
    firstcuptr: *mut u32,
    firstcuflagsptr: *mut u32,
    reqcuptr: *mut u32,
    reqcuflagsptr: *mut u32,
    bcptr: *mut branch_chain,
    mut open_caps: *mut open_capitem,
    cb: *mut compile_block,
    lengthptr: *mut PCRE2_SIZE,
) -> c_int {
    let mut code = *codeptr;
    let mut last_branch = code;
    let start_bracket = code;
    let lookbehind: bool;
    let mut capitem = open_capitem {
        next: ptr::null_mut(),
        number: 0,
        assert_depth: 0,
    };
    let mut capnumber: c_int = 0;
    let mut okreturn: c_int = 1;
    let mut pptr = *pptrptr;
    let mut firstcu: u32;
    let mut reqcu: u32;
    let mut lookbehindlength: u32;
    let mut lookbehindminlength: u32;
    let mut firstcuflags: u32;
    let mut reqcuflags: u32;
    let mut length: PCRE2_SIZE;
    let mut bc = branch_chain {
        outer: bcptr,
        current_branch: code,
    };

    if !(*(*cb).cx).stack_guard.is_none()
        && ((*(*cb).cx).stack_guard.unwrap())(
            (*cb).parens_depth as u32,
            (*(*cb).cx).stack_guard_data,
        ) != 0
    {
        *errorcodeptr = ERR33;
        (*cb).erroroffset = 0;
        return 0;
    }

    firstcu = 0;
    reqcu = 0;
    firstcuflags = REQ_UNSET;
    reqcuflags = REQ_UNSET;

    length = (2 + 2 * LINK_SIZE + skipunits as usize) as PCRE2_SIZE;

    lookbehind = *code == OP_ASSERTBACK
        || *code == OP_ASSERTBACK_NOT
        || *code == OP_ASSERTBACK_NA;

    if lookbehind {
        lookbehindlength = META_DATA(*pptr.sub(1));
        lookbehindminlength = *pptr;
        pptr = pptr.add(SIZEOFFSET);
    } else {
        lookbehindlength = 0;
        lookbehindminlength = 0;
    }

    if *code == OP_CBRA {
        capnumber = GET2(code, 1 + LINK_SIZE) as c_int;
        capitem.number = capnumber as u16;
        capitem.next = open_caps;
        capitem.assert_depth = (*cb).assert_depth;
        open_caps = &mut capitem;
    }

    PUT(code, 1, 0);
    code = code.add(1 + LINK_SIZE + skipunits as usize);

    loop {
        let branch_return: c_int;
        let mut branchfirstcu: u32 = 0;
        let mut branchreqcu: u32 = 0;
        let mut branchfirstcuflags: u32 = REQ_UNSET;
        let mut branchreqcuflags: u32 = REQ_UNSET;

        if lookbehind && lookbehindlength > 0 {
            if lookbehindminlength == LOOKBEHIND_MAX as u32
                || lookbehindminlength == lookbehindlength
            {
                wr!(code, OP_REVERSE);
                PUT2INC!(code, 0, lookbehindlength);
                length += (1 + IMM2_SIZE) as PCRE2_SIZE;
            } else {
                wr!(code, OP_VREVERSE);
                PUT2INC!(code, 0, lookbehindminlength);
                PUT2INC!(code, 0, lookbehindlength);
                length += (1 + 2 * IMM2_SIZE) as PCRE2_SIZE;
            }
        }

        branch_return = compile_branch(
            &mut options,
            &mut xoptions,
            &mut code,
            &mut pptr,
            errorcodeptr,
            &mut branchfirstcu,
            &mut branchfirstcuflags,
            &mut branchreqcu,
            &mut branchreqcuflags,
            &mut bc,
            open_caps,
            cb,
            if lengthptr.is_null() { ptr::null_mut() } else { &mut length },
        );
        if branch_return == 0 {
            return 0;
        }

        if branch_return < 0 {
            okreturn = -1;
        }

        if lengthptr.is_null() {
            if *last_branch != OP_ALT {
                firstcu = branchfirstcu;
                firstcuflags = branchfirstcuflags;
                reqcu = branchreqcu;
                reqcuflags = branchreqcuflags;
            } else {
                if firstcuflags != branchfirstcuflags || firstcu != branchfirstcu {
                    if firstcuflags < REQ_NONE {
                        if reqcuflags >= REQ_NONE {
                            reqcu = firstcu;
                            reqcuflags = firstcuflags;
                        }
                    }
                    firstcuflags = REQ_NONE;
                }

                if firstcuflags >= REQ_NONE
                    && branchfirstcuflags < REQ_NONE
                    && branchreqcuflags >= REQ_NONE
                {
                    branchreqcu = branchfirstcu;
                    branchreqcuflags = branchfirstcuflags;
                }

                if ((reqcuflags & !REQ_VARY) != (branchreqcuflags & !REQ_VARY))
                    || reqcu != branchreqcu
                {
                    reqcuflags = REQ_NONE;
                } else {
                    reqcu = branchreqcu;
                    reqcuflags |= branchreqcuflags;
                }
            }
        }

        if META_CODE(*pptr) != META_ALT {
            if lengthptr.is_null() {
                let mut branch_length = (code as usize - last_branch as usize) as u32;
                loop {
                    let prev_length = GET(last_branch, 1);
                    PUT(last_branch, 1, branch_length);
                    branch_length = prev_length;
                    last_branch = last_branch.sub(branch_length as usize);
                    if branch_length == 0 {
                        break;
                    }
                }
            }

            *code = OP_KET;
            PUT(code, 1, (code as usize - start_bracket as usize) as u32);
            code = code.add(1 + LINK_SIZE);

            *codeptr = code;
            *pptrptr = pptr;
            *firstcuptr = firstcu;
            *firstcuflagsptr = firstcuflags;
            *reqcuptr = reqcu;
            *reqcuflagsptr = reqcuflags;
            if !lengthptr.is_null() {
                if OFLOW_MAX - *lengthptr < length {
                    *errorcodeptr = ERR20;
                    return 0;
                }
                *lengthptr += length;
            }
            return okreturn;
        }

        if !lengthptr.is_null() {
            code = (*codeptr).add(1 + LINK_SIZE + skipunits as usize);
            length += (1 + LINK_SIZE) as PCRE2_SIZE;
        } else {
            *code = OP_ALT;
            PUT(code, 1, (code as usize - last_branch as usize) as u32);
            last_branch = code;
            bc.current_branch = last_branch;
            code = code.add(1 + LINK_SIZE);
        }

        lookbehindlength = META_DATA(*pptr);
        pptr = pptr.add(1);
    }
}

// ---------------------------------------------------------------------------
// External function to compile a pattern : pcre2_compile
// ---------------------------------------------------------------------------

const RSCAN_CACHE_SIZE: usize = 8;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_compile_8(
    mut pattern: PCRE2_SPTR,
    mut patlen: PCRE2_SIZE,
    mut options: u32,
    errorptr: *mut c_int,
    erroroffset: *mut PCRE2_SIZE,
    mut ccontext: *mut pcre2_compile_context,
) -> *mut pcre2_code {
    let mut utf: bool;
    let mut ucp: bool;
    let mut has_lookbehind: BOOL = FALSE;
    let zero_terminated: bool;
    let mut re: *mut pcre2_real_code = ptr::null_mut();
    let mut cb: compile_block = core::mem::zeroed();
    let tables: *const u8;

    let mut null_str: [PCRE2_UCHAR; 1] = [0xcd];
    let mut code: *mut PCRE2_UCHAR;
    let mut codestart: *mut PCRE2_UCHAR;
    let mut ptr: PCRE2_SPTR;
    let mut pptr: *mut u32;

    let mut length: PCRE2_SIZE = 1;
    let usedlength: PCRE2_SIZE;
    let mut re_blocksize: PCRE2_SIZE;
    let parsed_size_needed: isize;

    let mut firstcuflags: u32;
    let mut reqcuflags: u32;
    let mut firstcu: u32;
    let mut reqcu: u32;
    let mut setflags: u32 = 0;
    let mut xoptions: u32;

    let mut skipatstart: usize;
    let mut limit_heap: u32 = UINT32_MAX;
    let mut limit_match: u32 = UINT32_MAX;
    let mut limit_depth: u32 = UINT32_MAX;

    let mut newline: c_int = 0;
    let mut bsr: c_int = 0;
    let mut errorcode: c_int = 0;
    let regexrc: c_int;

    let mut i: u32;

    let mut optim_flags: u32 = if !ccontext.is_null() {
        (*ccontext).optimization_flags
    } else {
        PCRE2_OPTIMIZATION_ALL
    };

    let mut stack_groupinfo: [u32; GROUPINFO_DEFAULT_SIZE] = [0; GROUPINFO_DEFAULT_SIZE];
    let mut stack_parsed_pattern: [u32; PARSED_PATTERN_DEFAULT_SIZE] = [0; PARSED_PATTERN_DEFAULT_SIZE];
    let mut named_groups: [named_group; NAMED_GROUP_LIST_SIZE as usize] =
        core::mem::zeroed();

    let mut c16workspace: [u32; C16_WORK_SIZE] = [0; C16_WORK_SIZE];
    let cworkspace = c16workspace.as_mut_ptr() as *mut PCRE2_UCHAR;

    // ---- Check arguments ----
    if errorptr.is_null() {
        if !erroroffset.is_null() {
            *erroroffset = 0;
        }
        return ptr::null_mut();
    }
    if erroroffset.is_null() {
        *errorptr = ERR120;
        return ptr::null_mut();
    }
    *errorptr = ERR0;
    *erroroffset = 0;

    if pattern.is_null() {
        if patlen == 0 {
            pattern = null_str.as_ptr();
        } else {
            *errorptr = ERR16;
            return ptr::null_mut();
        }
    }

    if ccontext.is_null() {
        ccontext = core::ptr::addr_of_mut!(
            crate::pcre2_context::_pcre2_default_compile_context_8
        ) as *mut pcre2_compile_context;
    }

    if (options & PCRE2_MATCH_INVALID_UTF) != 0 {
        options |= PCRE2_UTF;
    }

    if (options & !PUBLIC_COMPILE_OPTIONS) != 0
        || ((*ccontext).extra_options & !PUBLIC_COMPILE_EXTRA_OPTIONS) != 0
    {
        *errorptr = ERR17;
        return ptr::null_mut();
    }

    if (options & PCRE2_LITERAL) != 0
        && ((options & !PUBLIC_LITERAL_COMPILE_OPTIONS) != 0
            || ((*ccontext).extra_options & !PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS) != 0)
    {
        *errorptr = ERR92;
        return ptr::null_mut();
    }

    zero_terminated = patlen == PCRE2_ZERO_TERMINATED;
    if zero_terminated {
        patlen = _pcre2_strlen(pattern);
    }

    if patlen > (*ccontext).max_pattern_length {
        *errorptr = ERR88;
        return ptr::null_mut();
    }

    if (options & PCRE2_NO_AUTO_POSSESS) != 0 {
        optim_flags &= !PCRE2_OPTIM_AUTO_POSSESS;
    }
    if (options & PCRE2_NO_DOTSTAR_ANCHOR) != 0 {
        optim_flags &= !PCRE2_OPTIM_DOTSTAR_ANCHOR;
    }
    if (options & PCRE2_NO_START_OPTIMIZE) != 0 {
        optim_flags &= !PCRE2_OPTIM_START_OPTIMIZE;
    }

    // ---- Initialize compile data ----
    tables = if !(*ccontext).tables.is_null() {
        (*ccontext).tables
    } else {
        _pcre2_default_tables_8.as_ptr()
    };

    cb.lcc = tables.add(lcc_offset);
    cb.fcc = tables.add(fcc_offset);
    cb.cbits = tables.add(cbits_offset);
    cb.ctypes = tables.add(ctypes_offset);

    cb.assert_depth = 0;
    cb.bracount = 0;
    cb.cx = ccontext;
    cb.dupnames = FALSE;
    cb.end_pattern = pattern.add(patlen);
    cb.erroroffset = 0;
    cb.external_flags = 0;
    cb.external_options = options;
    cb.groupinfo = stack_groupinfo.as_mut_ptr();
    cb.had_recurse = FALSE;
    cb.lastcapture = 0;
    cb.max_lookbehind = 0;
    cb.max_varlookbehind = (*ccontext).max_varlookbehind;
    cb.name_entry_size = 0;
    cb.name_table = ptr::null_mut();
    cb.named_groups = named_groups.as_mut_ptr();
    cb.named_group_list_size = NAMED_GROUP_LIST_SIZE;
    cb.names_found = 0;
    cb.parens_depth = 0;
    cb.parsed_pattern = stack_parsed_pattern.as_mut_ptr();
    cb.req_varyopt = 0;
    cb.start_code = cworkspace;
    cb.start_pattern = pattern;
    cb.start_workspace = cworkspace;
    cb.workspace_size = COMPILE_WORK_SIZE;
    cb.first_data = ptr::null_mut();
    cb.last_data = ptr::null_mut();
    cb.char_lists_size = 0;

    cb.top_backref = 0;
    cb.backref_map = 0;

    i = 0;
    while i < 10 {
        cb.small_ref_offset[i as usize] = PCRE2_UNSET;
        i += 1;
    }

    // Macro to route to error handling.
    // We use closures-free style: set errorcode and jump via labeled blocks.
    // The C code has HAD_EARLY_ERROR, HAD_CB_ERROR, HAD_ERROR, EXIT.
    // Implement via a state machine.

    xoptions = (*ccontext).extra_options;
    ptr = pattern;
    skipatstart = 0;

    // The overall control uses these "goto targets". We inline via labeled blocks.
    // err_ptr_from: 0 = none, 1 = HAD_EARLY_ERROR (compute offset from ptr),
    //               2 = HAD_CB_ERROR (offset from cb.erroroffset),
    //               3 = HAD_ERROR (offset already in *erroroffset)
    let mut err_kind: i32 = -1; // -1 = success so far

    'compile: {
        if (options & PCRE2_LITERAL) == 0 {
            'psoscan: while patlen - skipatstart >= 2
                && *ptr.add(skipatstart) as u32 == CHAR_LEFT_PARENTHESIS
                && *ptr.add(skipatstart + 1) as u32 == CHAR_ASTERISK
            {
                let mut matched_any = false;
                let mut ii = 0usize;
                while ii < PSO_LIST.len() {
                    let p = &PSO_LIST[ii];
                    if patlen - skipatstart - 2 >= p.length as usize
                        && _pcre2_strncmp_c8(
                            ptr.add(skipatstart + 2),
                            p.name.as_ptr() as *const c_char,
                            p.length as usize,
                        ) == 0
                    {
                        skipatstart += p.length as usize + 2;
                        match p.typ {
                            PSO_OPT => cb.external_options |= p.value,
                            PSO_XOPT => xoptions |= p.value,
                            PSO_FLG => setflags |= p.value,
                            PSO_NL => {
                                newline = p.value as c_int;
                                setflags |= PCRE2_NL_SET;
                            }
                            PSO_BSR => {
                                bsr = p.value as c_int;
                                setflags |= PCRE2_BSR_SET;
                            }
                            PSO_LIMM | PSO_LIMD | PSO_LIMH => {
                                let mut cc: u32 = 0;
                                let mut pp = skipatstart;
                                while pp < patlen && IS_DIGIT(*ptr.add(pp) as u32) {
                                    if cc > UINT32_MAX / 10 - 1 {
                                        break;
                                    }
                                    cc = cc * 10 + (*ptr.add(pp) as u32 - CHAR_0);
                                    pp += 1;
                                }
                                if pp >= patlen
                                    || pp == skipatstart
                                    || *ptr.add(pp) as u32 != CHAR_RIGHT_PARENTHESIS
                                {
                                    errorcode = ERR60;
                                    ptr = ptr.add(pp);
                                    utf = false;
                                    err_kind = 1;
                                    break 'compile;
                                }
                                if p.typ == PSO_LIMH {
                                    limit_heap = cc;
                                } else if p.typ == PSO_LIMM {
                                    limit_match = cc;
                                } else {
                                    limit_depth = cc;
                                }
                                pp += 1;
                                skipatstart = pp;
                            }
                            PSO_OPTMZ => {
                                optim_flags &= !p.value;
                                match p.value {
                                    PCRE2_OPTIM_AUTO_POSSESS => {
                                        cb.external_options |= PCRE2_NO_AUTO_POSSESS;
                                    }
                                    PCRE2_OPTIM_DOTSTAR_ANCHOR => {
                                        cb.external_options |= PCRE2_NO_DOTSTAR_ANCHOR;
                                    }
                                    PCRE2_OPTIM_START_OPTIMIZE => {
                                        cb.external_options |= PCRE2_NO_START_OPTIMIZE;
                                    }
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                        matched_any = true;
                        break; // out of table scan
                    }
                    ii += 1;
                }
                if ii >= PSO_LIST.len() && !matched_any {
                    break 'psoscan;
                }
            }
        }

        ptr = ptr.add(skipatstart);

        utf = (cb.external_options & PCRE2_UTF) != 0;
        if utf {
            if (options & PCRE2_NEVER_UTF) != 0 {
                errorcode = ERR74;
                err_kind = 1;
                break 'compile;
            }
            if (options & PCRE2_NO_UTF_CHECK) == 0 {
                errorcode = crate::pcre2_valid_utf::_pcre2_valid_utf_8(pattern, patlen, erroroffset);
                if errorcode != 0 {
                    err_kind = 3;
                    break 'compile;
                }
            }
        }

        ucp = (cb.external_options & PCRE2_UCP) != 0;
        if ucp && (cb.external_options & PCRE2_NEVER_UCP) != 0 {
            errorcode = ERR75;
            err_kind = 1;
            break 'compile;
        }

        if (xoptions & PCRE2_EXTRA_TURKISH_CASING) != 0 {
            if !utf && !ucp {
                errorcode = ERR104;
                err_kind = 1;
                break 'compile;
            }
            if !utf {
                errorcode = ERR105;
                err_kind = 1;
                break 'compile;
            }
            if (xoptions & PCRE2_EXTRA_CASELESS_RESTRICT) != 0 {
                errorcode = ERR106;
                err_kind = 1;
                break 'compile;
            }
        }

        if bsr == 0 {
            bsr = (*ccontext).bsr_convention as c_int;
        }

        if newline == 0 {
            newline = (*ccontext).newline_convention as c_int;
        }
        cb.nltype = NLTYPE_FIXED;
        match newline as u32 {
            PCRE2_NEWLINE_CR => {
                cb.nllen = 1;
                cb.nl[0] = CHAR_CR as u8;
            }
            PCRE2_NEWLINE_LF => {
                cb.nllen = 1;
                cb.nl[0] = CHAR_NL as u8;
            }
            PCRE2_NEWLINE_NUL => {
                cb.nllen = 1;
                cb.nl[0] = CHAR_NUL as u8;
            }
            PCRE2_NEWLINE_CRLF => {
                cb.nllen = 2;
                cb.nl[0] = CHAR_CR as u8;
                cb.nl[1] = CHAR_NL as u8;
            }
            PCRE2_NEWLINE_ANY => {
                cb.nltype = NLTYPE_ANY;
            }
            PCRE2_NEWLINE_ANYCRLF => {
                cb.nltype = NLTYPE_ANYCRLF;
            }
            _ => {
                errorcode = ERR56;
                err_kind = 1;
                break 'compile;
            }
        }

        // Parsed pattern buffer sizing.
        parsed_size_needed = max_parsed_pattern(ptr, cb.end_pattern, utf, options);
        let mut psize = parsed_size_needed;

        if ((*ccontext).extra_options & (PCRE2_EXTRA_MATCH_WORD | PCRE2_EXTRA_MATCH_LINE)) != 0 {
            psize += 4;
        }
        if (options & PCRE2_AUTO_CALLOUT) != 0 {
            psize += 4;
        }
        psize += 1;

        if psize > PARSED_PATTERN_DEFAULT_SIZE as isize {
            let heap_parsed_pattern = ((*ccontext).memctl.malloc.unwrap())(
                psize as usize * core::mem::size_of::<u32>(),
                (*ccontext).memctl.memory_data,
            ) as *mut u32;
            if heap_parsed_pattern.is_null() {
                *errorptr = ERR21;
                err_kind = 4; // EXIT directly
                break 'compile;
            }
            cb.parsed_pattern = heap_parsed_pattern;
        }
        cb.parsed_pattern_end = cb.parsed_pattern.add(psize as usize);

        errorcode = parse_regex(ptr, cb.external_options, xoptions, &mut has_lookbehind, &mut cb);
        if errorcode != 0 {
            err_kind = 2;
            break 'compile;
        }

        if has_lookbehind != FALSE {
            let mut loopcount: c_int = 0;
            if cb.bracount >= (GROUPINFO_DEFAULT_SIZE as u32) / 2 {
                cb.groupinfo = ((*ccontext).memctl.malloc.unwrap())(
                    (2 * (cb.bracount + 1)) as usize * core::mem::size_of::<u32>(),
                    (*ccontext).memctl.memory_data,
                ) as *mut u32;
                if cb.groupinfo.is_null() {
                    errorcode = ERR21;
                    cb.erroroffset = 0;
                    err_kind = 2;
                    break 'compile;
                }
            }
            memset(
                cb.groupinfo as *mut c_void,
                0,
                (2 * cb.bracount as usize + 1) * core::mem::size_of::<u32>(),
            );
            errorcode = check_lookbehinds(
                cb.parsed_pattern, ptr::null_mut(), ptr::null_mut(), &mut cb, &mut loopcount,
            );
            if errorcode != 0 {
                err_kind = 2;
                break 'compile;
            }
        }

        // Pre-compile: accumulate length.
        cb.erroroffset = patlen;
        pptr = cb.parsed_pattern;
        code = cworkspace;
        *code = OP_BRA;

        firstcu = 0;
        firstcuflags = 0;
        reqcu = 0;
        reqcuflags = 0;
        compile_regex(
            cb.external_options,
            xoptions,
            &mut code,
            &mut pptr,
            &mut errorcode,
            0,
            &mut firstcu,
            &mut firstcuflags,
            &mut reqcu,
            &mut reqcuflags,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut cb,
            &mut length,
        );

        if errorcode != 0 {
            err_kind = 2;
            break 'compile;
        }

        if length > MAX_PATTERN_SIZE
            || MAX_PATTERN_SIZE - length < (cb.char_lists_size / 1)
        {
            errorcode = ERR20;
            cb.erroroffset = 0;
            err_kind = 2;
            break 'compile;
        }

        re_blocksize = (cb.names_found as PCRE2_SIZE) * (cb.name_entry_size as PCRE2_SIZE);

        if cb.char_lists_size != 0 {
            re_blocksize = (re_blocksize + (core::mem::size_of::<u32>() - 1))
                & !(core::mem::size_of::<u32>() - 1);
            re_blocksize += cb.char_lists_size;
        }

        re_blocksize += length;

        if re_blocksize > (*ccontext).max_pattern_compiled_length {
            errorcode = ERR101;
            cb.erroroffset = 0;
            err_kind = 2;
            break 'compile;
        }

        re_blocksize += core::mem::size_of::<pcre2_real_code>();
        re = ((*ccontext).memctl.malloc.unwrap())(re_blocksize, (*ccontext).memctl.memory_data)
            as *mut pcre2_real_code;
        if re.is_null() {
            errorcode = ERR21;
            cb.erroroffset = 0;
            err_kind = 2;
            break 'compile;
        }

        // zero last 8 bytes of the header
        {
            let base = (re as *mut u8).add(core::mem::size_of::<pcre2_real_code>() - 8);
            memset(base as *mut c_void, 0, 8);
        }
        (*re).memctl = (*ccontext).memctl;
        (*re).tables = tables;
        (*re).executable_jit = ptr::null_mut();
        memset((*re).start_bitmap.as_mut_ptr() as *mut c_void, 0, 32);
        (*re).blocksize = re_blocksize;
        (*re).code_start = re_blocksize - length;
        (*re).magic_number = MAGIC_NUMBER;
        (*re).compile_options = options;
        (*re).overall_options = cb.external_options;
        (*re).extra_options = xoptions;
        (*re).flags = (PCRE2_CODE_UNIT_WIDTH / 8) | cb.external_flags | setflags;
        (*re).limit_heap = limit_heap;
        (*re).limit_match = limit_match;
        (*re).limit_depth = limit_depth;
        (*re).first_codeunit = 0;
        (*re).last_codeunit = 0;
        (*re).bsr_convention = bsr as u16;
        (*re).newline_convention = newline as u16;
        (*re).max_lookbehind = 0;
        (*re).minlength = 0;
        (*re).top_bracket = 0;
        (*re).top_backref = 0;
        (*re).name_entry_size = cb.name_entry_size;
        (*re).name_count = cb.names_found;
        (*re).optimization_flags = optim_flags;

        codestart = (re as *mut u8).add((*re).code_start) as *mut PCRE2_UCHAR;

        cb.parens_depth = 0;
        cb.assert_depth = 0;
        cb.lastcapture = 0;
        cb.name_table = (re as *mut u8).add(core::mem::size_of::<pcre2_real_code>())
            as *mut PCRE2_UCHAR;
        cb.start_code = codestart;
        cb.req_varyopt = 0;
        cb.had_accept = FALSE;
        cb.had_pruneorskip = FALSE;
        cb.char_lists_size = 0;

        if cb.names_found > 0 {
            let mut ng = cb.named_groups;
            let mut tablecount: u32 = 0;
            i = 0;
            while i < cb.names_found as u32 {
                if (*ng).length > 0 {
                    tablecount = crate::pcre2_compile_cgroup::_pcre2_compile_add_name_to_table8(
                        &mut cb, ng, tablecount,
                    );
                }
                i += 1;
                ng = ng.add(1);
            }
        }

        pptr = cb.parsed_pattern;
        code = codestart;
        *code = OP_BRA;
        regexrc = compile_regex(
            (*re).overall_options,
            (*re).extra_options,
            &mut code,
            &mut pptr,
            &mut errorcode,
            0,
            &mut firstcu,
            &mut firstcuflags,
            &mut reqcu,
            &mut reqcuflags,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut cb,
            ptr::null_mut(),
        );
        if regexrc < 0 {
            (*re).flags |= PCRE2_MATCH_EMPTY;
        }
        (*re).top_bracket = cb.bracount as u16;
        (*re).top_backref = cb.top_backref as u16;
        (*re).max_lookbehind = cb.max_lookbehind as u16;

        if cb.had_accept != FALSE {
            reqcu = 0;
            reqcuflags = REQ_NONE;
            (*re).flags |= PCRE2_HASACCEPT;
        }

        *code = OP_END;
        code = code.add(1);
        usedlength = (code as usize - codestart as usize) as PCRE2_SIZE;
        if usedlength > length {
            errorcode = ERR23;
            cb.erroroffset = 0;
            err_kind = 2;
            break 'compile;
        }

        (*re).blocksize -= length - usedlength;

        if errorcode == 0 && cb.had_recurse != FALSE {
            let mut rcode: *mut PCRE2_UCHAR;
            let mut rgroup: PCRE2_SPTR;
            let mut ccount: u32 = 0;
            let mut start: c_int = RSCAN_CACHE_SIZE as c_int;
            let mut rc: [recurse_cache; RSCAN_CACHE_SIZE] = core::mem::zeroed();

            rcode = find_recurse(codestart, utf);
            while !rcode.is_null() {
                let mut p: c_int;
                let groupnumber = GET(rcode, 1) as c_int;
                if groupnumber == 0 {
                    rgroup = codestart;
                } else {
                    let mut search_from = codestart as PCRE2_SPTR;
                    rgroup = ptr::null();
                    i = 0;
                    p = start;
                    while i < ccount {
                        if groupnumber == rc[p as usize].groupnumber {
                            rgroup = rc[p as usize].group;
                            break;
                        }
                        if groupnumber > rc[p as usize].groupnumber {
                            search_from = rc[p as usize].group;
                        }
                        i += 1;
                        p = (p + 1) & 7;
                    }

                    if rgroup.is_null() {
                        rgroup = crate::pcre2_find_bracket::_pcre2_find_bracket_8(
                            search_from, utf as BOOL, groupnumber,
                        );
                        if rgroup.is_null() {
                            errorcode = ERR53;
                            break;
                        }

                        start -= 1;
                        if start < 0 {
                            start = RSCAN_CACHE_SIZE as c_int - 1;
                        }
                        rc[start as usize].groupnumber = groupnumber;
                        rc[start as usize].group = rgroup;
                        if (ccount as usize) < RSCAN_CACHE_SIZE {
                            ccount += 1;
                        }
                    }
                }

                PUT(rcode, 1, (rgroup as usize - codestart as usize) as u32);
                rcode = find_recurse(rcode.add(1 + LINK_SIZE), utf);
            }
        }

        if errorcode == 0 && (optim_flags & PCRE2_OPTIM_AUTO_POSSESS) != 0 {
            let temp = codestart;
            let possessify_rc = crate::pcre2_auto_possess::_pcre2_auto_possessify_8(temp, &mut cb);
            if possessify_rc != 0 {
                errorcode = ERR80;
                cb.erroroffset = 0;
            }
        }

        if errorcode != 0 {
            err_kind = 2;
            break 'compile;
        }

        if ((*re).overall_options & PCRE2_ANCHORED) == 0 {
            let dotstar_anchor = (optim_flags & PCRE2_OPTIM_DOTSTAR_ANCHOR) != 0;
            if is_anchored(codestart, 0, &mut cb, 0, FALSE, dotstar_anchor as BOOL) != FALSE {
                (*re).overall_options |= PCRE2_ANCHORED;
            }
        }

        if (optim_flags & PCRE2_OPTIM_START_OPTIMIZE) != 0 {
            let mut minminlength: c_int = 0;
            let study_rc: c_int;

            if firstcuflags >= REQ_NONE {
                let mut assertedcuflags: u32 = 0;
                let assertedcu = find_firstassertedcu(codestart, &mut assertedcuflags, 0);
                if assertedcuflags < REQ_NONE && assertedcu != reqcu {
                    firstcu = assertedcu;
                    firstcuflags = assertedcuflags;
                }
            }

            if firstcuflags < REQ_NONE {
                (*re).first_codeunit = firstcu;
                (*re).flags |= PCRE2_FIRSTSET;
                minminlength += 1;

                if (firstcuflags & REQ_CASELESS) != 0 {
                    if firstcu < 128 || (!utf && !ucp && firstcu < 255) {
                        if *cb.fcc.add(firstcu as usize) as u32 != firstcu {
                            (*re).flags |= PCRE2_FIRSTCASELESS;
                        }
                    } else if ucp && !utf && UCD_OTHERCASE(firstcu) != firstcu {
                        (*re).flags |= PCRE2_FIRSTCASELESS;
                    }
                }
            } else if ((*re).overall_options & PCRE2_ANCHORED) == 0 {
                let dotstar_anchor = (optim_flags & PCRE2_OPTIM_DOTSTAR_ANCHOR) != 0;
                if is_startline(codestart, 0, &mut cb, 0, FALSE, dotstar_anchor as BOOL) != FALSE {
                    (*re).flags |= PCRE2_STARTLINE;
                }
            }

            if reqcuflags < REQ_NONE {
                if ((*re).overall_options & PCRE2_UTF) == 0
                    || firstcuflags >= REQ_NONE
                    || (firstcu & 0x80) == 0
                    || (reqcu & 0x80) == 0
                {
                    minminlength += 1;
                }

                if ((*re).overall_options & PCRE2_ANCHORED) == 0 || (reqcuflags & REQ_VARY) != 0 {
                    (*re).last_codeunit = reqcu;
                    (*re).flags |= PCRE2_LASTSET;

                    if (reqcuflags & REQ_CASELESS) != 0 {
                        if reqcu < 128 || (!utf && !ucp && reqcu < 255) {
                            if *cb.fcc.add(reqcu as usize) as u32 != reqcu {
                                (*re).flags |= PCRE2_LASTCASELESS;
                            }
                        } else if ucp && !utf && UCD_OTHERCASE(reqcu) != reqcu {
                            (*re).flags |= PCRE2_LASTCASELESS;
                        }
                    }
                }
            }

            study_rc = crate::pcre2_study::_pcre2_study_8(re);
            if study_rc != 0 {
                errorcode = ERR31;
                cb.erroroffset = 0;
                err_kind = 2;
                break 'compile;
            }

            if ((*re).flags & PCRE2_FIRSTMAPSET) != 0 && minminlength == 0 {
                minminlength = 1;
            }

            if ((*re).minlength as c_int) < minminlength {
                (*re).minlength = minminlength as u16;
            }
        }

        err_kind = 0; // success
    } // 'compile

    // ---- Error / exit handling ----
    match err_kind {
        0 | 4 => {
            // success or direct-EXIT (err_kind 4 already set *errorptr)
        }
        1 => {
            // HAD_EARLY_ERROR
            *erroroffset = (ptr as usize - pattern as usize) as PCRE2_SIZE;
            *errorptr = errorcode;
            pcre2_code_free_8(re);
            re = ptr::null_mut();
            free_first_data(&mut cb);
        }
        2 => {
            // HAD_CB_ERROR
            ptr = pattern.add(cb.erroroffset);
            *erroroffset = (ptr as usize - pattern as usize) as PCRE2_SIZE;
            *errorptr = errorcode;
            pcre2_code_free_8(re);
            re = ptr::null_mut();
            free_first_data(&mut cb);
        }
        3 => {
            // HAD_ERROR (offset already set by valid_utf)
            *errorptr = errorcode;
            pcre2_code_free_8(re);
            re = ptr::null_mut();
            free_first_data(&mut cb);
        }
        _ => {}
    }

    // EXIT
    if cb.parsed_pattern != stack_parsed_pattern.as_mut_ptr() {
        ((*ccontext).memctl.free.unwrap())(
            cb.parsed_pattern as *mut c_void,
            (*ccontext).memctl.memory_data,
        );
    }
    if cb.named_group_list_size > NAMED_GROUP_LIST_SIZE {
        ((*ccontext).memctl.free.unwrap())(
            cb.named_groups as *mut c_void,
            (*ccontext).memctl.memory_data,
        );
    }
    if cb.groupinfo != stack_groupinfo.as_mut_ptr() {
        ((*ccontext).memctl.free.unwrap())(
            cb.groupinfo as *mut c_void,
            (*ccontext).memctl.memory_data,
        );
    }

    re
}

unsafe fn free_first_data(cb: *mut compile_block) {
    if !(*cb).first_data.is_null() {
        let mut current_data = (*cb).first_data;
        loop {
            let next_data = (*current_data).next;
            ((*(*cb).cx).memctl.free.unwrap())(
                current_data as *mut c_void,
                (*(*cb).cx).memctl.memory_data,
            );
            current_data = next_data;
            if current_data.is_null() {
                break;
            }
        }
    }
}










