//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading`:
//!   * the C reference  `c_src/build/libpcre2.so`
//!   * the Rust build   `translation/target/release/libpcre2.so`
//!
//! Every call in every test goes through these dynamic symbols — the Rust crate is
//! never called directly, so the `#[no_mangle] extern "C"` export wrappers are part
//! of what is under test.
#![allow(dead_code)]
#![allow(non_snake_case)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::path::PathBuf;

pub type Ptr = *mut c_void;
pub type Sptr = *const u8;

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

pub const PCRE2_JIT_COMPLETE: u32 = 0x0000_0001;
pub const PCRE2_JIT_PARTIAL_SOFT: u32 = 0x0000_0002;
pub const PCRE2_JIT_PARTIAL_HARD: u32 = 0x0000_0004;
pub const PCRE2_JIT_INVALID_UTF: u32 = 0x0000_0100;
pub const PCRE2_JIT_TEST_ALLOC: u32 = 0x0000_0200;

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

pub const PCRE2_NEWLINE_CR: u32 = 1;
pub const PCRE2_NEWLINE_LF: u32 = 2;
pub const PCRE2_NEWLINE_CRLF: u32 = 3;
pub const PCRE2_NEWLINE_ANY: u32 = 4;
pub const PCRE2_NEWLINE_ANYCRLF: u32 = 5;
pub const PCRE2_NEWLINE_NUL: u32 = 6;
pub const PCRE2_BSR_UNICODE: u32 = 1;
pub const PCRE2_BSR_ANYCRLF: u32 = 2;

// ------------------------------------------------------------- runtime errors
pub const PCRE2_ERROR_NOMATCH: i32 = -1;
pub const PCRE2_ERROR_PARTIAL: i32 = -2;
pub const PCRE2_ERROR_BADDATA: i32 = -29;
pub const PCRE2_ERROR_MIXEDTABLES: i32 = -30;
pub const PCRE2_ERROR_BADMAGIC: i32 = -31;
pub const PCRE2_ERROR_BADMODE: i32 = -32;
pub const PCRE2_ERROR_BADOFFSET: i32 = -33;
pub const PCRE2_ERROR_BADOPTION: i32 = -34;
pub const PCRE2_ERROR_BADREPLACEMENT: i32 = -35;
pub const PCRE2_ERROR_BADUTFOFFSET: i32 = -36;
pub const PCRE2_ERROR_DFA_BADRESTART: i32 = -38;
pub const PCRE2_ERROR_DFA_UCOND: i32 = -40;
pub const PCRE2_ERROR_DFA_UFUNC: i32 = -41;
pub const PCRE2_ERROR_DFA_UITEM: i32 = -42;
pub const PCRE2_ERROR_DFA_WSSIZE: i32 = -43;
pub const PCRE2_ERROR_INTERNAL: i32 = -44;
pub const PCRE2_ERROR_JIT_BADOPTION: i32 = -45;
pub const PCRE2_ERROR_MATCHLIMIT: i32 = -47;
pub const PCRE2_ERROR_NOMEMORY: i32 = -48;
pub const PCRE2_ERROR_NOSUBSTRING: i32 = -49;
pub const PCRE2_ERROR_NOUNIQUESUBSTRING: i32 = -50;
pub const PCRE2_ERROR_NULL: i32 = -51;
pub const PCRE2_ERROR_RECURSELOOP: i32 = -52;
pub const PCRE2_ERROR_DEPTHLIMIT: i32 = -53;
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

pub const PCRE2_UNSET: usize = usize::MAX;
pub const PCRE2_ZERO_TERMINATED: usize = usize::MAX;

// ------------------------------------------------------- pattern_info keys
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

