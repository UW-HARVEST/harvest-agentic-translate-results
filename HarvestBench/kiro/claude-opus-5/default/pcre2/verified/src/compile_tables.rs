//! Translation of the file-scope tables and small helper functions of
//! `c_src/src/pcre2_compile.c` (roughly lines 60..3050).
//!
//! Built for the 8-bit library with `SUPPORT_UNICODE` (hence
//! `SUPPORT_WIDE_CHARS`), `LINK_SIZE == 2`, no JIT, no EBCDIC, no `PCRE2_DEBUG`.

#![allow(non_snake_case, non_upper_case_globals, unused_parens, dead_code)]

use core::ffi::{c_char, c_int};

use crate::chars::*;
use crate::compile_internal::*;
use crate::internal::*;
use crate::opcodes::*;
use crate::ucp::*;

/* Code parameters. */

pub(crate) const MAX_GROUP_NUMBER: u32 = 65535u32;
pub(crate) const MAX_REPEAT_COUNT: u32 = 65535u32;
pub(crate) const REPEAT_UNLIMITED: u32 = MAX_REPEAT_COUNT + 1;

/* MAX_NAME_SIZE is defined in pcre2_internal.h; it is the maximum length of a
subpattern name. */
pub(crate) const MAX_NAME_SIZE: usize = 128;

/* Values and flags for the unsigned xxcuflags variables that accompany xxcu
variables, which are concerned with first and required code units. */

pub(crate) const REQ_UNSET: u32 = 0xffffffffu32; /* Not yet found anything */
pub(crate) const REQ_NONE: u32 = 0xfffffffeu32; /* Found not fixed character */
pub(crate) const REQ_CASELESS: u32 = 0x00000001u32; /* Code unit in xxcu is caseless */
pub(crate) const REQ_VARY: u32 = 0x00000002u32; /* Code unit is followed by non-literal */

/* These flags are used in the groupinfo vector. */

pub(crate) const GI_SET_FIXED_LENGTH: u32 = 0x80000000u32;
pub(crate) const GI_NOT_FIXED_LENGTH: u32 = 0x40000000u32;
pub(crate) const GI_FIXED_LENGTH_MASK: u32 = 0x0000ffffu32;

/* Miscellaneous compile-time sizes. */

pub(crate) const COMPILE_WORK_SIZE: usize = 3000 * LINK_SIZE;
pub(crate) const GROUPINFO_DEFAULT_SIZE: usize = 256;
pub(crate) const WORK_SIZE_SAFETY_MARGIN: usize = 100;
pub(crate) const NAMED_GROUP_LIST_SIZE: usize = 20;
pub(crate) const PARSED_PATTERN_DEFAULT_SIZE: usize = 1024;
/* OFLOW_MAX = INT_MAX - 20 */
pub(crate) const OFLOW_MAX: c_int = c_int::MAX - 20;

/* This simple test for a decimal digit. */

#[inline]
pub(crate) fn IS_DIGIT(x: u32) -> bool {
    x >= CHAR_0 && x <= CHAR_9
}

/* Table to identify hex digits (non-EBCDIC version). The value in the table is
the binary hex digit value, or 0xff for non-hex digits. */

#[rustfmt::skip]
pub(crate) static xdigitab: [u8; 256] = [
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /*   0-  7 */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /*   8- 15 */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /*  16- 23 */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /*  24- 31 */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /*    - '  */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /*  ( - /  */
  0x00,0x01,0x02,0x03,0x04,0x05,0x06,0x07, /*  0 - 7  */
  0x08,0x09,0xff,0xff,0xff,0xff,0xff,0xff, /*  8 - ?  */
  0xff,0x0a,0x0b,0x0c,0x0d,0x0e,0x0f,0xff, /*  @ - G  */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /*  H - O  */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /*  P - W  */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /*  X - _  */
  0xff,0x0a,0x0b,0x0c,0x0d,0x0e,0x0f,0xff, /*  ` - g  */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /*  h - o  */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /*  p - w  */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /*  x -127 */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /* 128-135 */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /* 136-143 */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /* 144-151 */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /* 152-159 */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /* 160-167 */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /* 168-175 */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /* 176-183 */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /* 184-191 */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /* 192-199 */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /* 200-207 */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /* 208-215 */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /* 216-223 */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /* 224-231 */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /* 232-239 */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /* 240-247 */
  0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff, /* 248-255 */
];

/* XDIGIT(c) -- in 8-bit mode the table is indexed directly. */
#[inline]
pub(crate) fn XDIGIT(c: u32) -> u32 {
    xdigitab[c as usize] as u32
}

/* Table of extra lengths for each of the meta codes. Must be kept in step with
the META_ definitions. */

pub(crate) static meta_extra_lengths: [u8; 73] = [
    0,                          /* META_END */
    0,                          /* META_ALT */
    0,                          /* META_ATOMIC */
    0,                          /* META_BACKREF - more if group is >= 10 */
    (1 + SIZEOFFSET) as u8,     /* META_BACKREF_BYNAME */
    1,                          /* META_BIGVALUE */
    3,                          /* META_CALLOUT_NUMBER */
    (3 + SIZEOFFSET) as u8,     /* META_CALLOUT_STRING */
    0,                          /* META_CAPTURE */
    0,                          /* META_CIRCUMFLEX */
    0,                          /* META_CLASS */
    0,                          /* META_CLASS_EMPTY */
    0,                          /* META_CLASS_EMPTY_NOT */
    0,                          /* META_CLASS_END */
    0,                          /* META_CLASS_NOT */
    0,                          /* META_COND_ASSERT */
    SIZEOFFSET as u8,           /* META_COND_DEFINE */
    (1 + SIZEOFFSET) as u8,     /* META_COND_NAME */
    (1 + SIZEOFFSET) as u8,     /* META_COND_NUMBER */
    (1 + SIZEOFFSET) as u8,     /* META_COND_RNAME */
    (1 + SIZEOFFSET) as u8,     /* META_COND_RNUMBER */
    3,                          /* META_COND_VERSION */
    SIZEOFFSET as u8,           /* META_OFFSET */
    0,                          /* META_SCS */
    1,                          /* META_CAPTURE_NAME */
    1,                          /* META_CAPTURE_NUMBER */
    0,                          /* META_DOLLAR */
    0,                          /* META_DOT */
    0,                          /* META_ESCAPE - one more for ESC_P and ESC_p */
    0,                          /* META_KET */
    0,                          /* META_NOCAPTURE */
    2,                          /* META_OPTIONS */
    1,                          /* META_POSIX */
    1,                          /* META_POSIX_NEG */
    0,                          /* META_RANGE_ESCAPED */
    0,                          /* META_RANGE_LITERAL */
    SIZEOFFSET as u8,           /* META_RECURSE */
    (1 + SIZEOFFSET) as u8,     /* META_RECURSE_BYNAME */
    0,                          /* META_SCRIPT_RUN */
    0,                          /* META_LOOKAHEAD */
    0,                          /* META_LOOKAHEADNOT */
    SIZEOFFSET as u8,           /* META_LOOKBEHIND */
    SIZEOFFSET as u8,           /* META_LOOKBEHINDNOT */
    0,                          /* META_LOOKAHEAD_NA */
    SIZEOFFSET as u8,           /* META_LOOKBEHIND_NA */
    1,                          /* META_MARK - plus the string length */
    0,                          /* META_ACCEPT */
    0,                          /* META_FAIL */
    0,                          /* META_COMMIT */
    1,                          /* META_COMMIT_ARG - plus the string length */
    0,                          /* META_PRUNE */
    1,                          /* META_PRUNE_ARG - plus the string length */
    0,                          /* META_SKIP */
    1,                          /* META_SKIP_ARG - plus the string length */
    0,                          /* META_THEN */
    1,                          /* META_THEN_ARG - plus the string length */
    0,                          /* META_ASTERISK */
    0,                          /* META_ASTERISK_PLUS */
    0,                          /* META_ASTERISK_QUERY */
    0,                          /* META_PLUS */
    0,                          /* META_PLUS_PLUS */
    0,                          /* META_PLUS_QUERY */
    0,                          /* META_QUERY */
    0,                          /* META_QUERY_PLUS */
    0,                          /* META_QUERY_QUERY */
    2,                          /* META_MINMAX */
    2,                          /* META_MINMAX_PLUS */
    2,                          /* META_MINMAX_QUERY */
    0,                          /* META_ECLASS_AND */
    0,                          /* META_ECLASS_OR */
    0,                          /* META_ECLASS_SUB */
    0,                          /* META_ECLASS_XOR */
    0,                          /* META_ECLASS_NOT */
];

/* Types for skipping parts of a parsed pattern. */

pub(crate) const PSKIP_ALT: c_int = 0;
pub(crate) const PSKIP_CLASS: c_int = 1;
pub(crate) const PSKIP_KET: c_int = 2;

/* Table for handling alphanumeric escaped characters. Positive returns are
simple data values; negative values are for special things like \d and so on.
Zero means further processing is needed (for things like \x), or the escape is
invalid. This is the "normal" table for ASCII systems, running from '0' to 'z'. */

pub(crate) const ESCAPES_FIRST: u32 = CHAR_0;
pub(crate) const ESCAPES_LAST: u32 = CHAR_z;

/* UPPER_CASE(c) for the ASCII table: (c - 32). */
#[inline]
pub(crate) fn UPPER_CASE(c: u32) -> u32 {
    c - 32
}

