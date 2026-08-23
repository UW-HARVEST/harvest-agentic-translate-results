// Differential-test harness for the PCRE2 C -> Rust translation.
//
// BOTH libraries are loaded as shared objects through `libloading` and every
// call crosses the FFI boundary.  The Rust implementation is NEVER called
// directly as a Rust crate: that way the `#[no_mangle] extern "C"` export
// wrappers are part of what is under test, exactly as an external C consumer
// would see them.
//
// Both `.so`s export the *same* symbol names, so they are opened with
// RTLD_LOCAL and looked up through their own handle -- no interposition.

#![allow(dead_code)]

use libloading::os::unix::{Library as UnixLibrary, Symbol as UnixSymbol, RTLD_LOCAL, RTLD_NOW};
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::ptr;

// ----------------------------------------------------------------基本 types

pub type Sz = usize; // PCRE2_SIZE
pub type Sptr = *const u8; // PCRE2_SPTR  (8-bit width)
pub type Uchar = u8; // PCRE2_UCHAR
pub type Bool = c_int; // BOOL == int
pub type Ptr = *mut c_void;

pub type MallocFn = unsafe extern "C" fn(usize, *mut c_void) -> *mut c_void;
pub type FreeFn = unsafe extern "C" fn(*mut c_void, *mut c_void);
pub type CalloutFn = unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int;
pub type GuardFn = unsafe extern "C" fn(u32, *mut c_void) -> c_int;
pub type CaseFn = unsafe extern "C" fn(Sptr, Sz, *mut u8, Sz, c_int, *mut c_void) -> Sz;

// --------------------------------------------------------------- constants

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
pub const PCRE2_START_OPTIMIZE: u32 = 68;
pub const PCRE2_START_OPTIMIZE_OFF: u32 = 69;

pub const PCRE2_ZERO_TERMINATED: Sz = usize::MAX;
pub const PCRE2_UNSET: Sz = usize::MAX;

// error codes actually asserted on
pub const PCRE2_ERROR_NOMATCH: c_int = -1;
pub const PCRE2_ERROR_PARTIAL: c_int = -2;
pub const PCRE2_ERROR_BADDATA: c_int = -29;
pub const PCRE2_ERROR_MIXEDTABLES: c_int = -30;
pub const PCRE2_ERROR_BADMAGIC: c_int = -31;
pub const PCRE2_ERROR_BADMODE: c_int = -32;
pub const PCRE2_ERROR_BADOFFSET: c_int = -33;
pub const PCRE2_ERROR_BADOPTION: c_int = -34;
pub const PCRE2_ERROR_BADREPLACEMENT: c_int = -35;
pub const PCRE2_ERROR_BADUTFOFFSET: c_int = -36;
pub const PCRE2_ERROR_DFA_BADRESTART: c_int = -38;
pub const PCRE2_ERROR_DFA_UCOND: c_int = -40;
pub const PCRE2_ERROR_DFA_UFUNC: c_int = -41;
pub const PCRE2_ERROR_DFA_UITEM: c_int = -42;
pub const PCRE2_ERROR_DFA_WSSIZE: c_int = -43;
pub const PCRE2_ERROR_INTERNAL: c_int = -44;
pub const PCRE2_ERROR_JIT_BADOPTION: c_int = -45;
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

// ------------------------------------------------------------- the Api table

macro_rules! def_api {
    ( $( $field:ident : $sym:literal : $t:ty ),* $(,)? ) => {
        pub struct Api {
            pub name: &'static str,
            pub lib: &'static UnixLibrary,
            $( pub $field: $t, )*
        }

        impl Api {
            pub fn load(name: &'static str, path: &std::path::Path) -> Api {
                let lib: &'static UnixLibrary = Box::leak(Box::new(unsafe {
                    UnixLibrary::open(Some(path), RTLD_NOW | RTLD_LOCAL)
                }
                .unwrap_or_else(|e| panic!("cannot dlopen {}: {}", path.display(), e))));
                Api {
                    name,
                    lib,
                    $( $field: unsafe {
                        let s: UnixSymbol<$t> = lib
                            .get(concat!($sym, "\0").as_bytes())
                            .unwrap_or_else(|e| {
                                panic!("[{}] missing symbol {}: {}", name, $sym, e)
                            });
                        *s
                    }, )*
                }
            }

            /// Address of an exported *data* symbol.
            pub fn data(&self, sym: &str) -> *const u8 {
                let mut owned = sym.as_bytes().to_vec();
                owned.push(0);
                unsafe {
                    let s: UnixSymbol<*const u8> = self
                        .lib
                        .get(&owned)
                        .unwrap_or_else(|e| panic!("[{}] missing data symbol {}: {}", self.name, sym, e));
                    s.into_raw() as *const u8
                }
            }
        }
    };
}

