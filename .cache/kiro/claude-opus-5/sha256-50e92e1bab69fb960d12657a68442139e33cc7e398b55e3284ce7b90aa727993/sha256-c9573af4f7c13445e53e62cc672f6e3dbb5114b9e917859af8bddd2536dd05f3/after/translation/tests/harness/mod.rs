//! Differential-test harness: loads BOTH the C `libpcre2.so` and the Rust
//! `libpcre2.so` through `libloading` and exposes every exported symbol as a
//! typed function pointer. No Rust function is ever called directly.
#![allow(dead_code, non_snake_case, non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::sync::OnceLock;

pub type Sz = usize;
pub type Code = *mut c_void;
pub type MatchData = *mut c_void;
pub type Ctx = *mut c_void;

pub const PCRE2_UNSET: Sz = Sz::MAX;
pub const PCRE2_ZERO_TERMINATED: Sz = Sz::MAX;

// ---------------------------------------------------------------- option bits
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

// --------------------------------------------------------------- error codes
pub const PCRE2_ERROR_NOMATCH: i32 = -1;
pub const PCRE2_ERROR_PARTIAL: i32 = -2;
pub const PCRE2_ERROR_UTF8_ERR1: i32 = -3;
pub const PCRE2_ERROR_UTF8_ERR2: i32 = -4;
pub const PCRE2_ERROR_UTF8_ERR3: i32 = -5;
pub const PCRE2_ERROR_UTF8_ERR4: i32 = -6;
pub const PCRE2_ERROR_UTF8_ERR5: i32 = -7;
pub const PCRE2_ERROR_UTF8_ERR6: i32 = -8;
pub const PCRE2_ERROR_UTF8_ERR7: i32 = -9;
pub const PCRE2_ERROR_UTF8_ERR8: i32 = -10;
pub const PCRE2_ERROR_UTF8_ERR9: i32 = -11;
pub const PCRE2_ERROR_UTF8_ERR10: i32 = -12;
pub const PCRE2_ERROR_UTF8_ERR11: i32 = -13;
pub const PCRE2_ERROR_UTF8_ERR12: i32 = -14;
pub const PCRE2_ERROR_UTF8_ERR13: i32 = -15;
pub const PCRE2_ERROR_UTF8_ERR14: i32 = -16;
pub const PCRE2_ERROR_UTF8_ERR15: i32 = -17;
pub const PCRE2_ERROR_UTF8_ERR16: i32 = -18;
pub const PCRE2_ERROR_UTF8_ERR17: i32 = -19;
pub const PCRE2_ERROR_UTF8_ERR18: i32 = -20;
pub const PCRE2_ERROR_UTF8_ERR19: i32 = -21;
pub const PCRE2_ERROR_UTF8_ERR20: i32 = -22;
pub const PCRE2_ERROR_UTF8_ERR21: i32 = -23;
pub const PCRE2_ERROR_UTF16_ERR1: i32 = -24;
pub const PCRE2_ERROR_UTF16_ERR2: i32 = -25;
pub const PCRE2_ERROR_UTF16_ERR3: i32 = -26;
pub const PCRE2_ERROR_UTF32_ERR1: i32 = -27;
pub const PCRE2_ERROR_UTF32_ERR2: i32 = -28;
pub const PCRE2_ERROR_BADDATA: i32 = -29;
pub const PCRE2_ERROR_MIXEDTABLES: i32 = -30;
pub const PCRE2_ERROR_BADMAGIC: i32 = -31;
pub const PCRE2_ERROR_BADMODE: i32 = -32;
pub const PCRE2_ERROR_BADOFFSET: i32 = -33;
pub const PCRE2_ERROR_BADOPTION: i32 = -34;
pub const PCRE2_ERROR_BADREPLACEMENT: i32 = -35;
pub const PCRE2_ERROR_BADUTFOFFSET: i32 = -36;
pub const PCRE2_ERROR_CALLOUT: i32 = -37;
pub const PCRE2_ERROR_DFA_BADRESTART: i32 = -38;
pub const PCRE2_ERROR_DFA_RECURSE: i32 = -39;
pub const PCRE2_ERROR_DFA_UCOND: i32 = -40;
pub const PCRE2_ERROR_DFA_UFUNC: i32 = -41;
pub const PCRE2_ERROR_DFA_UITEM: i32 = -42;
pub const PCRE2_ERROR_DFA_WSSIZE: i32 = -43;
pub const PCRE2_ERROR_INTERNAL: i32 = -44;
pub const PCRE2_ERROR_JIT_BADOPTION: i32 = -45;
pub const PCRE2_ERROR_JIT_STACKLIMIT: i32 = -46;
pub const PCRE2_ERROR_MATCHLIMIT: i32 = -47;
pub const PCRE2_ERROR_NOMEMORY: i32 = -48;
pub const PCRE2_ERROR_NOSUBSTRING: i32 = -49;
pub const PCRE2_ERROR_NOUNIQUESUBSTRING: i32 = -50;
pub const PCRE2_ERROR_NULL: i32 = -51;
pub const PCRE2_ERROR_RECURSELOOP: i32 = -52;
pub const PCRE2_ERROR_DEPTHLIMIT: i32 = -53;
pub const PCRE2_ERROR_RECURSIONLIMIT: i32 = -53;
pub const PCRE2_ERROR_UNAVAILABLE: i32 = -54;
pub const PCRE2_ERROR_UNSET: i32 = -55;
pub const PCRE2_ERROR_BADOFFSETLIMIT: i32 = -56;
pub const PCRE2_ERROR_BADREPESCAPE: i32 = -57;
pub const PCRE2_ERROR_REPMISSINGBRACE: i32 = -58;
pub const PCRE2_ERROR_BADSUBSTITUTION: i32 = -59;
pub const PCRE2_ERROR_BADSUBSPATTERN: i32 = -60;
pub const PCRE2_ERROR_TOOMANYREPLACE: i32 = -61;
pub const PCRE2_ERROR_BADSERIALIZEDDATA: i32 = -62;
pub const PCRE2_ERROR_HEAPLIMIT: i32 = -63;
pub const PCRE2_ERROR_CONVERT_SYNTAX: i32 = -64;
pub const PCRE2_ERROR_INTERNAL_DUPMATCH: i32 = -65;
pub const PCRE2_ERROR_DFA_UINVALID_UTF: i32 = -66;
pub const PCRE2_ERROR_INVALIDOFFSET: i32 = -67;
pub const PCRE2_ERROR_JIT_UNSUPPORTED: i32 = -68;
pub const PCRE2_ERROR_REPLACECASE: i32 = -69;
pub const PCRE2_ERROR_TOOLARGEREPLACE: i32 = -70;
pub const PCRE2_ERROR_DIFFSUBSPATTERN: i32 = -71;
pub const PCRE2_ERROR_DIFFSUBSSUBJECT: i32 = -72;
pub const PCRE2_ERROR_DIFFSUBSOFFSET: i32 = -73;
pub const PCRE2_ERROR_DIFFSUBSOPTIONS: i32 = -74;
pub const PCRE2_ERROR_BAD_BACKSLASH_K: i32 = -75;
pub const PCRE2_ERROR_PARTIALSUBS: i32 = -76;

// --------------------------------------------------------- callout structures
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

