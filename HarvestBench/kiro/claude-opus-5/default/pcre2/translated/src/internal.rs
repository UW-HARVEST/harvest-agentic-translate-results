//! Translation of the mode-independent and 8-bit mode-dependent definitions in
//! `pcre2_internal.h`, `pcre2_intmodedep.h` and the public `pcre2.h`.
//!
//! Build configuration mirrored from `c_src/CMakeLists.txt` + `c_src/src/config.h`:
//! `PCRE2_CODE_UNIT_WIDTH == 8`, `SUPPORT_UNICODE`, no `SUPPORT_JIT`, no `EBCDIC`,
//! `LINK_SIZE == 2`.

#![allow(dead_code, non_upper_case_globals, non_camel_case_types)]

use core::ffi::{c_char, c_int, c_void};

/* ------------------------- Basic types ------------------------- */

pub type PCRE2_SIZE = usize;
pub type PCRE2_UCHAR = u8;
pub type PCRE2_SPTR = *const u8;
pub type BOOL = c_int;

pub const TRUE: BOOL = 1;
pub const FALSE: BOOL = 0;

pub const PCRE2_CODE_UNIT_WIDTH: u32 = 8;
pub const LINK_SIZE: usize = 2;
pub const IMM2_SIZE: usize = 2;
pub const MAX_PATTERN_SIZE: usize = 1 << 16;

pub const PCRE2_SIZE_MAX: PCRE2_SIZE = usize::MAX;
pub const PCRE2_ZERO_TERMINATED: PCRE2_SIZE = !0;
pub const PCRE2_UNSET: PCRE2_SIZE = !0;

pub const PCRE2_MAJOR: u32 = 10;
pub const PCRE2_MINOR: u32 = 48;

/* Configured limits, from config.h */

pub const HEAP_LIMIT: u32 = 20000000;
pub const MATCH_LIMIT: u32 = 10000000;
pub const MATCH_LIMIT_DEPTH: u32 = MATCH_LIMIT;
pub const MAX_VARLOOKBEHIND: u32 = 255;
pub const NEWLINE_DEFAULT: u32 = 2;
pub const PARENS_NEST_LIMIT: u32 = 250;

pub const NOTACHAR: u32 = 0xffffffff;
pub const MAX_UTF_CODE_POINT: u32 = 0x10ffff;
pub const MAX_NON_UTF_CHAR: u32 = 0xffffffff >> (32 - PCRE2_CODE_UNIT_WIDTH);
pub const COMPILE_ERROR_BASE: c_int = 100;
pub const START_FRAMES_SIZE: usize = 20480;
pub const DFA_START_RWS_SIZE: usize = 30720;
pub const BSR_DEFAULT: u32 = PCRE2_BSR_UNICODE;
pub const REQ_CU_MAX: usize = 5000;
pub const ECLASS_NEST_LIMIT: usize = 15;
pub const MAGIC_NUMBER: u32 = 0x50435245;
pub const RREF_ANY: u32 = 0xffff;
pub const MAX_UTF_SINGLE_CU: u32 = 127;
pub const MAX_MARK: u32 = (1 << 8) - 1;
pub const LOOKBEHIND_MAX: c_int = u16::MAX as c_int;
pub const UCD_BLOCK_SIZE: usize = 128;

pub const REFI_FLAG_CASELESS_RESTRICT: u32 = 0x1;
pub const REFI_FLAG_TURKISH_CASING: u32 = 0x2;

/* ------------------------- Public option bits ------------------------- */

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

pub const PCRE2_JIT_COMPLETE: u32 = 0x00000001;
pub const PCRE2_JIT_PARTIAL_SOFT: u32 = 0x00000002;
pub const PCRE2_JIT_PARTIAL_HARD: u32 = 0x00000004;
pub const PCRE2_JIT_INVALID_UTF: u32 = 0x00000100;
pub const PCRE2_JIT_TEST_ALLOC: u32 = 0x00000200;

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

pub const PCRE2_CONVERT_UTF: u32 = 0x00000001;
pub const PCRE2_CONVERT_NO_UTF_CHECK: u32 = 0x00000002;
pub const PCRE2_CONVERT_POSIX_BASIC: u32 = 0x00000004;
pub const PCRE2_CONVERT_POSIX_EXTENDED: u32 = 0x00000008;
pub const PCRE2_CONVERT_GLOB: u32 = 0x00000010;
pub const PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR: u32 = 0x00000030;
pub const PCRE2_CONVERT_GLOB_NO_STARSTAR: u32 = 0x00000050;

pub const PCRE2_NEWLINE_CR: u32 = 1;
pub const PCRE2_NEWLINE_LF: u32 = 2;
pub const PCRE2_NEWLINE_CRLF: u32 = 3;
pub const PCRE2_NEWLINE_ANY: u32 = 4;
pub const PCRE2_NEWLINE_ANYCRLF: u32 = 5;
pub const PCRE2_NEWLINE_NUL: u32 = 6;

pub const PCRE2_BSR_UNICODE: u32 = 1;
pub const PCRE2_BSR_ANYCRLF: u32 = 2;

/* ------------------------- Error codes ------------------------- */