def_api! {
    // ---- general information
    config: "pcre2_config_8": unsafe extern "C" fn(u32, Ptr) -> c_int,
    get_error_message: "pcre2_get_error_message_8": unsafe extern "C" fn(c_int, *mut u8, Sz) -> c_int,
    maketables: "pcre2_maketables_8": unsafe extern "C" fn(Ptr) -> *const u8,
    maketables_free: "pcre2_maketables_free_8": unsafe extern "C" fn(Ptr, *const u8),

    // ---- general context
    general_context_create: "pcre2_general_context_create_8":
        unsafe extern "C" fn(Option<MallocFn>, Option<FreeFn>, Ptr) -> Ptr,
    general_context_copy: "pcre2_general_context_copy_8": unsafe extern "C" fn(Ptr) -> Ptr,
    general_context_free: "pcre2_general_context_free_8": unsafe extern "C" fn(Ptr),

    // ---- compile context
    compile_context_create: "pcre2_compile_context_create_8": unsafe extern "C" fn(Ptr) -> Ptr,
    compile_context_copy: "pcre2_compile_context_copy_8": unsafe extern "C" fn(Ptr) -> Ptr,
    compile_context_free: "pcre2_compile_context_free_8": unsafe extern "C" fn(Ptr),
    set_bsr: "pcre2_set_bsr_8": unsafe extern "C" fn(Ptr, u32) -> c_int,
    set_character_tables: "pcre2_set_character_tables_8": unsafe extern "C" fn(Ptr, *const u8) -> c_int,
    set_compile_extra_options: "pcre2_set_compile_extra_options_8": unsafe extern "C" fn(Ptr, u32) -> c_int,
    set_max_pattern_length: "pcre2_set_max_pattern_length_8": unsafe extern "C" fn(Ptr, Sz) -> c_int,
    set_max_pattern_compiled_length: "pcre2_set_max_pattern_compiled_length_8":
        unsafe extern "C" fn(Ptr, Sz) -> c_int,
    set_max_varlookbehind: "pcre2_set_max_varlookbehind_8": unsafe extern "C" fn(Ptr, u32) -> c_int,
    set_newline: "pcre2_set_newline_8": unsafe extern "C" fn(Ptr, u32) -> c_int,
    set_parens_nest_limit: "pcre2_set_parens_nest_limit_8": unsafe extern "C" fn(Ptr, u32) -> c_int,
    set_compile_recursion_guard: "pcre2_set_compile_recursion_guard_8":
        unsafe extern "C" fn(Ptr, Option<GuardFn>, Ptr) -> c_int,
    set_optimize: "pcre2_set_optimize_8": unsafe extern "C" fn(Ptr, u32) -> c_int,

    // ---- match context
    match_context_create: "pcre2_match_context_create_8": unsafe extern "C" fn(Ptr) -> Ptr,
    match_context_copy: "pcre2_match_context_copy_8": unsafe extern "C" fn(Ptr) -> Ptr,
    match_context_free: "pcre2_match_context_free_8": unsafe extern "C" fn(Ptr),
    set_callout: "pcre2_set_callout_8": unsafe extern "C" fn(Ptr, Option<CalloutFn>, Ptr) -> c_int,
    set_substitute_callout: "pcre2_set_substitute_callout_8":
        unsafe extern "C" fn(Ptr, Option<CalloutFn>, Ptr) -> c_int,
    set_substitute_case_callout: "pcre2_set_substitute_case_callout_8":
        unsafe extern "C" fn(Ptr, Option<CaseFn>, Ptr) -> c_int,
    set_depth_limit: "pcre2_set_depth_limit_8": unsafe extern "C" fn(Ptr, u32) -> c_int,
    set_heap_limit: "pcre2_set_heap_limit_8": unsafe extern "C" fn(Ptr, u32) -> c_int,
    set_match_limit: "pcre2_set_match_limit_8": unsafe extern "C" fn(Ptr, u32) -> c_int,
    set_offset_limit: "pcre2_set_offset_limit_8": unsafe extern "C" fn(Ptr, Sz) -> c_int,
    set_recursion_limit: "pcre2_set_recursion_limit_8": unsafe extern "C" fn(Ptr, u32) -> c_int,
    set_recursion_memory_management: "pcre2_set_recursion_memory_management_8":
        unsafe extern "C" fn(Ptr, Option<MallocFn>, Option<FreeFn>, Ptr) -> c_int,

    // ---- convert context
    convert_context_create: "pcre2_convert_context_create_8": unsafe extern "C" fn(Ptr) -> Ptr,
    convert_context_copy: "pcre2_convert_context_copy_8": unsafe extern "C" fn(Ptr) -> Ptr,
    convert_context_free: "pcre2_convert_context_free_8": unsafe extern "C" fn(Ptr),
    set_glob_escape: "pcre2_set_glob_escape_8": unsafe extern "C" fn(Ptr, u32) -> c_int,
    set_glob_separator: "pcre2_set_glob_separator_8": unsafe extern "C" fn(Ptr, u32) -> c_int,

    // ---- compile
    compile: "pcre2_compile_8":
        unsafe extern "C" fn(Sptr, Sz, u32, *mut c_int, *mut Sz, Ptr) -> Ptr,
    code_free: "pcre2_code_free_8": unsafe extern "C" fn(Ptr),
    code_copy: "pcre2_code_copy_8": unsafe extern "C" fn(Ptr) -> Ptr,
    code_copy_with_tables: "pcre2_code_copy_with_tables_8": unsafe extern "C" fn(Ptr) -> Ptr,

    // ---- pattern info
    pattern_info: "pcre2_pattern_info_8": unsafe extern "C" fn(Ptr, u32, Ptr) -> c_int,
    callout_enumerate: "pcre2_callout_enumerate_8":
        unsafe extern "C" fn(Ptr, Option<CalloutFn>, Ptr) -> c_int,

    // ---- match
    match_data_create: "pcre2_match_data_create_8": unsafe extern "C" fn(u32, Ptr) -> Ptr,
    match_data_create_from_pattern: "pcre2_match_data_create_from_pattern_8":
        unsafe extern "C" fn(Ptr, Ptr) -> Ptr,
    match_data_free: "pcre2_match_data_free_8": unsafe extern "C" fn(Ptr),
    do_match: "pcre2_match_8":
        unsafe extern "C" fn(Ptr, Sptr, Sz, Sz, u32, Ptr, Ptr) -> c_int,
    dfa_match: "pcre2_dfa_match_8":
        unsafe extern "C" fn(Ptr, Sptr, Sz, Sz, u32, Ptr, Ptr, *mut c_int, Sz) -> c_int,
    get_mark: "pcre2_get_mark_8": unsafe extern "C" fn(Ptr) -> Sptr,
    get_match_data_size: "pcre2_get_match_data_size_8": unsafe extern "C" fn(Ptr) -> Sz,
    get_match_data_heapframes_size: "pcre2_get_match_data_heapframes_size_8":
        unsafe extern "C" fn(Ptr) -> Sz,
    get_ovector_count: "pcre2_get_ovector_count_8": unsafe extern "C" fn(Ptr) -> u32,
    get_ovector_pointer: "pcre2_get_ovector_pointer_8": unsafe extern "C" fn(Ptr) -> *mut Sz,
    get_startchar: "pcre2_get_startchar_8": unsafe extern "C" fn(Ptr) -> Sz,
    next_match: "pcre2_next_match_8": unsafe extern "C" fn(Ptr, *mut Sz, *mut u32) -> c_int,

    // ---- substring
    substring_copy_byname: "pcre2_substring_copy_byname_8":
        unsafe extern "C" fn(Ptr, Sptr, *mut u8, *mut Sz) -> c_int,
    substring_copy_bynumber: "pcre2_substring_copy_bynumber_8":
        unsafe extern "C" fn(Ptr, u32, *mut u8, *mut Sz) -> c_int,
    substring_free: "pcre2_substring_free_8": unsafe extern "C" fn(*mut u8),
    substring_get_byname: "pcre2_substring_get_byname_8":
        unsafe extern "C" fn(Ptr, Sptr, *mut *mut u8, *mut Sz) -> c_int,
    substring_get_bynumber: "pcre2_substring_get_bynumber_8":
        unsafe extern "C" fn(Ptr, u32, *mut *mut u8, *mut Sz) -> c_int,
    substring_length_byname: "pcre2_substring_length_byname_8":
        unsafe extern "C" fn(Ptr, Sptr, *mut Sz) -> c_int,
    substring_length_bynumber: "pcre2_substring_length_bynumber_8":
        unsafe extern "C" fn(Ptr, u32, *mut Sz) -> c_int,
    substring_nametable_scan: "pcre2_substring_nametable_scan_8":
        unsafe extern "C" fn(Ptr, Sptr, *mut Sptr, *mut Sptr) -> c_int,
    substring_number_from_name: "pcre2_substring_number_from_name_8":
        unsafe extern "C" fn(Ptr, Sptr) -> c_int,
    substring_list_free: "pcre2_substring_list_free_8": unsafe extern "C" fn(*mut *mut u8),
    substring_list_get: "pcre2_substring_list_get_8":
        unsafe extern "C" fn(Ptr, *mut *mut *mut u8, *mut *mut Sz) -> c_int,

    // ---- serialize
    serialize_encode: "pcre2_serialize_encode_8":
        unsafe extern "C" fn(*const Ptr, i32, *mut *mut u8, *mut Sz, Ptr) -> i32,
    serialize_decode: "pcre2_serialize_decode_8":
        unsafe extern "C" fn(*mut Ptr, i32, *const u8, Ptr) -> i32,
    serialize_get_number_of_codes: "pcre2_serialize_get_number_of_codes_8":
        unsafe extern "C" fn(*const u8) -> i32,
    serialize_free: "pcre2_serialize_free_8": unsafe extern "C" fn(*mut u8),

    // ---- substitute
    substitute: "pcre2_substitute_8":
        unsafe extern "C" fn(Ptr, Sptr, Sz, Sz, u32, Ptr, Ptr, Sptr, Sz, *mut u8, *mut Sz) -> c_int,

    // ---- convert
    pattern_convert: "pcre2_pattern_convert_8":
        unsafe extern "C" fn(Sptr, Sz, u32, *mut *mut u8, *mut Sz, Ptr) -> c_int,
    converted_pattern_free: "pcre2_converted_pattern_free_8": unsafe extern "C" fn(*mut u8),

    // ---- JIT (no-JIT stubs in this configuration)
    jit_compile: "pcre2_jit_compile_8": unsafe extern "C" fn(Ptr, u32) -> c_int,
    jit_match: "pcre2_jit_match_8":
        unsafe extern "C" fn(Ptr, Sptr, Sz, Sz, u32, Ptr, Ptr) -> c_int,
    jit_free_unused_memory: "pcre2_jit_free_unused_memory_8": unsafe extern "C" fn(Ptr),
    jit_stack_create: "pcre2_jit_stack_create_8": unsafe extern "C" fn(usize, usize, Ptr) -> Ptr,
    jit_stack_assign: "pcre2_jit_stack_assign_8": unsafe extern "C" fn(Ptr, Ptr, Ptr),
    jit_stack_free: "pcre2_jit_stack_free_8": unsafe extern "C" fn(Ptr),

    // ---- exported low-level internals
    p_valid_utf: "_pcre2_valid_utf_8": unsafe extern "C" fn(Sptr, Sz, *mut Sz) -> c_int,
    p_ord2utf: "_pcre2_ord2utf_8": unsafe extern "C" fn(u32, *mut u8) -> std::ffi::c_uint,
    p_strlen: "_pcre2_strlen_8": unsafe extern "C" fn(Sptr) -> Sz,
    p_strcmp: "_pcre2_strcmp_8": unsafe extern "C" fn(Sptr, Sptr) -> c_int,
    p_strcmp_c8: "_pcre2_strcmp_c8_8": unsafe extern "C" fn(Sptr, *const c_char) -> c_int,
    p_strncmp: "_pcre2_strncmp_8": unsafe extern "C" fn(Sptr, Sptr, usize) -> c_int,
    p_strncmp_c8: "_pcre2_strncmp_c8_8": unsafe extern "C" fn(Sptr, *const c_char, usize) -> c_int,
    p_strcpy_c8: "_pcre2_strcpy_c8_8": unsafe extern "C" fn(*mut u8, *const c_char) -> Sz,
    p_ckd_smul: "_pcre2_ckd_smul_8": unsafe extern "C" fn(*mut Sz, c_int, c_int) -> Bool,
    p_is_newline: "_pcre2_is_newline_8":
        unsafe extern "C" fn(Sptr, u32, Sptr, *mut u32, Bool) -> Bool,
    p_was_newline: "_pcre2_was_newline_8":
        unsafe extern "C" fn(Sptr, u32, Sptr, *mut u32, Bool) -> Bool,
    p_extuni: "_pcre2_extuni_8":
        unsafe extern "C" fn(u32, Sptr, Sptr, Sptr, Bool, *mut c_int) -> Sptr,
    p_script_run: "_pcre2_script_run_8": unsafe extern "C" fn(Sptr, Sptr, Bool) -> Bool,
    p_xclass: "_pcre2_xclass_8": unsafe extern "C" fn(u32, Sptr, *const u8, Bool) -> Bool,
    p_eclass: "_pcre2_eclass_8": unsafe extern "C" fn(u32, Sptr, Sptr, *const u8, Bool) -> Bool,
    p_find_bracket: "_pcre2_find_bracket_8": unsafe extern "C" fn(Sptr, Bool, c_int) -> Sptr,
    p_update_classbits: "_pcre2_update_classbits_8":
        unsafe extern "C" fn(u32, u32, Bool, *mut u8),
    p_get_hash_from_name: "_pcre2_compile_get_hash_from_name8":
        unsafe extern "C" fn(Sptr, u32) -> u16,
    p_study: "_pcre2_study_8": unsafe extern "C" fn(Ptr) -> c_int,
    p_memctl_malloc: "_pcre2_memctl_malloc_8": unsafe extern "C" fn(usize, Ptr) -> Ptr,
    p_jit_get_size: "_pcre2_jit_get_size_8": unsafe extern "C" fn(Ptr) -> usize,
    p_jit_get_target: "_pcre2_jit_get_target_8": unsafe extern "C" fn() -> *const c_char,
    p_jit_free: "_pcre2_jit_free_8": unsafe extern "C" fn(Ptr, Ptr),
    p_jit_free_rodata: "_pcre2_jit_free_rodata_8": unsafe extern "C" fn(Ptr, Ptr),
    p_auto_possessify: "_pcre2_auto_possessify_8": unsafe extern "C" fn(*mut u8, Ptr) -> c_int,
}