// --------------------------------------------------------------- symbol table
macro_rules! api {
    ( $( $field:ident : $ty:ty = $sym:literal ; )* ) => {
        pub struct Api {
            #[allow(unused)]
            lib: Library,
            pub name: &'static str,
            $( pub $field : $ty , )*
        }
        impl Api {
            unsafe fn build(path: &str, name: &'static str) -> Api {
                let lib = unsafe { Library::new(path) }
                    .unwrap_or_else(|e| panic!("cannot load {path}: {e}"));
                $(
                    let $field : $ty = {
                        let s: Symbol<$ty> = unsafe { lib.get($sym) }
                            .unwrap_or_else(|e| panic!("{path}: missing {}: {e}",
                                            String::from_utf8_lossy($sym)));
                        unsafe { *s.into_raw() }
                    };
                )*
                Api { lib, name, $( $field , )* }
            }
        }
        // The loaded library and its symbols are immutable for the process
        // lifetime; PCRE2 itself is thread-safe for these read-only handles.
        unsafe impl Send for Api {}
        unsafe impl Sync for Api {}
    };
}

api! {
    // ---- compile / code
    compile: unsafe extern "C" fn(*const u8, Sz, u32, *mut c_int, *mut Sz, Ctx) -> Code
        = b"pcre2_compile_8\0";
    code_free: unsafe extern "C" fn(Code) = b"pcre2_code_free_8\0";
    code_copy: unsafe extern "C" fn(Code) -> Code = b"pcre2_code_copy_8\0";
    code_copy_with_tables: unsafe extern "C" fn(Code) -> Code
        = b"pcre2_code_copy_with_tables_8\0";
    pattern_info: unsafe extern "C" fn(Code, u32, *mut c_void) -> c_int
        = b"pcre2_pattern_info_8\0";
    callout_enumerate: unsafe extern "C" fn(
        Code,
        Option<unsafe extern "C" fn(*mut CalloutEnumerateBlock, *mut c_void) -> c_int>,
        *mut c_void) -> c_int = b"pcre2_callout_enumerate_8\0";
    config: unsafe extern "C" fn(u32, *mut c_void) -> c_int = b"pcre2_config_8\0";

    // ---- match
    match_data_create: unsafe extern "C" fn(u32, Ctx) -> MatchData
        = b"pcre2_match_data_create_8\0";
    match_data_create_from_pattern: unsafe extern "C" fn(Code, Ctx) -> MatchData
        = b"pcre2_match_data_create_from_pattern_8\0";
    match_data_free: unsafe extern "C" fn(MatchData) = b"pcre2_match_data_free_8\0";
    do_match: unsafe extern "C" fn(Code, *const u8, Sz, Sz, u32, MatchData, Ctx) -> c_int
        = b"pcre2_match_8\0";
    dfa_match: unsafe extern "C" fn(
        Code, *const u8, Sz, Sz, u32, MatchData, Ctx, *mut c_int, Sz) -> c_int
        = b"pcre2_dfa_match_8\0";
    next_match: unsafe extern "C" fn(MatchData, *mut Sz, *mut u32) -> c_int
        = b"pcre2_next_match_8\0";
    get_mark: unsafe extern "C" fn(MatchData) -> *const u8 = b"pcre2_get_mark_8\0";
    get_ovector_count: unsafe extern "C" fn(MatchData) -> u32
        = b"pcre2_get_ovector_count_8\0";
    get_ovector_pointer: unsafe extern "C" fn(MatchData) -> *mut Sz
        = b"pcre2_get_ovector_pointer_8\0";
    get_startchar: unsafe extern "C" fn(MatchData) -> Sz = b"pcre2_get_startchar_8\0";
    get_match_data_size: unsafe extern "C" fn(MatchData) -> Sz
        = b"pcre2_get_match_data_size_8\0";
    get_match_data_heapframes_size: unsafe extern "C" fn(MatchData) -> Sz
        = b"pcre2_get_match_data_heapframes_size_8\0";

    // ---- substring
    substring_copy_byname: unsafe extern "C" fn(MatchData, *const u8, *mut u8, *mut Sz) -> c_int
        = b"pcre2_substring_copy_byname_8\0";
    substring_copy_bynumber: unsafe extern "C" fn(MatchData, u32, *mut u8, *mut Sz) -> c_int
        = b"pcre2_substring_copy_bynumber_8\0";
    substring_free: unsafe extern "C" fn(*mut u8) = b"pcre2_substring_free_8\0";
    substring_get_byname: unsafe extern "C" fn(MatchData, *const u8, *mut *mut u8, *mut Sz) -> c_int
        = b"pcre2_substring_get_byname_8\0";
    substring_get_bynumber: unsafe extern "C" fn(MatchData, u32, *mut *mut u8, *mut Sz) -> c_int
        = b"pcre2_substring_get_bynumber_8\0";
    substring_length_byname: unsafe extern "C" fn(MatchData, *const u8, *mut Sz) -> c_int
        = b"pcre2_substring_length_byname_8\0";
    substring_length_bynumber: unsafe extern "C" fn(MatchData, u32, *mut Sz) -> c_int
        = b"pcre2_substring_length_bynumber_8\0";
    substring_nametable_scan: unsafe extern "C" fn(
        Code, *const u8, *mut *const u8, *mut *const u8) -> c_int
        = b"pcre2_substring_nametable_scan_8\0";
    substring_number_from_name: unsafe extern "C" fn(Code, *const u8) -> c_int
        = b"pcre2_substring_number_from_name_8\0";
    substring_list_free: unsafe extern "C" fn(*mut *mut u8) = b"pcre2_substring_list_free_8\0";
    substring_list_get: unsafe extern "C" fn(MatchData, *mut *mut *mut u8, *mut *mut Sz) -> c_int
        = b"pcre2_substring_list_get_8\0";

    // ---- substitute
    substitute: unsafe extern "C" fn(
        Code, *const u8, Sz, Sz, u32, MatchData, Ctx, *const u8, Sz, *mut u8, *mut Sz) -> c_int
        = b"pcre2_substitute_8\0";

    // ---- serialize
    serialize_encode: unsafe extern "C" fn(*const Code, i32, *mut *mut u8, *mut Sz, Ctx) -> i32
        = b"pcre2_serialize_encode_8\0";
    serialize_decode: unsafe extern "C" fn(*mut Code, i32, *const u8, Ctx) -> i32
        = b"pcre2_serialize_decode_8\0";
    serialize_get_number_of_codes: unsafe extern "C" fn(*const u8) -> i32
        = b"pcre2_serialize_get_number_of_codes_8\0";
    serialize_free: unsafe extern "C" fn(*mut u8) = b"pcre2_serialize_free_8\0";

    // ---- convert
    pattern_convert: unsafe extern "C" fn(
        *const u8, Sz, u32, *mut *mut u8, *mut Sz, Ctx) -> c_int
        = b"pcre2_pattern_convert_8\0";
    converted_pattern_free: unsafe extern "C" fn(*mut u8)
        = b"pcre2_converted_pattern_free_8\0";

    // ---- errors
    get_error_message: unsafe extern "C" fn(c_int, *mut u8, Sz) -> c_int
        = b"pcre2_get_error_message_8\0";

    // ---- contexts
    general_context_create: unsafe extern "C" fn(
        Option<unsafe extern "C" fn(Sz, *mut c_void) -> *mut c_void>,
        Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
        *mut c_void) -> Ctx = b"pcre2_general_context_create_8\0";
    general_context_copy: unsafe extern "C" fn(Ctx) -> Ctx = b"pcre2_general_context_copy_8\0";
    general_context_free: unsafe extern "C" fn(Ctx) = b"pcre2_general_context_free_8\0";
    compile_context_create: unsafe extern "C" fn(Ctx) -> Ctx
        = b"pcre2_compile_context_create_8\0";
    compile_context_copy: unsafe extern "C" fn(Ctx) -> Ctx = b"pcre2_compile_context_copy_8\0";
    compile_context_free: unsafe extern "C" fn(Ctx) = b"pcre2_compile_context_free_8\0";
    match_context_create: unsafe extern "C" fn(Ctx) -> Ctx = b"pcre2_match_context_create_8\0";
    match_context_copy: unsafe extern "C" fn(Ctx) -> Ctx = b"pcre2_match_context_copy_8\0";
    match_context_free: unsafe extern "C" fn(Ctx) = b"pcre2_match_context_free_8\0";
    convert_context_create: unsafe extern "C" fn(Ctx) -> Ctx
        = b"pcre2_convert_context_create_8\0";
    convert_context_copy: unsafe extern "C" fn(Ctx) -> Ctx = b"pcre2_convert_context_copy_8\0";
    convert_context_free: unsafe extern "C" fn(Ctx) = b"pcre2_convert_context_free_8\0";

    set_bsr: unsafe extern "C" fn(Ctx, u32) -> c_int = b"pcre2_set_bsr_8\0";
    set_character_tables: unsafe extern "C" fn(Ctx, *const u8) -> c_int
        = b"pcre2_set_character_tables_8\0";
    set_compile_extra_options: unsafe extern "C" fn(Ctx, u32) -> c_int
        = b"pcre2_set_compile_extra_options_8\0";
    set_compile_recursion_guard: unsafe extern "C" fn(
        Ctx, Option<unsafe extern "C" fn(u32, *mut c_void) -> c_int>, *mut c_void) -> c_int
        = b"pcre2_set_compile_recursion_guard_8\0";
    set_max_pattern_length: unsafe extern "C" fn(Ctx, Sz) -> c_int
        = b"pcre2_set_max_pattern_length_8\0";
    set_max_pattern_compiled_length: unsafe extern "C" fn(Ctx, Sz) -> c_int
        = b"pcre2_set_max_pattern_compiled_length_8\0";
    set_max_varlookbehind: unsafe extern "C" fn(Ctx, u32) -> c_int
        = b"pcre2_set_max_varlookbehind_8\0";
    set_newline: unsafe extern "C" fn(Ctx, u32) -> c_int = b"pcre2_set_newline_8\0";
    set_optimize: unsafe extern "C" fn(Ctx, u32) -> c_int = b"pcre2_set_optimize_8\0";
    set_parens_nest_limit: unsafe extern "C" fn(Ctx, u32) -> c_int
        = b"pcre2_set_parens_nest_limit_8\0";
    set_callout: unsafe extern "C" fn(
        Ctx, Option<unsafe extern "C" fn(*mut CalloutBlock, *mut c_void) -> c_int>,
        *mut c_void) -> c_int = b"pcre2_set_callout_8\0";
    set_substitute_callout: unsafe extern "C" fn(
        Ctx, Option<unsafe extern "C" fn(*mut SubstituteCalloutBlock, *mut c_void) -> c_int>,
        *mut c_void) -> c_int = b"pcre2_set_substitute_callout_8\0";
    set_substitute_case_callout: unsafe extern "C" fn(
        Ctx,
        Option<unsafe extern "C" fn(*const u8, Sz, *mut u8, Sz, c_int, *mut c_void) -> Sz>,
        *mut c_void) -> c_int = b"pcre2_set_substitute_case_callout_8\0";
    set_depth_limit: unsafe extern "C" fn(Ctx, u32) -> c_int = b"pcre2_set_depth_limit_8\0";
    set_heap_limit: unsafe extern "C" fn(Ctx, u32) -> c_int = b"pcre2_set_heap_limit_8\0";
    set_match_limit: unsafe extern "C" fn(Ctx, u32) -> c_int = b"pcre2_set_match_limit_8\0";
    set_offset_limit: unsafe extern "C" fn(Ctx, Sz) -> c_int = b"pcre2_set_offset_limit_8\0";
    set_recursion_limit: unsafe extern "C" fn(Ctx, u32) -> c_int
        = b"pcre2_set_recursion_limit_8\0";
    set_recursion_memory_management: unsafe extern "C" fn(
        Ctx, *mut c_void, *mut c_void, *mut c_void) -> c_int
        = b"pcre2_set_recursion_memory_management_8\0";
    set_glob_escape: unsafe extern "C" fn(Ctx, u32) -> c_int = b"pcre2_set_glob_escape_8\0";
    set_glob_separator: unsafe extern "C" fn(Ctx, u32) -> c_int = b"pcre2_set_glob_separator_8\0";

    // ---- tables
    maketables: unsafe extern "C" fn(Ctx) -> *const u8 = b"pcre2_maketables_8\0";
    maketables_free: unsafe extern "C" fn(Ctx, *const u8) = b"pcre2_maketables_free_8\0";

    // ---- jit
    jit_compile: unsafe extern "C" fn(Code, u32) -> c_int = b"pcre2_jit_compile_8\0";
    jit_match: unsafe extern "C" fn(Code, *const u8, Sz, Sz, u32, MatchData, Ctx) -> c_int
        = b"pcre2_jit_match_8\0";
    jit_free_unused_memory: unsafe extern "C" fn(Ctx) = b"pcre2_jit_free_unused_memory_8\0";
    jit_stack_create: unsafe extern "C" fn(Sz, Sz, Ctx) -> *mut c_void
        = b"pcre2_jit_stack_create_8\0";
    jit_stack_assign: unsafe extern "C" fn(Ctx, *mut c_void, *mut c_void)
        = b"pcre2_jit_stack_assign_8\0";
    jit_stack_free: unsafe extern "C" fn(*mut c_void) = b"pcre2_jit_stack_free_8\0";
    priv_jit_free: unsafe extern "C" fn(*mut c_void, *mut c_void) = b"_pcre2_jit_free_8\0";
    priv_jit_free_rodata: unsafe extern "C" fn(*mut c_void, *mut c_void)
        = b"_pcre2_jit_free_rodata_8\0";
    priv_jit_get_size: unsafe extern "C" fn(*mut c_void) -> Sz = b"_pcre2_jit_get_size_8\0";
    priv_jit_get_target: unsafe extern "C" fn() -> *const c_char = b"_pcre2_jit_get_target_8\0";

    // ---- internal (PRIV) functions
    priv_ord2utf: unsafe extern "C" fn(u32, *mut u8) -> u32 = b"_pcre2_ord2utf_8\0";
    priv_valid_utf: unsafe extern "C" fn(*const u8, Sz, *mut Sz) -> c_int
        = b"_pcre2_valid_utf_8\0";
    priv_strlen: unsafe extern "C" fn(*const u8) -> Sz = b"_pcre2_strlen_8\0";
    priv_strcmp: unsafe extern "C" fn(*const u8, *const u8) -> c_int = b"_pcre2_strcmp_8\0";
    priv_strncmp: unsafe extern "C" fn(*const u8, *const u8, Sz) -> c_int
        = b"_pcre2_strncmp_8\0";
    priv_strcmp_c8: unsafe extern "C" fn(*const u8, *const c_char) -> c_int
        = b"_pcre2_strcmp_c8_8\0";
    priv_strncmp_c8: unsafe extern "C" fn(*const u8, *const c_char, Sz) -> c_int
        = b"_pcre2_strncmp_c8_8\0";
    priv_strcpy_c8: unsafe extern "C" fn(*mut u8, *const c_char) -> Sz
        = b"_pcre2_strcpy_c8_8\0";
    priv_ckd_smul: unsafe extern "C" fn(*mut Sz, c_int, c_int) -> c_int
        = b"_pcre2_ckd_smul_8\0";
    priv_is_newline: unsafe extern "C" fn(*const u8, u32, *const u8, *mut u32, c_int) -> c_int
        = b"_pcre2_is_newline_8\0";
    priv_was_newline: unsafe extern "C" fn(*const u8, u32, *const u8, *mut u32, c_int) -> c_int
        = b"_pcre2_was_newline_8\0";
    priv_extuni: unsafe extern "C" fn(
        u32, *const u8, *const u8, *const u8, c_int, *mut c_int) -> *const u8
        = b"_pcre2_extuni_8\0";
    priv_script_run: unsafe extern "C" fn(*const u8, *const u8, c_int) -> c_int
        = b"_pcre2_script_run_8\0";
    priv_find_bracket: unsafe extern "C" fn(*const u8, c_int, c_int) -> *const u8
        = b"_pcre2_find_bracket_8\0";
    priv_xclass: unsafe extern "C" fn(u32, *const u8, *const u8, c_int) -> c_int
        = b"_pcre2_xclass_8\0";
    priv_eclass: unsafe extern "C" fn(u32, *const u8, *const u8, *const u8, c_int) -> c_int
        = b"_pcre2_eclass_8\0";
    priv_memctl_malloc: unsafe extern "C" fn(Sz, *mut c_void) -> *mut c_void
        = b"_pcre2_memctl_malloc_8\0";
    priv_study: unsafe extern "C" fn(Code) -> c_int = b"_pcre2_study_8\0";

    // ---- exported data tables (as raw addresses)
    d_OP_lengths: *const u8 = b"_pcre2_OP_lengths_8\0";
    d_callout_start_delims: *const u8 = b"_pcre2_callout_start_delims_8\0";
    d_callout_end_delims: *const u8 = b"_pcre2_callout_end_delims_8\0";
    d_default_tables: *const u8 = b"_pcre2_default_tables_8\0";
    d_hspace_list: *const u32 = b"_pcre2_hspace_list_8\0";
    d_vspace_list: *const u32 = b"_pcre2_vspace_list_8\0";
    d_posix_class_maps: *const c_int = b"_pcre2_posix_class_maps8\0";
    d_ucd_records: *const u8 = b"_pcre2_ucd_records_8\0";
    d_ucd_stage1: *const u16 = b"_pcre2_ucd_stage1_8\0";
    d_ucd_stage2: *const u16 = b"_pcre2_ucd_stage2_8\0";
    d_ucd_caseless_sets: *const u32 = b"_pcre2_ucd_caseless_sets_8\0";
    d_ucd_digit_sets: *const u32 = b"_pcre2_ucd_digit_sets_8\0";
    d_ucd_script_sets: *const u32 = b"_pcre2_ucd_script_sets_8\0";
    d_ucd_boolprop_sets: *const u32 = b"_pcre2_ucd_boolprop_sets_8\0";
    d_ucd_nocase_ranges: *const u32 = b"_pcre2_ucd_nocase_ranges_8\0";
    d_ucd_nocase_ranges_size: *const u32 = b"_pcre2_ucd_nocase_ranges_size_8\0";
    d_ucd_turkish_dotted_i_caseset: *const u32
        = b"_pcre2_ucd_turkish_dotted_i_caseset_8\0";
    d_ucp_gbtable: *const u32 = b"_pcre2_ucp_gbtable_8\0";
    d_ucp_gentype: *const u32 = b"_pcre2_ucp_gentype_8\0";
    d_unicode_version: *const c_char = b"_pcre2_unicode_version_8\0";
    d_utf8_table1: *const c_int = b"_pcre2_utf8_table1\0";
    d_utf8_table1_size: *const c_int = b"_pcre2_utf8_table1_size\0";
    d_utf8_table2: *const u8 = b"_pcre2_utf8_table2\0";
    d_utf8_table3: *const u8 = b"_pcre2_utf8_table3\0";
    d_utf8_table4: *const u8 = b"_pcre2_utf8_table4\0";
    d_utt_size: *const Sz = b"_pcre2_utt_size_8\0";
    d_utt_names: *const c_char = b"_pcre2_utt_names_8\0";
    d_utt: *const u8 = b"_pcre2_utt_8\0";
    d_default_compile_context: *const u8 = b"_pcre2_default_compile_context_8\0";
    d_default_match_context: *const u8 = b"_pcre2_default_match_context_8\0";
    d_default_convert_context: *const u8 = b"_pcre2_default_convert_context_8\0";
}

