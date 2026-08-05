// Foundational shared definitions for PCRE2 (8-bit, SUPPORT_UNICODE, no JIT).
// Mirrors pcre2.h, pcre2_internal.h, pcre2_intmodedep.h, pcre2_ucp.h.
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_void};

// ---------- Basic types ----------
pub type PCRE2_UCHAR = u8;
pub type PCRE2_SPTR = *const u8;
pub type PCRE2_SIZE = usize;
pub type BOOL = c_int;
pub const TRUE: BOOL = 1;
pub const FALSE: BOOL = 0;

pub const PCRE2_SIZE_MAX: usize = usize::MAX;
pub const PCRE2_ZERO_TERMINATED: usize = !0usize;
pub const PCRE2_UNSET: usize = !0usize;

pub const CODE_UNIT_WIDTH: u32 = 8;
pub const PCRE2_CODE_UNIT_WIDTH: u32 = 8;
pub const LINK_SIZE: usize = 2;
pub const IMM2_SIZE: usize = 2;

// ---------- Config values ----------
pub const HEAP_LIMIT: u32 = 20000000;
pub const MATCH_LIMIT: u32 = 10000000;
pub const MATCH_LIMIT_DEPTH: u32 = 10000000;
pub const MAX_NAME_COUNT: u32 = 10000;
pub const MAX_NAME_SIZE: u32 = 128;
pub const MAX_VARLOOKBEHIND: u32 = 255;
pub const NEWLINE_DEFAULT: u16 = 2;
pub const PARENS_NEST_LIMIT: u32 = 250;
pub const START_FRAMES_SIZE: usize = 20480;
pub const DFA_START_RWS_SIZE: usize = 30720;

pub const MAX_UTF_CODE_POINT: u32 = 0x10ffff;
pub const NOTACHAR: u32 = 0xffffffff;
pub const COMPILE_ERROR_BASE: c_int = 100;
pub const MAGIC_NUMBER: u32 = 0x50435245;
pub const MAX_UTF_SINGLE_CU: u32 = 127;
pub const MAX_NON_UTF_CHAR: u32 = 0xff;
pub const REQ_CU_MAX: u32 = 5000;
pub const ECLASS_NEST_LIMIT: usize = 15;
pub const MAX_PATTERN_SIZE: usize = 1 << 16;
pub const LOOKBEHIND_MAX: c_int = u16::MAX as c_int;

// BSR
pub const PCRE2_BSR_UNICODE: u32 = 1;
pub const PCRE2_BSR_ANYCRLF: u32 = 2;
pub const BSR_DEFAULT: u16 = 1; // PCRE2_BSR_UNICODE

// Newline
pub const PCRE2_NEWLINE_CR: u32 = 1;
pub const PCRE2_NEWLINE_LF: u32 = 2;
pub const PCRE2_NEWLINE_CRLF: u32 = 3;
pub const PCRE2_NEWLINE_ANY: u32 = 4;
pub const PCRE2_NEWLINE_ANYCRLF: u32 = 5;
pub const PCRE2_NEWLINE_NUL: u32 = 6;

pub const NLTYPE_FIXED: u32 = 0;
pub const NLTYPE_ANY: u32 = 1;
pub const NLTYPE_ANYCRLF: u32 = 2;

// ---------- Compile options (pcre2.h) ----------
pub const PCRE2_ANCHORED: u32 = 0x80000000;
pub const PCRE2_NO_UTF_CHECK: u32 = 0x40000000;
pub const PCRE2_ENDANCHORED: u32 = 0x20000000;
pub const PCRE2_ALLOW_EMPTY_CLASS: u32 = 0x00000001;
pub const PCRE2_ALT_BSUX: u32 = 0x00000002;
pub const PCRE2_AUTO_CALLOUT: u32 = 0x00000004;
pub const PCRE2_CASELESS: u32 = 0x00000008;
pub const PCRE2_DOLLAR_ENDONLY: u32 = 0x00000010;
pub const PCRE2_DOTALL: u32 = 0x00000020;
pub const PCRE2_DUPNAMES: u32 = 0x00000040;
pub const PCRE2_EXTENDED: u32 = 0x00000080;
pub const PCRE2_FIRSTLINE: u32 = 0x00000100;
pub const PCRE2_MATCH_UNSET_BACKREF: u32 = 0x00000200;
pub const PCRE2_MULTILINE: u32 = 0x00000400;
pub const PCRE2_NEVER_UCP: u32 = 0x00000800;
pub const PCRE2_NEVER_UTF: u32 = 0x00001000;
pub const PCRE2_NO_AUTO_CAPTURE: u32 = 0x00002000;
pub const PCRE2_NO_AUTO_POSSESS: u32 = 0x00004000;
pub const PCRE2_NO_DOTSTAR_ANCHOR: u32 = 0x00008000;
pub const PCRE2_NO_START_OPTIMIZE: u32 = 0x00010000;
pub const PCRE2_UCP: u32 = 0x00020000;
pub const PCRE2_UNGREEDY: u32 = 0x00040000;
pub const PCRE2_UTF: u32 = 0x00080000;
pub const PCRE2_NEVER_BACKSLASH_C: u32 = 0x00100000;
pub const PCRE2_ALT_CIRCUMFLEX: u32 = 0x00200000;
pub const PCRE2_ALT_VERBNAMES: u32 = 0x00400000;
pub const PCRE2_USE_OFFSET_LIMIT: u32 = 0x00800000;
pub const PCRE2_EXTENDED_MORE: u32 = 0x01000000;
pub const PCRE2_LITERAL: u32 = 0x02000000;
pub const PCRE2_MATCH_INVALID_UTF: u32 = 0x04000000;
pub const PCRE2_ALT_EXTENDED_CLASS: u32 = 0x08000000;

// Extra options
pub const PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES: u32 = 0x00000001;
pub const PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL: u32 = 0x00000002;
pub const PCRE2_EXTRA_MATCH_WORD: u32 = 0x00000004;
pub const PCRE2_EXTRA_MATCH_LINE: u32 = 0x00000008;
pub const PCRE2_EXTRA_ESCAPED_CR_IS_LF: u32 = 0x00000010;
pub const PCRE2_EXTRA_ALT_BSUX: u32 = 0x00000020;
pub const PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK: u32 = 0x00000040;
pub const PCRE2_EXTRA_CASELESS_RESTRICT: u32 = 0x00000080;
pub const PCRE2_EXTRA_ASCII_BSD: u32 = 0x00000100;
pub const PCRE2_EXTRA_ASCII_BSS: u32 = 0x00000200;
pub const PCRE2_EXTRA_ASCII_BSW: u32 = 0x00000400;
pub const PCRE2_EXTRA_ASCII_POSIX: u32 = 0x00000800;
pub const PCRE2_EXTRA_ASCII_DIGIT: u32 = 0x00001000;
pub const PCRE2_EXTRA_PYTHON_OCTAL: u32 = 0x00002000;
pub const PCRE2_EXTRA_NO_BS0: u32 = 0x00004000;
pub const PCRE2_EXTRA_NEVER_CALLOUT: u32 = 0x00008000;
pub const PCRE2_EXTRA_TURKISH_CASING: u32 = 0x00010000;

// JIT options
pub const PCRE2_JIT_COMPLETE: u32 = 0x00000001;
pub const PCRE2_JIT_PARTIAL_SOFT: u32 = 0x00000002;
pub const PCRE2_JIT_PARTIAL_HARD: u32 = 0x00000004;
pub const PCRE2_JIT_INVALID_UTF: u32 = 0x00000100;
pub const PCRE2_JIT_TEST_ALLOC: u32 = 0x00000200;

// Match options
pub const PCRE2_NOTBOL: u32 = 0x00000001;
pub const PCRE2_NOTEOL: u32 = 0x00000002;
pub const PCRE2_NOTEMPTY: u32 = 0x00000004;
pub const PCRE2_NOTEMPTY_ATSTART: u32 = 0x00000008;
pub const PCRE2_PARTIAL_SOFT: u32 = 0x00000010;
pub const PCRE2_PARTIAL_HARD: u32 = 0x00000020;
pub const PCRE2_DFA_RESTART: u32 = 0x00000040;
pub const PCRE2_DFA_SHORTEST: u32 = 0x00000080;
pub const PCRE2_SUBSTITUTE_GLOBAL: u32 = 0x00000100;
pub const PCRE2_SUBSTITUTE_EXTENDED: u32 = 0x00000200;
pub const PCRE2_SUBSTITUTE_UNSET_EMPTY: u32 = 0x00000400;
pub const PCRE2_SUBSTITUTE_UNKNOWN_UNSET: u32 = 0x00000800;
pub const PCRE2_SUBSTITUTE_OVERFLOW_LENGTH: u32 = 0x00001000;
pub const PCRE2_NO_JIT: u32 = 0x00002000;
pub const PCRE2_COPY_MATCHED_SUBJECT: u32 = 0x00004000;
pub const PCRE2_SUBSTITUTE_LITERAL: u32 = 0x00008000;
pub const PCRE2_SUBSTITUTE_MATCHED: u32 = 0x00010000;
pub const PCRE2_SUBSTITUTE_REPLACEMENT_ONLY: u32 = 0x00020000;
pub const PCRE2_DISABLE_RECURSELOOP_CHECK: u32 = 0x00040000;

// Convert options
pub const PCRE2_CONVERT_UTF: u32 = 0x00000001;
pub const PCRE2_CONVERT_NO_UTF_CHECK: u32 = 0x00000002;
pub const PCRE2_CONVERT_POSIX_BASIC: u32 = 0x00000004;
pub const PCRE2_CONVERT_POSIX_EXTENDED: u32 = 0x00000008;
pub const PCRE2_CONVERT_GLOB: u32 = 0x00000010;
pub const PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR: u32 = 0x00000030;
pub const PCRE2_CONVERT_GLOB_NO_STARSTAR: u32 = 0x00000050;

// Optimize directives
pub const PCRE2_OPTIMIZATION_NONE: u32 = 0;
pub const PCRE2_OPTIMIZATION_FULL: u32 = 1;
pub const PCRE2_AUTO_POSSESS: u32 = 64;
pub const PCRE2_AUTO_POSSESS_OFF: u32 = 65;
pub const PCRE2_DOTSTAR_ANCHOR: u32 = 66;
pub const PCRE2_DOTSTAR_ANCHOR_OFF: u32 = 67;
pub const PCRE2_START_OPTIMIZE: u32 = 68;
pub const PCRE2_START_OPTIMIZE_OFF: u32 = 69;

pub const PCRE2_OPTIM_AUTO_POSSESS: u32 = 0x00000001;
pub const PCRE2_OPTIM_DOTSTAR_ANCHOR: u32 = 0x00000002;
pub const PCRE2_OPTIM_START_OPTIMIZE: u32 = 0x00000004;
pub const PCRE2_OPTIMIZATION_ALL: u32 = 0x00000007;