// ------------------------------------------------------------- config keys
pub const PCRE2_CONFIG_BSR: u32 = 0;
pub const PCRE2_CONFIG_JIT: u32 = 1;
pub const PCRE2_CONFIG_JITTARGET: u32 = 2;
pub const PCRE2_CONFIG_LINKSIZE: u32 = 3;
pub const PCRE2_CONFIG_MATCHLIMIT: u32 = 4;
pub const PCRE2_CONFIG_NEWLINE: u32 = 5;
pub const PCRE2_CONFIG_PARENSLIMIT: u32 = 6;
pub const PCRE2_CONFIG_DEPTHLIMIT: u32 = 7;
pub const PCRE2_CONFIG_UNICODE: u32 = 9;
pub const PCRE2_CONFIG_UNICODE_VERSION: u32 = 10;
pub const PCRE2_CONFIG_VERSION: u32 = 11;
pub const PCRE2_CONFIG_HEAPLIMIT: u32 = 12;
pub const PCRE2_CONFIG_NEVER_BACKSLASH_C: u32 = 13;
pub const PCRE2_CONFIG_COMPILED_WIDTHS: u32 = 14;
pub const PCRE2_CONFIG_TABLES_LENGTH: u32 = 15;

// ================================================================= loader
macro_rules! api {
    ( $( $field:ident : $t:ty ),* $(,)? ) => {
        pub struct Api {
            pub tag: &'static str,
            pub path: PathBuf,
            _lib: &'static Library,
            $( pub $field : $t, )*
        }
        impl Api {
            fn build(tag: &'static str, path: PathBuf) -> Api {
                let lib: &'static Library = Box::leak(Box::new(unsafe {
                    Library::new(&path).unwrap_or_else(|e| panic!("dlopen {:?}: {}", path, e))
                }));
                unsafe {
                    Api {
                        tag, path, _lib: lib,
                        $( $field : {
                            let s: Symbol<$t> = lib
                                .get(concat!(stringify!($field), "\0").as_bytes())
                                .unwrap_or_else(|e| panic!("{}: symbol {} missing: {}",
                                                           tag, stringify!($field), e));
                            *s
                        }, )*
                    }
                }
            }
        }
    };
}