#[rustfmt::skip]
pub(crate) static escapes: [i16; (ESCAPES_LAST - ESCAPES_FIRST + 1) as usize] = [
    /* 0 */ 0,                              /* 1 */ 0,
    /* 2 */ 0,                              /* 3 */ 0,
    /* 4 */ 0,                              /* 5 */ 0,
    /* 6 */ 0,                              /* 7 */ 0,
    /* 8 */ 0,                              /* 9 */ 0,
    /* : */ (ESCAPES_FIRST + 0x0a) as i16,  /* ; */ (ESCAPES_FIRST + 0x0b) as i16,
    /* < */ (ESCAPES_FIRST + 0x0c) as i16,  /* = */ (ESCAPES_FIRST + 0x0d) as i16,
    /* > */ (ESCAPES_FIRST + 0x0e) as i16,  /* ? */ (ESCAPES_FIRST + 0x0f) as i16,
    /* @ */ (ESCAPES_FIRST + 0x10) as i16,  /* A */ -(ESC_A as i16),
    /* B */ -(ESC_B as i16),                /* C */ -(ESC_C as i16),
    /* D */ -(ESC_D as i16),                /* E */ -(ESC_E as i16),
    /* F */ 0,                              /* G */ -(ESC_G as i16),
    /* H */ -(ESC_H as i16),                /* I */ 0,
    /* J */ 0,                              /* K */ -(ESC_K as i16),
    /* L */ 0,                              /* M */ 0,
    /* N */ -(ESC_N as i16),                /* O */ 0,
    /* P */ -(ESC_P as i16),                /* Q */ -(ESC_Q as i16),
    /* R */ -(ESC_R as i16),                /* S */ -(ESC_S as i16),
    /* T */ 0,                              /* U */ 0,
    /* V */ -(ESC_V as i16),                /* W */ -(ESC_W as i16),
    /* X */ -(ESC_X as i16),                /* Y */ 0,
    /* Z */ -(ESC_Z as i16),                /* [ */ (ESCAPES_FIRST + 0x2b) as i16,
    /* \ */ (ESCAPES_FIRST + 0x2c) as i16,  /* ] */ (ESCAPES_FIRST + 0x2d) as i16,
    /* ^ */ (ESCAPES_FIRST + 0x2e) as i16,  /* _ */ (ESCAPES_FIRST + 0x2f) as i16,
    /* ` */ (ESCAPES_FIRST + 0x30) as i16,  /* a */ CHAR_BEL as i16,
    /* b */ -(ESC_b as i16),                /* c */ 0,
    /* d */ -(ESC_d as i16),                /* e */ CHAR_ESC as i16,
    /* f */ CHAR_FF as i16,                 /* g */ 0,
    /* h */ -(ESC_h as i16),                /* i */ 0,
    /* j */ 0,                              /* k */ -(ESC_k as i16),
    /* l */ 0,                              /* m */ 0,
    /* n */ CHAR_LF as i16,                 /* o */ 0,
    /* p */ -(ESC_p as i16),                /* q */ 0,
    /* r */ CHAR_CR as i16,                 /* s */ -(ESC_s as i16),
    /* t */ CHAR_HT as i16,                 /* u */ 0,
    /* v */ -(ESC_v as i16),                /* w */ -(ESC_w as i16),
    /* x */ 0,                              /* y */ 0,
    /* z */ -(ESC_z as i16),
];

/* Compile-time concatenation of a fixed set of byte slices, producing a single
`[u8; N]`. Used to build the single-string name tables so their layout matches
the C `static const char name[]` concatenations exactly. */

const fn concat_len(parts: &[&[u8]]) -> usize {
    let mut total = 0usize;
    let mut i = 0usize;
    while i < parts.len() {
        total += parts[i].len();
        i += 1;
    }
    total
}

/* Table of special "verbs" like (*PRUNE). All the names are in a single
string. The empty name is a shorthand for MARK. */

const VERBNAMES_PARTS: &[&[u8]] = &[
    b"\0",
    STRING_MARK0,
    STRING_ACCEPT0,
    STRING_F0,
    STRING_FAIL0,
    STRING_COMMIT0,
    STRING_PRUNE0,
    STRING_SKIP0,
    STRING_THEN,
    b"\0", /* implicit terminator of the last C string literal */
];
pub(crate) const VERBNAMES_LEN: usize = concat_len(VERBNAMES_PARTS);

pub(crate) static verbnames: [u8; VERBNAMES_LEN] = {
    let mut out = [0u8; VERBNAMES_LEN];
    let mut pos = 0usize;
    let mut p = 0usize;
    while p < VERBNAMES_PARTS.len() {
        let part = VERBNAMES_PARTS[p];
        let mut j = 0usize;
        while j < part.len() {
            out[pos] = part[j];
            pos += 1;
            j += 1;
        }
        p += 1;
    }
    out
};

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct verbitem {
    pub len: u32,      /* Length of verb name */
    pub meta: u32,     /* Base META_ code */
    pub has_arg: c_int, /* Argument requirement */
}

pub(crate) static verbs: [verbitem; 9] = [
    verbitem { len: 0, meta: META_MARK, has_arg: 1 },  /* > 0 => must have an argument */
    verbitem { len: 4, meta: META_MARK, has_arg: 1 },
    verbitem { len: 6, meta: META_ACCEPT, has_arg: -1 }, /* < 0 => Optional argument, convert to pre-MARK */
    verbitem { len: 1, meta: META_FAIL, has_arg: -1 },
    verbitem { len: 4, meta: META_FAIL, has_arg: -1 },
    verbitem { len: 6, meta: META_COMMIT, has_arg: 0 },
    verbitem { len: 5, meta: META_PRUNE, has_arg: 0 }, /* Optional argument; bump META code if found */
    verbitem { len: 4, meta: META_SKIP, has_arg: 0 },
    verbitem { len: 4, meta: META_THEN, has_arg: 0 },
];

pub(crate) const verbcount: c_int = verbs.len() as c_int;

/* Verb opcodes, indexed by their META code offset from META_MARK. */

