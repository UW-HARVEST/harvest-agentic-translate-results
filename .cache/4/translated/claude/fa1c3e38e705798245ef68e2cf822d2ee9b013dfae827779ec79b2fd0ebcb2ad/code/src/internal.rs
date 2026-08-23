// Foundation module: types, constants, structures and cross-module declarations.
// Translated from c_src/src/pcre2_internal.h, pcre2_intmodedep.h, pcre2_compile.h,
// c_src/src/config.h and c_src/include/pcre2.h for
//   PCRE2_CODE_UNIT_WIDTH == 8, SUPPORT_UNICODE, LINK_SIZE == 2, no SUPPORT_JIT.
#![allow(dead_code, non_upper_case_globals, non_camel_case_types, non_snake_case)]

pub use crate::consts_pub::*;
pub use crate::meta::*;
pub use crate::opcodes::*;
pub use crate::ucp::*;

pub use core::ffi::{c_char, c_int, c_uint, c_void};
pub use core::mem::{align_of, offset_of, size_of};

// --------------------------------------------------------------------- types

pub type PCRE2_UCHAR = u8;
pub type PCRE2_SPTR = *const u8;
pub type PCRE2_SIZE = usize;
pub type BOOL = c_int;

pub const TRUE: BOOL = 1;
pub const FALSE: BOOL = 0;

pub const PCRE2_SIZE_MAX: PCRE2_SIZE = PCRE2_SIZE::MAX;
pub const PCRE2_ZERO_TERMINATED: PCRE2_SIZE = !(0 as PCRE2_SIZE);
pub const PCRE2_UNSET: PCRE2_SIZE = !(0 as PCRE2_SIZE);

pub type pcre2_malloc_fn = Option<unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void>;
pub type pcre2_free_fn = Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>;
pub type pcre2_jit_callback = Option<unsafe extern "C" fn(*mut c_void) -> *mut pcre2_real_jit_stack>;
pub type pcre2_callout_fn =
    Option<unsafe extern "C" fn(*mut pcre2_callout_block, *mut c_void) -> c_int>;
pub type pcre2_callout_enumerate_fn =
    Option<unsafe extern "C" fn(*mut pcre2_callout_enumerate_block, *mut c_void) -> c_int>;
pub type pcre2_substitute_callout_fn =
    Option<unsafe extern "C" fn(*mut pcre2_substitute_callout_block, *mut c_void) -> c_int>;
pub type pcre2_substitute_case_callout_fn = Option<
    unsafe extern "C" fn(PCRE2_SPTR, PCRE2_SIZE, *mut PCRE2_UCHAR, PCRE2_SIZE, c_int, *mut c_void)
        -> PCRE2_SIZE,
>;
pub type pcre2_stack_guard_fn = Option<unsafe extern "C" fn(u32, *mut c_void) -> c_int>;

// ------------------------------------------------------------- config.h values

pub const LINK_SIZE: usize = 2;
pub const IMM2_SIZE: usize = 2;
pub const MATCH_LIMIT: u32 = 10000000;
pub const MATCH_LIMIT_DEPTH: u32 = MATCH_LIMIT;
pub const HEAP_LIMIT: u32 = 20000000;
pub const NEWLINE_DEFAULT: u32 = 2;
pub const PARENS_NEST_LIMIT: u32 = 250;
pub const MAX_NAME_COUNT: u32 = 10000;
pub const MAX_NAME_SIZE: u32 = 128;
pub const MAX_VARLOOKBEHIND: u32 = 255;
pub const MAX_PATTERN_SIZE: PCRE2_SIZE = 1 << 16;
pub const PACKAGE_VERSION: &[u8] = b"10.48-DEV\0";

// ------------------------------------------------------- pcre2_internal.h values

pub const NOTACHAR: u32 = 0xffffffff;
pub const MAX_UTF_CODE_POINT: u32 = 0x10ffff;
pub const COMPILE_ERROR_BASE: i32 = 100;
pub const START_FRAMES_SIZE: PCRE2_SIZE = 20480;
pub const DFA_START_RWS_SIZE: usize = 30720;
pub const BSR_DEFAULT: u32 = PCRE2_BSR_UNICODE;

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

pub const PCRE2_MD_COPIED_SUBJECT: u32 = 0x01;

pub const MAGIC_NUMBER: u32 = 0x50435245;

pub const REQ_CU_MAX: PCRE2_SIZE = 5000;
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
// sizeof(PCRE2_UCHAR) == 1 in 8-bit mode
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