api! {
    // ---- general / compile / match contexts
    pcre2_general_context_create_8: unsafe extern "C" fn(
        Option<unsafe extern "C" fn(usize, Ptr) -> Ptr>,
        Option<unsafe extern "C" fn(Ptr, Ptr)>, Ptr) -> Ptr,
    pcre2_general_context_copy_8: unsafe extern "C" fn(Ptr) -> Ptr,
    pcre2_general_context_free_8: unsafe extern "C" fn(Ptr),
    pcre2_compile_context_create_8: unsafe extern "C" fn(Ptr) -> Ptr,
    pcre2_compile_context_copy_8: unsafe extern "C" fn(Ptr) -> Ptr,
    pcre2_compile_context_free_8: unsafe extern "C" fn(Ptr),
    pcre2_match_context_create_8: unsafe extern "C" fn(Ptr) -> Ptr,
    pcre2_match_context_copy_8: unsafe extern "C" fn(Ptr) -> Ptr,
    pcre2_match_context_free_8: unsafe extern "C" fn(Ptr),
    pcre2_convert_context_create_8: unsafe extern "C" fn(Ptr) -> Ptr,
    pcre2_convert_context_copy_8: unsafe extern "C" fn(Ptr) -> Ptr,
    pcre2_convert_context_free_8: unsafe extern "C" fn(Ptr),

    // ---- setters
    pcre2_set_bsr_8: unsafe extern "C" fn(Ptr, u32) -> i32,
    pcre2_set_character_tables_8: unsafe extern "C" fn(Ptr, *const u8) -> i32,
    pcre2_set_compile_extra_options_8: unsafe extern "C" fn(Ptr, u32) -> i32,
    pcre2_set_max_pattern_length_8: unsafe extern "C" fn(Ptr, usize) -> i32,
    pcre2_set_max_pattern_compiled_length_8: unsafe extern "C" fn(Ptr, usize) -> i32,
    pcre2_set_max_varlookbehind_8: unsafe extern "C" fn(Ptr, u32) -> i32,
    pcre2_set_newline_8: unsafe extern "C" fn(Ptr, u32) -> i32,
    pcre2_set_parens_nest_limit_8: unsafe extern "C" fn(Ptr, u32) -> i32,
    pcre2_set_compile_recursion_guard_8: unsafe extern "C" fn(
        Ptr, Option<unsafe extern "C" fn(u32, Ptr) -> i32>, Ptr) -> i32,
    pcre2_set_callout_8: unsafe extern "C" fn(
        Ptr, Option<unsafe extern "C" fn(Ptr, Ptr) -> i32>, Ptr) -> i32,
    pcre2_set_substitute_callout_8: unsafe extern "C" fn(
        Ptr, Option<unsafe extern "C" fn(Ptr, Ptr) -> i32>, Ptr) -> i32,
    pcre2_set_substitute_case_callout_8: unsafe extern "C" fn(
        Ptr, Option<unsafe extern "C" fn(Sptr, usize, *mut u8, usize, i32, Ptr) -> usize>,
        Ptr) -> i32,
    pcre2_set_depth_limit_8: unsafe extern "C" fn(Ptr, u32) -> i32,
    pcre2_set_heap_limit_8: unsafe extern "C" fn(Ptr, u32) -> i32,
    pcre2_set_match_limit_8: unsafe extern "C" fn(Ptr, u32) -> i32,
    pcre2_set_offset_limit_8: unsafe extern "C" fn(Ptr, usize) -> i32,
    pcre2_set_recursion_limit_8: unsafe extern "C" fn(Ptr, u32) -> i32,
    pcre2_set_recursion_memory_management_8: unsafe extern "C" fn(
        Ptr, Option<unsafe extern "C" fn(usize, Ptr) -> Ptr>,
        Option<unsafe extern "C" fn(Ptr, Ptr)>, Ptr) -> i32,
    pcre2_set_optimize_8: unsafe extern "C" fn(Ptr, u32) -> i32,
    pcre2_set_glob_escape_8: unsafe extern "C" fn(Ptr, u32) -> i32,
    pcre2_set_glob_separator_8: unsafe extern "C" fn(Ptr, u32) -> i32,

    // ---- compile / code
    pcre2_compile_8: unsafe extern "C" fn(Sptr, usize, u32, *mut i32, *mut usize, Ptr) -> Ptr,
    pcre2_code_free_8: unsafe extern "C" fn(Ptr),
    pcre2_code_copy_8: unsafe extern "C" fn(Ptr) -> Ptr,
    pcre2_code_copy_with_tables_8: unsafe extern "C" fn(Ptr) -> Ptr,
    pcre2_pattern_info_8: unsafe extern "C" fn(Ptr, u32, Ptr) -> i32,
    pcre2_callout_enumerate_8: unsafe extern "C" fn(
        Ptr, Option<unsafe extern "C" fn(Ptr, Ptr) -> i32>, Ptr) -> i32,

    // ---- match data
    pcre2_match_data_create_8: unsafe extern "C" fn(u32, Ptr) -> Ptr,
    pcre2_match_data_create_from_pattern_8: unsafe extern "C" fn(Ptr, Ptr) -> Ptr,
    pcre2_match_data_free_8: unsafe extern "C" fn(Ptr),
    pcre2_get_ovector_count_8: unsafe extern "C" fn(Ptr) -> u32,
    pcre2_get_ovector_pointer_8: unsafe extern "C" fn(Ptr) -> *mut usize,
    pcre2_get_startchar_8: unsafe extern "C" fn(Ptr) -> usize,
    pcre2_get_mark_8: unsafe extern "C" fn(Ptr) -> Sptr,
    pcre2_get_match_data_size_8: unsafe extern "C" fn(Ptr) -> usize,
    pcre2_get_match_data_heapframes_size_8: unsafe extern "C" fn(Ptr) -> usize,

    // ---- matching
    pcre2_match_8: unsafe extern "C" fn(Ptr, Sptr, usize, usize, u32, Ptr, Ptr) -> i32,
    pcre2_dfa_match_8: unsafe extern "C" fn(
        Ptr, Sptr, usize, usize, u32, Ptr, Ptr, *mut i32, usize) -> i32,
    pcre2_jit_match_8: unsafe extern "C" fn(Ptr, Sptr, usize, usize, u32, Ptr, Ptr) -> i32,
    pcre2_next_match_8: unsafe extern "C" fn(Ptr, *mut usize, *mut u32) -> i32,

    // ---- substitute / substring
    pcre2_substitute_8: unsafe extern "C" fn(
        Ptr, Sptr, usize, usize, u32, Ptr, Ptr, Sptr, usize, *mut u8, *mut usize) -> i32,
    pcre2_substring_copy_byname_8: unsafe extern "C" fn(Ptr, Sptr, *mut u8, *mut usize) -> i32,
    pcre2_substring_copy_bynumber_8: unsafe extern "C" fn(Ptr, u32, *mut u8, *mut usize) -> i32,
    pcre2_substring_free_8: unsafe extern "C" fn(*mut u8),
    pcre2_substring_get_byname_8: unsafe extern "C" fn(Ptr, Sptr, *mut *mut u8, *mut usize) -> i32,
    pcre2_substring_get_bynumber_8: unsafe extern "C" fn(Ptr, u32, *mut *mut u8, *mut usize) -> i32,
    pcre2_substring_length_byname_8: unsafe extern "C" fn(Ptr, Sptr, *mut usize) -> i32,
    pcre2_substring_length_bynumber_8: unsafe extern "C" fn(Ptr, u32, *mut usize) -> i32,
    pcre2_substring_list_free_8: unsafe extern "C" fn(*mut *mut u8),
    pcre2_substring_list_get_8: unsafe extern "C" fn(Ptr, *mut *mut *mut u8, *mut *mut usize) -> i32,
    pcre2_substring_nametable_scan_8: unsafe extern "C" fn(
        Ptr, Sptr, *mut *mut u8, *mut *mut u8) -> i32,
    pcre2_substring_number_from_name_8: unsafe extern "C" fn(Ptr, Sptr) -> i32,

    // ---- serialize
    pcre2_serialize_encode_8: unsafe extern "C" fn(
        *const Ptr, i32, *mut *mut u8, *mut usize, Ptr) -> i32,
    pcre2_serialize_decode_8: unsafe extern "C" fn(*mut Ptr, i32, *const u8, Ptr) -> i32,
    pcre2_serialize_get_number_of_codes_8: unsafe extern "C" fn(*const u8) -> i32,
    pcre2_serialize_free_8: unsafe extern "C" fn(*mut u8),

    // ---- convert
    pcre2_pattern_convert_8: unsafe extern "C" fn(
        Sptr, usize, u32, *mut *mut u8, *mut usize, Ptr) -> i32,
    pcre2_converted_pattern_free_8: unsafe extern "C" fn(*mut u8),

    // ---- misc
    pcre2_config_8: unsafe extern "C" fn(u32, Ptr) -> i32,
    pcre2_get_error_message_8: unsafe extern "C" fn(i32, *mut u8, usize) -> i32,
    pcre2_maketables_8: unsafe extern "C" fn(Ptr) -> *const u8,
    pcre2_maketables_free_8: unsafe extern "C" fn(Ptr, *const u8),

    // ---- JIT stubs (SUPPORT_JIT undefined)
    pcre2_jit_compile_8: unsafe extern "C" fn(Ptr, u32) -> i32,
    pcre2_jit_free_unused_memory_8: unsafe extern "C" fn(Ptr),
    pcre2_jit_stack_create_8: unsafe extern "C" fn(usize, usize, Ptr) -> Ptr,
    pcre2_jit_stack_assign_8: unsafe extern "C" fn(Ptr, Ptr, Ptr),
    pcre2_jit_stack_free_8: unsafe extern "C" fn(Ptr),
    _pcre2_jit_free_8: unsafe extern "C" fn(Ptr, Ptr),
    _pcre2_jit_free_rodata_8: unsafe extern "C" fn(Ptr, Ptr),
    _pcre2_jit_get_size_8: unsafe extern "C" fn(Ptr) -> usize,
    _pcre2_jit_get_target_8: unsafe extern "C" fn() -> *const i8,

    // ---- internal (PRIV) leaf functions
    _pcre2_strlen_8: unsafe extern "C" fn(Sptr) -> usize,
    _pcre2_strcmp_8: unsafe extern "C" fn(Sptr, Sptr) -> i32,
    _pcre2_strcmp_c8_8: unsafe extern "C" fn(Sptr, *const i8) -> i32,
    _pcre2_strncmp_8: unsafe extern "C" fn(Sptr, Sptr, usize) -> i32,
    _pcre2_strncmp_c8_8: unsafe extern "C" fn(Sptr, *const i8, usize) -> i32,
    _pcre2_strcpy_c8_8: unsafe extern "C" fn(*mut u8, *const i8) -> usize,
    _pcre2_ord2utf_8: unsafe extern "C" fn(u32, *mut u8) -> u32,
    _pcre2_valid_utf_8: unsafe extern "C" fn(Sptr, usize, *mut usize) -> i32,
    _pcre2_is_newline_8: unsafe extern "C" fn(Sptr, u32, Sptr, *mut u32, i32) -> i32,
    _pcre2_was_newline_8: unsafe extern "C" fn(Sptr, u32, Sptr, *mut u32, i32) -> i32,
    _pcre2_extuni_8: unsafe extern "C" fn(u32, Sptr, Sptr, Sptr, i32, *mut i32) -> Sptr,
    _pcre2_script_run_8: unsafe extern "C" fn(Sptr, Sptr, i32) -> i32,
    _pcre2_xclass_8: unsafe extern "C" fn(u32, Sptr, *const u8, i32) -> i32,
    _pcre2_eclass_8: unsafe extern "C" fn(u32, Sptr, Sptr, *const u8, i32) -> i32,
    _pcre2_ckd_smul_8: unsafe extern "C" fn(*mut usize, i32, i32) -> i32,
    _pcre2_update_classbits_8: unsafe extern "C" fn(u32, u32, i32, *mut u8),
    _pcre2_memctl_malloc_8: unsafe extern "C" fn(usize, Ptr) -> Ptr,
    _pcre2_find_bracket_8: unsafe extern "C" fn(Sptr, i32, i32) -> Sptr,
    // C prototype: uint16_t PRIV(compile_get_hash_from_name)(PCRE2_SPTR, uint32_t)
    _pcre2_compile_get_hash_from_name8: unsafe extern "C" fn(Sptr, u32) -> u16,
}

