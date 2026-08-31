//! Differential-test harness.
//!
//! Loads BOTH the C `libpcre2.so` and the Rust `libpcre2.so` through
//! `libloading` and exposes every exported symbol as a raw function pointer, so
//! that all calls cross a real FFI boundary (exercising the `#[no_mangle]`
//! wrappers exactly as an external consumer would).
#![allow(dead_code, non_snake_case, non_camel_case_types, unused_unsafe)]

pub mod corpus;

use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_uint};
use std::path::PathBuf;

pub type Sz = usize;
pub const PCRE2_ZERO_TERMINATED: Sz = !0usize;
pub const PCRE2_UNSET: Sz = !0usize;

// ---------------------------------------------------------------- constants

pub const PCRE2_ANCHORED: u32 = 0x8000_0000;
pub const PCRE2_NO_UTF_CHECK: u32 = 0x4000_0000;
pub const PCRE2_ENDANCHORED: u32 = 0x2000_0000;

pub const PCRE2_ALLOW_EMPTY_CLASS: u32 = 0x0000_0001;
pub const PCRE2_ALT_BSUX: u32 = 0x0000_0002;
pub const PCRE2_AUTO_CALLOUT: u32 = 0x0000_0004;
pub const PCRE2_CASELESS: u32 = 0x0000_0008;
pub const PCRE2_DOLLAR_ENDONLY: u32 = 0x0000_0010;
pub const PCRE2_DOTALL: u32 = 0x0000_0020;
pub const PCRE2_DUPNAMES: u32 = 0x0000_0040;
pub const PCRE2_EXTENDED: u32 = 0x0000_0080;
pub const PCRE2_FIRSTLINE: u32 = 0x0000_0100;
pub const PCRE2_MATCH_UNSET_BACKREF: u32 = 0x0000_0200;
pub const PCRE2_MULTILINE: u32 = 0x0000_0400;
pub const PCRE2_NEVER_UCP: u32 = 0x0000_0800;
pub const PCRE2_NEVER_UTF: u32 = 0x0000_1000;
pub const PCRE2_NO_AUTO_CAPTURE: u32 = 0x0000_2000;
pub const PCRE2_NO_AUTO_POSSESS: u32 = 0x0000_4000;
pub const PCRE2_NO_DOTSTAR_ANCHOR: u32 = 0x0000_8000;
pub const PCRE2_NO_START_OPTIMIZE: u32 = 0x0001_0000;
pub const PCRE2_UCP: u32 = 0x0002_0000;
pub const PCRE2_UNGREEDY: u32 = 0x0004_0000;
pub const PCRE2_UTF: u32 = 0x0008_0000;
pub const PCRE2_NEVER_BACKSLASH_C: u32 = 0x0010_0000;
pub const PCRE2_ALT_CIRCUMFLEX: u32 = 0x0020_0000;
pub const PCRE2_ALT_VERBNAMES: u32 = 0x0040_0000;
pub const PCRE2_USE_OFFSET_LIMIT: u32 = 0x0080_0000;
pub const PCRE2_EXTENDED_MORE: u32 = 0x0100_0000;
pub const PCRE2_LITERAL: u32 = 0x0200_0000;
pub const PCRE2_MATCH_INVALID_UTF: u32 = 0x0400_0000;
pub const PCRE2_ALT_EXTENDED_CLASS: u32 = 0x0800_0000;

pub const PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES: u32 = 0x0000_0001;
pub const PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL: u32 = 0x0000_0002;
pub const PCRE2_EXTRA_MATCH_WORD: u32 = 0x0000_0004;
pub const PCRE2_EXTRA_MATCH_LINE: u32 = 0x0000_0008;
pub const PCRE2_EXTRA_ESCAPED_CR_IS_LF: u32 = 0x0000_0010;
pub const PCRE2_EXTRA_ALT_BSUX: u32 = 0x0000_0020;
pub const PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK: u32 = 0x0000_0040;
pub const PCRE2_EXTRA_CASELESS_RESTRICT: u32 = 0x0000_0080;
pub const PCRE2_EXTRA_ASCII_BSD: u32 = 0x0000_0100;
pub const PCRE2_EXTRA_ASCII_BSS: u32 = 0x0000_0200;
pub const PCRE2_EXTRA_ASCII_BSW: u32 = 0x0000_0400;
pub const PCRE2_EXTRA_ASCII_POSIX: u32 = 0x0000_0800;
pub const PCRE2_EXTRA_ASCII_DIGIT: u32 = 0x0000_1000;
pub const PCRE2_EXTRA_PYTHON_OCTAL: u32 = 0x0000_2000;
pub const PCRE2_EXTRA_NO_BS0: u32 = 0x0000_4000;
pub const PCRE2_EXTRA_NEVER_CALLOUT: u32 = 0x0000_8000;
pub const PCRE2_EXTRA_TURKISH_CASING: u32 = 0x0001_0000;

pub const PCRE2_NOTBOL: u32 = 0x0000_0001;
pub const PCRE2_NOTEOL: u32 = 0x0000_0002;
pub const PCRE2_NOTEMPTY: u32 = 0x0000_0004;
pub const PCRE2_NOTEMPTY_ATSTART: u32 = 0x0000_0008;
pub const PCRE2_PARTIAL_SOFT: u32 = 0x0000_0010;
pub const PCRE2_PARTIAL_HARD: u32 = 0x0000_0020;
pub const PCRE2_DFA_RESTART: u32 = 0x0000_0040;
pub const PCRE2_DFA_SHORTEST: u32 = 0x0000_0080;
pub const PCRE2_SUBSTITUTE_GLOBAL: u32 = 0x0000_0100;
pub const PCRE2_SUBSTITUTE_EXTENDED: u32 = 0x0000_0200;
pub const PCRE2_SUBSTITUTE_UNSET_EMPTY: u32 = 0x0000_0400;
pub const PCRE2_SUBSTITUTE_UNKNOWN_UNSET: u32 = 0x0000_0800;
pub const PCRE2_SUBSTITUTE_OVERFLOW_LENGTH: u32 = 0x0000_1000;
pub const PCRE2_NO_JIT: u32 = 0x0000_2000;
pub const PCRE2_COPY_MATCHED_SUBJECT: u32 = 0x0000_4000;
pub const PCRE2_SUBSTITUTE_LITERAL: u32 = 0x0000_8000;
pub const PCRE2_SUBSTITUTE_MATCHED: u32 = 0x0001_0000;
pub const PCRE2_SUBSTITUTE_REPLACEMENT_ONLY: u32 = 0x0002_0000;
pub const PCRE2_DISABLE_RECURSELOOP_CHECK: u32 = 0x0004_0000;

pub const PCRE2_CONVERT_UTF: u32 = 0x0000_0001;
pub const PCRE2_CONVERT_NO_UTF_CHECK: u32 = 0x0000_0002;
pub const PCRE2_CONVERT_POSIX_BASIC: u32 = 0x0000_0004;
pub const PCRE2_CONVERT_POSIX_EXTENDED: u32 = 0x0000_0008;
pub const PCRE2_CONVERT_GLOB: u32 = 0x0000_0010;
pub const PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR: u32 = 0x0000_0030;
pub const PCRE2_CONVERT_GLOB_NO_STARSTAR: u32 = 0x0000_0050;

