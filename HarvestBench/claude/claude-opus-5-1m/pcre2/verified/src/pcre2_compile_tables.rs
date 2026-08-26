/* Translated from c_src/src/pcre2_compile.c lines 95-835 */

/* The forward declarations of the static functions compile_regex(),
get_branchlength(), set_lookbehind_lengths() and check_lookbehinds() (C lines
95-112) are not needed in Rust. */

/*************************************************
*      Code parameters and static tables         *
*************************************************/

pub const MAX_GROUP_NUMBER: u32 = 65535;
pub const MAX_REPEAT_COUNT: u32 = 65535;
pub const REPEAT_UNLIMITED: u32 = MAX_REPEAT_COUNT + 1;

/* COMPILE_WORK_SIZE specifies the size of stack workspace, which is used in
different ways in the different pattern scans. The parsing and group-
identifying pre-scan uses it to handle nesting, and needs it to be 16-bit
aligned for this. Having defined the size in code units, we set up
C16_WORK_SIZE as the number of elements in the 16-bit vector.

During the first compiling phase, when determining how much memory is required,
the regex is partly compiled into this space, but the compiled parts are
discarded as soon as they can be, so that hopefully there will never be an
overrun. The code does, however, check for an overrun, which can occur for
pathological patterns. The size of the workspace depends on LINK_SIZE because
the length of compiled items varies with this.

In the real compile phase, this workspace is not currently used. */

pub const COMPILE_WORK_SIZE: usize = 3000 * LINK_SIZE; /* Size in code units */

pub const C16_WORK_SIZE: usize =
    (COMPILE_WORK_SIZE * size_of::<PCRE2_UCHAR>()) / size_of::<u16>();

/* A uint32_t vector is used for caching information about the size of
capturing groups, to improve performance. A default is created on the stack of
this size. */

pub const GROUPINFO_DEFAULT_SIZE: usize = 256;

/* The overrun tests check for a slightly smaller size so that they detect the
overrun before it actually does run off the end of the data block. */

pub const WORK_SIZE_SAFETY_MARGIN: usize = 100;

/* This value determines the size of the initial vector that is used for
remembering named groups during the pre-compile. It is allocated on the stack,
but if it is too small, it is expanded, in a similar way to the workspace. The
value is the number of slots in the list. */

pub const NAMED_GROUP_LIST_SIZE: usize = 20;

/* The pre-compiling pass over the pattern creates a parsed pattern in a vector
of uint32_t. For short patterns this lives on the stack, with this size. Heap
memory is used for longer patterns. */

pub const PARSED_PATTERN_DEFAULT_SIZE: usize = 1024;

/* Maximum length value to check against when making sure that the variable
that holds the compiled pattern length does not overflow. We make it a bit less
than INT_MAX to allow for adding in group terminating code units, so that we
don't have to check them every time. */

pub const OFLOW_MAX: usize = (i32::MAX as usize) - 20; /* 2147483627 */

/* Table of extra lengths for each of the meta codes. Must be kept in step with
the definitions above. For some items these values are a basic length to which
a variable amount has to be added. */