pub const ECL_AND: u32 = 1;
pub const ECL_OR: u32 = 2;
pub const ECL_XOR: u32 = 3;
pub const ECL_NOT: u32 = 4;
pub const ECL_XCLASS: u32 = 5;
pub const ECL_ANY: u32 = 6;
pub const ECL_NONE: u32 = 7;

pub const RREF_ANY: u32 = 0xffff;

pub const REFI_FLAG_CASELESS_RESTRICT: u32 = 0x1;
pub const REFI_FLAG_TURKISH_CASING: u32 = 0x2;

pub const FIRST_AUTOTAB_OP: u32 = OP_NOT_DIGIT;
pub const LAST_AUTOTAB_LEFT_OP: u32 = OP_EXTUNI;
pub const LAST_AUTOTAB_RIGHT_OP: u32 = OP_DOLLM;

pub const UCD_BLOCK_SIZE: usize = 128;
pub const UCD_SCRIPTX_MASK: u32 = 0x3ff;
pub const UCD_BIDICLASS_SHIFT: u32 = 11;
pub const UCD_BPROPS_MASK: u32 = 0xfff;

pub const MAX_NON_UTF_CHAR: u32 = 0xff;
pub const MAX_UTF_SINGLE_CU: u32 = 127;
pub const MAX_MARK: u32 = 255;
pub const MAX_UCHAR_VALUE: u32 = 0xff;
pub const LOOKBEHIND_MAX: i32 = 65535;

// From pcre2_compile.h
pub const CLASS_IS_ECLASS: u32 = 0x1;
pub const PC_DIGIT: usize = 7;
pub const PC_GRAPH: usize = 8;
pub const PC_PRINT: usize = 9;
pub const PC_PUNCT: usize = 10;
pub const PC_XDIGIT: usize = 13;
pub const SIZEOFFSET: usize = 2;
pub const NAMED_GROUP_HASH_MASK: u16 = 0x7fff;
pub const NAMED_GROUP_IS_DUPNAME: u16 = 0x8000;

#[inline]
pub unsafe fn NAMED_GROUP_GET_HASH(ng: *const named_group) -> u16 {
    (*ng).hash_dup & NAMED_GROUP_HASH_MASK
}

// ------------------------------------------------------------- character names

pub const CHAR_NUL: u32 = 0x00;
pub const CHAR_HT: u32 = 0x09;
pub const CHAR_VT: u32 = 0x0b;
pub const CHAR_FF: u32 = 0x0c;
pub const CHAR_CR: u32 = 0x0d;
pub const CHAR_LF: u32 = 0x0a;
pub const CHAR_NL: u32 = CHAR_LF;
pub const CHAR_NEL: u32 = 0x85;
pub const CHAR_BS: u32 = 0x08;
pub const CHAR_BEL: u32 = 0x07;
pub const CHAR_ESC: u32 = 0x1b;
pub const CHAR_DEL: u32 = 0x7f;
pub const CHAR_NBSP: u32 = 0xa0;
pub const CHAR_SPACE: u32 = 0x20;
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

// ------------------------------------------------------------------ structures