// ----------------------------------------------------------------- loading

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn require(p: PathBuf, what: &str) -> PathBuf {
    assert!(
        p.exists(),
        "{} shared library not found at {}\n\
         Build it first:\n  C:    cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n  \
         Rust: cargo build --release",
        what,
        p.display()
    );
    p
}

/// The pair of libraries under test.
pub struct Pair {
    pub c: Api,
    pub r: Api,
}

static mut PAIR: Option<Pair> = None;
static ONCE: std::sync::Once = std::sync::Once::new();

/// Load (once per test binary) the C and the Rust `.so`.
pub fn pair() -> &'static Pair {
    unsafe {
        ONCE.call_once(|| {
            let root = crate_root();
            let c_path = require(root.join("c_src/build/libpcre2.so"), "C");
            // Prefer the release cdylib: the release profile is the one that
            // disables overflow checks / debug assertions, matching C.
            let rel = root.join("target/release/libpcre2.so");
            let dbg = root.join("target/debug/libpcre2.so");
            let r_path = if rel.exists() { rel } else { require(dbg, "Rust") };
            PAIR = Some(Pair {
                c: Api::load("C", &c_path),
                r: Api::load("rust", &r_path),
            });
        });
        #[allow(static_mut_refs)]
        PAIR.as_ref().unwrap()
    }
}