static meta_extra_lengths: [u8; 73] = [
    0,                        /* META_END */
    0,                        /* META_ALT */
    0,                        /* META_ATOMIC */
    0,                        /* META_BACKREF - more if group is >= 10 */
    (1 + SIZEOFFSET) as u8,   /* META_BACKREF_BYNAME */
    1,                        /* META_BIGVALUE */
    3,                        /* META_CALLOUT_NUMBER */
    (3 + SIZEOFFSET) as u8,   /* META_CALLOUT_STRING */
    0,                        /* META_CAPTURE */
    0,                        /* META_CIRCUMFLEX */
    0,                        /* META_CLASS */
    0,                        /* META_CLASS_EMPTY */
    0,                        /* META_CLASS_EMPTY_NOT */
    0,                        /* META_CLASS_END */
    0,                        /* META_CLASS_NOT */
    0,                        /* META_COND_ASSERT */
    SIZEOFFSET as u8,         /* META_COND_DEFINE */
    (1 + SIZEOFFSET) as u8,   /* META_COND_NAME */
    (1 + SIZEOFFSET) as u8,   /* META_COND_NUMBER */
    (1 + SIZEOFFSET) as u8,   /* META_COND_RNAME */
    (1 + SIZEOFFSET) as u8,   /* META_COND_RNUMBER */
    3,                        /* META_COND_VERSION */
    SIZEOFFSET as u8,         /* META_OFFSET */
    0,                        /* META_SCS */
    1,                        /* META_CAPTURE_NAME */
    1,                        /* META_CAPTURE_NUMBER */
    0,                        /* META_DOLLAR */
    0,                        /* META_DOT */
    0,                        /* META_ESCAPE - one more for ESC_P and ESC_p */
    0,                        /* META_KET */
    0,                        /* META_NOCAPTURE */
    2,                        /* META_OPTIONS */
    1,                        /* META_POSIX */
    1,                        /* META_POSIX_NEG */
    0,                        /* META_RANGE_ESCAPED */
    0,                        /* META_RANGE_LITERAL */
    SIZEOFFSET as u8,         /* META_RECURSE */
    (1 + SIZEOFFSET) as u8,   /* META_RECURSE_BYNAME */
    0,                        /* META_SCRIPT_RUN */
    0,                        /* META_LOOKAHEAD */
    0,                        /* META_LOOKAHEADNOT */
    SIZEOFFSET as u8,         /* META_LOOKBEHIND */
    SIZEOFFSET as u8,         /* META_LOOKBEHINDNOT */
    0,                        /* META_LOOKAHEAD_NA */
    SIZEOFFSET as u8,         /* META_LOOKBEHIND_NA */
    1,                        /* META_MARK - plus the string length */
    0,                        /* META_ACCEPT */
    0,                        /* META_FAIL */
    0,                        /* META_COMMIT */
    1,                        /* META_COMMIT_ARG - plus the string length */
    0,                        /* META_PRUNE */
    1,                        /* META_PRUNE_ARG - plus the string length */
    0,                        /* META_SKIP */
    1,                        /* META_SKIP_ARG - plus the string length */
    0,                        /* META_THEN */
    1,                        /* META_THEN_ARG - plus the string length */
    0,                        /* META_ASTERISK */
    0,                        /* META_ASTERISK_PLUS */
    0,                        /* META_ASTERISK_QUERY */
    0,                        /* META_PLUS */
    0,                        /* META_PLUS_PLUS */
    0,                        /* META_PLUS_QUERY */
    0,                        /* META_QUERY */
    0,                        /* META_QUERY_PLUS */
    0,                        /* META_QUERY_QUERY */
    2,                        /* META_MINMAX */
    2,                        /* META_MINMAX_PLUS */
    2,                        /* META_MINMAX_QUERY */
    0,                        /* META_ECLASS_AND */
    0,                        /* META_ECLASS_OR */
    0,                        /* META_ECLASS_SUB */
    0,                        /* META_ECLASS_XOR */
    0,                        /* META_ECLASS_NOT */
];

/* Types for skipping parts of a parsed pattern. */

pub const PSKIP_ALT: u32 = 0;
pub const PSKIP_CLASS: u32 = 1;
pub const PSKIP_KET: u32 = 2;

/* Values and flags for the unsigned xxcuflags variables that accompany xxcu
variables, which are concerned with first and required code units. A value
greater than or equal to REQ_NONE means "no code unit set"; otherwise the
matching xxcu variable is set, and the low valued bits are relevant. */

pub const REQ_UNSET: u32 = 0xffffffff; /* Not yet found anything */
pub const REQ_NONE: u32 = 0xfffffffe; /* Found not fixed character */
pub const REQ_CASELESS: u32 = 0x00000001; /* Code unit in xxcu is caseless */
pub const REQ_VARY: u32 = 0x00000002; /* Code unit is followed by non-literal */

/* These flags are used in the groupinfo vector. */

pub const GI_SET_FIXED_LENGTH: u32 = 0x80000000;
pub const GI_NOT_FIXED_LENGTH: u32 = 0x40000000;
pub const GI_FIXED_LENGTH_MASK: u32 = 0x0000ffff;