// Substitute case types
pub const PCRE2_SUBSTITUTE_CASE_LOWER: c_int = 1;
pub const PCRE2_SUBSTITUTE_CASE_UPPER: c_int = 2;
pub const PCRE2_SUBSTITUTE_CASE_TITLE_FIRST: c_int = 3;

// Callout flags
pub const PCRE2_CALLOUT_STARTMATCH: u32 = 0x00000001;
pub const PCRE2_CALLOUT_BACKTRACK: u32 = 0x00000002;

// pcre2_pattern_info request types
pub const PCRE2_INFO_ALLOPTIONS: u32 = 0;
pub const PCRE2_INFO_ARGOPTIONS: u32 = 1;
pub const PCRE2_INFO_BACKREFMAX: u32 = 2;
pub const PCRE2_INFO_BSR: u32 = 3;
pub const PCRE2_INFO_CAPTURECOUNT: u32 = 4;
pub const PCRE2_INFO_FIRSTCODEUNIT: u32 = 5;
pub const PCRE2_INFO_FIRSTCODETYPE: u32 = 6;
pub const PCRE2_INFO_FIRSTBITMAP: u32 = 7;
pub const PCRE2_INFO_HASCRORLF: u32 = 8;
pub const PCRE2_INFO_JCHANGED: u32 = 9;
pub const PCRE2_INFO_JITSIZE: u32 = 10;
pub const PCRE2_INFO_LASTCODEUNIT: u32 = 11;
pub const PCRE2_INFO_LASTCODETYPE: u32 = 12;
pub const PCRE2_INFO_MATCHEMPTY: u32 = 13;
pub const PCRE2_INFO_MATCHLIMIT: u32 = 14;
pub const PCRE2_INFO_MAXLOOKBEHIND: u32 = 15;
pub const PCRE2_INFO_MINLENGTH: u32 = 16;
pub const PCRE2_INFO_NAMECOUNT: u32 = 17;
pub const PCRE2_INFO_NAMEENTRYSIZE: u32 = 18;
pub const PCRE2_INFO_NAMETABLE: u32 = 19;
pub const PCRE2_INFO_NEWLINE: u32 = 20;
pub const PCRE2_INFO_DEPTHLIMIT: u32 = 21;
pub const PCRE2_INFO_SIZE: u32 = 22;
pub const PCRE2_INFO_HASBACKSLASHC: u32 = 23;
pub const PCRE2_INFO_FRAMESIZE: u32 = 24;
pub const PCRE2_INFO_HEAPLIMIT: u32 = 25;
pub const PCRE2_INFO_EXTRAOPTIONS: u32 = 26;

// pcre2_config request types
pub const PCRE2_CONFIG_BSR: u32 = 0;
pub const PCRE2_CONFIG_JIT: u32 = 1;
pub const PCRE2_CONFIG_JITTARGET: u32 = 2;
pub const PCRE2_CONFIG_LINKSIZE: u32 = 3;
pub const PCRE2_CONFIG_MATCHLIMIT: u32 = 4;
pub const PCRE2_CONFIG_NEWLINE: u32 = 5;
pub const PCRE2_CONFIG_PARENSLIMIT: u32 = 6;
pub const PCRE2_CONFIG_DEPTHLIMIT: u32 = 7;
pub const PCRE2_CONFIG_STACKRECURSE: u32 = 8;
pub const PCRE2_CONFIG_UNICODE: u32 = 9;
pub const PCRE2_CONFIG_UNICODE_VERSION: u32 = 10;
pub const PCRE2_CONFIG_VERSION: u32 = 11;
pub const PCRE2_CONFIG_HEAPLIMIT: u32 = 12;
pub const PCRE2_CONFIG_NEVER_BACKSLASH_C: u32 = 13;
pub const PCRE2_CONFIG_COMPILED_WIDTHS: u32 = 14;
pub const PCRE2_CONFIG_TABLES_LENGTH: u32 = 15;
pub const PCRE2_CONFIG_EFFECTIVE_LINKSIZE: u32 = 16;

pub const TABLES_LENGTH: usize = 1088; // lcc256+fcc256+cbits320+ctypes256

// ---------- Error codes ----------
pub const PCRE2_ERROR_NOMATCH: c_int = -1;
pub const PCRE2_ERROR_PARTIAL: c_int = -2;
pub const PCRE2_ERROR_UTF8_ERR1: c_int = -3;
pub const PCRE2_ERROR_UTF8_ERR21: c_int = -23;
pub const PCRE2_ERROR_BADDATA: c_int = -29;
pub const PCRE2_ERROR_MIXEDTABLES: c_int = -30;
pub const PCRE2_ERROR_BADMAGIC: c_int = -31;
pub const PCRE2_ERROR_BADMODE: c_int = -32;
pub const PCRE2_ERROR_BADOFFSET: c_int = -33;
pub const PCRE2_ERROR_BADOPTION: c_int = -34;
pub const PCRE2_ERROR_BADREPLACEMENT: c_int = -35;
pub const PCRE2_ERROR_BADUTFOFFSET: c_int = -36;
pub const PCRE2_ERROR_CALLOUT: c_int = -37;
pub const PCRE2_ERROR_DFA_BADRESTART: c_int = -38;
pub const PCRE2_ERROR_DFA_RECURSE: c_int = -39;
pub const PCRE2_ERROR_DFA_UCOND: c_int = -40;
pub const PCRE2_ERROR_DFA_UFUNC: c_int = -41;
pub const PCRE2_ERROR_DFA_UITEM: c_int = -42;
pub const PCRE2_ERROR_DFA_WSSIZE: c_int = -43;
pub const PCRE2_ERROR_INTERNAL: c_int = -44;
pub const PCRE2_ERROR_JIT_BADOPTION: c_int = -45;
pub const PCRE2_ERROR_JIT_STACKLIMIT: c_int = -46;
pub const PCRE2_ERROR_MATCHLIMIT: c_int = -47;
pub const PCRE2_ERROR_NOMEMORY: c_int = -48;
pub const PCRE2_ERROR_NOSUBSTRING: c_int = -49;
pub const PCRE2_ERROR_NOUNIQUESUBSTRING: c_int = -50;
pub const PCRE2_ERROR_NULL: c_int = -51;
pub const PCRE2_ERROR_RECURSELOOP: c_int = -52;
pub const PCRE2_ERROR_DEPTHLIMIT: c_int = -53;
pub const PCRE2_ERROR_UNAVAILABLE: c_int = -54;
pub const PCRE2_ERROR_UNSET: c_int = -55;
pub const PCRE2_ERROR_BADOFFSETLIMIT: c_int = -56;
pub const PCRE2_ERROR_BADREPESCAPE: c_int = -57;
pub const PCRE2_ERROR_REPMISSINGBRACE: c_int = -58;
pub const PCRE2_ERROR_BADSUBSTITUTION: c_int = -59;
pub const PCRE2_ERROR_BADSUBSPATTERN: c_int = -60;
pub const PCRE2_ERROR_TOOMANYREPLACE: c_int = -61;
pub const PCRE2_ERROR_BADSERIALIZEDDATA: c_int = -62;
pub const PCRE2_ERROR_HEAPLIMIT: c_int = -63;
pub const PCRE2_ERROR_CONVERT_SYNTAX: c_int = -64;
pub const PCRE2_ERROR_INTERNAL_DUPMATCH: c_int = -65;
pub const PCRE2_ERROR_DFA_UINVALID_UTF: c_int = -66;
pub const PCRE2_ERROR_INVALIDOFFSET: c_int = -67;
pub const PCRE2_ERROR_JIT_UNSUPPORTED: c_int = -68;
pub const PCRE2_ERROR_REPLACECASE: c_int = -69;
pub const PCRE2_ERROR_TOOLARGEREPLACE: c_int = -70;
pub const PCRE2_ERROR_DIFFSUBSPATTERN: c_int = -71;
pub const PCRE2_ERROR_DIFFSUBSSUBJECT: c_int = -72;
pub const PCRE2_ERROR_DIFFSUBSOFFSET: c_int = -73;
pub const PCRE2_ERROR_DIFFSUBSOPTIONS: c_int = -74;
pub const PCRE2_ERROR_BAD_BACKSLASH_K: c_int = -75;
pub const PCRE2_ERROR_PARTIALSUBS: c_int = -76;

// ---------- Private compiled-pattern flags ----------
pub const PCRE2_MODE8: u32 = 0x00000001;
pub const PCRE2_MODE16: u32 = 0x00000002;
pub const PCRE2_MODE32: u32 = 0x00000004;
pub const PCRE2_FIRSTSET: u32 = 0x00000010;
pub const PCRE2_FIRSTCASELESS: u32 = 0x00000020;
pub const PCRE2_FIRSTMAPSET: u32 = 0x00000040;
pub const PCRE2_LASTSET: u32 = 0x00000080;
pub const PCRE2_LASTCASELESS: u32 = 0x00000100;
pub const PCRE2_STARTLINE: u32 = 0x00000200;
pub const PCRE2_JCHANGED: u32 = 0x00000400;
pub const PCRE2_HASCRORLF: u32 = 0x00000800;
pub const PCRE2_HASTHEN: u32 = 0x00001000;
pub const PCRE2_MATCH_EMPTY: u32 = 0x00002000;
pub const PCRE2_BSR_SET: u32 = 0x00004000;
pub const PCRE2_NL_SET: u32 = 0x00008000;
pub const PCRE2_NOTEMPTY_SET: u32 = 0x00010000;
pub const PCRE2_NE_ATST_SET: u32 = 0x00020000;
pub const PCRE2_DEREF_TABLES: u32 = 0x00040000;
pub const PCRE2_NOJIT: u32 = 0x00080000;
pub const PCRE2_HASBKPORX: u32 = 0x00100000;
pub const PCRE2_DUPCAPUSED: u32 = 0x00200000;
pub const PCRE2_HASBKC: u32 = 0x00400000;
pub const PCRE2_HASACCEPT: u32 = 0x00800000;
pub const PCRE2_HASBSK: u32 = 0x01000000;
pub const PCRE2_MODE_MASK: u32 = PCRE2_MODE8 | PCRE2_MODE16 | PCRE2_MODE32;

pub const PCRE2_MATCHEDBY_INTERPRETER: u8 = 0;
pub const PCRE2_MATCHEDBY_DFA_INTERPRETER: u8 = 1;
pub const PCRE2_MATCHEDBY_JIT: u8 = 2;
pub const PCRE2_MD_COPIED_SUBJECT: u8 = 0x01;

// ---------- cbits / ctype offsets ----------
pub const cbit_space: usize = 0;
pub const cbit_xdigit: usize = 32;
pub const cbit_digit: usize = 64;
pub const cbit_upper: usize = 96;
pub const cbit_lower: usize = 128;
pub const cbit_word: usize = 160;
pub const cbit_graph: usize = 192;
pub const cbit_print: usize = 224;
pub const cbit_punct: usize = 256;
pub const cbit_cntrl: usize = 288;
pub const cbit_length: usize = 320;