fn crate_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> String {
    let p = crate_root().join("../c_src/build/libpcre2.so");
    p.canonicalize()
        .unwrap_or(p)
        .to_string_lossy()
        .into_owned()
}

pub fn rust_so_path() -> String {
    for prof in ["release", "debug"] {
        let p = crate_root().join(format!("target/{prof}/libpcre2.so"));
        if p.exists() {
            return p.to_string_lossy().into_owned();
        }
    }
    panic!("Rust libpcre2.so not built; run `cargo build --release`");
}

static C_API: OnceLock<Api> = OnceLock::new();
static R_API: OnceLock<Api> = OnceLock::new();

pub fn c() -> &'static Api {
    C_API.get_or_init(|| unsafe { Api::build(&c_so_path(), "C") })
}
pub fn r() -> &'static Api {
    R_API.get_or_init(|| unsafe { Api::build(&rust_so_path(), "Rust") })
}
/// Both implementations, C first.
pub fn both() -> [&'static Api; 2] {
    [c(), r()]
}

// --------------------------------------------------------------------- helpers

/// Deterministic xorshift PRNG so every run is reproducible.
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
    }
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo + 1)
    }
    pub fn pick<'a, T>(&mut self, v: &'a [T]) -> &'a T {
        &v[self.below(v.len())]
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}