pub(crate) static verbops: [u32; 11] = [
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

/* Table of "alpha assertions" like (*pla:...), similar to the (*VERB) table. */

const ALASNAMES_PARTS: &[&[u8]] = &[
    STRING_pla0,
    STRING_plb0,
    STRING_napla0,
    STRING_naplb0,
    STRING_nla0,
    STRING_nlb0,
    STRING_positive_lookahead0,
    STRING_positive_lookbehind0,
    STRING_non_atomic_positive_lookahead0,
    STRING_non_atomic_positive_lookbehind0,
    STRING_negative_lookahead0,
    STRING_negative_lookbehind0,
    STRING_scs0,
    STRING_scan_substring0,
    STRING_atomic0,
    STRING_sr0,
    STRING_asr0,
    STRING_script_run0,
    STRING_atomic_script_run,
    b"\0", /* implicit terminator of the last C string literal */
];
pub(crate) const ALASNAMES_LEN: usize = concat_len(ALASNAMES_PARTS);

pub(crate) static alasnames: [u8; ALASNAMES_LEN] = {
    let mut out = [0u8; ALASNAMES_LEN];
    let mut pos = 0usize;
    let mut p = 0usize;
    while p < ALASNAMES_PARTS.len() {
        let part = ALASNAMES_PARTS[p];
        let mut j = 0usize;
        while j < part.len() {
            out[pos] = part[j];
            pos += 1;
            j += 1;
        }
        p += 1;
    }
    out
};

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct alasitem {
    pub len: u32,  /* Length of name */
    pub meta: u32, /* Base META_ code */
}

pub(crate) static alasmeta: [alasitem; 19] = [
    alasitem { len: 3, meta: META_LOOKAHEAD },
    alasitem { len: 3, meta: META_LOOKBEHIND },
    alasitem { len: 5, meta: META_LOOKAHEAD_NA },
    alasitem { len: 5, meta: META_LOOKBEHIND_NA },
    alasitem { len: 3, meta: META_LOOKAHEADNOT },
    alasitem { len: 3, meta: META_LOOKBEHINDNOT },
    alasitem { len: 18, meta: META_LOOKAHEAD },
    alasitem { len: 19, meta: META_LOOKBEHIND },
    alasitem { len: 29, meta: META_LOOKAHEAD_NA },
    alasitem { len: 30, meta: META_LOOKBEHIND_NA },
    alasitem { len: 18, meta: META_LOOKAHEADNOT },
    alasitem { len: 19, meta: META_LOOKBEHINDNOT },
    alasitem { len: 3, meta: META_SCS },
    alasitem { len: 14, meta: META_SCS },
    alasitem { len: 6, meta: META_ATOMIC },
    alasitem { len: 2, meta: META_SCRIPT_RUN },       /* sr = script run */
    alasitem { len: 3, meta: META_ATOMIC_SCRIPT_RUN }, /* asr = atomic script run */
    alasitem { len: 10, meta: META_SCRIPT_RUN },       /* script run */
    alasitem { len: 17, meta: META_ATOMIC_SCRIPT_RUN }, /* atomic script run */
];

pub(crate) const alascount: c_int = alasmeta.len() as c_int;

/* Offsets from OP_STAR for case-independent and negative repeat opcodes. */

pub(crate) static chartypeoffset: [u32; 4] = [
    (OP_STAR - OP_STAR) as u32,
    (OP_STARI - OP_STAR) as u32,
    (OP_NOTSTAR - OP_STAR) as u32,
    (OP_NOTSTARI - OP_STAR) as u32,
];

/* Tables of names of POSIX character classes and their lengths. The names are
in a single string. The list of lengths is terminated by a zero length entry.
The first three must be alpha, lower, upper. */

const POSIX_NAMES_PARTS: &[&[u8]] = &[
    STRING_alpha0,
    STRING_lower0,
    STRING_upper0,
    STRING_alnum0,
    STRING_ascii0,
    STRING_blank0,
    STRING_cntrl0,
    STRING_digit0,
    STRING_graph0,
    STRING_print0,
    STRING_punct0,
    STRING_space0,
    STRING_word0,
    STRING_xdigit,
    b"\0", /* implicit terminator of the last C string literal */
];
pub(crate) const POSIX_NAMES_LEN: usize = concat_len(POSIX_NAMES_PARTS);

pub(crate) static posix_names: [u8; POSIX_NAMES_LEN] = {
    let mut out = [0u8; POSIX_NAMES_LEN];
    let mut pos = 0usize;
    let mut p = 0usize;
    while p < POSIX_NAMES_PARTS.len() {
        let part = POSIX_NAMES_PARTS[p];
        let mut j = 0usize;
        while j < part.len() {
            out[pos] = part[j];
            pos += 1;
            j += 1;
        }
        p += 1;
    }
    out
};

pub(crate) static posix_name_lengths: [u8; 15] =
    [5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 4, 6, 0];

/* Note: PRIV(posix_class_maps) is translated in compile_class.rs as
`crate::compile_class::POSIX_CLASS_MAPS`. */

/* The POSIX class Unicode property substitutes that are used in UCP mode must
be in the order of the POSIX class names, defined above. */

pub(crate) static posix_substitutes: [c_int; 28] = [
    PT_GC as c_int, ucp_L as c_int,   /* alpha */
    PT_PC as c_int, ucp_Ll as c_int,  /* lower */
    PT_PC as c_int, ucp_Lu as c_int,  /* upper */
    PT_ALNUM as c_int, 0,             /* alnum */
    -1, 0,                            /* ascii, treat as non-UCP */
    -1, 1,                            /* blank, treat as \h */
    PT_PC as c_int, ucp_Cc as c_int,  /* cntrl */
    PT_PC as c_int, ucp_Nd as c_int,  /* digit */
    PT_PXGRAPH as c_int, 0,           /* graph */
    PT_PXPRINT as c_int, 0,           /* print */
    PT_PXPUNCT as c_int, 0,           /* punct */
    PT_PXSPACE as c_int, 0,           /* space */
    PT_WORD as c_int, 0,              /* word  */
    PT_PXXDIGIT as c_int, 0,          /* xdigit */
];

/* Types for the pso "value" field. */

pub(crate) const PSO_OPT: u16 = 0; /* Value is an option bit */
pub(crate) const PSO_XOPT: u16 = 1; /* Value is an xoption bit */
pub(crate) const PSO_FLG: u16 = 2; /* Value is a flag bit */
pub(crate) const PSO_NL: u16 = 3; /* Value is a newline type */
pub(crate) const PSO_BSR: u16 = 4; /* Value is a \R type */
pub(crate) const PSO_LIMH: u16 = 5; /* Read integer value for heap limit */
pub(crate) const PSO_LIMM: u16 = 6; /* Read integer value for match limit */
pub(crate) const PSO_LIMD: u16 = 7; /* Read integer value for depth limit */
pub(crate) const PSO_OPTMZ: u16 = 8; /* Value is an optimization bit */

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct pso {
    pub name: *const c_char,
    pub length: u16,
    pub type_: u16,
    pub value: u32,
}

/* pso holds a raw pointer; it is only ever read, so it is safe to share. */
unsafe impl Sync for pso {}

/* This is a table of start-of-pattern options such as (*UTF) and settings such
as (*LIMIT_MATCH=nnnn) and (*CRLF). NB: STRING_UTFn_RIGHTPAR in 8-bit mode is
STRING_UTF8_RIGHTPAR, length 5. */

pub(crate) static pso_list: [pso; 23] = [
    pso { name: STRING_UTF8_RIGHTPAR.as_ptr() as *const c_char, length: 5, type_: PSO_OPT, value: PCRE2_UTF },
    pso { name: STRING_UTF_RIGHTPAR.as_ptr() as *const c_char, length: 4, type_: PSO_OPT, value: PCRE2_UTF },
    pso { name: STRING_UCP_RIGHTPAR.as_ptr() as *const c_char, length: 4, type_: PSO_OPT, value: PCRE2_UCP },
    pso { name: STRING_NOTEMPTY_RIGHTPAR.as_ptr() as *const c_char, length: 9, type_: PSO_FLG, value: PCRE2_NOTEMPTY_SET },
    pso { name: STRING_NOTEMPTY_ATSTART_RIGHTPAR.as_ptr() as *const c_char, length: 17, type_: PSO_FLG, value: PCRE2_NE_ATST_SET },
    pso { name: STRING_NO_AUTO_POSSESS_RIGHTPAR.as_ptr() as *const c_char, length: 16, type_: PSO_OPTMZ, value: PCRE2_OPTIM_AUTO_POSSESS },
    pso { name: STRING_NO_DOTSTAR_ANCHOR_RIGHTPAR.as_ptr() as *const c_char, length: 18, type_: PSO_OPTMZ, value: PCRE2_OPTIM_DOTSTAR_ANCHOR },
    pso { name: STRING_NO_JIT_RIGHTPAR.as_ptr() as *const c_char, length: 7, type_: PSO_FLG, value: PCRE2_NOJIT },
    pso { name: STRING_NO_START_OPT_RIGHTPAR.as_ptr() as *const c_char, length: 13, type_: PSO_OPTMZ, value: PCRE2_OPTIM_START_OPTIMIZE },
    pso { name: STRING_CASELESS_RESTRICT_RIGHTPAR.as_ptr() as *const c_char, length: 18, type_: PSO_XOPT, value: PCRE2_EXTRA_CASELESS_RESTRICT },
    pso { name: STRING_TURKISH_CASING_RIGHTPAR.as_ptr() as *const c_char, length: 15, type_: PSO_XOPT, value: PCRE2_EXTRA_TURKISH_CASING },
    pso { name: STRING_LIMIT_HEAP_EQ.as_ptr() as *const c_char, length: 11, type_: PSO_LIMH, value: 0 },
    pso { name: STRING_LIMIT_MATCH_EQ.as_ptr() as *const c_char, length: 12, type_: PSO_LIMM, value: 0 },
    pso { name: STRING_LIMIT_DEPTH_EQ.as_ptr() as *const c_char, length: 12, type_: PSO_LIMD, value: 0 },
    pso { name: STRING_LIMIT_RECURSION_EQ.as_ptr() as *const c_char, length: 16, type_: PSO_LIMD, value: 0 },
    pso { name: STRING_CR_RIGHTPAR.as_ptr() as *const c_char, length: 3, type_: PSO_NL, value: PCRE2_NEWLINE_CR },
    pso { name: STRING_LF_RIGHTPAR.as_ptr() as *const c_char, length: 3, type_: PSO_NL, value: PCRE2_NEWLINE_LF },
    pso { name: STRING_CRLF_RIGHTPAR.as_ptr() as *const c_char, length: 5, type_: PSO_NL, value: PCRE2_NEWLINE_CRLF },
    pso { name: STRING_ANY_RIGHTPAR.as_ptr() as *const c_char, length: 4, type_: PSO_NL, value: PCRE2_NEWLINE_ANY },
    pso { name: STRING_NUL_RIGHTPAR.as_ptr() as *const c_char, length: 4, type_: PSO_NL, value: PCRE2_NEWLINE_NUL },
    pso { name: STRING_ANYCRLF_RIGHTPAR.as_ptr() as *const c_char, length: 8, type_: PSO_NL, value: PCRE2_NEWLINE_ANYCRLF },
    pso { name: STRING_BSR_ANYCRLF_RIGHTPAR.as_ptr() as *const c_char, length: 12, type_: PSO_BSR, value: PCRE2_BSR_ANYCRLF },
    pso { name: STRING_BSR_UNICODE_RIGHTPAR.as_ptr() as *const c_char, length: 12, type_: PSO_BSR, value: PCRE2_BSR_UNICODE },
];

/* This table is used when converting repeating opcodes into possessified
versions. A zero value means there is no possessified version. The table is
truncated at OP_CALLOUT because all relevant opcodes are less than that. */

#[rustfmt::skip]
pub(crate) static opcode_possessify: [u8; (OP_CALLOUT + 1) as usize] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,   /* 0 - 15  */
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,   /* 16 - 31 */

    0,                          /* NOTI */
    OP_POSSTAR, 0,              /* STAR, MINSTAR */
    OP_POSPLUS, 0,              /* PLUS, MINPLUS */
    OP_POSQUERY, 0,             /* QUERY, MINQUERY */
    OP_POSUPTO, 0,              /* UPTO, MINUPTO */
    0,                          /* EXACT */
    0, 0, 0, 0,                 /* POS{STAR,PLUS,QUERY,UPTO} */

    OP_POSSTARI, 0,             /* STARI, MINSTARI */
    OP_POSPLUSI, 0,             /* PLUSI, MINPLUSI */
    OP_POSQUERYI, 0,            /* QUERYI, MINQUERYI */
    OP_POSUPTOI, 0,             /* UPTOI, MINUPTOI */
    0,                          /* EXACTI */
    0, 0, 0, 0,                 /* POS{STARI,PLUSI,QUERYI,UPTOI} */

    OP_NOTPOSSTAR, 0,           /* NOTSTAR, NOTMINSTAR */
    OP_NOTPOSPLUS, 0,           /* NOTPLUS, NOTMINPLUS */
    OP_NOTPOSQUERY, 0,          /* NOTQUERY, NOTMINQUERY */
    OP_NOTPOSUPTO, 0,           /* NOTUPTO, NOTMINUPTO */
    0,                          /* NOTEXACT */
    0, 0, 0, 0,                 /* NOTPOS{STAR,PLUS,QUERY,UPTO} */

    OP_NOTPOSSTARI, 0,          /* NOTSTARI, NOTMINSTARI */
    OP_NOTPOSPLUSI, 0,          /* NOTPLUSI, NOTMINPLUSI */
    OP_NOTPOSQUERYI, 0,         /* NOTQUERYI, NOTMINQUERYI */
    OP_NOTPOSUPTOI, 0,          /* NOTUPTOI, NOTMINUPTOI */
    0,                          /* NOTEXACTI */
    0, 0, 0, 0,                 /* NOTPOS{STARI,PLUSI,QUERYI,UPTOI} */

    OP_TYPEPOSSTAR, 0,          /* TYPESTAR, TYPEMINSTAR */
    OP_TYPEPOSPLUS, 0,          /* TYPEPLUS, TYPEMINPLUS */
    OP_TYPEPOSQUERY, 0,         /* TYPEQUERY, TYPEMINQUERY */
    OP_TYPEPOSUPTO, 0,          /* TYPEUPTO, TYPEMINUPTO */
    0,                          /* TYPEEXACT */
    0, 0, 0, 0,                 /* TYPEPOS{STAR,PLUS,QUERY,UPTO} */

    OP_CRPOSSTAR, 0,            /* CRSTAR, CRMINSTAR */
    OP_CRPOSPLUS, 0,            /* CRPLUS, CRMINPLUS */
    OP_CRPOSQUERY, 0,           /* CRQUERY, CRMINQUERY */
    OP_CRPOSRANGE, 0,           /* CRRANGE, CRMINRANGE */
    0, 0, 0, 0,                 /* CRPOS{STAR,PLUS,QUERY,RANGE} */

    0, 0, 0, 0,                 /* CLASS, NCLASS, XCLASS, ECLASS */
    0, 0,                       /* REF, REFI */
    0, 0,                       /* DNREF, DNREFI */
    0, 0,                       /* RECURSE, CALLOUT */
];