pub const ctype_space: u8 = 0x01;
pub const ctype_letter: u8 = 0x02;
pub const ctype_lcletter: u8 = 0x04;
pub const ctype_digit: u8 = 0x08;
pub const ctype_word: u8 = 0x10;

pub const lcc_offset: usize = 0;
pub const fcc_offset: usize = 256;
pub const cbits_offset: usize = 512;
pub const ctypes_offset: usize = cbits_offset + cbit_length;

// ---------- Property types ----------
pub const PT_LAMP: u32 = 0;
pub const PT_GC: u32 = 1;
pub const PT_PC: u32 = 2;
pub const PT_SC: u32 = 3;
pub const PT_SCX: u32 = 4;
pub const PT_ALNUM: u32 = 5;
pub const PT_SPACE: u32 = 6;
pub const PT_PXSPACE: u32 = 7;
pub const PT_WORD: u32 = 8;
pub const PT_CLIST: u32 = 9;
pub const PT_UCNC: u32 = 10;
pub const PT_BIDICL: u32 = 11;
pub const PT_BOOL: u32 = 12;
pub const PT_ANY: u32 = 13;
pub const PT_TABSIZE: usize = 13;
pub const PT_PXGRAPH: u32 = 14;
pub const PT_PXPRINT: u32 = 15;
pub const PT_PXPUNCT: u32 = 16;
pub const PT_PXXDIGIT: u32 = 17;
pub const PT_NOTSCRIPT: u32 = 255;

// XCLASS
pub const XCL_NOT: u8 = 0x01;
pub const XCL_MAP: u8 = 0x02;
pub const XCL_HASPROP: u8 = 0x04;
pub const XCL_END: u8 = 0;
pub const XCL_SINGLE: u8 = 1;
pub const XCL_RANGE: u8 = 2;
pub const XCL_PROP: u8 = 3;
pub const XCL_NOTPROP: u8 = 4;
pub const XCL_LIST: u32 = 0x10; // sizeof(PCRE2_UCHAR)==1
pub const XCL_CHAR_LIST_LOW_16_START: u32 = 0x100;
pub const XCL_CHAR_LIST_LOW_16_END: u32 = 0x7fff;
pub const XCL_CHAR_LIST_LOW_16_ADD: u32 = 0x0;
pub const XCL_CHAR_LIST_HIGH_16_START: u32 = 0x8000;
pub const XCL_CHAR_LIST_HIGH_16_END: u32 = 0xffff;
pub const XCL_CHAR_LIST_HIGH_16_ADD: u32 = 0x8000;
pub const XCL_CHAR_LIST_LOW_32_START: u32 = 0x10000;
pub const XCL_CHAR_LIST_LOW_32_END: u32 = 0x7fffffff;
pub const XCL_CHAR_LIST_LOW_32_ADD: u32 = 0x0;
pub const XCL_CHAR_LIST_HIGH_32_START: u32 = 0x80000000;
pub const XCL_CHAR_LIST_HIGH_32_END: u32 = 0xffffffff;
pub const XCL_CHAR_LIST_HIGH_32_ADD: u32 = 0x80000000;
pub const XCL_TYPE_MASK: u32 = 0xfff;
pub const XCL_TYPE_BIT_LEN: u32 = 3;
pub const XCL_BEGIN_WITH_RANGE: u32 = 0x4;
pub const XCL_ITEM_COUNT_MASK: u32 = 0x3;
pub const XCL_CHAR_END: u32 = 0x1;
pub const XCL_CHAR_SHIFT: u32 = 1;

pub const ECL_MAP: u8 = 0x01;
pub const ECL_AND: u8 = 1;
pub const ECL_OR: u8 = 2;
pub const ECL_XOR: u8 = 3;
pub const ECL_NOT: u8 = 4;
pub const ECL_XCLASS: u8 = 5;
pub const ECL_ANY: u8 = 6;
pub const ECL_NONE: u8 = 7;

// ESC_ escapes
pub const ESC_A: c_int = 1;
pub const ESC_G: c_int = 2;
pub const ESC_K: c_int = 3;
pub const ESC_B: c_int = 4;
pub const ESC_b: c_int = 5;
pub const ESC_D: c_int = 6;
pub const ESC_d: c_int = 7;
pub const ESC_S: c_int = 8;
pub const ESC_s: c_int = 9;
pub const ESC_W: c_int = 10;
pub const ESC_w: c_int = 11;
pub const ESC_N: c_int = 12;
pub const ESC_dum: c_int = 13;
pub const ESC_C: c_int = 14;
pub const ESC_P: c_int = 15;
pub const ESC_p: c_int = 16;
pub const ESC_R: c_int = 17;
pub const ESC_H: c_int = 18;
pub const ESC_h: c_int = 19;
pub const ESC_V: c_int = 20;
pub const ESC_v: c_int = 21;
pub const ESC_X: c_int = 22;
pub const ESC_Z: c_int = 23;
pub const ESC_z: c_int = 24;
pub const ESC_E: c_int = 25;
pub const ESC_Q: c_int = 26;
pub const ESC_g: c_int = 27;
pub const ESC_k: c_int = 28;
pub const ESC_ub: c_int = 29;

pub const RREF_ANY: u32 = 0xffff;
pub const REFI_FLAG_CASELESS_RESTRICT: u32 = 0x1;
pub const REFI_FLAG_TURKISH_CASING: u32 = 0x2;

// UCD access
pub const UCD_BLOCK_SIZE: usize = 128;
pub const UCD_SCRIPTX_MASK: u16 = 0x3ff;
pub const UCD_BIDICLASS_SHIFT: u16 = 11;
pub const UCD_BPROPS_MASK: u16 = 0xfff;
pub const ucd_boolprop_sets_item_size: usize = 2;
pub const ucd_script_sets_item_size: usize = 4;

// ---------- UCP category enums ----------
pub const ucp_C: u32 = 0;
pub const ucp_L: u32 = 1;
pub const ucp_M: u32 = 2;
pub const ucp_N: u32 = 3;
pub const ucp_P: u32 = 4;
pub const ucp_S: u32 = 5;
pub const ucp_Z: u32 = 6;

pub const ucp_Cc: u32 = 0;
pub const ucp_Cf: u32 = 1;
pub const ucp_Cn: u32 = 2;
pub const ucp_Co: u32 = 3;
pub const ucp_Cs: u32 = 4;
pub const ucp_Ll: u32 = 5;
pub const ucp_Lm: u32 = 6;
pub const ucp_Lo: u32 = 7;
pub const ucp_Lt: u32 = 8;
pub const ucp_Lu: u32 = 9;
pub const ucp_Mc: u32 = 10;
pub const ucp_Me: u32 = 11;
pub const ucp_Mn: u32 = 12;
pub const ucp_Nd: u32 = 13;
pub const ucp_Nl: u32 = 14;
pub const ucp_No: u32 = 15;
pub const ucp_Pc: u32 = 16;
pub const ucp_Pd: u32 = 17;
pub const ucp_Pe: u32 = 18;
pub const ucp_Pf: u32 = 19;
pub const ucp_Pi: u32 = 20;
pub const ucp_Po: u32 = 21;
pub const ucp_Ps: u32 = 22;
pub const ucp_Sc: u32 = 23;
pub const ucp_Sk: u32 = 24;
pub const ucp_Sm: u32 = 25;
pub const ucp_So: u32 = 26;
pub const ucp_Zl: u32 = 27;
pub const ucp_Zp: u32 = 28;
pub const ucp_Zs: u32 = 29;

// Grapheme break props
pub const ucp_gbCR: u32 = 0;
pub const ucp_gbLF: u32 = 1;
pub const ucp_gbControl: u32 = 2;
pub const ucp_gbExtend: u32 = 3;
pub const ucp_gbPrepend: u32 = 4;
pub const ucp_gbSpacingMark: u32 = 5;
pub const ucp_gbL: u32 = 6;
pub const ucp_gbV: u32 = 7;
pub const ucp_gbT: u32 = 8;
pub const ucp_gbLV: u32 = 9;
pub const ucp_gbLVT: u32 = 10;
pub const ucp_gbRegional_Indicator: u32 = 11;
pub const ucp_gbOther: u32 = 12;
pub const ucp_gbZWJ: u32 = 13;
pub const ucp_gbExtended_Pictographic: u32 = 14;

// Scripts used in code (exact values from pcre2_ucp.h)
pub const ucp_Latin: u32 = 0;
pub const ucp_Hangul: u32 = 22;
pub const ucp_Hiragana: u32 = 27;
pub const ucp_Katakana: u32 = 28;
pub const ucp_Bopomofo: u32 = 29;
pub const ucp_Han: u32 = 30;
pub const ucp_Unknown: u32 = 99;
pub const ucp_Common: u32 = 100;
pub const ucp_Inherited: u32 = 107;
pub const ucp_Script_Count: u32 = 175;

// MAPBIT/MAPSET helpers over u32 bitmap slices
#[inline]
pub fn MAPBIT(map: &[u32], n: u32) -> u32 { map[(n / 32) as usize] & (1u32 << (n % 32)) }
#[inline]
pub fn MAPSET(map: &mut [u32], n: u32) { map[(n / 32) as usize] |= 1u32 << (n % 32); }

#[inline]
pub fn UCD_SCRIPTX_PROP(p: &UcdRecord) -> u32 { (p.scriptx_bidiclass & UCD_SCRIPTX_MASK) as u32 }
#[inline]
pub fn UCD_BPROPS_PROP(p: &UcdRecord) -> u32 { (p.bprops & UCD_BPROPS_MASK) as u32 }
#[inline]
pub fn UCD_BIDICLASS_PROP(p: &UcdRecord) -> u32 { (p.scriptx_bidiclass >> UCD_BIDICLASS_SHIFT) as u32 }
#[inline]
pub fn UCD_ANY_I(ch: u32) -> bool {
    (ch | 0x20) == 0x69 || (ch | 1) == 0x0131
}
#[inline]
pub fn UCD_DOTTED_I(ch: u32) -> bool {
    ch == 0x69 || ch == 0x0130
}
#[inline]
pub fn UCD_FOLD_I_TURKISH(ch: u32) -> u32 {
    if ch == 0x0130 { 0x69 } else if ch == 0x49 { 0x0131 } else { ch }
}

pub const ucp_bidiL: u16 = 9;