pub const PCRE2_JIT_COMPLETE: u32 = 0x0000_0001;
pub const PCRE2_JIT_PARTIAL_SOFT: u32 = 0x0000_0002;
pub const PCRE2_JIT_PARTIAL_HARD: u32 = 0x0000_0004;
pub const PCRE2_JIT_INVALID_UTF: u32 = 0x0000_0100;
pub const PCRE2_JIT_TEST_ALLOC: u32 = 0x0000_0200;

pub const PCRE2_NEWLINE_CR: u32 = 1;
pub const PCRE2_NEWLINE_LF: u32 = 2;
pub const PCRE2_NEWLINE_CRLF: u32 = 3;
pub const PCRE2_NEWLINE_ANY: u32 = 4;
pub const PCRE2_NEWLINE_ANYCRLF: u32 = 5;
pub const PCRE2_NEWLINE_NUL: u32 = 6;

pub const PCRE2_BSR_UNICODE: u32 = 1;
pub const PCRE2_BSR_ANYCRLF: u32 = 2;

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

// pattern_info requests 0..=26
pub const INFO_MAX: u32 = 26;
pub const INFO_ALLOPTIONS: u32 = 0;
pub const INFO_ARGOPTIONS: u32 = 1;
pub const INFO_BACKREFMAX: u32 = 2;
pub const INFO_BSR: u32 = 3;
pub const INFO_CAPTURECOUNT: u32 = 4;
pub const INFO_FIRSTCODEUNIT: u32 = 5;
pub const INFO_FIRSTCODETYPE: u32 = 6;
pub const INFO_FIRSTBITMAP: u32 = 7;
pub const INFO_HASCRORLF: u32 = 8;
pub const INFO_JCHANGED: u32 = 9;
pub const INFO_JITSIZE: u32 = 10;
pub const INFO_LASTCODEUNIT: u32 = 11;
pub const INFO_LASTCODETYPE: u32 = 12;
pub const INFO_MATCHEMPTY: u32 = 13;
pub const INFO_MATCHLIMIT: u32 = 14;
pub const INFO_MAXLOOKBEHIND: u32 = 15;
pub const INFO_MINLENGTH: u32 = 16;
pub const INFO_NAMECOUNT: u32 = 17;
pub const INFO_NAMEENTRYSIZE: u32 = 18;
pub const INFO_NAMETABLE: u32 = 19;
pub const INFO_NEWLINE: u32 = 20;
pub const INFO_DEPTHLIMIT: u32 = 21;
pub const INFO_SIZE: u32 = 22;
pub const INFO_HASBACKSLASHC: u32 = 23;
pub const INFO_FRAMESIZE: u32 = 24;
pub const INFO_HEAPLIMIT: u32 = 25;
pub const INFO_EXTRAOPTIONS: u32 = 26;

pub const CONFIG_BSR: u32 = 0;
pub const CONFIG_JIT: u32 = 1;
pub const CONFIG_JITTARGET: u32 = 2;
pub const CONFIG_LINKSIZE: u32 = 3;
pub const CONFIG_MATCHLIMIT: u32 = 4;
pub const CONFIG_NEWLINE: u32 = 5;
pub const CONFIG_PARENSLIMIT: u32 = 6;
pub const CONFIG_DEPTHLIMIT: u32 = 7;
pub const CONFIG_STACKRECURSE: u32 = 8;
pub const CONFIG_UNICODE: u32 = 9;
pub const CONFIG_UNICODE_VERSION: u32 = 10;
pub const CONFIG_VERSION: u32 = 11;
pub const CONFIG_HEAPLIMIT: u32 = 12;
pub const CONFIG_NEVER_BACKSLASH_C: u32 = 13;
pub const CONFIG_COMPILED_WIDTHS: u32 = 14;
pub const CONFIG_TABLES_LENGTH: u32 = 15;
pub const CONFIG_EFFECTIVE_LINKSIZE: u32 = 16;

pub const ERR_NOMATCH: c_int = -1;
pub const ERR_PARTIAL: c_int = -2;
pub const ERR_BADDATA: c_int = -29;
pub const ERR_MIXEDTABLES: c_int = -30;
pub const ERR_BADMAGIC: c_int = -31;
pub const ERR_BADMODE: c_int = -32;
pub const ERR_BADOFFSET: c_int = -33;
pub const ERR_BADOPTION: c_int = -34;
pub const ERR_BADREPLACEMENT: c_int = -35;
pub const ERR_BADUTFOFFSET: c_int = -36;
pub const ERR_DFA_BADRESTART: c_int = -38;
pub const ERR_DFA_RECURSE: c_int = -39;
pub const ERR_DFA_UCOND: c_int = -40;
pub const ERR_DFA_UFUNC: c_int = -41;
pub const ERR_DFA_UITEM: c_int = -42;
pub const ERR_DFA_WSSIZE: c_int = -43;
pub const ERR_INTERNAL: c_int = -44;
pub const ERR_JIT_BADOPTION: c_int = -45;
pub const ERR_MATCHLIMIT: c_int = -47;
pub const ERR_NOMEMORY: c_int = -48;
pub const ERR_NOSUBSTRING: c_int = -49;
pub const ERR_NOUNIQUESUBSTRING: c_int = -50;
pub const ERR_NULL: c_int = -51;
pub const ERR_RECURSELOOP: c_int = -52;
pub const ERR_DEPTHLIMIT: c_int = -53;
pub const ERR_UNAVAILABLE: c_int = -54;
pub const ERR_UNSET: c_int = -55;
pub const ERR_BADOFFSETLIMIT: c_int = -56;
pub const ERR_BADREPESCAPE: c_int = -57;
pub const ERR_REPMISSINGBRACE: c_int = -58;
pub const ERR_BADSUBSTITUTION: c_int = -59;
pub const ERR_BADSUBSPATTERN: c_int = -60;
pub const ERR_TOOMANYREPLACE: c_int = -61;
pub const ERR_BADSERIALIZEDDATA: c_int = -62;
pub const ERR_HEAPLIMIT: c_int = -63;
pub const ERR_CONVERT_SYNTAX: c_int = -64;
pub const ERR_INTERNAL_DUPMATCH: c_int = -65;
pub const ERR_DFA_UINVALID_UTF: c_int = -66;
pub const ERR_INVALIDOFFSET: c_int = -67;
pub const ERR_JIT_UNSUPPORTED: c_int = -68;
pub const ERR_REPLACECASE: c_int = -69;
pub const ERR_TOOLARGEREPLACE: c_int = -70;
pub const ERR_DIFFSUBSPATTERN: c_int = -71;
pub const ERR_DIFFSUBSSUBJECT: c_int = -72;
pub const ERR_DIFFSUBSOFFSET: c_int = -73;
pub const ERR_DIFFSUBSOPTIONS: c_int = -74;
pub const ERR_BAD_BACKSLASH_K: c_int = -75;
pub const ERR_PARTIALSUBS: c_int = -76;