/*************************************************
*         Read a number, possibly signed         *
*************************************************/

/* This function is used to read numbers in the pattern. The initial pointer
must be at the sign or first digit of the number. When relative values
(introduced by + or -) are allowed, they are relative group numbers, and the
result must be greater than zero.

Returns:      TRUE  - a number was read
              FALSE - errorcode == 0 => no number was found
                      errorcode != 0 => an error occurred */

pub(crate) unsafe fn read_number(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    allow_sign: i32,
    mut max_value: u32,
    max_error: u32,
    intptr: *mut c_int,
    errorcodeptr: *mut c_int,
) -> BOOL {
    unsafe {
        let mut sign: c_int = 0;
        let mut n: u32 = 0;
        let mut ptr = *ptrptr;
        let mut yield_: BOOL = FALSE;

        /* PCRE2_ASSERT(max_value <= INT_MAX/10 - 1); */

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

        /* Note: the C returns here without updating *intptr or *ptrptr. */
        if ptr >= ptrend || !IS_DIGIT(*ptr as u32) {
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
                *errorcodeptr = ERR26; /* +0 and -0 are not allowed */
                *intptr = n as c_int;
                *ptrptr = ptr;
                return yield_;
            }

            if sign > 0 {
                n += allow_sign as u32;
            } else if n > allow_sign as u32 {
                *errorcodeptr = ERR15; /* Non-existent subpattern */
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
}

/*************************************************
*         Read repeat counts                     *
*************************************************/

/* Read an item of the form {n,m} and return the values when non-NULL pointers
are supplied. Repeat counts must be less than 65536 (MAX_REPEAT_COUNT); a
larger value is used for "unlimited".

Returns:         FALSE if not a repeat quantifier, errorcode set zero
                 FALSE on error, with errorcode set non-zero
                 TRUE on success, with pointer updated to point after '}' */

pub(crate) unsafe fn read_repeat_counts(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    minp: *mut u32,
    maxp: *mut u32,
    errorcodeptr: *mut c_int,
) -> BOOL {
    unsafe {
        let mut p = *ptrptr;
        let mut pp: PCRE2_SPTR;
        let mut yield_: BOOL = FALSE;
        let mut had_minimum: BOOL = FALSE;
        let mut min: c_int = 0;
        let mut max: c_int = REPEAT_UNLIMITED as c_int; /* Larger than MAX_REPEAT_COUNT */

        *errorcodeptr = 0;
        while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
            p = p.add(1);
        }

        /* Check the syntax before interpreting. */

        pp = p;
        if pp < ptrend && IS_DIGIT(*pp as u32) {
            had_minimum = TRUE;
            loop {
                pp = pp.add(1);
                if !(pp < ptrend && IS_DIGIT(*pp as u32)) {
                    break;
                }
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
                loop {
                    pp = pp.add(1);
                    if !(pp < ptrend && IS_DIGIT(*pp as u32)) {
                        break;
                    }
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

        /* Now process the quantifier for real. */

        if read_number(&mut p, ptrend, -1, MAX_REPEAT_COUNT, ERR5 as u32, &mut min, errorcodeptr)
            == FALSE
        {
            if *errorcodeptr != 0 {
                *ptrptr = p;
                return yield_; /* n too big */
            }
            p = p.add(1); /* Skip comma and subsequent spaces */
            while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
                p = p.add(1);
            }
            if read_number(&mut p, ptrend, -1, MAX_REPEAT_COUNT, ERR5 as u32, &mut max, errorcodeptr)
                == FALSE
            {
                if *errorcodeptr != 0 {
                    *ptrptr = p;
                    return yield_; /* m too big */
                }
            }
        }
        /* Have read one number. Deal with {n} or {n,} or {n,m} */
        else {
            while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
                p = p.add(1);
            }
            if *p as u32 == CHAR_RIGHT_CURLY_BRACKET {
                max = min;
            } else {
                /* Handle {n,} or {n,m} */
                p = p.add(1); /* Skip comma and subsequent spaces */
                while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
                    p = p.add(1);
                }
                if read_number(
                    &mut p,
                    ptrend,
                    -1,
                    MAX_REPEAT_COUNT,
                    ERR5 as u32,
                    &mut max,
                    errorcodeptr,
                ) == FALSE
                {
                    if *errorcodeptr != 0 {
                        *ptrptr = p;
                        return yield_; /* m too big */
                    }
                }

                if max < min {
                    *errorcodeptr = ERR4;
                    *ptrptr = p;
                    return yield_;
                }
            }
        }

        /* Valid quantifier exists */

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
}

/*************************************************
*            Handle escapes                      *
*************************************************/

/* This function is called when a \ has been encountered. It either returns a
positive value for a simple escape such as \d, or 0 for a data character, which
is placed in chptr. A backreference to group n is returned as -(n+1). On entry,
ptr is pointing at the character after \. On exit, it points after the final
code unit of the escape sequence.

Returns:         zero => a data character
                 positive => a special escape sequence
                 negative => a numerical back reference
                 on error, errorcodeptr is set non-zero */

pub(crate) unsafe fn check_escape(
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
    unsafe {
        let utf: BOOL = ((options & PCRE2_UTF) != 0) as BOOL;
        let mut alt_bsux: BOOL =
            (((options & PCRE2_ALT_BSUX) | (xoptions & PCRE2_EXTRA_ALT_BSUX)) != 0) as BOOL;
        let mut ptr = *ptrptr;
        let mut c: u32;
        let mut cc: u32 = 0;
        let mut escape: c_int = 0;
        let mut i: c_int = 0;

        /* If backslash is at the end of the string, it's an error. */

        if ptr >= ptrend {
            *errorcodeptr = ERR1;
            *chptr = 0;
            *ptrptr = ptr;
            return 0;
        }

        c = getcharinctest(&mut ptr, utf != 0); /* Get character value, increment pointer */
        *errorcodeptr = 0; /* Be optimistic */

        /* The labelled block emulates the C goto EXIT / ESCAPE_FAILED_FORWARD. A
        `break 'exit` jumps straight to the shared exit code; `break 'exit_fwd`
        advances past the offending character before exiting. */

        let mut exit_forward = false;

        'body: {
            /* Non-alphanumerics are literals, so we just leave the value in c. */

            if c < ESCAPES_FIRST || c > ESCAPES_LAST {
                /* Definitely literal */
            } else if {
                i = escapes[(c - ESCAPES_FIRST) as usize] as c_int;
                i != 0
            } {
                if i > 0 {
                    c = i as u32;
                    if c == CHAR_CR && (xoptions & PCRE2_EXTRA_ESCAPED_CR_IS_LF) != 0 {
                        c = CHAR_LF;
                    }
                } else {
                    /* Negative table entry */
                    escape = -i; /* Else return a special escape */
                    if !cb.is_null()
                        && (escape == ESC_P || escape == ESC_p || escape == ESC_X)
                    {
                        (*cb).external_flags |= PCRE2_HASBKPORX; /* Note \P, \p, or \X */
                    }

                    /* Perl supports \N{name} and \N{U+dddd}. */

                    if escape == ESC_N && ptr < ptrend && *ptr as u32 == CHAR_LEFT_CURLY_BRACKET {
                        let mut p = ptr.add(1);

                        /* Perl ignores spaces and tabs after { */
                        while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
                            p = p.add(1);
                        }

                        /* \N{U+ can be handled by the \x{ code, in UTF mode only. */
                        if ptrend.offset_from(p) > 1
                            && *p as u32 == CHAR_U
                            && *p.add(1) as u32 == CHAR_PLUS
                        {
                            if utf != 0 {
                                ptr = p.add(2);
                                escape = 0; /* Not a fancy escape after all */
                                /* goto COME_FROM_NU */
                                match do_come_from_nu(
                                    &mut ptr, ptrend, &mut c, errorcodeptr, xoptions, utf,
                                ) {
                                    NuResult::Break => break 'body,
                                    NuResult::EscapeFailedForward => {
                                        exit_forward = true;
                                        break 'body;
                                    }
                                    NuResult::Continue => {}
                                }
                                /* After \x{} processing completes, break out. */
                                break 'body;
                            }

                            /* Improve error offset. */
                            ptr = p.add(2);
                            while ptr < ptrend && XDIGIT(*ptr as u32) != 0xff {
                                ptr = ptr.add(1);
                            }
                            while ptr < ptrend
                                && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT)
                            {
                                ptr = ptr.add(1);
                            }
                            if ptr < ptrend && *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
                                ptr = ptr.add(1);
                            }

                            *errorcodeptr = ERR93;
                        }
                        /* Give an error in contexts where quantifiers are not allowed. */
                        else if isclass != 0 || cb.is_null() {
                            ptr = ptr.add(1); /* Skip over the opening brace */
                            *errorcodeptr = ERR37;
                        }
                        /* Give an error if what follows is not a quantifier. */
                        else {
                            if read_repeat_counts(
                                &mut p,
                                ptrend,
                                core::ptr::null_mut(),
                                core::ptr::null_mut(),
                                errorcodeptr,
                            ) == FALSE
                                && *errorcodeptr == 0
                            {
                                ptr = ptr.add(1); /* Skip over the opening brace */
                                *errorcodeptr = ERR37;
                            }
                        }
                    }
                }
            }
            /* Escapes that need further processing have a zero entry in the table. */
            else {
                let mut s: c_int = 0;
                let mut oldptr: PCRE2_SPTR;

                /* Filter calls from pcre2_substitute(). */

                if cb.is_null() {
                    if !(c >= CHAR_0 && c <= CHAR_9)
                        && c != CHAR_c
                        && c != CHAR_o
                        && c != CHAR_x
                        && c != CHAR_g
                    {
                        *errorcodeptr = ERR3;
                        break 'body;
                    }
                    alt_bsux = FALSE; /* Do not modify \x handling */
                }

                'switch: {
                    /* A number of Perl escapes are not handled by PCRE. */
                    if c == CHAR_F || c == CHAR_l || c == CHAR_L {
                        *errorcodeptr = ERR37;
                        break 'switch;
                    }

                    if c == CHAR_u {
                        if alt_bsux == FALSE {
                            *errorcodeptr = ERR37;
                        } else {
                            let mut xc: u32;

                            if ptr >= ptrend {
                                break 'switch;
                            }
                            if *ptr as u32 == CHAR_LEFT_CURLY_BRACKET
                                && (xoptions & PCRE2_EXTRA_ALT_BSUX) != 0
                            {
                                let mut hptr = ptr.add(1);

                                cc = 0;
                                while hptr < ptrend && {
                                    xc = XDIGIT(*hptr as u32);
                                    xc != 0xff
                                } {
                                    if (cc & 0xf0000000) != 0 {
                                        /* Test for 32-bit overflow */
                                        *errorcodeptr = ERR77;
                                        ptr = hptr; /* Show where */
                                        break; /* *hptr != } will cause another break below */
                                    }
                                    cc = (cc << 4) | xc;
                                    hptr = hptr.add(1);
                                }

                                if hptr == ptr.add(1) /* No hex digits */
                                    || hptr >= ptrend /* Hit end of input */
                                    || *hptr as u32 != CHAR_RIGHT_CURLY_BRACKET
                                {
                                    if isclass != 0 {
                                        break 'switch; /* In a class, treat as '\u' literal */
                                    }
                                    escape = ESC_ub; /* Special return */
                                    ptr = ptr.add(1); /* Skip { */
                                    break 'switch; /* Hex escape not recognized */
                                }

                                c = cc; /* Accept the code point */
                                ptr = hptr.add(1);
                            } else {
                                /* Must be exactly 4 hex digits */
                                if ptrend.offset_from(ptr) < 4 {
                                    break 'switch; /* Less than 4 chars */
                                }
                                cc = XDIGIT(*ptr as u32);
                                if cc == 0xff {
                                    break 'switch;
                                }
                                xc = XDIGIT(*ptr.add(1) as u32);
                                if xc == 0xff {
                                    break 'switch;
                                }
                                cc = (cc << 4) | xc;
                                xc = XDIGIT(*ptr.add(2) as u32);
                                if xc == 0xff {
                                    break 'switch;
                                }
                                cc = (cc << 4) | xc;
                                xc = XDIGIT(*ptr.add(3) as u32);
                                if xc == 0xff {
                                    break 'switch;
                                }
                                c = (cc << 4) | xc;
                                ptr = ptr.add(4);
                            }

                            if utf != 0 {
                                if c > 0x10ffffu32 {
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
                        break 'switch;
                    }

                    if c == CHAR_U {
                        /* \U is unrecognized unless PCRE2_ALT_BSUX or
                        PCRE2_EXTRA_ALT_BSUX is set, in which case it is a letter. */
                        if alt_bsux == FALSE {
                            *errorcodeptr = ERR37;
                        }
                        break 'switch;
                    }

                    if c == CHAR_g {
                        if isclass != 0 {
                            break 'switch;
                        }

                        if ptr >= ptrend {
                            *errorcodeptr = ERR57;
                            break 'switch;
                        }

                        if cb.is_null() {
                            /* Substitution strings */
                            if *ptr as u32 != CHAR_LESS_THAN_SIGN {
                                *errorcodeptr = ERR57;
                                break 'switch;
                            }

                            let mut p = ptr.add(1);

                            if read_number(
                                &mut p,
                                ptrend,
                                -1,
                                MAX_GROUP_NUMBER,
                                ERR61 as u32,
                                &mut s,
                                errorcodeptr,
                            ) == FALSE
                            {
                                if *errorcodeptr == 0 {
                                    escape = ESC_g; /* No number found */
                                }
                                break 'switch;
                            }

                            if p >= ptrend || *p as u32 != CHAR_GREATER_THAN_SIGN {
                                ptr = p;
                                *errorcodeptr = ERR119; /* Missing terminator for number */
                                break 'switch;
                            }

                            ptr = p.add(1);
                            escape = -(s + 1);
                            break 'switch;
                        }

                        if *ptr as u32 == CHAR_LESS_THAN_SIGN || *ptr as u32 == CHAR_APOSTROPHE {
                            escape = ESC_g;
                            break 'switch;
                        }

                        /* If there is a brace delimiter, try to read a numerical
                        reference. If there isn't one, assume a name and treat as \k. */

                        if *ptr as u32 == CHAR_LEFT_CURLY_BRACKET {
                            let mut p = ptr.add(1);

                            while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
                                p = p.add(1);
                            }
                            if read_number(
                                &mut p,
                                ptrend,
                                bracount as i32,
                                MAX_GROUP_NUMBER,
                                ERR61 as u32,
                                &mut s,
                                errorcodeptr,
                            ) == FALSE
                            {
                                if *errorcodeptr == 0 {
                                    escape = ESC_k; /* No number found */
                                }
                                break 'switch;
                            }
                            while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
                                p = p.add(1);
                            }

                            if p >= ptrend || *p as u32 != CHAR_RIGHT_CURLY_BRACKET {
                                ptr = p;
                                *errorcodeptr = ERR119; /* Missing terminator for number */
                                break 'switch;
                            }
                            ptr = p.add(1);
                        }
                        /* Read an undelimited number */
                        else {
                            if read_number(
                                &mut ptr,
                                ptrend,
                                bracount as i32,
                                MAX_GROUP_NUMBER,
                                ERR61 as u32,
                                &mut s,
                                errorcodeptr,
                            ) == FALSE
                            {
                                if *errorcodeptr == 0 {
                                    *errorcodeptr = ERR57; /* No number found */
                                }
                                break 'switch;
                            }
                        }

                        if s <= 0 {
                            *errorcodeptr = ERR15;
                            break 'switch;
                        }

                        escape = -(s + 1);
                        break 'switch;
                    }

                    /* Digits 1-9 and 0. */
                    if c >= CHAR_1 && c <= CHAR_9 {
                        let mut fall_to_octal = false;

                        if isclass != 0 {
                            /* Fall through to octal handling. */
                            fall_to_octal = true;
                        } else if (xoptions & PCRE2_EXTRA_PYTHON_OCTAL) != 0 {
                            /* Python-style disambiguation. */
                            if *ptr.sub(1) as u32 <= CHAR_7
                                && ptr.add(1) < ptrend
                                && *ptr as u32 >= CHAR_0
                                && *ptr as u32 <= CHAR_7
                                && *ptr.add(1) as u32 >= CHAR_0
                                && *ptr.add(1) as u32 <= CHAR_7
                            {
                                /* We peeked a three-digit octal, so fall through */
                                fall_to_octal = true;
                            } else {
                                ptr = ptr.sub(1); /* Back to the digit */

                                if read_number(
                                    &mut ptr,
                                    ptrend,
                                    -1,
                                    MAX_GROUP_NUMBER,
                                    0,
                                    &mut s,
                                    errorcodeptr,
                                ) == FALSE
                                {
                                    *errorcodeptr = ERR61;
                                    break 'switch;
                                }

                                escape = -(s + 1);
                                break 'switch;
                            }
                        } else {
                            /* Perl-style disambiguation. */
                            oldptr = ptr;
                            ptr = ptr.sub(1); /* Back to the digit */

                            if read_number(
                                &mut ptr,
                                ptrend,
                                -1,
                                MAX_GROUP_NUMBER,
                                0,
                                &mut s,
                                errorcodeptr,
                            ) == FALSE
                            {
                                s = c_int::MAX;
                            }

                            /* \1 to \9 are always back references. \8x and \9x are
                            too; \1x to \7x are octal escapes if there are not that
                            many previous captures. */

                            if s < 10 || c >= CHAR_8 || (s as u32) <= bracount {
                                if (s as u32) > MAX_GROUP_NUMBER {
                                    *errorcodeptr = ERR61;
                                } else {
                                    escape = -(s + 1); /* Indicates a back reference */
                                }
                                break 'switch;
                            }

                            ptr = oldptr; /* Put the pointer back and fall through */
                            fall_to_octal = true;
                        }

                        /* Handle a digit following \ when not a back reference. If
                        the first digit is 8 or 9, do not insert a binary zero. */

                        let _ = fall_to_octal;
                        if c >= CHAR_8 {
                            break 'switch;
                        }
                        /* Fall through to octal handling below. */
                    }

                    if c == CHAR_c {
                        if ptr >= ptrend {
                            *errorcodeptr = ERR2;
                            break 'switch;
                        }
                        c = *ptr as u32;
                        if c >= CHAR_a && c <= CHAR_z {
                            c = UPPER_CASE(c);
                        }

                        /* ASCII/Unicode environment. */
                        if c < 32 || c > 126 {
                            /* Excludes all non-printable ASCII */
                            *errorcodeptr = ERR68;
                            exit_forward = true;
                            break 'body;
                        }
                        c ^= 0x40;

                        ptr = ptr.add(1);
                        break 'switch;
                    }

                    if c == CHAR_o {
                        octal_o(
                            &mut ptr,
                            ptrend,
                            &mut c,
                            errorcodeptr,
                            xoptions,
                            utf,
                            &mut exit_forward,
                        );
                        if exit_forward {
                            break 'body;
                        }
                        break 'switch;
                    }

                    if c == CHAR_x {
                        if alt_bsux != 0 {
                            let mut xc: u32;
                            if ptrend.offset_from(ptr) < 2 {
                                break 'switch; /* Less than 2 characters */
                            }
                            cc = XDIGIT(*ptr as u32);
                            if cc == 0xff {
                                break 'switch; /* Not a hex digit */
                            }
                            xc = XDIGIT(*ptr.add(1) as u32);
                            if xc == 0xff {
                                break 'switch; /* Not a hex digit */
                            }
                            c = (cc << 4) | xc;
                            ptr = ptr.add(2);
                        } else {
                            /* Perl-style \x handling. */
                            if ptr < ptrend && *ptr as u32 == CHAR_LEFT_CURLY_BRACKET {
                                ptr = ptr.add(1);
                                while ptr < ptrend
                                    && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT)
                                {
                                    ptr = ptr.add(1);
                                }

                                /* COME_FROM_NU target */
                                match do_come_from_nu(
                                    &mut ptr, ptrend, &mut c, errorcodeptr, xoptions, utf,
                                ) {
                                    NuResult::Break => break 'switch,
                                    NuResult::EscapeFailedForward => {
                                        exit_forward = true;
                                        break 'body;
                                    }
                                    NuResult::Continue => {}
                                }
                            } else {
                                /* Read up to two hex digits after \x */
                                if ptr >= ptrend || {
                                    cc = XDIGIT(*ptr as u32);
                                    cc == 0xff
                                } {
                                    /* Not a hex digit */
                                    *errorcodeptr = ERR78;
                                    break 'switch;
                                }
                                ptr = ptr.add(1);
                                c = cc;

                                if ptr >= ptrend || {
                                    cc = XDIGIT(*ptr as u32);
                                    cc == 0xff
                                } {
                                    break 'switch; /* Not a hex digit */
                                }
                                ptr = ptr.add(1);
                                c = (c << 4) | cc;
                            }
                        }
                        break 'switch;
                    }

                    /* CHAR_0 and any digit falling through from above are octal. */
                    if c == CHAR_0 || (c >= CHAR_1 && c <= CHAR_7) {
                        c -= CHAR_0;
                        while {
                            i += 1;
                            i - 1 < 2
                        } && ptr < ptrend
                            && *ptr as u32 >= CHAR_0
                            && *ptr as u32 <= CHAR_7
                        {
                            c = c * 8 + *ptr as u32 - CHAR_0;
                            ptr = ptr.add(1);
                        }
                        if c > 0xff {
                            if (xoptions & PCRE2_EXTRA_PYTHON_OCTAL) != 0 {
                                *errorcodeptr = ERR102;
                            } else if utf == 0 {
                                *errorcodeptr = ERR51;
                            }
                        }

                        /* PCRE2_EXTRA_NO_BS0 disables the NUL escape '\0'. */
                        if (xoptions & PCRE2_EXTRA_NO_BS0) != 0 && c == 0 && i == 1 {
                            *errorcodeptr = ERR98;
                        }
                        break 'switch;
                    }

                    /* Any other alphanumeric following \ is an error. */
                    *errorcodeptr = ERR3;
                } /* 'switch */
            }
        } /* 'body */

        /* ESCAPE_FAILED_FORWARD: advance the pointer over the next character. */
        if exit_forward {
            ptr = ptr.add(1);
            if utf != 0 {
                forwardchartest(&mut ptr, ptrend);
            }
        }

        /* EXIT */
        *ptrptr = ptr;
        *chptr = c;
        escape
    }
}