// ---------- Character constants (ASCII/UTF-8) ----------
pub const CHAR_NUL: u32 = 0;
pub const CHAR_HT: u32 = 0x09;
pub const CHAR_LF: u32 = 0x0a;
pub const CHAR_NL: u32 = 0x0a;
pub const CHAR_VT: u32 = 0x0b;
pub const CHAR_FF: u32 = 0x0c;
pub const CHAR_CR: u32 = 0x0d;
pub const CHAR_NEL: u32 = 0x85;
pub const CHAR_BS: u32 = 0x08;
pub const CHAR_BEL: u32 = 0x07;
pub const CHAR_ESC: u32 = 0x1b;
pub const CHAR_DEL: u32 = 0x7f;
pub const CHAR_SPACE: u32 = 0x20;
pub const CHAR_NBSP: u32 = 0xa0;
pub const CHAR_EXCLAMATION_MARK: u32 = 0x21;
pub const CHAR_QUOTATION_MARK: u32 = 0x22;
pub const CHAR_NUMBER_SIGN: u32 = 0x23;
pub const CHAR_DOLLAR_SIGN: u32 = 0x24;
pub const CHAR_PERCENT_SIGN: u32 = 0x25;
pub const CHAR_AMPERSAND: u32 = 0x26;
pub const CHAR_APOSTROPHE: u32 = 0x27;
pub const CHAR_LEFT_PARENTHESIS: u32 = 0x28;
pub const CHAR_RIGHT_PARENTHESIS: u32 = 0x29;
pub const CHAR_ASTERISK: u32 = 0x2a;
pub const CHAR_PLUS: u32 = 0x2b;
pub const CHAR_COMMA: u32 = 0x2c;
pub const CHAR_MINUS: u32 = 0x2d;
pub const CHAR_DOT: u32 = 0x2e;
pub const CHAR_SLASH: u32 = 0x2f;
pub const CHAR_0: u32 = 0x30;
pub const CHAR_1: u32 = 0x31;
pub const CHAR_2: u32 = 0x32;
pub const CHAR_3: u32 = 0x33;
pub const CHAR_4: u32 = 0x34;
pub const CHAR_5: u32 = 0x35;
pub const CHAR_6: u32 = 0x36;
pub const CHAR_7: u32 = 0x37;
pub const CHAR_8: u32 = 0x38;
pub const CHAR_9: u32 = 0x39;
pub const CHAR_COLON: u32 = 0x3a;
pub const CHAR_SEMICOLON: u32 = 0x3b;
pub const CHAR_LESS_THAN_SIGN: u32 = 0x3c;
pub const CHAR_EQUALS_SIGN: u32 = 0x3d;
pub const CHAR_GREATER_THAN_SIGN: u32 = 0x3e;
pub const CHAR_QUESTION_MARK: u32 = 0x3f;
pub const CHAR_COMMERCIAL_AT: u32 = 0x40;
pub const CHAR_A: u32 = 0x41;
pub const CHAR_B: u32 = 0x42;
pub const CHAR_C: u32 = 0x43;
pub const CHAR_D: u32 = 0x44;
pub const CHAR_E: u32 = 0x45;
pub const CHAR_F: u32 = 0x46;
pub const CHAR_G: u32 = 0x47;
pub const CHAR_H: u32 = 0x48;
pub const CHAR_I: u32 = 0x49;
pub const CHAR_J: u32 = 0x4a;
pub const CHAR_K: u32 = 0x4b;
pub const CHAR_L: u32 = 0x4c;
pub const CHAR_M: u32 = 0x4d;
pub const CHAR_N: u32 = 0x4e;
pub const CHAR_O: u32 = 0x4f;
pub const CHAR_P: u32 = 0x50;
pub const CHAR_Q: u32 = 0x51;
pub const CHAR_R: u32 = 0x52;
pub const CHAR_S: u32 = 0x53;
pub const CHAR_T: u32 = 0x54;
pub const CHAR_U: u32 = 0x55;
pub const CHAR_V: u32 = 0x56;
pub const CHAR_W: u32 = 0x57;
pub const CHAR_X: u32 = 0x58;
pub const CHAR_Y: u32 = 0x59;
pub const CHAR_Z: u32 = 0x5a;
pub const CHAR_LEFT_SQUARE_BRACKET: u32 = 0x5b;
pub const CHAR_BACKSLASH: u32 = 0x5c;
pub const CHAR_RIGHT_SQUARE_BRACKET: u32 = 0x5d;
pub const CHAR_CIRCUMFLEX_ACCENT: u32 = 0x5e;
pub const CHAR_UNDERSCORE: u32 = 0x5f;
pub const CHAR_GRAVE_ACCENT: u32 = 0x60;
pub const CHAR_a: u32 = 0x61;
pub const CHAR_b: u32 = 0x62;
pub const CHAR_c: u32 = 0x63;
pub const CHAR_d: u32 = 0x64;
pub const CHAR_e: u32 = 0x65;
pub const CHAR_f: u32 = 0x66;
pub const CHAR_g: u32 = 0x67;
pub const CHAR_h: u32 = 0x68;
pub const CHAR_i: u32 = 0x69;
pub const CHAR_j: u32 = 0x6a;
pub const CHAR_k: u32 = 0x6b;
pub const CHAR_l: u32 = 0x6c;
pub const CHAR_m: u32 = 0x6d;
pub const CHAR_n: u32 = 0x6e;
pub const CHAR_o: u32 = 0x6f;
pub const CHAR_p: u32 = 0x70;
pub const CHAR_q: u32 = 0x71;
pub const CHAR_r: u32 = 0x72;
pub const CHAR_s: u32 = 0x73;
pub const CHAR_t: u32 = 0x74;
pub const CHAR_u: u32 = 0x75;
pub const CHAR_v: u32 = 0x76;
pub const CHAR_w: u32 = 0x77;
pub const CHAR_x: u32 = 0x78;
pub const CHAR_y: u32 = 0x79;
pub const CHAR_z: u32 = 0x7a;
pub const CHAR_LEFT_CURLY_BRACKET: u32 = 0x7b;
pub const CHAR_VERTICAL_LINE: u32 = 0x7c;
pub const CHAR_RIGHT_CURLY_BRACKET: u32 = 0x7d;
pub const CHAR_TILDE: u32 = 0x7e;