/// NUL-terminated byte vector (patterns/subjects are passed with an explicit
/// length, but a trailing NUL lets us also test PCRE2_ZERO_TERMINATED).
pub fn cs(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}
pub fn cb(s: &[u8]) -> Vec<u8> {
    let mut v = s.to_vec();
    v.push(0);
    v
}

/// The full observable result of a match through the FFI boundary.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MatchOut {
    pub rc: c_int,
    pub oveccount: u32,
    pub ovector: Vec<Sz>,
    pub startchar: Sz,
    pub mark: Option<Vec<u8>>,
    pub data_size: Sz,
}

/// Error codes returned by `pcre2_match`/`pcre2_dfa_match` from the initial
/// argument-plausibility checks, i.e. *before* `match_data->mark`,
/// `->startchar` and `->subject_length` are assigned. On those paths the fields
/// hold whatever was already in the heap block, so they are not observable
/// output and must not be compared.
pub fn early_reject(rc: c_int) -> bool {
    matches!(
        rc,
        PCRE2_ERROR_NULL
            | PCRE2_ERROR_BADOPTION
            | PCRE2_ERROR_BADOFFSET
            | PCRE2_ERROR_BADMAGIC
            | PCRE2_ERROR_BADMODE
            | PCRE2_ERROR_BADOFFSETLIMIT
            | PCRE2_ERROR_BADUTFOFFSET
            | PCRE2_ERROR_NOMEMORY
            | PCRE2_ERROR_HEAPLIMIT
            | PCRE2_ERROR_DFA_WSSIZE
            | PCRE2_ERROR_DFA_BADRESTART
            | PCRE2_ERROR_DFA_UINVALID_UTF
    ) || (-23..=-3).contains(&rc)
}