/* Result of the \N{U+ / \x{ shared "COME_FROM_NU" hex-brace handling. */
enum NuResult {
    /* Corresponds to the C `break` out of the switch after processing. */
    Break,
    /* Corresponds to `goto ESCAPE_FAILED_FORWARD`. */
    EscapeFailedForward,
    /* Not used, present for completeness. */
    Continue,
}

/* Shared \x{...} / \N{U+...} hex code point processing (the C COME_FROM_NU
label). On entry `*ptr` is positioned just after any leading spaces following
the '{' (or after "U+"). Reads hex digits and the closing brace. */
#[inline]
unsafe fn do_come_from_nu(
    ptr: &mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    c: &mut u32,
    errorcodeptr: *mut c_int,
    xoptions: u32,
    utf: BOOL,
) -> NuResult {
    unsafe {
        let mut cc: u32;
        let mut overflow: BOOL;

        if *ptr >= ptrend || **ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
            *errorcodeptr = ERR78;
            return NuResult::Break;
        }
        *c = 0;
        overflow = FALSE;

        while *ptr < ptrend && {
            cc = XDIGIT(**ptr as u32);
            cc != 0xff
        } {
            *ptr = ptr.add(1);
            if *c == 0 && cc == 0 {
                continue; /* Leading zeroes */
            }
            *c = (*c << 4) | cc;
            if (utf != 0 && *c > 0x10ffffu32) || (utf == 0 && *c > MAX_NON_UTF_CHAR) {
                overflow = TRUE;
                break;
            }
        }

        /* Perl ignores spaces and tabs before } */
        while *ptr < ptrend && (**ptr as u32 == CHAR_SPACE || **ptr as u32 == CHAR_HT) {
            *ptr = ptr.add(1);
        }

        /* On overflow, skip remaining hex digits */
        if overflow != 0 {
            while *ptr < ptrend && XDIGIT(**ptr as u32) != 0xff {
                *ptr = ptr.add(1);
            }
            *errorcodeptr = ERR34;
        } else if utf != 0
            && *c >= 0xd800
            && *c <= 0xdfff
            && (xoptions & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES) == 0
        {
            *errorcodeptr = ERR73;
        } else if *ptr < ptrend && **ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
            *ptr = ptr.add(1);
        } else {
            *errorcodeptr = ERR67;
            return NuResult::EscapeFailedForward;
        }

        NuResult::Break
    }
}