// ------------------------------------------------------------- tiny PRNG

/// SplitMix64 — deterministic, seedable, no external crates.
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn range(&mut self, lo: usize, hi_incl: usize) -> usize {
        lo + self.below(hi_incl - lo + 1)
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }
    pub fn chance(&mut self, one_in: usize) -> bool {
        self.below(one_in) == 0
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
    /// `pick` for slices-of-slices, returning the inner slice directly.
    pub fn pick_bytes<'a>(&mut self, xs: &[&'a [u8]]) -> &'a [u8] {
        xs[self.below(xs.len())]
    }
}

// ------------------------------------------------------- subject generators

/// Random ASCII-ish bytes drawn from a small alphabet: produces plenty of
/// matches instead of mostly-NOMATCH noise.
pub fn gen_ascii(rng: &mut Rng, max_len: usize) -> Vec<u8> {
    const AL: &[u8] = b"aAbBcC01 \t\r\n_-.:;xyzZ&@#";
    let n = rng.below(max_len + 1);
    (0..n).map(|_| *rng.pick(AL)).collect()
}

/// Random *valid* UTF-8 covering 1/2/3/4-byte forms.
pub fn gen_utf8(rng: &mut Rng, max_chars: usize) -> Vec<u8> {
    let n = rng.below(max_chars + 1);
    let mut out = Vec::new();
    for _ in 0..n {
        let cp: u32 = match rng.below(10) {
            0..=3 => rng.range(0x20, 0x7e) as u32,
            4..=5 => rng.range(0x80, 0x7ff) as u32,
            6..=7 => {
                // avoid surrogates
                let v = rng.range(0x800, 0xffff) as u32;
                if (0xd800..=0xdfff).contains(&v) {
                    0x2028
                } else {
                    v
                }
            }
            _ => rng.range(0x1_0000, 0x10_ffff) as u32,
        };
        let mut buf = [0u8; 4];
        let s = char::from_u32(cp).unwrap_or('?').encode_utf8(&mut buf);
        out.extend_from_slice(s.as_bytes());
    }
    out
}