pub const TABLES_LENGTH: usize = 1088; // 64*3 + 40*32 ... see pcre2_internal.h

// ---------------------------------------------------------------- structs

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CalloutBlock {
    pub version: u32,
    pub callout_number: u32,
    pub capture_top: u32,
    pub capture_last: u32,
    pub offset_vector: *mut Sz,
    pub mark: *const u8,
    pub subject: *const u8,
    pub subject_length: Sz,
    pub start_match: Sz,
    pub current_position: Sz,
    pub pattern_position: Sz,
    pub next_item_length: Sz,
    pub callout_string_offset: Sz,
    pub callout_string_length: Sz,
    pub callout_string: *const u8,
    pub callout_flags: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CalloutEnumerateBlock {
    pub version: u32,
    pub pattern_position: Sz,
    pub next_item_length: Sz,
    pub callout_number: u32,
    pub callout_string_offset: Sz,
    pub callout_string_length: Sz,
    pub callout_string: *const u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SubstituteCalloutBlock {
    pub version: u32,
    pub input: *const u8,
    pub output: *const u8,
    pub output_offsets: [Sz; 2],
    pub ovector: *mut Sz,
    pub oveccount: u32,
    pub subscount: u32,
}

// ---------------------------------------------------------------- fn types

pub type Code = *mut c_void;
pub type MatchData = *mut c_void;
pub type GContext = *mut c_void;
pub type CContext = *mut c_void;
pub type MContext = *mut c_void;
pub type CvContext = *mut c_void;
pub type JitStack = *mut c_void;

type FnConfig = unsafe extern "C" fn(u32, *mut c_void) -> c_int;
type FnCompile = unsafe extern "C" fn(*const u8, Sz, u32, *mut c_int, *mut Sz, CContext) -> Code;
type FnCodeFree = unsafe extern "C" fn(Code);
type FnCodeCopy = unsafe extern "C" fn(Code) -> Code;
type FnPatternInfo = unsafe extern "C" fn(Code, u32, *mut c_void) -> c_int;
type FnCalloutEnumerate = unsafe extern "C" fn(
    Code,
    Option<unsafe extern "C" fn(*mut CalloutEnumerateBlock, *mut c_void) -> c_int>,
    *mut c_void,
) -> c_int;
type FnMatchDataCreate = unsafe extern "C" fn(u32, GContext) -> MatchData;
type FnMatchDataCreateFromPattern = unsafe extern "C" fn(Code, GContext) -> MatchData;
type FnMatchDataFree = unsafe extern "C" fn(MatchData);
type FnMatch =
    unsafe extern "C" fn(Code, *const u8, Sz, Sz, u32, MatchData, MContext) -> c_int;
type FnDfaMatch = unsafe extern "C" fn(
    Code,
    *const u8,
    Sz,
    Sz,
    u32,
    MatchData,
    MContext,
    *mut c_int,
    Sz,
) -> c_int;
type FnGetMark = unsafe extern "C" fn(MatchData) -> *const u8;
type FnGetSz = unsafe extern "C" fn(MatchData) -> Sz;
type FnGetU32 = unsafe extern "C" fn(MatchData) -> u32;
type FnGetOvecPtr = unsafe extern "C" fn(MatchData) -> *mut Sz;
type FnNextMatch = unsafe extern "C" fn(MatchData, *mut Sz, *mut u32) -> c_int;
type FnSubstitute = unsafe extern "C" fn(
    Code,
    *const u8,
    Sz,
    Sz,
    u32,
    MatchData,
    MContext,
    *const u8,
    Sz,
    *mut u8,
    *mut Sz,
) -> c_int;
type FnSubstringCopyByname =
    unsafe extern "C" fn(MatchData, *const u8, *mut u8, *mut Sz) -> c_int;
type FnSubstringCopyBynumber = unsafe extern "C" fn(MatchData, u32, *mut u8, *mut Sz) -> c_int;
type FnSubstringFree = unsafe extern "C" fn(*mut u8);
type FnSubstringGetByname =
    unsafe extern "C" fn(MatchData, *const u8, *mut *mut u8, *mut Sz) -> c_int;
type FnSubstringGetBynumber =
    unsafe extern "C" fn(MatchData, u32, *mut *mut u8, *mut Sz) -> c_int;
type FnSubstringLengthByname = unsafe extern "C" fn(MatchData, *const u8, *mut Sz) -> c_int;
type FnSubstringLengthBynumber = unsafe extern "C" fn(MatchData, u32, *mut Sz) -> c_int;
type FnSubstringNametableScan =
    unsafe extern "C" fn(Code, *const u8, *mut *const u8, *mut *const u8) -> c_int;
type FnSubstringNumberFromName = unsafe extern "C" fn(Code, *const u8) -> c_int;
type FnSubstringListFree = unsafe extern "C" fn(*mut *mut u8);
type FnSubstringListGet =
    unsafe extern "C" fn(MatchData, *mut *mut *mut u8, *mut *mut Sz) -> c_int;
type FnSerializeEncode =
    unsafe extern "C" fn(*const Code, i32, *mut *mut u8, *mut Sz, GContext) -> i32;
type FnSerializeDecode = unsafe extern "C" fn(*mut Code, i32, *const u8, GContext) -> i32;
type FnSerializeGetNumber = unsafe extern "C" fn(*const u8) -> i32;
type FnSerializeFree = unsafe extern "C" fn(*mut u8);
type FnPatternConvert =
    unsafe extern "C" fn(*const u8, Sz, u32, *mut *mut u8, *mut Sz, CvContext) -> c_int;
type FnConvertedPatternFree = unsafe extern "C" fn(*mut u8);
type FnGetErrorMessage = unsafe extern "C" fn(c_int, *mut u8, Sz) -> c_int;
type FnMaketables = unsafe extern "C" fn(GContext) -> *const u8;
type FnMaketablesFree = unsafe extern "C" fn(GContext, *const u8);
type FnGCtxCreate = unsafe extern "C" fn(
    Option<unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void>,
    Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    *mut c_void,
) -> GContext;
type FnCtxCreate = unsafe extern "C" fn(GContext) -> *mut c_void;
type FnCtxCopy = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
type FnCtxFree = unsafe extern "C" fn(*mut c_void);
type FnSetU32 = unsafe extern "C" fn(*mut c_void, u32) -> c_int;
type FnSetSz = unsafe extern "C" fn(*mut c_void, Sz) -> c_int;
type FnSetTables = unsafe extern "C" fn(CContext, *const u8) -> c_int;
type FnSetCallout = unsafe extern "C" fn(
    MContext,
    Option<unsafe extern "C" fn(*mut CalloutBlock, *mut c_void) -> c_int>,
    *mut c_void,
) -> c_int;
type FnSetSubstCallout = unsafe extern "C" fn(
    MContext,
    Option<unsafe extern "C" fn(*mut SubstituteCalloutBlock, *mut c_void) -> c_int>,
    *mut c_void,
) -> c_int;
type FnSetSubstCaseCallout = unsafe extern "C" fn(
    MContext,
    Option<unsafe extern "C" fn(*const u8, Sz, *mut u8, Sz, c_int, *mut c_void) -> Sz>,
    *mut c_void,
) -> c_int;
type FnSetRecursionGuard = unsafe extern "C" fn(
    CContext,
    Option<unsafe extern "C" fn(u32, *mut c_void) -> c_int>,
    *mut c_void,
) -> c_int;
type FnSetRecursionMemMgmt = unsafe extern "C" fn(
    MContext,
    Option<unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void>,
    Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    *mut c_void,
) -> c_int;
type FnJitCompile = unsafe extern "C" fn(Code, u32) -> c_int;
type FnJitMatch =
    unsafe extern "C" fn(Code, *const u8, Sz, Sz, u32, MatchData, MContext) -> c_int;
type FnJitFreeUnused = unsafe extern "C" fn(GContext);
type FnJitStackCreate = unsafe extern "C" fn(usize, usize, GContext) -> JitStack;
type FnJitStackAssign = unsafe extern "C" fn(
    MContext,
    Option<unsafe extern "C" fn(*mut c_void) -> JitStack>,
    *mut c_void,
);
type FnJitStackFree = unsafe extern "C" fn(JitStack);

// internal helpers
type FnStrlen = unsafe extern "C" fn(*const u8) -> Sz;
type FnStrcmp = unsafe extern "C" fn(*const u8, *const u8) -> c_int;
type FnStrcmpC8 = unsafe extern "C" fn(*const u8, *const c_char) -> c_int;
type FnStrncmp = unsafe extern "C" fn(*const u8, *const u8, usize) -> c_int;
type FnStrncmpC8 = unsafe extern "C" fn(*const u8, *const c_char, usize) -> c_int;
type FnStrcpyC8 = unsafe extern "C" fn(*mut u8, *const c_char) -> Sz;
type FnOrd2utf = unsafe extern "C" fn(u32, *mut u8) -> c_uint;
type FnValidUtf = unsafe extern "C" fn(*const u8, Sz, *mut Sz) -> c_int;
type FnCkdSmul = unsafe extern "C" fn(*mut Sz, c_int, c_int) -> c_int;
type FnIsNewline = unsafe extern "C" fn(*const u8, u32, *const u8, *mut u32, c_int) -> c_int;
type FnWasNewline = unsafe extern "C" fn(*const u8, u32, *const u8, *mut u32, c_int) -> c_int;
type FnScriptRun = unsafe extern "C" fn(*const u8, *const u8, c_int) -> c_int;
type FnExtuni = unsafe extern "C" fn(u32, *const u8, *const u8, *const u8, c_int, *mut c_int) -> *const u8;
type FnFindBracket = unsafe extern "C" fn(*const u8, c_int, c_int) -> *const u8;
type FnXclass = unsafe extern "C" fn(u32, *const u8, *const u8, c_int) -> c_int;
type FnEclass = unsafe extern "C" fn(u32, *const u8, *const u8, *const u8, c_int) -> c_int;
type FnJitGetSize = unsafe extern "C" fn(*mut c_void) -> usize;
type FnJitGetTarget = unsafe extern "C" fn() -> *const c_char;
type FnStudy = unsafe extern "C" fn(Code) -> c_int;
type FnMemctlMalloc = unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void;
type FnJitFree = unsafe extern "C" fn(*mut c_void, *mut c_void);

// ---------------------------------------------------------------- Api

pub struct Api {
    pub name: &'static str,
    _lib: &'static libloading::Library,

    pub config: FnConfig,
    pub compile: FnCompile,
    pub code_free: FnCodeFree,
    pub code_copy: FnCodeCopy,
    pub code_copy_with_tables: FnCodeCopy,
    pub pattern_info: FnPatternInfo,
    pub callout_enumerate: FnCalloutEnumerate,
    pub match_data_create: FnMatchDataCreate,
    pub match_data_create_from_pattern: FnMatchDataCreateFromPattern,
    pub match_data_free: FnMatchDataFree,
    pub do_match: FnMatch,
    pub dfa_match: FnDfaMatch,
    pub get_mark: FnGetMark,
    pub get_match_data_size: FnGetSz,
    pub get_match_data_heapframes_size: FnGetSz,
    pub get_ovector_count: FnGetU32,
    pub get_ovector_pointer: FnGetOvecPtr,
    pub get_startchar: FnGetSz,
    pub next_match: FnNextMatch,
    pub substitute: FnSubstitute,
    pub substring_copy_byname: FnSubstringCopyByname,
    pub substring_copy_bynumber: FnSubstringCopyBynumber,
    pub substring_free: FnSubstringFree,
    pub substring_get_byname: FnSubstringGetByname,
    pub substring_get_bynumber: FnSubstringGetBynumber,
    pub substring_length_byname: FnSubstringLengthByname,
    pub substring_length_bynumber: FnSubstringLengthBynumber,
    pub substring_nametable_scan: FnSubstringNametableScan,
    pub substring_number_from_name: FnSubstringNumberFromName,
    pub substring_list_free: FnSubstringListFree,
    pub substring_list_get: FnSubstringListGet,
    pub serialize_encode: FnSerializeEncode,
    pub serialize_decode: FnSerializeDecode,
    pub serialize_get_number_of_codes: FnSerializeGetNumber,
    pub serialize_free: FnSerializeFree,
    pub pattern_convert: FnPatternConvert,
    pub converted_pattern_free: FnConvertedPatternFree,
    pub get_error_message: FnGetErrorMessage,
    pub maketables: FnMaketables,
    pub maketables_free: FnMaketablesFree,

    pub general_context_create: FnGCtxCreate,
    pub general_context_copy: FnCtxCopy,
    pub general_context_free: FnCtxFree,
    pub compile_context_create: FnCtxCreate,
    pub compile_context_copy: FnCtxCopy,
    pub compile_context_free: FnCtxFree,
    pub match_context_create: FnCtxCreate,
    pub match_context_copy: FnCtxCopy,
    pub match_context_free: FnCtxFree,
    pub convert_context_create: FnCtxCreate,
    pub convert_context_copy: FnCtxCopy,
    pub convert_context_free: FnCtxFree,

    pub set_bsr: FnSetU32,
    pub set_character_tables: FnSetTables,
    pub set_compile_extra_options: FnSetU32,
    pub set_max_pattern_length: FnSetSz,
    pub set_max_pattern_compiled_length: FnSetSz,
    pub set_max_varlookbehind: FnSetU32,
    pub set_newline: FnSetU32,
    pub set_parens_nest_limit: FnSetU32,
    pub set_compile_recursion_guard: FnSetRecursionGuard,
    pub set_optimize: FnSetU32,
    pub set_callout: FnSetCallout,
    pub set_substitute_callout: FnSetSubstCallout,
    pub set_substitute_case_callout: FnSetSubstCaseCallout,
    pub set_depth_limit: FnSetU32,
    pub set_heap_limit: FnSetU32,
    pub set_match_limit: FnSetU32,
    pub set_offset_limit: FnSetSz,
    pub set_recursion_limit: FnSetU32,
    pub set_recursion_memory_management: FnSetRecursionMemMgmt,
    pub set_glob_escape: FnSetU32,
    pub set_glob_separator: FnSetU32,

    pub jit_compile: FnJitCompile,
    pub jit_match: FnJitMatch,
    pub jit_free_unused_memory: FnJitFreeUnused,
    pub jit_stack_create: FnJitStackCreate,
    pub jit_stack_assign: FnJitStackAssign,
    pub jit_stack_free: FnJitStackFree,

    pub p_strlen: FnStrlen,
    pub p_strcmp: FnStrcmp,
    pub p_strcmp_c8: FnStrcmpC8,
    pub p_strncmp: FnStrncmp,
    pub p_strncmp_c8: FnStrncmpC8,
    pub p_strcpy_c8: FnStrcpyC8,
    pub p_ord2utf: FnOrd2utf,
    pub p_valid_utf: FnValidUtf,
    pub p_ckd_smul: FnCkdSmul,
    pub p_is_newline: FnIsNewline,
    pub p_was_newline: FnWasNewline,
    pub p_script_run: FnScriptRun,
    pub p_extuni: FnExtuni,
    pub p_find_bracket: FnFindBracket,
    pub p_xclass: FnXclass,
    pub p_eclass: FnEclass,
    pub p_jit_get_size: FnJitGetSize,
    pub p_jit_get_target: FnJitGetTarget,
    pub p_study: FnStudy,
    pub p_memctl_malloc: FnMemctlMalloc,
    pub p_jit_free: FnJitFree,
    pub p_jit_free_rodata: FnJitFree,
}

macro_rules! sym {
    ($lib:expr, $name:literal) => {{
        let s: libloading::Symbol<_> = unsafe { $lib.get(concat!($name, "\0").as_bytes()) }
            .unwrap_or_else(|e| panic!("missing symbol {}: {}", $name, e));
        unsafe { *s }
    }};
}

impl Api {
    pub fn load(path: &PathBuf, name: &'static str) -> Api {
        let lib: &'static libloading::Library = Box::leak(Box::new(
            unsafe { libloading::Library::new(path) }
                .unwrap_or_else(|e| panic!("cannot load {:?}: {}", path, e)),
        ));
        Api {
            name,
            _lib: lib,
            config: sym!(lib, "pcre2_config_8"),
            compile: sym!(lib, "pcre2_compile_8"),
            code_free: sym!(lib, "pcre2_code_free_8"),
            code_copy: sym!(lib, "pcre2_code_copy_8"),
            code_copy_with_tables: sym!(lib, "pcre2_code_copy_with_tables_8"),
            pattern_info: sym!(lib, "pcre2_pattern_info_8"),
            callout_enumerate: sym!(lib, "pcre2_callout_enumerate_8"),
            match_data_create: sym!(lib, "pcre2_match_data_create_8"),
            match_data_create_from_pattern: sym!(lib, "pcre2_match_data_create_from_pattern_8"),
            match_data_free: sym!(lib, "pcre2_match_data_free_8"),
            do_match: sym!(lib, "pcre2_match_8"),
            dfa_match: sym!(lib, "pcre2_dfa_match_8"),
            get_mark: sym!(lib, "pcre2_get_mark_8"),
            get_match_data_size: sym!(lib, "pcre2_get_match_data_size_8"),
            get_match_data_heapframes_size: sym!(lib, "pcre2_get_match_data_heapframes_size_8"),
            get_ovector_count: sym!(lib, "pcre2_get_ovector_count_8"),
            get_ovector_pointer: sym!(lib, "pcre2_get_ovector_pointer_8"),
            get_startchar: sym!(lib, "pcre2_get_startchar_8"),
            next_match: sym!(lib, "pcre2_next_match_8"),
            substitute: sym!(lib, "pcre2_substitute_8"),
            substring_copy_byname: sym!(lib, "pcre2_substring_copy_byname_8"),
            substring_copy_bynumber: sym!(lib, "pcre2_substring_copy_bynumber_8"),
            substring_free: sym!(lib, "pcre2_substring_free_8"),
            substring_get_byname: sym!(lib, "pcre2_substring_get_byname_8"),
            substring_get_bynumber: sym!(lib, "pcre2_substring_get_bynumber_8"),
            substring_length_byname: sym!(lib, "pcre2_substring_length_byname_8"),
            substring_length_bynumber: sym!(lib, "pcre2_substring_length_bynumber_8"),
            substring_nametable_scan: sym!(lib, "pcre2_substring_nametable_scan_8"),
            substring_number_from_name: sym!(lib, "pcre2_substring_number_from_name_8"),
            substring_list_free: sym!(lib, "pcre2_substring_list_free_8"),
            substring_list_get: sym!(lib, "pcre2_substring_list_get_8"),
            serialize_encode: sym!(lib, "pcre2_serialize_encode_8"),
            serialize_decode: sym!(lib, "pcre2_serialize_decode_8"),
            serialize_get_number_of_codes: sym!(lib, "pcre2_serialize_get_number_of_codes_8"),
            serialize_free: sym!(lib, "pcre2_serialize_free_8"),
            pattern_convert: sym!(lib, "pcre2_pattern_convert_8"),
            converted_pattern_free: sym!(lib, "pcre2_converted_pattern_free_8"),
            get_error_message: sym!(lib, "pcre2_get_error_message_8"),
            maketables: sym!(lib, "pcre2_maketables_8"),
            maketables_free: sym!(lib, "pcre2_maketables_free_8"),

            general_context_create: sym!(lib, "pcre2_general_context_create_8"),
            general_context_copy: sym!(lib, "pcre2_general_context_copy_8"),
            general_context_free: sym!(lib, "pcre2_general_context_free_8"),
            compile_context_create: sym!(lib, "pcre2_compile_context_create_8"),
            compile_context_copy: sym!(lib, "pcre2_compile_context_copy_8"),
            compile_context_free: sym!(lib, "pcre2_compile_context_free_8"),
            match_context_create: sym!(lib, "pcre2_match_context_create_8"),
            match_context_copy: sym!(lib, "pcre2_match_context_copy_8"),
            match_context_free: sym!(lib, "pcre2_match_context_free_8"),
            convert_context_create: sym!(lib, "pcre2_convert_context_create_8"),
            convert_context_copy: sym!(lib, "pcre2_convert_context_copy_8"),
            convert_context_free: sym!(lib, "pcre2_convert_context_free_8"),

            set_bsr: sym!(lib, "pcre2_set_bsr_8"),
            set_character_tables: sym!(lib, "pcre2_set_character_tables_8"),
            set_compile_extra_options: sym!(lib, "pcre2_set_compile_extra_options_8"),
            set_max_pattern_length: sym!(lib, "pcre2_set_max_pattern_length_8"),
            set_max_pattern_compiled_length: sym!(lib, "pcre2_set_max_pattern_compiled_length_8"),
            set_max_varlookbehind: sym!(lib, "pcre2_set_max_varlookbehind_8"),
            set_newline: sym!(lib, "pcre2_set_newline_8"),
            set_parens_nest_limit: sym!(lib, "pcre2_set_parens_nest_limit_8"),
            set_compile_recursion_guard: sym!(lib, "pcre2_set_compile_recursion_guard_8"),
            set_optimize: sym!(lib, "pcre2_set_optimize_8"),
            set_callout: sym!(lib, "pcre2_set_callout_8"),
            set_substitute_callout: sym!(lib, "pcre2_set_substitute_callout_8"),
            set_substitute_case_callout: sym!(lib, "pcre2_set_substitute_case_callout_8"),
            set_depth_limit: sym!(lib, "pcre2_set_depth_limit_8"),
            set_heap_limit: sym!(lib, "pcre2_set_heap_limit_8"),
            set_match_limit: sym!(lib, "pcre2_set_match_limit_8"),
            set_offset_limit: sym!(lib, "pcre2_set_offset_limit_8"),
            set_recursion_limit: sym!(lib, "pcre2_set_recursion_limit_8"),
            set_recursion_memory_management: sym!(
                lib,
                "pcre2_set_recursion_memory_management_8"
            ),
            set_glob_escape: sym!(lib, "pcre2_set_glob_escape_8"),
            set_glob_separator: sym!(lib, "pcre2_set_glob_separator_8"),

            jit_compile: sym!(lib, "pcre2_jit_compile_8"),
            jit_match: sym!(lib, "pcre2_jit_match_8"),
            jit_free_unused_memory: sym!(lib, "pcre2_jit_free_unused_memory_8"),
            jit_stack_create: sym!(lib, "pcre2_jit_stack_create_8"),
            jit_stack_assign: sym!(lib, "pcre2_jit_stack_assign_8"),
            jit_stack_free: sym!(lib, "pcre2_jit_stack_free_8"),

            p_strlen: sym!(lib, "_pcre2_strlen_8"),
            p_strcmp: sym!(lib, "_pcre2_strcmp_8"),
            p_strcmp_c8: sym!(lib, "_pcre2_strcmp_c8_8"),
            p_strncmp: sym!(lib, "_pcre2_strncmp_8"),
            p_strncmp_c8: sym!(lib, "_pcre2_strncmp_c8_8"),
            p_strcpy_c8: sym!(lib, "_pcre2_strcpy_c8_8"),
            p_ord2utf: sym!(lib, "_pcre2_ord2utf_8"),
            p_valid_utf: sym!(lib, "_pcre2_valid_utf_8"),
            p_ckd_smul: sym!(lib, "_pcre2_ckd_smul_8"),
            p_is_newline: sym!(lib, "_pcre2_is_newline_8"),
            p_was_newline: sym!(lib, "_pcre2_was_newline_8"),
            p_script_run: sym!(lib, "_pcre2_script_run_8"),
            p_extuni: sym!(lib, "_pcre2_extuni_8"),
            p_find_bracket: sym!(lib, "_pcre2_find_bracket_8"),
            p_xclass: sym!(lib, "_pcre2_xclass_8"),
            p_eclass: sym!(lib, "_pcre2_eclass_8"),
            p_jit_get_size: sym!(lib, "_pcre2_jit_get_size_8"),
            p_jit_get_target: sym!(lib, "_pcre2_jit_get_target_8"),
            p_study: sym!(lib, "_pcre2_study_8"),
            p_memctl_malloc: sym!(lib, "_pcre2_memctl_malloc_8"),
            p_jit_free: sym!(lib, "_pcre2_jit_free_8"),
            p_jit_free_rodata: sym!(lib, "_pcre2_jit_free_rodata_8"),
        }
    }

    /// Raw access to an exported data symbol.
    pub fn data(&self, name: &str) -> *const u8 {
        let mut b = name.as_bytes().to_vec();
        b.push(0);
        let s: libloading::Symbol<*const u8> = unsafe { self._lib.get(&b) }
            .unwrap_or_else(|e| panic!("missing data symbol {}: {}", name, e));
        unsafe { s.into_raw().into_raw() as *const u8 }
    }
}

