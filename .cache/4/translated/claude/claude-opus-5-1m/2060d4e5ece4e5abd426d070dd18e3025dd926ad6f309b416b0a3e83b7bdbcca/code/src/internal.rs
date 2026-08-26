// Translated from pcre2_internal.h / pcre2_intmodedep.h / config.h
// 8-bit code unit width, SUPPORT_UNICODE, no JIT, LINK_SIZE == 2.
#![allow(non_upper_case_globals, non_camel_case_types, dead_code)]

use core::ffi::{c_char, c_int, c_uint, c_void};

pub type PCRE2_UCHAR = u8;
pub type PCRE2_SPTR = *const u8;
pub type PCRE2_SIZE = usize;
pub type BOOL = c_int;

pub const TRUE: BOOL = 1;
pub const FALSE: BOOL = 0;

/* ---------------- config.h values ---------------- */

pub const HEAP_LIMIT: u32 = 20000000;
pub const LINK_SIZE: usize = 2;
pub const MATCH_LIMIT: u32 = 10000000;
pub const MATCH_LIMIT_DEPTH: u32 = MATCH_LIMIT;
pub const MAX_NAME_COUNT: u32 = 10000;
pub const MAX_NAME_SIZE: u32 = 128;
pub const MAX_VARLOOKBEHIND: u32 = 255;
pub const NEWLINE_DEFAULT: u32 = 2;
pub const PARENS_NEST_LIMIT: u32 = 250;
pub const PACKAGE_VERSION: &str = "10.48-DEV";

/* ---------------- Basic constants ---------------- */

pub const NOTACHAR: u32 = 0xffffffff;
pub const MAX_UTF_CODE_POINT: u32 = 0x10ffff;
pub const COMPILE_ERROR_BASE: c_int = 100;
pub const START_FRAMES_SIZE: usize = 20480;
pub const DFA_START_RWS_SIZE: usize = 30720;
pub const BSR_DEFAULT: u32 = crate::pcre2_pub::PCRE2_BSR_UNICODE;

pub const NLTYPE_FIXED: u32 = 0;
pub const NLTYPE_ANY: u32 = 1;
pub const NLTYPE_ANYCRLF: u32 = 2;

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

pub const PCRE2_MATCHEDBY_INTERPRETER: u32 = 0;
pub const PCRE2_MATCHEDBY_DFA_INTERPRETER: u32 = 1;
pub const PCRE2_MATCHEDBY_JIT: u32 = 2;

pub const PCRE2_MD_COPIED_SUBJECT: u8 = 0x01;

pub const MAGIC_NUMBER: u32 = 0x50435245;

pub const REQ_CU_MAX: usize = 5000;
pub const ECLASS_NEST_LIMIT: usize = 15;

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

pub const PCRE2_OPTIM_AUTO_POSSESS: u32 = 0x00000001;
pub const PCRE2_OPTIM_DOTSTAR_ANCHOR: u32 = 0x00000002;
pub const PCRE2_OPTIM_START_OPTIMIZE: u32 = 0x00000004;
pub const PCRE2_OPTIMIZATION_ALL: u32 = 0x00000007;

pub const MAX_NON_UTF_CHAR: u32 = 0xff;
pub const MAX_PATTERN_SIZE: usize = 1 << 16;
pub const IMM2_SIZE: usize = 2;
pub const MAX_MARK: u32 = (1u32 << 8) - 1;
pub const MAX_UTF_SINGLE_CU: u32 = 127;
pub const LOOKBEHIND_MAX: c_int = u16::MAX as c_int;

/* ---------------- Character names (ASCII) ---------------- */