pub const PCRE2_ERROR_END_BACKSLASH: c_int = 101;
pub const PCRE2_ERROR_END_BACKSLASH_C: c_int = 102;
pub const PCRE2_ERROR_UNKNOWN_ESCAPE: c_int = 103;
pub const PCRE2_ERROR_QUANTIFIER_OUT_OF_ORDER: c_int = 104;
pub const PCRE2_ERROR_QUANTIFIER_TOO_BIG: c_int = 105;
pub const PCRE2_ERROR_MISSING_SQUARE_BRACKET: c_int = 106;
pub const PCRE2_ERROR_ESCAPE_INVALID_IN_CLASS: c_int = 107;
pub const PCRE2_ERROR_CLASS_RANGE_ORDER: c_int = 108;
pub const PCRE2_ERROR_QUANTIFIER_INVALID: c_int = 109;
pub const PCRE2_ERROR_INTERNAL_UNEXPECTED_REPEAT: c_int = 110;
pub const PCRE2_ERROR_INVALID_AFTER_PARENS_QUERY: c_int = 111;
pub const PCRE2_ERROR_POSIX_CLASS_NOT_IN_CLASS: c_int = 112;
pub const PCRE2_ERROR_POSIX_NO_SUPPORT_COLLATING: c_int = 113;
pub const PCRE2_ERROR_MISSING_CLOSING_PARENTHESIS: c_int = 114;
pub const PCRE2_ERROR_BAD_SUBPATTERN_REFERENCE: c_int = 115;
pub const PCRE2_ERROR_NULL_PATTERN: c_int = 116;
pub const PCRE2_ERROR_BAD_OPTIONS: c_int = 117;
pub const PCRE2_ERROR_MISSING_COMMENT_CLOSING: c_int = 118;
pub const PCRE2_ERROR_PARENTHESES_NEST_TOO_DEEP: c_int = 119;
pub const PCRE2_ERROR_PATTERN_TOO_LARGE: c_int = 120;
pub const PCRE2_ERROR_HEAP_FAILED: c_int = 121;
pub const PCRE2_ERROR_UNMATCHED_CLOSING_PARENTHESIS: c_int = 122;
pub const PCRE2_ERROR_INTERNAL_CODE_OVERFLOW: c_int = 123;
pub const PCRE2_ERROR_MISSING_CONDITION_CLOSING: c_int = 124;
pub const PCRE2_ERROR_LOOKBEHIND_NOT_FIXED_LENGTH: c_int = 125;
pub const PCRE2_ERROR_ZERO_RELATIVE_REFERENCE: c_int = 126;
pub const PCRE2_ERROR_TOO_MANY_CONDITION_BRANCHES: c_int = 127;
pub const PCRE2_ERROR_CONDITION_ASSERTION_EXPECTED: c_int = 128;
pub const PCRE2_ERROR_BAD_RELATIVE_REFERENCE: c_int = 129;
pub const PCRE2_ERROR_UNKNOWN_POSIX_CLASS: c_int = 130;
pub const PCRE2_ERROR_INTERNAL_STUDY_ERROR: c_int = 131;
pub const PCRE2_ERROR_UNICODE_NOT_SUPPORTED: c_int = 132;
pub const PCRE2_ERROR_PARENTHESES_STACK_CHECK: c_int = 133;
pub const PCRE2_ERROR_CODE_POINT_TOO_BIG: c_int = 134;
pub const PCRE2_ERROR_LOOKBEHIND_TOO_COMPLICATED: c_int = 135;
pub const PCRE2_ERROR_LOOKBEHIND_INVALID_BACKSLASH_C: c_int = 136;
pub const PCRE2_ERROR_UNSUPPORTED_ESCAPE_SEQUENCE: c_int = 137;
pub const PCRE2_ERROR_CALLOUT_NUMBER_TOO_BIG: c_int = 138;
pub const PCRE2_ERROR_MISSING_CALLOUT_CLOSING: c_int = 139;
pub const PCRE2_ERROR_ESCAPE_INVALID_IN_VERB: c_int = 140;
pub const PCRE2_ERROR_UNRECOGNIZED_AFTER_QUERY_P: c_int = 141;
pub const PCRE2_ERROR_MISSING_NAME_TERMINATOR: c_int = 142;
pub const PCRE2_ERROR_DUPLICATE_SUBPATTERN_NAME: c_int = 143;
pub const PCRE2_ERROR_INVALID_SUBPATTERN_NAME: c_int = 144;
pub const PCRE2_ERROR_UNICODE_PROPERTIES_UNAVAILABLE: c_int = 145;
pub const PCRE2_ERROR_MALFORMED_UNICODE_PROPERTY: c_int = 146;
pub const PCRE2_ERROR_UNKNOWN_UNICODE_PROPERTY: c_int = 147;
pub const PCRE2_ERROR_SUBPATTERN_NAME_TOO_LONG: c_int = 148;
pub const PCRE2_ERROR_TOO_MANY_NAMED_SUBPATTERNS: c_int = 149;
pub const PCRE2_ERROR_CLASS_INVALID_RANGE: c_int = 150;
pub const PCRE2_ERROR_OCTAL_BYTE_TOO_BIG: c_int = 151;
pub const PCRE2_ERROR_INTERNAL_OVERRAN_WORKSPACE: c_int = 152;
pub const PCRE2_ERROR_INTERNAL_MISSING_SUBPATTERN: c_int = 153;
pub const PCRE2_ERROR_DEFINE_TOO_MANY_BRANCHES: c_int = 154;
pub const PCRE2_ERROR_BACKSLASH_O_MISSING_BRACE: c_int = 155;
pub const PCRE2_ERROR_INTERNAL_UNKNOWN_NEWLINE: c_int = 156;
pub const PCRE2_ERROR_BACKSLASH_G_SYNTAX: c_int = 157;
pub const PCRE2_ERROR_PARENS_QUERY_R_MISSING_CLOSING: c_int = 158;
pub const PCRE2_ERROR_VERB_ARGUMENT_NOT_ALLOWED: c_int = 159;
pub const PCRE2_ERROR_VERB_UNKNOWN: c_int = 160;
pub const PCRE2_ERROR_SUBPATTERN_NUMBER_TOO_BIG: c_int = 161;
pub const PCRE2_ERROR_SUBPATTERN_NAME_EXPECTED: c_int = 162;
pub const PCRE2_ERROR_INTERNAL_PARSED_OVERFLOW: c_int = 163;
pub const PCRE2_ERROR_INVALID_OCTAL: c_int = 164;
pub const PCRE2_ERROR_SUBPATTERN_NAMES_MISMATCH: c_int = 165;
pub const PCRE2_ERROR_MARK_MISSING_ARGUMENT: c_int = 166;
pub const PCRE2_ERROR_INVALID_HEXADECIMAL: c_int = 167;
pub const PCRE2_ERROR_BACKSLASH_C_SYNTAX: c_int = 168;
pub const PCRE2_ERROR_BACKSLASH_K_SYNTAX: c_int = 169;
pub const PCRE2_ERROR_INTERNAL_BAD_CODE_LOOKBEHINDS: c_int = 170;
pub const PCRE2_ERROR_BACKSLASH_N_IN_CLASS: c_int = 171;
pub const PCRE2_ERROR_CALLOUT_STRING_TOO_LONG: c_int = 172;
pub const PCRE2_ERROR_UNICODE_DISALLOWED_CODE_POINT: c_int = 173;
pub const PCRE2_ERROR_UTF_IS_DISABLED: c_int = 174;
pub const PCRE2_ERROR_UCP_IS_DISABLED: c_int = 175;
pub const PCRE2_ERROR_VERB_NAME_TOO_LONG: c_int = 176;
pub const PCRE2_ERROR_BACKSLASH_U_CODE_POINT_TOO_BIG: c_int = 177;
pub const PCRE2_ERROR_MISSING_OCTAL_OR_HEX_DIGITS: c_int = 178;
pub const PCRE2_ERROR_VERSION_CONDITION_SYNTAX: c_int = 179;
pub const PCRE2_ERROR_INTERNAL_BAD_CODE_AUTO_POSSESS: c_int = 180;
pub const PCRE2_ERROR_CALLOUT_NO_STRING_DELIMITER: c_int = 181;
pub const PCRE2_ERROR_CALLOUT_BAD_STRING_DELIMITER: c_int = 182;
pub const PCRE2_ERROR_BACKSLASH_C_CALLER_DISABLED: c_int = 183;
pub const PCRE2_ERROR_QUERY_BARJX_NEST_TOO_DEEP: c_int = 184;
pub const PCRE2_ERROR_BACKSLASH_C_LIBRARY_DISABLED: c_int = 185;
pub const PCRE2_ERROR_PATTERN_TOO_COMPLICATED: c_int = 186;
pub const PCRE2_ERROR_LOOKBEHIND_TOO_LONG: c_int = 187;
pub const PCRE2_ERROR_PATTERN_STRING_TOO_LONG: c_int = 188;
pub const PCRE2_ERROR_INTERNAL_BAD_CODE: c_int = 189;
pub const PCRE2_ERROR_INTERNAL_BAD_CODE_IN_SKIP: c_int = 190;
pub const PCRE2_ERROR_NO_SURROGATES_IN_UTF16: c_int = 191;
pub const PCRE2_ERROR_BAD_LITERAL_OPTIONS: c_int = 192;
pub const PCRE2_ERROR_SUPPORTED_ONLY_IN_UNICODE: c_int = 193;
pub const PCRE2_ERROR_INVALID_HYPHEN_IN_OPTIONS: c_int = 194;
pub const PCRE2_ERROR_ALPHA_ASSERTION_UNKNOWN: c_int = 195;
pub const PCRE2_ERROR_SCRIPT_RUN_NOT_AVAILABLE: c_int = 196;
pub const PCRE2_ERROR_TOO_MANY_CAPTURES: c_int = 197;
pub const PCRE2_ERROR_MISSING_OCTAL_DIGIT: c_int = 198;
pub const PCRE2_ERROR_BACKSLASH_K_IN_LOOKAROUND: c_int = 199;
pub const PCRE2_ERROR_MAX_VAR_LOOKBEHIND_EXCEEDED: c_int = 200;
pub const PCRE2_ERROR_PATTERN_COMPILED_SIZE_TOO_BIG: c_int = 201;
pub const PCRE2_ERROR_OVERSIZE_PYTHON_OCTAL: c_int = 202;
pub const PCRE2_ERROR_CALLOUT_CALLER_DISABLED: c_int = 203;
pub const PCRE2_ERROR_EXTRA_CASING_REQUIRES_UNICODE: c_int = 204;
pub const PCRE2_ERROR_TURKISH_CASING_REQUIRES_UTF: c_int = 205;
pub const PCRE2_ERROR_EXTRA_CASING_INCOMPATIBLE: c_int = 206;
pub const PCRE2_ERROR_ECLASS_NEST_TOO_DEEP: c_int = 207;
pub const PCRE2_ERROR_ECLASS_INVALID_OPERATOR: c_int = 208;
pub const PCRE2_ERROR_ECLASS_UNEXPECTED_OPERATOR: c_int = 209;
pub const PCRE2_ERROR_ECLASS_EXPECTED_OPERAND: c_int = 210;
pub const PCRE2_ERROR_ECLASS_MIXED_OPERATORS: c_int = 211;
pub const PCRE2_ERROR_ECLASS_HINT_SQUARE_BRACKET: c_int = 212;
pub const PCRE2_ERROR_PERL_ECLASS_UNEXPECTED_EXPR: c_int = 213;
pub const PCRE2_ERROR_PERL_ECLASS_EMPTY_EXPR: c_int = 214;
pub const PCRE2_ERROR_PERL_ECLASS_MISSING_CLOSE: c_int = 215;
pub const PCRE2_ERROR_PERL_ECLASS_UNEXPECTED_CHAR: c_int = 216;
pub const PCRE2_ERROR_EXPECTED_CAPTURE_GROUP: c_int = 217;
pub const PCRE2_ERROR_MISSING_OPENING_PARENTHESIS: c_int = 218;
pub const PCRE2_ERROR_MISSING_NUMBER_TERMINATOR: c_int = 219;
pub const PCRE2_ERROR_NULL_ERROROFFSET: c_int = 220;