impl Api {
    pub fn compile_ok(&self, pat: &[u8], options: u32, ctx: Ctx) -> (Code, c_int, Sz) {
        let mut err: c_int = 0;
        let mut off: Sz = 0;
        let code =
            unsafe { (self.compile)(pat.as_ptr(), pat.len(), options, &mut err, &mut off, ctx) };
        (code, err, off)
    }

    /// Read the observable match result. Only the ovector region the C API
    /// documents as defined for this return code is captured; bytes beyond it
    /// are untouched heap in *both* libraries and carry no meaning.
    pub fn read_match(&self, md: MatchData, rc: c_int, dfa: bool, capturecount: u32) -> MatchOut {
        unsafe {
            let n = (self.get_ovector_count)(md);
            let early = early_reject(rc);
            let defined_pairs: usize = if early {
                0
            } else if rc > 0 {
                if dfa {
                    (rc as usize).min(n as usize)
                } else {
                    ((capturecount + 1) as usize).min(n as usize)
                }
            } else if rc == 0 {
                n as usize
            } else if rc == PCRE2_ERROR_PARTIAL {
                if n > 0 { 1 } else { 0 }
            } else {
                0
            };
            let p = (self.get_ovector_pointer)(md);
            let ovector = if p.is_null() {
                Vec::new()
            } else {
                std::slice::from_raw_parts(p, defined_pairs * 2).to_vec()
            };
            let mark = if early {
                None
            } else {
                let mp = (self.get_mark)(md);
                if mp.is_null() {
                    None
                } else {
                    let mut v = Vec::new();
                    let mut i = 0usize;
                    while *mp.add(i) != 0 {
                        v.push(*mp.add(i));
                        i += 1;
                    }
                    Some(v)
                }
            };
            MatchOut {
                rc,
                oveccount: n,
                ovector,
                // pcre2_get_startchar is only defined after a successful or
                // partial match; on all other paths the field is never assigned
                // (see pcre2_match.c / pcre2_dfa_match.c).
                startchar: if rc >= 0 || rc == PCRE2_ERROR_PARTIAL {
                    (self.get_startchar)(md)
                } else {
                    0
                },
                mark,
                data_size: (self.get_match_data_size)(md),
            }
        }
    }
}

/// Assert equality with a descriptive message.
#[macro_export]
macro_rules! diff_eq {
    ($cv:expr, $rv:expr, $($arg:tt)*) => {{
        let cv = $cv;
        let rv = $rv;
        if cv != rv {
            panic!("C/Rust divergence: {}\n  C    = {:?}\n  Rust = {:?}",
                   format!($($arg)*), cv, rv);
        }
    }};
}

/// Read a NUL-terminated C string from a raw pointer.
pub unsafe fn cstr(p: *const u8) -> Vec<u8> {
    if p.is_null() {
        return Vec::new();
    }
    let mut v = Vec::new();
    let mut i = 0;
    while unsafe { *p.add(i) } != 0 {
        v.push(unsafe { *p.add(i) });
        i += 1;
    }
    v
}

// ============================================================================
//                        differential driver
// ============================================================================

pub const INFO_UINT32: &[u32] = &[0, 1, 2, 3, 4, 5, 6, 8, 9, 11, 12, 13, 17, 18, 20, 26];
pub const INFO_UINT32_MAYBE_UNSET: &[u32] = &[14, 21, 25];
pub const INFO_SIZE: &[u32] = &[15, 16, 22, 24];
pub const INFO_JITSIZE: u32 = 10;
pub const INFO_FIRSTBITMAP: u32 = 7;
pub const INFO_NAMETABLE: u32 = 19;
pub const INFO_HASBACKSLASHC: u32 = 23;

/// Everything `pcre2_pattern_info` can tell us about a compiled pattern,
/// captured as raw bytes so the comparison is exact.
#[derive(Debug, PartialEq, Eq)]
pub struct InfoOut {
    pub u32s: Vec<(u32, c_int, u32)>,
    pub sizes: Vec<(u32, c_int, Sz)>,
    pub bitmap: (c_int, Option<Vec<u8>>),
    pub nametable: (c_int, Vec<u8>),
    pub jitsize: (c_int, Sz),
    pub bad: Vec<(u32, c_int)>,
}

impl Api {
    pub fn info(&self, code: Code) -> InfoOut {
        let mut u32s = Vec::new();
        let mut sizes = Vec::new();
        unsafe {
            for &w in INFO_UINT32
                .iter()
                .chain(INFO_UINT32_MAYBE_UNSET)
                .chain(std::iter::once(&INFO_HASBACKSLASHC))
            {
                let mut v: u32 = 0xdead_beef;
                let rc = (self.pattern_info)(code, w, &mut v as *mut u32 as *mut c_void);
                u32s.push((w, rc, if rc == 0 { v } else { 0 }));
            }
            for &w in INFO_SIZE {
                let mut v: Sz = 0xdead_beef;
                let rc = (self.pattern_info)(code, w, &mut v as *mut Sz as *mut c_void);
                sizes.push((w, rc, if rc == 0 { v } else { 0 }));
            }
            let mut jsz: Sz = 0xdead_beef;
            let jrc = (self.pattern_info)(code, INFO_JITSIZE, &mut jsz as *mut Sz as *mut c_void);

            let mut bm: *const u8 = std::ptr::null();
            let brc = (self.pattern_info)(
                code,
                INFO_FIRSTBITMAP,
                &mut bm as *mut *const u8 as *mut c_void,
            );
            let bitmap = if brc == 0 && !bm.is_null() {
                (brc, Some(std::slice::from_raw_parts(bm, 32).to_vec()))
            } else {
                (brc, None)
            };

            let mut ncount: u32 = 0;
            let mut nsize: u32 = 0;
            (self.pattern_info)(code, 17, &mut ncount as *mut u32 as *mut c_void);
            (self.pattern_info)(code, 18, &mut nsize as *mut u32 as *mut c_void);
            let mut nt: *const u8 = std::ptr::null();
            let nrc = (self.pattern_info)(
                code,
                INFO_NAMETABLE,
                &mut nt as *mut *const u8 as *mut c_void,
            );
            let nametable = if nrc == 0 && !nt.is_null() {
                (nrc, std::slice::from_raw_parts(nt, (ncount * nsize) as usize).to_vec())
            } else {
                (nrc, Vec::new())
            };

            // out-of-range `what` values (ERRORS.md rows 101 / 176)
            let mut bad = Vec::new();
            for w in [27u32, 28, 99, 999, u32::MAX, u32::MAX - 1] {
                let mut v: Sz = 0;
                bad.push((w, (self.pattern_info)(code, w, &mut v as *mut Sz as *mut c_void)));
            }

            InfoOut {
                u32s,
                sizes,
                bitmap,
                nametable,
                jitsize: (jrc, if jrc == 0 { jsz } else { 0 }),
                bad,
            }
        }
    }
}