impl Api {
    /// Look up any exported symbol not present in the `Api` struct, as a typed function
    /// pointer. `T` must be an `unsafe extern "C" fn(..) -> ..` type.
    pub unsafe fn raw<T: Copy>(&self, name: &str) -> T {
        let s: Symbol<T> = self
            ._lib
            .get(format!("{}\0", name).as_bytes())
            .unwrap_or_else(|e| panic!("{}: symbol {} missing: {}", self.tag, name, e));
        *s
    }

    pub fn data(&self, name: &str) -> *const u8 {
        unsafe {
            let s: Symbol<*const u8> = self
                ._lib
                .get(format!("{}\0", name).as_bytes())
                .unwrap_or_else(|e| panic!("{}: data symbol {} missing: {}", self.tag, name, e));
            // libloading returns a pointer to the object itself for data symbols.
            s.into_raw().into_raw() as *const u8
        }
    }
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub fn c_so() -> PathBuf {
    root().join("c_src/build/libpcre2.so")
}
pub fn rust_so() -> PathBuf {
    root().join("translation/target/release/libpcre2.so")
}

/// Load both libraries. Returns `(c, rust)`.
pub fn both() -> &'static (Api, Api) {
    use std::sync::OnceLock;
    static ONCE: OnceLock<(Api, Api)> = OnceLock::new();
    ONCE.get_or_init(|| {
        for p in [c_so(), rust_so()] {
            assert!(
                p.exists(),
                "missing shared object {:?}\n  build C:    cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n  build Rust: cd translation && cargo build --release",
                p
            );
        }
        (Api::build("C", c_so()), Api::build("RUST", rust_so()))
    })
}