// ---------------------------------------------------------------- loading

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_PCRE2_SO") {
        return PathBuf::from(p);
    }
    crate_root()
        .parent()
        .unwrap()
        .join("c_src/build/libpcre2.so")
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_PCRE2_SO") {
        return PathBuf::from(p);
    }
    let rel = crate_root().join("target/release/libpcre2.so");
    if rel.exists() {
        return rel;
    }
    crate_root().join("target/debug/libpcre2.so")
}

static ONCE: std::sync::OnceLock<(Api, Api)> = std::sync::OnceLock::new();

/// Returns `(c_api, rust_api)`.
pub fn apis() -> &'static (Api, Api) {
    ONCE.get_or_init(|| {
        let c = Api::load(&c_so_path(), "C");
        let r = Api::load(&rust_so_path(), "RUST");
        (c, r)
    })
}

// ---------------------------------------------------------------- rng

/// Deterministic xorshift64* PRNG so every run is reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % (n as u64)) as usize
        }
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

// ---------------------------------------------------------------- helpers

/// Observation log: a byte string describing everything an API call produced.
/// Comparing two logs byte-for-byte is the differential assertion.
#[derive(Default, PartialEq, Eq, Clone)]
pub struct Log(pub Vec<u8>);

impl Log {
    pub fn new() -> Log {
        Log(Vec::new())
    }
    pub fn i(&mut self, v: i64) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self.0.push(b'|');
        self
    }
    pub fn u(&mut self, v: u64) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self.0.push(b'|');
        self
    }
    pub fn b(&mut self, v: &[u8]) -> &mut Self {
        self.0.extend_from_slice(&(v.len() as u64).to_le_bytes());
        self.0.extend_from_slice(v);
        self.0.push(b'|');
        self
    }
    pub fn tag(&mut self, s: &str) -> &mut Self {
        self.0.extend_from_slice(s.as_bytes());
        self.0.push(b'#');
        self
    }
}