/* This simple test for a decimal digit works for both ASCII/Unicode and EBCDIC
and is fast (a good compiler can turn it into a subtraction and unsigned
comparison). */

#[allow(unused_macros)]
macro_rules! IS_DIGIT {
    ($x:expr) => {
        ($x) >= CHAR_0 && ($x) <= CHAR_9
    };
}

/* The XDIGIT macro from the 8-bit section at the head of the C file (there is
no MAX_255 test in the 8-bit library). */

#[allow(unused_macros)]
macro_rules! XDIGIT {
    ($c:expr) => {
        *xdigitab.as_ptr().add(($c) as usize)
    };
}

/* Table to identify hex digits. The tables in chartables are dependent on the
locale, and may mark arbitrary characters as digits. We want to recognize only
0-9, a-z, and A-Z as hex digits, which is why we have a private table here. It
costs 256 bytes, but it is a lot faster than doing character value tests (at
least in some simple cases I timed), and in some applications one wants PCRE2
to compile efficiently as well as match efficiently. The value in the table is
the binary hex digit value, or 0xff for non-hex digits. */

/* This is the "normal" case, for ASCII systems, and EBCDIC systems running in
UTF-8 mode. */

static xdigitab: [u8; 256] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /*   0-  7 */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /*   8- 15 */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /*  16- 23 */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /*  24- 31 */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /*    - '  */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /*  ( - /  */
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, /*  0 - 7  */
    0x08, 0x09, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /*  8 - ?  */
    0xff, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0xff, /*  @ - G  */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /*  H - O  */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /*  P - W  */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /*  X - _  */
    0xff, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0xff, /*  ` - g  */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /*  h - o  */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /*  p - w  */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /*  x -127 */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /* 128-135 */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /* 136-143 */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /* 144-151 */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /* 152-159 */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /* 160-167 */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /* 168-175 */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /* 176-183 */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /* 184-191 */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /* 192-199 */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /* 2ff-207 */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /* 208-215 */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /* 216-223 */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /* 224-231 */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /* 232-239 */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /* 240-247 */
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, /* 248-255 */
];

/* Table for handling alphanumeric escaped characters. Positive returns are
simple data values; negative values are for special things like \d and so on.
Zero means further processing is needed (for things like \x), or the escape is
invalid. */

/* This is the "normal" table for ASCII systems or for EBCDIC systems running
in UTF-8 mode. It runs from '0' to 'z'. */

pub const ESCAPES_FIRST: u32 = CHAR_0;
pub const ESCAPES_LAST: u32 = CHAR_z;

#[allow(unused_macros)]
macro_rules! UPPER_CASE {
    ($c:expr) => {
        ($c) - 32
    };
}