/// Random bytes, mostly invalid UTF-8 — for the UTF validity checks.
pub fn gen_raw(rng: &mut Rng, max_len: usize) -> Vec<u8> {
    let n = rng.below(max_len + 1);
    (0..n).map(|_| rng.byte()).collect()
}

// ------------------------------------------------------------ comparators

/// The PCRE2 `pcre2_code` magic ('PCRE'), used to self-check the layout below.
pub const MAGIC_NUMBER: u32 = 0x5043_5245;

/// Compiled-code accessors. Mirrors `pcre2_real_code` from
/// `c_src/src/pcre2_intmodedep.h`, where
/// `#define CODE_BLOCKSIZE_TYPE PCRE2_SIZE`, i.e. `usize` — NOT `uint32_t`.
#[repr(C)]
pub struct RealCodeHead {
    pub memctl_malloc: *mut c_void,
    pub memctl_free: *mut c_void,
    pub memctl_data: *mut c_void,
    pub tables: *const u8,
    pub executable_jit: *mut c_void,
    pub start_bitmap: [u8; 32],
    pub blocksize: usize,
    pub code_start: usize,
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

/// Pointer to the first byte of compiled bytecode inside a `pcre2_code`.
pub unsafe fn bytecode_ptr(code: Ptr) -> Sptr {
    let head = check_head(code);
    (code as *const u8).add(head.code_start)
}

pub unsafe fn code_blocksize(code: Ptr) -> usize {
    check_head(code).blocksize
}

/// Validates that `RealCodeHead` really does describe this `pcre2_code` before
/// any offset derived from it is used — a silently wrong layout would make the
/// bytecode comparison meaningless.
pub unsafe fn check_head(code: Ptr) -> &'static RealCodeHead {
    let h = &*(code as *const RealCodeHead);
    assert_eq!(
        h.magic_number, MAGIC_NUMBER,
        "RealCodeHead layout is wrong: magic_number read as {:#x}, expected {:#x}",
        h.magic_number, MAGIC_NUMBER
    );
    assert!(
        h.code_start >= std::mem::size_of::<RealCodeHead>() && h.code_start < h.blocksize,
        "implausible code_start={} (blocksize={}, sizeof head={})",
        h.code_start,
        h.blocksize,
        std::mem::size_of::<RealCodeHead>()
    );
    h
}

/// Byte-for-byte comparison of two compiled patterns, skipping only the fields
/// that legitimately hold host addresses (allocator pointers, tables pointer,
/// JIT pointer).
/// `PCRE2_DEREF_TABLES` from `pcre2_internal.h`: set by
/// `pcre2_serialize_decode` because the character tables are then part of the
/// code block and must be released with it.
pub const PCRE2_DEREF_TABLES: u32 = 0x0004_0000;

pub unsafe fn assert_code_eq(a: Ptr, b: Ptr, ctx: &str) {
    assert_code_eq_masked(a, b, 0, ctx)
}

/// As `assert_code_eq`, but `allow_flags` names flag bits that are permitted to
/// differ (and must differ exactly in that way).
pub unsafe fn assert_code_eq_masked(a: Ptr, b: Ptr, allow_flags: u32, ctx: &str) {
    if allow_flags != 0 {
        let (ha, hb) = (&*(a as *const RealCodeHead), &*(b as *const RealCodeHead));
        assert_eq!(
            ha.flags & !allow_flags,
            hb.flags & !allow_flags,
            "{ctx}: `flags` differ outside the permitted mask {allow_flags:#x} \
             (a={:#x} b={:#x})",
            ha.flags,
            hb.flags
        );
    }
    assert_code_eq_inner(a, b, allow_flags, ctx)
}