pub const CHAR_HT: u32 = 0o11;
pub const CHAR_VT: u32 = 0o13;
pub const CHAR_FF: u32 = 0o14;
pub const CHAR_CR: u32 = 0o15;
pub const CHAR_LF: u32 = 0o12;
pub const CHAR_NL: u32 = CHAR_LF;
pub const CHAR_NEL: u32 = 0x85;
pub const CHAR_BS: u32 = 0o10;
pub const CHAR_BEL: u32 = 0o7;
pub const CHAR_ESC: u32 = 0o33;
pub const CHAR_DEL: u32 = 0o177;
pub const CHAR_NUL: u32 = 0;
pub const CHAR_SPACE: u32 = 0o40;
pub const CHAR_EXCLAMATION_MARK: u32 = 0o41;
pub const CHAR_QUOTATION_MARK: u32 = 0o42;
pub const CHAR_NUMBER_SIGN: u32 = 0o43;
pub const CHAR_DOLLAR_SIGN: u32 = 0o44;
pub const CHAR_PERCENT_SIGN: u32 = 0o45;
pub const CHAR_AMPERSAND: u32 = 0o46;
pub const CHAR_APOSTROPHE: u32 = 0o47;
pub const CHAR_LEFT_PARENTHESIS: u32 = 0o50;
pub const CHAR_RIGHT_PARENTHESIS: u32 = 0o51;
pub const CHAR_ASTERISK: u32 = 0o52;
pub const CHAR_PLUS: u32 = 0o53;
pub const CHAR_COMMA: u32 = 0o54;
pub const CHAR_MINUS: u32 = 0o55;
pub const CHAR_DOT: u32 = 0o56;
pub const CHAR_SLASH: u32 = 0o57;
pub const CHAR_0: u32 = 0o60;
pub const CHAR_1: u32 = 0o61;
pub const CHAR_2: u32 = 0o62;
pub const CHAR_3: u32 = 0o63;
pub const CHAR_4: u32 = 0o64;
pub const CHAR_5: u32 = 0o65;
pub const CHAR_6: u32 = 0o66;
pub const CHAR_7: u32 = 0o67;
pub const CHAR_8: u32 = 0o70;
pub const CHAR_9: u32 = 0o71;
pub const CHAR_COLON: u32 = 0o72;
pub const CHAR_SEMICOLON: u32 = 0o73;
pub const CHAR_LESS_THAN_SIGN: u32 = 0o74;
pub const CHAR_EQUALS_SIGN: u32 = 0o75;
pub const CHAR_GREATER_THAN_SIGN: u32 = 0o76;
pub const CHAR_QUESTION_MARK: u32 = 0o77;
pub const CHAR_COMMERCIAL_AT: u32 = 0o100;
pub const CHAR_A: u32 = 0o101;
pub const CHAR_B: u32 = 0o102;
pub const CHAR_C: u32 = 0o103;
pub const CHAR_D: u32 = 0o104;
pub const CHAR_E: u32 = 0o105;
pub const CHAR_F: u32 = 0o106;
pub const CHAR_G: u32 = 0o107;
pub const CHAR_H: u32 = 0o110;
pub const CHAR_I: u32 = 0o111;
pub const CHAR_J: u32 = 0o112;
pub const CHAR_K: u32 = 0o113;
pub const CHAR_L: u32 = 0o114;
pub const CHAR_M: u32 = 0o115;
pub const CHAR_N: u32 = 0o116;
pub const CHAR_O: u32 = 0o117;
pub const CHAR_P: u32 = 0o120;
pub const CHAR_Q: u32 = 0o121;
pub const CHAR_R: u32 = 0o122;
pub const CHAR_S: u32 = 0o123;
pub const CHAR_T: u32 = 0o124;
pub const CHAR_U: u32 = 0o125;
pub const CHAR_V: u32 = 0o126;
pub const CHAR_W: u32 = 0o127;
pub const CHAR_X: u32 = 0o130;
pub const CHAR_Y: u32 = 0o131;
pub const CHAR_Z: u32 = 0o132;
pub const CHAR_LEFT_SQUARE_BRACKET: u32 = 0o133;
pub const CHAR_BACKSLASH: u32 = 0o134;
pub const CHAR_RIGHT_SQUARE_BRACKET: u32 = 0o135;
pub const CHAR_CIRCUMFLEX_ACCENT: u32 = 0o136;
pub const CHAR_UNDERSCORE: u32 = 0o137;
pub const CHAR_GRAVE_ACCENT: u32 = 0o140;
pub const CHAR_a: u32 = 0o141;
pub const CHAR_b: u32 = 0o142;
pub const CHAR_c: u32 = 0o143;
pub const CHAR_d: u32 = 0o144;
pub const CHAR_e: u32 = 0o145;
pub const CHAR_f: u32 = 0o146;
pub const CHAR_g: u32 = 0o147;
pub const CHAR_h: u32 = 0o150;
pub const CHAR_i: u32 = 0o151;
pub const CHAR_j: u32 = 0o152;
pub const CHAR_k: u32 = 0o153;
pub const CHAR_l: u32 = 0o154;
pub const CHAR_m: u32 = 0o155;
pub const CHAR_n: u32 = 0o156;
pub const CHAR_o: u32 = 0o157;
pub const CHAR_p: u32 = 0o160;
pub const CHAR_q: u32 = 0o161;
pub const CHAR_r: u32 = 0o162;
pub const CHAR_s: u32 = 0o163;
pub const CHAR_t: u32 = 0o164;
pub const CHAR_u: u32 = 0o165;
pub const CHAR_v: u32 = 0o166;
pub const CHAR_w: u32 = 0o167;
pub const CHAR_x: u32 = 0o170;
pub const CHAR_y: u32 = 0o171;
pub const CHAR_z: u32 = 0o172;
pub const CHAR_LEFT_CURLY_BRACKET: u32 = 0o173;
pub const CHAR_VERTICAL_LINE: u32 = 0o174;
pub const CHAR_RIGHT_CURLY_BRACKET: u32 = 0o175;
pub const CHAR_TILDE: u32 = 0o176;
pub const CHAR_NBSP: u32 = 0xa0;