/* Shared \o{...} octal processing. Sets *exit_forward true if the C code would
`goto ESCAPE_FAILED_FORWARD`. */
#[inline]
unsafe fn octal_o(
    ptr: &mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    c: &mut u32,
    errorcodeptr: *mut c_int,
    xoptions: u32,
    utf: BOOL,
    exit_forward: &mut bool,
) {
    unsafe {
        let mut cc: u32;
        let mut overflow: BOOL;

        if *ptr >= ptrend || **ptr as u32 != CHAR_LEFT_CURLY_BRACKET {
            *errorcodeptr = ERR55;
            return;
        }
        *ptr = ptr.add(1);

        while *ptr < ptrend && (**ptr as u32 == CHAR_SPACE || **ptr as u32 == CHAR_HT) {
            *ptr = ptr.add(1);
        }
        if *ptr >= ptrend || **ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
            *errorcodeptr = ERR78;
            return;
        }

        *c = 0;
        overflow = FALSE;
        while *ptr < ptrend && **ptr as u32 >= CHAR_0 && **ptr as u32 <= CHAR_7 {
            cc = **ptr as u32;
            *ptr = ptr.add(1);
            if *c == 0 && cc == CHAR_0 {
                continue; /* Leading zeroes */
            }
            *c = (*c << 3) + (cc - CHAR_0);
            if *c > (if utf != 0 { 0x10ffffu32 } else { 0xffu32 }) {
                overflow = TRUE;
                break;
            }
        }

        while *ptr < ptrend && (**ptr as u32 == CHAR_SPACE || **ptr as u32 == CHAR_HT) {
            *ptr = ptr.add(1);
        }

        if overflow != 0 {
            while *ptr < ptrend && **ptr as u32 >= CHAR_0 && **ptr as u32 <= CHAR_7 {
                *ptr = ptr.add(1);
            }
            *errorcodeptr = ERR34;
        } else if utf != 0
            && *c >= 0xd800
            && *c <= 0xdfff
            && (xoptions & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES) == 0
        {
            *errorcodeptr = ERR73;
        } else if *ptr < ptrend && **ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
            *ptr = ptr.add(1);
        } else {
            *errorcodeptr = ERR64;
            *exit_forward = true;
        }
    }
}

/// Exported as `_pcre2_check_escape_8` (`PRIV(check_escape)`).
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
    unsafe {
        check_escape(
            ptrptr,
            ptrend,
            chptr,
            errorcodeptr,
            options,
            xoptions,
            bracount,
            isclass,
            cb,
        )
    }
}

/*************************************************
*               Handle \P and \p                 *
*************************************************/

/* This function is called after \P or \p has been encountered. On entry, the
contents of ptrptr are pointing after the P or p. On exit, it is left pointing
after the final code unit of the escape sequence.

Returns:         TRUE if the type value was found, or FALSE for an invalid type */

/* The C source passes these as C string literals, which carry an implicit
terminating NUL; the `&[u8]` constants in `chars` hold only the literal bytes. */
const CSTR_bidiclass: &[u8] = b"bidiclass\0";
const CSTR_bc: &[u8] = b"bc\0";
const CSTR_script: &[u8] = b"script\0";
const CSTR_sc: &[u8] = b"sc\0";
const CSTR_scriptextensions: &[u8] = b"scriptextensions\0";
const CSTR_scx: &[u8] = b"scx\0";