pub const PCRE2_ERROR_NOMATCH: c_int = -1;
pub const PCRE2_ERROR_PARTIAL: c_int = -2;

pub const PCRE2_ERROR_UTF8_ERR1: c_int = -3;
pub const PCRE2_ERROR_UTF8_ERR2: c_int = -4;
pub const PCRE2_ERROR_UTF8_ERR3: c_int = -5;
pub const PCRE2_ERROR_UTF8_ERR4: c_int = -6;
pub const PCRE2_ERROR_UTF8_ERR5: c_int = -7;
pub const PCRE2_ERROR_UTF8_ERR6: c_int = -8;
pub const PCRE2_ERROR_UTF8_ERR7: c_int = -9;
pub const PCRE2_ERROR_UTF8_ERR8: c_int = -10;
pub const PCRE2_ERROR_UTF8_ERR9: c_int = -11;
pub const PCRE2_ERROR_UTF8_ERR10: c_int = -12;
pub const PCRE2_ERROR_UTF8_ERR11: c_int = -13;
pub const PCRE2_ERROR_UTF8_ERR12: c_int = -14;
pub const PCRE2_ERROR_UTF8_ERR13: c_int = -15;
pub const PCRE2_ERROR_UTF8_ERR14: c_int = -16;
pub const PCRE2_ERROR_UTF8_ERR15: c_int = -17;
pub const PCRE2_ERROR_UTF8_ERR16: c_int = -18;
pub const PCRE2_ERROR_UTF8_ERR17: c_int = -19;
pub const PCRE2_ERROR_UTF8_ERR18: c_int = -20;
pub const PCRE2_ERROR_UTF8_ERR19: c_int = -21;
pub const PCRE2_ERROR_UTF8_ERR20: c_int = -22;
pub const PCRE2_ERROR_UTF8_ERR21: c_int = -23;