impl std::fmt::Debug for Log {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Show a readable-ish rendering: printable bytes plus hex escapes.
        write!(f, "Log[{} bytes] ", self.0.len())?;
        for &c in self.0.iter().take(4096) {
            if c.is_ascii_graphic() || c == b' ' {
                write!(f, "{}", c as char)?;
            } else {
                write!(f, "\\x{:02x}", c)?;
            }
        }
        Ok(())
    }
}

/// Asserts that running `f` against C and against Rust yields identical logs.
#[track_caller]
pub fn diff<F: Fn(&Api) -> Log>(label: &str, f: F) {
    let (c, r) = apis();
    let trace = std::env::var_os("PCRE2_DIFF_TRACE").is_some();
    if trace {
        eprintln!("[diff] C   {label}");
    }
    let lc = f(c);
    if trace {
        eprintln!("[diff] RUST {label}");
    }
    let lr = f(r);
    if lc != lr {
        // Find the first differing byte to make the report useful.
        let n = lc.0.len().min(lr.0.len());
        let mut at = n;
        for i in 0..n {
            if lc.0[i] != lr.0[i] {
                at = i;
                break;
            }
        }
        let lo = at.saturating_sub(48);
        let hi_c = (at + 48).min(lc.0.len());
        let hi_r = (at + 48).min(lr.0.len());
        panic!(
            "DIVERGENCE in {label}\n  first difference at byte {at}\
             \n  C   len={} …{:?}\n  RUST len={} …{:?}",
            lc.0.len(),
            &lc.0[lo..hi_c],
            lr.0.len(),
            &lr.0[lo..hi_r],
        );
    }
}