pub(crate) unsafe fn get_ucp(
    ptrptr: *mut PCRE2_SPTR,
    utf: BOOL,
    negptr: *mut BOOL,
    ptypeptr: *mut u16,
    pdataptr: *mut u16,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> BOOL {
    unsafe {
        let mut c: u32;
        let mut i: isize;
        let mut bot: PCRE2_SIZE;
        let mut top: PCRE2_SIZE;
        let mut ptr = *ptrptr;
        let mut name = [0u8; 50];
        let mut vptr: *mut PCRE2_UCHAR = core::ptr::null_mut();
        let mut ptscript: u16 = PT_NOTSCRIPT as u16;

        let _ = utf; /* Avoid unused warning */

        if ptr >= (*cb).end_pattern {
            *errorcodeptr = ERR46;
            *ptrptr = ptr;
            return FALSE;
        }
        c = getcharinctest(&mut ptr, utf != 0);
        *negptr = FALSE;

        if c == CHAR_LEFT_CURLY_BRACKET {
            if ptr >= (*cb).end_pattern {
                *errorcodeptr = ERR46;
                *ptrptr = ptr;
                return FALSE;
            }

            i = 0;
            'outer: while i < (name.len() as isize) - 1 {
                /* REDO loop */
                loop {
                    if ptr >= (*cb).end_pattern {
                        *errorcodeptr = ERR46;
                        *ptrptr = ptr;
                        return FALSE;
                    }
                    c = getcharinctest(&mut ptr, utf != 0);

                    /* Skip ignorable Unicode characters. */
                    if c == CHAR_UNDERSCORE
                        || c == CHAR_MINUS
                        || c == CHAR_SPACE
                        || (c >= CHAR_HT && c <= CHAR_CR)
                    {
                        continue;
                    }

                    /* First significant character being circumflex negates. */
                    if i == 0 && *negptr == FALSE && c == CHAR_CIRCUMFLEX_ACCENT {
                        *negptr = TRUE;
                        continue;
                    }
                    break;
                }

                if c == CHAR_RIGHT_CURLY_BRACKET {
                    break 'outer;
                }

                /* Names consist of ASCII letters and digits; equals and colon
                may occur as separators. */
                if c < CHAR_AMPERSAND || c > CHAR_z {
                    *errorcodeptr = ERR46;
                    *ptrptr = ptr;
                    return FALSE;
                }

                if c >= CHAR_A && c <= CHAR_Z {
                    c |= 0x20;
                } else if (c == CHAR_COLON || c == CHAR_EQUALS_SIGN) && vptr.is_null() {
                    vptr = name.as_mut_ptr().offset(i);
                }

                name[i as usize] = c as u8;
                i += 1;
            }

            /* Error if the loop didn't end with '}'. */
            if c != CHAR_RIGHT_CURLY_BRACKET {
                *errorcodeptr = ERR46;
                *ptrptr = ptr;
                return FALSE;
            }
            name[i as usize] = 0;
        }
        /* If { doesn't follow, there is just one following character. */
        else if c >= CHAR_A && c <= CHAR_Z {
            name[0] = (c | 0x20) as u8; /* Lower case */
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

        *ptrptr = ptr; /* Update pattern pointer */

        /* If the property contains ':' or '=' we have class name and value. */
        if !vptr.is_null() {
            let mut offset: isize = 0;
            let mut sname = [0u8; 8];

            *vptr = 0; /* Terminate property name */
            let namep = name.as_ptr();
            if crate::string_utils::strcmp_c8(namep, CSTR_bidiclass.as_ptr() as *const c_char) == 0
                || crate::string_utils::strcmp_c8(namep, CSTR_bc.as_ptr() as *const c_char) == 0
            {
                offset = 4;
                sname[0] = CHAR_b as u8;
                sname[1] = CHAR_i as u8; /* There is no strcpy_c8 function */
                sname[2] = CHAR_d as u8;
                sname[3] = CHAR_i as u8;
            } else if crate::string_utils::strcmp_c8(
                namep,
                CSTR_script.as_ptr() as *const c_char,
            ) == 0
                || crate::string_utils::strcmp_c8(namep, CSTR_sc.as_ptr() as *const c_char) == 0
            {
                ptscript = PT_SC as u16;
            } else if crate::string_utils::strcmp_c8(
                namep,
                CSTR_scriptextensions.as_ptr() as *const c_char,
            ) == 0
                || crate::string_utils::strcmp_c8(namep, CSTR_scx.as_ptr() as *const c_char) == 0
            {
                ptscript = PT_SCX as u16;
            } else {
                *errorcodeptr = ERR47;
                return FALSE;
            }

            /* Adjust the string in name[] as needed. The move copies
            (name + i - vptr) code units, including the terminating zero. */
            let vp1 = vptr.add(1);
            let count = (name.as_ptr().offset(i) as isize - vptr as isize) as usize;
            memmove(name.as_mut_ptr().offset(offset), vp1, count);
            if offset != 0 {
                memmove(name.as_mut_ptr(), sname.as_ptr(), offset as usize);
            }
        }

        /* Search for a recognized property using binary chop. */

        bot = 0;
        top = crate::ucptables::UTT_SIZE;

        while bot < top {
            let r: c_int;
            let mid = (bot + top) >> 1;
            r = crate::string_utils::strcmp_c8(
                name.as_ptr(),
                crate::ucptables::UTT_NAMES
                    .as_ptr()
                    .add(crate::ucptables::UTT[mid].name_offset as usize)
                    as *const c_char,
            );

            if r == 0 {
                *pdataptr = crate::ucptables::UTT[mid].value;
                if vptr.is_null() || ptscript == PT_NOTSCRIPT as u16 {
                    *ptypeptr = crate::ucptables::UTT[mid].type_;
                    return TRUE;
                }

                let ty = crate::ucptables::UTT[mid].type_ as u32;
                if ty == PT_SC {
                    *ptypeptr = PT_SC as u16;
                    return TRUE;
                } else if ty == PT_SCX {
                    *ptypeptr = ptscript;
                    return TRUE;
                }

                break; /* Non-script found */
            }

            if r > 0 {
                bot = mid + 1;
            } else {
                top = mid;
            }
        }

        *errorcodeptr = ERR47; /* Unrecognized property */
        FALSE
    }
}

/*************************************************
*           Check for POSIX class syntax         *
*************************************************/

/* This function is called when the sequence "[:" or "[." or "[=" is
encountered in a character class.

Returns:   TRUE or FALSE */

pub(crate) unsafe fn check_posix_syntax(
    mut ptr: PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    endptr: *mut PCRE2_SPTR,
) -> BOOL {
    unsafe {
        let terminator: PCRE2_UCHAR = *ptr;
        ptr = ptr.add(1);

        while ptrend.offset_from(ptr) >= 2 {
            if *ptr as u32 == CHAR_BACKSLASH
                && (*ptr.add(1) as u32 == CHAR_RIGHT_SQUARE_BRACKET
                    || *ptr.add(1) as u32 == CHAR_BACKSLASH)
            {
                ptr = ptr.add(1);
            } else if (*ptr as u32 == CHAR_LEFT_SQUARE_BRACKET && *ptr.add(1) == terminator)
                || *ptr as u32 == CHAR_RIGHT_SQUARE_BRACKET
            {
                return FALSE;
            } else if *ptr == terminator && *ptr.add(1) as u32 == CHAR_RIGHT_SQUARE_BRACKET {
                *endptr = ptr;
                return TRUE;
            }
            ptr = ptr.add(1);
        }

        FALSE
    }
}

/*************************************************
*          Check POSIX class name                *
*************************************************/

/* This function is called to check the name given in a POSIX-style class entry
such as [:alnum:].

Returns:     a value representing the name, or -1 if unknown */

pub(crate) unsafe fn check_posix_name(ptr: PCRE2_SPTR, len: c_int) -> c_int {
    unsafe {
        let mut pn = posix_names.as_ptr();
        let mut yield_: c_int = 0;
        while posix_name_lengths[yield_ as usize] != 0 {
            if len == posix_name_lengths[yield_ as usize] as c_int
                && crate::string_utils::strncmp_c8(ptr, pn as *const c_char, len as usize) == 0
            {
                return yield_;
            }
            pn = pn.add(posix_name_lengths[yield_ as usize] as usize + 1);
            yield_ += 1;
        }
        -1
    }
}

/*************************************************
*       Read a subpattern or VERB name           *
*************************************************/

/* This function reads the name of a subpattern or a (*VERB) or an
(*alpha_assertion). The initial pointer must be to the preceding character.

Returns:    TRUE if a name was read
            FALSE otherwise, with error code set */

pub(crate) unsafe fn read_name(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    utf: BOOL,
    terminator: u32,
    offsetptr: *mut PCRE2_SIZE,
    nameptr: *mut PCRE2_SPTR,
    namelenptr: *mut u32,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> BOOL {
    unsafe {
        let mut ptr = *ptrptr;
        let is_group: BOOL = (*ptr as u32 != CHAR_ASTERISK) as BOOL;
        ptr = ptr.add(1);
        let is_braced: BOOL = (terminator == CHAR_RIGHT_CURLY_BRACKET) as BOOL;

        if is_braced != 0 {
            while ptr < ptrend && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT) {
                ptr = ptr.add(1);
            }
        }

        if ptr >= ptrend {
            /* No characters in name */
            *errorcodeptr = if is_group != 0 { ERR62 } else { ERR60 };
            *ptrptr = ptr;
            return FALSE;
        }

        *nameptr = ptr;
        *offsetptr = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;

        /* In UTF mode, a group name may contain letters and decimal digits as
        defined by Unicode properties, and underscores, but must not start with
        a digit. */

        let mut handled_utf = false;
        if utf != 0 && is_group != 0 {
            let mut c: u32;
            let mut ty: u32;
            let mut p = ptr;

            c = getcharinc(&mut p); /* Peek at next character */
            ty = ucd_chartype(c);

            if ty == ucp_Nd {
                ptr = p;
                *errorcodeptr = ERR44;
                *ptrptr = ptr;
                return FALSE;
            }

            loop {
                if ty != ucp_Nd && UCP_GENTYPE[ty as usize] != ucp_L && c != CHAR_UNDERSCORE {
                    break;
                }
                ptr = p; /* Accept character and peek again */
                if p >= ptrend {
                    break;
                }
                c = getcharinc(&mut p);
                ty = ucd_chartype(c);
            }
            handled_utf = true;
        }

        /* Handle non-group names and group names in non-UTF modes. */
        if !handled_utf {
            if is_group != 0 && IS_DIGIT(*ptr as u32) {
                ptr = ptr.add(1);
                *errorcodeptr = ERR44;
                *ptrptr = ptr;
                return FALSE;
            }

            while ptr < ptrend
                && max_255(*ptr as u32)
                && ((*(*cb).ctypes.add(*ptr as usize)) & ctype_word) != 0
            {
                ptr = ptr.add(1);
            }
        }

        /* Check name length */
        if ptr.offset_from(*nameptr) > MAX_NAME_SIZE as isize {
            *errorcodeptr = ERR48;
            *ptrptr = ptr;
            return FALSE;
        }
        *namelenptr = ptr.offset_from(*nameptr) as u32;

        /* Subpattern names must not be empty, and their terminator is checked. */
        if is_group != 0 {
            if ptr == *nameptr {
                *errorcodeptr = ERR62; /* Subpattern name expected */
                *ptrptr = ptr;
                return FALSE;
            }
            if is_braced != 0 {
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
}

/**************************************************
*        Parse capturing bracket argument list    *
**************************************************/

/* Reads a list of capture references. The references can be numbers or names.

Returns: updated parsed_pattern pointer on success
         NULL otherwise */

pub(crate) unsafe fn parse_capture_list(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    utf: BOOL,
    mut parsed_pattern: *mut u32,
    mut offset: PCRE2_SIZE,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> *mut u32 {
    unsafe {
        let mut next_offset: PCRE2_SIZE;
        let mut ptr = *ptrptr;
        let mut name: PCRE2_SPTR = core::ptr::null();
        let mut terminator: PCRE2_UCHAR;
        let mut meta: u32;
        let mut namelen: u32 = 0;
        let mut i: c_int = 0;

        if ptr >= ptrend || *ptr as u32 != CHAR_LEFT_PARENTHESIS {
            *errorcodeptr = ERR118;
            *ptrptr = ptr;
            return core::ptr::null_mut();
        }

        loop {
            ptr = ptr.add(1);
            next_offset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;

            if ptr >= ptrend {
                *errorcodeptr = ERR117;
                *ptrptr = ptr;
                return core::ptr::null_mut();
            }

            /* Handle [+-]number cases */
            if read_number(
                &mut ptr,
                ptrend,
                (*cb).bracount as i32,
                MAX_GROUP_NUMBER,
                ERR61 as u32,
                &mut i,
                errorcodeptr,
            ) != FALSE
            {
                if i <= 0 {
                    *errorcodeptr = ERR15;
                    *ptrptr = ptr;
                    return core::ptr::null_mut();
                }
                meta = META_CAPTURE_NUMBER;
                namelen = i as u32;
            } else if *errorcodeptr != 0 {
                *ptrptr = ptr;
                return core::ptr::null_mut(); /* Number too big */
            } else {
                /* Handle 'name' or <name> cases. */
                if *ptr as u32 == CHAR_LESS_THAN_SIGN {
                    terminator = CHAR_GREATER_THAN_SIGN as PCRE2_UCHAR;
                } else if *ptr as u32 == CHAR_APOSTROPHE {
                    terminator = CHAR_APOSTROPHE as PCRE2_UCHAR;
                } else {
                    *errorcodeptr = ERR117;
                    *ptrptr = ptr;
                    return core::ptr::null_mut();
                }

                if read_name(
                    &mut ptr,
                    ptrend,
                    utf,
                    terminator as u32,
                    &mut next_offset,
                    &mut name,
                    &mut namelen,
                    errorcodeptr,
                    cb,
                ) == FALSE
                {
                    *ptrptr = ptr;
                    return core::ptr::null_mut();
                }

                meta = META_CAPTURE_NAME;
            }
            let _ = name;

            if offset == 0 || (next_offset - offset) >= 0x10000 {
                *parsed_pattern = META_OFFSET;
                parsed_pattern = parsed_pattern.add(1);
                putoffset(next_offset, &mut parsed_pattern);
                offset = next_offset;
            }

            /* The offset is encoded as a relative offset. */
            *parsed_pattern = meta | ((next_offset - offset) as u32);
            parsed_pattern = parsed_pattern.add(1);
            *parsed_pattern = namelen;
            parsed_pattern = parsed_pattern.add(1);
            offset = next_offset;

            if ptr >= ptrend {
                *errorcodeptr = ERR14;
                *ptrptr = ptr;
                return core::ptr::null_mut();
            }

            if *ptr as u32 == CHAR_RIGHT_PARENTHESIS {
                break;
            }

            if *ptr as u32 != CHAR_COMMA {
                *errorcodeptr = ERR24;
                *ptrptr = ptr;
                return core::ptr::null_mut();
            }
        }

        *ptrptr = ptr.add(1);
        parsed_pattern
    }
}

/*************************************************
*          Manage callouts at start of cycle     *
*************************************************/

/* At the start of a new item in parse_regex() we record the details of the
previous item in a prior callout, and also set up an automatic callout if
enabled.

Returns: possibly updated parsed_pattern pointer. */

pub(crate) unsafe fn manage_callouts(
    ptr: PCRE2_SPTR,
    pcalloutptr: *mut *mut u32,
    auto_callout: BOOL,
    mut parsed_pattern: *mut u32,
    cb: *mut compile_block,
) -> *mut u32 {
    unsafe {
        let mut previous_callout = *pcalloutptr;

        if !previous_callout.is_null() {
            *previous_callout.add(2) = (ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE
                - *previous_callout.add(1) as PCRE2_SIZE)
                as u32;
        }

        if auto_callout == FALSE {
            previous_callout = core::ptr::null_mut();
        } else {
            if previous_callout.is_null()
                || previous_callout != parsed_pattern.sub(4)
                || *previous_callout.add(3) != 255
            {
                previous_callout = parsed_pattern; /* Set up new automatic callout */
                parsed_pattern = parsed_pattern.add(4);
                *previous_callout.add(0) = META_CALLOUT_NUMBER;
                *previous_callout.add(2) = 0;
                *previous_callout.add(3) = 255;
            }
            *previous_callout.add(1) = ptr.offset_from((*cb).start_pattern) as u32;
        }

        *pcalloutptr = previous_callout;
        parsed_pattern
    }
}

/*************************************************
*          Handle \d, \D, \s, \S, \w, \W         *
*************************************************/

/* This function handles those escapes that may change when Unicode property
support is requested.

Returns:          updated value of parsed_pattern */

pub(crate) unsafe fn handle_escdsw(
    escape: c_int,
    mut parsed_pattern: *mut u32,
    options: u32,
    xoptions: u32,
) -> *mut u32 {
    unsafe {
        let mut ascii_option: u32 = 0;
        let mut prop: c_int = ESC_p;

        if escape == ESC_D {
            prop = ESC_P;
            ascii_option = PCRE2_EXTRA_ASCII_BSD;
        } else if escape == ESC_d {
            ascii_option = PCRE2_EXTRA_ASCII_BSD;
        } else if escape == ESC_S {
            prop = ESC_P;
            ascii_option = PCRE2_EXTRA_ASCII_BSS;
        } else if escape == ESC_s {
            ascii_option = PCRE2_EXTRA_ASCII_BSS;
        } else if escape == ESC_W {
            prop = ESC_P;
            ascii_option = PCRE2_EXTRA_ASCII_BSW;
        } else if escape == ESC_w {
            ascii_option = PCRE2_EXTRA_ASCII_BSW;
        }

        if (options & PCRE2_UCP) == 0 || (xoptions & ascii_option) != 0 {
            *parsed_pattern = META_ESCAPE + escape as u32;
            parsed_pattern = parsed_pattern.add(1);
        } else {
            *parsed_pattern = META_ESCAPE + prop as u32;
            parsed_pattern = parsed_pattern.add(1);
            if escape == ESC_d || escape == ESC_D {
                *parsed_pattern = (PT_PC << 16) | ucp_Nd;
                parsed_pattern = parsed_pattern.add(1);
            } else if escape == ESC_s || escape == ESC_S {
                *parsed_pattern = PT_SPACE << 16;
                parsed_pattern = parsed_pattern.add(1);
            } else if escape == ESC_w || escape == ESC_W {
                *parsed_pattern = PT_WORD << 16;
                parsed_pattern = parsed_pattern.add(1);
            }
        }

        parsed_pattern
    }
}

/*************************************************
* Maximum size of parsed_pattern for given input *
*************************************************/

/* This function determines the amount of memory to allocate for
parsed_pattern.

Returns:          the number of uint32_t units for parsed_pattern */

pub(crate) unsafe fn max_parsed_pattern(
    ptr: PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    utf: BOOL,
    options: u32,
) -> isize {
    unsafe {
        let big32count: PCRE2_SIZE = 0;
        let mut parsed_size_needed: isize;

        /* The 32-bit non-UTF scan is not applicable in 8-bit mode. */
        let _ = utf;

        parsed_size_needed = ptrend.offset_from(ptr) + big32count as isize;

        if (options & PCRE2_AUTO_CALLOUT) != 0 {
            parsed_size_needed += ptrend.offset_from(ptr) * 4;
        }

        parsed_size_needed
    }
}