// ============================================================ small helpers

/// Deterministic xorshift RNG so every test run uses the same inputs.
pub struct Rng(pub u64);
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
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            0
        } else {
            (self.next_u64() >> 11) as u32 % n
        }
    }
    pub fn pick<'a, T>(&mut self, v: &'a [T]) -> &'a T {
        &v[self.below(v.len() as u32) as usize]
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
}

/// One compiled-pattern probe: the observable result of `pcre2_compile`.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CompileOut {
    pub ok: bool,
    pub errorcode: i32,
    pub erroroffset: usize,
    /// pattern_info values for every key that returns a scalar
    pub info: Vec<(u32, i32, u64)>,
    /// the compiled byte image (PCRE2_INFO_SIZE bytes) with pointer fields masked out
    pub image: Vec<u8>,
}

/// Scalar pattern_info keys and the width of their output (4 = uint32_t, 8 = size_t).
pub const INFO_SCALARS: &[(u32, usize)] = &[
    (PCRE2_INFO_ALLOPTIONS, 4),
    (PCRE2_INFO_ARGOPTIONS, 4),
    (PCRE2_INFO_EXTRAOPTIONS, 4),
    (PCRE2_INFO_BACKREFMAX, 4),
    (PCRE2_INFO_BSR, 4),
    (PCRE2_INFO_CAPTURECOUNT, 4),
    (PCRE2_INFO_FIRSTCODEUNIT, 4),
    (PCRE2_INFO_FIRSTCODETYPE, 4),
    (PCRE2_INFO_HASCRORLF, 4),
    (PCRE2_INFO_JCHANGED, 4),
    (PCRE2_INFO_LASTCODEUNIT, 4),
    (PCRE2_INFO_LASTCODETYPE, 4),
    (PCRE2_INFO_MATCHEMPTY, 4),
    (PCRE2_INFO_MAXLOOKBEHIND, 4),
    (PCRE2_INFO_NAMECOUNT, 4),
    (PCRE2_INFO_NAMEENTRYSIZE, 4),
    (PCRE2_INFO_NEWLINE, 4),
    (PCRE2_INFO_HASBACKSLASHC, 4),
    (PCRE2_INFO_MATCHLIMIT, 4),
    (PCRE2_INFO_DEPTHLIMIT, 4),
    (PCRE2_INFO_HEAPLIMIT, 4),
    (PCRE2_INFO_MINLENGTH, 4),
    (PCRE2_INFO_JITSIZE, 8),
    (PCRE2_INFO_SIZE, 8),
    (PCRE2_INFO_FRAMESIZE, 8),
];