static escapes: [i16; 75] = [
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

/* Table of special "verbs" like (*PRUNE). This is a short table, so it is
searched linearly. Put all the names into a single string, in order to reduce
the number of relocations when a shared library is dynamically linked. The
string is built from string macros so that it works in UTF-8 mode on EBCDIC
platforms. */

#[repr(C)]
struct verbitem {
    len: c_uint,   /* Length of verb name */
    meta: u32,     /* Base META_ code */
    has_arg: c_int, /* Argument requirement */
}

/* "\0" (empty name is a shorthand for MARK) STRING_MARK0 STRING_ACCEPT0
STRING_F0 STRING_FAIL0 STRING_COMMIT0 STRING_PRUNE0 STRING_SKIP0 STRING_THEN
(the C string literal supplies the final terminating NUL). */

static verbnames: [u8; 43] = *b"\0MARK\0ACCEPT\0F\0FAIL\0COMMIT\0PRUNE\0SKIP\0THEN\0";

static verbs: [verbitem; 9] = [
    verbitem { len: 0, meta: META_MARK, has_arg: 1 }, /* > 0 => must have an argument */
    verbitem { len: 4, meta: META_MARK, has_arg: 1 },
    verbitem { len: 6, meta: META_ACCEPT, has_arg: -1 }, /* < 0 => Optional argument, convert to pre-MARK */
    verbitem { len: 1, meta: META_FAIL, has_arg: -1 },
    verbitem { len: 4, meta: META_FAIL, has_arg: -1 },
    verbitem { len: 6, meta: META_COMMIT, has_arg: 0 },
    verbitem { len: 5, meta: META_PRUNE, has_arg: 0 }, /* Optional argument; bump META code if found */
    verbitem { len: 4, meta: META_SKIP, has_arg: 0 },
    verbitem { len: 4, meta: META_THEN, has_arg: 0 },
];

static verbcount: c_int = 9; /* sizeof(verbs)/sizeof(verbitem) */

/* Verb opcodes, indexed by their META code offset from META_MARK. */

static verbops: [u32; 11] = [
    OP_MARK, OP_ACCEPT, OP_FAIL, OP_COMMIT, OP_COMMIT_ARG, OP_PRUNE,
    OP_PRUNE_ARG, OP_SKIP, OP_SKIP_ARG, OP_THEN, OP_THEN_ARG,
];

/* Table of "alpha assertions" like (*pla:...), similar to the (*VERB) table. */

#[repr(C)]
struct alasitem {
    len: c_uint, /* Length of name */
    meta: u32,   /* Base META_ code */
}

/* STRING_pla0 STRING_plb0 STRING_napla0 STRING_naplb0 STRING_nla0 STRING_nlb0
STRING_positive_lookahead0 STRING_positive_lookbehind0
STRING_non_atomic_positive_lookahead0 STRING_non_atomic_positive_lookbehind0
STRING_negative_lookahead0 STRING_negative_lookbehind0 STRING_scs0
STRING_scan_substring0 STRING_atomic0 STRING_sr0 STRING_asr0
STRING_script_run0 STRING_atomic_script_run (the C string literal supplies the
final terminating NUL). */

static alasnames: [u8; 229] = *b"pla\0plb\0napla\0naplb\0nla\0nlb\0\
positive_lookahead\0positive_lookbehind\0\
non_atomic_positive_lookahead\0non_atomic_positive_lookbehind\0\
negative_lookahead\0negative_lookbehind\0\
scs\0scan_substring\0atomic\0sr\0asr\0script_run\0atomic_script_run\0";

static alasmeta: [alasitem; 19] = [
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
    alasitem { len: 2, meta: META_SCRIPT_RUN }, /* sr = script run */
    alasitem { len: 3, meta: META_ATOMIC_SCRIPT_RUN }, /* asr = atomic script run */
    alasitem { len: 10, meta: META_SCRIPT_RUN }, /* script run */
    alasitem { len: 17, meta: META_ATOMIC_SCRIPT_RUN }, /* atomic script run */
];

static alascount: c_int = 19; /* sizeof(alasmeta)/sizeof(alasitem) */

/* Offsets from OP_STAR for case-independent and negative repeat opcodes. */

static chartypeoffset: [u32; 4] = [
    OP_STAR - OP_STAR,
    OP_STARI - OP_STAR,
    OP_NOTSTAR - OP_STAR,
    OP_NOTSTARI - OP_STAR,
];

/* Tables of names of POSIX character classes and their lengths. The names are
now all in a single string, to reduce the number of relocations when a shared
library is dynamically loaded. The list of lengths is terminated by a zero
length entry. The first three must be alpha, lower, upper, as this is assumed
for handling case independence.

The indices for several classes are stored in pcre2_compile.h - these must
be kept in sync with posix_names, posix_name_lengths, posix_class_maps,
and posix_substitutes. */

/* STRING_alpha0 STRING_lower0 STRING_upper0 STRING_alnum0 STRING_ascii0
STRING_blank0 STRING_cntrl0 STRING_digit0 STRING_graph0 STRING_print0
STRING_punct0 STRING_space0 STRING_word0 STRING_xdigit (the C string literal
supplies the final terminating NUL). */

static posix_names: [u8; 84] = *b"alpha\0lower\0upper\0alnum\0\
ascii\0blank\0cntrl\0digit\0\
graph\0print\0punct\0space\0\
word\0xdigit\0";

static posix_name_lengths: [u8; 15] = [5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 4, 6, 0];

/* Table of class bit maps for each POSIX class. Each class is formed from a
base map, with an optional addition or removal of another map. Then, for some
classes, there is some additional tweaking: for [:blank:] the vertical space
characters are removed, and for [:alpha:] and [:alnum:] the underscore
character is removed. The triples in the table consist of the base map offset,
second map offset or -1 if no second map, and a non-negative value for map
addition or a negative value for map subtraction (if there are two maps). The
absolute value of the third field has these meanings: 0 => no tweaking, 1 =>
remove vertical space characters, 2 => remove underscore.

PRIV(posix_class_maps) is _pcre2_posix_class_maps8, which is defined in
src/pcre2_compile_class.rs, so it is not repeated here. */

/* The POSIX class Unicode property substitutes that are used in UCP mode must
be in the order of the POSIX class names, defined above. */

static posix_substitutes: [c_int; 28] = [
    PT_GC as c_int, ucp_L as c_int,       /* alpha */
    PT_PC as c_int, ucp_Ll as c_int,      /* lower */
    PT_PC as c_int, ucp_Lu as c_int,      /* upper */
    PT_ALNUM as c_int, 0,                 /* alnum */
    -1, 0,                                /* ascii, treat as non-UCP */
    -1, 1,                                /* blank, treat as \h */
    PT_PC as c_int, ucp_Cc as c_int,      /* cntrl */
    PT_PC as c_int, ucp_Nd as c_int,      /* digit */
    PT_PXGRAPH as c_int, 0,               /* graph */
    PT_PXPRINT as c_int, 0,               /* print */
    PT_PXPUNCT as c_int, 0,               /* punct */
    PT_PXSPACE as c_int, 0,               /* space */   /* Xps is POSIX space, but from 8.34 */
    PT_WORD as c_int, 0,                  /* word  */   /* Perl and POSIX space are the same */
    PT_PXXDIGIT as c_int, 0,              /* xdigit */  /* Perl has additional hex digits */
];

/* Masks for checking option settings. When PCRE2_LITERAL is set, only a subset
are allowed. */

pub const PUBLIC_LITERAL_COMPILE_OPTIONS: u32 = PCRE2_ANCHORED
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

pub const PUBLIC_COMPILE_OPTIONS: u32 = PUBLIC_LITERAL_COMPILE_OPTIONS
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

pub const PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS: u32 = PCRE2_EXTRA_MATCH_LINE
    | PCRE2_EXTRA_MATCH_WORD
    | PCRE2_EXTRA_CASELESS_RESTRICT
    | PCRE2_EXTRA_TURKISH_CASING;

pub const PUBLIC_COMPILE_EXTRA_OPTIONS: u32 = PUBLIC_LITERAL_COMPILE_EXTRA_OPTIONS
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

/* This is a table of start-of-pattern options such as (*UTF) and settings such
as (*LIMIT_MATCH=nnnn) and (*CRLF). For completeness and backward
compatibility, (*UTFn) is supported in the relevant libraries, but (*UTF) is
generic and always supported. */

/* The C enum is an int, but its values are only ever used as the uint16_t
"type" field of a pso, so they are u16 here. */

pub const PSO_OPT: u16 = 0; /* Value is an option bit */
pub const PSO_XOPT: u16 = 1; /* Value is an xoption bit */
pub const PSO_FLG: u16 = 2; /* Value is a flag bit */
pub const PSO_NL: u16 = 3; /* Value is a newline type */
pub const PSO_BSR: u16 = 4; /* Value is a \R type */
pub const PSO_LIMH: u16 = 5; /* Read integer value for heap limit */
pub const PSO_LIMM: u16 = 6; /* Read integer value for match limit */
pub const PSO_LIMD: u16 = 7; /* Read integer value for depth limit */
pub const PSO_OPTMZ: u16 = 8; /* Value is an optimization bit */

#[repr(C)]
struct pso {
    name: *const c_char,
    length: u16,
    r#type: u16,
    value: u32,
}

/* The pso structure contains a raw pointer, which is not Sync; the table is
read-only static data, exactly as in C. */
unsafe impl Sync for pso {}

/* NB: STRING_UTFn_RIGHTPAR contains the length as well (5 for the 8-bit
library, whose name is "UTF8)"). */

static pso_list: [pso; 23] = [
    pso { name: b"UTF8)\0".as_ptr() as *const c_char, length: 5, r#type: PSO_OPT, value: PCRE2_UTF },
    pso { name: b"UTF)\0".as_ptr() as *const c_char, length: 4, r#type: PSO_OPT, value: PCRE2_UTF },
    pso { name: b"UCP)\0".as_ptr() as *const c_char, length: 4, r#type: PSO_OPT, value: PCRE2_UCP },
    pso { name: b"NOTEMPTY)\0".as_ptr() as *const c_char, length: 9, r#type: PSO_FLG, value: PCRE2_NOTEMPTY_SET },
    pso { name: b"NOTEMPTY_ATSTART)\0".as_ptr() as *const c_char, length: 17, r#type: PSO_FLG, value: PCRE2_NE_ATST_SET },
    pso { name: b"NO_AUTO_POSSESS)\0".as_ptr() as *const c_char, length: 16, r#type: PSO_OPTMZ, value: PCRE2_OPTIM_AUTO_POSSESS },
    pso { name: b"NO_DOTSTAR_ANCHOR)\0".as_ptr() as *const c_char, length: 18, r#type: PSO_OPTMZ, value: PCRE2_OPTIM_DOTSTAR_ANCHOR },
    pso { name: b"NO_JIT)\0".as_ptr() as *const c_char, length: 7, r#type: PSO_FLG, value: PCRE2_NOJIT },
    pso { name: b"NO_START_OPT)\0".as_ptr() as *const c_char, length: 13, r#type: PSO_OPTMZ, value: PCRE2_OPTIM_START_OPTIMIZE },
    pso { name: b"CASELESS_RESTRICT)\0".as_ptr() as *const c_char, length: 18, r#type: PSO_XOPT, value: PCRE2_EXTRA_CASELESS_RESTRICT },
    pso { name: b"TURKISH_CASING)\0".as_ptr() as *const c_char, length: 15, r#type: PSO_XOPT, value: PCRE2_EXTRA_TURKISH_CASING },
    pso { name: b"LIMIT_HEAP=\0".as_ptr() as *const c_char, length: 11, r#type: PSO_LIMH, value: 0 },
    pso { name: b"LIMIT_MATCH=\0".as_ptr() as *const c_char, length: 12, r#type: PSO_LIMM, value: 0 },
    pso { name: b"LIMIT_DEPTH=\0".as_ptr() as *const c_char, length: 12, r#type: PSO_LIMD, value: 0 },
    pso { name: b"LIMIT_RECURSION=\0".as_ptr() as *const c_char, length: 16, r#type: PSO_LIMD, value: 0 },
    pso { name: b"CR)\0".as_ptr() as *const c_char, length: 3, r#type: PSO_NL, value: PCRE2_NEWLINE_CR },
    pso { name: b"LF)\0".as_ptr() as *const c_char, length: 3, r#type: PSO_NL, value: PCRE2_NEWLINE_LF },
    pso { name: b"CRLF)\0".as_ptr() as *const c_char, length: 5, r#type: PSO_NL, value: PCRE2_NEWLINE_CRLF },
    pso { name: b"ANY)\0".as_ptr() as *const c_char, length: 4, r#type: PSO_NL, value: PCRE2_NEWLINE_ANY },
    pso { name: b"NUL)\0".as_ptr() as *const c_char, length: 4, r#type: PSO_NL, value: PCRE2_NEWLINE_NUL },
    pso { name: b"ANYCRLF)\0".as_ptr() as *const c_char, length: 8, r#type: PSO_NL, value: PCRE2_NEWLINE_ANYCRLF },
    pso { name: b"BSR_ANYCRLF)\0".as_ptr() as *const c_char, length: 12, r#type: PSO_BSR, value: PCRE2_BSR_ANYCRLF },
    pso { name: b"BSR_UNICODE)\0".as_ptr() as *const c_char, length: 12, r#type: PSO_BSR, value: PCRE2_BSR_UNICODE },
];

/* This table is used when converting repeating opcodes into possessified
versions as a result of an explicit possessive quantifier such as ++. A zero
value means there is no possessified version - in those cases the item in
question must be wrapped in ONCE brackets. The table is truncated at OP_CALLOUT
because all relevant opcodes are less than that. */

static opcode_possessify: [u8; 120] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, /* 0 - 15  */
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, /* 16 - 31 */
    0,                                              /* NOTI */
    OP_POSSTAR as u8, 0,                            /* STAR, MINSTAR */
    OP_POSPLUS as u8, 0,                            /* PLUS, MINPLUS */
    OP_POSQUERY as u8, 0,                           /* QUERY, MINQUERY */
    OP_POSUPTO as u8, 0,                            /* UPTO, MINUPTO */
    0,                                              /* EXACT */
    0, 0, 0, 0,                                     /* POS{STAR,PLUS,QUERY,UPTO} */
    OP_POSSTARI as u8, 0,                           /* STARI, MINSTARI */
    OP_POSPLUSI as u8, 0,                           /* PLUSI, MINPLUSI */
    OP_POSQUERYI as u8, 0,                          /* QUERYI, MINQUERYI */
    OP_POSUPTOI as u8, 0,                           /* UPTOI, MINUPTOI */
    0,                                              /* EXACTI */
    0, 0, 0, 0,                                     /* POS{STARI,PLUSI,QUERYI,UPTOI} */
    OP_NOTPOSSTAR as u8, 0,                         /* NOTSTAR, NOTMINSTAR */
    OP_NOTPOSPLUS as u8, 0,                         /* NOTPLUS, NOTMINPLUS */
    OP_NOTPOSQUERY as u8, 0,                        /* NOTQUERY, NOTMINQUERY */
    OP_NOTPOSUPTO as u8, 0,                         /* NOTUPTO, NOTMINUPTO */
    0,                                              /* NOTEXACT */
    0, 0, 0, 0,                                     /* NOTPOS{STAR,PLUS,QUERY,UPTO} */
    OP_NOTPOSSTARI as u8, 0,                        /* NOTSTARI, NOTMINSTARI */
    OP_NOTPOSPLUSI as u8, 0,                        /* NOTPLUSI, NOTMINPLUSI */
    OP_NOTPOSQUERYI as u8, 0,                       /* NOTQUERYI, NOTMINQUERYI */
    OP_NOTPOSUPTOI as u8, 0,                        /* NOTUPTOI, NOTMINUPTOI */
    0,                                              /* NOTEXACTI */
    0, 0, 0, 0,                                     /* NOTPOS{STARI,PLUSI,QUERYI,UPTOI} */
    OP_TYPEPOSSTAR as u8, 0,                        /* TYPESTAR, TYPEMINSTAR */
    OP_TYPEPOSPLUS as u8, 0,                        /* TYPEPLUS, TYPEMINPLUS */
    OP_TYPEPOSQUERY as u8, 0,                       /* TYPEQUERY, TYPEMINQUERY */
    OP_TYPEPOSUPTO as u8, 0,                        /* TYPEUPTO, TYPEMINUPTO */
    0,                                              /* TYPEEXACT */
    0, 0, 0, 0,                                     /* TYPEPOS{STAR,PLUS,QUERY,UPTO} */
    OP_CRPOSSTAR as u8, 0,                          /* CRSTAR, CRMINSTAR */
    OP_CRPOSPLUS as u8, 0,                          /* CRPLUS, CRMINPLUS */
    OP_CRPOSQUERY as u8, 0,                         /* CRQUERY, CRMINQUERY */
    OP_CRPOSRANGE as u8, 0,                         /* CRRANGE, CRMINRANGE */
    0, 0, 0, 0,                                     /* CRPOS{STAR,PLUS,QUERY,RANGE} */
    0, 0, 0, 0,                                     /* CLASS, NCLASS, XCLASS, ECLASS */
    0, 0,                                           /* REF, REFI */
    0, 0,                                           /* DNREF, DNREFI */
    0, 0,                                           /* RECURSE, CALLOUT */
];

/* Compile-time check that the table has the correct size.
STATIC_ASSERT(sizeof(opcode_possessify) == OP_CALLOUT+1, opcode_possessify); */
const _: () = assert!(opcode_possessify.len() == (OP_CALLOUT as usize) + 1);