unsafe fn assert_code_eq_inner(a: Ptr, b: Ptr, allow_flags: u32, ctx: &str) {
    let (ha, hb) = (a as *const RealCodeHead, b as *const RealCodeHead);
    let (sa, sb) = (code_blocksize(a), code_blocksize(b));
    assert_eq!(sa, sb, "{ctx}: blocksize differs (C={sa} rust={sb})");
    assert_eq!(
        (*ha).code_start,
        (*hb).code_start,
        "{ctx}: code_start differs"
    );

    macro_rules! f {
        ($($n:ident),*) => { $(
            assert_eq!((*ha).$n, (*hb).$n, "{}: field `{}` differs", ctx, stringify!($n));
        )* };
    }
    f!(
        start_bitmap,
        blocksize,
        code_start,
        magic_number,
        compile_options,
        overall_options,
        extra_options,
        limit_heap,
        limit_match,
        limit_depth,
        first_codeunit,
        last_codeunit,
        bsr_convention,
        newline_convention,
        max_lookbehind,
        minlength,
        top_bracket,
        top_backref,
        name_entry_size,
        name_count,
        optimization_flags
    );
    // `flags` is compared outside the permitted mask (see assert_code_eq_masked)
    assert_eq!(
        (*ha).flags & !allow_flags,
        (*hb).flags & !allow_flags,
        "{ctx}: field `flags` differs (a={:#x} b={:#x}, permitted mask {allow_flags:#x})",
        (*ha).flags,
        (*hb).flags
    );

    // everything from code_start to blocksize: name table + bytecode
    let off = (*ha).code_start as usize;
    let ba = std::slice::from_raw_parts((a as *const u8).add(off), sa - off);
    let bb = std::slice::from_raw_parts((b as *const u8).add(off), sa - off);
    if ba != bb {
        let at = ba.iter().zip(bb).position(|(x, y)| x != y).unwrap();
        panic!(
            "{ctx}: bytecode differs at offset {at} (abs {}) \nC   ={:02x?}\nrust={:02x?}",
            off + at,
            &ba[at.saturating_sub(8)..(at + 24).min(ba.len())],
            &bb[at.saturating_sub(8)..(at + 24).min(bb.len())],
        );
    }
}

/// Result of one match call, in a comparable form.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MatchOut {
    pub rc: c_int,
    pub ovector: Vec<Sz>,
    /// `None` when the C leaves it undefined for this return code.
    pub startchar: Option<Sz>,
    pub mark: Option<Vec<u8>>,
}

/// Reads only the parts of a `pcre2_match_data` that the C actually DEFINES for
/// the given return code.
///
/// `pcre2_match_data_create` does not zero the ovector, `mark` or `startchar`;
/// the matchers write only a documented prefix/subset. Comparing beyond that
/// would be comparing whatever the allocator happened to hand over, which
/// differs between the two libraries for reasons that are not behaviour.
///
///   * `rc > 0`  — the first `2*rc` ovector entries
///     (`rc = end_offset_top/2 + 1`, see `pcre2_match.c`).
///   * `rc == 0` — the ovector was too small, so all `2*oveccount` are written.
///   * `rc == PCRE2_ERROR_PARTIAL` — `ovector[0]` and `ovector[1]` only.
///   * any other `rc < 0` — no ovector entry is written.
///
/// `startchar` is zeroed just after argument validation and then written by the
/// matcher and by the UTF validity check, so it is defined for success,
/// NOMATCH, PARTIAL and the UTF error codes, but not for the early
/// argument-validation errors. `mark` is written only on the success / NOMATCH /
/// PARTIAL paths.
/// Which matcher produced the result — they define different fields.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Engine {
    /// `pcre2_match_8`: zeroes `startchar` right after argument validation, so
    /// it is defined for NOMATCH too.
    Match,
    /// `pcre2_dfa_match_8`: assigns `startchar` only on success and on a UTF
    /// error, never on NOMATCH.
    Dfa,
}

pub unsafe fn read_match_out(api: &Api, md: Ptr, rc: c_int) -> MatchOut {
    read_match_out_of(api, md, rc, Engine::Match)
}

pub unsafe fn read_match_out_of(api: &Api, md: Ptr, rc: c_int, engine: Engine) -> MatchOut {
    let oveccount = (api.get_ovector_count)(md) as usize;
    let ov = (api.get_ovector_pointer)(md);
    let n = if rc > 0 {
        (2 * rc as usize).min(2 * oveccount)
    } else if rc == 0 {
        2 * oveccount
    } else if rc == PCRE2_ERROR_PARTIAL {
        2.min(2 * oveccount)
    } else {
        0
    };
    let ovector = std::slice::from_raw_parts(ov, n).to_vec();
    let matcher_ran = rc >= 0 || rc == PCRE2_ERROR_NOMATCH || rc == PCRE2_ERROR_PARTIAL;
    // UTF validity errors (-3 .. -28) return early but do set startchar to the
    // offset of the bad code unit.
    let startchar_defined = match engine {
        Engine::Match => matcher_ran || (-28..=-3).contains(&rc),
        Engine::Dfa => rc >= 0 || rc == PCRE2_ERROR_PARTIAL || (-28..=-3).contains(&rc),
    };
    let markp = if matcher_ran { (api.get_mark)(md) } else { ptr::null() };
    let mark = if markp.is_null() {
        None
    } else {
        let mut v = Vec::new();
        let mut p = markp;
        while *p != 0 && v.len() < 4096 {
            v.push(*p);
            p = p.add(1);
        }
        Some(v)
    };
    MatchOut {
        rc,
        ovector,
        startchar: if startchar_defined {
            Some((api.get_startchar)(md))
        } else {
            None
        },
        mark,
    }
}