/// Compares two raw exported data blobs byte for byte.
#[track_caller]
pub fn diff_data(name: &str, len: usize) {
    let (c, r) = apis();
    let pc = c.data(name);
    let pr = r.data(name);
    assert!(!pc.is_null(), "C data symbol {name} null");
    assert!(!pr.is_null(), "RUST data symbol {name} null");
    let sc = unsafe { std::slice::from_raw_parts(pc, len) };
    let sr = unsafe { std::slice::from_raw_parts(pr, len) };
    if sc != sr {
        let at = (0..len).find(|&i| sc[i] != sr[i]).unwrap();
        panic!(
            "DATA DIVERGENCE in {name} at byte {at}: C={:#x} RUST={:#x}\n C:  {:?}\n RUST:{:?}",
            sc[at],
            sr[at],
            &sc[at.saturating_sub(8)..(at + 8).min(len)],
            &sr[at.saturating_sub(8)..(at + 8).min(len)],
        );
    }
}

// ------------------------------------------------------- high-level logging

/// Logs *every* `PCRE2_INFO_*` request for a compiled code, including the
/// variable-size outputs (name table, first-code-unit bitmap).
pub unsafe fn log_all_info(api: &Api, code: Code, l: &mut Log) {
    l.tag("info");
    // Fixed-size uint32 requests.
    for what in [
        INFO_ALLOPTIONS,
        INFO_ARGOPTIONS,
        INFO_BACKREFMAX,
        INFO_BSR,
        INFO_CAPTURECOUNT,
        INFO_FIRSTCODEUNIT,
        INFO_FIRSTCODETYPE,
        INFO_HASCRORLF,
        INFO_JCHANGED,
        INFO_LASTCODEUNIT,
        INFO_LASTCODETYPE,
        INFO_MATCHEMPTY,
        INFO_MATCHLIMIT,
        INFO_MAXLOOKBEHIND,
        INFO_MINLENGTH,
        INFO_NAMECOUNT,
        INFO_NAMEENTRYSIZE,
        INFO_NEWLINE,
        INFO_DEPTHLIMIT,
        INFO_HASBACKSLASHC,
        INFO_HEAPLIMIT,
        INFO_EXTRAOPTIONS,
    ] {
        let mut v: u32 = 0xDEAD_BEEF;
        let rc = (api.pattern_info)(code, what, &mut v as *mut u32 as *mut c_void);
        l.u(what as u64).i(rc as i64).u(v as u64);
    }
    // PCRE2_SIZE requests (pcre2_pattern_info.c lines 97-99).
    for what in [INFO_SIZE, INFO_JITSIZE, INFO_FRAMESIZE] {
        let mut v: Sz = 0xDEAD;
        let rc = (api.pattern_info)(code, what, &mut v as *mut Sz as *mut c_void);
        l.u(what as u64).i(rc as i64).u(v as u64);
    }
    // First-code-unit bitmap (32 bytes) or NULL.
    let mut bm: *const u8 = std::ptr::null();
    let rc = (api.pattern_info)(code, INFO_FIRSTBITMAP, &mut bm as *mut _ as *mut c_void);
    l.tag("bitmap").i(rc as i64).i(bm.is_null() as i64);
    if !bm.is_null() {
        l.b(std::slice::from_raw_parts(bm, 32));
    }
    // Name table.
    let mut nc: u32 = 0;
    let mut nes: u32 = 0;
    let mut nt: *const u8 = std::ptr::null();
    (api.pattern_info)(code, INFO_NAMECOUNT, &mut nc as *mut _ as *mut c_void);
    (api.pattern_info)(code, INFO_NAMEENTRYSIZE, &mut nes as *mut _ as *mut c_void);
    let rc = (api.pattern_info)(code, INFO_NAMETABLE, &mut nt as *mut _ as *mut c_void);
    l.tag("nametable").i(rc as i64).u(nc as u64).u(nes as u64);
    if !nt.is_null() && nc > 0 && nes > 0 {
        l.b(std::slice::from_raw_parts(
            nt,
            (nc as usize) * (nes as usize),
        ));
    }
    // Unknown request values, including out-of-range enum values across FFI.
    for what in [27u32, 100, 9999, u32::MAX, u32::MAX - 1] {
        let mut v: Sz = 0;
        let rc = (api.pattern_info)(code, what, &mut v as *mut Sz as *mut c_void);
        l.u(what as u64).i(rc as i64);
    }
    // Size-query form (where == NULL).
    for what in [INFO_SIZE, INFO_NAMETABLE, INFO_FIRSTBITMAP, INFO_CAPTURECOUNT] {
        let rc = (api.pattern_info)(code, what, std::ptr::null_mut());
        l.u(what as u64).i(rc as i64);
    }
}