pub const PCRE2_ERROR_UTF16_ERR1: c_int = -24;
pub const PCRE2_ERROR_UTF16_ERR2: c_int = -25;
pub const PCRE2_ERROR_UTF16_ERR3: c_int = -26;
pub const PCRE2_ERROR_UTF32_ERR1: c_int = -27;
pub const PCRE2_ERROR_UTF32_ERR2: c_int = -28;

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

/* ------------------------- Info and config codes ------------------------- */

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

pub const PCRE2_OPTIMIZATION_NONE: u32 = 0;
pub const PCRE2_OPTIMIZATION_FULL: u32 = 1;
pub const PCRE2_AUTO_POSSESS: u32 = 64;
pub const PCRE2_AUTO_POSSESS_OFF: u32 = 65;
pub const PCRE2_DOTSTAR_ANCHOR: u32 = 66;
pub const PCRE2_DOTSTAR_ANCHOR_OFF: u32 = 67;
pub const PCRE2_START_OPTIMIZE: u32 = 68;
pub const PCRE2_START_OPTIMIZE_OFF: u32 = 69;

pub const PCRE2_SUBSTITUTE_CASE_LOWER: c_int = 1;
pub const PCRE2_SUBSTITUTE_CASE_UPPER: c_int = 2;
pub const PCRE2_SUBSTITUTE_CASE_TITLE_FIRST: c_int = 3;

pub const PCRE2_CALLOUT_STARTMATCH: u32 = 0x00000001;
pub const PCRE2_CALLOUT_BACKTRACK: u32 = 0x00000002;

/* ------------------------- Private pattern flags ------------------------- */

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

pub const PCRE2_OPTIM_AUTO_POSSESS: u32 = 0x00000001;
pub const PCRE2_OPTIM_DOTSTAR_ANCHOR: u32 = 0x00000002;
pub const PCRE2_OPTIM_START_OPTIMIZE: u32 = 0x00000004;
pub const PCRE2_OPTIMIZATION_ALL: u32 = 0x00000007;

/* Newline types */

pub const NLTYPE_FIXED: u32 = 0;
pub const NLTYPE_ANY: u32 = 1;
pub const NLTYPE_ANYCRLF: u32 = 2;

/* Character class bitmap offsets and character type bits */

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
pub const TABLES_LENGTH: usize = ctypes_offset + 256;

/* Unicode property types */

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
pub const PT_TABSIZE: usize = PT_ANY as usize;
pub const PT_PXGRAPH: u32 = 14;
pub const PT_PXPRINT: u32 = 15;
pub const PT_PXPUNCT: u32 = 16;
pub const PT_PXXDIGIT: u32 = 17;
pub const PT_NOTSCRIPT: u32 = 255;

/* Extended class flags */

pub const XCL_NOT: u32 = 0x01;
pub const XCL_MAP: u32 = 0x02;
pub const XCL_HASPROP: u32 = 0x04;

pub const XCL_END: u32 = 0;
pub const XCL_SINGLE: u32 = 1;
pub const XCL_RANGE: u32 = 2;
pub const XCL_PROP: u32 = 3;
pub const XCL_NOTPROP: u32 = 4;
/* sizeof(PCRE2_UCHAR) == 1 in 8-bit mode */
pub const XCL_LIST: u32 = 0x10;

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

pub const ECL_MAP: u32 = 0x01;
pub const ECL_AND: u8 = 1;
pub const ECL_OR: u8 = 2;
pub const ECL_XOR: u8 = 3;
pub const ECL_NOT: u8 = 4;
pub const ECL_XCLASS: u8 = 5;
pub const ECL_ANY: u8 = 6;
pub const ECL_NONE: u8 = 7;

/* UCD access */

pub const UCD_SCRIPTX_MASK: u16 = 0x3ff;
pub const UCD_BIDICLASS_SHIFT: u16 = 11;
pub const UCD_BPROPS_MASK: u16 = 0xfff;

/* ------------------------- Structures ------------------------- */