impl Api {
    /// Compile a pattern and collect everything observable about the result.
    pub unsafe fn compile_probe(
        &self,
        pattern: &[u8],
        plen: usize,
        options: u32,
        ccontext: Ptr,
    ) -> CompileOut {
        let mut ec: i32 = 0;
        let mut eo: usize = 0;
        let code = (self.pcre2_compile_8)(
            pattern.as_ptr(),
            plen,
            options,
            &mut ec,
            &mut eo,
            ccontext,
        );
        if code.is_null() {
            return CompileOut {
                ok: false,
                errorcode: ec,
                erroroffset: eo,
                info: vec![],
                image: vec![],
            };
        }
        let mut info = vec![];
        for &(k, w) in INFO_SCALARS {
            let mut buf = [0u8; 8];
            let rc = (self.pcre2_pattern_info_8)(code, k, buf.as_mut_ptr() as Ptr);
            let v = if w == 4 {
                u32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]) as u64
            } else {
                u64::from_ne_bytes(buf)
            };
            info.push((k, rc, if rc == 0 { v } else { 0 }));
        }
        // name table
        let mut ncount: u32 = 0;
        let mut nsize: u32 = 0;
        (self.pcre2_pattern_info_8)(code, PCRE2_INFO_NAMECOUNT, &mut ncount as *mut u32 as Ptr);
        (self.pcre2_pattern_info_8)(
            code,
            PCRE2_INFO_NAMEENTRYSIZE,
            &mut nsize as *mut u32 as Ptr,
        );
        let mut ntab: *const u8 = std::ptr::null();
        (self.pcre2_pattern_info_8)(
            code,
            PCRE2_INFO_NAMETABLE,
            &mut ntab as *mut *const u8 as Ptr,
        );
        let mut image = vec![];
        if !ntab.is_null() {
            image.extend_from_slice(std::slice::from_raw_parts(
                ntab,
                (ncount * nsize) as usize,
            ));
        }
        // start-code bitmap
        let mut bm: *const u8 = std::ptr::null();
        if (self.pcre2_pattern_info_8)(
            code,
            PCRE2_INFO_FIRSTBITMAP,
            &mut bm as *mut *const u8 as Ptr,
        ) == 0
            && !bm.is_null()
        {
            image.extend_from_slice(std::slice::from_raw_parts(bm, 32));
        }
        // The serialized form of the compiled pattern is a deterministic byte image of
        // the whole `pcre2_real_code` block (pointer fields are zeroed by the encoder),
        // so comparing it compares every compiled opcode AND the struct layout.
        let mut sbytes: *mut u8 = std::ptr::null_mut();
        let mut ssize: usize = 0;
        let codes = [code];
        let rc = (self.pcre2_serialize_encode_8)(
            codes.as_ptr() as *const Ptr,
            1,
            &mut sbytes,
            &mut ssize,
            std::ptr::null_mut(),
        );
        if rc == 1 && !sbytes.is_null() {
            // skip the leading TABLES_LENGTH tables copy? no - include everything,
            // it is identical for identical tables and is part of the observable output.
            image.extend_from_slice(std::slice::from_raw_parts(sbytes, ssize));
            (self.pcre2_serialize_free_8)(sbytes);
        } else {
            image.extend_from_slice(b"SERIALIZE_FAIL");
            image.extend_from_slice(&rc.to_ne_bytes());
        }
        (self.pcre2_code_free_8)(code);
        CompileOut {
            ok: true,
            errorcode: ec,
            erroroffset: eo,
            info,
            image,
        }
    }
}