/* ---------------- Property types ---------------- */

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

pub const XCL_NOT: u32 = 0x01;
pub const XCL_MAP: u32 = 0x02;
pub const XCL_HASPROP: u32 = 0x04;

pub const XCL_END: u32 = 0;
pub const XCL_SINGLE: u32 = 1;
pub const XCL_RANGE: u32 = 2;
pub const XCL_PROP: u32 = 3;
pub const XCL_NOTPROP: u32 = 4;
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

pub const ECL_MAP: u32 = 0x01;
pub const ECL_AND: u32 = 1;
pub const ECL_OR: u32 = 2;
pub const ECL_XOR: u32 = 3;
pub const ECL_NOT: u32 = 4;
pub const ECL_XCLASS: u32 = 5;
pub const ECL_ANY: u32 = 6;
pub const ECL_NONE: u32 = 7;

/* ESC_ values */
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

/* ---------------- Opcodes ---------------- */

pub const OP_END: u32 = 0;
pub const OP_SOD: u32 = 1;
pub const OP_SOM: u32 = 2;
pub const OP_SET_SOM: u32 = 3;
pub const OP_NOT_WORD_BOUNDARY: u32 = 4;
pub const OP_WORD_BOUNDARY: u32 = 5;
pub const OP_NOT_DIGIT: u32 = 6;
pub const OP_DIGIT: u32 = 7;
pub const OP_NOT_WHITESPACE: u32 = 8;
pub const OP_WHITESPACE: u32 = 9;
pub const OP_NOT_WORDCHAR: u32 = 10;
pub const OP_WORDCHAR: u32 = 11;
pub const OP_ANY: u32 = 12;
pub const OP_ALLANY: u32 = 13;
pub const OP_ANYBYTE: u32 = 14;
pub const OP_NOTPROP: u32 = 15;
pub const OP_PROP: u32 = 16;
pub const OP_ANYNL: u32 = 17;
pub const OP_NOT_HSPACE: u32 = 18;
pub const OP_HSPACE: u32 = 19;
pub const OP_NOT_VSPACE: u32 = 20;
pub const OP_VSPACE: u32 = 21;
pub const OP_EXTUNI: u32 = 22;
pub const OP_EODN: u32 = 23;
pub const OP_EOD: u32 = 24;
pub const OP_DOLL: u32 = 25;
pub const OP_DOLLM: u32 = 26;
pub const OP_CIRC: u32 = 27;
pub const OP_CIRCM: u32 = 28;
pub const OP_CHAR: u32 = 29;
pub const OP_CHARI: u32 = 30;
pub const OP_NOT: u32 = 31;
pub const OP_NOTI: u32 = 32;
pub const OP_STAR: u32 = 33;
pub const OP_MINSTAR: u32 = 34;
pub const OP_PLUS: u32 = 35;
pub const OP_MINPLUS: u32 = 36;
pub const OP_QUERY: u32 = 37;
pub const OP_MINQUERY: u32 = 38;
pub const OP_UPTO: u32 = 39;
pub const OP_MINUPTO: u32 = 40;
pub const OP_EXACT: u32 = 41;
pub const OP_POSSTAR: u32 = 42;
pub const OP_POSPLUS: u32 = 43;
pub const OP_POSQUERY: u32 = 44;
pub const OP_POSUPTO: u32 = 45;
pub const OP_STARI: u32 = 46;
pub const OP_MINSTARI: u32 = 47;
pub const OP_PLUSI: u32 = 48;
pub const OP_MINPLUSI: u32 = 49;
pub const OP_QUERYI: u32 = 50;
pub const OP_MINQUERYI: u32 = 51;
pub const OP_UPTOI: u32 = 52;
pub const OP_MINUPTOI: u32 = 53;
pub const OP_EXACTI: u32 = 54;
pub const OP_POSSTARI: u32 = 55;
pub const OP_POSPLUSI: u32 = 56;
pub const OP_POSQUERYI: u32 = 57;
pub const OP_POSUPTOI: u32 = 58;
pub const OP_NOTSTAR: u32 = 59;
pub const OP_NOTMINSTAR: u32 = 60;
pub const OP_NOTPLUS: u32 = 61;
pub const OP_NOTMINPLUS: u32 = 62;
pub const OP_NOTQUERY: u32 = 63;
pub const OP_NOTMINQUERY: u32 = 64;
pub const OP_NOTUPTO: u32 = 65;
pub const OP_NOTMINUPTO: u32 = 66;
pub const OP_NOTEXACT: u32 = 67;
pub const OP_NOTPOSSTAR: u32 = 68;
pub const OP_NOTPOSPLUS: u32 = 69;
pub const OP_NOTPOSQUERY: u32 = 70;
pub const OP_NOTPOSUPTO: u32 = 71;
pub const OP_NOTSTARI: u32 = 72;
pub const OP_NOTMINSTARI: u32 = 73;
pub const OP_NOTPLUSI: u32 = 74;
pub const OP_NOTMINPLUSI: u32 = 75;
pub const OP_NOTQUERYI: u32 = 76;
pub const OP_NOTMINQUERYI: u32 = 77;
pub const OP_NOTUPTOI: u32 = 78;
pub const OP_NOTMINUPTOI: u32 = 79;
pub const OP_NOTEXACTI: u32 = 80;
pub const OP_NOTPOSSTARI: u32 = 81;
pub const OP_NOTPOSPLUSI: u32 = 82;
pub const OP_NOTPOSQUERYI: u32 = 83;
pub const OP_NOTPOSUPTOI: u32 = 84;
pub const OP_TYPESTAR: u32 = 85;
pub const OP_TYPEMINSTAR: u32 = 86;
pub const OP_TYPEPLUS: u32 = 87;
pub const OP_TYPEMINPLUS: u32 = 88;
pub const OP_TYPEQUERY: u32 = 89;
pub const OP_TYPEMINQUERY: u32 = 90;
pub const OP_TYPEUPTO: u32 = 91;
pub const OP_TYPEMINUPTO: u32 = 92;
pub const OP_TYPEEXACT: u32 = 93;
pub const OP_TYPEPOSSTAR: u32 = 94;
pub const OP_TYPEPOSPLUS: u32 = 95;
pub const OP_TYPEPOSQUERY: u32 = 96;
pub const OP_TYPEPOSUPTO: u32 = 97;
pub const OP_CRSTAR: u32 = 98;
pub const OP_CRMINSTAR: u32 = 99;
pub const OP_CRPLUS: u32 = 100;
pub const OP_CRMINPLUS: u32 = 101;
pub const OP_CRQUERY: u32 = 102;
pub const OP_CRMINQUERY: u32 = 103;
pub const OP_CRRANGE: u32 = 104;
pub const OP_CRMINRANGE: u32 = 105;
pub const OP_CRPOSSTAR: u32 = 106;
pub const OP_CRPOSPLUS: u32 = 107;
pub const OP_CRPOSQUERY: u32 = 108;
pub const OP_CRPOSRANGE: u32 = 109;
pub const OP_CLASS: u32 = 110;
pub const OP_NCLASS: u32 = 111;
pub const OP_XCLASS: u32 = 112;
pub const OP_ECLASS: u32 = 113;
pub const OP_REF: u32 = 114;
pub const OP_REFI: u32 = 115;
pub const OP_DNREF: u32 = 116;
pub const OP_DNREFI: u32 = 117;
pub const OP_RECURSE: u32 = 118;
pub const OP_CALLOUT: u32 = 119;
pub const OP_CALLOUT_STR: u32 = 120;
pub const OP_ALT: u32 = 121;
pub const OP_KET: u32 = 122;
pub const OP_KETRMAX: u32 = 123;
pub const OP_KETRMIN: u32 = 124;
pub const OP_KETRPOS: u32 = 125;
pub const OP_REVERSE: u32 = 126;
pub const OP_VREVERSE: u32 = 127;
pub const OP_ASSERT: u32 = 128;
pub const OP_ASSERT_NOT: u32 = 129;
pub const OP_ASSERTBACK: u32 = 130;
pub const OP_ASSERTBACK_NOT: u32 = 131;
pub const OP_ASSERT_NA: u32 = 132;
pub const OP_ASSERTBACK_NA: u32 = 133;
pub const OP_ASSERT_SCS: u32 = 134;
pub const OP_ONCE: u32 = 135;
pub const OP_SCRIPT_RUN: u32 = 136;
pub const OP_BRA: u32 = 137;
pub const OP_BRAPOS: u32 = 138;
pub const OP_CBRA: u32 = 139;
pub const OP_CBRAPOS: u32 = 140;
pub const OP_COND: u32 = 141;
pub const OP_SBRA: u32 = 142;
pub const OP_SBRAPOS: u32 = 143;
pub const OP_SCBRA: u32 = 144;
pub const OP_SCBRAPOS: u32 = 145;
pub const OP_SCOND: u32 = 146;
pub const OP_CREF: u32 = 147;
pub const OP_DNCREF: u32 = 148;
pub const OP_RREF: u32 = 149;
pub const OP_DNRREF: u32 = 150;
pub const OP_FALSE: u32 = 151;
pub const OP_TRUE: u32 = 152;
pub const OP_BRAZERO: u32 = 153;
pub const OP_BRAMINZERO: u32 = 154;
pub const OP_BRAPOSZERO: u32 = 155;
pub const OP_MARK: u32 = 156;
pub const OP_PRUNE: u32 = 157;
pub const OP_PRUNE_ARG: u32 = 158;
pub const OP_SKIP: u32 = 159;
pub const OP_SKIP_ARG: u32 = 160;
pub const OP_THEN: u32 = 161;
pub const OP_THEN_ARG: u32 = 162;
pub const OP_COMMIT: u32 = 163;
pub const OP_COMMIT_ARG: u32 = 164;
pub const OP_FAIL: u32 = 165;
pub const OP_ACCEPT: u32 = 166;
pub const OP_ASSERT_ACCEPT: u32 = 167;
pub const OP_CLOSE: u32 = 168;
pub const OP_SKIPZERO: u32 = 169;
pub const OP_DEFINE: u32 = 170;
pub const OP_NOT_UCP_WORD_BOUNDARY: u32 = 171;
pub const OP_UCP_WORD_BOUNDARY: u32 = 172;
pub const OP_TABLE_LENGTH: usize = 173;