pub type MallocFn = Option<unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void>;
pub type FreeFn = Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct pcre2_memctl {
    pub malloc: MallocFn,
    pub free: FreeFn,
    pub memory_data: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct open_capitem {
    pub next: *mut open_capitem,
    pub number: u16,
    pub assert_depth: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UcpTypeTable {
    pub name_offset: u16,
    pub type_: u16,
    pub value: u16,
}

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
pub struct pcre2_serialized_data {
    pub magic: u32,
    pub version: u32,
    pub config: u32,
    pub number_of_codes: i32,
}

#[repr(C)]
pub struct pcre2_real_general_context {
    pub memctl: pcre2_memctl,
}

pub type StackGuardFn = Option<unsafe extern "C" fn(u32, *mut c_void) -> c_int>;

#[repr(C)]
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

pub type CalloutFn = Option<unsafe extern "C" fn(*mut pcre2_callout_block, *mut c_void) -> c_int>;
pub type SubstituteCalloutFn =
    Option<unsafe extern "C" fn(*mut pcre2_substitute_callout_block, *mut c_void) -> c_int>;
pub type SubstituteCaseCalloutFn = Option<
    unsafe extern "C" fn(PCRE2_SPTR, PCRE2_SIZE, *mut PCRE2_UCHAR, PCRE2_SIZE, c_int, *mut c_void)
        -> PCRE2_SIZE,
>;

/* Note: SUPPORT_JIT is not defined, so the jit_callback fields are absent. */
#[repr(C)]
pub struct pcre2_real_match_context {
    pub memctl: pcre2_memctl,
    pub callout: CalloutFn,
    pub callout_data: *mut c_void,
    pub substitute_callout: SubstituteCalloutFn,
    pub substitute_callout_data: *mut c_void,
    pub substitute_case_callout: SubstituteCaseCalloutFn,
    pub substitute_case_callout_data: *mut c_void,
    pub offset_limit: PCRE2_SIZE,
    pub heap_limit: u32,
    pub match_limit: u32,
    pub depth_limit: u32,
}

#[repr(C)]
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

#[repr(C)]
pub struct pcre2_real_match_data {
    pub memctl: pcre2_memctl,
    pub code: *const pcre2_real_code,
    pub subject: PCRE2_SPTR,
    pub mark: PCRE2_SPTR,
    pub heapframes: *mut heapframe,
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
    /* Followed by `oveccount * 2` PCRE2_SIZE values. */
    pub ovector: [PCRE2_SIZE; 0],
}

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

#[repr(C)]
pub struct pcre2_real_jit_stack {
    pub memctl: pcre2_memctl,
    pub stack: *mut c_void,
}

/* ---------------- Private compile-time structures ---------------- */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct recurse_check {
    pub prev: *const recurse_check,
    pub group: PCRE2_SPTR,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct parsed_recurse_check {
    pub prev: *const parsed_recurse_check,
    pub groupptr: *const u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct recurse_cache {
    pub group: PCRE2_SPTR,
    pub groupnumber: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct branch_chain {
    pub outer: *const branch_chain,
    pub current_branch: *mut PCRE2_UCHAR,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct named_group {
    pub name: PCRE2_SPTR,
    pub number: u32,
    pub length: u16,
    pub hash_dup: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct compile_data {
    pub next: *mut compile_data,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct class_ranges {
    pub header: compile_data,
    pub char_lists_size: usize,
    pub char_lists_start: usize,
    pub range_list_size: u16,
    pub char_lists_types: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct recurse_arguments {
    pub header: compile_data,
    pub size: usize,
    pub skip_size: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
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
    /* SUPPORT_WIDE_CHARS is defined in 8-bit mode with SUPPORT_UNICODE */
    pub char_lists_size: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct dfa_recursion_info {
    pub prevrec: *const dfa_recursion_info,
    pub subject_position: PCRE2_SPTR,
    pub last_used_ptr: PCRE2_SPTR,
    pub group_num: u32,
}

/* ---------------- heapframe ---------------- */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_char_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub charptr: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
    pub c: u32,
    pub oc: hf_oc,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union hf_oc {
    pub oc: u32,
    pub occu: [PCRE2_UCHAR; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_charnot_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
    pub c: u32,
    pub oc: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_class_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub byte_map_address: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_xclass_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub xclass_data: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_eclass_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub eclass_data: PCRE2_SPTR,
    pub eclass_len: PCRE2_SIZE,
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_type_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
    pub ctype: u32,
    pub propvalue: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_ref_repeat {
    pub start: PCRE2_SPTR,
    pub offset: PCRE2_SIZE,
    pub length: PCRE2_SIZE,
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_op_bra {
    pub frame_type: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_op_brapos {
    pub start_eptr: PCRE2_SPTR,
    pub start_group: PCRE2_SPTR,
    pub frame_type: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_op_recurse {
    pub start_branch: PCRE2_SPTR,
    pub frame_type: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_op_assert_scs {
    pub saved_end_subject: PCRE2_SPTR,
    pub saved_eptr: PCRE2_SPTR,
    pub true_end_extra: PCRE2_SIZE,
    pub saved_moptions: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_op_cond {
    pub start_branch: PCRE2_SPTR,
    pub length: PCRE2_SIZE,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct hf_op_vreverse {
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union heapframe_fields {
    pub char_repeat: hf_char_repeat,
    pub charnot_repeat: hf_charnot_repeat,
    pub class_repeat: hf_class_repeat,
    pub xclass_repeat: hf_xclass_repeat,
    pub eclass_repeat: hf_eclass_repeat,
    pub type_repeat: hf_type_repeat,
    pub ref_repeat: hf_ref_repeat,
    pub op_bra: hf_op_bra,
    pub op_brapos: hf_op_brapos,
    pub op_recurse: hf_op_recurse,
    pub op_assert_scs: hf_op_assert_scs,
    pub op_cond: hf_op_cond,
    pub op_vreverse: hf_op_vreverse,
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
    pub fields: heapframe_fields,
    pub eptr: PCRE2_SPTR,
    pub start_match: PCRE2_SPTR,
    pub mark: PCRE2_SPTR,
    pub recurse_last_used: PCRE2_SPTR,
    pub current_recurse: u32,
    pub capture_last: u32,
    pub last_group_offset: PCRE2_SIZE,
    pub offset_top: PCRE2_SIZE,
    /* Followed by `2 * (top_bracket + 1)` PCRE2_SIZE values. */
    pub ovector: [PCRE2_SIZE; 0],
}

/// `offsetof(heapframe, ovector)` in the C code.
pub const HEAPFRAME_OVECTOR_OFFSET: usize = core::mem::offset_of!(heapframe, ovector);
/// `offsetof(heapframe_align, frame)` in the C code.
pub const HEAPFRAME_ALIGNMENT: usize = core::mem::align_of::<heapframe>();
/// `offsetof(pcre2_real_match_data, ovector)` in the C code.
pub const MATCH_DATA_OVECTOR_OFFSET: usize = core::mem::offset_of!(pcre2_real_match_data, ovector);

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

/* ------------------------- libc bindings ------------------------- */

unsafe extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    pub fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn strlen(s: *const c_char) -> usize;
    pub fn snprintf(buf: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
}

/// The default allocator used by PCRE2 when no custom one is supplied.
pub unsafe extern "C" fn default_malloc(size: usize, _data: *mut c_void) -> *mut c_void {
    unsafe { malloc(size) }
}

/// The default deallocator used by PCRE2 when no custom one is supplied.
pub unsafe extern "C" fn default_free(ptr: *mut c_void, _data: *mut c_void) {
    unsafe { free(ptr) }
}

/// `memcpy` for non-overlapping regions.
#[inline]
pub unsafe fn memcpy<T>(dst: *mut T, src: *const T, count: usize) {
    unsafe { core::ptr::copy_nonoverlapping(src, dst, count) };
}

/// `memmove`, permitting overlap.
#[inline]
pub unsafe fn memmove<T>(dst: *mut T, src: *const T, count: usize) {
    unsafe { core::ptr::copy(src, dst, count) };
}

/// `memset`.
#[inline]
pub unsafe fn memset(dst: *mut u8, value: u8, count: usize) {
    unsafe { core::ptr::write_bytes(dst, value, count) };
}

/* ------------------------- Code unit access helpers ------------------------- */

/* LINK_SIZE == 2 */

/// `PUT(a, n, d)`
#[inline]
pub unsafe fn put(a: *mut PCRE2_UCHAR, n: usize, d: i32) {
    unsafe {
        *a.add(n) = (d >> 8) as u8;
        *a.add(n + 1) = (d & 255) as u8;
    }
}

/// `GET(a, n)`
#[inline]
pub unsafe fn get(a: PCRE2_SPTR, n: usize) -> c_int {
    unsafe { (((*a.add(n) as u32) << 8) | (*a.add(n + 1) as u32)) as c_int }
}

/// `PUT2(a, n, d)`
#[inline]
pub unsafe fn put2(a: *mut PCRE2_UCHAR, n: usize, d: u32) {
    unsafe {
        *a.add(n) = (d >> 8) as u8;
        *a.add(n + 1) = (d & 255) as u8;
    }
}

/// `GET2(a, n)`
#[inline]
pub unsafe fn get2(a: PCRE2_SPTR, n: usize) -> u32 {
    unsafe { ((*a.add(n) as u32) << 8) | (*a.add(n + 1) as u32) }
}

/// `CU2BYTES(x)` -- code units to bytes (identity in 8-bit mode).
#[inline]
pub const fn cu2bytes(x: usize) -> usize {
    x
}

/// `BYTES2CU(x)` -- bytes to code units (identity in 8-bit mode).
#[inline]
pub const fn bytes2cu(x: usize) -> usize {
    x
}

/// `TABLE_GET(c, table, default)` -- in 8-bit mode the table is indexed directly.
#[inline]
pub unsafe fn table_get(c: u32, table: *const u8, _default: u32) -> u32 {
    unsafe { *table.add(c as usize) as u32 }
}

/// `MAX_255(c)` -- always true in 8-bit mode.
#[inline]
pub const fn max_255(_c: u32) -> bool {
    true
}

/// `CHMAX_255(c)` -- with Unicode support in 8-bit mode.
#[inline]
pub const fn chmax_255(c: u32) -> bool {
    c <= 255
}

/// `HASUTF8EXTRALEN(c)`
#[inline]
pub const fn hasutf8extralen(c: u32) -> bool {
    c >= 0xc0
}

/// `HAS_EXTRALEN(c)`
#[inline]
pub const fn has_extralen(c: u32) -> bool {
    c >= 0xc0
}

/// `NOT_FIRSTCU(c)`
#[inline]
pub const fn not_firstcu(c: u32) -> bool {
    (c & 0xc0) == 0x80
}

/// `GET_EXTRALEN(c)`
#[inline]
pub fn get_extralen(c: u32) -> u32 {
    UTF8_TABLE4[(c & 0x3f) as usize] as u32
}

/// `GETUTF8(c, eptr)` -- complete a multi-code-unit character without advancing.
#[inline]
pub unsafe fn getutf8(c: u32, eptr: PCRE2_SPTR) -> u32 {
    unsafe {
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
}

/// `GETUTF8LEN(c, eptr, len)` -- returns the character and the length increment.
#[inline]
pub unsafe fn getutf8len(c: u32, eptr: PCRE2_SPTR) -> (u32, u32) {
    unsafe {
        if (c & 0x20) == 0 {
            (((c & 0x1f) << 6) | (*eptr.add(1) as u32 & 0x3f), 1)
        } else if (c & 0x10) == 0 {
            (
                ((c & 0x0f) << 12)
                    | ((*eptr.add(1) as u32 & 0x3f) << 6)
                    | (*eptr.add(2) as u32 & 0x3f),
                2,
            )
        } else if (c & 0x08) == 0 {
            (
                ((c & 0x07) << 18)
                    | ((*eptr.add(1) as u32 & 0x3f) << 12)
                    | ((*eptr.add(2) as u32 & 0x3f) << 6)
                    | (*eptr.add(3) as u32 & 0x3f),
                3,
            )
        } else if (c & 0x04) == 0 {
            (
                ((c & 0x03) << 24)
                    | ((*eptr.add(1) as u32 & 0x3f) << 18)
                    | ((*eptr.add(2) as u32 & 0x3f) << 12)
                    | ((*eptr.add(3) as u32 & 0x3f) << 6)
                    | (*eptr.add(4) as u32 & 0x3f),
                4,
            )
        } else {
            (
                ((c & 0x01) << 30)
                    | ((*eptr.add(1) as u32 & 0x3f) << 24)
                    | ((*eptr.add(2) as u32 & 0x3f) << 18)
                    | ((*eptr.add(3) as u32 & 0x3f) << 12)
                    | ((*eptr.add(4) as u32 & 0x3f) << 6)
                    | (*eptr.add(5) as u32 & 0x3f),
                5,
            )
        }
    }
}

/// `GETCHAR(c, eptr)` -- get a character, assuming UTF mode, without advancing.
#[inline]
pub unsafe fn getchar_(eptr: PCRE2_SPTR) -> u32 {
    unsafe {
        let c = *eptr as u32;
        if c >= 0xc0 { getutf8(c, eptr) } else { c }
    }
}

/// `GETCHARTEST(c, eptr)` -- as `getchar_` but only decodes when `utf` is set.
#[inline]
pub unsafe fn getchartest(eptr: PCRE2_SPTR, utf: bool) -> u32 {
    unsafe {
        let c = *eptr as u32;
        if utf && c >= 0xc0 { getutf8(c, eptr) } else { c }
    }
}

/// `GETCHARINC(c, eptr)` -- get a character in UTF mode and advance the pointer.
#[inline]
pub unsafe fn getcharinc(eptr: &mut PCRE2_SPTR) -> u32 {
    unsafe {
        let c = **eptr as u32;
        *eptr = eptr.add(1);
        if c < 0xc0 {
            return c;
        }
        let p = *eptr;
        if (c & 0x20) == 0 {
            *eptr = eptr.add(1);
            ((c & 0x1f) << 6) | (*p as u32 & 0x3f)
        } else if (c & 0x10) == 0 {
            *eptr = eptr.add(2);
            ((c & 0x0f) << 12) | ((*p as u32 & 0x3f) << 6) | (*p.add(1) as u32 & 0x3f)
        } else if (c & 0x08) == 0 {
            *eptr = eptr.add(3);
            ((c & 0x07) << 18)
                | ((*p as u32 & 0x3f) << 12)
                | ((*p.add(1) as u32 & 0x3f) << 6)
                | (*p.add(2) as u32 & 0x3f)
        } else if (c & 0x04) == 0 {
            *eptr = eptr.add(4);
            ((c & 0x03) << 24)
                | ((*p as u32 & 0x3f) << 18)
                | ((*p.add(1) as u32 & 0x3f) << 12)
                | ((*p.add(2) as u32 & 0x3f) << 6)
                | (*p.add(3) as u32 & 0x3f)
        } else {
            *eptr = eptr.add(5);
            ((c & 0x01) << 30)
                | ((*p as u32 & 0x3f) << 24)
                | ((*p.add(1) as u32 & 0x3f) << 18)
                | ((*p.add(2) as u32 & 0x3f) << 12)
                | ((*p.add(3) as u32 & 0x3f) << 6)
                | (*p.add(4) as u32 & 0x3f)
        }
    }
}

/// `GETCHARINCTEST(c, eptr)`
#[inline]
pub unsafe fn getcharinctest(eptr: &mut PCRE2_SPTR, utf: bool) -> u32 {
    unsafe {
        if !utf {
            let c = **eptr as u32;
            *eptr = eptr.add(1);
            c
        } else {
            getcharinc(eptr)
        }
    }
}

/// `GETCHARLEN(c, eptr, len)` -- returns `(char, len_increment)`.
#[inline]
pub unsafe fn getcharlen(eptr: PCRE2_SPTR) -> (u32, u32) {
    unsafe {
        let c = *eptr as u32;
        if c >= 0xc0 { getutf8len(c, eptr) } else { (c, 0) }
    }
}

/// `GETCHARLENTEST(c, eptr, len)`
#[inline]
pub unsafe fn getcharlentest(eptr: PCRE2_SPTR, utf: bool) -> (u32, u32) {
    unsafe {
        let c = *eptr as u32;
        if utf && c >= 0xc0 { getutf8len(c, eptr) } else { (c, 0) }
    }
}

/// `PUTCHAR(c, p)` -- deposit a character, returning the number of code units.
#[inline]
pub unsafe fn putchar_(c: u32, p: *mut PCRE2_UCHAR, utf: bool) -> u32 {
    unsafe {
        if utf && c > MAX_UTF_SINGLE_CU {
            crate::ord2utf::ord2utf(c, p)
        } else {
            *p = c as u8;
            1
        }
    }
}

/// `BACKCHAR(eptr)`
#[inline]
pub unsafe fn backchar(eptr: &mut PCRE2_SPTR) {
    unsafe {
        while (**eptr & 0xc0) == 0x80 {
            *eptr = eptr.sub(1);
        }
    }
}

/// `FORWARDCHAR(eptr)`
#[inline]
pub unsafe fn forwardchar(eptr: &mut PCRE2_SPTR) {
    unsafe {
        while (**eptr & 0xc0) == 0x80 {
            *eptr = eptr.add(1);
        }
    }
}

/// `FORWARDCHARTEST(eptr, end)`
#[inline]
pub unsafe fn forwardchartest(eptr: &mut PCRE2_SPTR, end: PCRE2_SPTR) {
    unsafe {
        while *eptr < end && (**eptr & 0xc0) == 0x80 {
            *eptr = eptr.add(1);
        }
    }
}

/* ------------------------- UCD access ------------------------- */

pub use crate::ucd::{
    UCD_BOOLPROP_SETS, UCD_CASELESS_SETS, UCD_DIGIT_SETS, UCD_NOCASE_RANGES,
    UCD_NOCASE_RANGES_SIZE, UCD_RECORDS, UCD_SCRIPT_SETS, UCD_STAGE1, UCD_STAGE2,
    UCD_TURKISH_DOTTED_I_CASESET, UNICODE_VERSION,
};

/// `GET_UCD(ch)`
///
/// In 8-bit mode this is `REAL_GET_UCD(ch)`, which performs no bounds check: if
/// a caller passes a code point above `MAX_UTF_CODE_POINT` (only possible when
/// the application passes invalid UTF together with `PCRE2_NO_UTF_CHECK`, which
/// PCRE2 documents as undefined behaviour) the C reads outside the tables. The
/// accesses are unchecked here so that this matches the C rather than aborting
/// with a bounds-check panic.
#[inline]
pub fn get_ucd(ch: u32) -> &'static UcdRecord {
    let i = ch as usize;
    unsafe {
        let stage1 = *UCD_STAGE1.get_unchecked(i / UCD_BLOCK_SIZE) as usize;
        let idx = *UCD_STAGE2.get_unchecked(stage1 * UCD_BLOCK_SIZE + i % UCD_BLOCK_SIZE) as usize;
        UCD_RECORDS.get_unchecked(idx)
    }
}

/// `UCD_CHARTYPE(ch)`
#[inline]
pub fn ucd_chartype(ch: u32) -> u32 {
    get_ucd(ch).chartype as u32
}

/// `UCD_SCRIPT(ch)`
#[inline]
pub fn ucd_script(ch: u32) -> u32 {
    get_ucd(ch).script as u32
}

/// `UCD_CATEGORY(ch)`
#[inline]
pub fn ucd_category(ch: u32) -> u32 {
    UCP_GENTYPE[ucd_chartype(ch) as usize]
}

/// `UCD_GRAPHBREAK(ch)`
#[inline]
pub fn ucd_graphbreak(ch: u32) -> u32 {
    get_ucd(ch).gbprop as u32
}

/// `UCD_CASESET(ch)`
#[inline]
pub fn ucd_caseset(ch: u32) -> u32 {
    get_ucd(ch).caseset as u32
}

/// `UCD_OTHERCASE(ch)`
#[inline]
pub fn ucd_othercase(ch: u32) -> u32 {
    (ch as i32).wrapping_add(get_ucd(ch).other_case) as u32
}

/// `UCD_SCRIPTX_PROP(prop)`
#[inline]
pub fn ucd_scriptx_prop(prop: &UcdRecord) -> u32 {
    (prop.scriptx_bidiclass & UCD_SCRIPTX_MASK) as u32
}

/// `UCD_BIDICLASS_PROP(prop)`
#[inline]
pub fn ucd_bidiclass_prop(prop: &UcdRecord) -> u32 {
    (prop.scriptx_bidiclass >> UCD_BIDICLASS_SHIFT) as u32
}

/// `UCD_BPROPS_PROP(prop)`
#[inline]
pub fn ucd_bprops_prop(prop: &UcdRecord) -> u32 {
    (prop.bprops & UCD_BPROPS_MASK) as u32
}

/// `UCD_SCRIPTX(ch)`
#[inline]
pub fn ucd_scriptx(ch: u32) -> u32 {
    ucd_scriptx_prop(get_ucd(ch))
}

/// `UCD_BPROPS(ch)`
#[inline]
pub fn ucd_bprops(ch: u32) -> u32 {
    ucd_bprops_prop(get_ucd(ch))
}

/// `UCD_BIDICLASS(ch)`
#[inline]
pub fn ucd_bidiclass(ch: u32) -> u32 {
    ucd_bidiclass_prop(get_ucd(ch))
}

/// `UCD_ANY_I(ch)` -- 'i', 'I', U+0130 or U+0131.
#[inline]
pub const fn ucd_any_i(ch: u32) -> bool {
    (ch | 0x20) == 0x69 || (ch | 1) == 0x0131
}

/// `UCD_DOTTED_I(ch)`
#[inline]
pub const fn ucd_dotted_i(ch: u32) -> bool {
    ch == 0x69 || ch == 0x0130
}

/// `UCD_FOLD_I_TURKISH(ch)`
#[inline]
pub const fn ucd_fold_i_turkish(ch: u32) -> u32 {
    if ch == 0x0130 {
        0x69
    } else if ch == 0x49 {
        0x0131
    } else {
        ch
    }
}

/// `MAPBIT(map, n)`
#[inline]
pub fn mapbit(map: &[u32], n: u32) -> u32 {
    map[(n / 32) as usize] & (1u32 << (n % 32))
}

/// `MAPSET(map, n)`
#[inline]
pub fn mapset(map: &mut [u32], n: u32) {
    map[(n / 32) as usize] |= 1u32 << (n % 32);
}

/* ------------------------- Shared data tables ------------------------- */

pub static UTF8_TABLE1: [c_int; 6] = [0x7f, 0x7ff, 0xffff, 0x1fffff, 0x3ffffff, 0x7fffffff];
pub const UTF8_TABLE1_SIZE: u32 = 6;
pub static UTF8_TABLE2: [c_int; 6] = [0, 0xc0, 0xe0, 0xf0, 0xf8, 0xfc];
pub static UTF8_TABLE3: [c_int; 6] = [0xff, 0x1f, 0x0f, 0x07, 0x03, 0x01];
#[rustfmt::skip]
pub static UTF8_TABLE4: [u8; 64] = [
  1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
  1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
  2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,
  3,3,3,3,3,3,3,3,4,4,4,4,5,5,5,5];

/// `PRIV(ucp_gentype)` -- particular category to general category.
#[rustfmt::skip]
pub static UCP_GENTYPE: [u32; 30] = [
  crate::ucp::ucp_C, crate::ucp::ucp_C, crate::ucp::ucp_C, crate::ucp::ucp_C, crate::ucp::ucp_C,
  crate::ucp::ucp_L, crate::ucp::ucp_L, crate::ucp::ucp_L, crate::ucp::ucp_L, crate::ucp::ucp_L,
  crate::ucp::ucp_M, crate::ucp::ucp_M, crate::ucp::ucp_M,
  crate::ucp::ucp_N, crate::ucp::ucp_N, crate::ucp::ucp_N,
  crate::ucp::ucp_P, crate::ucp::ucp_P, crate::ucp::ucp_P, crate::ucp::ucp_P, crate::ucp::ucp_P,
  crate::ucp::ucp_P, crate::ucp::ucp_P,
  crate::ucp::ucp_S, crate::ucp::ucp_S, crate::ucp::ucp_S, crate::ucp::ucp_S,
  crate::ucp::ucp_Z, crate::ucp::ucp_Z, crate::ucp::ucp_Z,
];

/// `HSPACE_LIST`
pub static HSPACE_LIST: [u32; 20] = [
    0x09, 0x20, 0xa0, 0x1680, 0x180e, 0x2000, 0x2001, 0x2002, 0x2003, 0x2004, 0x2005, 0x2006,
    0x2007, 0x2008, 0x2009, 0x200a, 0x202f, 0x205f, 0x3000, NOTACHAR,
];

/// `VSPACE_LIST`
pub static VSPACE_LIST: [u32; 8] =
    [0x0a, 0x0b, 0x0c, 0x0d, 0x85, 0x2028, 0x2029, NOTACHAR];

/// `PRIV(callout_start_delims)`
pub static CALLOUT_START_DELIMS: [u32; 9] =
    [0x60, 0x27, 0x22, 0x5e, 0x25, 0x23, 0x24, 0x7b, 0];

/// `PRIV(callout_end_delims)`
pub static CALLOUT_END_DELIMS: [u32; 9] = [0x60, 0x27, 0x22, 0x5e, 0x25, 0x23, 0x24, 0x7d, 0];

/// `PRIV(ucp_gbtable)` -- grapheme break table.
pub static UCP_GBTABLE: [u32; 15] = {
    use crate::ucp::*;
    const ESZ: u32 = (1 << ucp_gbExtend) | (1 << ucp_gbSpacingMark) | (1 << ucp_gbZWJ);
    [
        1u32 << ucp_gbLF,
        0,
        0,
        ESZ,
        ESZ | (1u32 << ucp_gbPrepend)
            | (1u32 << ucp_gbL)
            | (1u32 << ucp_gbV)
            | (1u32 << ucp_gbT)
            | (1u32 << ucp_gbLV)
            | (1u32 << ucp_gbLVT)
            | (1u32 << ucp_gbOther)
            | (1u32 << ucp_gbRegional_Indicator),
        ESZ,
        ESZ | (1u32 << ucp_gbL) | (1u32 << ucp_gbV) | (1u32 << ucp_gbLV) | (1u32 << ucp_gbLVT),
        ESZ | (1u32 << ucp_gbV) | (1u32 << ucp_gbT),
        ESZ | (1u32 << ucp_gbT),
        ESZ | (1u32 << ucp_gbV) | (1u32 << ucp_gbT),
        ESZ | (1u32 << ucp_gbT),
        1u32 << ucp_gbRegional_Indicator,
        ESZ,
        ESZ | (1u32 << ucp_gbExtended_Pictographic),
        ESZ,
    ]
};

/* ------------------------- Memory helper ------------------------- */

/// `PRIV(memctl_malloc)`: allocate a block whose first field is a `pcre2_memctl`
/// copy, so that the block can later free itself.
#[inline]
pub unsafe fn memctl_malloc(size: usize, memctl: *mut pcre2_memctl) -> *mut c_void {
    unsafe {
        let yield_ = if memctl.is_null() {
            malloc(size)
        } else {
            ((*memctl).malloc.unwrap())(size, (*memctl).memory_data)
        };
        if yield_.is_null() {
            return core::ptr::null_mut();
        }
        let newmemctl = yield_ as *mut pcre2_memctl;
        if memctl.is_null() {
            (*newmemctl).malloc = Some(default_malloc);
            (*newmemctl).free = Some(default_free);
            (*newmemctl).memory_data = core::ptr::null_mut();
        } else {
            *newmemctl = *memctl;
        }
        yield_
    }
}

/// Exported as `_pcre2_memctl_malloc_8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_memctl_malloc_8(
    size: usize,
    memctl: *mut pcre2_memctl,
) -> *mut c_void {
    unsafe { memctl_malloc(size, memctl) }
}