/// The observable result of a match call.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MatchOut {
    pub rc: i32,
    pub ovector: Vec<usize>,
    pub startchar: usize,
    pub mark: Option<Vec<u8>>,
}

pub unsafe fn read_mark(api: &Api, md: Ptr) -> Option<Vec<u8>> {
    let m = (api.pcre2_get_mark_8)(md);
    if m.is_null() {
        None
    } else {
        let n = (api._pcre2_strlen_8)(m);
        Some(std::slice::from_raw_parts(m, n).to_vec())
    }
}

pub unsafe fn read_match(api: &Api, md: Ptr, rc: i32) -> MatchOut {
    let n = (api.pcre2_get_ovector_count_8)(md);
    let ov = (api.pcre2_get_ovector_pointer_8)(md);
    MatchOut {
        rc,
        ovector: std::slice::from_raw_parts(ov, (n * 2) as usize).to_vec(),
        startchar: (api.pcre2_get_startchar_8)(md),
        // The mark field is only defined after the matcher actually ran (a match, a
        // NOMATCH or a PARTIAL). After an early-return error such as
        // PCRE2_ERROR_BADOFFSETLIMIT the field still holds whatever the allocator
        // handed over, so dereferencing it is not valid for either library.
        mark: if rc >= PCRE2_ERROR_PARTIAL {
            read_mark(api, md)
        } else {
            None
        },
    }
}

/// A general context whose allocator ZEROES every block, so that fields the C leaves
/// untouched on an error path (the ovector, `startchar`, `mark`) are deterministic and can
/// be compared between the two libraries instead of holding allocator garbage.
pub unsafe extern "C" fn zeroing_malloc(size: usize, _d: Ptr) -> Ptr {
    let n = size.max(1);
    let layout = std::alloc::Layout::from_size_align(n + 32, 32).unwrap();
    let base = std::alloc::alloc_zeroed(layout);
    if base.is_null() {
        return std::ptr::null_mut();
    }
    *(base as *mut usize) = n + 32;
    base.add(32) as Ptr
}
pub unsafe extern "C" fn zeroing_free(p: Ptr, _d: Ptr) {
    if p.is_null() {
        return;
    }
    let base = (p as *mut u8).sub(32);
    let n = *(base as *mut usize);
    std::alloc::dealloc(base, std::alloc::Layout::from_size_align(n, 32).unwrap());
}

pub unsafe fn zeroing_context(api: &Api) -> Ptr {
    (api.pcre2_general_context_create_8)(Some(zeroing_malloc), Some(zeroing_free), std::ptr::null_mut())
}