/// Logs the serialized form of a compiled code — this is the whole compiled
/// bytecode, so a byte-for-byte match here means the compilers agree exactly.
pub unsafe fn log_serialized(api: &Api, code: Code, l: &mut Log) {
    let codes = [code];
    let mut bytes: *mut u8 = std::ptr::null_mut();
    let mut size: Sz = 0;
    let rc = (api.serialize_encode)(
        codes.as_ptr(),
        1,
        &mut bytes,
        &mut size,
        std::ptr::null_mut(),
    );
    l.tag("ser").i(rc as i64).u(size as u64);
    if rc == 1 && !bytes.is_null() {
        // The 32-byte header contains a magic/version/config plus the total
        // size; everything after it is the bytecode itself.
        l.b(std::slice::from_raw_parts(bytes, size));
        (api.serialize_free)(bytes);
    }
}

/// How many ovector entries `pcre2_match` / `pcre2_dfa_match` actually DEFINE
/// for a given return code.
///
/// This matters because the C code leaves the rest of the ovector untouched
/// (see `pcre2_match.c:1045-1053` and the NOMATCH tail at `:8236-8242`), so
/// those slots hold uninitialised heap bytes. Comparing them would compare
/// uninitialised memory — nondeterministic even C-against-C — so the harness
/// compares exactly the defined prefix and nothing more.
///
/// * `rc > 0`  — pairs `0 .. min(capture_count+1, oveccount)` are defined
///   (`rc` pairs are real captures, the remainder up to `capture_count+1` are
///   explicitly filled with `PCRE2_UNSET`).
/// * `rc == 0` — the ovector was too small: all `oveccount` pairs are defined.
/// * `PCRE2_ERROR_PARTIAL` — only pair 0.
/// * anything else — nothing is defined.
fn defined_ovector_entries(rc: c_int, oveccount: u32, capture_count: Option<u32>) -> usize {
    if rc > 0 {
        let pairs = match capture_count {
            Some(cc) => (cc + 1).min(oveccount),
            None => (rc as u32).min(oveccount),
        };
        2 * pairs as usize
    } else if rc == 0 {
        2 * oveccount as usize
    } else if rc == ERR_PARTIAL {
        2
    } else {
        0
    }
}