// -------------------------------------------------------------- reporting

/// Collects failures so a whole configuration row can be reported at once
/// instead of aborting on the first randomized input.
#[derive(Default)]
pub struct Diffs {
    pub rows: Vec<String>,
    pub checked: usize,
}

impl Diffs {
    pub fn new() -> Diffs {
        Diffs::default()
    }
    pub fn eq<T: PartialEq + std::fmt::Debug>(&mut self, what: &str, c: T, r: T) {
        self.checked += 1;
        if c != r {
            if self.rows.len() < 25 {
                self.rows.push(format!("{what}\n     C = {c:?}\n  rust = {r:?}"));
            }
        }
    }
    pub fn finish(self, row: &str) {
        if !self.rows.is_empty() {
            panic!(
                "CONFIGS row [{}]: {} of {} randomized inputs diverged:\n\n{}",
                row,
                self.rows.len(),
                self.checked,
                self.rows.join("\n\n")
            );
        }
        assert!(self.checked > 0, "row [{row}] asserted nothing");
    }
}

/// Escape a byte string for readable failure messages.
pub fn show(b: &[u8]) -> String {
    let mut s = String::from("\"");
    for &c in b {
        match c {
            b'\\' => s.push_str("\\\\"),
            b'"' => s.push_str("\\\""),
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(c as char),
            _ => s.push_str(&format!("\\x{c:02x}")),
        }
    }
    s.push('"');
    s
}

// ============================================================ pattern corpus

/// Patterns chosen to reach the distinct compile/match code paths: every
/// quantifier form, class form, assertion form, group form, escape form, verb,
/// and the Unicode-specific constructs.
pub const PATTERNS: &[&str] = &[
    // --- literals / empty / anchors
    "",
    "a",
    "abc",
    "^abc$",
    "\\Aabc\\z",
    "\\Aabc\\Z",
    "\\Gabc",
    "^",
    "$",
    "\\b",
    "\\Babc",
    // --- alternation
    "a|b",
    "abc|abd|abe",
    "(?:a|b|c)+",
    "|a",
    "a|",
    // --- quantifiers: greedy / lazy / possessive
    "a*",
    "a+",
    "a?",
    "a{3}",
    "a{2,}",
    "a{2,5}",
    "a{0,3}",
    "a*?",
    "a+?",
    "a??",
    "a{2,5}?",
    "a*+",
    "a++",
    "a?+",
    "a{2,5}+",
    "(ab)*",
    "(ab)+c",
    "(?:ab){2,4}",
    "[ab]{2,4}",
    ".*",
    ".+",
    ".{2,4}",
    // --- classes
    "[abc]",
    "[^abc]",
    "[a-z]",
    "[a-zA-Z0-9_]",
    "[]a]",
    "[^]a]",
    "[-a]",
    "[a-]",
    "[[:alpha:]]",
    "[[:^digit:]]",
    "[[:alnum:][:space:]]",
    "[\\d\\s]",
    "[\\D\\S\\W]",
    "[\\x00-\\x1f]",
    "[\\Q]^-\\E]",
    "[\\h\\v]",
    // --- escapes
    "\\d+",
    "\\D+",
    "\\w+",
    "\\W+",
    "\\s+",
    "\\S+",
    "\\h+",
    "\\H+",
    "\\v+",
    "\\V+",
    "\\R",
    "\\R+",
    "\\N",
    "\\N+",
    "\\x41",
    "\\x{41}",
    "\\101",
    "\\o{101}",
    "\\cA",
    "\\e\\a\\f\\n\\r\\t",
    "\\Qa.b*c\\E",
    "a\\Qb\\Ec",
    // --- groups & captures
    "(a)(b)(c)",
    "(a(b(c)))",
    "(?:abc)",
    "(?<n>a)(?<m>b)",
    "(?'n'a)",
    "(?P<n>a)",
    "(?>a+)b",
    "(?|(a)|(b))",
    // --- backreferences
    "(a)\\1",
    "(a)(b)\\2\\1",
    "(?<n>a)\\k<n>",
    "(?<n>a)\\k'n'",
    "(?<n>a)(?P=n)",
    "(a)\\g{1}",
    "(a)\\g{-1}",
    // --- assertions
    "a(?=b)",
    "a(?!b)",
    "(?<=a)b",
    "(?<!a)b",
    "(?<=ab|cd)x",
    "(?<=a{2,4})x",
    "(?*a)b",
    "a(?=b)(?=.)",
    // --- conditionals / recursion / subroutines
    "(a)?(?(1)b|c)",
    "(?<n>a)?(?(<n>)b|c)",
    "(?(?=a)ab|cd)",
    "\\((?:[^()]++|(?R))*\\)",
    "(a|b(?1))",
    "(?(DEFINE)(?<w>\\w+))(?&w)",
    "(a)(?2)?(b)",
    // --- inline options
    "(?i)abc",
    "(?i:abc)d",
    "(?-i)abc",
    "(?x) a b c ",
    "(?xx) a b c ",
    "(?s).",
    "(?m)^a$",
    "(?U)a+",
    "(?J)(?<n>a)|(?<n>b)",
    "(?n)(a)(b)",
    "(?i)(?-i)a",
    "(?^i)a",
    // --- \K and verbs
    "a\\Kb",
    "(*MARK:m1)a",
    "a(*SKIP)b|c",
    "a(*PRUNE)b|c",
    "a(*THEN)b|c",
    "a(*COMMIT)b|c",
    "a(*FAIL)|b",
    "a(*ACCEPT)b",
    "(*UTF)a",
    "(*UCP)\\w+",
    "(*CR)^a",
    "(*LF)^a",
    "(*CRLF)^a",
    "(*ANY)^a",
    "(*ANYCRLF)^a",
    "(*NUL)^a",
    "(*BSR_ANYCRLF)\\R",
    "(*BSR_UNICODE)\\R",
    "(*NO_START_OPT)abc",
    "(*NO_AUTO_POSSESS)a+b",
    "(*LIMIT_MATCH=1000)a+",
    "(*LIMIT_DEPTH=100)a+",
    "(*LIMIT_HEAP=1000)a+",
    // --- callouts
    "a(?C)b",
    "a(?C1)b",
    "a(?C{txt})b",
    // --- comments / whitespace
    "a(?#comment)b",
    "(?x)a # tail\n b",
    // --- Unicode properties (require UCP or UTF at match time)
    "\\p{L}+",
    "\\P{L}+",
    "\\p{Lu}",
    "\\p{Nd}",
    "\\p{Greek}",
    "\\p{^Greek}",
    "\\p{Any}",
    "\\p{Xan}",
    "\\p{Xps}",
    "\\p{Xsp}",
    "\\p{Xwd}",
    "\\p{Xuc}",
    "\\pL",
    "\\PL",
    "\\X",
    "\\X+",
    "\\p{Bidi_Control}",
    "\\p{Emoji}",
    "\\p{Script_Extensions=Latin}",
    // --- wide literals / script runs / extended classes
    "\\x{100}",
    "\\x{100}-\\x{200}",
    "[\\x{100}-\\x{200}]",
    "[^\\x{100}-\\x{200}]",
    "[\\x{100}\\p{L}a-c]",
    "(*script_run:\\w+)",
    "(*sr:.{2,})",
    "(*atomic_script_run:\\w+)",
    // --- nesting / larger shapes
    "((((((((((a))))))))))",
    "(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)",
    "(?:a|b|c|d|e|f|g|h){2,3}",
    "^(?:(a)|(b))+$",
    "(a+)+b",
    "[a-c]{1,3}[x-z]{1,3}",
    "\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}",
    "(?<y>\\d{4})-(?<m>\\d{2})-(?<d>\\d{2})",
];