/// A full library configuration: everything settable before compiling/matching.
#[derive(Debug, Clone, Default)]
pub struct Cfg {
    pub options: u32,
    pub extra_options: u32,
    pub newline: Option<u32>,
    pub bsr: Option<u32>,
    pub optimize: Vec<u32>,
    pub max_varlookbehind: Option<u32>,
    pub parens_nest_limit: Option<u32>,
    pub max_pattern_length: Option<Sz>,
    pub max_pattern_compiled_length: Option<Sz>,
    pub use_maketables: bool,
    pub match_options: u32,
    pub start_offset: Sz,
    pub ovecsize: Option<u32>,
    pub match_limit: Option<u32>,
    pub depth_limit: Option<u32>,
    pub heap_limit: Option<u32>,
    pub offset_limit: Option<Sz>,
    pub zero_terminated_pattern: bool,
    pub zero_terminated_subject: bool,
}

pub struct Built {
    pub ccontext: Ctx,
    pub mcontext: Ctx,
    pub tables: *const u8,
}

impl Api {
    /// Apply a `Cfg` to fresh compile/match contexts.
    pub fn make_contexts(&self, cfg: &Cfg) -> Built {
        unsafe {
            let cc = (self.compile_context_create)(std::ptr::null_mut());
            let mc = (self.match_context_create)(std::ptr::null_mut());
            assert!(!cc.is_null() && !mc.is_null());
            let mut tables = std::ptr::null();
            (self.set_compile_extra_options)(cc, cfg.extra_options);
            if let Some(v) = cfg.newline {
                (self.set_newline)(cc, v);
            }
            if let Some(v) = cfg.bsr {
                (self.set_bsr)(cc, v);
            }
            for &d in &cfg.optimize {
                (self.set_optimize)(cc, d);
            }
            if let Some(v) = cfg.max_varlookbehind {
                (self.set_max_varlookbehind)(cc, v);
            }
            if let Some(v) = cfg.parens_nest_limit {
                (self.set_parens_nest_limit)(cc, v);
            }
            if let Some(v) = cfg.max_pattern_length {
                (self.set_max_pattern_length)(cc, v);
            }
            if let Some(v) = cfg.max_pattern_compiled_length {
                (self.set_max_pattern_compiled_length)(cc, v);
            }
            if cfg.use_maketables {
                tables = (self.maketables)(std::ptr::null_mut());
                (self.set_character_tables)(cc, tables);
            }
            if let Some(v) = cfg.match_limit {
                (self.set_match_limit)(mc, v);
            }
            if let Some(v) = cfg.depth_limit {
                (self.set_depth_limit)(mc, v);
            }
            if let Some(v) = cfg.heap_limit {
                (self.set_heap_limit)(mc, v);
            }
            if let Some(v) = cfg.offset_limit {
                (self.set_offset_limit)(mc, v);
            }
            Built { ccontext: cc, mcontext: mc, tables }
        }
    }

    pub fn drop_contexts(&self, b: Built) {
        unsafe {
            (self.compile_context_free)(b.ccontext);
            (self.match_context_free)(b.mcontext);
            if !b.tables.is_null() {
                (self.maketables_free)(std::ptr::null_mut(), b.tables);
            }
        }
    }
}

/// Result of the whole compile -> info -> match -> substring pipeline.
#[derive(Debug, PartialEq, Eq)]
pub struct FullOut {
    pub compile_err: c_int,
    pub compile_off: Sz,
    pub compiled: bool,
    pub info: Option<InfoOut>,
    pub m: Option<MatchOut>,
    pub subs: Vec<(u32, c_int, Option<Vec<u8>>)>,
    pub sub_list: (c_int, Vec<Vec<u8>>),
}

/// Run the full pipeline for one implementation.
pub fn run_full(api: &Api, cfg: &Cfg, pattern: &[u8], subject: &[u8], dfa: bool) -> FullOut {
    let b = api.make_contexts(cfg);
    let patlen =
        if cfg.zero_terminated_pattern { PCRE2_ZERO_TERMINATED } else { pattern.len() };
    let mut err: c_int = 0;
    let mut off: Sz = 0;
    let code = unsafe {
        (api.compile)(pattern.as_ptr(), patlen, cfg.options, &mut err, &mut off, b.ccontext)
    };
    if code.is_null() {
        api.drop_contexts(b);
        return FullOut {
            compile_err: err,
            compile_off: off,
            compiled: false,
            info: None,
            m: None,
            subs: Vec::new(),
            sub_list: (0, Vec::new()),
        };
    }
    let info = api.info(code);
    let md = unsafe {
        match cfg.ovecsize {
            Some(n) => (api.match_data_create)(n, std::ptr::null_mut()),
            None => (api.match_data_create_from_pattern)(code, std::ptr::null_mut()),
        }
    };
    assert!(!md.is_null());
    let sublen =
        if cfg.zero_terminated_subject { PCRE2_ZERO_TERMINATED } else { subject.len() };
    let rc = unsafe {
        if dfa {
            let mut ws = [0i32; 256];
            (api.dfa_match)(
                code,
                subject.as_ptr(),
                sublen,
                cfg.start_offset,
                cfg.match_options,
                md,
                b.mcontext,
                ws.as_mut_ptr(),
                ws.len(),
            )
        } else {
            (api.do_match)(
                code,
                subject.as_ptr(),
                sublen,
                cfg.start_offset,
                cfg.match_options,
                md,
                b.mcontext,
            )
        }
    };
    let mut capcount: u32 = 0;
    unsafe { (api.pattern_info)(code, 4, &mut capcount as *mut u32 as *mut c_void) };
    let m = api.read_match(md, rc, dfa, capcount);

    // substring extraction for groups 0 ..= capturecount+1 (one past the top)
    let mut subs = Vec::new();
    for g in 0..(capcount + 2) {
        unsafe {
            let mut len: Sz = 0;
            let lrc = (api.substring_length_bynumber)(md, g, &mut len);
            let mut p: *mut u8 = std::ptr::null_mut();
            let mut glen: Sz = 0;
            let grc = (api.substring_get_bynumber)(md, g, &mut p, &mut glen);
            let val = if grc == 0 && !p.is_null() {
                let v = std::slice::from_raw_parts(p, glen).to_vec();
                (api.substring_free)(p);
                Some(v)
            } else {
                None
            };
            assert_eq!(lrc, grc, "{}: length/get rc disagree for group {g}", api.name);
            subs.push((g, grc, val));
        }
    }
    // substring list
    let sub_list = unsafe {
        let mut list: *mut *mut u8 = std::ptr::null_mut();
        let mut lens: *mut Sz = std::ptr::null_mut();
        let lrc = (api.substring_list_get)(md, &mut list, &mut lens);
        let mut v = Vec::new();
        if lrc == 0 && !list.is_null() {
            let n = if rc > 0 { rc as usize } else { 0 };
            for i in 0..n {
                let p = *list.add(i);
                let l = *lens.add(i);
                v.push(std::slice::from_raw_parts(p, l).to_vec());
            }
            (api.substring_list_free)(list);
        }
        (lrc, v)
    };

    unsafe {
        (api.match_data_free)(md);
        (api.code_free)(code);
    }
    api.drop_contexts(b);
    FullOut {
        compile_err: err,
        compile_off: off,
        compiled: true,
        info: Some(info),
        m: Some(m),
        subs,
        sub_list,
    }
}