// ---------- repr(C) data-table element structs ----------
#[repr(C)]
#[derive(Clone, Copy)]
pub struct UcdRecord {
    pub script: u8,
    pub chartype: u8,
    pub gbprop: u8,
    pub caseset: u8,
    pub other_case: i32,
    pub scriptx_bidiclass: u16,
    pub bprops: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UcpTypeTable {
    pub name_offset: u16,
    pub type_: u16,
    pub value: u16,
}

// ---------- Memory control ----------
pub type MallocFn = Option<unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void>;
pub type FreeFn = Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct pcre2_memctl {
    pub malloc: MallocFn,
    pub free: FreeFn,
    pub memory_data: *mut c_void,
}

// ---------- Public callback types ----------
pub type JitCallback = Option<unsafe extern "C" fn(*mut c_void) -> *mut c_void>;
pub type CalloutFn = Option<unsafe extern "C" fn(*mut pcre2_callout_block, *mut c_void) -> c_int>;
pub type SubstCalloutFn =
    Option<unsafe extern "C" fn(*mut pcre2_substitute_callout_block, *mut c_void) -> c_int>;
pub type SubstCaseCalloutFn = Option<
    unsafe extern "C" fn(PCRE2_SPTR, PCRE2_SIZE, *mut PCRE2_UCHAR, PCRE2_SIZE, c_int, *mut c_void) -> PCRE2_SIZE,
>;
pub type StackGuardFn = Option<unsafe extern "C" fn(u32, *mut c_void) -> c_int>;

// ---------- Public callout block structures ----------
#[repr(C)]
pub struct pcre2_callout_block {
    pub version: u32,
    pub callout_number: u32,
    pub capture_top: u32,
    pub capture_last: u32,
    pub offset_vector: *mut PCRE2_SIZE,
    pub mark: PCRE2_SPTR,
    pub subject: PCRE2_SPTR,
    pub subject_length: PCRE2_SIZE,
    pub start_match: PCRE2_SIZE,
    pub current_position: PCRE2_SIZE,
    pub pattern_position: PCRE2_SIZE,
    pub next_item_length: PCRE2_SIZE,
    pub callout_string_offset: PCRE2_SIZE,
    pub callout_string_length: PCRE2_SIZE,
    pub callout_string: PCRE2_SPTR,
    pub callout_flags: u32,
}

#[repr(C)]
pub struct pcre2_callout_enumerate_block {
    pub version: u32,
    pub pattern_position: PCRE2_SIZE,
    pub next_item_length: PCRE2_SIZE,
    pub callout_number: u32,
    pub callout_string_offset: PCRE2_SIZE,
    pub callout_string_length: PCRE2_SIZE,
    pub callout_string: PCRE2_SPTR,
}

#[repr(C)]
pub struct pcre2_substitute_callout_block {
    pub version: u32,
    pub input: PCRE2_SPTR,
    pub output: PCRE2_SPTR,
    pub output_offsets: [PCRE2_SIZE; 2],
    pub ovector: *mut PCRE2_SIZE,
    pub oveccount: u32,
    pub subscount: u32,
}

// ---------- Hidden context structures ----------
#[repr(C)]
#[derive(Clone, Copy)]
pub struct pcre2_real_general_context {
    pub memctl: pcre2_memctl,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct pcre2_real_compile_context {
    pub memctl: pcre2_memctl,
    pub stack_guard: StackGuardFn,
    pub stack_guard_data: *mut c_void,
    pub tables: *const u8,
    pub max_pattern_length: PCRE2_SIZE,
    pub max_pattern_compiled_length: PCRE2_SIZE,
    pub bsr_convention: u16,
    pub newline_convention: u16,
    pub parens_nest_limit: u32,
    pub extra_options: u32,
    pub max_varlookbehind: u32,
    pub optimization_flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct pcre2_real_match_context {
    pub memctl: pcre2_memctl,
    // SUPPORT_JIT is not defined, so no jit_callback fields.
    pub callout: CalloutFn,
    pub callout_data: *mut c_void,
    pub substitute_callout: SubstCalloutFn,
    pub substitute_callout_data: *mut c_void,
    pub substitute_case_callout: SubstCaseCalloutFn,
    pub substitute_case_callout_data: *mut c_void,
    pub offset_limit: PCRE2_SIZE,
    pub heap_limit: u32,
    pub match_limit: u32,
    pub depth_limit: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct pcre2_real_convert_context {
    pub memctl: pcre2_memctl,
    pub glob_separator: u32,
    pub glob_escape: u32,
}

#[repr(C)]
pub struct pcre2_real_code {
    pub memctl: pcre2_memctl,
    pub tables: *const u8,
    pub executable_jit: *mut c_void,
    pub start_bitmap: [u8; 32],
    pub blocksize: PCRE2_SIZE,
    pub code_start: PCRE2_SIZE,
    pub magic_number: u32,
    pub compile_options: u32,
    pub overall_options: u32,
    pub extra_options: u32,
    pub flags: u32,
    pub limit_heap: u32,
    pub limit_match: u32,
    pub limit_depth: u32,
    pub first_codeunit: u32,
    pub last_codeunit: u32,
    pub bsr_convention: u16,
    pub newline_convention: u16,
    pub max_lookbehind: u16,
    pub minlength: u16,
    pub top_bracket: u16,
    pub top_backref: u16,
    pub name_entry_size: u16,
    pub name_count: u16,
    pub optimization_flags: u32,
}

// The match data: ovector is variable length; header up to ovector matters.
#[repr(C)]
pub struct pcre2_real_match_data {
    pub memctl: pcre2_memctl,
    pub code: *const pcre2_real_code,
    pub subject: PCRE2_SPTR,
    pub mark: PCRE2_SPTR,
    pub heapframes: *mut c_void,
    pub heapframes_size: PCRE2_SIZE,
    pub subject_length: PCRE2_SIZE,
    pub start_offset: PCRE2_SIZE,
    pub leftchar: PCRE2_SIZE,
    pub rightchar: PCRE2_SIZE,
    pub startchar: PCRE2_SIZE,
    pub matchedby: u8,
    pub flags: u8,
    pub oveccount: u16,
    pub options: u32,
    pub rc: c_int,
    pub ovector: [PCRE2_SIZE; 1], // flexible array in practice
}

/// Offset (in bytes) of the ovector field in pcre2_real_match_data.
pub fn match_data_ovector_offset() -> usize {
    // memoffset-free: build via manual layout knowledge is error-prone; use core.
    // 12 usize-ish fields precede; compute with a dummy.
    #[repr(C)]
    struct Head {
        memctl: pcre2_memctl,
        code: *const pcre2_real_code,
        subject: PCRE2_SPTR,
        mark: PCRE2_SPTR,
        heapframes: *mut c_void,
        heapframes_size: PCRE2_SIZE,
        subject_length: PCRE2_SIZE,
        start_offset: PCRE2_SIZE,
        leftchar: PCRE2_SIZE,
        rightchar: PCRE2_SIZE,
        startchar: PCRE2_SIZE,
        matchedby: u8,
        flags: u8,
        oveccount: u16,
        options: u32,
        rc: c_int,
    }
    // ovector must be 8-aligned; the C struct places ovector right after rc with
    // padding to PCRE2_SIZE alignment.
    let base = core::mem::size_of::<Head>();
    let align = core::mem::align_of::<PCRE2_SIZE>();
    (base + align - 1) & !(align - 1)
}

pub type pcre2_general_context = pcre2_real_general_context;
pub type pcre2_compile_context = pcre2_real_compile_context;
pub type pcre2_match_context = pcre2_real_match_context;
pub type pcre2_convert_context = pcre2_real_convert_context;
pub type pcre2_code = pcre2_real_code;
pub type pcre2_match_data = pcre2_real_match_data;

#[repr(C)]
pub struct pcre2_real_jit_stack {
    pub memctl: pcre2_memctl,
    pub stack: *mut c_void,
}

// ---------- Serialized header ----------
#[repr(C)]
pub struct pcre2_serialized_data {
    pub magic: u32,
    pub version: u32,
    pub config: u32,
    pub number_of_codes: i32,
}

// ---------- small compile helper structs ----------
#[repr(C)]
pub struct open_capitem {
    pub next: *mut open_capitem,
    pub number: u16,
    pub assert_depth: u16,
}

#[repr(C)]
pub struct named_group {
    pub name: PCRE2_SPTR,
    pub number: u32,
    pub length: u16,
    pub hash_dup: u16,
}

// ---------- Private compile structures ----------
#[repr(C)]
pub struct recurse_check {
    pub prev: *mut recurse_check,
    pub group: PCRE2_SPTR,
}

#[repr(C)]
pub struct parsed_recurse_check {
    pub prev: *mut parsed_recurse_check,
    pub groupptr: *mut u32,
}

#[repr(C)]
pub struct recurse_cache {
    pub group: PCRE2_SPTR,
    pub groupnumber: c_int,
}

#[repr(C)]
pub struct branch_chain {
    pub outer: *mut branch_chain,
    pub current_branch: *mut PCRE2_UCHAR,
}

#[repr(C)]
pub struct compile_data {
    pub next: *mut compile_data,
}

#[repr(C)]
pub struct class_ranges {
    pub header: compile_data,
    pub char_lists_size: usize,
    pub char_lists_start: usize,
    pub range_list_size: u16,
    pub char_lists_types: u16,
}

#[repr(C)]
pub struct recurse_arguments {
    pub header: compile_data,
    pub size: usize,
    pub skip_size: usize,
}

#[repr(C)]
pub union class_bits_storage {
    pub classbits: [u8; 32],
    pub classwords: [u32; 8],
}

#[repr(C)]
pub struct compile_block {
    pub cx: *mut pcre2_real_compile_context,
    pub lcc: *const u8,
    pub fcc: *const u8,
    pub cbits: *const u8,
    pub ctypes: *const u8,
    pub start_workspace: *mut PCRE2_UCHAR,
    pub start_code: *mut PCRE2_UCHAR,
    pub start_pattern: PCRE2_SPTR,
    pub end_pattern: PCRE2_SPTR,
    pub name_table: *mut PCRE2_UCHAR,
    pub workspace_size: PCRE2_SIZE,
    pub small_ref_offset: [PCRE2_SIZE; 10],
    pub erroroffset: PCRE2_SIZE,
    pub classbits: class_bits_storage,
    pub names_found: u16,
    pub name_entry_size: u16,
    pub parens_depth: u16,
    pub assert_depth: u16,
    pub named_groups: *mut named_group,
    pub named_group_list_size: u32,
    pub external_options: u32,
    pub external_flags: u32,
    pub bracount: u32,
    pub lastcapture: u32,
    pub parsed_pattern: *mut u32,
    pub parsed_pattern_end: *mut u32,
    pub groupinfo: *mut u32,
    pub top_backref: u32,
    pub backref_map: u32,
    pub nltype: u32,
    pub nllen: u32,
    pub nl: [PCRE2_UCHAR; 4],
    pub class_op_used: [u8; ECLASS_NEST_LIMIT],
    pub req_varyopt: u32,
    pub max_varlookbehind: u32,
    pub max_lookbehind: c_int,
    pub had_accept: BOOL,
    pub had_pruneorskip: BOOL,
    pub had_recurse: BOOL,
    pub dupnames: BOOL,
    pub first_data: *mut compile_data,
    pub last_data: *mut compile_data,
    pub char_lists_size: usize, // SUPPORT_WIDE_CHARS (8-bit + unicode)
}

pub type ucp_type_table = UcpTypeTable;
pub type ucd_record = UcdRecord;

#[inline]
pub fn SELECT_VALUE8(value8: u32, _value: u32) -> u32 { value8 }
#[inline]
pub fn CLIST_ALIGN_TO(base: usize, align: usize) -> usize {
    (base + (align - 1)) & !(align - 1)
}

// ---------- Opcodes ----------
pub const OP_END: u8 = 0;
pub const OP_SOD: u8 = 1;
pub const OP_SOM: u8 = 2;
pub const OP_SET_SOM: u8 = 3;
pub const OP_NOT_WORD_BOUNDARY: u8 = 4;
pub const OP_WORD_BOUNDARY: u8 = 5;
pub const OP_NOT_DIGIT: u8 = 6;
pub const OP_DIGIT: u8 = 7;
pub const OP_NOT_WHITESPACE: u8 = 8;
pub const OP_WHITESPACE: u8 = 9;
pub const OP_NOT_WORDCHAR: u8 = 10;
pub const OP_WORDCHAR: u8 = 11;
pub const OP_ANY: u8 = 12;
pub const OP_ALLANY: u8 = 13;
pub const OP_ANYBYTE: u8 = 14;
pub const OP_NOTPROP: u8 = 15;
pub const OP_PROP: u8 = 16;
pub const OP_ANYNL: u8 = 17;
pub const OP_NOT_HSPACE: u8 = 18;
pub const OP_HSPACE: u8 = 19;
pub const OP_NOT_VSPACE: u8 = 20;
pub const OP_VSPACE: u8 = 21;
pub const OP_EXTUNI: u8 = 22;
pub const OP_EODN: u8 = 23;
pub const OP_EOD: u8 = 24;
pub const OP_DOLL: u8 = 25;
pub const OP_DOLLM: u8 = 26;
pub const OP_CIRC: u8 = 27;
pub const OP_CIRCM: u8 = 28;
pub const OP_CHAR: u8 = 29;
pub const OP_CHARI: u8 = 30;
pub const OP_NOT: u8 = 31;
pub const OP_NOTI: u8 = 32;
pub const OP_STAR: u8 = 33;
pub const OP_MINSTAR: u8 = 34;
pub const OP_PLUS: u8 = 35;
pub const OP_MINPLUS: u8 = 36;
pub const OP_QUERY: u8 = 37;
pub const OP_MINQUERY: u8 = 38;
pub const OP_UPTO: u8 = 39;
pub const OP_MINUPTO: u8 = 40;
pub const OP_EXACT: u8 = 41;
pub const OP_POSSTAR: u8 = 42;
pub const OP_POSPLUS: u8 = 43;
pub const OP_POSQUERY: u8 = 44;
pub const OP_POSUPTO: u8 = 45;
pub const OP_STARI: u8 = 46;
pub const OP_MINSTARI: u8 = 47;
pub const OP_PLUSI: u8 = 48;
pub const OP_MINPLUSI: u8 = 49;
pub const OP_QUERYI: u8 = 50;
pub const OP_MINQUERYI: u8 = 51;
pub const OP_UPTOI: u8 = 52;
pub const OP_MINUPTOI: u8 = 53;
pub const OP_EXACTI: u8 = 54;
pub const OP_POSSTARI: u8 = 55;
pub const OP_POSPLUSI: u8 = 56;
pub const OP_POSQUERYI: u8 = 57;
pub const OP_POSUPTOI: u8 = 58;
pub const OP_NOTSTAR: u8 = 59;
pub const OP_NOTMINSTAR: u8 = 60;
pub const OP_NOTPLUS: u8 = 61;
pub const OP_NOTMINPLUS: u8 = 62;
pub const OP_NOTQUERY: u8 = 63;
pub const OP_NOTMINQUERY: u8 = 64;
pub const OP_NOTUPTO: u8 = 65;
pub const OP_NOTMINUPTO: u8 = 66;
pub const OP_NOTEXACT: u8 = 67;
pub const OP_NOTPOSSTAR: u8 = 68;
pub const OP_NOTPOSPLUS: u8 = 69;
pub const OP_NOTPOSQUERY: u8 = 70;
pub const OP_NOTPOSUPTO: u8 = 71;
pub const OP_NOTSTARI: u8 = 72;
pub const OP_NOTMINSTARI: u8 = 73;
pub const OP_NOTPLUSI: u8 = 74;
pub const OP_NOTMINPLUSI: u8 = 75;
pub const OP_NOTQUERYI: u8 = 76;
pub const OP_NOTMINQUERYI: u8 = 77;
pub const OP_NOTUPTOI: u8 = 78;
pub const OP_NOTMINUPTOI: u8 = 79;
pub const OP_NOTEXACTI: u8 = 80;
pub const OP_NOTPOSSTARI: u8 = 81;
pub const OP_NOTPOSPLUSI: u8 = 82;
pub const OP_NOTPOSQUERYI: u8 = 83;
pub const OP_NOTPOSUPTOI: u8 = 84;
pub const OP_TYPESTAR: u8 = 85;
pub const OP_TYPEMINSTAR: u8 = 86;
pub const OP_TYPEPLUS: u8 = 87;
pub const OP_TYPEMINPLUS: u8 = 88;
pub const OP_TYPEQUERY: u8 = 89;
pub const OP_TYPEMINQUERY: u8 = 90;
pub const OP_TYPEUPTO: u8 = 91;
pub const OP_TYPEMINUPTO: u8 = 92;
pub const OP_TYPEEXACT: u8 = 93;
pub const OP_TYPEPOSSTAR: u8 = 94;
pub const OP_TYPEPOSPLUS: u8 = 95;
pub const OP_TYPEPOSQUERY: u8 = 96;
pub const OP_TYPEPOSUPTO: u8 = 97;
pub const OP_CRSTAR: u8 = 98;
pub const OP_CRMINSTAR: u8 = 99;
pub const OP_CRPLUS: u8 = 100;
pub const OP_CRMINPLUS: u8 = 101;
pub const OP_CRQUERY: u8 = 102;
pub const OP_CRMINQUERY: u8 = 103;
pub const OP_CRRANGE: u8 = 104;
pub const OP_CRMINRANGE: u8 = 105;
pub const OP_CRPOSSTAR: u8 = 106;
pub const OP_CRPOSPLUS: u8 = 107;
pub const OP_CRPOSQUERY: u8 = 108;
pub const OP_CRPOSRANGE: u8 = 109;
pub const OP_CLASS: u8 = 110;
pub const OP_NCLASS: u8 = 111;
pub const OP_XCLASS: u8 = 112;
pub const OP_ECLASS: u8 = 113;
pub const OP_REF: u8 = 114;
pub const OP_REFI: u8 = 115;
pub const OP_DNREF: u8 = 116;
pub const OP_DNREFI: u8 = 117;
pub const OP_RECURSE: u8 = 118;
pub const OP_CALLOUT: u8 = 119;
pub const OP_CALLOUT_STR: u8 = 120;
pub const OP_ALT: u8 = 121;
pub const OP_KET: u8 = 122;
pub const OP_KETRMAX: u8 = 123;
pub const OP_KETRMIN: u8 = 124;
pub const OP_KETRPOS: u8 = 125;
pub const OP_REVERSE: u8 = 126;
pub const OP_VREVERSE: u8 = 127;
pub const OP_ASSERT: u8 = 128;
pub const OP_ASSERT_NOT: u8 = 129;
pub const OP_ASSERTBACK: u8 = 130;
pub const OP_ASSERTBACK_NOT: u8 = 131;
pub const OP_ASSERT_NA: u8 = 132;
pub const OP_ASSERTBACK_NA: u8 = 133;
pub const OP_ASSERT_SCS: u8 = 134;
pub const OP_ONCE: u8 = 135;
pub const OP_SCRIPT_RUN: u8 = 136;
pub const OP_BRA: u8 = 137;
pub const OP_BRAPOS: u8 = 138;
pub const OP_CBRA: u8 = 139;
pub const OP_CBRAPOS: u8 = 140;
pub const OP_COND: u8 = 141;
pub const OP_SBRA: u8 = 142;
pub const OP_SBRAPOS: u8 = 143;
pub const OP_SCBRA: u8 = 144;
pub const OP_SCBRAPOS: u8 = 145;
pub const OP_SCOND: u8 = 146;
pub const OP_CREF: u8 = 147;
pub const OP_DNCREF: u8 = 148;
pub const OP_RREF: u8 = 149;
pub const OP_DNRREF: u8 = 150;
pub const OP_FALSE: u8 = 151;
pub const OP_TRUE: u8 = 152;
pub const OP_BRAZERO: u8 = 153;
pub const OP_BRAMINZERO: u8 = 154;
pub const OP_BRAPOSZERO: u8 = 155;
pub const OP_MARK: u8 = 156;
pub const OP_PRUNE: u8 = 157;
pub const OP_PRUNE_ARG: u8 = 158;
pub const OP_SKIP: u8 = 159;
pub const OP_SKIP_ARG: u8 = 160;
pub const OP_THEN: u8 = 161;
pub const OP_THEN_ARG: u8 = 162;
pub const OP_COMMIT: u8 = 163;
pub const OP_COMMIT_ARG: u8 = 164;
pub const OP_FAIL: u8 = 165;
pub const OP_ACCEPT: u8 = 166;
pub const OP_ASSERT_ACCEPT: u8 = 167;
pub const OP_CLOSE: u8 = 168;
pub const OP_SKIPZERO: u8 = 169;
pub const OP_DEFINE: u8 = 170;
pub const OP_NOT_UCP_WORD_BOUNDARY: u8 = 171;
pub const OP_UCP_WORD_BOUNDARY: u8 = 172;
pub const OP_TABLE_LENGTH: usize = 173;

pub const FIRST_AUTOTAB_OP: u8 = OP_NOT_DIGIT;
pub const LAST_AUTOTAB_LEFT_OP: u8 = OP_EXTUNI;
pub const LAST_AUTOTAB_RIGHT_OP: u8 = OP_DOLLM;

// ---------- GET/PUT link macros (8-bit, LINK_SIZE=2) ----------
#[inline]
pub unsafe fn GET(a: *const u8, n: usize) -> u32 {
    ((*a.add(n) as u32) << 8) | (*a.add(n + 1) as u32)
}
#[inline]
pub unsafe fn PUT(a: *mut u8, n: usize, d: u32) {
    *a.add(n) = (d >> 8) as u8;
    *a.add(n + 1) = (d & 255) as u8;
}
#[inline]
pub unsafe fn GET2(a: *const u8, n: usize) -> u32 {
    ((*a.add(n) as u32) << 8) | (*a.add(n + 1) as u32)
}
#[inline]
pub unsafe fn PUT2(a: *mut u8, n: usize, d: u32) {
    *a.add(n) = (d >> 8) as u8;
    *a.add(n + 1) = (d & 255) as u8;
}
#[inline]
pub fn CU2BYTES(x: usize) -> usize { x }
#[inline]
pub fn BYTES2CU(x: usize) -> usize { x }

// ---------- extern data tables (defined in tables_data.rs) ----------
extern "C" {
    pub static _pcre2_OP_lengths_8: [u8; OP_TABLE_LENGTH];
    pub static _pcre2_hspace_list_8: [u32; 20];
    pub static _pcre2_vspace_list_8: [u32; 8];
    pub static _pcre2_callout_start_delims_8: [u32; 9];
    pub static _pcre2_callout_end_delims_8: [u32; 9];
    pub static _pcre2_default_tables_8: [u8; TABLES_LENGTH];
    pub static _pcre2_utf8_table1: [i32; 6];
    pub static _pcre2_utf8_table1_size: u32;
    pub static _pcre2_utf8_table2: [i32; 6];
    pub static _pcre2_utf8_table3: [i32; 6];
    pub static _pcre2_utf8_table4: [u8; 64];
    pub static _pcre2_ucp_gentype_8: [u32; 29];
    pub static _pcre2_ucp_gbtable_8: [u32; 15];
    pub static _pcre2_ucd_records_8: [UcdRecord; 1563];
    pub static _pcre2_ucd_stage1_8: [u16; 8704];
    pub static _pcre2_ucd_stage2_8: [u16; 40192];
    pub static _pcre2_ucd_caseless_sets_8: [u32; 118];
    pub static _pcre2_ucd_boolprop_sets_8: [u32; 382];
    pub static _pcre2_ucd_script_sets_8: [u32; 476];
    pub static _pcre2_ucd_digit_sets_8: [u32; 78];
    pub static _pcre2_ucd_nocase_ranges_8: [u32; 82];
    pub static _pcre2_ucd_nocase_ranges_size_8: u32;
    pub static _pcre2_ucd_turkish_dotted_i_caseset_8: u32;
    pub static _pcre2_posix_class_maps8: [i32; 42];
    pub static _pcre2_utt_8: [UcpTypeTable; 518];
    pub static _pcre2_utt_names_8: [u8; 3834];
    pub static _pcre2_utt_size_8: usize;
}

// ---------- UCD access helpers ----------
#[inline]
pub fn GET_UCD(ch: u32) -> &'static UcdRecord {
    REAL_GET_UCD(ch)
}
#[inline]
pub fn REAL_GET_UCD(ch: u32) -> &'static UcdRecord {
    unsafe {
        let stage1 = _pcre2_ucd_stage1_8[(ch as usize) / UCD_BLOCK_SIZE] as usize;
        let idx = _pcre2_ucd_stage2_8[stage1 * UCD_BLOCK_SIZE + (ch as usize) % UCD_BLOCK_SIZE] as usize;
        &_pcre2_ucd_records_8[idx]
    }
}
#[inline]
pub fn UCD_CHARTYPE(ch: u32) -> u32 { GET_UCD(ch).chartype as u32 }
#[inline]
pub fn UCD_SCRIPT(ch: u32) -> u32 { GET_UCD(ch).script as u32 }
#[inline]
pub fn UCD_CATEGORY(ch: u32) -> u32 { unsafe { _pcre2_ucp_gentype_8[UCD_CHARTYPE(ch) as usize] } }
#[inline]
pub fn UCD_GRAPHBREAK(ch: u32) -> u32 { GET_UCD(ch).gbprop as u32 }
#[inline]
pub fn UCD_CASESET(ch: u32) -> u32 { GET_UCD(ch).caseset as u32 }
#[inline]
pub fn UCD_OTHERCASE(ch: u32) -> u32 { (ch as i32 + GET_UCD(ch).other_case) as u32 }
#[inline]
pub fn UCD_SCRIPTX(ch: u32) -> u32 { (GET_UCD(ch).scriptx_bidiclass & UCD_SCRIPTX_MASK) as u32 }
#[inline]
pub fn UCD_BPROPS(ch: u32) -> u32 { (GET_UCD(ch).bprops & UCD_BPROPS_MASK) as u32 }
#[inline]
pub fn UCD_BIDICLASS(ch: u32) -> u32 { (GET_UCD(ch).scriptx_bidiclass >> UCD_BIDICLASS_SHIFT) as u32 }

// ---------- UTF-8 character macros (8-bit) ----------
pub const MAYBE_UTF_MULTI: bool = true;

#[inline]
pub fn HASUTF8EXTRALEN(c: u32) -> bool { c >= 0xc0 }
#[inline]
pub fn HAS_EXTRALEN(c: u32) -> bool { HASUTF8EXTRALEN(c) }
#[inline]
pub unsafe fn GET_EXTRALEN(c: u32) -> u32 { _pcre2_utf8_table4[(c & 0x3f) as usize] as u32 }
#[inline]
pub fn NOT_FIRSTCU(c: u32) -> bool { (c & 0xc0) == 0x80 }

/// GETUTF8: decode remaining bytes of a UTF-8 char at eptr[0..], not advancing.
#[inline]
pub unsafe fn getutf8(c: u32, eptr: PCRE2_SPTR) -> u32 {
    if (c & 0x20) == 0 {
        ((c & 0x1f) << 6) | (*eptr.add(1) as u32 & 0x3f)
    } else if (c & 0x10) == 0 {
        ((c & 0x0f) << 12) | ((*eptr.add(1) as u32 & 0x3f) << 6) | (*eptr.add(2) as u32 & 0x3f)
    } else if (c & 0x08) == 0 {
        ((c & 0x07) << 18)
            | ((*eptr.add(1) as u32 & 0x3f) << 12)
            | ((*eptr.add(2) as u32 & 0x3f) << 6)
            | (*eptr.add(3) as u32 & 0x3f)
    } else if (c & 0x04) == 0 {
        ((c & 0x03) << 24)
            | ((*eptr.add(1) as u32 & 0x3f) << 18)
            | ((*eptr.add(2) as u32 & 0x3f) << 12)
            | ((*eptr.add(3) as u32 & 0x3f) << 6)
            | (*eptr.add(4) as u32 & 0x3f)
    } else {
        ((c & 0x01) << 30)
            | ((*eptr.add(1) as u32 & 0x3f) << 24)
            | ((*eptr.add(2) as u32 & 0x3f) << 18)
            | ((*eptr.add(3) as u32 & 0x3f) << 12)
            | ((*eptr.add(4) as u32 & 0x3f) << 6)
            | (*eptr.add(5) as u32 & 0x3f)
    }
}

/// GETCHAR: decode full char at eptr, not advancing (UTF known).
#[inline]
pub unsafe fn GETCHAR(eptr: PCRE2_SPTR) -> u32 {
    let c = *eptr as u32;
    if c >= 0xc0 { getutf8(c, eptr) } else { c }
}

/// GETCHARLEN: decode char at eptr; return (char, extra_len_added).
/// Mirrors C's `len++`-style extra length accumulation.
#[inline]
pub unsafe fn GETCHARLEN(eptr: PCRE2_SPTR) -> (u32, u32) {
    let c = *eptr as u32;
    if c < 0xc0 {
        return (c, 0);
    }
    let extra = if (c & 0x20) == 0 {
        1
    } else if (c & 0x10) == 0 {
        2
    } else if (c & 0x08) == 0 {
        3
    } else if (c & 0x04) == 0 {
        4
    } else {
        5
    };
    (getutf8(c, eptr), extra)
}

/// GETCHARINC: decode char and return (char, code_units_consumed).
#[inline]
pub unsafe fn GETCHARINC(eptr: PCRE2_SPTR) -> (u32, usize) {
    let c = *eptr as u32;
    if c < 0xc0 {
        return (c, 1);
    }
    let (val, extra) = GETCHARLEN(eptr);
    (val, 1 + extra as usize)
}

/// BACKCHAR: move ptr back over UTF-8 continuation bytes.
#[inline]
pub unsafe fn BACKCHAR(mut eptr: PCRE2_SPTR) -> PCRE2_SPTR {
    while (*eptr & 0xc0) == 0x80 {
        eptr = eptr.sub(1);
    }
    eptr
}

// ---------- libc bindings ----------
extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memset(dst: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn strlen(s: *const c_char) -> usize;
    pub fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
}


// ---------- heapframe (match backtracking frame) ----------
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HfCharRepeat {
    pub start_eptr: PCRE2_SPTR,
    pub charptr: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
    pub c: u32,
    // union { uint32_t oc; PCRE2_UCHAR occu[4]; } -> size 4, align 4
    pub oc: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HfCharnotRepeat {
    pub start_eptr: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
    pub c: u32,
    pub oc: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HfClassRepeat {
    pub start_eptr: PCRE2_SPTR,
    pub byte_map_address: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HfXclassRepeat {
    pub start_eptr: PCRE2_SPTR,
    pub xclass_data: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HfEclassRepeat {
    pub start_eptr: PCRE2_SPTR,
    pub eclass_data: PCRE2_SPTR,
    pub eclass_len: PCRE2_SIZE,
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HfTypeRepeat {
    pub start_eptr: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
    pub ctype: u32,
    pub propvalue: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HfRefRepeat {
    pub start: PCRE2_SPTR,
    pub offset: PCRE2_SIZE,
    pub length: PCRE2_SIZE,
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HfOpBra {
    pub frame_type: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HfOpBrapos {
    pub start_eptr: PCRE2_SPTR,
    pub start_group: PCRE2_SPTR,
    pub frame_type: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HfOpRecurse {
    pub start_branch: PCRE2_SPTR,
    pub frame_type: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HfOpAssertScs {
    pub saved_end_subject: PCRE2_SPTR,
    pub saved_eptr: PCRE2_SPTR,
    pub true_end_extra: PCRE2_SIZE,
    pub saved_moptions: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HfOpCond {
    pub start_branch: PCRE2_SPTR,
    pub length: PCRE2_SIZE,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HfOpVreverse {
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
pub union HeapframeFields {
    pub char_repeat: HfCharRepeat,
    pub charnot_repeat: HfCharnotRepeat,
    pub class_repeat: HfClassRepeat,
    pub xclass_repeat: HfXclassRepeat,
    pub eclass_repeat: HfEclassRepeat,
    pub type_repeat: HfTypeRepeat,
    pub ref_repeat: HfRefRepeat,
    pub op_bra: HfOpBra,
    pub op_brapos: HfOpBrapos,
    pub op_recurse: HfOpRecurse,
    pub op_assert_scs: HfOpAssertScs,
    pub op_cond: HfOpCond,
    pub op_vreverse: HfOpVreverse,
}

#[repr(C)]
pub struct heapframe {
    pub ecode: PCRE2_SPTR,
    pub back_frame: PCRE2_SIZE,
    pub rdepth: u32,
    pub group_frame_type: u32,
    pub return_id: u8,
    pub op: u8,
    pub byte1: u8,
    pub byte2: u8,
    pub fields: HeapframeFields,
    pub eptr: PCRE2_SPTR,
    pub start_match: PCRE2_SPTR,
    pub mark: PCRE2_SPTR,
    pub recurse_last_used: PCRE2_SPTR,
    pub current_recurse: u32,
    pub capture_last: u32,
    pub last_group_offset: PCRE2_SIZE,
    pub offset_top: PCRE2_SIZE,
    pub ovector: [PCRE2_SIZE; 131072],
}

// ---------- pcre2_compile.h shared definitions ----------
pub const META_END: u32 = 0x80000000;
pub const META_ALT: u32 = 0x80010000;
pub const META_ATOMIC: u32 = 0x80020000;
pub const META_BACKREF: u32 = 0x80030000;
pub const META_BACKREF_BYNAME: u32 = 0x80040000;
pub const META_BIGVALUE: u32 = 0x80050000;
pub const META_CALLOUT_NUMBER: u32 = 0x80060000;
pub const META_CALLOUT_STRING: u32 = 0x80070000;
pub const META_CAPTURE: u32 = 0x80080000;
pub const META_CIRCUMFLEX: u32 = 0x80090000;
pub const META_CLASS: u32 = 0x800a0000;
pub const META_CLASS_EMPTY: u32 = 0x800b0000;
pub const META_CLASS_EMPTY_NOT: u32 = 0x800c0000;
pub const META_CLASS_END: u32 = 0x800d0000;
pub const META_CLASS_NOT: u32 = 0x800e0000;
pub const META_COND_ASSERT: u32 = 0x800f0000;
pub const META_COND_DEFINE: u32 = 0x80100000;
pub const META_COND_NAME: u32 = 0x80110000;
pub const META_COND_NUMBER: u32 = 0x80120000;
pub const META_COND_RNAME: u32 = 0x80130000;
pub const META_COND_RNUMBER: u32 = 0x80140000;
pub const META_COND_VERSION: u32 = 0x80150000;
pub const META_OFFSET: u32 = 0x80160000;
pub const META_SCS: u32 = 0x80170000;
pub const META_CAPTURE_NAME: u32 = 0x80180000;
pub const META_CAPTURE_NUMBER: u32 = 0x80190000;
pub const META_DOLLAR: u32 = 0x801a0000;
pub const META_DOT: u32 = 0x801b0000;
pub const META_ESCAPE: u32 = 0x801c0000;
pub const META_KET: u32 = 0x801d0000;
pub const META_NOCAPTURE: u32 = 0x801e0000;
pub const META_OPTIONS: u32 = 0x801f0000;
pub const META_POSIX: u32 = 0x80200000;
pub const META_POSIX_NEG: u32 = 0x80210000;
pub const META_RANGE_ESCAPED: u32 = 0x80220000;
pub const META_RANGE_LITERAL: u32 = 0x80230000;
pub const META_RECURSE: u32 = 0x80240000;
pub const META_RECURSE_BYNAME: u32 = 0x80250000;
pub const META_SCRIPT_RUN: u32 = 0x80260000;
pub const META_LOOKAHEAD: u32 = 0x80270000;
pub const META_LOOKAHEADNOT: u32 = 0x80280000;
pub const META_LOOKBEHIND: u32 = 0x80290000;
pub const META_LOOKBEHINDNOT: u32 = 0x802a0000;
pub const META_LOOKAHEAD_NA: u32 = 0x802b0000;
pub const META_LOOKBEHIND_NA: u32 = 0x802c0000;
pub const META_MARK: u32 = 0x802d0000;
pub const META_ACCEPT: u32 = 0x802e0000;
pub const META_FAIL: u32 = 0x802f0000;
pub const META_COMMIT: u32 = 0x80300000;
pub const META_COMMIT_ARG: u32 = 0x80310000;
pub const META_PRUNE: u32 = 0x80320000;
pub const META_PRUNE_ARG: u32 = 0x80330000;
pub const META_SKIP: u32 = 0x80340000;
pub const META_SKIP_ARG: u32 = 0x80350000;
pub const META_THEN: u32 = 0x80360000;
pub const META_THEN_ARG: u32 = 0x80370000;
pub const META_ASTERISK: u32 = 0x80380000;
pub const META_ASTERISK_PLUS: u32 = 0x80390000;
pub const META_ASTERISK_QUERY: u32 = 0x803a0000;
pub const META_PLUS: u32 = 0x803b0000;
pub const META_PLUS_PLUS: u32 = 0x803c0000;
pub const META_PLUS_QUERY: u32 = 0x803d0000;
pub const META_QUERY: u32 = 0x803e0000;
pub const META_QUERY_PLUS: u32 = 0x803f0000;
pub const META_QUERY_QUERY: u32 = 0x80400000;
pub const META_MINMAX: u32 = 0x80410000;
pub const META_MINMAX_PLUS: u32 = 0x80420000;
pub const META_MINMAX_QUERY: u32 = 0x80430000;
pub const META_ECLASS_AND: u32 = 0x80440000;
pub const META_ECLASS_OR: u32 = 0x80450000;
pub const META_ECLASS_SUB: u32 = 0x80460000;
pub const META_ECLASS_XOR: u32 = 0x80470000;
pub const META_ECLASS_NOT: u32 = 0x80480000;
pub const META_ATOMIC_SCRIPT_RUN: u32 = 0x8fff0000;
pub const META_FIRST_QUANTIFIER: u32 = META_ASTERISK;
pub const META_LAST_QUANTIFIER: u32 = META_MINMAX_QUERY;
#[inline] pub fn META_CODE(x: u32) -> u32 { x & 0xffff0000 }
#[inline] pub fn META_DATA(x: u32) -> u32 { x & 0x0000ffff }
#[inline] pub fn META_DIFF(x: u32, y: u32) -> u32 { (x - y) >> 16 }
pub const SIZEOFFSET: usize = 1;
pub const CLASS_IS_ECLASS: u32 = 0x1;
pub const MAX_UCHAR_VALUE: u32 = 0xff;
#[inline] pub fn GET_MAX_CHAR_VALUE(utf: bool) -> u32 { if utf { MAX_UTF_CODE_POINT } else { MAX_UCHAR_VALUE } }
pub const PC_DIGIT: usize = 7;
pub const PC_GRAPH: usize = 8;
pub const PC_PRINT: usize = 9;
pub const PC_PUNCT: usize = 10;
pub const PC_XDIGIT: usize = 13;
pub const NAMED_GROUP_HASH_MASK: u16 = 0x7fff;
pub const NAMED_GROUP_IS_DUPNAME: u16 = 0x8000;
// Compile-time error codes ERR0..ERR120 (ERR0=COMPILE_ERROR_BASE=100)
pub const ERR0: c_int = 100;
pub const ERR1: c_int = 101;
pub const ERR2: c_int = 102;
pub const ERR3: c_int = 103;
pub const ERR4: c_int = 104;
pub const ERR5: c_int = 105;
pub const ERR6: c_int = 106;
pub const ERR7: c_int = 107;
pub const ERR8: c_int = 108;
pub const ERR9: c_int = 109;
pub const ERR10: c_int = 110;
pub const ERR11: c_int = 111;
pub const ERR12: c_int = 112;
pub const ERR13: c_int = 113;
pub const ERR14: c_int = 114;
pub const ERR15: c_int = 115;
pub const ERR16: c_int = 116;
pub const ERR17: c_int = 117;
pub const ERR18: c_int = 118;
pub const ERR19: c_int = 119;
pub const ERR20: c_int = 120;
pub const ERR21: c_int = 121;
pub const ERR22: c_int = 122;
pub const ERR23: c_int = 123;
pub const ERR24: c_int = 124;
pub const ERR25: c_int = 125;
pub const ERR26: c_int = 126;
pub const ERR27: c_int = 127;
pub const ERR28: c_int = 128;
pub const ERR29: c_int = 129;
pub const ERR30: c_int = 130;
pub const ERR31: c_int = 131;
pub const ERR32: c_int = 132;
pub const ERR33: c_int = 133;
pub const ERR34: c_int = 134;
pub const ERR35: c_int = 135;
pub const ERR36: c_int = 136;
pub const ERR37: c_int = 137;
pub const ERR38: c_int = 138;
pub const ERR39: c_int = 139;
pub const ERR40: c_int = 140;
pub const ERR41: c_int = 141;
pub const ERR42: c_int = 142;
pub const ERR43: c_int = 143;
pub const ERR44: c_int = 144;
pub const ERR45: c_int = 145;
pub const ERR46: c_int = 146;
pub const ERR47: c_int = 147;
pub const ERR48: c_int = 148;
pub const ERR49: c_int = 149;
pub const ERR50: c_int = 150;
pub const ERR51: c_int = 151;
pub const ERR52: c_int = 152;
pub const ERR53: c_int = 153;
pub const ERR54: c_int = 154;
pub const ERR55: c_int = 155;
pub const ERR56: c_int = 156;
pub const ERR57: c_int = 157;
pub const ERR58: c_int = 158;
pub const ERR59: c_int = 159;
pub const ERR60: c_int = 160;
pub const ERR61: c_int = 161;
pub const ERR62: c_int = 162;
pub const ERR63: c_int = 163;
pub const ERR64: c_int = 164;
pub const ERR65: c_int = 165;
pub const ERR66: c_int = 166;
pub const ERR67: c_int = 167;
pub const ERR68: c_int = 168;
pub const ERR69: c_int = 169;
pub const ERR70: c_int = 170;
pub const ERR71: c_int = 171;
pub const ERR72: c_int = 172;
pub const ERR73: c_int = 173;
pub const ERR74: c_int = 174;
pub const ERR75: c_int = 175;
pub const ERR76: c_int = 176;
pub const ERR77: c_int = 177;
pub const ERR78: c_int = 178;
pub const ERR79: c_int = 179;
pub const ERR80: c_int = 180;
pub const ERR81: c_int = 181;
pub const ERR82: c_int = 182;
pub const ERR83: c_int = 183;
pub const ERR84: c_int = 184;
pub const ERR85: c_int = 185;
pub const ERR86: c_int = 186;
pub const ERR87: c_int = 187;
pub const ERR88: c_int = 188;
pub const ERR89: c_int = 189;
pub const ERR90: c_int = 190;
pub const ERR91: c_int = 191;
pub const ERR92: c_int = 192;
pub const ERR93: c_int = 193;
pub const ERR94: c_int = 194;
pub const ERR95: c_int = 195;
pub const ERR96: c_int = 196;
pub const ERR97: c_int = 197;
pub const ERR98: c_int = 198;
pub const ERR99: c_int = 199;
pub const ERR100: c_int = 200;
pub const ERR101: c_int = 201;
pub const ERR102: c_int = 202;
pub const ERR103: c_int = 203;
pub const ERR104: c_int = 204;
pub const ERR105: c_int = 205;
pub const ERR106: c_int = 206;
pub const ERR107: c_int = 207;
pub const ERR108: c_int = 208;
pub const ERR109: c_int = 209;
pub const ERR110: c_int = 210;
pub const ERR111: c_int = 211;
pub const ERR112: c_int = 212;
pub const ERR113: c_int = 213;
pub const ERR114: c_int = 214;
pub const ERR115: c_int = 215;
pub const ERR116: c_int = 216;
pub const ERR117: c_int = 217;
pub const ERR118: c_int = 218;
pub const ERR119: c_int = 219;
pub const ERR120: c_int = 220;

// eclass_op_info (pcre2_compile.h)
#[repr(C)]
pub struct eclass_op_info {
    pub code_start: *mut PCRE2_UCHAR,
    pub length: PCRE2_SIZE,
    pub op_single_type: u8,
    pub bits: class_bits_storage,
}

#[inline]
pub unsafe fn SETBIT(a: *mut u8, b: u32) {
    *a.add((b >> 3) as usize) |= 1u8 << (b & 0x7);
}

// ---------- Match blocks (pcre2_intmodedep.h) ----------
#[repr(C)]
pub struct dfa_recursion_info {
    pub prevrec: *mut dfa_recursion_info,
    pub subject_position: PCRE2_SPTR,
    pub last_used_ptr: PCRE2_SPTR,
    pub group_num: u32,
}

#[repr(C)]
pub struct match_block {
    pub memctl: pcre2_memctl,
    pub heap_limit: u32,
    pub match_limit: u32,
    pub match_limit_depth: u32,
    pub match_call_count: u32,
    pub hitend: BOOL,
    pub hasthen: BOOL,
    pub hasbsk: BOOL,
    pub allowemptypartial: BOOL,
    pub allowlookaroundbsk: BOOL,
    pub lcc: *const u8,
    pub fcc: *const u8,
    pub ctypes: *const u8,
    pub start_offset: PCRE2_SIZE,
    pub end_offset_top: PCRE2_SIZE,
    pub partial: u16,
    pub bsr_convention: u16,
    pub name_count: u16,
    pub name_entry_size: u16,
    pub name_table: PCRE2_SPTR,
    pub start_code: PCRE2_SPTR,
    pub start_subject: PCRE2_SPTR,
    pub check_subject: PCRE2_SPTR,
    pub end_subject: PCRE2_SPTR,
    pub true_end_subject: PCRE2_SPTR,
    pub end_match_ptr: PCRE2_SPTR,
    pub start_used_ptr: PCRE2_SPTR,
    pub last_used_ptr: PCRE2_SPTR,
    pub mark: PCRE2_SPTR,
    pub nomatch_mark: PCRE2_SPTR,
    pub verb_ecode_ptr: PCRE2_SPTR,
    pub verb_skip_ptr: PCRE2_SPTR,
    pub verb_current_recurse: u32,
    pub moptions: u32,
    pub poptions: u32,
    pub skip_arg_count: u32,
    pub ignore_skip_arg: u32,
    pub nltype: u32,
    pub nllen: u32,
    pub nl: [PCRE2_UCHAR; 4],
    pub cb: *mut pcre2_callout_block,
    pub callout_data: *mut c_void,
    pub callout: CalloutFn,
}

#[repr(C)]
pub struct dfa_match_block {
    pub memctl: pcre2_memctl,
    pub start_code: PCRE2_SPTR,
    pub start_subject: PCRE2_SPTR,
    pub end_subject: PCRE2_SPTR,
    pub start_used_ptr: PCRE2_SPTR,
    pub last_used_ptr: PCRE2_SPTR,
    pub tables: *const u8,
    pub start_offset: PCRE2_SIZE,
    pub heap_limit: u32,
    pub heap_used: PCRE2_SIZE,
    pub match_limit: u32,
    pub match_limit_depth: u32,
    pub match_call_count: u32,
    pub moptions: u32,
    pub poptions: u32,
    pub nltype: u32,
    pub nllen: u32,
    pub allowemptypartial: BOOL,
    pub nl: [PCRE2_UCHAR; 4],
    pub bsr_convention: u16,
    pub cb: *mut pcre2_callout_block,
    pub callout_data: *mut c_void,
    pub callout: CalloutFn,
    pub recursive: *mut dfa_recursion_info,
}