#[repr(C)]
#[derive(Copy, Clone)]
pub struct pcre2_memctl {
    pub malloc: pcre2_malloc_fn,
    pub free: pcre2_free_fn,
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

#[repr(C)]
#[derive(Copy, Clone)]
pub struct pcre2_serialized_data {
    pub magic: u32,
    pub version: u32,
    pub config: u32,
    pub number_of_codes: i32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct pcre2_real_general_context {
    pub memctl: pcre2_memctl,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct pcre2_real_compile_context {
    pub memctl: pcre2_memctl,
    pub stack_guard: pcre2_stack_guard_fn,
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
    pub callout: pcre2_callout_fn,
    pub callout_data: *mut c_void,
    pub substitute_callout: pcre2_substitute_callout_fn,
    pub substitute_callout_data: *mut c_void,
    pub substitute_case_callout: pcre2_substitute_case_callout_fn,
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
#[derive(Copy, Clone)]
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
    // followed by the list of ranges (start/end pairs)
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

// ------------------------------------------------------------------ heapframe

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
    // fields below must be copied from the previous frame
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

pub const HEAPFRAME_ALIGNMENT: usize = align_of::<heapframe>();

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
    pub callout: pcre2_callout_fn,
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
    pub callout: pcre2_callout_fn,
    pub recursive: *mut dfa_recursion_info,
}

// ----------------------------------------------------- public callout structures

pub const PCRE2_CALLOUT_STARTMATCH: u32 = 0x00000001;
pub const PCRE2_CALLOUT_BACKTRACK: u32 = 0x00000002;

#[repr(C)]
#[derive(Copy, Clone)]
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
#[derive(Copy, Clone)]
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
#[derive(Copy, Clone)]
pub struct pcre2_substitute_callout_block {
    pub version: u32,
    pub input: PCRE2_SPTR,
    pub output: PCRE2_SPTR,
    pub output_offsets: [PCRE2_SIZE; 2],
    pub ovector: *mut PCRE2_SIZE,
    pub oveccount: u32,
    pub subscount: u32,
}

// From pcre2_compile.h
#[repr(C)]
#[derive(Copy, Clone)]
pub struct eclass_op_info {
    pub code_start: *mut PCRE2_UCHAR,
    pub length: PCRE2_SIZE,
    pub op_single_type: u8,
    pub bits: class_bits_storage,
}

// ------------------------------------------------------------------- libc

extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memset(dst: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    pub fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn strlen(s: *const c_char) -> usize;
    pub fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    pub fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn isspace(c: c_int) -> c_int;
    pub fn isdigit(c: c_int) -> c_int;
    pub fn isalnum(c: c_int) -> c_int;
    pub fn isalpha(c: c_int) -> c_int;
    pub fn islower(c: c_int) -> c_int;
    pub fn isupper(c: c_int) -> c_int;
    pub fn ispunct(c: c_int) -> c_int;
    pub fn isgraph(c: c_int) -> c_int;
    pub fn isprint(c: c_int) -> c_int;
    pub fn iscntrl(c: c_int) -> c_int;
    pub fn isxdigit(c: c_int) -> c_int;
    pub fn tolower(c: c_int) -> c_int;
    pub fn toupper(c: c_int) -> c_int;
}

// ------------------------------------------------- library-internal functions
// These are defined in the various modules of this crate with #[unsafe(no_mangle)];
// declaring them here means any module can call them by their linker name without
// having to know which module defines them (exactly as in C).

extern "C" {
    // pcre2_auto_possess.c
    pub fn _pcre2_auto_possessify_8(code: *mut PCRE2_UCHAR, cb: *const compile_block) -> c_int;
    // pcre2_compile.c
    pub fn _pcre2_check_escape_8(
        ptrptr: *mut PCRE2_SPTR,
        ptrend: PCRE2_SPTR,
        chptr: *mut u32,
        errorcodeptr: *mut c_int,
        options: u32,
        xoptions: u32,
        bracount: u32,
        isclass: BOOL,
        cb: *mut compile_block,
    ) -> c_int;
    // pcre2_chkdint.c
    pub fn _pcre2_ckd_smul_8(r: *mut PCRE2_SIZE, a: c_int, b: c_int) -> BOOL;
    // pcre2_extuni.c
    pub fn _pcre2_extuni_8(
        c: u32,
        eptr: PCRE2_SPTR,
        start_subject: PCRE2_SPTR,
        end_subject: PCRE2_SPTR,
        utf: BOOL,
        xcount: *mut c_int,
    ) -> PCRE2_SPTR;
    // pcre2_find_bracket.c
    pub fn _pcre2_find_bracket_8(code: PCRE2_SPTR, utf: BOOL, number: c_int) -> PCRE2_SPTR;
    // pcre2_newline.c
    pub fn _pcre2_is_newline_8(
        ptr: PCRE2_SPTR,
        type_: u32,
        endptr: PCRE2_SPTR,
        lenptr: *mut u32,
        utf: BOOL,
    ) -> BOOL;
    pub fn _pcre2_was_newline_8(
        ptr: PCRE2_SPTR,
        type_: u32,
        startptr: PCRE2_SPTR,
        lenptr: *mut u32,
        utf: BOOL,
    ) -> BOOL;
    // pcre2_jit_compile.c (stubs)
    pub fn _pcre2_jit_free_rodata_8(current: *mut c_void, allocator_data: *mut c_void);
    pub fn _pcre2_jit_free_8(executable_jit: *mut c_void, memctl: *mut pcre2_memctl);
    pub fn _pcre2_jit_get_size_8(executable_jit: *mut c_void) -> usize;
    pub fn _pcre2_jit_get_target_8() -> *const c_char;
    // pcre2_context.c
    pub fn _pcre2_memctl_malloc_8(size: usize, memctl: *mut pcre2_memctl) -> *mut c_void;
    // pcre2_ord2utf.c
    pub fn _pcre2_ord2utf_8(cvalue: u32, buffer: *mut PCRE2_UCHAR) -> c_uint;
    // pcre2_script_run.c
    pub fn _pcre2_script_run_8(ptr: PCRE2_SPTR, endptr: PCRE2_SPTR, utf: BOOL) -> BOOL;
    // pcre2_string_utils.c
    pub fn _pcre2_strcmp_8(str1: PCRE2_SPTR, str2: PCRE2_SPTR) -> c_int;
    pub fn _pcre2_strcmp_c8_8(str1: PCRE2_SPTR, str2: *const c_char) -> c_int;
    pub fn _pcre2_strcpy_c8_8(buffer: *mut PCRE2_UCHAR, vptr: *const c_char) -> PCRE2_SIZE;
    pub fn _pcre2_strlen_8(str: PCRE2_SPTR) -> PCRE2_SIZE;
    pub fn _pcre2_strncmp_8(str1: PCRE2_SPTR, str2: PCRE2_SPTR, len: usize) -> c_int;
    pub fn _pcre2_strncmp_c8_8(str1: PCRE2_SPTR, str2: *const c_char, len: usize) -> c_int;
    // pcre2_study.c
    pub fn _pcre2_study_8(re: *mut pcre2_real_code) -> c_int;
    // pcre2_valid_utf.c
    pub fn _pcre2_valid_utf_8(
        string: PCRE2_SPTR,
        length: PCRE2_SIZE,
        erroroffset: *mut PCRE2_SIZE,
    ) -> c_int;
    // pcre2_xclass.c
    pub fn _pcre2_xclass_8(c: u32, data: PCRE2_SPTR, char_lists_end: *const u8, utf: BOOL) -> BOOL;
    pub fn _pcre2_eclass_8(
        c: u32,
        data_start: PCRE2_SPTR,
        data_end: PCRE2_SPTR,
        char_lists_end: *const u8,
        utf: BOOL,
    ) -> BOOL;
    // pcre2_compile_class.c
    pub fn _pcre2_update_classbits_8(ptype: u32, pdata: u32, negated: BOOL, classbits: *mut u8);
    pub fn _pcre2_compile_class_not_nested_8(
        options: u32,
        xoptions: u32,
        start_ptr: *mut u32,
        pcode: *mut *mut PCRE2_UCHAR,
        negate_class: BOOL,
        has_bitmap: *mut BOOL,
        errorcodeptr: *mut c_int,
        cb: *mut compile_block,
        lengthptr: *mut PCRE2_SIZE,
    ) -> *mut u32;
    pub fn _pcre2_compile_class_nested_8(
        options: u32,
        xoptions: u32,
        pptr: *mut *mut u32,
        pcode: *mut *mut PCRE2_UCHAR,
        errorcodeptr: *mut c_int,
        cb: *mut compile_block,
        lengthptr: *mut PCRE2_SIZE,
    ) -> BOOL;
    // pcre2_compile_cgroup.c
    pub fn _pcre2_compile_get_hash_from_name8(name: PCRE2_SPTR, length: u32) -> u16;
    pub fn _pcre2_compile_find_named_group8(
        name: PCRE2_SPTR,
        length: u32,
        cb: *mut compile_block,
    ) -> *mut named_group;
    pub fn _pcre2_compile_add_name_to_table8(
        cb: *mut compile_block,
        ng: *mut named_group,
        tablecount: u32,
    ) -> u32;
    pub fn _pcre2_compile_find_dupname_details8(
        name: PCRE2_SPTR,
        length: u32,
        indexptr: *mut c_int,
        countptr: *mut c_int,
        errorcodeptr: *mut c_int,
        cb: *mut compile_block,
    ) -> BOOL;
    pub fn _pcre2_compile_parse_scan_substr_args8(
        pptr: *mut u32,
        errorcodeptr: *mut c_int,
        cb: *mut compile_block,
        lengthptr: *mut PCRE2_SIZE,
    ) -> *mut u32;
    pub fn _pcre2_compile_parse_recurse_args8(
        pptr_start: *mut u32,
        offset: PCRE2_SIZE,
        errorcodeptr: *mut c_int,
        cb: *mut compile_block,
    ) -> BOOL;
}

// ----------------------------------------------------------- public functions
// Declared so that modules can call each other exactly as the C code does.

extern "C" {
    pub fn pcre2_config_8(what: u32, where_: *mut c_void) -> c_int;

    pub fn pcre2_general_context_copy_8(
        gcontext: *mut pcre2_real_general_context,
    ) -> *mut pcre2_real_general_context;
    pub fn pcre2_general_context_create_8(
        private_malloc: pcre2_malloc_fn,
        private_free: pcre2_free_fn,
        memory_data: *mut c_void,
    ) -> *mut pcre2_real_general_context;
    pub fn pcre2_general_context_free_8(gcontext: *mut pcre2_real_general_context);

    pub fn pcre2_compile_context_copy_8(
        ccontext: *mut pcre2_real_compile_context,
    ) -> *mut pcre2_real_compile_context;
    pub fn pcre2_compile_context_create_8(
        gcontext: *mut pcre2_real_general_context,
    ) -> *mut pcre2_real_compile_context;
    pub fn pcre2_compile_context_free_8(ccontext: *mut pcre2_real_compile_context);

    pub fn pcre2_match_context_copy_8(
        mcontext: *mut pcre2_real_match_context,
    ) -> *mut pcre2_real_match_context;
    pub fn pcre2_match_context_create_8(
        gcontext: *mut pcre2_real_general_context,
    ) -> *mut pcre2_real_match_context;
    pub fn pcre2_match_context_free_8(mcontext: *mut pcre2_real_match_context);

    pub fn pcre2_convert_context_copy_8(
        ccontext: *mut pcre2_real_convert_context,
    ) -> *mut pcre2_real_convert_context;
    pub fn pcre2_convert_context_create_8(
        gcontext: *mut pcre2_real_general_context,
    ) -> *mut pcre2_real_convert_context;
    pub fn pcre2_convert_context_free_8(ccontext: *mut pcre2_real_convert_context);

    pub fn pcre2_compile_8(
        pattern: PCRE2_SPTR,
        patlen: PCRE2_SIZE,
        options: u32,
        errorptr: *mut c_int,
        erroroffset: *mut PCRE2_SIZE,
        ccontext: *mut pcre2_real_compile_context,
    ) -> *mut pcre2_real_code;
    pub fn pcre2_code_free_8(code: *mut pcre2_real_code);
    pub fn pcre2_code_copy_8(code: *const pcre2_real_code) -> *mut pcre2_real_code;
    pub fn pcre2_code_copy_with_tables_8(code: *const pcre2_real_code) -> *mut pcre2_real_code;

    pub fn pcre2_pattern_info_8(
        code: *const pcre2_real_code,
        what: u32,
        where_: *mut c_void,
    ) -> c_int;
    pub fn pcre2_callout_enumerate_8(
        code: *const pcre2_real_code,
        callback: pcre2_callout_enumerate_fn,
        callout_data: *mut c_void,
    ) -> c_int;

    pub fn pcre2_match_data_create_8(
        oveccount: u32,
        gcontext: *mut pcre2_real_general_context,
    ) -> *mut pcre2_real_match_data;
    pub fn pcre2_match_data_create_from_pattern_8(
        code: *const pcre2_real_code,
        gcontext: *mut pcre2_real_general_context,
    ) -> *mut pcre2_real_match_data;
    pub fn pcre2_match_data_free_8(match_data: *mut pcre2_real_match_data);

    pub fn pcre2_match_8(
        code: *const pcre2_real_code,
        subject: PCRE2_SPTR,
        length: PCRE2_SIZE,
        start_offset: PCRE2_SIZE,
        options: u32,
        match_data: *mut pcre2_real_match_data,
        mcontext: *mut pcre2_real_match_context,
    ) -> c_int;
    pub fn pcre2_dfa_match_8(
        code: *const pcre2_real_code,
        subject: PCRE2_SPTR,
        length: PCRE2_SIZE,
        start_offset: PCRE2_SIZE,
        options: u32,
        match_data: *mut pcre2_real_match_data,
        mcontext: *mut pcre2_real_match_context,
        workspace: *mut c_int,
        wscount: PCRE2_SIZE,
    ) -> c_int;
    pub fn pcre2_jit_match_8(
        code: *const pcre2_real_code,
        subject: PCRE2_SPTR,
        length: PCRE2_SIZE,
        start_offset: PCRE2_SIZE,
        options: u32,
        match_data: *mut pcre2_real_match_data,
        mcontext: *mut pcre2_real_match_context,
    ) -> c_int;
    pub fn pcre2_jit_compile_8(code: *mut pcre2_real_code, options: u32) -> c_int;

    pub fn pcre2_get_mark_8(match_data: *mut pcre2_real_match_data) -> PCRE2_SPTR;
    pub fn pcre2_get_match_data_size_8(match_data: *mut pcre2_real_match_data) -> PCRE2_SIZE;
    pub fn pcre2_get_match_data_heapframes_size_8(
        match_data: *mut pcre2_real_match_data,
    ) -> PCRE2_SIZE;
    pub fn pcre2_get_ovector_count_8(match_data: *mut pcre2_real_match_data) -> u32;
    pub fn pcre2_get_ovector_pointer_8(match_data: *mut pcre2_real_match_data) -> *mut PCRE2_SIZE;
    pub fn pcre2_get_startchar_8(match_data: *mut pcre2_real_match_data) -> PCRE2_SIZE;
    pub fn pcre2_next_match_8(
        match_data: *mut pcre2_real_match_data,
        offset: *mut PCRE2_SIZE,
        options: *mut u32,
    ) -> c_int;

    pub fn pcre2_substring_copy_byname_8(
        match_data: *mut pcre2_real_match_data,
        stringname: PCRE2_SPTR,
        buffer: *mut PCRE2_UCHAR,
        sizeptr: *mut PCRE2_SIZE,
    ) -> c_int;
    pub fn pcre2_substring_copy_bynumber_8(
        match_data: *mut pcre2_real_match_data,
        stringnumber: u32,
        buffer: *mut PCRE2_UCHAR,
        sizeptr: *mut PCRE2_SIZE,
    ) -> c_int;
    pub fn pcre2_substring_free_8(string: *mut PCRE2_UCHAR);
    pub fn pcre2_substring_get_byname_8(
        match_data: *mut pcre2_real_match_data,
        stringname: PCRE2_SPTR,
        stringptr: *mut *mut PCRE2_UCHAR,
        sizeptr: *mut PCRE2_SIZE,
    ) -> c_int;
    pub fn pcre2_substring_get_bynumber_8(
        match_data: *mut pcre2_real_match_data,
        stringnumber: u32,
        stringptr: *mut *mut PCRE2_UCHAR,
        sizeptr: *mut PCRE2_SIZE,
    ) -> c_int;
    pub fn pcre2_substring_length_byname_8(
        match_data: *mut pcre2_real_match_data,
        stringname: PCRE2_SPTR,
        sizeptr: *mut PCRE2_SIZE,
    ) -> c_int;
    pub fn pcre2_substring_length_bynumber_8(
        match_data: *mut pcre2_real_match_data,
        stringnumber: u32,
        sizeptr: *mut PCRE2_SIZE,
    ) -> c_int;
    pub fn pcre2_substring_nametable_scan_8(
        code: *const pcre2_real_code,
        stringname: PCRE2_SPTR,
        firstptr: *mut PCRE2_SPTR,
        lastptr: *mut PCRE2_SPTR,
    ) -> c_int;
    pub fn pcre2_substring_number_from_name_8(
        code: *const pcre2_real_code,
        stringname: PCRE2_SPTR,
    ) -> c_int;
    pub fn pcre2_substring_list_free_8(list: *mut *mut PCRE2_UCHAR);
    pub fn pcre2_substring_list_get_8(
        match_data: *mut pcre2_real_match_data,
        listptr: *mut *mut *mut PCRE2_UCHAR,
        lengthsptr: *mut *mut PCRE2_SIZE,
    ) -> c_int;

    pub fn pcre2_serialize_encode_8(
        codes: *const *const pcre2_real_code,
        number_of_codes: i32,
        serialized_bytes: *mut *mut u8,
        serialized_size: *mut PCRE2_SIZE,
        gcontext: *mut pcre2_real_general_context,
    ) -> i32;
    pub fn pcre2_serialize_decode_8(
        codes: *mut *mut pcre2_real_code,
        number_of_codes: i32,
        bytes: *const u8,
        gcontext: *mut pcre2_real_general_context,
    ) -> i32;
    pub fn pcre2_serialize_get_number_of_codes_8(bytes: *const u8) -> i32;
    pub fn pcre2_serialize_free_8(bytes: *mut u8);

    pub fn pcre2_substitute_8(
        code: *const pcre2_real_code,
        subject: PCRE2_SPTR,
        length: PCRE2_SIZE,
        start_offset: PCRE2_SIZE,
        options: u32,
        match_data: *mut pcre2_real_match_data,
        mcontext: *mut pcre2_real_match_context,
        replacement: PCRE2_SPTR,
        rlength: PCRE2_SIZE,
        buffer: *mut PCRE2_UCHAR,
        blength: *mut PCRE2_SIZE,
    ) -> c_int;

    pub fn pcre2_pattern_convert_8(
        pattern: PCRE2_SPTR,
        plength: PCRE2_SIZE,
        options: u32,
        buffptr: *mut *mut PCRE2_UCHAR,
        bufflenptr: *mut PCRE2_SIZE,
        cconvert: *mut pcre2_real_convert_context,
    ) -> c_int;
    pub fn pcre2_converted_pattern_free_8(converted_pattern: *mut PCRE2_UCHAR);

    pub fn pcre2_get_error_message_8(
        enumber: c_int,
        buffer: *mut PCRE2_UCHAR,
        size: PCRE2_SIZE,
    ) -> c_int;
    pub fn pcre2_maketables_8(gcontext: *mut pcre2_real_general_context) -> *const u8;
    pub fn pcre2_maketables_free_8(gcontext: *mut pcre2_real_general_context, tables: *const u8);
}

// -------------------------------------------------------------- shared tables

pub use crate::pcre2_chartables::_pcre2_default_tables_8;
pub use crate::pcre2_tables::{
    _pcre2_OP_lengths_8, _pcre2_callout_end_delims_8, _pcre2_callout_start_delims_8,
    _pcre2_hspace_list_8, _pcre2_utf8_table1, _pcre2_utf8_table1_size, _pcre2_utf8_table2,
    _pcre2_utf8_table3, _pcre2_utf8_table4, _pcre2_ucp_gbtable_8, _pcre2_ucp_gentype_8,
    _pcre2_utt_8, _pcre2_utt_names_8, _pcre2_utt_size_8, _pcre2_vspace_list_8,
};
pub use crate::pcre2_ucd::{
    _pcre2_ucd_boolprop_sets_8, _pcre2_ucd_caseless_sets_8, _pcre2_ucd_digit_sets_8,
    _pcre2_ucd_nocase_ranges_8, _pcre2_ucd_nocase_ranges_size_8, _pcre2_ucd_records_8,
    _pcre2_ucd_script_sets_8, _pcre2_ucd_stage1_8, _pcre2_ucd_stage2_8,
    _pcre2_ucd_turkish_dotted_i_caseset_8, _pcre2_unicode_version_8,
};

pub use crate::pcre2_compile_class::_pcre2_posix_class_maps8;

pub use crate::pcre2_context::{
    _pcre2_default_compile_context_8, _pcre2_default_convert_context_8,
    _pcre2_default_match_context_8,
};

// --------------------------------------------------------------- UCD accessors

#[inline]
pub unsafe fn GET_UCD(ch: u32) -> *const ucd_record {
    let c = ch as i32;
    let stage1 = _pcre2_ucd_stage1_8[(c / UCD_BLOCK_SIZE as i32) as usize] as usize;
    let idx = _pcre2_ucd_stage2_8[stage1 * UCD_BLOCK_SIZE + (c % UCD_BLOCK_SIZE as i32) as usize];
    _pcre2_ucd_records_8.as_ptr().add(idx as usize)
}

#[inline]
pub fn UCD_SCRIPTX_PROP(prop: *const ucd_record) -> u32 {
    unsafe { ((*prop).scriptx_bidiclass as u32) & UCD_SCRIPTX_MASK }
}

#[inline]
pub fn UCD_BIDICLASS_PROP(prop: *const ucd_record) -> u32 {
    unsafe { ((*prop).scriptx_bidiclass as u32) >> UCD_BIDICLASS_SHIFT }
}

#[inline]
pub fn UCD_BPROPS_PROP(prop: *const ucd_record) -> u32 {
    unsafe { ((*prop).bprops as u32) & UCD_BPROPS_MASK }
}

#[inline]
pub unsafe fn UCD_CHARTYPE(ch: u32) -> u32 {
    (*GET_UCD(ch)).chartype as u32
}

#[inline]
pub unsafe fn UCD_SCRIPT(ch: u32) -> u32 {
    (*GET_UCD(ch)).script as u32
}

#[inline]
pub unsafe fn UCD_CATEGORY(ch: u32) -> u32 {
    _pcre2_ucp_gentype_8[UCD_CHARTYPE(ch) as usize]
}

#[inline]
pub unsafe fn UCD_GRAPHBREAK(ch: u32) -> u32 {
    (*GET_UCD(ch)).gbprop as u32
}

#[inline]
pub unsafe fn UCD_CASESET(ch: u32) -> u32 {
    (*GET_UCD(ch)).caseset as u32
}

#[inline]
pub unsafe fn UCD_OTHERCASE(ch: u32) -> u32 {
    (ch as i32 + (*GET_UCD(ch)).other_case) as u32
}

#[inline]
pub unsafe fn UCD_SCRIPTX(ch: u32) -> u32 {
    UCD_SCRIPTX_PROP(GET_UCD(ch))
}

#[inline]
pub unsafe fn UCD_BPROPS(ch: u32) -> u32 {
    UCD_BPROPS_PROP(GET_UCD(ch))
}

#[inline]
pub unsafe fn UCD_BIDICLASS(ch: u32) -> u32 {
    UCD_BIDICLASS_PROP(GET_UCD(ch))
}

/// match any of the four characters 'i', 'I', U+0130, U+0131
#[inline]
pub fn UCD_ANY_I(ch: u32) -> bool {
    (ch | 0x20u32) == 0x69u32 || (ch | 1u32) == 0x0131u32
}

#[inline]
pub fn UCD_DOTTED_I(ch: u32) -> bool {
    ch == 0x69u32 || ch == 0x0130u32
}

#[inline]
pub fn UCD_FOLD_I_TURKISH(ch: u32) -> u32 {
    if ch == 0x0130u32 {
        0x69u32
    } else if ch == 0x49u32 {
        0x0131u32
    } else {
        ch
    }
}

// ---- Layout assertions: values taken from the C build (gcc, x86_64) ----
const _: () = assert!(size_of::<pcre2_memctl>() == 24);
const _: () = assert!(size_of::<pcre2_real_general_context>() == 24);
const _: () = assert!(size_of::<pcre2_real_compile_context>() == 88);
const _: () = assert!(size_of::<pcre2_real_match_context>() == 96);
const _: () = assert!(size_of::<pcre2_real_convert_context>() == 32);
const _: () = assert!(size_of::<pcre2_real_code>() == 152);
const _: () = assert!(size_of::<pcre2_real_match_data>() == 1048696);
const _: () = assert!(size_of::<heapframe>() == 1048696);
const _: () = assert!(size_of::<match_block>() == 272);
const _: () = assert!(size_of::<dfa_match_block>() == 168);
const _: () = assert!(size_of::<compile_block>() == 360);
const _: () = assert!(size_of::<named_group>() == 16);
const _: () = assert!(size_of::<ucd_record>() == 12);
const _: () = assert!(size_of::<ucp_type_table>() == 6);
const _: () = assert!(size_of::<open_capitem>() == 16);
const _: () = assert!(size_of::<pcre2_callout_block>() == 112);
const _: () = assert!(size_of::<pcre2_callout_enumerate_block>() == 56);
const _: () = assert!(size_of::<pcre2_substitute_callout_block>() == 56);
const _: () = assert!(size_of::<class_ranges>() == 32);
const _: () = assert!(size_of::<recurse_arguments>() == 24);
const _: () = assert!(size_of::<compile_data>() == 8);
const _: () = assert!(size_of::<eclass_op_info>() == 56);
const _: () = assert!(size_of::<dfa_recursion_info>() == 32);
const _: () = assert!(size_of::<pcre2_serialized_data>() == 16);
const _: () = assert!(size_of::<pcre2_real_jit_stack>() == 32);
const _: () = assert!(size_of::<class_bits_storage>() == 32);
const _: () = assert!(size_of::<branch_chain>() == 16);
const _: () = assert!(offset_of!(pcre2_real_code, tables) == 24);
const _: () = assert!(offset_of!(pcre2_real_code, start_bitmap) == 40);
const _: () = assert!(offset_of!(pcre2_real_code, blocksize) == 72);
const _: () = assert!(offset_of!(pcre2_real_code, optimization_flags) == 144);
const _: () = assert!(offset_of!(pcre2_real_match_data, ovector) == 120);
const _: () = assert!(offset_of!(pcre2_real_match_data, rc) == 112);
const _: () = assert!(offset_of!(pcre2_real_match_data, matchedby) == 104);
const _: () = assert!(offset_of!(heapframe, fields) == 32);
const _: () = assert!(offset_of!(heapframe, eptr) == 64);
const _: () = assert!(offset_of!(heapframe, ovector) == 120);
const _: () = assert!(offset_of!(heapframe, offset_top) == 112);
const _: () = assert!(offset_of!(match_block, nl) == 244);
const _: () = assert!(offset_of!(match_block, callout) == 264);
const _: () = assert!(offset_of!(match_block, start_offset) == 88);
const _: () = assert!(offset_of!(match_block, partial) == 104);
const _: () = assert!(offset_of!(dfa_match_block, recursive) == 160);
const _: () = assert!(offset_of!(dfa_match_block, nl) == 128);
const _: () = assert!(offset_of!(dfa_match_block, bsr_convention) == 132);
const _: () = assert!(offset_of!(compile_block, classbits) == 176);
const _: () = assert!(offset_of!(compile_block, char_lists_size) == 352);
const _: () = assert!(offset_of!(compile_block, nl) == 288);
const _: () = assert!(offset_of!(compile_block, class_op_used) == 292);
const _: () = assert!(HEAPFRAME_ALIGNMENT == 8);