/// Compact, field-level description of where two `FullOut`s differ.
pub fn explain(co: &FullOut, ro: &FullOut) -> String {
    let mut d = Vec::new();
    if co.compile_err != ro.compile_err {
        d.push(format!("compile_err: C={} Rust={}", co.compile_err, ro.compile_err));
    }
    if co.compile_off != ro.compile_off {
        d.push(format!("compile_off: C={} Rust={}", co.compile_off, ro.compile_off));
    }
    if co.compiled != ro.compiled {
        d.push(format!("compiled: C={} Rust={}", co.compiled, ro.compiled));
    }
    if let (Some(ci), Some(ri)) = (&co.info, &ro.info) {
        for (a, b) in ci.u32s.iter().zip(&ri.u32s) {
            if a != b {
                d.push(format!("info[what={}] u32: C=(rc {},v {}) Rust=(rc {},v {})",
                               a.0, a.1, a.2, b.1, b.2));
            }
        }
        for (a, b) in ci.sizes.iter().zip(&ri.sizes) {
            if a != b {
                d.push(format!("info[what={}] size: C=(rc {},v {}) Rust=(rc {},v {})",
                               a.0, a.1, a.2, b.1, b.2));
            }
        }
        if ci.bitmap != ri.bitmap {
            d.push(format!("info FIRSTBITMAP: C={:?} Rust={:?}", ci.bitmap, ri.bitmap));
        }
        if ci.nametable != ri.nametable {
            d.push(format!("info NAMETABLE: C={:02x?} Rust={:02x?}", ci.nametable, ri.nametable));
        }
        if ci.jitsize != ri.jitsize {
            d.push(format!("info JITSIZE: C={:?} Rust={:?}", ci.jitsize, ri.jitsize));
        }
        if ci.bad != ri.bad {
            d.push(format!("info bad-what: C={:?} Rust={:?}", ci.bad, ri.bad));
        }
    } else if co.info.is_some() != ro.info.is_some() {
        d.push("info presence differs".into());
    }
    match (&co.m, &ro.m) {
        (Some(cm), Some(rm)) => {
            if cm.rc != rm.rc {
                d.push(format!("match rc: C={} Rust={}", cm.rc, rm.rc));
            }
            if cm.oveccount != rm.oveccount {
                d.push(format!("oveccount: C={} Rust={}", cm.oveccount, rm.oveccount));
            }
            if cm.ovector != rm.ovector {
                d.push(format!("ovector: C={:?} Rust={:?}", cm.ovector, rm.ovector));
            }
            if cm.startchar != rm.startchar {
                d.push(format!("startchar: C={} Rust={}", cm.startchar, rm.startchar));
            }
            if cm.mark != rm.mark {
                d.push(format!("mark: C={:?} Rust={:?}", cm.mark, rm.mark));
            }
            if cm.data_size != rm.data_size {
                d.push(format!("match_data_size: C={} Rust={}", cm.data_size, rm.data_size));
            }
        }
        (a, b) if a.is_some() != b.is_some() => d.push("match presence differs".into()),
        _ => {}
    }
    if co.subs != ro.subs {
        for (a, b) in co.subs.iter().zip(&ro.subs) {
            if a != b {
                d.push(format!("substring[{}]: C={:?} Rust={:?}", a.0, (a.1, &a.2), (b.1, &b.2)));
            }
        }
        if co.subs.len() != ro.subs.len() {
            d.push(format!("substring count: C={} Rust={}", co.subs.len(), ro.subs.len()));
        }
    }
    if co.sub_list != ro.sub_list {
        d.push(format!("substring_list: C={:?} Rust={:?}", co.sub_list, ro.sub_list));
    }
    if d.is_empty() {
        d.push("(structurally equal but PartialEq disagreed)".into());
    }
    d.join("\n   ")
}

/// Run one configuration through BOTH .so files and require identical output.
pub fn differential(cfg: &Cfg, pattern: &[u8], subject: &[u8], dfa: bool) {
    let co = run_full(c(), cfg, pattern, subject, dfa);
    let ro = run_full(r(), cfg, pattern, subject, dfa);
    if co != ro {
        panic!(
            "DIVERGENCE\n cfg     = {cfg:?}\n dfa     = {dfa}\n pattern = {:?}\n subject = {:?}\n   {}",
            String::from_utf8_lossy(pattern),
            String::from_utf8_lossy(subject),
            explain(&co, &ro),
        );
    }
}

// ============================================================================
//                        pattern / subject corpora
// ============================================================================

/// A small regex grammar. Generates syntactically plausible patterns (most
/// compile, some do not - both outcomes are compared).
pub fn random_pattern(rng: &mut Rng, depth: u32) -> String {
    let atoms: &[&str] = &[
        "a", "b", "c", "z", "A", "Z", "0", "9", ".", "\\d", "\\D", "\\w", "\\W", "\\s", "\\S",
        "\\h", "\\H", "\\v", "\\V", "\\R", "\\X", "\\N", "\\b", "\\B", "\\A", "\\Z", "\\z", "\\G",
        "\\Q a.b \\E", "[a-z]", "[^a-z]", "[[:alpha:]]", "[[:^digit:]]", "[\\d\\s]", "[]]",
        "\\p{L}", "\\p{Nd}", "\\P{L}", "\\p{Greek}", "\\p{Han}", "^", "$", "\\x41", "\\x{263a}",
        "\\101", "\\o{101}", "\\n", "\\r", "\\t", "\\f", "\\e", "\\a", "\\cA", "é", "日",
        "\\K", "(?i)a", "(?-i)b", "(?s:.)", "(?m:^)", "(?x: a b )", "(?J)(?<n>a)",
    ];
    let quants: &[&str] = &[
        "", "*", "+", "?", "{2}", "{0,3}", "{2,}", "{,3}", "*?", "+?", "??", "{2,4}?", "*+", "++",
        "?+", "{1,3}+",
    ];
    let verbs: &[&str] = &[
        "(*FAIL)", "(*ACCEPT)", "(*COMMIT)", "(*PRUNE)", "(*SKIP)", "(*THEN)", "(*MARK:m1)",
        "(*PRUNE:p)", "(*SKIP:s)", "(*THEN:t)",
    ];
    if depth == 0 {
        let a = rng.pick(atoms);
        let q = rng.pick(quants);
        return format!("{a}{q}");
    }
    let n = rng.range(1, 3);
    let mut out = String::new();
    for _ in 0..n {
        let choice = rng.below(14);
        let piece = match choice {
            0 => {
                let a = rng.pick(atoms).to_string();
                let q = rng.pick(quants);
                format!("{a}{q}")
            }
            1 => format!("({}){}", random_pattern(rng, depth - 1), rng.pick(quants)),
            2 => format!("(?:{}){}", random_pattern(rng, depth - 1), rng.pick(quants)),
            3 => format!(
                "(?<g{}>{}){}",
                rng.below(4),
                random_pattern(rng, depth - 1),
                rng.pick(quants)
            ),
            4 => format!("(?={})", random_pattern(rng, depth - 1)),
            5 => format!("(?!{})", random_pattern(rng, depth - 1)),
            6 => format!("(?<=a{})", random_pattern(rng, 0)),
            7 => format!("(?<!a{})", random_pattern(rng, 0)),
            8 => format!("(?>{}){}", random_pattern(rng, depth - 1), rng.pick(quants)),
            9 => format!(
                "{}|{}",
                random_pattern(rng, depth - 1),
                random_pattern(rng, depth - 1)
            ),
            10 => format!("(a)(?({})b|c)", rng.range(1, 2)),
            11 => rng.pick(verbs).to_string(),
            12 => format!("(a|b)\\{}", rng.range(1, 2)),
            _ => format!("[{}{}]{}", rng.pick(&["a-c", "^x", "\\d", "0-9a-fA-F", "-", "]"]),
                         rng.pick(&["", "z", "\\w"]), rng.pick(quants)),
        };
        out.push_str(&piece);
    }
    out
}