/// True for the return codes where the match engine reached the point that
/// assigns `match_data->mark`.
pub fn mark_is_defined(rc: c_int) -> bool {
    rc >= 0
        || matches!(
            rc,
            // Reached only from inside the matching engine, i.e. after
            // `match_data->mark` has been assigned
            // (pcre2_match.c:8167, pcre2_dfa_match.c:3690).
            ERR_NOMATCH
                | ERR_PARTIAL
                | ERR_MATCHLIMIT
                | ERR_DEPTHLIMIT
                | ERR_RECURSELOOP
                | ERR_BAD_BACKSLASH_K
                | ERR_DFA_UITEM
                | ERR_DFA_UCOND
                | ERR_DFA_RECURSE
        )
}

/// Logs the complete outcome of a match: rc, ovector count, the DEFINED part of
/// the ovector, start char, mark, and the match-data sizes.
pub unsafe fn log_match_result(api: &Api, md: MatchData, rc: c_int, l: &mut Log) {
    log_match_result_cc(api, md, rc, None, l)
}

/// As `log_match_result`, but derives the capture count from `code` so that the
/// explicitly-`PCRE2_UNSET`-filled tail of the ovector is compared too.
pub unsafe fn log_match_result_full(
    api: &Api,
    code: Code,
    md: MatchData,
    rc: c_int,
    l: &mut Log,
) {
    let mut cc: u32 = 0;
    let ok = !code.is_null()
        && (api.pattern_info)(code, INFO_CAPTURECOUNT, &mut cc as *mut u32 as *mut c_void) == 0;
    log_match_result_cc(api, md, rc, if ok { Some(cc) } else { None }, l)
}

unsafe fn log_match_result_cc(
    api: &Api,
    md: MatchData,
    rc: c_int,
    capture_count: Option<u32>,
    l: &mut Log,
) {
    l.tag("m").i(rc as i64);
    if md.is_null() {
        return;
    }
    let n = (api.get_ovector_count)(md);
    l.u(n as u64);
    let ov = (api.get_ovector_pointer)(md);
    let defined = defined_ovector_entries(rc, n, capture_count);
    l.u(defined as u64);
    if !ov.is_null() {
        for i in 0..defined {
            l.u(*ov.add(i) as u64);
        }
    }
    // startchar is only written on success and on a partial match.
    if rc >= 0 || rc == ERR_PARTIAL {
        l.u((api.get_startchar)(md) as u64);
    }
    // `match_data->mark` is written at the "Fill in fields that are always
    // returned in the match data" point (pcre2_match.c:8167) and again from
    // mb->nomatch_mark at :8211. The EARLY argument-validation returns
    // (NULL/BADOPTION/BADOFFSET/BADMAGIC/BADMODE/BADOFFSETLIMIT/BADUTFOFFSET/
    // UTF8_ERRn/NOMEMORY) return before that point, leaving `mark`
    // uninitialised, so it must not be read for those codes.
    if mark_is_defined(rc) {
        let mk = (api.get_mark)(md);
        l.i(mk.is_null() as i64);
        if !mk.is_null() {
            l.b(&cstr(mk));
        }
    }
    l.u((api.get_match_data_size)(md) as u64);
}

/// Compiles a pattern, logging the error code and offset on failure.
/// Returns the code (possibly null).
pub unsafe fn compile_logged(
    api: &Api,
    pat: &[u8],
    patlen: Sz,
    options: u32,
    ccontext: CContext,
    l: &mut Log,
) -> Code {
    let mut errcode: c_int = 0x7FFF;
    let mut erroffset: Sz = 0xDEAD;
    let p = if pat.is_empty() {
        // Non-null pointer into an empty slice is fine; use a static byte.
        b"\0".as_ptr()
    } else {
        pat.as_ptr()
    };
    let code = (api.compile)(p, patlen, options, &mut errcode, &mut erroffset, ccontext);
    l.tag("c")
        .i(code.is_null() as i64)
        .i(errcode as i64)
        .u(erroffset as u64);
    code
}

// -------------------------------------------- corrupting a compiled pattern

/// `MAGIC_NUMBER` from `pcre2_internal.h:542` ('PCRE').
pub const MAGIC_NUMBER: u32 = 0x5043_5245;
/// `PCRE2_MODE8` from `pcre2_internal.h:503`.
pub const PCRE2_MODE8: u32 = 0x0000_0001;

/// Locates the `magic_number` field inside a `pcre2_real_code` block.
///
/// `pcre2_real_code` (pcre2_intmodedep.h:660) begins with `pcre2_memctl`,
/// `tables`, `executable_jit`, `start_bitmap[32]`, `blocksize` and
/// `code_start`, so the offset is layout-dependent. Rather than hard-coding it,
/// scan the head of the block for the magic value — this is robust and asserts
/// that the value really is there.
pub unsafe fn magic_ptr(code: Code) -> *mut u32 {
    let base = code as *mut u8;
    for off in (0..256usize).step_by(4) {
        let p = base.add(off) as *mut u32;
        if *p == MAGIC_NUMBER {
            return p;
        }
    }
    panic!("magic_number not found in the first 256 bytes of the compiled code");
}

/// `flags` sits four `uint32_t` after `magic_number`
/// (magic_number, compile_options, overall_options, extra_options, flags).
pub unsafe fn flags_ptr(code: Code) -> *mut u32 {
    magic_ptr(code).add(4)
}

/// Runs `f` with the code's magic number corrupted, then restores it.
pub unsafe fn with_bad_magic<R>(code: Code, f: impl FnOnce() -> R) -> R {
    let p = magic_ptr(code);
    let saved = *p;
    *p = 0xDEAD_BEEF;
    let r = f();
    *p = saved;
    r
}

/// Runs `f` with the code-unit-width bit cleared from `flags`, then restores it.
pub unsafe fn with_bad_mode<R>(code: Code, f: impl FnOnce() -> R) -> R {
    let p = flags_ptr(code);
    let saved = *p;
    *p = saved & !PCRE2_MODE8;
    let r = f();
    *p = saved;
    r
}

/// Reads a NUL-terminated C string from a pointer (bounded to avoid runaway).
pub unsafe fn cstr(p: *const u8) -> Vec<u8> {
    if p.is_null() {
        return Vec::new();
    }
    let mut v = Vec::new();
    let mut i = 0usize;
    while i < 1 << 20 {
        let b = *p.add(i);
        if b == 0 {
            break;
        }
        v.push(b);
        i += 1;
    }
    v
}