pub const FIRST_AUTOTAB_OP: u32 = OP_NOT_DIGIT;
pub const LAST_AUTOTAB_LEFT_OP: u32 = OP_EXTUNI;
pub const LAST_AUTOTAB_RIGHT_OP: u32 = OP_DOLLM;

pub const RREF_ANY: u32 = 0xffff;
pub const REFI_FLAG_CASELESS_RESTRICT: u32 = 0x1;
pub const REFI_FLAG_TURKISH_CASING: u32 = 0x2;

/* ---------------- Structures ---------------- */

pub type MallocFn = Option<unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void>;
pub type FreeFn = Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct pcre2_memctl {
    pub malloc: MallocFn,
    pub free: FreeFn,
    pub memory_data: *mut c_void,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct open_capitem {
    pub next: *mut open_capitem,
    pub number: u16,
    pub assert_depth: u16,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ucp_type_table {
    pub name_offset: u16,
    pub type_: u16,
    pub value: u16,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ucd_record {
    pub script: u8,
    pub chartype: u8,
    pub gbprop: u8,
    pub caseset: u8,
    pub other_case: i32,
    pub scriptx_bidiclass: u16,
    pub bprops: u16,
}

pub const UCD_BLOCK_SIZE: usize = 128;
pub const UCD_SCRIPTX_MASK: u16 = 0x3ff;
pub const UCD_BIDICLASS_SHIFT: u16 = 11;
pub const UCD_BPROPS_MASK: u16 = 0xfff;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct pcre2_serialized_data {
    pub magic: u32,
    pub version: u32,
    pub config: u32,
    pub number_of_codes: i32,
}

/* Public callout blocks */

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

pub type CalloutFn = Option<unsafe extern "C" fn(*mut pcre2_callout_block, *mut c_void) -> c_int>;
pub type SubstCalloutFn =
    Option<unsafe extern "C" fn(*mut pcre2_substitute_callout_block, *mut c_void) -> c_int>;
pub type SubstCaseCalloutFn = Option<
    unsafe extern "C" fn(
        PCRE2_SPTR,
        PCRE2_SIZE,
        *mut PCRE2_UCHAR,
        PCRE2_SIZE,
        c_int,
        *mut c_void,
    ) -> PCRE2_SIZE,
>;
pub type StackGuardFn = Option<unsafe extern "C" fn(u32, *mut c_void) -> c_int>;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct pcre2_real_general_context {
    pub memctl: pcre2_memctl,
}

#[repr(C)]
#[derive(Copy, Clone)]
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
#[derive(Copy, Clone)]
pub struct pcre2_real_match_context {
    pub memctl: pcre2_memctl,
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
#[derive(Copy, Clone)]
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
    pub ovector: [PCRE2_SIZE; 131072],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct recurse_check {
    pub prev: *mut recurse_check,
    pub group: PCRE2_SPTR,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct parsed_recurse_check {
    pub prev: *mut parsed_recurse_check,
    pub groupptr: *mut u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct recurse_cache {
    pub group: PCRE2_SPTR,
    pub groupnumber: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct branch_chain {
    pub outer: *mut branch_chain,
    pub current_branch: *mut PCRE2_UCHAR,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct named_group {
    pub name: PCRE2_SPTR,
    pub number: u32,
    pub length: u16,
    pub hash_dup: u16,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct compile_data {
    pub next: *mut compile_data,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct class_ranges {
    pub header: compile_data,
    pub char_lists_size: usize,
    pub char_lists_start: usize,
    pub range_list_size: u16,
    pub char_lists_types: u16,
    /* Followed by the list of ranges (start/end pairs) */
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct recurse_arguments {
    pub header: compile_data,
    pub size: usize,
    pub skip_size: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
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
    pub char_lists_size: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct pcre2_real_jit_stack {
    pub memctl: pcre2_memctl,
    pub stack: *mut c_void,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct dfa_recursion_info {
    pub prevrec: *mut dfa_recursion_info,
    pub subject_position: PCRE2_SPTR,
    pub last_used_ptr: PCRE2_SPTR,
    pub group_num: u32,
}

/* ---- heapframe ---- */

#[repr(C)]
#[derive(Copy, Clone)]
pub union hf_oc {
    pub oc: u32,
    pub occu: [PCRE2_UCHAR; 4],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_char_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub charptr: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
    pub c: u32,
    pub oc: hf_oc,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_charnot_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
    pub c: u32,
    pub oc: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_class_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub byte_map_address: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_xclass_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub xclass_data: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_eclass_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub eclass_data: PCRE2_SPTR,
    pub eclass_len: PCRE2_SIZE,
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_type_repeat {
    pub start_eptr: PCRE2_SPTR,
    pub min: u32,
    pub max: u32,
    pub ctype: u32,
    pub propvalue: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_ref_repeat {
    pub start: PCRE2_SPTR,
    pub offset: PCRE2_SIZE,
    pub length: PCRE2_SIZE,
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_op_bra {
    pub frame_type: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_op_brapos {
    pub start_eptr: PCRE2_SPTR,
    pub start_group: PCRE2_SPTR,
    pub frame_type: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_op_recurse {
    pub start_branch: PCRE2_SPTR,
    pub frame_type: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_op_assert_scs {
    pub saved_end_subject: PCRE2_SPTR,
    pub saved_eptr: PCRE2_SPTR,
    pub true_end_extra: PCRE2_SIZE,
    pub saved_moptions: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_op_cond {
    pub start_branch: PCRE2_SPTR,
    pub length: PCRE2_SIZE,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct hf_op_vreverse {
    pub min: u32,
    pub max: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union hf_fields {
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
    pub fields: hf_fields,
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

pub const HEAPFRAME_ALIGNMENT: usize = 8;

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

/* eclass_op_info from pcre2_compile.h */
#[repr(C)]
#[derive(Copy, Clone)]
pub struct eclass_op_info {
    pub code_start: *mut PCRE2_UCHAR,
    pub length: PCRE2_SIZE,
    pub op_single_type: u8,
    pub bits: class_bits_storage,
}

/* ---------------- libc bindings (mirror the C code exactly) ---------------- */

extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    pub fn memmove(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    pub fn memset(d: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    pub fn strlen(s: *const c_char) -> usize;
}

/* ---------------- Helper wrapper for Sync statics ---------------- */

#[repr(transparent)]
pub struct SyncPtr(pub *const c_char);
unsafe impl Sync for SyncPtr {}

/* ---------------- UTF-8 / GET / PUT helpers ---------------- */

#[inline(always)]
pub unsafe fn GET(a: *const u8, n: usize) -> u32 {
    ((*a.add(n) as u32) << 8) | (*a.add(n + 1) as u32)
}

#[inline(always)]
pub unsafe fn PUT(a: *mut u8, n: usize, d: u32) {
    *a.add(n) = (d >> 8) as u8;
    *a.add(n + 1) = (d & 255) as u8;
}

#[inline(always)]
pub unsafe fn GET2(a: *const u8, n: usize) -> u32 {
    ((*a.add(n) as u32) << 8) | (*a.add(n + 1) as u32)
}

#[inline(always)]
pub unsafe fn PUT2(a: *mut u8, n: usize, d: u32) {
    *a.add(n) = (d >> 8) as u8;
    *a.add(n + 1) = (d & 255) as u8;
}

#[inline(always)]
pub fn CU2BYTES(x: usize) -> usize {
    x
}
#[inline(always)]
pub fn BYTES2CU(x: usize) -> usize {
    x
}

#[inline(always)]
pub fn HASUTF8EXTRALEN(c: u32) -> bool {
    c >= 0xc0
}
#[inline(always)]
pub fn HAS_EXTRALEN(c: u32) -> bool {
    c >= 0xc0
}
#[inline(always)]
pub fn NOT_FIRSTCU(c: u32) -> bool {
    (c & 0xc0) == 0x80
}
#[inline(always)]
pub fn GET_EXTRALEN(c: u32) -> u32 {
    unsafe { *crate::tables::_pcre2_utf8_table4.as_ptr().add((c & 0x3f) as usize) as u32 }
}

#[inline(always)]
pub fn MAX_255(_c: u32) -> bool {
    true
}
#[inline(always)]
pub fn CHMAX_255(c: u32) -> bool {
    c <= 255
}
#[inline(always)]
pub unsafe fn TABLE_GET(c: u32, table: *const u8, _default: u32) -> u32 {
    *table.add(c as usize) as u32
}

/* GETUTF8 family: returns the decoded char given first byte c and pointer eptr
(pointing at the first byte). */
#[inline(always)]
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

/* Number of extra bytes consumed by getutf8 for a leading byte c. */
#[inline(always)]
pub fn utf8_extra(c: u32) -> usize {
    if (c & 0x20) == 0 {
        1
    } else if (c & 0x10) == 0 {
        2
    } else if (c & 0x08) == 0 {
        3
    } else if (c & 0x04) == 0 {
        4
    } else {
        5
    }
}

/* GETUTF8INC: c is the already-consumed first byte, eptr points just past it.
Returns (char, new_eptr). */
#[inline(always)]
pub unsafe fn getutf8inc(c: u32, eptr: PCRE2_SPTR) -> (u32, PCRE2_SPTR) {
    if (c & 0x20) == 0 {
        (((c & 0x1f) << 6) | (*eptr as u32 & 0x3f), eptr.add(1))
    } else if (c & 0x10) == 0 {
        (
            ((c & 0x0f) << 12) | ((*eptr as u32 & 0x3f) << 6) | (*eptr.add(1) as u32 & 0x3f),
            eptr.add(2),
        )
    } else if (c & 0x08) == 0 {
        (
            ((c & 0x07) << 18)
                | ((*eptr as u32 & 0x3f) << 12)
                | ((*eptr.add(1) as u32 & 0x3f) << 6)
                | (*eptr.add(2) as u32 & 0x3f),
            eptr.add(3),
        )
    } else if (c & 0x04) == 0 {
        (
            ((c & 0x03) << 24)
                | ((*eptr as u32 & 0x3f) << 18)
                | ((*eptr.add(1) as u32 & 0x3f) << 12)
                | ((*eptr.add(2) as u32 & 0x3f) << 6)
                | (*eptr.add(3) as u32 & 0x3f),
            eptr.add(4),
        )
    } else {
        (
            ((c & 0x01) << 30)
                | ((*eptr as u32 & 0x3f) << 24)
                | ((*eptr.add(1) as u32 & 0x3f) << 18)
                | ((*eptr.add(2) as u32 & 0x3f) << 12)
                | ((*eptr.add(3) as u32 & 0x3f) << 6)
                | (*eptr.add(4) as u32 & 0x3f),
            eptr.add(5),
        )
    }
}

/* PUTCHAR */
#[inline(always)]
pub unsafe fn PUTCHAR(utf: bool, c: u32, p: *mut PCRE2_UCHAR) -> usize {
    if utf && c > MAX_UTF_SINGLE_CU {
        crate::ord2utf::_pcre2_ord2utf_8(c, p) as usize
    } else {
        *p = c as u8;
        1
    }
}

/* UCD access. The C macro REAL_GET_UCD() performs unchecked table lookups; in
the 8-bit library there is no MAX_UTF_CODE_POINT guard (that exists only in the
32-bit library), so with PCRE2_NO_UTF_CHECK and a malformed subject the C reads
past the end of the tables. Raw pointer arithmetic is used here so that the
generated code is the same as the C's rather than a bounds-check panic. */
#[inline(always)]
pub fn GET_UCD(ch: u32) -> &'static ucd_record {
    unsafe {
        let stage1 = crate::ucd_data::_pcre2_ucd_stage1_8.as_ptr();
        let stage2 = crate::ucd_data::_pcre2_ucd_stage2_8.as_ptr();
        let records = crate::ucd_data::_pcre2_ucd_records_8.as_ptr();
        let blk = *stage1.add((ch as usize) / UCD_BLOCK_SIZE) as usize;
        let idx =
            *stage2.add(blk * UCD_BLOCK_SIZE + (ch as usize) % UCD_BLOCK_SIZE) as usize;
        &*records.add(idx)
    }
}

#[inline(always)]
pub fn UCD_SCRIPTX_PROP(p: &ucd_record) -> u32 {
    (p.scriptx_bidiclass & UCD_SCRIPTX_MASK) as u32
}
#[inline(always)]
pub fn UCD_BIDICLASS_PROP(p: &ucd_record) -> u32 {
    (p.scriptx_bidiclass >> UCD_BIDICLASS_SHIFT) as u32
}
#[inline(always)]
pub fn UCD_BPROPS_PROP(p: &ucd_record) -> u32 {
    (p.bprops & UCD_BPROPS_MASK) as u32
}
#[inline(always)]
pub fn UCD_CHARTYPE(ch: u32) -> u32 {
    GET_UCD(ch).chartype as u32
}
#[inline(always)]
pub fn UCD_SCRIPT(ch: u32) -> u32 {
    GET_UCD(ch).script as u32
}
#[inline(always)]
pub fn UCD_CATEGORY(ch: u32) -> u32 {
    unsafe { *crate::tables::_pcre2_ucp_gentype_8.as_ptr().add(UCD_CHARTYPE(ch) as usize) }
}
#[inline(always)]
pub fn UCD_GRAPHBREAK(ch: u32) -> u32 {
    GET_UCD(ch).gbprop as u32
}
#[inline(always)]
pub fn UCD_CASESET(ch: u32) -> u32 {
    GET_UCD(ch).caseset as u32
}
#[inline(always)]
pub fn UCD_OTHERCASE(ch: u32) -> u32 {
    ((ch as i32).wrapping_add(GET_UCD(ch).other_case)) as u32
}
#[inline(always)]
pub fn UCD_SCRIPTX(ch: u32) -> u32 {
    UCD_SCRIPTX_PROP(GET_UCD(ch))
}
#[inline(always)]
pub fn UCD_BPROPS(ch: u32) -> u32 {
    UCD_BPROPS_PROP(GET_UCD(ch))
}
#[inline(always)]
pub fn UCD_BIDICLASS(ch: u32) -> u32 {
    UCD_BIDICLASS_PROP(GET_UCD(ch))
}
#[inline(always)]
pub fn UCD_ANY_I(ch: u32) -> bool {
    (ch | 0x20) == 0x69 || (ch | 1) == 0x0131
}
#[inline(always)]
pub fn UCD_DOTTED_I(ch: u32) -> bool {
    ch == 0x69 || ch == 0x0130
}
#[inline(always)]
pub fn UCD_FOLD_I_TURKISH(ch: u32) -> u32 {
    if ch == 0x0130 {
        0x69
    } else if ch == 0x49 {
        0x0131
    } else {
        ch
    }
}

#[inline(always)]
pub fn MAPBIT(map: &[u32], n: u32) -> u32 {
    map[(n / 32) as usize] & (1u32 << (n % 32))
}

/* Slice-based accessor for script/boolprop sets which are indexed with an
offset into the big global vectors. */
#[inline(always)]
pub fn script_set_bit(offset: usize, n: u32) -> bool {
    unsafe {
        let m = crate::ucd_data::_pcre2_ucd_script_sets_8.as_ptr();
        (*m.add(offset + (n / 32) as usize) & (1u32 << (n % 32))) != 0
    }
}

#[inline(always)]
pub fn boolprop_set_bit(offset: usize, n: u32) -> bool {
    unsafe {
        let m = crate::ucd_data::_pcre2_ucd_boolprop_sets_8.as_ptr();
        (*m.add(offset + (n / 32) as usize) & (1u32 << (n % 32))) != 0
    }
}

#[inline(always)]
pub unsafe fn SETBIT(a: *mut u8, b: u32) {
    *a.add((b >> 3) as usize) |= 1u8 << (b & 0x7);
}

/* CLIST_ALIGN_TO */
#[inline(always)]
pub fn CLIST_ALIGN_TO(base: usize, align: usize) -> usize {
    (base + (align - 1)) & !(align - 1)
}