/// Random subject bytes. `utf` restricts output to valid UTF-8.
pub fn random_subject(rng: &mut Rng, utf: bool) -> Vec<u8> {
    let n = rng.below(24);
    if utf {
        let pool: &[char] = &[
            'a', 'b', 'c', 'z', 'A', 'Z', '0', '9', ' ', '\n', '\r', '\t', '.', '-', 'é', 'ß',
            'İ', 'ı', '日', '本', 'Δ', 'д', '\u{85}', '\u{2028}', '\u{2029}', '\u{1F600}',
            '\u{301}', '\u{0660}',
        ];
        let mut s = String::new();
        for _ in 0..n {
            s.push(*rng.pick(pool));
        }
        s.into_bytes()
    } else {
        let pool: &[u8] = &[
            b'a', b'b', b'c', b'z', b'A', b'Z', b'0', b'9', b' ', b'\n', b'\r', b'\t', b'.', b'-',
            0x00, 0x80, 0xc2, 0x85, 0xff, 0xe2,
        ];
        (0..n).map(|_| *rng.pick(pool)).collect()
    }
}

/// Curated pattern set covering the syntax the compiler branches on.
pub fn curated_patterns() -> Vec<&'static str> {
    vec![
        "", "a", "abc", "a*", "a+", "a?", "a{3}", "a{2,5}", "a{2,}", "a{,4}", "a*?", "a*+",
        ".", ".*", ".+", "^a", "a$", "^$", "^.*$", "(a)", "(a)(b)", "(?:ab)", "(a|b)",
        "(a|b|c)*", "[abc]", "[^abc]", "[a-z0-9]", "[[:alpha:][:digit:]]", "[\\d-\\s]",
        "\\d+", "\\w+\\s\\w+", "\\bword\\b", "\\Qa.b\\E", "a\\z", "a\\Z", "\\Aa",
        "(?i)AbC", "(?m)^x$", "(?s)a.b", "(?x) a  b # comment\n c",
        "(?<name>a)(?<other>b)", "(?<n>a)|(?<n>b)", "\\k<name>", "(?P<n>a)(?P=n)",
        "(a)\\1", "(a)(b)\\2\\1", "(?1)(a)", "(a)(?1)", "(?R)?a", "a(?R)?b",
        "(?<n>a)(?&n)", "(?(1)a|b)(c)", "(?(<n>)x|y)(?<n>z)", "(?(R)a|b)",
        "(?=a)a", "(?!a)b", "(?<=a)b", "(?<!a)b", "(?<=ab|cde)x", "(?*a)b", "(?<*a)b",
        "(?>a*)b", "a*+b", "(?|(a)|(b))", "(?C1)a", "(?C{txt})b",
        "(*FAIL)", "a(*FAIL)|b", "(*COMMIT)a|b", "a(*PRUNE)b|c", "a(*SKIP)b|ab",
        "a(*THEN)b|ac", "(*MARK:one)a|(*MARK:two)b", "(*ACCEPT)a",
        "\\X+", "\\R+", "\\N+", "\\H\\h\\V\\v",
        "\\p{L}+", "\\p{^L}+", "\\P{Nd}", "\\p{Greek}", "\\p{Han}", "\\p{Any}",
        "\\p{Xan}", "\\p{Xps}", "\\p{Xsp}", "\\p{Xuc}", "\\p{Xwd}",
        "\\x{1F600}", "\\x41\\x42", "\\101\\102", "\\o{101}", "\\N{U+0041}",
        "\\cA", "\\e", "\\a", "\\f", "\\t", "\\n", "\\r", "\\0", "\\00", "\\000",
        "[\\x00-\\xff]", "[^\\x00]", "[\\p{L}\\d]", "[[:^alpha:]]",
        "(?i)[a-z]", "(?i)\\w", "(?i)ß", "(?i)İ", "(?i)K",
        "a{0}b", "(a){0}b", "(?:a){0,0}", "a**", "((((((((((a))))))))))",
        "(?:(?:(?:(?:a))))", "[a-\\d]", "x(?#comment)y",
        "\\g1(a)", "\\g{1}(a)", "\\g<1>(a)", "\\g'1'(a)", "(a)\\g{-1}",
        "(?'n'a)(?P>n)", "(?+1)(a)", "(a)(?-1)",
        "^(?:(?=a)|(?=b))c", "(?(?=a)b|c)",
        "\\B\\b", "(?<n>)(?<m>)", "(?J)(?<n>a)(?<n>b)",
        "a(?i:b)c", "a(?-i:b)c", "(?i)a(?-i)b",
        "[[:word:]]", "[[:graph:]]", "[[:print:]]", "[[:punct:]]", "[[:cntrl:]]",
        "[[:xdigit:]]", "[[:space:]]", "[[:blank:]]", "[[:ascii:]]", "[[:lower:]]",
        "[[:upper:]]",
        "\\p{Lu}\\p{Ll}", "(?:a|b|c|d|e|f|g|h)+", "(a+)+b",
        "\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}",
        "(?<y>\\d{4})-(?<m>\\d{2})-(?<d>\\d{2})",
        "[^\\n]*", "(?s)[^\\n]*", "\\s*$", "^\\s*",
        "a|", "|a", "(|a)", "(a|)",
        "(?:)", "()", "()*", "(){2}",
        "\\p{Latin}\\p{Cyrillic}", "(*script_run:\\w+)", "(*sr:\\w+)",
        "(*atomic_script_run:\\w+)", "(*asr:\\w+)",
        "(*UTF)a", "(*UCP)\\w", "(*CR)a$", "(*LF)a$", "(*CRLF)a$", "(*ANY)a$",
        "(*ANYCRLF)a$", "(*NUL)a$", "(*BSR_ANYCRLF)\\R", "(*BSR_UNICODE)\\R",
        "(*LIMIT_MATCH=100)a", "(*LIMIT_DEPTH=50)a", "(*LIMIT_HEAP=100)a",
        "(*NOTEMPTY)a*", "(*NOTEMPTY_ATSTART)a*", "(*NO_AUTO_POSSESS)a*b",
        "(*NO_DOTSTAR_ANCHOR).*a", "(*NO_START_OPT)abc", "(*NO_JIT)abc",
        "(?i)(?<n>a)(?&n)", "\\p{Bidi_Control}", "\\p{Bidi_Class:L}",
        "[\\N{U+0041}-\\N{U+005A}]", "\\p{Cased}", "\\p{Changes_When_Casefolded}",
    ]
}

/// Curated subject set.
pub fn curated_subjects() -> Vec<&'static str> {
    vec![
        "", "a", "b", "ab", "abc", "aaa", "aaab", "xyz", "AbC", "ABC", "0", "123", "a1b2",
        " ", "  ", "\n", "\r", "\r\n", "\n\r", "a\nb", "a\rb", "a\r\nb", "a\0b",
        "hello world", "The quick brown fox", "word boundary test",
        "é", "ééé", "ßß", "İıIi", "日本語テスト", "ΔΕΛΤΑ", "дом",
        "\u{85}", "\u{2028}", "\u{2029}", "a\u{85}b", "\u{1F600}\u{1F601}",
        "e\u{301}", "\u{1F1FA}\u{1F1F8}",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab", "192.168.0.1", "2024-01-31",
        "one two three four five", "\ta\tb\t", "!@#$%^&*()", "[]{}()<>",
        "\\", "\\\\", "a.b", "a*b",
    ]
}