/// Subject strings covering ASCII, all newline conventions, multi-byte UTF-8,
/// combining sequences, mixed scripts and the empty string.
pub const SUBJECTS: &[&str] = &[
    "",
    "a",
    "ab",
    "abc",
    "abd",
    "aaa",
    "aaaa",
    "aaab",
    "xxabcxx",
    "ABC",
    "AbC",
    "a1b2c3",
    "  a  b  ",
    "\ta\tb\t",
    "a\nb",
    "a\r\nb",
    "a\rb",
    "a\x0bb",
    "a\x0cb",
    "a\0b",
    "\n",
    "\r\n",
    "\n\n\n",
    "abc\ndef\nghi",
    "()",
    "(a(b)c)",
    "(((())))",
    "2024-01-31",
    "192.168.0.1",
    "hello world",
    "\u{85}",
    "a\u{85}b",
    "a\u{2028}b",
    "a\u{2029}b",
    "\u{e9}",
    "e\u{301}",
    "a\u{301}\u{302}b",
    "\u{100}",
    "\u{100}\u{200}",
    "\u{1ff}",
    "\u{3b1}\u{3b2}\u{3b3}",
    "\u{410}\u{411}",
    "\u{5d0}\u{5d1}",
    "\u{627}\u{628}",
    "\u{3042}\u{3044}",
    "\u{4e00}\u{4e8c}",
    "\u{1f600}",
    "\u{1f468}\u{200d}\u{1f469}",
    "\u{1f1e6}\u{1f1e7}",
    "\u{1100}\u{1161}\u{11a8}",
    "a\u{1f600}b",
    "\u{10ffff}",
    "Ab\u{130}c",
    "\u{131}\u{130}i I",
    "abc\u{410}def",
    "m1",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab",
];

// ---------------------------------------------------------- coverage records

/// Declares which `CONFIGS.md` rows a test file signs off.
/// `check_coverage.py` greps the `cfg_rows:` field out of `tests/phase_b_*.rs`,
/// so the spelling is load-bearing.
pub struct CfgCov {
    pub cfg_rows: &'static [u32],
    pub note: &'static str,
}

/// Sanity check for a file's coverage declaration.
pub fn check_coverage_decl(cov: &[CfgCov]) {
    assert!(!cov.is_empty(), "empty coverage declaration");
    let mut seen = std::collections::BTreeSet::new();
    for c in cov {
        assert!(!c.cfg_rows.is_empty(), "coverage entry {:?} lists no rows", c.note);
        assert!(!c.note.is_empty(), "coverage entry for {:?} has no note", c.cfg_rows);
        for &r in c.cfg_rows {
            assert!(r >= 1 && r <= 456, "CONFIGS.md row {r} out of range 1..=456");
            assert!(seen.insert(r), "CONFIGS.md row {r} declared twice in one file");
        }
    }
    println!("this file signs off {} CONFIGS.md rows", seen.len());
}
